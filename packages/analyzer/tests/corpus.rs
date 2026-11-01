//! Corpus tests (design §6): every ```fa block of the documentation and of the
//! README is analyzed, as a standalone file and as a test file, then its
//! truncated and garbled versions. The analyzer must never panic, its
//! diagnostics must be inside the text and render, and the editor queries
//! must answer at any offset.

use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use fa_analyzer::Session;

fn markdown_files(directory: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			markdown_files(&path, out);
		} else if path.extension().is_some_and(|extension| extension == "md") {
			out.push(path);
		}
	}
}

/// Every ```fa block, with the page it comes from.
fn snippets() -> Vec<(PathBuf, String)> {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
	let mut pages = vec![root.join("README.md")];
	markdown_files(&root.join("documentation"), &mut pages);
	pages.sort();
	let mut snippets = Vec::new();
	for page in pages {
		let Ok(text) = std::fs::read_to_string(&page) else {
			continue;
		};
		let mut current: Option<String> = None;
		for line in text.lines() {
			match &mut current {
				None if line.trim() == "```fa" => current = Some(String::new()),
				Some(code) if line.trim() == "```" => {
					snippets.push((page.clone(), std::mem::take(code)));
					current = None;
				}
				Some(code) => {
					code.push_str(line);
					code.push('\n');
				}
				None => {}
			}
		}
	}
	snippets
}

/// A small deterministic random generator (xorshift), to garble the snippets.
struct Random(u64);

impl Random {
	fn below(&mut self, limit: usize) -> usize {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		if limit == 0 {
			0
		} else {
			(self.0 % limit as u64) as usize
		}
	}
}

const INSERTIONS: [&str; 16] = [
	"{", "}", "(", ")", "[", "]", "\"", "\"{", "}\"", "=>", ".", ",", "\n", "é", "..", "-1",
];

/// Truncations of a snippet, then versions with random deletions, duplications and insertions.
fn variants(code: &str, truncations: usize, garbles: usize, seed: u64) -> Vec<String> {
	let characters: Vec<char> = code.chars().collect();
	let mut variants = Vec::new();
	let step = (characters.len() / truncations.max(1)).max(1);
	for end in (0..characters.len()).step_by(step) {
		variants.push(characters[..end].iter().collect());
	}
	let mut random = Random(seed | 1);
	for _ in 0..garbles {
		let mut garbled = characters.clone();
		for _ in 0..1 + random.below(3) {
			let position = random.below(garbled.len() + 1);
			match random.below(3) {
				0 => {
					let end = (position + 1 + random.below(8)).min(garbled.len());
					garbled.drain(position.min(end)..end);
				}
				1 => {
					let end = (position + random.below(16)).min(garbled.len());
					let copy: Vec<char> = garbled[position.min(end)..end].to_vec();
					garbled.splice(position..position, copy);
				}
				_ => {
					let insertion = INSERTIONS[random.below(INSERTIONS.len())];
					garbled.splice(position..position, insertion.chars());
				}
			}
		}
		variants.push(garbled.into_iter().collect());
	}
	variants
}

/// Analyzes `code` as `module`, checks the output, and runs the queries every
/// `query_step` bytes (none when 0).
fn check(module: &str, code: &str, query_step: usize) {
	let mut session = Session::new();
	let file = session.set_source(module, Arc::from(code));
	session.analyze_file(file);
	let diagnostics = session.diagnostics(file);
	for diagnostic in diagnostics {
		let span = diagnostic.location.span;
		assert!(span.start <= span.end && span.end as usize <= code.len());
		assert!(!diagnostic.message.is_empty());
	}
	let rendered = session.render_diagnostics(file, "main.fa");
	assert_eq!(
		rendered.is_empty(),
		diagnostics
			.iter()
			.all(|d| d.severity == fa_analyzer::Severity::Hint)
	);
	if query_step > 0 {
		for offset in (0..=code.len() as u32 + 1).step_by(query_step) {
			let _ = session.hover(file, offset);
			let _ = session.definition(file, offset);
			let _ = session.completions(file, offset);
		}
	}
}

fn run(page: &Path, module: &str, code: &str, query_step: usize) {
	let result = panic::catch_unwind(AssertUnwindSafe(|| check(module, code, query_step)));
	assert!(
		result.is_ok(),
		"the analyzer panicked on a snippet of {} (as `{module}`):\n{code}",
		page.display()
	);
}

#[test]
fn every_documentation_snippet() {
	let snippets = snippets();
	assert!(
		snippets.len() > 100,
		"only {} snippets found",
		snippets.len()
	);
	let start = Instant::now();
	for (page, code) in &snippets {
		run(page, "main", code, 13);
		run(page, "main.test", code, 0);
	}
	println!(
		"{} snippets analyzed with queries in {:?}",
		snippets.len(),
		start.elapsed()
	);
}

#[test]
fn truncated_and_garbled_documentation_snippets() {
	for (index, (page, code)) in snippets().iter().enumerate() {
		for variant in variants(code, 6, 4, index as u64 + 1) {
			run(page, "main", &variant, 0);
		}
	}
}
