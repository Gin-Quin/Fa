use crate::nodes::*;
use crate::tokenize::tokenize;
use crate::tokens::{ Token, TokenKind };

struct Context<'input> {
	tree: *mut SemanticTree<'input>, // pointer to the semantic tree
	input: *const str, // input string
	tokens: *const [Token], // all tokens
	token: Token,
	index: usize,
	offset: usize,
}

impl<'input> Context<'input> {
	fn slice(&self) -> &'input str {
		unsafe {
			let input: &str = &*self.input;
			&input[self.offset..self.offset + (self.token.length as usize)]
		}
	}

	/// Print the current token.
	fn debug(&self, message: &str) {
		unsafe {
			let input: &str = &*self.input;
			println!(
				"{}: '{}' ({:?})",
				message,
				&input[self.offset..self.offset + (self.token.length as usize)],
				self.token.kind
			);
		}
	}
}

/// Skip all space tokens for the given context.
fn skip_spaces(context: *mut Context) {
	unsafe {
		let index: &mut usize = &mut (*context).index;
		let offset: &mut usize = &mut (*context).offset;
		let tokens: &[Token] = &*(*context).tokens;
		let token: &mut Token = &mut (*context).token;

		while *index < tokens.len() && token.kind == TokenKind::Space {
			*index += 1;
			*offset += token.length as usize;
			*token = tokens.get_unchecked(*index).clone();
		}
	}
}

/// Return the current token and the position of the next token.
fn go_to_next_token(context: *mut Context) {
	unsafe {
		let tokens: &[Token] = &*(*context).tokens;
		let index: &mut usize = &mut (*context).index;

		if *index < tokens.len() {
			*index += 1;
			(*context).offset += (*context).token.length as usize;
			if *index < tokens.len() {
				(*context).token = tokens.get_unchecked(*index).clone();
			}
			skip_spaces(context);
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
pub fn parse_expression<'input>(input: &'input str) -> SemanticTree<'input> {
	let tokens: Vec<Token> = tokenize(input.as_bytes());
	let mut tree = SemanticTree::new();

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
		offset: 0,
	};

	skip_spaces(&mut context);

	if !done(&context) {
		expression_left(&mut context, Priority::None, TokenKind::None);
	}

	tree
}

fn expression_left<'input>(
	context: &mut Context<'input>,
	priority: Priority,
	stop_at: TokenKind
) -> usize {
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let token: Token = context.token;
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

	let node: Node<'input> = match token.kind {
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
			println!("--> Group: {}", expression);
			Node::Group { expression }
		}
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let mut left = tree.insert(node);

	println!("--> Left done, go to next token");

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

	println!("---> Leave left");

	left
}

fn expression_right<'input>(
	context: &mut Context<'input>,
	priority: Priority,
	stop_at: TokenKind,
	left: usize
) -> usize {
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let token: Token = context.token;

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
                println!("--> Operation, go to next token");
                go_to_next_token(context);
                let right = expression_left(context, $priority, stop_at);
                Node::$node_type {
                    left,
                    right,
                }
            }
		};
	}

	let node: Node<'input> = match token.kind {
		TokenKind::Stop => Stop!(),

		// Additive operators
		TokenKind::Plus => Operation!(Add, Priority::Additive),
		TokenKind::Minus => Operation!(Subtract, Priority::Additive),

		// Multiplicative operators
		TokenKind::Star => Operation!(Multiply, Priority::Multiplicative),
		TokenKind::Slash => Operation!(Divide, Priority::Multiplicative),
		TokenKind::DoubleSlash => Operation!(IntegerDivide, Priority::Multiplicative),
		TokenKind::Percent => Operation!(Modulo, Priority::Multiplicative),

		// Exponentiation operator
		TokenKind::DoubleStar => Operation!(Power, Priority::Exponentiation),

		// Comparison operators
		TokenKind::LessThan => Operation!(LessThan, Priority::Comparison),
		TokenKind::LessThanOrEqual => Operation!(LessThanOrEqual, Priority::Comparison),
		TokenKind::GreaterThan => Operation!(GreaterThan, Priority::Comparison),
		TokenKind::GreaterThanOrEqual =>
			Operation!(GreaterThanOrEqual, Priority::Comparison),

		// Equality operators
		TokenKind::DoubleEqual => Operation!(Equal, Priority::Equality),
		TokenKind::NotEqual => Operation!(NotEqual, Priority::Equality),
		TokenKind::Is => Operation!(Is, Priority::Equality),

		// Logical operators
		TokenKind::And => Operation!(And, Priority::And),
		TokenKind::Or => Operation!(Or, Priority::Or),

		// Assignment operator
		TokenKind::Equal => Operation!(Equal, Priority::Assignment),

		// Arrow operators
		TokenKind::Arrow => Operation!(Arrow, Priority::Assignment),
		TokenKind::FatArrow => Operation!(FatArrow, Priority::Assignment),

		// Union operator
		TokenKind::Union => Operation!(Union, Priority::Union),
		TokenKind::Pipe => Operation!(Pipe, Priority::Pipe),

		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	tree.insert(node)
}
