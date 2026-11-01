//! The prelude as Rust tables (decision A12): one file per area, combined
//! into one registry. A [`BuiltinId`] is an index into [`all`].
//!
//! A built-in has a dotted path: `Integer`, `console`, `console.log`. A name
//! is looked up with [`lookup`], a member of a namespace with [`member`].

pub mod console;
pub mod scalars;

use std::sync::OnceLock;

use rustc_hash::FxHashMap;

use crate::ids::{BuiltinId, TypeId};
use crate::types::TypeStore;

/// What a built-in is.
#[derive(Copy, Clone, Debug)]
pub enum BuiltinKind {
	/// A type name (`Integer`): the type it denotes.
	Type(TypeId),
	/// A namespace of members (`console`): not a value by itself.
	Namespace,
	/// A value (functions included): its type, built in the session's store.
	Value(fn(&mut TypeStore) -> TypeId),
}

/// A built-in of the prelude.
#[derive(Copy, Clone, Debug)]
pub struct Builtin {
	/// The dotted path: `console.log`.
	pub path: &'static str,
	/// What it is.
	pub kind: BuiltinKind,
}

impl Builtin {
	/// The last segment of the path: `log`.
	pub fn name(&self) -> &'static str {
		self.path.rsplit('.').next().unwrap_or(self.path)
	}
}

struct Registry {
	all: Vec<Builtin>,
	by_path: FxHashMap<&'static str, BuiltinId>,
}

fn registry() -> &'static Registry {
	static REGISTRY: OnceLock<Registry> = OnceLock::new();
	REGISTRY.get_or_init(|| {
		let all: Vec<Builtin> = [scalars::BUILTINS, console::BUILTINS]
			.into_iter()
			.flatten()
			.copied()
			.collect();
		let by_path = all
			.iter()
			.enumerate()
			.map(|(index, builtin)| (builtin.path, BuiltinId::new(index)))
			.collect();
		Registry { all, by_path }
	})
}

/// Every built-in, indexed by [`BuiltinId`].
pub fn all() -> &'static [Builtin] {
	&registry().all
}

/// A built-in.
pub fn get(id: BuiltinId) -> &'static Builtin {
	&registry().all[id.index()]
}

/// The top-level built-in named `name`.
pub fn lookup(name: &str) -> Option<BuiltinId> {
	if name.contains('.') {
		return None;
	}
	registry().by_path.get(name).copied()
}

/// The member `name` of the built-in namespace `parent`.
pub fn member(parent: BuiltinId, name: &str) -> Option<BuiltinId> {
	let path = format!("{}.{name}", get(parent).path);
	registry().by_path.get(path.as_str()).copied()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn paths_are_unique_and_found() {
		let mut paths: Vec<_> = all().iter().map(|builtin| builtin.path).collect();
		paths.sort();
		let count = paths.len();
		paths.dedup();
		assert_eq!(paths.len(), count, "a built-in path is registered twice");
		let console = lookup("console").unwrap();
		let log = member(console, "log").unwrap();
		assert_eq!(get(log).name(), "log");
		assert!(lookup("console.log").is_none());
		assert!(matches!(
			get(lookup("Int8").unwrap()).kind,
			BuiltinKind::Type(TypeId::INT8)
		));
	}
}
