use fa_parser_v2::parse::*;

fn main() {
	let tree = parse_single_statement("toto(1) + 5");
	// let tree = parse_single_statement("a: toto = 12");
	println!("\n{:#?}\n", &tree);
	println!("\n[RESULT]\n{}", tree.to_string());
}
