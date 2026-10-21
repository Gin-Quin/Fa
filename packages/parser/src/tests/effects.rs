use crate::parsing::parse::parse;
use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn try_expressions() {
	assert_statement("let score = try loadScore()", "let score = try loadScore()");
	assert_statement(
		"let user = try await fetchUser(id)",
		"let user = try (await fetchUser(id))",
	);
	assert_statement(
		"let user = try database.findUser(id)",
		"let user = try database.findUser(id)",
	);
}

#[test]
fn throw_expressions() {
	assert_statement(
		"throw ApiError.NetworkError(\"Serious network issue\")",
		"throw ApiError.NetworkError(\"Serious network issue\")",
	);
	assert_statement(
		"if user is none { throw UserError.NotFound(id) }",
		"if (user is none) {\n\tthrow UserError.NotFound(id)\n}",
	);
}

#[test]
fn catch_expressions() {
	assert_statement(
		"let score = loadScore() catch error => 0",
		"let score = (loadScore() catch error => 0)",
	);
	assert_statement(
		"let score = loadScore()\n\tcatch error => 0",
		"let score = (loadScore() catch error => 0)",
	);
	assert_statement(
		"let score = loadScore() catch error => when error is {\nTimeoutError => 0\nelse => throw error\n}",
		"let score = (loadScore() catch error => when error is {\n\tTimeoutError => 0\n\telse => throw error\n})",
	);
	assert_statement(
		"let score = loadScore() catch error => {\nconsole.log(error)\n0\n}",
		"let score = (loadScore() catch error => {\n\tconsole.log(error)\n\t0\n})",
	);
}

#[test]
#[should_panic]
fn catch_requires_an_error_name() {
	parse_single_statement("let score = loadScore() catch => 0");
}

#[test]
fn async_expressions() {
	assert_statement(
		"let data = await fetchData(url)",
		"let data = (await fetchData(url))",
	);
	assert_statement(
		"let task = start fetchData(url)",
		"let task = (start fetchData(url))",
	);
	assert_statement(
		"start do { console.log(item) }",
		"(start do {\n\tconsole.log(item)\n})",
	);
	assert_statement(
		"let sums = await [start worker1.sum(), start worker2.sum()]",
		"let sums = (await [(start worker1.sum()), (start worker2.sum())])",
	);
	assert_statement("await a + b", "((await a) + b)");
}

#[test]
fn start_as_a_name() {
	assert_statement("range.start = 2", "range.start = 2");
	assert_statement("Range(start = 1, end = 3)", "Range(start = 1, end = 3)");
}

#[test]
fn provide_statements() {
	assert_statement(
		"provide Database = PostgresDatabase(\"postgres://localhost/app\")",
		"provide Database = PostgresDatabase(\"postgres://localhost/app\")",
	);
	assert_statement(
		"provide Session = { user = currentUser }",
		"provide Session = {\n\tuser = currentUser\n}",
	);
}

#[test]
fn assert_statements() {
	assert_statement("assert score == 42", "assert (score == 42)");
	assert_statement("assert user is not none", "assert (user is not none)");
}

#[test]
fn function_with_effects() {
	let tree = parse(
		"function getUser = (id: String): User throws DbError | UserError requires database: Database => {\n\tlet user = try database.findUser(id)\n\tif user is none {\n\t\tthrow UserError.NotFound(id)\n\t}\n\tuser\n}",
	);
	assert_eq!(
		tree.to_string(),
		"function getUser = (id: String): User throws (DbError | UserError) requires database: Database => {\n\tlet user = try database.findUser(id)\n\tif (user is none) {\n\tthrow UserError.NotFound(id)\n}\n\tuser\n};",
	);
}
