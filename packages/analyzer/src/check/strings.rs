//! String literals, templates, printable values, members and concatenation.
//! Positions and literal length facts count UTF-16 code units.
//! `codeUnitAt` shares the indexing proofs; flags record allocation-free
//! code point counts, string indexing and UTF-8 iteration for the backends.

use std::sync::Arc;

use crate::builtins::{
	self,
	strings::{is_string_like, utf16_length},
};
use crate::check::Walker;
use crate::diagnostics::codes::values::{E_STR_7, E_STR_11, E_STR_12};
use crate::ids::{NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{TemplatePart, TemplateParts};
use crate::types::TypeKind;

impl Walker<'_> {
	/// A template `"Hello {name}"`: every interpolated value must have a
	/// `toString`. Always a `String`.
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
	/// `String`, with `E-STR-7` for every operand that isn't a string (Fa never converts to a string implicitly).
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

	/// Keep allocation-free string operations explicit in the public contract.
	pub(super) fn note_code_point_count(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: crate::syntax::NodeList<'_>,
	) {
		if members.len() == 1
			&& members
				.first()
				.is_some_and(|member| self.tree.text(member.index()) == "length")
			&& self.direct_string_call(target, "String#codePoints")
		{
			self.set_flag(n, crate::analysis::NodeFlags::CODE_POINT_COUNT);
		}
	}

	pub(super) fn direct_string_call(&self, n: NodeId, path: &str) -> bool {
		self.a.calls.get(&self.strip_groups(n)).is_some_and(|call|
			matches!(call.callee, crate::analysis::Callee::Builtin(id) if builtins::get(id).path == path))
	}

	/// `codeUnitAt` uses exactly the bounds proof and error path of indexing.
	pub(super) fn string_api_throws(
		&mut self,
		n: NodeId,
		callee: NodeId,
		kind: crate::analysis::Callee,
		arguments: &[crate::analysis::Argument],
		throws: TypeId,
		comptime: bool,
	) -> TypeId {
		if matches!(kind, crate::analysis::Callee::Builtin(id) if builtins::get(id).path == "String#codeUnitAt")
			&& let Some(target) = self.callee_receiver_node(callee)
			&& let Some(crate::analysis::Argument::Given(index)) = arguments.first()
		{
			if !comptime {
				self.check_index_bounds(n, target, *index);
			}
			return TypeId::NEVER;
		}
		throws
	}

	/// The member `name` of a string value (type part): `length`, a method.
	/// Sets the member's resolution; `None` when strings have no such member.
	/// The `length` of a literal is its literal UTF-16 code unit count.
	pub fn string_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		name: &str,
	) -> Option<TypeId> {
		let id = builtins::instance_member(&self.s.types, receiver, name)?;
		self.set_resolution(member, Resolution::Builtin(id));
		if let (TypeKind::StringLit(text), "length") = (self.s.types.kind(receiver), name) {
			let count = utf16_length(self.s.types.syms.resolve(text));
			return Some(self.s.types.int_literal((count as i128).into()));
		}
		builtins::value_type(&mut self.s.types, id)
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
			// the unused-value warning is tested by its own cases
			.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
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
	fn removed_character_type_names_the_replacement() {
		let (session, file) = analyze_source("let c: Character = \"F\"\n");
		let diagnostics = session.diagnostics(file);
		let diagnostic = diagnostics.iter().find(|d| d.code.id == "TY007").unwrap();
		assert!(diagnostic.message.contains("use `String`"));
	}

	#[test]
	fn removed_byte_view_names_the_explicit_encoding() {
		let (session, file) = analyze_source("let bytes = \"abc\".bytes()\n");
		assert!(
			session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.code.id == "E-OBJ-5"
					&& diagnostic.message.contains("text.toUtf8()"))
		);
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
			"Integer<min = 0>"
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
				"let c: String = \"a\"\nlet u = c.toUpperCase()\n",
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
