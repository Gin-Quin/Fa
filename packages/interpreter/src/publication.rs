//! Borrowed environments may circulate within one machine, never in caches.

use std::collections::HashSet;
use std::sync::Arc;

use crate::{Error, Interpreter, Value};

impl Interpreter<'_> {
	pub(crate) fn publish(&self, result: Result<Value, Error>) -> Result<Value, Error> {
		match &result {
			Ok(value) | Err(Error::Thrown(value)) => self.portable(value)?,
			Err(_) => {}
		}
		result
	}

	pub(crate) fn portable(&self, value: &Value) -> Result<(), Error> {
		let mut pending = vec![value];
		let mut seen = HashSet::new();
		while let Some(value) = pending.pop() {
			let (kind, pointer, children) = match value {
				Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
					(0, Arc::as_ptr(items) as usize, items.len())
				}
				Value::Object(object) => (1, Arc::as_ptr(object) as usize, object.fields.len()),
				Value::Map(entries) => (
					2,
					Arc::as_ptr(entries) as usize,
					entries.len().saturating_mul(2),
				),
				Value::Member(member) => (
					3,
					Arc::as_ptr(member) as usize,
					usize::from(member.data.is_some()),
				),
				Value::Function(closure) => {
					if closure.environment.is_some() {
						return Err(Error::Stop(
							"internal: a borrowed callable environment cannot leave its evaluation"
								.into(),
						));
					}
					(
						4,
						Arc::as_ptr(closure) as usize,
						closure.captured.len() + closure.bound.len(),
					)
				}
				_ => continue,
			};
			if seen.contains(&(kind, pointer)) {
				continue;
			}
			let stack_bytes = if pending.len().saturating_add(children) > pending.capacity() {
				pending
					.len()
					.saturating_add(children)
					.saturating_mul(2 * size_of::<&Value>())
			} else {
				0
			};
			let seen_bytes = if seen.len() == seen.capacity() {
				seen.len()
					.saturating_add(1)
					.saturating_mul(4 * size_of::<(u8, usize)>())
			} else {
				0
			};
			self.reserve(stack_bytes.saturating_add(seen_bytes))?;
			seen.insert((kind, pointer));
			match value {
				Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
					pending.extend(items.iter())
				}
				Value::Object(object) => {
					pending.extend(object.fields.iter().map(|(_, value)| value))
				}
				Value::Map(entries) => {
					pending.extend(entries.iter().flat_map(|(key, value)| [key, value]))
				}
				Value::Member(member) => pending.extend(member.data.iter()),
				Value::Function(closure) => {
					pending.extend(closure.captured.iter());
					pending.extend(closure.bound.iter().map(|(_, value)| value));
				}
				_ => unreachable!("only container values enter the publication walk"),
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ir::{Expr, FnKey, Function, GlobalKey};
	use crate::loans::{Loan, Owner};
	use crate::value::{Closure, Member, MemberKind};

	struct EmptyHost;
	impl crate::Host for EmptyHost {
		fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
			unreachable!()
		}
		fn global(&mut self, _: GlobalKey) -> Result<Value, Error> {
			unreachable!()
		}
	}

	#[test]
	fn every_container_and_error_publication_rejects_a_borrowed_environment() {
		let mut closure = Closure::new(FnKey(0, 0), Vec::new(), Vec::new());
		closure.environment = Some(vec![Loan::new(Owner {
			frame: 0,
			generation: 1,
			slot: 0,
		})]);
		let borrowed = Value::Function(Arc::new(closure));
		let containers = [
			borrowed.clone(),
			Value::array(vec![borrowed.clone()]),
			Value::Tuple(Arc::new(vec![borrowed.clone()])),
			Value::Set(Arc::new(vec![borrowed.clone()])),
			Value::Map(Arc::new(vec![(borrowed.clone(), Value::None)])),
			Value::Map(Arc::new(vec![(Value::None, borrowed.clone())])),
			Value::object(vec![("nested", borrowed.clone())]),
			Value::Member(Arc::new(Member {
				name: "Failure".into(),
				kind: MemberKind::Error,
				data: Some(borrowed.clone()),
			})),
			Value::Function(Arc::new(Closure::new(
				FnKey(0, 1),
				vec![borrowed.clone()],
				Vec::new(),
			))),
			Value::Function(Arc::new(Closure::new(
				FnKey(0, 1),
				Vec::new(),
				vec![(0, borrowed.clone())],
			))),
		];
		let mut host = EmptyHost;
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		for value in containers {
			assert!(
				matches!(interpreter.evaluate(&Expr::Const(value.clone()), 0), Err(Error::Stop(message)) if message.contains("borrowed callable"))
			);
			assert!(
				matches!(interpreter.publish(Err(Error::Thrown(value))), Err(Error::Stop(message)) if message.contains("borrowed callable"))
			);
		}
		let mut receiver = Value::array(Vec::new());
		assert!(
			matches!(crate::builtins::mutate(&mut interpreter, "Array#push", &mut receiver, vec![borrowed]), Err(Error::Stop(message)) if message.contains("borrowed callable"))
		);
		assert_eq!(receiver, Value::None);
	}

	#[test]
	fn owned_environments_are_portable_and_can_be_shared_in_a_cache() {
		let closure = Value::Function(Arc::new(Closure::new(
			FnKey(0, 1),
			vec![Value::array(vec![Value::int(2)])],
			Vec::new(),
		)));
		let mut host = EmptyHost;
		let mut interpreter = Interpreter::new(&mut host, crate::Limits::default());
		assert_eq!(
			interpreter.evaluate(&Expr::Const(closure.clone()), 0),
			Ok(closure)
		);
	}
}
