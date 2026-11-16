//! The evaluator's explicit continuation and call stacks. No Fa call,
//! default argument, callback, or expression evaluation recurses in Rust.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ethnum::I256;

use crate::callbacks::{Callback, Progress};
use crate::code::{Argument, Call, Code, Item, Node, Op, Part, Place, Step};
use crate::eval::{self, Error, Interpreter};
use crate::ir::{ArithOp, CompareOp, CopyPlan, FnKey, Num, Slot, TypeTest};
use crate::loans::{Loan, Out, Owner, Step as PlaceStep};
use crate::value::{Closure, Member, MemberKind, Object, Value, equals};
use crate::{builtins, memory};

static FRAME_GENERATION: AtomicU64 = AtomicU64::new(1);

type Eval = Result<Out, Flow>;

#[derive(Debug)]
enum Flow {
	Break(Out),
	Continue,
	Return(Out),
	Error(Error),
}

impl From<Error> for Flow {
	fn from(error: Error) -> Self {
		Self::Error(error)
	}
}

impl From<eval::Flow> for Flow {
	fn from(flow: eval::Flow) -> Self {
		match flow {
			eval::Flow::Break(value) => Self::Break(value.into()),
			eval::Flow::Continue => Self::Continue,
			eval::Flow::Return(value) => Self::Return(value.into()),
			eval::Flow::Error(error) => Self::Error(error),
		}
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

enum Action {
	Eval(Node),
	Done(Eval),
	Call(Target, Vec<Option<Out>>),
}

enum Target {
	Function(FnKey),
	Value(Value),
}

struct Frame {
	generation: u64,
	code: Arc<Code>,
	locals: Vec<Out>,
}

enum Input {
	Expr(Node),
	Constant(Value),
	Spread(Node),
	Default,
	Variadic(Vec<Item>),
}

struct Gather {
	remaining: std::vec::IntoIter<Input>,
	values: Vec<Option<Out>>,
	spread: bool,
	finish: Finish,
}

enum Finish {
	Arguments(Target, Vec<usize>),
	Builtin(&'static str),
	Mutate(&'static str, Place),
	Array,
	Set,
	Map,
	Tuple,
	Object(Vec<Arc<str>>),
	Template,
	Range(bool),
	Place(Place, PlaceAction),
}

enum PlaceAction {
	Read,
	Lend,
	Write(Value),
	Mutate(&'static str, Vec<Value>),
}

struct ResolvedPlace {
	slot: Slot,
	steps: Vec<PlaceStep>,
}

enum Binary {
	Arith(ArithOp, Num),
	Equals(bool),
	Compare(CompareOp),
	IsIn(bool),
	Index,
}

enum Unary {
	Negate(Num),
	Convert(Num),
	Copy(Arc<CopyPlan>),
	Not,
	Boolean,
	Is(TypeTest, bool),
	MemberData,
	Member(Arc<str>, MemberKind),
	Field(Arc<str>, bool),
	Assert(Arc<str>),
	Let(Slot),
	Bind(Slot),
	Break,
	Return,
	Throw,
}

enum Iteration {
	Range {
		current: I256,
		end: Option<I256>,
		inclusive: bool,
		finished: bool,
	},
	Values {
		values: Arc<Vec<Value>>,
		position: usize,
	},
}

struct For {
	item: Option<Slot>,
	index: Option<Slot>,
	position: i64,
	values: Iteration,
	body: Node,
}

enum CallbackFinish {
	Value,
	Mutate(Loan),
}

enum Continuation {
	Block(std::vec::IntoIter<Node>),
	If(Node, Option<Node>),
	Unary(Unary),
	BinaryLeft(Binary, Node),
	BinaryRight(Binary, Out),
	ShortCircuit {
		right: Node,
		short: bool,
	},
	Fallback(Node),
	Callee(Call),
	Gather(Gather),
	Assign(Place),
	Loop(Node),
	WhileCondition(Node, Node),
	WhileBody(Node, Node),
	ForIterable(Option<Slot>, Option<Slot>, Node),
	ForBody(For),
	Catch(Option<Slot>, Node),
	Function,
	Default {
		arguments: std::vec::IntoIter<Option<Out>>,
		parameter: usize,
		slot: Slot,
	},
	Callback(Box<Callback>, CallbackFinish),
}

pub(crate) struct Machine<'i, 'h> {
	interpreter: &'i mut Interpreter<'h>,
	frames: Vec<Frame>,
	stack: Vec<Continuation>,
}

impl<'i, 'h> Machine<'i, 'h> {
	pub fn evaluate(interpreter: &'i mut Interpreter<'h>, code: Arc<Code>) -> eval::Eval {
		let body = code.body;
		Self::new(interpreter, code)?.run(Action::Eval(body))
	}

	pub fn call(
		interpreter: &'i mut Interpreter<'h>,
		callee: Value,
		arguments: Vec<Option<Value>>,
	) -> Result<Value, Error> {
		let code = Arc::new(Code::expression(&crate::ir::Expr::none(), 0, || Ok(()))?);
		Self::new(interpreter, code)?
			.run(Action::Call(
				Target::Value(callee),
				arguments
					.into_iter()
					.map(|value| value.map(Out::Value))
					.collect(),
			))
			.map_err(Error::from)
	}

	pub fn builtin(
		interpreter: &'i mut Interpreter<'h>,
		path: &str,
		arguments: Vec<Value>,
	) -> Result<Value, Error> {
		let code = Arc::new(Code::expression(&crate::ir::Expr::none(), 0, || Ok(()))?);
		let mut machine = Self::new(interpreter, code)?;
		let action = machine
			.start_callback(path, arguments, CallbackFinish::Value)
			.unwrap_or_else(|flow| Action::Done(Err(flow)));
		machine.run(action).map_err(Error::from)
	}

	fn new(interpreter: &'i mut Interpreter<'h>, code: Arc<Code>) -> Result<Self, Error> {
		interpreter.reserve((code.slots as usize).saturating_mul(size_of::<Out>()))?;
		let locals = vec![Out::Value(Value::None); code.slots as usize];
		Ok(Self {
			interpreter,
			frames: vec![Frame {
				generation: FRAME_GENERATION.fetch_add(1, Ordering::Relaxed),
				code,
				locals,
			}],
			stack: Vec::new(),
		})
	}

	fn run(mut self, mut action: Action) -> eval::Eval {
		loop {
			let next = match action {
				Action::Eval(node) => self.enter(node),
				Action::Call(target, arguments) => self.call_function(target, arguments),
				Action::Done(result) => {
					let Some(continuation) = self.stack.pop() else {
						return result
							.and_then(|value| self.value(value))
							.map_err(|flow| eval::Flow::Error(Error::from(flow)));
					};
					self.resume(continuation, result)
				}
			};
			action = next.unwrap_or_else(|flow| Action::Done(Err(flow)));
		}
	}

	fn frame(&self) -> &Frame {
		self.frames.last().expect("an evaluation frame")
	}
	fn locals(&mut self) -> &mut Vec<Out> {
		&mut self.frames.last_mut().expect("an evaluation frame").locals
	}
	fn done(value: impl Into<Out>) -> Result<Action, Flow> {
		Ok(Action::Done(Ok(value.into())))
	}
	fn evaluate_after(&mut self, node: Node, continuation: Continuation) -> Result<Action, Flow> {
		self.stack.push(continuation);
		Ok(Action::Eval(node))
	}

	fn value(&self, value: Out) -> Result<Value, Flow> {
		match value {
			Out::Value(value) => Ok(value),
			Out::Loan(loan) => Ok(self.read_loan(&loan)?.clone()),
		}
	}

	fn read_loan(&self, loan: &Loan) -> Result<&Value, Flow> {
		self.interpreter.reserve(loan.lookup_bytes())?;
		let frame = self
			.frames
			.get(loan.owner.frame)
			.filter(|frame| frame.generation == loan.owner.generation)
			.ok_or_else(|| Error::Stop("internal: a reference outlived its frame".into()))?;
		let Some(Out::Value(mut_value)) = frame.locals.get(loan.owner.slot as usize) else {
			return Err(
				Error::Stop("internal: a loan must designate an owning root".into()).into(),
			);
		};
		let mut value = mut_value;
		for step in loan.steps() {
			value = match step {
				PlaceStep::Field(name) => builtins::field_ref(value, name)?,
				PlaceStep::Index(index) => builtins::index_ref(value, index)?,
				PlaceStep::Capture(index) => Self::environment_value(value, *index, false)?,
				PlaceStep::Bound(index) => Self::environment_value(value, *index, true)?,
			};
		}
		Ok(value)
	}

	fn environment_value(value: &Value, index: usize, bound: bool) -> Result<&Value, Flow> {
		let Value::Function(closure) = value else {
			return Err(
				Error::Stop("internal: an environment root is not a function".into()).into(),
			);
		};
		let value = if bound {
			closure.bound.get(index).map(|(_, value)| value)
		} else {
			closure.captured.get(index)
		};
		value.ok_or_else(|| Error::Stop("internal: an environment entry is missing".into()).into())
	}

	fn loan(&self, place: &ResolvedPlace) -> Result<Loan, Flow> {
		self.interpreter
			.reserve(Loan::projection_bytes(place.steps.len()))?;
		let frame = self.frame();
		let mut loan = match &frame.locals[place.slot as usize] {
			Out::Loan(loan) => {
				self.read_loan(loan)?;
				loan.clone()
			}
			Out::Value(_) => Loan::new(Owner {
				frame: self.frames.len() - 1,
				generation: frame.generation,
				slot: place.slot,
			}),
		};
		for step in &place.steps {
			loan.push(step.clone());
		}
		Ok(loan)
	}

	fn enter(&mut self, node: Node) -> Result<Action, Flow> {
		self.interpreter.step()?;
		// The Arc keeps this immutable code alive while calls below may ask
		// the host to prepare another function.
		let code = Arc::clone(&self.frame().code);
		match &code.nodes[node] {
			Op::Const(value) => Self::done(value.clone()),
			Op::Local(slot) => Self::done(self.value(self.frame().locals[*slot as usize].clone())?),
			Op::Move(slot) => {
				if matches!(self.frame().locals[*slot as usize], Out::Loan(_)) {
					return Err(Error::Stop("internal: cannot move through a borrow".into()).into());
				}
				Self::done(std::mem::replace(
					&mut self.locals()[*slot as usize],
					Out::Value(Value::None),
				))
			}
			Op::Copy(plan, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Copy(Arc::clone(plan))))
			}
			Op::Global(key) => Self::done(self.interpreter.host.global(*key)?),
			Op::Function(function) => Self::done(Value::Function(Arc::new(Closure::new(
				*function,
				Vec::new(),
				Vec::new(),
			)))),
			Op::Closure(function, captures, borrow) => {
				self.interpreter
					.reserve(captures.len().saturating_mul(if *borrow {
						size_of::<Loan>()
					} else {
						size_of::<Value>()
					}))?;
				let mut closure = Closure::new(*function, Vec::new(), Vec::new());
				if *borrow && !captures.is_empty() {
					closure.environment = Some(
						captures
							.iter()
							.map(|&slot| {
								self.loan(&ResolvedPlace {
									slot,
									steps: Vec::new(),
								})
							})
							.collect::<Result<_, _>>()?,
					);
				} else {
					closure.captured = captures
						.iter()
						.map(|&slot| self.value(self.frame().locals[slot as usize].clone()))
						.collect::<Result<_, _>>()?;
				}
				Self::done(Value::Function(Arc::new(closure)))
			}
			Op::Let(slot, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Let(*slot)))
			}
			Op::Assign(place, value) => {
				self.evaluate_after(*value, Continuation::Assign(place.clone()))
			}
			Op::Read(place) => self.place(place.clone(), PlaceAction::Read),
			Op::Lend(place) => self.place(place.clone(), PlaceAction::Lend),
			Op::Bind(slot, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Bind(*slot)))
			}
			Op::Block(nodes) => {
				let mut nodes = nodes.clone().into_iter();
				match nodes.next() {
					Some(node) => self.evaluate_after(node, Continuation::Block(nodes)),
					None => Self::done(Value::None),
				}
			}
			Op::If(condition, then, otherwise) => {
				self.evaluate_after(*condition, Continuation::If(*then, *otherwise))
			}
			Op::Loop(body) => self.evaluate_after(*body, Continuation::Loop(*body)),
			Op::While(condition, body) => {
				self.evaluate_after(*condition, Continuation::WhileCondition(*condition, *body))
			}
			Op::For {
				item,
				index,
				iterable,
				body,
			} => self.evaluate_after(*iterable, Continuation::ForIterable(*item, *index, *body)),
			Op::Break(value) => match value {
				Some(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Break)),
				None => Err(Flow::Break(Value::None.into())),
			},
			Op::Continue => Err(Flow::Continue),
			Op::Return(value) => match value {
				Some(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Return)),
				None => Err(Flow::Return(Value::None.into())),
			},
			Op::Throw(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Throw)),
			Op::Catch {
				body,
				slot,
				handler,
			} => self.evaluate_after(*body, Continuation::Catch(*slot, *handler)),
			Op::Assert(condition, text) => self.evaluate_after(
				*condition,
				Continuation::Unary(Unary::Assert(Arc::clone(text))),
			),
			Op::Arith(op, num, left, right) => self.binary(Binary::Arith(*op, *num), *left, *right),
			Op::Negate(num, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Negate(*num)))
			}
			Op::Convert(num, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Convert(*num)))
			}
			Op::Equals(left, right, negated) => {
				self.binary(Binary::Equals(*negated), *left, *right)
			}
			Op::Compare(op, left, right) => self.binary(Binary::Compare(*op), *left, *right),
			Op::And(left, right) => self.evaluate_after(
				*left,
				Continuation::ShortCircuit {
					right: *right,
					short: false,
				},
			),
			Op::Or(left, right) => self.evaluate_after(
				*left,
				Continuation::ShortCircuit {
					right: *right,
					short: true,
				},
			),
			Op::Not(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Not)),
			Op::Fallback(left, right) => self.evaluate_after(*left, Continuation::Fallback(*right)),
			Op::Is(value, test, negated) => self.evaluate_after(
				*value,
				Continuation::Unary(Unary::Is(test.clone(), *negated)),
			),
			Op::IsIn(value, collection, negated) => {
				self.binary(Binary::IsIn(*negated), *value, *collection)
			}
			Op::Call(callee, arguments) => {
				self.evaluate_after(*callee, Continuation::Callee(arguments.clone()))
			}
			Op::CallFunction(key, arguments) => {
				self.arguments(Target::Function(*key), arguments.clone())
			}
			Op::Builtin(path, arguments) => self.gather(
				arguments.iter().map(|&node| Input::Expr(node)).collect(),
				Finish::Builtin(path),
			),
			Op::Mutate(path, place, arguments) => self.gather(
				arguments.iter().map(|&node| Input::Expr(node)).collect(),
				Finish::Mutate(path, place.clone()),
			),
			Op::Array(items) => self.gather_items(items.clone(), Finish::Array),
			Op::Set(items) => self.gather_items(items.clone(), Finish::Set),
			Op::Map(items) => self.gather_items(items.clone(), Finish::Map),
			Op::Tuple(nodes) => self.gather(
				nodes.iter().map(|&node| Input::Expr(node)).collect(),
				Finish::Tuple,
			),
			Op::Object(fields) => self.gather(
				fields.iter().map(|(_, node)| Input::Expr(*node)).collect(),
				Finish::Object(fields.iter().map(|(name, _)| Arc::clone(name)).collect()),
			),
			Op::Member { name, kind, data } => match data {
				Some(data) => self.evaluate_after(
					*data,
					Continuation::Unary(Unary::Member(Arc::clone(name), *kind)),
				),
				None => Self::done(Value::Member(Arc::new(Member {
					name: Arc::clone(name),
					kind: *kind,
					data: None,
				}))),
			},
			Op::MemberData(value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::MemberData))
			}
			Op::Field(value, name, optional) => self.evaluate_after(
				*value,
				Continuation::Unary(Unary::Field(Arc::clone(name), *optional)),
			),
			Op::Index(value, index) => self.binary(Binary::Index, *value, *index),
			Op::Range(start, end, inclusive) => {
				let start = start
					.map(Input::Expr)
					.unwrap_or(Input::Constant(Value::None));
				let end = end.map(Input::Expr).unwrap_or(Input::Constant(Value::None));
				self.gather(vec![start, end], Finish::Range(*inclusive))
			}
			Op::Template(parts) => self.gather(
				parts
					.iter()
					.map(|part| match part {
						Part::Text(text) => Input::Constant(Value::String(Arc::clone(text))),
						Part::Value(node) => Input::Expr(*node),
					})
					.collect(),
				Finish::Template,
			),
			Op::Impure(what) => Err(Error::Impure(what.to_string()).into()),
			Op::Unsupported(what) => Err(Error::Unsupported(what.to_string()).into()),
		}
	}

	fn binary(&mut self, op: Binary, left: Node, right: Node) -> Result<Action, Flow> {
		self.evaluate_after(left, Continuation::BinaryLeft(op, right))
	}

	fn arguments(&mut self, target: Target, call: Call) -> Result<Action, Flow> {
		let order = if call.order.is_empty() {
			(0..call.arguments.len()).collect()
		} else {
			call.order
		};
		let mut arguments: Vec<_> = call.arguments.into_iter().map(Some).collect();
		let mut inputs = Vec::with_capacity(arguments.len());
		for &parameter in &order {
			let argument = arguments
				.get_mut(parameter)
				.and_then(Option::take)
				.ok_or_else(|| Error::Stop("internal: invalid argument evaluation order".into()))?;
			inputs.push(match argument {
				Argument::Value(node) => Input::Expr(node),
				Argument::Default => Input::Default,
				Argument::Variadic(items) => Input::Variadic(items),
			});
		}
		if inputs.len() != arguments.len() {
			return Err(
				Error::Stop("internal: incomplete argument evaluation order".into()).into(),
			);
		}
		self.gather(inputs, Finish::Arguments(target, order))
	}

	fn gather_items(&mut self, items: Vec<Item>, finish: Finish) -> Result<Action, Flow> {
		self.gather(
			items
				.into_iter()
				.map(|item| {
					if item.spread {
						Input::Spread(item.node)
					} else {
						Input::Expr(item.node)
					}
				})
				.collect(),
			finish,
		)
	}

	fn gather(&mut self, inputs: Vec<Input>, finish: Finish) -> Result<Action, Flow> {
		let values = Vec::with_capacity(inputs.len());
		self.gather_next(Gather {
			remaining: inputs.into_iter(),
			values,
			spread: false,
			finish,
		})
	}

	fn gather_next(&mut self, mut gather: Gather) -> Result<Action, Flow> {
		while let Some(input) = gather.remaining.next() {
			match input {
				Input::Constant(value) => gather.values.push(Some(value.into())),
				Input::Default => gather.values.push(None),
				Input::Expr(node) | Input::Spread(node) => {
					gather.spread = matches!(input, Input::Spread(_));
					return self.evaluate_after(node, Continuation::Gather(gather));
				}
				Input::Variadic(items) => {
					gather.spread = false;
					self.stack.push(Continuation::Gather(gather));
					return self.gather_items(items, Finish::Array);
				}
			}
		}
		self.finish(gather.finish, gather.values)
	}

	fn place(&mut self, place: Place, action: PlaceAction) -> Result<Action, Flow> {
		let inputs = place
			.steps
			.iter()
			.filter_map(|step| match step {
				Step::Index(node) => Some(Input::Expr(*node)),
				_ => None,
			})
			.collect();
		self.gather(inputs, Finish::Place(place, action))
	}

	fn finish(&mut self, finish: Finish, values: Vec<Option<Out>>) -> Result<Action, Flow> {
		if let Finish::Arguments(target, order) = finish {
			let mut arguments = vec![None; values.len()];
			for (parameter, value) in order.into_iter().zip(values) {
				arguments[parameter] = value;
			}
			return Ok(Action::Call(target, arguments));
		}
		let values: Vec<Value> = values
			.into_iter()
			.map(|value| self.value(value.expect("a supplied value")))
			.collect::<Result<_, _>>()?;
		match finish {
			Finish::Arguments(..) => unreachable!(),
			Finish::Array => Self::done(Value::Array(Arc::new(values))),
			Finish::Tuple => Self::done(Value::Tuple(Arc::new(values))),
			Finish::Set => {
				let mut set = Vec::new();
				for value in values {
					if !crate::value::contains(&set, &value) {
						set.push(value);
					}
				}
				Self::done(Value::Set(Arc::new(set)))
			}
			Finish::Map => {
				let mut entries = Vec::new();
				for value in values {
					let (key, value) = builtins::pair(value)?;
					builtins::map_insert(&mut entries, key, value);
				}
				Self::done(Value::Map(Arc::new(entries)))
			}
			Finish::Object(names) => Self::done(Value::Object(Arc::new(Object {
				fields: names.into_iter().zip(values).collect(),
			}))),
			Finish::Template => {
				let pieces: Vec<Arc<str>> = values
					.into_iter()
					.map(|value| match value {
						Value::String(text) => text,
						other => Arc::from(other.to_text()),
					})
					.collect();
				let size = pieces
					.iter()
					.fold(0usize, |size, piece| size.saturating_add(piece.len()));
				self.interpreter.reserve_text(size)?;
				let mut text = String::with_capacity(size);
				for piece in pieces {
					text.push_str(&piece);
				}
				Self::done(Value::String(Arc::from(text)))
			}
			Finish::Range(inclusive) => {
				let mut values = values.into_iter();
				Self::done(Value::Range {
					start: range_bound(values.next().expect("a start"))?,
					end: range_bound(values.next().expect("an end"))?,
					inclusive,
				})
			}
			Finish::Builtin(path) => {
				if Callback::handles(path) {
					self.start_callback(path, values, CallbackFinish::Value)
				} else {
					Self::done(builtins::call_internal(self.interpreter, path, values)?)
				}
			}
			Finish::Mutate(path, place) => self.place(place, PlaceAction::Mutate(path, values)),
			Finish::Place(place, action) => {
				let mut keys = values.into_iter();
				let resolved = ResolvedPlace {
					slot: place.slot,
					steps: place
						.steps
						.into_iter()
						.map(|step| match step {
							Step::Field(name) => PlaceStep::Field(name),
							Step::Index(_) => PlaceStep::Index(keys.next().expect("a place index")),
						})
						.collect(),
				};
				let loan = self.loan(&resolved)?;
				match action {
					PlaceAction::Read => Self::done(self.read_loan(&loan)?.clone()),
					PlaceAction::Lend => {
						self.read_loan(&loan)?;
						Self::done(Out::Loan(loan))
					}
					PlaceAction::Write(value) => {
						*Self::locate(&mut self.frames, &loan)? = value;
						Self::done(Value::None)
					}
					PlaceAction::Mutate(path, mut arguments) if Callback::mutating(path) => {
						let receiver = self.read_loan(&loan)?.clone();
						arguments.insert(0, receiver);
						self.start_callback(path, arguments, CallbackFinish::Mutate(loan))
					}
					PlaceAction::Mutate(path, arguments) => Self::done(builtins::mutate_internal(
						self.interpreter,
						path,
						Self::locate(&mut self.frames, &loan)?,
						arguments,
					)?),
				}
			}
		}
	}

	fn locate<'f>(frames: &'f mut [Frame], place: &Loan) -> Result<&'f mut Value, Flow> {
		let frame = frames
			.get_mut(place.owner.frame)
			.filter(|frame| frame.generation == place.owner.generation)
			.ok_or_else(|| Error::Stop("internal: a reference outlived its frame".into()))?;
		let Some(Out::Value(mut_value)) = frame.locals.get_mut(place.owner.slot as usize) else {
			return Err(
				Error::Stop("internal: a loan must designate an owning root".into()).into(),
			);
		};
		let mut current = mut_value;
		for step in place.steps() {
			current = match step {
				PlaceStep::Field(name) => builtins::field_mut(current, name)?,
				PlaceStep::Index(key) => builtins::index_mut(current, key.clone())?,
				PlaceStep::Capture(_) | PlaceStep::Bound(_) => {
					return Err(
						Error::Stop("internal: cannot modify an owned capture".into()).into(),
					);
				}
			};
		}
		Ok(current)
	}

	fn call_function(
		&mut self,
		target: Target,
		mut arguments: Vec<Option<Out>>,
	) -> Result<Action, Flow> {
		let (key, closure) = match target {
			Target::Function(key) => (key, None),
			Target::Value(Value::Function(closure)) => (closure.function, Some(closure)),
			Target::Value(other) => {
				return Err(Error::Stop(format!("`{}` is not a function", other.to_text())).into());
			}
		};
		let code = self.interpreter.code(key)?;
		if self.interpreter.depth >= self.interpreter.limits.depth {
			return Err(Error::Depth(self.interpreter.limits.depth).into());
		}
		let extra = usize::from(closure.is_some());
		self.interpreter.reserve(Loan::projection_bytes(
			code.captures.len() + code.params.len(),
		))?;
		self.interpreter
			.reserve((code.slots as usize + extra).saturating_mul(size_of::<Out>()))?;
		let generation = FRAME_GENERATION.fetch_add(1, Ordering::Relaxed);
		let owner = Owner {
			frame: self.frames.len(),
			generation,
			slot: code.slots,
		};
		let mut locals = vec![Out::Value(Value::None); code.slots as usize];
		if let Some(closure) = closure {
			let count = closure
				.environment
				.as_ref()
				.map_or(closure.captured.len(), Vec::len);
			if count != code.captures.len() {
				return Err(Error::Stop(
					"internal: a callable's capture count differs from its code".into(),
				)
				.into());
			}
			for (index, &slot) in code.captures.iter().enumerate() {
				let loan = if let Some(environment) = &closure.environment {
					let loan = &environment[index];
					self.read_loan(loan)?;
					loan.clone()
				} else {
					let mut loan = Loan::new(owner);
					loan.push(PlaceStep::Capture(index));
					loan
				};
				locals[slot as usize] = Out::Loan(loan);
			}
			if !closure.bound.is_empty() {
				let mut given = arguments.into_iter();
				arguments = (0..code.params.len())
					.map(|parameter| {
						match closure
							.bound
							.iter()
							.position(|(bound, _)| *bound == parameter)
						{
							Some(index) => {
								let mut loan = Loan::new(owner);
								loan.push(PlaceStep::Bound(index));
								Some(Out::Loan(loan))
							}
							None => given.next().flatten(),
						}
					})
					.collect();
			}
			locals.push(Value::Function(closure).into());
		} else if !code.captures.is_empty() {
			return Err(
				Error::Stop("internal: a capturing function has no environment".into()).into(),
			);
		}
		self.interpreter.depth += 1;
		self.frames.push(Frame {
			generation,
			code,
			locals,
		});
		self.stack.push(Continuation::Function);
		self.parameters(arguments.into_iter(), 0)
	}

	fn parameters(
		&mut self,
		mut arguments: std::vec::IntoIter<Option<Out>>,
		mut index: usize,
	) -> Result<Action, Flow> {
		while let Some(&(slot, default)) = self.frame().code.params.get(index) {
			match (arguments.next().flatten(), default) {
				(Some(value), _) => self.locals()[slot as usize] = value,
				(None, Some(default)) => {
					return self.evaluate_after(
						default,
						Continuation::Default {
							arguments,
							parameter: index + 1,
							slot,
						},
					);
				}
				(None, None) => self.locals()[slot as usize] = Value::None.into(),
			}
			index += 1;
		}
		Ok(Action::Eval(self.frame().code.body))
	}

	fn start_callback(
		&mut self,
		path: &str,
		arguments: Vec<Value>,
		finish: CallbackFinish,
	) -> Result<Action, Flow> {
		if let Some(receiver) = arguments.first() {
			self.interpreter
				.reserve(memory::values_size(builtins::count(receiver).unwrap_or(0)))?;
		}
		self.callback(Callback::new(path, arguments)?, None, finish)
	}

	fn callback(
		&mut self,
		callback: Box<Callback>,
		value: Option<Value>,
		finish: CallbackFinish,
	) -> Result<Action, Flow> {
		match callback.advance(value)? {
			Progress::Call {
				function,
				arguments,
				next,
			} => {
				self.stack.push(Continuation::Callback(next, finish));
				Ok(Action::Call(
					Target::Value(function),
					arguments
						.into_iter()
						.map(|value| Some(value.into()))
						.collect(),
				))
			}
			Progress::Done(value) => match finish {
				CallbackFinish::Value => Self::done(value),
				CallbackFinish::Mutate(place) => {
					*Self::locate(&mut self.frames, &place)? = value;
					Self::done(Value::None)
				}
			},
		}
	}

	fn resume(&mut self, continuation: Continuation, result: Eval) -> Result<Action, Flow> {
		// Only these boundaries intercept control flow. Every other pending
		// operation propagates it without performing another side effect.
		match continuation {
			Continuation::Function => {
				let result = match result {
					Ok(value) => self.returned(value),
					Err(Flow::Return(value)) => self.returned(value),
					other => other,
				};
				self.frames.pop().expect("the called frame");
				self.interpreter.depth -= 1;
				match result {
					Ok(value) | Err(Flow::Return(value)) => Self::done(value),
					Err(Flow::Break(_) | Flow::Continue) => Self::done(Value::None),
					Err(flow) => Err(flow),
				}
			}
			Continuation::Default {
				arguments,
				parameter,
				slot,
			} => {
				let value = match result {
					Ok(value) => value,
					Err(Flow::Error(error)) => return Err(error.into()),
					Err(_) => Value::None.into(),
				};
				self.locals()[slot as usize] = value;
				self.parameters(arguments, parameter)
			}
			Continuation::Loop(body) => match result {
				Ok(_) | Err(Flow::Continue) => self.evaluate_after(body, Continuation::Loop(body)),
				Err(Flow::Break(value)) => Self::done(value),
				Err(flow) => Err(flow),
			},
			Continuation::WhileBody(condition, body) => match result {
				Ok(_) | Err(Flow::Continue) => {
					self.evaluate_after(condition, Continuation::WhileCondition(condition, body))
				}
				Err(Flow::Break(_)) => Self::done(Value::None),
				Err(flow) => Err(flow),
			},
			Continuation::ForBody(state) => match result {
				Ok(_) | Err(Flow::Continue) => self.for_next(state),
				Err(Flow::Break(_)) => Self::done(Value::None),
				Err(flow) => Err(flow),
			},
			Continuation::Catch(slot, handler) => match result {
				Err(Flow::Error(Error::Thrown(error))) => {
					if let Some(slot) = slot {
						self.locals()[slot as usize] = error.into();
					}
					Ok(Action::Eval(handler))
				}
				other => Ok(Action::Done(other)),
			},
			other => {
				let value = result?;
				match other {
					Continuation::Block(mut nodes) => match nodes.next() {
						Some(node) => self.evaluate_after(node, Continuation::Block(nodes)),
						None => Self::done(value),
					},
					Continuation::If(then, otherwise) => match boolean(self.value(value)?)? {
						true => Ok(Action::Eval(then)),
						false => match otherwise {
							Some(node) => Ok(Action::Eval(node)),
							None => Self::done(Value::None),
						},
					},
					Continuation::Unary(op) => self.unary(op, value),
					Continuation::BinaryLeft(op, right) => {
						self.evaluate_after(right, Continuation::BinaryRight(op, value))
					}
					Continuation::BinaryRight(op, left) => self.binary_result(op, left, value),
					Continuation::ShortCircuit { right, short } => {
						if boolean(self.value(value)?)? == short {
							Self::done(Value::Boolean(short))
						} else {
							self.evaluate_after(right, Continuation::Unary(Unary::Boolean))
						}
					}
					Continuation::Fallback(right) => {
						let none = match &value {
							Out::Value(value) => matches!(value, Value::None),
							Out::Loan(loan) => matches!(self.read_loan(loan)?, Value::None),
						};
						if none {
							Ok(Action::Eval(right))
						} else {
							Self::done(value)
						}
					}
					Continuation::Callee(arguments) => {
						self.arguments(Target::Value(self.value(value)?), arguments)
					}
					Continuation::Gather(mut gather) => {
						if gather.spread {
							let value = self.value(value)?;
							self.interpreter.reserve(memory::values_size(
								gather
									.values
									.len()
									.saturating_add(builtins::count(&value).unwrap_or(0)),
							))?;
							gather.values.extend(
								builtins::iterate(&value)?
									.into_iter()
									.map(|value| Some(value.into())),
							);
						} else {
							gather.values.push(Some(value));
						}
						self.gather_next(gather)
					}
					Continuation::Assign(place) => {
						self.place(place, PlaceAction::Write(self.value(value)?))
					}
					Continuation::WhileCondition(condition, body) => {
						if boolean(self.value(value)?)? {
							self.evaluate_after(body, Continuation::WhileBody(condition, body))
						} else {
							Self::done(Value::None)
						}
					}
					Continuation::ForIterable(item, index, body) => {
						let values = match self.value(value)? {
							Value::Range {
								start,
								end,
								inclusive,
							} => Iteration::Range {
								current: start.ok_or_else(|| {
									Error::Stop("a range without a start can't be iterated".into())
								})?,
								end,
								inclusive,
								finished: false,
							},
							Value::Array(values) | Value::Set(values) | Value::Tuple(values) => {
								Iteration::Values {
									values,
									position: 0,
								}
							}
							other => Iteration::Values {
								values: Arc::new(builtins::iterate(&other)?),
								position: 0,
							},
						};
						self.for_next(For {
							item,
							index,
							position: 0,
							values,
							body,
						})
					}
					Continuation::Callback(callback, finish) => {
						self.callback(callback, Some(self.value(value)?), finish)
					}
					_ => unreachable!("handled control boundary"),
				}
			}
		}
	}

	fn returned(&self, value: Out) -> Eval {
		let Out::Loan(loan) = &value else {
			return Ok(value);
		};
		if loan.owner.frame != self.frames.len() - 1 {
			return Ok(value);
		}
		if !self
			.frame()
			.code
			.params
			.iter()
			.any(|&(slot, _)| slot == loan.owner.slot)
		{
			return Err(
				Error::Stop("internal: a returned reference escapes a local owner".into()).into(),
			);
		}
		// The VM has no borrowed backing in Value. Retain only the selected
		// value before the call-owned argument temporary is released.
		self.value(value).map(Out::Value)
	}

	fn unary(&mut self, op: Unary, value: Out) -> Result<Action, Flow> {
		match op {
			Unary::Bind(slot) => {
				// An empty optional reference has no designated place. Keep it
				// empty when the source reference is subsequently reseated.
				let value = if let Out::Loan(loan) = &value
					&& matches!(self.read_loan(loan)?, Value::None)
				{
					Value::None.into()
				} else {
					value
				};
				self.locals()[slot as usize] = value;
				return Self::done(Value::None);
			}
			Unary::Return => return Err(Flow::Return(value)),
			Unary::Break => return Err(Flow::Break(value)),
			_ => {}
		}
		let value = self.value(value)?;
		let result = match op {
			Unary::Negate(num) => eval::negate(num, &value)?,
			Unary::Convert(num) => eval::convert(num, &value)?,
			Unary::Copy(plan) => crate::copies::copy(self.interpreter, &plan, &value)?,
			Unary::Not => Value::Boolean(!boolean(value)?),
			Unary::Boolean => Value::Boolean(boolean(value)?),
			Unary::Is(test, negated) => {
				Value::Boolean(crate::types::test(&test, &value) != negated)
			}
			Unary::MemberData => match value {
				Value::Member(member) => member.data.clone().unwrap_or(Value::None),
				other => other,
			},
			Unary::Member(name, kind) => Value::Member(Arc::new(Member {
				name,
				kind,
				data: Some(value),
			})),
			Unary::Field(name, optional) => {
				if optional && matches!(value, Value::None) {
					Value::None
				} else {
					builtins::field(&value, &name)?
				}
			}
			Unary::Assert(text) => {
				if !boolean(value)? {
					return Err(Error::Stop(format!("assertion failed: {text}")).into());
				}
				Value::None
			}
			Unary::Let(slot) => {
				self.locals()[slot as usize] = value.into();
				Value::None
			}
			Unary::Break | Unary::Return | Unary::Bind(_) => {
				unreachable!("forwarded before inspecting the value")
			}
			Unary::Throw => return Err(Error::Thrown(value).into()),
		};
		Self::done(result)
	}

	fn binary_result(&mut self, op: Binary, left: Out, right: Out) -> Result<Action, Flow> {
		let left = self.value(left)?;
		let right = self.value(right)?;
		Self::done(match op {
			Binary::Arith(op, num) => eval::arith(op, num, &left, &right)?,
			Binary::Equals(negated) => Value::Boolean(equals(&left, &right) != negated),
			Binary::Compare(op) => Value::Boolean(eval::compare(op, &left, &right)?),
			Binary::IsIn(negated) => Value::Boolean(builtins::includes(&right, &left)? != negated),
			Binary::Index => builtins::index(&left, &right)?,
		})
	}

	fn for_next(&mut self, mut state: For) -> Result<Action, Flow> {
		let value = match &mut state.values {
			Iteration::Values { values, position } => match values.get(*position) {
				Some(value) => {
					*position += 1;
					value.clone()
				}
				None => return Self::done(Value::None),
			},
			Iteration::Range {
				current,
				end,
				inclusive,
				finished,
			} => {
				if *finished
					|| end.is_some_and(|end| {
						if *inclusive {
							*current > end
						} else {
							*current >= end
						}
					}) {
					return Self::done(Value::None);
				}
				let value = Value::Integer(*current);
				match current.checked_add(I256::ONE) {
					Some(next) => *current = next,
					None => *finished = true,
				};
				value
			}
		};
		if let Some(slot) = state.item {
			self.locals()[slot as usize] = value.into();
		}
		if let Some(slot) = state.index {
			self.locals()[slot as usize] = Value::int(state.position).into();
		}
		state.position += 1;
		self.evaluate_after(state.body, Continuation::ForBody(state))
	}
}

fn boolean(value: Value) -> Result<bool, Error> {
	match value {
		Value::Boolean(value) => Ok(value),
		other => Err(Error::Stop(format!(
			"a condition must be a `Boolean`, not `{}`",
			other.to_text()
		))),
	}
}

fn range_bound(value: Value) -> Result<Option<I256>, Error> {
	match value {
		Value::Integer(value) => Ok(Some(value)),
		Value::None => Ok(None),
		other => Err(Error::Stop(format!(
			"a range bound must be an integer, not `{}`",
			other.to_text()
		))),
	}
}

#[cfg(test)]
mod ownership_tests {
	use super::*;
	use crate::ir::{Expr, GlobalKey};

	struct EmptyHost;
	impl crate::Host for EmptyHost {
		fn function(&mut self, _key: FnKey) -> Result<Arc<crate::ir::Function>, Error> {
			Err(Error::Unsupported("no function in this test".into()))
		}
		fn global(&mut self, _key: GlobalKey) -> Result<Value, Error> {
			Err(Error::Unsupported("no global in this test".into()))
		}
	}

	struct OneFunction(Arc<crate::ir::Function>);
	impl crate::Host for OneFunction {
		fn function(&mut self, _key: FnKey) -> Result<Arc<crate::ir::Function>, Error> {
			Ok(Arc::clone(&self.0))
		}
		fn global(&mut self, _key: GlobalKey) -> Result<Value, Error> {
			Err(Error::Unsupported("no global in this test".into()))
		}
	}

	#[test]
	fn closure_creation_and_invocation_borrow_the_original_capture() {
		let expr = Expr::Closure {
			function: FnKey(0, 0),
			captures: vec![0],
			borrow: true,
		};
		let code = Arc::new(Code::expression(&expr, 1, || Ok(())).unwrap());
		let mut host = OneFunction(Arc::new(crate::ir::Function {
			name: "capture".into(),
			params: Vec::new(),
			captures: vec![0],
			slots: 1,
			body: Expr::none(),
		}));
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, Arc::clone(&code)).unwrap();
		machine.locals()[0] = Value::array(vec![Value::int(1)]).into();
		let Action::Done(Ok(Out::Value(closure))) = machine.enter(code.body).unwrap() else {
			panic!("a closure value")
		};
		machine
			.call_function(Target::Value(closure), Vec::new())
			.unwrap();
		let loan = machine
			.loan(&ResolvedPlace {
				slot: 0,
				steps: Vec::new(),
			})
			.unwrap();
		assert_eq!(loan.owner.frame, 0);
		let Value::Array(items) = machine.read_loan(&loan).unwrap() else {
			panic!("a captured array")
		};
		assert_eq!(Arc::strong_count(items), 1);
		let original = Arc::as_ptr(items);
		builtins::mutate_internal(
			machine.interpreter,
			"Array#push",
			Machine::locate(&mut machine.frames, &loan).unwrap(),
			vec![Value::int(2)],
		)
		.unwrap();
		let Value::Array(items) = machine.read_loan(&loan).unwrap() else {
			panic!("a captured array")
		};
		assert_eq!(Arc::as_ptr(items), original);
		assert_eq!(items.len(), 2);
	}

	#[test]
	fn stale_capture_roots_fail_before_entering_a_call() {
		let code = Arc::new(Code::expression(&Expr::none(), 0, || Ok(())).unwrap());
		let mut host = OneFunction(Arc::new(crate::ir::Function {
			name: "stale".into(),
			params: Vec::new(),
			captures: vec![0],
			slots: 1,
			body: Expr::none(),
		}));
		let mut closure = Closure::new(FnKey(0, 0), Vec::new(), Vec::new());
		closure.environment = Some(vec![Loan::new(Owner {
			frame: 0,
			generation: 0,
			slot: 0,
		})]);
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, code).unwrap();
		assert!(
			matches!(machine.call_function(Target::Value(Value::Function(Arc::new(closure))), Vec::new()), Err(Flow::Error(Error::Stop(message))) if message.contains("outlived"))
		);
		assert_eq!(machine.frames.len(), 1);
		assert_eq!(machine.interpreter.depth, 0);
	}

	#[test]
	fn invoking_an_owned_environment_does_not_clone_its_payload() {
		let code = Arc::new(Code::expression(&Expr::none(), 0, || Ok(())).unwrap());
		let mut host = OneFunction(Arc::new(crate::ir::Function {
			name: "owned".into(),
			params: Vec::new(),
			captures: vec![0],
			slots: 1,
			body: Expr::none(),
		}));
		let closure = Arc::new(Closure::new(
			FnKey(0, 0),
			vec![Value::array(vec![Value::int(1)])],
			Vec::new(),
		));
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, code).unwrap();
		machine
			.call_function(
				Target::Value(Value::Function(Arc::clone(&closure))),
				Vec::new(),
			)
			.unwrap();
		let loan = machine
			.loan(&ResolvedPlace {
				slot: 0,
				steps: Vec::new(),
			})
			.unwrap();
		let Value::Array(items) = machine.read_loan(&loan).unwrap() else {
			panic!("an owned capture")
		};
		assert_eq!(Arc::strong_count(items), 1);
		assert!(Machine::locate(&mut machine.frames, &loan).is_err());
	}

	#[test]
	fn forwarded_loans_do_not_add_an_array_owner_during_the_call() {
		let code = Arc::new(Code::expression(&Expr::none(), 1, || Ok(())).unwrap());
		let mut host = OneFunction(Arc::new(crate::ir::Function {
			name: "borrowed".into(),
			params: vec![crate::ir::Param {
				slot: 0,
				default: None,
			}],
			captures: Vec::new(),
			slots: 1,
			body: Expr::none(),
		}));
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, Arc::clone(&code)).unwrap();
		let mut items = Vec::with_capacity(4);
		items.push(Value::int(1));
		let items = Arc::new(items);
		let original = Arc::as_ptr(&items);
		let original_buffer = items.as_ptr();
		machine.locals()[0] = Value::Array(items).into();
		let source = ResolvedPlace {
			slot: 0,
			steps: Vec::new(),
		};
		let loan = machine.loan(&source).unwrap();
		assert!(matches!(
			machine.call_function(
				Target::Function(FnKey(0, 1)),
				vec![Some(Out::Loan(loan.clone()))]
			),
			Ok(Action::Eval(_))
		));
		let forwarded = machine.loan(&source).unwrap();
		assert_eq!(forwarded.owner, loan.owner);
		let Value::Array(current) = machine.read_loan(&forwarded).unwrap() else {
			panic!("an array loan")
		};
		assert_eq!(Arc::strong_count(current), 1);
		builtins::mutate(
			machine.interpreter,
			"Array#push",
			Machine::locate(&mut machine.frames, &forwarded).unwrap(),
			vec![Value::int(2)],
		)
		.unwrap();
		let Value::Array(current) = machine.read_loan(&forwarded).unwrap() else {
			panic!("an array loan")
		};
		assert_eq!(Arc::strong_count(current), 1);
		assert_eq!(Arc::as_ptr(current), original);
		assert_eq!(current.as_ptr(), original_buffer);
		assert_eq!(current.len(), 2);
		let mut stale = forwarded;
		stale.owner.generation += 1;
		assert!(
			matches!(machine.read_loan(&stale), Err(Flow::Error(Error::Stop(message))) if message.contains("outlived its frame"))
		);
		let continuation = machine.stack.pop().unwrap();
		machine
			.resume(continuation, Ok(Value::None.into()))
			.unwrap();
		assert_eq!(machine.interpreter.depth, 0);
	}

	#[test]
	fn reading_a_nested_place_preserves_shared_owner_storage() {
		let expr = Expr::Read(crate::ir::Place {
			slot: 0,
			steps: vec![
				crate::ir::Step::Field(Arc::from("child")),
				crate::ir::Step::Field(Arc::from("count")),
			],
		});
		let code = Arc::new(Code::expression(&expr, 1, || Ok(())).unwrap());
		let body = code.body;
		let mut host = EmptyHost;
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, code).unwrap();
		let child = Arc::new(Object {
			fields: vec![(Arc::from("count"), Value::int(7))],
		});
		let root = Arc::new(Object {
			fields: vec![(Arc::from("child"), Value::Object(Arc::clone(&child)))],
		});
		machine.locals()[0] = Value::Object(Arc::clone(&root)).into();
		let Ok(Action::Done(Ok(Out::Value(value)))) = machine.enter(body) else {
			panic!("a stored field must be readable");
		};
		assert_eq!(value, Value::int(7));
		let Out::Value(Value::Object(current)) = &machine.frame().locals[0] else {
			panic!("the owner remains an object")
		};
		assert!(Arc::ptr_eq(current, &root));
		let Value::Object(current_child) = current.get("child").unwrap() else {
			panic!("the child remains an object")
		};
		assert!(Arc::ptr_eq(current_child, &child));
	}

	#[test]
	fn reading_an_absent_map_key_throws_without_inserting_it() {
		let code = Arc::new(Code::expression(&Expr::none(), 1, || Ok(())).unwrap());
		let mut host = EmptyHost;
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, code).unwrap();
		let entries = Arc::new(Vec::new());
		machine.locals()[0] = Value::Map(Arc::clone(&entries)).into();
		let result = machine.finish(
			Finish::Place(
				Place {
					slot: 0,
					steps: vec![Step::Index(0)],
				},
				PlaceAction::Read,
			),
			vec![Some(Value::String(Arc::from("missing")).into())],
		);
		assert!(matches!(result, Err(Flow::Error(Error::Thrown(_)))));
		let Out::Value(Value::Map(current)) = &machine.frame().locals[0] else {
			panic!("the owner remains a map")
		};
		assert!(Arc::ptr_eq(current, &entries));
		assert!(current.is_empty());
	}

	#[test]
	fn moving_an_array_does_not_leave_a_copy_on_write_owner() {
		let code = Arc::new(Code::expression(&Expr::Move(0), 1, || Ok(())).unwrap());
		let body = code.body;
		let mut host = EmptyHost;
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		let mut machine = Machine::new(&mut interpreter, code).unwrap();
		let mut items = Vec::with_capacity(4);
		items.push(Value::int(1));
		let storage = Arc::new(items);
		let original = Arc::as_ptr(&storage);
		let original_items = storage.as_ptr();
		machine.locals()[0] = Value::Array(storage).into();
		let Ok(Action::Done(Ok(Out::Value(Value::Array(mut moved))))) = machine.enter(body) else {
			panic!("a move must return the array");
		};
		assert!(matches!(machine.locals()[0], Out::Value(Value::None)));
		assert_eq!(Arc::as_ptr(&moved), original);
		assert_eq!(Arc::strong_count(&moved), 1);
		Arc::make_mut(&mut moved).push(Value::int(2));
		assert_eq!(Arc::as_ptr(&moved), original);
		assert_eq!(moved.as_ptr(), original_items);
	}
}
