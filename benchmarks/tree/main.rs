//! A pointer-linked binary tree: build a complete tree of the given depth, sum it eight times.
//! From the Goose suite's `bench/rust/tree_box.rs` (the "rust Box nodes" row: one allocation
//! per node, like the recursive `Node?` children of the Fa version).

use std::env;

const DEFAULT_SIZE: i64 = 21;
const PASSES: i64 = 8;

struct Node {
	value: i32,
	left: Option<Box<Node>>,
	right: Option<Box<Node>>,
}

/// xorshift64, as `rng.goose`.
fn xs_next(x: u64) -> u64 {
	let mut v = x;
	v ^= v << 13;
	v ^= v >> 7;
	v ^= v << 17;
	v
}

/// Reduction into `[0, k)`, as C's `uint64_t` modulo; the result fits an `i64`.
fn xs_mod(x: u64, k: i64) -> i64 {
	(x % k as u64) as i64
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn build(depth: i64, seed: u64) -> Box<Node> {
	let mut node = Box::new(Node {
		value: xs_mod(seed, 1000) as i32,
		left: None,
		right: None,
	});
	if depth > 0 {
		node.left = Some(build(depth - 1, xs_next(seed)));
		node.right = Some(build(depth - 1, xs_next(seed + 1)));
	}
	node
}

/// The pass index is folded into every node's contribution, so the eight passes are eight
/// different computations that can't be hoisted out of the loop.
fn sum_tree(node: &Node, pass: i64) -> i64 {
	let mut sum = (node.value as i64) ^ pass;
	if let Some(left) = &node.left {
		sum += sum_tree(left, pass);
	}
	if let Some(right) = &node.right {
		sum += sum_tree(right, pass);
	}
	sum
}

fn main() {
	let depth = size_argument(DEFAULT_SIZE);
	let root = build(depth, 12345);
	let count = (1_i64 << (depth + 1)) - 1;
	let mut total: i64 = 0;
	for pass in 0..PASSES {
		total += sum_tree(&root, pass);
	}
	println!("{count}");
	println!("{total}");
}
