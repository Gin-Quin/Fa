use crate::tokenize::Tokenizer;
use crate::tokens::{Token, TokenKind};
use crate::typed_syntax_tree::TypedSyntaxTree;

pub struct Context {
	pub tree: *mut TypedSyntaxTree, // pointer to the semantic tree
	pub input: *const str,          // input string
	pub lexer: Tokenizer<'static>,
	pub last_token: Token,
	pub token: Token,
	pub next_token: Token,
}

impl Context {
	pub fn new(input: &'static str, tree: &mut TypedSyntaxTree) -> Self {
		let mut lexer = Tokenizer::new(input.as_bytes());
		let token = lexer.next_token();
		let next_token = lexer.next_token();
		Context {
			tree,
			input,
			lexer,
			last_token: Token {
				kind: TokenKind::None,
				start: 0,
				end: 0,
			},
			token,
			next_token,
		}
	}

	pub fn slice(&self) -> &'static str {
		let input: &str = unsafe { &*self.input };
		&input[self.token.start..self.token.end]
	}

	pub fn slice_at(&self, index: usize) -> &'static str {
		let input: &str = unsafe { &*self.input };
		&input[self.token.start + index..self.token.end]
	}

	/// Print the current token.
	pub fn debug(&self, message: &str) {
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

	pub fn lookup_next_token_kind(&self) -> TokenKind {
		self.next_token.kind
	}

	/// Return the current token and the position of the next token.
	pub fn go_to_next_token(&mut self) {
		self.last_token = self.token.clone();
		self.token = self.next_token.clone();
		self.next_token = self.lexer.next_token();
	}

	/// Return true if all tokens have been processed.
	pub fn done(&self) -> bool {
		self.token.kind == TokenKind::End
	}
}
