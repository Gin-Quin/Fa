use crate::nodes::HoistedSymbol;
use crate::source::SourceMap;
use crate::tokenize::tokenize_with_source_map;
use crate::tokens::{Token, TokenKind};
use crate::typed_syntax_tree::TypedSyntaxTree;

pub struct Context {
	pub tree: *mut TypedSyntaxTree, // pointer to the semantic tree
	pub input: *const str,          // input string
	pub tokens: Vec<Token>,
	pub token_index: usize,
	pub hoisted_scopes: Vec<Vec<HoistedSymbol>>,
	pub source_map: SourceMap,
}

impl Context {
	pub fn new(input: &'static str, tree: &mut TypedSyntaxTree) -> Self {
		let (tokens, source_map) = tokenize_with_source_map(input.as_bytes());
		let mut hoisted_scopes = Vec::with_capacity(16);
		hoisted_scopes.push(Vec::new());
		Context {
			tree,
			input,
			tokens,
			token_index: 0,
			hoisted_scopes,
			source_map,
		}
	}

	fn token_at(&self, index: usize) -> Token {
		self.tokens.get(index).copied().unwrap_or(Token {
			kind: TokenKind::End,
			start: 0,
			end: 0,
		})
	}

	pub fn token(&self) -> Token {
		self.token_at(self.token_index)
	}

	pub fn next_token(&self) -> Token {
		self.token_at(self.token_index + 1)
	}

	pub fn last_token(&self) -> Token {
		if self.token_index == 0 {
			return Token {
				kind: TokenKind::Empty,
				start: 0,
				end: 0,
			};
		}
		self.token_at(self.token_index - 1)
	}

	pub fn slice(&self) -> &'static str {
		let input: &str = unsafe { &*self.input };
		let token = self.token();
		&input[token.start..token.end]
	}

	pub fn slice_at(&self, index: usize) -> &'static str {
		let input: &str = unsafe { &*self.input };
		let token = self.token();
		&input[token.start + index..token.end]
	}

	/// Print the current token.
	pub fn debug(&self, message: &str) {
		unsafe {
			let input: &str = &*self.input;
			let token = self.token();
			println!(
				"{}: '{}' ({:?})",
				message,
				&input[token.start..token.end],
				token.kind
			);
		}
	}

	pub fn lookup_next_token_kind(&self) -> TokenKind {
		self.next_token().kind
	}

	/// Return the current token and the position of the next token.
	pub fn go_to_next_token(&mut self) {
		self.token_index = self.token_index.saturating_add(1);
	}

	/// Return true if all tokens have been processed.
	pub fn done(&self) -> bool {
		self.token().kind == TokenKind::End
	}

	pub fn enter_hoisted_scope(&mut self) {
		self.hoisted_scopes.push(Vec::new());
	}

	pub fn exit_hoisted_scope(&mut self) -> Vec<HoistedSymbol> {
		self.hoisted_scopes.pop().unwrap_or_default()
	}

	pub fn add_hoisted_symbol(&mut self, name: &'static str, reference: usize) {
		if let Some(scope) = self.hoisted_scopes.last_mut() {
			scope.push(HoistedSymbol { name, reference });
		}
	}
}
