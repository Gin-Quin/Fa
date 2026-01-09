use std::fs;

use crate::parsing::parse::parse;

#[cfg(test)]
fn parse_fixture(name: &str) {
	let path = format!("{}/src/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
	let source = fs::read_to_string(&path).expect("fixture should be readable");
	let source = Box::leak(source.into_boxed_str());
	let tree = parse(source);
	assert!(!tree.nodes.is_empty(), "fixture should parse into nodes");
}

#[test]
fn parse_short_fixture() {
	parse_fixture("short-file.fa");
}

#[test]
fn parse_medium_fixture() {
	parse_fixture("medium-file.fa");
}

#[test]
fn parse_long_fixture() {
	parse_fixture("long-file.fa");
}
