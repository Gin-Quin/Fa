use fa_parser_v2::parse::parse_expression;

fn main() {
	let tree = parse_expression("a + 2 + 3 + 4");
	println!("{:#?}", &tree);
}
