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
	assert_statement("x = 5", "let x = 5");
	assert_statement("value = true", "let value = true");

	// Value declarations with expressions
	assert_statement("x = a + b", "let x = (a + b)");
	assert_statement("y = a * b + c", "let y = ((a * b) + c)");

	// Value declarations with complex expressions
	assert_statement("result = a and b or c", "let result = ((a and b) or c)");
	assert_statement(
		"comparison = a < b and c > d",
		"let comparison = ((a < b) and (c > d))"
	);
}

#[test]
fn value_declarations_with_type() {
	// Simple type annotations
	assert_statement("x: Int", "let x: Int");
	assert_statement("name: String", "let name: String");
	assert_statement("flag: Boolean", "let flag: Boolean");

	// // Type annotations with generic types
	// assert_statement("values: Array(Int)", "let values: Array(Int)");
	// assert_statement("dict: Map(String, Int)", "let dict: Map(String, Int)");

	// // Type annotations with complex types
	// assert_statement(
	// 	"callback: Function(Int, String) -> Boolean",
	// 	"let callback: Function(Int, String) -> Boolean"
	// );
	// assert_statement("nested: Option(Array(Int))", "let nested: Option(Array(Int))");
}

#[test]
fn value_declarations_with_type_and_value() {
	// Simple declarations with type and value
	assert_statement("x: Int = 5", "let x: Int = 5");
	assert_statement("flag: Boolean = true", "let flag: Boolean = true");

	// // Declarations with generic types and values
	// assert_statement("values: Array(Int) = [1, 2, 3]", "let values: Array(Int) = [1, 2, 3]");
	// assert_statement(
	// 	"pair: Tuple(Int, String) = (1, \"one\")",
	// 	"let pair: Tuple(Int, String) = (1, \"one\")"
	// );

	// Declarations with expressions
	assert_statement("sum: Int = a + b * c", "let sum: Int = (a + (b * c))");
	assert_statement(
		"valid: Boolean = x > 0 and y < 10",
		"let valid: Boolean = ((x > 0) and (y < 10))"
	);
}

// #[test]
// fn complex_value_declarations() {
// 	// Complex type annotations with nested generics
// 	assert_statement(
// 		"x: Generic(Toto, A + B * z) = 12 - 4 * 3",
// 		"let x: Generic(Toto, (A + (B * z))) = (12 - (4 * 3))"
// 	);
// 	assert_statement(
// 		"matrix: Array(Array(Int)) = [[1, 2], [3, 4]]",
// 		"let matrix: Array(Array(Int)) = [[1, 2], [3, 4]]"
// 	);

// 	// Complex type annotations with function types
// 	assert_statement(
// 		"transform: Function(A) -> Map(B, C) = a |> process |> convert",
// 		"let transform: Function(A) -> Map(B, C) = (a |> process |> convert)"
// 	);

// 	// Type annotations with union types
// 	assert_statement(
// 		"result: Success | Error = compute()",
// 		"let result: (Success | Error) = compute()"
// 	);
// 	assert_statement(
// 		"value: Option(Int | String) = getValue()",
// 		"let value: Option((Int | String)) = getValue()"
// 	);

// 	// Complex expressions with multiple operations
// 	assert_statement(
// 		"calculation: Number = (a + b) * (c - d) / e ** f",
// 		"let calculation: Number = ((((a + b)) * ((c - d))) / (e ** f))"
// 	);
// 	assert_statement(
// 		"complex: Type(A | B, C & D) = x << y >> z",
// 		"let complex: Type((A | B), (C & D)) = ((x << y) >> z)"
// 	);
// }
