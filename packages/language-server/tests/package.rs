//! The language server on a big package (design/parallel-pipeline.md,
//! step 7): the package is loaded with the parallel pipeline, the closed
//! files keep only their interfaces, and an edit analyzes again only the
//! files that read what changed.
//!
//! The fast test generates a package of 60 files. The ignored one measures
//! a package of 1,000 files of 500 lines with 5 open documents: its memory
//! (well under 512 MB) and the latency of an edit of a leaf module and of a
//! core module. Run it in release mode, on a package of
//! `benchmarks/compiler/generate.ts` (`FA_LS_PACKAGE`), or on one it
//! generates:
//!
//! ```sh
//! bun benchmarks/compiler/generate.ts --files 1000 --lines 500
//! FA_LS_PACKAGE=benchmarks/build/compiler/1000x500-s1 \
//!     cargo test --release -p fa-language-server --test package -- --ignored --nocapture
//! ```

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fa_language_server::{AnalysisBackend, AnalyzerBackend};
use tower_lsp::lsp_types::Url;

/// The tests of this file measure the memory of the whole process: one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// A field of `/proc/self/status` in MB (`VmRSS`, `VmHWM`), when there is one (Linux).
fn status_mb(field: &str) -> Option<u64> {
	let status = std::fs::read_to_string("/proc/self/status").ok()?;
	let line = status.lines().find(|line| line.starts_with(field))?;
	let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
	Some(kb / 1024)
}

/// Writes a package of `count` files: `library/m0000.fa` … (namespaces with
/// types and functions, each calling two earlier files), and
/// `application/main.fa` calling the last one.
fn generate(root: &Path, count: usize, blocks: usize) {
	std::fs::create_dir_all(root.join("library")).unwrap();
	std::fs::create_dir_all(root.join("application")).unwrap();
	std::fs::write(root.join("package.fa"), "export = { name = \"big\" }\n").unwrap();
	for index in 0..count {
		let mut text = String::from("export namespace = {\n");
		text.push_str("\ttype Item = {\n\t\tid: Integer\n\t\tweight: Integer\n");
		let _ = writeln!(
			text,
			"\t\tfunction score = (self): Integer => self.id * {} + self.weight\n\t}}",
			index % 7 + 2
		);
		text.push_str("\ttype Shape = Union {\n\t\tCircle = { radius: Integer }\n\t\tSquare = { side: Integer }\n\t}\n");
		let _ = writeln!(
			text,
			"\tfunction make = (seed: Integer): Item => Item {{ id = seed, weight = seed %% {} }}",
			index % 11 + 7
		);
		for block in 0..blocks {
			let _ = write!(
				text,
				"\tfunction area{block} = (shape: Shape): Integer => when shape is {{\n\t\tShape.Circle as {{ radius }} => radius * radius * 3\n\t\tShape.Square as {{ side }} => side * side\n\t}}\n\
				\tfunction compute{block} = (x: Integer): Integer => {{\n\t\tlet items = [make(x), make(x + 2)]\n\t\tmutable result = area{block}(Shape.Circle {{ radius = x %% 7 }})\n\t\tfor item in items {{\n\t\t\tresult += item.score() %% 13\n\t\t}}\n"
			);
			for used in [index / 2, index / 3]
				.into_iter()
				.filter(|&used| used < index)
			{
				let _ = writeln!(
					text,
					"\t\tresult += m{used:04}.compute{block}(x %% 100) %% 1000 + m{used:04}.make(x).score() %% 100"
				);
			}
			text.push_str("\t\tresult %% 50000\n\t}\n");
		}
		text.push_str("}\n");
		std::fs::write(root.join(format!("library/m{index:04}.fa")), text).unwrap();
	}
	std::fs::write(
		root.join("application/main.fa"),
		format!(
			"export function = (_arguments: Array<String>): Integer => {{\n\tconsole.log(library.m{:04}.compute0(1))\n\t0\n}}\n",
			count - 1
		),
	)
	.unwrap();
}

/// What a session over a package measured (printed).
#[derive(Debug)]
#[allow(dead_code)]
struct Measures {
	/// The time of the initial load (the first document opened).
	load: Duration,
	/// The resident memory after the open documents are analyzed, in MB.
	rss: u64,
	/// The peak resident memory of the process, in MB.
	peak: u64,
	/// The trees kept (the files of the open documents' units).
	kept_trees: usize,
	/// An edit of a leaf module (read by no file) that changes its interface.
	leaf_edit: Duration,
	/// An edit of the body of a core module (read by many), its interface kept.
	core_body_edit: Duration,
	/// An edit of a core module that changes its interface, and the number
	/// of open documents it republished.
	core_interface_edit: (Duration, usize),
}

/// Opens 5 documents of a package (the application, a leaf, a core module
/// and two others), then edits a leaf and a core module.
fn measure(root: &Path, leaf: &str, core: &str, others: [&str; 2]) -> (AnalyzerBackend, Measures) {
	let url = |module: &str| Url::from_file_path(root.join(format!("{module}.fa"))).unwrap();
	let text = |module: &str| std::fs::read_to_string(root.join(format!("{module}.fa"))).unwrap();
	let mut backend = AnalyzerBackend::new();
	let started = Instant::now();
	backend.set_document(
		&url("application/main"),
		Arc::from(text("application/main")),
	);
	let load = started.elapsed();
	for module in [leaf, core, others[0], others[1]] {
		backend.set_document(&url(module), Arc::from(text(module)));
		assert!(
			backend
				.diagnostics(&url(module))
				.iter()
				.all(|diagnostic| diagnostic.severity
					!= fa_language_server::backend::Severity::Error),
			"{module}: {:?}",
			backend.diagnostics(&url(module))
		);
	}
	let rss = status_mb("VmRSS").unwrap_or(0);
	let kept_trees = backend
		.package(root)
		.map_or(0, |package| package.kept_trees());
	// a leaf: a new function changes its interface, read by nobody
	let leaf_text = text(leaf).replacen(
		"export namespace = {\n",
		"export namespace = {\n\tfunction added = (): Integer => 1\n",
		1,
	);
	let started = Instant::now();
	backend.set_document(&url(leaf), Arc::from(leaf_text));
	let leaf_edit = started.elapsed();
	// a core module: a body change, then a new function (its interface)
	let core_text = text(core);
	let body = core_text.replacen("\t\tresult %% ", "\t\tresult %% 7 + result %% ", 1);
	assert_ne!(body, core_text);
	let started = Instant::now();
	backend.set_document(&url(core), Arc::from(body.as_str()));
	let core_body_edit = started.elapsed();
	let interface = body.replacen(
		"export namespace = {\n",
		"export namespace = {\n\tfunction added = (): Integer => 1\n",
		1,
	);
	let started = Instant::now();
	let republished = backend.set_document(&url(core), Arc::from(interface)).len();
	let core_interface_edit = (started.elapsed(), republished);
	let peak = status_mb("VmHWM").unwrap_or(0);
	let measures = Measures {
		load,
		rss,
		peak,
		kept_trees,
		leaf_edit,
		core_body_edit,
		core_interface_edit,
	};
	(backend, measures)
}

#[test]
fn a_package_keeps_the_trees_of_its_open_documents_only() {
	let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
	let directory = tempfile::TempDir::new().unwrap();
	let root = directory.path().canonicalize().unwrap();
	generate(&root, 60, 2);
	let (backend, measures) = measure(
		&root,
		"library/m0059",
		"library/m0001",
		["library/m0020", "library/m0040"],
	);
	eprintln!("60 files, 5 open: {measures:?}");
	// the units of the 5 open documents (no cycle: one file each)
	assert_eq!(measures.kept_trees, 5);
	// an edit of the core module's interface analyzes its readers again
	// (and republishes the open ones among them, here `m0002` and `m0003`
	// are closed): the edited document at least
	assert!(measures.core_interface_edit.1 >= 1);
	let main = Url::from_file_path(root.join("application/main.fa")).unwrap();
	assert!(backend.diagnostics(&main).is_empty());
	if let Some(peak) = status_mb("VmHWM") {
		assert!(peak < 512, "peak {peak} MB");
	}
}

/// The package of `FA_LS_PACKAGE` (from `benchmarks/compiler/generate.ts`),
/// or 1,000 generated files.
fn big_package() -> (Option<tempfile::TempDir>, PathBuf) {
	if let Some(path) = std::env::var_os("FA_LS_PACKAGE") {
		return (None, PathBuf::from(path).canonicalize().unwrap());
	}
	let directory = tempfile::TempDir::new().unwrap();
	let root = directory.path().canonicalize().unwrap();
	generate(&root, 1000, 11);
	(Some(directory), root)
}

/// 1,000 files of about 500 lines, 5 open documents: well under 512 MB.
#[test]
#[ignore]
fn a_thousand_files_with_five_open_documents_stay_well_under_512_mb() {
	let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
	let (_directory, root) = big_package();
	// the generator of the benchmarks names its files by level
	let modules: Vec<String> = std::fs::read_dir(root.join("library"))
		.unwrap()
		.flatten()
		.map(|entry| entry.file_name().to_string_lossy().to_string())
		.filter(|name| name.starts_with('m') && name.ends_with(".fa") && !name.contains(".test."))
		.map(|name| format!("library/{}", name.trim_end_matches(".fa")))
		.collect::<std::collections::BTreeSet<_>>()
		.into_iter()
		.collect();
	let leaf = modules.last().unwrap().clone();
	let core = modules[0].clone();
	let others = [
		modules[modules.len() / 3].clone(),
		modules[modules.len() * 2 / 3].clone(),
	];
	let (_backend, measures) = measure(&root, &leaf, &core, [&others[0], &others[1]]);
	eprintln!("{} files, 5 open: {measures:#?}", modules.len());
	assert!(measures.peak < 512, "peak {} MB", measures.peak);
}
