//! Node and browser-global execution, sharing annotations, discovery, workers,
//! timeouts and comparisons with the Bun tests in the parent module.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use super::{Expected, Outcome};

fn node() -> Option<PathBuf> {
	static NODE: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
	NODE.get_or_init(|| {
		let local = std::env::var_os("HOME")
			.map(PathBuf::from)
			.map(|home| home.join(".local/bin/node"));
		local
			.into_iter()
			.chain([PathBuf::from("node")])
			.find(|path| {
				Command::new(path)
					.arg("--version")
					.output()
					.is_ok_and(|output| output.status.success())
			})
	})
	.clone()
}

pub(super) fn command(target: &str, entry: &Path) -> Command {
	let mut command = Command::new(node().expect("Node was checked before running the suite"));
	if target == "web" {
		command.args([
			"--experimental-vm-modules",
			"--disable-warning=ExperimentalWarning",
		]);
		command.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/web-runner.mjs"));
		command.arg(entry);
	} else {
		command.arg(entry);
	}
	command
}

fn programs(target: &'static str) -> Vec<(PathBuf, Expected)> {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let mut programs = Vec::new();
	for (directory, analyzer, asynchronous) in [
		(manifest.join("tests/programs"), false, false),
		(manifest.join("../analyzer/tests/cases"), true, false),
		(manifest.join("../codegen-llvm/tests/programs"), false, true),
	] {
		for path in super::fa_files(&directory) {
			if !super::selected(&path)
				|| (asynchronous
					&& !path
						.file_name()
						.unwrap()
						.to_string_lossy()
						.starts_with("async-"))
			{
				continue;
			}
			let text = fs::read_to_string(&path).unwrap();
			if analyzer && !super::runnable_case(&text) {
				continue;
			}
			let mut expected = if asynchronous { super::async_expectations(&path, &text) } else { super::expectations(&text) };
			assert!(
				expected.annotated,
				"{} has no output annotation",
				path.display()
			);
			if expected.skip_targets.iter().any(|skip| skip == target) {
				continue;
			}

			expected.target = target;
			programs.push((path, expected));
		}
	}
	programs
}

fn key(path: &Path) -> String {
	let packages = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.canonicalize()
		.unwrap();
	path.canonicalize()
		.unwrap()
		.strip_prefix(packages)
		.unwrap()
		.to_string_lossy()
		.replace('\\', "/")
}

fn skips_file(target: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/expected-skips")
		.join(format!("{target}.txt"))
}

fn expected_skips(target: &str) -> BTreeMap<String, String> {
	let text = fs::read_to_string(skips_file(target)).expect("the target's ratchet file exists");
	let mut skips = BTreeMap::new();
	for line in text
		.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
	{
		let (path, reason) = line
			.split_once(": ")
			.expect("ratchet lines are `path: reason`");
		assert!(!reason.is_empty(), "every ratchet entry needs a reason");
		assert!(
			skips.insert(path.to_string(), reason.to_string()).is_none(),
			"duplicate ratchet entry {path}"
		);
	}
	skips
}

/// These are successful negative tests, not backend gaps. Require every error
/// to name a documented target restriction, so an unrelated error can't hide.
fn documented_refusal(outcome: &Outcome, target: &str) -> bool {
	if !outcome.build_failed {
		return false;
	}
	let errors: Vec<_> = outcome
		.stderr
		.lines()
		.filter(|line| line.contains("error["))
		.collect();
	!errors.is_empty()
		&& errors.iter().all(|line| {
			(line.contains("error[AN004]")
				&& line.contains(&format!("doesn't exist on target `{target}`")))
				|| (line.contains("error[FFI008]")
					&& line.contains(&format!("on target `{target}`")))
				|| line.contains("error[E-NUM-19]")
				|| line.contains("error[CONC001]")
				|| (line.contains("error[CONC010]") && line.contains(target))
		})
}

fn run_target(target: &'static str) {
	if node().is_none() {
		eprintln!(
			"Node is not installed (~/.local/bin/node or PATH): {target} end-to-end tests are skipped"
		);
		return;
	}
	let started = Instant::now();
	let programs = programs(target);
	let expected = expected_skips(target);
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
		.join(format!("fa-{target}-programs-{}", std::process::id()));
	let update = std::env::var_os("FA_UPDATE_SKIPS").is_some();
	let mut seen = BTreeSet::new();
	let mut actual = BTreeMap::new();
	let mut failures = Vec::new();
	let (mut passed, mut refused) = (0, 0);
	for (path, annotation, outcome) in super::run_all(programs, &scratch) {
		let key = key(&path);
		seen.insert(key.clone());
		let negative = documented_refusal(&outcome, target);
		let problem = if negative && !(annotation.exit == 1 && !annotation.stderr.is_empty()) {
			None
		} else {
			super::compare(&annotation, &outcome)
		};
		match problem {
			None if expected.contains_key(&key) && !update => failures.push(format!(
				"{key}: passes now; remove it from {}",
				skips_file(target).display()
			)),
			None => {
				if negative {
					refused += 1;
				} else {
					passed += 1;
				}
				if super::verbose() {
					eprintln!(
						"{target} ok {key}{}",
						if negative {
							" (documented refusal)"
						} else {
							""
						}
					);
				}
			}
			Some(problem) => {
				if !expected.contains_key(&key) && !update {
					failures.push(format!("{key}:\n    {problem}"));
				}
				if super::verbose() || update {
					eprintln!("{target} gap {key}:\n    {problem}");
				}
				actual.insert(
					key.clone(),
					expected
						.get(&key)
						.cloned()
						.unwrap_or_else(|| problem.lines().next().unwrap().to_string()),
				);
			}
		}
	}
	let filtered = std::env::var("FA_PROGRAMS").is_ok_and(|value| !value.is_empty());
	if !filtered {
		for stale in expected
			.keys()
			.filter(|key| !update && !seen.contains(*key))
		{
			failures.push(format!(
				"{stale}: ratchet entry wasn't exercised; remove it"
			));
		}
		if update {
			let mut text = format!(
				"# Known failures on {target}; every entry is executed.\n# An unlisted failure or a listed success fails tests/programs.rs.\n"
			);
			for (key, reason) in &actual {
				text.push_str(&format!("{key}: {reason}\n"));
			}
			fs::create_dir_all(skips_file(target).parent().unwrap()).unwrap();
			fs::write(skips_file(target), text).unwrap();
		}
	}
	fs::remove_dir_all(scratch).unwrap();
	eprintln!(
		"{target}: {passed} passed, {refused} documented refusals, {} ratcheted failures, {} unexpected failures ({:.2}s)",
		actual.len(),
		failures.len(),
		started.elapsed().as_secs_f64()
	);
	assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn node_programs() {
	run_target("node");
}

#[test]
fn web_programs() {
	run_target("web");
}

/// Exercise the runner independently of Fa code generation: a generator bug
/// must never be mistaken for a sandbox omission (or a Node global leak).
#[test]
fn web_sandbox_contract() {
	if node().is_none() {
		return;
	}
	let scratch =
		Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("web-contract-{}", std::process::id()));
	fs::create_dir_all(&scratch).unwrap();
	let entry = scratch.join("main.js");
	let cases = [
		(
			"globals",
			r#"
for (const name of ['process', 'Buffer', 'require', 'Bun', 'setImmediate', 'global']) {
  if (typeof globalThis[name] !== 'undefined') throw Error(name + ' leaked');
}
for (const name of ['console','TextEncoder','TextDecoder','URL','fetch','crypto','performance',
 'WebAssembly','Worker','MessageChannel','SharedArrayBuffer','Atomics','queueMicrotask','structuredClone']) {
  if (typeof globalThis[name] === 'undefined') throw Error(name + ' missing');
}
if ('nodeTiming' in performance) throw Error('Node performance leaked');
const timer = setTimeout(() => { throw Error('cancelled timer ran'); }, 100);
if (typeof timer !== 'number') throw Error('Node timer leaked');
clearTimeout(timer);
const channel = new MessageChannel();
if ('on' in channel.port1 || 'unref' in channel.port1) throw Error('Node port leaked');
channel.port1.close(); channel.port2.close();
const original = { items: [new Map([['x', 3]])] }; original.self = original;
const value = structuredClone(original);
if (Object.getPrototypeOf(value) !== Object.prototype || !(value.items instanceof Array)
 || !(value.items[0] instanceof Map) || value.self !== value) throw Error('wrong clone realm');
console.log('web globals');
function faExit(code) {} faExit(0);
"#,
			0,
			"web globals\n",
		),
		("plain", "console.log('plain main');", 0, "plain main\n"),
		(
			"status",
			"function faExit(code) {} queueMicrotask(() => faExit(37));",
			37,
			"",
		),
		("node-import", "await import('node:fs');", 1, ""),
		("bun-import", "await import('bun:ffi');", 1, ""),
		(
			"exception",
			"throw Error('unhandled sandbox error');",
			1,
			"",
		),
		(
			"worker",
			r#"
function faExit(code) {}
const worker = new Worker(new URL('./worker.js', import.meta.url), {type:'module'});
worker.onmessage = ({data}) => {
 if (Object.getPrototypeOf(data) !== Object.prototype || !(data.items instanceof Array)) throw Error('wrong worker clone realm');
 console.log(data.items[0]); faExit(0);
};
worker.postMessage({ items: ['worker globals'] });
"#,
			0,
			"worker globals\n",
		),
	];
	fs::write(scratch.join("worker.js"), "if (typeof process !== 'undefined' || typeof Buffer !== 'undefined' || typeof Bun !== 'undefined' || typeof require !== 'undefined') throw Error('worker Node leak'); onmessage = ({data}) => { if (Object.getPrototypeOf(data) !== Object.prototype || !(data.items instanceof Array)) throw Error('wrong input realm'); postMessage(data); };").unwrap();
	for (name, source, code, stdout) in cases {
		fs::write(&entry, source).unwrap();
		let outcome =
			super::limited(|| super::execute(&mut command("web", &entry), &scratch, name));
		assert_eq!(outcome.exit, code, "{name}: {outcome:?}");
		assert_eq!(outcome.stdout, stdout, "{name}: {outcome:?}");
		if name.ends_with("import") {
			assert!(
				outcome.stderr.contains("web sandbox refuses import"),
				"{outcome:?}"
			);
		}
	}
	fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn only_documented_target_diagnostics_count_as_refusals() {
	let mut outcome = Outcome {
		build_failed: true,
		stdout: String::new(),
		exit: -3,
		stderr: "error[AN004]: `File.read` doesn't exist on target `web`".into(),
	};
	assert!(documented_refusal(&outcome, "web"));
	assert!(!documented_refusal(&outcome, "node"));
	outcome
		.stderr
		.push_str("\nerror[E-TYPE-1]: unrelated failure");
	assert!(!documented_refusal(&outcome, "web"));
}

#[test]
fn target_restrictions_are_rejected_at_build_time() {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
		.join(format!("target-refusals-{}", std::process::id()));
	fs::create_dir_all(&scratch).unwrap();
	for (index, (target, file, code)) in [
		("web", "std-library.fa", "AN004"),
		("web", "foreign-libc.foreign.fa", "FFI008"),
		("node", "foreign-libc.foreign.fa", "FFI008"),
	]
	.into_iter()
	.enumerate()
	{
		let expected = Expected {
			target,
			..Expected::default()
		};
		let outcome = super::limited(|| {
			super::run(
				&manifest.join("tests/programs").join(file),
				&expected,
				&scratch,
				index,
			)
		});
		assert!(
			outcome.build_failed,
			"{file} must be refused on {target}: {outcome:?}"
		);
		assert!(
			outcome.stderr.contains(&format!("error[{code}]")),
			"{outcome:?}"
		);
		assert!(documented_refusal(&outcome, target), "{outcome:?}");
	}
	fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn shared_annotations_select_targets_and_exclude_diagnostic_cases() {
	let expected = super::expectations(
		"console.log(3) -- prints: 3\n-- skip-target: node web\n-- args: one two\n-- exit: 7",
	);
	assert_eq!(expected.skip_targets, ["node", "web"]);
	assert_eq!(expected.args, ["one", "two"]);
	assert_eq!(expected.exit, 7);
	assert!(!super::runnable_case(
		"call() -- error EFF007\n-- prints: 3"
	));
	assert!(!super::runnable_case(
		"call() -- error: EFF007\n-- prints: 3"
	));
	assert!(super::runnable_case("-- stderr: error: Boom\n-- exit: 1"));
}
