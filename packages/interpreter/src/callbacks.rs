//! Suspending builtin algorithms retain positions, never item snapshots.

use std::sync::Arc;

use ethnum::I256;

use crate::eval::Error;
use crate::loans::Out;
use crate::value::{Value, contains};

#[derive(Clone, Copy)]
pub(crate) enum Container {
	Array,
	Set,
	Map,
}

enum Kind {
	Find,
	Filter(bool),
	Map(bool),
	Quantifier(bool),
	Reduce,
	Sort,
}

pub(crate) enum Argument {
	Item(usize),
	Value(Out),
}

pub(crate) enum Output {
	Value(Out),
	Found(Option<usize>),
	Selected(Container, Vec<usize>),
	Sorted(Vec<usize>),
}

pub(crate) enum Progress {
	Call {
		function: Value,
		arguments: Vec<Argument>,
		next: Box<Callback>,
	},
	Compare {
		left: usize,
		right: usize,
		next: Box<Callback>,
	},
	Done(Output),
}

pub(crate) struct Callback {
	kind: Kind,
	container: Container,
	function: Option<Value>,
	length: usize,
	position: usize,
	selected: Vec<usize>,
	output: Vec<Value>,
	total: Out,
	permutation: Vec<usize>,
	sort_position: usize,
}

impl Callback {
	pub fn handles(path: &str) -> bool {
		matches!(
			path,
			"Array#find"
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
		matches!(path, "Array#sortInPlace" | "Array#deleteWhere")
	}

	pub fn new(
		path: &str,
		container: Container,
		length: usize,
		arguments: Vec<Out>,
	) -> Result<Box<Self>, Error> {
		let mut arguments = arguments.into_iter();
		let function = arguments.next().and_then(|value| match value {
			Out::Value(value) => Some(value),
			Out::Loan(_) => None,
		});
		let kind = if path.ends_with("#find") {
			Kind::Find
		} else if path.ends_with("#filter") {
			Kind::Filter(false)
		} else if path.ends_with("#deleteWhere") {
			Kind::Filter(true)
		} else if path.ends_with("#map") {
			Kind::Map(matches!(container, Container::Set))
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
		if !sort && function.is_none() {
			return Err(Error::Stop("missing callback argument".into()));
		}
		let total = if matches!(kind, Kind::Reduce) {
			arguments
				.next()
				.ok_or_else(|| Error::Stop("missing initial accumulator".into()))?
		} else {
			Value::None.into()
		};
		let selected = if matches!(kind, Kind::Filter(_)) {
			Vec::with_capacity(length)
		} else {
			Vec::new()
		};
		let output = if matches!(kind, Kind::Map(_)) {
			Vec::with_capacity(length)
		} else {
			Vec::new()
		};
		Ok(Box::new(Self {
			kind,
			container,
			function,
			length,
			position: usize::from(sort),
			selected,
			output,
			total,
			permutation: if sort {
				(0..length).collect()
			} else {
				Vec::new()
			},
			sort_position: 1,
		}))
	}

	pub fn advance(mut self: Box<Self>, returned: Option<Value>) -> Result<Progress, Error> {
		if matches!(self.kind, Kind::Sort) {
			return self.sort(returned);
		}
		if let Some(value) = returned {
			match self.kind {
				Kind::Find => {
					if boolean(value)? {
						return Ok(Progress::Done(Output::Found(Some(self.position))));
					}
				}
				Kind::Filter(inverse) => {
					if boolean(value)? != inverse {
						self.selected.push(self.position);
					}
				}
				Kind::Map(_) => self.output.push(value),
				Kind::Quantifier(every) => {
					if boolean(value)? != every {
						return Ok(Progress::Done(Output::Value(Value::Boolean(!every).into())));
					}
				}
				Kind::Reduce => self.total = value.into(),
				Kind::Sort => unreachable!(),
			}
			self.position += 1;
		}
		if self.position >= self.length {
			return self.finish();
		}
		let item = Argument::Item(self.position);
		let index = Argument::Value(Value::int(self.position as i64).into());
		let arguments = match self.kind {
			Kind::Reduce => vec![
				Argument::Value(std::mem::replace(&mut self.total, Value::None.into())),
				item,
				index,
			],
			Kind::Find | Kind::Filter(true) | Kind::Quantifier(_) => vec![item],
			_ => vec![item, index],
		};
		Ok(Progress::Call {
			function: self.function.as_ref().expect("a checked callback").clone(),
			arguments,
			next: self,
		})
	}

	fn finish(self: Box<Self>) -> Result<Progress, Error> {
		let output = match self.kind {
			Kind::Find => Output::Found(None),
			Kind::Quantifier(every) => Output::Value(Value::Boolean(every).into()),
			Kind::Reduce => Output::Value(self.total),
			Kind::Filter(_) => Output::Selected(self.container, self.selected),
			Kind::Map(true) => {
				let mut set = self.output;
				let mut position = 0;
				while position < set.len() {
					if contains(&set[..position], &set[position]) {
						set.remove(position);
					} else {
						position += 1;
					}
				}
				Output::Value(Value::Set(Arc::new(set)).into())
			}
			Kind::Map(false) => Output::Value(Value::array(self.output).into()),
			Kind::Sort => unreachable!(),
		};
		Ok(Progress::Done(output))
	}

	fn sort(mut self: Box<Self>, returned: Option<Value>) -> Result<Progress, Error> {
		if let Some(value) = returned {
			let order = value
				.as_int()
				.ok_or_else(|| Error::Stop(format!("`{}` is not an integer", value.to_text())))?;
			if order > I256::ZERO {
				self.permutation
					.swap(self.sort_position - 1, self.sort_position);
				self.sort_position -= 1;
			} else {
				self.sort_position = 0;
			}
		}
		loop {
			if self.position >= self.length {
				return Ok(Progress::Done(Output::Sorted(self.permutation)));
			}
			if self.sort_position == 0 {
				self.position += 1;
				self.sort_position = self.position;
				continue;
			}
			let left = self.permutation[self.sort_position - 1];
			let right = self.permutation[self.sort_position];
			return Ok(match &self.function {
				Some(function) => Progress::Call {
					function: function.clone(),
					arguments: vec![Argument::Item(left), Argument::Item(right)],
					next: self,
				},
				None => Progress::Compare {
					left,
					right,
					next: self,
				},
			});
		}
	}
}

fn boolean(value: Value) -> Result<bool, Error> {
	value
		.as_bool()
		.ok_or_else(|| Error::Stop(format!("`{}` is not a `Boolean`", value.to_text())))
}
