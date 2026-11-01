//! Strings and characters (values brief §7, `values/strings.md`,
//! `operations/string.md`): literal typing and the adaptation to `Character`,
//! templates, printable values, the members of string values, concatenation.
//!
//! The handlers `check_string` and `check_template` of `expr.rs` call these.
//! Hooks for the other areas:
//!
//! - `+` (`numbers.rs`): when an operand is a string, call
//!   [`Walker::string_concatenation`] from the type part instead of the
//!   arithmetic rule; `+=` on a `String` target checks its value the same way.
//! - sinks (`mod.rs`): before reporting `E-TYPE-1`, ask
//!   [`Walker::string_sink_mismatch`], which reports `E-STR-8` for a `String`
//!   where a `Character` is expected.
//! - calls (`calls.rs`): the arguments of `console.log` / `console.error`
//!   (`builtins::console::prints`) go through [`Walker::check_printable`].
//! - member access (`expr.rs`): [`Walker::string_member`] resolves a member
//!   of a string value.

use std::sync::Arc;

use crate::builtins::{
	self,
	strings::{grapheme_count, is_string_like},
};
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::values::{E_OBJ_5, E_STR_7, E_STR_8, E_STR_9, E_STR_11, E_STR_12};
use crate::ids::{NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, NodeList, TemplatePart, TemplateParts, View};
use crate::types::{Mode, TypeKind, TypeStore};

/// Whether an expected type takes a `Character`: `Character`, or a union
/// with a `Character` member (`Character?`).
fn takes_character(types: &TypeStore, expected: TypeId) -> bool {
	match types.kind(expected) {
		TypeKind::Character => true,
		TypeKind::Union(list) => types.list_items(list).contains(&TypeId::CHARACTER),
		_ => false,
	}
}

impl Walker<'_> {
	/// The type of a string literal (type part): its literal type, or
	/// `Character` where one is expected and the text is one grapheme
	/// cluster (`E-STR-9` when it isn't).
	pub(super) fn string_literal_type(
		&mut self,
		n: NodeId,
		text: &str,
		expected: Option<TypeId>,
	) -> TypeId {
		let literal = self.s.types.string_literal(text);
		let Some(expected) = expected else {
			return literal;
		};
		if self.s.types.is_recovering(expected)
			|| self
				.s
				.types
				.assignable(literal, expected, Mode::Copy)
				.is_ok() || !takes_character(&self.s.types, expected)
		{
			return literal;
		}
		let count = grapheme_count(text);
		if count == 1 {
			return TypeId::CHARACTER;
		}
		let found = if count == 0 {
			"an empty string".to_string()
		} else {
			format!("{count} characters")
		};
		let diagnostic = self.diagnostic(
			&E_STR_9,
			n,
			format!(
				"a `Character` holds exactly one grapheme cluster, this literal has {found}: use a `String`"
			),
		);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}

	/// A template `"Hello {name}"`: every interpolated value must have a
	/// `toString`. Always a `String`, even where a `Character` is expected.
	pub(super) fn template_type(&mut self, n: NodeId, parts: TemplateParts<'_>) -> TypeId {
		let mut values = smallvec::SmallVec::<[(NodeId, TypeId); 4]>::new();
		for part in parts.iter() {
			if let TemplatePart::Expression(value) = part {
				let ty = self.check(value, None);
				values.push((value, ty));
			}
		}
		self.memo(n, |w| {
			for (value, ty) in values {
				w.check_printable(value, ty);
			}
			TypeId::STRING
		})
	}

	/// Reports a value that has no `toString` (type part): `Void`
	/// (`E-STR-11`) or a function (`E-STR-12`). Interpolation and the
	/// arguments of `console.log` go through it.
	pub fn check_printable(&mut self, value: NodeId, ty: TypeId) {
		let types = &self.s.types;
		// not `is_recovering`: a function type may hold `Unknown` (the
		// parameters of `console.log`) and still be a function
		if ty.is_unset() || ty == TypeId::UNKNOWN {
			return;
		}
		let has_function = |ty: TypeId| matches!(types.kind(ty), TypeKind::Function(_));
		let function = has_function(ty)
			|| matches!(types.kind(ty), TypeKind::Union(list)
				if types.list_items(list).iter().any(|&member| has_function(member)));
		let (code, message) = if ty == TypeId::VOID {
			(
				&E_STR_11,
				"this expression has no value (`Void`): there is nothing to print".to_string(),
			)
		} else if function {
			(
				&E_STR_12,
				format!(
					"a function (`{}`) has no `toString`: call it, or print something else",
					self.display(ty)
				),
			)
		} else {
			return;
		};
		let diagnostic = self.diagnostic(code, value, message);
		self.report(diagnostic);
	}

	/// The type of `a + b + …` when an operand is a string (type part): a
	/// `String`, with `E-STR-7` for every operand that isn't a string or a
	/// `Character` (Fa never converts to a string implicitly).
	pub fn string_concatenation(&mut self, operands: &[(NodeId, TypeId)]) -> TypeId {
		for &(operand, ty) in operands {
			if ty.is_unset() || self.s.types.is_recovering(ty) || is_string_like(&self.s.types, ty)
			{
				continue;
			}
			let tree = Arc::clone(&self.tree);
			let text = tree.text(operand.index());
			let example = if text.len() <= 20 && !text.contains(['\n', '"']) {
				format!(": use `\"…{{{text}}}\"` or `{text}.toString()`")
			} else {
				": interpolate it, `\"…{value}\"`, or call `toString()`".to_string()
			};
			let diagnostic = self.diagnostic(
				&E_STR_7,
				operand,
				format!(
					"`+` joins strings: expected a `String`, found a `{}`{example}",
					self.display(ty)
				),
			);
			self.report(diagnostic);
		}
		TypeId::STRING
	}

	/// Reports `E-STR-8` when a `String` lands where a `Character` is
	/// expected (a flow-part check, like `sink`); returns whether it did.
	/// A literal never gets here: it adapts, or reports `E-STR-9`.
	pub fn string_sink_mismatch(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		let types = &self.s.types;
		let is_string = matches!(
			types.kind(actual),
			TypeKind::String | TypeKind::StringLit(_)
		);
		if !is_string || !takes_character(types, target) {
			return false;
		}
		let diagnostic = self.diagnostic(
			&E_STR_8,
			n,
			format!(
				"a `String` may hold several characters, a `{}` is expected: take one with `text[index]` or `text.get(index)`",
				self.display(target)
			),
		);
		self.report_flow(diagnostic);
		true
	}

	/// The member `name` of a string value (type part): `length`, a method.
	/// Sets the member's resolution; `None` when strings have no such member.
	/// The `length` of a literal is its literal grapheme count.
	pub fn string_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		name: &str,
	) -> Option<TypeId> {
		let id = builtins::instance_member(&self.s.types, receiver, name)?;
		self.set_resolution(member, Resolution::Builtin(id));
		if let (TypeKind::StringLit(text), "length") = (self.s.types.kind(receiver), name) {
			let count = grapheme_count(self.s.types.syms.resolve(text));
			return Some(self.s.types.int_literal((count as i128).into()));
		}
		builtins::value_type(&mut self.s.types, id)
	}

	/// `value.member…` on a value that is not a built-in namespace: the
	/// members of strings for now; other values report `AN001` (field access
	/// comes with objects, E1).
	pub(super) fn check_value_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> TypeId {
		let receiver = self.check(target, None);
		self.memo(n, |w| {
			let tree = Arc::clone(&w.tree);
			let mut current = receiver;
			for member in members.iter() {
				if current == TypeId::UNKNOWN {
					return TypeId::UNKNOWN;
				}
				let View::Name(name) = syntax::view(&tree, member) else {
					return TypeId::UNKNOWN;
				};
				let text = tree.name(name);
				if !is_string_like(&w.s.types, current) {
					return w.not_supported_here(n, "field access");
				}
				match w.string_member(member, current, text) {
					Some(ty) => current = ty,
					None => {
						let message = format!(
							"a `{}` has no field or method `{text}`: see the string methods (`length`, `includes`, `split`…); no method changes a string",
							w.display(w.s.types.widen(current))
						);
						let diagnostic = w.diagnostic(&E_OBJ_5, member, message);
						w.report(diagnostic);
						return TypeId::UNKNOWN;
					}
				}
			}
			current
		})
	}

	/// `AN001` for a node whose children were already walked.
	fn not_supported_here(&mut self, n: NodeId, what: &str) -> TypeId {
		let message = format!("{what} is not supported yet by the analyzer");
		let diagnostic = self.diagnostic(&AN001, n, message);
		self.report(diagnostic);
		TypeId::UNKNOWN
	}
}

#[cfg(test)]
mod tests {
	use crate::analyze_source;
	use crate::ids::{NodeId, TypeId};
	use crate::symbols::Resolution;

	fn codes(source: &str) -> Vec<&'static str> {
		let (session, file) = analyze_source(source);
		session
			.diagnostics(file)
			.iter()
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	/// The display of the type of the last node whose text is `text`.
	fn type_of(source: &str, text: &str) -> String {
		let (session, file) = analyze_source(source);
		let tree = session.tree(file);
		let node = (0..tree.nodes.len())
			.rev()
			.find(|&node| tree.text(node) == text)
			.unwrap_or_else(|| panic!("no node `{text}`"));
		session.display_type(session.analysis(file).type_of(NodeId::new(node)))
	}

	fn symbol_type(source: &str, name: &str) -> TypeId {
		let (session, file) = analyze_source(source);
		let analysis = session.analysis(file);
		let symbol = analysis
			.symbols
			.symbols()
			.iter()
			.find(|symbol| session.types().syms.resolve(symbol.name) == name)
			.unwrap();
		symbol.ty
	}

	#[test]
	fn literals_are_strings_and_bindings_widen() {
		assert!(codes("let greeting = \"Hello\"\nconsole.log(greeting)\n").is_empty());
		assert_eq!(
			type_of("let greeting = \"Hello\"\n", "\"Hello\""),
			"\"Hello\""
		);
		assert_eq!(
			symbol_type("let greeting = \"Hello\"\n", "greeting"),
			TypeId::STRING
		);
		assert_eq!(symbol_type("let s: String = \"a\"\n", "s"), TypeId::STRING);
	}

	#[test]
	fn literals_adapt_to_character() {
		for source in [
			"let snow: Character = \"☃\"\n",
			"let flag: Character = \"🇫🇷\"\n",
			"let accent: Character = \"e\\u{301}\"\n",
		] {
			assert!(codes(source).is_empty(), "{source}");
		}
		assert_eq!(
			type_of("let flag: Character = \"🇫🇷\"\n", "\"🇫🇷\""),
			"Character"
		);
		assert_eq!(codes("let letters: Character = \"ab\"\n"), ["E-STR-9"]);
		assert_eq!(codes("let e: Character = \"\"\n"), ["E-STR-9"]);
	}

	#[test]
	fn optional_characters_take_a_character() {
		let mut types = crate::types::TypeStore::new();
		let optional = types.optional(TypeId::CHARACTER);
		assert!(super::takes_character(&types, optional));
		assert!(super::takes_character(&types, TypeId::CHARACTER));
		assert!(!super::takes_character(&types, TypeId::STRING));
		let optional_string = types.optional(TypeId::STRING);
		assert!(!super::takes_character(&types, optional_string));
	}

	#[test]
	fn characters_are_strings_not_the_reverse() {
		assert!(codes("let first: Character = \"F\"\nlet text: String = first\n").is_empty());
		// E-STR-8 needs the sink hook (E1): E-TYPE-1 until then
		let found = codes("let text = \"F\"\nlet c: Character = text\n");
		assert_eq!(found.len(), 1);
		assert!(["E-STR-8", "E-TYPE-1"].contains(&found[0]));
	}

	#[test]
	fn templates_are_strings_of_printable_values() {
		assert!(codes("let name = \"Fa\"\nconsole.log(\"Hello {name}\")\n").is_empty());
		assert!(codes("console.log(\"Braces: {{ and }}\")\n").is_empty());
		assert!(codes("let n = 1\nlet s = \"{n} {true} {\"nested {n}\"}\"\n").is_empty());
		assert_eq!(
			type_of("let n = 1\nlet s = \"a{n}\"\n", "\"a{n}\""),
			"String"
		);
		assert_eq!(
			symbol_type("let n = 1\nlet s = \"a{n}\"\n", "s"),
			TypeId::STRING
		);
		assert_eq!(
			codes("let x = console.log(\"a\")\nlet s = \"{console.log(1)}\"\n"),
			["E-DECL-9", "E-STR-11"]
		);
		assert_eq!(codes("let s = \"{console.log}\"\n"), ["E-STR-12"]);
		assert_eq!(codes("let s = \"{missing}\"\n"), ["E-DECL-1"]);
	}

	#[test]
	fn members_of_strings() {
		assert_eq!(
			type_of("let n = \"héllo\".length\n", "\"héllo\".length"),
			"5"
		);
		assert_eq!(
			symbol_type("let n = \"héllo\".length\n", "n"),
			TypeId::INTEGER
		);
		assert_eq!(
			type_of("let t = \"a\"\nlet n = t.length\n", "t.length"),
			"Integer(min = 0)"
		);
		assert!(
			codes(
				"let t = \"hello world\"\nlet a = t.includes(\"world\")\nlet b = t.toUpperCase()\nlet c = \"a,b\".split(\",\")\n"
			)
			.is_empty()
		);
		assert_eq!(
			type_of(
				"let t = \"a\"\nlet i = t.indexOf(\"b\")\n",
				"t.indexOf(\"b\")"
			),
			"Integer?"
		);
		assert_eq!(
			type_of(
				"let c: Character = \"a\"\nlet u = c.toUpperCase()\n",
				"c.toUpperCase()"
			),
			"String"
		);
		assert_eq!(codes("let t = \"a\"\nt.push(\"d\")\n"), ["E-OBJ-5"]);
		assert_eq!(
			codes("let t = \"a\"\nlet n = t.includes(1)\n"),
			["E-TYPE-1"]
		);
		let (session, file) = analyze_source("let n = \"a\".length\n");
		let tree = session.tree(file);
		let length = (0..tree.nodes.len())
			.find(|&node| tree.text(node) == "length")
			.unwrap();
		let Resolution::Builtin(id) = session.analysis(file).resolution(NodeId::new(length)) else {
			panic!("`length` is not resolved");
		};
		assert_eq!(crate::builtins::get(id).path, "String#length");
	}

	#[test]
	fn associated_functions_of_built_in_types() {
		// defaults need the call binding of E1: `locale` is given here
		let source = "let r = String.compareLocale(\"a\", \"b\", none)\n";
		assert!(codes(source).is_empty());
		assert_eq!(
			type_of(source, "String.compareLocale(\"a\", \"b\", none)"),
			"Integer"
		);
		assert_eq!(
			type_of("let parse = Integer.parse\n", "Integer.parse"),
			"(String) => Integer throws ParseError"
		);
		assert_eq!(codes("let x = Integer.nope\n"), ["AN001"]);
	}
}
