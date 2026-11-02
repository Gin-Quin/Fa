//! The view layer over the parser's tree (decision A5, design §7).
//!
//! This is the only module that matches on `fa_parser::Node`. [`view`] turns a
//! node into a [`View`], the analyzer's own enum, which also normalizes the
//! parser's quirks:
//!
//! - n-ary operators (`a + b + c` is one node with three operands) are kept as
//!   operand lists, with a fixed meaning: [`View::Arithmetic`], [`View::Logic`]
//!   and [`View::Fallback`] fold **left** (`((a + b) + c)`), except `**`, which
//!   folds **right** ([`ArithOp::is_right_associative`]; the parser always gives
//!   it two operands);
//! - a comparison chain `a < b <= c` is [`View::Comparison`]: it means
//!   `a < b and b <= c`, each middle operand evaluated once;
//! - integer literals are decoded to `I256`;
//! - `Invalid` and `Placeholder` nodes are [`View::Error`]: no type, no diagnostic
//!   (the parser already reported the error);
//! - a node the analyzer doesn't handle yet is [`View::Unsupported`], so a new
//!   parser node never breaks the build: it shows up as `AN001` until mapped.
//!
//! [`block_declarations`] is the hoisting scan of a block (one pass over its
//! statements, never into bodies). The parser's own `hoisted_symbols` are not read.

use ethnum::I256;
use fa_parser::nodes::{
	ComparisonOperator, CompoundOperator, DeclarationKind, FunctionSignature, PatternField,
	StringPart, WhenBranch, WhenBranchPattern,
};
use fa_parser::{Node, TypedSyntaxTree};

pub use fa_parser::nodes::{EffectKind, Name, TestModifier};

use crate::diagnostics::SourceSpan;
use crate::ids::NodeId;

fn id(index: usize) -> NodeId {
	NodeId::new(index)
}

fn optional(index: Option<usize>) -> Option<NodeId> {
	index.map(id)
}

/// A list of child nodes, borrowed from the tree.
#[derive(Copy, Clone, Debug)]
pub struct NodeList<'t>(&'t [usize]);

impl<'t> NodeList<'t> {
	/// The number of nodes.
	pub fn len(self) -> usize {
		self.0.len()
	}

	/// Whether the list is empty.
	pub fn is_empty(self) -> bool {
		self.0.is_empty()
	}

	/// The node at `index`.
	pub fn get(self, index: usize) -> Option<NodeId> {
		self.0.get(index).copied().map(id)
	}

	/// The first node.
	pub fn first(self) -> Option<NodeId> {
		self.get(0)
	}

	/// The last node.
	pub fn last(self) -> Option<NodeId> {
		self.0.last().copied().map(id)
	}

	/// The list without its first `count` nodes.
	pub fn skip(self, count: usize) -> NodeList<'t> {
		NodeList(self.0.get(count..).unwrap_or(&[]))
	}

	/// The nodes, in source order.
	pub fn iter(self) -> impl DoubleEndedIterator<Item = NodeId> + ExactSizeIterator + 't {
		self.0.iter().copied().map(id)
	}
}

/// An arithmetic operator.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ArithOp {
	/// `+`
	Add,
	/// `-`
	Subtract,
	/// `*`
	Multiply,
	/// `/` (always a `Float`)
	Divide,
	/// `//`
	IntegerDivide,
	/// `%%`
	Remainder,
	/// `**`
	Power,
}

impl ArithOp {
	/// The operator as written.
	pub fn symbol(self) -> &'static str {
		match self {
			ArithOp::Add => "+",
			ArithOp::Subtract => "-",
			ArithOp::Multiply => "*",
			ArithOp::Divide => "/",
			ArithOp::IntegerDivide => "//",
			ArithOp::Remainder => "%%",
			ArithOp::Power => "**",
		}
	}

	/// `**` folds right: `a ** b ** c` is `a ** (b ** c)`. The others fold left.
	pub fn is_right_associative(self) -> bool {
		self == ArithOp::Power
	}
}

/// An ordering operator of a comparison chain.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum CompareOp {
	/// `<`
	Less,
	/// `<=`
	LessOrEqual,
	/// `>`
	Greater,
	/// `>=`
	GreaterOrEqual,
}

impl CompareOp {
	/// The operator as written.
	pub fn symbol(self) -> &'static str {
		match self {
			CompareOp::Less => "<",
			CompareOp::LessOrEqual => "<=",
			CompareOp::Greater => ">",
			CompareOp::GreaterOrEqual => ">=",
		}
	}
}

/// The operators of a comparison chain (`operands.len() == operators.len() + 1`).
#[derive(Copy, Clone, Debug)]
pub struct CompareOps<'t>(&'t [ComparisonOperator]);

impl CompareOps<'_> {
	/// The number of operators.
	pub fn len(self) -> usize {
		self.0.len()
	}

	/// Whether there is no operator (never, for a parsed comparison).
	pub fn is_empty(self) -> bool {
		self.0.is_empty()
	}

	/// The operator between operands `index` and `index + 1`.
	pub fn get(self, index: usize) -> Option<CompareOp> {
		self.0.get(index).map(|operator| match operator {
			ComparisonOperator::LessThan => CompareOp::Less,
			ComparisonOperator::LessThanOrEqual => CompareOp::LessOrEqual,
			ComparisonOperator::GreaterThan => CompareOp::Greater,
			ComparisonOperator::GreaterThanOrEqual => CompareOp::GreaterOrEqual,
		})
	}

	/// The operators, in source order.
	pub fn iter(self) -> impl Iterator<Item = CompareOp> {
		(0..self.len()).filter_map(move |index| self.get(index))
	}
}

/// `and` or `or`.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum LogicOp {
	/// `and`
	And,
	/// `or`
	Or,
}

/// The keyword of a declaration.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BindingKind {
	/// `let`
	Let,
	/// `mutable`
	Mutable,
	/// `use` (M2)
	Use,
	/// `mutable use` (M2)
	MutableUse,
	/// `static` (M3)
	Static,
	/// `reactive` (M3)
	Reactive,
	/// `derived` (M3)
	Derived,
}

impl BindingKind {
	/// The keyword as written.
	pub fn keyword(self) -> &'static str {
		match self {
			BindingKind::Let => "let",
			BindingKind::Mutable => "mutable",
			BindingKind::Use => "use",
			BindingKind::MutableUse => "mutable use",
			BindingKind::Static => "static",
			BindingKind::Reactive => "reactive",
			BindingKind::Derived => "derived",
		}
	}
}

/// One part of a string template.
#[derive(Copy, Clone, Debug)]
pub enum TemplatePart<'t> {
	/// Literal text (escapes already decoded).
	Text(&'t str),
	/// An interpolated expression.
	Expression(NodeId),
}

/// The parts of a string template.
#[derive(Copy, Clone, Debug)]
pub struct TemplateParts<'t>(&'t [StringPart]);

impl<'t> TemplateParts<'t> {
	/// The parts, in source order.
	pub fn iter(self) -> impl Iterator<Item = TemplatePart<'t>> + 't {
		self.0.iter().map(|part| match part {
			StringPart::Literal(text) => TemplatePart::Text(text),
			StringPart::Expression(index) => TemplatePart::Expression(id(*index)),
		})
	}
}

/// One arm of a `when`.
#[derive(Copy, Clone, Debug)]
pub struct WhenArm {
	/// The pattern; `None` for `else`.
	pub pattern: Option<NodeId>,
	/// The value bound with `as`, ex: `Shape.Circle as { radius }`.
	pub binding: Option<NodeId>,
	/// A `Block` node, or an expression.
	pub value: NodeId,
}

/// The arms of a `when`.
#[derive(Copy, Clone, Debug)]
pub struct WhenArms<'t>(&'t [WhenBranch]);

impl<'t> WhenArms<'t> {
	/// The number of arms.
	pub fn len(self) -> usize {
		self.0.len()
	}

	/// Whether there is no arm.
	pub fn is_empty(self) -> bool {
		self.0.is_empty()
	}

	/// The arms, in source order.
	pub fn iter(self) -> impl Iterator<Item = WhenArm> + 't {
		self.0.iter().map(|branch| WhenArm {
			pattern: match branch.pattern {
				WhenBranchPattern::Expression(pattern) => Some(id(pattern)),
				WhenBranchPattern::Else => None,
			},
			binding: optional(branch.binding),
			value: id(branch.value),
		})
	}
}

/// A field of an object pattern: `a`, `a as b`, `a = 4`, `nested { c }`.
#[derive(Copy, Clone, Debug)]
pub struct PatternFieldView {
	/// The field name.
	pub name: Name,
	/// The bound name, when it differs (`as b`).
	pub alias: Option<Name>,
	/// A nested pattern.
	pub nested: Option<NodeId>,
	/// The default value.
	pub default: Option<NodeId>,
}

/// The fields of an object pattern.
#[derive(Copy, Clone, Debug)]
pub struct PatternFields<'t>(&'t [PatternField]);

impl<'t> PatternFields<'t> {
	/// The fields, in source order.
	pub fn iter(self) -> impl Iterator<Item = PatternFieldView> + 't {
		self.0.iter().map(|field| PatternFieldView {
			name: field.name,
			alias: field.alias,
			nested: optional(field.nested),
			default: optional(field.default),
		})
	}
}

/// The signature of a lambda or a function type.
#[derive(Copy, Clone, Debug)]
pub struct SignatureView<'t> {
	/// `async`
	pub is_async: bool,
	/// The parameters (identifiers, `name: Type` assignments, destructured parameters).
	pub parameters: NodeList<'t>,
	/// The declared return type.
	pub returns: Option<NodeId>,
	/// `throws E`
	pub throws: Option<NodeId>,
	/// `requires r: R`
	pub requires: NodeList<'t>,
	/// `mutates p`
	pub mutates: NodeList<'t>,
	/// `invalidates p`
	pub invalidates: NodeList<'t>,
}

impl<'t> SignatureView<'t> {
	fn new(signature: &'t FunctionSignature) -> Self {
		SignatureView {
			is_async: signature.is_async,
			parameters: NodeList(&signature.parameters),
			returns: optional(signature.return_type_expression),
			throws: optional(signature.errors),
			requires: NodeList(&signature.requirements),
			mutates: NodeList(&signature.mutations),
			invalidates: NodeList(&signature.invalidations),
		}
	}
}

/// The analyzer's view of a node. Each variant is handled by one function of
/// one `check/*.rs` file (see the dispatch in `check/mod.rs`).
#[derive(Copy, Clone, Debug)]
pub enum View<'t> {
	// ── structure ────────────────────────────────────────────────────────
	/// The file.
	Module { statements: NodeList<'t> },
	/// `{ … }` after `if`, `for`, `=>`, …
	Block { statements: NodeList<'t> },
	/// A parser error (`Invalid`) or a `...` placeholder: no type, no diagnostic.
	Error,
	/// A node the analyzer doesn't handle yet: `AN001`. The name is the parser's.
	Unsupported(&'static str),

	// ── literals and names ───────────────────────────────────────────────
	/// An integer literal.
	Integer(I256),
	/// A float literal.
	Float(f64),
	/// A big integer literal, as written (`-12_000n`).
	BigInt(&'t str),
	/// A string literal without interpolation (escapes decoded).
	String(&'t str),
	/// A string with interpolations.
	Template { parts: TemplateParts<'t> },
	/// `true` or `false`.
	Boolean(bool),
	/// `none`.
	None,
	/// A name.
	Name(Name),
	/// `.red` where a `Color` is expected.
	ImplicitMember(Name),
	/// The `Error` keyword (the built-in error type).
	ErrorType,

	// ── composed values ──────────────────────────────────────────────────
	/// `(expression)`
	Group(NodeId),
	/// `()`
	EmptyGroup,
	/// `[a, b, c]`
	Array { items: NodeList<'t> },
	/// `a, b, c`
	Tuple { items: NodeList<'t> },
	/// `{ a = 1, b, ...c }`: items are `Assignment`, `Name` or `Spread` nodes.
	Object { items: NodeList<'t> },
	/// `Point { x = 1 }`
	TypedObject {
		target: NodeId,
		members: NodeList<'t>,
	},
	/// `...value`
	Spread(NodeId),
	/// `a..b`, `a..=b`, `a..`, `..b`, `..`
	Range {
		start: Option<NodeId>,
		end: Option<NodeId>,
		inclusive: bool,
	},
	/// `a.b.c`: `members` are `Name` nodes (or `Error` for a missing one, `user.`).
	Access {
		target: NodeId,
		members: NodeList<'t>,
	},
	/// `a?.b?.c`
	OptionalAccess {
		target: NodeId,
		members: NodeList<'t>,
	},
	/// `items[index]`
	Index { target: NodeId, index: NodeId },
	/// `f(a, b = 1)`: named arguments are `Assignment` nodes.
	Call {
		callee: NodeId,
		arguments: NodeList<'t>,
	},

	// ── operators ────────────────────────────────────────────────────────
	/// `+ - * / // %% **`, n-ary: folds left, `**` right.
	Arithmetic { op: ArithOp, operands: NodeList<'t> },
	/// `-x`
	Negate(NodeId),
	/// `(x)%`, `50%`
	Percentage(NodeId),
	/// `a == b` (`negated`: `a != b`)
	Equality {
		left: NodeId,
		right: NodeId,
		negated: bool,
	},
	/// `a < b <= c`: `a < b and b <= c`.
	Comparison {
		operands: NodeList<'t>,
		operators: CompareOps<'t>,
	},
	/// `and`, `or`, n-ary: folds left.
	Logic { op: LogicOp, operands: NodeList<'t> },
	/// `not x`
	Not(NodeId),
	/// `a ?? b ?? c`, folds left.
	Fallback { operands: NodeList<'t> },
	/// `a is T`, `a is not T`
	Is {
		value: NodeId,
		ty: NodeId,
		negated: bool,
	},
	/// `a is in b`, `a is not in b`
	IsIn {
		value: NodeId,
		collection: NodeId,
		negated: bool,
	},

	// ── declarations and assignments ─────────────────────────────────────
	/// `let`, `mutable`, `static`, `reactive`, `derived`.
	Declaration {
		kind: BindingKind,
		name: Name,
		annotation: Option<NodeId>,
		value: Option<NodeId>,
	},
	/// `use name = place`, `mutable use name = place` (M2).
	Use {
		name: Name,
		annotation: Option<NodeId>,
		value: NodeId,
		mutable: bool,
	},
	/// `reuse name = place` (M2).
	Reuse { name: Name, value: NodeId },
	/// `let { a, b } = object`
	Destructuring {
		kind: BindingKind,
		pattern: NodeId,
		value: NodeId,
	},
	/// `target = value`, `name: Type = value` (also the fields of an object literal
	/// and named arguments, handled by their parent).
	Assignment {
		target: NodeId,
		annotation: Option<NodeId>,
		value: Option<NodeId>,
	},
	/// `target += value` and the other arithmetic compound assignments.
	CompoundAssignment {
		op: ArithOp,
		target: NodeId,
		value: NodeId,
	},
	/// `target .= { field = value }`
	UpdateFields { target: NodeId, value: NodeId },
	/// `type Name(Parameters) = expression`
	TypeDecl {
		name: Name,
		parameters: NodeList<'t>,
		value: NodeId,
	},
	/// `namespace Name = { … }`
	Namespace { name: Name, value: NodeId },
	/// `function name(Generics): Type = expression`
	Function {
		name: Name,
		is_async: bool,
		generics: NodeList<'t>,
		annotation: Option<NodeId>,
		value: NodeId,
	},
	/// `export = value`, `export mutable: T = value`
	ExportValue {
		mutable: bool,
		annotation: Option<NodeId>,
		value: NodeId,
	},
	/// `export function(Generics) = …`
	ExportFunction {
		is_async: bool,
		generics: NodeList<'t>,
		annotation: Option<NodeId>,
		value: NodeId,
	},
	/// `export type(Parameters) = …`
	ExportType {
		parameters: NodeList<'t>,
		value: NodeId,
	},
	/// `export namespace …`
	ExportNamespace(NodeId),

	// ── functions ────────────────────────────────────────────────────────
	/// A lambda `(a: Integer): Integer => a + 1`; in a type, a function type whose
	/// body is the return type.
	Lambda {
		signature: SignatureView<'t>,
		body: NodeId,
	},
	/// The parameters of a function.
	Parameters { items: NodeList<'t> },
	/// `{ x, y }: Point` as a parameter.
	DestructuredParameter { pattern: NodeId, annotation: NodeId },
	/// `f then g then h`
	Compose { operands: NodeList<'t> },
	/// `f with (y = 0)`
	PartialApplication {
		function: NodeId,
		arguments: NodeList<'t>,
	},
	/// `provide Database = value` (M2 requirements).
	Provide { ty: NodeId, value: NodeId },

	// ── control flow ─────────────────────────────────────────────────────
	/// `if c { … } else …`: `else_body` is a `Block` or an `If`.
	If {
		condition: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
		compile_time: bool,
	},
	/// `when subject { pattern => value, … }`
	When { subject: NodeId, arms: WhenArms<'t> },
	/// `while condition { … }`
	While { condition: NodeId, body: NodeId },
	/// `while let name = value { … }`
	WhileLet {
		pattern: NodeId,
		value: NodeId,
		body: NodeId,
	},
	/// `for pattern, index in iterable { … }`
	For {
		pattern: Option<NodeId>,
		index: Option<Name>,
		iterable: NodeId,
		body: NodeId,
		parallel: Option<NodeList<'t>>,
		compile_time: bool,
	},
	/// `loop { … }`
	Loop(NodeId),
	/// `do { … }`
	Do(NodeId),
	/// `break`, `break value`
	Break(Option<NodeId>),
	/// `continue`
	Continue,
	/// `return`, `return value`
	Return(Option<NodeId>),

	// ── errors ───────────────────────────────────────────────────────────
	/// `try expression`
	Try(NodeId),
	/// `throw error`
	Throw(NodeId),
	/// `expression catch error => handler`
	Catch {
		expression: NodeId,
		error_name: Name,
		handler: NodeId,
	},

	// ── tests ────────────────────────────────────────────────────────────
	/// `test "name" { … }`
	Test {
		name: NodeId,
		modifier: Option<TestModifier>,
		body: NodeId,
		compile_time: bool,
	},
	/// `assert condition`
	Assert(NodeId),

	// ── patterns ─────────────────────────────────────────────────────────
	/// `{ a, b as c, d = 4, nested { e }, ...rest }`
	ObjectPattern {
		fields: PatternFields<'t>,
		rest: Option<NodeId>,
	},
	/// `[first, second, ...others]`
	ArrayPattern {
		items: NodeList<'t>,
		rest: Option<NodeId>,
	},
	/// `(number, name)`
	TuplePattern { items: NodeList<'t> },

	// ── type-only constructs ─────────────────────────────────────────────
	/// `A | B`
	Union { operands: NodeList<'t> },
	/// `T?`
	Optional(NodeId),
	/// `T in owner` (M2 references)
	RefType { item: NodeId, owner: NodeId },
	/// `Signature throws E`
	Throws { signature: NodeId, errors: NodeId },
	/// `Signature requires r: R`
	Requires {
		signature: NodeId,
		requirements: NodeList<'t>,
	},
	/// `Signature mutates p`, `Signature invalidates p`
	Effect {
		signature: NodeId,
		kind: EffectKind,
		paths: NodeList<'t>,
	},
	/// `readonly field: T` in a type.
	Readonly(NodeId),
}

fn binding_kind(kind: DeclarationKind) -> BindingKind {
	match kind {
		DeclarationKind::Let => BindingKind::Let,
		DeclarationKind::Mutable => BindingKind::Mutable,
		DeclarationKind::Use => BindingKind::Use,
		DeclarationKind::MutableUse => BindingKind::MutableUse,
	}
}

fn arith(op: CompoundOperator) -> Option<ArithOp> {
	Some(match op {
		CompoundOperator::Add => ArithOp::Add,
		CompoundOperator::Subtract => ArithOp::Subtract,
		CompoundOperator::Multiply => ArithOp::Multiply,
		CompoundOperator::Power => ArithOp::Power,
		CompoundOperator::Divide => ArithOp::Divide,
		CompoundOperator::IntegerDivide => ArithOp::IntegerDivide,
		CompoundOperator::Remainder => ArithOp::Remainder,
		CompoundOperator::Update => return None,
	})
}

/// Decodes an integer literal to `I256` (every integer literal fits).
pub fn decode_integer(literal: fa_parser::nodes::IntegerLiteral) -> I256 {
	let magnitude = I256::from(literal.magnitude);
	if literal.is_negative {
		-magnitude
	} else {
		magnitude
	}
}

/// The view of node `n`. A node index out of range is [`View::Error`].
pub fn view(tree: &TypedSyntaxTree, n: NodeId) -> View<'_> {
	let Some(node) = tree.nodes.get(n.index()) else {
		return View::Error;
	};
	fn list(items: &[usize]) -> NodeList<'_> {
		NodeList(items)
	}
	fn arithmetic(op: ArithOp, operands: &[usize]) -> View<'_> {
		View::Arithmetic {
			op,
			operands: NodeList(operands),
		}
	}
	let two = |operands: &[usize]| {
		(
			operands.first().copied().map(id),
			operands.get(1).copied().map(id),
		)
	};
	match node {
		Node::Module { statements, .. } => View::Module {
			statements: list(statements),
		},
		Node::Block { statements, .. } => View::Block {
			statements: list(statements),
		},
		Node::Invalid | Node::Placeholder => View::Error,

		Node::Identifier { name } => View::Name(*name),
		Node::ImplicitMember { name } => View::ImplicitMember(*name),
		Node::Integer(literal) => View::Integer(decode_integer(*literal)),
		Node::Number(value) => View::Float(*value),
		Node::BigInteger(name) => View::BigInt(tree.name(*name)),
		Node::StringLiteral(text) => View::String(text),
		Node::StringTemplate { parts } => View::Template {
			parts: TemplateParts(parts),
		},
		Node::None => View::None,
		Node::Error => View::ErrorType,
		Node::Boolean(value) => View::Boolean(*value),

		Node::List { items } => View::Array { items: list(items) },
		Node::Tuple { items } => View::Tuple { items: list(items) },
		Node::Members { items } => View::Object { items: list(items) },
		Node::MemberIndex { target, members } => View::TypedObject {
			target: id(*target),
			members: list(members),
		},
		Node::Spread { right } => View::Spread(id(*right)),
		Node::Range {
			start,
			end,
			inclusive,
		} => View::Range {
			start: optional(*start),
			end: optional(*end),
			inclusive: *inclusive,
		},
		Node::Access { operands } | Node::OptionalAccess { operands } => {
			let Some((target, members)) = operands.split_first() else {
				return View::Error;
			};
			let (target, members) = (id(*target), list(members));
			if matches!(node, Node::Access { .. }) {
				View::Access { target, members }
			} else {
				View::OptionalAccess { target, members }
			}
		}
		Node::Index { target, index } => View::Index {
			target: id(*target),
			index: id(*index),
		},
		Node::FunctionCall {
			function,
			parameters,
		} => View::Call {
			callee: id(*function),
			arguments: list(parameters),
		},
		Node::Group { expression } => View::Group(id(*expression)),
		Node::EmptyGroup => View::EmptyGroup,

		Node::Add { operands } => arithmetic(ArithOp::Add, operands),
		Node::Subtract { operands } => arithmetic(ArithOp::Subtract, operands),
		Node::Multiply { operands } => arithmetic(ArithOp::Multiply, operands),
		Node::Divide { operands } => arithmetic(ArithOp::Divide, operands),
		Node::IntegerDivide { operands } => arithmetic(ArithOp::IntegerDivide, operands),
		Node::Remainder { operands } => arithmetic(ArithOp::Remainder, operands),
		Node::Power { operands } => arithmetic(ArithOp::Power, operands),
		Node::Negate { right } => View::Negate(id(*right)),
		Node::Percentage { value } => View::Percentage(id(*value)),
		Node::Equal { operands } | Node::NotEqual { operands } => match two(operands) {
			(Some(left), Some(right)) => View::Equality {
				left,
				right,
				negated: matches!(node, Node::NotEqual { .. }),
			},
			_ => View::Error,
		},
		Node::Comparison {
			operands,
			operators,
		} => {
			if operands.len() != operators.len() + 1 {
				return View::Error;
			}
			View::Comparison {
				operands: list(operands),
				operators: CompareOps(operators),
			}
		}
		Node::And { operands } => View::Logic {
			op: LogicOp::And,
			operands: list(operands),
		},
		Node::Or { operands } => View::Logic {
			op: LogicOp::Or,
			operands: list(operands),
		},
		Node::Not { right } => View::Not(id(*right)),
		Node::Fallback { operands } => View::Fallback {
			operands: list(operands),
		},
		Node::Is {
			left,
			right,
			negated,
		} => View::Is {
			value: id(*left),
			ty: id(*right),
			negated: *negated,
		},
		Node::IsIn {
			left,
			right,
			negated,
		} => View::IsIn {
			value: id(*left),
			collection: id(*right),
			negated: *negated,
		},

		Node::Let {
			name,
			type_expression,
			expression,
		}
		| Node::Mutable {
			name,
			type_expression,
			expression,
		}
		| Node::Reactive {
			name,
			type_expression,
			expression,
		}
		| Node::Derived {
			name,
			type_expression,
			expression,
		} => View::Declaration {
			kind: match node {
				Node::Let { .. } => BindingKind::Let,
				Node::Mutable { .. } => BindingKind::Mutable,
				Node::Reactive { .. } => BindingKind::Reactive,
				_ => BindingKind::Derived,
			},
			name: *name,
			annotation: optional(*type_expression),
			value: optional(*expression),
		},
		Node::Static {
			name,
			type_expression,
			expression,
		} => View::Declaration {
			kind: BindingKind::Static,
			name: *name,
			annotation: optional(*type_expression),
			value: Some(id(*expression)),
		},
		Node::Use {
			name,
			type_expression,
			expression,
			is_mutable,
		} => View::Use {
			name: *name,
			annotation: optional(*type_expression),
			value: id(*expression),
			mutable: *is_mutable,
		},
		Node::Reuse { name, expression } => View::Reuse {
			name: *name,
			value: id(*expression),
		},
		Node::Destructuring {
			kind,
			pattern,
			expression,
		} => View::Destructuring {
			kind: binding_kind(*kind),
			pattern: id(*pattern),
			value: id(*expression),
		},
		Node::Assignment {
			name,
			type_expression,
			expression,
		} => View::Assignment {
			target: id(*name),
			annotation: optional(*type_expression),
			value: optional(*expression),
		},
		Node::CompoundAssignment {
			operator,
			target,
			expression,
		} => match arith(*operator) {
			Some(op) => View::CompoundAssignment {
				op,
				target: id(*target),
				value: id(*expression),
			},
			None => View::UpdateFields {
				target: id(*target),
				value: id(*expression),
			},
		},
		Node::Type {
			name,
			parameters,
			expression,
		} => View::TypeDecl {
			name: *name,
			parameters: list(parameters),
			value: id(*expression),
		},
		Node::Namespace { name, expression } => View::Namespace {
			name: *name,
			value: id(*expression),
		},
		Node::Function {
			name,
			is_async,
			parameters,
			type_expression,
			expression,
		} => View::Function {
			name: *name,
			is_async: *is_async,
			generics: list(parameters),
			annotation: optional(*type_expression),
			value: id(*expression),
		},
		Node::ExportValue {
			is_mutable,
			type_expression,
			expression,
		} => View::ExportValue {
			mutable: *is_mutable,
			annotation: optional(*type_expression),
			value: id(*expression),
		},
		Node::ExportFunction {
			is_async,
			parameters,
			type_expression,
			expression,
		} => View::ExportFunction {
			is_async: *is_async,
			generics: list(parameters),
			annotation: optional(*type_expression),
			value: id(*expression),
		},
		Node::ExportType {
			parameters,
			expression,
		} => View::ExportType {
			parameters: list(parameters),
			value: id(*expression),
		},
		Node::ExportNamespace { expression } => View::ExportNamespace(id(*expression)),

		Node::ArrowFunction { signature, body } => View::Lambda {
			signature: SignatureView::new(signature),
			body: id(*body),
		},
		Node::Parameters { items } => View::Parameters { items: list(items) },
		Node::DestructuredParameter {
			pattern,
			type_expression,
		} => View::DestructuredParameter {
			pattern: id(*pattern),
			annotation: id(*type_expression),
		},
		Node::Compose { operands } => View::Compose {
			operands: list(operands),
		},
		Node::PartialApplication {
			function,
			arguments,
		} => View::PartialApplication {
			function: id(*function),
			arguments: list(arguments),
		},
		Node::Provide {
			type_expression,
			expression,
		} => View::Provide {
			ty: id(*type_expression),
			value: id(*expression),
		},

		Node::If {
			is_compile_time,
			condition,
			then_body,
			else_body,
		} => View::If {
			condition: id(*condition),
			then_body: id(*then_body),
			else_body: optional(*else_body),
			compile_time: *is_compile_time,
		},
		Node::When {
			expression,
			branches,
		} => View::When {
			subject: id(*expression),
			arms: WhenArms(branches),
		},
		Node::While { expression, body } => View::While {
			condition: id(*expression),
			body: id(*body),
		},
		Node::WhileLet {
			pattern,
			expression,
			body,
		} => View::WhileLet {
			pattern: id(*pattern),
			value: id(*expression),
			body: id(*body),
		},
		Node::For {
			pattern,
			index,
			iterable,
			parallel,
			body,
			is_compile_time,
		} => View::For {
			pattern: optional(*pattern),
			index: *index,
			iterable: id(*iterable),
			body: id(*body),
			parallel: parallel.as_deref().map(NodeList),
			compile_time: *is_compile_time,
		},
		Node::Loop { body } => View::Loop(id(*body)),
		Node::Do { body } => View::Do(id(*body)),
		Node::Break { expression } => View::Break(optional(*expression)),
		Node::Continue => View::Continue,
		Node::Return { expression } => View::Return(optional(*expression)),

		Node::Try { right } => View::Try(id(*right)),
		Node::Throw { right } => View::Throw(id(*right)),
		Node::Catch {
			expression,
			error_name,
			handler,
		} => View::Catch {
			expression: id(*expression),
			error_name: *error_name,
			handler: id(*handler),
		},

		Node::Test {
			name,
			modifier,
			body,
			is_compile_time,
		} => View::Test {
			name: id(*name),
			modifier: *modifier,
			body: id(*body),
			compile_time: *is_compile_time,
		},
		Node::Assert { right } => View::Assert(id(*right)),

		Node::ObjectPattern { fields, rest } => View::ObjectPattern {
			fields: PatternFields(fields),
			rest: optional(*rest),
		},
		Node::ArrayPattern { items, rest } => View::ArrayPattern {
			items: list(items),
			rest: optional(*rest),
		},
		Node::TuplePattern { items } => View::TuplePattern { items: list(items) },

		Node::Union { operands } => View::Union {
			operands: list(operands),
		},
		Node::Optional { value } => View::Optional(id(*value)),
		Node::In { left, right } => View::RefType {
			item: id(*left),
			owner: id(*right),
		},
		Node::Throws { signature, errors } => View::Throws {
			signature: id(*signature),
			errors: id(*errors),
		},
		Node::Requires {
			signature,
			requirements,
		} => View::Requires {
			signature: id(*signature),
			requirements: list(requirements),
		},
		Node::Effect {
			signature,
			kind,
			paths,
		} => View::Effect {
			signature: id(*signature),
			kind: *kind,
			paths: list(paths),
		},
		Node::Readonly { right } => View::Readonly(id(*right)),

		// out of the first version's scope, or tier 3: `AN001` until mapped
		Node::Await { .. }
		| Node::Start { .. }
		| Node::Watch { .. }
		| Node::Relation { .. }
		| Node::InRelation { .. } => View::Unsupported(node.kind_name()),
	}
}

/// The children of node `n`, in source order (every child, including names
/// that are not references, like the members of an access).
pub fn children(tree: &TypedSyntaxTree, n: NodeId) -> impl Iterator<Item = NodeId> + use<> {
	let children: Vec<usize> = match tree.nodes.get(n.index()) {
		Some(node) => node.children().collect(),
		None => Vec::new(),
	};
	children.into_iter().map(id)
}

/// The parser's name of the node kind, for messages (`"ArrowFunction"`).
pub fn kind_name(tree: &TypedSyntaxTree, n: NodeId) -> &'static str {
	tree.nodes.get(n.index()).map_or("Invalid", Node::kind_name)
}

/// The span of a node.
pub fn span(tree: &TypedSyntaxTree, n: NodeId) -> SourceSpan {
	tree.spans
		.get(n.index())
		.map_or(SourceSpan::default(), |span| {
			SourceSpan::new(span.start as u32, span.end as u32)
		})
}

/// The span of a name.
pub fn name_span(name: Name) -> SourceSpan {
	SourceSpan::new(name.start, name.end)
}

/// What a hoisted declaration declares.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum HoistKind {
	/// `function`
	Function,
	/// `type`
	Type,
	/// `namespace`
	Namespace,
	/// A value declaration at the module level of a package file (`let`, `mutable`, `static`).
	Value,
}

/// A declaration visible in its whole block (design §3.2).
#[derive(Copy, Clone, Debug)]
pub struct HoistedDecl {
	/// The declaration node.
	pub node: NodeId,
	/// The declared name.
	pub name: Name,
	/// What it declares.
	pub kind: HoistKind,
}

/// The declarations of a block (or module) to declare `Pending` when entering
/// it: `function`, `type` and `namespace`, plus, when `values` is set (the module
/// level of a package file), `let`, `mutable` and `static`. Only the block's own
/// statements are scanned, never the bodies. Declarations with an empty name
/// (after a parse error) are skipped.
pub fn block_declarations(tree: &TypedSyntaxTree, block: NodeId, values: bool) -> Vec<HoistedDecl> {
	let statements = match view(tree, block) {
		// the members of a namespace are the items of its braces
		View::Module { statements }
		| View::Block { statements }
		| View::Object { items: statements } => statements,
		_ => return Vec::new(),
	};
	statements
		.iter()
		.filter_map(|node| {
			let (name, kind) = match view(tree, node) {
				View::Function { name, .. } => (name, HoistKind::Function),
				View::TypeDecl { name, .. } => (name, HoistKind::Type),
				View::Namespace { name, .. } => (name, HoistKind::Namespace),
				View::Declaration {
					name,
					kind: BindingKind::Let | BindingKind::Mutable | BindingKind::Static,
					..
				} if values => (name, HoistKind::Value),
				_ => return None,
			};
			(!name.is_empty()).then_some(HoistedDecl { node, name, kind })
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The first statement of a parsed source.
	fn first(source: &str) -> (TypedSyntaxTree, NodeId) {
		let tree = fa_parser::parse(source);
		let View::Module { statements } = view(&tree, id(tree.root)) else {
			panic!("the root is not a module");
		};
		let statement = statements.first().expect("no statement");
		(tree, statement)
	}

	/// Asserts the view of the first statement of `source`.
	fn assert_view(source: &str, check: impl FnOnce(&TypedSyntaxTree, View<'_>) -> bool) {
		let (tree, n) = first(source);
		let shown = format!("{:?}", view(&tree, n));
		assert!(check(&tree, view(&tree, n)), "{source}: {shown}");
	}

	fn text(tree: &TypedSyntaxTree, n: NodeId) -> &str {
		tree.text(n.index())
	}

	#[test]
	fn literals() {
		assert_view(
			"42",
			|_, v| matches!(v, View::Integer(x) if x == I256::from(42)),
		);
		assert_view(
			"-7",
			|_, v| matches!(v, View::Integer(x) if x == I256::from(-7)),
		);
		assert_view(
			"340282366920938463463374607431768211455",
			|_, v| matches!(v, View::Integer(x) if x == I256::from(u128::MAX)),
		);
		assert_view("1.5", |_, v| matches!(v, View::Float(x) if x == 1.5));
		assert_view("12n", |_, v| matches!(v, View::BigInt("12n")));
		assert_view("\"hi\"", |_, v| matches!(v, View::String("hi")));
		assert_view(
			"\"a{b}c\"",
			|_, v| matches!(v, View::Template { parts } if parts.iter().count() == 3),
		);
		assert_view("true", |_, v| matches!(v, View::Boolean(true)));
		assert_view("none", |_, v| matches!(v, View::None));
		assert_view("Error", |_, v| matches!(v, View::ErrorType));
		assert_view("50%", |_, v| matches!(v, View::Percentage(_)));
	}

	#[test]
	fn names_access_and_calls() {
		assert_view(
			"count",
			|t, v| matches!(v, View::Name(n) if t.name(n) == "count"),
		);
		assert_view("let c: Color = .red", |t, v| {
			let View::Declaration {
				value: Some(value), ..
			} = v
			else {
				return false;
			};
			matches!(view(t, value), View::ImplicitMember(n) if t.name(n) == "red")
		});
		assert_view("a.b.c", |t, v| {
			matches!(v, View::Access { target, members }
				if text(t, target) == "a" && members.len() == 2
				&& matches!(view(t, members.get(1).unwrap()), View::Name(_)))
		});
		assert_view(
			"a?.b",
			|_, v| matches!(v, View::OptionalAccess { members, .. } if members.len() == 1),
		);
		assert_view("items[0]", |_, v| matches!(v, View::Index { .. }));
		assert_view("console.log(x, 1)", |t, v| {
			matches!(v, View::Call { callee, arguments }
				if matches!(view(t, callee), View::Access { .. }) && arguments.len() == 2)
		});
		assert_view("(a)", |_, v| matches!(v, View::Group(_)));
		assert_view("()", |_, v| matches!(v, View::EmptyGroup));
	}

	#[test]
	fn composed_values() {
		assert_view(
			"[1, 2]",
			|_, v| matches!(v, View::Array { items } if items.len() == 2),
		);
		assert_view("let t = (1, 2)", |t, v| {
			let View::Declaration {
				value: Some(value), ..
			} = v
			else {
				return false;
			};
			let value = match view(t, value) {
				View::Group(inner) => inner,
				_ => value,
			};
			matches!(view(t, value), View::Tuple { items } if items.len() == 2)
		});
		assert_view(
			"{ a = 1, b }",
			|_, v| matches!(v, View::Object { items } if items.len() == 2),
		);
		assert_view("Point { x = 1 }", |_, v| {
			matches!(v, View::TypedObject { .. })
		});
		assert_view(
			"[...a]",
			|t, v| matches!(v, View::Array { items } if matches!(view(t, items.get(0).unwrap()), View::Spread(_))),
		);
		assert_view("a..=b", |_, v| {
			matches!(
				v,
				View::Range {
					start: Some(_),
					end: Some(_),
					inclusive: true
				}
			)
		});
	}

	#[test]
	fn operators() {
		assert_view(
			"a + b + c",
			|_, v| matches!(v, View::Arithmetic { op: ArithOp::Add, operands } if operands.len() == 3),
		);
		assert_view("a - b", |_, v| {
			matches!(
				v,
				View::Arithmetic {
					op: ArithOp::Subtract,
					..
				}
			)
		});
		assert_view("a * b", |_, v| {
			matches!(
				v,
				View::Arithmetic {
					op: ArithOp::Multiply,
					..
				}
			)
		});
		assert_view("a / b", |_, v| {
			matches!(
				v,
				View::Arithmetic {
					op: ArithOp::Divide,
					..
				}
			)
		});
		assert_view("a // b", |_, v| {
			matches!(
				v,
				View::Arithmetic {
					op: ArithOp::IntegerDivide,
					..
				}
			)
		});
		assert_view("a %% b", |_, v| {
			matches!(
				v,
				View::Arithmetic {
					op: ArithOp::Remainder,
					..
				}
			)
		});
		assert_view("a ** b ** c", |t, v| {
			let View::Arithmetic {
				op: ArithOp::Power,
				operands,
			} = v
			else {
				return false;
			};
			operands.len() == 2
				&& op_is_power(view(t, operands.get(1).unwrap()))
				&& ArithOp::Power.is_right_associative()
		});
		assert_view("-a", |_, v| matches!(v, View::Negate(_)));
		assert_view("a == b", |_, v| {
			matches!(v, View::Equality { negated: false, .. })
		});
		assert_view("a != b", |_, v| {
			matches!(v, View::Equality { negated: true, .. })
		});
		assert_view("0 <= i < n", |_, v| {
			matches!(v, View::Comparison { operands, operators }
				if operands.len() == 3
				&& operators.iter().collect::<Vec<_>>() == [CompareOp::LessOrEqual, CompareOp::Less])
		});
		assert_view(
			"a and b and c",
			|_, v| matches!(v, View::Logic { op: LogicOp::And, operands } if operands.len() == 3),
		);
		assert_view("a or b", |_, v| {
			matches!(
				v,
				View::Logic {
					op: LogicOp::Or,
					..
				}
			)
		});
		assert_view("not a", |_, v| matches!(v, View::Not(_)));
		assert_view("a ?? b", |_, v| matches!(v, View::Fallback { .. }));
		assert_view("a is not B", |_, v| {
			matches!(v, View::Is { negated: true, .. })
		});
		assert_view("a is in b", |_, v| {
			matches!(v, View::IsIn { negated: false, .. })
		});
	}

	fn op_is_power(view: View<'_>) -> bool {
		matches!(
			view,
			View::Arithmetic {
				op: ArithOp::Power,
				..
			}
		)
	}

	#[test]
	fn declarations() {
		assert_view("let a: Integer = 1", |t, v| {
			matches!(v, View::Declaration { kind: BindingKind::Let, name, annotation: Some(_), value: Some(_) }
				if t.name(name) == "a")
		});
		assert_view("mutable x: Integer", |_, v| {
			matches!(
				v,
				View::Declaration {
					kind: BindingKind::Mutable,
					value: None,
					..
				}
			)
		});
		assert_view("static v = 1", |_, v| {
			matches!(
				v,
				View::Declaration {
					kind: BindingKind::Static,
					..
				}
			)
		});
		assert_view("use r = xs[0]", |_, v| {
			matches!(v, View::Use { mutable: false, .. })
		});
		assert_view("mutable use r = xs[0]", |_, v| {
			matches!(v, View::Use { mutable: true, .. })
		});
		assert_view("reuse r = xs[1]", |_, v| matches!(v, View::Reuse { .. }));
		assert_view("let { a, b } = o", |t, v| {
			matches!(v, View::Destructuring { kind: BindingKind::Let, pattern, .. }
				if matches!(view(t, pattern), View::ObjectPattern { .. }))
		});
		assert_view(
			"x = 1",
			|t, v| matches!(v, View::Assignment { target, value: Some(_), .. } if text(t, target) == "x"),
		);
		assert_view("x += 1", |_, v| {
			matches!(
				v,
				View::CompoundAssignment {
					op: ArithOp::Add,
					..
				}
			)
		});
		assert_view("s .= { a = 1 }", |_, v| {
			matches!(v, View::UpdateFields { .. })
		});
		assert_view(
			"type P = { x: Integer }",
			|t, v| matches!(v, View::TypeDecl { name, .. } if t.name(name) == "P"),
		);
		assert_view("namespace N = { }", |_, v| {
			matches!(v, View::Namespace { .. })
		});
		assert_view("function f = (a: Integer): Integer => a", |t, v| {
			let View::Function { name, value, .. } = v else {
				return false;
			};
			t.name(name) == "f"
				&& matches!(view(t, value), View::Lambda { signature, .. }
					if signature.parameters.len() == 1 && signature.returns.is_some())
		});
		assert_view("export = 42", |_, v| matches!(v, View::ExportValue { .. }));
		assert_view("export function = () => 1", |_, v| {
			matches!(v, View::ExportFunction { .. })
		});
		assert_view("export type = Integer", |_, v| {
			matches!(v, View::ExportType { .. })
		});
	}

	#[test]
	fn control_flow() {
		assert_view("if a { 1 } else { 2 }", |t, v| {
			matches!(v, View::If { then_body, else_body: Some(_), compile_time: false, .. }
				if matches!(view(t, then_body), View::Block { .. }))
		});
		assert_view("when x { 1 => a\n else => b }", |_, v| {
			matches!(v, View::When { arms, .. }
				if arms.len() == 2 && arms.iter().last().unwrap().pattern.is_none())
		});
		assert_view("while a { }", |_, v| matches!(v, View::While { .. }));
		assert_view("while let v = s.pop() { }", |_, v| {
			matches!(v, View::WhileLet { .. })
		});
		assert_view("for item, i in items { }", |_, v| {
			matches!(
				v,
				View::For {
					pattern: Some(_),
					index: Some(_),
					parallel: None,
					..
				}
			)
		});
		assert_view("loop { break 1 }", |t, v| {
			let View::Loop(body) = v else { return false };
			let View::Block { statements } = view(t, body) else {
				return false;
			};
			matches!(view(t, statements.get(0).unwrap()), View::Break(Some(_)))
		});
		assert_view("do { continue }", |_, v| matches!(v, View::Do(_)));
		assert_view("return", |_, v| matches!(v, View::Return(None)));
	}

	#[test]
	fn errors_tests_patterns_and_types() {
		assert_view("try f()", |_, v| matches!(v, View::Try(_)));
		assert_view("throw e", |_, v| matches!(v, View::Throw(_)));
		assert_view(
			"f() catch error => 0",
			|t, v| matches!(v, View::Catch { error_name, .. } if t.name(error_name) == "error"),
		);
		assert_view("test \"adds\" { assert 1 == 1 }", |t, v| {
			let View::Test {
				body,
				modifier: None,
				..
			} = v
			else {
				return false;
			};
			let View::Block { statements } = view(t, body) else {
				return false;
			};
			matches!(view(t, statements.get(0).unwrap()), View::Assert(_))
		});
		assert_view("let [a, ...r] = xs", |t, v| {
			matches!(v, View::Destructuring { pattern, .. }
				if matches!(view(t, pattern), View::ArrayPattern { rest: Some(_), .. }))
		});
		assert_view("type U = A | B", |t, v| {
			matches!(v, View::TypeDecl { value, .. }
				if matches!(view(t, value), View::Union { operands } if operands.len() == 2))
		});
		assert_view("let x: Integer? = none", |t, v| {
			matches!(v, View::Declaration { annotation: Some(a), .. }
				if matches!(view(t, a), View::Optional(_)))
		});
		assert_view("type F = (a: Integer) => Integer throws E", |t, v| {
			let View::TypeDecl { value, .. } = v else {
				return false;
			};
			matches!(view(t, value), View::Lambda { .. } | View::Throws { .. })
		});
		assert_view("let { a as b, c = 1 } = o", |t, v| {
			let View::Destructuring { pattern, .. } = v else {
				return false;
			};
			let View::ObjectPattern { fields, .. } = view(t, pattern) else {
				return false;
			};
			let fields: Vec<_> = fields.iter().collect();
			fields.len() == 2 && fields[0].alias.is_some() && fields[1].default.is_some()
		});
	}

	#[test]
	fn parser_errors_are_error_views() {
		let tree = fa_parser::parse("let = = (");
		let errors = (0..tree.nodes.len())
			.filter(|&index| matches!(view(&tree, id(index)), View::Error))
			.count();
		assert!(errors > 0);
		assert!(matches!(view(&tree, NodeId(u32::MAX)), View::Error));
	}

	#[test]
	fn children_are_complete() {
		let tree = fa_parser::parse("let a = f(1, b.c) + [2, 3]\nif a { x = 1 }\n");
		let mut seen = vec![false; tree.nodes.len()];
		seen[tree.root] = true;
		for index in 0..tree.nodes.len() {
			for child in children(&tree, id(index)) {
				seen[child.index()] = true;
			}
		}
		assert!(seen.iter().all(|&seen| seen), "a node is no one's child");
	}

	#[test]
	fn hoisting_scans_only_the_block() {
		let source = "let a = 1\nfunction f = () => { function inner = () => 1\n1 }\ntype T = Integer\nnamespace N = { }\n";
		let tree = fa_parser::parse(source);
		let root = id(tree.root);
		let names = |values| {
			block_declarations(&tree, root, values)
				.iter()
				.map(|declaration| (tree.name(declaration.name).to_string(), declaration.kind))
				.collect::<Vec<_>>()
		};
		assert_eq!(
			names(false),
			[
				("f".to_string(), HoistKind::Function),
				("T".to_string(), HoistKind::Type),
				("N".to_string(), HoistKind::Namespace),
			]
		);
		assert_eq!(names(true)[0], ("a".to_string(), HoistKind::Value));
		assert_eq!(names(true).len(), 4);
	}
}
