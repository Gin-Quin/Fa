//! The order of evaluation of the arguments of a call (values brief §4.2,
//! types brief §4.2.4): `Analysis::call` gives the arguments in parameter
//! order (named arguments moved to their parameter, defaults filled in),
//! but they are evaluated in the order they are written. A backend builds
//! the values in that order, then passes them in parameter order (with
//! temporaries when the two orders differ and a value has effects).

use fa_parser::TypedSyntaxTree;

use crate::analysis::Argument;
use crate::ids::NodeId;
use crate::syntax::NodeList;

/// The value nodes of a call's arguments (`Argument::Given`, and every
/// value of an `Argument::Variadic`), each with the index of its parameter,
/// in parameter order.
pub fn argument_values(arguments: &[Argument]) -> Vec<(usize, NodeId)> {
	let mut values = Vec::new();
	for (parameter, argument) in arguments.iter().enumerate() {
		match argument {
			Argument::Given(value) => values.push((parameter, *value)),
			Argument::Variadic(items) => values.extend(items.iter().map(|&item| (parameter, item))),
			Argument::Default | Argument::Missing | Argument::Receiver(_) => {}
		}
	}
	values
}

/// The position of each value node among the written arguments of the call
/// (`written`: the argument nodes as written, a named argument `name =
/// value` holding its value): the rank in the order of evaluation.
/// `usize::MAX` for a node that isn't written there.
pub fn written_positions(
	tree: &TypedSyntaxTree,
	written: NodeList<'_>,
	values: &[NodeId],
) -> Vec<usize> {
	let spans: Vec<(usize, usize)> = written
		.iter()
		.map(|argument| {
			let span = tree.spans[argument.index()];
			(span.start, span.end.max(span.start + 1))
		})
		.collect();
	values
		.iter()
		.map(|value| {
			let start = tree.spans[value.index()].start;
			spans
				.iter()
				.position(|&(from, to)| from <= start && start < to)
				.unwrap_or(usize::MAX)
		})
		.collect()
}

/// Whole parameters in written evaluation order. Positional and variadic
/// arguments precede named arguments, so a variadic group is contiguous.
/// A method's receiver is evaluated first; omitted arguments have no work
/// here and their defaults are evaluated later in parameter order.
pub fn parameter_order(
	tree: &TypedSyntaxTree,
	written: NodeList<'_>,
	arguments: &[Argument],
) -> Vec<usize> {
	let values = argument_values(arguments);
	let nodes: Vec<_> = values.iter().map(|&(_, node)| node).collect();
	let positions = written_positions(tree, written, &nodes);
	let mut ranks = vec![usize::MAX; arguments.len()];
	for ((parameter, _), rank) in values.into_iter().zip(positions) {
		ranks[parameter] = ranks[parameter].min(rank.saturating_add(1));
	}
	for (parameter, argument) in arguments.iter().enumerate() {
		if matches!(argument, Argument::Receiver(_)) {
			ranks[parameter] = 0;
		}
	}
	let mut order: Vec<_> = (0..arguments.len()).collect();
	order.sort_by_key(|&parameter| ranks[parameter]);
	if order.iter().copied().eq(0..arguments.len()) {
		Vec::new()
	} else {
		order
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::analysis::Callee;
	use crate::syntax::{self, View};

	#[test]
	fn named_arguments_are_evaluated_in_the_written_order() {
		let source =
			"function move = (x: Integer, y: Integer) => x - y\nconsole.log(move(y = 1, x = 2))\n";
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let (call, written) = (0..tree.nodes.len())
			.map(NodeId::new)
			.find_map(|n| match (syntax::view(tree, n), analysis.call(n)) {
				(View::Call { arguments, .. }, Some(info))
					if matches!(info.callee, Callee::Function(_)) =>
				{
					Some((info.clone(), arguments))
				}
				_ => None,
			})
			.expect("the call of `move`");
		let values = argument_values(&call.arguments);
		assert_eq!(values.iter().map(|&(p, _)| p).collect::<Vec<_>>(), [0, 1]);
		let nodes: Vec<NodeId> = values.iter().map(|&(_, node)| node).collect();
		// `x = 2` is written second, `y = 1` first
		assert_eq!(written_positions(tree, written, &nodes), [1, 0]);
		assert_eq!(parameter_order(tree, written, &call.arguments), [1, 0]);
	}
}
