use crate::source::SourceSpan;

/// A name written in the source, like a declared name or a field name, stored as its position
/// in the source: `TypedSyntaxTree::name` gives its text. It is `Copy` and 8 bytes, and the tree
/// doesn't borrow the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Name {
	pub start: u32,
	pub end: u32,
}

impl Name {
	pub fn new(start: usize, end: usize) -> Self {
		Name {
			start: start as u32,
			end: end as u32,
		}
	}

	pub fn span(self) -> SourceSpan {
		SourceSpan::new(self.start as usize, self.end as usize)
	}

	/// True for a name that is missing from the source (after an error).
	pub fn is_empty(self) -> bool {
		self.start == self.end
	}
}

/// A declaration hoisted to the top of its block (only functions, for now), so that it can be
/// used before the line that declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoistedSymbol {
	pub name: Name,
	pub reference: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhenBranchPattern {
	Expression(usize),
	Else,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
	Literal(String),
	Expression(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
	// `mutates path`: the function modifies the content at this path
	Mutates,
	// `invalidates path`: the function can move or remove elements, invalidating references
	Invalidates,
}

impl EffectKind {
	pub fn keyword(&self) -> &'static str {
		match self {
			EffectKind::Mutates => "mutates",
			EffectKind::Invalidates => "invalidates",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestModifier {
	Skip,
	Only,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationKind {
	Let,
	Mutable,
	Use,
	MutableUse,
}

impl DeclarationKind {
	pub fn keyword(&self) -> &'static str {
		match self {
			DeclarationKind::Let => "let",
			DeclarationKind::Mutable => "mutable",
			DeclarationKind::Use => "use",
			DeclarationKind::MutableUse => "mutable use",
		}
	}
}

/// A field of an object pattern, ex: `a`, `a as b`, `a = 4`, `nested { c }`
#[derive(Debug, Clone, PartialEq)]
pub struct PatternField {
	pub name: Name,
	pub alias: Option<Name>,
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
	Remainder,
	// `.=`: assigns each field of an object literal, ex: `settings .= { port = 8080 }`
	Update,
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
			CompoundOperator::Remainder => "%%=",
			CompoundOperator::Update => ".=",
		}
	}
}

/// An operator of a (chainable) comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
}

impl ComparisonOperator {
	pub fn symbol(&self) -> &'static str {
		match self {
			ComparisonOperator::LessThan => "<",
			ComparisonOperator::LessThanOrEqual => "<=",
			ComparisonOperator::GreaterThan => ">",
			ComparisonOperator::GreaterThanOrEqual => ">=",
		}
	}
}

/// The value of an integer literal. It is stored as a magnitude and a sign so that every literal
/// of every integer type fits, from `-2^127` (`Int128`) to `2^128 - 1` (`UInt128`): whether it
/// fits its expected type is decided by the checker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerLiteral {
	pub magnitude: u128,
	pub is_negative: bool,
}

impl IntegerLiteral {
	pub fn new(value: i128) -> Self {
		IntegerLiteral {
			magnitude: value.unsigned_abs(),
			is_negative: value < 0,
		}
	}

	/// The value as an `i128`, or `None` if it doesn't fit.
	pub fn to_i128(self) -> Option<i128> {
		if self.is_negative {
			0i128.checked_sub_unsigned(self.magnitude)
		} else {
			i128::try_from(self.magnitude).ok()
		}
	}

	/// The value as an `i64` (an `Integer`), or `None` if it doesn't fit.
	pub fn to_i64(self) -> Option<i64> {
		self.to_i128().and_then(|value| i64::try_from(value).ok())
	}
}

impl std::fmt::Display for IntegerLiteral {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.is_negative && self.magnitude != 0 {
			write!(formatter, "-{}", self.magnitude)
		} else {
			write!(formatter, "{}", self.magnitude)
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenBranch {
	pub pattern: WhenBranchPattern,
	// the value bound with `as`, ex: `Shape.Circle as { radius }`
	pub binding: Option<usize>,
	// a `Block` node, or an expression
	pub value: usize,
}

/// The signature of a lambda or of a function type:
/// `async <Item>(a: Integer): Integer throws E requires r: R mutates a invalidates b`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FunctionSignature {
	pub is_async: bool,
	// the generic parameters, ex: `<Item>` in `<Item>(items: Array<Item>) => items.get(0)`
	pub generic_parameters: Vec<usize>,
	pub parameters: Vec<usize>,
	pub return_type_expression: Option<usize>,
	pub errors: Option<usize>,
	pub requirements: Vec<usize>,
	pub mutations: Vec<usize>,
	pub invalidations: Vec<usize>,
}

/// A node of the syntax tree. Children are referenced by their index in `TypedSyntaxTree::nodes`
/// (see `Node::children`). A block (`{ ... }` after `if`, `for`, `=>`...) is a `Block` node.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
	Module {
		statements: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	Block {
		// the body of `if`, `for`, `while`, `loop`, `do`, `watch`, `test`, a function, a `when`
		// branch or a `catch` handler
		statements: Vec<usize>,
		hoisted_symbols: Vec<HoistedSymbol>,
	},
	// code that could not be parsed: a diagnostic always explains why
	Invalid,
	// `...` written alone, to leave out code in an example, ex: `type Config = { ... }`
	Placeholder,

	/* ------------------------------- Primitives ------------------------------- */
	Identifier {
		name: Name,
	},
	ImplicitMember {
		// a member of the expected type, written without the type, ex: `.red` in
		// `let favoriteColor: Color = .red`
		name: Name,
	},
	Integer(IntegerLiteral),
	Number(f64),
	// the literal as written, ex: `-12_000n`
	BigInteger(Name),
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
		// ex: `try loadScore()`, `try scores[index] = 0`
		right: usize,
	},
	Throw {
		// ex: `throw ApiError.Timeout(duration = 30)`
		right: usize,
	},
	Catch {
		// ex: `loadScore() catch error => 0`
		expression: usize,
		error_name: Name,
		// a `Block` node, or an expression
		handler: usize,
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
		index: Option<Name>,
		iterable: usize,
		// `Some(arguments)` for a `parallel` loop, ex: `parallel(threads = 2)`
		parallel: Option<Box<[usize]>>,
		// a `Block` node
		body: usize,
		is_compile_time: bool,
	},
	While {
		expression: usize,
		body: usize,
	},
	WhileLet {
		// ex: `while let value = stack.pop() { ... }`
		pattern: usize,
		expression: usize,
		body: usize,
	},
	Loop {
		body: usize,
	},
	Do {
		body: usize,
	},
	Watch {
		// reactive block, ex: `watch { console.log(count) }`
		body: usize,
	},
	If {
		// `@if` / `@else` when `is_compile_time` is true
		is_compile_time: bool,
		condition: usize,
		// a `Block` node
		then_body: usize,
		// a `Block` node, or an `If` node for `else if`
		else_body: Option<usize>,
	},
	When {
		expression: usize,
		branches: Vec<WhenBranch>,
	},
	// `+`, `-`, `*`, `/`, `//`, `%%`, `??`, `and`, `or`, `|` and `then` are associative or
	// left-associative: a chain of the same operator is stored flat, ex: `a + b + c` is
	// `Add { operands: [a, b, c] }`
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
	Remainder {
		// ex: `a %% b`
		operands: Vec<usize>,
	},
	Fallback {
		// ex: `user?.name ?? "anonymous"` (optionals only)
		operands: Vec<usize>,
	},
	Power {
		// always two operands: `**` is right-associative, `a ** b ** c` is `a ** (b ** c)`
		operands: Vec<usize>,
	},
	Equal {
		// always two operands
		operands: Vec<usize>,
	},
	NotEqual {
		// always two operands
		operands: Vec<usize>,
	},
	Comparison {
		// a comparison, or a chain of comparisons, ex: `0 <= index < length` has the operands
		// `[0, index, length]` and the operators `[<=, <]` (`operands.len() == operators.len() + 1`)
		operands: Vec<usize>,
		operators: Vec<ComparisonOperator>,
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
	InRelation {
		// many-to-many relation, ex: `in Self->bestFriends` in `Bag<in Self->bestFriends>`
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
		// ex: `items[0]`, `text[1..]`, `players[..]` (every element, in effect paths)
		target: usize,
		index: usize,
	},
	MemberIndex {
		// an object literal of a named type, ex: `Point { x = 1, y = 2 }`, or a nested key set,
		// ex: `nested { bar }` in `{ foo, nested { bar } }`
		target: usize,
		members: Vec<usize>,
	},
	Relation {
		left: usize,
		right: usize,
	},
	Access {
		// ex: `a.b.c`: the first operand is any expression, the next ones are field names
		// (`Identifier` nodes, or `Invalid` for a missing name, like in `user.`). A call or an
		// index applies to the whole access before it: `a.b(c).d` is
		// `Access [FunctionCall { Access [a, b], [c] }, d]`
		operands: Vec<usize>,
	},
	OptionalAccess {
		// ex: `a?.b?.c`, with the same shape as `Access`
		operands: Vec<usize>,
	},

	/* -------------------------------- Patterns -------------------------------- */
	ObjectPattern {
		// ex: `{ a, b as c, d = 4, nested { e }, ...rest }`
		fields: Vec<PatternField>,
		// the `Identifier` of `...rest`, always last
		rest: Option<usize>,
	},
	ArrayPattern {
		// ex: `[first, second, ...others]`
		items: Vec<usize>,
		// the `Identifier` of `...others`, always last
		rest: Option<usize>,
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
		name: Name,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	Mutable {
		name: Name,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	Use {
		// `mutable use` declares a reference that can be moved with `reuse`
		name: Name,
		type_expression: Option<usize>,
		expression: usize,
		is_mutable: bool,
	},
	Reuse {
		// moves a `mutable use` reference, ex: `reuse best = player`
		name: Name,
		expression: usize,
	},
	Static {
		// compile-time value, ex: `static version = @readFile("version.txt")`
		name: Name,
		type_expression: Option<usize>,
		expression: usize,
	},
	Test {
		// ex: `test "adds numbers" { ... }`, `test skip "..." { }`, `@test "..." { }`
		name: usize,
		modifier: Option<TestModifier>,
		// a `Block` node
		body: usize,
		is_compile_time: bool,
	},
	Type {
		// ex: `type Point<NumberType> = { x: NumberType }`
		name: Name,
		parameters: Vec<usize>,
		expression: usize,
	},
	Reactive {
		name: Name,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	Derived {
		name: Name,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	Namespace {
		name: Name,
		expression: usize,
	},
	Function {
		// ex: `async function name = (a: Integer): Integer => a + 1`,
		// `function ascending: Comparator = (a, b) => a - b`
		// the expression is any function: a lambda, an alias, a composition, ... The generic
		// parameters start the signature of the lambda, ex: `function first = <Item>(…) => …`
		name: Name,
		is_async: bool,
		// the function type, ex: `Comparator` in `function ascending: Comparator = ...`
		type_expression: Option<usize>,
		expression: usize,
	},
	Parameters {
		// the parameters of a function, ex: `(a: Integer, self, { x, y }: Point)`
		items: Vec<usize>,
	},
	DestructuredParameter {
		// ex: `{ x, y }: Point`
		pattern: usize,
		type_expression: usize,
	},
	ArrowFunction {
		// a lambda, ex: `(a: Integer): Integer throws E requires r: R => a + 1`, or a function
		// type, whose body is the return type, ex: `(a: Integer) => Integer`
		signature: Box<FunctionSignature>,
		// a `Block` node, or an expression
		body: usize,
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
	Effect {
		// ex: `Void mutates self.balance` (in a function type, or in a signature before `=>`)
		signature: usize,
		kind: EffectKind,
		paths: Vec<usize>,
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
		// `name = value`, `name: Type` or `name: Type = value`
		name: usize,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	ExportValue {
		// ex: `export = 42`, `export mutable: Pool<Person> = []`
		is_mutable: bool,
		type_expression: Option<usize>,
		expression: usize,
	},
	ExportFunction {
		// ex: `export function = ...`, `export async function = <Item>(…) => …`
		is_async: bool,
		type_expression: Option<usize>,
		expression: usize,
	},
	ExportType {
		// ex: `export type = ...`, `export type<First, Second> = ...`
		parameters: Vec<usize>,
		expression: usize,
	},
	ExportNamespace {
		expression: usize,
	},

	/* -------------------------------- Functions ------------------------------- */
	FunctionCall {
		function: usize,
		parameters: Vec<usize>,
	},
	GenericApplication {
		// explicit generic parameters, ex: `Array<Integer>`, `Integer<min = 0>`, `first<Float>`
		target: usize,
		arguments: Vec<usize>,
	},
}

impl Node {
	/// The indices of every child node, in source order (except for the signature of a lambda,
	/// listed before its body).
	pub fn children(&self) -> impl Iterator<Item = usize> + use<> {
		let mut children = Vec::new();
		self.for_each_child(|child| children.push(child));
		children.into_iter()
	}

	/// Call `visit` with the index of every child node, in source order (except for the
	/// signature of a lambda, visited before its body). Doesn't allocate.
	pub fn for_each_child(&self, mut visit: impl FnMut(usize)) {
		let visit = &mut visit as &mut dyn FnMut(usize);
		let all = |indices: &[usize], visit: &mut dyn FnMut(usize)| {
			for index in indices {
				visit(*index);
			}
		};
		let optional = |index: &Option<usize>, visit: &mut dyn FnMut(usize)| {
			if let Some(index) = index {
				visit(*index);
			}
		};
		match self {
			Node::Invalid
			| Node::Placeholder
			| Node::Identifier { .. }
			| Node::ImplicitMember { .. }
			| Node::Integer(_)
			| Node::Number(_)
			| Node::BigInteger(_)
			| Node::StringLiteral(_)
			| Node::None
			| Node::Error
			| Node::Boolean(_)
			| Node::Continue
			| Node::EmptyGroup => {}

			Node::Module { statements, .. } | Node::Block { statements, .. } => {
				all(statements, visit)
			}
			Node::StringTemplate { parts } => {
				for part in parts {
					if let StringPart::Expression(index) = part {
						visit(*index);
					}
				}
			}
			Node::List { items }
			| Node::Tuple { items }
			| Node::Members { items }
			| Node::TuplePattern { items }
			| Node::Parameters { items } => all(items, visit),
			Node::Add { operands }
			| Node::Subtract { operands }
			| Node::Multiply { operands }
			| Node::Divide { operands }
			| Node::IntegerDivide { operands }
			| Node::Remainder { operands }
			| Node::Fallback { operands }
			| Node::Power { operands }
			| Node::Equal { operands }
			| Node::NotEqual { operands }
			| Node::Comparison { operands, .. }
			| Node::And { operands }
			| Node::Or { operands }
			| Node::Union { operands }
			| Node::Compose { operands }
			| Node::Access { operands }
			| Node::OptionalAccess { operands } => all(operands, visit),

			Node::Negate { right }
			| Node::Spread { right }
			| Node::Not { right }
			| Node::Try { right }
			| Node::Throw { right }
			| Node::Await { right }
			| Node::Start { right }
			| Node::Assert { right }
			| Node::InRelation { right }
			| Node::Readonly { right } => visit(*right),
			Node::Percentage { value } | Node::Optional { value } => visit(*value),
			Node::Group { expression }
			| Node::ExportNamespace { expression }
			| Node::Reuse { expression, .. }
			| Node::Namespace { expression, .. } => visit(*expression),
			Node::Loop { body } | Node::Do { body } | Node::Watch { body } => visit(*body),
			Node::Return { expression } | Node::Break { expression } => optional(expression, visit),
			Node::Catch {
				expression,
				handler,
				..
			} => {
				visit(*expression);
				visit(*handler);
			}
			Node::Provide {
				type_expression,
				expression,
			} => {
				visit(*type_expression);
				visit(*expression);
			}
			Node::For {
				pattern,
				iterable,
				parallel,
				body,
				..
			} => {
				if let Some(parallel) = parallel {
					all(parallel, visit);
				}
				optional(pattern, visit);
				visit(*iterable);
				visit(*body);
			}
			Node::While { expression, body } => {
				visit(*expression);
				visit(*body);
			}
			Node::WhileLet {
				pattern,
				expression,
				body,
			} => {
				visit(*pattern);
				visit(*expression);
				visit(*body);
			}
			Node::If {
				condition,
				then_body,
				else_body,
				..
			} => {
				visit(*condition);
				visit(*then_body);
				optional(else_body, visit);
			}
			Node::When {
				expression,
				branches,
			} => {
				visit(*expression);
				for branch in branches {
					if let WhenBranchPattern::Expression(pattern) = branch.pattern {
						visit(pattern);
					}
					optional(&branch.binding, visit);
					visit(branch.value);
				}
			}
			Node::Is { left, right, .. }
			| Node::IsIn { left, right, .. }
			| Node::In { left, right }
			| Node::Relation { left, right } => {
				visit(*left);
				visit(*right);
			}
			Node::Range { start, end, .. } => {
				optional(start, visit);
				optional(end, visit);
			}
			Node::PartialApplication {
				function,
				arguments,
			} => {
				visit(*function);
				all(arguments, visit);
			}
			Node::Index { target, index } => {
				visit(*target);
				visit(*index);
			}
			Node::MemberIndex { target, members } => {
				visit(*target);
				all(members, visit);
			}
			Node::ObjectPattern { fields, rest } => {
				for field in fields {
					optional(&field.nested, visit);
					optional(&field.default, visit);
				}
				optional(rest, visit);
			}
			Node::ArrayPattern { items, rest } => {
				all(items, visit);
				optional(rest, visit);
			}
			Node::Destructuring {
				pattern,
				expression,
				..
			} => {
				visit(*pattern);
				visit(*expression);
			}
			Node::Assignment {
				name,
				type_expression,
				expression,
			} => {
				visit(*name);
				optional(type_expression, visit);
				optional(expression, visit);
			}
			Node::Let {
				type_expression,
				expression,
				..
			}
			| Node::Mutable {
				type_expression,
				expression,
				..
			}
			| Node::Reactive {
				type_expression,
				expression,
				..
			}
			| Node::Derived {
				type_expression,
				expression,
				..
			} => {
				optional(type_expression, visit);
				optional(expression, visit);
			}
			Node::Use {
				type_expression,
				expression,
				..
			}
			| Node::Static {
				type_expression,
				expression,
				..
			}
			| Node::ExportValue {
				type_expression,
				expression,
				..
			} => {
				optional(type_expression, visit);
				visit(*expression);
			}
			Node::Test { name, body, .. } => {
				visit(*name);
				visit(*body);
			}
			Node::Type {
				parameters,
				expression,
				..
			}
			| Node::ExportType {
				parameters,
				expression,
			} => {
				all(parameters, visit);
				visit(*expression);
			}
			Node::Function {
				type_expression,
				expression,
				..
			}
			| Node::ExportFunction {
				type_expression,
				expression,
				..
			} => {
				optional(type_expression, visit);
				visit(*expression);
			}
			Node::DestructuredParameter {
				pattern,
				type_expression,
			} => {
				visit(*pattern);
				visit(*type_expression);
			}
			Node::ArrowFunction { signature, body } => {
				all(&signature.generic_parameters, visit);
				all(&signature.parameters, visit);
				optional(&signature.return_type_expression, visit);
				optional(&signature.errors, visit);
				all(&signature.requirements, visit);
				all(&signature.mutations, visit);
				all(&signature.invalidations, visit);
				visit(*body);
			}
			Node::Throws { signature, errors } => {
				visit(*signature);
				visit(*errors);
			}
			Node::Requires {
				signature,
				requirements,
			} => {
				visit(*signature);
				all(requirements, visit);
			}
			Node::Effect {
				signature, paths, ..
			} => {
				visit(*signature);
				all(paths, visit);
			}
			Node::CompoundAssignment {
				target, expression, ..
			} => {
				visit(*target);
				visit(*expression);
			}
			Node::FunctionCall {
				function,
				parameters,
			} => {
				visit(*function);
				all(parameters, visit);
			}
			Node::GenericApplication { target, arguments } => {
				visit(*target);
				all(arguments, visit);
			}
		}
	}

	/// The name of the variant, like `"Identifier"` or `"ArrowFunction"`.
	pub fn kind_name(&self) -> &'static str {
		match self {
			Node::Module { .. } => "Module",
			Node::Block { .. } => "Block",
			Node::Invalid => "Invalid",
			Node::Placeholder => "Placeholder",
			Node::Identifier { .. } => "Identifier",
			Node::ImplicitMember { .. } => "ImplicitMember",
			Node::Integer(_) => "Integer",
			Node::Number(_) => "Number",
			Node::BigInteger(_) => "BigInteger",
			Node::StringLiteral(_) => "StringLiteral",
			Node::StringTemplate { .. } => "StringTemplate",
			Node::None => "None",
			Node::Error => "Error",
			Node::Boolean(_) => "Boolean",
			Node::List { .. } => "List",
			Node::Tuple { .. } => "Tuple",
			Node::Members { .. } => "Members",
			Node::Negate { .. } => "Negate",
			Node::Spread { .. } => "Spread",
			Node::Not { .. } => "Not",
			Node::Return { .. } => "Return",
			Node::Try { .. } => "Try",
			Node::Throw { .. } => "Throw",
			Node::Catch { .. } => "Catch",
			Node::Await { .. } => "Await",
			Node::Start { .. } => "Start",
			Node::Assert { .. } => "Assert",
			Node::Provide { .. } => "Provide",
			Node::Break { .. } => "Break",
			Node::Continue => "Continue",
			Node::For { .. } => "For",
			Node::While { .. } => "While",
			Node::WhileLet { .. } => "WhileLet",
			Node::Loop { .. } => "Loop",
			Node::Do { .. } => "Do",
			Node::Watch { .. } => "Watch",
			Node::If { .. } => "If",
			Node::When { .. } => "When",
			Node::Add { .. } => "Add",
			Node::Subtract { .. } => "Subtract",
			Node::Multiply { .. } => "Multiply",
			Node::Divide { .. } => "Divide",
			Node::IntegerDivide { .. } => "IntegerDivide",
			Node::Remainder { .. } => "Remainder",
			Node::Fallback { .. } => "Fallback",
			Node::Power { .. } => "Power",
			Node::Equal { .. } => "Equal",
			Node::NotEqual { .. } => "NotEqual",
			Node::Comparison { .. } => "Comparison",
			Node::And { .. } => "And",
			Node::Or { .. } => "Or",
			Node::Is { .. } => "Is",
			Node::IsIn { .. } => "IsIn",
			Node::In { .. } => "In",
			Node::InRelation { .. } => "InRelation",
			Node::Range { .. } => "Range",
			Node::Union { .. } => "Union",
			Node::Compose { .. } => "Compose",
			Node::PartialApplication { .. } => "PartialApplication",
			Node::Percentage { .. } => "Percentage",
			Node::Optional { .. } => "Optional",
			Node::Index { .. } => "Index",
			Node::MemberIndex { .. } => "MemberIndex",
			Node::Relation { .. } => "Relation",
			Node::Access { .. } => "Access",
			Node::OptionalAccess { .. } => "OptionalAccess",
			Node::ObjectPattern { .. } => "ObjectPattern",
			Node::ArrayPattern { .. } => "ArrayPattern",
			Node::TuplePattern { .. } => "TuplePattern",
			Node::Destructuring { .. } => "Destructuring",
			Node::EmptyGroup => "EmptyGroup",
			Node::Group { .. } => "Group",
			Node::Let { .. } => "Let",
			Node::Mutable { .. } => "Mutable",
			Node::Use { .. } => "Use",
			Node::Reuse { .. } => "Reuse",
			Node::Static { .. } => "Static",
			Node::Test { .. } => "Test",
			Node::Type { .. } => "Type",
			Node::Reactive { .. } => "Reactive",
			Node::Derived { .. } => "Derived",
			Node::Namespace { .. } => "Namespace",
			Node::Function { .. } => "Function",
			Node::Parameters { .. } => "Parameters",
			Node::DestructuredParameter { .. } => "DestructuredParameter",
			Node::ArrowFunction { .. } => "ArrowFunction",
			Node::Throws { .. } => "Throws",
			Node::Requires { .. } => "Requires",
			Node::Effect { .. } => "Effect",
			Node::Readonly { .. } => "Readonly",
			Node::CompoundAssignment { .. } => "CompoundAssignment",
			Node::Assignment { .. } => "Assignment",
			Node::ExportValue { .. } => "ExportValue",
			Node::ExportFunction { .. } => "ExportFunction",
			Node::ExportType { .. } => "ExportType",
			Node::ExportNamespace { .. } => "ExportNamespace",
			Node::FunctionCall { .. } => "FunctionCall",
			Node::GenericApplication { .. } => "GenericApplication",
		}
	}
}
