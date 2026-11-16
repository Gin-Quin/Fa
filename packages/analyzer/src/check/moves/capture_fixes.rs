//! Explicit snapshots at the same point as an implicit capture store.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::check::Walker;
use crate::diagnostics::{Fix, Location, SourceSpan, TextEdit};
use crate::ids::{NodeId, SymbolId};
use crate::symbols::Resolution;
use crate::syntax::{self, View};

impl Walker<'_> {
	pub(super) fn capture_snapshot_fix(
		&self,
		lambda: NodeId,
		declaration: Option<NodeId>,
		captures: &[SymbolId],
	) -> Option<Fix> {
		let mut used: FxHashSet<_> = self
			.a
			.symbols
			.symbols()
			.iter()
			.map(|symbol| self.s.types.syms.resolve(symbol.name).to_string())
			.collect();
		let mut names = FxHashMap::default();
		let mut bindings = Vec::new();
		for &symbol in captures {
			let source = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name);
			let mut name = format!("{source}Snapshot");
			let mut suffix = 2;
			while !used.insert(name.clone()) {
				name = format!("{source}Snapshot{suffix}");
				suffix += 1;
			}
			bindings.push(format!("let {name} = copy {source}"));
			names.insert(symbol, name);
		}
		let span = self.span(lambda);
		let mut rewritten = self.tree.text(lambda.index()).to_string();
		let mut changes = Vec::new();
		let mut pending = vec![lambda];
		while let Some(node) = pending.pop() {
			if matches!(self.view(node), View::Name(_))
				&& let Resolution::Local(symbol) = self.a.resolution(node)
				&& let Some(name) = names.get(&symbol)
			{
				let shorthand = self.parent(node).is_some_and(|parent| {
					matches!(
						self.view(parent),
						View::Object { .. } | View::TypedObject { .. }
					)
				});
				let replacement = if shorthand {
					format!("{} = {name}", self.tree.text(node.index()))
				} else {
					name.clone()
				};
				changes.push((self.span(node), replacement));
			}
			pending.extend(syntax::children(&self.tree, node));
		}
		changes.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
		for (change, replacement) in changes {
			if change.start < span.start || change.end > span.end {
				return None;
			}
			rewritten.replace_range(
				(change.start - span.start) as usize..(change.end - span.start) as usize,
				&replacement,
			);
		}
		let start = declaration
			.map(|node| self.span(node).start)
			.unwrap_or(span.start);
		let prefix = &self.tree.source[..start as usize];
		let indent: String = prefix
			.rsplit('\n')
			.next()
			.unwrap_or("")
			.chars()
			.take_while(|character| matches!(character, ' ' | '\t'))
			.collect();
		let mut edits = Vec::new();
		if let Some(declaration) = declaration {
			if !self.parent(declaration).is_some_and(|parent| {
				matches!(self.view(parent), View::Block { .. } | View::Module { .. })
			}) {
				return None;
			}
			let text = format!("{}\n{indent}", bindings.join(&format!("\n{indent}")));
			edits.push(TextEdit {
				location: Location {
					file: self.file,
					span: SourceSpan::new(start, start),
				},
				replacement: text,
			});
		} else {
			let inner = format!("{indent}\t");
			rewritten = format!(
				"do {{\n{inner}{}\n{inner}{rewritten}\n{indent}}}",
				bindings.join(&format!("\n{inner}"))
			);
		}
		edits.push(TextEdit {
			location: Location {
				file: self.file,
				span,
			},
			replacement: rewritten,
		});
		Some(Fix {
			title: "Capture independent explicit snapshots".to_string(),
			edits,
		})
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::analyzed;

	fn fixed(mut source: String) -> String {
		for _ in 0..4 {
			let (session, file) = crate::analyze_source(&source);
			let errors: Vec<_> = session
				.diagnostics(file)
				.iter()
				.filter(|diagnostic| diagnostic.code.id != "W-DECL-2")
				.collect();
			if errors.is_empty() {
				analyzed(&source);
				return source;
			}
			assert!(
				errors
					.iter()
					.all(|diagnostic| diagnostic.code.id == "MEM004"),
				"{errors:#?}\n{source}"
			);
			let diagnostic = errors
				.iter()
				.find(|diagnostic| diagnostic.message.contains("structure capture"))
				.copied()
				.unwrap_or(errors[0]);
			let [fix] = diagnostic.fixes.as_slice() else {
				panic!("one snapshot/copy fix: {diagnostic:#?}")
			};
			let mut edits = fix.edits.clone();
			edits.sort_by_key(|edit| std::cmp::Reverse(edit.location.span.start));
			for edit in edits {
				assert_eq!(edit.location.file, file);
				source.replace_range(
					edit.location.span.start as usize..edit.location.span.end as usize,
					&edit.replacement,
				);
			}
		}
		panic!("snapshot fixes did not converge: {source}")
	}

	#[test]
	fn snapshots_keep_anonymous_and_named_captures_at_their_evaluation_point() {
		for source in [
			"type Reader = () => Integer\nfunction keep = (items: Array<Integer>): Reader => () => items.length\n",
			"type Reader = () => Integer\nfunction keep = (items: Array<Integer>): Reader => {\nfunction read = (): Integer => items.length\nread\n}\n",
			"type Reader = () => Integer\nfunction keep = (left: Array<Integer>, right: Array<Integer>): Reader => () => left.length + right.length\n",
		] {
			let result = fixed(source.to_string());
			assert!(result.contains("copy "), "{result}");
		}
	}

	#[test]
	fn snapshots_preserve_shorthand_keys_and_avoid_existing_names() {
		let result = fixed("type Result = { items: Array<Integer> }\ntype Reader = () => Result\nfunction keep = (items: Array<Integer>, itemsSnapshot: Integer): Reader => () => ({ items })\n".to_string());
		assert!(result.contains("itemsSnapshot2"), "{result}");
		assert!(result.contains("items = copy itemsSnapshot2"), "{result}");
	}
}
