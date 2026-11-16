//! Unit tests of recursive types, type operations and structural typing
//! (E1, M2).

use super::test_support::{analyzed, codes, node, symbol_type};
use crate::ids::NodeId;
use crate::types::TypeKind;

// ── recursive types (types brief §3.6) ───────────────────────────────────

#[test]
fn recursive_references_are_boxed_edges() {
	let (session, file) = analyzed(
		"type Node = { value: String, child: Node? }\nlet node = Node { value = \"a\", child = Node { value = \"b\", child = none } }\nlet next = node.child?.value\n",
	);
	assert_eq!(symbol_type(&session, file, "next"), "String?");
	// the field holds a `Rec` edge whose target is the declared type
	let types = session.types();
	let analysis = session.analysis(file);
	let node_symbol = analysis
		.symbols
		.symbols()
		.iter()
		.find(|symbol| types.syms.resolve(symbol.name) == "Node")
		.expect("the type");
	let declared = node_symbol.ty;
	let child = types
		.fields_of(declared)
		.iter()
		.find(|field| types.syms.resolve(field.name) == "child")
		.expect("a field")
		.ty;
	assert_eq!(types.display(child), "Node?");
	let TypeKind::Union(list) = types.kind(child) else {
		panic!("an optional");
	};
	let edge = types
		.list_items(list)
		.iter()
		.copied()
		.find(|&member| matches!(types.kind(member), TypeKind::Rec(_)))
		.expect("a recursive edge");
	assert_eq!(types.rec_target(edge), Some(declared));
	assert!(
		types
			.flags(declared)
			.contains(crate::types::TypeFlags::RECURSIVE | crate::types::TypeFlags::COMPARABLE)
	);
}

#[test]
fn recursion_needs_an_indirection_and_its_own_parameters() {
	assert_eq!(
		codes("type Node = { value: String, child: Node }\n"),
		["TY060"]
	);
	assert_eq!(codes("type A = { b: B }\ntype B = { a: A }\n"), ["TY060"]);
	assert!(codes("type A = { b: B? }\ntype B = { a: A? }\n").is_empty());
	assert!(codes("type Folder = { name: String, folders: Array<Folder> }\n").is_empty());
	assert_eq!(
		codes("type List<Item> = { head: Item, tail: List<Array<Item>>? }\n"),
		["GN020"]
	);
}

#[test]
fn recursive_types_are_compared_coinductively() {
	assert!(
		codes(
			"type Node = { value: String, child: Node? }\ntype Link = { value: String, child: Link? }\nlet node = Node { value = \"a\", child = none }\nlet link: Link = node\n"
		)
		.is_empty()
	);
	assert_eq!(
		codes(
			"type Node = { value: String, child: Node? }\ntype Other = { value: Integer, child: Other? }\nlet node = Node { value = \"a\", child = none }\nlet other: Other = node\n"
		),
		["E-TYPE-1"]
	);
}

#[test]
fn generic_recursive_types_instantiate() {
	let (session, file) = analyzed(
		"type List<Item> = { head: Item, tail: List<Item>? }\nlet l = List { head = 1, tail = List { head = 2, tail = none } }\nlet second = l.tail?.head\nlet typed: List<String> = { head = \"a\", tail = { head = \"b\", tail = none } }\n",
	);
	assert_eq!(symbol_type(&session, file, "second"), "Integer?");
	assert_eq!(symbol_type(&session, file, "typed"), "List<String>");
	assert_eq!(
		codes(
			"type List<Item> = { head: Item, tail: List<Item>? }\nlet bad: List<String> = { head = \"a\", tail = { head = 2, tail = none } }\n"
		),
		["E-TYPE-1"]
	);
}

// ── type operations (types brief §3.10, §3.12.4) ─────────────────────────

#[test]
fn differences_remove_members() {
	let (session, file) = analyzed(
		"type A = \"hello\" | \"world\" | \"you\"\ntype B = A - \"hello\"\nlet b: B = \"world\"\ntype R = Integer<min = 0, max = 10> - 0\nlet r: R = 1\ntype Split = Integer<min = 0, max = 10> - 5\nlet s: Split = 6\ntype Shape = Union { Circle = { radius: Float }, Point, Square = { side: Float } }\ntype Round = Shape - Shape.Point\nlet round: Round = Shape.Circle { radius = 1.0 }\nlet shape: Shape = round\nlet name = when round is { Shape.Circle => \"c\", Shape.Square => \"s\" }\n",
	);
	assert_eq!(
		symbol_type(&session, file, "r"),
		"Integer<min = 1, max = 10>"
	);
	assert_eq!(
		symbol_type(&session, file, "round"),
		"Shape.Circle | Shape.Square"
	);
	assert_eq!(codes("type B = (\"a\" | \"b\") - \"c\"\n"), ["TY111"]);
	assert_eq!(codes("type B = String - \"a\"\n"), ["TY110"]);
	assert_eq!(
		codes("type R = Integer<min = 0, max = 10> - 0\nlet r: R = 0\n"),
		["E-NUM-6"]
	);
	// `?` > `-` > `|`
	assert!(
		codes("type A = \"x\" | \"y\"\ntype C = A | A - A\nlet a: A = \"x\"\nlet c: C = a\n")
			.is_empty()
	);
}

#[test]
fn spreads_in_object_types() {
	let (session, file) = analyzed(
		"type Parent = { fieldA: Integer, fieldB = 13, readonly id: Integer = 0 }\ntype Child = { ...Parent, fieldC: String }\nlet child = Child { fieldA = 1, fieldC = \"c\" }\nlet b = child.fieldB\n",
	);
	assert_eq!(symbol_type(&session, file, "b"), "Integer");
	assert_eq!(
		codes("type A = { foo: String }\ntype B = { foo: Integer }\ntype C = { ...A, ...B }\n"),
		["TY101"]
	);
	assert_eq!(
		codes("type A = { foo: String }\ntype C = { ...A, foo: Integer }\n"),
		["TY101"]
	);
	assert!(codes("type A = { foo: String }\ntype C = { ...A, foo: String }\n").is_empty());
	assert_eq!(codes("type T = { ...Integer }\n"), ["TY100"]);
	assert_eq!(
		codes(
			"type Parent = { readonly id: Integer }\ntype Child = { ...Parent }\nmutable c = Child { id = 1 }\nc.id = 2\n"
		),
		["E-OBJ-10"]
	);
}

#[test]
fn object_type_functions() {
	analyzed(
		"type User = { id: Integer, name: String, address: { city: String, zip: String } }\ntype Slim = Omit<User, { address }>\ntype Ids = Pick<User, { id, address { city } }>\ntype Draft = Partial<User>\ntype K = KeyType<Map<String, Integer>>\nlet slim: Slim = { id = 1, name = \"a\" }\nlet ids: Ids = { id = 1, address = { city = \"c\" } }\nlet draft: Draft = {}\nlet k: K = \"key\"\n",
	);
	assert_eq!(
		codes("type User = { id: Integer }\ntype Q = Pick<User, { missing }>\n"),
		["TF010"]
	);
	assert_eq!(codes("type Q = Partial<Integer>\n"), ["TF021"]);
	assert_eq!(codes("type Q = KeyType<Array<Integer>>\n"), ["TF022"]);
	assert_eq!(
		codes(
			"type User = { id: Integer, name: String }\ntype Back = Required<Partial<User>>\nlet u: Back = { id = 1 }\n"
		),
		["E-OBJ-2"]
	);
	assert!(
		codes(
			"type A = { foo: String }\ntype B = { foo: Integer }\ntype C = { ...A, ...Omit<B, { foo }> }\n"
		)
		.is_empty()
	);
}

#[test]
fn spreads_in_object_literals() {
	let types = "type S = { host: String, port: Integer }\nlet d = S { host = \"h\", port = 1 }\n";
	let (session, file) = analyzed(&format!(
		"{types}let custom: S = {{ ...d, port = 8080 }}\nlet loose = {{ ...d, extra = true }}\n"
	));
	assert_eq!(
		symbol_type(&session, file, "loose"),
		"{ extra: Boolean, host: String, port: Integer }"
	);
	let analysis = session.analysis(file);
	let literal = node(&session, file, "{ ...d, port = 8080 }");
	let info = analysis.object(literal).expect("an object literal");
	assert!(matches!(
		info.fields.as_slice(),
		[
			(_, crate::analysis::FieldValue::Spread(_)),
			(_, crate::analysis::FieldValue::Given(_))
		]
	));
	assert_eq!(
		codes(&format!("{types}let s: S = {{ ...d, port = \"x\" }}\n")),
		["E-OBJ-7"]
	);
	assert_eq!(
		codes(&format!("{types}let s: S = {{ port = 2, ...d }}\n")),
		["E-OBJ-8"]
	);
	assert_eq!(codes("let n = 3\nlet o = { ...n }\n"), ["E-OBJ-9"]);
	assert_eq!(
		codes(&format!(
			"{types}type T = {{ host: String }}\nlet t: T = {{ ...d }}\n"
		)),
		["E-OBJ-1"]
	);
}

// ── structural typing (advanced brief §10) ───────────────────────────────

const HUMANS: &str = "type Human = { name: String, function describe = (self): String => \"human\" }\ntype Animal = { name: String, function describe = (self): String => \"animal\" }\ntype SuperHuman = { name: String, powers: String }\n";

#[test]
fn copies_narrow_and_views_do_not() {
	let (session, file) = analyzed(&format!(
		"{HUMANS}let hero = SuperHuman {{ name = \"a\", powers = \"b\" }}\nlet human: Human = hero\nfunction greet = (human: Human) => human.name\nlet greeting = greet(hero)\nmutable humans: Array<Human> = []\nhumans.push(hero)\nlet copies: Array<Human> = [hero]\n"
	));
	let analysis = session.analysis(file);
	let types = session.types();
	let coercion = |text: &str| {
		analysis
			.coercions
			.iter()
			.filter(|&(&node, _)| session.tree(file).text(node.index()) == text)
			.map(|(_, &ty)| types.display(ty))
			.collect::<Vec<_>>()
	};
	// the binding, the pushed element and the element of the literal are copies
	assert_eq!(coercion("hero").len(), 3);
	assert!(coercion("hero").iter().all(|ty| ty == "Human"));
	// the argument of `greet` is a view: nothing recorded on `greet(hero)`'s argument
	let call = node(&session, file, "greet(hero)");
	let info = analysis.call(call).expect("a call");
	let crate::analysis::Argument::Given(argument) = info.arguments[0] else {
		panic!("an argument");
	};
	assert!(!analysis.coercions.contains_key(&argument));
}

#[test]
fn methods_are_those_of_the_written_type() {
	let (session, file) = analyzed(&format!(
		"{HUMANS}let animal = Animal {{ name = \"Zeus\" }}\nlet human: Human = animal\nlet a = human.describe()\nlet b = animal.describe()\n"
	));
	let analysis = session.analysis(file);
	let tree = session.tree(file);
	let calls: Vec<NodeId> = (0..tree.nodes.len())
		.filter(|&index| tree.text(index) == "describe")
		.map(NodeId::new)
		.collect();
	let resolved: Vec<String> = calls
		.iter()
		.filter_map(|&member| match analysis.resolution(member) {
			crate::symbols::Resolution::Local(function) => {
				let decl = analysis.symbols.symbol(function).decl_node;
				Some(tree.text(decl.index()).to_string())
			}
			_ => None,
		})
		.collect();
	assert_eq!(resolved.len(), 2);
	assert!(resolved[0].contains("\"human\""), "{resolved:?}");
	assert!(resolved[1].contains("\"animal\""), "{resolved:?}");
}

#[test]
fn missing_fields_and_nominal_types() {
	assert_eq!(
		codes(
			"type UserId = { user: Integer }\ntype ProductId = { product: Integer }\nlet u = UserId { user = 1 }\nlet p: ProductId = u\n"
		),
		["STR001"]
	);
	assert_eq!(
		codes(
			"type Contact = { name: String, email: String? }\ntype Human = { name: String }\nlet h = Human { name = \"a\" }\nlet c: Contact = h\n"
		),
		["STR003"]
	);
	assert_eq!(
		codes(
			"type A = Union { User = Integer }\ntype B = Union { Product = Integer }\nlet a: A = A.User(1)\nlet b: B = a\n"
		),
		["E-TYPE-1"]
	);
	assert_eq!(
		codes(
			"function readId = <Item: { id: Integer }>(item: Item): Integer => item.id\ntype Comment = { text: String }\nlet x = readId(Comment { text = \"Hello\" })\n"
		),
		["STR001"]
	);
}

#[test]
fn type_of_a_parameter_is_an_implicit_generic() {
	let (session, file) = analyzed(&format!(
		"{HUMANS}function renamed = (human: Human, name: String): Type<human> => {{\n\tmutable duplicate = human\n\tduplicate.name = name\n\tduplicate\n}}\nlet clark = renamed(SuperHuman {{ name = \"Kal\", powers = \"flight\" }}, \"Clark\")\nlet powers = clark.powers\nlet john = renamed(Human {{ name = \"Ana\" }}, \"John\")\nlet count = 3\nlet same: Type<count> = 4\n"
	));
	assert_eq!(symbol_type(&session, file, "clark"), "SuperHuman");
	assert_eq!(symbol_type(&session, file, "john"), "Human");
	assert_eq!(symbol_type(&session, file, "same"), "Integer");
	let analysis = session.analysis(file);
	let call = node(
		&session,
		file,
		"renamed(Human { name = \"Ana\" }, \"John\")",
	);
	assert!(analysis.instantiation(call).is_some());
	assert_eq!(codes("let x: Type<Integer> = 1\n"), ["TF020"]);
}
