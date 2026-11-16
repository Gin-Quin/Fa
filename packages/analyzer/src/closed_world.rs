//! The exact module manifest shared by closed whole-program proofs.

use std::collections::BTreeSet;

use crate::ids::FileId;
use crate::session::Session;

pub(crate) const SEPARATOR: char = '\u{1f}';
const MODULE_SEPARATOR: char = '\u{1e}';

pub(crate) fn claims(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let mut claims = BTreeSet::new();
	let modules = session.region_modules();
	let Some(first) = modules.first() else {
		return claims;
	};
	let leader = format!("{first:?}");
	claims.insert(format!("requires{SEPARATOR}{leader}"));
	for &file in files {
		let module = session.module(file);
		claims.insert(format!("module{SEPARATOR}{module:?}"));
		if module == first {
			let modules: Vec<_> = modules.iter().map(|module| format!("{module:?}")).collect();
			claims.insert(format!(
				"manifest{SEPARATOR}{leader}{SEPARATOR}{}",
				modules.join(&MODULE_SEPARATOR.to_string())
			));
		}
	}
	claims
}

pub(crate) fn complete(claims: &BTreeSet<String>) -> bool {
	let mut requires = BTreeSet::new();
	let mut manifests = BTreeSet::new();
	let mut modules = BTreeSet::new();
	for claim in claims {
		let parts: Vec<_> = claim.split(SEPARATOR).collect();
		match parts.as_slice() {
			["requires", leader] => {
				requires.insert(*leader);
			}
			["module", module] => {
				modules.insert(*module);
			}
			["manifest", leader, expected] => {
				manifests.insert((*leader, *expected));
			}
			_ => {}
		}
	}
	if requires.len() != 1 || manifests.len() != 1 {
		return false;
	}
	let (leader, expected) = manifests.first().expect("one manifest");
	let expected: BTreeSet<_> = expected.split(MODULE_SEPARATOR).collect();
	requires.contains(leader) && expected == modules && modules.contains(leader)
}

pub(crate) fn checked(session: &Session, file: FileId) -> bool {
	let source = session.source_file(file);
	source.done
		&& !source.deferred
		&& !session
			.diagnostics(file)
			.iter()
			.any(|diagnostic| diagnostic.severity == crate::Severity::Error)
}
