//! The per-file results of the analysis: side tables indexed by node.
//!
//! Types, resolutions, flags, symbols and diagnostics are filled by the walk
//! (`check/`): dense tables indexed by node, then sparse tables for the
//! minority node classes (arithmetic, sinks with a conversion, functions).
//! The backend-facing part is `types`, `resolutions`, `flags`, `arith`,
//! `coercions` and `symbols` (design §2.6), read through the accessors.
//!
//! # What the backends read
//!
//! - [`Analysis::type_of`]: the type of every expression node. A node flagged
//!   [`NodeFlags::TYPE_EXPR`] is a type written in the source (an annotation,
//!   the callee of a conversion `Int8(x)`): its type is the type it denotes.
//! - [`Analysis::resolution`]: what a name refers to. A local symbol's
//!   declaration is `symbols.symbol(id).decl_node`; a parameter is a
//!   [`SymbolKind::Parameter`](crate::symbols::SymbolKind) whose `decl_node`
//!   is the parameter node; the name of a named argument (`speed` in
//!   `move(1, 2, speed = 3)`) resolves to the parameter it fills.
//! - [`Analysis::flags_of`]: the runtime checks to emit (overflow, sinks,
//!   divisors, conversions…).
//! - [`Analysis::arith`] (an [`ArithInfo`] by value) and
//!   [`Analysis::interval`]: the interval, compute type and root bit of an
//!   integer node. The tables store interned [`IntervalId`]s
//!   (`interval_values`); read them through these accessors.
//! - [`Analysis::function`]: per function declaration and per lambda, its
//!   signature, parameters and body ([`FnInfo`]).
//! - [`Analysis::call`]: per call, what it calls and which argument fills
//!   which parameter, defaults and variadic parts included ([`CallInfo`]):
//!   named arguments are already put in parameter order; the receiver of a
//!   method call is [`Argument::Receiver`].
//! - [`Analysis::type_decl`]: per `type` declaration, its fields in
//!   declaration order (the property order of its JavaScript objects) with
//!   their defaults, and its functions ([`TypeDecl`]).
//! - [`Analysis::object`]: per object literal, the type built and every
//!   field in declaration order: given, default or `none` ([`ObjectInfo`]).
//! - Named unions: a member's tag is its index in the nominal type
//!   (`TypeKind::Member(nominal, index)`), its data `TypeStore::payload`; a
//!   member built with its data is a call with `Callee::Member`.
//! - `coercions`: also the object member of an anonymous union a value is
//!   stored as.
//! - Generics: [`Analysis::instantiation`] gives, per call of a generic
//!   function (`FnInfo::generics` order), per generic application
//!   `sum(Float)`, per typed literal or member construction of a generic type
//!   (`TypeDecl::generics` order), and per generic function value given where
//!   a function type is expected (`names.map(identity)`, on the name node),
//!   the types its parameters are bound to. They may contain the
//!   `TypeKind::Param` of an enclosing generic declaration: a backend
//!   monomorphizing substitutes along the chain of calls. The signature of a
//!   generic function uses `TypeKind::Param`; `TypeStore::substitute` with
//!   `Bindings` gives an instance. A generic built-in method (`Array#map`)
//!   has its own parameters (owned by the `prelude` module).
//! - A function value given for a callback type may declare fewer parameters
//!   than the callback (`FnSig::ignorable`, the `index` of collection
//!   callbacks).
//! - The number API is `Resolution::Builtin` on the member node, recognized
//!   by its path (`builtins/numbers.rs`): methods `Integer#…` (every integer
//!   type: `toString`, `abs`, `clamp`, `bitAnd`/`bitOr`/`bitXor`/`bitNot`,
//!   `shiftLeft`/`shiftRight`, `wrapping…`/`saturating…`/`checked…`
//!   `Add`/`Sub`/`Mul`/`Pow`), `Float#…` (`Float` and `Float32`: `round`
//!   halves away from zero, `floor`, `ceil`, `truncate`, `sqrt`, `abs`,
//!   `clamp`, `isNaN`, `isFinite`, `toString`), `BigInt#…`, `Percentage#…`;
//!   their result has the receiver's type (its storage type for an integer).
//!   `x.abs()` on an integer is an arithmetic node (`arith`, overflow bits).
//!   Constants are values: `Int8.minValue`… have a literal type in their
//!   storage (the value is the type's), `Float.nan`, `Float.infinity`,
//!   `Float.maxValue`, `Float.epsilon` (and `Float32.…`) are `Float`s.
//!   `Integer.round(x)` / `floor` / `ceil` / `truncate` convert a float and
//!   throw `ArithmeticError` (NaN, infinite, out of range) unless the analyzer
//!   proved them (a literal that fits): `CONVERSION_CHECK` + `CAN_FAIL` when
//!   they can fail. A shift has `SHIFT_CHECK` + `CAN_FAIL` when its amount is
//!   not proven in `[0, width)`. `Math.max(a, b)` / `Math.min(a, b)` are calls
//!   with `Callee::Builtin`, two `Argument::Given`, the result has the shared
//!   kind (the base integer type, `interval` when narrower). `Float.parse`
//!   throws `ParseError`.
//! - `Array.from(source, capacity = n)` / `Array(T).from(…)`: a call with
//!   `Callee::Builtin` (`Array.from`) and the arguments `[source, capacity]`
//!   (`Argument::Default` without capacity); it copies the source's items
//!   into a new array of the call's type; `capacity` is a hint.
//! - Tuples: `TypeKind::Tuple`; a tuple value is `View::Tuple` (in a group);
//!   `t[0]` is an `Index` whose index is an integer literal (in range,
//!   checked), never `INDEX_CHECK`. Equality is element-wise.
//! - Ranges: `TypeId::RANGE` values (`a..b`, `..=`, open bounds); the fields
//!   are `Range#start`, `Range#end` (`Integer?`) and `Range#includesEnd`,
//!   readable and assignable on a `mutable` range. `for i in range` over a
//!   `Range` value (not a literal) iterates from `start` (a missing start
//!   stops the program) by one. `items[range]` / `text[range]` is a slice (an
//!   `Index` whose index has the type `Range`): the array type / `String`,
//!   `INDEX_CHECK` + `CAN_FAIL` (`IndexError`) unless proven; bounds go from
//!   `-length` to `length`, missing ones are the ends, an inverted range is
//!   empty. `x is in range` (a `Range` value) is a membership test.
//! - Destructuring ([`Analysis::pattern`]): per pattern node of a `let` /
//!   `mutable` declaration, a `for` variable or a destructured parameter.
//!   A destructured parameter is a `FnParam` whose node is the
//!   `DestructuredParameter` and whose symbol is anonymous (named by the
//!   pattern's text): bind its pattern from that parameter.
//! - Effects ([`crate::effects`]): `FnSig::effects` of a named function or
//!   method holds its `mutates` / `invalidates` paths, declared or inferred
//!   (`TypeStore::effects`, `effects::path_text`); a lambda's type carries
//!   only the effects written on it. Effects need no code: a path root
//!   `PathRoot::Param(i)` indexes `FnInfo::params` (`self` included,
//!   `PathRoot::SelfParam` for `self`). The LLVM backend may read them for
//!   attributes: a pointer parameter that is the root of no path is
//!   read-only during the call, and two parameters never alias (the alias
//!   rule, `REF013`).
//! - References: `use x = place` and `mutable use x = place` are
//!   `View::Use` nodes declaring a symbol of kind `Use` / `MutableUse` whose
//!   `decl_node` is that node; its value (flag `PLACE`, through a `try`) is
//!   the place the symbol designates. Reading or writing `x` reads or
//!   writes that place, evaluated once at the declaration (advanced brief
//!   §6.6: a JavaScript object is its own reference; a primitive place is a
//!   container and a key). `reuse x = place` (`View::Reuse`, its node
//!   resolves to the `mutable use` symbol) designates a new place from
//!   there on; `reuse x = none` an absent one.
//! - A `for` variable over a collection (`SymbolKind::LoopVariable { copy:
//!   false }`) designates the current item: an assignment to it (`slot =
//!   sum`, `score += 1`) or below it (`body.vx -= …`) writes the item in the
//!   collection (JavaScript: an index loop over a collection of primitives).
//!   The collection's length can't change during the loop (`REF012`).
//! - Requirements: `FnSig::requires` lists a function's requirements
//!   (declared, and inferred for a single-expression function), sorted by
//!   the stable hash of their key type (decision D11): one hidden
//!   parameter each, after the regular ones, in that order. Inside the
//!   body, a requirement is a symbol of kind `Requirement { index }`.
//!   [`Analysis::requirement_args`] gives, per call, what fills each one
//!   ([`RequirementArg`]): the value of a `provide` statement
//!   (`View::Provide`: an alias of a place, or a temporary evaluated once
//!   at the statement), or the caller's own requirement of that key.
//! - Closures: `FnInfo::captures` lists the locals of enclosing functions a
//!   lambda or a local `function` uses; [`Analysis::escapes`] the ones that
//!   escape (given to a method that stores them, assigned to a place,
//!   returned; not yet: a field of an object literal): those copy their
//!   captures when they are created; the others use the variables
//!   themselves (they run before their creator returns).

use bitflags::bitflags;
use rustc_hash::FxHashMap;

use crate::diagnostics::Diagnostic;
use crate::ids::{
	BuiltinId, FileId, IntervalId, ListId, NodeId, ParamId, ScopeId, SigId, Sym, SymbolId, TypeId,
};
use crate::intern::Interner;
use crate::symbols::{Resolution, Symbol, SymbolTable};
use crate::types::{ComputeType, FieldFlags, Interval};

bitflags! {
	/// Per-node facts for the backends (design §2.6).
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
	pub struct NodeFlags: u32 {
		/// The node may exceed the native compute type (values brief §6.7).
		const OVERFLOW_NATIVE = 1 << 0;
		/// A number-represented node may exceed ±(2^53 − 1).
		const OVERFLOW_JS = 1 << 1;
		/// The root value must be range-checked against its sink type.
		const SINK_CHECK = 1 << 2;
		/// The divisor is not proven non-zero: `DivisionByZero` path.
		const DIVISOR_CHECK = 1 << 3;
		/// The index or slice is not proven: `IndexError` path.
		const INDEX_CHECK = 1 << 4;
		/// The conversion is not proven to fit: `ArithmeticError` path.
		const CONVERSION_CHECK = 1 << 5;
		/// The exponent is not proven non-negative.
		const EXPONENT_CHECK = 1 << 6;
		/// The shift amount is not proven in `[0, width)`.
		const SHIFT_CHECK = 1 << 7;
		/// `INT_MIN // -1` and `INT_MIN %% -1` guards are needed (values brief D-49).
		const MIN_DIV_GUARD = 1 << 8;
		/// A payload access without re-testing the tag.
		const NARROWED = 1 << 9;
		/// Unreachable code.
		const UNREACHABLE = 1 << 10;
		/// A type expression: its type is the type it denotes.
		const TYPE_EXPR = 1 << 11;
		/// A constant folded by the analyzer.
		const CONST_FOLDED = 1 << 12;
		/// A place (assignment target, `use`, effect argument).
		const PLACE = 1 << 13;
		/// The read of a local (a name) is its last use, in a copy position
		/// (the value of a `let`/`mutable` or an assignment, a returned
		/// value, an item given to `push`/`add`): the value can be moved
		/// instead of copied (native: no copy and no drop of the source;
		/// JavaScript: no copy, D15). Only for values that a copy would
		/// duplicate (objects, collections, strings, unions holding them).
		const MOVE = 1 << 14;
		/// The expression can fail with an error.
		const CAN_FAIL = 1 << 15;
	}
}

impl NodeFlags {
	/// The flags a `-- no-check` annotation forbids: every runtime check.
	pub const CHECKS: NodeFlags = NodeFlags::OVERFLOW_NATIVE
		.union(NodeFlags::OVERFLOW_JS)
		.union(NodeFlags::SINK_CHECK)
		.union(NodeFlags::DIVISOR_CHECK)
		.union(NodeFlags::INDEX_CHECK)
		.union(NodeFlags::CONVERSION_CHECK)
		.union(NodeFlags::EXPONENT_CHECK)
		.union(NodeFlags::SHIFT_CHECK);
}

/// The analysis of one file.
pub struct Analysis {
	/// The analyzed file.
	pub file: FileId,
	/// The type of every node (`UNSET` for nodes without one).
	pub types: Vec<TypeId>,
	/// What every name or member node refers to (`Unresolved` for other nodes).
	pub resolutions: Vec<Resolution>,
	/// The flags of every node.
	pub flags: Vec<NodeFlags>,
	/// The symbols and the scope arena of the file.
	pub symbols: SymbolTable,
	/// Parse diagnostics first, then the analysis' own, sorted by span.
	pub diagnostics: Vec<Diagnostic>,

	// ── sparse tables (design §2.6): only the nodes of a minority class ──
	/// Every integer arithmetic node (`+ - * // %% **`, unary `-`, compound
	/// assignment) and every range-checked sink: its interval (an id into
	/// `interval_values`) and compute type. Read it with [`Analysis::arith`].
	pub arith: FxHashMap<NodeId, ArithEntry>,
	/// The implicit conversion of a value at its sink, by value node: the
	/// target type of a union injection, `Integer` ↔ `Int64`, `Float32` →
	/// `Float`, `Character` → `String`, and of a `SINK_CHECK` range check.
	pub coercions: FxHashMap<NodeId, TypeId>,
	/// Integer nodes whose flow fact is narrower than their type (hover,
	/// proofs): an id into `interval_values`. Read it with [`Analysis::interval`].
	pub intervals: FxHashMap<NodeId, IntervalId>,
	/// The intervals of `arith` and `intervals`, interned (most nodes share
	/// a few intervals: 4 bytes per node instead of 64).
	pub interval_values: Interner<Interval>,
	/// The type arguments of a generic call, typed literal or generic function value (M2).
	pub instantiations: FxHashMap<NodeId, ListId>,
	/// Every function declaration and lambda node (both keys for a
	/// declaration whose value is a lambda).
	pub functions: FxHashMap<NodeId, FnInfo>,
	/// Every call node: its callee and its arguments in parameter order.
	pub calls: FxHashMap<NodeId, CallInfo>,
	/// Per call of a function with requirements: what provides each one, in
	/// the order of the callee's `FnSig::requires` (sorted by the stable
	/// hash of their keys, decision D11): the hidden arguments, after the
	/// regular ones.
	pub requirement_args: FxHashMap<NodeId, Vec<RequirementArg>>,
	/// The closures that escape (lambda nodes; for a local `function`, its
	/// lambda): stored in a collection, returned. They copy the values they
	/// capture (`FnInfo::captures`) when created; the others see the
	/// captured variables themselves (advanced brief §5.11, D3).
	pub escapes: rustc_hash::FxHashSet<NodeId>,
	/// Every `type` declaration node: its type, fields in declaration order,
	/// functions.
	pub type_decls: FxHashMap<NodeId, TypeDecl>,
	/// Every object literal node (typed or not, and named-union members built
	/// with braces): its type and its fields in declaration order.
	pub objects: FxHashMap<NodeId, ObjectInfo>,
	/// Every destructuring pattern node (`ObjectPattern`, `ArrayPattern`,
	/// `TuplePattern`) of a declaration, a `for` or a parameter: what each
	/// element reads and binds ([`PatternInfo`]).
	pub patterns: FxHashMap<NodeId, PatternInfo>,
}

/// A destructuring pattern (values brief §16): `let { a, b as c } = o`,
/// `let [first, second] = items`, `let (x, y) = pair`, nested patterns,
/// `for (key, value) in …`, `({ x, y }: Point) => …`.
///
/// The value is evaluated once; the elements are read in order. An
/// `ArrayPattern` whose length isn't proven has `INDEX_CHECK` and `CAN_FAIL`
/// (`IndexError { index = items - 1, length }` after one length check).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternInfo {
	/// The type of the destructured value.
	pub ty: TypeId,
	/// The elements, in source order: the fields of an object pattern, the
	/// items of an array or tuple pattern (their index is their position).
	pub elements: Vec<PatternElement>,
	/// `...rest`: its symbol (`decl_node`: the rest name node) and its type
	/// (an object type without the named fields, or an `Array` of the rest).
	pub rest: Option<(SymbolId, TypeId)>,
}

/// One element of a destructuring pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternElement {
	/// The field read (object patterns), `None` for an item (by position).
	pub field: Option<Sym>,
	/// What the element binds.
	pub binding: PatternBinding,
	/// The type of the element as read from the value (the field type, the
	/// item type); `UNKNOWN` for a field absent from the value's type.
	pub ty: TypeId,
	/// The default value (`d = 4`): used when the field holds `none`, or always
	/// when the field is absent from the type (`absent`).
	pub default: Option<NodeId>,
	/// The field is not in the value's type: the binding is the default.
	pub absent: bool,
}

/// What an element of a pattern binds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatternBinding {
	/// A name: its symbol (for an object pattern field, `decl_node` is the
	/// pattern node; for an item, the name node).
	Symbol(SymbolId),
	/// A nested pattern node, itself in [`Analysis::patterns`].
	Nested(NodeId),
	/// Nothing (an invalid element, already reported).
	None,
}

/// A field of an object type, as declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDecl {
	/// The field node (`x: Integer = 0`, `readonly y: T`).
	pub node: NodeId,
	/// The name (`#secret` for a private field).
	pub name: Sym,
	/// The type.
	pub ty: TypeId,
	/// The default value, evaluated at each creation that omits the field.
	pub default: Option<NodeId>,
	/// `readonly`, `#private`.
	pub flags: FieldFlags,
}

/// A `type` declaration (design §2.6): what the backends need to build and
/// print its values, and to call its functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
	/// Its symbol.
	pub symbol: SymbolId,
	/// The declared type: an object type, a named union, an enumeration, or
	/// the aliased type.
	pub ty: TypeId,
	/// The fields of an object type, in declaration order (the order of the
	/// properties of its JavaScript objects, `design/decisions.md`).
	pub fields: Vec<FieldDecl>,
	/// The functions declared in the type: methods (first parameter `self`)
	/// and associated functions, in declaration order.
	pub functions: Vec<SymbolId>,
	/// The scope of its functions, when it has a body.
	pub scope: Option<ScopeId>,
	/// Its generic parameters (`type Point(NumberType) = …`), in order: an
	/// application `Point(Float)` is the type substituted.
	pub generics: Vec<ParamId>,
}

/// What fills a field of an object literal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
	/// The value written in the literal.
	Given(NodeId),
	/// Omitted: the default value of the field's declaration.
	Default(NodeId),
	/// Omitted, optional without default: `none`.
	None,
}

/// An object literal (design §2.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
	/// The object type built (for a named-union member, its data type).
	pub ty: TypeId,
	/// The declaration it is built from, when there is one.
	pub decl: Option<NodeId>,
	/// The member of a named union, when built as `Shape.Circle { … }`.
	pub member: Option<TypeId>,
	/// Every field, in declaration order (literal order for an anonymous type).
	pub fields: Vec<(Sym, FieldValue)>,
}

/// The stored form of [`ArithInfo`]: its interval is interned.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ArithEntry {
	/// The values the node may take (`Analysis::interval_values`).
	pub interval: IntervalId,
	/// The type the tree is computed in.
	pub compute: ComputeType,
	/// The root of its arithmetic tree.
	pub root: bool,
}

/// The arithmetic facts of an integer node, for the backends (design §3.6),
/// as [`Analysis::arith`] gives them.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ArithInfo {
	/// The values the node may take, from the stable loop round. Kept by
	/// value: the fact store of the walk doesn't outlive it.
	pub interval: Interval,
	/// The type the tree is computed in (values brief §6.7).
	pub compute: ComputeType,
	/// The root of its arithmetic tree: its parent is not an arithmetic node
	/// of the same tree.
	pub root: bool,
}

/// What the backends need of a function or a lambda node.
#[derive(Clone, Debug, PartialEq)]
pub struct FnInfo {
	/// The function type: declared, or inferred for a single-expression body;
	/// `UNKNOWN` when its signature couldn't be resolved.
	pub ty: TypeId,
	/// Its signature: parameter types, return type (declared, or inferred from
	/// a single-expression body, or `Void` for a block without return type),
	/// `throws` (`NEVER`: none), effects (empty in M1). `None` with `UNKNOWN`.
	pub sig: Option<SigId>,
	/// The symbol of a named function (`None` for a lambda).
	pub symbol: Option<SymbolId>,
	/// The `function name = …` node, `None` for a lambda.
	pub declaration: Option<NodeId>,
	/// The lambda node (`ArrowFunction`) holding the signature and the body.
	pub lambda: NodeId,
	/// The parameters, in order.
	pub params: Vec<FnParam>,
	/// The body: a `Block` node, or an expression.
	pub body: NodeId,
	/// Whether the body is a block (its value is its last expression, or `return`).
	pub block_body: bool,
	/// Whether the return type is written in the source.
	pub declared_return: bool,
	/// The scope of the parameters, shared with the top-level declarations of a block body.
	pub scope: ScopeId,
	/// The errors the body may throw: the declared ones, or the inferred ones
	/// for an entry function, a test or a lambda (`NEVER` for none).
	pub throws: TypeId,
	/// The local symbols of enclosing functions it reads or writes, in order
	/// of first use: by reference while it doesn't escape, copied at its
	/// creation when it does (`Analysis::escapes`). Module-level values are
	/// never captured.
	pub captures: Vec<SymbolId>,
	/// Its generic parameters (`function first(Item) = …`), in order: the
	/// `TypeKind::Param` its signature uses. A call records the types they
	/// are bound to in `instantiations`, in this order.
	pub generics: Vec<ParamId>,
}

/// A parameter of a function or lambda.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnParam {
	/// The parameter node (`a: Integer = 1`, `...values: Integer`, `x`).
	pub node: NodeId,
	/// Its symbol (kind `Parameter`); its `ty` is the type seen by the body
	/// (`Array(T)` for a variadic parameter).
	pub symbol: SymbolId,
	/// The default value, evaluated in the callee at each call that omits the
	/// argument, after the previous parameters are bound (it can read them).
	pub default: Option<NodeId>,
	/// `...values: T`: collects the remaining positional arguments into an `Array(T)`.
	pub variadic: bool,
}

/// What a call calls.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Callee {
	/// A function declaration of this file (a direct call): the `function` node.
	/// Named arguments and defaults only exist for these.
	Function(NodeId),
	/// A built-in function of the prelude (`console.log`).
	Builtin(BuiltinId),
	/// A conversion to a number type (`Int8(x)`, `Float(n)`): the target type.
	Conversion(TypeId),
	/// Any other value of a function type (positional arguments only).
	Value,
	/// A member of a named union built with its data, `Shape.Labelled("north")`:
	/// the member type; the one argument is the data.
	Member(TypeId),
}

/// What fills one parameter at a call.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Argument {
	/// The value node of the argument (for a named argument, its value, not the
	/// `name = value` node).
	Given(NodeId),
	/// Not given: the callee evaluates the parameter's default value.
	Default,
	/// The positional arguments collected by a variadic parameter, in order: a
	/// `Spread` node (`...numbers`) spreads a collection.
	Variadic(Vec<NodeId>),
	/// Missing (already reported).
	Missing,
	/// The receiver of a method call `value.method(…)` (the `self`
	/// parameter): the callee `Access` node without its last member.
	Receiver(NodeId),
}

/// What provides a requirement at a call (advanced brief §9.3, §9.7).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RequirementArg {
	/// The value of this `provide` statement: an alias of its place when the
	/// value is a place (native: its address; JavaScript: the value), else
	/// the temporary it evaluated once, owned by its block.
	Provided(NodeId),
	/// The enclosing function's own requirement of this key: its hidden
	/// parameter at the key's position in its `FnSig::requires`.
	Own(TypeId),
	/// Not provided (reported, `REQ001`).
	Missing,
}

/// A call (design §2.6): its callee and its arguments in parameter order.
#[derive(Clone, Debug)]
pub struct CallInfo {
	/// What is called.
	pub callee: Callee,
	/// One entry per parameter of the callee, in parameter order.
	pub arguments: Vec<Argument>,
}

impl Analysis {
	/// An empty analysis of a file with `nodes` nodes.
	pub fn new(file: FileId, nodes: usize) -> Self {
		Self {
			file,
			types: vec![TypeId::UNSET; nodes],
			resolutions: vec![Resolution::Unresolved; nodes],
			flags: vec![NodeFlags::empty(); nodes],
			symbols: SymbolTable::new(),
			diagnostics: Vec::new(),
			arith: FxHashMap::default(),
			coercions: FxHashMap::default(),
			intervals: FxHashMap::default(),
			interval_values: Interner::new(),
			instantiations: FxHashMap::default(),
			functions: FxHashMap::default(),
			calls: FxHashMap::default(),
			escapes: rustc_hash::FxHashSet::default(),
			requirement_args: FxHashMap::default(),
			type_decls: FxHashMap::default(),
			objects: FxHashMap::default(),
			patterns: FxHashMap::default(),
		}
	}

	/// A destructuring pattern: what its elements read and bind.
	pub fn pattern(&self, node: NodeId) -> Option<&PatternInfo> {
		self.patterns.get(&node)
	}

	/// The type of a node.
	pub fn type_of(&self, node: NodeId) -> TypeId {
		self.types
			.get(node.index())
			.copied()
			.unwrap_or(TypeId::UNSET)
	}

	/// What a name or member node refers to.
	pub fn resolution(&self, node: NodeId) -> Resolution {
		self.resolutions
			.get(node.index())
			.copied()
			.unwrap_or_default()
	}

	/// The flags of a node.
	pub fn flags_of(&self, node: NodeId) -> NodeFlags {
		self.flags.get(node.index()).copied().unwrap_or_default()
	}

	/// Whether a node has all these flags.
	pub fn has_flags(&self, node: NodeId, flags: NodeFlags) -> bool {
		self.flags_of(node).contains(flags)
	}

	/// The symbols and scopes of the file.
	pub fn symbols(&self) -> &SymbolTable {
		&self.symbols
	}

	/// The local symbol a name refers to (`None` for the prelude, other
	/// modules, fields…).
	pub fn symbol_of(&self, node: NodeId) -> Option<&Symbol> {
		match self.resolution(node) {
			Resolution::Local(symbol) => Some(self.symbols.symbol(symbol)),
			_ => None,
		}
	}

	/// The symbol a declaration node declares (`let`, `function`, `type`…).
	pub fn declared_by(&self, node: NodeId) -> Option<&Symbol> {
		self.symbols
			.symbol_of_decl(node)
			.map(|symbol| self.symbols.symbol(symbol))
	}

	/// The arithmetic facts of an integer arithmetic node or range-checked sink.
	pub fn arith(&self, node: NodeId) -> Option<ArithInfo> {
		let entry = self.arith.get(&node)?;
		Some(ArithInfo {
			interval: self.interval_value(entry.interval),
			compute: entry.compute,
			root: entry.root,
		})
	}

	/// An interval of `interval_values`.
	pub fn interval_value(&self, id: IntervalId) -> Interval {
		*self.interval_values.resolve(id.0)
	}

	/// Interns an interval into `interval_values`.
	pub(crate) fn interval_id(&mut self, interval: Interval) -> IntervalId {
		IntervalId(self.interval_values.intern(interval))
	}

	/// Records the arithmetic facts of a node.
	pub(crate) fn set_arith(&mut self, node: NodeId, info: ArithInfo) {
		let interval = self.interval_id(info.interval);
		self.arith.insert(
			node,
			ArithEntry {
				interval,
				compute: info.compute,
				root: info.root,
			},
		);
	}

	/// Records the interval of an integer node.
	pub(crate) fn set_interval(&mut self, node: NodeId, interval: Interval) {
		let id = self.interval_id(interval);
		self.intervals.insert(node, id);
	}

	/// The implicit conversion of a value at its sink: the type it is converted to.
	pub fn coercion(&self, node: NodeId) -> Option<TypeId> {
		self.coercions.get(&node).copied()
	}

	/// The interval of an integer node, when narrower than its type.
	pub fn interval(&self, node: NodeId) -> Option<Interval> {
		self.intervals.get(&node).map(|&id| self.interval_value(id))
	}

	/// The type arguments of a generic call or literal (M2).
	pub fn instantiation(&self, node: NodeId) -> Option<ListId> {
		self.instantiations.get(&node).copied()
	}

	/// The information of a function declaration or lambda node.
	pub fn function(&self, node: NodeId) -> Option<&FnInfo> {
		self.functions.get(&node)
	}

	/// The `type` declaration at a node.
	pub fn type_decl(&self, node: NodeId) -> Option<&TypeDecl> {
		self.type_decls.get(&node)
	}

	/// The object literal at a node.
	pub fn object(&self, node: NodeId) -> Option<&ObjectInfo> {
		self.objects.get(&node)
	}

	/// The call at a node: its callee and its arguments in parameter order.
	pub fn call(&self, node: NodeId) -> Option<&CallInfo> {
		self.calls.get(&node)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flags_by_name() {
		assert_eq!(
			NodeFlags::from_name("SINK_CHECK"),
			Some(NodeFlags::SINK_CHECK)
		);
		assert_eq!(
			NodeFlags::from_name("OVERFLOW_NATIVE"),
			Some(NodeFlags::OVERFLOW_NATIVE)
		);
		assert_eq!(NodeFlags::from_name("NOPE"), None);
		assert!(NodeFlags::CHECKS.contains(NodeFlags::DIVISOR_CHECK));
		assert!(!NodeFlags::CHECKS.contains(NodeFlags::NARROWED));
	}

	#[test]
	fn out_of_range_nodes_have_no_type() {
		let analysis = Analysis::new(FileId(0), 2);
		assert_eq!(analysis.type_of(NodeId(5)), TypeId::UNSET);
		assert_eq!(analysis.flags_of(NodeId(1)), NodeFlags::empty());
		assert!(analysis.arith(NodeId(1)).is_none());
		assert!(analysis.coercion(NodeId(1)).is_none());
		assert!(analysis.function(NodeId(1)).is_none());
		assert!(analysis.symbol_of(NodeId(1)).is_none());
	}

	#[test]
	fn accessors_read_the_side_tables_of_a_walk() {
		let (session, file) = crate::analyze_source("let answer = 42\nconsole.log(answer)\n");
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let read = (0..tree.nodes.len())
			.rev()
			.find(|&node| tree.text(node) == "answer")
			.map(NodeId::new)
			.unwrap();
		let symbol = analysis.symbol_of(read).unwrap();
		assert_eq!(session.types().syms.resolve(symbol.name), "answer");
		assert!(analysis.declared_by(NodeId::new(tree.root)).is_none());
		let declared = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| analysis.declared_by(node).is_some())
			.unwrap();
		assert_eq!(analysis.declared_by(declared).unwrap().ty, TypeId::INTEGER);
		assert_eq!(analysis.symbols().symbols().len(), 1);
		assert!(!analysis.has_flags(read, NodeFlags::TYPE_EXPR));
	}
}
