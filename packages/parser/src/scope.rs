pub struct Scope {
	pub symbols: Vec<SymbolState>,
}

pub enum SymbolState {
	Declaration { name: &'static str, node: usize },
	Narrowing { constraints: Vec<Constraint> },
}

// TODO: Implement constraints
pub enum Constraint {
	Is {},
	IsNot {},
	IsIn {},
	IsNotIn {},
	IsGreaterThan {},
	IsLessThan {},
}

impl Scope {
	pub fn new() -> Self {
		Scope {
			symbols: Vec::new(),
		}
	}
}
