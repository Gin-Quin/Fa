//! The evaluator: a tree walk over [`Expr`] with a step budget.
//!
//! Every evaluated expression is one step (decision D26); a call of a
//! function deeper than [`Limits::depth`] is reported like the budget
//! (the native stack of the compiler is the limit there). Errors thrown by
//! the code ([`Error::Thrown`]) and stops ([`Error::Stop`],
//! [`Error::Overflow`]) end the evaluation: the analyzer reports them at the
//! `@` call (`META005`, `META013`).

use std::sync::Arc;

use ethnum::I256;

use crate::builtins;
use crate::ir::{Arg, ArithOp, CompareOp, Expr, FnKey, Function, Item, Num, Part, Place, Step};
use crate::value::{Closure, Member, MemberKind, Object, Value, equals};

/// The bounds of one evaluation (advanced brief §12.3).
#[derive(Copy, Clone, Debug)]
pub struct Limits {
	/// The number of evaluated expressions (100 million by default).
	pub steps: u64,
	/// The depth of nested calls.
	pub depth: u32,
}

impl Default for Limits {
	fn default() -> Self {
		Self {
			steps: 100_000_000,
			depth: 10_000,
		}
	}
}

/// Why an evaluation stopped.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
	/// An error thrown by the code and not caught (`META005`).
	Thrown(Value),
	/// A stop: a failed `assert`, a failed check (`META005`).
	Stop(String),
	/// An integer overflow (`META013`).
	Overflow(String),
	/// More steps than [`Limits::steps`] (`META003`).
	Steps(u64),
	/// Calls nested deeper than [`Limits::depth`] (`META003`).
	Depth(u32),
	/// Code with an outside effect was reached (`META002`): what it is.
	Impure(String),
	/// Code the interpreter can't run yet: what it is.
	Unsupported(String),
}

impl Error {
	/// The message of the error, for a diagnostic.
	pub fn message(&self) -> String {
		match self {
			Error::Thrown(value) => format!("the error `{}` was thrown", value.to_text()),
			Error::Stop(reason) => reason.clone(),
			Error::Overflow(what) => what.clone(),
			Error::Steps(limit) => format!("more than {limit} evaluation steps"),
			Error::Depth(limit) => format!("more than {limit} nested calls"),
			Error::Impure(what) => format!("`{what}` is not deterministic"),
			Error::Unsupported(what) => format!("{what} can't be evaluated at compile time yet"),
		}
	}
}

/// What the interpreter asks the analyzer.
pub trait Host {
	/// The lowered function of a key.
	fn function(&mut self, key: FnKey) -> Result<Arc<Function>, Error>;
	/// The value of a module-level value (evaluated once).
	fn global(&mut self, key: crate::ir::GlobalKey) -> Result<Value, Error>;
}

/// How control leaves an expression (an implementation detail of the evaluator).
#[doc(hidden)]
pub enum Flow {
	Break(Value),
	Continue,
	Return(Value),
	Error(Error),
}

impl From<Error> for Flow {
	fn from(error: Error) -> Self {
		Flow::Error(error)
	}
}

pub(crate) type Eval = Result<Value, Flow>;

/// The interpreter of one evaluation (one `@` call, one `static` value).
pub struct Interpreter<'h> {
	host: &'h mut dyn Host,
	limits: Limits,
	steps: u64,
	depth: u32,
}

impl<'h> Interpreter<'h> {
	/// An interpreter asking `host` for functions and globals.
	pub fn new(host: &'h mut dyn Host, limits: Limits) -> Self {
		Self {
			host,
			limits,
			steps: 0,
			depth: 0,
		}
	}

	/// The steps evaluated so far.
	pub fn steps(&self) -> u64 {
		self.steps
	}

	/// Evaluates an expression lowered with `slots` locals.
	pub fn evaluate(&mut self, expr: &Expr, slots: u32) -> Result<Value, Error> {
		let mut frame = vec![Value::None; slots as usize];
		match self.eval(expr, &mut frame) {
			Ok(value) | Err(Flow::Return(value)) | Err(Flow::Break(value)) => Ok(value),
			Err(Flow::Continue) => Ok(Value::None),
			Err(Flow::Error(error)) => Err(error),
		}
	}

	/// Calls a function value with its arguments in parameter order.
	pub fn call_value(&mut self, callee: &Value, arguments: Vec<Value>) -> Result<Value, Error> {
		let Value::Function(closure) = callee else {
			return Err(Error::Stop(format!(
				"`{}` is not a function",
				callee.to_text()
			)));
		};
		let arguments = arguments.into_iter().map(Some).collect();
		self.call_closure(closure, arguments)
	}

	fn step(&mut self) -> Result<(), Flow> {
		self.steps += 1;
		if self.steps > self.limits.steps {
			return Err(Flow::Error(Error::Steps(self.limits.steps)));
		}
		Ok(())
	}

	pub(crate) fn eval(&mut self, expr: &Expr, frame: &mut Vec<Value>) -> Eval {
		self.step()?;
		match expr {
			Expr::Const(value) => Ok(value.clone()),
			Expr::Local(slot) => Ok(frame[*slot as usize].clone()),
			Expr::Block(statements) => {
				let mut last = Value::None;
				for statement in statements {
					last = self.eval(statement, frame)?;
				}
				Ok(last)
			}
			Expr::If(condition, then, otherwise) => {
				if self.condition(condition, frame)? {
					self.eval(then, frame)
				} else if let Some(otherwise) = otherwise {
					self.eval(otherwise, frame)
				} else {
					Ok(Value::None)
				}
			}
			Expr::CallFunction(function, arguments) => {
				self.call_function(*function, arguments, frame)
			}
			_ => self.eval_other(expr, frame),
		}
	}

	#[inline(never)]
	fn call_function(
		&mut self,
		function: FnKey,
		arguments: &[Arg],
		frame: &mut Vec<Value>,
	) -> Eval {
		let arguments = self.arguments(arguments, frame)?;
		let function = self.host.function(function)?;
		Ok(self.call(&function, arguments, Vec::new())?)
	}

	/// The other expressions (kept out of [`Interpreter::eval`], whose frame
	/// stays small: it is on the stack once per nested expression).
	#[inline(never)]
	fn eval_other(&mut self, expr: &Expr, frame: &mut Vec<Value>) -> Eval {
		match expr {
			Expr::Global(key) => Ok(self.host.global(*key)?),
			Expr::Function(function) => Ok(Value::Function(Arc::new(Closure {
				function: *function,
				captured: Vec::new(),
				bound: Vec::new(),
			}))),
			Expr::Closure { function, captures } => Ok(Value::Function(Arc::new(Closure {
				function: *function,
				captured: captures
					.iter()
					.map(|&slot| frame[slot as usize].clone())
					.collect(),
				bound: Vec::new(),
			}))),
			Expr::Let(slot, value) => {
				let value = self.eval(value, frame)?;
				frame[*slot as usize] = value;
				Ok(Value::None)
			}
			Expr::Assign(place, value) => {
				let value = self.eval(value, frame)?;
				let target = self.place(place, frame)?;
				*target = value;
				Ok(Value::None)
			}
			Expr::Read(place) => Ok(self.place(place, frame)?.clone()),
			Expr::Loop(body) => loop {
				match self.eval(body, frame) {
					Ok(_) | Err(Flow::Continue) => {}
					Err(Flow::Break(value)) => return Ok(value),
					Err(other) => return Err(other),
				}
			},
			Expr::While(condition, body) => {
				while self.condition(condition, frame)? {
					match self.eval(body, frame) {
						Ok(_) | Err(Flow::Continue) => {}
						Err(Flow::Break(_)) => break,
						Err(other) => return Err(other),
					}
				}
				Ok(Value::None)
			}
			Expr::For {
				item,
				index,
				iterable,
				body,
			} => self.for_loop(*item, *index, iterable, body, frame),
			Expr::Break(value) => {
				let value = match value {
					Some(value) => self.eval(value, frame)?,
					None => Value::None,
				};
				Err(Flow::Break(value))
			}
			Expr::Continue => Err(Flow::Continue),
			Expr::Return(value) => {
				let value = match value {
					Some(value) => self.eval(value, frame)?,
					None => Value::None,
				};
				Err(Flow::Return(value))
			}
			Expr::Throw(error) => {
				let error = self.eval(error, frame)?;
				Err(Flow::Error(Error::Thrown(error)))
			}
			Expr::Catch {
				body,
				slot,
				handler,
			} => match self.eval(body, frame) {
				Err(Flow::Error(Error::Thrown(error))) => {
					if let Some(slot) = slot {
						frame[*slot as usize] = error;
					}
					self.eval(handler, frame)
				}
				other => other,
			},
			Expr::Assert(condition, text) => {
				if self.condition(condition, frame)? {
					Ok(Value::None)
				} else {
					Err(Flow::Error(Error::Stop(format!(
						"assertion failed: {text}"
					))))
				}
			}
			Expr::Arith(op, num, left, right) => {
				let left = self.eval(left, frame)?;
				let right = self.eval(right, frame)?;
				Ok(arith(*op, *num, &left, &right)?)
			}
			Expr::Negate(num, value) => {
				let value = self.eval(value, frame)?;
				Ok(negate(*num, &value)?)
			}
			Expr::Convert(num, value) => {
				let value = self.eval(value, frame)?;
				Ok(convert(*num, &value)?)
			}
			Expr::Equals(left, right, negated) => {
				let left = self.eval(left, frame)?;
				let right = self.eval(right, frame)?;
				Ok(Value::Boolean(equals(&left, &right) != *negated))
			}
			Expr::Compare(op, left, right) => {
				let left = self.eval(left, frame)?;
				let right = self.eval(right, frame)?;
				Ok(Value::Boolean(compare(*op, &left, &right)?))
			}
			Expr::And(left, right) => {
				if !self.condition(left, frame)? {
					return Ok(Value::Boolean(false));
				}
				Ok(Value::Boolean(self.condition(right, frame)?))
			}
			Expr::Or(left, right) => {
				if self.condition(left, frame)? {
					return Ok(Value::Boolean(true));
				}
				Ok(Value::Boolean(self.condition(right, frame)?))
			}
			Expr::Not(value) => Ok(Value::Boolean(!self.condition(value, frame)?)),
			Expr::Fallback(value, fallback) => match self.eval(value, frame)? {
				Value::None => self.eval(fallback, frame),
				value => Ok(value),
			},
			Expr::Is(value, test, negated) => {
				let value = self.eval(value, frame)?;
				Ok(Value::Boolean(crate::types::test(test, &value) != *negated))
			}
			Expr::IsIn(value, collection, negated) => {
				let value = self.eval(value, frame)?;
				let collection = self.eval(collection, frame)?;
				let found = builtins::includes(&collection, &value)?;
				Ok(Value::Boolean(found != *negated))
			}
			Expr::Call(callee, arguments) => {
				let callee = self.eval(callee, frame)?;
				let arguments = self.arguments(arguments, frame)?;
				let Value::Function(closure) = callee else {
					return Err(Flow::Error(Error::Stop(format!(
						"`{}` is not a function",
						callee.to_text()
					))));
				};
				Ok(self.call_closure(&closure, arguments)?)
			}
			Expr::Const(_)
			| Expr::Local(_)
			| Expr::Block(_)
			| Expr::If(..)
			| Expr::CallFunction(..) => {
				unreachable!("evaluated by `eval`")
			}
			Expr::Builtin(path, arguments) => {
				let mut values = Vec::with_capacity(arguments.len());
				for argument in arguments {
					values.push(self.eval(argument, frame)?);
				}
				Ok(builtins::call(self, path, values)?)
			}
			Expr::Mutate(path, place, arguments) => {
				let mut values = Vec::with_capacity(arguments.len());
				for argument in arguments {
					values.push(self.eval(argument, frame)?);
				}
				// a callback (`deleteWhere`, `sortInPlace`) runs before the change
				let receiver = self.place(place, frame)?.clone();
				let (changed, result) = builtins::mutate(self, path, receiver, values)?;
				*self.place(place, frame)? = changed;
				Ok(result)
			}
			Expr::Array(items) => Ok(Value::Array(Arc::new(self.items(items, frame)?))),
			Expr::Set(items) => {
				let mut set: Vec<Value> = Vec::new();
				for item in self.items(items, frame)? {
					if !crate::value::contains(&set, &item) {
						set.push(item);
					}
				}
				Ok(Value::Set(Arc::new(set)))
			}
			Expr::Map(items) => {
				let mut entries: Vec<(Value, Value)> = Vec::new();
				for item in self.items(items, frame)? {
					let (key, value) = builtins::pair(item)?;
					builtins::map_insert(&mut entries, key, value);
				}
				Ok(Value::Map(Arc::new(entries)))
			}
			Expr::Tuple(items) => {
				let mut values = Vec::with_capacity(items.len());
				for item in items {
					values.push(self.eval(item, frame)?);
				}
				Ok(Value::Tuple(Arc::new(values)))
			}
			Expr::Object(fields) => {
				let mut object = Object {
					fields: Vec::with_capacity(fields.len()),
				};
				for (name, value) in fields {
					let value = self.eval(value, frame)?;
					object.fields.push((Arc::clone(name), value));
				}
				Ok(Value::Object(Arc::new(object)))
			}
			Expr::Member { name, kind, data } => {
				let data = match data {
					Some(data) => Some(self.eval(data, frame)?),
					None => None,
				};
				Ok(Value::Member(Arc::new(Member {
					name: Arc::clone(name),
					kind: *kind,
					data,
				})))
			}
			Expr::MemberData(value) => match self.eval(value, frame)? {
				Value::Member(member) => Ok(member.data.clone().unwrap_or(Value::None)),
				other => Ok(other),
			},
			Expr::Field(value, name, optional) => {
				let value = self.eval(value, frame)?;
				if *optional && matches!(value, Value::None) {
					return Ok(Value::None);
				}
				Ok(builtins::field(&value, name)?)
			}
			Expr::Index(value, index) => {
				let value = self.eval(value, frame)?;
				let index = self.eval(index, frame)?;
				Ok(builtins::index(&value, &index)?)
			}
			Expr::Range(start, end, inclusive) => {
				let mut bound = |bound: &Option<Box<Expr>>, frame: &mut Vec<Value>| match bound {
					Some(bound) => match self.eval(bound, frame)? {
						Value::Integer(value) => Ok(Some(value)),
						Value::None => Ok(None),
						other => Err(Flow::Error(Error::Stop(format!(
							"a range bound must be an integer, not `{}`",
							other.to_text()
						)))),
					},
					None => Ok(None),
				};
				let start = bound(start, frame)?;
				let end = bound(end, frame)?;
				Ok(Value::Range {
					start,
					end,
					inclusive: *inclusive,
				})
			}
			Expr::Template(parts) => {
				let mut text = String::new();
				for part in parts {
					match part {
						Part::Text(literal) => text.push_str(literal),
						Part::Value(value) => text.push_str(&self.eval(value, frame)?.to_text()),
					}
				}
				Ok(Value::String(Arc::from(text)))
			}
			Expr::Impure(what) => Err(Flow::Error(Error::Impure(what.to_string()))),
			Expr::Unsupported(what) => Err(Flow::Error(Error::Unsupported(what.to_string()))),
		}
	}

	fn condition(&mut self, expr: &Expr, frame: &mut Vec<Value>) -> Result<bool, Flow> {
		match self.eval(expr, frame)? {
			Value::Boolean(value) => Ok(value),
			other => Err(Flow::Error(Error::Stop(format!(
				"a condition must be a `Boolean`, not `{}`",
				other.to_text()
			)))),
		}
	}

	fn items(&mut self, items: &[Item], frame: &mut Vec<Value>) -> Result<Vec<Value>, Flow> {
		let mut values = Vec::with_capacity(items.len());
		for item in items {
			match item {
				Item::One(value) => values.push(self.eval(value, frame)?),
				Item::Spread(collection) => {
					let collection = self.eval(collection, frame)?;
					values.extend(builtins::iterate(&collection)?);
				}
			}
		}
		Ok(values)
	}

	fn arguments(
		&mut self,
		arguments: &[Arg],
		frame: &mut Vec<Value>,
	) -> Result<Vec<Option<Value>>, Flow> {
		let mut values = Vec::with_capacity(arguments.len());
		for argument in arguments {
			values.push(match argument {
				Arg::Value(value) => Some(self.eval(value, frame)?),
				Arg::Default => None,
				Arg::Variadic(items) => Some(Value::Array(Arc::new(self.items(items, frame)?))),
			});
		}
		Ok(values)
	}

	/// Calls a closure: its bound arguments fill their parameters first.
	pub(crate) fn call_closure(
		&mut self,
		closure: &Closure,
		arguments: Vec<Option<Value>>,
	) -> Result<Value, Error> {
		let function = self.host.function(closure.function)?;
		let arguments = if closure.bound.is_empty() {
			arguments
		} else {
			let mut given = arguments.into_iter();
			(0..function.params.len())
				.map(
					|index| match closure.bound.iter().find(|(bound, _)| *bound == index) {
						Some((_, value)) => Some(value.clone()),
						None => given.next().flatten(),
					},
				)
				.collect()
		};
		self.call(&function, arguments, closure.captured.clone())
	}

	/// Calls a lowered function.
	#[inline(never)]
	pub(crate) fn call(
		&mut self,
		function: &Function,
		arguments: Vec<Option<Value>>,
		captured: Vec<Value>,
	) -> Result<Value, Error> {
		if self.depth >= self.limits.depth {
			return Err(Error::Depth(self.limits.depth));
		}
		self.depth += 1;
		let result = self.call_inner(function, arguments, captured);
		self.depth -= 1;
		result
	}

	fn call_inner(
		&mut self,
		function: &Function,
		arguments: Vec<Option<Value>>,
		captured: Vec<Value>,
	) -> Result<Value, Error> {
		let mut frame = vec![Value::None; function.slots as usize];
		for (&slot, value) in function.captures.iter().zip(captured) {
			frame[slot as usize] = value;
		}
		let mut arguments = arguments.into_iter();
		for param in &function.params {
			let value = match arguments.next().flatten() {
				Some(value) => value,
				None => match &param.default {
					Some(default) => match self.eval(default, &mut frame) {
						Ok(value) => value,
						Err(Flow::Error(error)) => return Err(error),
						Err(_) => Value::None,
					},
					None => Value::None,
				},
			};
			frame[param.slot as usize] = value;
		}
		match self.eval(&function.body, &mut frame) {
			Ok(value) | Err(Flow::Return(value)) => Ok(value),
			Err(Flow::Break(_) | Flow::Continue) => Ok(Value::None),
			Err(Flow::Error(error)) => Err(error),
		}
	}

	fn for_loop(
		&mut self,
		item: Option<u32>,
		index: Option<u32>,
		iterable: &Expr,
		body: &Expr,
		frame: &mut Vec<Value>,
	) -> Eval {
		let iterable = self.eval(iterable, frame)?;
		// an open range is iterated lazily
		if let Value::Range {
			start, end: None, ..
		} = &iterable
		{
			let Some(mut current) = *start else {
				return Err(Flow::Error(Error::Stop(
					"a range without a start can't be iterated".into(),
				)));
			};
			let mut position = 0i64;
			loop {
				if let Some(slot) = item {
					frame[slot as usize] = Value::Integer(current);
				}
				if let Some(slot) = index {
					frame[slot as usize] = Value::int(position);
				}
				match self.eval(body, frame) {
					Ok(_) | Err(Flow::Continue) => {}
					Err(Flow::Break(_)) => break,
					Err(other) => return Err(other),
				}
				current += I256::ONE;
				position += 1;
			}
			return Ok(Value::None);
		}
		let values = builtins::iterate(&iterable)?;
		for (position, value) in values.into_iter().enumerate() {
			if let Some(slot) = item {
				frame[slot as usize] = value;
			}
			if let Some(slot) = index {
				frame[slot as usize] = Value::int(position as i64);
			}
			match self.eval(body, frame) {
				Ok(_) | Err(Flow::Continue) => {}
				Err(Flow::Break(_)) => break,
				Err(other) => return Err(other),
			}
		}
		Ok(Value::None)
	}

	/// The value at a place, to read or change it (copy on write).
	fn place<'f>(
		&mut self,
		place: &Place,
		frame: &'f mut Vec<Value>,
	) -> Result<&'f mut Value, Flow> {
		// the indices first: they may read the frame
		let mut keys = Vec::with_capacity(place.steps.len());
		for step in &place.steps {
			keys.push(match step {
				Step::Field(_) => None,
				Step::Index(index) => Some(self.eval(index, frame)?),
			});
		}
		let mut current = &mut frame[place.slot as usize];
		for (step, key) in place.steps.iter().zip(keys) {
			current = match (step, key) {
				(Step::Field(name), _) => builtins::field_mut(current, name)?,
				(Step::Index(_), Some(key)) => builtins::index_mut(current, key)?,
				(Step::Index(_), None) => unreachable!("an index step has a key"),
			};
		}
		Ok(current)
	}
}

impl From<Flow> for Error {
	fn from(flow: Flow) -> Self {
		match flow {
			Flow::Error(error) => error,
			_ => Error::Stop("`break`, `continue` or `return` outside of its construct".into()),
		}
	}
}

// ── numbers ──────────────────────────────────────────────────────────────

fn overflow(op: &str, left: &Value, right: &Value) -> Error {
	Error::Overflow(format!(
		"`{} {op} {}` overflows at compile time",
		left.to_text(),
		right.to_text()
	))
}

fn division_by_zero() -> Error {
	Error::Thrown(Value::error("DivisionByZero", Vec::new()))
}

fn arithmetic_error(message: &str) -> Error {
	Error::Thrown(Value::error(
		"ArithmeticError",
		vec![("message", Value::string(message))],
	))
}

/// Rounds a float to the kind's precision.
fn round(num: Num, value: f64) -> Value {
	match num {
		Num::Float32 => Value::Float(value as f32 as f64),
		Num::Percentage => Value::Percentage(value),
		_ => Value::Float(value),
	}
}

fn in_range(num: Num, value: I256) -> bool {
	match num {
		Num::Int { min, max } => min <= value && value <= max,
		_ => true,
	}
}

/// An arithmetic operation of the kind `num`.
pub fn arith(op: ArithOp, num: Num, left: &Value, right: &Value) -> Result<Value, Error> {
	let symbol = match op {
		ArithOp::Add => "+",
		ArithOp::Subtract => "-",
		ArithOp::Multiply => "*",
		ArithOp::Divide => "/",
		ArithOp::IntegerDivide => "//",
		ArithOp::Remainder => "%%",
		ArithOp::Power => "**",
	};
	if let (Num::Int { .. }, Value::Integer(a), Value::Integer(b)) = (num, left, right)
		&& op != ArithOp::Divide
	{
		let (a, b) = (*a, *b);
		let result = match op {
			ArithOp::Add => a.checked_add(b),
			ArithOp::Subtract => a.checked_sub(b),
			ArithOp::Multiply => a.checked_mul(b),
			ArithOp::IntegerDivide | ArithOp::Remainder => {
				if b == I256::ZERO {
					return Err(division_by_zero());
				}
				let quotient = a / b;
				let floor = if (a % b != I256::ZERO) && ((a < I256::ZERO) != (b < I256::ZERO)) {
					quotient - I256::ONE
				} else {
					quotient
				};
				Some(if op == ArithOp::IntegerDivide {
					floor
				} else {
					a - floor * b
				})
			}
			ArithOp::Power => {
				if b < I256::ZERO {
					return Err(arithmetic_error("a negative exponent"));
				}
				let mut result = Some(I256::ONE);
				let mut count = b;
				while count > I256::ZERO && result.is_some() {
					result = result.and_then(|value| value.checked_mul(a));
					count -= I256::ONE;
					// beyond the widest integer: an overflow anyway
					if count > I256::from(512u32) && a.abs() > I256::ONE {
						result = None;
					}
				}
				result
			}
			ArithOp::Divide => unreachable!("`/` is a float division"),
		};
		return match result {
			Some(value) if in_range(num, value) => Ok(Value::Integer(value)),
			_ => Err(overflow(symbol, left, right)),
		};
	}
	let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) else {
		return Err(Error::Stop(format!(
			"`{}` and `{}` are not numbers",
			left.to_text(),
			right.to_text()
		)));
	};
	let value = match op {
		ArithOp::Add => a + b,
		ArithOp::Subtract => a - b,
		ArithOp::Multiply => a * b,
		ArithOp::Divide => a / b,
		ArithOp::IntegerDivide => (a / b).floor(),
		ArithOp::Remainder => a - b * (a / b).floor(),
		ArithOp::Power => a.powf(b),
	};
	let num = if op == ArithOp::Divide && matches!(num, Num::Int { .. }) {
		Num::Float
	} else {
		num
	};
	Ok(round(num, value))
}

/// `-x`.
pub fn negate(num: Num, value: &Value) -> Result<Value, Error> {
	match value {
		Value::Integer(value) => match value.checked_neg() {
			Some(result) if in_range(num, result) => Ok(Value::Integer(result)),
			_ => Err(Error::Overflow(format!(
				"`-{value}` overflows at compile time"
			))),
		},
		Value::Float(value) => Ok(round(num, -value)),
		Value::Percentage(value) => Ok(Value::Percentage(-value)),
		other => Err(Error::Stop(format!(
			"`{}` is not a number",
			other.to_text()
		))),
	}
}

/// A conversion to a number type.
pub fn convert(num: Num, value: &Value) -> Result<Value, Error> {
	match (num, value) {
		(Num::Int { .. }, Value::Integer(integer)) => {
			if in_range(num, *integer) {
				Ok(value.clone())
			} else {
				Err(arithmetic_error(&format!(
					"{integer} is out of the range of the converted type"
				)))
			}
		}
		(Num::Int { .. }, Value::Float(float)) => {
			if float.fract() == 0.0 && float.is_finite() {
				let integer = I256::from(*float as i128);
				if in_range(num, integer) {
					return Ok(Value::Integer(integer));
				}
			}
			Err(Error::Stop(format!(
				"{} can't be converted to an integer",
				crate::value::format_float(*float)
			)))
		}
		(Num::Float | Num::Float32, Value::Integer(integer)) => Ok(round(num, integer.as_f64())),
		(Num::Float | Num::Float32, Value::Float(float) | Value::Percentage(float)) => {
			Ok(round(num, *float))
		}
		(Num::Percentage, other) => Ok(Value::Percentage(other.as_f64().unwrap_or(f64::NAN))),
		(_, other) => Err(Error::Stop(format!(
			"`{}` can't be converted to a number",
			other.to_text()
		))),
	}
}

/// An ordering comparison of two numbers, strings or characters.
pub fn compare(op: CompareOp, left: &Value, right: &Value) -> Result<bool, Error> {
	use std::cmp::Ordering;
	let ordering = match (left, right) {
		(Value::Integer(a), Value::Integer(b)) => Some(a.cmp(b)),
		(Value::String(a), Value::String(b)) => Some(a.cmp(b)),
		_ => match (left.as_f64(), right.as_f64()) {
			(Some(a), Some(b)) => a.partial_cmp(&b),
			_ => {
				return Err(Error::Stop(format!(
					"`{}` and `{}` can't be ordered",
					left.to_text(),
					right.to_text()
				)));
			}
		},
	};
	// NaN: every comparison is false
	let Some(ordering) = ordering else {
		return Ok(false);
	};
	Ok(match op {
		CompareOp::Less => ordering == Ordering::Less,
		CompareOp::LessOrEqual => ordering != Ordering::Greater,
		CompareOp::Greater => ordering == Ordering::Greater,
		CompareOp::GreaterOrEqual => ordering != Ordering::Less,
	})
}

/// Whether a value is a member of a kind.
pub fn is_member(value: &Value, kind: MemberKind) -> bool {
	matches!(value, Value::Member(member) if member.kind == kind)
}
