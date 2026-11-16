//! Ownership constraints on immutable escaping function environments.

use crate::check::Walker;
use crate::diagnostics::{
	Location,
	codes::advanced::{MEM003, MEM004},
};
use crate::ids::{NodeId, SymbolId};
use crate::syntax::{self, View};
use crate::types::{FieldFlags, Ownership, TypeKind};

impl Walker<'_> {
	/// Borrowing cannot supply an owned capture. A later direct source use
	/// also proves that the capture needs an independent snapshot.
	pub(super) fn check_structure_capture_stores(&mut self, graph: &super::graph::Graph) {
		let functions: Vec<_> = self
			.a
			.escapes
			.iter()
			.filter_map(|&lambda| self.a.function(lambda).cloned())
			.collect();
		for info in functions {
			let creation = info.declaration.unwrap_or(info.lambda);
			let Some(events) = graph.at.get(&creation) else {
				continue;
			};
			if !events.iter().any(|&index| graph.reachable[index]) {
				continue;
			}
			let mut copies = Vec::new();
			for symbol in info.captures {
				let Some(mut ty) = self.capture_types.get(&(info.lambda, symbol)).copied() else {
					continue;
				};
				while let TypeKind::Ref(target, _) = self.s.types.kind(ty) {
					ty = target;
				}
				if !self.s.types.is_structure(ty) {
					continue;
				}
				let owned = self.ownership_source_owned(symbol, creation);
				let later = graph.later_uses_where(symbol, |node| {
					let creation = matches!(
						self.view(node),
						View::Lambda { .. } | View::Function { .. } | View::ExportFunction { .. }
					) && self.a.function(node).is_some_and(|info| {
						info.declaration.unwrap_or(info.lambda) == node
							&& self.a.escapes.contains(&info.lambda)
					});
					creation
						|| matches!(self.view(node), View::Name(_))
							&& self.move_places.get(&node).is_some_and(|places| {
								places
									.iter()
									.any(|&place| self.place_symbol(place) == Some(symbol))
							})
				});
				let witness = events
					.iter()
					.filter(|&&index| graph.reachable[index])
					.find_map(|&index| later[index]);
				if !owned || witness.is_some() {
					copies.push((symbol, ty, witness));
				}
			}
			if copies.is_empty() || !self.reported_once.insert((info.lambda, "capture-copy")) {
				continue;
			}
			let description = copies
				.iter()
				.map(|(symbol, ty, later)| {
					let name = self
						.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(*symbol).name);
					let reason = later
						.map(|node| {
							if node == creation {
								format!(
									"used again on the next loop iteration (line {})",
									self.line_of(node)
								)
							} else {
								format!("used again on line {}", self.line_of(node))
							}
						})
						.unwrap_or_else(|| "borrowed from its owner".to_string());
					format!("`{name}` (`{}`), {reason}", self.display(*ty))
				})
				.collect::<Vec<_>>()
				.join("; ");
			let symbols: Vec<_> = copies.iter().map(|&(symbol, _, _)| symbol).collect();
			let field = symbols
				.iter()
				.find_map(|&symbol| self.capture_field_snapshot(info.body, symbol))
				.unwrap_or_default();
			let reason = self.unknown_callback_retention_hint(info.lambda);
			let mut diagnostic = self.diagnostic(&MEM004, creation, format!("this escaping function would silently copy a structure capture: {description}; create `let snapshot = copy value` before capturing it, keep the function nonescaping to borrow it, or move the later use before the capture.{field}{reason}"));
			for (symbol, _, later) in copies {
				if let Some(node) = later {
					diagnostic = diagnostic.with_related(
						Location {
							file: self.file,
							span: self.span(node),
						},
						"this later use keeps the source alive",
					);
				} else {
					diagnostic = diagnostic.with_related(
						Location {
							file: self.file,
							span: self.span(self.a.symbols.symbol(symbol).decl_node),
						},
						"this binding borrows the value and cannot give away its ownership",
					);
				}
			}
			if let Some(fix) = self.capture_snapshot_fix(info.lambda, info.declaration, &symbols) {
				diagnostic = diagnostic.with_fix(fix);
			}
			self.report(diagnostic);
		}
	}

	/// An owned environment reads its captures at creation, including when
	/// the function is never called. Keep the source-place snapshot.
	pub(super) fn capture_creation_reads(
		&mut self,
		reads: &mut rustc_hash::FxHashMap<NodeId, Vec<SymbolId>>,
	) {
		let functions: Vec<_> = self
			.a
			.escapes
			.iter()
			.filter_map(|&lambda| self.a.function(lambda))
			.map(|info| {
				(
					info.lambda,
					info.declaration.unwrap_or(info.lambda),
					info.captures.clone(),
				)
			})
			.collect();
		for (lambda, creation, captures) in functions {
			let mut places = Vec::new();
			for symbol in captures {
				if let Some(recorded) = self.capture_places.get(&(lambda, symbol)) {
					places.extend(recorded.iter().copied());
				}
			}
			self.observe_places(creation, places, reads);
		}
	}

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
		let reason = self.unknown_callback_retention_hint(lambda);
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
	fn escaping_named_functions_retain_a_started_local_callee() {
		assert_eq!(
			codes(
				"type Held = { name: String, function destroy = (self) => {} }\ntype Work = () => Task<Integer>\nfunction bad = (held: Held): Work => {\nfunction read = (): Integer => held.name.length\nfunction outer = (): Task<Integer> => start read()\nouter\n}\n"
			),
			["MEM003"]
		);
	}

	#[test]
	fn a_detached_call_owns_its_local_callee_environment() {
		assert_eq!(
			codes(
				"type Held = { name: String, function destroy = (self) => {} }\nfunction bad = (held: Held) => {\nasync function read = () => { await Task.yield()\nconsole.log(held) }\nstart read()\n}\n"
			),
			["MEM003"]
		);
	}

	#[test]
	fn owned_structure_captures_require_independence_from_borrows_and_later_uses() {
		for source in [
			"type Reader = () => Integer\nfunction keep = (items: Array<Integer>): Reader => () => items.length\n",
			"type Reader = () => Integer\nfunction keep = (): Array<Reader> => {\nlet items = [1]\nlet callbacks: Array<Reader> = [() => items.length]\nconsole.log(items.length)\ncallbacks\n}\n",
		] {
			assert_eq!(codes(source), ["MEM004"]);
		}
	}

	#[test]
	fn repeated_structure_captures_require_a_snapshot_but_exclusive_uses_do_not() {
		assert_eq!(
			codes(
				"type Reader = () => Integer\nfunction repeated = (): Array<Reader> => {\nlet items = [1]\nmutable callbacks: Array<Reader> = []\nfor 0..3 { callbacks.push(() => items.length) }\ncallbacks\n}\n"
			),
			["MEM004"]
		);
		analyzed(
			"type Reader = () => Integer\nfunction exclusive = (choose: Boolean): Integer => {\nlet items = [1]\nif choose {\nlet callbacks: Array<Reader> = [() => items.length]\nreturn callbacks[0]()\n}\nitems.length\n}\n",
		);
	}

	#[test]
	fn an_escaping_capture_reads_its_source_when_it_is_created() {
		assert_eq!(
			codes(
				"type Reader = () => Integer\nfunction make = (): (Array<Integer>, Array<Reader>) => {\nlet items = [1]\nlet first = items\nlet callbacks: Array<Reader> = [() => items.length]\n(first, callbacks)\n}\n"
			),
			["MEM004"]
		);
	}

	#[test]
	fn unique_capture_diagnostic_explains_unknown_retention_and_field_snapshot() {
		for (field, code) in [
			("task: Task<Integer>", "MEM003"),
			("items: Array<Integer>", "MEM004"),
		] {
			let source = format!(
				"type Reader = () => Integer\nfunction run = (operation: (Reader) => Integer, holder: {{ {field}, count: Integer }}): Integer => operation(() => holder.count)\n"
			);
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
			assert_eq!(diagnostic.code.id, code);
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
