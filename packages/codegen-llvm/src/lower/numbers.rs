//! The number API (values brief §6.13, §6.14): `Math.max`, `Integer.round`
//! and friends, the constants, and the methods of integers and floats
//! (`toString`, `abs`, `clamp`, bitwise operations and shifts, `wrapping*`,
//! `saturating*`, `checked*`, `sqrt`, `isNaN`…).

use fa_analyzer::analysis::{Argument, CallInfo, NodeFlags};
use fa_analyzer::ids::{NodeId, NominalId};
use fa_analyzer::syntax::View;
use inkwell::values::{BasicMetadataValueEnum, FloatValue, IntValue};
use inkwell::{FloatPredicate, IntPredicate};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Enumerations store declaration indices, independently of pinned values.
	pub(crate) fn enum_integer(
		&self,
		value: Val<'ctx>,
		nominal: NominalId,
		int: IntRepr,
	) -> Result<Val<'ctx>, Error> {
		let types = self.types();
		let count = types.sym_list(types.nominal(nominal).members).len();
		let pinned = types.as_int(types.payload(nominal, 0)).is_some();
		let result = if pinned {
			let entries: Vec<_> = (0..count)
				.map(|index| {
					let literal = types
						.as_int(types.payload(nominal, index as u16))
						.and_then(|ty| ty.literal())
						.expect("every member of a pinned enumeration has a literal value");
					self.const_int(int.bits, literal)
				})
				.collect();
			let ty = self.int_type(int.bits);
			let array = ty.const_array(&entries);
			let table = self
				.module
				.add_global(array.get_type(), None, "fa.enum_values");
			table.set_initializer(&array);
			table.set_constant(true);
			table.set_linkage(inkwell::module::Linkage::Private);
			let pointer = unsafe {
				self.builder.build_in_bounds_gep(
					array.get_type(),
					table.as_pointer_value(),
					&[self.context.i32_type().const_zero(), value.int()],
					"",
				)?
			};
			self.builder.build_load(ty, pointer, "")?.into_int_value()
		} else {
			self.int_to(
				value.int(),
				IntRepr {
					bits: 32,
					signed: false,
				},
				int,
			)?
		};
		Ok(Val::new(Repr::Int(int), result))
	}

	/// The value of a constant of the number types (`Integer.maxValue`,
	/// `Float.infinity`…), or `None` for another path.
	pub(crate) fn number_constant(
		&self,
		path: &str,
		node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		// Instance properties still evaluate their receiver, even when their
		// result has a literal type (`makeInlineArray().capacity`).
		if path.contains('#') {
			return Ok(None);
		}
		let repr = self.node_repr(node)?;
		if let Repr::Int(int) = repr
			&& let Some(literal) = self
				.types()
				.as_int(self.type_of(node))
				.and_then(|int| int.literal())
		{
			return Ok(Some(Val::new(repr, self.const_int(int.bits, literal))));
		}
		let float32 = path.starts_with("Float32.");
		let value = match path.rsplit('.').next() {
			Some("maxValue") if float32 => f32::MAX as f64,
			Some("maxValue") => f64::MAX,
			Some("infinity") => f64::INFINITY,
			Some("nan") => f64::NAN,
			Some("epsilon") if float32 => f32::EPSILON as f64,
			Some("epsilon") => f64::EPSILON,
			_ => return Ok(None),
		};
		Ok(Some(match repr {
			Repr::F32 => Val::new(repr, self.context.f32_type().const_float(value)),
			_ => Val::new(Repr::F64, self.context.f64_type().const_float(value)),
		}))
	}

	/// The value arguments of a built-in call (without the receiver).
	fn given_values(&mut self, info: &CallInfo) -> Result<Option<Vec<(Val<'ctx>, NodeId)>>, Error> {
		let mut values = Vec::new();
		for argument in &info.arguments {
			if let Argument::Given(node) = argument {
				let value = self.expr(*node)?;
				if value.repr == Repr::Never {
					return Ok(None);
				}
				values.push((value, *node));
			}
		}
		Ok(Some(values))
	}

	/// `Math.*`, `Integer.*`, `Float.*`, and the methods of numbers.
	pub(crate) fn number_call(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		// `x.abs()` computes like `-x` (an arithmetic node with its checks)
		if matches!(path, "Integer#abs" | "Float#abs" | "Percentage#abs")
			&& let View::Call { callee, .. } = self.view(node)
			&& let View::Access { target, members } = self.view(callee)
			&& members.len() == 1
		{
			return self.absolute(node, target);
		}
		let receiver = if path.contains('#') {
			let receiver = self.receiver(node)?;
			if receiver.repr == Repr::Never {
				return Ok(receiver);
			}
			Some(receiver)
		} else {
			None
		};
		if path == "BigInt#abs"
			&& let Some(receiver) = receiver
		{
			return self.big_absolute(receiver);
		}
		let Some(values) = self.given_values(info)? else {
			return Ok(Val::never());
		};
		let repr = self.node_repr(node)?;
		let method = path.rsplit(['#', '.']).next().unwrap_or(path);
		match (path, receiver, values.as_slice()) {
			("Math.max" | "Math.min", None, [(a, _), (b, _)]) => {
				self.extremum(*a, *b, path == "Math.max", node)
			}
			(
				"Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate",
				None,
				[(value, value_node)],
			) => self.float_to_integer(node, method, *value, *value_node),
			(_, Some(receiver), arguments) if path.starts_with("Float#") => {
				self.float_method(node, method, receiver, arguments)
			}
			(_, Some(receiver), []) if method == "toString" => {
				if let Some(text) = self.integer_string(receiver)? {
					return Ok(text);
				}
				self.call_runtime("fa_b_reset", &[])?;
				self.append_value(receiver, node)?;
				let text = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
				Ok(self.fresh(text))
			}
			(_, Some(receiver), arguments) if path.starts_with("Integer#") => {
				let Repr::Int(int) = repr else {
					return Err(self.unsupported(node.index(), &format!("`{path}`")));
				};
				let receiver = self.convert(receiver, repr, node.index())?.int();
				let mut others = Vec::new();
				for (value, argument) in arguments {
					others.push((*value, *argument));
				}
				self.integer_method(node, method, int, receiver, &others)
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// `Math.max(a, b)` and `Math.min(a, b)`.
	fn extremum(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		max: bool,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		match (a.repr, b.repr) {
			(Repr::Int(x), Repr::Int(y)) => {
				let common = Repr::Int(x.common(y));
				let Repr::Int(int) = common else {
					unreachable!("an integer");
				};
				let a = self.convert(a, common, node.index())?.int();
				let b = self.convert(b, common, node.index())?.int();
				let predicate = match (max, int.signed) {
					(true, true) => IntPredicate::SGT,
					(true, false) => IntPredicate::UGT,
					(false, true) => IntPredicate::SLT,
					(false, false) => IntPredicate::ULT,
				};
				let first = self.builder.build_int_compare(predicate, a, b, "")?;
				let result = self.builder.build_select(first, a, b, "")?;
				Ok(Val::new(common, result))
			}
			_ => {
				let work = if a.repr == Repr::F32 && b.repr == Repr::F32 {
					Repr::F32
				} else {
					Repr::F64
				};
				let a = self.convert(a, work, node.index())?;
				let b = self.convert(b, work, node.index())?;
				let a = a.value.expect("a float").into_float_value();
				let b = b.value.expect("a float").into_float_value();
				// JavaScript's `Math.max`: NaN wins, -0 < +0
				let name = if max { "llvm.maximum" } else { "llvm.minimum" };
				let intrinsic = self.intrinsic(name, &[a.get_type().into()])?;
				Ok(Val::new(
					work,
					self.call_value(intrinsic, &[a.into(), b.into()])?,
				))
			}
		}
	}

	/// `Integer.round(x)` (half away from zero), `floor`, `ceil`, `truncate`:
	/// an `ArithmeticError` for NaN, an infinity or a value beyond `Integer`.
	fn float_to_integer(
		&mut self,
		node: NodeId,
		method: &str,
		value: Val<'ctx>,
		value_node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let value = self.convert(value, Repr::F64, value_node.index())?;
		let value = value.value.expect("a float").into_float_value();
		// `fptosi` truncates, and `trunc(x)` is in range exactly when `x` is
		// (doubles near ±2^63 are integers): no call of `trunc` (a function of
		// the C library on the x86-64 baseline, without SSE 4.1)
		let rounded = if method == "truncate" {
			value
		} else {
			self.float_round(method, value)?
		};
		let f64 = self.context.f64_type();
		let low = self.builder.build_float_compare(
			FloatPredicate::OGE,
			rounded,
			f64.const_float(-9_223_372_036_854_775_808.0),
			"",
		)?;
		let high = self.builder.build_float_compare(
			FloatPredicate::OLT,
			rounded,
			f64.const_float(9_223_372_036_854_775_808.0),
			"",
		)?;
		let fits = self.builder.build_and(low, high, "")?;
		let outside = self.builder.build_not(fits, "")?;
		// Ordered bounds exclude NaN and infinities too. Classify the
		// original value only on failure, preserving the exact error message
		// while keeping a single guard before the otherwise undefined cast.
		self.guard(outside, |this| {
			let magnitude =
				this.call_value(this.intrinsic("llvm.fabs", &[f64.into()])?, &[value.into()])?;
			let finite = this.builder.build_float_compare(
				FloatPredicate::OLT,
				magnitude.into_float_value(),
				f64.const_float(f64::INFINITY),
				"",
			)?;
			let finite = this
				.builder
				.build_int_z_extend(finite, this.context.i32_type(), "")?;
			this.raise_runtime("fa_error_float", &[value.into(), finite.into()])
		})?;
		let _ = node;
		let integer =
			self.builder
				.build_float_to_signed_int(rounded, self.context.i64_type(), "")?;
		Ok(Val::new(Repr::Int(IntRepr::I64), integer))
	}

	/// `round` (half away from zero), `floor`, `ceil`, `truncate` of a float.
	fn float_round(
		&self,
		method: &str,
		value: FloatValue<'ctx>,
	) -> Result<FloatValue<'ctx>, Error> {
		let name = match method {
			"round" => "llvm.round",
			"floor" => "llvm.floor",
			"ceil" => "llvm.ceil",
			_ => "llvm.trunc",
		};
		let intrinsic = self.intrinsic(name, &[value.get_type().into()])?;
		Ok(self
			.call_value(intrinsic, &[value.into()])?
			.into_float_value())
	}

	/// The methods of `Float` and `Float32`.
	fn float_method(
		&mut self,
		node: NodeId,
		method: &str,
		receiver: Val<'ctx>,
		arguments: &[(Val<'ctx>, NodeId)],
	) -> Result<Val<'ctx>, Error> {
		if method == "toString" {
			self.call_runtime("fa_b_reset", &[])?;
			self.append_value(receiver, node)?;
			let text = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
			return Ok(self.fresh(text));
		}
		let work = if receiver.repr == Repr::F32 {
			Repr::F32
		} else {
			Repr::F64
		};
		let value = self
			.convert(receiver, work, node.index())?
			.value
			.expect("a float")
			.into_float_value();
		let ty = value.get_type();
		Ok(match method {
			"round" | "floor" | "ceil" | "truncate" => {
				Val::new(work, self.float_round(method, value)?)
			}
			"sqrt" | "abs" => {
				let name = if method == "sqrt" {
					"llvm.sqrt"
				} else {
					"llvm.fabs"
				};
				let intrinsic = self.intrinsic(name, &[ty.into()])?;
				Val::new(work, self.call_value(intrinsic, &[value.into()])?)
			}
			"isNaN" => Val::new(
				Repr::Bool,
				self.builder
					.build_float_compare(FloatPredicate::UNO, value, value, "")?,
			),
			"isFinite" => {
				let fabs = self.intrinsic("llvm.fabs", &[ty.into()])?;
				let magnitude = self.call_value(fabs, &[value.into()])?.into_float_value();
				let infinity = ty.const_float(f64::INFINITY);
				Val::new(
					Repr::Bool,
					self.builder.build_float_compare(
						FloatPredicate::OLT,
						magnitude,
						infinity,
						"",
					)?,
				)
			}
			"clamp" => {
				let [(min, _), (max, _)] = arguments else {
					return Err(self.unsupported(node.index(), "`clamp`"));
				};
				let receiver = Val::new(work, value);
				let low = self.extremum(receiver, *min, true, node)?;
				self.extremum(low, *max, false, node)?
			}
			_ => return Err(self.unsupported(node.index(), &format!("`Float#{method}`"))),
		})
	}

	/// The methods of integers, on the receiver in its storage.
	fn integer_method(
		&mut self,
		node: NodeId,
		method: &str,
		int: IntRepr,
		receiver: IntValue<'ctx>,
		arguments: &[(Val<'ctx>, NodeId)],
	) -> Result<Val<'ctx>, Error> {
		let repr = Repr::Int(int);
		let other = |this: &Self, index: usize| -> Result<IntValue<'ctx>, Error> {
			let (value, argument) = arguments
				.get(index)
				.ok_or_else(|| this.unsupported(node.index(), method))?;
			Ok(this.convert(*value, repr, argument.index())?.int())
		};
		let result = match method {
			"bitAnd" => self.builder.build_and(receiver, other(self, 0)?, "")?,
			"bitOr" => self.builder.build_or(receiver, other(self, 0)?, "")?,
			"bitXor" => self.builder.build_xor(receiver, other(self, 0)?, "")?,
			"bitNot" => self.builder.build_not(receiver, "")?,
			"clamp" => {
				let low = self.extremum(
					Val::new(repr, receiver),
					Val::new(repr, other(self, 0)?),
					true,
					node,
				)?;
				let clamped = self.extremum(low, Val::new(repr, other(self, 1)?), false, node)?;
				return self.convert(clamped, repr, node.index());
			}
			"shiftLeft" | "shiftRight" => {
				let (amount, amount_node) = arguments
					.first()
					.ok_or_else(|| self.unsupported(node.index(), method))?;
				let amount = self
					.convert(*amount, Repr::Int(IntRepr::I64), amount_node.index())?
					.int();
				if self
					.analysis
					.flags_of(node)
					.contains(NodeFlags::SHIFT_CHECK)
				{
					let width = self.context.i64_type().const_int(int.bits as u64, false);
					let outside =
						self.builder
							.build_int_compare(IntPredicate::UGE, amount, width, "")?;
					self.guard(outside, |this| {
						this.raise_runtime("fa_error_shift", &[amount.into(), width.into()])
					})?;
				}
				let amount = self.int_to(amount, IntRepr::I64, int)?;
				if method == "shiftLeft" {
					self.builder.build_left_shift(receiver, amount, "")?
				} else {
					self.builder
						.build_right_shift(receiver, amount, int.signed, "")?
				}
			}
			"wrappingAdd" => self.builder.build_int_add(receiver, other(self, 0)?, "")?,
			"wrappingSub" => self.builder.build_int_sub(receiver, other(self, 0)?, "")?,
			"wrappingMul" => self.builder.build_int_mul(receiver, other(self, 0)?, "")?,
			"wrappingPow" if int.bits <= 64 => {
				let wide = self.int_to(receiver, int, IntRepr::I64)?;
				let (exponent, exponent_node) = arguments
					.first()
					.ok_or_else(|| self.unsupported(node.index(), method))?;
				let exponent = self
					.convert(*exponent, Repr::Int(IntRepr::I64), exponent_node.index())?
					.int();
				let site = self.site("Integer", node);
				let unchecked = self.context.i32_type().const_zero();
				let pow = self.runtime.function("fa_pow_i64")?;
				let result = self
					.call_value(
						pow,
						&[wide.into(), exponent.into(), unchecked.into(), site.into()],
					)?
					.into_int_value();
				self.int_to(result, IntRepr::I64, int)?
			}
			"saturatingAdd" | "saturatingSub" | "saturatingMul" | "checkedAdd" | "checkedSub"
			| "checkedMul"
				if int.bits <= 64 =>
			{
				// the exact result, in 128 bits, against the receiver's type
				let other = other(self, 0)?;
				let i128 = IntRepr {
					bits: 128,
					signed: true,
				};
				let a = self.int_to(receiver, int, i128)?;
				let b = self.int_to(other, int, i128)?;
				let exact = match &method[method.len() - 3..] {
					"Add" => self.builder.build_int_add(a, b, "")?,
					"Sub" => self.builder.build_int_sub(a, b, "")?,
					_ => self.builder.build_int_mul(a, b, "")?,
				};
				let bounds = self
					.types()
					.as_int(self.type_of(node))
					.map(|int| (int.min, int.max))
					.unwrap_or_else(|| {
						let range = fa_analyzer::types::IntStorage::Default.range();
						(range.lo, range.hi)
					});
				let min = self.const_int(128, bounds.0);
				let max = self.const_int(128, bounds.1);
				let below = self
					.builder
					.build_int_compare(IntPredicate::SLT, exact, min, "")?;
				let above = self
					.builder
					.build_int_compare(IntPredicate::SGT, exact, max, "")?;
				if method.starts_with("saturating") {
					let clamped = self.builder.build_select(below, min, exact, "")?;
					let clamped = self
						.builder
						.build_select(above, max, clamped.into_int_value(), "")?
						.into_int_value();
					self.int_to(clamped, i128, int)?
				} else {
					let outside = self.builder.build_or(below, above, "")?;
					let name = self.session.display_type(self.type_of(node));
					let site = self.site(&name, node);
					let low =
						self.builder
							.build_int_truncate(exact, self.context.i64_type(), "")?;
					let shifted = self.builder.build_right_shift(
						exact,
						exact.get_type().const_int(64, false),
						false,
						"",
					)?;
					let high =
						self.builder
							.build_int_truncate(shifted, self.context.i64_type(), "")?;
					let arguments: Vec<BasicMetadataValueEnum<'ctx>> =
						vec![low.into(), high.into(), site.into()];
					self.guard(outside, |this| {
						this.raise_runtime("fa_error_checked", &arguments)
					})?;
					self.int_to(exact, i128, int)?
				}
			}
			_ => {
				return Err(self.unsupported(node.index(), &format!("`Integer#{method}`")));
			}
		};
		Ok(Val::new(repr, result))
	}
}
