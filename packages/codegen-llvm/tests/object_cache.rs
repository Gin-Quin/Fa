//! The object cache of development builds per unit (`units.rs`,
//! `facts.rs`): a unit's object is reused while its sources, the interfaces
//! it reads and the whole-program facts its lowering consulted are the
//! same. An edit that changes a fact rebuilds only the units that read it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use fa_analyzer::modules::CodebaseResolver;
use fa_codegen_llvm::{Options, SplitOptions, Target};

/// The files of the package: `counts` lowers maps of `Map<String, Int32>`
/// (it reads whether something keeps a reference into their values),
/// `double` has no map, `main` is given.
fn files(main: &str) -> Vec<(String, String)> {
	vec![
		(
			"package.fa".to_string(),
			"export = { name = \"cache\" }\n".to_string(),
		),
		(
			"library/counts.fa".to_string(),
			"export function = (): Int32 => {\n\tmutable counts = Map<String, Int32>.from([], capacity = 2)\n\tcounts[\"a\"] = 1\n\tcounts.get(\"a\") ?? 0\n}\n"
				.to_string(),
		),
		(
			"library/double.fa".to_string(),
			"export function = (x: Integer): Integer => x * 2\n".to_string(),
		),
		("application/main.fa".to_string(), main.to_string()),
	]
}

/// Builds the package per unit with the object cache; the modification
/// time of each cached object, by unit.
fn build(directory: &Path, main: &str) -> BTreeMap<String, SystemTime> {
	let files = files(main);
	build_files(directory, &files, &["counts", "double", "main"])
}

fn build_files(
	directory: &Path,
	files: &[(String, String)],
	names: &[&str],
) -> BTreeMap<String, SystemTime> {
	let mut session = fa_analyzer::session::Session::new();
	session.set_target(Target::Native.analysis_target());
	session.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
	let run = fa_analyzer::pipeline::Options {
		entry: Some("application/main".to_string()),
		..fa_analyzer::pipeline::Options::default()
	};
	let cache = directory.join("objects");
	let split = SplitOptions {
		lto: false,
		cache: Some(cache.clone()),
		runtime_cache: Some(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-runtime")),
		..SplitOptions::default()
	};
	let (report, outcome) = fa_codegen_llvm::compile_package(
		&mut session,
		&run,
		Target::Native,
		&Options {
			leak_check: true,
			..Options::default()
		},
		&split,
		&directory.join("main.objects"),
		&directory.join("main"),
		&fa_codegen_llvm::Libraries::default(),
	);
	assert!(
		matches!(outcome, fa_codegen_llvm::PackageOutcome::Built(_)),
		"{outcome:?}: {:?}",
		report
			.modules
			.iter()
			.flat_map(|module| module.diagnostics.iter().map(|d| d.message.clone()))
			.collect::<Vec<_>>()
	);
	let mut times = BTreeMap::new();
	for entry in std::fs::read_dir(&cache).unwrap().flatten() {
		let name = entry.file_name().to_string_lossy().to_string();
		if !name.ends_with(".o") {
			continue;
		}
		let unit = names
			.iter()
			.copied()
			.find(|unit| name.contains(unit))
			.unwrap_or("other");
		let modified = entry.metadata().unwrap().modified().unwrap();
		// the newest object of each unit
		let newest = times.entry(unit.to_string()).or_insert(modified);
		*newest = (*newest).max(modified);
	}
	times
}

/// The units whose newest object is newer than before.
fn rebuilt(
	before: &BTreeMap<String, SystemTime>,
	after: &BTreeMap<String, SystemTime>,
) -> Vec<String> {
	after
		.iter()
		.filter(|(unit, time)| before.get(*unit) != Some(time))
		.map(|(unit, _)| unit.clone())
		.collect()
}

#[test]
fn a_changed_fact_rebuilds_only_the_units_that_read_it() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let plain =
		"export function = () => {\n\tconsole.log(library.counts(), library.double(2))\n}\n";
	let first = build(&directory, plain);
	assert_eq!(
		first.keys().cloned().collect::<Vec<_>>(),
		["counts", "double", "main"]
	);
	// nothing changed: every object from the cache
	std::thread::sleep(std::time::Duration::from_millis(20));
	let again = build(&directory, plain);
	assert!(rebuilt(&first, &again).is_empty(), "{first:?} {again:?}");
	// `main` changes without changing a fact: `main` only
	let other =
		"export function = () => {\n\tconsole.log(library.double(2), library.counts())\n}\n";
	std::thread::sleep(std::time::Duration::from_millis(20));
	let edited = build(&directory, other);
	assert_eq!(rebuilt(&again, &edited), ["main"]);
	// `main` keeps a reference into maps of `Map<String, Int32>` (a lambda
	// reads one): the fact `counts` read changes, `double` stays cached
	let referencing = "export function = () => {\n\tmutable other = Map<String, Int32>.from([], capacity = 2)\n\tother[\"x\"] = 1\n\tconsole.log(library.counts(), library.double(2), [1].map((x) => (other.get(\"x\") ?? 0) + x))\n}\n";
	std::thread::sleep(std::time::Duration::from_millis(20));
	let referenced = build(&directory, referencing);
	assert_eq!(rebuilt(&edited, &referenced), ["counts", "main"]);
	// back: `main` from the cache (its source and facts as before), `counts`
	// built again (one object per unit's key: the latest facts' one)
	std::thread::sleep(std::time::Duration::from_millis(20));
	let back = build(&directory, other);
	assert_eq!(rebuilt(&referenced, &back), ["counts"]);
}

#[test]
fn a_remote_escape_invalidates_cached_region_builders() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-region-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"regionCache\" }"),
		("library/model.fa", "export namespace = { type Tree = Union { Leaf = { text: String }, Branch = { left: Tree, right: Tree } } }"),
		("library/build.fa", "export function = (depth: Integer): model.Tree => if depth == 0 { model.Tree.Leaf { text = \"leaf\" } } else { model.Tree.Branch { left = build(depth - 1), right = build(depth - 1) } }"),
		("library/count.fa", "export function = (tree: model.Tree): Integer => when tree is {\nmodel.Tree.Leaf => 1\nmodel.Tree.Branch as { left, right } => count(left) + count(right)\n}"),
		("library/double.fa", "export function = (value: Integer): Integer => value * 2"),
		("application/main.fa", "export function = () => {\nconsole.log(library.count(library.build(3)))\nconsole.log(library.double(2))\n}"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	let names = ["model", "build", "count", "double", "main", "escape"];
	let first = build_files(&directory, &files, &names);
	let run = |expected| {
		let output = std::process::Command::new(directory.join("main"))
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
		assert!(output.stderr.is_empty(), "{output:?}");
	};
	run("8\n4\n");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let again = build_files(&directory, &files, &names);
	assert!(rebuilt(&first, &again).is_empty());
	files.push((
		"library/escape.fa".into(),
		"export function = (tree: model.Tree): Integer => {\nlet box = { tree }\nwhen box.tree is {\nmodel.Tree.Leaf => 0\nmodel.Tree.Branch => 1\n}\n}".into(),
	));
	files.iter_mut().find(|(path, _)| path == "application/main.fa").unwrap().1 = "export function = () => {\nconsole.log(library.count(library.build(3)))\nconsole.log(library.double(2))\nconsole.log(library.escape(library.build(0)))\n}".into();
	std::thread::sleep(std::time::Duration::from_millis(20));
	let escaped = build_files(&directory, &files, &names);
	let changed = rebuilt(&again, &escaped);
	assert!(changed.iter().any(|unit| unit == "build"), "{changed:?}");
	assert!(!changed.iter().any(|unit| unit == "double"), "{changed:?}");
	run("8\n4\n0\n");
}

#[test]
fn a_remote_array_escape_invalidates_cached_child_region_builders() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-child-region-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"childRegionCache\" }"),
		("library/model.fa", "export namespace = { type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } } }"),
		("library/build.fa", "export function = (depth: Integer): model.Tree => if depth == 0 { model.Tree.Leaf { value = 1 } } else {\nmutable items: Array<model.Tree> = []\nitems.push(build(depth - 1))\nitems.push(build(depth - 1))\nmodel.Tree.List { items }\n}"),
		("library/count.fa", "export function = (tree: model.Tree): Integer => when tree is {\nmodel.Tree.Leaf => 1\nmodel.Tree.List as { items } => {\nmutable total = 0\nfor item in items { total += count(item) }\ntotal\n}\n}"),
		("library/double.fa", "export function = (value: Integer): Integer => value * 2"),
		("application/main.fa", "export function = () => {\nlet roots: Array<library.model.Tree> = [library.build(3), library.build(1)]\nmutable total = 0\nfor root in roots { total += library.count(root) }\nconsole.log(total)\nconsole.log(library.double(2))\n}"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	let names = ["model", "build", "count", "double", "main", "escape"];
	let first = build_files(&directory, &files, &names);
	// An exported recursive builder keeps its arena body as a private
	// symbol in the cached object. This verifies physical activation, not
	// merely the consultation of a positive frontend claim.
	let builder_has_arena = |times: &BTreeMap<String, SystemTime>| {
		std::fs::read_dir(directory.join("objects"))
			.unwrap()
			.flatten()
			.any(|entry| {
				let name = entry.file_name().to_string_lossy().to_string();
				name.ends_with(".o")
					&& name.contains("build")
					&& entry.metadata().unwrap().modified().unwrap() == times["build"]
					&& {
						let object = std::fs::read(entry.path()).unwrap();
						object
							.windows(b".arena".len())
							.any(|window| window == b".arena")
							&& object
								.windows(b"fa_array_init_region_local".len())
								.any(|window| window == b"fa_array_init_region_local")
					}
			})
	};
	assert!(builder_has_arena(&first));
	let run = |expected| {
		let output = std::process::Command::new(directory.join("main"))
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
		assert!(output.stderr.is_empty(), "{output:?}");
	};
	run("10\n4\n");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let again = build_files(&directory, &files, &names);
	assert!(rebuilt(&first, &again).is_empty());
	files.push((
		"library/escape.fa".into(),
		"export function = (): Array<model.Tree> => [build(0)]".into(),
	));
	let main = &mut files
		.iter_mut()
		.find(|(path, _)| path == "application/main.fa")
		.unwrap()
		.1;
	let end = main.rfind("\n}").unwrap();
	main.insert_str(
		end,
		"\nlet escaped = library.escape()\nconsole.log(escaped.length)",
	);
	let escaped = build_files(&directory, &files, &names);
	let changed = rebuilt(&again, &escaped);
	assert!(changed.iter().any(|unit| unit == "build"), "{changed:?}");
	assert!(!changed.iter().any(|unit| unit == "double"), "{changed:?}");
	assert!(!builder_has_arena(&escaped));
	run("10\n4\n1\n");
}

#[test]
fn a_private_region_type_is_known_by_the_global_box_helpers() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory =
		PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-private-region-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"privateRegion\" }"),
		("library/work.fa", "type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } }\nfunction build = (depth: Integer): Tree => if depth == 0 { Tree.Leaf { value = 1 } } else {\nmutable items: Array<Tree> = []\nitems.push(build(depth - 1))\nitems.push(build(depth - 1))\nTree.List { items }\n}\nfunction count = (tree: Tree): Integer => when tree is {\nTree.Leaf => 1\nTree.List as { items } => {\nmutable total = 0\nfor item in items { total += count(item) }\ntotal\n}\n}\nexport function = (depth: Integer): Integer => count(build(depth))\n"),
		("application/main.fa", "export function = () => console.log(library.work(3))"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	build_files(&directory, &files, &["work", "main"]);
	let active = std::fs::read_dir(directory.join("objects"))
		.unwrap()
		.flatten()
		.any(|entry| {
			let name = entry.file_name().to_string_lossy().to_string();
			name.ends_with(".o")
				&& name.contains("work")
				&& std::fs::read(entry.path())
					.unwrap()
					.windows(b".arena".len())
					.any(|window| window == b".arena")
		});
	assert!(active, "the private builder must actually use its region");
	let output = std::process::Command::new(directory.join("main"))
		.output()
		.unwrap();
	assert!(output.status.success(), "{output:?}");
	assert_eq!(String::from_utf8_lossy(&output.stdout), "8\n");
	assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn a_remote_capacity_read_invalidates_a_consulted_fact() {
	use fa_codegen_llvm::facts::{FactLog, ProgramFacts, ROOT_REGIONS};

	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"capacityCache\" }"),
		("library/model.fa", "export namespace = { type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } } }"),
		("library/build.fa", "export function = (depth: Integer): model.Tree => { if depth == 0 { return model.Tree.Leaf { value = 1 } }\nmutable items: Array<model.Tree> = []\nitems.push(build(depth - 1))\nmodel.Tree.List { items } }"),
		("library/read.fa", "export function = (tree: model.Tree): Integer => when tree is { model.Tree.Leaf => 1, model.Tree.List as { items } => items.length }"),
		("application/main.fa", "export function = () => console.log(library.read(library.build(3)))"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	let analyze = |files: &[(String, String)]| {
		let mut session = fa_analyzer::session::Session::new();
		session.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
		let program = session.analyze_package(Some("application/main"));
		assert!(program.files.iter().all(|&file| {
			session
				.diagnostics(file)
				.iter()
				.all(|diagnostic| diagnostic.severity != fa_analyzer::Severity::Error)
		}));
		let mut facts = ProgramFacts::of_unit(&session, &program.files);
		facts.finish();
		facts
	};
	let unobserved = analyze(&files);
	let key = unobserved
		.set(ROOT_REGIONS)
		.unwrap()
		.iter()
		.find(|key| key.starts_with("capacity-unobserved\u{1f}"))
		.unwrap();
	let log = FactLog::default();
	assert!(unobserved.consulted(ROOT_REGIONS, Some(&log)).contains(key));
	let cached = log.to_text();
	assert!(FactLog::still_holds(&cached, Some(&unobserved)));
	let (_, reader) = files
		.iter_mut()
		.find(|(path, _)| path == "library/read.fa")
		.unwrap();
	*reader = reader.replace("items.length", "items.capacity");
	let observed = analyze(&files);
	assert!(!FactLog::still_holds(&cached, Some(&observed)));
	let log = FactLog::default();
	assert!(!observed.consulted(ROOT_REGIONS, Some(&log)).contains(key));
	assert!(!FactLog::still_holds(&log.to_text(), Some(&unobserved)));
	// Other proof answers remain reusable: the capacity observer does not
	// change the root's ownership and does not invalidate unrelated facts.
	let log = FactLog::default();
	for key in unobserved
		.set(ROOT_REGIONS)
		.unwrap()
		.iter()
		.filter(|key| !key.starts_with("capacity-unobserved\u{1f}"))
	{
		assert!(unobserved.consulted(ROOT_REGIONS, Some(&log)).contains(key));
	}
	assert!(FactLog::still_holds(&log.to_text(), Some(&observed)));
}

#[test]
fn a_remote_observer_rebuilds_exact_array_field_reservations() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory =
		PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-array-field-capacity-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"fieldCapacityCache\" }"),
		("library/model.fa", "export namespace = { type Bucket = { items: Array<Integer> } }"),
		("library/build.fa", "export function = (count: Integer): model.Bucket => { mutable bucket = model.Bucket { items = [] }\nfor index in 0..count { bucket.items.push(index) }\nbucket }"),
		("library/read.fa", "export function = (bucket: model.Bucket): Integer => bucket.items.length"),
		("library/double.fa", "export function = (value: Integer): Integer => value * 2"),
		("application/main.fa", "export function = () => { console.log(library.read(library.build(3)))\nconsole.log(library.double(2)) }"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	let names = ["model", "build", "read", "double", "main"];
	let run = |expected| {
		let output = std::process::Command::new(directory.join("main"))
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
		assert!(output.stderr.is_empty(), "{output:?}");
	};
	let first = build_files(&directory, &files, &names);
	run("3\n4\n");
	let exact = std::fs::read_dir(directory.join("objects"))
		.unwrap()
		.flatten()
		.any(|entry| {
			let name = entry.file_name().to_string_lossy().into_owned();
			name.ends_with(".o")
				&& name.contains("build")
				&& std::fs::read(entry.path())
					.unwrap()
					.windows(b"fa_array_reserve".len())
					.any(|window| window == b"fa_array_reserve")
		});
	assert!(exact, "the builder must actually use exact reservation");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let again = build_files(&directory, &files, &names);
	assert!(rebuilt(&first, &again).is_empty());
	let (_, reader) = files
		.iter_mut()
		.find(|(path, _)| path == "library/read.fa")
		.unwrap();
	*reader = reader.replace("items.length", "items.capacity");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let observed = build_files(&directory, &files, &names);
	let changed = rebuilt(&again, &observed);
	assert!(changed.iter().any(|unit| unit == "build"), "{changed:?}");
	assert!(!changed.iter().any(|unit| unit == "double"), "{changed:?}");
	// A stale exact reservation would expose 3 through the new reader.
	run("4\n4\n");
}

#[test]
fn a_remote_capacity_read_rebuilds_compact_array_payloads() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory =
		PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-region-capacity-object-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"capacityCache\" }"),
		("library/model.fa", "export namespace = { type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } } }"),
		("library/build.fa", "export function = (depth: Integer): model.Tree => { if depth == 0 { return model.Tree.Leaf { value = 1 } }\nmutable items: Array<model.Tree> = []\nitems.push(build(depth - 1))\nitems.push(build(depth - 1))\nmodel.Tree.List { items } }"),
		("library/read.fa", "export function = (tree: model.Tree): Integer => when tree is { model.Tree.Leaf => 0, model.Tree.List as { items } => items.length }"),
		("library/double.fa", "export function = (value: Integer): Integer => value * 2"),
		("application/main.fa", "export function = () => {\nconsole.log(library.read(library.build(1)))\nconsole.log(library.double(2))\n}"),
	].into_iter().map(|(path, text)| (path.into(), text.into())).collect();
	let names = ["model", "build", "read", "double", "main"];
	let run = |expected| {
		let output = std::process::Command::new(directory.join("main"))
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
		assert!(output.stderr.is_empty(), "{output:?}");
	};
	let first = build_files(&directory, &files, &names);
	run("2\n4\n");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let again = build_files(&directory, &files, &names);
	assert!(rebuilt(&first, &again).is_empty());
	let (_, reader) = files
		.iter_mut()
		.find(|(path, _)| path == "library/read.fa")
		.unwrap();
	*reader = reader.replace("items.length", "items.capacity");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let observed = build_files(&directory, &files, &names);
	let changed = rebuilt(&again, &observed);
	assert!(changed.iter().any(|unit| unit == "build"), "{changed:?}");
	assert!(!changed.iter().any(|unit| unit == "double"), "{changed:?}");
	// The reader now observes the growth capacity of the builder's original
	// two-element buffer. Reusing its old, compacted payload would print 2.
	run("4\n4\n");
}

#[test]
fn remote_array_and_opaque_uses_invalidate_exact_extents() {
	if !fa_codegen_llvm::llvm_available() {
		eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
		return;
	}
	let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-array-extent-cache");
	let _ = std::fs::remove_dir_all(&directory);
	std::fs::create_dir_all(&directory).unwrap();
	let mut files: Vec<(String, String)> = [
		("package.fa", "export = { name = \"extentCache\" }"),
		("library/model.fa", "export namespace = { type Cell = { value: Float } }"),
		("library/shape.fa", "function extent = (items: Array<model.Cell>): Integer => items.length\nexport function = (): Integer => { mutable items: Array<model.Cell> = [{ value = 1.0 }, { value = 2.0 }, { value = 3.0 }, { value = 4.0 }, { value = 5.0 }]\nfor item in items { item.value += 0.25 }\nitems.reserve(100)\nextent(items) }"),
		("library/probe.fa", "export function = (): Integer => 0"),
		("application/main.fa", "export function = () => console.log(library.shape() + library.probe())"),
	].into_iter().map(|(path, source)| (path.into(), source.into())).collect();
	let names = ["model", "shape", "probe", "main"];
	let run = |expected: &str| {
		let output = std::process::Command::new(directory.join("main"))
			.output()
			.unwrap();
		assert!(output.status.success(), "{output:?}");
		assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
		assert!(output.stderr.is_empty(), "{output:?}");
	};
	let first = build_files(&directory, &files, &names);
	run("5\n");
	let consulted = std::fs::read_dir(directory.join("objects"))
		.unwrap()
		.flatten()
		.filter_map(|entry| {
			let name = entry.file_name().to_string_lossy().into_owned();
			(name.contains("shape") && name.ends_with(".facts"))
				.then(|| std::fs::read_to_string(entry.path()).unwrap())
		})
		.collect::<String>();
	assert!(
		consulted.contains("proofs.claims\topaque\t0\n"),
		"{consulted}"
	);
	assert!(
		consulted
			.lines()
			.any(|line| line.starts_with("proofs.claims\ti\u{1f}array:") && line.ends_with("\t0")),
		"{consulted}"
	);
	let again = build_files(&directory, &files, &names);
	assert!(rebuilt(&first, &again).is_empty());
	let mut baseline = again;
	for (probe, expected) in [
		(
			"export function = (): Integer => { let items: Array<model.Cell> = []\nitems.length }",
			"5\n",
		),
		(
			"export function = (): Integer => { let parsed: Integer = std.json.Json.parse(\"1\") catch _error => 0\nparsed }",
			"6\n",
		),
	] {
		std::thread::sleep(std::time::Duration::from_millis(20));
		files
			.iter_mut()
			.find(|(path, _)| path == "library/probe.fa")
			.unwrap()
			.1 = probe.into();
		let changed = build_files(&directory, &files, &names);
		assert!(
			rebuilt(&baseline, &changed).contains(&"shape".to_string()),
			"{changed:?}"
		);
		run(expected);
		std::thread::sleep(std::time::Duration::from_millis(20));
		files
			.iter_mut()
			.find(|(path, _)| path == "library/probe.fa")
			.unwrap()
			.1 = "export function = (): Integer => 0".into();
		let restored = build_files(&directory, &files, &names);
		assert!(rebuilt(&changed, &restored).contains(&"shape".to_string()));
		run("5\n");
		baseline = restored;
	}
}
