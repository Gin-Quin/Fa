//! What the analysis gives the backends for minimal representations: the
//! module-level constants and the locals that never escape their frame.

use super::test_support::analyzed;
use crate::ids::FileId;
use crate::session::Session;

/// The symbols named `name` of the file, in declaration order.
fn symbols_named(session: &Session, file: FileId, name: &str) -> Vec<crate::ids::SymbolId> {
	let analysis = session.analysis(file);
	analysis
		.symbols
		.symbols()
		.iter()
		.enumerate()
		.filter(|(_, symbol)| session.types().syms.resolve(symbol.name) == name)
		.map(|(index, _)| crate::ids::SymbolId::new(index))
		.collect()
}

/// The constant value of the module-level value `name`, printed.
fn constant(session: &Session, file: FileId, name: &str) -> Option<String> {
	let symbol = *symbols_named(session, file, name).first()?;
	session
		.analysis(file)
		.module_constant(symbol)
		.map(|value| format!("{value:?}"))
}

/// Whether the local `name` (the last one of that name) stays in its frame.
fn stays(session: &Session, file: FileId, name: &str) -> bool {
	let symbol = *symbols_named(session, file, name)
		.last()
		.unwrap_or_else(|| panic!("no `{name}`"));
	session.analysis(file).is_non_escaping(symbol)
}

#[test]
fn module_lets_of_constant_values_are_constants() {
	let (session, file) = analyzed(
		"let limit = 10\n\
		 let doubled = limit * 2\n\
		 let name = \"fa\"\n\
		 let greeting = \"hello {name}\"\n\
		 let point = { x = 1, y = -2 }\n\
		 let table = [1, 2, 3]\n\
		 mutable counter = 0\n\
		 let computed = [3, 1].sort()\n\
		 function read = () => limit + doubled + counter + computed.length\n\
		 console.log(read(), greeting, point, table)\n",
	);
	assert!(constant(&session, file, "limit").is_some_and(|value| value.contains("10")));
	assert!(constant(&session, file, "doubled").is_some_and(|value| value.contains("20")));
	assert!(constant(&session, file, "greeting").is_some_and(|value| value.contains("hello fa")));
	assert!(constant(&session, file, "point").is_some());
	assert!(constant(&session, file, "table").is_some());
	// a `mutable`, a call: not constants
	assert_eq!(constant(&session, file, "counter"), None);
	assert_eq!(constant(&session, file, "computed"), None);
}

#[test]
fn locals_of_a_function_are_not_module_constants() {
	let (session, file) =
		analyzed("function f = (): Integer => {\n\tlet inner = 3\n\tinner\n}\nconsole.log(f())\n");
	assert_eq!(constant(&session, file, "inner"), None);
}

#[test]
fn locals_read_in_place_stay_in_their_frame() {
	let (session, file) = analyzed(
		"function show = (text: String) => console.log(text)\n\
		 function run = (count: Integer) => {\n\
		 \tlet label = \"item {count}\"\n\
		 \tmutable line = \"> \"\n\
		 \tline += label\n\
		 \tshow(line)\n\
		 \tconsole.log(label.length, line == label, label.get(0))\n\
		 \tfor character in label {\n\
		 \t\tconsole.log(character)\n\
		 \t}\n\
		 \tlet copy = label\n\
		 \tconsole.log(copy)\n\
		 }\n\
		 run(1)\n",
	);
	assert!(stays(&session, file, "label"));
	assert!(stays(&session, file, "line"));
	assert!(stays(&session, file, "copy"));
}

#[test]
fn locals_that_leave_their_frame_escape() {
	let (session, file) = analyzed(
		"type Box = { value: String }\n\
		 mutable kept: Array<String> = []\n\
		 function returned = (): String => {\n\
		 \tlet text = \"a\"\n\
		 \treturn text\n\
		 }\n\
		 function last = (): String => {\n\
		 \tlet value = \"b\"\n\
		 \tvalue\n\
		 }\n\
		 function stored = () mutates kept => {\n\
		 \tlet item = \"c\"\n\
		 \tkept.push(item)\n\
		 }\n\
		 function boxed = (): Box => {\n\
		 \tlet inside = \"d\"\n\
		 \tBox { value = inside }\n\
		 }\n\
		 function moved = (): String => {\n\
		 \tlet first = \"e\"\n\
		 \tlet second = first\n\
		 \tsecond\n\
		 }\n\
		 console.log(returned(), last(), boxed(), moved())\n\
		 stored()\n",
	);
	for name in ["text", "value", "item", "inside", "first", "second"] {
		assert!(!stays(&session, file, name), "`{name}` escapes");
	}
}
