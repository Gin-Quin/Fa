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
pub enum DeclarationKind {
	Let,
	Mutable,
	Use,
}

impl DeclarationKind {
	pub fn keyword(&self) -> &'static str {
		match self {
			DeclarationKind::Let => "let",
			DeclarationKind::Mutable => "mutable",
			DeclarationKind::Use => "use",
		}
	}
}

/// A field of an object pattern, ex: `a`, `a as b`, `a = 4`, `nested { c }`
#[derive(Debug, Clone)]
pub struct PatternField {
	pub name: &'static str,
	pub alias: Option<&'static str>,
	pub nested: Option<usize>,
	pub default: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOperator {
	Add,
	Subtract,
	Multiply,
	Power,
	Divide,
	IntegerDivide,
}

impl CompoundOperator {
	pub fn symbol(&self) -> &'static str {
		match self {
			CompoundOperator::Add => "+=",
			CompoundOperator::Subtract => "-=",
			CompoundOperator::Multiply => "*=",
			CompoundOperator::Power => "**=",
			CompoundOperator::Divide => "/=",
			CompoundOperator::IntegerDivide => "//=",
		}
	}
}

#[derive(Debug, Clone)]
pub struct WhenBranch {
	pub pattern: WhenBranchPattern,
	// the value bound with `as`, ex: `Shape.Circle as { radius }`
	pub binding: Option<usize>,
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
	None,
	Error,
	Boolean(bool),

	/* ------------------------------- Composed ------------------------------- */
	List {
		// ex: `[a, b, c]`
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
	Try {
		// ex: `try loadScore()`
		right: usize,
	},
	Throw {
		// ex: `throw ApiError.Timeout(duration = 30)`
		right: usize,
	},
	Catch {
		// ex: `loadScore() catch error => 0`
		expression: usize,
		error_name: &'static str,
		handler: FunctionBody,
	},
	Await {
		right: usize,
	},
	Start {
		right: usize,
	},
	Assert {
		right: usize,
	},
	Provide {
		// ex: `provide Database = PostgresDatabase(url)`
		type_expression: usize,
		expression: usize,
	},
	Break {
		expression: Option<usize>,
	},
	Continue,
	For {
		// ex: `for item, index in items { ... }`, `for parallel { x } in points { ... }`
		pattern: Option<usize>,
		index: Option<&'static str>,
		iterable: usize,
		// `Some(arguments)` for a `parallel` loop, ex: `parallel(threads = 2)`
		parallel: Option<Vec<usize>>,
		body: Vec<usize>,
		is_compile_time: bool,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	While {
		expression: usize,
		body: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	WhileLet {
		// ex: `while let value = stack.pop() { ... }`
		pattern: usize,
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
		// ex: `a is b`, `a is not b`
		left: usize,
		right: usize,
		negated: bool,
	},
	IsIn {
		// ex: `a is in b`, `a is not in b`
		left: usize,
		right: usize,
		negated: bool,
	},
	In {
		// owned reference type, ex: `Person in persons`
		left: usize,
		right: usize,
	},
	Range {
		// ex: `a..b`, `a..=b`, `a..`, `..b`, `..`
		start: Option<usize>,
		end: Option<usize>,
		inclusive: bool,
	},
	Union {
		operands: Vec<usize>,
	},
	Compose {
		// ex: `a then b then c`
		operands: Vec<usize>,
	},
	PartialApplication {
		// ex: `xyz with (y = 0)`
		function: usize,
		arguments: Vec<usize>,
	},
	Percentage {
		value: usize,
	},
	Optional {
		value: usize,
	},
	Index {
		target: usize,
		index: usize,
	},
	MemberIndex {
		target: usize,
		members: Vec<usize>,
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

	/* -------------------------------- Patterns -------------------------------- */
	ObjectPattern {
		// ex: `{ a, b as c, d = 4, nested { e } }`
		fields: Vec<PatternField>,
	},
	ArrayPattern {
		// ex: `[first, second]`
		items: Vec<usize>,
	},
	TuplePattern {
		// ex: `(number, name)`
		items: Vec<usize>,
	},
	Destructuring {
		// ex: `let { a, b } = object`
		kind: DeclarationKind,
		pattern: usize,
		expression: usize,
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
		// ex: `async function name = (a: Integer): Integer => a + 1`
		// the expression is any function: a lambda, an alias, a composition, ...
		name: &'static str,
		is_async: bool,
		expression: usize,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Parameters {
		// the parameters of a function, ex: `(a: Integer, mutable self, { x, y }: Point)`
		items: Vec<usize>,
	},
	DestructuredParameter {
		// ex: `{ x, y }: Point`
		pattern: usize,
		type_expression: usize,
	},
	ArrowFunction {
		// ex: `(a: Integer): Integer throws E requires r: R => a + 1`
		parameters: Vec<usize>,
		return_type_expression: Option<usize>,
		errors: Option<usize>,
		requirements: Vec<usize>,
		body: FunctionBody,
	},
	Throws {
		// ex: `User throws NotFound` (in a function type, or in a signature before `=>`)
		signature: usize,
		errors: usize,
	},
	Requires {
		// ex: `User requires database: Database` (in a function type, or in a signature before `=>`)
		signature: usize,
		requirements: Vec<usize>,
	},
	Readonly {
		// ex: `readonly balance: Integer = 0` (in a type)
		right: usize,
	},
	CompoundAssignment {
		// ex: `a += 1`
		operator: CompoundOperator,
		target: usize,
		expression: usize,
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
