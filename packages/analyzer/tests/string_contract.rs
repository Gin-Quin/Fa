//! The string redesign's public facts, tested on the exact expression rather
//! than the union of all flags on its source line.
use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::analyze_source;
use fa_analyzer::diagnostics::Severity;
use fa_analyzer::ids::NodeId;

fn assert_flags(source: &str, expression: &str, present: NodeFlags, absent: NodeFlags) {
	let (session, file) = analyze_source(source);
	let errors: Vec<_> = session
		.diagnostics(file)
		.iter()
		.filter(|d| d.severity == Severity::Error)
		.collect();
	assert!(errors.is_empty(), "{source}\n{errors:#?}");
	let tree = session.tree(file);
	let nodes: Vec<_> = (0..tree.nodes.len())
		.filter(|&n| tree.text(n) == expression)
		.collect();
	assert!(!nodes.is_empty(), "no expression {expression}");
	for n in nodes {
		let flags = session.analysis(file).flags_of(NodeId::new(n));
		assert!(
			flags.contains(present),
			"{expression}: {flags:?} lacks {present:?}"
		);
		assert!(
			!flags.intersects(absent),
			"{expression}: {flags:?} contains {absent:?}"
		);
	}
}

#[test]
fn direct_counts_and_encodings_are_identified_by_resolution() {
	for receiver in ["text", "(text)", "holder.text", "(holder.text)"] {
		let context = "let text = \"a😀\"\nlet holder = { text = \"a😀\" }\n";
		for call in [
			format!("{receiver}.codePoints()"),
			format!("({receiver}.codePoints())"),
			format!("({receiver}.codePoints)()"),
		] {
			let expression = format!("{call}.length");
			assert_flags(
				&format!("{context}console.log({expression})"),
				&expression,
				NodeFlags::CODE_POINT_COUNT,
				NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
			);
		}
		let call = format!("{receiver}.toUtf8()");
		assert_flags(
			&format!("{context}for byte in ({call}) {{ console.log(byte) }}"),
			&call,
			NodeFlags::UTF8_ITERATION,
			NodeFlags::CAN_FAIL,
		);
		let grouped_method = format!("({receiver}.toUtf8)()");
		assert_flags(
			&format!("{context}for byte in {grouped_method} {{ console.log(byte) }}"),
			&grouped_method,
			NodeFlags::UTF8_ITERATION,
			NodeFlags::CAN_FAIL,
		);
	}
	assert_flags(
		"let text = \"a😀\"\nlet points = text.codePoints()\nconsole.log(points.length)",
		"points.length",
		NodeFlags::empty(),
		NodeFlags::CODE_POINT_COUNT,
	);
	assert_flags(
		"let text = \"a😀\"\nlet bytes = text.toUtf8()\nfor byte in bytes { console.log(byte) }",
		"text.toUtf8()",
		NodeFlags::empty(),
		NodeFlags::UTF8_ITERATION,
	);
	assert_flags(
		"let text = { codePoints = () => [1], toUtf8 = () => [1] }\nconsole.log(text.codePoints().length)\nfor byte in text.toUtf8() { console.log(byte) }",
		"text.codePoints().length",
		NodeFlags::empty(),
		NodeFlags::CODE_POINT_COUNT,
	);
	assert_flags(
		"let text = { toUtf8 = () => [1] }\nfor byte in text.toUtf8() { console.log(byte) }",
		"text.toUtf8()",
		NodeFlags::empty(),
		NodeFlags::UTF8_ITERATION,
	);
	assert_flags(
		"let text = \"a😀\"\nlet points = text.codePoints\nconsole.log(points().length)",
		"points().length",
		NodeFlags::empty(),
		NodeFlags::CODE_POINT_COUNT,
	);
	assert_flags(
		"let text = \"a😀\"\nlet encode = text.toUtf8\nfor byte in encode() { console.log(byte) }",
		"encode()",
		NodeFlags::empty(),
		NodeFlags::UTF8_ITERATION,
	);
}

#[test]
fn scalar_index_facts_do_not_leak_to_slices_or_arrays() {
	for expression in ["text[0]", "(text)[0]", "holder.text[0]"] {
		assert_flags(
			&format!(
				"let text = \"😀\"\nlet holder = {{ text = \"😀\" }}\nconsole.log(try {expression})"
			),
			expression,
			NodeFlags::STRING_CODE_UNIT,
			NodeFlags::empty(),
		);
	}
	assert_flags(
		"let text = \"😀\"\nconsole.log(text[0..1])",
		"text[0..1]",
		NodeFlags::empty(),
		NodeFlags::STRING_CODE_UNIT | NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
	);
	assert_flags(
		"let array = [\"long\"]\nconsole.log(array[0])",
		"array[0]",
		NodeFlags::empty(),
		NodeFlags::STRING_CODE_UNIT,
	);
	assert_flags(
		"let text = \"Fa\"\nwhen text[0] is { \"F\" => console.log(1), else => console.log(2) }",
		"text[0]",
		NodeFlags::STRING_CODE_UNIT,
		NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
	);
	assert_flags(
		"let text = \"Fa\"\nuse slice = text[0..1]\nconsole.log(try slice[0])",
		"slice[0]",
		NodeFlags::STRING_CODE_UNIT,
		NodeFlags::empty(),
	);
}

#[test]
fn method_bounds_and_flags_follow_the_actual_receiver() {
	for callee in ["text.codeUnitAt", "(text.codeUnitAt)", "(text).codeUnitAt"] {
		let expression = format!("{callee}(0)");
		assert_flags(
			&format!("let text = \"😀\"\nconsole.log({expression})"),
			&expression,
			NodeFlags::empty(),
			NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
		);
	}
	for callee in ["holder.text.codeUnitAt", "(holder.text).codeUnitAt"] {
		let expression = format!("{callee}(i)");
		assert_flags(
			&format!(
				"function read = (holder: {{ text: String }}, i: Integer) => try {expression}"
			),
			&expression,
			NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
			NodeFlags::empty(),
		);
	}
	assert_flags(
		"let text = \"😀\"\nlet at = text.codeUnitAt\nconsole.log(try at(0))",
		"at(0)",
		NodeFlags::CAN_FAIL,
		NodeFlags::INDEX_CHECK,
	);
}

#[test]
fn generic_bodies_keep_string_facts() {
	let source = "function count = <T>(unused: T, text: String) => text.codePoints().length\nconsole.log(count(1, \"😀\"))";
	assert_flags(
		source,
		"text.codePoints().length",
		NodeFlags::CODE_POINT_COUNT,
		NodeFlags::empty(),
	);
	let source = "function visit = <T>(unused: T, text: String) => { for byte in text.toUtf8() { console.log(byte) } }\nvisit(1, \"😀\")";
	assert_flags(
		source,
		"text.toUtf8()",
		NodeFlags::UTF8_ITERATION,
		NodeFlags::empty(),
	);
}

#[test]
fn utf8_literal_length_proofs_survive_groups() {
	for expression in [
		"\"é😀\".toUtf8()",
		"(\"é😀\").toUtf8()",
		"(\"é😀\".toUtf8)()",
	] {
		assert_flags(
			&format!("let bytes = {expression}\nconsole.log(bytes[5])"),
			"bytes[5]",
			NodeFlags::empty(),
			NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
		);
	}
}

fn assert_diagnostic(source: &str, code: &str, span: &str, hints: &[&str]) {
	let (session, file) = analyze_source(source);
	let diagnostics = session.diagnostics(file);
	let diagnostic = diagnostics
		.iter()
		.find(|d| d.code.id == code)
		.unwrap_or_else(|| panic!("missing {code}: {diagnostics:#?}"));
	let location = diagnostic.location.span;
	assert_eq!(
		&source[location.start as usize..location.end as usize],
		span
	);
	for hint in hints {
		assert!(
			diagnostic.message.contains(hint),
			"{hint:?} missing: {}",
			diagnostic.message
		);
	}
}

#[test]
fn new_diagnostics_explain_the_fix_at_the_rejected_expression() {
	assert_diagnostic(
		"let unit = \"😀\"[2]",
		"E-NUM-17",
		"\"😀\"[2]",
		&["length is 2", "get(index)", "length - 1"],
	);
	assert_diagnostic(
		"function read = (text: String, index: Integer) => text[index]",
		"E-ERR-3",
		"text[index]",
		&["IndexError", "try", "catch"],
	);
	assert_diagnostic(
		"let decoded = String.fromUtf8(Bytes.from([255]))",
		"E-ERR-3",
		"String.fromUtf8(Bytes.from([255]))",
		&["Utf8Error", "try", "catch"],
	);
	assert_diagnostic(
		"mutable bytes = Bytes.filled(2)\nbytes.push(1)",
		"E-OBJ-5",
		"push",
		&["length is fixed", "Array<Byte>", "Bytes.from(items)"],
	);
	assert_diagnostic(
		"function send = (data: Bytes) => console.log(data)\nsend(\"hello\")",
		"E-TYPE-1",
		"\"hello\"",
		&["toUtf8()"],
	);
	assert_diagnostic(
		"function print = (text: String) => console.log(text)\nprint(Bytes.from([104]))",
		"E-TYPE-1",
		"Bytes.from([104])",
		&["String.fromUtf8", "Utf8Error"],
	);
	assert_diagnostic(
		"use { File } = std.fs\nlet text: String = try File.read(\"file\")",
		"E-TYPE-1",
		"try File.read(\"file\")",
		&["readText", "String.fromUtf8"],
	);
	assert_diagnostic(
		"use { File } = std.fs\nFile.readBytes(\"file\")",
		"AN001",
		"readBytes",
		&["File.read(path)", "Bytes", "readText"],
	);
	assert_diagnostic(
		"let schema = Schema.Character",
		"E-OBJ-5",
		"Character",
		&["Schema.String", "Schema.Bytes"],
	);
	assert_diagnostic(
		"type Retired = Schema.Character",
		"TY007",
		"Character",
		&["Schema.String", "Schema.Bytes"],
	);
}

#[test]
fn locale_data_is_refused_at_compile_time_with_a_runtime_fix() {
	assert_diagnostic(
		"static order = String.compareLocale(\"a\", \"b\")",
		"META002",
		"String.compareLocale(\"a\", \"b\")",
		&["locale data", "runtime"],
	);
	assert_flags(
		"let order = String.compareLocale(\"a\", \"b\")\nconsole.log(order)",
		"String.compareLocale(\"a\", \"b\")",
		NodeFlags::empty(),
		NodeFlags::CAN_FAIL,
	);
}

#[test]
fn compile_time_operations_keep_semantic_flags_without_runtime_errors() {
	assert_flags(
		"static unit = \"ab\"[0]\nconsole.log(unit)",
		"\"ab\"[0]",
		NodeFlags::STRING_CODE_UNIT,
		NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL,
	);
	assert_flags(
		"static count = \"a😀\".codePoints().length\nconsole.log(count)",
		"\"a😀\".codePoints().length",
		NodeFlags::CODE_POINT_COUNT,
		NodeFlags::CAN_FAIL,
	);
}

#[test]
fn editor_surface_has_no_retired_type_or_byte_view() {
	for (source, expected, absent) in [
		(
			"let text = \"😀\"\ntext.\n",
			&[
				"get",
				"codeUnitAt",
				"characters",
				"codePoints",
				"utf8Length",
				"toUtf8",
			][..],
			&["bytes"][..],
		),
		(
			"let bytes = Bytes.filled(2)\nbytes.\n",
			&["length", "get", "toArray", "toString"][..],
			&["push", "insert", "delete", "reserve", "capacity"][..],
		),
		(
			"use { File } = std.fs\nFile.\n",
			&["read", "readText", "write", "writeText"][..],
			&["readBytes"][..],
		),
	] {
		let (session, file) = analyze_source(source);
		let completions = session.completions(file, source.len() as u32 - 1);
		for name in expected {
			assert!(
				completions.iter().any(|item| item.label == *name),
				"missing {name}: {completions:?}"
			);
		}
		for name in absent {
			assert!(
				!completions.iter().any(|item| item.label == *name),
				"retired {name}: {completions:?}"
			);
		}
	}
	for (source, name, expected) in [
		(
			"let text = \"😀\"\nlet optional = text.get(0)",
			"optional",
			"String?",
		),
		(
			"for point in \"😀\" { console.log(point) }",
			"point",
			"String",
		),
		("let encoded = \"😀\".toUtf8()", "encoded", "Bytes"),
		("let unit = \"😀\".codeUnitAt(0)", "unit", "UInt16"),
	] {
		let (session, file) = analyze_source(source);
		let hover = session
			.hover(file, source.find(name).unwrap() as u32)
			.unwrap();
		assert!(hover.markdown.contains(expected), "{}", hover.markdown);
		assert!(!hover.markdown.contains("Character"));
		assert!(!hover.markdown.contains("grapheme"));
	}
}
