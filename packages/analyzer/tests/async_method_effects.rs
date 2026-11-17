//! Blocking capability follows the methods actually called in a generic body.

use std::sync::Arc;

use fa_analyzer::diagnostics::Severity;
use fa_analyzer::pipeline::{self, NoSink};
use fa_analyzer::{CodebaseResolver, Session};

const TYPES: &str = r#"
type Slow<label: String> = {
	name: String
	async function describe = (self): String => {
		await std.time.sleep(1)
		"{label}: {self.name}"
	}
}
type Fast = {
	name: String
	async function describe = (self): String => self.name
}
"#;

fn assert_target(source: &str, target: &str, blocked: bool) {
	let mut session = Session::new();
	session.set_target(target);
	let file = session.set_source("main", Arc::from(source));
	session.analyze_file(file);
	let diagnostics = session.diagnostics(file);
	let errors: Vec<_> = diagnostics
		.iter()
		.filter(|d| d.severity == Severity::Error)
		.collect();
	if blocked {
		assert_eq!(errors.len(), 1, "{target}: {errors:#?}");
		assert_eq!(errors[0].code.id, "CONC010", "{target}: {errors:#?}");
		assert!(errors[0].message.contains("sleep"), "{errors:#?}");
		assert!(errors[0].message.contains("await"), "{errors:#?}");
	} else {
		assert!(errors.is_empty(), "{target}: {errors:#?}");
	}
}

#[test]
fn blocking_requirements_follow_generic_forwarding_and_explicit_applications() {
	let declarations = r#"
async function inner = <Item: { describe: async (self) => String }>(item: Item): String => await item.describe()
async function outer = <Renamed: { describe: async (self) => String }>(item: Renamed): String => await inner(item)
let value = Slow<"async"> { name = "Ada" }
"#;
	for call in ["outer(value)", "outer<Slow<\"async\">>(value)"] {
		for awaited in [false, true] {
			let source = format!(
				"{TYPES}\n{declarations}\nconsole.log({}{call})",
				if awaited { "await " } else { "" }
			);
			for target in ["bun", "node", "web", "native", "wasm"] {
				assert_target(
					&source,
					target,
					!awaited && matches!(target, "web" | "wasm"),
				);
			}
		}
	}
}

#[test]
fn unused_constraints_and_other_generic_parameters_do_not_restrict_calls() {
	let source = format!(
		r#"{TYPES}
async function first = <Left: {{ describe: async (self) => String }}, Right: {{ describe: async (self) => String }}>(left: Left, right: Right): String => await left.describe()
async function unused = <Item: {{ describe: async (self) => String }}>(item: Item): Integer => 1
let fast = Fast {{ name = "fast" }}
let slow = Slow<"slow"> {{ name = "Ada" }}
console.log(first(fast, slow), unused(slow))
"#
	);
	for target in ["bun", "node", "web", "native", "wasm"] {
		assert_target(&source, target, false);
	}
	assert_target(
		&source.replace("first(fast, slow)", "first(slow, fast)"),
		"web",
		true,
	);
}

#[test]
fn selected_generic_owner_methods_contribute_their_own_constraints() {
	let source = format!(
		r#"{TYPES}
type Wrapped<Item: {{ describe: async (self) => String }}> = {{
	value: Item
	async function describe = (self): String => await self.value.describe()
}}
async function invoke = <Item: {{ describe: async (self) => String }}>(item: Item): String => await item.describe()
let wrapped = Wrapped<Slow<"nested">> {{ value = Slow {{ name = "Ada" }} }}
console.log(invoke(wrapped))
"#
	);
	assert_target(&source, "web", true);
	assert_target(
		&source.replace(
			"console.log(invoke(wrapped))",
			"console.log(await invoke(wrapped))",
		),
		"web",
		false,
	);
}

#[test]
fn recursive_generic_forwarding_reaches_a_finite_effect_fixpoint() {
	let source = format!(
		r#"{TYPES}
async function repeat = <Item: {{ describe: async (self) => String }}>(item: Item, count: Integer): String => {{
	if count == 0 {{ return await item.describe() }}
	return await repeat(item, count - 1)
}}
console.log(repeat(Slow<"recursive"> {{ name = "Ada" }}, 2))
"#
	);
	assert_target(&source, "web", true);
	assert_target(
		&source.replace("console.log(repeat(", "console.log(await repeat("),
		"web",
		false,
	);
}

#[test]
fn imported_requirements_survive_interface_merging_and_target_changes() {
	let files = vec![
		("package.fa".to_string(), "export = { name = \"asyncMethods\" }".to_string()),
		("library/inner.fa".to_string(), "export async function = <Item: { describe: async (self) => String }>(item: Item): String => await item.describe()".to_string()),
		("library/outer.fa".to_string(), "export async function = <Renamed: { describe: async (self) => String }>(item: Renamed): String => await inner(item)".to_string()),
		("application/main.fa".to_string(), format!("{TYPES}\nexport function = () => console.log(library.outer(Slow<\"imported\"> {{ name = \"Ada\" }}))")),
	];
	let mut session = Session::new();
	let mut previous = None;
	for target in ["bun", "web", "node", "web"] {
		if previous.is_some() {
			session = Session::sharing_types_of(&session);
		}
		session.set_target(target);
		session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
		let options = pipeline::Options {
			entry: Some("application/main".to_string()),
			threads: Some(2),
			retarget: previous.is_some(),
			..Default::default()
		};
		let report = pipeline::analyze_package(&mut session, &options, previous.as_ref(), &NoSink);
		let errors: Vec<_> = report
			.modules
			.iter()
			.flat_map(|module| &module.diagnostics)
			.filter(|d| d.severity == Severity::Error)
			.collect();
		if target == "web" {
			assert_eq!(errors.len(), 1, "{errors:#?}");
			assert_eq!(errors[0].code.id, "CONC010", "{errors:#?}");
			assert!(errors[0].message.contains("sleep"), "{errors:#?}");
		} else {
			assert!(errors.is_empty(), "{errors:#?}");
		}
		assert!(
			report
				.modules
				.iter()
				.any(|module| module.module == "application/main"
					&& module.analyzed
					&& module.reads_target)
		);
		let interface = report.interface(&session, "library/outer").unwrap();
		assert!(
			interface
				.async_summaries
				.values()
				.any(|summary| !summary.methods.is_empty())
		);
		let before = interface.hash(session.types());
		let mut changed = interface.clone();
		for summary in changed.async_summaries.values_mut() {
			summary.methods.clear();
		}
		assert_ne!(
			before,
			changed.hash(session.types()),
			"conditional methods participate in interface invalidation"
		);
		previous = Some(report);
	}
}
