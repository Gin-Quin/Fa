//! Print the syntax tree of the standard input, with the kind and span of every node, to see
//! how a construct is parsed: `cargo run -p fa-formatter --example dump < file.fa`.

use std::io::Read;

use fa_parser::{TypedSyntaxTree, parse};

fn main() {
	let mut source = String::new();
	std::io::stdin()
		.read_to_string(&mut source)
		.expect("the standard input should be readable");
	let tree = parse(&source);
	for diagnostic in &tree.diagnostics {
		eprintln!("{}", diagnostic.display(&tree.source_map));
	}
	println!("{tree}");
	dump(&tree, tree.root, 0);
	for comment in &tree.comments {
		println!(
			"comment {}..{} {:?}",
			comment.start,
			comment.end,
			&tree.source[comment.start..comment.end]
		);
	}
}

fn dump(tree: &TypedSyntaxTree, index: usize, depth: usize) {
	let span = tree.span(index);
	let node = tree.at(index);
	let text: String = tree.text(index).chars().take(40).collect();
	println!(
		"{}{} {}..{} {text:?}",
		"  ".repeat(depth),
		node.kind_name(),
		span.start,
		span.end
	);
	for child in node.children() {
		dump(tree, child, depth + 1);
	}
}
