//! Parse every ```fa block of the documentation (`documentation/**/*.md` and `README.md`).
//!
//! - `documentation_snippets_parse`: every block parses without error, except the examples of
//!   errors listed in `EXPECTED_ERRORS`, which must report one.
//! - `truncated_documentation_snippets_never_panic`: truncated and garbled versions of every
//!   block are parsed, and the parser must neither panic nor hang. The `#[ignore]`d
//!   `garbled_documentation_snippets_never_panic` does the same with many more variants.
//!
//! The documentation folder can be changed with the `FA_DOCUMENTATION` environment variable.
//! `cargo test -p fa-parser --test doc_snippets -- --nocapture` prints the failing blocks.

use std::panic;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use fa_parser::{TypedSyntaxTree, parse};

/// The examples of syntax errors of the documentation: a file, and a line of the block.
const EXPECTED_ERRORS: [(&str, &str); 3] = [
	("types/declaring-types.md", "mutable type MyType"),
	("values/numbers.md", "let a = fifty%"),
	("values/tuples.md", "let dotted = pair.0"),
];

#[path = "support/snippets.rs"]
mod support;

use support::{Snippet, snippets};

fn is_expected_error(snippet: &Snippet) -> bool {
	EXPECTED_ERRORS.iter().any(|(file, line)| {
		snippet.file.ends_with(file) && snippet.code.lines().any(|code| code.contains(line))
	})
}

/// Parse `source` on another thread, and return the error messages, or a description of the
/// panic or of the timeout.
fn parse_safely(source: String) -> Result<Vec<String>, String> {
	let (sender, receiver) = mpsc::channel();
	thread::spawn(move || {
		let result = panic::catch_unwind(|| {
			let tree = parse(&source);
			tree.diagnostics
				.iter()
				.filter(|diagnostic| diagnostic.is_error())
				.map(|diagnostic| diagnostic.display(&tree.source_map).to_string())
				.collect::<Vec<_>>()
		});
		let _ = sender.send(result.map_err(|error| {
			error
				.downcast_ref::<String>()
				.cloned()
				.or_else(|| error.downcast_ref::<&str>().map(|text| text.to_string()))
				.unwrap_or_else(|| "panic".to_string())
		}));
	});
	match receiver.recv_timeout(Duration::from_secs(10)) {
		Ok(result) => result.map_err(|panic| format!("panicked: {panic}")),
		Err(_) => Err("did not finish in 10 seconds".to_string()),
	}
}

#[test]
fn documentation_snippets_parse() {
	let snippets = snippets();
	if snippets.is_empty() {
		eprintln!("no documentation found: skipped");
		return;
	}

	let mut failures = Vec::new();
	let mut expected = 0;
	for snippet in &snippets {
		let result = parse_safely(snippet.code.clone());
		match (result, is_expected_error(snippet)) {
			(Err(problem), _) => failures.push(format!("{}: {problem}", snippet.location())),
			(Ok(errors), false) if !errors.is_empty() => failures.push(format!(
				"{}:\n{}\n  {}",
				snippet.location(),
				snippet.code,
				errors.join("\n  ")
			)),
			(Ok(errors), true) if errors.is_empty() => failures.push(format!(
				"{}: an example of error, but the parser reports nothing",
				snippet.location()
			)),
			(Ok(_), true) => expected += 1,
			(Ok(_), false) => {}
		}
	}

	println!(
		"{} blocks, {} parse, {expected} examples of errors reported",
		snippets.len(),
		snippets.len() - failures.len() - expected
	);
	assert!(
		failures.is_empty(),
		"{} of {} blocks fail:\n\n{}",
		failures.len(),
		snippets.len(),
		failures.join("\n\n")
	);
}

/// A small deterministic random generator (xorshift), to garble the snippets.
struct Random(u64);

impl Random {
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}

	fn below(&mut self, limit: usize) -> usize {
		if limit == 0 {
			0
		} else {
			(self.next() % limit as u64) as usize
		}
	}
}

const INSERTIONS: [&str; 24] = [
	"{", "}", "(", ")", "[", "]", "\"", "\"{", "}\"", "--", "---", "=>", ".", ",", "\n", "@", "#",
	"é", "\\", "{{", "..", "...", "-1", "!",
];

/// Variants of a snippet: truncations, then random deletions, duplications and insertions.
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

fn assert_variants_never_panic(truncations: usize, garbles: usize) {
	panic::set_hook(Box::new(|_| {}));
	let snippets = snippets();
	let mut failures = Vec::new();
	let mut count = 0;
	for (index, snippet) in snippets.iter().enumerate() {
		// one thread per snippet, to catch a hang without waiting for each variant
		let sources = variants(&snippet.code, truncations, garbles, index as u64 + 1);
		count += sources.len();
		let result = parse_safely_all(sources);
		if let Err((source, problem)) = result {
			failures.push(format!("{} {problem} on:\n{source}", snippet.location()));
		}
	}
	let _ = panic::take_hook();
	println!("{count} variants parsed");
	assert!(
		failures.is_empty(),
		"{} snippets make the parser fail:\n\n{}",
		failures.len(),
		failures.join("\n\n")
	);
}

/// Parse every source on one thread; return the first one that panics or hangs.
fn parse_safely_all(sources: Vec<String>) -> Result<(), (String, String)> {
	let (sender, receiver) = mpsc::channel();
	let sources_for_thread = sources.clone();
	thread::spawn(move || {
		for (index, source) in sources_for_thread.into_iter().enumerate() {
			let _ = sender.send(index);
			let result = panic::catch_unwind(|| {
				parse(&source);
			});
			if result.is_err() {
				let _ = sender.send(usize::MAX);
				return;
			}
		}
		let _ = sender.send(usize::MAX - 1);
	});
	let mut current = 0;
	loop {
		match receiver.recv_timeout(Duration::from_secs(10)) {
			Ok(index) if index == usize::MAX - 1 => return Ok(()),
			Ok(usize::MAX) => return Err((sources[current].clone(), "panicked".to_string())),
			Ok(index) => current = index,
			Err(_) => return Err((sources[current].clone(), "hung".to_string())),
		}
	}
}

/// The nodes of the signature of a lambda, like `(a: Integer): Integer throws E`: they are parsed
/// before the `=>`, then copied into `FunctionSignature`, so they are not reachable from the root.
const SIGNATURE_NODES: [&str; 8] = [
	"Parameters",
	"Group",
	"EmptyGroup",
	"Tuple",
	"Assignment",
	"Throws",
	"Requires",
	"Effect",
];

/// Check the shape of the tree of a snippet: every node is reached at most once from the root
/// through `Node::children` (the tree shares no node), its span lies in the span of its parent,
/// the only nodes not reached are the signatures of lambdas, and `Node::kind_name` is the name of
/// the variant. Return the problems found.
fn check_tree_shape(tree: &TypedSyntaxTree) -> Vec<String> {
	let mut problems = Vec::new();
	let mut reached = vec![false; tree.nodes.len()];
	reached[tree.root] = true;
	let mut stack = vec![tree.root];
	while let Some(index) = stack.pop() {
		let span = tree.span(index);
		let kind = tree.at(index).kind_name();
		for child in tree.at(index).children() {
			let child_kind = tree.at(child).kind_name();
			if reached[child] {
				problems.push(format!(
					"the {child_kind} `{}` is reached twice, the second time from the {kind} `{}`",
					tree.text(child),
					tree.text(index)
				));
				continue;
			}
			let child_span = tree.span(child);
			if child_span.start < span.start || child_span.end > span.end {
				problems.push(format!(
					"the span of the {child_kind} `{}` is out of the span of its parent, the {kind} `{}`",
					tree.text(child),
					tree.text(index)
				));
			}
			reached[child] = true;
			stack.push(child);
		}
	}
	for (index, reached) in reached.iter().enumerate() {
		let kind = tree.at(index).kind_name();
		let debug = format!("{:?}", tree.at(index));
		if !debug.starts_with(kind) || debug[kind.len()..].starts_with(char::is_alphanumeric) {
			problems.push(format!("`kind_name` is {kind} for {debug}"));
		}
		if !reached && !SIGNATURE_NODES.contains(&kind) {
			problems.push(format!(
				"the {kind} `{}` is not reachable from the root",
				tree.text(index)
			));
		}
	}
	problems
}

#[test]
fn documentation_trees_are_well_formed() {
	let mut failures = Vec::new();
	for snippet in snippets() {
		let tree = parse(&snippet.code);
		if tree.has_errors() {
			continue;
		}
		for problem in check_tree_shape(&tree) {
			failures.push(format!("{}: {problem}", snippet.location()));
		}
	}
	assert!(
		failures.is_empty(),
		"{} blocks have a malformed tree:\n\n{}",
		failures.len(),
		failures.join("\n")
	);
}

#[test]
fn truncated_documentation_snippets_never_panic() {
	assert_variants_never_panic(12, 12);
}

#[test]
#[ignore]
fn garbled_documentation_snippets_never_panic() {
	assert_variants_never_panic(usize::MAX, 400);
}
