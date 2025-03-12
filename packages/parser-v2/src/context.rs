use crate::tokens::{ Token, TokenKind };
use crate::typed_syntax_tree::TypedSyntaxTree;

pub struct Context {
	pub tree: *mut TypedSyntaxTree, // pointer to the semantic tree
	pub input: *const str, // input string
	pub tokens: *const [Token], // all tokens
	pub token: Token,
	pub index: usize,
}

impl Context {
	pub fn new(
		input: &'static str,
		tree: &mut TypedSyntaxTree,
		tokens: &[Token]
	) -> Self {
		Context {
			tree,
			input,
			tokens: &*tokens,
			token: unsafe {
				tokens.get_unchecked(0).clone()
			},
			index: 0,
		}
	}

	pub fn slice(&self) -> &'static str {
		unsafe {
			let input: &str = &*self.input;
			&input[self.token.start..self.token.end]
		}
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
		let tokens: &[Token] = unsafe { &*self.tokens };
		let next_index: usize = self.index + 1;
		if next_index < tokens.len() {
			unsafe { tokens.get_unchecked(next_index).kind }
		} else {
			TokenKind::End
		}
	}

	/// Return the current token and the position of the next token.
	pub fn go_to_next_token(&mut self) {
		unsafe {
			let index: &mut usize = &mut self.index;
			*index += 1;
			let tokens: &[Token] = &*self.tokens;
			self.token = if *index < tokens.len() {
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
	pub fn done(&self) -> bool {
		unsafe {
			let tokens: &[Token] = &*self.tokens;
			self.index >= tokens.len()
		}
	}
}
