#[derive(Debug, Clone)]
pub enum Node {
	Identifier(&'static str),
	Integer(i32),
	// Float(f64),
	Boolean(bool),
	// String(&str),
	// Array(Vec<Node>),
	// Tuple(Vec<Node>),

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
	},
	Not {
		right: usize,
	},
	Add {
		left: usize,
		right: usize,
	},
	Subtract {
		left: usize,
		right: usize,
	},
	Multiply {
		left: usize,
		right: usize,
	},
	Divide {
		left: usize,
		right: usize,
	},
	IntegerDivide {
		left: usize,
		right: usize,
	},
	Modulo {
		left: usize,
		right: usize,
	},
	Power {
		left: usize,
		right: usize,
	},
	Equal {
		left: usize,
		right: usize,
	},
	NotEqual {
		left: usize,
		right: usize,
	},
	LessThan {
		left: usize,
		right: usize,
	},
	LessThanOrEqual {
		left: usize,
		right: usize,
	},
	GreaterThan {
		left: usize,
		right: usize,
	},
	GreaterThanOrEqual {
		left: usize,
		right: usize,
	},
	And {
		left: usize,
		right: usize,
	},
	Or {
		left: usize,
		right: usize,
	},
	Is {
		left: usize,
		right: usize,
	},
	FatArrow {
		left: usize,
		right: usize,
	},
	Union {
		left: usize,
		right: usize,
	},
	Pipe {
		left: usize,
		right: usize,
	},
	Insert {
		left: usize,
		right: usize,
	},
	Extract {
		left: usize,
		right: usize,
	},

	/* --------------------------------- Groups --------------------------------- */
	Group {
		expression: usize,
	},
}
