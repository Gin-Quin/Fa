use std::fs;

use crate::parsing::parse::parse;

#[cfg(test)]
fn parse_fixture(name: &str) {
	let path = format!("{}/src/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
	let source = fs::read_to_string(&path).expect("fixture should be readable");
	let tree = parse(&source);
	assert!(!tree.nodes.is_empty(), "fixture should parse into nodes");
}

#[test]
fn parse_long_fixture() {
	parse_fixture("long-file.fa");
}
