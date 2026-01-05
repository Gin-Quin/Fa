use crate::{tokens::Token, types::Type};

#[derive(Debug, Clone)]
pub enum ArrowFunctionBody {
	Block(Vec<usize>),
	Expression(usize),
}

#[derive(Debug, Clone)]
pub enum IfElseBody {
	If(usize),
	Block(Vec<usize>),
}

#[derive(Debug, Clone)]
pub enum WhenBranchValue {
	Block(Vec<usize>),
	Expression(usize),
}

#[derive(Debug, Clone)]
pub enum WhenBranchPattern {
	Expression(usize),
	Else,
}

#[derive(Debug, Clone)]
pub enum StringPart {
	Literal(String),
	Expression(usize),
}

#[derive(Debug, Clone)]
pub struct WhenBranch {
	pub pattern: WhenBranchPattern,
	pub value: WhenBranchValue,
}

#[derive(Debug, Clone)]
pub enum Node {
	Module {
		statements: Vec<usize>,
	},
	DanglingToken {
		token: Token,
	},

	/* ------------------------------- Primitives ------------------------------- */
	Identifier(&'static str),
	Integer(i64),
	Number(f64),
	BigInteger(&'static str),
	StringLiteral(String),
	StringTemplate {
		parts: Vec<StringPart>,
	},
	Null,
	Boolean(bool),
	True,
	False,

	/* ------------------------------- Composed ------------------------------- */
	List {
		// ex: `[a, b, c]`
		items: Vec<usize>,
	},
	StaticList {
		// ex: `@[1, 2, 3]`
		items: Vec<usize>,
	},
	Tuple {
		// ex: `a, b, c`
		items: Vec<usize>,
	},
	Members {
		// an "object literal", i.e. member declarations inside brackets
		// ex: `{ a = 12, b: String = "Hello", #x, foo }`
		items: Vec<usize>,
	},
	StaticMembers {
		// ex: `@{ a = 12 }`
		items: Vec<usize>,
	},

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
	},
	Spread {
		right: usize,
	},
	Not {
		right: usize,
	},
	Return {
		expression: Option<usize>,
	},
	Break {
		expression: Option<usize>,
	},
	Continue,
	For {
		expression: usize,
		body: Vec<usize>,
		is_compile_time: bool,
	},
	While {
		expression: usize,
		body: Vec<usize>,
	},
	Loop {
		body: Vec<usize>,
	},
	If {
		condition: usize,
		then_body: Vec<usize>,
		else_body: Option<IfElseBody>,
	},
	When {
		expression: usize,
		branches: Vec<WhenBranch>,
	},
	Add {
		operands: Vec<usize>,
	},
	Subtract {
		operands: Vec<usize>,
	},
	Multiply {
		operands: Vec<usize>,
	},
	Divide {
		operands: Vec<usize>,
	},
	IntegerDivide {
		operands: Vec<usize>,
	},
	Modulo {
		operands: Vec<usize>,
	},
	Power {
		operands: Vec<usize>,
	},
	Equal {
		operands: Vec<usize>,
	},
	NotEqual {
		operands: Vec<usize>,
	},
	LessThan {
		operands: Vec<usize>,
	},
	LessThanOrEqual {
		operands: Vec<usize>,
	},
	GreaterThan {
		operands: Vec<usize>,
	},
	GreaterThanOrEqual {
		operands: Vec<usize>,
	},
	And {
		operands: Vec<usize>,
	},
	Or {
		operands: Vec<usize>,
	},
	Is {
		left: usize,
		right: usize,
	},
	Union {
		operands: Vec<usize>,
	},
	Intersection {
		operands: Vec<usize>,
	},
	Pipe {
		operands: Vec<usize>,
	},
	Compose {
		operands: Vec<usize>,
	},
	Percentage {
		value: usize,
	},
	Optional {
		value: usize,
	},
	Assert {
		value: usize,
	},
	OptionalFunctionCall {
		function: usize,
		parameters: Vec<usize>,
	},
	OptionalIndex {
		target: usize,
		index: usize,
	},
	Insert {
		left: usize,
		right: usize,
	},
	Extract {
		left: usize,
		right: usize,
	},
	Relation {
		left: usize,
		right: usize,
	},
	Access {
		operands: Vec<usize>,
	},
	OptionalAccess {
		operands: Vec<usize>,
	},

	/* --------------------------------- Groups --------------------------------- */
	EmptyGroup,
	Group {
		expression: usize,
	},

	/* ------------------------------ Declarations ------------------------------ */
	Let {
		right: usize,
		resolved_type: Option<Type>,
	},
	Mutable {
		right: usize,
		resolved_type: Option<Type>,
	},
	Static {
		right: usize,
	},
	Type {
		right: usize,
		resolved_type: Option<Type>,
	},
	UnionDeclaration {
		right: usize,
	},
	Enum {
		right: usize,
	},
	Fields {
		right: usize,
	},
	Reactive {
		right: usize,
		resolved_type: Option<Type>,
	},
	Derived {
		right: usize,
		resolved_type: Option<Type>,
	},
	Namespace {
		right: usize,
	},
	Function {
		name: &'static str,
		value: usize,
	},
	ArrowFunction {
		parameters: Option<usize>,
		parenthesized_parameters: bool,
		return_type_expression: Option<usize>,
		body: ArrowFunctionBody,
	},
	Assignment {
		// an equal statement
		name: usize,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},

	/* -------------------------------- Functions ------------------------------- */
	FunctionCall {
		function: usize,
		parameters: Vec<usize>,
	},
}
