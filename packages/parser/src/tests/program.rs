use crate::fa;
use crate::ast;
use ast::Expression::*;

#[test]
fn empty_program() {
	let program = fa::ProgramParser::new().parse("").unwrap();

	assert_eq!(program.len(), 0);
}

#[test]
fn empty_lines_program() {
	let program = fa::ProgramParser::new().parse("\n\n").unwrap();

	assert_eq!(program.len(), 0);
}

#[test]
fn empty_lines_between_statements() {
	let program = fa::ProgramParser::new().parse("\n\n1\n\n\n\n\n\n2\n\n3\n\n").unwrap();

	assert_eq!(program.len(), 3);
}

#[test]
fn commas_as_line_breaks() {
	let program = fa::ProgramParser::new().parse("1,2,3").unwrap();

	assert_eq!(program.len(), 3);
}

#[test]
fn commas_plus_line_breaks() {
	let program = fa::ProgramParser::new().parse("1,\n\n2,3,\n\n\n\n").unwrap();

	assert_eq!(program.len(), 3);
}
