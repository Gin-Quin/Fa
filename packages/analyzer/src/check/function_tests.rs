//! Unit tests of declarations, functions, calls and type expressions (E1):
//! diagnostics and the backend-facing side tables.

use crate::analysis::{Argument, Callee};
use crate::analyze_source;
use crate::ids::{FileId, NodeId, TypeId};
use crate::session::Session;
use crate::symbols::{Resolution, SymbolKind};

/// The codes emitted for `source`, in order.
fn codes(source: &str) -> Vec<&'static str> {
	let (session, file) = analyze_source(source);
	session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| diagnostic.code.id)
		.collect()
}

/// Asserts that `source` checks without any diagnostic.
fn assert_clean(source: &str) {
	let (session, file) = analyze_source(source);
	let found: Vec<String> = session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
		.collect();
	assert!(found.is_empty(), "{source}\n{found:#?}");
}

/// The first node whose text is exactly `text` (the outermost one).
fn node(session: &Session, file: FileId, text: &str) -> NodeId {
	let tree = session.tree(file);
	let found = (0..tree.nodes.len())
		.filter(|&node| tree.text(node) == text)
		.max_by_key(|&node| node)
		.unwrap_or_else(|| panic!("no node `{text}`"));
	NodeId::new(found)
}

/// The displayed type of the outermost node whose text is `text`.
fn type_of(session: &Session, file: FileId, text: &str) -> String {
	session.display_type(session.analysis(file).type_of(node(session, file, text)))
}

/// The displayed type of the symbol named `name`.
fn symbol_type(session: &Session, file: FileId, name: &str) -> String {
	let analysis = session.analysis(file);
	let symbol = analysis
		.symbols
		.symbols()
		.iter()
		.find(|symbol| session.types().syms.resolve(symbol.name) == name)
		.unwrap_or_else(|| panic!("no symbol `{name}`"));
	session.display_type(symbol.ty)
}

// ── declarations and signatures ─────────────────────────────────────────

#[test]
fn a_function_with_a_declared_return_type() {
	let source = "function pick = (a: Integer, b: Boolean): Integer => a\nlet r = pick(1, true)\n";
	assert_clean(source);
	let (session, file) = analyze_source(source);
	assert_eq!(
		symbol_type(&session, file, "pick"),
		"(Integer, Boolean) => Integer"
	);
	assert_eq!(symbol_type(&session, file, "r"), "Integer");
	assert_eq!(symbol_type(&session, file, "b"), "Boolean");
}

#[test]
fn an_inferred_return_type_is_widened() {
	let source = "function answer = () => 42\nlet a = answer()\n";
	assert_clean(source);
	let (session, file) = analyze_source(source);
	assert_eq!(symbol_type(&session, file, "answer"), "() => Integer");
	assert_eq!(symbol_type(&session, file, "a"), "Integer");
}

#[test]
fn functions_are_hoisted_at_the_top_level() {
	let source = "console.log(later())\nfunction later = () => true\n";
	assert_clean(source);
	let (session, file) = analyze_source(source);
	assert_eq!(type_of(&session, file, "later()"), "Boolean");
}

#[test]
fn fn_info_describes_the_function_for_the_backends() {
	let source = "function f = (a: Integer, b: Int8 = 2, ...rest: Integer): Integer? => a\n";
	assert_clean(source);
	let (session, file) = analyze_source(source);
	let analysis = session.analysis(file);
	let declaration = analysis
		.symbols
		.symbols()
		.iter()
		.find(|symbol| symbol.kind == SymbolKind::Function)
		.unwrap()
		.decl_node;
	let info = analysis.function(declaration).expect("no FnInfo");
	assert_eq!(info.declaration, Some(declaration));
	assert!(analysis.function(info.lambda).is_some());
	assert_eq!(
		session.display_type(info.ty),
		"(Integer, Int8, ...Integer) => Integer?"
	);
	assert!(info.sig.is_some() && info.declared_return && !info.block_body);
	assert_eq!(info.params.len(), 3);
	assert!(info.params[1].default.is_some() && !info.params[1].variadic);
	assert!(info.params[2].variadic);
	let rest = analysis.symbols.symbol(info.params[2].symbol);
	assert_eq!(rest.kind, SymbolKind::Parameter { index: 2 });
	assert_eq!(session.display_type(rest.ty), "Array(Integer)");
	// the body resolves `a` to the parameter
	let body = info.body;
	assert_eq!(
		analysis.resolution(body),
		Resolution::Local(info.params[0].symbol)
	);
}

#[test]
fn parameters_need_a_type_without_context() {
	assert_eq!(
		codes("function f = (x) => x\n"),
		["FN061"],
		"a parameter without type"
	);
	assert_eq!(codes("function f = (x: Void) => 1\n"), ["E-DECL-9"]);
	assert_eq!(
		codes("function bad = (...a: Integer, b: Integer) => b\n"),
		["FN028"]
	);
}

#[test]
fn defaults_are_checked_and_see_only_earlier_parameters() {
	assert_clean("function f = (a: Integer, b: Integer = a) => b\nf(1)\n");
	assert_eq!(
		codes("function bad = (start: Integer = end, end: Integer = 0) => start\n"),
		["FN020"]
	);
	assert_eq!(
		codes("function f = (flag: Boolean = 1) => flag\n"),
		["E-TYPE-1"]
	);
	assert_eq!(
		codes("function f = (small: Int8 = 300) => small\n"),
		["E-NUM-6"]
	);
}

#[test]
fn a_body_declaration_can_t_reuse_a_parameter_name() {
	assert_eq!(
		codes("function f = (a: Integer) => {\n\tlet a = 2\n}\n"),
		["E-DECL-2"]
	);
	assert_eq!(
		codes("function f = (a: Integer, a: Integer) => 1\n"),
		["E-DECL-2"]
	);
	// an inner block is another scope
	assert_clean("let a = 1\nfunction f = (a: Boolean) => a\n");
}

#[test]
fn a_lambda_is_not_the_value_of_a_declaration() {
	// the parser reports it; E-DECL-6 stands for it if the parser accepts it
	assert_eq!(
		codes("let add = (a: Integer, b: Integer) => a\n"),
		["AN002"]
	);
	// a function name is a plain function value
	assert_clean("function add = (a: Integer): Integer => a\nlet f = add\nlet r = f(1)\n");
}

// ── bodies and return ───────────────────────────────────────────────────

#[test]
fn block_bodies_return_their_last_value() {
	assert_clean("function f = (x: Integer): Integer => {\n\tlet y = x\n\ty\n}\n");
	assert_eq!(
		codes("function f = (x: Integer): Integer => {\n\tlet y = x\n}\n"),
		["FN011"]
	);
	assert_eq!(
		codes("function f = (x: Integer): Boolean => {\n\tx\n}\n"),
		["E-TYPE-1"]
	);
	assert_eq!(codes("function f = (): Integer => {\n}\n"), ["FN011"]);
}

#[test]
fn a_block_without_return_type_drops_its_last_value() {
	assert_eq!(
		codes("function f = (x: Integer) => {\n\tx\n}\n"),
		["WFN001"]
	);
	assert_clean("function f = (x: Integer) => {\n\tconsole.log(x)\n}\n");
	let (session, file) = analyze_source("function f = (x: Integer) => {\n\tx\n}\n");
	assert_eq!(symbol_type(&session, file, "f"), "(Integer) => Void");
	assert_eq!(
		codes("function f = () => {\n\tconsole.log(1)\n}\nlet r = f()\n"),
		["E-DECL-9"]
	);
}

#[test]
fn return_statements() {
	assert_clean("function f = (x: Integer): Integer => {\n\treturn x\n}\n");
	assert_eq!(
		codes("function f = (x: Integer): Integer => {\n\treturn\n}\n"),
		["FN011"]
	);
	assert_eq!(
		codes("function f = (x: Integer) => {\n\treturn x\n}\n"),
		["FN013"]
	);
	assert_clean("function f = (x: Integer) => {\n\treturn\n}\n");
	assert_eq!(
		codes("function f = (): Boolean => {\n\treturn 1\n}\n"),
		["E-TYPE-1"]
	);
	assert_eq!(codes("return 1\n"), ["FN014"]);
	// the end after a `return` is unreachable: no missing value
	assert_clean("function f = (x: Integer): Integer => {\n\tconsole.log(x)\n\treturn x\n}\n");
}

#[test]
fn never_functions_can_t_return() {
	assert_eq!(
		codes("function fail = (): Never => {\n\tconsole.log(1)\n}\n"),
		["FN012"]
	);
	assert_eq!(
		codes("function fail = (): Never => {\n\treturn\n}\n"),
		["FN012"]
	);
}

// ── recursion and hoisting ──────────────────────────────────────────────

#[test]
fn recursion_needs_a_declared_return_type() {
	assert_eq!(
		codes("function countdown = (n: Integer) => countdown(n)\n"),
		["EFF020"]
	);
	assert_clean("function countdown = (n: Integer): Integer => countdown(n)\n");
	// through another function: one diagnostic
	assert_eq!(
		codes("function a = (n: Integer) => b(n)\nfunction b = (n: Integer) => a(n)\n"),
		["EFF020"]
	);
	assert_clean(
		"function a = (n: Integer): Integer => b(n)\nfunction b = (n: Integer): Integer => a(n)\n",
	);
}

#[test]
fn local_functions_are_visible_after_their_declaration() {
	assert_eq!(
		codes(
			"function main = () => {\n\tfunction a = (n: Integer): Integer => b(n)\n\tfunction b = (n: Integer): Integer => a(n)\n}\n"
		),
		["FN015"]
	);
	assert_eq!(
		codes("function main = () => {\n\tlater()\n\tfunction later = () => 1\n}\n"),
		["E-DECL-1"]
	);
	assert_clean(
		"function main = () => {\n\tfunction self2 = (n: Integer): Integer => self2(n)\n\tconsole.log(self2(1))\n}\n",
	);
}

#[test]
fn a_named_function_body_does_not_see_the_surrounding_function() {
	// `return` in the inner function returns from it, not from `outer`
	assert_clean(
		"function outer = (): Integer => {\n\tfunction inner = (): Boolean => {\n\t\treturn true\n\t}\n\t1\n}\n",
	);
}

// ── calls ───────────────────────────────────────────────────────────────

const MOVE: &str = "function move = (x: Integer, y: Integer, speed: Integer = 1) => x\n";

#[test]
fn named_arguments_and_defaults() {
	for call in [
		"move(1, 2)",
		"move(1, 2, speed = 3)",
		"move(1, speed = 3, y = 2)",
		"move(x = 1, y = 2, speed = 3)",
	] {
		assert_clean(&format!("{MOVE}{call}\n"));
	}
	let (session, file) = analyze_source(&format!("{MOVE}move(1, speed = 3, y = 2)\n"));
	let analysis = session.analysis(file);
	let call = analysis
		.call(node(&session, file, "move(1, speed = 3, y = 2)"))
		.expect("no CallInfo");
	assert!(matches!(call.callee, Callee::Function(_)));
	let given = |text: &str| Argument::Given(node(&session, file, text));
	assert_eq!(call.arguments, [given("1"), given("2"), given("3")]);
	// the name of a named argument resolves to its parameter
	let speed = node(&session, file, "speed = 3");
	let tree = session.tree(file);
	let name = (0..tree.nodes.len())
		.map(NodeId::new)
		.find(|&n| tree.text(n.index()) == "speed" && session.span(file, n).start > 70)
		.unwrap();
	assert!(matches!(analysis.resolution(name), Resolution::Local(_)));
	assert_eq!(session.display_type(analysis.type_of(speed)), "3");
	let defaults = analysis
		.call(node(&session, file, "move(1, speed = 3, y = 2)"))
		.unwrap();
	assert_eq!(defaults.arguments.len(), 3);
}

#[test]
fn defaults_are_recorded_for_omitted_arguments() {
	let (session, file) = analyze_source(&format!("{MOVE}move(1, 2)\n"));
	let call = session
		.analysis(file)
		.call(node(&session, file, "move(1, 2)"))
		.unwrap();
	assert_eq!(call.arguments[2], Argument::Default);
}

#[test]
fn argument_binding_errors() {
	let cases: [(&str, &[&str]); 7] = [
		("move(x = 1, 2)", &["FN021"]),
		("move(1, 2, x = 3)", &["FN022"]),
		("move(1)", &["FN024"]),
		("move(1, 2, 3, 4)", &["FN025"]),
		("move(1, 2, z = 3)", &["FN023"]),
		("move(true, 2)", &["E-TYPE-1"]),
		("move(1, ...2)", &["FN029"]),
	];
	for (call, expected) in cases {
		assert_eq!(codes(&format!("{MOVE}{call}\n")), expected, "{call}");
	}
}

#[test]
fn variadic_parameters() {
	let sum = "function sum = (...values: Integer): Integer => 0\n";
	assert_clean(&format!("{sum}sum()\nsum(1, 2, 3)\n"));
	assert_eq!(codes(&format!("{sum}sum(values = 1)\n")), ["FN026"]);
	assert_eq!(codes(&format!("{sum}sum(1, true)\n")), ["E-TYPE-1"]);
	let (session, file) = analyze_source(&format!("{sum}sum(1, 2)\n"));
	let call = session
		.analysis(file)
		.call(node(&session, file, "sum(1, 2)"))
		.unwrap();
	let one = node(&session, file, "1");
	let two = node(&session, file, "2");
	assert_eq!(call.arguments, [Argument::Variadic(vec![one, two])]);
}

#[test]
fn named_arguments_don_t_go_through_function_values() {
	assert_eq!(
		codes(
			"function add = (a: Integer, b: Integer): Integer => a\nlet f = add\nf(a = 1, b = 2)\n"
		),
		["FN027"]
	);
	// an alias keeps them
	assert_clean(
		"function add = (a: Integer, b: Integer): Integer => a\nfunction plus = add\nplus(a = 1, b = 2)\n",
	);
}

#[test]
fn calling_a_value_that_is_not_a_function() {
	assert_eq!(codes("let x = 1\nx(2)\n"), ["E-TYPE-1"]);
	assert_eq!(codes("nope(1)\n"), ["E-DECL-1"]);
}

#[test]
fn lambdas_take_their_types_from_the_expected_function_type() {
	assert_clean(
		"function apply = (f: (x: Integer) => Integer, v: Integer): Integer => f(v)\nlet r = apply((x) => x, 1)\n",
	);
	assert_eq!(
		codes(
			"function apply = (f: (x: Integer) => Boolean, v: Integer): Boolean => f(v)\napply((x) => x, 1)\n"
		),
		["E-TYPE-1"]
	);
	// no expected function type: annotate the parameters (lenient when the context is unknown)
	assert_eq!(
		codes("function f = (b: Boolean) => b\nf((x) => x)\n"),
		["FN061"]
	);
	assert_eq!(codes("console.log((x) => x)\n"), ["E-STR-12"]);
	assert!(codes("nope((x) => x)\n") == ["E-DECL-1"]);
	let (session, file) = analyze_source("nope((x: Integer) => true)\n");
	assert_eq!(session.diagnostics(file).len(), 1);
	assert_eq!(
		type_of(&session, file, "(x: Integer) => true"),
		"(Integer) => Boolean"
	);
}

// ── conversions and type expressions ────────────────────────────────────

#[test]
fn integer_conversions_are_builtin_calls() {
	let (session, file) = analyze_source("let small = Int8(3)\n");
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(symbol_type(&session, file, "small"), "Int8");
	let call = session
		.analysis(file)
		.call(node(&session, file, "Int8(3)"))
		.unwrap();
	assert_eq!(call.callee, Callee::Conversion(TypeId::INT8));
	assert_eq!(codes("let b = Int8(true)\n"), ["E-NUM-16"]);
	assert_eq!(codes("let b = Int8(1, 2)\n"), ["FN025"]);
	assert_eq!(codes("let b = Int8()\n"), ["FN024"]);
	assert_eq!(codes("let t = Integer\n"), ["TY008"]);
}

#[test]
fn ranged_integer_types() {
	let (session, file) =
		analyze_source("let a: Integer(min = 0, max = 10) = 5\nlet b: Int8(min = -3) = 1\n");
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(
		symbol_type(&session, file, "a"),
		"Integer(min = 0, max = 10)"
	);
	assert_eq!(symbol_type(&session, file, "b"), "Int8(min = -3)");
	assert_eq!(
		codes("let a: Integer(min = 5, max = 1) = 5\n"),
		["E-NUM-13"]
	);
	assert_eq!(codes("let a: Int8(max = 300) = 1\n"), ["E-NUM-13"]);
	assert_eq!(codes("let a: Int8(size = 3) = 1\n"), ["E-NUM-13"]);
	assert_eq!(
		codes("let a: Integer(min = 0, max = 10) = 11\n"),
		["E-NUM-6"]
	);
}

#[test]
fn scalar_and_optional_type_expressions() {
	let (session, file) = analyze_source(
		"let a: Integer? = none\nlet b: Boolean = true\nlet c: Float = 1\nlet d: Int8? = 3\n",
	);
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(symbol_type(&session, file, "a"), "Integer?");
	assert_eq!(symbol_type(&session, file, "d"), "Int8?");
	assert_eq!(codes("let a: Void? = none\n"), ["E-DECL-9"]);
	assert_eq!(codes("let x = 1\nlet a: x = 1\n"), ["TY008"]);
	assert_eq!(codes("let a: Never = 1\n"), ["E-TYPE-1"]);
}

#[test]
fn function_types_in_annotations() {
	let (session, file) =
		analyze_source("function f = (g: (a: Integer, ...b: Boolean) => Integer?) => g\n");
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(
		symbol_type(&session, file, "g"),
		"(Integer, ...Boolean) => Integer?"
	);
	assert_clean(
		"function ascending: (a: Integer, b: Integer) => Integer = (a, b) => a\nlet r = ascending(1, 2)\n",
	);
}

#[test]
fn a_top_level_function_sees_the_values_declared_after_it() {
	assert_clean("function show = () => console.log(answer)\nlet answer = 42\nshow()\n");
	// called before the value exists
	assert_eq!(
		codes("function show = () => console.log(answer)\nshow()\nlet answer = 42\n"),
		["E-DECL-1"]
	);
}

// ── hooks for strings (E3) ──────────────────────────────────────────────

#[test]
fn built_in_defaults_may_be_omitted() {
	assert_clean("let padded = \"7\".padStart(3)\n");
}

#[test]
fn a_string_where_a_character_is_expected() {
	assert_eq!(
		codes("function f = (c: Character) => c\nlet s: String = \"ab\"\nf(s)\n"),
		["E-STR-8"]
	);
	let (session, file) = analyze_source("let c: Character = \"a\"\nlet s: String = c\n");
	assert!(session.diagnostics(file).is_empty());
	let read = node(&session, file, "c");
	assert_eq!(session.analysis(file).coercion(read), Some(TypeId::STRING));
}

#[test]
fn ranged_values_keep_their_type() {
	let (session, file) = analyze_source(
		"function clamp = (x: Integer(min = 0)) => x\nlet c = clamp(1)\nlet d: Integer(min = 0) = 1\nlet e = d\n",
	);
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(symbol_type(&session, file, "c"), "Integer(min = 0)");
	assert_eq!(symbol_type(&session, file, "e"), "Integer(min = 0)");
}
