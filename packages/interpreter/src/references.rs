//! Internal, evaluated places. Frame generations detect a stale handle;
//! keys are captured once, and no interpreter address is exported as Fa data.

use std::sync::Arc;

use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Key {
	Field(Arc<str>),
	Index(Value),
}

/// A place in an active evaluation frame, never an owned Fa value.
#[derive(Clone, Debug)]
pub struct Reference {
	pub(crate) frame: usize,
	pub(crate) generation: u64,
	pub(crate) slot: crate::ir::Slot,
	pub(crate) keys: Vec<Key>,
}

impl Reference {
	pub(crate) fn child(&self, key: Key) -> Self {
		let mut reference = self.clone();
		reference.keys.push(key);
		reference
	}
}
