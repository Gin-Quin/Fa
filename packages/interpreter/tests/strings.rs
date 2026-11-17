use std::sync::Arc;

use fa_interpreter::ir::{Expr, FnKey, Function, GlobalKey, Item, Part, Place, Step, TypeTest};
use fa_interpreter::text::Text;
use fa_interpreter::{Error, Host, Interpreter, Limits, Value};

struct NoHost;
impl Host for NoHost {
	fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
		unreachable!()
	}
	fn global(&mut self, _: GlobalKey) -> Result<Value, Error> {
		unreachable!()
	}
}

fn run(expr: Expr, slots: u32) -> Result<Value, Error> {
	Interpreter::new(&mut NoHost, Limits::default()).evaluate(&expr, slots)
}
fn value(v: Value) -> Expr {
	Expr::Const(v)
}
fn string(s: &str) -> Value {
	Value::string(s)
}
fn units(u: &[u16]) -> Value {
	Value::String(Text::from_units(u))
}
fn bytes(b: &[u8]) -> Value {
	Value::Bytes(Arc::new(b.to_vec()))
}
fn call(path: &'static str, args: Vec<Value>) -> Result<Value, Error> {
	run(
		Expr::Builtin(path, args.into_iter().map(value).collect()),
		0,
	)
}
fn slice(s: Value, from: Option<i64>, to: Option<i64>, inclusive: bool) -> Result<Value, Error> {
	let range = Value::Range {
		start: from.map(Into::into),
		end: to.map(Into::into),
		inclusive,
	};
	run(Expr::Index(value(s).boxed(), value(range).boxed()), 0)
}

#[test]
fn lengths_units_and_encodings_are_distinct() {
	for (s, length, utf8) in [
		("", 0, 0),
		("a\r\nb", 4, 4),
		("héllo", 5, 6),
		("日本語", 3, 9),
		("😀", 2, 4),
		("🇫🇷", 4, 8),
		("e\u{301}", 2, 3),
	] {
		for path in ["String#length", "String#capacity"] {
			assert_eq!(call(path, vec![string(s)]), Ok(Value::int(length)));
		}
		assert_eq!(
			call("String#utf8Length", vec![string(s)]),
			Ok(Value::int(utf8))
		);
		assert_eq!(
			call("String#isWellFormed", vec![string(s)]),
			Ok(Value::Boolean(true))
		);
		assert_eq!(
			call("String#isEmpty", vec![string(s)]),
			Ok(Value::Boolean(s.is_empty()))
		);
	}
}

#[test]
fn indexing_slicing_and_optional_access_preserve_lone_surrogates() {
	let text = string("é日😀");
	for (index, unit) in [
		(0, 0xe9),
		(1, 0x65e5),
		(2, 0xd83d),
		(3, 0xde00),
		(-1, 0xde00),
	] {
		assert_eq!(
			run(
				Expr::Index(
					value(text.clone()).boxed(),
					value(Value::int(index)).boxed()
				),
				0
			),
			Ok(units(&[unit]))
		);
		assert_eq!(
			call("String#get", vec![text.clone(), Value::int(index)]),
			Ok(units(&[unit]))
		);
		assert_eq!(
			call("String#codeUnitAt", vec![text.clone(), Value::int(index)]),
			Ok(Value::int(i64::from(unit)))
		);
	}
	assert_eq!(
		call("String#get", vec![text.clone(), Value::int(4)]),
		Ok(Value::None)
	);
	assert!(matches!(
		call("String#codeUnitAt", vec![text.clone(), Value::int(-5)]),
		Err(Error::Thrown(_))
	));
	assert_eq!(
		slice(text.clone(), Some(2), Some(3), false),
		Ok(units(&[0xd83d]))
	);
	assert_eq!(
		slice(text.clone(), Some(-1), None, false),
		Ok(units(&[0xde00]))
	);
	assert_eq!(slice(text.clone(), None, Some(-1), true), Ok(text.clone()));
	assert_eq!(slice(text.clone(), Some(3), Some(1), false), Ok(string("")));
	assert!(matches!(
		slice(text, None, Some(4), true),
		Err(Error::Thrown(_))
	));
}

#[test]
fn concatenation_templates_and_to_string_join_pairs() {
	let high = units(&[0xd83d]);
	let low = units(&[0xde00]);
	assert_eq!(
		call("String#isWellFormed", vec![high.clone()]),
		Ok(Value::Boolean(false))
	);
	assert_eq!(
		call("String#toString", vec![high.clone()]),
		Ok(high.clone())
	);
	let expr = Expr::Template(vec![
		Part::Value(value(high.clone())),
		Part::Value(value(low.clone())),
	]);
	assert_eq!(run(expr, 0), Ok(string("😀")));
	let place = Place {
		slot: 0,
		steps: vec![],
	};
	let expr = Expr::Block(vec![
		Expr::Let(0, value(high.clone()).boxed()),
		Expr::Assign(
			place.clone(),
			Expr::Template(vec![
				Part::Value(Expr::Read(place)),
				Part::Value(value(low.clone())),
			])
			.boxed(),
		),
		Expr::Local(0),
	]);
	assert_eq!(run(expr, 1), Ok(string("😀")));
	assert_eq!(
		call(
			"Array#join",
			vec![Value::array(vec![high, low]), string("")]
		),
		Ok(string("😀"))
	);
	assert_eq!(
		call("String.from", vec![units(&[0xd800]), Value::int(4096)]),
		Ok(units(&[0xd800]))
	);
}

#[test]
fn ordering_is_by_code_unit_and_equality_does_not_normalize() {
	use fa_interpreter::ir::CompareOp;
	for (a, b) in [
		(string("😀"), string("\u{e000}")),
		(units(&[0xd800]), units(&[0xdc00])),
		(string("Z"), string("a")),
	] {
		assert_eq!(
			run(
				Expr::Compare(CompareOp::Less, value(a).boxed(), value(b).boxed()),
				0
			),
			Ok(Value::Boolean(true))
		);
	}
	assert_ne!(string("é"), string("e\u{301}"));
	assert_eq!(Text::from_units(&[0xd83d, 0xde00]).as_str(), Some("😀"));
	assert!(Text::from_units(&[0xd83d]).as_str().is_none());
}

#[test]
fn searches_match_units_and_report_unit_positions() {
	let text = string("é日😀e\u{301}😀");
	for (needle, first, last) in [
		(string("日"), 1, 1),
		(string("😀"), 2, 6),
		(string("e"), 4, 4),
		(units(&[0xd83d]), 2, 6),
		(units(&[0xde00]), 3, 7),
		(string(""), 0, 8),
	] {
		assert_eq!(
			call("String#includes", vec![text.clone(), needle.clone()]),
			Ok(Value::Boolean(true))
		);
		assert_eq!(
			call("String#indexOf", vec![text.clone(), needle.clone()]),
			Ok(Value::int(first))
		);
		assert_eq!(
			call("String#lastIndexOf", vec![text.clone(), needle]),
			Ok(Value::int(last))
		);
	}
	assert_eq!(
		call("String#indexOf", vec![text, string("x")]),
		Ok(Value::None)
	);
	assert_eq!(
		call("String#startsWith", vec![string("😀é"), units(&[0xd83d])]),
		Ok(Value::Boolean(true))
	);
	assert_eq!(
		call("String#endsWith", vec![string("é😀"), units(&[0xde00])]),
		Ok(Value::Boolean(true))
	);
}

#[test]
fn split_handles_empty_separators_and_half_pair_matches() {
	assert_eq!(
		call("String#split", vec![string("é日😀"), string("")]),
		Ok(Value::array(vec![
			string("é"),
			string("日"),
			units(&[0xd83d]),
			units(&[0xde00])
		]))
	);
	assert_eq!(
		call("String#split", vec![string(""), string("")]),
		Ok(Value::array(vec![]))
	);
	assert_eq!(
		call("String#split", vec![string(""), string(",")]),
		Ok(Value::array(vec![string("")]))
	);
	assert_eq!(
		call("String#split", vec![string("é😀日😀"), units(&[0xde00])]),
		Ok(Value::array(vec![
			units(&[0xe9, 0xd83d]),
			units(&[0x65e5, 0xd83d]),
			string("")
		]))
	);
}

#[test]
fn replacements_are_literal_and_match_units_including_empty_matches() {
	assert_eq!(
		call(
			"String#replace",
			vec![string("a-b"), string("-"), string("$&")]
		),
		Ok(string("a$&b"))
	);
	assert_eq!(
		call(
			"String#replaceAll",
			vec![string("é-日-😀"), string("-"), string("$$$&$`$'")]
		),
		Ok(string("é$$$&$`$'日$$$&$`$'😀"))
	);
	assert_eq!(
		call(
			"String#replace",
			vec![string("é😀日😀"), units(&[0xd83d]), string("x")]
		),
		Ok(units(&[0xe9, 120, 0xde00, 0x65e5, 0xd83d, 0xde00]))
	);
	assert_eq!(
		call(
			"String#replaceAll",
			vec![string("😀"), string(""), string("x")]
		),
		Ok(units(&[120, 0xd83d, 120, 0xde00, 120]))
	);
	assert_eq!(
		call("String#replace", vec![string(""), string(""), string("é")]),
		Ok(string("é"))
	);
	assert_eq!(
		call(
			"String#replaceAll",
			vec![string("aaa"), string("aa"), string("b")]
		),
		Ok(string("ba"))
	);
}

#[test]
fn padding_repeat_case_and_trim_preserve_units() {
	assert_eq!(
		call(
			"String#padEnd",
			vec![string("é"), Value::int(2), string("😀")]
		),
		Ok(units(&[0xe9, 0xd83d]))
	);
	assert_eq!(
		call(
			"String#padStart",
			vec![string("日"), Value::int(4), string("😀")]
		),
		Ok(units(&[0xd83d, 0xde00, 0xd83d, 0x65e5]))
	);
	assert_eq!(
		call(
			"String#padEnd",
			vec![string("😀"), Value::int(7), string("")]
		),
		Ok(string("😀"))
	);
	assert_eq!(
		call(
			"String#repeat",
			vec![units(&[0xdc00, 0xd800]), Value::int(2)]
		),
		Ok(units(&[0xdc00, 0xd800, 0xdc00, 0xd800]))
	);
	assert_eq!(
		call("String#toUpperCase", vec![units(&[0xdf, 0xd800, 0x65e5])]),
		Ok(units(&[83, 83, 0xd800, 0x65e5]))
	);
	assert_eq!(
		call("String#toLowerCase", vec![string("É日😀ΟΣ")]),
		Ok(string("é日😀ος"))
	);
	for (path, expected) in [
		("String#trim", "é日😀"),
		("String#trimStart", "é日😀 \u{feff}"),
		("String#trimEnd", "\u{feff} é日😀"),
	] {
		assert_eq!(
			call(path, vec![string("\u{feff} é日😀 \u{feff}")]),
			Ok(string(expected))
		);
	}
	assert_eq!(
		call("String#trim", vec![string("\u{85}x\u{85}")]),
		Ok(string("\u{85}x\u{85}"))
	);
}

#[test]
fn code_points_and_graphemes_are_explicit_views() {
	assert_eq!(
		call("String#codePoints", vec![string("é日😀")]),
		Ok(Value::array(vec![
			Value::int(233),
			Value::int(26085),
			Value::int(128512)
		]))
	);
	assert_eq!(
		call("String#codePoints", vec![units(&[0xd800, 0x301])]),
		Ok(Value::array(vec![Value::int(0xd800), Value::int(0x301)]))
	);
	assert_eq!(
		call("String#characters", vec![string("e\u{301}🇫🇷👩🏽‍💻")]),
		Ok(Value::array(vec![
			string("e\u{301}"),
			string("🇫🇷"),
			string("👩🏽‍💻")
		]))
	);
	assert_eq!(
		call("String#characters", vec![units(&[0xd800, 0x301, 97])]),
		Ok(Value::array(vec![units(&[0xd800, 0x301]), string("a")]))
	);
}

#[test]
fn string_loops_use_code_points_and_code_unit_indices() {
	let out = Place {
		slot: 0,
		steps: vec![],
	};
	let input = units(&[0xe9, 0x65e5, 0xd83d, 0xde00, 0xd800, 97]);
	let expr = Expr::Block(vec![
		Expr::Let(0, Expr::Array(vec![]).boxed()),
		Expr::For {
			item: Some(1),
			index: Some(2),
			iterable: value(input).boxed(),
			body: Expr::Mutate(
				"Array#push",
				out.clone(),
				vec![Expr::Tuple(vec![Expr::Local(1), Expr::Local(2)])],
			)
			.boxed(),
		},
		Expr::Read(out),
	]);
	let expected = [
		(string("é"), 0),
		(string("日"), 1),
		(string("😀"), 2),
		(units(&[0xd800]), 4),
		(string("a"), 5),
	]
	.into_iter()
	.map(|(s, i)| Value::Tuple(Arc::new(vec![s, Value::int(i)])))
	.collect();
	assert_eq!(run(expr, 3), Ok(Value::array(expected)));
}

#[test]
fn bytes_are_distinct_compact_values_with_independent_copies() {
	let data = bytes(&[1, 2, 3]);
	for (test, expected) in [
		(TypeTest::Bytes, true),
		(TypeTest::Array, false),
		(TypeTest::String, false),
	] {
		assert_eq!(
			run(Expr::Is(value(data.clone()).boxed(), test, false), 0),
			Ok(Value::Boolean(expected))
		);
	}
	assert_ne!(
		data,
		Value::array(vec![Value::int(1), Value::int(2), Value::int(3)])
	);
	assert_eq!(
		call(
			"Bytes.from",
			vec![Value::array(vec![
				Value::int(1),
				Value::int(2),
				Value::int(3)
			])]
		),
		Ok(data.clone())
	);
	assert_eq!(
		call("Bytes.filled", vec![Value::int(3)]),
		Ok(bytes(&[0, 0, 0]))
	);
	assert_eq!(
		call("Bytes.filled", vec![Value::int(2), Value::int(255)]),
		Ok(bytes(&[255, 255]))
	);
	assert_eq!(
		call("Bytes#get", vec![data.clone(), Value::int(-1)]),
		Ok(Value::int(3))
	);
	assert_eq!(
		call("Bytes#get", vec![data.clone(), Value::int(3)]),
		Ok(Value::None)
	);
	assert_eq!(
		call("Bytes#toString", vec![data.clone()]),
		Ok(string("[1, 2, 3]"))
	);
	assert_eq!(
		call("Bytes#toArray", vec![data.clone()]),
		Ok(Value::array(vec![
			Value::int(1),
			Value::int(2),
			Value::int(3)
		]))
	);
	assert_eq!(
		slice(data.clone(), Some(1), None, false),
		Ok(bytes(&[2, 3]))
	);
	let place = Place {
		slot: 1,
		steps: vec![
			Step::Index(value(Value::Range {
				start: Some(1.into()),
				end: Some(3.into()),
				inclusive: false,
			})),
			Step::Index(value(Value::int(-1))),
		],
	};
	let expr = Expr::Block(vec![
		Expr::Let(0, value(data.clone()).boxed()),
		Expr::Let(1, Expr::Local(0).boxed()),
		Expr::Assign(place, value(Value::int(255)).boxed()),
		Expr::Tuple(vec![Expr::Local(0), Expr::Local(1)]),
	]);
	assert_eq!(
		run(expr, 2),
		Ok(Value::Tuple(Arc::new(vec![data, bytes(&[1, 2, 255])])))
	);
}

#[test]
fn utf8_decoding_reports_positions_and_keeps_boms() {
	for (input, position, lossy) in [
		(vec![255, 97], 0, "�a"),
		(vec![97, 255, 98], 1, "a�b"),
		(vec![97, 0xf0, 0x9f], 1, "a�"),
		(vec![0xed, 0xa0, 0x80], 0, "���"),
		(vec![0xe2, 0x28, 0xa1], 0, "�(�"),
	] {
		assert_eq!(
			call("String.fromUtf8", vec![bytes(&input)]),
			Err(Error::Thrown(Value::error(
				"Utf8Error",
				vec![("position", Value::int(position))]
			)))
		);
		assert_eq!(
			call("String.fromUtf8Lossy", vec![bytes(&input)]),
			Ok(string(lossy))
		);
	}
	for s in ["", "é日😀", "\u{feff}a"] {
		let encoded = call("String#toUtf8", vec![string(s)]).unwrap();
		assert_eq!(encoded, bytes(s.as_bytes()));
		assert_eq!(call("String.fromUtf8", vec![encoded]), Ok(string(s)));
	}
	assert_eq!(
		call(
			"String#toUtf8",
			vec![units(&[0xd800, 0xd800, 0xdc00, 0xdc00])]
		),
		Ok(bytes("�𐀀�".as_bytes()))
	);
}

#[test]
fn allocation_limits_cover_strings_bytes_and_expansions() {
	let limits = Limits {
		memory: 1024,
		..Limits::default()
	};
	for (path, args) in [
		("String#repeat", vec![string("é😀"), Value::int(1_000_000)]),
		(
			"String#padEnd",
			vec![string("日"), Value::int(1_000_000), string("😀")],
		),
		("Bytes.filled", vec![Value::int(1_000_000)]),
		("String#split", vec![string(&"😀".repeat(100)), string("")]),
		(
			"String#replaceAll",
			vec![
				string(&"é".repeat(100)),
				string(""),
				string(&"日".repeat(100)),
			],
		),
	] {
		let expr = Expr::Builtin(path, args.into_iter().map(value).collect());
		assert_eq!(
			Interpreter::new(&mut NoHost, limits).evaluate(&expr, 0),
			Err(Error::Memory(1024)),
			"{path}"
		);
	}
	let empty = Expr::Builtin(
		"String#repeat",
		vec![value(string("")), value(Value::Integer(ethnum::I256::MAX))],
	);
	assert_eq!(
		Interpreter::new(&mut NoHost, limits).evaluate(&empty, 0),
		Ok(string(""))
	);
}

#[test]
fn spreading_strings_walks_points_and_bytes_walks_bytes() {
	let expr = Expr::Array(vec![Item::Spread(value(string("é日😀")))]);
	assert_eq!(
		run(expr, 0),
		Ok(Value::array(vec![string("é"), string("日"), string("😀")]))
	);
	let expr = Expr::Array(vec![Item::Spread(value(bytes(&[195, 169])))]);
	assert_eq!(
		run(expr, 0),
		Ok(Value::array(vec![Value::int(195), Value::int(169)]))
	);
}

#[test]
fn utf8_loops_and_point_counts_do_not_allocate_their_views() {
	let source = units(&[0xe9, 0x65e5, 0xd83d, 0xde00, 0xd800]);
	let out = Place {
		slot: 0,
		steps: vec![],
	};
	let expr = Expr::Block(vec![
		Expr::Let(0, Expr::Array(vec![]).boxed()),
		Expr::ForUtf8 {
			item: Some(1),
			index: Some(2),
			iterable: value(source).boxed(),
			body: Expr::Mutate(
				"Array#push",
				out.clone(),
				vec![Expr::Tuple(vec![Expr::Local(1), Expr::Local(2)])],
			)
			.boxed(),
		},
		Expr::Read(out),
	]);
	let expected = "é日😀�"
		.as_bytes()
		.iter()
		.enumerate()
		.map(|(i, b)| {
			Value::Tuple(Arc::new(vec![
				Value::int(i64::from(*b)),
				Value::int(i as i64),
			]))
		})
		.collect();
	assert_eq!(run(expr, 3), Ok(Value::array(expected)));
	let large = string(&"😀".repeat(10_000));
	let limits = Limits {
		memory: 1024,
		..Limits::default()
	};
	let looped = Expr::ForUtf8 {
		item: None,
		index: None,
		iterable: value(large.clone()).boxed(),
		body: Expr::Break(None).boxed(),
	};
	assert_eq!(
		Interpreter::new(&mut NoHost, limits).evaluate(&looped, 0),
		Ok(Value::None)
	);
	let count = Expr::Builtin("String#codePointCount", vec![value(large)]);
	assert_eq!(
		Interpreter::new(&mut NoHost, limits).evaluate(&count, 0),
		Ok(Value::int(10_000))
	);
}

#[test]
fn locale_comparison_is_explicitly_deferred_to_the_runtime_machine() {
	let error = call(
		"String.compareLocale",
		vec![string("é"), string("z"), string("fr")],
	)
	.unwrap_err();
	assert!(
		matches!(error,Error::Unsupported(ref why) if why.contains("locale data of the machine"))
	);
}

#[test]
fn json_reads_utf8_bytes_and_preserves_escaped_surrogates() {
	use fa_interpreter::value::{Member, MemberKind};
	let schema = Value::Member(Arc::new(Member {
		name: Arc::from("Schema.String"),
		kind: MemberKind::Union,
		data: None,
	}));
	assert_eq!(
		call(
			"Json.parse",
			vec![
				bytes("\"é日😀\"".as_bytes()),
				schema.clone(),
				Value::Boolean(true)
			]
		),
		Ok(string("é日😀"))
	);
	assert_eq!(
		call(
			"Json.parse",
			vec![bytes(b"\"\\ud800\""), schema.clone(), Value::Boolean(true)]
		),
		Ok(units(&[0xd800]))
	);
	assert_eq!(
		call(
			"Json.parse",
			vec![
				units(&[34, 0xd800, 34]),
				schema.clone(),
				Value::Boolean(true)
			]
		),
		Ok(units(&[0xd800]))
	);
	// Private-use characters are ordinary data even next to a lone unit.
	assert_eq!(
		call(
			"Json.parse",
			vec![
				units(&[34, 0xe000, 0xd800, 0xe001, 34]),
				schema.clone(),
				Value::Boolean(true)
			]
		),
		Ok(units(&[0xe000, 0xd800, 0xe001]))
	);
	for bad in [bytes(&[255]), bytes(b"["), bytes(b"1")] {
		assert!(
			matches!(call("Json.parse",vec![bad,schema.clone(),Value::Boolean(true)]),Err(Error::Thrown(Value::Member(ref m))) if m.name.as_ref() == "JsonError")
		);
	}
}

#[test]
fn byte_copy_on_write_is_checked_before_allocating() {
	let data = bytes(&vec![0; 4096]);
	let place = Place {
		slot: 0,
		steps: vec![Step::Index(value(Value::int(0)))],
	};
	let expr = Expr::Block(vec![
		Expr::Let(0, value(data).boxed()),
		Expr::Assign(place, value(Value::int(255)).boxed()),
	]);
	let limits = Limits {
		memory: 1024,
		..Limits::default()
	};
	assert_eq!(
		Interpreter::new(&mut NoHost, limits).evaluate(&expr, 1),
		Err(Error::Memory(1024))
	);
}

#[test]
fn composite_interpolation_preserves_raw_units_until_output() {
	let composite = Value::array(vec![units(&[0xd800])]);
	let formatted = run(Expr::Template(vec![Part::Value(value(composite))]), 0).unwrap();
	assert_eq!(formatted, units(&[91, 34, 0xd800, 34, 93]));
	assert_eq!(formatted.to_text(), "[\"�\"]");
}

#[test]
fn slices_check_their_output_before_copying() {
	let limits = Limits {
		memory: 1024,
		..Limits::default()
	};
	for input in [string(&"é日😀".repeat(1000)), bytes(&vec![0; 4096])] {
		let range = Value::Range {
			start: None,
			end: Some(2000.into()),
			inclusive: false,
		};
		let expr = Expr::Index(value(input).boxed(), value(range).boxed());
		assert_eq!(
			Interpreter::new(&mut NoHost, limits).evaluate(&expr, 0),
			Err(Error::Memory(1024))
		);
	}
}
