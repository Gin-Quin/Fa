use fa_parser_v2::parse::*;

fn main() {
	let tree = parse_single_statement("a + b + c");
	println!("{:#?}", &tree);
}
