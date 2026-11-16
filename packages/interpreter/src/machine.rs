//! The evaluator's explicit continuation and call stacks. No Fa call,
//! default argument, callback, or expression evaluation recurses in Rust.

use std::sync::Arc;

use ethnum::I256;

use crate::callbacks::{Callback, Progress};
use crate::code::{Argument, Code, Item, Node, Op, Part, Place, Step};
use crate::eval::{self, Error, Eval, Flow, Interpreter};
use crate::ir::{ArithOp, CompareOp, FnKey, Num, Slot, TypeTest};
use crate::text::Text;
use crate::value::{Closure, Member, MemberKind, Object, Value, equals};
use crate::{builtins, memory};

enum Action {
	Eval(Node),
	Done(Eval),
	Call(Target, Vec<Option<Value>>),
}

enum Target {
	Function(FnKey),
	Value(Value),
}

struct Frame {
	code: Arc<Code>,
	locals: Vec<Value>,
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
	values: Vec<Option<Value>>,
	spread: bool,
	finish: Finish,
}

enum Finish {
	Arguments(Target),
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
	Write(Value),
	Mutate(&'static str, Vec<Value>),
}

enum PlaceStep {
	Field(Arc<str>),
	Index(Value),
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
	Not,
	Boolean,
	Is(TypeTest, bool),
	MemberData,
	Member(Arc<str>, MemberKind),
	Field(Arc<str>, bool),
	Assert(Arc<str>),
	Let(Slot),
	Break,
	Return,
	Throw,
}

enum Iteration {
	Utf8 {
		text: Text,
		offset: usize,
		buffer: [u8; 4],
		position: usize,
		length: usize,
	},
	Text {
		text: Text,
		offset: usize,
	},
	Bytes {
		bytes: Arc<Vec<u8>>,
		offset: usize,
	},
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
	Mutate(ResolvedPlace),
}

enum Continuation {
	Block(std::vec::IntoIter<Node>),
	If(Node, Option<Node>),
	Unary(Unary),
	BinaryLeft(Binary, Node),
	BinaryRight(Binary, Value),
	ShortCircuit {
		right: Node,
		short: bool,
	},
	Fallback(Node),
	Callee(Vec<Argument>),
	Gather(Gather),
	Assign(Place),
	Loop(Node),
	WhileCondition(Node, Node),
	WhileBody(Node, Node),
	ForIterable(Option<Slot>, Option<Slot>, Node, bool),
	ForBody(For),
	Catch(Option<Slot>, Node),
	Function,
	Default {
		arguments: std::vec::IntoIter<Option<Value>>,
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
	pub fn evaluate(interpreter: &'i mut Interpreter<'h>, code: Arc<Code>) -> Eval {
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
			.run(Action::Call(Target::Value(callee), arguments))
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
		interpreter.reserve(memory::values_size(code.slots as usize))?;
		let locals = vec![Value::None; code.slots as usize];
		Ok(Self {
			interpreter,
			frames: vec![Frame { code, locals }],
			stack: Vec::new(),
		})
	}

	fn run(mut self, mut action: Action) -> Eval {
		loop {
			let next = match action {
				Action::Eval(node) => self.enter(node),
				Action::Call(target, arguments) => self.call_function(target, arguments),
				Action::Done(result) => {
					let Some(continuation) = self.stack.pop() else {
						return result;
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
	fn locals(&mut self) -> &mut Vec<Value> {
		&mut self.frames.last_mut().expect("an evaluation frame").locals
	}
	fn done(value: Value) -> Result<Action, Flow> {
		Ok(Action::Done(Ok(value)))
	}
	fn evaluate_after(&mut self, node: Node, continuation: Continuation) -> Result<Action, Flow> {
		self.stack.push(continuation);
		Ok(Action::Eval(node))
	}

	fn enter(&mut self, node: Node) -> Result<Action, Flow> {
		self.interpreter.step()?;
		// The Arc keeps this immutable code alive while calls below may ask
		// the host to prepare another function.
		let code = Arc::clone(&self.frame().code);
		match &code.nodes[node] {
			Op::Const(value) => Self::done(value.clone()),
			Op::Local(slot) => Self::done(self.frame().locals[*slot as usize].clone()),
			Op::Global(key) => Self::done(self.interpreter.host.global(*key)?),
			Op::Function(function) => Self::done(Value::Function(Arc::new(Closure {
				function: *function,
				captured: Vec::new(),
				bound: Vec::new(),
			}))),
			Op::Closure(function, captures) => Self::done(Value::Function(Arc::new(Closure {
				function: *function,
				captured: captures
					.iter()
					.map(|&slot| self.frame().locals[slot as usize].clone())
					.collect(),
				bound: Vec::new(),
			}))),
			Op::Let(slot, value) => {
				self.evaluate_after(*value, Continuation::Unary(Unary::Let(*slot)))
			}
			Op::Assign(place, value) => {
				self.evaluate_after(*value, Continuation::Assign(place.clone()))
			}
			Op::Read(place) => self.place(place.clone(), PlaceAction::Read),
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
				utf8,
				item,
				index,
				iterable,
				body,
			} => self.evaluate_after(
				*iterable,
				Continuation::ForIterable(*item, *index, *body, *utf8),
			),
			Op::Break(value) => match value {
				Some(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Break)),
				None => Err(Flow::Break(Value::None)),
			},
			Op::Continue => Err(Flow::Continue),
			Op::Return(value) => match value {
				Some(value) => self.evaluate_after(*value, Continuation::Unary(Unary::Return)),
				None => Err(Flow::Return(Value::None)),
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
						Part::Text(text) => {
							Input::Constant(Value::String(Text::from(Arc::clone(text))))
						}
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

	fn arguments(&mut self, target: Target, arguments: Vec<Argument>) -> Result<Action, Flow> {
		self.gather(
			arguments
				.into_iter()
				.map(|argument| match argument {
					Argument::Value(value) => Input::Expr(value),
					Argument::Default => Input::Default,
					Argument::Variadic(items) => Input::Variadic(items),
				})
				.collect(),
			Finish::Arguments(target),
		)
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
				Input::Constant(value) => gather.values.push(Some(value)),
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

	fn finish(&mut self, finish: Finish, values: Vec<Option<Value>>) -> Result<Action, Flow> {
		if let Finish::Arguments(target) = finish {
			return Ok(Action::Call(target, values));
		}
		let values: Vec<Value> = values
			.into_iter()
			.map(|value| value.expect("a supplied value"))
			.collect();
		match finish {
			Finish::Arguments(_) => unreachable!(),
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
				let pieces: Vec<Text> = values
					.into_iter()
					.map(|value| match value {
						Value::String(text) => text,
						other => Text::from(other.to_text()),
					})
					.collect();
				let bytes = pieces
					.iter()
					.fold(0usize, |n, p| n.saturating_add(p.utf8_len()));
				let units = pieces.iter().fold(0usize, |n, p| n.saturating_add(p.len()));
				self.interpreter.reserve_text(bytes)?;
				if pieces.iter().any(|p| !p.is_well_formed()) {
					self.interpreter.reserve(units.saturating_mul(10))?;
				}
				Self::done(Value::String(Text::concat(&pieces)))
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
					Self::done(builtins::call(self.interpreter, path, values)?)
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
				match action {
					PlaceAction::Read => Self::done(Self::read(self.locals(), &resolved)?),
					PlaceAction::Write(value) => {
						self.reserve_edit(&resolved)?;
						Self::edit(self.locals(), &resolved, |slot| {
							*slot = value;
							Ok(Value::None)
						})?;
						Self::done(Value::None)
					}
					PlaceAction::Mutate(path, mut arguments) if Callback::mutating(path) => {
						let receiver = Self::read(self.locals(), &resolved)?;
						arguments.insert(0, receiver);
						self.start_callback(path, arguments, CallbackFinish::Mutate(resolved))
					}
					PlaceAction::Mutate(path, arguments) => {
						self.reserve_edit(&resolved)?;
						let frame = self.frames.last_mut().expect("an evaluation frame");
						let result = Self::edit(&mut frame.locals, &resolved, |receiver| {
							Ok(builtins::mutate(
								self.interpreter,
								path,
								receiver,
								arguments,
							)?)
						})?;
						Self::done(result)
					}
				}
			}
		}
	}

	/// Preflight copy-on-write and borrowed-slice buffers before changing a
	/// place. Parent copies make their owned children shared as well.
	fn reserve_edit(&self, place: &ResolvedPlace) -> Result<(), Error> {
		let mut current = &self.frame().locals[place.slot as usize];
		let mut shared = false;
		let mut view: Option<(usize, usize)> = None;
		let mut bytes = 0usize;
		for step in &place.steps {
			while let Value::Member(member) = current {
				shared |= Arc::strong_count(member) > 1;
				let Some(data) = &member.data else {
					break;
				};
				current = data;
			}
			match (current, step) {
				(Value::Object(object), PlaceStep::Field(name)) => {
					if shared || Arc::strong_count(object) > 1 {
						bytes = bytes.saturating_add(
							object
								.fields
								.len()
								.saturating_mul(std::mem::size_of::<(Arc<str>, Value)>()),
						);
						shared = true;
					}
					let Some(field) = object.get(name) else {
						break;
					};
					current = field;
				}
				(Value::Array(items) | Value::Tuple(items), PlaceStep::Index(key)) => {
					let (start, length) = view.unwrap_or((0, items.len()));
					if shared || Arc::strong_count(items) > 1 {
						bytes = bytes.saturating_add(memory::values_size(length));
						shared = true;
					}
					if let Value::Range {
						start: from,
						end,
						inclusive,
					} = key
					{
						let (from, to) = builtins::slice_bounds(*from, *end, *inclusive, length)?;
						bytes = bytes.saturating_add(memory::values_size(to - from));
						view = Some((start + from, to - from));
						shared = true;
					} else if let Some(index) = key.as_int() {
						current = &items[start + builtins::position(index, length)?];
						view = None;
					} else {
						break;
					}
				}
				(Value::Bytes(data), PlaceStep::Index(key)) => {
					let (start, length) = view.unwrap_or((0, data.len()));
					if view.is_none() && (shared || Arc::strong_count(data) > 1) {
						bytes = bytes.saturating_add(data.len());
					}
					if let Value::Range {
						start: from,
						end,
						inclusive,
					} = key
					{
						let (from, to) = builtins::slice_bounds(*from, *end, *inclusive, length)?;
						bytes = bytes.saturating_add(to - from);
						view = Some((start + from, to - from));
					} else {
						break;
					}
				}
				_ => break,
			}
		}
		self.interpreter.reserve(bytes)
	}

	fn read(locals: &[Value], place: &ResolvedPlace) -> Result<Value, Flow> {
		let mut current = locals[place.slot as usize].clone();
		for step in &place.steps {
			current = match step {
				PlaceStep::Field(name) => builtins::field(&current, name)?,
				PlaceStep::Index(key) => builtins::index(&current, key)?,
			};
		}
		Ok(current)
	}

	fn edit<F>(locals: &mut [Value], place: &ResolvedPlace, action: F) -> Result<Value, Flow>
	where
		F: FnOnce(&mut Value) -> Result<Value, Flow>,
	{
		Self::edit_steps(&mut locals[place.slot as usize], &place.steps, action)
	}

	fn edit_steps<F>(current: &mut Value, steps: &[PlaceStep], action: F) -> Result<Value, Flow>
	where
		F: FnOnce(&mut Value) -> Result<Value, Flow>,
	{
		let Some((step, rest)) = steps.split_first() else {
			return action(current);
		};
		match step {
			PlaceStep::Field(name) => {
				Self::edit_steps(builtins::field_mut(current, name)?, rest, action)
			}
			PlaceStep::Index(key)
				if matches!(current, Value::Bytes(_)) || matches!(key, Value::Range { .. }) =>
			{
				let mut selected = builtins::index(current, key)?;
				let result = Self::edit_steps(&mut selected, rest, action)?;
				builtins::set_index(current, key, selected)?;
				Ok(result)
			}
			PlaceStep::Index(key) => {
				Self::edit_steps(builtins::index_mut(current, key.clone())?, rest, action)
			}
		}
	}

	fn call_function(
		&mut self,
		target: Target,
		mut arguments: Vec<Option<Value>>,
	) -> Result<Action, Flow> {
		let (key, captured, bound) = match target {
			Target::Function(key) => (key, Vec::new(), Vec::new()),
			Target::Value(Value::Function(closure)) => (
				closure.function,
				closure.captured.clone(),
				closure.bound.clone(),
			),
			Target::Value(other) => {
				return Err(Error::Stop(format!("`{}` is not a function", other.to_text())).into());
			}
		};
		let code = self.interpreter.code(key)?;
		if !bound.is_empty() {
			let mut given = arguments.into_iter();
			arguments = (0..code.params.len())
				.map(
					|index| match bound.iter().find(|(bound, _)| *bound == index) {
						Some((_, value)) => Some(value.clone()),
						None => given.next().flatten(),
					},
				)
				.collect();
		}
		if self.interpreter.depth >= self.interpreter.limits.depth {
			return Err(Error::Depth(self.interpreter.limits.depth).into());
		}
		self.interpreter
			.reserve(memory::values_size(code.slots as usize))?;
		self.interpreter.depth += 1;
		let mut locals = vec![Value::None; code.slots as usize];
		for (&slot, value) in code.captures.iter().zip(captured) {
			locals[slot as usize] = value;
		}
		self.frames.push(Frame { code, locals });
		self.stack.push(Continuation::Function);
		self.parameters(arguments.into_iter(), 0)
	}

	fn parameters(
		&mut self,
		mut arguments: std::vec::IntoIter<Option<Value>>,
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
				(None, None) => self.locals()[slot as usize] = Value::None,
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
		if path == "Json.parse"
			&& let Some(input) = arguments.first()
		{
			let length = match input {
				Value::Bytes(b) => b.len(),
				Value::String(t) => t.utf8_len(),
				_ => 0,
			};
			self.interpreter.reserve(length.saturating_mul(64))?;
		}
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
		match callback.advance(value, self.interpreter)? {
			Progress::Call {
				function,
				arguments,
				next,
			} => {
				self.stack.push(Continuation::Callback(next, finish));
				Ok(Action::Call(
					Target::Value(function),
					arguments.into_iter().map(Some).collect(),
				))
			}
			Progress::Done(value) => match finish {
				CallbackFinish::Value => Self::done(value),
				CallbackFinish::Mutate(place) => {
					self.reserve_edit(&place)?;
					Self::edit(self.locals(), &place, |slot| {
						*slot = value;
						Ok(Value::None)
					})?;
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
					Err(_) => Value::None,
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
						self.locals()[slot as usize] = error;
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
					Continuation::If(then, otherwise) => match boolean(value)? {
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
						if boolean(value)? == short {
							Self::done(Value::Boolean(short))
						} else {
							self.evaluate_after(right, Continuation::Unary(Unary::Boolean))
						}
					}
					Continuation::Fallback(right) => {
						if matches!(value, Value::None) {
							Ok(Action::Eval(right))
						} else {
							Self::done(value)
						}
					}
					Continuation::Callee(arguments) => {
						self.arguments(Target::Value(value), arguments)
					}
					Continuation::Gather(mut gather) => {
						if gather.spread {
							self.interpreter.reserve(memory::values_size(
								gather
									.values
									.len()
									.saturating_add(builtins::count(&value).unwrap_or(0)),
							))?;
							gather
								.values
								.extend(builtins::iterate(&value)?.into_iter().map(Some));
						} else {
							gather.values.push(Some(value));
						}
						self.gather_next(gather)
					}
					Continuation::Assign(place) => self.place(place, PlaceAction::Write(value)),
					Continuation::WhileCondition(condition, body) => {
						if boolean(value)? {
							self.evaluate_after(body, Continuation::WhileBody(condition, body))
						} else {
							Self::done(Value::None)
						}
					}
					Continuation::ForIterable(item, index, body, utf8) => {
						let values = match value {
							Value::String(text) if utf8 => Iteration::Utf8 {
								text,
								offset: 0,
								buffer: [0; 4],
								position: 0,
								length: 0,
							},
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
							Value::String(text) => Iteration::Text { text, offset: 0 },
							Value::Bytes(bytes) => Iteration::Bytes { bytes, offset: 0 },
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
						self.callback(callback, Some(value), finish)
					}
					_ => unreachable!("handled control boundary"),
				}
			}
		}
	}

	fn unary(&mut self, op: Unary, value: Value) -> Result<Action, Flow> {
		let result = match op {
			Unary::Negate(num) => eval::negate(num, &value)?,
			Unary::Convert(num) => eval::convert(num, &value)?,
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
				self.locals()[slot as usize] = value;
				Value::None
			}
			Unary::Break => return Err(Flow::Break(value)),
			Unary::Return => return Err(Flow::Return(value)),
			Unary::Throw => return Err(Error::Thrown(value).into()),
		};
		Self::done(result)
	}

	fn binary_result(&mut self, op: Binary, left: Value, right: Value) -> Result<Action, Flow> {
		Self::done(match op {
			Binary::Arith(op, num) => eval::arith(op, num, &left, &right)?,
			Binary::Equals(negated) => Value::Boolean(equals(&left, &right) != negated),
			Binary::Compare(op) => Value::Boolean(eval::compare(op, &left, &right)?),
			Binary::IsIn(negated) => Value::Boolean(builtins::includes(&right, &left)? != negated),
			Binary::Index => builtins::index(&left, &right)?,
		})
	}

	fn for_next(&mut self, mut state: For) -> Result<Action, Flow> {
		let mut width = 1;
		let value = match &mut state.values {
			Iteration::Utf8 {
				text,
				offset,
				buffer,
				position,
				length,
			} => {
				if *position == *length {
					let Some(code) = text.next_scalar(offset) else {
						return Self::done(Value::None);
					};
					*length = char::from_u32(code)
						.unwrap_or(char::REPLACEMENT_CHARACTER)
						.encode_utf8(buffer)
						.len();
					*position = 0;
				}
				let value = Value::int(i64::from(buffer[*position]));
				*position += 1;
				value
			}
			Iteration::Text { text, offset } => match text.next_point(offset) {
				Some((text, units)) => {
					width = units as i64;
					Value::String(text)
				}
				None => return Self::done(Value::None),
			},
			Iteration::Bytes { bytes, offset } => match bytes.get(*offset) {
				Some(byte) => {
					*offset += 1;
					Value::int(i64::from(*byte))
				}
				None => return Self::done(Value::None),
			},
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
			self.locals()[slot as usize] = value;
		}
		if let Some(slot) = state.index {
			self.locals()[slot as usize] = Value::int(state.position);
		}
		state.position += width;
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
