//! Effect paths and effect sets of signatures (advanced brief §3, types
//! brief §4.3): normalization, `covers`, printing.
//!
//! A path is a root (a parameter, `self`, a module-level value, a
//! requirement) and steps (`.field`, `[index]`, `[..]`, `.length`,
//! `.size`), interned in the [`TypeStore`] as a [`PathId`]. An effect set
//! holds `mutates` and `invalidates` paths, normalized: a path covered by
//! another of the same kind is dropped, and they are sorted.

use crate::ids::{EffectSetId, PathId};
use crate::types::{ConstValue, PathRoot, PathStep, TypeStore};

/// Whether `declared` covers `actual` (advanced brief §3.6): same root, and
/// each step of `declared` matches the step of `actual` at its position
/// (`[..]` matches any index), `declared` being no longer than `actual`.
pub fn path_covers(types: &TypeStore, declared: PathId, actual: PathId) -> bool {
	if declared == actual {
		return true;
	}
	let (d, d_steps) = types.path_data(declared);
	let (a, a_steps) = types.path_data(actual);
	if d.root != a.root || d_steps.len() > a_steps.len() {
		return false;
	}
	d_steps
		.iter()
		.zip(a_steps)
		.all(|(d, a)| step_covers(*d, *a))
}

fn step_covers(declared: PathStep, actual: PathStep) -> bool {
	declared == actual
		|| (declared == PathStep::All
			&& matches!(
				actual,
				PathStep::All
					| PathStep::IndexParam(_)
					| PathStep::IndexConst(_)
					| PathStep::IndexFrom(_)
					| PathStep::Last
			))
}

/// Whether `actual` is strictly below `declared` through at least one
/// owning edge (an element step): `mutates c` then covers `invalidates
/// c[i]` (decision D41: deleting elements may be declared `mutates self`).
fn below_through_element(types: &TypeStore, declared: PathId, actual: PathId) -> bool {
	let (_, d_steps) = types.path_data(declared);
	let (_, a_steps) = types.path_data(actual);
	path_covers(types, declared, actual)
		&& a_steps[d_steps.len()..].iter().any(|step| {
			matches!(
				step,
				PathStep::All
					| PathStep::IndexParam(_)
					| PathStep::IndexConst(_)
					| PathStep::IndexFrom(_)
					| PathStep::Last
			)
		})
}

/// Whether the effect set `declared` covers the effect `actual`
/// (`invalidates` when `invalidates` is true): `invalidates D` covers both
/// kinds, `mutates D` covers `mutates`, and `invalidates` strictly below
/// an element of `D`.
pub fn set_covers(
	types: &TypeStore,
	declared: EffectSetId,
	actual: PathId,
	invalidates: bool,
) -> bool {
	let (mutates, invalidated) = types.effects(declared);
	invalidated
		.iter()
		.any(|&path| path_covers(types, path, actual))
		|| mutates.iter().any(|&path| {
			if invalidates {
				below_through_element(types, path, actual)
			} else {
				path_covers(types, path, actual)
			}
		})
}

/// Whether a declared path covers no actual effect (`EFF011`).
pub fn covers_any(
	types: &TypeStore,
	declared: PathId,
	declared_invalidates: bool,
	actual: &[(PathId, bool)],
) -> bool {
	actual.iter().any(|&(path, invalidates)| {
		if declared_invalidates || !invalidates {
			path_covers(types, declared, path)
		} else {
			below_through_element(types, declared, path)
		}
	})
}

/// Interns a normalized effect set: duplicates and paths covered by another
/// path of the same kind are dropped, the rest sorted by their printed form.
pub fn normalized_set(
	types: &mut TypeStore,
	mutates: &[PathId],
	invalidates: &[PathId],
) -> EffectSetId {
	let keep = |types: &TypeStore, paths: &[PathId]| -> Vec<PathId> {
		let mut kept: Vec<PathId> = Vec::new();
		for (index, &path) in paths.iter().enumerate() {
			if kept.contains(&path) {
				continue;
			}
			let covered = paths
				.iter()
				.enumerate()
				.any(|(other, &by)| other != index && by != path && path_covers(types, by, path));
			if !covered {
				kept.push(path);
			}
		}
		kept.sort_by_cached_key(|&path| path_text(types, path, &|_| None));
		kept
	};
	let invalidated = keep(types, invalidates);
	// `invalidates P` implies `mutates P`
	let mutated: Vec<PathId> = keep(types, mutates)
		.into_iter()
		.filter(|&path| !invalidated.iter().any(|&by| path_covers(types, by, path)))
		.collect();
	if mutated.is_empty() && invalidated.is_empty() {
		return EffectSetId::EMPTY;
	}
	types.effect_set(&mutated, &invalidated)
}

/// `path_text` asks the name of the requirement `i` as the parameter
/// `REQUIREMENT_NAMES + i`.
pub const REQUIREMENT_NAMES: u16 = 0x8000;

/// Prints a path: `player.score`, `players[..].score`, `people.size`.
/// `param` names a parameter by its index (`None`: `#index`).
pub fn path_text(types: &TypeStore, path: PathId, param: &dyn Fn(u16) -> Option<String>) -> String {
	let (data, steps) = types.path_data(path);
	let mut text = match data.root {
		PathRoot::Param(index) => param(index).unwrap_or_else(|| format!("#{index}")),
		PathRoot::SelfParam => "self".to_string(),
		PathRoot::ModuleValue(key) => {
			let names = types.decl_key_data(key).names;
			types
				.sym_list(names)
				.last()
				.map(|&name| types.syms.resolve(name).to_string())
				.unwrap_or_default()
		}
		PathRoot::Requirement(index) => {
			param(REQUIREMENT_NAMES + index).unwrap_or_else(|| format!("requirement #{index}"))
		}
	};
	for step in steps {
		match *step {
			PathStep::Field(name) => {
				text.push('.');
				text.push_str(types.syms.resolve(name));
			}
			PathStep::Private(name) => {
				text.push_str(".#");
				text.push_str(types.syms.resolve(name));
			}
			PathStep::IndexParam(index) => {
				let name = param(index).unwrap_or_else(|| format!("#{index}"));
				text.push_str(&format!("[{name}]"));
			}
			PathStep::IndexConst(value) => match types.const_value(value) {
				ConstValue::Int(value) => text.push_str(&format!("[{value}]")),
				_ => text.push_str("[..]"),
			},
			PathStep::IndexFrom(index) => {
				let name = param(index).unwrap_or_else(|| format!("#{index}"));
				text.push_str(&format!("[{name}..]"));
			}
			PathStep::Last => text.push_str("[last]"),
			PathStep::All => text.push_str("[..]"),
			PathStep::Length => text.push_str(".length"),
			PathStep::Size => text.push_str(".size"),
		}
	}
	text
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::ConstId;

	struct Fixture {
		types: TypeStore,
	}

	impl Fixture {
		fn path(&mut self, root: PathRoot, steps: &[&str]) -> PathId {
			let steps: Vec<PathStep> = steps
				.iter()
				.map(|step| match *step {
					"[..]" => PathStep::All,
					"length" => PathStep::Length,
					"size" => PathStep::Size,
					"[0]" => PathStep::IndexConst(self.zero()),
					"[p1]" => PathStep::IndexParam(1),
					field => PathStep::Field(self.types.syms.intern(field)),
				})
				.collect();
			self.types.path(root, &steps)
		}

		fn zero(&mut self) -> ConstId {
			self.types.constant(ConstValue::Int(ethnum::I256::ZERO))
		}
	}

	#[test]
	fn declared_paths_cover_their_sub_paths() {
		let mut fx = Fixture {
			types: TypeStore::new(),
		};
		let player = fx.path(PathRoot::Param(0), &[]);
		let score = fx.path(PathRoot::Param(0), &["score"]);
		let name = fx.path(PathRoot::Param(0), &["name"]);
		let other = fx.path(PathRoot::Param(1), &["score"]);
		assert!(path_covers(&fx.types, player, score));
		assert!(path_covers(&fx.types, score, score));
		assert!(!path_covers(&fx.types, score, name));
		assert!(!path_covers(&fx.types, score, player));
		assert!(!path_covers(&fx.types, score, other));
		let all = fx.path(PathRoot::Param(0), &["[..]", "score"]);
		let first = fx.path(PathRoot::Param(0), &["[0]", "score"]);
		let at = fx.path(PathRoot::Param(0), &["[p1]", "score"]);
		assert!(path_covers(&fx.types, all, first));
		assert!(path_covers(&fx.types, all, at));
		assert!(!path_covers(&fx.types, first, all));
	}

	#[test]
	fn sets_cover_by_kind() {
		let mut fx = Fixture {
			types: TypeStore::new(),
		};
		let items = fx.path(PathRoot::SelfParam, &[]);
		let element = fx.path(PathRoot::SelfParam, &["[..]"]);
		let size = fx.path(PathRoot::SelfParam, &["size"]);
		let mutates_items = normalized_set(&mut fx.types, &[items], &[]);
		// `mutates self` covers the deletion of elements (D41), not of itself
		assert!(set_covers(&fx.types, mutates_items, element, true));
		assert!(!set_covers(&fx.types, mutates_items, items, true));
		assert!(set_covers(&fx.types, mutates_items, size, false));
		let invalidates_element = normalized_set(&mut fx.types, &[], &[element]);
		assert!(set_covers(&fx.types, invalidates_element, element, false));
		assert!(!set_covers(&fx.types, invalidates_element, size, false));
	}

	#[test]
	fn normalization_drops_covered_paths() {
		let mut fx = Fixture {
			types: TypeStore::new(),
		};
		let player = fx.path(PathRoot::Param(0), &[]);
		let score = fx.path(PathRoot::Param(0), &["score"]);
		let set = normalized_set(&mut fx.types, &[score, player, score], &[]);
		assert_eq!(fx.types.effects(set).0, &[player]);
		assert_eq!(normalized_set(&mut fx.types, &[], &[]), EffectSetId::EMPTY);
		let both = normalized_set(&mut fx.types, &[score], &[score]);
		assert_eq!(fx.types.effects(both), (&[][..], &[score][..]));
		let text = path_text(&fx.types, score, &|_| Some("player".to_string()));
		assert_eq!(text, "player.score");
	}
}
