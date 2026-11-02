//! The layout and the canonical forms of the formatter, on small examples.

use fa_formatter::{Options, format, format_with, is_formatted};
use fa_parser::parse;

/// Format `source`, check the result against `expected`, and check that formatting it again
/// changes nothing.
fn assert_formats(source: &str, expected: &str) {
	let formatted = format(source).unwrap_or_else(|errors| {
		panic!("refused:\n{source}\n{}", errors[0].message);
	});
	assert_eq!(
		formatted, expected,
		"\n--- source\n{source}\n--- formatted\n{formatted}"
	);
	assert_eq!(
		format(&formatted).as_deref(),
		Ok(formatted.as_str()),
		"not idempotent"
	);
}

/// The comments of a source, without their spaces.
fn comments(source: &str) -> Vec<String> {
	let tree = parse(source);
	tree.comments
		.iter()
		.map(|span| source[span.start..span.end].split_whitespace().collect())
		.collect()
}

/// Format `source` and check that every comment is kept, that the code means the same, and
/// that formatting is idempotent.
fn assert_keeps_comments(source: &str) -> String {
	let options = Options {
		rewrites: false,
		..Options::default()
	};
	let formatted = format_with(source, &options).expect("the source should be formatted");
	let tree = parse(&formatted);
	assert!(
		!tree.has_errors(),
		"the formatted code has errors:\n{formatted}\n{:?}",
		tree.diagnostics
	);
	assert_eq!(tree.to_string(), parse(source).to_string(), "\n{formatted}");
	assert_eq!(comments(&formatted), comments(source), "\n{formatted}");
	assert_eq!(
		format_with(&formatted, &options).as_deref(),
		Ok(formatted.as_str()),
		"not idempotent"
	);
	formatted
}

/* --------------------------------- Layout --------------------------------- */

#[test]
fn indents_with_tabs_and_spaces_operators() {
	assert_formats(
		"function   add=(a:Integer,b:Integer):Integer=>{\n    let c=a+b*2\n  c-1\n}\n",
		"function add = (a: Integer, b: Integer): Integer => {\n\tlet c = a + b * 2\n\tc - 1\n}\n",
	);
}

#[test]
fn spaces_out_a_subtraction() {
	assert_formats("let q = a-1\n", "let q = a - 1\n");
}

#[test]
fn keeps_at_most_one_blank_line() {
	assert_formats(
		"\n\nlet a = 1\n\n\n\nlet b = 2\nlet c = 3\n\n\n",
		"let a = 1\n\nlet b = 2\nlet c = 3\n",
	);
	assert_formats(
		"if a {\n\n\tb()\n\n\n\tc()\n\n}\n",
		"if a {\n\tb()\n\n\tc()\n}\n",
	);
}

#[test]
fn breaks_a_long_call_one_argument_per_line() {
	assert_formats(
		"let result = compute(firstArgumentWithALongName, secondArgumentWithALongName, thirdArgumentWithALongName)\n",
		"let result = compute(\n\tfirstArgumentWithALongName\n\tsecondArgumentWithALongName\n\tthirdArgumentWithALongName\n)\n",
	);
}

#[test]
fn breaks_long_parameters() {
	assert_formats(
		"function register = (name: String, email: String, age: Integer, address: Address, phone: String) => {\n\tsave()\n}\n",
		"function register = (\n\tname: String\n\temail: String\n\tage: Integer\n\taddress: Address\n\tphone: String\n) => {\n\tsave()\n}\n",
	);
}

#[test]
fn joins_short_lists_and_calls() {
	assert_formats("let sum = add(\n\t1\n\t2\n)\n", "let sum = add(1, 2)\n");
	assert_formats("let a = [ 1,2 , 3 ]\n", "let a = [1, 2, 3]\n");
}

#[test]
fn keeps_objects_and_lists_written_over_several_lines() {
	assert_formats(
		"let point: Point = {\n\tx = 1, y = 2\n}\nlet b = [\n\t1\n\t2\n]\n",
		"let point: Point = {\n\tx = 1\n\ty = 2\n}\nlet b = [\n\t1\n\t2\n]\n",
	);
	assert_formats(
		"let point: Point = { x = 1,\n\ty = 2 }\n",
		"let point: Point = { x = 1, y = 2 }\n",
	);
}

#[test]
fn breaks_a_long_object() {
	assert_formats(
		"let settings = Settings { host = \"localhost\", port = 8080, secure = false, timeout = 30, retries = 5 }\n",
		"let settings = Settings {\n\thost = \"localhost\"\n\tport = 8080\n\tsecure = false\n\ttimeout = 30\n\tretries = 5\n}\n",
	);
}

#[test]
fn breaks_a_long_chain_of_calls_before_each_dot() {
	assert_formats(
		"let names = users.filter((user) => user.isActive and user.age > 18).map((user) => user.name.toUpperCase())\n",
		"let names = users\n\t.filter((user) => user.isActive and user.age > 18)\n\t.map((user) => user.name.toUpperCase())\n",
	);
	assert_formats(
		"let names = users\n.filter((user) => user.isActive)\n\t.map((user) => user.name)\n",
		"let names = users\n\t.filter((user) => user.isActive)\n\t.map((user) => user.name)\n",
	);
}

#[test]
fn breaks_a_long_operation_before_its_operators() {
	assert_formats(
		"let isAllowed = user.isAdministrator or user.isOwnerOf(document) or document.sharedWith.includes(user.id)\n",
		"let isAllowed = user.isAdministrator\n\tor user.isOwnerOf(document)\n\tor document.sharedWith.includes(user.id)\n",
	);
}

#[test]
fn hugs_a_last_lambda_with_a_block() {
	assert_formats(
		"items.forEach((item) => {\nconsole.log(item)\n})\n",
		"items.forEach((item) => {\n\tconsole.log(item)\n})\n",
	);
	assert_formats(
		"setTimer((item) => {\nconsole.log(item)\n}, 30)\n",
		"setTimer((item) => {\n\tconsole.log(item)\n}, 30)\n",
	);
}

#[test]
fn hugs_a_list_or_an_object_written_over_several_lines() {
	assert_formats(
		"try await Task.all([\nstart load(a)\nstart load(b)\n])\n",
		"try await Task.all([\n\tstart load(a)\n\tstart load(b)\n])\n",
	);
	assert_formats(
		"configure({\nname = \"farm\"\n})\n",
		"configure({\n\tname = \"farm\"\n})\n",
	);
}

#[test]
fn breaks_the_parameters_before_the_return_type() {
	assert_formats(
		"function logRequest(Context: { ...Request, ...Timestamp }) = (request: Context): { ...Context, ...Logged } => {\n\trequest\n}\n",
		"function logRequest(Context: { ...Request, ...Timestamp }) = (\n\trequest: Context\n): { ...Context, ...Logged } => {\n\trequest\n}\n",
	);
}

#[test]
fn keeps_a_when_of_one_branch_on_one_line() {
	assert_formats(
		"type NoMatch(V) = when V is { String => Integer }\n",
		"type NoMatch(V) = when V is { String => Integer }\n",
	);
	assert_formats(
		"let a = when r is { Shape.Circle => 1, else => 2 }\n",
		"let a = when r is {\n\tShape.Circle => 1\n\telse => 2\n}\n",
	);
}

#[test]
fn moves_a_long_lambda_body_to_the_next_line() {
	assert_formats(
		"function createFarm = (name: CString): Pointer(Farm) => Pointer.allocate(Farm { name = name.toString() })\n",
		"function createFarm = (name: CString): Pointer(Farm) =>\n\tPointer.allocate(Farm { name = name.toString() })\n",
	);
}

#[test]
fn keeps_a_one_line_block_of_one_statement() {
	assert_formats(
		"let label = if score >= 50 {   \"pass\" } else { \"fail\" }\n",
		"let label = if score >= 50 { \"pass\" } else { \"fail\" }\n",
	);
	assert_formats("if a { b() }\n", "if a { b() }\n");
	assert_formats("if a {\n\tb()\n}\n", "if a {\n\tb()\n}\n");
	assert_formats("for item in items { }\n", "for item in items {}\n");
}

#[test]
fn writes_when_branches_on_their_own_lines() {
	assert_formats(
		"let label = when shape is { Shape.Circle as { radius } => radius, else => 0 }\n",
		"let label = when shape is {\n\tShape.Circle as { radius } => radius\n\telse => 0\n}\n",
	);
}

#[test]
fn separates_implicit_members_with_commas_on_several_lines() {
	// a line starting with `.` continues the previous one
	assert_formats(
		"let colors: Array(Color) = [\n\t.red\n\t, .green\n]\nlet name = when color is { .red => \"red\", .green => \"green\" }\n",
		"let colors: Array(Color) = [\n\t.red,\n\t.green\n]\nlet name = when color is {\n\t.red => \"red\",\n\t.green => \"green\"\n}\n",
	);
}

#[test]
fn keeps_literals_as_written() {
	assert_formats(
		"let a = 0xFF\nlet b = 1_000_000\nlet c = 1.5e9\nlet d = 12n\nlet e = \"tab\\t{name} {{x}}\"\nlet f = 50%\n",
		"let a = 0xFF\nlet b = 1_000_000\nlet c = 1.5e9\nlet d = 12n\nlet e = \"tab\\t{name} {{x}}\"\nlet f = 50%\n",
	);
}

#[test]
fn keeps_a_multi_line_string() {
	assert_formats(
		"if a {\n  let message = \"First line\n  Second line\"\n}\n",
		"if a {\n\tlet message = \"First line\n  Second line\"\n}\n",
	);
}

#[test]
fn normalizes_line_endings() {
	assert_formats("let a = 1\r\nlet b = 2\r\n", "let a = 1\nlet b = 2\n");
}

#[test]
fn never_glues_a_minus_to_a_number_or_a_minus() {
	assert_keeps_comments("let a = - 5\nlet b = - -x\nlet c = -(-x)\nlet d = -x\n");
	assert_formats("let a = - -x\n", "let a = - -x\n");
}

#[test]
fn formats_an_empty_source() {
	assert_formats("", "");
	assert_formats("\n\n", "");
}

#[test]
fn refuses_code_with_syntax_errors() {
	let errors = format("let a = (1 +\n").expect_err("a syntax error");
	assert!(!errors.is_empty());
	assert!(!is_formatted("let a = (\n"));
	assert!(is_formatted("let a = 1\n"));
}

/* ------------------------------- Rewrites ------------------------------- */

#[test]
fn rewrites_a_percentage_call() {
	assert_formats(
		"let total = Percentage(width + margin)\nlet half = Percentage(50)\nlet same = Percentage((x))\n",
		"let total = (width + margin)%\nlet half = 50%\nlet same = (x)%\n",
	);
}

#[test]
fn rewrites_a_test_of_a_range_written_in_place() {
	assert_formats(
		"let a = score is in 0..60\nlet b = score is not in 0..=60\nlet c = score is in allowed\n",
		"let a = score is 0..60\nlet b = score is not 0..=60\nlet c = score is in allowed\n",
	);
}

#[test]
fn writes_the_parentheses_of_a_negative_literal_before_a_method() {
	assert_formats("let a = -2.4.abs()\n", "let a = (-2.4).abs()\n");
}

#[test]
fn writes_the_field_shorthand() {
	assert_formats(
		"let person = Person { name = name, age = 31 }\n",
		"let person = Person { name, age = 31 }\n",
	);
}

#[test]
fn removes_the_type_repeated_before_an_object_literal() {
	assert_formats(
		"let point: Point(Integer) = Point(Integer) { x = 1, y = 2 }\nlet other: Point = Origin { x = 1 }\n",
		"let point: Point(Integer) = { x = 1, y = 2 }\nlet other: Point = Origin { x = 1 }\n",
	);
}

#[test]
fn writes_an_anonymous_structure_parameter() {
	assert_formats(
		"function greet = ({ name, greeting = \"Hello\" }: { name: String, greeting: String }) => {\n\tconsole.log(\"{greeting}, {name}!\")\n}\n",
		"function greet = ({ name: String, greeting: String = \"Hello\" }) => {\n\tconsole.log(\"{greeting}, {name}!\")\n}\n",
	);
	// a named type, or a renamed field, stays destructured
	assert_formats(
		"function f = ({ x, y }: Point, { a as b }: { a: Integer }) => x\n",
		"function f = ({ x, y }: Point, { a as b }: { a: Integer }) => x\n",
	);
}

#[test]
fn keeps_the_tree_without_the_rewrites() {
	let options = Options {
		rewrites: false,
		..Options::default()
	};
	let source = "let a = Percentage(x)\nlet b = s is in 0..60\nlet c = { id = id }\n";
	assert_eq!(format_with(source, &options).as_deref(), Ok(source));
}

/* ------------------------------- Comments ------------------------------- */

#[test]
fn keeps_leading_and_trailing_comments() {
	assert_formats(
		"-- first\n\n---\nDocumentation.\n---\nfunction add = (a: Integer, b: Integer) => a + b   -- the sum\n-- last\n",
		"-- first\n\n---\nDocumentation.\n---\nfunction add = (a: Integer, b: Integer) => a + b -- the sum\n-- last\n",
	);
}

#[test]
fn aligns_the_comments_that_end_consecutive_lines() {
	assert_formats(
		"let count = 8 -- items\nlet maximum = 1000 -- per queue\nlet t = 30 -- ms\n",
		"let count = 8      -- items\nlet maximum = 1000 -- per queue\nlet t = 30         -- ms\n",
	);
	// already aligned, with more spaces than needed
	assert_formats(
		"let a = 1         -- one\nlet bb = 2    -- two\n",
		"let a = 1  -- one\nlet bb = 2 -- two\n",
	);
	// in a block, the fields of an object
	assert_formats(
		"let pool = {\n\tcount = 8\n\tmaximum = 1000 -- queued\n\ttimeout = 30 -- ms\n}\n",
		"let pool = {\n\tcount = 8\n\tmaximum = 1000 -- queued\n\ttimeout = 30   -- ms\n}\n",
	);
}

#[test]
fn a_blank_line_or_a_line_without_comment_ends_the_aligned_comments() {
	assert_formats(
		"let a = 1 -- one\nlet bbbb = 2 -- two\n\nlet c = 3 -- three\nlet d = 4\nlet eeeee = 5 -- five\nlet f = 6 -- six\n",
		"let a = 1    -- one\nlet bbbb = 2 -- two\n\nlet c = 3 -- three\nlet d = 4\nlet eeeee = 5 -- five\nlet f = 6     -- six\n",
	);
	// nor a change of indentation (the alignment would depend on the width of a tab)
	assert_formats(
		"if a { -- open\n\tb() -- call\n\tccc() -- other\n}\n",
		"if a { -- open\n\tb()   -- call\n\tccc() -- other\n}\n",
	);
	// a comment on its own line is not at the end of a line of code
	assert_formats(
		"let a = 1 -- one\n-- alone\nlet bbbb = 2 -- two\n",
		"let a = 1 -- one\n-- alone\nlet bbbb = 2 -- two\n",
	);
	// nor a comment followed by code, nor a comment over several lines
	assert_formats(
		"let a = f(x --- x ---, y) -- one\nlet bbbbbbbbbbbbbbbbbbbbbbbbb = 2 -- two\n",
		"let a = f(x, --- x --- y)         -- one\nlet bbbbbbbbbbbbbbbbbbbbbbbbb = 2 -- two\n",
	);
}

#[test]
fn keeps_the_blank_line_that_separates_a_comment_from_a_declaration() {
	assert_formats(
		"-- for maintainers\n\n\nfunction add = (a: Integer) => a\n",
		"-- for maintainers\n\nfunction add = (a: Integer) => a\n",
	);
}

#[test]
fn reindents_a_block_comment() {
	assert_formats(
		"if a {\n        ---\n        A block comment\n            indented more\n        ---\n        b()\n}\n",
		"if a {\n\t---\n\tA block comment\n\t    indented more\n\t---\n\tb()\n}\n",
	);
}

#[test]
fn keeps_a_comment_after_an_opening_bracket() {
	assert_formats(
		"for item, index in 0..3 { -- error: E-LOOP-3\n\tconsole.log(item)\n}\n",
		"for item, index in 0..3 { -- error: E-LOOP-3\n\tconsole.log(item)\n}\n",
	);
	assert_formats("if a { -- nothing\n}\n", "if a { -- nothing\n}\n");
	assert_formats("let a = f( -- nothing\n)\n", "let a = f( -- nothing\n)\n");
}

#[test]
fn keeps_the_comments_of_lists_and_calls() {
	let formatted = assert_keeps_comments(
		"let a = [1, -- one\n\t2 --- two ---, 3]\nlet b = f(\n\t-- before\n\tx, -- after x\n\ty\n\t-- at the end\n)\n",
	);
	assert_eq!(
		formatted,
		"let a = [\n\t1 -- one\n\t2 --- two ---\n\t3\n]\nlet b = f(\n\t-- before\n\tx -- after x\n\ty\n\t-- at the end\n)\n"
	);
}

#[test]
fn keeps_the_comments_inside_expressions() {
	assert_keeps_comments("let total = first + -- the first\n\tsecond + --- inline --- third\n");
	assert_keeps_comments(
		"let names = users -- all\n\t.filter((user) => user.isActive) -- active\n\t.map((user) => user.name)\n",
	);
	assert_keeps_comments("let a = -- the value\n\t5\n");
	assert_keeps_comments(
		"function f = (a: Integer -- a\n, b: Integer) -- after\n\t=> -- arrow\n\ta + b\n",
	);
	assert_keeps_comments(
		"let x = when a is { -- subject\n\t1 => b -- one\n\t-- other\n\telse => c\n} -- end\n",
	);
	assert_keeps_comments(
		"if a { b } -- then\nelse { c }\n"
			.replace("\nelse", " else")
			.as_str(),
	);
	assert_keeps_comments("let { a, -- a\n\tb } = c\n");
	assert_keeps_comments("type T = { -- t\n\ta: Integer -- a\n\t--- b --- b: String\n}\n");
	assert_keeps_comments("let s = \"{a}\" ---a------b---\n");
}

#[test]
fn keeps_the_comments_of_an_empty_block() {
	assert_keeps_comments("function f = () => {\n\t-- nothing yet\n}\n");
	assert_keeps_comments("let f = when a is {\n\t-- nothing yet\n}\n");
	assert_keeps_comments("-- only a comment\n");
	assert_keeps_comments("--- only a block comment ---");
}
