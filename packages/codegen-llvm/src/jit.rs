//! Programs loaded in memory (design/decisions.md, "Interpreter scope and
//! JIT for native tests"): `fa run --target=native` and
//! `fa test --target=native` compile the module to an object in memory and
//! hand it to LLVM's JIT (ORC's `LLJIT`), which links it in this process
//! against the C library the compiler itself uses. It is the lowering, the
//! passes and the code generator of a native build, without the object file
//! and without the linker.
//!
//! - **Thread-local variables** are emulated (`fa_use_emulated_tls`): a JIT
//!   has no thread-local storage segment, so each access calls
//!   `__emutls_get_address` (libgcc or compiler-rt, already in the process).
//! - **`exit`** is [`exit_now`]: the program ends the process without the
//!   exit handlers of the compiler.
//! - **Tests** run in a child each ([`LoadedProgram::run_in_child`], `fork`):
//!   fresh module-level values, and a stop ends only its test.
//!
//! A program that links C libraries, another triple than the host's and
//! `FA_JIT=0` keep the linked executable ([`crate::jit_available`]), and so
//! does a program the JIT can't load ([`Error::Jit`]). `FA_JIT_TRACE=1`
//! prints which one runs.

use std::ffi::{CString, OsStr, c_char, c_int, c_void};
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::ptr;
use std::time::{Duration, Instant};

use fa_analyzer::session::Session;
use inkwell::targets::FileType;
use llvm_sys::error::{LLVMDisposeErrorMessage, LLVMErrorRef, LLVMGetErrorMessage};
use llvm_sys::orc2::lljit::{
	LLVMOrcCreateLLJIT, LLVMOrcDisposeLLJIT, LLVMOrcLLJITAddObjectFile,
	LLVMOrcLLJITGetExecutionSession, LLVMOrcLLJITGetGlobalPrefix, LLVMOrcLLJITGetMainJITDylib,
	LLVMOrcLLJITLookup, LLVMOrcLLJITMangleAndIntern, LLVMOrcLLJITRef,
};
use llvm_sys::orc2::{
	LLVMJITEvaluatedSymbol, LLVMJITSymbolFlags, LLVMJITSymbolGenericFlags, LLVMOrcAbsoluteSymbols,
	LLVMOrcCSymbolMapPair, LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess,
	LLVMOrcDisposeMaterializationUnit, LLVMOrcExecutionSessionSetErrorReporter,
	LLVMOrcJITDylibAddGenerator, LLVMOrcJITDylibDefine,
};

use crate::artifact::Timings;
use crate::compile::{Inputs, prepare, with_module};
use crate::error::Error;
use crate::options::Libraries;
use crate::target::{Options, Target};
use crate::{ChildOutput, LoadedProgram, Source};

/// `FA_JIT_TRACE=1` prints what the JIT loads on the standard error.
fn trace() -> bool {
	std::env::var_os("FA_JIT_TRACE").is_some()
}

/// A program loaded in this process: its `main`.
struct JitProgram {
	jit: LLVMOrcLLJITRef,
	main: unsafe extern "C" fn(c_int, *const *const c_char) -> c_int,
	/// The time of each phase (`link` is the JIT's link in memory).
	timings: Timings,
}

impl std::fmt::Debug for JitProgram {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_struct("JitProgram")
			.field("timings", &self.timings)
			.finish_non_exhaustive()
	}
}

/// Compiles an analyzed program (which must have no error diagnostics) and
/// loads it in this process. [`Error::Jit`] when the JIT can't load the
/// object: the caller links an executable instead.
pub fn load(
	session: &Session,
	source: Source<'_>,
	options: &Options,
) -> Result<Box<dyn LoadedProgram>, Error> {
	let prepared = prepare(session, &source, options)?;
	let libraries = Libraries::default();
	let inputs = Inputs {
		session,
		files: &prepared.files,
		entry: prepared.entry,
		init_order: prepared.init_order,
		tests: prepared.tests,
		target: Target::Native,
		options: &prepared.options,
		libraries: &libraries,
		emulated_tls: true,
	};
	with_module(inputs, |built| {
		let mut timings = built.timings;
		let start = Instant::now();
		let object = built
			.machine
			.write_to_memory_buffer(built.module, FileType::Object)?;
		timings.emit = start.elapsed();
		let start = Instant::now();
		let (jit, main) = link_in_memory(vec![object])?;
		timings.link = start.elapsed();
		Ok(Box::new(JitProgram { jit, main, timings }) as Box<dyn LoadedProgram>)
	})
}

/// Loads the objects of a program compiled per unit (`units.rs`: the
/// units, the entry and the runtime, compiled with emulated thread-local
/// variables) in this process. The files are copied: the cache may replace
/// them while the program runs.
pub(crate) fn load_objects(
	objects: &[std::path::PathBuf],
	mut timings: Timings,
) -> Result<Box<dyn LoadedProgram>, Error> {
	let start = Instant::now();
	let mut buffers = Vec::with_capacity(objects.len());
	for object in objects {
		let bytes = std::fs::read(object)?;
		buffers.push(
			inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
				&bytes,
				&object.display().to_string(),
			),
		);
	}
	let (jit, main) = link_in_memory(buffers)?;
	timings.link = start.elapsed();
	Ok(Box::new(JitProgram { jit, main, timings }))
}

/// The message of an error of ORC, which is consumed.
fn message(error: LLVMErrorRef) -> String {
	// SAFETY: `error` is a failure value that nothing else owns
	unsafe {
		let text = LLVMGetErrorMessage(error);
		let message = std::ffi::CStr::from_ptr(text)
			.to_string_lossy()
			.into_owned();
		LLVMDisposeErrorMessage(text);
		message
	}
}

fn check(error: LLVMErrorRef) -> Result<(), Error> {
	if error.is_null() {
		Ok(())
	} else {
		Err(Error::Jit(message(error)))
	}
}

/// The errors ORC reports while it links: printed with `FA_JIT_TRACE=1`.
extern "C" fn report_error(_context: *mut c_void, error: LLVMErrorRef) {
	let message = message(error);
	if trace() {
		eprintln!("jit: {message}");
	}
}

/// Ends the process as `exit` does for the program's own output, without
/// the exit handlers and the static destructors of the compiler (which a
/// forked child must not run, and which the program doesn't need).
extern "C" fn exit_now(code: c_int) -> ! {
	// SAFETY: flushes the C streams, then ends the process
	unsafe {
		libc::fflush(ptr::null_mut());
		libc::_exit(code)
	}
}

/// Links objects in this process and finds their `main`.
fn link_in_memory(
	objects: Vec<inkwell::memory_buffer::MemoryBuffer>,
) -> Result<
	(
		LLVMOrcLLJITRef,
		unsafe extern "C" fn(c_int, *const *const c_char) -> c_int,
	),
	Error,
> {
	let start = Instant::now();
	let count = objects.len();
	// SAFETY: the calls follow the ownership rules of the C API of ORC: the
	// JIT owns the generator, the absolute symbols and the object once they
	// are added, and is disposed of on every failure.
	unsafe {
		let mut jit = ptr::null_mut();
		check(LLVMOrcCreateLLJIT(&mut jit, ptr::null_mut()))?;
		let loaded = (|| {
			// a failed link is reported by the lookup below: ORC must not
			// print it too (a program it can't load is linked instead)
			LLVMOrcExecutionSessionSetErrorReporter(
				LLVMOrcLLJITGetExecutionSession(jit),
				report_error,
				ptr::null_mut(),
			);
			let dylib = LLVMOrcLLJITGetMainJITDylib(jit);
			// `exit` of the program, before the process's own
			let name = LLVMOrcLLJITMangleAndIntern(jit, c"exit".as_ptr());
			let flags = LLVMJITSymbolGenericFlags::LLVMJITSymbolGenericFlagsExported as u8
				| LLVMJITSymbolGenericFlags::LLVMJITSymbolGenericFlagsCallable as u8;
			let mut pair = LLVMOrcCSymbolMapPair {
				Name: name,
				Sym: LLVMJITEvaluatedSymbol {
					Address: exit_now as *const () as usize as u64,
					Flags: LLVMJITSymbolFlags {
						GenericFlags: flags,
						TargetFlags: 0,
					},
				},
			};
			let symbols = LLVMOrcAbsoluteSymbols(&mut pair, 1);
			if let Err(error) = check(LLVMOrcJITDylibDefine(dylib, symbols)) {
				// not taken by the JIT when it fails
				LLVMOrcDisposeMaterializationUnit(symbols);
				return Err(error);
			}
			// the C library and libgcc of this process
			let mut generator = ptr::null_mut();
			check(LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess(
				&mut generator,
				LLVMOrcLLJITGetGlobalPrefix(jit),
				None,
				ptr::null_mut(),
			))?;
			LLVMOrcJITDylibAddGenerator(dylib, generator);
			for object in objects {
				let buffer = object.as_mut_ptr();
				// the JIT owns the buffer from here, even when it fails
				std::mem::forget(object);
				check(LLVMOrcLLJITAddObjectFile(jit, dylib, buffer))?;
			}
			let mut address = 0;
			check(LLVMOrcLLJITLookup(jit, &mut address, c"main".as_ptr()))?;
			Ok(std::mem::transmute::<
				usize,
				unsafe extern "C" fn(c_int, *const *const c_char) -> c_int,
			>(address as usize))
		})();
		match loaded {
			Ok(main) => {
				// `FA_JIT_TRACE=1`: says that the program is loaded, not linked
				if trace() {
					eprintln!(
						"jit: loaded {count} object{} in {:.1} ms",
						if count == 1 { "" } else { "s" },
						start.elapsed().as_secs_f64() * 1000.0
					);
				}
				Ok((jit, main))
			}
			Err(error) => {
				if trace() {
					eprintln!("jit: the program is linked instead: {error}");
				}
				let disposed = LLVMOrcDisposeLLJIT(jit);
				if !disposed.is_null() {
					message(disposed);
				}
				Err(error)
			}
		}
	}
}

/// `argv`: the name of the program, then its arguments (bytes after a NUL
/// are dropped, as the C library would).
struct Arguments {
	_strings: Vec<CString>,
	pointers: Vec<*const c_char>,
}

impl Arguments {
	fn new(name: &str, arguments: &[&OsStr]) -> Self {
		let text = |bytes: &[u8]| {
			let end = bytes
				.iter()
				.position(|&byte| byte == 0)
				.unwrap_or(bytes.len());
			CString::new(&bytes[..end]).expect("no NUL before the end")
		};
		let strings: Vec<CString> = std::iter::once(text(name.as_bytes()))
			.chain(arguments.iter().map(|argument| text(argument.as_bytes())))
			.collect();
		let mut pointers: Vec<*const c_char> =
			strings.iter().map(|string| string.as_ptr()).collect();
		pointers.push(ptr::null());
		Arguments {
			_strings: strings,
			pointers,
		}
	}

	fn count(&self) -> c_int {
		(self.pointers.len() - 1) as c_int
	}
}

impl JitProgram {
	fn enter(&self, arguments: &Arguments) -> i32 {
		// SAFETY: `main` is the entry of the loaded program, and `argv` is
		// alive for the call
		unsafe {
			libc::signal(libc::SIGPIPE, libc::SIG_DFL);
			(self.main)(arguments.count(), arguments.pointers.as_ptr())
		}
	}
}

impl LoadedProgram for JitProgram {
	fn run(&self, name: &str, arguments: &[&OsStr]) -> i32 {
		let arguments = Arguments::new(name, arguments);
		let _ = std::io::stdout().flush();
		let _ = std::io::stderr().flush();
		self.enter(&arguments)
	}

	fn run_in_child(
		&self,
		name: &str,
		arguments: &[&OsStr],
		timeout: Duration,
	) -> std::io::Result<Option<ChildOutput>> {
		// everything the child needs is allocated before the fork
		let arguments = Arguments::new(name, arguments);
		let _ = std::io::stdout().flush();
		let _ = std::io::stderr().flush();
		let mut pipe = [0 as c_int; 2];
		// SAFETY: plain system calls; the child only calls async-signal-safe
		// functions before entering the program
		unsafe {
			if libc::pipe(pipe.as_mut_ptr()) != 0 {
				return Err(std::io::Error::last_os_error());
			}
			let [read, write] = pipe;
			let child = libc::fork();
			if child < 0 {
				let error = std::io::Error::last_os_error();
				libc::close(read);
				libc::close(write);
				return Err(error);
			}
			if child == 0 {
				libc::close(read);
				libc::dup2(write, 2);
				libc::close(write);
				let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
				if null >= 0 {
					libc::dup2(null, 0);
					libc::close(null);
				}
				exit_now(self.enter(&arguments));
			}
			libc::close(write);
			let output = wait_for_child(child, read, timeout);
			libc::close(read);
			output
		}
	}
}

/// Reads the standard error of a child until it closes it, then waits for
/// its end; kills it at the deadline.
///
/// # Safety
/// `child` is a child of this process that nothing else waits for, and
/// `stderr` the read end of its pipe.
unsafe fn wait_for_child(
	child: libc::pid_t,
	stderr: c_int,
	timeout: Duration,
) -> std::io::Result<Option<ChildOutput>> {
	let deadline = Instant::now() + timeout;
	let kill = || {
		// SAFETY: the child is ours and not waited for yet
		unsafe {
			libc::kill(child, libc::SIGKILL);
			let mut status = 0;
			libc::waitpid(child, &mut status, 0);
		}
	};
	let mut written = Vec::new();
	let mut chunk = [0u8; 4096];
	loop {
		let remaining = deadline.saturating_duration_since(Instant::now());
		if remaining.is_zero() {
			kill();
			return Ok(None);
		}
		let mut descriptor = libc::pollfd {
			fd: stderr,
			events: libc::POLLIN,
			revents: 0,
		};
		let milliseconds = remaining.as_millis().clamp(1, c_int::MAX as u128) as c_int;
		// SAFETY: one valid descriptor
		let ready = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
		if ready < 0 {
			let error = std::io::Error::last_os_error();
			if error.kind() == std::io::ErrorKind::Interrupted {
				continue;
			}
			kill();
			return Err(error);
		}
		if ready == 0 {
			continue;
		}
		// SAFETY: `chunk` is writable for its length
		let count = unsafe { libc::read(stderr, chunk.as_mut_ptr() as *mut c_void, chunk.len()) };
		if count < 0 {
			let error = std::io::Error::last_os_error();
			if error.kind() == std::io::ErrorKind::Interrupted {
				continue;
			}
			kill();
			return Err(error);
		}
		if count == 0 {
			break;
		}
		written.extend_from_slice(&chunk[..count as usize]);
	}
	// the child closed its standard error: it is ending, or it closed it
	// and still runs until the deadline
	let mut pause = Duration::from_micros(50);
	loop {
		let mut status = 0;
		// SAFETY: the child is ours and not waited for yet
		let ended = unsafe { libc::waitpid(child, &mut status, libc::WNOHANG) };
		if ended < 0 {
			return Err(std::io::Error::last_os_error());
		}
		if ended == child {
			let code = if libc::WIFEXITED(status) {
				libc::WEXITSTATUS(status)
			} else {
				128 + libc::WTERMSIG(status)
			};
			return Ok(Some(ChildOutput {
				code,
				stderr: written,
			}));
		}
		if Instant::now() >= deadline {
			kill();
			return Ok(None);
		}
		std::thread::sleep(pause);
		pause = (pause * 2).min(Duration::from_millis(5));
	}
}

impl Drop for JitProgram {
	fn drop(&mut self) {
		// SAFETY: the JIT is alive and no code of the program runs anymore
		let error = unsafe { LLVMOrcDisposeLLJIT(self.jit) };
		if !error.is_null() {
			message(error);
		}
	}
}
