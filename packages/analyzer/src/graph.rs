//! The module graph (design/parallel-pipeline.md §2.1): the files of a
//! program, an edge from each file to the files its scan says it may read
//! ([`crate::scan`]), and its strongly connected components, the units of
//! analysis: a cycle between files is analyzed as one unit.

/// The strongly connected components of a graph, each listed after the
/// components it reaches (dependencies first). `edges[i]` lists the nodes
/// node `i` reaches; the nodes are visited in index order, so the result
/// depends only on the graph. Each component lists its nodes in index order.
pub fn components(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
	const UNSEEN: usize = usize::MAX;
	let count = edges.len();
	let mut index = vec![UNSEEN; count];
	let mut low = vec![0; count];
	let mut on_stack = vec![false; count];
	let mut stack: Vec<usize> = Vec::new();
	let mut out: Vec<Vec<usize>> = Vec::new();
	let mut next = 0;
	// the explicit call stack: (node, position in its edges)
	let mut calls: Vec<(usize, usize)> = Vec::new();
	for root in 0..count {
		if index[root] != UNSEEN {
			continue;
		}
		calls.push((root, 0));
		index[root] = next;
		low[root] = next;
		next += 1;
		stack.push(root);
		on_stack[root] = true;
		while let Some(&mut (node, ref mut position)) = calls.last_mut() {
			if let Some(&target) = edges[node].get(*position) {
				*position += 1;
				if index[target] == UNSEEN {
					index[target] = next;
					low[target] = next;
					next += 1;
					stack.push(target);
					on_stack[target] = true;
					calls.push((target, 0));
				} else if on_stack[target] {
					low[node] = low[node].min(index[target]);
				}
				continue;
			}
			calls.pop();
			if let Some(&(parent, _)) = calls.last() {
				low[parent] = low[parent].min(low[node]);
			}
			if low[node] == index[node] {
				let mut component = Vec::new();
				while let Some(member) = stack.pop() {
					on_stack[member] = false;
					component.push(member);
					if member == node {
						break;
					}
				}
				component.sort_unstable();
				out.push(component);
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::components;

	#[test]
	fn components_come_after_their_dependencies() {
		// 0 → 1 → 2 → 1, 3 → 0, 4 alone
		let edges = vec![vec![1], vec![2], vec![1], vec![0], vec![]];
		assert_eq!(
			components(&edges),
			vec![vec![1, 2], vec![0], vec![3], vec![4]]
		);
	}

	#[test]
	fn a_long_chain_does_not_recurse() {
		let count = 200_000;
		let edges: Vec<Vec<usize>> = (0..count)
			.map(|node| {
				if node + 1 < count {
					vec![node + 1]
				} else {
					vec![]
				}
			})
			.collect();
		let found = components(&edges);
		assert_eq!(found.len(), count);
		assert_eq!(found[0], vec![count - 1]);
	}
}
