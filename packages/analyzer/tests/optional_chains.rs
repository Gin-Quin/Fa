//! Absence introduced by a chain is distinct from a nullable result.
use std::sync::Arc;

use fa_analyzer::{Session, Severity};

fn assert_errors(source: &str, codes: &[&str]) {
	for target in [
		"bun",
		"node",
		"web",
		"native",
		"wasm",
		"native-freestanding",
	] {
		let mut session = Session::new();
		session.set_target(target);
		let file = session.set_source("main", Arc::from(source));
		session.analyze_file(file);
		let diagnostics = session.diagnostics(file);
		let errors: Vec<_> = diagnostics
			.iter()
			.filter(|d| d.severity == Severity::Error)
			.collect();
		assert_eq!(
			errors.iter().map(|d| d.code.id).collect::<Vec<_>>(),
			codes,
			"{target}: {errors:#?}"
		);
	}
}

const BOX: &str = r#"
type Callback = () => String
type Box = {
	child: { name: String }?
	callback: Callback?
	function maybe = (self): String? => none
	function factory = (self): Callback? => none
	async function describe = (self): String => "Ada"
	async function maybeAsync = (self): String? => none
}
"#;

#[test]
fn nullable_intermediates_require_their_own_guard() {
	for body in [
		"box?.maybe().trim()",
		"box?.child.name",
		"box?.factory()()",
		"box?.callback()",
	] {
		let code = if body.ends_with("()") && !body.contains("trim") {
			"E-TYPE-1"
		} else {
			"E-OPT-1"
		};
		assert_errors(
			&format!("{BOX}\nfunction bad = (box: Box?) => {body}"),
			&[code],
		);
	}
	assert_errors(
		&format!("{BOX}\nfunction good = (box: Box?) => box?.maybe()?.trim()"),
		&[],
	);
}

#[test]
fn optional_start_requires_handling_absence_before_await_or_assignment() {
	for body in [
		"let task: Task<String?> = start box?.describe()",
		"let task = start box?.describe()\nconsole.log(await task)",
		"console.log(await start box?.describe())",
	] {
		assert_errors(
			&format!("{BOX}\nasync function bad = (box: Box?) => {{\n{body}\n}}"),
			&[if body.starts_with("let task:") {
				"E-OPT-1"
			} else {
				"CONC015"
			}],
		);
	}
	assert_errors(
		&format!(
			r#"{BOX}
async function good = (box: Box?) => {{
	let task: Task<String>? = start box?.describe()
	if task is not none {{ console.log(await task) }}
	console.log(await box?.describe())
	start box?.describe()
}}"#
		),
		&[],
	);
}

#[test]
fn generic_awaited_and_reactive_nullable_results_keep_their_absence() {
	assert_errors(
		"function bad = <Item: { maybe: (self) => String? }>(box: Item?) => box?.maybe().trim()",
		&["E-OPT-1"],
	);
	assert_errors(
		&format!("{BOX}\nasync function bad = (box: Box?) => await box?.maybeAsync().trim()"),
		&["E-OPT-1"],
	);
	assert_errors(
		&format!("{BOX}\nreactive box: Box? = none\nderived text = box?.maybe().trim()"),
		&["E-OPT-1"],
	);
}
