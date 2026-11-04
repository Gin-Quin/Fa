//! Unit tests of sets, maps and the collection effects (E1, M2).

use super::test_support::{analyzed, codes, node, symbol_type};
use crate::analysis::{Argument, Callee, NodeFlags};
use crate::builtins::{self, array::EffectStep};
use crate::symbols::Resolution;
use crate::types::{CollKind, PathRoot, PathStep, TypeKind};

#[test]
fn list_literals_take_the_expected_collection() {
	let (session, file) = analyzed(
		"let s: Set<Integer> = [1, 2, 2]\nlet m: Map<String, Integer> = [(\"a\", 1), (\"b\", 2)]\nlet e: Map<String, Integer> = []\nlet t: Map<Tuple<Integer, Integer>, String> = [((0, 0), \"origin\")]\n",
	);
	assert_eq!(symbol_type(&session, file, "s"), "Set(Integer)");
	assert_eq!(symbol_type(&session, file, "m"), "Map(String, Integer)");
	assert_eq!(symbol_type(&session, file, "e"), "Map(String, Integer)");
	let pair = node(&session, file, "(\"a\", 1)");
	assert_eq!(
		session.display_type(session.analysis(file).type_of(pair)),
		"Tuple(String, Integer)"
	);
	assert_eq!(
		codes("let m: Map<String, Integer> = [1]\n"),
		["CL005"],
		"a map item is a pair"
	);
	assert_eq!(
		codes("let m: Map<String, Integer> = [(\"a\", 1), (\"a\", 2)]\n"),
		["CL006"]
	);
	assert_eq!(
		codes("let m: Map<String, Integer> = [(\"a\", \"b\")]\n"),
		["E-TYPE-1"]
	);
}

#[test]
fn set_elements_and_map_keys_need_equality_and_are_exact() {
	assert_eq!(
		codes("type Handler = { run: () => Void }\nlet hs: Set<Handler> = []\n"),
		["CL020"]
	);
	assert_eq!(codes("let m: Map<() => Void, Integer> = []\n"), ["CL020"]);
	// map values may be anything
	assert!(codes("let m: Map<String, () => Void> = []\n").is_empty());
	let types = "type Human = { name: String }\ntype Hero = { name: String, power: String }\n";
	assert_eq!(
		codes(&format!(
			"{types}function f = (s: Set(Human)) => s.size\nlet heroes: Set(Hero) = []\nf(heroes)\n"
		)),
		["CL010"]
	);
	// map values follow the general rule: a read-only view of supersets
	assert!(
		codes(&format!(
			"{types}function f = (m: Map(String, Human)) => m.size\nlet heroes: Map(String, Hero) = []\nf(heroes)\n"
		))
		.is_empty()
	);
	assert_eq!(codes("let s: Set = []\n"), ["TY003"]);
	assert_eq!(codes("let m: Map<String> = []\n"), ["TY003"]);
}

#[test]
fn members_of_sets_and_maps() {
	let (session, file) = analyzed(
		"mutable s: Set<Integer> = [1]\nlet added = s.add(2)\nlet taken = s.take(1)\nlet u = s.union([3])\nlet doubled = s.map((x) => x * 2)\nlet total = s.sum()\nmutable m: Map<String, Integer> = []\nm[\"a\"] = 1\nlet v = m[\"a\"] catch error => 0\nlet g = m.get(\"a\")\nlet o = m.getOrInsert(\"b\", 2)\nlet d = m.delete(\"a\")\nlet mapped = m.map((key, value) => value > 1)\nlet found = m.find((key, value) => value > 1)\nlet n = m.size\n",
	);
	for (name, ty) in [
		("added", "Boolean"),
		("taken", "Integer?"),
		("u", "Set(Integer)"),
		("doubled", "Array(Integer)"),
		("total", "Integer"),
		("v", "Integer"),
		("g", "Integer?"),
		("o", "Integer"),
		("d", "Boolean"),
		("mapped", "Map(String, Boolean)"),
		("found", "Tuple(String, Integer)?"),
		("n", "Integer(min = 0)"),
	] {
		assert_eq!(symbol_type(&session, file, name), ty, "{name}");
	}
	let analysis = session.analysis(file);
	let add = node(&session, file, "add");
	let Resolution::Builtin(builtin) = analysis.resolution(add) else {
		panic!("`add` is a built-in");
	};
	assert_eq!(builtins::get(builtin).path, "Set#add");
	// a map read may miss its key; a map write never fails
	let read = node(&session, file, "m[\"a\"]");
	assert!(analysis.flags_of(read).contains(NodeFlags::INDEX_CHECK));
	assert_eq!(
		codes("let s: Set<Integer> = []\nlet n = s.length\n"),
		["CL007"]
	);
	assert_eq!(codes("let a = [1]\nlet n = a.size\n"), ["CL007"]);
	assert_eq!(codes("let s: Set<Integer> = []\ns.add(1)\n"), ["EFF007"]);
	assert_eq!(
		codes("mutable g: GrowingMap<String, Integer> = []\ng.delete(\"a\")\n"),
		["CL040"]
	);
	assert_eq!(
		codes("let s: Set<Integer> = []\nlet x = s[0]\n"),
		["E-TYPE-1"]
	);
	assert_eq!(
		codes("let m: Map<String, Integer> = []\nlet x = m[0]\n"),
		["E-TYPE-1"]
	);
}

#[test]
fn method_types_carry_the_effects_of_the_reference_table() {
	let (session, file) = analyzed(
		"mutable m: Map<String, Integer> = []\nm.delete(\"a\")\nm.clear()\nmutable a = [1]\na.pop()\n",
	);
	let analysis = session.analysis(file);
	let effects_of = |text: &str| {
		let member = node(&session, file, text);
		let TypeKind::Function(sig) = session.types().kind(analysis.type_of(member)) else {
			panic!("`{text}` is a method");
		};
		let (mutated, invalidated) = session.types().effects(session.types().sig(sig).effects);
		let describe = |paths: &[crate::ids::PathId]| -> Vec<(PathRoot, Vec<PathStep>)> {
			paths
				.iter()
				.map(|&path| {
					let (path, steps) = session.types().path_data(path);
					(path.root, steps.to_vec())
				})
				.collect()
		};
		(describe(mutated), describe(invalidated))
	};
	assert_eq!(
		effects_of("delete"),
		(
			vec![],
			vec![(PathRoot::SelfParam, vec![PathStep::IndexParam(0)])]
		)
	);
	assert_eq!(
		effects_of("clear"),
		(vec![(PathRoot::SelfParam, vec![])], vec![])
	);
	assert_eq!(
		effects_of("pop"),
		(vec![], vec![(PathRoot::SelfParam, vec![PathStep::Last])])
	);
	// the static table, for element writes
	let write = builtins::array::effects(CollKind::Map, "[]=");
	assert_eq!(
		write.iter().map(|effect| effect.step).collect::<Vec<_>>(),
		[EffectStep::Element, EffectStep::Size]
	);
	assert!(builtins::array::effects(CollKind::Set, "includes").is_empty());
}

#[test]
fn from_copies_collections_with_options() {
	let (session, file) = analyzed(
		"type User = { id: Integer, email: String }\nlet s = Set.from([1, 2, 2])\nlet f = Set<Float>.from([1, 2])\nlet m = Map.from([(\"a\", 1)])\nlet a = Array.from(s)\nlet users = Set.from([User { id = 1, email = \"a\" }], key = (user) => user.id)\n",
	);
	assert_eq!(symbol_type(&session, file, "s"), "Set(Integer)");
	assert_eq!(symbol_type(&session, file, "f"), "Set(Float)");
	assert_eq!(symbol_type(&session, file, "m"), "Map(String, Integer)");
	assert_eq!(symbol_type(&session, file, "a"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "users"), "Set(User)");
	let analysis = session.analysis(file);
	let call = node(
		&session,
		file,
		"Set.from([User { id = 1, email = \"a\" }], key = (user) => user.id)",
	);
	let info = analysis.call(call).expect("a call");
	let Callee::Builtin(builtin) = info.callee else {
		panic!("a built-in call");
	};
	assert_eq!(builtins::get(builtin).path, "Set.from");
	assert!(matches!(
		info.arguments.as_slice(),
		[Argument::Given(_), Argument::Given(_), Argument::Default]
	));
	assert_eq!(codes("let s = Set.from([1], size = 3)\n"), ["FN023"]);
	assert_eq!(codes("let m = Map.from([1, 2])\n"), ["CL005"]);
	assert_eq!(codes("let s: Set<Integer> = Set.from([1])\n"), ["WCL001"]);
	assert_eq!(
		codes("type H = { run: () => Void }\nlet s = Set.from([H { run = () => {} }])\n"),
		["CL020"]
	);
}

#[test]
fn loops_over_sets_and_maps() {
	let (session, file) = analyzed(
		"let m: Map<String, Integer> = [(\"a\", 1)]\nfor (key, value) in m {\n\tconsole.log(key, value)\n}\nlet s: Set<String> = [\"a\"]\nfor tag in s {\n\tconsole.log(tag)\n}\n",
	);
	assert_eq!(symbol_type(&session, file, "key"), "String");
	assert_eq!(symbol_type(&session, file, "value"), "Integer");
	assert_eq!(symbol_type(&session, file, "tag"), "String");
	assert_eq!(
		codes("mutable s: Set<String> = []\nfor tag in s {\n\ttag = \"x\"\n}\n"),
		["CL031"]
	);
	assert_eq!(
		codes("mutable m: Map<String, Integer> = []\nfor (key, value) in m {\n\tkey = \"x\"\n}\n"),
		["CL031"]
	);
}

#[test]
fn iterators_are_typed_and_tracked() {
	let (session, file) = analyzed(
		"let numbers = [1, 2, 3]\nlet evens = numbers.values().filter((n) => n %% 2 == 0)\nlet first = evens.firstValues(1).toArray()\nlet names: Map<String, Integer> = [(\"a\", 1)]\nlet keys = names.keys().toArray()\n",
	);
	assert_eq!(symbol_type(&session, file, "evens"), "Iterator(Integer)");
	assert_eq!(symbol_type(&session, file, "first"), "Array(Integer)");
	assert_eq!(symbol_type(&session, file, "keys"), "Array(String)");
	assert_eq!(
		codes(
			"mutable items = [1]\nlet it = items.values()\nitems.push(2)\nlet all = it.toArray()\n"
		),
		["REF001"]
	);
	assert_eq!(
		codes("function f = (items: Array<Integer>) => items.values()\n"),
		["CL030"]
	);
	assert_eq!(
		codes("let s: Set<Integer> = [1]\nfor x, i in s {\n\tconsole.log(x)\n}\n"),
		["CL032"]
	);
}

#[test]
fn pools_give_fresh_references() {
	let (session, file) = analyzed(
		"type H = { name: String }\nmutable pool: Pool<H> = []\nuse a = pool.add({ name = \"a\" })\nuse b = pool.add({ name = \"b\" })\npool.delete(a)\nconsole.log(b.name)\nlet n = pool.size\n",
	);
	assert_eq!(symbol_type(&session, file, "n"), "Integer(min = 0)");
	// `delete` invalidates the deleted element only
	assert_eq!(
		codes(
			"type H = { name: String }\nmutable pool: Pool<H> = []\nuse a = pool.add({ name = \"a\" })\npool.delete(a)\nconsole.log(a.name)\n"
		),
		["REF001"]
	);
	// an element of another pool (REF020, alias CL050)
	assert_eq!(
		codes(
			"type H = { name: String }\nmutable p: Pool<H> = []\nmutable q: Pool<H> = []\nuse a = p.add({ name = \"a\" })\nq.delete(a)\n"
		),
		["REF020"]
	);
	assert_eq!(codes("let c: Columnar<Integer> = []\n"), ["CL021"]);
	assert_eq!(
		codes("type H = { name: String }\nmutable g: GrowingPool<H> = []\ng.clear()\n"),
		["CL040"]
	);
	assert_eq!(
		builtins::effects::effects_of("Pool#delete"),
		[builtins::effects::BuiltinEffect::InvalidatesArgument(0)]
	);
	assert_eq!(
		builtins::effects::effects_of_kind(CollKind::GrowingArray, "push"),
		[builtins::effects::BuiltinEffect::MutatesSize]
	);
}

#[test]
fn relations_are_recorded_for_the_backends() {
	use crate::analysis::{CollectionRef, RelationKind, RelationSide};
	let (session, file) = analyzed(
		"type Town = {\n\tpeople: Pool<Person> = []\n\ttype Person = {\n\t\tname: String\n\t\tbestFriend: (Person in people)?\n\t\tbestFriendOf: Pool<people->bestFriend>\n\t\tmentor: (Person in people)?\n\t\tmentee: (people->mentor)?\n\t\tfriends: Pool<Person in people>\n\t\tfriendOf: Pool<in people->friends>\n\t\tspouse: (people->spouse)?\n\t}\n}\nmutable town: Town = {}\nuse ana = town.people.add({ name = \"Ana\" })\nuse bao = town.people.add({ name = \"Bao\" })\nana.bestFriend = bao\nana.friends.add(bao)\ntown.people.delete(bao)\n",
	);
	let analysis = session.analysis(file);
	let kinds: Vec<RelationKind> = analysis
		.relations
		.iter()
		.map(|relation| relation.kind)
		.collect();
	assert_eq!(
		kinds,
		[
			RelationKind::OneToMany,
			RelationKind::OneToOne,
			RelationKind::ManyToMany,
			RelationKind::Symmetric
		]
	);
	let relation = &analysis.relations[0];
	assert_eq!(session.types().syms.resolve(relation.direct), "bestFriend");
	assert_eq!(relation.holder, relation.target);
	assert!(matches!(relation.holder, CollectionRef::Field { .. }));
	let reverse = relation.reverse.expect("a reverse side");
	assert_eq!(session.types().syms.resolve(reverse), "bestFriendOf");
	assert_eq!(
		analysis
			.relation_fields
			.get(&(relation.target_type, reverse)),
		Some(&(0, RelationSide::Reverse))
	);
	// the write of `bestFriend` and of `friends`, and the removal of `bao`
	let mut written: Vec<u32> = analysis
		.relation_writes
		.values()
		.flatten()
		.copied()
		.collect();
	written.sort();
	assert_eq!(written, [0, 2]);
	let unlinks: Vec<&Vec<u32>> = analysis.relation_unlinks.values().collect();
	assert_eq!(unlinks, [&vec![0, 1, 2, 3]]);
	assert_eq!(symbol_type(&session, file, "ana"), "Person");
}
