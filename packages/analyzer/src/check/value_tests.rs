//! Unit tests of the tables the backends read for the values of E3:
//! patterns, `Math` and `Array.from` calls, slices, number members.

use crate::analysis::{Argument, Callee, NodeFlags, PatternBinding};
use crate::analyze_source;
use crate::builtins;
use crate::ids::{NodeId, TypeId};
use crate::session::Session;
use crate::symbols::Resolution;
use crate::syntax::{self, View};

/// The first node whose text is `text` and whose view matches `wanted`.
fn find(
	session: &Session,
	file: crate::ids::FileId,
	text: &str,
	wanted: fn(&View) -> bool,
) -> NodeId {
	let tree = session.tree(file);
	(0..tree.nodes.len())
		.map(NodeId::new)
		.find(|&n| tree.text(n.index()) == text && wanted(&syntax::view(tree, n)))
		.unwrap_or_else(|| panic!("no node `{text}`"))
}

#[test]
fn object_patterns_record_their_fields_defaults_and_symbols() {
	let source = "type O = { a: Integer?, b: String }\nlet o: O = { a = none, b = \"x\" }\nlet { a = 3, b as label, c = 1.5 } = o\nconsole.log(a, label, c)\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let pattern = find(&session, file, "{ a = 3, b as label, c = 1.5 }", |view| {
		matches!(view, View::ObjectPattern { .. })
	});
	let info = analysis.pattern(pattern).expect("a pattern");
	assert_eq!(info.elements.len(), 3);
	let names: Vec<String> = info
		.elements
		.iter()
		.map(|element| match element.binding {
			PatternBinding::Symbol(symbol) => {
				let symbol = analysis.symbols.symbol(symbol);
				format!(
					"{}: {}",
					session.types().syms.resolve(symbol.name),
					session.display_type(symbol.ty)
				)
			}
			_ => "?".to_string(),
		})
		.collect();
	assert_eq!(names, ["a: Integer", "label: String", "c: Float"]);
	assert!(info.elements[0].default.is_some() && !info.elements[0].absent);
	assert!(info.elements[2].absent);
	assert_eq!(info.elements[2].ty, TypeId::UNKNOWN);
}

#[test]
fn array_patterns_say_when_the_length_is_checked() {
	let source = "function f = (values: Array(Integer)): Integer throws IndexError => {\n\ttry let [first, ...rest] = values\n\tfirst\n}\nlet [x, y] = [1, 2]\nconsole.log(f([1]) catch e => 0, x, y)\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let checked = find(&session, file, "[first, ...rest]", |view| {
		matches!(view, View::ArrayPattern { .. })
	});
	assert!(analysis.has_flags(checked, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL));
	let (rest, ty) = analysis.pattern(checked).unwrap().rest.unwrap();
	assert_eq!(
		session
			.types()
			.syms
			.resolve(analysis.symbols.symbol(rest).name),
		"rest"
	);
	assert_eq!(session.display_type(ty), "Array(Integer)");
	let proven = find(&session, file, "[x, y]", |view| {
		matches!(view, View::ArrayPattern { .. })
	});
	assert!(!analysis.has_flags(proven, NodeFlags::INDEX_CHECK));
}

#[test]
fn math_and_from_calls_are_recorded() {
	let source = "let n = 4\nlet m = Math.max(n, 1)\nmutable a = Array(Integer).from([], capacity = n)\nlet b = Array.from([1.5])\nconsole.log(m, a, b)\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let max = find(&session, file, "Math.max(n, 1)", |view| {
		matches!(view, View::Call { .. })
	});
	let call = analysis.call(max).unwrap();
	assert!(matches!(call.callee, Callee::Builtin(id) if builtins::get(id).path == "Math.max"));
	assert_eq!(call.arguments.len(), 2);
	assert_eq!(analysis.interval(max).map(|i| i.lo), Some(4.into()));
	let from = find(
		&session,
		file,
		"Array(Integer).from([], capacity = n)",
		|view| matches!(view, View::Call { .. }),
	);
	let call = analysis.call(from).unwrap();
	assert!(matches!(call.callee, Callee::Builtin(id) if builtins::get(id).path == "Array.from"));
	assert!(matches!(call.arguments[1], Argument::Given(_)));
	assert_eq!(
		session.display_type(analysis.type_of(from)),
		"Array(Integer)"
	);
	let copy = find(&session, file, "Array.from([1.5])", |view| {
		matches!(view, View::Call { .. })
	});
	assert_eq!(analysis.call(copy).unwrap().arguments[1], Argument::Default);
	assert_eq!(session.display_type(analysis.type_of(copy)), "Array(Float)");
}

#[test]
fn number_members_and_range_fields_resolve_to_builtins() {
	let source = "let x = 2.0\nlet r = x.sqrt()\nlet range = 1..3\nlet s = range.start\nlet t = [1, 2, 3][..2]\nconsole.log(r, s, t)\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let path = |text: &str| {
		let member = find(&session, file, text, |view| matches!(view, View::Name(_)));
		match analysis.resolution(member) {
			Resolution::Builtin(id) => builtins::get(id).path,
			other => panic!("{text}: {other:?}"),
		}
	};
	assert_eq!(path("sqrt"), "Float#sqrt");
	assert_eq!(path("start"), "Range#start");
	let slice = find(&session, file, "[1, 2, 3][..2]", |view| {
		matches!(view, View::Index { .. })
	});
	assert!(!analysis.has_flags(slice, NodeFlags::INDEX_CHECK));
	assert_eq!(
		session.display_type(analysis.type_of(slice)),
		"Array(Integer)"
	);
}

#[test]
fn tuples_index_with_literals() {
	let source = "let pair = (1, \"one\")\nlet n = pair[0]\nlet t = pair[1]\nconsole.log(n, t)\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let second = find(&session, file, "pair[1]", |view| {
		matches!(view, View::Index { .. })
	});
	assert_eq!(analysis.type_of(second), TypeId::STRING);
	let pair = find(&session, file, "(1, \"one\")", |view| {
		matches!(view, View::Group(_))
	});
	assert_eq!(
		session.display_type(analysis.type_of(pair)),
		"Tuple(Integer, String)"
	);
}
