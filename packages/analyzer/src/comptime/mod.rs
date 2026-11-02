//! Compile-time evaluation (advanced brief §12): what the analysis runs on
//! the interpreter (`fa-interpreter`) during its walk.
//!
//! - [`lower`]: the checked tree to the interpreter's code.
//! - [`State`]: per session, the target being compiled (the analysis is
//!   parameterized by it, D22), the lowered functions and the values of
//!   module-level values (memoized), the purity of each function.
//! - [`exact_type`]: the exact compile-time type of a value (the type of a
//!   `static`, of `@parseJson`…).
//! - [`inputs`]: the files compile-time code reads (`@readFile`,
//!   `@readDirectory`), inside the root, recorded as build inputs.
//!
//! The walk's handlers are in `check/comptime.rs`.

pub mod host;
pub mod inputs;
pub mod lower;
pub mod schema;

use std::sync::Arc;

use rustc_hash::FxHashMap;

use fa_interpreter::ir::{Expr, FnKey, Function, GlobalKey, Item, Part, Place, Step};
use fa_interpreter::{Error, Value};

use crate::ids::TypeId;
use crate::types::{FieldEntry, FieldFlags, FloatBits, TypeStore};

/// The targets of `architecture/packages`.
pub const TARGETS: [&str; 7] = ["web", "bun", "node", "deno", "native", "wasm", "rust"];

/// Whether a target runs JavaScript (its numbers are JavaScript's, D49).
pub fn is_js_target(target: &str) -> bool {
	matches!(target, "web" | "bun" | "node" | "deno")
}

/// The value of a module-level value, while and after it is evaluated.
#[derive(Clone, Debug)]
pub enum GlobalState {
	/// Being evaluated: reading it again is a cycle.
	InProgress,
	/// Evaluated.
	Done(Result<Value, Error>),
}

/// The compile-time state of a session.
#[derive(Debug)]
pub struct State {
	/// The target the analysis is for (`bun` by default): the value of
	/// `target`, and the arithmetic of compile-time code (D49).
	pub target: String,
	/// The merged configuration of the package for the target: the value of
	/// `configuration` (none for a standalone file).
	pub configuration: Option<Value>,
	/// The lowered functions, by key.
	pub functions: FxHashMap<FnKey, Arc<Function>>,
	/// The values of module-level values, by key.
	pub globals: FxHashMap<GlobalKey, GlobalState>,
	/// The impure primitive each function may reach (`None`: pure).
	pub purity: FxHashMap<FnKey, Option<String>>,
}

impl Default for State {
	fn default() -> Self {
		Self {
			target: "bun".to_string(),
			configuration: None,
			functions: FxHashMap::default(),
			globals: FxHashMap::default(),
			purity: FxHashMap::default(),
		}
	}
}

/// The exact compile-time type of a value (advanced brief §12.2): a literal
/// type for a scalar, an object type of exact fields, an array of the union
/// of its items (widened beyond a few distinct items), a tuple of exact items.
pub fn exact_type(types: &mut TypeStore, value: &Value) -> TypeId {
	exact(types, value, 0)
}

fn exact(types: &mut TypeStore, value: &Value, depth: u32) -> TypeId {
	if depth > 32 {
		return TypeId::UNKNOWN;
	}
	match value {
		Value::None => TypeId::NONE,
		Value::Boolean(value) => types.bool_literal(*value),
		Value::Integer(value) => types.int_literal(*value),
		Value::Float(value) => types.float_literal(FloatBits::F64, *value),
		Value::Percentage(_) => TypeId::PERCENTAGE,
		// a long text is not interned twice (its literal type would copy it)
		Value::String(text) if text.len() > MAX_LITERAL_TEXT => TypeId::STRING,
		Value::String(text) => types.string_literal(text),
		Value::Array(items) => {
			let item = join(types, items, depth);
			types.array(item)
		}
		Value::Set(items) => {
			let item = join(types, items, depth);
			types.collection(crate::types::CollKind::Set, item, TypeId::NONE)
		}
		Value::Map(entries) => {
			let keys: Vec<Value> = entries.iter().map(|(key, _)| key.clone()).collect();
			let values: Vec<Value> = entries.iter().map(|(_, value)| value.clone()).collect();
			let key = join(types, &keys, depth);
			let value = join(types, &values, depth);
			types.collection(crate::types::CollKind::Map, value, key)
		}
		Value::Tuple(items) => {
			let items: Vec<TypeId> = items
				.iter()
				.map(|item| exact(types, item, depth + 1))
				.collect();
			types.tuple(&items)
		}
		Value::Object(object) => {
			let mut fields: Vec<FieldEntry> = object
				.fields
				.iter()
				.map(|(name, value)| FieldEntry {
					name: types.syms.intern(name),
					ty: exact(types, value, depth + 1),
					flags: FieldFlags::empty(),
				})
				.collect();
			fields.sort_by(|a, b| types.syms.resolve(a.name).cmp(types.syms.resolve(b.name)));
			types.object(&fields, None)
		}
		Value::Range { .. } => TypeId::RANGE,
		Value::Member(_) | Value::Function(_) => TypeId::UNKNOWN,
	}
}

/// The longest text whose exact type is its literal type (a longer one is a `String`).
const MAX_LITERAL_TEXT: usize = 64 << 10;

/// The union of the exact types of items; their widened types beyond 8.
fn join(types: &mut TypeStore, items: &[Value], depth: u32) -> TypeId {
	if items.is_empty() {
		return TypeId::NEVER;
	}
	let mut members: Vec<TypeId> = Vec::new();
	let mut widened = false;
	for item in items {
		// once widened, the literal types of the other items are not interned
		// (an array of a million numbers would add a million types)
		let ty = if widened {
			widened_exact(types, item, depth + 1)
		} else {
			exact(types, item, depth + 1)
		};
		if !members.contains(&ty) {
			members.push(ty);
			if !widened && members.len() > 8 {
				widened = true;
				let mut kept: Vec<TypeId> = Vec::new();
				for member in members {
					let ty = types.widen(member);
					if !kept.contains(&ty) {
						kept.push(ty);
					}
				}
				members = kept;
			}
		}
	}
	if members.len() == 1 {
		return members[0];
	}
	types.union(&members)
}

/// The widened exact type of a value, without interning its literal type.
fn widened_exact(types: &mut TypeStore, value: &Value, depth: u32) -> TypeId {
	match value {
		Value::Boolean(_) => TypeId::BOOLEAN,
		Value::Integer(_) => TypeId::of_storage(crate::types::IntStorage::Default),
		Value::Float(_) => TypeId::FLOAT,
		Value::String(_) => TypeId::STRING,
		other => {
			let ty = exact(types, other, depth);
			types.widen(ty)
		}
	}
}

/// The first impure primitive an expression may reach, through the
/// functions it names (`functions` lowers one; `memo` keeps each answer).
pub fn impurity(
	expr: &Expr,
	functions: &mut dyn FnMut(FnKey) -> Option<Option<String>>,
) -> Option<String> {
	let mut found = None;
	visit(expr, &mut |expr| {
		if found.is_some() {
			return;
		}
		match expr {
			Expr::Impure(what) => found = Some(what.to_string()),
			Expr::Function(key) | Expr::CallFunction(key, _) => {
				if let Some(Some(what)) = functions(*key) {
					found = Some(what);
				}
			}
			Expr::Closure { function, .. } => {
				if let Some(Some(what)) = functions(*function) {
					found = Some(what);
				}
			}
			_ => {}
		}
	});
	found
}

/// Visits every expression of a tree.
pub fn visit(expr: &Expr, f: &mut dyn FnMut(&Expr)) {
	f(expr);
	let place = |place: &Place, f: &mut dyn FnMut(&Expr)| {
		for step in &place.steps {
			if let Step::Index(index) = step {
				visit(index, f);
			}
		}
	};
	let items = |items: &[Item], f: &mut dyn FnMut(&Expr)| {
		for item in items {
			match item {
				Item::One(value) | Item::Spread(value) => visit(value, f),
			}
		}
	};
	match expr {
		Expr::Const(_)
		| Expr::Local(_)
		| Expr::Global(_)
		| Expr::Function(_)
		| Expr::Closure { .. }
		| Expr::Continue
		| Expr::Impure(_)
		| Expr::Unsupported(_) => {}
		Expr::Let(_, value)
		| Expr::Loop(value)
		| Expr::Throw(value)
		| Expr::Assert(value, _)
		| Expr::Negate(_, value)
		| Expr::Convert(_, value)
		| Expr::Not(value)
		| Expr::Is(value, ..)
		| Expr::MemberData(value)
		| Expr::Field(value, ..) => visit(value, f),
		Expr::Assign(target, value) => {
			place(target, f);
			visit(value, f);
		}
		Expr::Read(target) => place(target, f),
		Expr::Block(values) | Expr::Tuple(values) | Expr::Builtin(_, values) => {
			for value in values {
				visit(value, f);
			}
		}
		Expr::Mutate(_, target, values) => {
			place(target, f);
			for value in values {
				visit(value, f);
			}
		}
		Expr::If(condition, then, otherwise) => {
			visit(condition, f);
			visit(then, f);
			if let Some(otherwise) = otherwise {
				visit(otherwise, f);
			}
		}
		Expr::While(a, b)
		| Expr::Arith(_, _, a, b)
		| Expr::Equals(a, b, _)
		| Expr::Compare(_, a, b)
		| Expr::And(a, b)
		| Expr::Or(a, b)
		| Expr::Fallback(a, b)
		| Expr::IsIn(a, b, _)
		| Expr::Index(a, b) => {
			visit(a, f);
			visit(b, f);
		}
		Expr::For { iterable, body, .. } => {
			visit(iterable, f);
			visit(body, f);
		}
		Expr::Break(value) | Expr::Return(value) => {
			if let Some(value) = value {
				visit(value, f);
			}
		}
		Expr::Catch { body, handler, .. } => {
			visit(body, f);
			visit(handler, f);
		}
		Expr::Call(callee, arguments) => {
			visit(callee, f);
			for argument in arguments {
				visit_argument(argument, f);
			}
		}
		Expr::CallFunction(_, arguments) => {
			for argument in arguments {
				visit_argument(argument, f);
			}
		}
		Expr::Array(values) | Expr::Set(values) | Expr::Map(values) => items(values, f),
		Expr::Object(fields) => {
			for (_, value) in fields {
				visit(value, f);
			}
		}
		Expr::Member { data, .. } => {
			if let Some(data) = data {
				visit(data, f);
			}
		}
		Expr::Range(start, end, _) => {
			if let Some(start) = start {
				visit(start, f);
			}
			if let Some(end) = end {
				visit(end, f);
			}
		}
		Expr::Template(parts) => {
			for part in parts {
				if let Part::Value(value) = part {
					visit(value, f);
				}
			}
		}
	}
}

fn visit_argument(argument: &fa_interpreter::ir::Arg, f: &mut dyn FnMut(&Expr)) {
	match argument {
		fa_interpreter::ir::Arg::Value(value) => visit(value, f),
		fa_interpreter::ir::Arg::Default => {}
		fa_interpreter::ir::Arg::Variadic(items) => {
			for item in items {
				match item {
					Item::One(value) | Item::Spread(value) => visit(value, f),
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn exact_types_of_values() {
		let mut types = TypeStore::new();
		let value = Value::object(vec![
			("port", Value::int(8080)),
			("debug", Value::Boolean(false)),
		]);
		let ty = exact_type(&mut types, &value);
		assert_eq!(types.display(ty), "{ debug: false, port: 8080 }");
		let fruits = Value::array(vec![Value::string("apple"), Value::string("banana")]);
		let ty = exact_type(&mut types, &fruits);
		assert!(
			types.display(ty).starts_with("Array("),
			"{}",
			types.display(ty)
		);
		assert!(is_js_target("bun") && !is_js_target("native"));
	}

	#[test]
	fn the_exact_type_of_a_large_array_adds_few_types() {
		let mut types = TypeStore::new();
		let before = types.len();
		let numbers = Value::array((0..100_000).map(Value::int).collect());
		let ty = exact_type(&mut types, &numbers);
		assert_eq!(types.display(ty), "Array(Integer)");
		let texts = Value::array((0..1_000).map(|n| Value::string(&n.to_string())).collect());
		let ty = exact_type(&mut types, &texts);
		assert_eq!(types.display(ty), "Array(String)");
		assert!(types.len() - before < 100, "{} types", types.len() - before);
		// a long text is a `String`, not a literal type holding a copy of it
		let long = Value::string(&"a".repeat(MAX_LITERAL_TEXT + 1));
		assert_eq!(exact_type(&mut types, &long), TypeId::STRING);
	}
}
