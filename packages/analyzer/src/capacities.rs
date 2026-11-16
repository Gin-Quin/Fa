//! Closed permission to change unobserved Array reservation sizes.
//!
//! Deliberately program-wide: a capacity read through a widened type, alias,
//! pattern or function value removes the permission for every array. Opaque
//! generic/key-loop/foreign code and missing graph contributions reject it.

use std::collections::BTreeSet;

use crate::closed_world;
use crate::ids::FileId;
use crate::session::Session;
use crate::symbols::Resolution;

pub const UNOBSERVED: &str = "unobserved";
const REJECT: &str = "reject";

pub fn unit_claims(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let mut claims = closed_world::claims(session, files);
	for &file in files {
		let analysis = session.analysis(file);
		if !closed_world::checked(session, file)
			|| !analysis.extern_calls.is_empty()
			|| !session.source_file(file).interface.externs.is_empty()
			|| !analysis.key_loop_bodies.is_empty()
			|| analysis.functions.values().any(|info| !info.generics.is_empty())
			|| analysis.resolutions.iter().any(|resolution| matches!(resolution, Resolution::Builtin(id) if crate::builtins::get(*id).path == "Array#capacity"))
		{
			claims.insert(REJECT.to_string());
		}
	}
	claims
}

pub fn resolve_claims(claims: BTreeSet<String>) -> BTreeSet<String> {
	if claims.contains(REJECT) || !closed_world::complete(&claims) {
		return BTreeSet::new();
	}
	BTreeSet::from([UNOBSERVED.to_string()])
}

#[cfg(test)]
mod tests {
	use super::*;

	fn claims(source: &str) -> BTreeSet<String> {
		let mut session = Session::new();
		let file = session.set_source("main", source.into());
		session.analyze_file(file);
		assert!(
			closed_world::checked(&session, file),
			"{:?}",
			session.diagnostics(file)
		);
		unit_claims(&session, &[file])
	}

	#[test]
	fn every_array_capacity_observer_removes_the_permission() {
		for (suffix, unobserved) in [
			("console.log(items.length)", true),
			("console.log(items.capacity)", false),
			("let alias = items\nconsole.log(alias.capacity)", false),
			(
				"function read = (values: Array<Integer>): Integer => values.capacity\nlet getter = read\nconsole.log(getter(items))",
				false,
			),
			(
				"let object = { values = items }\nlet { values } = object\nconsole.log(values.capacity)",
				false,
			),
			(
				"function read = <T>(values: Array<T>): Integer => values.length\nconsole.log(read(items))",
				false,
			),
		] {
			let claims = claims(&format!("let items: Array<Integer> = [1, 2]\n{suffix}\n"));
			assert_eq!(
				resolve_claims(claims.clone()).contains(UNOBSERVED),
				unobserved,
				"{claims:?}: {suffix}"
			);
		}
	}

	#[test]
	fn missing_contributions_never_grant_permission() {
		let claims = claims("console.log(1)");
		assert!(resolve_claims(claims.clone()).contains(UNOBSERVED));
		for prefix in ["module\u{1f}", "manifest\u{1f}", "requires\u{1f}"] {
			let incomplete = claims
				.iter()
				.filter(|claim| !claim.starts_with(prefix))
				.cloned()
				.collect();
			assert!(resolve_claims(incomplete).is_empty(), "{prefix}");
		}
		assert!(resolve_claims(BTreeSet::new()).is_empty());
	}

	#[test]
	fn remote_observers_and_missing_units_reject_the_same_fact() {
		use crate::pipeline::{self, UnitSink};
		use std::sync::Mutex;

		struct Claims(Mutex<Vec<BTreeSet<String>>>);
		impl UnitSink for Claims {
			fn unit(&self, session: &mut Session, files: &[FileId]) {
				self.0.lock().unwrap().push(unit_claims(session, files));
			}
		}
		for property in ["length", "capacity"] {
			let files: Vec<(String, String)> = vec![
				(
					"package.fa".into(),
					"export = { name = \"capacityProof\" }".into(),
				),
				(
					"library/read.fa".into(),
					format!(
						"export function = (items: Array<Integer>): Integer => items.{property}"
					),
				),
				(
					"application/main.fa".into(),
					"export function = () => console.log(library.read([1, 2]))".into(),
				),
			];
			let mut session = Session::new();
			session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
			let program = session.analyze_package(Some("application/main"));
			assert!(
				program
					.files
					.iter()
					.all(|&file| closed_world::checked(&session, file))
			);
			let expected = resolve_claims(unit_claims(&session, &program.files));
			assert_eq!(expected.contains(UNOBSERVED), property == "length");
			let mut parallel = Session::new();
			parallel.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
			let sink = Claims(Mutex::new(Vec::new()));
			let report = pipeline::analyze_package(
				&mut parallel,
				&pipeline::Options {
					entry: Some("application/main".into()),
					threads: Some(2),
					..Default::default()
				},
				None,
				&sink,
			);
			assert!(!report.has_errors(), "{:?}", report.modules);
			let contributions = sink.0.into_inner().unwrap();
			assert_eq!(
				resolve_claims(contributions.iter().flatten().cloned().collect()),
				expected
			);
			for missing in 0..contributions.len() {
				let incomplete = contributions
					.iter()
					.enumerate()
					.filter(|(index, _)| *index != missing)
					.flat_map(|(_, claims)| claims.iter().cloned())
					.collect();
				assert!(resolve_claims(incomplete).is_empty());
			}
		}
	}
}
