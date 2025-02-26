#[derive(Debug)]
pub enum Node<'input> {
	Identifier(&'input str),
	Integer(i32),
	// Float(f64),
	Boolean(bool),
	// String(&str),
	// Array(Vec<Node>),
	// Tuple(Vec<Node>),

	/* ------------------------------- Operations ------------------------------- */
	Add {
		left: usize,
		right: usize,
	},
	// Subtract {
	// 	left: &Node,
	// 	right: &Node,
	// },
	Multiply {
		left: usize,
		right: usize,
	},
	// Divide {
	// 	left: &Node,
	// 	right: &Node,
	// },
	// Modulo,
	// Power,
}
