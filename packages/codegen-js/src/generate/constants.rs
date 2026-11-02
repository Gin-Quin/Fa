//! Compile-time values (advanced brief §12): a node the analysis evaluated
//! (`Analysis::constant`: an `@` call, the read of a `static`, `target`)
//! is emitted as its value, `static foo = 12; console.log(foo)` as
//! `console.log(12)`; a `static` declaration emits nothing; `@if` emits its
//! kept branch (`Analysis::comptime_branch`); `@for` emits its body once
//! per item (`Analysis::unrolled`), the variable a `const` of the item.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};
use fa_interpreter::Value;

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// The JavaScript of a compile-time value of type `ty`, at node `n` (for
	/// an error when the value can't be written).
	pub(crate) fn constant_value(&mut self, n: NodeId, value: &Value, ty: TypeId) -> Expr {
		match self.comptime_text(value, ty) {
			Some(expr) => expr,
			None => self.unsupported(n, "this compile-time value"),
		}
	}

	fn comptime_text(&self, value: &Value, ty: TypeId) -> Option<Expr> {
		let types = self.types;
		let ty = if ty.is_unset() { TypeId::UNKNOWN } else { ty };
		Some(match value {
			Value::None => Expr::constant("undefined", Prec::Primary),
			Value::Boolean(value) => Expr::constant(value.to_string(), Prec::Primary),
			Value::Integer(integer) => {
				let bigint = match types.kind(ty) {
					TypeKind::BigInt | TypeKind::BigIntLit(_) => true,
					TypeKind::Int(_) => types
						.as_int(ty)
						.is_some_and(|int| int.storage.is_js_bigint()),
					_ => false,
				};
				let prec = if *integer < ethnum::I256::ZERO {
					Prec::Unary
				} else {
					Prec::Primary
				};
				match types.kind(ty) {
					TypeKind::Float(_) | TypeKind::FloatLit(..) => {
						js::float_literal(integer.as_f64())
					}
					_ if bigint => Expr::constant(format!("{integer}n"), prec),
					_ => Expr::constant(integer.to_string(), prec),
				}
			}
			Value::Float(value) | Value::Percentage(value) => js::float_literal(*value),
			Value::String(text) => Expr::constant(js::string_literal(text), Prec::Primary),
			Value::Array(items) | Value::Tuple(items) => {
				let item_types: Vec<TypeId> = match types.kind(ty) {
					TypeKind::Collection(_, item, _) => vec![item; items.len()],
					TypeKind::Tuple(list) => types.list_items(list).to_vec(),
					_ => vec![TypeId::UNKNOWN; items.len()],
				};
				let parts = items
					.iter()
					.zip(
						item_types
							.into_iter()
							.chain(std::iter::repeat(TypeId::UNKNOWN)),
					)
					.map(|(item, ty)| self.comptime_text(item, ty).map(|expr| expr.text))
					.collect::<Option<Vec<_>>>()?;
				Expr::constant(format!("[{}]", parts.join(", ")), Prec::Primary)
			}
			Value::Set(items) => {
				let item = match types.kind(ty) {
					TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, item, _) => item,
					_ => TypeId::UNKNOWN,
				};
				if !items.iter().all(is_primitive) {
					return None;
				}
				let parts = items
					.iter()
					.map(|value| self.comptime_text(value, item).map(|expr| expr.text))
					.collect::<Option<Vec<_>>>()?;
				Expr::constant(format!("new Set([{}])", parts.join(", ")), Prec::Primary)
			}
			Value::Map(entries) => {
				let (key, item) = match types.kind(ty) {
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, item, key) => {
						(key, item)
					}
					_ => (TypeId::UNKNOWN, TypeId::UNKNOWN),
				};
				if !entries.iter().all(|(key, _)| is_primitive(key)) {
					return None;
				}
				let parts = entries
					.iter()
					.map(|(k, v)| {
						Some(format!(
							"[{}, {}]",
							self.comptime_text(k, key)?.text,
							self.comptime_text(v, item)?.text
						))
					})
					.collect::<Option<Vec<_>>>()?;
				Expr::constant(format!("new Map([{}])", parts.join(", ")), Prec::Primary)
			}
			Value::Object(object) => {
				let fields = types.fields_of(ty).to_vec();
				let parts = object
					.fields
					.iter()
					.map(|(name, value)| {
						let field_ty = fields
							.iter()
							.find(|field| types.syms.resolve(field.name) == &**name)
							.map_or(TypeId::UNKNOWN, |field| field.ty);
						let key = if is_identifier(name) {
							name.to_string()
						} else {
							js::string_literal(name)
						};
						Some(format!(
							"{key}: {}",
							self.comptime_text(value, field_ty)?.text
						))
					})
					.collect::<Option<Vec<_>>>()?;
				if parts.is_empty() {
					Expr::constant("{}", Prec::Primary)
				} else {
					Expr::constant(format!("{{ {} }}", parts.join(", ")), Prec::Primary)
				}
			}
			// members, ranges and functions are built by the runtime: not yet
			Value::Member(_) | Value::Range { .. } | Value::Function(_) => return None,
		})
	}

	/// `@if`: the branch the analysis kept, or nothing.
	pub(crate) fn comptime_if_into(
		&mut self,
		n: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
		dest: Dest,
	) {
		let branch = match self.analysis.comptime_branch(n) {
			Some(true) => Some(then_body),
			Some(false) => else_body,
			None => None,
		};
		match branch {
			Some(branch) => self.into(branch, dest),
			None => self.finish(
				Expr::constant("undefined", Prec::Primary),
				&dest,
				TypeId::VOID,
			),
		}
	}

	/// `@for`: the body once per item, in a block binding the variable.
	/// Without items (a range bounded by a compile-time parameter, known
	/// per call), a loop.
	pub(crate) fn comptime_for(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) {
		let Some(items) = self.analysis.unrolled.get(&n).cloned() else {
			self.for_statement(n, pattern, has_index, iterable, body);
			return;
		};
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let item_type = variable.map_or(TypeId::UNKNOWN, |symbol| {
			self.analysis.symbols.symbol(symbol).ty
		});
		let index = if has_index {
			self.analysis
				.symbols
				.symbols()
				.iter()
				.position(|symbol| {
					symbol.decl_node == n
						&& matches!(
							symbol.kind,
							fa_analyzer::symbols::SymbolKind::LoopVariable { copy: true }
						)
				})
				.map(fa_analyzer::ids::SymbolId::new)
		} else {
			None
		};
		for (position, item) in items.iter().enumerate() {
			self.out.open("");
			self.push_scope();
			if let Some(symbol) = variable {
				let value = self.constant_value(n, item, item_type);
				let name = self.declare(symbol);
				self.out.line(format!("const {name} = {};", value.text));
			}
			if let Some(symbol) = index {
				let name = self.declare(symbol);
				self.out.line(format!("const {name} = {position};"));
			}
			self.block_into(body, Dest::Discard);
			self.pop_scope();
			self.out.close("");
		}
	}
}

fn is_primitive(value: &Value) -> bool {
	matches!(
		value,
		Value::None | Value::Boolean(_) | Value::Integer(_) | Value::Float(_) | Value::String(_)
	)
}

fn is_identifier(name: &str) -> bool {
	name.chars()
		.next()
		.is_some_and(|c| c.is_alphabetic() || c == '_')
		&& name.chars().all(|c| c.is_alphanumeric() || c == '_')
}
