//! The JavaScript runtime of Fa, embedded from `packages/codegen-js/runtime/`.
//!
//! Each runtime file is an ES2022 module (the Bun tests import them as
//! such). Inside a file, every helper starts with a marker line
//!
//! ```js
//! // @fa-helper faFloorDivide deps: DivisionByZero
//! ```
//!
//! and ends where the next marker starts (or at the end of the file). The
//! text before the first marker (the file's comment and its `import` lines)
//! belongs to no helper. A helper is one exported function or class (plus its
//! private module-level state, like a cache), and has no top-level side
//! effect, so any subset of the helpers is a valid program prefix.
//!
//! [`collect_helpers`] resolves the dependencies of a list of helpers and
//! returns their code, deduplicated, with every dependency before its
//! dependents (a class must be declared before a class extending it).

use std::collections::HashMap;
use std::sync::OnceLock;

/// The runtime files, in canonical order: helpers are emitted in this order,
/// then in their order inside the file, unless a dependency comes later.
pub const RUNTIME_FILES: &[(&str, &str)] = &[
	("stops.js", include_str!("../runtime/stops.js")),
	("errors.js", include_str!("../runtime/errors.js")),
	("integers.js", include_str!("../runtime/integers.js")),
	("floats.js", include_str!("../runtime/floats.js")),
	("strings.js", include_str!("../runtime/strings.js")),
	("collections.js", include_str!("../runtime/collections.js")),
	("equality.js", include_str!("../runtime/equality.js")),
	("format.js", include_str!("../runtime/format.js")),
	("copy.js", include_str!("../runtime/copy.js")),
	("relations.js", include_str!("../runtime/relations.js")),
	("reactive.js", include_str!("../runtime/reactive.js")),
	("foreign.js", include_str!("../runtime/foreign.js")),
	("concurrency.js", include_str!("../runtime/concurrency.js")),
	("system.js", include_str!("../runtime/system.js")),
	("net.js", include_str!("../runtime/net.js")),
	("main.js", include_str!("../runtime/main.js")),
	("tasks.js", include_str!("../runtime/tasks.js")),
	("workers.js", include_str!("../runtime/workers.js")),
];

const MARKER: &str = "// @fa-helper ";
const DEPENDENCIES: &str = " deps:";

/// One helper of the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Helper {
	/// The name the helper exports (`faFloorDivide`, `FaError`).
	pub name: &'static str,
	/// The runtime file that defines it (`integers.js`).
	pub file: &'static str,
	/// The helpers it uses, as written in its marker.
	pub dependencies: Vec<&'static str>,
	/// Its code, from the line after its marker to the next marker, with
	/// `export` declarations.
	pub code: &'static str,
}

struct Registry {
	helpers: Vec<Helper>,
	by_name: HashMap<&'static str, usize>,
}

fn registry() -> &'static Registry {
	static REGISTRY: OnceLock<Registry> = OnceLock::new();
	REGISTRY.get_or_init(|| {
		let helpers: Vec<Helper> = RUNTIME_FILES
			.iter()
			.flat_map(|(file, source)| parse_helpers(file, source))
			.collect();
		let mut by_name = HashMap::with_capacity(helpers.len());
		for (index, helper) in helpers.iter().enumerate() {
			if by_name.insert(helper.name, index).is_some() {
				panic!("the runtime helper `{}` is defined twice", helper.name);
			}
		}
		Registry { helpers, by_name }
	})
}

/// Splits a runtime file into its helpers, at the marker lines.
fn parse_helpers(file: &'static str, source: &'static str) -> Vec<Helper> {
	let mut helpers = Vec::new();
	// (name, dependencies, start of the code)
	let mut current: Option<(&'static str, Vec<&'static str>, usize)> = None;
	let mut offset = 0;
	for line in source.split_inclusive('\n') {
		if let Some(marker) = line.trim_end().strip_prefix(MARKER) {
			if let Some((name, dependencies, start)) = current.take() {
				helpers.push(Helper {
					name,
					file,
					dependencies,
					code: source[start..offset].trim_end(),
				});
			}
			let (name, dependencies) = match marker.split_once(DEPENDENCIES) {
				Some((name, dependencies)) => {
					(name.trim(), dependencies.split_whitespace().collect())
				}
				None => (marker.trim(), Vec::new()),
			};
			current = Some((name, dependencies, offset + line.len()));
		}
		offset += line.len();
	}
	if let Some((name, dependencies, start)) = current {
		helpers.push(Helper {
			name,
			file,
			dependencies,
			code: source[start..].trim_end(),
		});
	}
	helpers
}

/// Every helper of the runtime, in canonical order.
pub fn helpers() -> &'static [Helper] {
	&registry().helpers
}

/// The helper with this name.
pub fn helper(name: &str) -> Option<&'static Helper> {
	let registry = registry();
	registry
		.by_name
		.get(name)
		.map(|&index| &registry.helpers[index])
}

/// The code of a helper (with its `export`), without its dependencies.
pub fn runtime_helper(name: &str) -> Option<&'static str> {
	helper(name).map(|helper| helper.code)
}

/// The direct dependencies of a helper.
pub fn helper_dependencies(name: &str) -> Option<&'static [&'static str]> {
	helper(name).map(|helper| helper.dependencies.as_slice())
}

/// The helpers needed by `names` (the helpers themselves and their
/// dependencies, transitively), each once, every dependency before its
/// dependents, and otherwise in canonical order whatever the order of `names`.
///
/// Panics on an unknown helper name: the backend only asks for helpers it
/// knows.
pub fn resolve_helpers<I, S>(names: I) -> Vec<&'static Helper>
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	let registry = registry();
	let mut roots: Vec<usize> = names
		.into_iter()
		.map(|name| {
			let name = name.as_ref();
			*registry
				.by_name
				.get(name)
				.unwrap_or_else(|| panic!("unknown runtime helper `{name}`"))
		})
		.collect();
	roots.sort_unstable();
	roots.dedup();

	// Depth-first, post-order: a helper is emitted after its dependencies. A
	// cycle (two functions calling each other) is fine: functions are hoisted.
	let mut visited = vec![false; registry.helpers.len()];
	let mut order = Vec::new();
	for root in roots {
		visit(registry, root, &mut visited, &mut order);
	}
	order
		.into_iter()
		.map(|index| &registry.helpers[index])
		.collect()
}

fn visit(registry: &Registry, index: usize, visited: &mut [bool], order: &mut Vec<usize>) {
	if visited[index] {
		return;
	}
	visited[index] = true;
	let mut dependencies: Vec<usize> = registry.helpers[index]
		.dependencies
		.iter()
		.map(|name| {
			*registry.by_name.get(name).unwrap_or_else(|| {
				panic!(
					"the runtime helper `{}` depends on `{name}`, which doesn't exist",
					registry.helpers[index].name
				)
			})
		})
		.collect();
	dependencies.sort_unstable();
	for dependency in dependencies {
		visit(registry, dependency, visited, order);
	}
	order.push(index);
}

/// The code of the helpers needed by `names` (see [`resolve_helpers`]), as
/// one JavaScript chunk that keeps the `export`s: a valid ES module, to write
/// as the runtime module of a program (`runtime.js`), which its modules
/// import, so that its classes (`FaError`...) are shared by all of them.
pub fn collect_helpers<I, S>(names: I) -> String
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	let mut chunk = String::new();
	for helper in resolve_helpers(names) {
		chunk.push_str(helper.code);
		chunk.push_str("\n\n");
	}
	chunk.truncate(chunk.trim_end().len());
	if !chunk.is_empty() {
		chunk.push('\n');
	}
	chunk
}

/// Like [`collect_helpers`], without the `export` keywords: a chunk to inline
/// at the start of a single-module program.
pub fn collect_helpers_inline<I, S>(names: I) -> String
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	collect_helpers(names)
		.split_inclusive('\n')
		.map(|line| line.strip_prefix("export ").unwrap_or(line))
		.collect()
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use super::*;

	fn is_identifier_character(character: char) -> bool {
		character.is_ascii_alphanumeric() || character == '_' || character == '$'
	}

	/// The code without its comments (`//` to the end of the line, `/* */`).
	fn strip_comments(code: &str) -> String {
		let mut stripped = String::with_capacity(code.len());
		let mut rest = code;
		while !rest.is_empty() {
			if let Some(after) = rest.strip_prefix("/*") {
				rest = after.split_once("*/").map_or("", |(_, after)| after);
			} else if let Some(after) = rest.strip_prefix("//") {
				rest = after.split_once('\n').map_or("", |(_, after)| after);
				stripped.push('\n');
			} else {
				let character = rest.chars().next().unwrap();
				stripped.push(character);
				rest = &rest[character.len_utf8()..];
			}
		}
		stripped
	}

	/// Whether `code` mentions the identifier `name` as a whole word.
	fn mentions(code: &str, name: &str) -> bool {
		code.match_indices(name).any(|(start, _)| {
			let before = code[..start].chars().next_back();
			let after = code[start + name.len()..].chars().next();
			!before.is_some_and(is_identifier_character)
				&& !after.is_some_and(is_identifier_character)
		})
	}

	#[test]
	fn every_runtime_file_is_embedded() {
		let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/runtime");
		let mut on_disk: Vec<String> = std::fs::read_dir(directory)
			.unwrap()
			.map(|entry| entry.unwrap().file_name().into_string().unwrap())
			.filter(|name| name.ends_with(".js") && !name.ends_with(".test.js"))
			.collect();
		on_disk.sort();
		let mut embedded: Vec<String> = RUNTIME_FILES
			.iter()
			.map(|(file, _)| file.to_string())
			.collect();
		embedded.sort();
		assert_eq!(on_disk, embedded);
	}

	#[test]
	fn helpers_are_parsed_from_the_markers() {
		assert!(helpers().len() > 100, "only {} helpers", helpers().len());
		let floor = helper("faFloorDivide").unwrap();
		assert_eq!(floor.file, "integers.js");
		assert_eq!(
			floor.dependencies,
			vec!["DivisionByZero", "faFloorQuotient"]
		);
		assert!(floor.code.contains("export function faFloorDivide(a, b) {"));
		assert!(floor.code.ends_with('}'));
		assert!(!floor.code.contains("@fa-helper"));
		assert_eq!(helper_dependencies("faWrap"), Some(&[][..]));
		assert_eq!(runtime_helper("nothing"), None);
	}

	#[test]
	fn each_helper_exports_exactly_itself() {
		for helper in helpers() {
			let exports: Vec<&str> = helper
				.code
				.lines()
				.filter_map(|line| line.strip_prefix("export "))
				.collect();
			assert_eq!(
				exports.len(),
				1,
				"`{}` must export one declaration: {exports:?}",
				helper.name
			);
			let declared = exports[0]
				.strip_prefix("function ")
				.or_else(|| exports[0].strip_prefix("async function "))
				.or_else(|| exports[0].strip_prefix("class "))
				.or_else(|| exports[0].strip_prefix("const "))
				.unwrap_or_else(|| panic!("`{}` exports `{}`", helper.name, exports[0]));
			let declared: String = declared
				.chars()
				.take_while(|&c| is_identifier_character(c))
				.collect();
			assert_eq!(declared, helper.name);
		}
	}

	#[test]
	fn dependencies_exist_and_are_not_the_helper_itself() {
		for helper in helpers() {
			let mut seen = HashSet::new();
			for dependency in &helper.dependencies {
				assert!(
					self::helper(dependency).is_some(),
					"`{}` depends on unknown `{dependency}`",
					helper.name
				);
				assert_ne!(
					*dependency, helper.name,
					"`{}` depends on itself",
					helper.name
				);
				assert!(
					seen.insert(dependency),
					"`{}` lists `{dependency}` twice",
					helper.name
				);
			}
		}
	}

	#[test]
	fn every_helper_used_in_code_is_a_declared_dependency() {
		let mut missing = Vec::new();
		for helper in helpers() {
			let code = strip_comments(helper.code);
			for other in helpers() {
				if other.name != helper.name
					&& mentions(&code, other.name)
					&& !helper.dependencies.contains(&other.name)
				{
					missing.push(format!("`{}` uses `{}`", helper.name, other.name));
				}
			}
		}
		assert!(missing.is_empty(), "undeclared dependencies: {missing:#?}");
	}

	#[test]
	fn every_declared_dependency_is_used() {
		let mut unused = Vec::new();
		for helper in helpers() {
			let code = strip_comments(helper.code);
			for dependency in &helper.dependencies {
				if !mentions(&code, dependency) {
					unused.push(format!("`{}` declares `{dependency}`", helper.name));
				}
			}
		}
		assert!(unused.is_empty(), "unused dependencies: {unused:#?}");
	}

	#[test]
	fn collecting_resolves_dependencies_transitively() {
		let names: Vec<&str> = resolve_helpers(["faIntegerRound"])
			.iter()
			.map(|h| h.name)
			.collect();
		for required in [
			"faIntegerRound",
			"faFloatToIntegerResult",
			"faRound",
			"ArithmeticError",
			"FaError",
			"faOverflow",
			"faStop",
			"FaStop",
		] {
			assert!(
				names.contains(&required),
				"missing `{required}` in {names:?}"
			);
		}
	}

	#[test]
	fn collecting_puts_dependencies_first_and_deduplicates() {
		let resolved = resolve_helpers([
			"IndexError",
			"faIndex",
			"IndexError",
			"FaError",
			"faNormalizeIndex",
		]);
		let names: Vec<&str> = resolved.iter().map(|h| h.name).collect();
		let unique: HashSet<&str> = names.iter().copied().collect();
		assert_eq!(unique.len(), names.len(), "duplicates in {names:?}");
		for (position, helper) in resolved.iter().enumerate() {
			for dependency in &helper.dependencies {
				let dependency_position = names.iter().position(|name| name == dependency).unwrap();
				// A cycle can only involve functions, which are hoisted; the
				// classes this test reaches have no cycle.
				if !helper.code.contains("function") || helper.code.contains(" extends ") {
					assert!(
						dependency_position < position,
						"`{dependency}` after `{}`",
						helper.name
					);
				}
			}
		}
		let position = |name: &str| names.iter().position(|n| *n == name).unwrap();
		assert!(position("FaError") < position("IndexError"));
		assert!(position("faNormalizeIndex") < position("faIndex"));
	}

	#[test]
	fn collecting_does_not_depend_on_the_order_of_the_names() {
		assert_eq!(
			collect_helpers(["faStringLength", "faFloorModulo", "faFormat"]),
			collect_helpers(["faFormat", "faFloorModulo", "faStringLength", "faFormat"])
		);
	}

	#[test]
	fn collected_chunks_are_separated_and_end_with_a_newline() {
		let chunk = collect_helpers(["faWrap", "faWrapBig"]);
		assert!(chunk.starts_with("/**"));
		assert!(chunk.ends_with("}\n"));
		assert_eq!(chunk.matches("export function").count(), 2);
		assert!(chunk.contains("}\n\n/**"));
		assert_eq!(collect_helpers(Vec::<&str>::new()), "");
	}

	#[test]
	fn inline_chunks_have_no_exports() {
		let chunk = collect_helpers_inline(["faFormat", "faRun", "IndexError"]);
		assert!(!chunk.contains("export "));
		assert!(chunk.contains("\nfunction faFormat(value) {"));
		assert!(chunk.contains("\nclass IndexError extends FaError {"));
	}

	#[test]
	fn module_state_belongs_to_its_helper() {
		let chunk = collect_helpers(["faStringLength"]);
		assert!(chunk.contains("let faSegmenterInstance;"));
		assert!(chunk.contains("export function faSegmenter()"));
	}

	#[test]
	#[should_panic(expected = "unknown runtime helper `faNothing`")]
	fn collecting_an_unknown_helper_panics() {
		collect_helpers(["faNothing"]);
	}
}
