use crate::nodes::*;
use crate::tokenize::tokenize;
use crate::tokens::{ Token, TokenKind };

#[test]
fn test_parse_expression() {
	let input = "a + b + c * d * e + f * g";
	let semantic_tree = parse_expression(input);
	println!("{}", semantic_tree_to_string(&semantic_tree));
}

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
		println!("go_to_next_token {}/{}", (*context).index, tokens.len());

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
		expression_left(&mut context, Priority::None);
	}

	tree
}

fn expression_left<'input>(context: &mut Context<'input>, priority: Priority) -> usize {
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let token: Token = context.token;

	context.debug("PARSE LEFT");

	let node: Node<'input> = match token.kind {
		TokenKind::Identifier => Node::Identifier(context.slice()),
		TokenKind::Integer => Node::Integer(context.slice().parse::<i32>().unwrap()),
		TokenKind::True => Node::Boolean(true),
		TokenKind::False => Node::Boolean(false),
		TokenKind::Minus => {
			go_to_next_token(context);
			let right = expression_left(context, Priority::Additive);
			Node::Negate { right }
		}
		_ => panic!("Expected left token, received: {:?}", token),
	};

	let mut left = tree.insert(node);

	go_to_next_token(context);

	while !done(context) {
		let right = expression_right(context, priority, left);
		if right == left {
			println!("Yield!");
			break;
		} else {
			left = right;
			// println!("left -> go_to_next_token");
			// go_to_next_token(context);
		}
	}

	left
}

fn expression_right<'input>(
	context: &mut Context<'input>,
	priority: Priority,
	left: usize
) -> usize {
	let tree: &mut SemanticTree<'input> = unsafe { &mut *context.tree };
	let token: Token = context.token;

	context.debug("PARSE RIGHT");

	macro_rules! Stop {
		() => { return left };
	}

	macro_rules! OperationNode {
		($node_type:ident, $priority:expr) => {
            if priority > $priority { Stop!() }
            else {
                go_to_next_token(context);
                let right = expression_left(context, $priority);
                Node::$node_type {
                    left,
                    right,
                }
            }
		};
	}

	let node: Node<'input> = match token.kind {
		TokenKind::Stop => Stop!(),
		TokenKind::Plus => OperationNode!(Add, Priority::Additive),
		TokenKind::Minus => OperationNode!(Subtract, Priority::Additive),
		TokenKind::Star => OperationNode!(Multiply, Priority::Multiplicative),
		_ => {
			panic!("Unexpected token '{}' ({:?})", context.slice(), token.kind);
		}
	};

	tree.insert(node)
}
