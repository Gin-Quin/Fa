//! Indexed execution nodes. Flattening retains the checked IR's evaluation
//! order and expression count, while continuations own indices rather than
//! borrowing a function body across a call to the host.

use std::sync::Arc;

use crate::eval::Error;
use crate::ir::{self, ArithOp, CompareOp, FnKey, GlobalKey, Num, Slot, TypeTest};
use crate::value::{MemberKind, Value};

pub(crate) type Node = usize;

pub(crate) struct Code {
	pub nodes: Vec<Op>,
	pub body: Node,
	pub slots: u32,
	pub captures: Vec<Slot>,
	pub params: Vec<(Slot, Option<Node>)>,
}

impl Code {
	pub fn expression(
		expr: &ir::Expr,
		slots: u32,
		check: impl FnMut() -> Result<(), Error>,
	) -> Result<Self, Error> {
		let mut builder = Builder::default();
		let body = builder.add(expr);
		Ok(Self {
			nodes: builder.finish(check)?,
			body,
			slots,
			captures: Vec::new(),
			params: Vec::new(),
		})
	}

	pub fn function(
		function: &ir::Function,
		check: impl FnMut() -> Result<(), Error>,
	) -> Result<Self, Error> {
		let mut builder = Builder::default();
		let params = function
			.params
			.iter()
			.map(|param| {
				(
					param.slot,
					param.default.as_ref().map(|expr| builder.add(expr)),
				)
			})
			.collect();
		let body = builder.add(&function.body);
		Ok(Self {
			nodes: builder.finish(check)?,
			body,
			slots: function.slots,
			captures: function.captures.clone(),
			params,
		})
	}
}

#[derive(Clone)]
pub(crate) struct Item {
	pub node: Node,
	pub spread: bool,
}

#[derive(Clone)]
pub(crate) enum Argument {
	Value(Node),
	Default,
	Variadic(Vec<Item>),
}

#[derive(Clone)]
pub(crate) enum Step {
	Field(Arc<str>),
	Index(Node),
}

#[derive(Clone)]
pub(crate) struct Place {
	pub slot: Slot,
	pub steps: Vec<Step>,
}

#[derive(Clone)]
pub(crate) enum Part {
	Text(Arc<str>),
	Value(Node),
}

#[derive(Clone)]
pub(crate) enum Op {
	Const(Value),
	Local(Slot),
	Global(GlobalKey),
	Function(FnKey),
	Closure(FnKey, Vec<Slot>),
	Partial(FnKey, Node, Vec<(usize, Node)>),
	Let(Slot, Node),
	Assign(Place, Node),
	Read(Place),
	Block(Vec<Node>),
	If(Node, Node, Option<Node>),
	Loop(Node),
	While(Node, Node),
	For {
		item: Option<Slot>,
		index: Option<Slot>,
		iterable: Node,
		body: Node,
	},
	Break(Option<Node>),
	Continue,
	Return(Option<Node>),
	Throw(Node),
	Catch {
		body: Node,
		slot: Option<Slot>,
		handler: Node,
	},
	Assert(Node, Arc<str>),
	Arith(ArithOp, Num, Node, Node),
	Negate(Num, Node),
	Convert(Num, Node),
	Equals(Node, Node, bool),
	Compare(CompareOp, Node, Node),
	And(Node, Node),
	Or(Node, Node),
	Not(Node),
	Fallback(Node, Node),
	Is(Node, TypeTest, bool),
	IsIn(Node, Node, bool),
	Instantiate(Node, FnKey),
	Call(Node, Vec<Argument>),
	CallFunction(FnKey, Vec<Argument>),
	Builtin(&'static str, Vec<Node>),
	Mutate(&'static str, Place, Vec<Node>),
	Array(Vec<Item>),
	Set(Vec<Item>),
	Map(Vec<Item>),
	Tuple(Vec<Node>),
	Object(Vec<(Arc<str>, Node)>),
	Member {
		name: Arc<str>,
		kind: MemberKind,
		data: Option<Node>,
	},
	MemberData(Node),
	Field(Node, Arc<str>, bool),
	Index(Node, Node),
	Range(Option<Node>, Option<Node>, bool),
	Template(Vec<Part>),
	Impure(Arc<str>),
	Unsupported(Arc<str>),
}

#[derive(Default)]
struct Builder<'a> {
	pending: Vec<&'a ir::Expr>,
}

impl<'a> Builder<'a> {
	fn add(&mut self, expression: &'a ir::Expr) -> Node {
		let node = self.pending.len();
		self.pending.push(expression);
		node
	}

	fn arguments(&mut self, arguments: &'a [ir::Arg]) -> Vec<Argument> {
		arguments
			.iter()
			.map(|argument| match argument {
				ir::Arg::Value(value) => Argument::Value(self.add(value)),
				ir::Arg::Default => Argument::Default,
				ir::Arg::Variadic(items) => Argument::Variadic(self.items(items)),
			})
			.collect()
	}

	fn items(&mut self, items: &'a [ir::Item]) -> Vec<Item> {
		items
			.iter()
			.map(|item| match item {
				ir::Item::One(value) => Item {
					node: self.add(value),
					spread: false,
				},
				ir::Item::Spread(value) => Item {
					node: self.add(value),
					spread: true,
				},
			})
			.collect()
	}

	fn place(&mut self, place: &'a ir::Place) -> Place {
		Place {
			slot: place.slot,
			steps: place
				.steps
				.iter()
				.map(|step| match step {
					ir::Step::Field(name) => Step::Field(Arc::clone(name)),
					ir::Step::Index(index) => Step::Index(self.add(index)),
				})
				.collect(),
		}
	}

	fn finish(mut self, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<Op>, Error> {
		let mut nodes = Vec::new();
		while let Some(&expression) = self.pending.get(nodes.len()) {
			if nodes.len().is_multiple_of(1024) {
				check()?;
			}
			let op = match expression {
				ir::Expr::Const(value) => Op::Const(value.clone()),
				ir::Expr::Local(slot) => Op::Local(*slot),
				ir::Expr::Global(key) => Op::Global(*key),
				ir::Expr::Function(key) => Op::Function(*key),
				ir::Expr::Closure { function, captures } => {
					Op::Closure(*function, captures.clone())
				}
				ir::Expr::Partial {
					site,
					function,
					bound,
				} => Op::Partial(
					*site,
					self.add(function),
					bound
						.iter()
						.map(|(index, value)| (*index, self.add(value)))
						.collect(),
				),
				ir::Expr::Let(slot, value) => Op::Let(*slot, self.add(value)),
				ir::Expr::Assign(place, value) => Op::Assign(self.place(place), self.add(value)),
				ir::Expr::Read(place) => Op::Read(self.place(place)),
				ir::Expr::Block(statements) => {
					Op::Block(statements.iter().map(|node| self.add(node)).collect())
				}
				ir::Expr::If(condition, then, otherwise) => Op::If(
					self.add(condition),
					self.add(then),
					otherwise.as_ref().map(|node| self.add(node)),
				),
				ir::Expr::Loop(body) => Op::Loop(self.add(body)),
				ir::Expr::While(condition, body) => Op::While(self.add(condition), self.add(body)),
				ir::Expr::For {
					item,
					index,
					iterable,
					body,
				} => Op::For {
					item: *item,
					index: *index,
					iterable: self.add(iterable),
					body: self.add(body),
				},
				ir::Expr::Break(value) => Op::Break(value.as_ref().map(|node| self.add(node))),
				ir::Expr::Continue => Op::Continue,
				ir::Expr::Return(value) => Op::Return(value.as_ref().map(|node| self.add(node))),
				ir::Expr::Throw(value) => Op::Throw(self.add(value)),
				ir::Expr::Catch {
					body,
					slot,
					handler,
				} => Op::Catch {
					body: self.add(body),
					slot: *slot,
					handler: self.add(handler),
				},
				ir::Expr::Assert(condition, text) => {
					Op::Assert(self.add(condition), Arc::clone(text))
				}
				ir::Expr::Arith(op, num, left, right) => {
					Op::Arith(*op, *num, self.add(left), self.add(right))
				}
				ir::Expr::Negate(num, value) => Op::Negate(*num, self.add(value)),
				ir::Expr::Convert(num, value) => Op::Convert(*num, self.add(value)),
				ir::Expr::Equals(left, right, negated) => {
					Op::Equals(self.add(left), self.add(right), *negated)
				}
				ir::Expr::Compare(op, left, right) => {
					Op::Compare(*op, self.add(left), self.add(right))
				}
				ir::Expr::And(left, right) => Op::And(self.add(left), self.add(right)),
				ir::Expr::Or(left, right) => Op::Or(self.add(left), self.add(right)),
				ir::Expr::Not(value) => Op::Not(self.add(value)),
				ir::Expr::Fallback(left, right) => Op::Fallback(self.add(left), self.add(right)),
				ir::Expr::Is(value, test, negated) => {
					Op::Is(self.add(value), test.clone(), *negated)
				}
				ir::Expr::IsIn(value, collection, negated) => {
					Op::IsIn(self.add(value), self.add(collection), *negated)
				}
				ir::Expr::Instantiate(value, application) => {
					Op::Instantiate(self.add(value), *application)
				}
				ir::Expr::Call(callee, arguments) => {
					Op::Call(self.add(callee), self.arguments(arguments))
				}
				ir::Expr::CallFunction(key, arguments) => {
					Op::CallFunction(*key, self.arguments(arguments))
				}
				ir::Expr::Builtin(path, arguments) => {
					Op::Builtin(path, arguments.iter().map(|node| self.add(node)).collect())
				}
				ir::Expr::Mutate(path, place, arguments) => Op::Mutate(
					path,
					self.place(place),
					arguments.iter().map(|node| self.add(node)).collect(),
				),
				ir::Expr::Array(items) => Op::Array(self.items(items)),
				ir::Expr::Set(items) => Op::Set(self.items(items)),
				ir::Expr::Map(items) => Op::Map(self.items(items)),
				ir::Expr::Tuple(items) => {
					Op::Tuple(items.iter().map(|node| self.add(node)).collect())
				}
				ir::Expr::Object(fields) => Op::Object(
					fields
						.iter()
						.map(|(name, value)| (Arc::clone(name), self.add(value)))
						.collect(),
				),
				ir::Expr::Member { name, kind, data } => Op::Member {
					name: Arc::clone(name),
					kind: *kind,
					data: data.as_ref().map(|node| self.add(node)),
				},
				ir::Expr::MemberData(value) => Op::MemberData(self.add(value)),
				ir::Expr::Field(value, name, optional) => {
					Op::Field(self.add(value), Arc::clone(name), *optional)
				}
				ir::Expr::Index(value, index) => Op::Index(self.add(value), self.add(index)),
				ir::Expr::Range(start, end, inclusive) => Op::Range(
					start.as_ref().map(|node| self.add(node)),
					end.as_ref().map(|node| self.add(node)),
					*inclusive,
				),
				ir::Expr::Template(parts) => Op::Template(
					parts
						.iter()
						.map(|part| match part {
							ir::Part::Text(text) => Part::Text(Arc::clone(text)),
							ir::Part::Value(value) => Part::Value(self.add(value)),
						})
						.collect(),
				),
				ir::Expr::Impure(what) => Op::Impure(Arc::clone(what)),
				ir::Expr::Unsupported(what) => Op::Unsupported(Arc::clone(what)),
			};
			nodes.push(op);
		}
		check()?;
		Ok(nodes)
	}
}
