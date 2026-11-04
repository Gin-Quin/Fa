//! The queries of editors (design §1): the node at an offset, hover,
//! go-to-definition and completions. LSP-agnostic: offsets are byte offsets
//! into the file's text; the language server converts positions.

use std::fmt::Write as _;

use crate::builtins::{self, BuiltinKind, Signature, Ty};
use crate::diagnostics::SourceSpan;
use crate::ids::{BuiltinId, FileId, NodeId, ScopeId, TypeId};
use crate::session::Session;
use crate::symbols::{Resolution, Symbol, SymbolKind};
use crate::types::{Interval, TypeKind, int::INFINITY};

/// What to show when the cursor rests on code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hover {
	/// The code the hover is about.
	pub span: SourceSpan,
	/// Markdown: a ```fa block with the declaration or the type, then notes.
	pub markdown: String,
}

/// Where a name is declared.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Definition {
	/// The file of the declaration (for a declaration of a module whose
	/// file is not loaded in the session: the file asked about).
	pub file: FileId,
	/// The module of the declaration, when it is another one's whose tree
	/// is not in the session (its interface gave the spans).
	pub module: Option<crate::ids::ModuleId>,
	/// The whole declaration.
	pub span: SourceSpan,
	/// The declared name, inside `span`.
	pub name_span: SourceSpan,
}

/// What a completion suggests.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompletionKind {
	/// An immutable value (`let`, a parameter, `static`…).
	Value,
	/// A `mutable` value.
	Variable,
	/// A function.
	Function,
	/// A type.
	Type,
	/// A namespace, or a built-in namespace like `console`.
	Namespace,
	/// A method of the value before the `.`.
	Method,
	/// A field or property of the value before the `.`.
	Field,
}

/// A suggestion of completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
	/// The name to insert.
	pub label: String,
	/// What it is.
	pub kind: CompletionKind,
	/// Its type, or its signature.
	pub detail: Option<String>,
}

/// The keyword that declares a kind of symbol, for hovers.
fn keyword(kind: SymbolKind) -> &'static str {
	match kind {
		SymbolKind::Let => "let",
		SymbolKind::Mutable => "mutable",
		SymbolKind::Use => "use",
		SymbolKind::MutableUse => "mutable use",
		SymbolKind::Static => "static",
		SymbolKind::Parameter { .. } | SymbolKind::SelfParameter => "parameter",
		SymbolKind::LoopVariable { .. } => "for",
		SymbolKind::CatchBinding => "catch",
		SymbolKind::PatternBinding => "let",
		SymbolKind::Function => "function",
		SymbolKind::Type | SymbolKind::TypeParameter => "type",
		SymbolKind::Namespace => "namespace",
		SymbolKind::Test => "test",
		SymbolKind::Reactive => "reactive",
		SymbolKind::Derived => "derived",
		SymbolKind::NameAlias | SymbolKind::Implicit => "let",
		SymbolKind::Requirement { .. } => "requires",
	}
}

/// `0..=10`, `0..` or `..=10` for an interval (±∞ bounds left out).
pub fn interval_text(interval: Interval) -> String {
	let lo = (interval.lo > -INFINITY).then(|| interval.lo.to_string());
	let hi = (interval.hi < INFINITY).then(|| interval.hi.to_string());
	match (lo, hi) {
		(Some(lo), Some(hi)) if lo == hi => lo,
		(Some(lo), Some(hi)) => format!("{lo}..={hi}"),
		(Some(lo), None) => format!("{lo}.."),
		(None, Some(hi)) => format!("..={hi}"),
		(None, None) => "..".to_string(),
	}
}

impl Session {
	/// The innermost node whose span contains `offset` (its end included, for
	/// a cursor right after a name).
	pub fn node_at(&self, file: FileId, offset: u32) -> Option<NodeId> {
		self.nodes_at(file, offset).next()
	}

	/// The nodes whose span contains `offset`, innermost first.
	fn nodes_at(&self, file: FileId, offset: u32) -> impl Iterator<Item = NodeId> + use<> {
		let tree = self.tree(file);
		let mut nodes: Vec<(u32, NodeId)> = tree
			.spans
			.iter()
			.enumerate()
			.filter(|(_, span)| span.start as u32 <= offset && offset <= span.end as u32)
			.map(|(index, span)| ((span.end - span.start) as u32, NodeId::new(index)))
			.collect();
		nodes.sort_by_key(|&(width, node)| (width, std::cmp::Reverse(node)));
		nodes.into_iter().map(|(_, node)| node)
	}

	/// The symbol whose declared name contains `offset`.
	fn declared_name_at(&self, file: FileId, offset: u32) -> Option<&Symbol> {
		self.analysis(file)
			.symbols()
			.symbols()
			.iter()
			.find(|symbol| {
				symbol.name_span.start != symbol.name_span.end && symbol.name_span.contains(offset)
			})
	}

	/// The hover at an offset: a declared name, a name with what it refers to
	/// and its type there (with the interval when it is narrower than the
	/// type: `count: Integer — here 0..=10`), or the type of an expression.
	pub fn hover(&self, file: FileId, offset: u32) -> Option<Hover> {
		if let Some(symbol) = self.declared_name_at(file, offset) {
			return Some(Hover {
				span: symbol.name_span,
				markdown: self.symbol_markdown(symbol, symbol.ty, None),
			});
		}
		let analysis = self.analysis(file);
		for node in self.nodes_at(file, offset) {
			let span = self.span(file, node);
			let ty = analysis.type_of(node);
			match analysis.resolution(node) {
				Resolution::Local(symbol) => {
					let symbol = analysis.symbols().symbol(symbol);
					let ty = if ty.is_unset() { symbol.ty } else { ty };
					let markdown = self.symbol_markdown(symbol, ty, analysis.interval(node));
					return Some(Hover { span, markdown });
				}
				Resolution::Builtin(id) => {
					return Some(Hover {
						span,
						markdown: self.builtin_markdown(id),
					});
				}
				_ => {}
			}
			if ty.is_unset() || ty == TypeId::VOID || ty == TypeId::UNKNOWN {
				continue;
			}
			let mut markdown = format!("```fa\n{}\n```", self.display_type(ty));
			if let Some(interval) = analysis.interval(node) {
				let _ = write!(markdown, "\n\nhere `{}`", interval_text(interval));
			}
			return Some(Hover { span, markdown });
		}
		None
	}

	/// `let count: Integer`, then the interval of this read.
	fn symbol_markdown(&self, symbol: &Symbol, ty: TypeId, interval: Option<Interval>) -> String {
		let name = self.types.syms.resolve(symbol.name);
		let shown = if ty.is_unset() || ty == TypeId::UNKNOWN {
			String::new()
		} else {
			self.display_type(ty)
		};
		let mut markdown = match symbol.kind {
			SymbolKind::Type if !shown.is_empty() => format!("```fa\ntype {name} = {shown}\n```"),
			SymbolKind::Function if !shown.is_empty() => {
				format!("```fa\nfunction {name}: {shown}\n```")
			}
			_ if shown.is_empty() => format!("```fa\n{} {name}\n```", keyword(symbol.kind)),
			_ => format!("```fa\n{} {name}: {shown}\n```", keyword(symbol.kind)),
		};
		if let Some(interval) = interval {
			let _ = write!(markdown, "\n\nhere `{}`", interval_text(interval));
		}
		markdown
	}

	/// The hover of a built-in: its signature with parameter names, or its type.
	fn builtin_markdown(&self, id: BuiltinId) -> String {
		let builtin = builtins::get(id);
		let path = builtin.path.replace('#', ".");
		let text = match builtin.kind {
			BuiltinKind::Function(signature) => self.signature_text(&path, signature),
			BuiltinKind::Namespace => format!("namespace {path}"),
			BuiltinKind::Type(_) | BuiltinKind::BuiltType(_) => format!("type {path}"),
			BuiltinKind::Property(ty) => format!("{path}: {}", ty.describe(&self.types)),
			BuiltinKind::Value(_) => match self.builtin_type_text(id) {
				Some(ty) => format!("let {path}: {ty}"),
				None => format!("let {path}"),
			},
		};
		format!("```fa\n{text}\n```\n\n(prelude)")
	}

	/// `String.padStart(length: Integer, filler: String = …): String`
	fn signature_text(&self, path: &str, signature: &Signature) -> String {
		let show = |ty: Ty| ty.describe(&self.types);
		let last = signature.params.len().saturating_sub(1);
		let params: Vec<String> = signature
			.params
			.iter()
			.enumerate()
			.map(|(index, param)| {
				let variadic = if signature.variadic && index == last {
					"..."
				} else {
					""
				};
				let ty = match param.ty {
					Ty::Printable => String::new(),
					ty => format!(": {}", show(ty)),
				};
				let default = if param.default { " = …" } else { "" };
				format!("{variadic}{}{ty}{default}", param.name)
			})
			.collect();
		let mut text = format!("{path}({}): {}", params.join(", "), show(signature.ret));
		if let Some(throws) = signature.throws {
			let _ = write!(text, " throws {}", show(throws));
		}
		text
	}

	/// Where the name at `offset` is declared: a local symbol of the file.
	pub fn definition(&self, file: FileId, offset: u32) -> Option<Definition> {
		// a declaration of another file (modules.rs)
		let external = self.nodes_at(file, offset).find_map(|node| {
			let resolution = self.analysis(file).resolution(node);
			self.external(resolution)
		});
		if let Some((other, symbol)) = external {
			let symbol = self.analysis(other).symbols.symbol(symbol);
			return Some(Definition {
				file: other,
				module: None,
				span: self.span(other, symbol.decl_node),
				name_span: symbol.name_span,
			});
		}
		// a module whose tree is dropped, or of another unit: its interface
		let keyed = self.nodes_at(file, offset).find_map(|node| {
			match self.analysis(file).resolution(node) {
				crate::symbols::Resolution::External(key) => self.keyed(key),
				_ => None,
			}
		});
		if let Some((module, keyed)) = keyed {
			return Some(Definition {
				file,
				module: Some(module),
				span: keyed.span,
				name_span: keyed.name_span,
			});
		}
		let symbol = match self.declared_name_at(file, offset) {
			Some(symbol) => symbol,
			None => self
				.nodes_at(file, offset)
				.find_map(|node| self.analysis(file).symbol_of(node))?,
		};
		Some(Definition {
			file,
			module: None,
			span: self.span(file, symbol.decl_node),
			name_span: symbol.name_span,
		})
	}

	/// The names that can be written at `offset`: after `value.`, the members
	/// of the value; elsewhere, the symbols visible there (declared before, or
	/// hoisted), then the prelude.
	pub fn completions(&self, file: FileId, offset: u32) -> Vec<Completion> {
		let text = self.text(file);
		let mut offset = (offset as usize).min(text.len());
		while !text.is_char_boundary(offset) {
			offset -= 1;
		}
		let before = text[..offset].trim_end_matches(|character: char| {
			character.is_ascii_alphanumeric() || character == '_'
		});
		if before.ends_with('.') && !before.ends_with("..") {
			return self.member_completions(file, (before.len() - 1) as u32);
		}
		let mut completions = Vec::new();
		let mut seen = rustc_hash::FxHashSet::default();
		let analysis = self.analysis(file);
		let mut scope = self.scope_at(file, offset as u32);
		while let Some(id) = scope {
			let current = analysis.symbols().scope(id);
			for &symbol in current.symbols.iter().rev() {
				let symbol = analysis.symbols().symbol(symbol);
				let hoisted = matches!(
					symbol.kind,
					SymbolKind::Function | SymbolKind::Type | SymbolKind::Namespace
				);
				let declared = self.span(file, symbol.decl_node).end as usize <= offset;
				let name = self.types.syms.resolve(symbol.name);
				if (hoisted || declared)
					&& symbol.kind != SymbolKind::Test
					&& !name.is_empty()
					&& seen.insert(name.to_string())
				{
					completions.push(Completion {
						label: name.to_string(),
						kind: match symbol.kind {
							SymbolKind::Mutable | SymbolKind::MutableUse | SymbolKind::Reactive => {
								CompletionKind::Variable
							}
							SymbolKind::Function => CompletionKind::Function,
							SymbolKind::Type | SymbolKind::TypeParameter => CompletionKind::Type,
							SymbolKind::Namespace => CompletionKind::Namespace,
							_ => CompletionKind::Value,
						},
						detail: (!symbol.ty.is_unset() && symbol.ty != TypeId::UNKNOWN)
							.then(|| self.display_type(symbol.ty)),
					});
				}
			}
			scope = current.parent;
		}
		for (index, builtin) in builtins::all().iter().enumerate() {
			if builtin.path.contains(['.', '#'])
				|| builtins::std_library::gated(builtin.path)
				|| !seen.insert(builtin.path.to_string())
			{
				continue;
			}
			let id = BuiltinId::new(index);
			completions.push(self.builtin_completion(id, builtin.path));
		}
		completions
	}

	/// A built-in as a completion.
	fn builtin_completion(&self, id: BuiltinId, label: &str) -> Completion {
		let builtin = builtins::get(id);
		let (kind, detail) = match builtin.kind {
			BuiltinKind::Type(_) | BuiltinKind::BuiltType(_) => (CompletionKind::Type, None),
			BuiltinKind::Namespace => (CompletionKind::Namespace, None),
			BuiltinKind::Function(signature) => (
				if builtin.path.contains('#') {
					CompletionKind::Method
				} else {
					CompletionKind::Function
				},
				Some(self.signature_text(builtin.name(), signature)),
			),
			BuiltinKind::Property(ty) => (CompletionKind::Field, Some(ty.describe(&self.types))),
			BuiltinKind::Value(_) => (CompletionKind::Value, self.builtin_type_text(id)),
		};
		Completion {
			label: label.to_string(),
			kind,
			detail,
		}
	}

	/// The members of the value that ends at the `.` at `dot`.
	fn member_completions(&self, file: FileId, dot: u32) -> Vec<Completion> {
		let analysis = self.analysis(file);
		let tree = self.tree(file);
		// the largest expression that ends right before the `.`
		let target = (0..tree.nodes.len())
			.filter(|&node| {
				tree.spans[node].end as u32 == dot && tree.spans[node].start as u32 != dot
			})
			.min_by_key(|&node| tree.spans[node].start)
			.map(NodeId::new);
		let Some(target) = target else {
			return Vec::new();
		};
		if let Resolution::Builtin(id) = analysis.resolution(target)
			&& builtins::has_members(id)
		{
			let path = builtins::get(id).path;
			// `std.concurrency.`: the members of a module of `std`
			if let Some(module) = path
				.strip_prefix("std.")
				.and_then(builtins::std_library::module)
			{
				return module
					.members
					.iter()
					.filter_map(|(name, _)| {
						let member = builtins::std_library::member(module.name, name)?;
						Some(self.builtin_completion(member, name))
					})
					.collect();
			}
			let prefix = format!("{path}.");
			return builtins::all()
				.iter()
				.enumerate()
				.filter_map(|(index, builtin)| {
					let name = builtin.path.strip_prefix(prefix.as_str())?;
					Some(self.builtin_completion(BuiltinId::new(index), name))
				})
				.collect();
		}
		let ty = analysis.type_of(target);
		if ty.is_unset() {
			return Vec::new();
		}
		let mut completions = Vec::new();
		if builtins::strings::is_string_like(&self.types, ty) {
			for (index, builtin) in builtins::all().iter().enumerate() {
				if let Some(name) = builtin.path.strip_prefix("String#") {
					completions.push(self.builtin_completion(BuiltinId::new(index), name));
				}
			}
		}
		if let TypeKind::Object(_) = self.types.kind(ty) {
			for field in self.types.fields_of(ty) {
				completions.push(Completion {
					label: self.types.syms.resolve(field.name).to_string(),
					kind: if matches!(self.types.kind(field.ty), TypeKind::Function(_)) {
						CompletionKind::Method
					} else {
						CompletionKind::Field
					},
					detail: Some(self.display_type(field.ty)),
				});
			}
		}
		completions
	}

	/// The innermost scope whose owner contains `offset` (the module scope
	/// when none does).
	fn scope_at(&self, file: FileId, offset: u32) -> Option<ScopeId> {
		let scopes = self.analysis(file).symbols().scopes();
		let mut best: Option<(u32, ScopeId)> = None;
		for (index, scope) in scopes.iter().enumerate() {
			let span = self.span(file, scope.owner);
			let width = span.end - span.start;
			let contains = span.start <= offset && offset <= span.end;
			if contains && best.is_none_or(|(best, _)| width < best) {
				best = Some((width, ScopeId::new(index)));
			}
		}
		best.map(|(_, scope)| scope).or_else(|| {
			// after the end of the module's span: the module scope
			scopes
				.iter()
				.position(|scope| scope.kind == crate::symbols::ScopeKind::Module)
				.map(ScopeId::new)
		})
	}
}

#[cfg(test)]
mod tests {
	use ethnum::I256;

	use super::*;
	use crate::analyze_source;

	fn offset_of(source: &str, needle: &str, nth: usize) -> u32 {
		source.match_indices(needle).nth(nth).expect("needle").0 as u32
	}

	#[test]
	fn hover_on_a_declaration_a_read_and_an_expression() {
		let source =
			"let answer = 42\nmutable count: Int8 = 1\nconsole.log(answer + 1, \"a\".length)\n";
		let (session, file) = analyze_source(source);
		let hover = session
			.hover(file, offset_of(source, "answer", 0) + 2)
			.unwrap();
		assert_eq!(hover.markdown, "```fa\nlet answer: Integer\n```");
		assert_eq!(hover.span, SourceSpan::new(4, 10));
		let hover = session.hover(file, offset_of(source, "count", 0)).unwrap();
		assert_eq!(hover.markdown, "```fa\nmutable count: Int8\n```");
		let read = session.hover(file, offset_of(source, "answer", 1)).unwrap();
		assert_eq!(
			read.markdown,
			"```fa\nlet answer: Integer\n```\n\nhere `42`"
		);
		let log = session.hover(file, offset_of(source, "log", 0)).unwrap();
		assert_eq!(
			log.markdown,
			"```fa\nconsole.log(...values): Void\n```\n\n(prelude)"
		);
		let length = session.hover(file, offset_of(source, "length", 0)).unwrap();
		assert!(
			length
				.markdown
				.starts_with("```fa\nString.length: Integer<min = 0>\n```"),
			"{}",
			length.markdown
		);
		let literal = session
			.hover(file, offset_of(source, "\"a\"", 0) + 1)
			.unwrap();
		assert_eq!(literal.markdown, "```fa\n\"a\"\n```");
		assert!(session.hover(file, source.len() as u32 + 10).is_none());
	}

	#[test]
	fn hover_shows_builtin_signatures_with_names_and_defaults() {
		let source = "let t = \"7\".padStart(3, \"0\")\nlet n = Integer.parse\n";
		let (session, file) = analyze_source(source);
		let pad = session
			.hover(file, offset_of(source, "padStart", 0))
			.unwrap();
		assert!(
			pad.markdown
				.contains("String.padStart(length: Integer, filler: String = …): String"),
			"{}",
			pad.markdown
		);
		let parse = session.hover(file, offset_of(source, "parse", 0)).unwrap();
		assert!(
			parse
				.markdown
				.contains("Integer.parse(text: String): Integer throws ParseError"),
			"{}",
			parse.markdown
		);
	}

	#[test]
	fn intervals_are_written_without_infinite_bounds() {
		assert_eq!(interval_text(Interval::from_i128(0, 10)), "0..=10");
		assert_eq!(interval_text(Interval::from_i128(3, 3)), "3");
		assert_eq!(interval_text(Interval::new(I256::ZERO, INFINITY)), "0..");
		assert_eq!(
			interval_text(Interval::new(-INFINITY, I256::new(5))),
			"..=5"
		);
		assert_eq!(interval_text(Interval::new(-INFINITY, INFINITY)), "..");
	}

	#[test]
	fn definition_of_a_read_and_of_a_declaration() {
		let source = "let answer = 42\nconsole.log(answer)\n";
		let (session, file) = analyze_source(source);
		let definition = session
			.definition(file, offset_of(source, "answer", 1))
			.unwrap();
		assert_eq!(definition.name_span, SourceSpan::new(4, 10));
		assert_eq!(definition.span.start, 0);
		assert_eq!(session.definition(file, 5), Some(definition));
		assert!(
			session
				.definition(file, offset_of(source, "log", 0))
				.is_none()
		);
	}

	#[test]
	fn completions_list_the_visible_names_then_the_prelude() {
		let source = "let first = 1\n\nlet later = 2\n";
		let (session, file) = analyze_source(source);
		let names: Vec<Completion> = session.completions(file, 14);
		let labels: Vec<&str> = names.iter().map(|c| c.label.as_str()).collect();
		assert!(labels.contains(&"first"));
		assert!(!labels.contains(&"later"), "declared after the cursor");
		assert!(labels.contains(&"console"));
		assert!(labels.contains(&"Integer"));
		assert!(labels.contains(&"arguments"));
		assert!(!labels.iter().any(|label| label.contains(['.', '#'])));
		let first = names.iter().find(|c| c.label == "first").unwrap();
		assert_eq!(first.kind, CompletionKind::Value);
		assert_eq!(first.detail.as_deref(), Some("Integer"));
	}

	#[test]
	fn completions_after_a_dot_list_the_members() {
		let source = "let t = \"a\"\nlet n = t.\n";
		let (session, file) = analyze_source(source);
		let members = session.completions(file, offset_of(source, "t.", 0) + 2);
		let labels: Vec<&str> = members.iter().map(|c| c.label.as_str()).collect();
		assert!(labels.contains(&"length"), "{labels:?}");
		assert!(labels.contains(&"includes"));
		let length = members.iter().find(|c| c.label == "length").unwrap();
		assert_eq!(length.kind, CompletionKind::Field);
		let source = "console.\n";
		let (session, file) = analyze_source(source);
		let labels: Vec<String> = session
			.completions(file, 8)
			.into_iter()
			.map(|c| c.label)
			.collect();
		assert_eq!(labels, ["log", "error"]);
	}

	#[test]
	fn queries_on_broken_code_do_not_panic() {
		for source in ["", "let = = (", "x.", "\"é", "let a = 1\n."] {
			let (session, file) = analyze_source(source);
			for offset in 0..=source.len() as u32 + 1 {
				let _ = session.hover(file, offset);
				let _ = session.definition(file, offset);
				let _ = session.completions(file, offset);
				let _ = session.node_at(file, offset);
			}
		}
	}
}
