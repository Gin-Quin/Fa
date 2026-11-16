//! Compile-time values (advanced brief §12): a node the analysis evaluated
//! (`Analysis::constant`: an `@` call, the read of a `static`, `target`)
//! is emitted as its value, `static foo = 12; console.log(foo)` as
//! `console.log(12)`; a `static` declaration emits nothing; `@if` emits its
//! kept branch (`Analysis::comptime_branch`); `@for` emits its body once
//! per item (`Analysis::unrolled`), the variable a `const` of the item.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, NominalKind, TypeKind};
use fa_interpreter::Value;

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// The JavaScript of a compile-time value of type `ty`, at node `n` (for
	/// an error when the value can't be written).
	pub(crate) fn constant_value(&mut self, n: NodeId, value: &Value, ty: TypeId) -> Expr {
		match self.comptime_text(value, ty) {
			Some(expr) => expr,
			None => self.unsupported(
				n,
				if value.has_lone_surrogates() {
					"string constants with lone surrogates (not emitted yet)"
				} else {
					"this compile-time value"
				},
			),
		}
	}

	/// A member of a named union or an enumeration of type `ty` (`Schema.Named
	/// { … }`): its tag and data, like a member built at run time.
	fn member_constant(
		&mut self,
		member: &fa_interpreter::value::Member,
		ty: TypeId,
	) -> Option<Expr> {
		let types = self.types;
		let (nominal, index) = types.union_members(&ty).iter().find_map(|&candidate| {
			let nominal = match types.kind(candidate) {
				TypeKind::NamedUnion(nominal, _)
				| TypeKind::Member(nominal, _)
				| TypeKind::Enum(nominal, _)
				| TypeKind::EnumMember(nominal, _) => nominal,
				_ => return None,
			};
			let count = types.sym_list(types.nominal(nominal).members).len() as u16;
			(0..count)
				.find(|&index| self.member_qualified_name(nominal, index) == *member.name)
				.map(|index| (nominal, index))
		})?;
		match (types.nominal(nominal).kind, &member.data) {
			(NominalKind::Enum, _) => Some(self.enum_value(nominal, index)),
			(NominalKind::Union, None) => {
				Some(self.nominal_member_value(NodeId::new(0), nominal, index))
			}
			(NominalKind::Union, Some(data)) => {
				let payload = types.payload(nominal, index);
				let tag = js::string_literal(&member.name);
				let data = self.comptime_text(data, payload)?;
				Some(match types.kind(payload) {
					TypeKind::Object(_) => {
						let fields = data
							.text
							.trim_start_matches('{')
							.trim_end_matches('}')
							.trim();
						if fields.is_empty() {
							Expr::primary(format!("{{ $t: {tag} }}"))
						} else {
							Expr::primary(format!("{{ $t: {tag}, {fields} }}"))
						}
					}
					_ => Expr::primary(format!("{{ $t: {tag}, $v: {} }}", data.text)),
				})
			}
			_ => None,
		}
	}

	fn comptime_text(&mut self, value: &Value, ty: TypeId) -> Option<Expr> {
		let types = self.types;
		let ty = if ty.is_unset() { TypeId::UNKNOWN } else { ty };
		Some(match value {
			Value::None => Expr::constant("undefined", Prec::Primary),
			Value::Boolean(value) => Expr::constant(value.to_string(), Prec::Primary),
			Value::Integer(integer) => {
				let bigint = match types.kind(ty) {
					TypeKind::BigInt | TypeKind::BigIntLit(_) => true,
					TypeKind::Fields(_) => types.fields_width(ty) > 32,
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
			Value::Bytes(_) => return None, // The Bytes emitter is updated on strings/js.
			Value::String(text) => {
				Expr::constant(js::string_literal(text.as_str()?), Prec::Primary)
			}
			// an empty pool, set or map (a field default of a type of another
			// file): an empty collection of its class
			Value::Array(items)
				if items.is_empty()
					&& matches!(
						types.kind(ty),
						TypeKind::Collection(kind, ..)
							if !matches!(kind, CollKind::Array | CollKind::GrowingArray)
					) =>
			{
				let class = match types.kind(ty) {
					TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, item, _) => {
						if self.is_primitive(item) {
							"Set".to_string()
						} else {
							self.helper("FaSet").to_string()
						}
					}
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key) => {
						if self.is_primitive(key) {
							"Map".to_string()
						} else {
							self.helper("FaHashMap").to_string()
						}
					}
					TypeKind::Collection(kind, ..) if super::collections::is_pool(kind) => {
						if self.is_value_pool(ty) {
							self.helper("FaValuePool").to_string()
						} else {
							"Set".to_string()
						}
					}
					_ => return None,
				};
				Expr::primary(format!("new {class}()"))
			}
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
			Value::Member(member) => return self.member_constant(member, ty),
			// a range: a `FaRange`, as a range literal builds it (objects.rs)
			Value::Range {
				start,
				end,
				inclusive,
			} => {
				let bound = |bound: &Option<ethnum::I256>| {
					bound.map_or_else(|| "undefined".to_string(), |bound| bound.to_string())
				};
				let class = self.helper("FaRange");
				Expr::primary(format!(
					"new {class}({}, {}, {inclusive})",
					bound(start),
					bound(end)
				))
			}
			// functions are built by the runtime: not yet
			Value::Function(_) => return None,
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
		let branch = match self.comptime_branch(n) {
			Some(true) => Some(then_body),
			Some(false) => else_body,
			None => None,
		};
		match branch {
			Some(branch) => self.into(branch, dest),
			None => self.finish(n, Expr::constant("undefined", Prec::Primary), &dest),
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
		// over a key set (fields.rs)
		if self.analysis.key_loops.contains(&n) {
			self.key_loop(n, pattern, iterable, body);
			return;
		}
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
		let results = self.analysis.unrolled_items.get(&n).cloned();
		for (position, item) in items.iter().enumerate() {
			// the branches and values of this item (an `@for` analyzed per item)
			let per_item = results
				.as_ref()
				.and_then(|results| results.get(position))
				.cloned();
			let overlay = per_item.is_some();
			if let Some(per_item) = per_item {
				self.item_results.push(per_item);
			}
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
			if overlay {
				self.item_results.pop();
			}
		}
	}

	/// The branch an `@if` keeps: the item's being emitted, else the analysis'.
	pub(crate) fn comptime_branch(&self, n: NodeId) -> Option<bool> {
		self.item_results
			.iter()
			.rev()
			.find_map(|results| results.branches.get(&n).copied())
			.or_else(|| self.analysis.comptime_branch(n))
	}

	/// The bounds of a node whose value is a compile-time range (`static
	/// span = 1..5`), folded where it is printed, iterated or sliced with.
	pub(crate) fn constant_range(
		&self,
		n: NodeId,
	) -> Option<(Option<ethnum::I256>, Option<ethnum::I256>, bool)> {
		match self.comptime_value(n)? {
			Value::Range {
				start,
				end,
				inclusive,
			} => Some((start, end, inclusive)),
			_ => None,
		}
	}

	/// The compile-time value of a node: the item's being emitted, else the analysis'.
	pub(crate) fn comptime_value(&self, n: NodeId) -> Option<Value> {
		self.item_results
			.iter()
			.rev()
			.find_map(|results| results.constants.get(&n).cloned())
			.or_else(|| self.analysis.constant(n).cloned())
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
