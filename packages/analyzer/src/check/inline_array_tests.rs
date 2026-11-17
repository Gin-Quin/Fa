use crate::analysis::NodeFlags;
use crate::check::test_support::{analyzed, codes, node, symbol_type};

#[test]
fn inline_capacity_and_literal_length_are_facts() {
	let source = "mutable values: InlineArray<Integer, 2> = []\nvalues.push(1)\nvalues.push(2)\nconsole.log(values[1], values.capacity)\n";
	let (session, file) = analyzed(source);
	assert_eq!(
		symbol_type(&session, file, "values"),
		"InlineArray<Integer, 2>"
	);
	let analysis = session.analysis(file);
	for text in ["values.push(1)", "values.push(2)"] {
		assert!(!analysis.has_flags(node(&session, file, text), NodeFlags::CAPACITY_CHECK));
	}
	assert!(!analysis.has_flags(node(&session, file, "values[1]"), NodeFlags::INDEX_CHECK));
}

#[test]
fn inline_unknown_fullness_requires_try_and_a_capacity_guard() {
	let source = "function append = (values: InlineArray<Integer, 4>) throws CapacityError mutates values.length => { try values.push(1) }\n";
	let (session, file) = analyzed(source);
	assert!(session.analysis(file).has_flags(
		node(&session, file, "values.push(1)"),
		NodeFlags::CAPACITY_CHECK
	));
	assert!(codes("function append = (values: InlineArray<Integer, 4>) mutates values.length => { values.push(1) }\n").contains(&"E-ERR-3"));
}

#[test]
fn inline_length_guards_prove_room() {
	analyzed(
		"function append = (values: InlineArray<Integer, 4>) mutates values.length => { if values.length < 4 { values.push(1) } }\n",
	);
}

#[test]
fn inline_fullness_and_bad_capacities_have_actionable_diagnostics() {
	for source in [
		"mutable values: InlineArray<Integer, 0> = []\nvalues.push(1)\n",
		"mutable values: InlineArray<Integer, 1> = [1]\nvalues.push(2)\n",
		"let values: InlineArray<Integer, 1> = [1, 2]\n",
		"let values: InlineArray<Integer, -1> = []\n",
		"function make = (n: Integer) => { let values: InlineArray<Integer, n> = [] }\n",
	] {
		let (session, file) = crate::analyze_source(source);
		assert!(
			session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.message.contains("capacity")),
			"{source}"
		);
	}
}

#[test]
fn inline_push_preserves_item_references() {
	analyzed(
		"mutable values: InlineArray<Integer, 3> = [1]\nuse first = values[0]\nvalues.push(2)\nfirst += 10\nconsole.log(values)\n",
	);
}

#[test]
fn inline_delete_invalidates_only_the_shifted_suffix() {
	analyzed(
		"mutable values: InlineArray<Integer, 3> = [1, 2, 3]\nuse first = values[0]\nvalues.delete(1)\nconsole.log(first)\n",
	);
	assert!(codes("mutable values: InlineArray<Integer, 3> = [1, 2, 3]\nuse last = values[2]\nvalues.delete(1)\nconsole.log(last)\n").contains(&"REF001"));
}

#[test]
fn inline_reference_items_are_rejected() {
	assert!(
		codes(
			"mutable nodes: GrowingPool<Integer> = []\nlet values: InlineArray<Integer in nodes, 4> = []\n"
		)
		.contains(&"REF010")
	);
}

#[test]
fn inline_unique_category_is_inherited_from_items() {
	let (session, file) = analyzed("let values: InlineArray<Task<Integer>, 1> = []\n");
	let analysis = session.analysis(file);
	let ty = analysis
		.symbols
		.symbols()
		.iter()
		.find(|symbol| session.types().syms.resolve(symbol.name) == "values")
		.expect("values")
		.ty;
	assert!(session.types().owns_unique(ty));
}

#[test]
fn inline_aliased_reference_items_are_rejected() {
	assert!(codes("mutable nodes: GrowingPool<Integer> = []\ntype Borrow = Integer in nodes\nlet values: InlineArray<Borrow?, 4> = []\n").contains(&"REF010"));
}

#[test]
fn inline_capacity_survives_compile_time_evaluation() {
	let (session, file) = analyzed(
		"function build = (): InlineArray<Integer, 4> => [1]\nstatic values = @build()\nstatic capacity = values.capacity\nconsole.log(capacity)\n",
	);
	assert_eq!(
		symbol_type(&session, file, "values"),
		"InlineArray<Integer, 4>"
	);
	assert_eq!(symbol_type(&session, file, "capacity"), "4");
}

#[test]
fn inline_capacity_can_be_a_compile_time_parameter() {
	analyzed(
		"function empty = <Item, capacity: UInt32>(): InlineArray<Item, capacity> => []\nlet values = empty<Integer, 4>()\nconsole.log(values.capacity)\n",
	);
}

#[test]
fn inline_bounded_while_proves_pushes_and_the_final_index() {
	let source = "mutable values: InlineArray<Integer, 4> = []\nwhile values.length < 4 { values.push(values.length) }\nconsole.log(values[3])\n";
	let (session, file) = analyzed(source);
	let analysis = session.analysis(file);
	assert!(!analysis.has_flags(
		node(&session, file, "values.push(values.length)"),
		NodeFlags::CAPACITY_CHECK
	));
	assert!(!analysis.has_flags(node(&session, file, "values[3]"), NodeFlags::INDEX_CHECK));
}

#[test]
fn inline_capacity_alone_is_substituted() {
	let (session, file) = analyzed(
		"function empty = <capacity: UInt32>(): InlineArray<Integer, capacity> => []\nmutable values = empty<2>()\nfunction append = <capacity: UInt32>(items: InlineArray<Integer, capacity>) throws CapacityError mutates items.length => { try items.push(9) }\ntry append<2>(values)\n",
	);
	assert_eq!(
		symbol_type(&session, file, "values"),
		"InlineArray<Integer, 2>"
	);
}

#[test]
fn inline_arrays_do_not_inherit_scalar_ordering() {
	for item in ["Integer", "String", "Int64"] {
		for operation in ["<", "<=", ">", ">="] {
			let ordinary = codes(&format!(
				"function compare = (a: Array<{item}>, b: Array<{item}>): Boolean => a {operation} b\n"
			));
			let inline = codes(&format!(
				"function compare = (a: InlineArray<{item}, 2>, b: InlineArray<{item}, 2>): Boolean => a {operation} b\n"
			));
			assert!(!ordinary.is_empty());
			assert_eq!(inline, ordinary, "{item} {operation}");
		}
	}
}
