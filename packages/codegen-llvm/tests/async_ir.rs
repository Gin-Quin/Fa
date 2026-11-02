//! The golden IR of async instances (`design/async.md` §3.1): the
//! coroutines the lowering builds for `tests/golden/*.fa`, before any pass,
//! compared with the `.ll` files next to them (`FA_UPDATE_GOLDEN=1`
//! rewrites them; review the diff). Only the async instances are compared:
//! the rest of the module is the runtime and the synchronous code.

#[cfg(not(fa_llvm))]
#[test]
fn async_ir_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::Path;

	use fa_codegen_llvm::{Options, Target, compile_file, lowered_ir};

	/// The definitions of the async instances of a module (`define … .async(…`).
	fn async_instances(ir: &str) -> String {
		let mut kept = String::new();
		let mut inside = false;
		for line in ir.lines() {
			if line.starts_with("define") && line.contains(".async\"(") {
				inside = true;
			}
			if inside {
				kept.push_str(line);
				kept.push('\n');
				if line == "}" {
					inside = false;
					kept.push('\n');
				}
			}
		}
		kept
	}

	#[test]
	fn the_async_instances_of_an_await_match_their_golden_ir() {
		let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
		let source = directory.join("await.fa");
		let ir = lowered_ir(&source, Target::Native, &Options::default()).expect("lowered");
		let actual = async_instances(&ir);
		assert!(
			actual.contains("@llvm.coro.suspend"),
			"no suspension point:\n{actual}"
		);
		let golden = directory.join("await.ll");
		if std::env::var_os("FA_UPDATE_GOLDEN").is_some() {
			std::fs::write(&golden, &actual).expect("written");
			return;
		}
		let expected = std::fs::read_to_string(&golden).unwrap_or_default();
		assert!(
			expected == actual,
			"the IR of {} changed (FA_UPDATE_GOLDEN=1 rewrites it):\n{actual}",
			source.display()
		);
	}

	/// Compiles a source at `O3` (the IR written next to the executable):
	/// the size of the executable and the IR.
	fn compiled(name: &str, source: &str, options: Options) -> (u64, String) {
		let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("async-ir");
		std::fs::create_dir_all(&directory).expect("a directory");
		let path = directory.join(format!("{name}.fa"));
		std::fs::write(&path, source).expect("written");
		let options = Options {
			emit_ir: true,
			..options
		};
		let artifact = compile_file(&path, Target::Native, &options, &directory.join(name))
			.unwrap_or_else(|error| panic!("{name}: {error}"));
		let ir = std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file");
		(artifact.output_size, ir)
	}

	/// The functions of the event loop and of tasks.
	const ASYNC_SYMBOLS: &[&str] = &[
		"fa_run_main",
		"fa_task_spawn",
		"fa_task_drop",
		"fa_resume",
		"fa_destroy",
		"fa_coop_budget",
		"llvm.coro",
		".async",
	];

	#[test]
	fn a_program_in_synchronous_mode_has_no_async_code() {
		let plain = "function twice = (x: Integer): Integer => x * 2\nconsole.log(twice(21))\n";
		let blocking =
			"async function twice = (x: Integer): Integer => x * 2\nconsole.log(twice(21))\n";
		let (plain_size, plain_ir) = compiled("plain", plain, Options::default());
		let (blocking_size, blocking_ir) = compiled("blocking", blocking, Options::default());
		for symbol in ASYNC_SYMBOLS {
			assert!(
				!plain_ir.contains(symbol),
				"`{symbol}` in a synchronous program"
			);
			assert!(
				!blocking_ir.contains(symbol),
				"`{symbol}` in a blocking call"
			);
		}
		assert_eq!(
			plain_size, blocking_size,
			"an async function called blocking costs nothing"
		);
		// forced off: `start` runs at once, without the event loop
		let started = "async function twice = (x: Integer): Integer => x * 2\nlet task = start twice(21)\nconsole.log(await task)\n";
		let options = Options {
			asynchronous: Some(false),
			..Options::default()
		};
		let (_, started_ir) = compiled("started", started, options);
		for symbol in [
			"fa_run_main",
			"fa_task_spawn",
			"fa_task_drop",
			"fa_resume",
			"llvm.coro",
			".async",
		] {
			assert!(
				!started_ir.contains(symbol),
				"`{symbol}` in synchronous mode"
			);
		}
		// the same program in asynchronous mode has its event loop
		let (_, async_ir) = compiled("started-async", started, Options::default());
		assert!(async_ir.contains("fa_run_main"));
	}
}
