//! Constants handed to the backends retain malformed strings exactly.

use fa_analyzer::ids::TypeId;
use fa_analyzer::session::analyze_source;
use fa_interpreter::Value;

#[test]
fn malformed_constants_and_bytes_survive_analysis_and_interfaces() {
	let (session, file) = analyze_source(
		"static high = \"😀\"[0]\nstatic low = \"😀\"[1]\nstatic joined = high + low\nstatic binary = \"é日😀\".toUtf8()\nstatic record = { half = high, bytes = binary }\n",
	);
	let errors: Vec<_> = session
		.diagnostics(file)
		.iter()
		.filter(|d| d.severity == fa_analyzer::Severity::Error)
		.collect();
	assert!(errors.is_empty(), "{errors:?}");
	let analysis = session.analysis(file);
	let find = |name: &str| {
		let (symbol, value) = analysis
			.static_values
			.iter()
			.find(|(symbol, _)| {
				session
					.types
					.syms
					.resolve(analysis.symbols.symbol(**symbol).name)
					== name
			})
			.unwrap();
		(*symbol, value)
	};
	let (high_symbol, high) = find("high");
	let Value::String(high) = high else {
		panic!("a string constant");
	};
	assert_eq!(high.units().collect::<Vec<_>>(), [0xd83d]);
	assert!(high.as_str().is_none());
	assert_eq!(analysis.symbols.symbol(high_symbol).ty, TypeId::STRING);
	let (_, joined) = find("joined");
	assert_eq!(joined.as_str(), Some("😀"));
	let (_, binary) = find("binary");
	assert!(matches!(binary,Value::Bytes(bytes) if bytes.as_slice() == "é日😀".as_bytes()));
	let (_, record) = find("record");
	let Value::Object(record) = record else {
		panic!("an object constant");
	};
	assert_eq!(record.get("half"), Some(&Value::String(high.clone())));
	assert_eq!(record.get("bytes"), Some(binary));
}
