use slab::Slab;

use crate::nodes::*;
use crate::priority::*;
use crate::tokenize::tokenize;
use crate::tokens::{ Token, TokenKind };
use crate::typed_syntax_tree::TypedSyntaxTree;

struct Context {
	tree: *mut TypedSyntaxTree, // pointer to the semantic tree
	input: *const str, // input string
	tokens: *const [Token], // all tokens
	token: Token,
	index: usize,
}

impl Context {
	fn slice(&self) -> &'static str {
		unsafe {
			let input: &str = &*self.input;
			&input[self.token.start..self.token.end]
		}
	}

	/// Print the current token.
	fn debug(&self, message: &str) {
		unsafe {
			let input: &str = &*self.input;
			println!(
				"{}: '{}' ({:?})",
				message,
				&input[self.token.start..self.token.end],
				self.token.kind
			);
		}
	}
}

/// Return the current token and the position of the next token.
fn go_to_next_token(context: *mut Context) {
	unsafe {
		let index: &mut usize = &mut (*context).index;
		*index += 1;
		let tokens: &[Token] = &*(*context).tokens;
		(*context).token = if *index < tokens.len() {
			tokens.get_unchecked(*index).clone()
		} else {
			Token {
				kind: TokenKind::End,
				start: 0,
				end: 0,
			}
		};
	}
}

/// Return true if all tokens have been processed.
fn done(context: *const Context) -> bool {
	unsafe {
		let tokens: &[Token] = &*(*context).tokens;
		(*context).index >= tokens.len()
	}
}

/// Parse an expression from the given input.
pub fn parse_expression(input: &'static str) -> TypedSyntaxTree {
	let tokens: Vec<Token> = tokenize(input.as_bytes());
	let mut tree = TypedSyntaxTree {
		input,
		nodes: Slab::new(),
	};

	if tokens.len() == 0 {
		return tree;
	}

	let mut context = Context {
		tree: &mut tree,
		input,
		tokens: &*tokens,
		token: unsafe {
			tokens.get_unchecked(0).clone()
		},
		index: 0,
	};

	if !done(&context) {
		expression_left(&mut context, Priority::None, TokenKind::None);
	}

	tree
}

fn expression_left(
	context: &mut Context,
	priority: Priority,
	stop_at: TokenKind
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;
	let mut increment_at_the_end = true;

	context.debug("PARSE LEFT");

	macro_rules! Prefix {
		($node_type:ident, $priority:expr) => {
            {
                go_to_next_token(context);
                increment_at_the_end = false;
                let right = expression_left(context, $priority, stop_at);
                Node::$node_type { right }
            }
		};
	}

	let node: Node = match token.kind {
		TokenKind::Identifier => Node::Identifier(context.slice()),
		TokenKind::Integer => Node::Integer(context.slice().parse::<i32>().unwrap()),
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::Minus => Prefix!(Negate, Priority::Prefix),
		TokenKind::Not => Prefix!(Not, Priority::Not),
		TokenKind::ParenthesisOpen => {
			// group expression or tuple (no function calls, see `expression_right`)
			go_to_next_token(context);

			// check if the next token is a parenthesis close
			if context.token.kind == TokenKind::ParenthesisClose {
				panic!("Empty groups are not allowed");
			}

			let expression = expression_left(
				context,
				Priority::None,
				TokenKind::ParenthesisClose
			);
			Node::Group { expression }
		}
		_ => {
			return tree.nodes.insert(Node::UnexpectedTokenError {
				token: token.clone(),
			});
		}
	};

	let mut left = tree.nodes.insert(node);

	if increment_at_the_end {
		go_to_next_token(context);
	}

	while !done(context) {
		let right = expression_right(context, priority, stop_at, left);
		if right == left {
			break;
		} else {
			left = right;
		}
	}

	left
}

fn expression_right(
	context: &mut Context,
	priority: Priority,
	stop_at: TokenKind,
	left: usize
) -> usize {
	let tree: &mut TypedSyntaxTree = unsafe { &mut *context.tree };
	let token = &context.token;

	context.debug("PARSE RIGHT");

	if token.kind == stop_at {
		return left;
	}

	macro_rules! Stop {
		() => { return left };
	}

	macro_rules! Operation {
		($node_type:ident, $priority:expr) => {
            if priority >= $priority { Stop!() }
            else {
                go_to_next_token(context);
                let right = expression_left(context, $priority, stop_at);
                Node::$node_type {
                    left,
                    right,
                }
            }
		};
	}

	macro_rules! List {
		($node_type:ident, $elements:ident, $priority:expr) => {
			if priority >= $priority { Stop!() }
			else {
				let left_node = unsafe { tree.nodes.get_unchecked_mut(left).clone() };

				match left_node {
					Node::$node_type { mut $elements } => {
						tree.nodes.remove(left);
						go_to_next_token(context);
						let right = expression_left(context, $priority, stop_at);
						$elements.push(right);
						Node::$node_type { $elements }
					}
					_ => {
						go_to_next_token(context);
						let right = expression_left(context, $priority, stop_at);
						Node::$node_type {
							$elements: vec![left, right],
						}
					}
				}
			}
		};
	}

	let node: Node = match token.kind {
		TokenKind::Stop => Stop!(),

		// Operators
		TokenKind::Plus => List!(Add, operands, Priority::Additive),
		TokenKind::Minus => List!(Subtract, operands, Priority::Additive),
		TokenKind::Star => List!(Multiply, operands, Priority::Multiplicative),
		TokenKind::Slash => List!(Divide, operands, Priority::Multiplicative),
		TokenKind::Modulo => List!(Modulo, operands, Priority::Multiplicative),
		TokenKind::DoubleStar => List!(Power, operands, Priority::Exponentiation),
		TokenKind::LessThan => List!(LessThan, operands, Priority::Comparison),
		TokenKind::LessThanOrEqual =>
			List!(LessThanOrEqual, operands, Priority::Comparison),
		TokenKind::GreaterThan => List!(GreaterThan, operands, Priority::Comparison),
		TokenKind::GreaterThanOrEqual =>
			List!(GreaterThanOrEqual, operands, Priority::Comparison),
		TokenKind::DoubleEqual => List!(Equal, operands, Priority::Equality),
		TokenKind::NotEqual => List!(NotEqual, operands, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),
		TokenKind::And => List!(And, operands, Priority::And),
		TokenKind::Or => List!(Or, operands, Priority::Or),
		TokenKind::Equal => List!(Equal, operands, Priority::Assignment),
		TokenKind::Union => List!(Union, operands, Priority::Union),
		TokenKind::Pipe => List!(Pipe, operands, Priority::Pipe),
		TokenKind::Insert => Operation!(Insert, Priority::Transfer),
		TokenKind::Extract => Operation!(Extract, Priority::Transfer),

		TokenKind::Comma => List!(Tuple, items, Priority::Comma),

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	tree.nodes.insert(node)
}
