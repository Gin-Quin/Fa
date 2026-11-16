//! Ownership constraints on immutable escaping function environments.

use crate::check::Walker;
use crate::diagnostics::{Location, codes::advanced::MEM003};
use crate::ids::{NodeId, SymbolId};
use crate::syntax::{self, View};
use crate::types::{FieldFlags, Ownership, TypeKind};

impl Walker<'_> {
	pub(crate) fn check_unique_captures(&mut self, lambda: NodeId, at: NodeId) {
		let Some(info) = self.a.function(lambda).cloned() else {
			return;
		};
		let captures: Vec<_> = info
			.captures
			.iter()
			.copied()
			.filter_map(|symbol| {
				let mut ty = self.capture_types.get(&(lambda, symbol)).copied()?;
				// An escaping `use` capture stores the designated value.
				while let TypeKind::Ref(target, _) = self.s.types.kind(ty) {
					ty = target;
				}
				self.s.types.is_unique(ty).then_some((symbol, ty))
			})
			.collect();
		if captures.is_empty() || !self.reported_once.insert((lambda, "unique-capture")) {
			return;
		}
		let names = captures
			.iter()
			.map(|(symbol, ty)| {
				format!(
					"`{}` (`{}`)",
					self.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(*symbol).name),
					self.display(*ty)
				)
			})
			.collect::<Vec<_>>()
			.join(", ");
		let field = captures
			.iter()
			.find_map(|(symbol, _)| self.capture_field_snapshot(info.body, *symbol))
			.unwrap_or_default();
		let reason = if self.unknown_callback_escapes.contains_key(&lambda) {
			" The callback may reach a function value whose type doesn't say whether it keeps the callback; call a known borrowing function directly, or give a callback whose captures it can own."
		} else {
			""
		};
		let message = format!(
			"an escaping function cannot own the unique capture {names}: `copy` cannot duplicate it; keep the function nonescaping, or keep the unique owner in its owning scope and pass it explicitly when needed.{field}{reason}"
		);
		let mut diagnostic = self
			.diagnostic(&MEM003, info.declaration.unwrap_or(lambda), message)
			.with_related(
				Location {
					file: self.file,
					span: self.span(at),
				},
				"this use lets the function escape",
			);
		for (symbol, _) in captures {
			diagnostic = diagnostic.with_related(
				Location {
					file: self.file,
					span: self.span(self.a.symbols.symbol(symbol).decl_node),
				},
				"this value is unique or owns a unique value",
			);
		}
		self.report(diagnostic);
	}

	fn capture_field_snapshot(&self, body: NodeId, symbol: SymbolId) -> Option<String> {
		let mut pending = vec![body];
		while let Some(node) = pending.pop() {
			if let View::Access { target, members } = self.view(node)
				&& members.len() == 1
				&& crate::lower::copies::place_root(&self.tree, &self.a, target) == Some(symbol)
				&& let Some(member) = members.last()
				&& let View::Name(name) = self.view(member)
				&& let Some(name) = self.s.types.syms.get(self.tree.name(name))
				&& let Some(field) =
					self.s
						.types
						.fields_of(self.a.type_of(target))
						.iter()
						.find(|field| {
							field.name == name
								&& !field
									.flags
									.intersects(FieldFlags::METHOD | FieldFlags::RELATION)
						}) {
				let copy = match self.s.types.ownership(field.ty) {
					Ownership::Ordinary | Ownership::SharedHandle => "",
					Ownership::Structure => "copy ",
					_ => {
						pending.extend(syntax::children(&self.tree, node));
						continue;
					}
				};
				return Some(format!(
					" Field-path capture is not implemented yet: bind only the needed field before the function, e.g. `let {} = {copy}{}`, then capture that local.",
					self.s.types.syms.resolve(name),
					self.tree.text(node.index())
				));
			}
			pending.extend(syntax::children(&self.tree, node));
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::analyzed;

	fn codes(source: &str) -> Vec<&'static str> {
		let mut session = crate::Session::new();
		let file = session.set_source("captures.foreign", std::sync::Arc::from(source));
		session.analyze_file(file);
		session
			.diagnostics(file)
			.iter()
			.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	#[test]
	fn escaping_functions_cannot_own_unique_captures() {
		let resource = "type Reader = () => Integer\ntype Held = { name: String, function destroy = (self) => {} }\nfunction size = (value: Held): Integer => value.name.length\n";
		for body in [
			"function bad = (value: Held): Reader => () => size(value)\n",
			"function bad = (value: Held): Reader => {\nuse borrowed = value\n() => size(borrowed)\n}\n",
		] {
			assert_eq!(codes(&format!("{resource}{body}")), ["MEM003"]);
		}
		assert_eq!(
			codes(
				"type Reader = () => Integer\nfunction bad = (task: Task<Integer>): Reader => () => {\nuse borrowed = task\n0\n}\n"
			),
			["MEM003"]
		);
		assert_eq!(
			codes(
				"use { TcpStream } = std.net\ntype Reader = () => Integer\nfunction size = (stream: TcpStream): Integer => { use borrowed = stream\n0 }\nfunction bad = (stream: TcpStream): Reader => () => size(stream)\n"
			),
			["MEM003"]
		);
	}

	#[test]
	fn an_owned_function_environment_retains_its_captured_function_handles() {
		let resource = "type Reader = () => Integer\ntype Held = { name: String, function destroy = (self) => {} }\nfunction size = (value: Held): Integer => value.name.length\n";
		for body in [
			"function bad = (value: Held): Reader => {\nfunction read = (): Integer => size(value)\nlet selected = read\n() => selected()\n}\n",
			"function bad = (value: Held): Reader => {\nfunction read = (): Integer => size(value)\nlet selected = read\nfunction next = (): Integer => selected()\nlet last = next\n() => last()\n}\n",
			"function bad = (value: Held): Reader => {\nfunction read = (): Integer => size(value)\nfunction outer = (): Integer => read()\nouter\n}\n",
		] {
			assert_eq!(codes(&format!("{resource}{body}")), ["MEM003"]);
		}
	}

	#[test]
	fn unique_capture_diagnostic_explains_unknown_retention_and_field_snapshot() {
		let source = "type Reader = () => Integer\nfunction run = (operation: (Reader) => Integer, holder: { task: Task<Integer>, count: Integer }): Integer => operation(() => holder.count)\n";
		let mut session = crate::Session::new();
		let file = session.set_source("captures.fa", std::sync::Arc::from(source));
		session.analyze_file(file);
		let diagnostics: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
			.collect();
		assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
		let diagnostic = diagnostics[0];
		assert_eq!(diagnostic.code.id, "MEM003");
		assert!(
			diagnostic
				.message
				.contains("type doesn't say whether it keeps the callback"),
			"{}",
			diagnostic.message
		);
		assert!(
			diagnostic.message.contains("let count = holder.count"),
			"{}",
			diagnostic.message
		);
		assert!(!diagnostic.related.is_empty());
	}

	#[test]
	fn capture_category_uses_the_type_narrowed_at_creation() {
		assert!(codes("type Reader = () => Integer\ntype Held = { name: String, function destroy = (self) => {} }\nfunction make = (value: Held | Integer): Reader => if value is Integer { () => value } else { () => 0 }\n").is_empty());
	}

	#[test]
	fn borrowed_resources_and_shared_handles_are_not_unique_copies() {
		assert!(codes("type Held = { name: String, function destroy = (self) => {} }\nfunction size = (value: Held): Integer => {\nfunction read = (): Integer => value.name.length\nread()\n}\n").is_empty());
		analyzed(
			"use { AtomicInteger } = std.concurrency\ntype Reader = () => Integer\nfunction read = (counter: AtomicInteger): Reader => () => counter.load()\n",
		);
	}
}
