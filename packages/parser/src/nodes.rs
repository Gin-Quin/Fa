use crate::{tokens::Token, types::Type};

#[derive(Debug, Clone)]
pub struct HoistedSymbol {
	pub name: &'static str,
	pub reference: usize,
}

#[derive(Debug, Clone)]
pub enum FunctionBody {
	Block {
		statements: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Expression(usize),
}

#[derive(Debug, Clone)]
pub enum IfElseBody {
	If(usize),
	Block(Vec<usize>),
}

#[derive(Debug, Clone)]
pub enum WhenBranchValue {
	Block {
		statements: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractSymbolKind {
	Alias,
	Copy,
}

#[derive(Debug, Clone)]
pub struct ExtractSymbol {
	pub name: &'static str,
	pub kind: ExtractSymbolKind,
	pub default_expression: Option<usize>,
	pub resolved_type: Option<Type>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionKind {
	Index,
	Member,
	TupleMember,
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
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	DanglingToken {
		token: Token,
	},

	/* ------------------------------- Primitives ------------------------------- */
	Identifier {
		name: &'static str,
	},
	Integer(i64),
	Number(f64),
	BigInteger(&'static str),
	StringLiteral(String),
	StringTemplate {
		parts: Vec<StringPart>,
	},
	Null,
	Error,
	Boolean(bool),

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
	ExtractCopy {
		name: &'static str,
		expression: Option<usize>,
	},
	ExtractAlias {
		name: &'static str,
		expression: Option<usize>,
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
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	While {
		expression: usize,
		body: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Loop {
		body: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Do {
		body: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	If {
		condition: usize,
		then_body: Vec<usize>,
		else_body: Option<IfElseBody>,
		hoisted_then_symbols: Vec<HoistedSymbol>,
		hoisted_else_symbols: Vec<HoistedSymbol>,
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
	Index {
		target: usize,
		index: usize,
	},
	MemberIndex {
		target: usize,
		members: Vec<usize>,
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
		symbols: Vec<ExtractSymbol>,
		extraction_kind: ExtractionKind,
		default_kind: Option<ExtractSymbolKind>,
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
		name: &'static str,
		type_expression: Option<usize>,
		expression: Option<usize>,
		resolved_type: Option<Type>,
	},
	Mutable {
		name: &'static str,
		type_expression: Option<usize>,
		expression: Option<usize>,
		resolved_type: Option<Type>,
	},
	Use {
		name: &'static str,
		type_expression: Option<usize>,
		expression: usize,
		resolved_type: Option<Type>,
	},
	Static {
		right: usize,
	},
	Type {
		name: &'static str,
		expression: usize,
		resolved_type: Option<Type>,
	},
	UnionDeclaration {
		name: &'static str,
		expression: usize,
	},
	ErrorsDeclaration {
		name: &'static str,
		expression: usize,
	},
	Enum {
		name: &'static str,
		expression: usize,
	},
	Fields {
		name: &'static str,
		expression: usize,
	},
	Reactive {
		name: &'static str,
		type_expression: Option<usize>,
		expression: Option<usize>,
		resolved_type: Option<Type>,
	},
	Derived {
		name: &'static str,
		type_expression: Option<usize>,
		expression: Option<usize>,
		resolved_type: Option<Type>,
	},
	Namespace {
		name: &'static str,
		expression: usize,
	},
	Function {
		name: &'static str,
		parameters: Option<usize>,
		return_type_expression: Option<usize>,
		body: FunctionBody,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Method {
		name: &'static str,
		parameters: Option<usize>,
		return_type_expression: Option<usize>,
		body: FunctionBody,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	ArrowFunction {
		parameters: Option<usize>,
		parenthesized_parameters: bool,
		return_type_expression: Option<usize>,
		body: FunctionBody,
	},
	Assignment {
		// an equal statement
		name: usize,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	ExportValue {
		type_expression: Option<usize>,
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportFunction {
		type_expression: Option<usize>,
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportType {
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportNamespace {
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportUnion {
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportErrors {
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportEnum {
		expression: usize,
		resolved_type: Option<Type>,
	},
	ExportFields {
		expression: usize,
		resolved_type: Option<Type>,
	},

	/* -------------------------------- Functions ------------------------------- */
	FunctionCall {
		function: usize,
		parameters: Vec<usize>,
	},
}
