use crate::fa;

use crate::ast::Expression::*;

#[test]
fn single_argument() {
	let ast = fa::ExpressionParser::new().parse("myFunction(12)").unwrap();
	match *ast {
		Call(function, parameters) => {
			assert_eq!(*function, Identifier("myFunction".to_string()));
			assert_eq!(parameters.len(), 1);
			assert_eq!(*parameters[0], Integer(12));
		}
		_ => panic!("Expected a Call(myFunction, [12]), got {:?}", ast),
	}
}

#[test]
fn no_arguments() {
	let ast = fa::ExpressionParser::new().parse("myFunction()").unwrap();
	match *ast {
		Call(function, parameters) => {
			assert_eq!(*function, Identifier("myFunction".to_string()));
			assert_eq!(parameters.len(), 0);
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn many_arguments() {
	let ast = fa::ExpressionParser::new().parse("myFunction(12, hello, 7)").unwrap();
	match *ast {
		Call(function, parameters) => {
			assert_eq!(*function, Identifier("myFunction".to_string()));
			assert_eq!(parameters.len(), 3);
			assert_eq!(*parameters[0], Integer(12));
			assert_eq!(*parameters[1], Identifier("hello".to_string()));
			assert_eq!(*parameters[2], Integer(7));
		}
		_ => panic!("Expected a Call(myFunction, [12, hello, 7]), got {:?}", ast),
	}
}

#[test]
fn chained_function_call() {
	let ast = fa::ExpressionParser::new().parse("myFunction(12, hello, 7)(121)").unwrap();
	match *ast {
		Call(function, parameters) => {
			match *function {
				Call(function, parameters) => {
					assert_eq!(*function, Identifier("myFunction".to_string()));
					assert_eq!(parameters.len(), 3);
					assert_eq!(*parameters[0], Integer(12));
					assert_eq!(*parameters[1], Identifier("hello".to_string()));
					assert_eq!(*parameters[2], Integer(7));
				}
				_ =>
					panic!(
						"Expected a Call(myFunction, [12, hello, 7]), got {:?}",
						function
					),
			}
			assert_eq!(parameters.len(), 1);
			assert_eq!(*parameters[0], Integer(121));
		}
		_ => panic!("Expected a Call(myFunction, [12, hello, 7]), got {:?}", ast),
	}
}
