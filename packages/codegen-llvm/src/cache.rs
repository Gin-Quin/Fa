//! Optional reuse of optimized LLVM modules, never of test outcomes. Analysis,
//! lowering, verification, object emission, linking and execution still run.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use sha2::{Digest, Sha256};

use crate::Options;

const FORMAT: &[u8] = b"fa-llvm-module-1\n";
const MAX_MODULE_BYTES: u64 = 16 << 20;

fn hex(bytes: &[u8]) -> String {
	use std::fmt::Write;
	let mut text = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		write!(text, "{byte:02x}").expect("writing to a string");
	}
	text
}

fn directory() -> Option<PathBuf> {
	std::env::var_os("FA_LLVM_CACHE")
		.filter(|path| !path.is_empty())
		.map(PathBuf::from)
}

/// Atomic publication makes concurrent compilers see a complete entry or a miss.
fn publish(path: &Path, bytes: &[u8]) -> io::Result<()> {
	static NEXT: AtomicU64 = AtomicU64::new(0);
	let parent = path
		.parent()
		.ok_or_else(|| io::Error::other("cache directory"))?;
	fs::create_dir_all(parent)?;
	let temporary = parent.join(format!(
		".{}-{}.tmp",
		std::process::id(),
		NEXT.fetch_add(1, Ordering::Relaxed)
	));
	let result = (|| {
		let mut file = OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&temporary)?;
		file.write_all(bytes)?;
		drop(file);
		fs::rename(&temporary, path)
	})();
	let _ = fs::remove_file(temporary);
	result
}

/// Identity memoization uses inode and nanosecond ctime as well as mtime. Restoring
/// a binary's old mtime cannot reuse its old digest. Other platforms hash afresh.
#[cfg(unix)]
fn file_key(metadata: &fs::Metadata) -> Option<String> {
	use std::os::unix::fs::MetadataExt;
	Some(format!(
		"{}-{}-{}-{}-{}-{}-{}",
		metadata.dev(),
		metadata.ino(),
		metadata.len(),
		metadata.mtime(),
		metadata.mtime_nsec(),
		metadata.ctime(),
		metadata.ctime_nsec()
	))
}

#[cfg(not(unix))]
fn file_key(_metadata: &fs::Metadata) -> Option<String> {
	None
}

fn file_digest(path: &Path, memo: Option<&Path>) -> io::Result<[u8; 32]> {
	let mut file = File::open(path)?;
	let metadata = file.metadata()?;
	let key = file_key(&metadata);
	let memo = memo
		.zip(key.as_ref())
		.map(|(directory, key)| directory.join(key));
	if let Some(digest) = memo
		.as_ref()
		.and_then(|path| fs::read(path).ok())
		.and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
	{
		return Ok(digest);
	}
	let mut hash = Sha256::new();
	let mut buffer = [0; 64 * 1024];
	loop {
		let count = file.read(&mut buffer)?;
		if count == 0 {
			break;
		}
		hash.update(&buffer[..count]);
	}
	let digest = hash.finalize().into();
	if let Some(memo) = memo
		&& file_key(&file.metadata()?) == key
	{
		let _ = publish(&memo, &digest);
	}
	Ok(digest)
}

/// Resolve the LLVM library actually loaded, including LD_LIBRARY_PATH overrides.
#[cfg(unix)]
fn llvm_library() -> Option<PathBuf> {
	use std::ffi::{CStr, OsStr};
	use std::os::unix::ffi::OsStrExt;
	let mut information = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
	let symbol = llvm_sys::core::LLVMContextCreate as *const ();
	// SAFETY: dladdr writes the supplied Dl_info; the address is a live LLVM symbol.
	if unsafe { libc::dladdr(symbol.cast(), information.as_mut_ptr()) } == 0 {
		return None;
	}
	// SAFETY: successful dladdr initialized the structure and owns its filename.
	let information = unsafe { information.assume_init() };
	if information.dli_fname.is_null() {
		return None;
	}
	// SAFETY: dladdr returns a null-terminated filename valid while LLVM is loaded.
	let name = unsafe { CStr::from_ptr(information.dli_fname) };
	Some(PathBuf::from(OsStr::from_bytes(name.to_bytes())))
}

#[cfg(not(unix))]
fn llvm_library() -> Option<PathBuf> {
	None
}

/// Includes all Rust code, the embedded runtime/shim, build flags, dependencies,
/// and the actual LLVM shared library. A failed identity disables persistent reuse.
/// Linux opens the running executable, even if its pathname was replaced meanwhile.
pub(crate) fn compiler_key() -> Option<&'static [u8; 32]> {
	static KEY: OnceLock<Option<[u8; 32]>> = OnceLock::new();
	KEY.get_or_init(|| {
		let root = directory().or_else(|| {
			std::env::var_os("XDG_CACHE_HOME")
				.map(PathBuf::from)
				.or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
				.map(|root| root.join("fa/compiler-identities"))
		});
		let memo = root.map(|root| root.join("identities"));
		let executable = if cfg!(target_os = "linux") {
			PathBuf::from("/proc/self/exe")
		} else {
			std::env::current_exe().ok()?
		};
		let library = llvm_library()?;
		let mut hash = Sha256::new();
		hash.update(b"fa-compiler-identity-1");
		hash.update(file_digest(&executable, memo.as_deref()).ok()?);
		hash.update(file_digest(&library, memo.as_deref()).ok()?);
		Some(hash.finalize().into())
	})
	.as_ref()
}

fn module_key(
	input: &[u8],
	compiler: &[u8; 32],
	options: &Options,
	emulated_tls: bool,
) -> [u8; 32] {
	let mut hash = Sha256::new();
	hash.update(FORMAT);
	hash.update(compiler);
	hash.update(format!("{options:?}/{emulated_tls}"));
	hash.update(input);
	hash.finalize().into()
}

pub(crate) struct ModuleCache {
	path: PathBuf,
	name: String,
}

impl ModuleCache {
	pub fn new(module: &Module<'_>, options: &Options, emulated_tls: bool) -> Option<Self> {
		let root = directory()?;
		let compiler = compiler_key()?;
		let input = module.write_bitcode_to_memory();
		if input.as_slice().len() as u64 > MAX_MODULE_BYTES {
			return None;
		}
		let key = module_key(input.as_slice(), compiler, options, emulated_tls);
		Some(Self {
			path: root
				.join("modules")
				.join(hex(compiler))
				.join(format!("{}.bc", hex(&key))),
			name: module.get_name().to_str().ok()?.to_string(),
		})
	}

	pub fn load<'ctx>(&self, context: &'ctx Context) -> Option<Module<'ctx>> {
		let module = self.read(context);
		if std::env::var_os("FA_LLVM_CACHE_TRACE").is_some() {
			eprintln!(
				"FA_LLVM_CACHE\t{}\t{}",
				if module.is_some() { "hit" } else { "miss" },
				self.path.display()
			);
		}
		module
	}

	fn read<'ctx>(&self, context: &'ctx Context) -> Option<Module<'ctx>> {
		if fs::metadata(&self.path).ok()?.len() > MAX_MODULE_BYTES {
			return None;
		}
		let bytes = fs::read(&self.path).ok()?;
		let bytes = bytes.strip_prefix(FORMAT)?;
		let checksum = bytes.get(..32)?;
		let payload = bytes.get(32..)?;
		if &Sha256::digest(payload)[..] != checksum {
			return None;
		}
		let buffer = MemoryBuffer::create_from_memory_range_copy(payload, "cached module");
		let module = Module::parse_bitcode_from_buffer(&buffer, context).ok()?;
		module.verify().ok()?;
		module.set_name(&self.name);
		Some(module)
	}

	pub fn store(&self, module: &Module<'_>) {
		let buffer = module.write_bitcode_to_memory();
		if buffer.as_slice().len() as u64 + FORMAT.len() as u64 + 32 > MAX_MODULE_BYTES {
			return;
		}
		let mut bytes = Vec::with_capacity(FORMAT.len() + 32 + buffer.as_slice().len());
		bytes.extend_from_slice(FORMAT);
		bytes.extend_from_slice(&Sha256::digest(buffer.as_slice()));
		bytes.extend_from_slice(buffer.as_slice());
		let _ = publish(&self.path, &bytes);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::FileTimes;

	struct Scratch(PathBuf);
	impl Scratch {
		fn new() -> Self {
			static NEXT: AtomicU64 = AtomicU64::new(0);
			let path = std::env::temp_dir().join(format!(
				"fa-llvm-cache-{}-{}",
				std::process::id(),
				NEXT.fetch_add(1, Ordering::Relaxed)
			));
			fs::create_dir_all(&path).unwrap();
			Self(path)
		}
	}
	impl Drop for Scratch {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.0);
		}
	}

	#[test]
	fn compiler_ir_options_and_tls_have_distinct_keys() {
		let options = Options::default();
		let key = module_key(b"IR", &[1; 32], &options, false);
		assert_ne!(key, module_key(b"changed IR", &[1; 32], &options, false));
		assert_ne!(key, module_key(b"IR", &[2; 32], &options, false));
		assert_ne!(key, module_key(b"IR", &[1; 32], &options, true));
		for changed in [
			Options {
				opt_level: crate::OptLevel::O0,
				..options
			},
			Options {
				overflow_checks: !options.overflow_checks,
				..options
			},
			Options {
				leak_check: !options.leak_check,
				..options
			},
			Options {
				asynchronous: Some(true),
				..options
			},
			Options {
				size: true,
				..options
			},
			Options {
				emit_ir: true,
				..options
			},
		] {
			assert_ne!(key, module_key(b"IR", &[1; 32], &changed, false));
		}
	}

	#[test]
	fn cached_modules_round_trip_and_corruption_is_a_miss() {
		let scratch = Scratch::new();
		let context = Context::create();
		let module = context.create_module("cached");
		let structure = context.opaque_struct_type("Example");
		structure.set_body(&[context.i32_type().into()], false);
		module.add_function(
			"example",
			context.i32_type().fn_type(&[structure.into()], false),
			None,
		);
		let cache = ModuleCache {
			path: scratch.0.join("entry.bc"),
			name: "cached".to_string(),
		};
		assert!(cache.read(&context).is_none());
		cache.store(&module);
		let restored_context = Context::create();
		assert_eq!(
			cache.read(&restored_context).unwrap().print_to_string(),
			module.print_to_string()
		);
		let mut bytes = fs::read(&cache.path).unwrap();
		*bytes.last_mut().unwrap() ^= 1;
		fs::write(&cache.path, bytes).unwrap();
		assert!(
			cache.read(&context).is_none(),
			"corrupt payload must be recompiled"
		);
		fs::write(&cache.path, FORMAT).unwrap();
		assert!(
			cache.read(&context).is_none(),
			"truncated entry must be recompiled"
		);
	}

	#[test]
	fn concurrent_publication_is_complete_and_leaves_no_temporary_files() {
		let scratch = Scratch::new();
		let cache = ModuleCache {
			path: scratch.0.join("entry.bc"),
			name: "cached".to_string(),
		};
		std::thread::scope(|scope| {
			for _ in 0..8 {
				let cache = &cache;
				scope.spawn(move || {
					let context = Context::create();
					cache.store(&context.create_module("same module"));
				});
			}
		});
		assert!(cache.read(&Context::create()).is_some());
		assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
	}

	#[test]
	fn compiler_digest_changes_even_when_mtime_and_length_are_restored() {
		let scratch = Scratch::new();
		let compiler = scratch.0.join("compiler");
		let memo = scratch.0.join("identities");
		fs::write(&compiler, b"compiler a").unwrap();
		let modified = fs::metadata(&compiler).unwrap().modified().unwrap();
		let first = file_digest(&compiler, Some(&memo)).unwrap();
		assert_eq!(file_digest(&compiler, Some(&memo)).unwrap(), first);
		std::thread::sleep(std::time::Duration::from_millis(2));
		fs::write(&compiler, b"compiler b").unwrap();
		File::options()
			.write(true)
			.open(&compiler)
			.unwrap()
			.set_times(FileTimes::new().set_modified(modified))
			.unwrap();
		assert_eq!(
			fs::metadata(&compiler).unwrap().modified().unwrap(),
			modified
		);
		assert_ne!(file_digest(&compiler, Some(&memo)).unwrap(), first);
	}

	#[test]
	fn unwritable_cache_is_a_miss() {
		let scratch = Scratch::new();
		let blocker = scratch.0.join("not a directory");
		fs::write(&blocker, b"").unwrap();
		let cache = ModuleCache {
			path: blocker.join("entry.bc"),
			name: "uncached".to_string(),
		};
		let context = Context::create();
		cache.store(&context.create_module("uncached"));
		assert!(cache.read(&context).is_none());
	}
}
