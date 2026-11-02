//! The number API (`fa_analyzer::analysis`, "The number API"): methods of
//! integers, floats, big integers and percentages, `Math.max`/`Math.min`,
//! the float to integer conversions `Integer.round(x)`…, `Float.parse` and
//! the constants (`Int8.maxValue`, `Float.nan`…).
//!
//! Operations on integers of up to 32 bits are inlined with JavaScript's
//! 32-bit operators where they are exactly the Fa operation (`a & b`,
//! `Math.imul(a, b) | 0` for `Int32#wrappingMul`); the 64-bit `Integer`,
//! the bigint-represented types and the operations that can fail go
//! through the runtime helpers.

use ethnum::I256;
use fa_analyzer::analysis::{Argument, NodeFlags};
use fa_analyzer::ids::NodeId;
use fa_analyzer::types::{FloatBits, IntStorage};

use crate::generate::{Generator, Kind, Repr};
use crate::js::{self, Expr, Prec};

/// Whether a path is one of the number API's functions or methods.
pub(crate) fn is_number_api(path: &str) -> bool {
	["Integer#", "Float#", "BigInt#", "Percentage#", "Math."]
		.iter()
		.any(|prefix| path.starts_with(prefix))
		|| matches!(
			path,
			"Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate" | "Float.parse"
		)
}

impl Generator<'_> {
	/// A call of the number API.
	pub(crate) fn number_api_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let given: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		// a built-in method's receiver is not among its arguments
		if !path.contains('#') {
			let values = self.in_order(&given, |g, node| g.expr(node));
			return self.number_function(n, path, &given, values);
		}
		let receiver_type = self.receiver_type(callee);
		let receiver = self.receiver(callee);
		let (lines, values) = self.capture(|g| g.in_order(&given, |g, node| g.expr(node)));
		let receiver = if lines.is_empty() || receiver.stable {
			receiver
		} else {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", receiver.text));
			Expr::name(temp, true)
		};
		self.out.append(lines);
		let name = path.split_once('#').map_or(path, |(_, name)| name);
		match self.kind(receiver_type) {
			Kind::Int(storage) => self.integer_method(n, name, storage, receiver, values),
			Kind::Float(bits) => self.float_method(n, name, bits, receiver, values),
			Kind::BigInt => self.big_int_method(n, name, receiver),
			Kind::Percentage => match name {
				"toString" => {
					let helper = self.helper("faPercentageToString");
					js::call(helper, &[receiver.text])
				}
				"abs" => js::call("Math.abs", &[receiver.text]),
				_ => self.unsupported(n, &format!("`{path}`")),
			},
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	/// `Math.max(a, b)`, `Integer.round(x)`, `Float.parse(text)`.
	fn number_function(
		&mut self,
		n: NodeId,
		path: &str,
		nodes: &[NodeId],
		values: Vec<Expr>,
	) -> Expr {
		let location = self.location(n);
		match (path, values.as_slice()) {
			("Math.max" | "Math.min", [a, b]) => {
				let bigint = self.repr(self.type_of(n)) == Repr::BigInt;
				if !bigint {
					let name = if path == "Math.max" { "max" } else { "min" };
					return js::call(&format!("Math.{name}"), &[a.text.clone(), b.text.clone()]);
				}
				let mut operands = Vec::new();
				for (value, &node) in [a, b].into_iter().zip(nodes) {
					let value = if self.repr(self.type_of(node)) == Repr::Number {
						self.big_int_of(node, value)
					} else {
						value.clone()
					};
					operands.push(value);
				}
				let (a_first, a_later) = self.twice(operands[0].clone());
				let (b_first, b_later) = self.twice(operands[1].clone());
				let operator = if path == "Math.max" { ">" } else { "<" };
				let test = js::binary(&a_first, operator, &b_first, Prec::Relational);
				js::conditional(&test, &a_later, &b_later)
			}
			("Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate", [x]) => {
				if self.has(n, NodeFlags::CONVERSION_CHECK) {
					let helper = match path {
						"Integer.round" => "faIntegerRound",
						"Integer.floor" => "faIntegerFloor",
						"Integer.ceil" => "faIntegerCeil",
						_ => "faIntegerTruncate",
					};
					let helper = self.helper(helper);
					return js::call(helper, &[x.text.clone(), location]);
				}
				// proven to fit: the rounding, without `-0` (D-76)
				let rounded = match path {
					"Integer.round" => {
						let helper = self.helper("faRound");
						js::call(helper, std::slice::from_ref(&x.text))
					}
					"Integer.floor" => {
						return js::call("Math.floor", std::slice::from_ref(&x.text));
					}
					"Integer.ceil" => js::call("Math.ceil", std::slice::from_ref(&x.text)),
					_ => js::call("Math.trunc", std::slice::from_ref(&x.text)),
				};
				js::binary(
					&rounded,
					"+",
					&Expr::constant("0", Prec::Primary),
					Prec::Additive,
				)
			}
			("Float.parse", [text]) => {
				let helper = self.helper("faParseFloat");
				js::call(helper, std::slice::from_ref(&text.text))
			}
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	/// A method of an integer of a storage.
	fn integer_method(
		&mut self,
		n: NodeId,
		name: &str,
		storage: IntStorage,
		receiver: Expr,
		values: Vec<Expr>,
	) -> Expr {
		let other = values.first().cloned();
		match (name, other) {
			("toString", _) => js::call("String", std::slice::from_ref(&receiver.text)),
			("abs", _) => self.abs_of(n, receiver),
			("clamp", _) if values.len() == 2 => {
				if storage.is_js_bigint() {
					let helper = self.helper("faClamp");
					return js::call(
						helper,
						&[
							receiver.text.clone(),
							values[0].text.clone(),
							values[1].text.clone(),
						],
					);
				}
				let low = js::call("Math.max", &[receiver.text.clone(), values[0].text.clone()]);
				js::call("Math.min", &[low.text, values[1].text.clone()])
			}
			("bitAnd" | "bitOr" | "bitXor" | "bitNot" | "shiftLeft" | "shiftRight", other) => {
				self.bitwise_method(n, name, storage, receiver, other)
			}
			(_, Some(other))
				if name.starts_with("wrapping")
					|| name.starts_with("saturating")
					|| name.starts_with("checked") =>
			{
				self.arithmetic_method(n, name, storage, receiver, other)
			}
			_ => self.unsupported(n, &format!("`Integer#{name}`")),
		}
	}

	/// The arguments every bitwise or arithmetic helper of the runtime
	/// takes after its operands: the width, the signedness and the location.
	fn integer_shape(&self, n: NodeId, storage: IntStorage) -> Vec<String> {
		vec![
			storage.bits().to_string(),
			storage.is_signed().to_string(),
			self.location(n),
		]
	}

	/// `bitAnd`, `bitOr`, `bitXor`, `bitNot`, `shiftLeft`, `shiftRight`:
	/// JavaScript's 32-bit operators up to 32 bits, bigint operators for the
	/// bigint types, runtime helpers in between.
	fn bitwise_method(
		&mut self,
		n: NodeId,
		name: &str,
		storage: IntStorage,
		receiver: Expr,
		other: Option<Expr>,
	) -> Expr {
		let (bits, signed, bigint) = (storage.bits(), storage.is_signed(), storage.is_js_bigint());
		// a number of up to 32 bits: JavaScript's 32-bit operators
		let small = !bigint && bits <= 32;
		let mut arguments = vec![receiver.text.clone()];
		arguments.extend(other.iter().map(|other| other.text.clone()));
		arguments.extend(self.integer_shape(n, storage));
		match (name, other) {
			("bitNot", _) if small => {
				// the complement of a signed value in range stays in range
				let not = js::prefix("~", &receiver);
				if signed { not } else { wrap(not, bits, signed) }
			}
			("bitNot", _) => {
				let helper = self.helper("faBitNot");
				js::call(helper, &arguments)
			}
			("shiftLeft" | "shiftRight", Some(amount))
				if small && !self.has(n, NodeFlags::SHIFT_CHECK) =>
			{
				if name == "shiftRight" {
					let operator = if signed { ">>" } else { ">>>" };
					return js::binary(&receiver, operator, &amount, Prec::Shift);
				}
				let shifted = js::binary(&receiver, "<<", &amount, Prec::Shift);
				if bits == 32 && signed {
					shifted
				} else {
					wrap(shifted, bits, signed)
				}
			}
			("shiftLeft" | "shiftRight", Some(_)) => {
				let helper = if name == "shiftLeft" {
					"faShiftLeft"
				} else {
					"faShiftRight"
				};
				let helper = self.helper(helper);
				js::call(helper, &arguments)
			}
			(_, Some(other)) if small || bigint => {
				let (operator, prec) = match name {
					"bitAnd" => ("&", Prec::BitAnd),
					"bitOr" => ("|", Prec::BitOr),
					_ => ("^", Prec::BitXor),
				};
				let result = js::binary(&receiver, operator, &other, prec);
				// both operands are in range: only an unsigned 32-bit result needs `>>> 0`
				if small && bits == 32 && !signed {
					unsigned32(result)
				} else {
					result
				}
			}
			(_, Some(_)) => {
				let helper = match name {
					"bitAnd" => "faBitAnd",
					"bitOr" => "faBitOr",
					_ => "faBitXor",
				};
				let helper = self.helper(helper);
				js::call(helper, &arguments)
			}
			_ => self.unsupported(n, &format!("`Integer#{name}`")),
		}
	}

	/// `wrapping…`, `saturating…` and `checked…` `Add`/`Sub`/`Mul`/`Pow`:
	/// inline up to 32 bits for wrapping additions and products, runtime
	/// helpers otherwise.
	fn arithmetic_method(
		&mut self,
		n: NodeId,
		name: &str,
		storage: IntStorage,
		receiver: Expr,
		other: Expr,
	) -> Expr {
		let (bits, signed, bigint) = (storage.bits(), storage.is_signed(), storage.is_js_bigint());
		if !bigint && bits <= 32 && matches!(name, "wrappingAdd" | "wrappingSub" | "wrappingMul") {
			let exact = match name {
				"wrappingAdd" => js::binary(&receiver, "+", &other, Prec::Additive),
				"wrappingSub" => js::binary(&receiver, "-", &other, Prec::Additive),
				_ if bits == 32 => {
					let product =
						js::call("Math.imul", &[receiver.text.clone(), other.text.clone()]);
					return if signed { product } else { unsigned32(product) };
				}
				// at most 2^32 in magnitude: exact before the wrap
				_ => js::binary(&receiver, "*", &other, Prec::Multiplicative),
			};
			return wrap(exact, bits, signed);
		}
		let helper = match name {
			"wrappingAdd" => "faWrappingAdd",
			"wrappingSub" => "faWrappingSub",
			"wrappingMul" => "faWrappingMul",
			"wrappingPow" => "faWrappingPow",
			"saturatingAdd" => "faSaturatingAdd",
			"saturatingSub" => "faSaturatingSub",
			"saturatingMul" => "faSaturatingMul",
			"saturatingPow" => "faSaturatingPow",
			"checkedAdd" => "faCheckedAdd",
			"checkedSub" => "faCheckedSub",
			"checkedMul" => "faCheckedMul",
			"checkedPow" => "faCheckedPow",
			_ => return self.unsupported(n, &format!("`Integer#{name}`")),
		};
		// a bigint power takes a bigint exponent
		let other = if name.ends_with("Pow") && bigint {
			js::call("BigInt", &[other.text])
		} else {
			other
		};
		let bound = |value: I256| {
			if bigint {
				format!("{value}n")
			} else {
				value.to_string()
			}
		};
		let range = storage.range();
		let mut arguments = vec![receiver.text, other.text];
		if name.starts_with("wrapping") {
			arguments.extend(self.integer_shape(n, storage));
		} else {
			arguments.push(bound(range.lo));
			arguments.push(bound(range.hi));
			if name.starts_with("checked") {
				arguments.push(js::string_literal(storage.name()));
			}
			arguments.push(self.location(n));
		}
		let helper = self.helper(helper);
		js::call(helper, &arguments)
	}

	/// A method of a float.
	fn float_method(
		&mut self,
		n: NodeId,
		name: &str,
		bits: FloatBits,
		receiver: Expr,
		values: Vec<Expr>,
	) -> Expr {
		let text = receiver.text.clone();
		match name {
			"toString" => match bits {
				FloatBits::F64 => js::call("String", &[text]),
				FloatBits::F32 => {
					let helper = self.helper("faFloat32ToString");
					js::call(helper, &[text])
				}
			},
			"round" => {
				let helper = self.helper("faRound");
				js::call(helper, &[text])
			}
			"floor" => js::call("Math.floor", &[text]),
			"ceil" => js::call("Math.ceil", &[text]),
			"truncate" => js::call("Math.trunc", &[text]),
			"abs" => js::call("Math.abs", &[text]),
			"sqrt" => {
				let root = js::call("Math.sqrt", &[text]);
				match bits {
					FloatBits::F64 => root,
					FloatBits::F32 => js::call("Math.fround", &[root.text]),
				}
			}
			"isNaN" => js::call("Number.isNaN", &[text]),
			"isFinite" => js::call("Number.isFinite", &[text]),
			"clamp" if values.len() == 2 => {
				let helper = self.helper("faClamp");
				js::call(
					helper,
					&[text, values[0].text.clone(), values[1].text.clone()],
				)
			}
			_ => self.unsupported(n, &format!("`Float#{name}`")),
		}
	}

	/// A method of a `BigInt`.
	fn big_int_method(&mut self, n: NodeId, name: &str, receiver: Expr) -> Expr {
		match name {
			"toString" => js::call("String", &[receiver.text]),
			"abs" => self.abs_of(n, receiver),
			_ => self.unsupported(n, &format!("`BigInt#{name}`")),
		}
	}

	/// A constant of the number API (`Int8.maxValue`, `Float.nan`), if the
	/// member is one.
	pub(crate) fn number_constant_member(&mut self, n: NodeId, path: &str) -> Option<Expr> {
		let (ty_name, constant) = path.split_once('.')?;
		let float32 = ty_name == "Float32";
		let float = match constant {
			"nan" => Some("NaN"),
			"infinity" => Some("Infinity"),
			"maxValue" if ty_name == "Float" => Some("Number.MAX_VALUE"),
			"maxValue" if float32 => Some("3.4028234663852886e+38"),
			"epsilon" if ty_name == "Float" => Some("Number.EPSILON"),
			"epsilon" if float32 => Some("1.1920928955078125e-7"),
			_ => None,
		};
		if let Some(float) = float {
			return Some(Expr::constant(float, Prec::Primary));
		}
		if !matches!(constant, "minValue" | "maxValue" | "jsMaxSafeInteger") {
			return None;
		}
		let ty = self.type_of(n);
		let int = *self.types.as_int(ty)?;
		let value = int.literal()?;
		if int.storage.is_js_bigint() {
			let prec = if value < I256::ZERO {
				Prec::Unary
			} else {
				Prec::Primary
			};
			return Some(Expr::constant(format!("{value}n"), prec));
		}
		// `E-NUM-19` beyond ±(2^53 − 1)
		Some(self.integer_literal(n, value))
	}
}

/// A 32-bit operation's result read as unsigned: `x >>> 0`.
fn unsigned32(value: Expr) -> Expr {
	js::binary(
		&value,
		">>>",
		&Expr::constant("0", Prec::Primary),
		Prec::Shift,
	)
}

/// An integer (exact, at most 2^53 in magnitude, or a 32-bit operator's
/// result) wrapped to a storage of 8, 16 or 32 bits.
pub(crate) fn wrap(value: Expr, bits: u32, signed: bool) -> Expr {
	let constant = |text: String| Expr::constant(text, Prec::Primary);
	match (bits, signed) {
		(32, true) => js::binary(&value, "|", &constant("0".into()), Prec::BitOr),
		(32, false) => unsigned32(value),
		(_, true) => {
			let shift = constant((32 - bits).to_string());
			let left = js::binary(&value, "<<", &shift, Prec::Shift);
			js::binary(&left, ">>", &shift, Prec::Shift)
		}
		(_, false) => js::binary(
			&value,
			"&",
			&constant(((1u64 << bits) - 1).to_string()),
			Prec::BitAnd,
		),
	}
}
