//! Lexical readiness facts, separate from capturefulness and ownership.
//! External borrowed-wrapper states are completed by call propagation.

use rustc_hash::FxHashMap;

use super::graph::Graph;
use crate::analysis::{
	DeclarationReadiness as Ready, FunctionCaptureState, NamedFunctionReadiness, NodeFlags,
};
use crate::check::Walker;
use crate::ids::NodeId;
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

impl Walker<'_> {
	pub(crate) fn publish_local_function_readiness(&mut self) {
		let mut reads = self.move_reads.clone();
		let mut references = Vec::new();
		// Key snapshots can retain a resolved function body while its
		// per-key expression types are unset. Keep its dependencies;
		// missing CFG/capture facts yield Pending, not an omitted read.
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if !matches!(self.view(node), View::Name(_))
				|| self.a.has_flags(node, NodeFlags::TYPE_EXPR)
			{
				continue;
			}
			let Resolution::Local(symbol) = self.a.resolution(node) else {
				continue;
			};
			let data = self.a.symbols.symbol(symbol);
			if data.kind != SymbolKind::Function {
				continue;
			}
			let Some(info) = self.a.function(data.decl_node) else {
				continue;
			};
			let Some(declaration) = info.declaration else {
				continue;
			};
			reads.entry(node).or_default().push(symbol);
			references.push((node, symbol, declaration, info.lambda));
		}
		let root = NodeId::new(self.tree.root);
		let mut graphs = FxHashMap::default();
		let commits = FxHashMap::default();
		let mut before = FxHashMap::default();
		let mut published = FxHashMap::default();
		for (node, symbol, declaration, lambda) in references {
			let state = self.a.function_capture_states.get(&lambda).copied();
			let context = self.ownership_body(node);
			let (borrowed, owned) = if matches!(state, None | Some(FunctionCaptureState::Pending)) {
				(Ready::Pending, Ready::Pending)
			} else if state == Some(FunctionCaptureState::Empty) {
				(Ready::Ready, Ready::Ready)
			} else if context == lambda {
				(Ready::SelfInvocation, Ready::SelfInvocation)
			} else if self.ownership_body(declaration) == context {
				let body = self.a.function(context).map_or(root, |info| info.body);
				let graph = graphs
					.entry(context)
					.or_insert_with(|| Graph::build(self, body, &reads, &commits));
				let key = (context, symbol);
				let positions = before.entry(key).or_insert_with(|| {
					let block = self.function_declaration_block(declaration);
					graph.before_definition(block, symbol)
				});
				let ready = match positions
					.as_ref()
					.and_then(|positions| graph.definition_reached(node, positions))
				{
					Some(true) => Ready::Ready,
					Some(false) => Ready::Early,
					None => Ready::Pending,
				};
				(ready, ready)
			} else {
				// An owned caller keeps an initialized handle. A borrowed
				// wrapper may run on either side of the declaration; do not
				// infer its state from where that wrapper was written.
				(Ready::Pending, Ready::Ready)
			};
			published.insert(
				node,
				NamedFunctionReadiness {
					declaration,
					borrowed,
					owned,
				},
			);
		}
		self.a.named_function_readiness = published;
		self.propagate_function_readiness();
	}

	pub(super) fn function_declaration_block(&self, mut declaration: NodeId) -> NodeId {
		while let Some(parent) = self.parent(declaration) {
			if matches!(self.view(parent), View::Block { .. } | View::Module { .. }) {
				return parent;
			}
			declaration = parent;
		}
		NodeId::new(self.tree.root)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn states(source: &str, function: &str) -> Vec<NamedFunctionReadiness> {
		let (session, file) = crate::analyze_source(source);
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let mut states: Vec<_> = analysis
			.named_function_readiness
			.iter()
			.filter(|(node, _)| tree.text(node.index()) == function)
			.map(|(node, &state)| (*node, state))
			.collect();
		states.sort_by_key(|(node, _)| *node);
		assert!(
			!states.is_empty(),
			"no readiness for {function}: {:?}",
			session.diagnostics(file)
		);
		assert_eq!(
			analysis.named_function_readiness,
			analysis.clone().named_function_readiness
		);
		states.into_iter().map(|(_, state)| state).collect()
	}

	#[test]
	fn local_reads_keep_the_declaration_barrier() {
		let found = states(
			"function make = (n: Integer): Integer => {\n let before = read()\n function read = (): Integer => n\n before + read()\n}\n",
			"read",
		);
		assert_eq!(
			found.iter().map(|state| state.borrowed).collect::<Vec<_>>(),
			[Ready::Early, Ready::Ready]
		);
	}

	#[test]
	fn captureless_values_need_no_declaration_environment() {
		let found = states(
			"function make = (): Integer => {\n let handle = read\n function read = (): Integer => 1\n handle()\n}\n",
			"read",
		);
		assert_eq!(found[0].borrowed, Ready::Ready);
	}

	#[test]
	fn a_borrowed_wrapper_does_not_snapshot_readiness_at_its_declaration() {
		let found = states(
			"function make = (n: Integer): () => Integer => {\n function read = (): Integer => n\n function invoke = (): Integer => read()\n invoke\n}\n",
			"read",
		);
		assert_eq!(found[0].borrowed, Ready::Pending);
		assert_eq!(found[0].owned, Ready::Ready);
	}

	#[test]
	fn recursion_uses_the_running_invocation() {
		let found = states(
			"function make = (n: Integer): () => Integer => {\n function read = (count: Integer = 0): Integer => if count == 0 { n } else { read(count - 1) }\n read\n}\n",
			"read",
		);
		assert_eq!(found[0].borrowed, Ready::SelfInvocation);
		assert_eq!(found[0].owned, Ready::SelfInvocation);
		assert_eq!(found[1].borrowed, Ready::Ready);
	}

	#[test]
	fn each_loop_body_has_a_fresh_declaration_barrier() {
		let found = states(
			"function run = (): Integer => {\n mutable total = 0\n for index in 0..2 {\n total += read()\n function read = (): Integer => index\n total += read()\n }\n total\n}\n",
			"read",
		);
		assert_eq!(
			found.iter().map(|state| state.borrowed).collect::<Vec<_>>(),
			[Ready::Early, Ready::Ready]
		);
	}
	#[test]
	fn wrapper_readiness_joins_actual_call_positions() {
		let source = "function make = (n: Integer): Integer => {\n function invoke = (): Integer => read()\n let before = invoke()\n function read = (): Integer => n\n before + invoke()\n}\n";
		let found = states(source, "read");
		assert_eq!(found[0].borrowed, Ready::Mixed);
	}

	#[test]
	fn wrappers_called_on_one_side_keep_a_precise_state() {
		for (body, expected) in [
			(
				"let value = invoke()\n function read = (): Integer => n\n value",
				Ready::Early,
			),
			("function read = (): Integer => n\n invoke()", Ready::Ready),
		] {
			let source = format!(
				"function make = (n: Integer): Integer => {{\n function invoke = (): Integer => read()\n {body}\n}}\n"
			);
			assert_eq!(states(&source, "read")[0].borrowed, expected);
		}
	}

	#[test]
	fn readiness_flows_through_multiple_borrowed_wrappers() {
		let source = "function make = (n: Integer): Integer => {\n function first = (): Integer => second()\n function second = (): Integer => read()\n let before = first()\n function read = (): Integer => n\n before + first()\n}\n";
		assert_eq!(states(source, "read")[0].borrowed, Ready::Mixed);
	}

	#[test]
	fn borrowed_callback_uses_the_receiving_calls_position() {
		let source = "function invoke = (callback: () => Integer): Integer => callback()\n function make = (n: Integer): Integer => {\n function wrapper = (): Integer => read()\n let before = invoke(wrapper)\n function read = (): Integer => n\n before + invoke(wrapper)\n}\n";
		assert_eq!(states(source, "read")[0].borrowed, Ready::Mixed);
	}

	#[test]
	fn omitted_defaults_inherit_the_calls_readiness() {
		let source = "function make = (n: Integer): Integer => {\n function invoke = (value: Integer = read()): Integer => value\n let before = invoke()\n function read = (): Integer => n\n before + invoke()\n}\n";
		assert_eq!(states(source, "read")[0].borrowed, Ready::Mixed);
	}

	#[test]
	fn written_arguments_do_not_activate_a_default_context() {
		let source = "function make = (n: Integer): Integer => {\n function invoke = (value: Integer = read()): Integer => value\n let before = invoke(0)\n function read = (): Integer => n\n before + invoke(1)\n}\n";
		assert_eq!(states(source, "read")[0].borrowed, Ready::Pending);
	}

	#[test]
	fn mutually_calling_borrowed_wrappers_converge() {
		let source = "function make = (n: Integer): Integer => {\n function first = (again: Boolean): Integer => if again { second() } else { read() }\n function second = (): Integer => first(false)\n let before = first(false)\n function read = (): Integer => n\n before + second()\n}\n";
		assert_eq!(states(source, "read")[0].borrowed, Ready::Mixed);
	}
}
