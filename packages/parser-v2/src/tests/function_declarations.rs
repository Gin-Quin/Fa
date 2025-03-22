use crate::parse::parse_single_statement;

#[cfg(test)]
// Helper function to check if statement parsing results match expected output
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn empty_parameters() {
	assert_statement(
		"function myFunction() => expression",
		"function myFunction() => expression"
	);
}

#[test]
fn empty_block() {
	assert_statement(
		"function myFunction(): Result { }",
		"function myFunction(): Result {\n\t\n}"
	);
}

#[test]
fn simple_declaration() {
	// Function with no parameters and expression body
	assert_statement(
		"function myFunction() => expression",
		"function myFunction() => expression"
	);
	assert_statement("function getValue() => 42", "function getValue() => 42");
	assert_statement("function isValid() => true", "function isValid() => true");
}

#[test]
fn with_return_type() {
	// Functions with return type and block body
	assert_statement(
		"function myFunction(): Result { return value }",
		"function myFunction(): Result {\n\treturn value\n}"
	);
}

#[test]
fn with_parameters() {
	// Functions with parameters
	assert_statement("function add(a, b) => a + b", "function add(a, b) => (a + b)");

	// Functions with parameters and return type
	assert_statement(
		"function multiply(a: Number, b: Number): Number { return a * b }",
		"function multiply(a: Number, b: Number): Number {\n\treturn (a * b)\n}"
	);
}

#[test]
fn with_optional_parameters() {
	// Functions with optional parameters (default values)
	assert_statement(
		"function increment(x, step = 1) => x + step",
		"function increment(x, step = 1) => (x + step)"
	);
	assert_statement(
		"function createUser(name: String, age: Number = 30) => user",
		"function createUser(name: String, age: Number = 30) => user"
	);
}

#[test]
fn with_complex_bodies() {
	// Functions with multi-statement bodies
	assert_statement(
		"function process(): Result { 
			intermediate = transform(input)
			result = calculate(intermediate)
			return result
		}",
		"function process(): Result {\n\tintermediate = transform(input)\n\tresult = calculate(intermediate)\n\treturn result\n}"
	);
}

// #[test]
// fn closure_function_declarations() {
// 	// Arrow functions assigned to variables
// 	assert_statement(
// 		"let myFunction = (a, b) => a + b",
// 		"let myFunction = (a, b) => (a + b)"
// 	);
// 	assert_statement("let getValue = () => 42", "let getValue = () => 42");

// 	// Arrow functions with block bodies
// 	assert_statement(
// 		"let processData = (data): Result => {
// 			result = transform(data)
// 			return result
// 		}",
// 		"let processData = (data): Result => {\n\tresult = transform(data)\n\treturn result\n}"
// 	);
// }
