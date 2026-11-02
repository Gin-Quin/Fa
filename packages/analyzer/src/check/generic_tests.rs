//! Unit tests of generics and of the `Array` API (E1, M2).

use crate::analysis::Callee;
use crate::analyze_source;
use crate::ids::{FileId, NodeId};
use crate::session::Session;

fn codes(source: &str) -> Vec<&'static str> {
	let (session, file) = analyze_source(source);
	session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| diagnostic.code.id)
		.collect()
}

fn analyzed(source: &str) -> (Session, FileId) {
	let (session, file) = analyze_source(source);
	let found: Vec<String> = session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
		.collect();
	assert!(found.is_empty(), "{source}\n{found:#?}");
	(session, file)
}

fn symbol_type(session: &Session, file: FileId, name: &str) -> String {
	let analysis = session.analysis(file);
	let symbol = analysis
		.symbols
		.symbols()
		.iter()
		.rev()
		.find(|symbol| session.types().syms.resolve(symbol.name) == name)
		.unwrap_or_else(|| panic!("no symbol `{name}`"));
	session.display_type(symbol.ty)
}

fn node(session: &Session, file: FileId, text: &str) -> NodeId {
	let tree = session.tree(file);
	let found = (0..tree.nodes.len())
		.rev()
		.find(|&node| tree.text(node) == text)
		.unwrap_or_else(|| panic!("no node `{text}`"));
	NodeId::new(found)
}

const FIRST: &str = "function first(Item) = (items: Array(Item)): Item? => items.get(0)\n";
const PICK: &str = "function pick(Value: Integer | Float) = (value: Value): Value => value\n";

#[test]
fn inference_from_arguments_and_expected_type() {
	let (session, file) = analyzed(&format!(
		"{FIRST}{PICK}let a = first([\"Ada\"])\nlet b = pick(2)\nlet c: Float = pick(2)\n"
	));
	assert_eq!(symbol_type(&session, file, "a"), "String?");
	assert_eq!(symbol_type(&session, file, "b"), "Integer");
	assert_eq!(symbol_type(&session, file, "c"), "Float");
	// the backends get the type arguments of each call
	let analysis = session.analysis(file);
	let call = node(&session, file, "pick(2)");
	let list = analysis.instantiation(call).expect("an instantiation");
	assert_eq!(session.types().list_items(list).len(), 1);
}

#[test]
fn explicit_arguments_and_constraints() {
	let (session, file) = analyzed(&format!("{PICK}let f = pick(Float)(2)\n"));
	assert_eq!(symbol_type(&session, file, "f"), "Float");
	assert_eq!(codes(&format!("{PICK}let s = pick(\"a\")\n")), ["GN010"]);
	assert_eq!(
		codes(&format!("{PICK}let s = pick(String)(\"a\")\n")),
		["GN010"]
	);
	assert_eq!(
		codes("function empty(Item) = (): Array(Item) => []\nlet nothing = empty()\n"),
		["GN015"]
	);
	analyzed(
		"function empty(Item) = (): Array(Item) => []\nlet numbers: Array(Integer) = empty()\n",
	);
	assert_eq!(codes("function f(A, A) = (a: A) => a\n"), ["GN004"]);
	assert_eq!(
		codes("function f(A = Integer, B) = (a: A) => a\n"),
		["GN005"]
	);
	assert_eq!(
		codes("function f(A: Integer = String) = (a: A) => a\n"),
		["GN003"]
	);
}

#[test]
fn generic_types() {
	let point = "type Point(N) = { x: N, y: N }\n";
	let (session, file) = analyzed(&format!(
		"{point}let p: Point(Float) = {{ x = 1, y = 2 }}\nlet q = Point {{ x = 1, y = 2 }}\nlet x = p.x\n"
	));
	assert_eq!(symbol_type(&session, file, "p"), "Point(Float)");
	assert_eq!(symbol_type(&session, file, "q"), "Point(Integer)");
	assert_eq!(symbol_type(&session, file, "x"), "Float");
	assert_eq!(
		codes(&format!("{point}let m = Point {{ x = 1, y = 1.5 }}\n")),
		["GN014"]
	);
	assert_eq!(
		codes(&format!("{point}let m: Point = {{ x = 1, y = 2 }}\n")),
		["TY003"]
	);
	assert_eq!(
		codes(&format!(
			"{point}let m: Point(Integer, Float) = {{ x = 1, y = 2 }}\n"
		)),
		["GN007"]
	);
}

#[test]
fn generic_unions_and_methods() {
	let result = "type Result(Value, Failure) = Union { Ok = Value, Failed = Failure }\n";
	let (session, file) = analyzed(&format!(
		"{result}let a: Result(Integer, String) = Result.Ok(1)\nlet b = Result(Integer, String).Failed(\"no\")\n"
	));
	assert_eq!(symbol_type(&session, file, "b"), "Result(Integer, String)");
	assert_eq!(codes(&format!("{result}let c = Result.Ok(1)\n")), ["GN015"]);
	let stack = "type Stack(Item) = {\n\titems: Array(Item)\n\tfunction top = (self): Item? => self.items.get(0)\n}\n";
	let (session, file) = analyzed(&format!(
		"{stack}let s = Stack(Integer) {{ items = [1] }}\nlet t = s.top()\n"
	));
	assert_eq!(symbol_type(&session, file, "t"), "Integer?");
}

#[test]
fn constraints_give_members_in_generic_bodies() {
	analyzed(
		"type User = { id: Integer, name: String }\nfunction idOf(Entity: { id: Integer }) = (entity: Entity): Integer => entity.id\nlet i = idOf(User { id = 1, name = \"a\" })\n",
	);
	assert_eq!(
		codes("function nameOf(Value) = (value: Value) => value.name\n"),
		["GN016"]
	);
}

#[test]
fn generic_functions_as_values() {
	let identity = "function identity(Value) = (value: Value): Value => value\n";
	let (session, file) = analyzed(&format!("{identity}let names = [\"Ada\"].map(identity)\n"));
	assert_eq!(symbol_type(&session, file, "names"), "Array(String)");
	assert_eq!(codes(&format!("{identity}let f = identity\n")), ["GN015"]);
	analyzed(&format!(
		"{identity}let f: (value: Integer) => Integer = identity\n"
	));
}

#[test]
fn array_methods() {
	let (session, file) = analyzed(
		"mutable users = [\"Ana\"]\nusers.push(\"Bao\")\nusers.insert(0, \"Cleo\")\nlet last = users.pop()\nlet numbers = [1, 2, 3]\nlet doubled = numbers.map((x) => x * 2)\nlet labels = numbers.map((x, index) => \"{index}\")\nlet evens = numbers.filter((x) => x %% 2 == 0)\nlet total = numbers.reduce((sum, x) => sum + x, 0)\nlet big = numbers.find((x) => x > 1)\nlet sorted = numbers.sort()\nlet sum = numbers.sum()\nlet where = numbers.indexOf(2)\n",
	);
	assert_eq!(symbol_type(&session, file, "last"), "String?");
	assert_eq!(symbol_type(&session, file, "doubled"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "labels"), "Array(String)");
	assert_eq!(symbol_type(&session, file, "evens"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "total"), "Integer");
	assert_eq!(symbol_type(&session, file, "big"), "Integer?");
	assert_eq!(symbol_type(&session, file, "sorted"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "where"), "Integer(min = 0)?");
	let call = session
		.analysis(file)
		.call(node(&session, file, "numbers.map((x) => x * 2)"))
		.expect("CallInfo");
	assert!(matches!(call.callee, Callee::Builtin(_)));
	assert_eq!(codes("let fixed = [1]\nfixed.clear()\n"), ["EFF007"]);
	assert_eq!(codes("let n = [1].filter((x) => x)\n"), ["E-TYPE-1"]);
}

#[test]
fn constraints_may_require_methods() {
	let describe = "function describeAll(Item: { describe: (self) => String }) = (items: Array(Item)) => {\n\tfor item in items {\n\t\tconsole.log(item.describe())\n\t}\n}\n";
	let (session, file) = analyzed(&format!(
		"type Dog = {{ name: String, function describe = (self): String => \"dog\" }}\n{describe}describeAll([Dog {{ name = \"Rex\" }}])\n"
	));
	// inside the generic body, the call resolves on the constraint's method
	let analysis = session.analysis(file);
	let member = node(&session, file, "describe");
	assert!(matches!(
		analysis.resolution(member),
		crate::symbols::Resolution::Field(..)
	));
	assert_eq!(
		codes(&format!(
			"type Cat = {{ name: String }}\n{describe}describeAll([Cat {{ name = \"Tom\" }}])\n"
		)),
		["GN013"]
	);
	assert_eq!(
		codes(&format!(
			"type Cow = {{ name: String, function describe = (self): Integer => 1 }}\n{describe}describeAll([Cow {{ name = \"Milk\" }}])\n"
		)),
		["GN013"]
	);
}

#[test]
fn compile_time_value_parameters() {
	let (session, file) = analyzed(
		"type Matrix(Element, rows: Integer, columns: Integer) = { values: Array(Element) }\ntype A = Matrix(Float, 4, 4)\ntype B = Matrix(Float, 2 + 2, 4)\nlet a: A = { values = [] }\nlet b: B = a\nfunction repeatText(times: Integer(min = 0)) = (text: String): String => {\n\tmutable result = \"\"\n\tfor 0..times {\n\t\tresult += text\n\t}\n\tresult\n}\nlet twice = repeatText(2)(\"ab\")\n",
	);
	assert_eq!(symbol_type(&session, file, "twice"), "String");
	// the value is bound to its literal type
	let analysis = session.analysis(file);
	let application = node(&session, file, "repeatText(2)");
	let list = analysis
		.instantiation(application)
		.expect("an instantiation");
	assert_eq!(
		session.display_type(session.types().list_items(list)[0]),
		"2"
	);
	assert_eq!(codes("type Bad(scale: Float) = { x: Float }\n"), ["GN006"]);
	assert_eq!(
		codes("type M(rows: Integer) = { x: Integer }\nlet n = 3\ntype T = M(n)\n"),
		["GN008"]
	);
}
