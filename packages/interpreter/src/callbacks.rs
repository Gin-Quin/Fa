//! Suspending builtin algorithms retain positions, never item snapshots.

use ethnum::I256;

use crate::eval::Error;
use crate::loans::Out;
use crate::value::Value;

#[derive(Clone, Copy)]
pub(crate) enum Container {
	Array,
	Set,
	Map,
	Pool,
}

enum Kind {
	Json(Box<crate::json::Reader>),
	Find,
	FindIndex,
	Filter,
	Delete,
	Map,
	Quantifier(bool),
	Reduce,
	Sort,
}

pub(crate) enum Argument {
	Item(usize),
	Key(usize),
	MapValue(usize),
	Value(Out),
}

pub(crate) enum Output {
	Value(Out),
	Found(Option<usize>),
	Selected(Container, Vec<usize>),
	Deleted { kept: usize, unvisited: usize },
	Sorted(Vec<usize>),
	Mapped(Container, Vec<Value>),
}

pub(crate) enum Progress {
	Swap {
		left: usize,
		right: usize,
		next: Box<Callback>,
	},
	Delete {
		position: usize,
		kept: usize,
		delete: bool,
		next: Box<Callback>,
	},
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
	kept: usize,
	staged: bool,
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
			"Json.parse"
				| "Array#find"
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
				| "Map#deleteWhere"
				| "Set#deleteWhere"
				| "Pool#deleteWhere"
				| "Columnar#deleteWhere"
				| "OrderedColumnar#deleteWhere"
		)
	}

	pub fn new(
		path: &str,
		container: Container,
		length: usize,
		arguments: Vec<Out>,
		staged: bool,
	) -> Result<Box<Self>, Error> {
		let mut arguments = arguments.into_iter();
		let first = arguments.next().and_then(|value| match value {
			Out::Value(value) => Some(value),
			Out::Loan(_) => None,
		});
		let function = if path == "Json.parse" {
			None
		} else {
			first.clone()
		};
		let kind = if path == "Json.parse" {
			let input = first.ok_or_else(|| Error::Stop("missing JSON input".into()))?;
			let schema = match arguments.next() {
				Some(Out::Value(value)) => value,
				_ => return Err(Error::Stop("missing JSON schema".into())),
			};
			let strict = match arguments.next() {
				Some(Out::Value(value)) => value.as_bool().unwrap_or(false),
				_ => false,
			};
			Kind::Json(Box::new(crate::json::Reader::new(&input, schema, strict)?))
		} else if path.ends_with("#findIndex") {
			Kind::FindIndex
		} else if path.ends_with("#find") {
			Kind::Find
		} else if path.ends_with("#filter") {
			Kind::Filter
		} else if path.ends_with("#deleteWhere") {
			Kind::Delete
		} else if path.ends_with("#map") {
			Kind::Map
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
		if !sort && !matches!(kind, Kind::Json(_)) && function.is_none() {
			return Err(Error::Stop("missing callback argument".into()));
		}
		let total = if matches!(kind, Kind::Reduce) {
			arguments
				.next()
				.ok_or_else(|| Error::Stop("missing initial accumulator".into()))?
		} else {
			Value::None.into()
		};
		let selected = if matches!(kind, Kind::Filter) {
			Vec::with_capacity(length)
		} else {
			Vec::new()
		};
		let output = if matches!(kind, Kind::Map) {
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
			permutation: if sort && staged {
				(0..length).collect()
			} else {
				Vec::new()
			},
			sort_position: 1,
			kept: 0,
			staged,
		}))
	}

	pub fn advance(
		mut self: Box<Self>,
		returned: Option<Value>,
		interpreter: &mut crate::Interpreter<'_>,
	) -> Result<Progress, Error> {
		if let Kind::Json(reader) = &mut self.kind {
			return match reader.advance(returned, || interpreter.step().map_err(Error::from))? {
				crate::json::ReadProgress::Done(value) => {
					Ok(Progress::Done(Output::Value(value.into())))
				}
				crate::json::ReadProgress::Call(function) => Ok(Progress::Call {
					function,
					arguments: Vec::new(),
					next: self,
				}),
			};
		}
		if matches!(self.kind, Kind::Sort) {
			return self.sort(returned);
		}
		if let Some(value) = returned {
			match self.kind {
				Kind::Find | Kind::FindIndex => {
					if boolean(value)? {
						return Ok(Progress::Done(if matches!(self.kind, Kind::FindIndex) {
							Output::Value(Value::int(self.position as i64).into())
						} else {
							Output::Found(Some(self.position))
						}));
					}
				}
				Kind::Filter => {
					if boolean(value)? {
						self.selected.push(self.position);
					}
				}
				Kind::Delete => {
					let delete = boolean(value)?;
					let position = self.position;
					let kept = self.kept;
					self.position += 1;
					self.kept += usize::from(!delete);
					return Ok(Progress::Delete {
						position,
						kept,
						delete,
						next: self,
					});
				}
				Kind::Map => self.output.push(value),
				Kind::Quantifier(every) => {
					if boolean(value)? != every {
						return Ok(Progress::Done(Output::Value(Value::Boolean(!every).into())));
					}
				}
				Kind::Reduce => self.total = value.into(),
				Kind::Sort | Kind::Json(_) => unreachable!(),
			}
			self.position += 1;
		}
		if self.position >= self.length {
			return self.finish();
		}
		let mut arguments = if matches!(self.kind, Kind::Reduce) {
			vec![Argument::Value(std::mem::replace(
				&mut self.total,
				Value::None.into(),
			))]
		} else {
			Vec::new()
		};
		match self.container {
			Container::Map => {
				arguments.push(Argument::Key(self.position));
				arguments.push(Argument::MapValue(self.position));
			}
			Container::Array => {
				arguments.push(Argument::Item(self.position));
				if !matches!(self.kind, Kind::Delete) {
					arguments.push(Argument::Value(Value::int(self.position as i64).into()));
				}
			}
			Container::Set | Container::Pool => arguments.push(Argument::Item(self.position)),
		}
		Ok(Progress::Call {
			function: self.function.as_ref().expect("a checked callback").clone(),
			arguments,
			next: self,
		})
	}

	fn finish(self: Box<Self>) -> Result<Progress, Error> {
		let output = match self.kind {
			Kind::Find | Kind::FindIndex => Output::Found(None),
			Kind::Quantifier(every) => Output::Value(Value::Boolean(every).into()),
			Kind::Reduce => Output::Value(self.total),
			Kind::Filter => Output::Selected(self.container, self.selected),
			Kind::Delete => Output::Deleted {
				kept: self.kept,
				unvisited: self.position,
			},
			Kind::Map => Output::Mapped(self.container, self.output),
			Kind::Sort | Kind::Json(_) => unreachable!(),
		};
		Ok(Progress::Done(output))
	}

	/// Completed deletions survive a predicate failure; the failing item
	/// and the unvisited tail remain after the retained prefix.
	pub fn interrupted(self: Box<Self>) -> Option<Output> {
		if !matches!(self.kind, Kind::Delete) {
			return None;
		}
		Some(Output::Deleted {
			kept: self.kept,
			unvisited: self.position,
		})
	}

	fn sort(mut self: Box<Self>, returned: Option<Value>) -> Result<Progress, Error> {
		if let Some(value) = returned {
			let positive = match value {
				Value::Integer(order) => order > I256::ZERO,
				Value::Float(order) | Value::Percentage(order) => order > 0.0,
				_ => return Err(Error::Stop("a comparator must return a number".into())),
			};
			if positive {
				let right = self.sort_position;
				self.sort_position -= 1;
				if self.staged {
					self.permutation.swap(self.sort_position, right);
				} else {
					let left = self.sort_position;
					return Ok(Progress::Swap {
						left,
						right,
						next: self,
					});
				}
			} else {
				self.sort_position = 0;
			}
		}
		loop {
			if self.position >= self.length {
				return Ok(Progress::Done(if self.staged {
					Output::Sorted(self.permutation)
				} else {
					Output::Value(Value::None.into())
				}));
			}
			if self.sort_position == 0 {
				self.position += 1;
				self.sort_position = self.position;
				continue;
			}
			let left = if self.staged {
				self.permutation[self.sort_position - 1]
			} else {
				self.sort_position - 1
			};
			let right = if self.staged {
				self.permutation[self.sort_position]
			} else {
				self.sort_position
			};
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

/// Compaction consumes the original slot; no retained item is cloned.
pub(crate) fn compact_item<T>(
	items: &mut [T],
	position: usize,
	kept: usize,
	delete: bool,
	empty: T,
) {
	if delete {
		items[position] = empty;
	} else if position != kept {
		items[kept] = std::mem::replace(&mut items[position], empty);
	}
}

pub(crate) fn finish_deletions<T>(
	items: &mut Vec<T>,
	kept: usize,
	unvisited: usize,
	empty: impl Fn() -> T,
) {
	if kept != unvisited {
		for position in unvisited..items.len() {
			compact_item(items, position, kept + position - unvisited, false, empty());
		}
	}
	items.truncate(kept + items.len() - unvisited);
}
