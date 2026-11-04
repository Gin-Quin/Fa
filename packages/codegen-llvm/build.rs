//! Links `libLLVM` 18 when `LLVM_SYS_181_PREFIX` points at an LLVM 18 install
//! (see `scripts/setup-llvm.sh`), and enables the `fa_llvm` cfg.
//!
//! - `FA_LLVM_LINK=dynamic` (default): links `libLLVM-18.so`, with an rpath
//!   to its directory so that tests and examples run without `LD_LIBRARY_PATH`.
//! - `FA_LLVM_LINK=static`: links the static component libraries; it needs the
//!   development symlinks of zlib, zstd, libxml2 and tinfo (`LIBRARY_PATH`).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const PREFIX_VARIABLE: &str = "LLVM_SYS_181_PREFIX";

fn main() {
	println!("cargo::rustc-check-cfg=cfg(fa_llvm)");
	println!("cargo::rerun-if-env-changed={PREFIX_VARIABLE}");
	println!("cargo::rerun-if-env-changed=FA_LLVM_LINK");

	let Some(prefix) = env::var_os(PREFIX_VARIABLE).map(PathBuf::from) else {
		println!(
			"cargo::warning=fa-codegen-llvm: {PREFIX_VARIABLE} is not set, the LLVM backend is disabled (run scripts/setup-llvm.sh)"
		);
		return;
	};
	let llvm_config = prefix.join("bin/llvm-config");
	let version = llvm_config_output(&llvm_config, &["--version"]);
	if !version.starts_with("18.") {
		panic!("{PREFIX_VARIABLE} points at LLVM {version}, but LLVM 18 is required");
	}

	let library_directory = llvm_config_output(&llvm_config, &["--libdir"]);
	println!("cargo::rustc-link-search=native={library_directory}");
	build_shim(&llvm_config);
	match env::var("FA_LLVM_LINK").as_deref() {
		Ok("static") => {
			let components = [
				"core",
				"passes",
				"target",
				"executionengine",
				"x86",
				"aarch64",
				"webassembly",
			];
			let mut arguments = vec!["--link-static", "--libs"];
			arguments.extend(components);
			for library in llvm_config_output(&llvm_config, &arguments).split_whitespace() {
				println!(
					"cargo::rustc-link-lib=static={}",
					library.trim_start_matches("-l")
				);
			}
			let system = llvm_config_output(&llvm_config, &["--link-static", "--system-libs"]);
			for library in system.split_whitespace() {
				println!(
					"cargo::rustc-link-lib=dylib={}",
					library.trim_start_matches("-l")
				);
			}
			println!("cargo::rustc-link-lib=dylib=stdc++");
		}
		Ok("dynamic") | Err(_) => {
			let libraries = llvm_config_output(&llvm_config, &["--link-shared", "--libs"]);
			for library in libraries.split_whitespace() {
				println!(
					"cargo::rustc-link-lib=dylib={}",
					library.trim_start_matches("-l")
				);
			}
			println!("cargo::rustc-link-arg=-Wl,-rpath,{library_directory}");
		}
		Ok(other) => panic!("FA_LLVM_LINK must be `dynamic` or `static`, not `{other}`"),
	}

	println!("cargo::rustc-cfg=fa_llvm");
	println!("cargo::rustc-env=FA_LLVM_PREFIX={}", prefix.display());
}

/// Compiles the C++ part of the backend (`shim/`: what the C API of LLVM
/// lacks, the ThinLTO bitcode writer) into a static library, linked before
/// LLVM.
fn build_shim(llvm_config: &Path) {
	println!("cargo::rerun-if-changed=shim/thin_bitcode.cpp");
	println!("cargo::rerun-if-env-changed=CXX");
	let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
	let object = out.join("thin_bitcode.o");
	let compiler = env::var_os("CXX").unwrap_or_else(|| "c++".into());
	let flags = llvm_config_output(llvm_config, &["--cxxflags"]);
	let status = Command::new(&compiler)
		.args(["-c", "-O2", "-fPIC", "-w"])
		.args(flags.split_whitespace())
		.arg("shim/thin_bitcode.cpp")
		.arg("-o")
		.arg(&object)
		.status()
		.unwrap_or_else(|error| panic!("cannot run the C++ compiler {compiler:?}: {error}"));
	assert!(status.success(), "compiling shim/thin_bitcode.cpp failed");
	let library = out.join("libfa_llvm_shim.a");
	let _ = std::fs::remove_file(&library);
	let status = Command::new("ar")
		.arg("crs")
		.arg(&library)
		.arg(&object)
		.status()
		.unwrap_or_else(|error| panic!("cannot run ar: {error}"));
	assert!(status.success(), "archiving the shim failed");
	println!("cargo::rustc-link-search=native={}", out.display());
	println!("cargo::rustc-link-lib=static=fa_llvm_shim");
	println!("cargo::rustc-link-lib=dylib=stdc++");
}

fn llvm_config_output(llvm_config: &Path, arguments: &[&str]) -> String {
	let output = Command::new(llvm_config)
		.args(arguments)
		.output()
		.unwrap_or_else(|error| panic!("cannot run {}: {error}", llvm_config.display()));
	if !output.status.success() {
		panic!(
			"{} {arguments:?} failed: {}",
			llvm_config.display(),
			String::from_utf8_lossy(&output.stderr)
		);
	}
	String::from_utf8(output.stdout)
		.expect("llvm-config prints UTF-8")
		.trim()
		.to_string()
}
