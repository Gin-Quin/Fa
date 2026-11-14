//! The determinism of the parallel pipeline (design/parallel-pipeline.md,
//! P10): a package analyzed and generated with 1, 2 and 8 threads, under
//! several schedule seeds, gives byte-identical diagnostics and JavaScript,
//! identical to the sequential analysis (one session, every file kept).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::ids::FileId;
use fa_analyzer::pipeline::{self, UnitSink};
use fa_analyzer::{CodebaseResolver, Session, Severity};
use fa_codegen_js::{EntryInputs, EntryModule, EntryOptions, ModuleSink, Options, Target};

/// A package of `count` library files in levels: each file is a namespace
/// with object, union and error types, a generic function, a `static`,
/// and functions calling the files of earlier levels; three files call each
/// other in a cycle; `errors` files have an error.
fn package(count: usize, errors: &[usize]) -> Vec<(String, String)> {
	let mut files = vec![(
		"package.fa".to_string(),
		"export = {\n\tname = \"p\"\n\tversion = \"0.1.0\"\n\ttargets = [\"bun\", \"native\"]\n}\n"
			.to_string(),
	)];
	for index in 0..count {
		let mut text = String::from("export namespace = {\n");
		text.push_str(&format!(
			"\ttype Item = {{\n\t\tid: Integer\n\t\tweight: Integer\n\t\tfunction score = (self): Integer => self.id * {} + self.weight\n\t}}\n",
			index % 7 + 2
		));
		text.push_str("\ttype Shape = Union {\n\t\tCircle = { radius: Integer }\n\t\tEmpty\n\t}\n");
		text.push_str("\ttype Failure = Errors { Missing = { key: String }, Invalid }\n");
		text.push_str(&format!("\tstatic base = {}\n", index % 5 + 1));
		text.push_str("\tfunction square = (x: Integer): Integer => x * x\n");
		text.push_str("\tstatic table = @square(base)\n");
		text.push_str(&format!(
			"\tfunction make = (seed: Integer): Item => Item {{ id = seed, weight = seed %% {} }}\n",
			index % 11 + 3
		));
		text.push_str("\tfunction firstOr = <Value>(values: Array<Value>, fallback: Value): Value => values.get(0) ?? fallback\n");
		text.push_str("\tfunction area = (shape: Shape): Integer => when shape is {\n\t\tShape.Circle as { radius } => radius * radius\n\t\tShape.Empty => 0\n\t}\n");
		text.push_str("\tfunction compute = (x: Integer): Integer => {\n\t\tmutable result = make(x).score() + table + firstOr([x], 0)\n");
		for used in [index / 2, index / 3]
			.into_iter()
			.filter(|&used| used < index)
			.collect::<BTreeSet<_>>()
		{
			text.push_str(&format!(
				"\t\tresult += m{used:03}.compute(x %% 50) %% 100 + m{used:03}.make(x).score() %% 10 + m{used:03}.base\n"
			));
		}
		if index == count / 2 {
			text.push_str("\t\tresult += ringA.call(x %% 5)\n");
		}
		if index % 5 == 1 {
			text.push_str(
				"\t\t@if target is \"native\" {\n\t\t\tresult += 1\n\t\t} @else {\n\t\t\tresult += 2\n\t\t}\n",
			);
		}
		if errors.contains(&index) {
			text.push_str("\t\tlet wrong: Integer = \"text\"\n\t\tresult += missing\n");
		}
		text.push_str("\t\tresult %% 1000\n\t}\n}\n");
		files.push((format!("library/m{index:03}.fa"), text));
	}
	for (name, next, step) in [
		("ringA", "ringB", 1),
		("ringB", "ringC", 2),
		("ringC", "ringA", 3),
	] {
		files.push((
			format!("library/{name}.fa"),
			format!(
				"export namespace = {{\n\tfunction call = (x: Integer): Integer => if x <= 0 {{ 0 }} else {{ {next}.call(x - 1) + {step} }}\n}}\n"
			),
		));
	}
	let calls: String = (count.saturating_sub(4)..count)
		.map(|index| format!("\ttotal += library.m{index:03}.compute({index})\n"))
		.collect();
	files.push((
		"application/main.fa".to_string(),
		format!(
			"export function = (arguments: Array<String>): Integer => {{\n\tmutable total = 0\n{calls}\tconsole.log(total)\n\t0\n}}\n"
		),
	));
	files
}

/// What a run gives: the rendered diagnostics of every file, and the
/// JavaScript modules (`None` when the package has errors).
type Outputs = (String, Option<BTreeMap<String, String>>);

/// Collects modules in memory.
struct Collect(Mutex<BTreeMap<String, String>>);

impl ModuleSink for Collect {
	fn write(&self, path: &str, code: &str) -> std::io::Result<()> {
		self.0
			.lock()
			.unwrap()
			.insert(path.to_string(), code.to_string());
		Ok(())
	}
}

fn render(path: &str, text: &str, diagnostics: &[fa_analyzer::Diagnostic]) -> String {
	render_all(diagnostics, &SourceView::new(path, text))
}

fn resolver(files: &[(String, String)]) -> CodebaseResolver {
	CodebaseResolver::from_texts(files, None)
}

/// The sequential analysis: one session, every file kept.
fn sequential(files: &[(String, String)]) -> Outputs {
	let mut session = Session::new();
	session.set_resolver(Box::new(resolver(files)));
	let program = session.analyze_package(Some("application/main"));
	let mut all: Vec<FileId> = program
		.files
		.iter()
		.chain(&program.tests)
		.copied()
		.collect();
	all.sort_by(|&a, &b| session.module(a).cmp(session.module(b)));
	let rendered: String = all
		.iter()
		.map(|&file| {
			render(
				&session.display_path(file),
				session.text(file),
				session.diagnostics(file),
			)
		})
		.collect();
	let options = Options::new(Target::Bun, "p");
	let sink = Collect(Mutex::new(BTreeMap::new()));
	let generated = fa_codegen_js::generate_program_into(
		&mut session,
		&program,
		&options,
		&EntryOptions::default(),
		&sink,
	);
	let modules = generated.ok().map(|_| sink.0.into_inner().unwrap());
	(rendered, modules)
}

/// Generates each unit's modules as it is analyzed.
struct Generate {
	sink: Collect,
	written: Mutex<Vec<(String, Vec<&'static str>, bool)>>,
	failed: Mutex<bool>,
}

impl UnitSink for Generate {
	fn unit(&self, session: &mut Session, files: &[FileId]) {
		let errors = files.iter().any(|&file| {
			session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.severity == Severity::Error)
		});
		if errors {
			*self.failed.lock().unwrap() = true;
			return;
		}
		let options = Options::new(Target::Bun, "p");
		for &file in files {
			match fa_codegen_js::generate_unit_module(session, file, &options, false, &self.sink) {
				Ok(Ok(module)) => self.written.lock().unwrap().push((
					session.module(file).to_string(),
					module.helpers,
					module.has_main,
				)),
				_ => *self.failed.lock().unwrap() = true,
			}
		}
	}
}

/// The pipeline with a number of threads and a schedule seed.
fn parallel(files: &[(String, String)], threads: usize, seed: u64) -> Outputs {
	let mut session = Session::new();
	session.set_resolver(Box::new(resolver(files)));
	let options = pipeline::Options {
		entry: Some("application/main".to_string()),
		threads: Some(threads),
		seed,
		// a small budget: most trees are parsed again when their unit runs
		tree_budget: Some(64 << 10),
		..pipeline::Options::default()
	};
	let generate = Generate {
		sink: Collect(Mutex::new(BTreeMap::new())),
		written: Mutex::new(Vec::new()),
		failed: Mutex::new(false),
	};
	let report = pipeline::analyze_package(&mut session, &options, None, &generate);
	let texts: BTreeMap<String, String> = files
		.iter()
		.filter_map(|(path, text)| Some((path.strip_suffix(".fa")?.to_string(), text.clone())))
		.collect();
	let rendered: String = report
		.modules
		.iter()
		.map(|module| {
			render(
				&format!("{}.fa", module.module),
				&texts[&module.module],
				&module.diagnostics,
			)
		})
		.collect();
	if report.has_errors() || *generate.failed.lock().unwrap() {
		return (rendered, None);
	}
	let mut written = generate.written.into_inner().unwrap();
	written.sort();
	let mut helpers = BTreeSet::new();
	for (_, used, _) in &written {
		helpers.extend(used.iter().copied());
	}
	let entry = report.entry.as_ref().map(|module| EntryModule {
		module: module.clone(),
		has_main: written
			.iter()
			.any(|(other, _, main)| other == module && *main),
		export: report.entry_export(&session, module),
		asynchronous: false,
	});
	let inputs = EntryInputs {
		init_order: report.init_order.clone(),
		entry,
		tests: Vec::new(),
	};
	let main = fa_codegen_js::entry_module(
		&inputs,
		&Options::new(Target::Bun, "p"),
		&EntryOptions::default(),
		&mut helpers,
	);
	let mut modules = generate.sink.0.into_inner().unwrap();
	modules.insert("main.js".to_string(), main);
	modules.insert(
		"$runtime.js".to_string(),
		fa_codegen_js::runtime_module(&helpers),
	);
	(rendered, Some(modules))
}

fn assert_same(expected: &Outputs, found: &Outputs, what: &str) {
	assert_eq!(expected.0, found.0, "diagnostics differ ({what})");
	match (&expected.1, &found.1) {
		(Some(expected), Some(found)) => {
			assert_eq!(
				expected.keys().collect::<Vec<_>>(),
				found.keys().collect::<Vec<_>>(),
				"modules differ ({what})"
			);
			for (path, code) in expected {
				assert_eq!(code, &found[path], "{path} differs ({what})");
			}
		}
		(None, None) => {}
		_ => panic!("one run generated, the other didn't ({what})"),
	}
}

#[test]
fn the_pipeline_is_deterministic_and_matches_the_sequential_analysis() {
	let files = package(40, &[]);
	let expected = sequential(&files);
	assert!(expected.0.is_empty(), "{}", expected.0);
	assert!(expected.1.is_some());
	for threads in [1, 2, 8] {
		for seed in [0, 1, 2] {
			let found = parallel(&files, threads, seed);
			assert_same(
				&expected,
				&found,
				&format!("{threads} threads, seed {seed}"),
			);
		}
	}
}

#[test]
fn the_diagnostics_of_the_pipeline_are_deterministic() {
	let files = package(30, &[3, 11, 20]);
	let expected = sequential(&files);
	assert!(expected.0.contains("error["), "{}", expected.0);
	for threads in [1, 2, 8] {
		for seed in [0, 5] {
			let found = parallel(&files, threads, seed);
			assert_same(
				&expected,
				&found,
				&format!("{threads} threads, seed {seed}"),
			);
		}
	}
}
