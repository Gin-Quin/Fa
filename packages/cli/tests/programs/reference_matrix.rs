//! Deterministic constrained pairwise reference programs. The oracle is derived
//! from collection stability, ownership and escape rules, independently of codegen.
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;
use std::sync::Arc;

use fa_analyzer::{Session, Severity};

use super::{Expected, execute, run};

const KINDS: [&str; 11] = [
	"Array",
	"GrowingArray",
	"Map",
	"GrowingMap",
	"Pool",
	"GrowingPool",
	"Columnar",
	"GrowingColumnar",
	"OrderedColumnar",
	"Set",
	"GrowingSet",
];
const SIZES: [u8; 6] = [11, 8, 12, 9, 2, 2];

/// kind × obtain × travel × meanwhile × object/primitive × write/read.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Row([u8; 6]);
impl Row {
	fn map(self) -> bool {
		matches!(self.0[0], 2 | 3)
	}
	fn indexed(self) -> bool {
		matches!(self.0[0], 0 | 1 | 8)
	}
	fn set(self) -> bool {
		self.0[0] >= 9
	}
	fn growing(self) -> bool {
		matches!(self.0[0], 1 | 3 | 5 | 7 | 10)
	}
	fn wrapped(self) -> bool {
		self.0[1] == 6 || matches!(self.0[0], 6..=8)
	}
	fn object(self) -> bool {
		self.0[4] == 1
	}
	fn feasible(self) -> bool {
		(self.0[1] != 0 || self.indexed() || self.map())
			&& (self.0[3] != 1 || self.indexed() || self.map())
	}
	fn pairs(self) -> Vec<(usize, u8, usize, u8)> {
		(0..6)
			.flat_map(|a| ((a + 1)..6).map(move |b| (a, self.0[a], b, self.0[b])))
			.collect()
	}
	fn rejected(self) -> Vec<&'static str> {
		let mut codes = Vec::new();
		if self.growing() && matches!(self.0[3], 2 | 5 | 6 | 8) {
			codes.extend(["CL040", "EFF011"]);
		}
		if self.0[2] >= 9 {
			codes.push("MEM004");
		}
		if self.set() && self.0[5] == 1 {
			codes.push("E-DECL-3");
		}
		let unstable_growth = matches!(self.0[0], 0 | 8 | 9);
		let invalidates = matches!(self.0[3], 2 | 4 | 5 | 6 | 7 | 8)
			|| (matches!(self.0[3], 3 | 7) && unstable_growth);
		if invalidates && (self.0[2] != 1 || self.0[5] == 1) {
			codes.extend(["REF001", "REF002"]);
		}
		codes
	}
}

fn rows(full: bool) -> Vec<Row> {
	let mut candidates = Vec::new();
	for k in 0..SIZES[0] {
		for o in 0..SIZES[1] {
			for t in 0..SIZES[2] {
				for a in 0..SIZES[3] {
					for v in 0..2 {
						for w in 0..2 {
							let row = Row([k, o, t, a, v, w]);
							if row.feasible() {
								candidates.push(row);
							}
						}
					}
				}
			}
		}
	}
	// Fixed xorshift shuffle avoids systematic bias toward the first options.
	let mut seed = 0x6a09e667f3bcc909_u64;
	for i in (1..candidates.len()).rev() {
		seed ^= seed << 13;
		seed ^= seed >> 7;
		seed ^= seed << 17;
		candidates.swap(i, seed as usize % (i + 1));
	}
	let mut uncovered: BTreeSet<_> = candidates.iter().flat_map(|row| row.pairs()).collect();
	let mut chosen = Vec::new();
	while !uncovered.is_empty() {
		let row = *candidates
			.iter()
			.max_by_key(|row| {
				row.pairs()
					.iter()
					.filter(|pair| uncovered.contains(pair))
					.count()
			})
			.unwrap();
		for pair in row.pairs() {
			uncovered.remove(&pair);
		}
		chosen.push(row);
		if !full && chosen.len() == 36 {
			break;
		}
	}
	if full {
		assert!(uncovered.is_empty());
	}
	chosen
}

fn source(row: Row, id: usize) -> (String, i32) {
	let [kind, obtain, travel, action, _, write] = row.0;
	let referent = if row.object() { "Node" } else { "Integer" };
	let element = if row.wrapped() { "Item" } else { referent };
	let collection_of = |item: &str| {
		if row.map() {
			format!("{}<String, {item}>", KINDS[kind as usize])
		} else {
			format!("{}<{item}>", KINDS[kind as usize])
		}
	};
	let collection = collection_of(element);
	let value = |n: i32| {
		let value = if row.object() {
			format!("{{ value = {n} }}")
		} else {
			n.to_string()
		};
		if row.wrapped() {
			format!("{{ payload = {value} }}")
		} else {
			value
		}
	};
	let literal = |n| {
		if row.map() {
			format!("[(\"kept\", {}), (\"other\", {})]", value(n), value(n + 1))
		} else {
			format!("[{}, {}]", value(n), value(n + 1))
		}
	};
	let project = if row.wrapped() { ".payload" } else { "" };
	let first = if row.map() {
		"values.get(\"kept\")"
	} else {
		"values.find((item) => true)"
	};
	let get = |owner: &str| format!("(try at({owner})){project}");
	let read = |name: &str| {
		if row.object() {
			format!("{name}.value")
		} else {
			name.to_string()
		}
	};
	let mut text = format!(
		r#"type Node = {{ value: Integer }}
type Item = {{ payload: {referent} }}
type Missing = Errors {{ Absent }}
type Callback = () => Integer
type Owner = {{ items: {collection}, other: {collection} }}
function at = (values: {collection}): {element} in values throws Missing => {{
 use item = {first}
 if item is none {{ throw Missing.Absent }}
 item
}}
function generic = <T>(values: {generic_collection}): T in values throws Missing => {{
 use item = {first}
 if item is none {{ throw Missing.Absent }}
 item
}}
function defaulted = (values: {collection}, item: {element} in values = try at(values)): {element} in values throws Missing => item
type Outer = {{
 type Reader = {{
  marker{id}: Boolean = false
  function first = <T>(self, values: {generic_collection}): T in values throws Missing => try generic(values)
 }}
}}
function asParameter = (values: {collection}, item: {referent} in values): Integer => {parameter_read}
function exercise = (flag: Boolean): {result_type} throws {errors} => {{
 mutable owner: Owner = {{ items = {initial}, other = {other} }}
 let initialCheck = try at(owner.items)
 function inner = (values: {collection}): {element} in values throws Missing => try at(values)
"#,
		generic_collection = collection_of("T"),
		result_type = if travel >= 9 { "Callback" } else { "Integer" },
		errors = if obtain == 0 && !matches!(travel, 5 | 6) {
			if row.map() {
				"Missing | KeyNotFound"
			} else {
				"Missing | IndexError"
			}
		} else {
			"Missing"
		},
		parameter_read = read("item"),
		initial = literal(7),
		other = literal(11)
	);
	let origin = match obtain {
		0 => {
			if row.map() {
				format!("try owner.items[\"kept\"]{project}")
			} else {
				format!("try owner.items[0]{project}")
			}
		}
		1 => {
			text.push_str(&format!(
				" use found = {}\n if found is none {{ throw Missing.Absent }}\n",
				first.replace("values", "owner.items")
			));
			format!("found{project}")
		}
		2 | 6 => get("owner.items"),
		3 => format!("(try defaulted(owner.items)){project}"),
		4 => format!("(try inner(owner.items)){project}"),
		5 => {
			text.push_str(
				" mutable reader: Outer.Reader = {}\n use methodItem = try reader.first(owner.items)\n",
			);
			format!("methodItem{project}")
		}
		7 => format!("(try generic(owner.items)){project}"),
		_ => unreachable!(),
	};
	let mut selected_other = false;
	match travel {
		2..=4 => {
			text.push_str(&format!(" mutable use r = {origin}\n"));
			let other = get("owner.other");
			match travel {
				2 => { text.push_str(&format!(" if flag {{ reuse r = {other} }}\n")); selected_other=true; },
				3 => text.push_str(&format!(" if flag {{ reuse r = {origin} }} else {{ reuse r = {other} }}\n")),
				4 => { text.push_str(&format!(" for i in 0..2 {{ if flag {{ reuse r = {other} }} }}\n")); selected_other=true; },
				_ => unreachable!(),
			}
		},
		5 => {
			text.push_str(&format!(" function conditional = (values: {collection}, flag: Boolean): {referent} in values throws Missing => {{ if flag {{ {} }} else {{ {} }} }}\n use r = try conditional(owner.items, flag)\n",get("values"),get("values")));
		},
		6 => text.push_str(&format!(" function loopResult = (values: {collection}): {referent} in values throws Missing => loop {{ break {} }}\n use r = try loopResult(owner.items)\n",get("values"))),
		_ => text.push_str(&format!(" use original = {origin}\n use r = original\n")),
	}
	if travel == 1 {
		text.push_str(&format!(" let snapshot = {}\n", read("r")));
	}
	if travel >= 8 {
		text.push_str(&format!(
			" function captured = (): Integer => {}\n",
			read("r")
		));
		match travel {
			9 => text.push_str(" let alias = captured\n"),
			10 => text.push_str(" let holder = { callback = captured }\n"),
			11 => text.push_str(" let escaped = loop { break captured }\n"),
			_ => {}
		}
	}
	let growth = if matches!(row.0[0], 0 | 1) {
		format!("values.push({})", value(15))
	} else if row.map() {
		format!("values[\"new\"] = {}", value(15))
	} else {
		format!("values.add({})", value(15))
	};
	let removal = if row.indexed() {
		"values.delete(0)".to_string()
	} else if row.map() {
		"values.delete(\"kept\")".to_string()
	} else if row.set() {
		format!("values.delete({})", value(7))
	} else {
		"use removed = try at(values)\n values.delete(removed)".to_string()
	};
	let action_text = match action {
		0 => "let ignored = values.length"
			.replace("length", if row.indexed() { "length" } else { "size" }),
		1 => format!("use changed = try at(values)\n changed = {}", value(9)),
		2 | 6 => "values.clear()".to_string(),
		3 | 7 => growth,
		4 => format!("owner.items = {}", literal(17)),
		5 | 8 => removal,
		_ => unreachable!(),
	};
	if action >= 6 {
		// A whole-owner effect deliberately exercises substituted effects.
		let throws = if action == 8 && !row.indexed() && !row.map() && !row.set() {
			" throws Missing"
		} else {
			""
		};
		let attempt = if throws.is_empty() { "" } else { "try " };
		text.push_str(&format!(" function act = (values: {collection}){throws} mutates values => {{ {action_text} }}\n {attempt}act(owner.items)\n"));
	} else {
		text.push_str(&format!(
			" {}\n",
			action_text.replace("values", "owner.items")
		));
	}
	if write == 1 {
		text.push_str(&format!(" {} += 1\n", read("r")));
	}
	let result = if travel == 9 {
		"() => alias()".to_string()
	} else if travel == 10 {
		"() => holder.callback()".to_string()
	} else if travel == 11 {
		"() => escaped()".to_string()
	} else if travel == 1 {
		"snapshot".to_string()
	} else if travel == 7 {
		"asParameter(owner.items, r)".to_string()
	} else if travel == 8 {
		"captured()".to_string()
	} else {
		read("r")
	};
	text.push_str(&format!(" {result}\n}}\n"));
	// Unique top-level declarations allow bounded batches without namespace
	// capture changing the semantics tested above.
	for name in [
		"Node",
		"Item",
		"Missing",
		"Callback",
		"Owner",
		"Outer",
		"Reader",
		"at",
		"generic",
		"defaulted",
		"asParameter",
		"exercise",
	] {
		let mut replaced = String::new();
		let mut word = String::new();
		for c in text.chars().chain(std::iter::once(' ')) {
			if c.is_ascii_alphanumeric() || c == '_' {
				word.push(c);
			} else {
				if word == name {
					replaced.push_str(&format!("{name}{id}"));
				} else {
					replaced.push_str(&word);
				}
				word.clear();
				replaced.push(c);
			}
		}
		text = replaced;
	}
	let snapshot = 7;
	let current = if selected_other {
		11
	} else if action == 1 {
		9
	} else {
		7
	};
	let expected = if travel == 1 {
		snapshot
	} else {
		current + i32::from(write)
	};
	(text, expected)
}

#[test]
fn generated_reference_matrix() {
	let full = std::env::var_os("FA_REFERENCE_MATRIX_FULL").is_some();
	let rows = rows(full);
	let scratch = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("reference-matrix");
	fs::create_dir_all(&scratch).unwrap();
	let mut accepted = Vec::new();
	let mut failures = Vec::new();
	let mut manifest = String::new();
	for (id, row) in rows.iter().enumerate() {
		let (source, expected) = source(*row, id);
		let path = scratch.join(format!("case-{id}.fa"));
		fs::write(&path, &source).unwrap();
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(source.as_str()));
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|d| d.severity == Severity::Error)
			.collect();
		let allowed = row.rejected();
		manifest.push_str(&format!(
			"{id}\t{row:?}\t{}\t{expected}\n",
			allowed.join(",")
		));
		if allowed.is_empty() && errors.is_empty() {
			accepted.push((source, expected.to_string(), id));
		} else if errors.is_empty() || errors.iter().any(|d| !allowed.contains(&d.code.id)) {
			failures.push(format!(
				"{row:?} {}: expected {allowed:?}, got {:?}",
				path.display(),
				errors
					.iter()
					.map(|d| (d.code.id, &d.message))
					.collect::<Vec<_>>()
			));
		}
	}
	fs::write(scratch.join("manifest.tsv"), manifest).unwrap();
	assert!(failures.is_empty(), "{}", failures.join("\n"));
	eprintln!(
		"reference matrix: {} cases, {} accepted, {} rejected ({})",
		rows.len(),
		accepted.len(),
		rows.len() - accepted.len(),
		if full {
			"all feasible choice pairs"
		} else {
			"first 36 pairwise rows, fixed seed"
		}
	);
	// Fixed seeds keep the reported joins and non-syntactic escapes present
	// even when their higher-order combination is absent from a pairwise sample.
	for (name, source, allowed) in [
		(
			"nested-result-invalidation",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-nested-result-invalidation.fa"
			),
			&["REF001"][..],
		),
		(
			"joined-effect-model",
			include_str!("../../../analyzer/tests/cases/references/review2-joined-effect-model.fa"),
			&["REF001"][..],
		),
		(
			"joined-effect-model-builtin",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-joined-effect-model-builtin.fa"
			),
			&["REF001"][..],
		),
		(
			"joined-owner-call",
			include_str!("../../../analyzer/tests/cases/references/review2-joined-owner-call.fa"),
			&["REF009", "EFF004"][..],
		),
		(
			"joined-owner-dependent",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-joined-owner-dependent.fa"
			),
			&["REF020", "REF009"][..],
		),
		(
			"joined-owner-argument",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-joined-owner-argument.fa"
			),
			&["REF020", "REF009", "EFF011"][..],
		),
		(
			"alias-closure-escape",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-alias-closure-escape.fa"
			),
			&["MEM004"][..],
		),
		(
			"loop-closure-escape",
			include_str!("../../../analyzer/tests/cases/references/review2-loop-closure-escape.fa"),
			&["MEM004"][..],
		),
		(
			"loop-closure-observe",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-loop-closure-observe.fa"
			),
			&["MEM004"][..],
		),
		(
			"closure-field-observe",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-closure-field-observe.fa"
			),
			&["MEM004"][..],
		),
		(
			"transitive-closure-alias",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-transitive-closure-alias.fa"
			),
			&["MEM004"][..],
		),
		(
			"readonly-results",
			include_str!("../../../analyzer/tests/cases/references/review2-set-reference-write.fa"),
			&["E-DECL-3", "EFF011"][..],
		),
		(
			"owner-metadata",
			include_str!("../../../analyzer/tests/cases/references/review2-owner-metadata.fa"),
			&["REF009", "REF020"][..],
		),
		(
			"conditional-provenance",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-conditional-provenance.fa"
			),
			&["REF009", "MEM004"][..],
		),
		(
			"negative-index",
			include_str!(
				"../../../analyzer/tests/cases/references/review2-negative-index-invalidation.fa"
			),
			&["REF001"][..],
		),
	] {
		let (session, file) = fa_analyzer::analyze_source(source);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|d| d.severity == Severity::Error)
			.map(|d| d.code.id)
			.collect();
		assert!(
			!errors.is_empty() && errors.iter().all(|code| allowed.contains(code)),
			"fixed seed {name}: {errors:?}, expected {allowed:?}"
		);
	}
	for (source, expected, id) in [
		(
			include_str!("../../../analyzer/tests/cases/references/review2-nested-result-paths.fa"),
			"[8, 8, 4]",
			100002,
		),
		(
			include_str!(
				"../../../analyzer/tests/cases/references/review2-local-scalar-selection.fa"
			),
			"[5, 15, 20]",
			100003,
		),
		(
			include_str!(
				"../../../analyzer/tests/cases/references/review2-opaque-result-aliases.fa"
			),
			"[1, 1]",
			100004,
		),
		(
			include_str!("../../../analyzer/tests/cases/references/review2-static-locator-keys.fa"),
			"[53, 24, 34]",
			100005,
		),
		(
			include_str!(
				"../../../analyzer/tests/cases/references/review2-negative-index-snapshot.fa"
			),
			"[11, 2]",
			100006,
		),
	] {
		let source = source
			.split("static ")
			.next()
			.unwrap()
			.replace("Node", &format!("NestedNode{id}"))
			.replace("exercise", &format!("exercise{id}"));
		let mut source = source;
		for name in ["field", "preserve", "slot", "pick", "observe", "Pick"] {
			source = source.replace(name, &format!("{name}{id}"));
		}
		let source = if matches!(id, 100002 | 100004 | 100005 | 100006) {
			source.replace(
				&format!("exercise{id} = ()"),
				&format!("exercise{id} = (_flag: Boolean)"),
			)
		} else {
			source
		};
		accepted.push((source, expected.to_string(), id));
	}
	// Fixed regression seed covers key-only, static-key, general projection,
	// nested-container and function-value conventions in one program.
	let locator = include_str!("../../../analyzer/tests/cases/references/review2-locator-abi.fa");
	accepted.push((
		locator
			.split("static result")
			.next()
			.unwrap()
			.replace("exercise", "exercise100000")
			.replace("exercise100000 = ()", "exercise100000 = (_flag: Boolean)"),
		"[7, 18, 9, 6]".to_string(),
		100000,
	));
	let map =
		include_str!("../../../analyzer/tests/cases/references/review2-map-object-replacement.fa");
	let map = map
		.split("static result")
		.next()
		.unwrap()
		.replace("Node", "MapNode")
		.replace("exercise", "mapExercise");
	accepted.push((format!("{map}\nfunction exercise100001 = (_flag: Boolean): Integer throws KeyNotFound => try mapExercise(\"kept\")\n"), "9".to_string(), 100001));
	for (batch, cases) in accepted.chunks(8).enumerate() {
		let mut text = String::new();
		let mut lines = Vec::new();
		for (source, expected, id) in cases {
			text.push_str(source);
			text.push_str(&format!("static result{id} = @exercise{id}(true)\nconsole.log(result{id})\nconsole.log(try exercise{id}(true))\n"));
			lines.extend([expected.to_string(), expected.to_string()]);
		}
		let path = scratch.join(format!("batch-{batch}.fa"));
		fs::write(&path, &text).unwrap();
		for target in ["bun", "node", "web"] {
			let expected = Expected {
				stdout: lines.clone(),
				target,
				..Expected::default()
			};
			let outcome = run(
				&path,
				&expected,
				&scratch,
				10000
					+ batch * 10 + match target {
					"bun" => 0,
					"node" => 1,
					_ => 2,
				},
			);
			assert_eq!(outcome.exit, 0, "{target}: {outcome:?}; {}", path.display());
			assert_eq!(
				outcome.stdout.lines().collect::<Vec<_>>(),
				lines,
				"{target}: {}",
				path.display()
			);
		}
		// Native compilation includes the allocator's leak check. The ordinary
		// LLVM-unavailable configuration keeps the JavaScript/evaluator tier.
		for target in [
			fa_codegen_llvm::Target::Native,
			fa_codegen_llvm::Target::Wasm,
			fa_codegen_llvm::Target::NativeFreestanding,
		] {
			let mut session = Session::new();
			session.set_target(target.analysis_target());
			let file = session.set_source("main", Arc::from(text.as_str()));
			session.analyze_file(file);
			let options = fa_codegen_llvm::Options {
				leak_check: true,
				..Default::default()
			};
			let artifact = match fa_codegen_llvm::compile(
				&session,
				file,
				&path.display().to_string(),
				target,
				&options,
				&scratch.join(format!("batch-{batch}-{target:?}")),
			) {
				Ok(a) => a,
				Err(fa_codegen_llvm::Error::Unavailable) => break,
				Err(e) => panic!("{target:?}: {e}; {}", path.display()),
			};
			let mut command = if let Some(loader) = artifact.loader {
				let mut c = Command::new(
					std::env::var_os("HOME")
						.map(std::path::PathBuf::from)
						.unwrap()
						.join(".local/bin/node"),
				);
				c.arg(loader);
				c
			} else {
				Command::new(artifact.output)
			};
			let outcome = execute(&mut command, &scratch, &format!("batch-{batch}-{target:?}"));
			assert_eq!(outcome.exit, 0, "{target:?}: {outcome:?}");
			assert_eq!(
				outcome.stdout.lines().collect::<Vec<_>>(),
				lines,
				"{target:?}: {}",
				path.display()
			);
		}
	}
}
