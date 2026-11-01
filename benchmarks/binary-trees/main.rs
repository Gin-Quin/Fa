//! Binary trees: allocation and deallocation of many small recursive values.

use std::env;

const DEFAULT_SIZE: i64 = 17;
const MIN_DEPTH: i64 = 4;

struct Tree {
	left: Option<Box<Tree>>,
	right: Option<Box<Tree>>,
}

impl Tree {
	fn bottom_up(depth: i64) -> Tree {
		if depth > 0 {
			Tree {
				left: Some(Box::new(Tree::bottom_up(depth - 1))),
				right: Some(Box::new(Tree::bottom_up(depth - 1))),
			}
		} else {
			Tree {
				left: None,
				right: None,
			}
		}
	}

	fn item_count(&self) -> i64 {
		1 + self.left.as_ref().map_or(0, |left| left.item_count())
			+ self.right.as_ref().map_or(0, |right| right.item_count())
	}
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let max_depth = size_argument(DEFAULT_SIZE).max(MIN_DEPTH + 2);

	let stretch_depth = max_depth + 1;
	let stretch_tree = Tree::bottom_up(stretch_depth);
	println!(
		"stretch tree of depth {stretch_depth}\t check: {}",
		stretch_tree.item_count()
	);
	drop(stretch_tree);

	let long_lived_tree = Tree::bottom_up(max_depth);

	for step in 0..=(max_depth - MIN_DEPTH) / 2 {
		let depth = MIN_DEPTH + 2 * step;
		let iterations =
			2_i64.pow(u32::try_from(max_depth - depth + MIN_DEPTH).expect("a small exponent"));
		let mut check = 0;
		for _ in 0..iterations {
			check += Tree::bottom_up(depth).item_count();
		}
		println!("{iterations}\t trees of depth {depth}\t check: {check}");
	}

	println!(
		"long lived tree of depth {max_depth}\t check: {}",
		long_lived_tree.item_count()
	);
}
