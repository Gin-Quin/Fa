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
	let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa_executor_test");
	let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
	let built = Command::new(&compiler)
		.args([
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
		])
		.arg(manifest.join("runtime/test/executor_test.c"))
		.arg("-o")
		.arg(&output)
		.output();
	match built {
		Ok(built) if built.status.success() => {}
		Ok(built) => {
			eprintln!(
				"skipped: `{compiler}` can't build the test with the sanitizers:\n{}",
				String::from_utf8_lossy(&built.stderr)
			);
			return;
		}
		Err(error) => {
			eprintln!("skipped: no C compiler `{compiler}`: {error}");
			return;
		}
	}
	let run = Command::new(&output).output().expect("the test runs");
	assert!(
		run.status.success(),
		"{}\n{}",
		String::from_utf8_lossy(&run.stdout),
		String::from_utf8_lossy(&run.stderr)
	);
}
