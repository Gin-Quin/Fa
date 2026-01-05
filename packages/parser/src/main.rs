use fa_parser::parsing::parse::*;

fn main() {
	let tree = parse("if x is 12 { 13 } else { 14 }");
	// let tree = parse_single_statement("a: toto = 12");
	println!("\n{:#?}\n", &tree);
	println!("\n[RESULT]\n{}", tree.to_string());
}
