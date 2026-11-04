//! Unit tests of type declarations, objects, unions, enumerations, `when`,
//! arrays, methods and optionals (E1, M1b): diagnostics and side tables.

use super::test_support::{codes, node, symbol_type};
use crate::analysis::{Argument, Callee, FieldValue};
use crate::analyze_source;
use crate::ids::FileId;
use crate::session::Session;
use crate::symbols::Resolution;

fn assert_clean(source: &str) {
	let (session, file) = analyze_source(source);
	let found: Vec<String> = session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
		.collect();
	assert!(found.is_empty(), "{source}\n{found:#?}");
}

fn type_of(session: &Session, file: FileId, text: &str) -> String {
	session.display_type(session.analysis(file).type_of(node(session, file, text)))
}

const POINT: &str = "type Point = { x: Integer, y: Integer = 0 }\n";

// ── declarations ────────────────────────────────────────────────────────

#[test]
fn aliases_and_literal_types() {
	assert_clean(
		"type Greeting = \"hello\"\ntype Id = Integer\nlet g: Greeting = \"hello\"\nlet i: Id = 3\nlet n: Integer = i\n",
	);
	assert_eq!(
		codes("type Greeting = \"hello\"\nlet g: Greeting = \"bye\"\n"),
		["E-TYPE-1"]
	);
	assert_eq!(codes("let x = 3\ntype T = x\n"), ["TY008"]);
	let (session, file) =
		analyze_source("type Direction = \"north\" | \"south\"\nlet d: Direction = \"north\"\n");
	assert!(session.diagnostics(file).is_empty());
	assert_eq!(symbol_type(&session, file, "d"), "\"north\" | \"south\"");
}

#[test]
fn object_types_record_fields_in_declaration_order() {
	let source = "type Person = { name: String, readonly age: Integer = 0, #secret = 1 }\n";
	let (session, file) = analyze_source(source);
	assert!(session.diagnostics(file).is_empty());
	let analysis = session.analysis(file);
	let decl = analysis
		.type_decls
		.values()
		.next()
		.expect("a type declaration");
	let names: Vec<&str> = decl
		.fields
		.iter()
		.map(|field| session.types().syms.resolve(field.name))
		.collect();
	assert_eq!(names, ["name", "age", "#secret"]);
	assert!(decl.fields[1].default.is_some());
	assert!(
		decl.fields[1]
			.flags
			.contains(crate::types::FieldFlags::READONLY)
	);
	assert!(
		decl.fields[2]
			.flags
			.contains(crate::types::FieldFlags::PRIVATE)
	);
}

#[test]
fn declaration_errors() {
	assert_eq!(codes("type T = { a: Integer, a: String }\n"), ["TY010"]);
	assert_eq!(
		codes("type T = { b: Integer, function b = (self) => 1 }\n"),
		["E-OBJ-12"]
	);
	assert_eq!(
		codes("type T = { a: Integer, b: Integer = a }\n"),
		["TY013"]
	);
	assert_eq!(codes("type T = { a: Void }\n"), ["E-DECL-9"]);
	assert_eq!(codes("type T = Integer | Float\n"), ["TY040"]);
}

// ── literals ────────────────────────────────────────────────────────────

#[test]
fn typed_and_untyped_literals() {
	let source = format!(
		"{POINT}let a = Point {{ x = 1 }}\nlet b: Point = {{ x = 2, y = 3 }}\nlet c = {{ n = 1, s = \"a\" }}\n"
	);
	let (session, file) = analyze_source(&source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "a"), "Point");
	assert_eq!(symbol_type(&session, file, "b"), "Point");
	assert_eq!(
		symbol_type(&session, file, "c"),
		"{ n: Integer, s: String }"
	);
	// the backends get every field in declaration order, defaults included
	let analysis = session.analysis(file);
	let info = analysis
		.object(node(&session, file, "Point { x = 1 }"))
		.expect("ObjectInfo");
	assert!(info.decl.is_some());
	assert!(matches!(info.fields[0].1, FieldValue::Given(_)));
	assert!(matches!(info.fields[1].1, FieldValue::Default(_)));
}

#[test]
fn literal_errors() {
	let cases: [(&str, &[&str]); 7] = [
		("let p: Point = { x = 1, z = 3 }", &["E-OBJ-1"]),
		("let p: Point = { y = 1 }", &["E-OBJ-2"]),
		("let p = Point { x = 1, x = 2 }", &["E-OBJ-3"]),
		("let p = Point { x = true }", &["E-TYPE-1"]),
		("let p = Point(1, 2)", &["E-OBJ-6"]),
		("let p: Point = Point { x = 1 }", &["W-OBJ-1"]),
		("let o = { a = 1, a = 2 }", &["E-OBJ-3"]),
	];
	for (line, expected) in cases {
		assert_eq!(codes(&format!("{POINT}{line}\n")), expected, "{line}");
	}
	assert_eq!(codes("let o = { a = none }\n"), ["E-DECL-11"]);
	// an optional field defaults to `none`
	assert_clean("type C = { name: String, email: String? }\nlet c: C = { name = \"a\" }\n");
}

#[test]
fn field_access_and_writes() {
	let base = format!("{POINT}mutable p = Point {{ x = 1 }}\nlet q = Point {{ x = 2 }}\n");
	assert_clean(&format!("{base}p.x = 3\np.y += 1\nlet sum = p.x + q.y\n"));
	assert_eq!(codes(&format!("{base}q.x = 3\n")), ["E-DECL-4"]);
	assert_eq!(codes(&format!("{base}console.log(p.z)\n")), ["E-OBJ-5"]);
	let (session, file) = analyze_source(&format!("{base}let read = q.x\n"));
	assert_eq!(symbol_type(&session, file, "read"), "Integer");
	assert!(matches!(
		session.analysis(file).resolution(node(&session, file, "x")),
		Resolution::Field(..)
	));
	assert_eq!(
		codes("type A = { readonly balance: Integer = 0 }\nmutable a = A {}\na.balance = 1\n"),
		["E-OBJ-10"]
	);
	assert_eq!(
		codes("type C = { #count: Integer = 0 }\nlet c = C {}\nconsole.log(c.#count)\n"),
		["E-OBJ-4"]
	);
	assert_eq!(
		codes("type C = { #count: Integer = 0 }\nlet c = C { #count = 1 }\n"),
		["E-OBJ-4"]
	);
}

#[test]
fn update_fields() {
	let base =
		"type S = { port: Integer, secure: Boolean }\nmutable s = S { port = 1, secure = false }\n";
	assert_clean(&format!("{base}s .= {{ port = 2, secure = true }}\n"));
	assert_eq!(codes(&format!("{base}s .= {{ host = 2 }}\n")), ["E-OBJ-5"]);
	assert_eq!(
		codes(&format!("{base}s .= {{ port = true }}\n")),
		["E-TYPE-1"]
	);
}

#[test]
fn structural_assignability() {
	assert_clean(
		"type Human = { name: String }\ntype Hero = { name: String, power: String }\nlet hero = Hero { name = \"a\", power = \"b\" }\nlet human: Human = hero\n",
	);
	assert_eq!(
		codes(
			"type Contact = { name: String, email: String? }\ntype Human = { name: String }\nlet h = Human { name = \"a\" }\nlet c: Contact = h\n"
		),
		["STR003"]
	);
}

// ── methods ─────────────────────────────────────────────────────────────

const COUNTER: &str = "type Counter = {\n\tvalue: Integer\n\tfunction double = (self): Integer => self.value * 2\n\tfunction make = (start: Integer): Counter => Counter { value = start }\n\tfunction zero = (): Counter => Self.make(0)\n}\n";

#[test]
fn methods_and_associated_functions() {
	let source = format!("{COUNTER}let c = Counter.make(3)\nlet d = c.double()\n");
	let (session, file) = analyze_source(&source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "c"), "Counter");
	assert_eq!(symbol_type(&session, file, "d"), "Integer");
	let call = session
		.analysis(file)
		.call(node(&session, file, "c.double()"))
		.expect("CallInfo");
	assert!(matches!(call.callee, Callee::Function(_)));
	assert!(matches!(call.arguments[0], Argument::Receiver(_)));
	assert_eq!(symbol_type(&session, file, "self"), "Counter");
}

#[test]
fn self_and_unqualified_functions() {
	assert_eq!(codes("function f = (self) => 1\n"), ["FN040"]);
	assert_eq!(
		codes("type U = { function m = (self: U) => 1 }\n"),
		["FN041"]
	);
	assert_eq!(
		codes("type T = { function make = (): T => T {}\nfunction again = (): T => make() }\n"),
		["FN043"]
	);
	assert_eq!(codes("let o = { function f = () => 1 }\n"), ["FN042"]);
}

// ── named unions and enumerations ───────────────────────────────────────

const SHAPE: &str =
	"type Shape = Union {\n\tCircle = { radius: Float }\n\tLabelled = String\n\tPoint\n}\n";

#[test]
fn named_union_members() {
	let source = format!(
		"{SHAPE}let a = Shape.Circle {{ radius = 1.0 }}\nlet b = Shape.Labelled(\"n\")\nlet c = Shape.Point\nfunction r = (circle: Shape.Circle) => circle.radius\n"
	);
	let (session, file) = analyze_source(&source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	// a member widens to the whole union (D53)
	assert_eq!(symbol_type(&session, file, "a"), "Shape");
	assert_eq!(
		type_of(&session, file, "Shape.Labelled(\"n\")"),
		"Shape.Labelled"
	);
	let call = session
		.analysis(file)
		.call(node(&session, file, "Shape.Labelled(\"n\")"))
		.unwrap();
	assert!(matches!(call.callee, Callee::Member(_)));
	let construction: [(&str, &str); 6] = [
		("let x = Shape.Circle(1.0)", "TY052"),
		("let x = Shape.Labelled { value = \"x\" }", "TY053"),
		("let x = Shape.Point()", "TY054"),
		("let x: Shape = { radius = 1.0 }", "TY055"),
		("let x = Shape.Labelled(1)", "E-TYPE-1"),
		("let x: String = Shape.Labelled(\"a\")", "E-TYPE-1"),
	];
	for (line, code) in construction {
		assert_eq!(codes(&format!("{SHAPE}{line}\n")), [code], "{line}");
	}
	assert_eq!(
		codes("type U = Union { A = \"a\", B = \"a\" }\n"),
		["TY051"]
	);
	assert_eq!(codes("type U = Union { A, A }\n"), ["TY050"]);
	// a literal member accepts its literal
	assert_clean("type G = Union { Hello = \"hello\", Other = Integer }\nlet g: G = \"hello\"\n");
}

#[test]
fn enumerations() {
	let color = "type Color = Enum { red, green }\n";
	let (session, file) = analyze_source(&format!(
		"{color}let a = Color.red\nlet b: Color = .green\nlet all = Color.members\nlet name = a.toString()\n"
	));
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "a"), "Color");
	assert_eq!(symbol_type(&session, file, "all"), "Array(Color)");
	assert_eq!(symbol_type(&session, file, "name"), "String");
	assert_eq!(codes("let c = .red\n"), ["TY084"]);
	assert_eq!(codes("type S = Enum { a = 0, b }\n"), ["TY081"]);
	assert_eq!(codes("type S = Enum { a = 1, b = 1 }\n"), ["TY082"]);
	assert_eq!(codes("type S = Enum { a = -1, b = 2 }\n"), ["TY083"]);
	assert_eq!(codes("type S = Enum { a, a }\n"), ["TY080"]);
	assert_eq!(
		codes("type A = Enum { x }\ntype B = Enum { x }\nlet b: B = A.x\n"),
		["E-TYPE-1"]
	);
}

// ── when ────────────────────────────────────────────────────────────────

#[test]
fn when_arms_bind_and_join() {
	let source = format!(
		"{SHAPE}function describe = (s: Shape): String => when s is {{\n\tShape.Circle as {{ radius }} => \"circle\"\n\tShape.Labelled as text => text\n\tShape.Point => \"point\"\n}}\n"
	);
	assert_clean(&source);
	let (session, file) = analyze_source(&source);
	assert_eq!(symbol_type(&session, file, "radius"), "Float");
	// the data of a member that is not an object
	assert_eq!(symbol_type(&session, file, "text"), "String");
	assert_clean(
		"let n = 3\nlet label = when n is {\n\t1 | 2 => \"small\"\n\telse => \"big\"\n}\n",
	);
}

#[test]
fn when_errors() {
	assert_eq!(
		codes(&format!(
			"{SHAPE}function f = (s: Shape) => when s is {{\n\tShape.Circle => 1\n}}\n"
		)),
		["E-WHEN-1"]
	);
	assert_eq!(
		codes("let s = \"a\"\nwhen s is { \"a\" => 1 }\n"),
		["E-WHEN-1"]
	);
	assert_eq!(
		codes("function f = (p: Int8) => when p is {\n\t300 => 1\n\telse => 2\n}\n"),
		["E-WHEN-3"]
	);
	assert_eq!(
		codes("let s = 3\nwhen s is {\n\telse => 1\n\t3 => 2\n}\n"),
		["E-WHEN-8"]
	);
	assert_eq!(
		codes(&format!(
			"{SHAPE}function f = (s: Shape) => when s is {{\n\tPoint => 1\n\telse => 2\n}}\n"
		)),
		["TY057"]
	);
	assert_clean("let b = true\nlet n = when b is {\n\ttrue => 1\n\tfalse => 0\n}\n");
}

// ── arrays ──────────────────────────────────────────────────────────────

#[test]
fn arrays() {
	let (session, file) = analyze_source(
		"mutable xs = [1, 2]\nxs.push(3)\nlet n = xs.length\nlet first = xs[0]\nlet names: Array<String> = []\nfor x, i in xs {\n\tconsole.log(x, i)\n}\n",
	);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "xs"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "n"), "Integer(min = 0)");
	assert_eq!(symbol_type(&session, file, "first"), "Integer");
	assert_eq!(symbol_type(&session, file, "x"), "Integer");
	assert_eq!(codes("let xs = [1]\nxs.push(2)\n"), ["EFF007"]);
	assert_eq!(codes("let xs = []\n"), ["E-DECL-11"]);
	assert_eq!(codes("let xs = [1, 0.5]\n"), ["E-IF-2"]);
	assert_eq!(codes("mutable xs = [1]\nxs.push(true)\n"), ["E-TYPE-1"]);
	assert_eq!(codes("let xs = [1]\nxs[0] = 2\n"), ["E-DECL-4"]);
	assert_eq!(codes("let xs = [1]\nlet y = xs[true]\n"), ["E-TYPE-1"]);
}

// ── optionals ───────────────────────────────────────────────────────────

#[test]
fn optionals() {
	let base = "type U = { name: String, next: { city: String }? }\nlet u: U? = none\n";
	let (session, file) = analyze_source(&format!(
		"{base}let a = u?.name\nlet b = u?.next?.city ?? \"none\"\nlet c: Integer? = none\nlet d = c ?? 3\n"
	));
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "a"), "String?");
	assert_eq!(symbol_type(&session, file, "b"), "String");
	assert_eq!(symbol_type(&session, file, "d"), "Integer");
	assert_eq!(codes(&format!("{base}console.log(u.name)\n")), ["E-OPT-1"]);
	assert_eq!(codes("let n = 5\nlet m = n ?? 0\n"), ["E-OPT-6"]);
	assert_eq!(
		codes("type U = { n: Integer }\nlet u = U { n = 1 }\nlet x = u?.n\n"),
		["E-OPT-3"]
	);
	assert_eq!(codes("let x: Integer = none\n"), ["E-OPT-2"]);
	assert_clean("let x: Optional<Integer> = 3\nlet y: Integer | None = x\n");
}

#[test]
fn anonymous_unions_of_objects() {
	let shapes =
		"type Shape = { kind: \"circle\", radius: Float } | { kind: \"square\", side: Float }\n";
	let (session, file) = analyze_source(&format!(
		"{shapes}let s: Shape = {{ kind = \"circle\", radius = 1.0 }}\nlet k = s.kind\n"
	));
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	assert_eq!(symbol_type(&session, file, "k"), "\"circle\" | \"square\"");
	assert_eq!(
		codes(
			"type A = { a: Integer }\ntype B = { b: Integer }\ntype AB = { a: Integer, b: Integer }\nlet both = AB { a = 1, b = 2 }\nlet u: A | B = both\n"
		),
		["TY041"]
	);
}
