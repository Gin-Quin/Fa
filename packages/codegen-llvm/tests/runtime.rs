//! The event loop of the C runtime under AddressSanitizer and
//! UndefinedBehaviorSanitizer (`runtime/test/executor_test.c`, which
//! includes `fa_runtime.c`): tasks, timers, wakes from other threads,
//! cancellation, mutexes, join nodes and the blocking pool. Skipped when the
//! C compiler (`$CC`, else `cc`) can't build with the sanitizers.

use std::path::Path;
use std::process::Command;

#[test]
fn the_event_loop_of_the_runtime_passes_its_tests_with_sanitizers() {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let temporary = Path::new(env!("CARGO_TARGET_TMPDIR"));
	std::fs::create_dir_all(temporary).expect("the test's temporary directory");
	let output = temporary.join("fa_executor_test");
	let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
	let flags = [
		"-std=gnu11",
		"-O1",
		"-g",
		"-fsanitize=address,undefined",
		"-fno-sanitize=nonnull-attribute",
		"-fno-sanitize-recover=undefined",
		"-fno-omit-frame-pointer",
		"-fno-builtin",
		"-Wno-builtin-declaration-mismatch",
		"-Wno-attributes",
		"-pthread",
	];
	// Probe the toolchain independently: a broken runtime test must fail,
	// rather than look like a machine without the sanitizers.
	let probe = temporary.join("fa_sanitizer_probe.c");
	std::fs::write(&probe, "int main(void) { return 0; }\n").expect("the sanitizer probe");
	let supported = Command::new(&compiler)
		.args(flags)
		.arg(&probe)
		.arg("-o")
		.arg(temporary.join("fa_sanitizer_probe"))
		.output();
	match supported {
		Ok(built) if built.status.success() => {}
		Ok(built) => {
			eprintln!(
				"skipped: `{compiler}` can't build with the sanitizers:\n{}",
				String::from_utf8_lossy(&built.stderr)
			);
			return;
		}
		Err(error) => {
			eprintln!("skipped: no C compiler `{compiler}`: {error}");
			return;
		}
	}
	let built = Command::new(&compiler)
		.args(flags)
		.arg(manifest.join("runtime/test/executor_test.c"))
		.arg("-o")
		.arg(&output)
		.output()
		.expect("the C compiler ran for the probe");
	assert!(
		built.status.success(),
		"the runtime test doesn't compile:\n{}",
		String::from_utf8_lossy(&built.stderr)
	);
	let run = Command::new(&output).output().expect("the test runs");
	let stderr = String::from_utf8_lossy(&run.stderr);
	if stderr.contains("ReserveShadowMemoryRange failed") {
		// AddressSanitizer reserves terabytes of address space: `ulimit -v`
		eprintln!("skipped: AddressSanitizer can't reserve its memory (ulimit -v?)");
		return;
	}
	assert!(
		run.status.success(),
		"{}\n{}",
		String::from_utf8_lossy(&run.stdout),
		stderr
	);
}
