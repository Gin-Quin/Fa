//! `fa migrate-generics [--write] [--verify] [--rust] [--verbose] <files or folders>`: a hidden
//! command that moves the parenthesized generic lists to angle brackets (decision "Generics
//! with angle brackets", `design/decisions.md`), with `fa_analyzer::migrate`: the analysis tells
//! the applications (`Array(Integer)`, `sum(Float)`) from the calls, which keep their
//! parentheses. Without `--write`, nothing is written: the files that would change are listed.
//!
//! A file is analyzed in its program: its package, or the files next to it. A file of the
//! analyzer's cases with `-- file:` sections is a program of its own, each section analyzed as
//! a file (like `packages/analyzer/tests/cases.rs`). With `--verify`, every migrated file is
//! analyzed again, and its diagnostics must be the same, at the same places.
//!
//! With `--rust`, the string literals of the `.rs` files are read as Fa code: a literal that
//! parses without errors is analyzed alone and migrated, its escapes kept.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fa_analyzer::ids::FileId;
use fa_analyzer::migrate::{Migration, angle_bracket_generics};
use fa_analyzer::{CodebaseResolver, Session};
use fa_codebase_parser::SKIPPED_FOLDERS;

use crate::USAGE;

#[derive(Default)]
struct Options {
	write: bool,
	verify: bool,
	rust: bool,
	verbose: bool,
}

#[derive(Default)]
struct Totals {
	files: usize,
	changed_files: usize,
	lists: usize,
	kept_calls: usize,
	kept_declarations: usize,
	verified: usize,
	mismatches: usize,
	failures: usize,
}

pub(crate) fn command(arguments: &[OsString]) -> i32 {
	let mut options = Options::default();
	let mut paths = Vec::new();
	for argument in arguments {
		match argument.to_str() {
			Some("--write") => options.write = true,
			Some("--verify") => options.verify = true,
			Some("--rust") => options.rust = true,
			Some("--verbose") => options.verbose = true,
			Some(option) if option.starts_with("--") => {
				eprintln!("fa migrate-generics: unknown option `{option}`");
				return USAGE;
			}
			_ => paths.push(PathBuf::from(argument)),
		}
	}
	if paths.is_empty() {
		eprintln!("fa migrate-generics: give the files or folders to migrate");
		return USAGE;
	}
	let extension = if options.rust { "rs" } else { "fa" };
	let mut files = Vec::new();
	for path in &paths {
		if path.is_dir() {
			collect_files(path, extension, &mut files);
		} else {
			files.push(path.clone());
		}
	}
	files.sort();

	let mut totals = Totals::default();
	for file in &files {
		totals.files += 1;
		let Ok(text) = fs::read_to_string(file) else {
			eprintln!("fa migrate-generics: can't read `{}`", file.display());
			totals.failures += 1;
			continue;
		};
		let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			if options.rust {
				migrate_rust(&text, &options)
			} else if has_sections(&text) {
				migrate_sections(&text, &options)
			} else {
				migrate_file(file, &text, &options)
			}
		}));
		let Ok(outcome) = outcome else {
			eprintln!("{}: the analyzer panicked", file.display());
			totals.failures += 1;
			continue;
		};
		totals.lists += outcome.lists;
		totals.kept_calls += outcome.kept_calls.len();
		if options.verbose {
			for call in &outcome.kept_calls {
				println!("{}: kept the call `{call}`", file.display());
			}
		}
		for declaration in &outcome.kept_declarations {
			println!(
				"{}: kept the parenthesized list of `{declaration}` (an alias or an annotated function: a generic function value)",
				file.display()
			);
		}
		totals.kept_declarations += outcome.kept_declarations.len();
		if options.verify && outcome.verified {
			totals.verified += 1;
		}
		for mismatch in &outcome.mismatches {
			println!("{}: {mismatch}", file.display());
		}
		totals.mismatches += outcome.mismatches.len();
		if outcome.migrated == text {
			continue;
		}
		totals.changed_files += 1;
		if options.write {
			if let Err(error) = fs::write(file, &outcome.migrated) {
				eprintln!(
					"fa migrate-generics: can't write `{}`: {error}",
					file.display()
				);
				totals.failures += 1;
			}
		} else {
			println!("{}: {} lists", file.display(), outcome.lists);
		}
	}
	println!(
		"{} files, {} changed, {} generic lists{}, {} calls of capitalized names kept, {} generic function declarations kept{}{}",
		totals.files,
		totals.changed_files,
		totals.lists,
		if options.write { " rewritten" } else { "" },
		totals.kept_calls,
		totals.kept_declarations,
		if options.verify {
			format!(
				", {} files analyzed again: {} differences",
				totals.verified, totals.mismatches
			)
		} else {
			String::new()
		},
		if totals.failures > 0 {
			format!(", {} failures", totals.failures)
		} else {
			String::new()
		},
	);
	i32::from(totals.mismatches > 0 || totals.failures > 0)
}

/// What the migration of a file gives.
#[derive(Default)]
struct Outcome {
	migrated: String,
	lists: usize,
	kept_calls: Vec<String>,
	kept_declarations: Vec<String>,
	verified: bool,
	mismatches: Vec<String>,
}

impl Outcome {
	fn add(&mut self, source: &str, migration: &Migration) {
		self.lists += migration.lists;
		self.kept_calls.extend(
			migration
				.kept_calls
				.iter()
				.map(|span| source[span.start..span.end].replace('\n', " ")),
		);
	}
}

/// A diagnostic, compared before and after the migration: its code, its message and its start
/// (in the original text).
type Key = (&'static str, String, usize);

fn keys(session: &Session, file: FileId, migration: Option<&Migration>) -> Vec<Key> {
	let mut keys: Vec<Key> = session
		.diagnostics(file)
		.iter()
		.filter(|diagnostic| diagnostic.location.file == file)
		.map(|diagnostic| {
			let start = diagnostic.location.span.start as usize;
			let start = migration.map_or(start, |migration| migration.original_offset(start));
			(diagnostic.code.id, diagnostic.message.clone(), start)
		})
		.collect();
	keys.sort();
	keys
}

fn compare(before: &[Key], after: &[Key], label: &str, mismatches: &mut Vec<String>) {
	if before == after {
		return;
	}
	for key in before.iter().filter(|key| !after.contains(key)) {
		mismatches.push(format!(
			"{label}: only before: {} at {}: {}",
			key.0, key.2, key.1
		));
	}
	for key in after.iter().filter(|key| !before.contains(key)) {
		mismatches.push(format!(
			"{label}: only after: {} at {}: {}",
			key.0, key.2, key.1
		));
	}
}

/* ------------------------------ A file on disk ----------------------------- */

/// A file of a package or a standalone program, analyzed in its program.
fn migrate_file(path: &Path, text: &str, options: &Options) -> Outcome {
	let mut outcome = Outcome {
		migrated: text.to_string(),
		..Outcome::default()
	};
	let (mut session, module) = load(path, text);
	let Some(file) = session.loaded_module(&module) else {
		return outcome;
	};
	session.analyze_file(file);
	let migration = angle_bracket_generics(session.tree(file), session.analysis(file));
	outcome.add(text, &migration);
	if migration.is_empty() {
		return outcome;
	}
	outcome.migrated = migration.apply(text);
	if options.verify {
		let before = keys(&session, file, None);
		let again = session.set_source(&module, Arc::from(outcome.migrated.as_str()));
		session.analyze_file(again);
		let after = keys(&session, again, Some(&migration));
		compare(&before, &after, "analysis", &mut outcome.mismatches);
		outcome.verified = true;
	}
	outcome
}

/// A session holding the program of a file, and the module of the file. A file of a package is
/// analyzed with its package, any other file with the files next to it; a program that can't be
/// loaded leaves the file alone.
fn load(path: &Path, text: &str) -> (Session, String) {
	let stem = path
		.file_stem()
		.map(|stem| stem.to_string_lossy().to_string())
		.unwrap_or_default();
	let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
	let directory = absolute.parent().unwrap_or(Path::new("."));
	if let Some(package) = fa_codebase_parser::discover(directory).package
		&& let Ok(relative) = absolute.strip_prefix(&package)
		&& let Ok(project) = crate::project::locate(Some(&package))
		&& let Ok(loaded) = crate::project::load(project, "bun")
	{
		let module = relative
			.to_string_lossy()
			.replace('\\', "/")
			.trim_end_matches(".fa")
			.to_string();
		let mut session = loaded.session;
		if session.loaded_module(&module).is_none() {
			session.file_of_module(&module);
		}
		if session.loaded_module(&module).is_some() {
			return (session, module);
		}
	}
	if let Ok(project) = crate::project::locate(Some(path))
		&& let Ok(loaded) = crate::project::load(project, "bun")
		&& loaded.session.loaded_module(&stem).is_some()
	{
		return (loaded.session, stem);
	}
	let mut session = Session::new();
	session.set_source(&stem, Arc::from(text));
	(session, stem)
}

/* -------------------------- A case with sections -------------------------- */

/// Whether a file holds several files, in `-- file: path` sections.
fn has_sections(text: &str) -> bool {
	text.lines()
		.any(|line| line.trim_start().starts_with("-- file:"))
}

/// A section of a case: its path, and its range in the case's text.
struct Section {
	path: String,
	start: usize,
	end: usize,
}

/// The sections of a case (`packages/analyzer/tests/cases.rs`): the lines before the first
/// `-- file:` are `main.fa`.
fn sections(text: &str) -> Vec<Section> {
	let mut sections = Vec::new();
	let mut current = Section {
		path: "main.fa".to_string(),
		start: 0,
		end: 0,
	};
	let mut offset = 0;
	for line in text.split_inclusive('\n') {
		if let Some(path) = line.trim_start().strip_prefix("-- file:") {
			current.end = offset;
			let has_code = text[current.start..current.end].lines().any(|line| {
				let line = line.trim();
				!line.is_empty() && !line.starts_with("--")
			});
			if has_code || !sections.is_empty() {
				sections.push(current);
			}
			current = Section {
				path: path.trim().to_string(),
				start: offset + line.len(),
				end: 0,
			};
		}
		offset += line.len();
	}
	current.end = text.len();
	sections.push(current);
	sections
}

/// Analyzes the sections of a case as one program, like the cases' harness.
fn analyze_sections(texts: &[(String, String)]) -> (Session, Vec<Option<FileId>>) {
	let entry = texts
		.iter()
		.find(|(path, _)| path == "main.fa")
		.or_else(|| texts.iter().find(|(path, _)| path.ends_with(".fa")))
		.unwrap_or(&texts[0])
		.0
		.clone();
	let resolver = CodebaseResolver::from_texts(texts, Some(&entry));
	let is_package = resolver.codebase.layout == fa_codebase_parser::Layout::Package;
	let mut session = Session::new();
	session.set_resolver(Box::new(resolver));
	if is_package {
		let entry = texts
			.iter()
			.any(|(path, _)| path == "application/main.fa")
			.then_some("application/main");
		session.analyze_package(entry);
	} else {
		let module = entry.strip_suffix(".fa").unwrap_or(&entry).to_string();
		session.analyze_program(&module);
	}
	let mut files = Vec::new();
	for (path, _) in texts {
		let file = path.strip_suffix(".fa").and_then(|module| {
			let file = session
				.loaded_module(module)
				.or_else(|| session.file_of_module(module))?;
			session.analyze_file(file);
			Some(file)
		});
		files.push(file);
	}
	(session, files)
}

fn migrate_sections(text: &str, options: &Options) -> Outcome {
	let sections = sections(text);
	let texts: Vec<(String, String)> = sections
		.iter()
		.map(|section| {
			(
				section.path.clone(),
				text[section.start..section.end].to_string(),
			)
		})
		.collect();
	let (session, files) = analyze_sections(&texts);
	let mut outcome = Outcome::default();
	let mut migrations = Vec::new();
	for ((path, source), file) in texts.iter().zip(&files) {
		let migration = match file {
			Some(file) => angle_bracket_generics(session.tree(*file), session.analysis(*file)),
			None => Migration::default(),
		};
		outcome.add(source, &migration);
		migrations.push((path.clone(), migration.apply(source), migration));
	}
	// the case, with each section migrated (the `-- file:` lines are kept)
	let mut migrated = String::with_capacity(text.len());
	let mut position = 0;
	for (section, (_, section_text, _)) in sections.iter().zip(&migrations) {
		migrated.push_str(&text[position..section.start]);
		migrated.push_str(section_text);
		position = section.end;
	}
	migrated.push_str(&text[position..]);
	outcome.migrated = migrated;

	if options.verify
		&& migrations
			.iter()
			.any(|(_, _, migration)| !migration.is_empty())
	{
		let again_texts: Vec<(String, String)> = migrations
			.iter()
			.map(|(path, text, _)| (path.clone(), text.clone()))
			.collect();
		let (again, again_files) = analyze_sections(&again_texts);
		for (index, (path, _, migration)) in migrations.iter().enumerate() {
			let (Some(before), Some(after)) = (files[index], again_files[index]) else {
				continue;
			};
			compare(
				&keys(&session, before, None),
				&keys(&again, after, Some(migration)),
				path,
				&mut outcome.mismatches,
			);
		}
		outcome.verified = true;
	}
	outcome
}

/* ---------------------------- Rust string literals ---------------------------- */

/// The string literals of a Rust file holding Fa code, migrated with their escapes kept.
fn migrate_rust(text: &str, options: &Options) -> Outcome {
	let mut outcome = Outcome::default();
	let mut edits: Vec<(usize, usize, String)> = Vec::new();
	for literal in string_literals(text) {
		let tree = fa_parser::parse(&literal.value);
		if tree.has_errors() || !literal.value.contains('(') {
			continue;
		}
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(literal.value.as_str()));
		session.analyze_file(file);
		let migration = angle_bracket_generics(session.tree(file), session.analysis(file));
		if migration.is_empty() {
			continue;
		}
		outcome.add(&literal.value, &migration);
		if options.verbose {
			println!("  {:?}", literal.value);
		}
		for edit in &migration.edits {
			edits.push((
				literal.source_offset(edit.start),
				literal.source_offset(edit.end),
				edit.text.clone(),
			));
		}
	}
	edits.sort();
	let mut migrated = String::with_capacity(text.len());
	let mut position = 0;
	for (start, end, replacement) in edits {
		migrated.push_str(&text[position..start]);
		migrated.push_str(&replacement);
		position = end;
	}
	migrated.push_str(&text[position..]);
	outcome.migrated = migrated;
	outcome
}

/// A string literal of a Rust file: its value, and the offset in the file of each byte of the
/// value (the start of its escape for an escaped character).
struct Literal {
	value: String,
	offsets: Vec<usize>,
	end: usize,
}

impl Literal {
	fn source_offset(&self, index: usize) -> usize {
		self.offsets.get(index).copied().unwrap_or(self.end)
	}
}

/// The string literals of a Rust source: `"…"` with its escapes, and `r"…"`, `r#"…"#`.
/// Comments and character literals are skipped.
fn string_literals(text: &str) -> Vec<Literal> {
	let bytes = text.as_bytes();
	let mut literals = Vec::new();
	let mut index = 0;
	while index < bytes.len() {
		match bytes[index] {
			b'/' if bytes.get(index + 1) == Some(&b'/') => {
				while index < bytes.len() && bytes[index] != b'\n' {
					index += 1;
				}
			}
			b'/' if bytes.get(index + 1) == Some(&b'*') => {
				index += 2;
				while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
				{
					index += 1;
				}
				index += 2;
			}
			// a character literal or a lifetime
			b'\'' => {
				if bytes.get(index + 1) == Some(&b'\\') {
					index += 2;
					while index < bytes.len() && bytes[index] != b'\'' {
						index += 1;
					}
					index += 1;
				} else if bytes.get(index + 2) == Some(&b'\'') {
					index += 3;
				} else {
					index += 1;
				}
			}
			b'r' if (index == 0 || !is_identifier_byte(bytes[index - 1]))
				&& matches!(bytes.get(index + 1), Some(b'"' | b'#')) =>
			{
				let mut hashes = 0;
				let mut position = index + 1;
				while bytes.get(position) == Some(&b'#') {
					hashes += 1;
					position += 1;
				}
				if bytes.get(position) != Some(&b'"') {
					index += 1;
					continue;
				}
				let start = position + 1;
				let closing = format!("\"{}", "#".repeat(hashes));
				let end = text[start..]
					.find(&closing)
					.map_or(text.len(), |offset| start + offset);
				literals.push(Literal {
					value: text[start..end].to_string(),
					offsets: (start..end).collect(),
					end,
				});
				index = end + closing.len();
			}
			b'"' => {
				let mut value = String::new();
				let mut offsets = Vec::new();
				let mut position = index + 1;
				while position < bytes.len() && bytes[position] != b'"' {
					let escape_start = position;
					if bytes[position] == b'\\' {
						let escaped = bytes.get(position + 1).copied().unwrap_or(b'\\');
						position += 2;
						let character = match escaped {
							b'n' => Some('\n'),
							b't' => Some('\t'),
							b'r' => Some('\r'),
							b'0' => Some('\0'),
							b'\\' => Some('\\'),
							b'"' => Some('"'),
							b'\'' => Some('\''),
							// a line continuation: the new line and the spaces after it
							b'\n' => {
								while position < bytes.len()
									&& bytes[position].is_ascii_whitespace()
								{
									position += 1;
								}
								None
							}
							// `\x41`, `\u{…}`: kept out of the value (rare in Fa snippets)
							_ => Some('\u{FFFD}'),
						};
						if let Some(character) = character {
							for _ in 0..character.len_utf8() {
								offsets.push(escape_start);
							}
							value.push(character);
						}
					} else {
						let character = text[position..].chars().next().expect("a character");
						for byte in 0..character.len_utf8() {
							offsets.push(position + byte);
						}
						value.push(character);
						position += character.len_utf8();
					}
				}
				literals.push(Literal {
					value,
					offsets,
					end: position,
				});
				index = position + 1;
			}
			_ => index += 1,
		}
	}
	literals
}

fn is_identifier_byte(byte: u8) -> bool {
	byte.is_ascii_alphanumeric() || byte == b'_'
}

fn collect_files(directory: &Path, extension: &str, files: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if path.is_dir() {
			if !name.starts_with('.') && !SKIPPED_FOLDERS.contains(&name.as_ref()) {
				collect_files(&path, extension, files);
			}
		} else if path.extension().is_some_and(|found| found == extension) {
			files.push(path);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sections_of_a_case() {
		let text = "-- error: X\n-- file: a.fa\nexport = 1\n-- file: main.fa\nlet b = 2\n";
		let sections = sections(text);
		let paths: Vec<&str> = sections
			.iter()
			.map(|section| section.path.as_str())
			.collect();
		assert_eq!(paths, ["a.fa", "main.fa"]);
		assert_eq!(&text[sections[0].start..sections[0].end], "export = 1\n");
		assert_eq!(&text[sections[1].start..sections[1].end], "let b = 2\n");
	}

	#[test]
	fn a_case_with_sections_is_migrated_section_by_section() {
		let text = "-- file: pair.fa\nexport type(Left, Right) = { left: Left, right: Right }\n-- file: main.fa\nlet pair: @import(\"pair.fa\")(Integer, String) = { left = 1, right = \"a\" }\n";
		let options = Options {
			verify: true,
			..Options::default()
		};
		let outcome = migrate_sections(text, &options);
		assert!(
			outcome
				.migrated
				.contains("export type<Left, Right> = { left: Left, right: Right }")
		);
		assert!(outcome.migrated.starts_with("-- file: pair.fa\n"));
		assert!(outcome.mismatches.is_empty(), "{:?}", outcome.mismatches);
	}

	#[test]
	fn rust_string_literals_keep_their_escapes() {
		let text = "let a = \"type Pair(Left, Right) = {\\n\\tleft: Left\\n}\";\nlet b = r#\"let x: Array(Integer) = []\"#;\nlet c = \"f(x)\";\n";
		let outcome = migrate_rust(text, &Options::default());
		assert_eq!(
			outcome.migrated,
			"let a = \"type Pair<Left, Right> = {\\n\\tleft: Left\\n}\";\nlet b = r#\"let x: Array<Integer> = []\"#;\nlet c = \"f(x)\";\n"
		);
		assert_eq!(outcome.lists, 2);
	}
}
