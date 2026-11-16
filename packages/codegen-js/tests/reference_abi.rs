use fa_analyzer::Severity;
use fa_codegen_js::{Options, Target, generate};

#[test]
fn fixed_result_keys_select_container_entries() {
	let source =
		include_str!("../../analyzer/tests/cases/references/review2-static-locator-keys.fa");
	let (mut session, file) = fa_analyzer::analyze_source(source);
	let errors: Vec<_> = session
		.diagnostics(file)
		.iter()
		.filter(|diagnostic| diagnostic.severity == Severity::Error)
		.collect();
	assert!(errors.is_empty(), "{errors:?}");
	let output = generate(&mut session, file, &Options::new(Target::Bun, "main.fa")).unwrap();
	for entry in ["fixedIndex$index", "fixedEntry$entry", "fixedWide$entry"] {
		let body = output
			.program
			.split(&format!("function {entry}("))
			.nth(1)
			.unwrap_or_else(|| panic!("missing {entry}: {}", output.program));
		let body = body.split("\n}").next().unwrap();
		assert!(!body.contains("faReferenceResult("), "{entry}: {body}");
	}
	assert!(!output.program.contains("function negativeIndex$index("));
}
