//! Built-ins that suspend while calling Fa code. Their state lives beside
//! ordinary expression continuations, so a callback can recurse through
//! another built-in without growing the native stack.

use std::sync::Arc;

use ethnum::I256;

use crate::builtins;
use crate::eval::{self, Error};
use crate::ir::CompareOp;
use crate::value::{Pool, Value, contains};

enum Container {
	Array,
	Pool,
	Set,
	Map,
}
enum Kind {
	Find,
	FindIndex,
	Filter(Container, bool),
	Map(bool),
	Quantifier(bool),
	Reduce,
	Sort,
}

pub(crate) struct Callback {
	kind: Kind,
	function: Option<Value>,
	items: Vec<Value>,
	position: usize,
	output: Vec<Value>,
	total: Value,
	sort_position: usize,
}

pub(crate) enum Progress {
	Call {
		function: Value,
		arguments: Vec<Value>,
		next: Box<Callback>,
	},
	Done(Value),
}

impl Callback {
	pub fn handles(path: &str) -> bool {
		if let Some(("Pool" | "Columnar" | "OrderedColumnar", operation)) = path.split_once('#') {
			return matches!(
				operation,
				"find" | "filter" | "map" | "every" | "some" | "reduce"
			);
		}
		matches!(
			path,
			"Array#find"
				| "Pool#find"
				| "Columnar#find"
				| "OrderedColumnar#find"
				| "Array#findIndex"
				| "Set#find" | "Map#find"
				| "Array#filter"
				| "Set#filter"
				| "Map#filter"
				| "Array#map"
				| "Set#map" | "Map#map"
				| "Array#every"
				| "Set#every"
				| "Map#every"
				| "Array#some"
				| "Set#some" | "Map#some"
				| "Array#reduce"
				| "Set#reduce"
				| "Map#reduce"
				| "Array#sort"
		)
	}

	pub fn mutating(path: &str) -> bool {
		matches!(
			path,
			"Array#sortInPlace"
				| "Array#deleteWhere"
				| "Pool#deleteWhere"
				| "Columnar#deleteWhere"
				| "OrderedColumnar#deleteWhere"
		)
	}

	pub fn new(path: &str, arguments: Vec<Value>) -> Result<Box<Self>, Error> {
		let receiver = arguments.first().cloned().unwrap_or(Value::None);
		let arg = |index: usize| {
			arguments
				.get(index)
				.cloned()
				.ok_or_else(|| Error::Stop(format!("missing argument {index}")))
		};
		let container = match receiver {
			Value::Set(_) => Container::Set,
			Value::Pool(_) => Container::Pool,
			Value::Map(_) => Container::Map,
			_ => Container::Array,
		};
		let kind = if path.ends_with("#findIndex") {
			Kind::FindIndex
		} else if path.ends_with("#find") {
			Kind::Find
		} else if path.ends_with("#filter") {
			Kind::Filter(container, false)
		} else if path.ends_with("#deleteWhere") {
			Kind::Filter(container, true)
		} else if path.ends_with("#map") {
			Kind::Map(matches!(receiver, Value::Set(_)))
		} else if path.ends_with("#every") {
			Kind::Quantifier(true)
		} else if path.ends_with("#some") {
			Kind::Quantifier(false)
		} else if path.ends_with("#reduce") {
			Kind::Reduce
		} else {
			Kind::Sort
		};
		let sort = matches!(kind, Kind::Sort);
		let function = if sort {
			arguments.get(1).cloned()
		} else {
			Some(arg(1)?)
		};
		let total = if matches!(kind, Kind::Reduce) {
			arg(2)?
		} else {
			Value::None
		};
		let items = if sort {
			receiver.items().unwrap_or(&[]).to_vec()
		} else {
			builtins::iterate(&receiver)?
		};
		Ok(Box::new(Self {
			kind,
			function,
			items,
			position: usize::from(sort),
			output: Vec::new(),
			total,
			sort_position: 1,
		}))
	}

	pub fn advance(mut self: Box<Self>, returned: Option<Value>) -> Result<Progress, Error> {
		if matches!(self.kind, Kind::Sort) {
			return self.sort(returned);
		}
		if let Some(value) = returned {
			let item = &self.items[self.position];
			match &self.kind {
				Kind::Find => {
					if boolean(value)? {
						return Ok(Progress::Done(item.clone()));
					}
				}
				Kind::FindIndex => {
					if boolean(value)? {
						return Ok(Progress::Done(Value::int(self.position as i64)));
					}
				}
				Kind::Filter(_, inverse) => {
					if boolean(value)? != *inverse {
						self.output.push(item.clone());
					}
				}
				Kind::Map(_) => self.output.push(value),
				Kind::Quantifier(every) => {
					if boolean(value)? != *every {
						return Ok(Progress::Done(Value::Boolean(!every)));
					}
				}
				Kind::Reduce => self.total = value,
				Kind::Sort => unreachable!(),
			}
			self.position += 1;
		}
		let Some(item) = self.items.get(self.position) else {
			return self.finish();
		};
		let arguments = match self.kind {
			Kind::Reduce => vec![
				std::mem::replace(&mut self.total, Value::None),
				item.clone(),
				Value::int(self.position as i64),
			],
			Kind::Filter(_, true) | Kind::Find | Kind::FindIndex | Kind::Quantifier(_) => {
				vec![item.clone()]
			}
			_ => vec![item.clone(), Value::int(self.position as i64)],
		};
		Ok(Progress::Call {
			function: self.function.as_ref().expect("a callback").clone(),
			arguments,
			next: self,
		})
	}

	fn finish(self: Box<Self>) -> Result<Progress, Error> {
		let value = match self.kind {
			Kind::Find | Kind::FindIndex => Value::None,
			Kind::Quantifier(every) => Value::Boolean(every),
			Kind::Reduce => self.total,
			Kind::Filter(Container::Set, _) => Value::Set(Arc::new(self.output)),
			Kind::Filter(Container::Pool, _) => Value::Pool(Arc::new(Pool::new(self.output))),
			Kind::Map(true) => {
				let mut set = Vec::new();
				for value in self.output {
					if !contains(&set, &value) {
						set.push(value);
					}
				}
				Value::Set(Arc::new(set))
			}
			Kind::Filter(Container::Map, _) => {
				let entries = self
					.output
					.into_iter()
					.map(builtins::pair)
					.collect::<Result<Vec<_>, _>>()?;
				Value::Map(Arc::new(entries))
			}
			_ => Value::Array(Arc::new(self.output)),
		};
		Ok(Progress::Done(value))
	}

	fn sort(mut self: Box<Self>, returned: Option<Value>) -> Result<Progress, Error> {
		if let Some(value) = returned {
			let order = value
				.as_int()
				.ok_or_else(|| Error::Stop(format!("`{}` is not an integer", value.to_text())))?;
			if order > I256::ZERO {
				self.items.swap(self.sort_position - 1, self.sort_position);
				self.sort_position -= 1;
			} else {
				self.sort_position = 0;
			}
		}
		loop {
			if self.position >= self.items.len() {
				return Ok(Progress::Done(Value::Array(Arc::new(self.items))));
			}
			if self.sort_position == 0 {
				self.position += 1;
				self.sort_position = self.position;
				continue;
			}
			if let Some(function) = &self.function {
				return Ok(Progress::Call {
					function: function.clone(),
					arguments: vec![
						self.items[self.sort_position - 1].clone(),
						self.items[self.sort_position].clone(),
					],
					next: self,
				});
			}
			if eval::compare(
				CompareOp::Greater,
				&self.items[self.sort_position - 1],
				&self.items[self.sort_position],
			)? {
				self.items.swap(self.sort_position - 1, self.sort_position);
				self.sort_position -= 1;
			} else {
				self.sort_position = 0;
			}
		}
	}
}

fn boolean(value: Value) -> Result<bool, Error> {
	value
		.as_bool()
		.ok_or_else(|| Error::Stop(format!("`{}` is not a `Boolean`", value.to_text())))
}
