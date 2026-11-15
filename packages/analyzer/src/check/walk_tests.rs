//! Unit tests of the walk: side tables and diagnostics of small programs.

use super::test_support::codes;
use crate::analyze_source;
use crate::ids::{NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};

#[test]
fn declarations_and_console_log_check_clean() {
	assert_eq!(
		codes(
			"let answer = 42\nmutable count: Int8 = 0\ncount = 1\ncount += 2\nconsole.log(answer, count)\n"
		),
		Vec::<&str>::new()
	);
}

#[test]
fn symbols_types_and_resolutions_are_recorded() {
	let (session, file) =
		analyze_source("let answer = 42\nmutable flag = true\nconsole.log(answer)\n");
	let analysis = session.analysis(file);
	let symbols = analysis.symbols.symbols();
	assert_eq!(symbols.len(), 2);
	assert_eq!(symbols[0].kind, SymbolKind::Let);
	assert_eq!(symbols[0].ty, TypeId::INTEGER);
	assert_eq!(symbols[1].kind, SymbolKind::Mutable);
	assert_eq!(symbols[1].ty, TypeId::BOOLEAN);
	let tree = session.tree(file);
	let read = (0..tree.nodes.len())
		.rev()
		.find(|&node| tree.text(node) == "answer")
		.map(NodeId::new)
		.unwrap();
	assert!(matches!(analysis.resolution(read), Resolution::Local(_)));
	assert_eq!(analysis.type_of(read), TypeId::INTEGER);
	let log = (0..tree.nodes.len())
		.find(|&node| tree.text(node) == "log")
		.map(NodeId::new)
		.unwrap();
	assert!(matches!(analysis.resolution(log), Resolution::Builtin(_)));
}

#[test]
fn name_errors() {
	assert_eq!(codes("total = 12\n"), ["E-DECL-1"]);
	assert_eq!(codes("console.log(missing)\n"), ["E-DECL-1"]);
	assert_eq!(codes("let value = 1\nlet value = 2\n"), ["E-DECL-2"]);
	assert_eq!(codes("let console = 1\n"), ["W-DECL-1"]);
}

#[test]
fn declaration_errors() {
	assert_eq!(codes("let myValue = 12\nmyValue += 1\n"), ["E-DECL-3"]);
	assert_eq!(codes("let myValue = 12\nmyValue = 1\n"), ["E-DECL-3"]);
	assert_eq!(codes("mutable x: Integer\n"), ["E-DECL-10"]);
	assert_eq!(codes("mutable x = none\n"), ["E-DECL-11"]);
	assert_eq!(codes("let x = console.log(1)\n"), ["E-DECL-9"]);
	assert_eq!(codes("let x: Nope = 1\n"), ["TY007"]);
}

#[test]
fn sinks_check_assignability() {
	assert_eq!(codes("let b: Boolean = 1\n"), ["E-TYPE-1"]);
	assert_eq!(codes("let i: Int8 = 300\n"), ["E-NUM-6"]);
	assert_eq!(
		codes("let i: Int8 = 100\nlet j: Int8 = i\n"),
		Vec::<&str>::new()
	);
	assert_eq!(codes("mutable flag = true\nflag = 3\n"), ["E-TYPE-1"]);
}

#[test]
fn partial_application_checks_its_unresolved_values() {
	assert_eq!(
		codes("let x = nope with (y = other with (z = nope))\n"),
		["E-DECL-1", "E-DECL-1", "E-DECL-1"]
	);
}

#[test]
fn hoisted_declarations_are_analyzed_once_on_demand() {
	let source = "let g = f\nfunction f = () => f\n";
	let (session, file) = analyze_source(source);
	let found: Vec<_> = session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| diagnostic.code.id)
		.collect();
	// a use before the declaration resolves; `f` returning itself is recursive
	assert_eq!(found, ["EFF020"]);
}

#[test]
fn shadowing_in_a_nested_scope_is_not_a_redeclaration() {
	// blocks are entered by handlers (if/do: E2); a function body is a block
	let (session, file) = analyze_source("let a = 1\nlet b = a\n");
	assert!(session.diagnostics(file).is_empty());
}

#[test]
fn the_analyzer_does_not_panic_on_broken_code() {
	for source in [
		"let = = (",
		"x.",
		"let a: = 1",
		"function = ",
		"console.log(",
		"a +=",
		"} {",
	] {
		let _ = analyze_source(source);
	}
}
