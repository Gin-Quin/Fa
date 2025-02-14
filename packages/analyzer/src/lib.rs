use fa_parser::*;

pub mod tst;

use tst::*;

#[test]
fn test() {
	println!("test");
	let program = parser::ProgramParser
		::new()
		.parse("x = 12, 12+14, y = () => { 21 }")
		.unwrap();
	let tst = TypeSymbolTree::new(&program);
	println!("{:?}", tst);
	assert_eq!(program.len(), 3);
	assert_eq!(tst.body.len(), 2);
}
