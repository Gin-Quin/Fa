//! The determinism of the parallel pipeline on the features whose analysis
//! crosses files (design/parallel-pipeline.md, P10): relations, reactivity,
//! requirements, async functions and tasks, generic and recursive types,
//! the same object structure declared by many files, a cycle, errors. The
//! package is analyzed with 1, 2 and 8 threads under several schedule
//! seeds, then again for another target: the diagnostics, the published
//! interfaces, the initialization order and what is analyzed again must
//! be the same in every run, and the diagnostics the sequential analysis's.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use fa_analyzer::pipeline::{self, NoSink, Report};
use fa_analyzer::types::TypeStore;
use fa_analyzer::{CodebaseResolver, Diagnostic, Session};

/// The files of the package: the feature files, levels of files calling
/// earlier ones (each declares the same `Item` structure with a method),
/// a cycle, and `errors` files with errors.
fn package(levels: usize, errors: &[usize]) -> Vec<(String, String)> {
	let mut files: Vec<(String, String)> = [
		(
			"package.fa",
			"export = {\n\tname = \"p\"\n\tversion = \"0.1.0\"\n\ttargets = [\"bun\", \"native\"]\n}\n",
		),
		(
			"library/town.fa",
			"export namespace = {
	type Town = {
		people: Pool<Person> = []
		type Person = {
			name: String
			bestFriend: (Person in people)?
			bestFriendOf: Pool<people->bestFriend>
			friends: Pool<Person in people>
			friendOf: Pool<in people->friends>
		}
	}
	function count = (): Integer => {
		mutable town: Town = {}
		use ana = town.people.add({ name = \"Ana\" })
		use bao = town.people.add({ name = \"Bao\" })
		ana.bestFriend = bao
		ana.friends.add(bao)
		bao.bestFriendOf.size + bao.friendOf.size
	}
}
",
		),
		(
			"library/counter.fa",
			"export namespace = {
	type Counter = { label: String, reactive count: Integer }
	function next = (start: Integer): Integer => {
		mutable counter = Counter { label = \"clicks\", count = start }
		derived following = counter.count + town.count()
		counter.count = start * 2
		following
	}
}
",
		),
		(
			"library/greeting.fa",
			"export namespace = {
	type Logger = { info: (message: String) => Void }
	function greet = (name: String) requires logger: Logger => logger.info(\"Hello, {name}: {counter.next(2)}\")
}
",
		),
		(
			"library/fetch.fa",
			"export namespace = {
	type Response = { body: String }
	type FetchError = Errors { Missing = { url: String } }
	async function get = (url: String): Response throws FetchError => {
		if url == \"\" {
			throw FetchError.Missing { url }
		}
		Response { body = url }
	}
	async function both = (a: String, b: String): Integer => {
		let first = start get(a)
		let second = await get(b) catch error => Response { body = \"\" }
		let firstBody = await first catch error => Response { body = \"\" }
		firstBody.body.length + second.body.length + counter.next(1)
	}
}
",
		),
		(
			"library/tree.fa",
			"export namespace = {
	type Tree<Value> = { value: Value, children: Array<Tree<Value>> }
	function leaf = <Value>(value: Value): Tree<Value> => Tree { value, children = [] }
	function size = <Value>(tree: Tree<Value>): Integer => {
		mutable total = 1
		for child in tree.children {
			total += size(child)
		}
		total
	}
}
",
		),
	]
	.into_iter()
	.map(|(path, text)| (path.to_string(), text.to_string()))
	.collect();
	for index in 0..levels {
		let mut text = String::from("export namespace = {\n");
		text.push_str(&format!(
			"\ttype Item = {{\n\t\tid: Integer\n\t\tfunction score = (self): Integer => self.id * {}\n\t}}\n",
			index % 3 + 2
		));
		text.push_str(&format!("\tstatic base = {}\n", index % 4 + 1));
		text.push_str("\tfunction make = (seed: Integer): Item => Item { id = seed }\n");
		text.push_str("\tfunction compute = (x: Integer): Integer => {\n");
		text.push_str("\t\tmutable result = make(x).score() + base + tree.size(tree.leaf(x))\n");
		for used in [index / 2, index / 3]
			.into_iter()
			.filter(|&used| used < index)
			.collect::<BTreeSet<_>>()
		{
			text.push_str(&format!(
				"\t\tresult += m{used:02}.compute(x %% 7) + m{used:02}.make(x).score()\n"
			));
		}
		match index % 4 {
			0 => text.push_str("\t\tresult += counter.next(x)\n"),
			1 => text.push_str("\t\tresult += town.count()\n"),
			2 => text.push_str("\t\t@if target is \"native\" {\n\t\t\tresult += 1\n\t\t}\n"),
			_ => text.push_str("\t\tresult += ringA.call(x %% 3)\n"),
		}
		if errors.contains(&index) {
			text.push_str("\t\tlet wrong: Integer = \"text\"\n\t\tresult += missing\n");
		}
		text.push_str("\t\tresult %% 1000\n\t}\n}\n");
		files.push((format!("library/m{index:02}.fa"), text));
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
	let calls: String = (levels.saturating_sub(3)..levels)
		.map(|index| format!("\tconsole.log(library.m{index:02}.compute({index}))\n"))
		.collect();
	files.push((
		"application/main.fa".to_string(),
		format!(
			"export function = (arguments: Array(String)): Integer => {{
	provide library.greeting.Logger = {{ info = (message) => console.log(message) }}
	library.greeting.greet(\"Ana\")
{calls}	start do {{
		let total = await library.fetch.both(\"a\", \"bc\")
		console.log(total)
	}}
	0
}}
"
		),
	));
	files
}

/// A diagnostic, independent of the file ids of the session.
fn diagnostic(module: &str, diagnostic: &Diagnostic) -> String {
	format!(
		"{module} {} {}..{} {}\n",
		diagnostic.code.id,
		diagnostic.location.span.start,
		diagnostic.location.span.end,
		diagnostic.message
	)
}

/// What a run gives, as text: the diagnostics of each module, whether it
/// was analyzed and read the target, the hash of its published interface,
/// the initialization order, the entry.
fn outputs(session: &Session, report: &Report) -> String {
	let types = TypeStore::view(session.types.global());
	let mut out = String::new();
	for module in &report.modules {
		let hash = report
			.interface(session, &module.module)
			.map(|interface| interface.hash(&types));
		let _ = writeln!(
			out,
			"{} analyzed={} reads_target={} interface={hash:?}",
			module.module, module.analyzed, module.reads_target
		);
		for found in &module.diagnostics {
			out.push_str(&diagnostic(&module.module, found));
		}
	}
	let _ = writeln!(out, "init {:?}", report.init_order);
	let _ = writeln!(out, "entry {:?}", report.entry);
	out
}

/// The pipeline for `bun` then, for the modules that read the target, for
/// `native`, with a number of threads and a schedule seed.
fn parallel(files: &[(String, String)], threads: usize, seed: u64) -> (String, String) {
	let options = pipeline::Options {
		entry: Some("application/main".to_string()),
		threads: Some(threads),
		seed,
		// a small budget: most trees are parsed again when their unit runs
		tree_budget: Some(32 << 10),
		..pipeline::Options::default()
	};
	let mut session = Session::new();
	session.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
	let report = pipeline::analyze_package(&mut session, &options, None, &NoSink);
	assert!(!report.cancelled);
	let first = outputs(&session, &report);
	let mut other = Session::sharing_types_of(&session);
	other.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
	other.set_target("native");
	let retarget = pipeline::Options {
		retarget: true,
		..options
	};
	let again = pipeline::analyze_package(&mut other, &retarget, Some(&report), &NoSink);
	(first, outputs(&other, &again))
}

/// The diagnostics of the sequential analysis (one session, every file
/// kept), in the form of [`outputs`]' diagnostic lines.
fn sequential(files: &[(String, String)]) -> BTreeSet<String> {
	let mut session = Session::new();
	session.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
	let program = session.analyze_package(Some("application/main"));
	program
		.files
		.iter()
		.chain(&program.tests)
		.flat_map(|&file| {
			let module = session.module(file).to_string();
			session
				.diagnostics(file)
				.iter()
				.map(|found| diagnostic(&module, found))
				.collect::<Vec<_>>()
		})
		.collect()
}

/// The diagnostic lines of [`outputs`].
fn diagnostic_lines(outputs: &str) -> BTreeSet<String> {
	outputs
		.lines()
		.filter(|line| {
			!line.contains(" analyzed=")
				&& !line.starts_with("init ")
				&& !line.starts_with("entry ")
		})
		.map(|line| format!("{line}\n"))
		.collect()
}

fn assert_deterministic(files: &[(String, String)]) -> (String, String) {
	let expected = parallel(files, 1, 0);
	for threads in [1, 2, 8] {
		for seed in [0, 1, 7, 12_345] {
			if (threads, seed) == (1, 0) {
				continue;
			}
			let found = parallel(files, threads, seed);
			assert_eq!(
				expected.0, found.0,
				"the first run differs ({threads} threads, seed {seed})"
			);
			assert_eq!(
				expected.1, found.1,
				"the run for another target differs ({threads} threads, seed {seed})"
			);
		}
	}
	assert_eq!(
		diagnostic_lines(&expected.0),
		sequential(files),
		"the pipeline and the sequential analysis disagree"
	);
	expected
}

#[test]
fn the_pipeline_is_deterministic_on_features_crossing_files() {
	let files = package(24, &[]);
	let (first, again) = assert_deterministic(&files);
	assert!(
		diagnostic_lines(&first).is_empty(),
		"unexpected diagnostics:\n{first}"
	);
	// every module published its interface
	assert!(!first.contains("interface=None"), "{first}");
	// the run for another target keeps the modules that don't read it
	assert!(again.contains("analyzed=false"), "{again}");
	assert!(again.contains("analyzed=true"), "{again}");
}

#[test]
fn the_diagnostics_of_the_pipeline_are_deterministic_with_errors() {
	let files = package(16, &[3, 9, 14]);
	let (first, _) = assert_deterministic(&files);
	assert!(first.contains("library/m03"), "{first}");
	assert!(first.contains("library/m09"), "{first}");
}

#[test]
fn a_second_target_analyzes_nothing_when_no_file_reads_it() {
	// async code (`await`, `start`) and calls of functions that never wait
	// don't read the target: only a blocking call of something that may
	// block does (review 27)
	let files: Vec<(String, String)> = [
		(
			"package.fa",
			"export = {\n\tname = \"p\"\n\tversion = \"0.1.0\"\n\ttargets = [\"bun\", \"native\"]\n}\n",
		),
		(
			"library/twice.fa",
			"export async function = (n: Integer): Integer => n * 2\n",
		),
		(
			"library/add.fa",
			"export function = (a: Integer, b: Integer): Integer => a + b\n",
		),
		(
			"application/main.fa",
			"export async function = () => {\n\tlet task = start library.twice(1)\n\tconsole.log(await task, await library.twice(2), library.add(1, 2))\n}\n",
		),
	]
	.into_iter()
	.map(|(path, text)| (path.to_string(), text.to_string()))
	.collect();
	let (first, again) = parallel(&files, 2, 0);
	assert!(
		diagnostic_lines(&first).is_empty(),
		"unexpected diagnostics:\n{first}"
	);
	assert!(!first.contains("reads_target=true"), "{first}");
	assert!(!again.contains("analyzed=true"), "{first}\n{again}");
}

#[test]
fn imported_key_loop_applications_are_checked_deterministically() {
	let files: Vec<_> = [
		("library/bump.fa", "export function = <Value, keys: Keys<Value>>(value: Value) => {\n\t@for key in keys {\n\t\tconsole.log(value[key] + 1)\n\t}\n}\n"),
		("application/main.fa", "type Mixed = { count: Integer, label: String }\nexport function = () => {\n\tlet value: Mixed = { count = 1, label = \"fruit\" }\n\tlibrary.bump<Mixed, { count, label }>(value)\n}\n"),
	].into_iter().map(|(path, source)| (path.to_string(), source.to_string())).collect();
	let (first, _) = assert_deterministic(&files);
	assert!(first.contains("TF011"), "{first}");
	assert!(first.contains("key `label`"), "{first}");
	assert!(first.contains("application/main.fa:4:"), "{first}");
}

#[test]
fn changing_a_callers_key_set_rechecks_the_unchanged_provider() {
	let mut files: Vec<_> = [
		("library/bump.fa", "export function = <Value, keys: Keys<Value>>(value: Value) => {\n\t@for key in keys {\n\t\tconsole.log(value[key] + 1)\n\t}\n}\n"),
		("application/main.fa", "type Mixed = { count: Integer, label: String }\nexport function = () => {\n\tlet value: Mixed = { count = 1, label = \"fruit\" }\n\tlibrary.bump<Mixed, { count, label }>(value)\n}\n"),
	].into_iter().map(|(path, source)| (path.to_string(), source.to_string())).collect();
	let mut session = Session::new();
	session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
	let options = pipeline::Options {
		entry: Some("application/main".to_string()),
		threads: Some(2),
		..Default::default()
	};
	let first = pipeline::analyze_package(&mut session, &options, None, &NoSink);
	assert!(first.has_errors());
	files[1].1 = files[1]
		.1
		.replace("{ count, label }>(value)", "{ count }>(value)");
	session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
	let dirty = std::iter::once("application/main".to_string()).collect();
	let options = pipeline::Options {
		dirty: Some(dirty),
		..options
	};
	let again = pipeline::analyze_package(&mut session, &options, Some(&first), &NoSink);
	assert!(!again.has_errors(), "{}", outputs(&session, &again));
}
