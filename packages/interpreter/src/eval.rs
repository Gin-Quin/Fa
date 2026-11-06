//! The evaluator: checked IR executed by explicit continuations and a step budget.
//!
//! Every evaluated expression is one step (decision D26); a call of a
//! function deeper than [`Limits::depth`] is reported like the budget
//! independently of the native stack. Errors thrown by
//! the code ([`Error::Thrown`]) and stops ([`Error::Stop`],
//! [`Error::Overflow`]) end the evaluation: the analyzer reports them at the
//! `@` call (`META005`, `META013`).

use std::collections::HashMap;
use std::sync::Arc;

use ethnum::I256;

use crate::code::Code;
use crate::ir::{ArithOp, CompareOp, Expr, FnKey, Function, Num};
use crate::machine::Machine;
use crate::memory;
use crate::value::{MemberKind, Value};

/// The default of [`Limits::memory`]: 256 MB.
pub const DEFAULT_MEMORY: usize = 256 << 20;

/// The bounds of one evaluation (advanced brief §12.3).
#[derive(Copy, Clone, Debug)]
pub struct Limits {
	/// The number of evaluated expressions (100 million by default).
	pub steps: u64,
	/// The depth of nested calls.
	pub depth: u32,
	/// The bytes of memory an evaluation may use (256 MB by default, less
	/// when the address space is limited: see [`crate::memory`]).
	pub memory: usize,
}

impl Default for Limits {
	fn default() -> Self {
		Self {
			steps: 100_000_000,
			depth: 10_000,
			memory: memory::default_budget(),
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
	/// More memory than [`Limits::memory`] bytes (`META003`).
	Memory(usize),
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
			Error::Memory(limit) => format!("more than {} MB of memory", limit >> 20),
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
	pub(crate) host: &'h mut dyn Host,
	pub(crate) limits: Limits,
	steps: u64,
	pub(crate) depth: u32,
	compiled: HashMap<FnKey, Arc<Code>>,
	/// The heap this thread had allocated when the evaluation started
	/// ([`crate::memory`]).
	baseline: usize,
}

impl<'h> Interpreter<'h> {
	/// An interpreter asking `host` for functions and globals.
	pub fn new(host: &'h mut dyn Host, limits: Limits) -> Self {
		Self {
			host,
			limits,
			steps: 0,
			depth: 0,
			compiled: HashMap::new(),
			baseline: memory::allocated(),
		}
	}

	/// The steps evaluated so far.
	pub fn steps(&self) -> u64 {
		self.steps
	}

	/// Evaluates an expression lowered with `slots` locals.
	pub fn evaluate(&mut self, expr: &Expr, slots: u32) -> Result<Value, Error> {
		let code = Arc::new(Code::expression(expr, slots, || self.reserve(0))?);
		match Machine::evaluate(self, code) {
			Ok(value) | Err(Flow::Return(value)) | Err(Flow::Break(value)) => Ok(value),
			Err(Flow::Continue) => Ok(Value::None),
			Err(Flow::Error(error)) => Err(error),
		}
	}

	/// Calls a function value with its arguments in parameter order.
	pub fn call_value(&mut self, callee: &Value, arguments: Vec<Value>) -> Result<Value, Error> {
		Machine::call(
			self,
			callee.clone(),
			arguments.into_iter().map(Some).collect(),
		)
	}

	pub(crate) fn code(&mut self, key: FnKey) -> Result<Arc<Code>, Error> {
		if let Some(code) = self.compiled.get(&key) {
			return Ok(Arc::clone(code));
		}
		let function = self.host.function(key)?;
		let code = Arc::new(Code::function(&function, || self.reserve(0))?);
		self.compiled.insert(key, Arc::clone(&code));
		Ok(code)
	}

	pub(crate) fn callback_builtin(
		&mut self,
		path: &str,
		arguments: Vec<Value>,
	) -> Result<Value, Error> {
		Machine::builtin(self, path, arguments)
	}

	pub(crate) fn step(&mut self) -> Result<(), Flow> {
		self.steps += 1;
		if self.steps > self.limits.steps {
			return Err(Flow::Error(Error::Steps(self.limits.steps)));
		}
		if self.steps.is_multiple_of(1024) {
			self.reserve(0)?;
		}
		Ok(())
	}

	/// [`Interpreter::reserve`] for a text of `bytes`, built as a `String`
	/// then copied into the `Arc<str>` of its value.
	pub fn reserve_text(&self, bytes: usize) -> Result<(), Error> {
		self.reserve(bytes.saturating_mul(2))
	}

	/// Checks that `bytes` more of memory stay within [`Limits::memory`],
	/// before building a value of that size (see [`crate::memory`]).
	pub fn reserve(&self, bytes: usize) -> Result<(), Error> {
		// what this thread allocated since the start (it may have freed
		// more than that: then nothing)
		let grown = memory::allocated().wrapping_sub(self.baseline);
		let used = if grown > isize::MAX as usize {
			0
		} else {
			grown
		};
		if used.saturating_add(bytes) > self.limits.memory {
			return Err(Error::Memory(self.limits.memory));
		}
		Ok(())
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
