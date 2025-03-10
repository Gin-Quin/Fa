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
		if *index < tokens.len() {
			(*context).token = tokens.get_unchecked(*index).clone();
		}
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
		_ => panic!("Expected left token, received: {:?}", token),
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

	let node: Node = match token.kind {
		TokenKind::Stop => Stop!(),

		// Operators
		TokenKind::Plus => Operation!(Add, Priority::Additive),
		TokenKind::Minus => Operation!(Subtract, Priority::Additive),
		TokenKind::Star => Operation!(Multiply, Priority::Multiplicative),
		TokenKind::Slash => Operation!(Divide, Priority::Multiplicative),
		TokenKind::DoubleSlash => Operation!(IntegerDivide, Priority::Multiplicative),
		TokenKind::Modulo => Operation!(Modulo, Priority::Multiplicative),
		TokenKind::DoubleStar => Operation!(Power, Priority::Exponentiation),
		TokenKind::LessThan => Operation!(LessThan, Priority::Comparison),
		TokenKind::LessThanOrEqual => Operation!(LessThanOrEqual, Priority::Comparison),
		TokenKind::GreaterThan => Operation!(GreaterThan, Priority::Comparison),
		TokenKind::GreaterThanOrEqual =>
			Operation!(GreaterThanOrEqual, Priority::Comparison),
		TokenKind::DoubleEqual => Operation!(Equal, Priority::Equality),
		TokenKind::NotEqual => Operation!(NotEqual, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),
		TokenKind::And => Operation!(And, Priority::And),
		TokenKind::Or => Operation!(Or, Priority::Or),
		TokenKind::Equal => Operation!(Equal, Priority::Assignment),
		TokenKind::FatArrow => Operation!(FatArrow, Priority::Assignment),
		TokenKind::Union => Operation!(Union, Priority::Union),
		TokenKind::Pipe => Operation!(Pipe, Priority::Pipe),
		TokenKind::Insert => Operation!(Insert, Priority::Transfer),
		TokenKind::Extract => Operation!(Extract, Priority::Transfer),

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	tree.nodes.insert(node)
}
