use std::collections::HashMap;

use crate::scope::{Scope, SymbolState};
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
	pub scopes: Vec<Scope>,
	pub member_access_depth: usize,
	pub external_symbol_scopes: Vec<HashMap<&'static str, Vec<usize>>>,
}

impl Context {
	pub fn new(input: &'static str, tree: &mut TypedSyntaxTree) -> Self {
		let mut lexer = Tokenizer::new(input.as_bytes());
		let token = lexer.next_token();
		let next_token = lexer.next_token();
		let mut scopes = Vec::with_capacity(16);
		scopes.push(Scope::new());
		let mut external_symbol_scopes = Vec::with_capacity(16);
		external_symbol_scopes.push(HashMap::new());
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
			scopes,
			member_access_depth: 0,
			external_symbol_scopes,
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

	pub fn enter_scope(&mut self) {
		self.scopes.push(Scope::new());
		self.external_symbol_scopes.push(HashMap::new());
	}

	pub fn exit_scope(&mut self) {
		self.scopes.pop();
		if let Some(mut scope) = self.external_symbol_scopes.pop() {
			if let Some(parent) = self.external_symbol_scopes.last_mut() {
				for (name, mut references) in scope.drain() {
					parent.entry(name).or_default().append(&mut references);
				}
			}
		}
	}

	pub fn declare_symbol(&mut self, name: &'static str, node: usize) {
		if let Some(scope) = self.scopes.last_mut() {
			scope.symbols.push(SymbolState::Declaration { name, node });
		}
	}

	pub fn resolve_symbol(&self, name: &str) -> Option<usize> {
		for scope in self.scopes.iter().rev() {
			for symbol in scope.symbols.iter().rev() {
				if let SymbolState::Declaration {
					name: symbol_name,
					node,
				} = symbol
				{
					if *symbol_name == name {
						return Some(*node);
					}
				}
			}
		}
		None
	}

	pub fn add_external_reference(&mut self, name: &'static str, reference: usize) {
		if let Some(scope) = self.external_symbol_scopes.last_mut() {
			scope.entry(name).or_default().push(reference);
		}
	}

	pub fn take_external_references(&mut self, name: &'static str) -> Option<Vec<usize>> {
		self.external_symbol_scopes
			.last_mut()
			.and_then(|scope| scope.remove(name))
	}

	pub fn push_member_access(&mut self) {
		self.member_access_depth += 1;
	}

	pub fn pop_member_access(&mut self) {
		self.member_access_depth = self.member_access_depth.saturating_sub(1);
	}

	pub fn is_in_member_access(&self) -> bool {
		self.member_access_depth > 0
	}
}
