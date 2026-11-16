//! Machine-local, non-owning paths. No loan is a cacheable Fa value.

use std::sync::Arc;

use crate::ir::Slot;
use crate::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Owner {
	pub frame: usize,
	pub generation: u64,
	pub slot: Slot,
}

#[derive(Clone, Debug)]
pub(crate) enum Step {
	Field(Arc<str>),
	Index(Value),
	Capture(usize),
	Bound(usize),
	Item(usize),
	Entry(usize),
}

/// A map entry is a tuple view of its stored key and value, not an owned
/// tuple containing copies of them.
#[derive(Clone, Copy, Debug)]
pub(crate) enum View<'v> {
	Value(&'v Value),
	Entry(&'v (Value, Value)),
}

impl<'v> View<'v> {
	pub fn materialize(self) -> Value {
		match self {
			Self::Value(value) => value.clone(),
			Self::Entry((key, value)) => Value::Tuple(Arc::new(vec![key.clone(), value.clone()])),
		}
	}

	pub fn project(self, step: &Step) -> Result<Self, crate::Error> {
		use crate::{Error, builtins};
		if let Self::Entry(pair) = self {
			let position = match step {
				Step::Index(key) => builtins::position(builtins::integer(key)?, 2)?,
				Step::Item(index) if *index < 2 => *index,
				_ => {
					return Err(Error::Stop(
						"internal: a map entry is projected by tuple position".into(),
					));
				}
			};
			return Ok(Self::Value(if position == 0 { &pair.0 } else { &pair.1 }));
		}
		let Self::Value(value) = self else {
			unreachable!()
		};
		Ok(Self::Value(match step {
			Step::Field(name) => builtins::field_ref(value, name)?,
			Step::Index(index) => builtins::index_ref(value, index)?,
			Step::Item(index) => match value {
				Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
					items.get(*index).ok_or_else(|| {
						Error::Stop("internal: an iterated item was invalidated".into())
					})?
				}
				_ => {
					return Err(Error::Stop(
						"internal: an item view requires a collection".into(),
					));
				}
			},
			Step::Entry(index) => {
				let Value::Map(entries) = value else {
					return Err(Error::Stop("internal: an entry view requires a map".into()));
				};
				return Ok(Self::Entry(entries.get(*index).ok_or_else(|| {
					Error::Stop("internal: an iterated entry was invalidated".into())
				})?));
			}
			Step::Capture(index) | Step::Bound(index) => {
				let Value::Function(closure) = value else {
					return Err(Error::Stop(
						"internal: an environment root is not a function".into(),
					));
				};
				let found = if matches!(step, Step::Bound(_)) {
					closure.bound.get(*index).map(|(_, value)| value)
				} else {
					closure.captured.get(*index)
				};
				found.ok_or_else(|| {
					Error::Stop("internal: an environment entry is missing".into())
				})?
			}
		}))
	}
}

#[derive(Clone, Default, Debug)]
struct Path(Option<Arc<Link>>);

#[derive(Debug)]
struct Link {
	parent: Path,
	step: Step,
}

impl Drop for Path {
	fn drop(&mut self) {
		// A reseated deep reference can release its entire path at once.
		// Do not turn that release into recursion on the Rust stack.
		while let Some(link) = self.0.take() {
			let Ok(mut link) = Arc::try_unwrap(link) else {
				break;
			};
			self.0 = link.parent.0.take();
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct Loan {
	pub owner: Owner,
	path: Path,
	depth: usize,
}

impl Loan {
	pub fn new(owner: Owner) -> Self {
		Self {
			owner,
			path: Path::default(),
			depth: 0,
		}
	}

	pub fn push(&mut self, step: Step) {
		self.depth += 1;
		self.path = Path(Some(Arc::new(Link {
			parent: self.path.clone(),
			step,
		})));
	}

	pub fn steps(&self) -> Vec<&Step> {
		let mut steps = Vec::with_capacity(self.depth);
		let mut next = &self.path;
		while let Some(link) = &next.0 {
			steps.push(&link.step);
			next = &link.parent;
		}
		steps.reverse();
		steps
	}

	pub fn lookup_bytes(&self) -> usize {
		self.depth.saturating_mul(size_of::<&Step>())
	}
	pub fn projection_bytes(steps: usize) -> usize {
		steps.saturating_mul(size_of::<Link>() + 2 * size_of::<usize>())
	}
}

/// Internal expression results and bindings retain ownership provenance.
#[derive(Clone, Debug)]
pub(crate) enum Out {
	Value(Value),
	Loan(Loan),
}

impl From<Value> for Out {
	fn from(value: Value) -> Self {
		Self::Value(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_deep_persistent_path_releases_without_native_recursion() {
		std::thread::Builder::new()
			.stack_size(64 << 10)
			.spawn(|| {
				let mut loan = Loan::new(Owner {
					frame: 0,
					generation: 1,
					slot: 0,
				});
				for _ in 0..100_000 {
					loan.push(Step::Field(Arc::from("next")));
				}
				assert_eq!(loan.steps().len(), 100_000);
				drop(loan);
			})
			.unwrap()
			.join()
			.unwrap();
	}
}
