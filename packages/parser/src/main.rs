use fa_parser::parse::*;

fn main() {
	let tree = parse("...(12, 4, \"hi\")");
	// let tree = parse_single_statement("a: toto = 12");
	println!("\n{:#?}\n", &tree);
	println!("\n[RESULT]\n{}", tree.to_string());
}
