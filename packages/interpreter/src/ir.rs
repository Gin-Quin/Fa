//! The code the interpreter runs: analyzed Fa, lowered by the analyzer.
//!
//! The analyzer turns the checked tree of a function (or of a compile-time
//! expression) into an [`Expr`] tree where every decision of the analysis is
//! explicit: names are frame slots, globals or functions; calls name their
//! callee and give their arguments in parameter order; arithmetic carries the
//! range of its result under the target (decision D49); `when` is a chain of
//! tests; built-in methods are named by their prelude path (`Array#push`).
//!
//! Functions and module-level values are named by opaque keys
//! ([`FnKey`], [`GlobalKey`]) that the [`Host`](crate::Host) resolves on
//! demand, so a function is lowered only when compile-time code reaches it.

use std::sync::Arc;

use ethnum::I256;

use crate::value::{MemberKind, Value};

/// A function, as the host names it (the analyzer: file and node).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FnKey(pub u32, pub u32);

/// A module-level value, as the host names it (the analyzer: file and node).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalKey(pub u32, pub u32);

/// A local of a function's frame.
pub type Slot = u32;

/// A lowered function.
#[derive(Clone, Debug)]
pub struct Function {
	/// Its name, for messages (`add`, `<lambda>`).
	pub name: String,
	/// Its parameters, in order: parameter `i` is bound to `params[i].slot`.
	pub params: Vec<Param>,
	/// The slots receiving the captured values of a closure, in the order of
	/// [`Expr::Closure::captures`].
	pub captures: Vec<Slot>,
	/// The number of slots of its frame.
	pub slots: u32,
	/// Its body: its value is the function's result (or `return`).
	pub body: Expr,
}

/// A parameter of a lowered function.
#[derive(Clone, Debug)]
pub struct Param {
	/// The slot it is bound to.
	pub slot: Slot,
	/// Its default value, evaluated in the callee after the previous
	/// parameters are bound, when the call omits it.
	pub default: Option<Expr>,
}

/// The kind of number an operation computes, with the range of its result
/// under the target being compiled (decision D49).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Num {
	/// An integer: a result outside `min..=max` is an overflow (`META013`).
	Int { min: I256, max: I256 },
	/// `Float` (64 bits).
	Float,
	/// `Float32`: rounded after every operation.
	Float32,
	/// `Percentage`.
	Percentage,
}

/// An arithmetic operator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArithOp {
	Add,
	Subtract,
	Multiply,
	/// `/`: always a float.
	Divide,
	/// `//`: rounds toward negative infinity.
	IntegerDivide,
	/// `%%`: the remainder of `//` (the sign of the divisor).
	Remainder,
	Power,
}

/// An ordering operator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompareOp {
	Less,
	LessOrEqual,
	Greater,
	GreaterOrEqual,
}

/// A test of the type of a value (`is`, `when`).
#[derive(Clone, Debug, PartialEq)]
pub enum TypeTest {
	/// Every value.
	Any,
	/// `none`.
	None,
	/// A boolean.
	Boolean,
	/// An integer in a range.
	Integer { min: I256, max: I256 },
	/// A float.
	Float,
	/// A string.
	String,
	/// Binary data, distinct from an array.
	Bytes,
	/// An array.
	Array,
	/// A map.
	Map,
	/// A set.
	Set,
	/// A tuple of this many items.
	Tuple(usize),
	/// An object.
	Object,
	/// A function.
	Function,
	/// A range.
	Range,
	/// This exact value (a literal type).
	Literal(Value),
	/// A member, by qualified name (`Shape.Circle`).
	Member(Arc<str>),
	/// Any error.
	Error,
	/// Any of these.
	Union(Vec<TypeTest>),
}

/// One item of a list literal or variadic argument.
#[derive(Clone, Debug)]
pub enum Item {
	/// A value.
	One(Expr),
	/// `...collection`: its items.
	Spread(Expr),
}

/// What fills one parameter at a call.
#[derive(Clone, Debug)]
pub enum Arg {
	/// A value.
	Value(Expr),
	/// A borrowed caller place, written back on return and on error.
	Borrow(Place),
	/// Omitted: the parameter's default.
	Default,
	/// The items collected by a variadic parameter.
	Variadic(Vec<Item>),
}

/// A step below the root of a place.
#[derive(Clone, Debug)]
pub enum Step {
	/// `.field`
	Field(Arc<str>),
	/// `[index]` (or `[key]` of a map).
	Index(Expr),
}

/// An assignable place: a local and the steps below it.
#[derive(Clone, Debug)]
pub struct Place {
	/// The local.
	pub slot: Slot,
	/// The fields and items below it.
	pub steps: Vec<Step>,
}

/// One part of a string template.
#[derive(Clone, Debug)]
pub enum Part {
	/// Literal text.
	Text(Arc<str>),
	/// An interpolated value (its `toString`).
	Value(Expr),
}

/// A lowered expression. Every evaluated node is one step of the budget
/// (decision D26).
#[derive(Clone, Debug)]
pub enum Expr {
	/// A constant.
	Const(Value),
	/// A local.
	Local(Slot),
	/// A module-level value (evaluated once by the host).
	Global(GlobalKey),
	/// A named function as a value.
	Function(FnKey),
	/// A lambda or local function: captures the current values of these
	/// slots (copies), received by the function's `captures`.
	Closure {
		function: FnKey,
		captures: Vec<Slot>,
	},
	/// Binds a local (a declaration): its value is `none`.
	Let(Slot, Box<Expr>),
	/// Writes a place: its value is `none`.
	Assign(Place, Box<Expr>),
	/// Reads a place (a compound assignment reads its target).
	Read(Place),
	/// Statements; the value of the last one (`none` when empty).
	Block(Vec<Expr>),
	/// `if`: the value of the branch taken (`none` without `else`).
	If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
	/// `loop { … }`: until `break`, whose value it takes.
	Loop(Box<Expr>),
	/// `while condition { … }`.
	While(Box<Expr>, Box<Expr>),
	/// `for item, index in iterable { … }`: arrays, sets, maps (tuples of
	/// key and value), ranges, strings (characters).
	For {
		item: Option<Slot>,
		index: Option<Slot>,
		iterable: Box<Expr>,
		body: Box<Expr>,
	},
	/// A direct loop over `text.toUtf8()`, encoding one scalar at a time.
	ForUtf8 {
		item: Option<Slot>,
		index: Option<Slot>,
		iterable: Box<Expr>,
		body: Box<Expr>,
	},
	/// `break`, `break value`.
	Break(Option<Box<Expr>>),
	/// `continue`.
	Continue,
	/// `return`, `return value`.
	Return(Option<Box<Expr>>),
	/// `throw error`.
	Throw(Box<Expr>),
	/// `body catch error => handler`.
	Catch {
		body: Box<Expr>,
		slot: Option<Slot>,
		handler: Box<Expr>,
	},
	/// `assert condition`: a stop when false, with the condition's text.
	Assert(Box<Expr>, Arc<str>),
	/// Arithmetic, folded left (`**` folded right by the lowering).
	Arith(ArithOp, Num, Box<Expr>, Box<Expr>),
	/// `-x`.
	Negate(Num, Box<Expr>),
	/// A conversion to a number type (`Int8(x)`, `Float(n)`).
	Convert(Num, Box<Expr>),
	/// `==` (`negated`: `!=`).
	Equals(Box<Expr>, Box<Expr>, bool),
	/// An ordering comparison.
	Compare(CompareOp, Box<Expr>, Box<Expr>),
	/// `and`, short-circuit.
	And(Box<Expr>, Box<Expr>),
	/// `or`, short-circuit.
	Or(Box<Expr>, Box<Expr>),
	/// `not x`.
	Not(Box<Expr>),
	/// `a ?? b`: `b` when `a` is `none`.
	Fallback(Box<Expr>, Box<Expr>),
	/// `value is T` (`negated`: `is not`).
	Is(Box<Expr>, TypeTest, bool),
	/// `value is in collection` (`negated`: `is not in`).
	IsIn(Box<Expr>, Box<Expr>, bool),
	/// A call of a function value.
	Call(Box<Expr>, Vec<Arg>),
	/// A direct call of a function.
	CallFunction(FnKey, Vec<Arg>),
	/// A built-in function or method that doesn't change its arguments,
	/// by its prelude path (`String#length`, `Array#map`, `Integer.parse`);
	/// a method's receiver is the first argument.
	Builtin(&'static str, Vec<Expr>),
	/// A built-in method changing its receiver (`Array#push`, `Set#add`).
	Mutate(&'static str, Place, Vec<Expr>),
	/// A list literal.
	Array(Vec<Item>),
	/// A set literal.
	Set(Vec<Item>),
	/// A map literal: tuples of key and value.
	Map(Vec<Item>),
	/// A tuple.
	Tuple(Vec<Expr>),
	/// An object, fields in declaration order.
	Object(Vec<(Arc<str>, Expr)>),
	/// A member of a named union, an enumeration or an error type.
	Member {
		name: Arc<str>,
		kind: MemberKind,
		data: Option<Box<Expr>>,
	},
	/// The data of a member (`as` in a `when` arm).
	MemberData(Box<Expr>),
	/// `value.field` (`optional`: `value?.field`, `none` on `none`).
	Field(Box<Expr>, Arc<str>, bool),
	/// `value[index]`: an item, a map value, a character, a slice.
	Index(Box<Expr>, Box<Expr>),
	/// `a..b`, `a..=b`, open bounds.
	Range(Option<Box<Expr>>, Option<Box<Expr>>, bool),
	/// A string with interpolations.
	Template(Vec<Part>),
	/// Code with an outside effect (I/O, the clock, `console`…): reaching it
	/// is `META002`.
	Impure(Arc<str>),
	/// Code the interpreter can't run yet: reaching it is an error naming it.
	Unsupported(Arc<str>),
}

impl Expr {
	/// A boxed expression.
	pub fn boxed(self) -> Box<Expr> {
		Box::new(self)
	}

	/// The `none` constant.
	pub fn none() -> Expr {
		Expr::Const(Value::None)
	}
}
