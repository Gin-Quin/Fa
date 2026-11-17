//! End-to-end tests: Fa programs run through `fa run` on Bun, and through
//! `fa build` on Node and in a web-global sandbox (see `programs/targets.rs`).
//! All targets share annotations, filtering, comparisons and bounded workers.
//!
//! Two sets of programs:
//!
//! - `tests/programs/*.fa` (this crate): the programs of the JavaScript
//!   backend;
//! - the cases of the analyzer (`packages/analyzer/tests/cases/**/*.fa`) that
//!   check without errors and say what they print, so that every brief test
//!   with an expected output runs once the analyzer and the backend support it.
//!
//! Annotations are Fa comments, on their own line or after code:
//!
//! ```fa
//! console.log(1 + 2)        -- prints: 3
//! -- prints: a second line
//! -- prints: "quoted"       (the brief's notation of a printed string: `quoted`)
//! -- prints: "line" ×3      (the line, three times; `<TAB>` is a tab)
//! -- exit: 101              (the exit code; 0 by default)
//! -- stderr: integer overflow    (a text that stderr contains)
//! -- args: first second      (the program's arguments; or `-- arguments:`)
//! -- overflow-checks: false  (`fa run --overflow-checks=false`)
//! -- skip-target: native wasm (also accepts `bun`, `node` and `web`)
//! -- asynchronous: false     (`fa run --asynchronous=false`)
//! ```
//!
//! The async programs of the LLVM backend
//! (`packages/codegen-llvm/tests/programs/async-*.fa`) also run here: the
//! same output on Bun as natively (design/async.md, step J1), except where
//! A1 lets JavaScript interleave tasks earlier ([`JAVASCRIPT_ORDER`]: the
//! same lines, in another order), and the programs marked `-- pending:` (a
//! construct the JavaScript backend lacks yet: their failures don't count).
//!
//! The same annotations as the LLVM backend's end-to-end tests
//! (`packages/codegen-llvm/tests/programs.rs`), which also run these
//! programs on `native` and `wasm`.
//!
//! Bun is found on the `PATH` or in `~/.bun/bin`; without it, the tests
//! print a message and pass. `FA_PROGRAMS=filter` runs only the programs
//! whose path contains `filter`; `FA_VERBOSE=1` lists every result.
//! Node is found at `~/.local/bin/node` or on PATH; without it only Node/web
//! tests are skipped. Their exact failure ratchets are `expected-skips/*.txt`:
//! an unlisted failure, listed success or stale entry fails. `FA_UPDATE_SKIPS=1`
//! refreshes them after a complete run (never a filtered run); review the diff.

#[path = "programs/targets.rs"]
mod targets;

#[path = "programs/string_differential.rs"]
mod string_differential;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// How many programs a test runs at the same time.
const WORKERS: usize = 3;

/// How many programs run at the same time, all tests together (they run in
/// parallel): their memory adds up.
const RUNNING_PROGRAMS: usize = 3;

/// The number of programs running, and its change.
static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

/// Runs `f` once fewer than [`RUNNING_PROGRAMS`] others are running.
fn limited<R>(f: impl FnOnce() -> R) -> R {
	let (running, changed) = &RUNNING;
	let mut count = running.lock().unwrap_or_else(|poison| poison.into_inner());
	while *count >= RUNNING_PROGRAMS {
		count = changed
			.wait(count)
			.unwrap_or_else(|poison| poison.into_inner());
	}
	*count += 1;
	drop(count);
	struct Permit;
	impl Drop for Permit {
		fn drop(&mut self) {
			*RUNNING
				.0
				.lock()
				.unwrap_or_else(|poison| poison.into_inner()) -= 1;
			RUNNING.1.notify_one();
		}
	}
	let _permit = Permit;
	f()
}

/// A filesystem fixture may write a fixed external path (async-files.fa).
/// Keep its executions on different targets apart; pure fixtures run freely.
static ACTIVE_PATHS: (Mutex<std::collections::BTreeSet<PathBuf>>, Condvar) = (
	Mutex::new(std::collections::BTreeSet::new()),
	Condvar::new(),
);

struct ProgramPermit(PathBuf);
impl ProgramPermit {
	fn acquire(path: &Path) -> Option<Self> {
		if !fs::read_to_string(path)
			.expect("a readable program")
			.contains("std.fs")
		{
			return None;
		}
		let path = path.canonicalize().expect("a program exists");
		let (active, changed) = &ACTIVE_PATHS;
		let mut paths = active.lock().unwrap_or_else(|poison| poison.into_inner());
		while paths.contains(&path) {
			paths = changed
				.wait(paths)
				.unwrap_or_else(|poison| poison.into_inner());
		}
		paths.insert(path.clone());
		Some(Self(path))
	}
}
impl Drop for ProgramPermit {
	fn drop(&mut self) {
		ACTIVE_PATHS
			.0
			.lock()
			.unwrap_or_else(|poison| poison.into_inner())
			.remove(&self.0);
		ACTIVE_PATHS.1.notify_all();
	}
}

/// The longest a program may run.
const TIMEOUT: Duration = Duration::from_secs(20);

/// What a program is expected to do.
#[derive(Debug, Default, Clone)]
struct Expected {
	stdout: Vec<String>,
	exit: i32,
	stderr: Vec<String>,
	args: Vec<String>,
	/// Options of `fa run`, given before the file (`-- overflow-checks:`).
	options: Vec<String>,
	/// `-- skip-target: bun node web …`: omit the named targets.
	skip_targets: Vec<String>,
	/// Empty means Bun; other targets build and execute through `targets`.
	target: &'static str,
	/// Run as `fa build --minify=true` then `bun main.js`, instead of `fa run`.
	minified: bool,
	/// Whether the file says anything about its output.
	annotated: bool,
	/// `-- pending: reason`: a construct of the program isn't supported yet
	/// (by the analyzer or the backend); its run isn't a failure.
	pending: Option<String>,
}

/// The start of the comment of a line (`--` outside a string).
fn find_comment(line: &str) -> Option<usize> {
	let bytes = line.as_bytes();
	let mut in_string = false;
	let mut index = 0;
	while index + 1 < bytes.len() {
		match bytes[index] {
			b'\\' if in_string => index += 1,
			b'"' => in_string = !in_string,
			b'-' if !in_string && bytes[index + 1] == b'-' => return Some(index),
			_ => {}
		}
		index += 1;
	}
	None
}

/// The annotations of a program.
fn expectations(text: &str) -> Expected {
	let mut expected = Expected::default();
	for line in text.lines() {
		let Some(start) = find_comment(line) else {
			continue;
		};
		let comment = line[start + 2..].trim_start();
		if let Some(printed) = comment.strip_prefix("prints:") {
			let printed = printed.strip_prefix(' ').unwrap_or(printed).trim_end();
			printed_lines(&printed.replace("<TAB>", "\t"), &mut expected.stdout);
			expected.annotated = true;
		} else if let Some(code) = comment.strip_prefix("exit:") {
			expected.exit = code.trim().parse().unwrap_or(-1);
			expected.annotated = true;
		} else if let Some(text) = comment.strip_prefix("stderr:") {
			expected.stderr.push(text.trim().to_string());
		} else if let Some(args) = comment
			.strip_prefix("args:")
			.or_else(|| comment.strip_prefix("arguments:"))
		{
			expected.args = args.split_whitespace().map(str::to_string).collect();
		} else if let Some(checks) = comment.strip_prefix("overflow-checks:") {
			expected
				.options
				.push(format!("--overflow-checks={}", checks.trim()));
		} else if let Some(targets) = comment.strip_prefix("skip-target:") {
			expected
				.skip_targets
				.extend(targets.split_whitespace().map(str::to_string));
		} else if let Some(mode) = comment.strip_prefix("asynchronous:") {
			expected
				.options
				.push(format!("--asynchronous={}", mode.trim()));
		} else if let Some(reason) = comment.strip_prefix("pending:") {
			expected.pending = Some(reason.trim().to_string());
		}
	}
	expected
}

/// The lines of a `prints:` annotation: `"text"` is `text`, and
/// `"text" ×3` is three lines `text` (the notation of the briefs).
fn printed_lines(printed: &str, lines: &mut Vec<String>) {
	let repeated = printed.rsplit_once(" ×").and_then(|(line, count)| {
		let count: usize = count.parse().ok()?;
		let line = line.strip_prefix('"')?.strip_suffix('"')?;
		Some((line, count))
	});
	if let Some((line, count)) = repeated {
		lines.extend(std::iter::repeat_n(line.to_string(), count));
		return;
	}
	match printed
		.strip_prefix('"')
		.and_then(|line| line.strip_suffix('"'))
	{
		Some(line) if !line.contains('"') => lines.push(line.to_string()),
		_ => lines.push(printed.to_string()),
	}
}

/// What a run gave.
#[derive(Debug)]
struct Outcome {
	build_failed: bool,
	stdout: String,
	stderr: String,
	exit: i32,
}

/// Runs `fa run <options> <program> <args>` (or the minified build of the
/// program) with a timeout.
fn run(program: &Path, expected: &Expected, scratch: &Path, index: usize) -> Outcome {
	let _program_permit = ProgramPermit::acquire(program);
	let files = scratch.join(format!("{index}-files"));
	let entry = multi_file_entry(program, &files);
	let program = entry.as_deref().unwrap_or(program);
	let mut command = if expected.minified || !expected.target.is_empty() {
		let directory = scratch.join(format!("{index}-build"));
		let target = if expected.target.is_empty() {
			"bun"
		} else {
			expected.target
		};
		let mut build = Command::new(env!("CARGO_BIN_EXE_fa"));
		build
			.args([
				"build",
				&format!("--target={target}"),
				"--overflow-checks=true",
			])
			.args(&expected.options)
			.arg(format!("--output={}", directory.display()))
			.arg(program);
		build.arg(format!("--minify={}", expected.minified));
		let built = execute(&mut build, scratch, &format!("{index}-build"));
		if built.exit != 0 {
			return Outcome {
				build_failed: true,
				stdout: String::new(),
				stderr: built.stderr,
				exit: built.exit,
			};
		}
		let entry = directory.join(target).join("main.js");
		if target == "bun" {
			let mut command = Command::new(fa::find_bun().expect("Bun"));
			command.arg(entry);
			command
		} else {
			targets::command(target, &entry)
		}
	} else {
		let mut command = Command::new(env!("CARGO_BIN_EXE_fa"));
		command.arg("run").args(&expected.options).arg(program);
		command
	};

	command.args(&expected.args);
	execute(&mut command, scratch, &index.to_string())
}

/// File-backed output avoids pipe deadlocks; builds and runs share the timeout
/// and process-group cleanup. Cargo provides the one already-built `fa` binary.
fn execute(command: &mut Command, scratch: &Path, name: &str) -> Outcome {
	let stdout_path = scratch.join(format!("{name}.stdout"));
	let stderr_path = scratch.join(format!("{name}.stderr"));
	#[cfg(unix)]
	{
		use std::os::unix::process::CommandExt;
		command.process_group(0);
	}
	let started = Instant::now();
	let mut child = command
		.env("XDG_CACHE_HOME", scratch.join("cache"))
		.stdin(Stdio::null())
		.stdout(fs::File::create(&stdout_path).unwrap())
		.stderr(fs::File::create(&stderr_path).unwrap())
		.spawn()
		.expect("`fa` runs");
	let exit = wait_for_exit(&mut child, TIMEOUT);
	if std::env::var_os("FA_TEST_TIMINGS").is_some() {
		eprintln!(
			"FA_TEST_PROCESS\t{name}\t{:.6}\t{command:?}",
			started.elapsed().as_secs_f64()
		);
	}
	let mut stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
	match exit {
		-2 => stderr.push_str("\nharness: timed out after 20 seconds\n"),
		-4 => stderr.push_str("\nharness: resident memory exceeded 512 MiB\n"),
		_ => {}
	}
	Outcome {
		build_failed: false,
		stdout: fs::read_to_string(&stdout_path).unwrap_or_default(),
		stderr,
		exit,
	}
}

#[cfg(unix)]
unsafe extern "C" {
	fn kill(pid: std::ffi::c_int, signal: std::ffi::c_int) -> std::ffi::c_int;
}

/// A timed-out `fa run` owns a Bun (or native) child: terminate its whole
/// process group before waiting for the facade. Killing only `fa` left Bun
/// running forever, with inherited files and the full-suite lock still open.
fn wait_for_exit(child: &mut Child, timeout: Duration) -> i32 {
	let started = Instant::now();
	loop {
		if let Some(status) = child.try_wait().unwrap() {
			return status.code().unwrap_or(-1);
		}
		// Bound resident memory, not virtual address space: JS engines and
		// mimalloc reserve large sparse arenas. The machine is shared.
		let over_memory_limit = resident_kib(child.id()).is_some_and(|rss| rss > 512 * 1024);
		if started.elapsed() > timeout || over_memory_limit {
			#[cfg(unix)]
			{
				let group = i32::try_from(child.id()).expect("a POSIX process id fits i32");
				// SAFETY: this child was spawned in its own process group;
				// its unreaped PID cannot be reused by an unrelated process.
				unsafe { kill(-group, 9) };
			}
			let _ = child.kill();
			let _ = child.wait();
			return if over_memory_limit { -4 } else { -2 };
		}
		std::thread::sleep(Duration::from_millis(5));
	}
}

fn resident_kib(pid: u32) -> Option<usize> {
	#[cfg(target_os = "linux")]
	{
		// A newly spawned child can briefly retain this runner's image and
		// its LLVM heap before exec. Charge the executed program, not that
		// inherited address space (which may already exceed the child limit).
		let image = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
		if image == fs::read_link("/proc/self/exe").ok()? {
			return None;
		}
		let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
		let own: Option<usize> = status.lines().find_map(|line| {
			line.strip_prefix("VmRSS:")?
				.split_whitespace()
				.next()?
				.parse()
				.ok()
		});
		// The facade starts a compiler or runtime child. Bound each descendant's
		// RSS as well; its sparse virtual address space is intentionally unbounded.
		let children =
			fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).unwrap_or_default();
		children
			.split_whitespace()
			.filter_map(|pid| pid.parse().ok())
			.filter_map(resident_kib)
			.chain(own)
			.max()
	}
	#[cfg(not(target_os = "linux"))]
	{
		let _ = pid;
		None
	}
}

#[cfg(target_os = "linux")]
#[test]
fn resident_memory_is_charged_after_exec() {
	use std::os::unix::process::CommandExt;

	unsafe extern "C" {
		fn fork() -> i32;
		fn pause() -> i32;
		fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
	}
	// SAFETY: the child calls only async-signal-safe pause until the parent
	// kills it; it never touches Rust state inherited from other test threads.
	let pid = unsafe { fork() };
	assert!(pid >= 0, "fork the inherited runner image");
	if pid == 0 {
		loop {
			unsafe { pause() };
		}
	}
	let inherited = resident_kib(pid as u32);
	// SAFETY: this unreaped child is ours and cannot have its PID reused.
	unsafe {
		kill(pid, 9);
		waitpid(pid, std::ptr::null_mut(), 0);
	}
	assert!(inherited.is_none(), "charged the runner's inherited heap");

	let mut child = Command::new("sh")
		.args(["-c", "sleep 60"])
		.process_group(0)
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap();
	let started = Instant::now();
	let running = loop {
		if let Some(rss) = resident_kib(child.id()) {
			break Some(rss);
		}
		if started.elapsed() > Duration::from_secs(2) {
			break None;
		}
		std::thread::sleep(Duration::from_millis(5));
	};
	// SAFETY: the child was spawned in this new process group and is unreaped.
	unsafe { kill(-(child.id() as i32), 9) };
	child.wait().unwrap();
	assert!(
		running.is_some_and(|rss| rss > 0),
		"ignored the executed image"
	);
}

#[cfg(target_os = "linux")]
#[test]
fn a_timeout_stops_the_program_below_the_fa_process() {
	use std::os::unix::process::CommandExt;

	let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-timeout");
	fs::create_dir_all(&directory).unwrap();
	let pid_file = directory.join(format!("child-{}.pid", std::process::id()));
	let _ = fs::remove_file(&pid_file);
	let mut child = Command::new("sh")
		.args(["-c", "sleep 60 & echo $! > \"$1\"; wait", "sh"])
		.arg(&pid_file)
		.process_group(0)
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap();
	let started = Instant::now();
	let pid = loop {
		if let Some(pid) = fs::read_to_string(&pid_file)
			.ok()
			.and_then(|text| text.trim().parse::<i32>().ok())
		{
			break pid;
		}
		if started.elapsed() > Duration::from_secs(5) {
			wait_for_exit(&mut child, Duration::ZERO);
			panic!("the facade didn't start its child");
		}
		std::thread::sleep(Duration::from_millis(5));
	};
	assert_eq!(wait_for_exit(&mut child, Duration::from_millis(20)), -2);
	let started = Instant::now();
	let stopped = loop {
		let state = fs::read_to_string(format!("/proc/{pid}/stat")).ok();
		// An orphan may wait briefly for init to reap it; a zombie has no
		// running code or open files, so cannot hold the suite lock.
		if state.is_none()
			|| state
				.as_deref()
				.is_some_and(|state| matches!(state.split_whitespace().nth(2), Some("Z" | "X")))
		{
			break true;
		}
		if started.elapsed() > Duration::from_secs(2) {
			break false;
		}
		std::thread::sleep(Duration::from_millis(5));
	};
	if !stopped {
		// SAFETY: a child left by a broken timeout must not outlive a failed test.
		unsafe { kill(pid, 9) };
	}
	assert!(stopped, "the timed-out program still runs as process {pid}");
}

/// A program of several files (`-- file: path` sections, like the LLVM
/// backend's `async-workers.fa`): its files written into `directory`, and
/// its package root, or the path of its entry (`main.fa`, else the first
/// `.fa` file). `None` for a program of one file.
fn multi_file_entry(program: &Path, directory: &Path) -> Option<PathBuf> {
	let text = fs::read_to_string(program).ok()?;
	let mut files: Vec<(String, String)> = Vec::new();
	for line in text.lines() {
		if let Some(path) = line.trim_start().strip_prefix("-- file:") {
			files.push((path.trim().to_string(), String::new()));
		} else if let Some((_, file)) = files.last_mut() {
			file.push_str(line);
			file.push('\n');
		}
	}
	if files.is_empty() {
		return None;
	}
	let _ = fs::remove_dir_all(directory);
	for (path, text) in &files {
		let path = directory.join(path);
		fs::create_dir_all(path.parent()?).ok()?;
		fs::write(&path, text).ok()?;
	}
	if files.iter().any(|(path, _)| path == "package.fa") {
		return Some(directory.to_path_buf());
	}
	let entry = files
		.iter()
		.find(|(path, _)| path == "main.fa")
		.or_else(|| files.iter().find(|(path, _)| path.ends_with(".fa")))?;
	Some(directory.join(&entry.0))
}

/// Why a program's run differs from its annotations, if it does.
fn compare(expected: &Expected, outcome: &Outcome) -> Option<String> {
	let printed: Vec<&str> = outcome.stdout.lines().collect();
	let mut problems = Vec::new();
	if outcome.build_failed && (expected.exit != 1 || expected.stderr.is_empty()) {
		problems.push("build failed".to_string());
	}
	if printed != expected.stdout {
		problems.push(format!(
			"stdout: expected {:?}, got {:?}",
			expected.stdout, printed
		));
	}
	if outcome.exit != expected.exit {
		problems.push(format!(
			"exit code: expected {}, got {}",
			expected.exit, outcome.exit
		));
	}
	for text in &expected.stderr {
		if !outcome.stderr.contains(text.as_str()) {
			problems.push(format!("stderr doesn't contain {text:?}"));
		}
	}
	if problems.is_empty() {
		None
	} else {
		if !outcome.stderr.is_empty() {
			problems.push(format!("stderr: {}", outcome.stderr.trim_end()));
		}
		Some(problems.join("\n    "))
	}
}

/// Runs programs on a few workers; returns `(path, expected, outcome)` in
/// the order of `programs`.
fn run_all(
	programs: Vec<(PathBuf, Expected)>,
	scratch: &Path,
) -> Vec<(PathBuf, Expected, Outcome)> {
	fs::create_dir_all(scratch).unwrap();
	let queue = Mutex::new(programs.into_iter().enumerate());
	let results = Mutex::new(Vec::new());
	std::thread::scope(|scope| {
		for _ in 0..WORKERS {
			scope.spawn(|| {
				loop {
					let next = queue.lock().unwrap().next();
					let Some((index, (path, mut expected))) = next else {
						break;
					};
					let mut outcome = limited(|| run(&path, &expected, scratch, index));
					if JAVASCRIPT_ORDER
						.iter()
						.any(|(file, _)| path.file_name().is_some_and(|name| name == *file))
					{
						expected.stdout.sort();
						let mut lines: Vec<&str> = outcome.stdout.lines().collect();
						lines.sort_unstable();
						outcome.stdout = lines.join("\n");
					}
					results
						.lock()
						.unwrap()
						.push((index, path, expected, outcome));
				}
			});
		}
	});
	let mut results = results.into_inner().unwrap();
	results.sort_by_key(|(index, ..)| *index);
	results
		.into_iter()
		.map(|(_, path, expected, outcome)| (path, expected, outcome))
		.collect()
}

/// The `.fa` files under a directory, recursively, sorted.
fn fa_files(directory: &Path) -> Vec<PathBuf> {
	let mut files = Vec::new();
	let mut pending = vec![directory.to_path_buf()];
	while let Some(directory) = pending.pop() {
		let Ok(entries) = fs::read_dir(&directory) else {
			continue;
		};
		for entry in entries {
			let path = entry.unwrap().path();
			if path.is_dir() {
				pending.push(path);
			} else if path.extension().is_some_and(|extension| extension == "fa") {
				files.push(path);
			}
		}
	}
	files.sort();
	files
}

/// Whether the programs are filtered in, by `FA_PROGRAMS`.
fn selected(path: &Path) -> bool {
	match std::env::var("FA_PROGRAMS") {
		Ok(filter) if !filter.is_empty() => path.to_string_lossy().contains(&filter),
		_ => true,
	}
}

fn verbose() -> bool {
	std::env::var("FA_VERBOSE").is_ok_and(|value| !value.is_empty() && value != "0")
}

/// Whether Bun is available (otherwise the tests are skipped).
fn has_bun() -> bool {
	if fa::find_bun().is_some() {
		return true;
	}
	eprintln!("Bun is not installed (PATH or ~/.bun/bin): the end-to-end tests are skipped");
	false
}

#[test]
fn programs_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
	let programs: Vec<(PathBuf, Expected)> = fa_files(&directory)
		.into_iter()
		.filter(|path| selected(path))
		.map(|path| {
			let text = fs::read_to_string(&path).unwrap();
			let expected = expectations(&text);
			assert!(
				expected.annotated,
				"{} has no `-- prints:` nor `-- exit:` annotation",
				path.display()
			);
			(path, expected)
		})
		.filter(|(_, expected)| !expected.skip_targets.iter().any(|target| target == "bun"))
		.collect();
	let count = programs.len();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-programs");
	let mut failures = Vec::new();
	let mut pending = 0;
	for (path, expected, outcome) in run_all(programs, &scratch) {
		let name = path.strip_prefix(&directory).unwrap_or(&path).display();
		match (compare(&expected, &outcome), &expected.pending) {
			(None, None) if verbose() => eprintln!("ok   {name}"),
			(None, None) => {}
			(None, Some(reason)) => {
				eprintln!("{name} is pending ({reason}) but passes: remove its `-- pending:`")
			}
			(Some(problem), Some(reason)) => {
				pending += 1;
				if verbose() {
					eprintln!("pending {name} ({reason}):\n    {problem}");
				}
			}
			(Some(problem), None) => failures.push(format!("{name}:\n    {problem}")),
		}
	}
	eprintln!(
		"{} of {count} programs passed ({pending} pending)",
		count - failures.len() - pending
	);
	assert!(
		failures.is_empty(),
		"{} programs failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

/// The async programs of the LLVM backend whose tasks interleave earlier on
/// JavaScript, as A1 allows (an `await` that doesn't wait still lets the
/// other tasks run, and a started task begins at the starter's next
/// `await`): `(file name, reason)`. Their lines are compared in any order.
const JAVASCRIPT_ORDER: &[(&str, &str)] = &[];

/// The async programs of the LLVM backend whose stderr is native only:
/// `(file name, reason)`.
const NATIVE_STDERR: &[(&str, &str)] = &[(
	"async-long-turn.fa",
	"the dev profile's warning about long turns is native only",
)];

fn async_expectations(path: &Path, text: &str) -> Expected {
	let mut expected = expectations(text);
	if NATIVE_STDERR
		.iter()
		.any(|(file, _)| path.file_name().is_some_and(|name| name == *file))
	{
		expected.stderr.clear();
	}
	expected
}

#[test]
fn async_programs_print_what_they_print_natively() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../codegen-llvm/tests/programs");
	let programs: Vec<(PathBuf, Expected)> = fa_files(&directory)
		.into_iter()
		.filter(|path| {
			path.file_name()
				.is_some_and(|name| name.to_string_lossy().starts_with("async-"))
		})
		.filter(|path| selected(path))
		.map(|path| {
			let expected = async_expectations(&path, &fs::read_to_string(&path).unwrap());
			(path, expected)
		})
		.filter(|(_, expected)| !expected.skip_targets.iter().any(|target| target == "bun"))
		.collect();
	assert!(!programs.is_empty() || std::env::var("FA_PROGRAMS").is_ok());
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-async");
	let mut failures = Vec::new();
	for (path, expected, outcome) in run_all(programs, &scratch) {
		let name = path.file_name().unwrap().to_string_lossy().into_owned();
		// `-- pending: reason`: a construct the JavaScript backend lacks yet
		match (compare(&expected, &outcome), &expected.pending) {
			(Some(problem), None) => failures.push(format!("{name}:\n    {problem}")),
			(Some(_), Some(_)) => {}
			(None, Some(reason)) => {
				eprintln!("{name} is pending ({reason}) but passes: remove its `-- pending:`")
			}
			(None, None) if verbose() => eprintln!("ok   {name}"),
			(None, None) => {}
		}
	}
	assert!(
		failures.is_empty(),
		"{} async programs failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

/// The programs also run minified (`fa build --minify=true`): printing, errors,
/// stops, unions and names are the parts a minifier could break.
const MINIFIED: &[&str] = &[
	"errors.fa",
	"javascript-names.fa",
	"named-arguments.fa",
	"printing.fa",
	"stack-overflow.fa",
	"unions-and-enums.fa",
	"value-semantics.fa",
];

#[test]
fn minified_programs_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
	let programs: Vec<(PathBuf, Expected)> = MINIFIED
		.iter()
		.map(|name| directory.join(name))
		.filter(|path| selected(path))
		.map(|path| {
			let mut expected = expectations(&fs::read_to_string(&path).unwrap());
			expected.minified = true;
			(path, expected)
		})
		.collect();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-minified");
	let failures: Vec<String> = run_all(programs, &scratch)
		.into_iter()
		.filter_map(|(path, expected, outcome)| {
			let problem = compare(&expected, &outcome)?;
			Some(format!("{}:\n    {problem}", path.display()))
		})
		.collect();
	assert!(
		failures.is_empty(),
		"{} minified programs failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

/// Analyzer cases whose run is known to differ, because of a gap of the
/// analyzer or of the backend: `(file name, reason)`. They are run and
/// reported, not counted as failures; one that passes is reported too.
const ANALYZER_GAPS: &[(&str, &str)] = &[];

/// The analyzer cases that the analyzer checks but the JavaScript backend
/// doesn't generate, or doesn't run as annotated, yet (`fa_analyzer::analysis`
/// documents how to lower what the analyzer types). A listed case that runs
/// as annotated is reported, to be removed from the list.
const BACKEND_GAPS: &[&str] = &[];

/// The analyzer cases that describe a run: valid (no expected diagnostic),
/// not pending, one file, not a test file, with a `prints:` or `exit:`.
fn runnable_case(text: &str) -> bool {
	let mut annotated = false;
	for line in text.lines() {
		let Some(start) = find_comment(line) else {
			continue;
		};
		let comment = line[start + 2..].trim_start();
		let key = comment
			.split(|c: char| c == ':' || c.is_whitespace())
			.next()
			.unwrap_or("");
		match key {
			"pending" | "file" | "kind" | "error" => return false,
			"prints" | "exit" => annotated = true,
			_ => {}
		}
	}
	annotated
}

#[test]
fn analyzer_cases_print_what_they_say() {
	if !has_bun() {
		return;
	}
	let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../analyzer/tests/cases");
	let mut not_checked = Vec::new();
	let mut not_generated = Vec::new();
	let mut native_only = 0;
	let mut backend_gaps = 0;
	let mut programs = Vec::new();
	for path in fa_files(&directory) {
		if !selected(&path) {
			continue;
		}
		let text = fs::read_to_string(&path).unwrap();
		if !runnable_case(&text) {
			continue;
		}
		let (mut session, file) = fa_analyzer::analyze_source(&text);
		let errors = session
			.diagnostics(file)
			.iter()
			.any(|diagnostic| diagnostic.severity == fa_analyzer::Severity::Error);
		if errors {
			not_checked.push(path);
			continue;
		}
		let options = fa_codegen_js::Options::new(fa_codegen_js::Target::Bun, "main.fa");
		let relative = path
			.strip_prefix(&directory)
			.unwrap_or(&path)
			.to_string_lossy()
			.replace('\\', "/");
		let backend_gap = BACKEND_GAPS.contains(&relative.as_str());
		let generated = fa_codegen_js::generate(&mut session, file, &options);
		if backend_gap && generated.is_err() {
			backend_gaps += 1;
			continue;
		}
		if let Err(error) = generated {
			// a value certainly beyond ±(2^53 − 1): a native-only case (values brief D-9)
			if error
				.diagnostics
				.iter()
				.all(|diagnostic| diagnostic.code.id == "E-NUM-19")
			{
				native_only += 1;
				continue;
			}
			not_generated.push(format!(
				"{}: {}",
				path.strip_prefix(&directory).unwrap_or(&path).display(),
				error.to_string().trim_end()
			));
			continue;
		}
		let expected = expectations(&text);
		if !expected.skip_targets.iter().any(|target| target == "bun") {
			programs.push((path, expected));
		}
	}
	let count = programs.len();
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fa-analyzer-cases");
	let mut failures = Vec::new();
	let mut gaps = 0;
	let mut ran_backend_gaps = 0;
	for (path, expected, outcome) in run_all(programs, &scratch) {
		let name = path.strip_prefix(&directory).unwrap_or(&path).display();
		let file = path.file_name().unwrap_or_default().to_string_lossy();
		let gap = ANALYZER_GAPS
			.iter()
			.find(|(gap, _)| *gap == file)
			.map(|(_, reason)| reason);
		let relative = name.to_string().replace('\\', "/");
		if BACKEND_GAPS.contains(&relative.as_str()) {
			match compare(&expected, &outcome) {
				None => eprintln!("{relative} passes: remove it from `BACKEND_GAPS`"),
				Some(_) => ran_backend_gaps += 1,
			}
			continue;
		}
		match (compare(&expected, &outcome), gap) {
			(None, None) if verbose() => eprintln!("ok   {name}"),
			(None, None) => {}
			(None, Some(_)) => eprintln!("{name} passes: remove it from `ANALYZER_GAPS`"),
			(Some(_), Some(reason)) => {
				gaps += 1;
				eprintln!("known analyzer gap, {name}: {reason}");
			}
			(Some(problem), None) => failures.push(format!("{name}:\n    {problem}")),
		}
	}
	eprintln!(
		"analyzer cases: {} of {count} run as annotated ({gaps} known analyzer gaps, {ran_backend_gaps} known backend gaps); {} don't check yet; {} not generated ({backend_gaps} known backend gaps); {native_only} native only (E-NUM-19)",
		count - failures.len() - gaps - ran_backend_gaps,
		not_checked.len(),
		not_generated.len()
	);
	if verbose() {
		for line in &not_generated {
			eprintln!("not generated: {line}");
		}
	}
	assert!(
		not_generated.is_empty(),
		"the JavaScript backend refused {} valid cases:\n{}",
		not_generated.len(),
		not_generated.join("\n")
	);
	assert!(
		failures.is_empty(),
		"{} cases failed:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

#[path = "programs/reference_matrix.rs"]
mod reference_matrix;
