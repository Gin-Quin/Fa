use fa_parser::parsing::parse::*;

fn main() {
	let tree = parse("toto >> { let x = 12, use z = 432 }\n x + 2");
	// let tree = parse_single_statement("a: toto = 12");
	println!("\n{:#?}\n", &tree);
	println!("\n[RESULT]\n{}", tree.to_string());
}
