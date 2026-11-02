use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::nodes::{HoistedSymbol, Name, Node};
use crate::source::SourceSpan;
use crate::tokenize::tokenize_range;
use crate::tokens::{Token, TokenKind};
use crate::typed_syntax_tree::TypedSyntaxTree;

/// The deepest nesting of expressions the parser accepts. Deeper code is reported instead of
/// overflowing the stack.
pub const MAX_DEPTH: usize = 128;

/// The state of the parser: the tokens, the position in them, and the tree being built.
pub struct Context<'source> {
	pub source: &'source str,
	pub tree: TypedSyntaxTree,
	pub tokens: Vec<Token>,
	pub token_index: usize,
	pub hoisted_scopes: Vec<Vec<HoistedSymbol>>,
	// the current nesting: the `parse_expression` and `parse_pattern` calls, and the links of the
	// chains being parsed, like `f()()`
	pub depth: usize,
	// set once the nesting limit is reported, to report it once
	pub depth_exceeded: bool,
}

impl<'source> Context<'source> {
	/// `shared` holds the same text as `source`: the tree keeps it.
	pub fn new(source: &'source str, shared: Arc<str>) -> Self {
		debug_assert_eq!(source, &*shared);
		let mut tree = TypedSyntaxTree::new(shared);
		let tokens = tokenize_range(
			source,
			0,
			source.len(),
			false,
			&mut tree.diagnostics,
			Some(&mut tree.comments),
		);
		// most tokens become a node
		tree.nodes.reserve(tokens.len());
		tree.spans.reserve(tokens.len());
		let mut hoisted_scopes = Vec::with_capacity(16);
		hoisted_scopes.push(Vec::new());
		Context {
			source,
			tree,
			tokens,
			token_index: 0,
			hoisted_scopes,
			depth: 0,
			depth_exceeded: false,
		}
	}

	#[inline]
	fn token_at(&self, index: usize) -> Token {
		match self.tokens.get(index) {
			Some(token) => *token,
			None => {
				let end = self
					.tokens
					.last()
					.map_or(self.source.len(), |token| token.end);
				Token::new(TokenKind::End, end, end)
			}
		}
	}

	#[inline]
	pub fn token(&self) -> Token {
		self.token_at(self.token_index)
	}

	#[inline]
	pub fn kind(&self) -> TokenKind {
		self.token().kind
	}

	#[inline]
	pub fn next_token(&self) -> Token {
		self.token_at(self.token_index + 1)
	}

	#[inline]
	pub fn lookup_next_token_kind(&self) -> TokenKind {
		self.next_token().kind
	}

	pub fn last_token(&self) -> Token {
		if self.token_index == 0 {
			return Token::new(TokenKind::Empty, 0, 0);
		}
		self.token_at(self.token_index - 1)
	}

	/// The end of the last consumed token.
	pub fn last_end(&self) -> usize {
		if self.token_index == 0 {
			self.token().start
		} else {
			self.token_at(self.token_index - 1).end
		}
	}

	/// The text of the current token.
	#[inline]
	pub fn slice(&self) -> &'source str {
		let token = self.token();
		self.text(token.start, token.end)
	}

	/// The current token, as a name.
	#[inline]
	pub fn name(&self) -> Name {
		let token = self.token();
		Name::new(token.start, token.end)
	}

	/// The text of a name.
	pub fn name_text(&self, name: Name) -> &'source str {
		self.text(name.start as usize, name.end as usize)
	}

	#[inline]
	pub fn text(&self, start: usize, end: usize) -> &'source str {
		self.source.get(start..end).unwrap_or("")
	}

	/// Move to the next token (the `End` token is never passed).
	#[inline]
	pub fn go_to_next_token(&mut self) {
		if self.token_index + 1 < self.tokens.len() {
			self.token_index += 1;
		}
	}

	/// Return true if all tokens have been processed.
	#[inline]
	pub fn done(&self) -> bool {
		self.kind() == TokenKind::End
	}

	/// Skip the new lines at the current position.
	pub fn skip_stops(&mut self) {
		while self.kind() == TokenKind::Stop {
			self.go_to_next_token();
		}
	}

	pub fn enter_hoisted_scope(&mut self) {
		self.hoisted_scopes.push(Vec::new());
	}

	pub fn exit_hoisted_scope(&mut self) -> Vec<HoistedSymbol> {
		self.hoisted_scopes.pop().unwrap_or_default()
	}

	pub fn add_hoisted_symbol(&mut self, name: Name, reference: usize) {
		if let Some(scope) = self.hoisted_scopes.last_mut() {
			scope.push(HoistedSymbol { name, reference });
		}
	}

	/// Insert a node in the tree and return its index.
	#[inline]
	pub fn insert(&mut self, node: Node, start: usize, end: usize) -> usize {
		self.tree
			.insert(node, SourceSpan::new(start, end.max(start)))
	}

	#[inline]
	pub fn at(&self, index: usize) -> &Node {
		self.tree.at(index)
	}

	#[inline]
	pub fn span(&self, index: usize) -> SourceSpan {
		self.tree.span(index)
	}

	/// Report an error. A second error at the same position is dropped: it is almost always a
	/// consequence of the first one.
	pub fn error(&mut self, message: impl Into<String>, start: usize, end: usize) {
		self.push_error(None, message.into(), start, end);
	}

	/// Report an error that breaks a rule with a code in the specification (`E-DST-9`).
	pub fn error_with_code(
		&mut self,
		code: &'static str,
		message: impl Into<String>,
		start: usize,
		end: usize,
	) {
		self.push_error(Some(code), message.into(), start, end);
	}

	fn push_error(
		&mut self,
		code: Option<&'static str>,
		message: String,
		start: usize,
		end: usize,
	) {
		if let Some(last) = self.tree.diagnostics.last()
			&& last.span.start == start
			&& last.is_error()
		{
			return;
		}
		let mut diagnostic = Diagnostic::error(message, SourceSpan::new(start, end.max(start)));
		diagnostic.code = code;
		self.tree.diagnostics.push(diagnostic);
	}

	/// Report an error on the current token.
	pub fn error_at_token(&mut self, message: impl Into<String>) {
		let (start, end) = self.token_span_for_error();
		self.error(message, start, end);
	}

	/// Report an error with a code (see [`Context::error_with_code`]) on the current token.
	pub fn error_at_token_with_code(&mut self, code: &'static str, message: impl Into<String>) {
		let (start, end) = self.token_span_for_error();
		self.error_with_code(code, message, start, end);
	}

	fn token_span_for_error(&self) -> (usize, usize) {
		let token = self.token();
		if token.kind == TokenKind::Stop {
			// point at the end of the line rather than at the line break
			(self.last_end(), self.last_end())
		} else {
			(token.start, token.end)
		}
	}

	/// Report that the code is nested deeper than `MAX_DEPTH` (once per file), skip the rest of
	/// the statement, and return an `Invalid` node for it.
	pub fn skip_too_deep_code(&mut self) -> usize {
		if !self.depth_exceeded {
			self.depth_exceeded = true;
			self.error_at_token(format!(
				"This code is nested too deeply (more than {MAX_DEPTH} levels): split it into smaller functions"
			));
		}
		let start = self.token().start;
		self.skip_to_statement_end();
		let end = self.last_end();
		self.insert(Node::Invalid, start, end)
	}

	/// Insert an `Invalid` node at the current position, for something missing.
	pub fn insert_missing(&mut self) -> usize {
		let position = self.last_end();
		self.insert(Node::Invalid, position, position)
	}

	/// Describe the current token for a diagnostic, like "`)`" or "the name `foo`".
	/// Whether the current token is a keyword (`if`, `loop`, `true`): a keyword used where a
	/// name is expected is `E-LEX-3`.
	pub fn token_is_keyword(&self) -> bool {
		let kind = self.kind();
		kind != TokenKind::Identifier
			&& kind != TokenKind::Error
			&& self
				.slice()
				.bytes()
				.next()
				.is_some_and(|byte| byte.is_ascii_lowercase())
	}

	pub fn describe_token(&self) -> String {
		let token = self.token();
		match token.kind {
			TokenKind::Identifier => format!("`{}`", self.slice()),
			TokenKind::Invalid => format!("`{}`", self.slice()),
			kind => kind.describe().to_string(),
		}
	}

	/// Skip tokens up to the end of the current statement: a new line or the end of the file
	/// outside of brackets, or a closing bracket that was opened before. Used to recover after
	/// an error.
	pub fn skip_to_statement_end(&mut self) {
		self.skip_until(&[]);
	}

	/// Like `skip_to_statement_end`, but also stop before one of `kinds` outside of brackets.
	pub fn skip_until(&mut self, kinds: &[TokenKind]) {
		let mut depth: usize = 0;
		loop {
			match self.kind() {
				TokenKind::End => return,
				TokenKind::Stop if depth == 0 => return,
				kind if depth == 0 && kinds.contains(&kind) => return,
				TokenKind::ParenthesisOpen
				| TokenKind::ParenthesisOpenFunctionDeclaration
				| TokenKind::BracesOpen
				| TokenKind::BracketsOpen => depth += 1,
				TokenKind::ParenthesisClose | TokenKind::BracesClose | TokenKind::BracketsClose => {
					if depth == 0 {
						return;
					}
					depth -= 1;
				}
				_ => {}
			}
			self.go_to_next_token();
		}
	}

	/// Parse the tokens of `source[start..end]` with `parse`, then come back to the current
	/// tokens. Used for the expressions interpolated in strings.
	pub fn with_sub_tokens<T>(
		&mut self,
		start: usize,
		end: usize,
		parse: impl FnOnce(&mut Self) -> T,
	) -> T {
		let tokens = tokenize_range(
			self.source,
			start,
			end,
			false,
			&mut self.tree.diagnostics,
			None,
		);
		let saved_tokens = std::mem::replace(&mut self.tokens, tokens);
		let saved_index = std::mem::replace(&mut self.token_index, 0);
		let result = parse(self);
		self.tokens = saved_tokens;
		self.token_index = saved_index;
		result
	}
}
