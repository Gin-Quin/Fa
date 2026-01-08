pub struct Scope {
	pub symbols: Vec<SymbolState>,
}

pub enum SymbolState {
	Declaration { node: usize },
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
