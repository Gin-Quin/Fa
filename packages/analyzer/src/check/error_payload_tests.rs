use std::sync::Arc;

use super::test_support::{analyzed, codes};
use crate::session::Session;
use crate::types::Ownership;

#[test]
fn a_unique_error_payload_is_rejected_at_its_declaration() {
	for payload in [
		"Task<Integer>",
		"{ task: Task<Integer> }",
		"Array<Task<Integer>>",
		"(String, Task<Integer>?)",
	] {
		let source = format!("type Failure = Errors {{ Bad = {payload} }}\n");
		let (session, file) = crate::analyze_source(&source);
		let diagnostics: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|d| d.code.id == "MEM005")
			.collect();
		assert_eq!(
			diagnostics.len(),
			1,
			"{source}: {:?}",
			session.diagnostics(file)
		);
		let diagnostic = diagnostics[0];
		assert!(diagnostic.message.contains("owns a unique value"));
		assert!(diagnostic.message.contains("identifier or a description"));
		let span = diagnostic.location.span;
		assert_eq!(&source[span.start as usize..span.end as usize], payload);
	}
}

#[test]
fn error_payloads_allow_ordinary_data_collections_and_shared_handles() {
	let source = "use { Mutex, Channel, AtomicInteger, AtomicBoolean } = std.concurrency\n\
		type Failure = Errors {\n\
			Empty\n\
			Text = { message: String }\n\
			Items = { items: Array<Integer> }\n\
			Protected = { mutex: Mutex<Task<Integer>>, channel: Channel<Integer>, count: AtomicInteger, ready: AtomicBoolean }\n\
		}\n";
	analyzed(source);
}

#[test]
fn recursive_resources_are_rejected_after_their_targets_resolve() {
	let source = "type Held = { failure: Failure?, function destroy = (self) => {} }\n\
		type Failure = Errors { Bad = { owner: Held } }\n";
	let mut session = Session::new();
	let file = session.set_source("payload.foreign", Arc::from(source));
	session.analyze_file(file);
	let found: Vec<_> = session
		.diagnostics(file)
		.iter()
		.map(|d| d.code.id)
		.collect();
	assert_eq!(found, ["MEM005"]);
}

#[test]
fn known_unique_generic_payloads_are_rejected_and_dependent_ones_are_deferred() {
	assert_eq!(
		codes("type Failure<Item> = Errors { Bad = { task: Task<Item> } }\n"),
		["MEM005"]
	);
	assert!(codes("type Failure<Item> = Errors { Bad = { item: Item } }\n").is_empty());
}

#[test]
fn every_standard_library_error_payload_is_data() {
	let mut types = crate::types::TypeStore::new();
	for error in crate::builtins::errors::ERRORS {
		let ty = crate::builtins::errors::error_type(&mut types, error.name).unwrap();
		assert!(!types.is_unique(ty), "{}", error.name);
		assert_ne!(types.ownership(ty), Ownership::Dependent, "{}", error.name);
	}
}
