use crate::parse::parse_single_statement;

#[cfg(test)]
// Helper function to check if statement parsing results match expected output
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn value_declarations_with_value() {
	// Simple value declarations
	// assert_statement("x = 5", "x = 5");
	// assert_statement("value = true", "value = true");

	// // Value declarations with expressions
	// assert_statement("x = a + b", "x = (a + b)");
	// assert_statement("y = a * b + c", "y = ((a * b) + c)");

	// // Value declarations with complex expressions
	assert_statement("result = a and b or c", "result = ((a and b) or c)");
	assert_statement(
		"comparison = a < b and c > d",
		"comparison = ((a < b) and (c > d))",
	);
}

#[test]
fn value_declarations_with_type() {
	// Simple type annotations
	assert_statement("x: Int", "x: Int");
	assert_statement("name: String", "name: String");
	assert_statement("flag: Boolean", "flag: Boolean");

	// // Type annotations with generic types
	// assert_statement("values: Array(Int)", "values: Array(Int)");
	// assert_statement("dict: Map(String, Int)", "dict: Map(String, Int)");

	// // Type annotations with complex types
	// assert_statement(
	// 	"callback: Function(Int, String) -> Boolean",
	// 	"callback: Function(Int, String) -> Boolean"
	// );
	// assert_statement("nested: Option(Array(Int))", "nested: Option(Array(Int))");
}

#[test]
fn value_declarations_with_type_and_value() {
	// Simple declarations with type and value
	assert_statement("x: Int = 5", "x: Int = 5");
	assert_statement("flag: Boolean = true", "flag: Boolean = true");

	// // Declarations with generic types and values
	// assert_statement("values: Array(Int) = [1, 2, 3]", "values: Array(Int) = [1, 2, 3]");
	// assert_statement(
	// 	"pair: Tuple(Int, String) = (1, \"one\")",
	// 	"pair: Tuple(Int, String) = (1, \"one\")"
	// );

	// Declarations with expressions
	assert_statement("sum: Int = a + b * c", "sum: Int = (a + (b * c))");
	assert_statement(
		"valid: Boolean = x > 0 and y < 10",
		"valid: Boolean = ((x > 0) and (y < 10))",
	);
}

// #[test]
// fn complex_value_declarations() {
// 	// Complex type annotations with nested generics
// 	assert_statement(
// 		"x: Generic(Toto, A + B * z) = 12 - 4 * 3",
// 		"x: Generic(Toto, (A + (B * z))) = (12 - (4 * 3))"
// 	);
// 	assert_statement(
// 		"matrix: Array(Array(Int)) = [[1, 2], [3, 4]]",
// 		"matrix: Array(Array(Int)) = [[1, 2], [3, 4]]"
// 	);

// 	// Complex type annotations with function types
// 	assert_statement(
// 		"transform: Function(A) -> Map(B, C) = a |> process |> convert",
// 		"transform: Function(A) -> Map(B, C) = (a |> process |> convert)"
// 	);

// 	// Type annotations with union types
// 	assert_statement(
// 		"result: Success | Error = compute()",
// 		"result: (Success | Error) = compute()"
// 	);
// 	assert_statement(
// 		"value: Option(Int | String) = getValue()",
// 		"value: Option((Int | String)) = getValue()"
// 	);

// 	// Complex expressions with multiple operations
// 	assert_statement(
// 		"calculation: Number = (a + b) * (c - d) / e ** f",
// 		"calculation: Number = ((((a + b)) * ((c - d))) / (e ** f))"
// 	);
// 	assert_statement(
// 		"complex: Type(A | B, C & D) = x << y >> z",
// 		"complex: Type((A | B), (C & D)) = ((x << y) >> z)"
// 	);
// }
