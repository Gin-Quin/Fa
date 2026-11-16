//! Converting copies execute a type graph and a value with explicit work
//! stacks. Unchanged subvalues use the VM's immutable, copy-on-write storage.

use std::sync::Arc;

use crate::eval::{self, Error, Flow, Interpreter};
use crate::ir::{CopyPlan, CopyStep};
use crate::memory;
use crate::value::{Object, Value, contains, equals};

enum Job<'a> {
	Visit(u32, &'a Value),
	Array(usize),
	Tuple(usize),
	Object(&'a [(Arc<str>, u32)]),
	Set(usize),
	Map(usize),
}

pub(crate) fn copy(
	interpreter: &mut Interpreter<'_>,
	plan: &CopyPlan,
	source: &Value,
) -> Result<Value, Flow> {
	let mut work = vec![Job::Visit(plan.root, source)];
	let mut values = Vec::new();
	while let Some(job) = work.pop() {
		interpreter.step()?;
		match job {
			Job::Visit(index, value) => {
				let step = plan.steps.get(index as usize).ok_or_else(invalid_plan)?;
				visit(interpreter, step, value, &mut work, &mut values)?;
			}
			Job::Array(count) => {
				let items = values.split_off(values.len() - count);
				values.push(Value::Array(Arc::new(items)));
			}
			Job::Tuple(count) => {
				let items = values.split_off(values.len() - count);
				values.push(Value::Tuple(Arc::new(items)));
			}
			Job::Object(fields) => {
				let items = values.split_off(values.len() - fields.len());
				let fields = fields
					.iter()
					.zip(items)
					.map(|((name, _), value)| (Arc::clone(name), value))
					.collect();
				values.push(Value::Object(Arc::new(Object { fields })));
			}
			Job::Set(count) => {
				let items = values.split_off(values.len() - count);
				let mut unique = Vec::with_capacity(count);
				for item in items {
					if !contains(&unique, &item) {
						unique.push(item)
					}
				}
				values.push(Value::Set(Arc::new(unique)));
			}
			Job::Map(count) => {
				let mut items = values.split_off(values.len() - count * 2).into_iter();
				let mut entries: Vec<(Value, Value)> = Vec::with_capacity(count);
				while let (Some(key), Some(value)) = (items.next(), items.next()) {
					if let Some((_, old)) = entries.iter_mut().find(|(old, _)| equals(old, &key)) {
						*old = value;
					} else {
						entries.push((key, value));
					}
				}
				values.push(Value::Map(Arc::new(entries)));
			}
		}
	}
	values.pop().ok_or_else(|| invalid_plan().into())
}

fn visit<'a>(
	interpreter: &mut Interpreter<'_>,
	step: &'a CopyStep,
	value: &'a Value,
	work: &mut Vec<Job<'a>>,
	values: &mut Vec<Value>,
) -> Result<(), Flow> {
	match step {
		CopyStep::Value => values.push(value.clone()),
		CopyStep::Convert(num) => values.push(eval::convert(*num, value)?),
		CopyStep::Alias(next) => work.push(Job::Visit(*next, value)),
		CopyStep::MemberData(next) => {
			let Value::Member(member) = value else {
				return Err(invalid_plan().into());
			};
			let data = member.data.as_ref().ok_or_else(invalid_plan)?;
			work.push(Job::Visit(*next, data));
		}
		CopyStep::Select(cases) => {
			let (_, next) = cases
				.iter()
				.find(|(test, _)| crate::types::test(test, value))
				.ok_or_else(invalid_plan)?;
			work.push(Job::Visit(*next, value));
		}
		CopyStep::Array(item) | CopyStep::Set(item) => {
			let (Value::Array(items) | Value::Set(items)) = value else {
				return Err(invalid_plan().into());
			};
			interpreter.reserve(memory::values_size(items.len()).saturating_mul(2))?;
			work.push(if matches!(step, CopyStep::Set(_)) {
				Job::Set(items.len())
			} else {
				Job::Array(items.len())
			});
			work.extend(items.iter().rev().map(|value| Job::Visit(*item, value)));
		}
		CopyStep::Tuple(items) => {
			let Value::Tuple(values) = value else {
				return Err(invalid_plan().into());
			};
			interpreter.reserve(memory::values_size(items.len()).saturating_mul(2))?;
			if items.len() != values.len() {
				return Err(invalid_plan().into());
			}
			work.push(Job::Tuple(items.len()));
			work.extend(
				items
					.iter()
					.zip(values.iter())
					.rev()
					.map(|(plan, value)| Job::Visit(*plan, value)),
			);
		}
		CopyStep::Object(fields) => {
			let Value::Object(object) = value else {
				return Err(invalid_plan().into());
			};
			interpreter.reserve(memory::values_size(fields.len()).saturating_mul(3))?;
			work.push(Job::Object(fields));
			for (name, next) in fields.iter().rev() {
				let field = object.get(name).ok_or_else(invalid_plan)?;
				work.push(Job::Visit(*next, field));
			}
		}
		CopyStep::Map { key, value: item } => {
			let Value::Map(entries) = value else {
				return Err(invalid_plan().into());
			};
			interpreter.reserve(memory::values_size(entries.len()).saturating_mul(4))?;
			work.push(Job::Map(entries.len()));
			for (k, v) in entries.iter().rev() {
				work.push(Job::Visit(*item, v));
				work.push(Job::Visit(*key, k));
			}
		}
	}
	Ok(())
}

fn invalid_plan() -> Error {
	Error::Stop("the value does not match its converting-copy type".into())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ir::{FnKey, Function, GlobalKey, Num, TypeTest};
	use crate::{Host, Limits};

	struct NoHost;
	impl Host for NoHost {
		fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
			Err(invalid_plan())
		}
		fn global(&mut self, _: GlobalKey) -> Result<Value, Error> {
			Err(invalid_plan())
		}
	}

	/// Peel test values so the pre-existing Rust Arc destructor, outside
	/// the evaluator, is not itself a recursive part of this copy test.
	fn release_chain(mut value: Value) {
		while let Value::Object(object) = value {
			let mut fields = Arc::try_unwrap(object).expect("independent nodes").fields;
			let next = fields
				.iter()
				.position(|(name, _)| &**name == "next")
				.unwrap();
			value = fields.swap_remove(next).1;
		}
	}

	#[test]
	fn converting_copy_walks_deep_graphs_with_a_step_budget() {
		let value_name: Arc<str> = Arc::from("value");
		let next_name: Arc<str> = Arc::from("next");
		let mut source = Value::None;
		for index in 0..100000 {
			source = Value::Object(Arc::new(Object {
				fields: vec![
					(Arc::clone(&value_name), Value::Float(index as f64)),
					(Arc::clone(&next_name), source),
				],
			}));
		}
		let plan = CopyPlan {
			root: 0,
			steps: vec![
				CopyStep::Object(vec![
					(Arc::clone(&value_name), 1),
					(Arc::clone(&next_name), 2),
				]),
				CopyStep::Convert(Num::Float),
				CopyStep::Select(vec![(TypeTest::None, 3), (TypeTest::Object, 0)]),
				CopyStep::Value,
			],
		};
		let mut host = NoHost;
		let mut interpreter = Interpreter::new(&mut host, Limits::default());
		let copied = copy(&mut interpreter, &plan, &source)
			.unwrap_or_else(|flow| panic!("{}", Error::from(flow).message()));
		let (mut left, mut right) = (&source, &copied);
		let mut count = 0;
		while let (Value::Object(a), Value::Object(b)) = (left, right) {
			assert!(!Arc::ptr_eq(a, b));
			assert!(matches!(b.get("value"), Some(Value::Float(_))));
			left = a.get("next").unwrap();
			right = b.get("next").unwrap();
			count += 1;
		}
		assert_eq!(count, 100000);
		let mut limited = Interpreter::new(
			&mut host,
			Limits {
				steps: 8,
				..Limits::default()
			},
		);
		assert!(matches!(
			copy(&mut limited, &plan, &source),
			Err(Flow::Error(Error::Steps(8)))
		));
		release_chain(source);
		release_chain(copied);
	}
}
