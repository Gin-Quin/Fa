//! Keep the analyzer cases tied to the actual examples on the three pages.
//! Context and expected diagnostic codes live in the cases next to the code.
#[test]
fn every_string_and_bytes_example_is_an_analyzer_case() {
	for (name, page) in [
		(
			"strings",
			include_str!("../../../documentation/values/strings.md"),
		),
		(
			"operations",
			include_str!("../../../documentation/operations/string.md"),
		),
		(
			"bytes",
			include_str!("../../../documentation/values/bytes.md"),
		),
	] {
		for (index, block) in page.split("```fa\n").skip(1).enumerate() {
			let path = format!(
				"{}/tests/cases/values/07-strings-and-characters/spec-examples/{name}-{:02}.fa",
				env!("CARGO_MANIFEST_DIR"),
				index + 1
			);
			let cases = std::fs::read_to_string(path).unwrap();
			let code = block.split("```").next().unwrap();
			let code = code
				.replace("-- error: `word`", "-- error: E-DECL-16 (`word`")
				.replace("-- error: expected", "-- error: E-STR-7 (expected");
			assert!(
				cases.contains(&code),
				"missing documentation example:\n{code}"
			);
		}
	}
}
