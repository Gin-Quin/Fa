//! The memory of a long editing session: thousands of edits of a document
//! must not grow the analyzer's memory without bound (the session's stores
//! are append-only, and rebuilt from the open documents when they have
//! grown too much).
//!
//! The long sessions are ignored by default; run them in release mode:
//!
//! ```sh
//! cargo test --release -p fa-language-server --test memory -- --ignored --nocapture
//! ```

use std::sync::{Arc, Mutex};

use fa_language_server::{AnalysisBackend, AnalyzerBackend};
use tower_lsp::lsp_types::Url;

/// The tests of this file measure the memory of the whole process: one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// A field of `/proc/self/status` in kB (`VmRSS`, `VmHWM`), when there is one (Linux).
fn status_kb(field: &str) -> Option<u64> {
	let status = std::fs::read_to_string("/proc/self/status").ok()?;
	let line = status.lines().find(|line| line.starts_with(field))?;
	line.split_whitespace().nth(1)?.parse().ok()
}

/// About `lines` lines of typical code: the benchmark programs, each in its
/// own scope, with a compile-time value.
fn typical_code(lines: usize) -> String {
	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks");
	let mut names: Vec<_> = std::fs::read_dir(&root)
		.map(|entries| entries.flatten().map(|entry| entry.path()).collect())
		.unwrap_or_default();
	names.sort();
	let mut programs = String::new();
	for directory in names {
		if let Ok(text) = std::fs::read_to_string(directory.join("main.fa")) {
			programs += &format!("do {{\n{text}\n}}\n");
		}
	}
	let mut text = String::from(
		"function table = (size: Integer): Array(Integer) => {\n\
		 \tmutable items: Array(Integer) = []\n\
		 \tfor index in 0..size {\n\
		 \t\titems.push(index * 2)\n\
		 \t}\n\
		 \titems\n\
		 }\n",
	);
	while text.lines().count() < lines {
		text += &programs;
	}
	text
}

/// The text of the document after edit `index`: the base text with a few
/// lines that change at every edit (new literal types, new names, a new
/// compile-time value).
fn edited(base: &str, index: usize) -> String {
	format!(
		"{base}let edit{index} = {index}\n\
		 let label = \"version {index}\"\n\
		 static values{} = @table({})\n\
		 console.log(edit{index}, label, values{}.length)\n",
		index % 3,
		2000 + index % 5,
		index % 3,
	)
}

struct Measure {
	/// The resident memory after each tenth of the edits, in MB.
	samples: Vec<u64>,
	/// The peak resident memory of the process, in MB.
	peak: u64,
}

/// Edits a document `edits` times (each edit analyzed, then its diagnostics
/// read, like the language server does after the debounce): a standalone
/// document, or the main file of a package.
fn edit_session(lines: usize, edits: usize, package: bool) -> (AnalyzerBackend, Measure) {
	let base = typical_code(lines);
	let directory = tempfile::TempDir::new().unwrap();
	let uri = if package {
		let root = directory.path();
		for (path, text) in [
			("package.fa", "export = { name = \"p\" }\n"),
			("library/answer.fa", "export function = (): Integer => 42\n"),
			("application/main.fa", "export function = () => {}\n"),
		] {
			let path = root.join(path);
			std::fs::create_dir_all(path.parent().unwrap()).unwrap();
			std::fs::write(path, text).unwrap();
		}
		Url::from_file_path(root.join("library/answer.fa")).unwrap()
	} else {
		Url::parse("file:///project/main.fa").unwrap()
	};
	let mut backend = AnalyzerBackend::new();
	let mut samples = Vec::new();
	for index in 0..edits {
		backend.set_document(&uri, Arc::from(edited(&base, index).as_str()));
		std::hint::black_box(backend.diagnostics(&uri).len());
		if (index + 1) % (edits / 10).max(1) == 0 {
			samples.push(status_kb("VmRSS").unwrap_or(0) / 1024);
		}
	}
	let peak = status_kb("VmHWM").unwrap_or(0) / 1024;
	(backend, Measure { samples, peak })
}

/// The memory of the session stays bounded: the file versions are forgotten
/// and the session is rebuilt, so the last edits use no more memory than the
/// first ones.
#[test]
fn a_short_editing_session_keeps_its_memory_bounded() {
	let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
	let (backend, measure) = edit_session(300, 200, false);
	eprintln!(
		"200 edits of 300 lines: {measure:?}",
		measure = measure.samples
	);
	let session = backend.session();
	assert!(
		session.comptime_entries() < 100,
		"the compile-time values of the old versions are dropped: {}",
		session.comptime_entries()
	);
	if status_kb("VmHWM").is_some() {
		assert!(measure.peak < 512, "peak {} MB", measure.peak);
	}
}

/// 10 000 edits of a 2 000-line document stay below 512 MB.
#[test]
#[ignore]
fn ten_thousand_edits_stay_below_512_mb() {
	let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
	let (_backend, measure) = edit_session(2000, 10_000, false);
	eprintln!(
		"10 000 edits of 2 000 lines: RSS by tenth {:?} MB, peak {} MB",
		measure.samples, measure.peak
	);
	assert!(measure.peak < 512, "peak {} MB", measure.peak);
}

/// 10 000 edits of a 2 000-line file of a package stay below 512 MB.
#[test]
#[ignore]
fn ten_thousand_edits_of_a_package_file_stay_below_512_mb() {
	let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
	let (_backend, measure) = edit_session(2000, 10_000, true);
	eprintln!(
		"10 000 edits of a 2 000-line package file: RSS by tenth {:?} MB, peak {} MB",
		measure.samples, measure.peak
	);
	assert!(measure.peak < 512, "peak {} MB", measure.peak);
}
