//! The runtime checks in the IR are exactly the ones the analyzer's bits ask
//! for (design/analyzer-design.md §3.6): each program is compiled at `O0`
//! (no pass but dead-code removal) and its IR is searched.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::PathBuf;
	use std::sync::Arc;

	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{OptLevel, Options, Target, compile};

	/// The IR of the Fa functions and of `main` (the runtime functions start with `fa_`).
	fn ir_of(name: &str, source: &str) -> String {
		ir_with(name, source, true)
	}

	/// The IR with or without the overflow checks (`--overflow-checks=false`).
	fn ir_with(name: &str, source: &str, overflow_checks: bool) -> String {
		ir_at(name, source, overflow_checks, OptLevel::O0)
	}

	/// The optimized IR (`O3`, the default of `fa build`, which wraps).
	fn optimized_ir(name: &str, source: &str) -> String {
		ir_at(name, source, false, OptLevel::O3)
	}

	fn ir_at(name: &str, source: &str, overflow_checks: bool, opt_level: OptLevel) -> String {
		ir_at_target(name, source, overflow_checks, opt_level, Target::Native)
	}

	fn ir_at_target(
		name: &str,
		source: &str,
		overflow_checks: bool,
		opt_level: OptLevel,
		target: Target,
	) -> String {
		let ir = complete_ir_at_target(name, source, overflow_checks, opt_level, target);
		// keep the program's own functions
		let mut kept = String::new();
		let mut keep = false;
		for line in ir.lines() {
			if line.starts_with("define ") {
				keep =
					line.contains("@main(") || line.contains("@_start(") || line.contains("@fa.");
			}
			if keep {
				kept.push_str(line);
				kept.push('\n');
			}
			if line == "}" {
				keep = false;
			}
		}
		kept
	}

	fn complete_ir_at_target(
		name: &str,
		source: &str,
		overflow_checks: bool,
		opt_level: OptLevel,
		target: Target,
	) -> String {
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(source));
		session.analyze_file(file);
		let options = Options {
			opt_level,
			emit_ir: true,
			overflow_checks,
			..Options::default()
		};
		let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join(name);
		let artifact = compile(&session, file, "test.fa", target, &options, &output)
			.unwrap_or_else(|error| panic!("{name}: {error}"));
		std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file")
	}

	fn count(ir: &str, pattern: &str) -> usize {
		ir.matches(pattern).count()
	}

	#[test]
	fn appended_templates_check_the_total_before_reserving_on_both_pointer_widths() {
		let source = "function append = (piece: String): String => {\nmutable out = \"\"\nout += \"{piece}{piece}{piece}\"\nout\n}\nconsole.log(append(\"abc\"))\n";
		for (name, target, integer) in [
			("append-size-native", Target::Native, "i64"),
			("append-size-wasm", Target::Wasm, "i32"),
		] {
			let ir = ir_at_target(name, source, false, OptLevel::O0, target);
			let body = function(&ir, "fa.append");
			let checked: Vec<_> = body
				.lines()
				.filter(|line| line.contains(&format!("call {integer} @fa_string_size(")))
				.collect();
			assert_eq!(
				checked.len(),
				3,
				"each piece must enter the checked size chain: {body}"
			);
			let last = checked
				.last()
				.expect("the last checked sum")
				.trim()
				.split_once(" = ")
				.expect("a result")
				.0;
			let reserve = body
				.lines()
				.find(|line| line.contains("call void @fa_string_reserve("))
				.expect("the reservation");
			assert!(
				reserve.contains(&format!("{integer} {last})")),
				"reserve must use the checked total: {reserve}\n{body}"
			);
		}
	}

	#[test]
	fn inline_utf8_bytes_are_not_assumed_to_be_a_positive_raw_length() {
		let ir = ir_of(
			"inline-utf8-length",
			"function suffix = (text: String): String => text + \"!\"\nconsole.log(suffix(\"éééééééé\"))\n",
		);
		let mut checked = 0;
		for line in ir.lines() {
			let Some((_, comparison)) = line.split_once(" = icmp sge i64 ") else {
				continue;
			};
			let Some(length) = comparison.strip_suffix(", 0") else {
				continue;
			};
			assert!(
				ir.lines().any(|definition| definition
					.trim_start()
					.starts_with(&format!("{length} = select i1 "))),
				"the nonnegative length must select the inline length first: {line}\n{ir}"
			);
			checked += 1;
		}
		assert!(
			checked > 0,
			"the regression must exercise the length assumption: {ir}"
		);
	}

	#[test]
	fn callback_instances_keep_the_nonthrowing_calls_plain() {
		let ir = ir_of(
			"callback-instances",
			r#"type E = Errors { Bad }
type F = Errors { Bad }
function run = (action: () => Integer): Integer => action()
function good = (): Integer => 2
function failE = (): Integer throws E => { throw E.Bad }
function failF = (): Integer throws F => { throw F.Bad }
console.log(run(good), run(failE) catch _error => -1, run(failF) catch _error => -2)
"#,
		);
		let plain = ir
			.lines()
			.find(|line| line.starts_with("define ") && line.contains("@fa.run("))
			.expect("the nonthrowing instance");
		assert!(plain.contains("i64 @fa.run("), "{plain}");
		let specialized: Vec<_> = ir
			.lines()
			.filter(|line| line.starts_with("define ") && line.contains("@fa.run.callback"))
			.collect();
		assert_eq!(specialized.len(), 2, "{ir}");
		for header in specialized {
			assert!(header.contains("{ i64, ptr }"), "{header}");
		}
	}

	#[test]
	fn immutable_literals_use_readonly_data_for_every_collection() {
		for (kind, literal, measure) in [
			("Array<Integer>", "[2, 3, 5]", "length"),
			("GrowingArray<Integer>", "[2, 3, 5]", "length"),
			("Map<String, Integer>", "[(\"abcdefghijk\", 3)]", "size"),
			// entries in the probe array, with and without a tag
			("Map<UInt16, UInt8>", "[(2, 3), (0, 1)]", "size"),
			("Map<UInt32, UInt8>", "[(2, 3), (0, 1)]", "size"),
			("GrowingMap<Integer, Integer>", "[(2, 3)]", "size"),
			("Set<String>", "[\"x\", \"x\", \"é\"]", "size"),
			("Set<Float>", "[0.0, -0.0, 1.5]", "size"),
			("GrowingSet<Integer>", "[2, 3]", "size"),
			("Pool<String>", "[\"hello\"]", "size"),
			("GrowingPool<Integer>", "[2, 3]", "size"),
			("Columnar<{ value: Integer }>", "[{ value = 3 }]", "size"),
			(
				"GrowingColumnar<{ value: Integer }>",
				"[{ value = 3 }]",
				"size",
			),
		] {
			let name = format!("readonly-{}", kind.split('<').next().unwrap());
			let source = format!(
				"let global: {kind} = {literal}\nfunction read = (): Integer => {{ let local: {kind} = {literal}\nlocal.{measure} + global.{measure} }}\nconsole.log(read())\n"
			);
			let ir = ir_of(&name, &source);
			for constructor in [
				"@fa_array_new(",
				"@fa_table_new",
				"@fa_pool_new(",
				"@fa_columnar_new(",
				"@fa_segmented_new(",
				"@fa_alloc(",
				"@heap_alloc(",
			] {
				assert!(!ir.contains(constructor), "{kind}: {ir}");
			}
			let full = std::fs::read_to_string(
				PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
					.join("fa-checks")
					.join(format!("{name}.ll")),
			)
			.unwrap();
			assert!(
				full.lines()
					.any(|line| line.contains("@fa.collection.static") && line.contains("constant")),
				"{kind}: {full}"
			);
		}
	}

	#[test]
	fn runtime_immutable_literals_use_fixed_entry_allocas() {
		for (kind, literal, measure) in [
			("Array<Integer>", "[n, n + 1]", "length"),
			("GrowingArray<Integer>", "[n, n + 1]", "length"),
			("Map<Integer, Integer>", "[(n, n + 1)]", "size"),
			("GrowingMap<Integer, Integer>", "[(n, n + 1)]", "size"),
			("Set<Integer>", "[n, n + 1]", "size"),
			("GrowingSet<Integer>", "[n, n + 1]", "size"),
			("Pool<Integer>", "[n, n + 1]", "size"),
			("GrowingPool<Integer>", "[n, n + 1]", "size"),
		] {
			let name = format!("frame-{}", kind.split('<').next().unwrap());
			let source = format!(
				"function read = (n: Integer): Integer => {{ let local: {kind} = {literal}\nlocal.{measure} }}\nconsole.log(read(3))\n"
			);
			let ir = ir_of(&name, &source);
			assert!(ir.contains("collection.storage = alloca ["), "{kind}: {ir}");
			for constructor in [
				"@fa_array_new(",
				"@fa_table_new",
				"@fa_pool_new(",
				"@fa_segmented_new(",
				"@fa_alloc(",
				"@heap_alloc(",
			] {
				assert!(!ir.contains(constructor), "{kind}: {ir}");
			}
		}
	}

	#[test]
	fn collection_and_frame_budgets_keep_the_heap_fallback() {
		let large = std::iter::repeat_n("n", 300).collect::<Vec<_>>().join(", ");
		let source = format!(
			"function read = (n: Integer): Integer => {{\nlet values = [{large}]\nvalues.get(299) ?? 0\n}}\nconsole.log(read(3))\n"
		);
		let ir = ir_of("large-frame-fallback", &source);
		assert!(ir.contains("@fa_array_new("), "{ir}");
		assert!(!ir.contains("collection.storage = alloca"), "{ir}");
		let small = std::iter::repeat_n("n", 100).collect::<Vec<_>>().join(", ");
		let declarations = (0..5)
			.map(|i| format!("let a{i} = [{small}]\n"))
			.collect::<String>();
		let source = format!(
			"function read = (n: Integer): Integer => {{\n{declarations}(a0.get(0) ?? 0) + (a1.get(0) ?? 0) + (a2.get(0) ?? 0) + (a3.get(0) ?? 0) + (a4.get(0) ?? 0)\n}}\nconsole.log(read(3))\n"
		);
		let ir = ir_of("total-frame-fallback", &source);
		assert!(ir.contains("collection.storage = alloca"), "{ir}");
		assert!(ir.contains("@fa_array_new("), "{ir}");
	}

	#[test]
	fn escaping_literals_start_on_the_heap_and_split_receives_a_destination() {
		let source = "function returned = (n: Integer): Array<Integer> => {\nlet items = [n, n + 1]\nitems\n}\nconsole.log(returned(2))\n";
		let ir = ir_of("returned-collection-heap", source);
		assert!(!ir.contains("collection.storage = alloca"), "{ir}");
		assert!(ir.contains("@fa_array_new_into("), "{ir}");
		assert!(ir.contains("ptr null, i64 0, i64 2"), "{ir}");
		assert!(!ir.contains("call fastcc ptr @fa.copy"), "{ir}");
		let source = "function parts = (text: String): Integer => {\nlet words = text.split(\" \" )\nwords.length\n}\nconsole.log(parts(\"one two\"))\n";
		let ir = ir_of("split-caller-destination", source);
		assert!(
			ir.contains("collection.storage = alloca [2048 x i8]"),
			"{ir}"
		);
		assert!(ir.contains("@fa_string_split_into("), "{ir}");
		assert!(!ir.contains("@fa_string_split("), "{ir}");
	}

	#[test]
	fn fresh_array_methods_receive_bounded_caller_destinations() {
		for (name, expression) in [
			("map", "source.map((n) => n + 1)"),
			("filter", "source.filter((n) => n > 0)"),
			("from", "Array.from(source)"),
		] {
			let source = format!(
				"function read = (source: Array<Integer>): Integer => {{\nlet result = {expression}\nresult.length\n}}\nconsole.log(read([1, 2, 3]))\n"
			);
			let ir = ir_of(&format!("caller-{name}"), &source);
			assert!(
				ir.contains("collection.storage = alloca [2048 x i8]"),
				"{name}: {ir}"
			);
			assert!(ir.contains("@fa_array_new_into("), "{name}: {ir}");
		}
	}

	#[test]
	fn large_aggregate_results_use_and_forward_the_callers_return_slot() {
		let ir = ir_of(
			"aggregate-return-slots",
			r#"
type Data = { a: Integer, b: Integer, c: Integer }
function make = (n: Integer): Data => Data { a = n, b = n + 1, c = n + 2 }
function relay = (n: Integer): Data => if n > 0 { relay(n - 1) } else { make(n) }
function pair = (n: Integer): Tuple<Integer, Integer, Integer> => (n, n + 1, n + 2)
type Wrapped = { data: Data }
function wrapped = (n: Integer): Wrapped => Wrapped { data = make(n) }
let data = wrapped(2)
let tuple = pair(3)
console.log(data.data.a, tuple[0], relay(2).a)
"#,
		);
		for name in ["fa.make", "fa.relay", "fa.pair", "fa.wrapped"] {
			let body = function(&ir, name);
			assert!(body.lines().next().unwrap().contains("void"), "{body}");
			assert!(body.lines().next().unwrap().contains("sret("), "{body}");
			assert!(!body.contains("return.result = alloca"), "{body}");
			assert!(!body.contains("ret {"), "{body}");
		}
		let make = function(&ir, "fa.make");
		assert!(make.contains("result.field"), "{make}");
		assert!(!make.contains("insertvalue"), "{make}");
	}

	#[test]
	fn small_aggregate_results_stay_in_ssa() {
		let ir = ir_of(
			"small-aggregate-registers",
			r#"
type Data = { a: Integer, b: Integer }
function make = (n: Integer): Data => Data { a = n, b = n + 1 }
function relay = (n: Integer): Data => if n > 0 { relay(n - 1) } else { make(n) }
function pair = (n: Integer): Tuple<Integer, Integer> => (n, n + 1)
function parse = (text: String): Data throws ParseError => Data { a = try Integer.parse(text), b = 0 }
let data = parse("2") catch error => Data { a = -1, b = -1 }
console.log(relay(2).a, pair(3)[0], data.a)
"#,
		);
		for name in ["fa.make", "fa.relay", "fa.pair", "fa.parse"] {
			let body = function(&ir, name);
			assert!(!body.lines().next().unwrap().contains("void"), "{body}");
			assert!(!body.contains("sret("), "{body}");
			assert!(body.contains("ret {"), "{body}");
			assert!(!body.contains("return.result"), "{body}");
		}
	}

	#[test]
	fn aggregate_return_choice_counts_scalar_leaves_on_each_target() {
		let source = r#"
type One = { a: Integer }
type Two = { a: One, b: Integer }
type Three = { a: Boolean, b: Boolean, c: Boolean }
function one = (n: Integer): One => One { a = n }
function two = (n: Integer): Two => Two { a = one(n), b = n }
function three = (n: Integer): Three => Three { a = n > 0, b = n > 1, c = n > 2 }
console.log(one(1).a, two(2).b, three(3).a)
"#;
		for target in [Target::Native, Target::Wasm] {
			let ir = ir_at_target(
				&format!("aggregate-leaves-{}", target.name()),
				source,
				true,
				OptLevel::O0,
				target,
			);
			assert!(!function(&ir, "fa.one").contains("sret("), "{ir}");
			assert_eq!(
				function(&ir, "fa.two").contains("sret("),
				target == Target::Wasm,
				"{ir}"
			);
			assert!(function(&ir, "fa.three").contains("sret("), "{ir}");
		}
	}

	#[test]
	fn direct_call_cycles_limit_collection_frames_including_methods_and_generics() {
		let ir = ir_of(
			"recursive-collection-budgets",
			r#"
function first = (n: Integer, text: String): Integer => {
 let words = text.split(" ")
 if n > 0 { second(n - 1, text) + words.length } else { words.length }
}
function second = (n: Integer, text: String): Integer => {
 let words = text.split(" ")
 if n > 0 { generic(n - 1, 7, text) + words.length } else { words.length }
}
type Walker = {
 function walk = (n: Integer, text: String): Integer => {
  let words = text.split(" ")
  if n > 0 { Walker.walk(n - 1, text) + words.length } else { words.length }
 }
}
function generic = <T>(n: Integer, value: T, text: String): Integer => {
 let words = text.split(" ")
 if n > 0 { first(n - 1, text) + words.length } else { words.length }
}
console.log(first(1, "a b"), Walker.walk(1, "a b"), generic(1, 7, "a b"))
"#,
		);
		for name in ["fa.first", "fa.second", "fa.walk", "fa.generic.0"] {
			let body = function(&ir, name);
			assert!(body.contains("alloca [128 x i8]"), "{body}");
			assert!(!body.contains("alloca [2048 x i8]"), "{body}");
		}
	}

	#[test]
	fn call_offers_use_only_the_remaining_recursive_frame_budget() {
		let literal = std::iter::repeat_n("n", 54).collect::<Vec<_>>().join(", ");
		let source = format!(
			"function count = (n: Integer, text: String): Integer => {{\nlet exact = [{literal}]\nlet words = text.split(\" \")\nif n > 0 {{ count(n - 1, text) + exact.length + words.length }} else {{ exact.length + words.length }}\n}}\nconsole.log(count(1, \"a b\"))\n"
		);
		let ir = ir_of("remaining-recursive-budget", &source);
		let body = function(&ir, "fa.count");
		assert!(body.contains("alloca [464 x i8]"), "{body}");
		assert!(!body.contains("alloca [48 x i8]"), "{body}");
		assert!(!body.contains("@fa_string_split_into("), "{body}");
		let source = source.replace(&literal, "n");
		let ir = ir_of("remaining-recursive-offer", &source);
		let body = function(&ir, "fa.count");
		assert!(body.contains("alloca [48 x i8]"), "{body}");
		assert!(body.contains("alloca [128 x i8]"), "{body}");
		assert!(body.contains("@fa_string_split_into("), "{body}");

		let source = r#"
function count = (n: Integer, text: String): Integer => {
 let a = text.split(" ")
 let b = text.split(" ")
 let c = text.split(" ")
 let d = text.split(" ")
 if n > 0 { count(n - 1, text) + a.length + b.length + c.length + d.length } else { a.length + b.length + c.length + d.length }
}
console.log(count(1, "a b"))
"#;
		let ir = ir_of("shared-recursive-offers", source);
		let body = function(&ir, "fa.count");
		assert_eq!(body.matches("alloca [128 x i8]").count(), 1, "{body}");
		assert_eq!(body.matches("@fa_string_split_into(").count(), 1, "{body}");
		assert_eq!(body.matches("@fa_string_split(").count(), 3, "{body}");

		// Exact literals consume the total budget, independently of offer order.
		for (length, offer) in [(50, Some(80)), (54, None)] {
			let exact = (32 + length * 8 + 15) & !15;
			let literal = std::iter::repeat_n("n", length)
				.collect::<Vec<_>>()
				.join(", ");
			let declarations = format!("let exact = [{literal}]\nlet words = text.split(\" \")");
			let source = format!(
				"function count = (n: Integer, text: String): Integer => {{\n{declarations}\nif n > 0 {{ count(n - 1, text) + exact.length + words.length }} else {{ exact.length + words.length }}\n}}\nconsole.log(count(1, \"a b\"))\n"
			);
			let ir = ir_of(&format!("literal-before-offer-{length}"), &source);
			let body = function(&ir, "fa.count");
			assert!(body.contains(&format!("alloca [{exact} x i8]")), "{body}");
			assert_eq!(
				body.contains("@fa_string_split_into("),
				offer.is_some(),
				"{body}"
			);
			if let Some(bytes) = offer {
				assert!(body.contains(&format!("alloca [{bytes} x i8]")), "{body}");
			}
		}
		for (length, fits) in [(42, true), (50, false)] {
			let exact = (32 + length * 8 + 15) & !15;
			let literal = std::iter::repeat_n("n", length)
				.collect::<Vec<_>>()
				.join(", ");
			let declarations = format!("let words = text.split(\" \")\nlet exact = [{literal}]");
			let source = format!(
				"function count = (n: Integer, text: String): Integer => {{\n{declarations}\nif n > 0 {{ count(n - 1, text) + exact.length + words.length }} else {{ exact.length + words.length }}\n}}\nconsole.log(count(1, \"a b\"))\n"
			);
			let ir = ir_of(&format!("offer-before-literal-{length}"), &source);
			let body = function(&ir, "fa.count");
			assert!(body.contains("alloca [128 x i8]"), "{body}");
			assert_eq!(
				body.contains(&format!("alloca [{exact} x i8]")),
				fits,
				"{body}"
			);
		}
	}

	#[test]
	fn readonly_string_parameters_borrow_inline_bytes_without_snapshots() {
		let ir = ir_of(
			"readonly-string-bytes",
			r#"
function read = (text: String, n: Integer): Integer => if n > 0 {
 read(text, n - 1) + text.split(" ").length
} else { text.split(" ").length }
function change = (text: String): Integer => {
 mutable local = text
 local += "extra"
 local.split(" ").length
}
mutable text = "a b"
console.log(read(text, 1), change(text))
"#,
		);
		let read = function(&ir, "fa.read");
		assert!(
			read.lines()
				.any(|line| line.contains("select i1") && line.contains("ptr %0,")),
			"{read}"
		);
		assert!(function(&ir, "fa.change").contains("inline_string"), "{ir}");
		let ir = optimized_ir(
			"recursive-split-scratch",
			include_str!("programs/recursive-collection-offers.fa"),
		);
		let body = function(&ir, "fa.four");
		assert!(body.contains("@fa_string_split_into("), "{body}");
		assert!(!body.contains("alloca %struct.Splitting"), "{body}");
		assert!(!body.contains("inline_string"), "{body}");
	}

	#[test]
	fn throwing_aggregate_returns_keep_errors_outside_the_result_slot() {
		let ir = ir_of(
			"aggregate-return-errors",
			r#"
type Data = { number: Integer, padding: Integer, more: Integer }
function parse = (text: String): Data throws ParseError => Data { number = try Integer.parse(text), padding = 0, more = 0 }
let value = parse("2") catch error => Data { number = -1, padding = 0, more = 0 }
console.log(value.number)
"#,
		);
		let body = function(&ir, "fa.parse");
		assert!(
			body.lines().next().unwrap().contains("fastcc ptr"),
			"{body}"
		);
		assert!(!body.contains("sret("), "{body}");
		assert!(!body.contains("ret {"), "{body}");
	}

	#[test]
	fn collection_functions_forward_bounded_buffers_for_every_literal_layout() {
		for (kind, literal, measure) in [
			("Array<Integer>", "[n, n + 1]", "length"),
			("GrowingArray<Integer>", "[n, n + 1]", "length"),
			("Map<Integer, Integer>", "[(n, n + 1)]", "size"),
			("GrowingMap<Integer, Integer>", "[(n, n + 1)]", "size"),
			("Set<Integer>", "[n, n + 1]", "size"),
			("GrowingSet<Integer>", "[n, n + 1]", "size"),
			("Pool<Integer>", "[n, n + 1]", "size"),
			("GrowingPool<Integer>", "[n, n + 1]", "size"),
		] {
			let source = format!(
				"function make = (n: Integer): {kind} => {literal}\nfunction read = (n: Integer): Integer => {{ let local = make(n)\nlocal.{measure} }}\nconsole.log(read(2))\n"
			);
			let ir = ir_of(
				&format!("return-{}", kind.split('<').next().unwrap()),
				&source,
			);
			let make = function(&ir, "fa.make");
			assert!(
				make.lines()
					.next()
					.unwrap()
					.contains("(ptr noundef %0, i64 noundef %1, i64 noundef %2)"),
				"{kind}: {make}"
			);
			assert!(
				!make.contains("collection.storage = alloca"),
				"{kind}: {make}"
			);
			let read = function(&ir, "fa.read");
			assert!(
				read.contains("collection.storage = alloca [2048 x i8]"),
				"{kind}: {read}"
			);
			assert!(read.contains("i64 2048"), "{kind}: {read}");
			assert!(!make.contains("call fastcc ptr @fa.copy"), "{kind}: {make}");
		}
	}

	#[test]
	fn literal_elements_use_their_final_aggregate_return_slot() {
		let ir = ir_of(
			"literal-item-destinations",
			r#"
type Data = { number: Integer, text: String }
function make = (n: Integer): Data => Data { number = n, text = "value {n}" }
function list = (n: Integer): Array<Data> => [make(n), make(n + 1)]
console.log(list(2).length)
"#,
		);
		let list = function(&ir, "fa.list");
		assert!(list.contains("call fastcc void @fa.make(ptr"), "{list}");
		assert!(!list.contains("return.result = alloca"), "{list}");
		assert!(!list.contains("store {"), "{list}");
	}

	#[test]
	fn an_immutable_returned_owner_coalesces_with_its_caller_slot() {
		let ir = ir_of(
			"coalesced-return-local",
			r#"
type Data = { number: Integer, text: String }
function make = (n: Integer): Data => Data { number = n, text = "value {n}" }
function returned = (n: Integer): Data => {
 let result = make(n)
 if n > 0 { return result }
 return result
}
console.log(returned(2).number)
"#,
		);
		let returned = function(&ir, "fa.returned");
		assert!(returned.contains("@fa.make(ptr %0"), "{returned}");
		assert!(
			!returned.contains("alloca { i64, { ptr, i64, i64 } }"),
			"{returned}"
		);
		assert!(!returned.contains("store {"), "{returned}");
		assert!(
			!returned.contains("call fastcc void @fa.drop"),
			"{returned}"
		);
	}

	#[test]
	fn coalesced_local_ids_do_not_alias_imported_globals() {
		use fa_analyzer::modules::CodebaseResolver;
		let fields = (0..64)
			.map(|i| format!("mutable v{i} = {i}\n"))
			.collect::<String>();
		let sum = (0..64)
			.map(|i| format!("remote.v{i}"))
			.collect::<Vec<_>>()
			.join(" + ");
		let padding = (0..64)
			.map(|i| format!("p{i}: Integer"))
			.collect::<Vec<_>>()
			.join(", ");
		let library = format!(
			"function padding = ({padding}): Integer => p0\nexport namespace = {{\n{fields}}}\n"
		);
		let main = format!(
			"use remote = library.values\ntype Data = {{ number: Integer, text: String }}\nfunction build = (n: Integer): Data => {{\nlet result = Data {{ number = n + {sum}, text = \"owned payload {{n}} beyond the inline limit\" }}\nreturn result\n}}\nexport function = () => {{ let item = build(7)\nconsole.log(item.number) }}\n"
		);
		let files = vec![
			("library/values.fa".to_string(), library),
			("application/main.fa".to_string(), main),
		];
		let mut session = Session::new();
		session.set_resolver(Box::new(CodebaseResolver::from_texts(
			&files,
			Some("application/main.fa"),
		)));
		let program = session.analyze_package(Some("application/main"));
		let main = session.file_of_module("application/main").unwrap();
		let library = session.file_of_module("library/values").unwrap();
		let index = session
			.analysis(main)
			.symbols
			.symbols()
			.iter()
			.position(|symbol| session.types().syms.resolve(symbol.name) == "result")
			.unwrap();
		assert!(
			session
				.types()
				.syms
				.resolve(session.analysis(library).symbols.symbols()[index].name)
				.starts_with('v'),
			"fixture must collide file-local IDs"
		);
		let options = Options {
			opt_level: OptLevel::O0,
			emit_ir: true,
			leak_check: true,
			..Options::default()
		};
		let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join("return-global-ids");
		let artifact =
			fa_codegen_llvm::compile_program(&session, &program, Target::Native, &options, &output)
				.unwrap();
		let ir = std::fs::read_to_string(artifact.ir.as_ref().unwrap()).unwrap();
		let build = function(&ir, "fa.build");
		assert!(
			!build.contains("local = alloca"),
			"the returned local must be coalesced: {build}"
		);
		let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
		assert!(
			result.status.success(),
			"{}",
			String::from_utf8_lossy(&result.stderr)
		);
		assert_eq!(String::from_utf8_lossy(&result.stdout), "2023\n");
	}

	#[test]
	fn reservation_precedes_a_push_loop_without_growth_calls_in_the_loop() {
		let ir = ir_of(
			"reserve-loop",
			r#"
function fill = (items: Array<Integer>, count: Integer<min = 0>) mutates items => {
    items.reserve(count)
    for i in 0..count { items.push(i) }
}
mutable items: Array<Integer> = []
fill(items, 100)
console.log(items.length)
"#,
		);
		let fill = function(&ir, "fa.fill");
		assert_eq!(count(fill, "@fa_array_reserve("), 1, "{fill}");
		assert!(!fill.contains("@fa_array_grow("), "{fill}");
	}

	#[test]
	fn an_unproven_sum_is_checked_and_proven_differences_are_nsw() {
		let ir = ir_of(
			"fib",
			"function fib = (n: Integer): Integer => if n < 2 { n } else { fib(n - 1) + fib(n - 2) }\nconsole.log(fib(30))\n",
		);
		assert_eq!(
			count(&ir, "call { i64, i1 } @llvm.sadd.with.overflow.i64"),
			1,
			"{ir}"
		);
		assert_eq!(count(&ir, "@llvm.ssub.with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "sub nsw i64"), 2, "{ir}");
	}

	#[test]
	fn a_sum_of_int8_into_int16_has_no_check() {
		let ir = ir_of(
			"no-check",
			"function widen = (a: Int8, b: Int8): Int16 => a + b\nconsole.log(widen(100, 100))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "@fa_overflow"), 0, "{ir}");
		// bounded by the storage of its operands: computed in 16 bits
		assert_eq!(count(&ir, "add nsw i16"), 1, "{ir}");
	}

	#[test]
	fn a_sum_of_int8_into_int8_is_checked_at_its_sink_only() {
		let ir = ir_of(
			"sink",
			"function add = (a: Int8, b: Int8): Int8 => a + b\nconsole.log(add(100, 27))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
		assert_eq!(count(&ir, "icmp slt i64"), 1, "{ir}");
		assert_eq!(count(&ir, "icmp sgt i64"), 1, "{ir}");
	}

	#[test]
	fn a_product_of_integers_is_checked_once() {
		let ir = ir_of(
			"product",
			"function product = (a: Integer, b: Integer): Integer => a * b\nconsole.log(product(6, 7))\n",
		);
		assert_eq!(count(&ir, "@llvm.smul.with.overflow.i64("), 1, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
	}

	#[test]
	fn unsigned_64_bit_operands_compute_in_128_bits() {
		let ir = ir_of(
			"uint64",
			"function add = (a: UInt64, b: UInt64): UInt64 => a + b\nconsole.log(add(1, 2))\n",
		);
		assert_eq!(count(&ir, "with.overflow"), 0, "{ir}");
		assert_eq!(count(&ir, "add nsw i128"), 1, "{ir}");
		assert_eq!(count(&ir, "call void @fa_overflow("), 1, "{ir}");
	}

	#[test]
	fn divisions_check_the_divisor_and_the_minimum_only_when_needed() {
		let ir = ir_of(
			"divisions",
			"function half = (a: Integer): Integer => a // 2\nfunction divide = (a: Integer, b: Integer): Integer throws DivisionByZero => try a // b\nfunction rest = (a: Int8, b: Int8): Int8 throws DivisionByZero => try a %% b\nconsole.log(half(7), divide(7, 2) catch error => 0, rest(7, 2) catch error => 0)\n",
		);
		assert_eq!(
			count(&ir, "call ptr @fa_error_division_by_zero()"),
			2,
			"{ir}"
		);
		// `Integer.minValue // -1` overflows; the `Int8` remainder computes in 64 bits
		let minimum_guards = ir
			.lines()
			.filter(|line| line.contains("icmp eq i64") && line.ends_with(", -9223372036854775808"))
			.count();
		assert_eq!(minimum_guards, 1, "{ir}");
		assert_eq!(count(&ir, "sdiv i64"), 3, "{ir}");
	}

	#[test]
	fn an_unproven_conversion_is_checked() {
		let ir = ir_of(
			"conversion",
			"function narrow = (value: Integer): Int8 throws ArithmeticError => try Int8(value)\nfunction small = (value: Int8): Int16 => Int16(value)\nconsole.log(narrow(5) catch error => 0, small(5))\n",
		);
		assert_eq!(count(&ir, "call ptr @fa_error_conversion("), 1, "{ir}");
	}

	#[test]
	fn without_overflow_checks_integers_wrap() {
		let source = "function fib = (n: Integer): Integer => if n < 2 { n } else { fib(n - 1) + fib(n - 2) }\nfunction add = (a: Int8, b: Int8): Int8 => a + b\nfunction product = (a: Integer, b: Integer): Integer => a * b\nconsole.log(fib(30), add(1, 2), product(6, 7))\n";
		let checked = ir_of("checked", source);
		assert_eq!(count(&checked, "with.overflow"), 2, "{checked}");
		assert_eq!(count(&checked, "call void @fa_overflow("), 3, "{checked}");
		let unchecked = ir_with("unchecked", source, false);
		assert_eq!(count(&unchecked, "with.overflow"), 0, "{unchecked}");
		assert_eq!(count(&unchecked, "@fa_overflow"), 0, "{unchecked}");
		assert_eq!(count(&unchecked, " nsw "), 0, "{unchecked}");
		assert_eq!(count(&unchecked, " nuw "), 0, "{unchecked}");
		// the `Int8` sink truncates
		assert!(unchecked.contains("trunc i64"), "{unchecked}");
	}

	#[test]
	fn without_overflow_checks_the_other_checks_stay() {
		let source = "function divide = (a: Integer, b: Integer): Integer throws DivisionByZero => try a // b\nfunction narrow = (value: Integer): Int8 throws ArithmeticError => try Int8(value)\nconsole.log(divide(7, 2) catch error => 0, narrow(5) catch error => 0)\n";
		let unchecked = ir_with("other-checks", source, false);
		assert_eq!(
			count(&unchecked, "call ptr @fa_error_division_by_zero()"),
			1,
			"{unchecked}"
		);
		assert_eq!(
			count(&unchecked, "call ptr @fa_error_conversion("),
			1,
			"{unchecked}"
		);
		// `MIN // -1` divides by 1 instead of stopping
		assert_eq!(count(&unchecked, "@fa_overflow"), 0, "{unchecked}");
		assert_eq!(
			count(function(&unchecked, "fa.divide"), "select i1"),
			2,
			"{unchecked}"
		);
	}

	/// The text of one function of the IR.
	fn function<'ir>(ir: &'ir str, name: &str) -> &'ir str {
		let signature = format!("@{name}(");
		let mut offset = 0;
		let start = ir
			.lines()
			.find_map(|line| {
				let at = offset;
				offset += line.len() + 1;
				(line.starts_with("define ") && line.contains(&signature)).then_some(at)
			})
			.unwrap_or_else(|| panic!("no `{name}` in {ir}"));
		let end = ir[start..].find("\n}").map_or(ir.len(), |end| start + end);
		&ir[start..end]
	}

	#[test]
	fn a_byte_kernel_with_checked_indices_is_vectorized() {
		// `blur`: the checks `0 <= i + k < length` leave the main loop
		// (`irce`), and the sums of bytes are computed on 16-bit lanes;
		// with a run-time size and with a constant one (the headers of the
		// arrays are read before the loops, `hoisting.rs`)
		let program = "function blur = (source: Array<UInt8>, destination: Array<UInt8>, width: Integer) throws IndexError mutates destination => {\n\tfor y in 1..width - 1 {\n\t\tfor x in 1..width - 1 {\n\t\t\tlet i = y * width + x\n\t\t\tlet sum: Integer = try source[i - width] + source[i - 1] * 2 + source[i] * 4 + source[i + 1] * 2 + source[i + width]\n\t\t\ttry destination[i] = sum // 16\n\t\t}\n\t}\n}\nlet width = arguments.length + 40\nmutable a = Array<UInt8>.from([], capacity = width * width)\nmutable b = Array<UInt8>.from([], capacity = width * width)\nfor index in 0..width * width {\n\ta.push(index %% 256)\n\tb.push(0)\n}\nfor 0..4 {\n\ttry blur(a, b, width)\n\ttry blur(b, a, width)\n}\nconsole.log(a.get(width + 1) ?? 0)\n";
		for (name, width) in [("blur", "arguments.length + 40"), ("blur-constant", "40")] {
			let source = program.replace("arguments.length + 40", width);
			let ir = optimized_ir(name, &source);
			let blur = function(&ir, "fa.blur");
			assert!(blur.contains("load <16 x i8>"), "{name}: {blur}");
			assert!(blur.contains("<16 x i16>"), "{name}: {blur}");
		}
	}

	#[test]
	fn an_index_read_from_a_proven_field_is_not_checked() {
		// every store of `Link.next` is an index of `links`, which never
		// shrinks (`fa_analyzer::proofs`): `items[from].next` is checked,
		// `items[items[from].next]` is not; a store out of bounds keeps it
		let program = "type Link = { next: UInt32 }\nmutable links: Array<Link> = [{ next = 1 }, { next = 0 }]\nfunction hop = (items: Array<Link>, from: UInt32): UInt32 throws IndexError => try items[try items[from].next].next\nfunction grow = (items: Array<Link>) mutates items => {\n\tlet last: UInt32 = items.length - 1\n\titems.push({ next = last })\n}\ngrow(links)\nconsole.log(hop(links, 2) catch error => 99)\n";
		let ir = ir_of("hop", program);
		assert_eq!(count(function(&ir, "fa.hop"), "fa_error_index"), 1);
		let wide = program.replace("{ next = last }", "{ next = last + 5 }");
		let ir = ir_of("hop-wide", &wide);
		assert_eq!(count(function(&ir, "fa.hop"), "fa_error_index"), 2);
	}

	#[test]
	fn a_multi_file_program_proves_private_arrays_with_every_files_stores() {
		for (other, checks) in [
			("export function = (): UInt32 => 0\n", 0),
			(
				"export function = (): UInt32 => {\n\tlet wide = { next = UInt32(1000000), extra = 1 }\n\twide.next\n}\n",
				1,
			),
		] {
			let files = [
				("package.fa".to_string(), "export = { name = \"indices\" }\n".to_string()),
				("library/chain.fa".to_string(), "type Link = { next: UInt32 }\nmutable links: Array<Link> = [{ next = 0 }]\nfunction hop = (items: Array<Link>, from: UInt32): UInt32 throws IndexError => try items[try items[from].next].next\nexport function = (): UInt32 => hop(links, 0) catch error => 99\n".to_string()),
				("library/other.fa".to_string(), other.to_string()),
				("application/main.fa".to_string(), "export function = () => console.log(library.chain(), library.other())\n".to_string()),
			];
			let mut session = Session::new();
			session.set_target("native");
			session.set_resolver(Box::new(fa_analyzer::CodebaseResolver::from_texts(
				&files, None,
			)));
			let program = session.analyze_package(Some("application/main"));
			let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
				.join("fa-checks")
				.join(format!("private-indices-{checks}"));
			let options = Options {
				opt_level: OptLevel::O0,
				emit_ir: true,
				..Options::default()
			};
			let artifact = fa_codegen_llvm::compile_program(
				&session,
				&program,
				Target::Native,
				&options,
				&output,
			)
			.expect("the package compiles");
			let ir = std::fs::read_to_string(artifact.ir.expect("IR emitted")).expect("IR file");
			let mut own = String::new();
			let mut keep = false;
			for line in ir.lines() {
				if line.starts_with("define ") {
					keep = line.contains("@fa.");
				}
				if keep {
					own.push_str(line);
					own.push('\n');
				}
				if line == "}" {
					keep = false;
				}
			}
			assert_eq!(count(&own, "fa_error_index"), checks, "{own}");
		}
	}

	#[test]
	fn a_field_counted_up_from_zero_is_never_negative() {
		// `get(self.position)` counts a negative index from the end: a select
		// on `position < 0`, which the proof that every store of the field
		// is non-negative (`index_proofs.rs`) takes out, with or without
		// overflow checks (a step of one), unless a store may be negative
		let program = "type Cursor = {\n\ttext: String\n\tposition: Integer\n\tfunction current = (self): Byte => self.text.toUtf8().get(self.position) ?? 0\n\tfunction advance = (self) mutates self => {\n\t\tself.position += 1\n\t}\n}\nmutable cursor = Cursor { text = \"ab\", position = 0 }\nwhile cursor.current() != 0 {\n\tcursor.advance()\n}\nconsole.log(cursor.position)\n";
		for checked in [true, false] {
			let ir = ir_with("cursor", program, checked);
			let current = function(&ir, "fa.current");
			assert_eq!(count(current, "icmp slt"), 0, "{current}");
		}
		let negative = program.replace(
			"self.position += 1",
			"self.position += 1\n\t\tself.position -= 2",
		);
		let ir = ir_of("cursor-negative", &negative);
		assert!(count(function(&ir, "fa.current"), "icmp slt") > 0);
		// a step of more than one may wrap without overflow checks
		let large = program.replace("self.position += 1", "self.position += 2");
		let ir = ir_with("cursor-large", &large, true);
		assert_eq!(count(function(&ir, "fa.current"), "icmp slt"), 0);
		let ir = ir_with("cursor-large-wrap", &large, false);
		assert!(count(function(&ir, "fa.current"), "icmp slt") > 0);
	}

	#[test]
	fn a_field_never_written_after_construction_is_read_once_by_a_method() {
		// `self.text` is only set by the literal: `words` reads it (and its
		// bytes and length, past the inline-or-heap test) once at its entry,
		// and the small methods it calls on `self` are expanded into it
		// (`frozen.rs`); a store anywhere, or a whole value of the type
		// assigned, makes it read at each use again
		let program = "type Scanner = {\n\ttext: String\n\tposition: Integer\n\tcount: Integer\n\tfunction current = (self): Byte => self.text.toUtf8().get(self.position) ?? 0\n\tfunction skipSpaces = (self) mutates self => {\n\t\twhile self.current() == 32 {\n\t\t\tself.position += 1\n\t\t}\n\t}\n\tfunction words = (self): Integer mutates self => {\n\t\tloop {\n\t\t\tself.skipSpaces()\n\t\t\tif self.current() == 0 {\n\t\t\t\tbreak\n\t\t\t}\n\t\t\twhile self.current() != 32 and self.current() != 0 {\n\t\t\t\tself.position += 1\n\t\t\t}\n\t\t\tself.count += 1\n\t\t}\n\t\tself.count\n\t}\n}\nmutable scanner = Scanner { text = \"one two\", position = 0, count = 0 }\nconsole.log(scanner.words())\n";
		let inline_test = "2305843009213693952";
		let ir = ir_of("frozen", program);
		let words = function(&ir, "fa.words");
		assert!(
			!words.contains("@fa.current(") && !words.contains("@fa.skipSpaces("),
			"{words}"
		);
		assert_eq!(count(words, inline_test), 1, "{words}");
		for (name, change) in [
			("frozen-field-written", "scanner.text = \"x\"\n"),
			("frozen-field-appended", "scanner.text += \"x\"\n"),
			(
				"frozen-whole-written",
				"scanner = Scanner { text = \"\", position = 0, count = 0 }\n",
			),
		] {
			let ir = ir_of(name, &format!("{program}{change}"));
			let words = function(&ir, "fa.words");
			assert!(words.contains("@fa.current("), "{name}: {words}");
		}
	}

	#[test]
	fn optional_recursive_sums_use_an_accumulator_loop() {
		let ir = optimized_ir(
			"recursive-optional-sum",
			"type Tree = { left: Tree?, right: Tree?, function count = (self): Integer => 1 + (self.left?.count() ?? 0) + (self.right?.count() ?? 0) }\nfunction build = (n: Integer): Tree => if n > 0 { Tree { left = build(n - 1), right = build(n - 1) } } else { Tree {} }\nlet n = try Integer.parse(arguments.get(0) ?? \"5\")\nconsole.log(build(n).count())\n",
		);
		let count = function(&ir, "fa.count");
		assert_eq!(
			count.matches("call fastcc i64 @fa.count(").count(),
			1,
			"{count}"
		);
		assert!(count.contains("phi i64"), "{count}");
	}

	#[test]
	fn empty_arena_leaves_do_not_call_the_recursive_builder() {
		let ir = optimized_ir(
			"recursive-empty-leaves",
			"type Tree = { left: Tree?, right: Tree? }\nfunction build = (n: Integer): Tree => if n > 0 { Tree { left = build(n - 1), right = build(n - 1) } } else { Tree {} }\nlet n = try Integer.parse(arguments.get(0) ?? \"5\")\nconsole.log(build(n))\n",
		);
		let build = function(&ir, "fa.build.arena");
		assert!(
			build.contains("icmp sgt i64 %0, 1") || build.contains("icmp eq i64 %0, 1"),
			"{build}"
		);
	}

	#[test]
	fn byte_arithmetic_is_computed_in_16_bits() {
		let ir = ir_of(
			"narrow",
			"function mix = (a: UInt8, b: UInt8, c: UInt8): Integer => a + b * 2 + c * 4\nconsole.log(mix(1, 2, 3))\n",
		);
		assert_eq!(count(&ir, "mul nuw i16"), 2, "{ir}");
		assert_eq!(count(&ir, "add nuw i16"), 2, "{ir}");
		assert_eq!(count(&ir, "zext i16"), 3, "{ir}");
	}

	#[test]
	fn ascii_literals_are_simple_strings() {
		// the top bit of the capacity word: one byte per character
		let ir = ir_of(
			"simple",
			"let ascii = \"abc\"\nlet other = \"héllo\"\nconsole.log(ascii.length, other.length)\n",
		);
		assert!(ir.contains("i64 3, i64 -9223372036854775808 }"), "{ir}");
		assert!(ir.contains("i64 6, i64 1152921504606846981 }"), "{ir}");
	}

	#[test]
	fn indexed_string_literal_comparisons_read_code_units() {
		let ir = ir_of(
			"unit-comparisons",
			r#"
function equal = (text: String, index: Integer): Boolean throws IndexError => try text[index] == "é"
function choose = (text: String, index: Integer): Integer throws IndexError => when try text[index] is { "a" => 1, "é" => 2, else => 3 }
console.log(try equal("é", 0), try choose("a", 0))
"#,
		);
		for name in ["fa.equal", "fa.choose"] {
			let body = function(&ir, name);
			assert!(body.contains("@string_ascii_unit_fallback("), "{body}");
			assert!(body.contains("@string_code_unit_slow("), "{body}");
			assert!(body.contains("load i8"), "{body}");
			assert!(!body.contains("@fa_string_get("), "{body}");
		}
	}

	#[test]
	fn ascii_index_fallback_reads_only_its_input() {
		let ir = complete_ir_at_target(
			"ascii-unit-effects",
			r#"
let text = arguments.get(0) ?? "é😀"
let index = Integer(arguments.get(1) ?? "0") catch _error => -1
console.log(text.codeUnitAt(index) catch _error => 0)
"#,
			false,
			OptLevel::O3,
			Target::Native,
		);
		let body = function(&ir, "string_ascii_unit_fallback");
		let attribute = body
			.lines()
			.next()
			.expect("function header")
			.split_whitespace()
			.find(|word| word.starts_with('#'))
			.expect("attribute group");
		let attributes = ir
			.lines()
			.find(|line| line.starts_with(&format!("attributes {attribute} =")))
			.expect("defined attributes");
		assert!(attributes.contains("memory(argmem: read)"), "{attributes}");
		assert!(
			!body.contains("store ") && !body.contains("call "),
			"{body}"
		);
		assert!(ir.contains("@string_code_unit_slow("), "{ir}");
	}

	#[test]
	fn locally_caught_unused_index_errors_need_no_payload() {
		let ir = ir_of(
			"caught-index-payload",
			r#"
function ignored = (text: String, index: Integer): UInt16 => text.codeUnitAt(index) catch _error => 0
function observed = (text: String, index: Integer): String => {
	let unit = text.codeUnitAt(index) catch error => return "{error}"
	"{unit}"
}
console.log(ignored("a", 2), observed("a", 2))
"#,
		);
		assert!(
			!function(&ir, "fa.ignored").contains("@fa_error_index("),
			"{ir}"
		);
		for helper in ["@string_length_slow(", "@unit_count("] {
			assert!(!function(&ir, "fa.ignored").contains(helper), "{ir}");
		}
		assert!(
			function(&ir, "fa.observed").contains("@fa_error_index("),
			"{ir}"
		);
	}

	#[test]
	fn utf8_conversions_consume_only_temporaries_and_dead_locals() {
		let ir = ir_of(
			"utf8-moves",
			r#"
function encode_live = (text: String): Bytes => text.toUtf8()
function encode_dead = (): Bytes => {
	mutable text = String.from("a long string whose allocation is transferred", capacity = 256)
	text.toUtf8()
}
function decode_live = (bytes: Bytes): String throws Utf8Error => try String.fromUtf8(bytes)
function decode_dead = (): String throws Utf8Error => try String.fromUtf8(Bytes.filled(160, 97))
console.log(encode_live("abc"), encode_dead(), try decode_live(Bytes.filled(1, 97)), try decode_dead())
"#,
		);
		for (name, helper, moved) in [
			("fa.encode_live", "@fa_string_to_utf8(", false),
			("fa.encode_dead", "@fa_string_to_utf8(", true),
			("fa.decode_live", "@fa_string_decode_bytes(", false),
			("fa.decode_dead", "@fa_string_decode_bytes(", true),
		] {
			let body = function(&ir, name);
			let call = body
				.lines()
				.find(|line| line.contains(helper))
				.expect("conversion call");
			assert!(
				call.contains(if moved { "i32 1" } else { "i32 0" }),
				"{body}"
			);
		}
	}

	#[test]
	fn ascii_packed_text_keeps_its_short_and_extended_headers() {
		let source = format!(
			r#"
type Entry = Union {{ Text = {{ id: Int32, text: String }}, Number = {{ value: Integer }} }}
mutable entries: Array<Entry> = []
entries.push(Entry.Text {{ id = 0, text = "abc" }})
entries.push(Entry.Text {{ id = 0, text = "{}" }})
entries.push(Entry.Number {{ value = 1 }})
mutable total = 0
for entry in entries {{
	total += when entry is {{ Entry.Text as item => Integer(item.id) + item.text.length, Entry.Number as item => item.value }}
}}
console.log(total)
"#,
			"x".repeat(128)
		);
		for (name, target, extended) in [
			("packed-ascii-64", Target::Native, 142),
			("packed-ascii-32", Target::Wasm, 138),
		] {
			let ir = ir_at_target(name, &source, false, OptLevel::O3, target);
			let grows: Vec<_> = ir
				.lines()
				.filter(|line| line.contains("@packed_grow("))
				.collect();
			assert!(
				grows.iter().any(|line| line.contains("i64 noundef 9)")),
				"{name}: {grows:?}; IR bytes {} packed {}",
				ir.len(),
				count(&ir, "packed")
			);
			assert!(
				grows
					.iter()
					.any(|line| line.contains(&format!("i64 noundef {extended})"))),
				"{grows:?}"
			);
		}
	}

	#[test]
	fn cached_string_length_preserves_the_value_header() {
		let ir = optimized_ir(
			"string-cache-header",
			r#"
function measure = (items: Array<String>): Integer => {
	mutable total = 0
	for item in items {
		total += item.length + item.utf8Length()
	}
	total
}
console.log(measure(["é😀".repeat(100)]))
"#,
		);
		// An owned length query reads its fixed byte/unit base without changing
		// the item's three words. The subsequent byte length reuses them.
		let body = function(&ir, "main");
		let call = body.find("@string_length_slow(").expect("a cached query");
		let after = body[call..]
			.lines()
			.skip(1)
			.take_while(|line| !line.is_empty());
		assert!(
			after.into_iter().all(|line| !line.contains(" = load ")),
			"{body}"
		);
	}

	#[test]
	fn utf8_iteration_encodes_with_bounded_scratch_without_bytes_allocation() {
		let source = r#"
function sum = (text: String): Integer => {
	mutable total = 0
	for byte in text.toUtf8() { total += byte }
	total
}
console.log(sum("é😀"))
"#;
		for target in [Target::Native, Target::Wasm] {
			let ir = ir_at_target(
				&format!("utf8-cursor-{}", target.name()),
				source,
				true,
				OptLevel::O0,
				target,
			);
			assert!(ir.contains("@fa_next_utf8_byte("), "{ir}");
			assert!(!ir.contains("@fa_array_new"), "{ir}");
			let word = if target == Target::Wasm { "i32" } else { "i64" };
			assert!(
				ir.contains(&format!("alloca {{ {word}, i32, i32 }}")),
				"{ir}"
			);
		}
	}

	#[test]
	fn ordinary_string_operations_do_not_link_grapheme_tables() {
		let source = r#"
let text = arguments.get(0) ?? "é😀"
console.log(text.length, text.split("").length)
console.log(text.replace("é", "x").padStart(5, "0"))
"#;
		for target in [Target::Native, Target::Wasm] {
			let ir = complete_ir_at_target(
				&format!("units-no-graphemes-{}", target.name()),
				source,
				false,
				OptLevel::O3,
				target,
			);
			assert!(
				!ir.contains("@fa_graphemes("),
				"grapheme count linked on {target:?}"
			);
			assert!(
				!ir.contains("@next_boundary("),
				"segmentation linked on {target:?}"
			);
			assert!(
				!ir.contains("@fa_next_grapheme("),
				"segmentation wrapper linked on {target:?}"
			);
			assert!(!ir.contains("@FA_GRAPHEME_RANGES"), "{ir}");
		}
	}

	#[test]
	fn explicit_characters_links_unicode_grapheme_properties() {
		for target in [Target::Native, Target::Wasm] {
			let ir = complete_ir_at_target(
				&format!("explicit-graphemes-{}", target.name()),
				"let text = arguments.get(0) ?? \"é😀\"\nconsole.log(text.characters())\n",
				false,
				OptLevel::O3,
				target,
			);
			assert!(ir.contains("@FA_GRAPHEME_RANGES"), "{ir}");
		}
	}

	#[test]
	fn recursive_optionals_have_no_separate_presence_word() {
		let ir = ir_of(
			"nullable-edges",
			"type Node = { value: Integer, next: Node? }\nlet list = Node { value = 1, next = Node { value = 2 } }\nmutable copy = list\ncopy.next = none\nconsole.log(list.next?.value ?? 0, copy.next?.value ?? 0)\n",
		);
		assert!(!ir.contains("{ i1, ptr }"), "{ir}");
		assert!(ir.contains("{ ptr, i64 }"), "{ir}");
		assert!(
			!ir.contains("@fa_arena_alloc("),
			"mutable edges use general ownership: {ir}"
		);
	}

	#[test]
	fn recursive_collection_items_need_no_extra_box() {
		let ir = ir_of(
			"inline-recursive-items",
			"type Node = { value: Integer, children: Array<Node> }\nfunction build = (depth: Integer): Node => { mutable node = Node { value = depth, children = [] }\nif depth > 0 { node.children.push(build(depth - 1)) }\nnode }\nconsole.log(build(3))\n",
		);
		let build = function(&ir, "fa.build");
		assert!(build.contains("store { ptr, i64 }"), "{build}");
		assert!(!build.contains("@fa_alloc("), "{build}");
		assert!(!build.contains("@fa_arena_alloc("), "{build}");
	}

	#[test]
	fn collection_slots_inline_objects_with_other_direct_recursive_edges() {
		let ir = ir_of(
			"mixed-recursive-items",
			"type Node = { value: Integer, next: Node?, children: Array<Node> }\nfunction append = (node: Node, child: Node): Void mutates node => { node.children.push(child) }\nmutable root = Node { value = 0, children = [] }\nappend(root, Node { value = 1, children = [] })\nconsole.log(root.children.length)\n",
		);
		let append = function(&ir, "fa.append");
		assert!(append.contains("store { ptr, ptr, i64 }"), "{append}");
		assert!(!append.contains("@fa_alloc("), "{append}");
	}

	#[test]
	fn collection_callbacks_borrow_recursive_items_without_boxes() {
		let ir = ir_of(
			"borrow-recursive-callback-items",
			"type Node = { value: Integer, next: Node?, children: Array<Node> }\nfunction arrayHas = (items: Array<Node>): Boolean => items.some((item) => item.value == 1)\nfunction mapHas = (items: Map<Integer, Node>): Boolean => items.some((key, value) => value.value == 1)\nfunction setHas = (items: Set<Node>): Boolean => items.some((item) => item.value == 1)\nfunction poolHas = (items: Pool<Node>): Boolean => items.find((item) => item.value == 1) is not none\nlet nodes: Array<Node> = []\nlet mapped: Map<Integer, Node> = []\nlet keys: Set<Node> = []\nlet pooled: Pool<Node> = []\nconsole.log(arrayHas(nodes), mapHas(mapped), setHas(keys), poolHas(pooled))\n",
		);
		for name in ["fa.arrayHas", "fa.mapHas", "fa.setHas", "fa.poolHas"] {
			let body = function(&ir, name);
			assert!(!body.contains("@fa_alloc("), "{body}");
			assert!(!body.contains("@fa_arena_alloc("), "{body}");
			assert!(!body.contains("@fa.copy"), "{body}");
		}
	}

	#[test]
	fn recursive_collection_aliases_reuse_the_existing_container() {
		let ir = ir_of(
			"recursive-container-aliases",
			"type NestedMap = Map<Integer, NestedMap>\nmutable maps: NestedMap = []\nlet emptyMap: NestedMap = []\nmaps[0] = emptyMap\ntype NestedGrowing = GrowingArray<NestedGrowing>\nmutable growing: NestedGrowing = []\nlet emptyGrowing: NestedGrowing = []\ngrowing.push(emptyGrowing)\nconsole.log(maps, growing)\n",
		);
		let main = function(&ir, "main");
		assert!(!main.contains("@fa_alloc("), "{main}");
		assert!(!main.contains("@fa_arena_alloc("), "{main}");
	}

	#[test]
	fn optional_owning_payloads_are_compared_only_when_present() {
		let ir = ir_of(
			"optional-owning-equality",
			"type Payload = { values: Array<Integer> }\nfunction equal = (a: Payload?, b: Payload?): Boolean => a == b\nconsole.log(equal(none, none), equal(none, Payload { values = [] }))\n",
		);
		let equal = function(&ir, "fa.equal");
		let (presence, payload) = equal
			.split_once("compare_optional_values:")
			.expect("guarded payload equality");
		let header = "getelementptr inbounds { ptr, i64, i64 }";
		assert!(!presence.contains(header), "{equal}");
		assert!(payload.contains(header), "{equal}");
	}

	#[test]
	fn array_iteration_reads_its_storage_before_the_loop() {
		let ir = ir_of(
			"iteration-storage",
			"type Item = { value: Integer }\nfunction change = (item: Item) mutates item => { item.value += 1 }\nfunction all = (items: Array<Item>) mutates items => { for item in items { change(item) } }\nmutable items = [Item { value = 1 }, Item { value = 2 }]\nall(items)\nconsole.log(items)\n",
		);
		let all = function(&ir, "fa.all");
		let (setup, body) = all.split_once("for_item:").expect("an iteration");
		let header = "getelementptr inbounds { ptr, i64, i64 }";
		assert_eq!(count(setup, header), 2, "{all}");
		assert_eq!(count(body, header), 0, "{all}");
	}

	#[test]
	fn empty_array_fields_reserve_before_the_loop_without_omitting_push_checks() {
		let ir = ir_of(
			"reserve-array-fields",
			"type Bucket = { items: Array<Integer> }\nfunction fill = (count: Integer): Bucket => { mutable bucket = Bucket { items = [] }\nfor index in 0..count { bucket.items.push(index) }\nbucket }\nconsole.log(fill(9))\n",
		);
		let fill = function(&ir, "fa.fill");
		let reserve = fill.find("@fa_array_room(").expect("a reservation");
		let body = fill.find("for_body:").expect("a counted loop");
		assert!(reserve < body, "{fill}");
		assert!(fill[body..].contains("@fa_array_grow("), "{fill}");
	}

	/// A program with the loops that `lower/copies.rs` turns into block
	/// copies, fills and searches.
	const COPY_LOOPS: &str = "function copyAll = (target: Array<Byte>, source: Array<Byte>) mutates target => {\n\tfor byte in source {\n\t\ttarget.push(byte)\n\t}\n}\nfunction copyRange = (target: Array<Byte>, source: Array<Byte>, start: Integer, end: Integer) throws IndexError mutates target => {\n\tfor position in start..end {\n\t\ttarget.push(try source[position])\n\t}\n}\nfunction pad = (target: Array<Byte>, count: Integer) mutates target => {\n\tfor _ in 0..count {\n\t\ttarget.push(48)\n\t}\n}\nfunction count = (text: Array<Byte>): Integer => {\n\tmutable found = 0\n\tfor byte in text {\n\t\tif byte == 59 {\n\t\t\tfound += 1\n\t\t}\n\t}\n\tfound\n}\nmutable target: Array<Byte> = []\nlet source: Array<Byte> = [1, 59, 3]\ncopyAll(target, source)\ntry copyRange(target, source, 0, 2)\npad(target, arguments.length)\nconsole.log(target, count(target))\n";

	#[test]
	fn a_loop_pushing_the_items_of_another_array_is_one_block_copy() {
		let ir = ir_of("copy-loops-items", COPY_LOOPS);
		let all = function(&ir, "fa.copyAll");
		assert_eq!(count(all, "@fa_array_extend("), 1, "{all}");
		assert!(!all.contains("@fa_array_grow("), "{all}");
		assert!(!all.contains("for_item_body"), "{all}");
	}

	#[test]
	fn a_range_copy_checks_its_bounds_once_and_keeps_the_rounds_for_the_errors() {
		let ir = ir_of("copy-loops-range", COPY_LOOPS);
		let range = function(&ir, "fa.copyRange");
		let (check, rest) = range.split_once("copy_range:").expect("a hoisted check");
		assert!(check.contains("icmp sle"), "{range}");
		assert!(!check.contains("@fa_error_index("), "{range}");
		let (copy, rounds) = rest.split_once("copy_range_rounds:").expect("the rounds");
		assert_eq!(count(copy, "@fa_array_extend("), 1, "{range}");
		assert!(rounds.contains("@fa_error_index("), "{range}");
		assert!(rounds.contains("@fa_array_grow("), "{range}");
	}

	#[test]
	fn a_loop_pushing_one_byte_is_a_fill_and_a_byte_test_a_search() {
		let ir = ir_of("copy-loops-fill", COPY_LOOPS);
		let pad = function(&ir, "fa.pad");
		assert_eq!(count(pad, "@fa_array_fill("), 1, "{pad}");
		assert!(!pad.contains("@fa_array_grow("), "{pad}");
		let search = function(&ir, "fa.count");
		assert_eq!(count(search, "@fa_bytes_next("), 1, "{search}");
	}

	#[test]
	fn consecutive_pushes_keep_the_header_in_registers() {
		// only the cold paths that grow the array read it again (enough
		// pushes for `many` to stay a function of its own)
		let pushes: String = (0..24)
			.map(|index| format!("\ttarget.push({index})\n"))
			.collect();
		let source = format!(
			"function many = (target: Array<Byte>) mutates target => {{\n{pushes}}}\nmutable a: Array<Byte> = []\nmutable b: Array<Byte> = []\nmany(a)\nmany(b)\nmany(a)\nconsole.log(a, b)\n"
		);
		let ir = optimized_ir("consecutive-pushes", &source);
		let many = function(&ir, "fa.many");
		assert_eq!(count(many, "@fa_array_grow("), 24, "{many}");
		let hot: usize = many
			.split("\n\n")
			.filter(|block| !block.starts_with("push_grow"))
			.map(|block| count(block, " = load "))
			.sum();
		// the slot, the length, the capacity and the items, once
		assert_eq!(hot, 4, "{many}");
	}

	#[test]
	fn growing_lookup_has_no_index_dependent_branch_beyond_bounds() {
		let source = "function lookup = (items: GrowingArray<Integer>, index: Integer<min = 0>): Integer throws IndexError => try items[index]\nmutable items: GrowingArray<Integer> = []\nfor value in 0..1000 { items.push(value) }\nconsole.log(try lookup(items, 7), try lookup(items, 900))\n";
		for target in [Target::Native, Target::NativeFreestanding, Target::Wasm] {
			let ir = ir_at_target(
				&format!("growing-branchless-{}", target.name()),
				source,
				false,
				OptLevel::O0,
				target,
			);
			let lookup = function(&ir, "fa.lookup");
			// Exactly the bounds guard and the index-independent segment-count
			// guard. The lookup itself contains no branch and no select.
			assert_eq!(count(lookup, "br i1"), 2, "{lookup}");
			assert_eq!(count(lookup, "icmp "), 2, "{lookup}");
			assert_eq!(count(lookup, "@llvm.ctlz.i64("), 1, "{lookup}");
			assert!(lookup.contains("segment_code = or i64"), "{lookup}");
			assert!(!lookup.contains("select i1"), "{lookup}");
			assert!(lookup.contains("item_address = add"), "{lookup}");
			assert!(lookup.contains("item_pointer = inttoptr"), "{lookup}");
			assert!(!lookup.contains("segment_start"), "{lookup}");
		}
	}

	#[test]
	fn growing_default_k_requires_a_closed_empty_origin() {
		let prefix = "mutable items: GrowingArray<Integer> = []\nfor value in 0..40 { items.push(value) }\nlet index = try Integer.parse(arguments.get(0) ?? \"9\")\n";
		let plain = ir_of(
			"growing-default-origin",
			&format!("{prefix}console.log(try items[index])\n"),
		);
		assert!(!plain.contains("segment_k"), "{plain}");
		for (name, change) in [
			("reserve", "items.reserve(1000)"),
			(
				"replace",
				"items = GrowingArray<Integer>.from([1, 2], capacity = 100)",
			),
			("copy", "let other = items\nconsole.log(other)"),
		] {
			let ir = ir_of(
				&format!("growing-default-reject-{name}"),
				&format!("{prefix}{change}\nconsole.log(try items[index])\n"),
			);
			assert!(ir.contains("segment_k"), "{name}: {ir}");
		}
	}

	#[test]
	fn growing_index_loops_load_the_first_segment_before_the_loop() {
		let ir = optimized_ir(
			"growing-index-loop",
			"function write = (items: GrowingArray<Byte>, count: Integer) throws IndexError mutates items => {\nmutable position = count - 1\nwhile position >= 0 {\ntry items[position] = 7\nposition -= 1\n}\n}\nmutable a: GrowingArray<Byte> = [1, 2, 3]\nmutable b: GrowingArray<Byte> = [1, 2, 3, 4]\ntry write(a, arguments.length)\ntry write(b, arguments.length)\nconsole.log(a, b)\n",
		);
		assert!(ir.contains("contiguous"), "{ir}");
		assert!(ir.contains("first_items"), "{ir}");
		for block in ir.split("\n\n") {
			let label = block.lines().next().unwrap_or("");
			if label.starts_with("while_body") && !label.contains(".lr.ph") {
				assert!(
					!block.contains("first_items") || !block.contains(" = load ptr"),
					"{block}"
				);
			}
		}
	}

	#[test]
	fn growing_pushes_advance_a_cursor_in_registers() {
		// a push onto a `GrowingArray` compares its cursor with the end of the
		// segment, stores and advances; only the cold path finds a segment
		// (enough pushes for `many` to stay a function of its own)
		let pushes: String = (0..24)
			.map(|index| format!("\ttarget.push({index})\n"))
			.collect();
		let source = format!(
			"function many = (target: GrowingArray<Byte>) mutates target => {{\n{pushes}}}\nmutable a: GrowingArray<Byte> = []\nmutable b: GrowingArray<Byte> = []\nmany(a)\nmany(b)\nmany(a)\nconsole.log(a, b)\n"
		);
		let ir = optimized_ir("growing-pushes", &source);
		let many = function(&ir, "fa.many");
		assert_eq!(count(many, "@fa_segmented_next("), 24, "{many}");
		assert!(!many.contains("@llvm.ctlz"), "{many}");
		let hot: usize = many
			.split("\n\n")
			.filter(|block| !block.starts_with("push_next_segment"))
			.map(|block| count(block, " = load "))
			.sum();
		// the slot, the cursor, the end and the length, once
		assert_eq!(hot, 4, "{many}");
	}

	#[test]
	fn float_to_integer_classifies_errors_only_on_the_cold_path() {
		let ir = ir_of(
			"float-conversion-guard",
			"function convert = (value: Float32): Integer throws ArithmeticError => try Integer.truncate(value)\nconsole.log(try convert(3.5))\n",
		);
		let convert = function(&ir, "fa.convert");
		assert_eq!(count(convert, "@fa_error_float("), 1, "{convert}");
		let (hot, cold) = convert.split_once("check_failed:").expect("a cold guard");
		assert!(!hot.contains("@llvm.fabs"), "{convert}");
		assert!(cold.contains("@llvm.fabs"), "{convert}");
	}

	#[test]
	fn directly_returned_passive_locals_move_on_each_exit() {
		let ir = ir_of(
			"return-local",
			"function build = (early: Boolean): Array<Integer> => { mutable items = [1, 2]\nif early { return (items) }\nitems.push(3)\nitems }\nconsole.log(build(true), build(false))\n",
		);
		let build = function(&ir, "fa.build");
		assert!(!build.contains("@fa.copy"), "{build}");
	}

	#[test]
	fn references_and_captures_keep_early_returns_on_the_copy_path() {
		let ir = ir_of(
			"return-aliased-local",
			include_str!("programs/return-local-ownership.fa"),
		);
		for name in ["fa.referenced", "fa.captured"] {
			let body = function(&ir, name);
			assert!(body.contains("@fa.copy"), "{body}");
		}
	}

	#[test]
	fn immutable_global_literals_are_folded_in_functions() {
		let ir = optimized_ir(
			"constant-global",
			"let letters = \"abc\"\nfunction letter = (i: Integer): String throws IndexError => try letters[i]\nconsole.log(try letter(arguments.length))\n",
		);
		assert!(!ir.contains("@fa.global"), "{ir}");
		assert!(ir.contains("i64 3"), "{ir}");
	}

	#[test]
	fn an_immutable_literal_arrays_length_needs_no_header_read() {
		let ir = ir_of(
			"constant-array-length",
			"let values = [1, 2]\nfunction size = (depth: Integer): Integer => if depth > 0 { size(depth - 1) } else { values.length }\nconsole.log(size(2))\n",
		);
		let size = function(&ir, "fa.size");
		assert!(size.contains("store i64 2"), "{size}");
		assert!(
			!size.contains("getelementptr inbounds { ptr, i64, i64 }"),
			"{size}"
		);
	}

	#[test]
	fn large_readonly_parameters_use_the_callers_storage() {
		let ir = ir_of(
			"borrowed-aggregate",
			"type Data = { a: Integer, b: Integer, c: Integer, d: Integer }\nfunction read = (data: Data, depth: Integer): Integer => if depth > 0 { read(data, depth - 1) } else { data.a + data.d }\nconsole.log(read(Data { a = 1, b = 2, c = 3, d = 4 }, 2))\n",
		);
		let read = function(&ir, "fa.read");
		assert!(
			read.lines().next().expect("a signature").contains("(ptr "),
			"{read}"
		);
		assert!(!read.contains("alloca { i64, i64, i64, i64 }"), "{read}");
	}

	#[test]
	fn small_nonrecursive_unions_store_their_payload_inline() {
		let ir = ir_of(
			"inline-union",
			"type Small = Union { Empty, Flag = { flag: Boolean, value: Int16 }, Text = { text: String } }\nfunction make = (value: Int16): Small => Small.Flag { flag = value > 0, value }\nconsole.log(make(2))\n",
		);
		let make = function(&ir, "fa.make");
		assert!(
			make.starts_with("define internal fastcc { i64, [3 x i64] }"),
			"{make}"
		);
		assert!(!make.contains("@fa_alloc("), "{make}");
		assert!(!make.contains("@fa_heap_alloc("), "{make}");
		assert!(!make.contains("@fa.copy_box("), "{make}");
		// Boolean padding is explicitly initialized before loading whole words.
		assert!(make.contains("store i8"), "{make}");
	}

	#[test]
	fn a_bare_inline_member_boxed_for_return_has_static_storage() {
		let ir = ir_of(
			"inline-union-static-box",
			"type Choice = Union { Empty, Text = String }\ntype Any = Choice | Integer\nfunction escaped = (n: Integer): Any => if n > 0 { escaped(n - 1) } else { Choice.Empty }\nconsole.log(escaped(2))\n",
		);
		let escaped = function(&ir, "fa.escaped");
		assert!(escaped.contains("@fa.member"), "{escaped}");
	}

	#[test]
	fn integer_keys_are_compared_bitwise_without_a_comparator() {
		let ir = ir_of(
			"injective-integer-hash",
			"mutable items: Map<Integer, Integer> = [(1, 2)]\nitems[3] = 4\nconsole.log(items.get(1), items.get(3), items.get(5))\n",
		);
		assert!(!ir.contains("@fa.key_equals("), "{ir}");
		assert!(ir.contains("@fa_table_find("), "{ir}");
	}

	#[test]
	fn single_byte_string_comparisons_do_not_call_the_general_byte_loop() {
		let ir = optimized_ir(
			"single-byte-equality",
			"let word = arguments.get(0) ?? \"\"\nconsole.log(word == \" \", word != \" \", \" \" == word)\n",
		);
		assert!(!ir.contains("@string_equal_bytes("), "{ir}");
		assert!(!ir.contains("@fa_string_equal("), "{ir}");
		assert!(!ir.contains("@fa_string_equal_flags("), "{ir}");
		assert!(ir.contains("load i8"), "{ir}");
	}

	#[test]
	fn recursive_unions_with_passive_owned_payloads_use_arenas() {
		let ir = ir_of(
			"passive-arenas",
			include_str!("programs/passive-tree-arenas.fa"),
		);
		let build = function(&ir, "fa.build.arena");
		assert!(build.contains("@fa_arena_alloc("), "{build}");
		assert!(build.contains("@fa_arena_cleanup("), "{build}");
		assert!(ir.contains("@fa.arena.payloads."), "{ir}");
	}

	#[test]
	fn recursive_unions_with_members_without_data_use_arenas() {
		let ir = ir_of(
			"constant-member-arenas",
			include_str!("programs/arena-constant-members.fa"),
		);
		assert!(
			function(&ir, "fa.build.arena").contains("@fa_arena_alloc("),
			"{ir}"
		);
		assert!(
			function(&ir, "fa.grow.arena").contains("@fa_arena_alloc"),
			"{ir}"
		);
	}

	#[test]
	fn mutations_through_a_payload_alias_disable_tree_arenas() {
		let source = "type Label = { text: String }\ntype Tree = Union { Leaf = { label: Label }, Branch = { left: Tree, right: Tree } }\nfunction build = (n: Integer): Tree => if n == 0 { Tree.Leaf { label = { text = \"short\" } } } else { Tree.Branch { left = build(n - 1), right = build(n - 1) } }\nfunction change = (label: Label) mutates label => { label.text += \"a long appended value that needs a heap allocation\" }\nmutable tree = build(0)\nif tree is Tree.Leaf { use alias = tree.label\nchange(alias)\nconsole.log(tree.label.text) }\n";
		let ir = ir_of("mutated-arena-payload", source);
		assert!(!ir.contains("@fa.build.arena"), "{ir}");
	}

	#[test]
	fn a_recursive_field_is_boxed_before_its_value_is_computed() {
		// a tree built recursively is laid out in the order of a walk
		let ir = ir_of(
			"early-box",
			"type Node = {\n\tvalue: Integer\n\tleft: Node?\n\n\tfunction build = (depth: Integer): Node => if depth > 0 {\n\t\tNode { value = depth, left = Node.build(depth - 1) }\n\t} else {\n\t\tNode { value = 0 }\n\t}\n}\nconsole.log(Node.build(3).value)\n",
		);
		let build = function(&ir, "fa.build.arena")
			.split_once('\n')
			.expect("a body")
			.1;
		let allocation = build.find("@fa_arena_alloc(").expect("an arena allocation");
		let recursion = build.find("@fa.build.arena(").expect("a recursive call");
		assert!(allocation < recursion, "{build}");
		assert!(!build.contains("return.result"), "{build}");
		assert!(!build.contains("@fa_alloc("), "{build}");
		assert!(!build.contains("@fa_arena_enter("), "{build}");
	}

	/// The whole IR of a program (not only its functions).
	fn full_ir(name: &str, source: &str) -> Result<String, fa_codegen_llvm::Error> {
		let mut session = Session::new();
		let file = session.set_source("main", Arc::from(source));
		session.analyze_file(file);
		let options = Options {
			emit_ir: true,
			opt_level: OptLevel::O0,
			..Options::default()
		};
		let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join(name);
		let artifact = compile(&session, file, "test.fa", Target::Native, &options, &output)?;
		Ok(std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file"))
	}

	#[test]
	fn a_generic_function_value_is_a_table_of_the_instances_its_type_is_called_at() {
		// `transform` is called at two types: the lambda's table holds its two
		// instances, and the value is the usual pair (no box)
		let ir = full_ir(
			"generic-values",
			"function describe = (transform: <T>(value: T) => T): String => \"{transform(1)} {transform(\"a\")}\"\nconsole.log(describe(<T>(value: T) => value))\n",
		)
		.expect("compiled");
		let table = ir
			.lines()
			.find(|line| line.contains("@fa.generic_table.0.entries ="))
			.expect("a table");
		assert!(table.contains("[2 x ptr]"), "{table}");
		assert!(table.contains("constant"), "{table}");
	}

	#[test]
	fn a_generic_function_type_that_names_itself_is_not_written() {
		// a generic function type is written as the type of a parameter
		// only (`GN030`): the type of a function that takes itself has no
		// name, so no class of instances grows without end (the polymorphic
		// recursion that `generic_values.rs` reports)
		let error = full_ir(
			"generic-values-growth",
			"type Grow = <T>(value: T, depth: Integer, again: Grow) => Integer\nfunction grow: Grow = <T>(value: T, depth: Integer, again: Grow): Integer => {\n\tif depth == 0 {\n\t\treturn 0\n\t}\n\tagain([value], depth - 1, again)\n}\nconsole.log(grow(1, 3, grow))\n",
		)
		.expect_err("a generic function type outside a parameter");
		let message = error.to_string();
		assert!(message.contains("test.fa:1:13: error[GN030]"), "{message}");
	}

	#[test]
	fn disjoint_array_metadata_needs_a_non_wrapping_offset() {
		let source = |offset: &str| {
			let (throws, attempt) = if offset == "1" {
				("", "")
			} else {
				(" throws IndexError", "try ")
			};
			format!(
				"type Body = {{ value: Integer }}\nfunction interact = (items: Array<Body>): Void{throws} mutates items => {{\nfor body, i in items {{\nfor j in i + {offset}..items.length {{\n{attempt}use other = items[j]\nbody.value += other.value\nother.value += body.value\n}}\n}}\n}}\n{attempt}interact([{{ value = 1 }}, {{ value = 2 }}, {{ value = 3 }}])\n"
			)
		};
		let small = ir_with("disjoint-small-offset", &source("1"), false);
		let small = function(&small, "fa.interact");
		assert!(
			small.contains("!alias.scope") && small.contains("!noalias"),
			"{small}"
		);
		let large = ir_with(
			"disjoint-wrapping-offset",
			&source("9223372036854775807"),
			false,
		);
		let large = function(&large, "fa.interact");
		assert!(
			!large.contains("!alias.scope") && !large.contains("!noalias"),
			"{large}"
		);
	}
	#[test]
	fn int32_maps_store_only_key_and_value_in_probe_slots() {
		let source = |key: &str| {
			format!(
				"mutable values: Map<{key}, UInt32> = [(0, 42)]\nvalues[1] = 7\nconsole.log(values.get(1))\n"
			)
		};
		let compact = ir_with("compact-key32", &source("Int32"), false);
		assert!(
			compact.contains("i64 6597069766656"),
			"compact layout flag missing"
		);
		let narrow = ir_with("legacy-key16", &source("Int16"), false);
		assert!(
			!narrow.contains("i64 6597069766656"),
			"only a 32-bit key occupies its whole presence word"
		);
		assert!(
			narrow.contains("i64 2199023255552"),
			"legacy inline layout flag missing"
		);
	}

	#[test]
	fn dynamic_append_loops_reserve_inline_values_too() {
		let ir = ir_with(
			"inline-dynamic-loop",
			"function repeat = (count: Integer): String => {\nmutable text = \"\"\nfor 0..count { text += \"x\" }\ntext\n}\nconsole.log(repeat(7))\n",
			false,
		);
		let body = function(&ir, "fa.repeat");
		assert!(body.contains("@fa_string_reserve_inline("), "{body}");
		assert!(body.contains("reserved_inline"), "{body}");
	}
	#[test]
	fn non_escaping_strings_have_independent_frame_buffers() {
		let ir = ir_of(
			"frame-strings",
			"function check = (prefix: String) => {\n mutable text = \"\"\n text += prefix\n let copy = text\n text = \"changed\"\n console.log(copy, text)\n}\ncheck(\"123456789012345678901234567890\")\n",
		);
		assert!(ir.contains("alloca [64 x i8]"), "{ir}");
		assert!(ir.contains("@fa_string_assign_local("), "{ir}");
	}

	#[test]
	fn small_literal_mutables_keep_the_inline_representation() {
		let ir = ir_of(
			"small-inline-mutables",
			"function check = () => {\n mutable text = \"first\"\n console.log(text)\n text = \"second\"\n console.log(text)\n}\ncheck()\n",
		);
		assert!(!ir.contains("alloca [64 x i8]"), "{ir}");
	}
	#[test]
	fn recursive_union_with_a_payload_larger_than_an_arena_chunk_stays_general() {
		let mut source = String::from("type Pad0 = Integer\n");
		for level in 1..=13 {
			source.push_str(&format!(
				"type Pad{level} = (Pad{}, Pad{})\n",
				level - 1,
				level - 1
			));
		}
		source.push_str("type Tree = Union { Leaf = { pad: Pad13 }, Branch = { left: Tree, right: Tree } }\nfunction build = (depth: Integer): Tree => {\nif depth > 0 { return Tree.Branch { left = build(depth - 1), right = build(depth - 1) } }\nlet pad0 = depth\n");
		for level in 1..=13 {
			source.push_str(&format!(
				"let pad{level} = (pad{}, pad{})\n",
				level - 1,
				level - 1
			));
		}
		source.push_str("Tree.Leaf { pad = pad13 }\n}\nfunction count = (tree: Tree): Integer => when tree is {\nTree.Leaf => 1\nTree.Branch as { left, right } => count(left) + count(right)\n}\nconsole.log(count(build(1)))\n");
		let ir = ir_of("region-large-payload", &source);
		assert!(!ir.contains("@fa_arena_enter("), "{ir}");
	}

	#[test]
	fn immutable_direct_union_edges_use_the_same_link_domain_as_arrays() {
		let ir = ir_with(
			"direct-union-edges",
			include_str!("programs/region-direct-edges.fa"),
			false,
		);
		assert!(
			ir.contains("@fa_arena_alloc_near(i64 6, i64 12")
				&& ir.contains("getelementptr inbounds [3 x i16]")
				&& ir.contains("getelementptr inbounds [3 x i32]"),
			"a pair selects between two i16 and two i32 links: {ir}"
		);
		assert!(
			ir.contains("getelementptr ([2 x i32], ptr null, i32 1)"),
			"a unary node stores one link: {ir}"
		);
		assert!(
			ir.contains("link") && ir.contains("i32"),
			"link encode/decode stays typed: {ir}"
		);
		assert!(
			ir.lines()
				.any(|line| line.contains("member_tag") && line.contains("load i8")),
			"a small, statically known root needs only its distinct low tag byte: {ir}"
		);
	}

	#[test]
	fn roots_with_colliding_low_tag_bytes_keep_full_discriminants() {
		let mut source = String::from(
			"type T = Union {\nLeaf = { value: Int32 }\nPair = { left: T, right: T }\n",
		);
		for index in 0..256 {
			source.push_str(&format!("M{index}\n"));
		}
		source.push_str(
			"}\nfunction build = (depth: Integer): T => if depth > 0 { T.Pair { left = build(depth - 1), right = build(0) } } else { T.Leaf { value = 7 } }\nfunction isPair = (node: T): Boolean => node is T.Pair\nassert not isPair(T.M255)\nconsole.log(isPair(build(1)))\n",
		);
		let ir = ir_with("wide-root-tags", &source, false);
		assert!(
			ir.lines()
				.any(|line| line.contains("member_tag") && line.contains("load i16")),
			"more than 256 members must not compare only their low byte: {ir}"
		);
		assert!(
			ir.contains("and i16") && ir.contains("-16385"),
			"the width flag is not part of the member id: {ir}"
		);
		let binary = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join("wide-root-tags");
		let output = std::process::Command::new(binary)
			.output()
			.expect("the regression program runs");
		assert!(
			output.status.success(),
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);
		assert_eq!(output.stdout, b"true\n");
	}

	#[test]
	fn far_relative_edges_keep_full_tags_in_large_member_families() {
		let mut source = String::from(
			"type T = Union {\nLeaf = { value: Int32 }\nPair = { left: T, right: T }\n",
		);
		for index in 0..256 {
			source.push_str(&format!("M{index} = {{ value{index}: Int32 }}\n"));
		}
		source.push_str("}\nfunction build = (depth: Integer): T => if depth == 0 { T.Leaf { value = 7 } } else { T.Pair { left = build(depth - 1), right = build(depth - 1) } }\nfunction isPair = (node: T): Boolean => node is T.Pair\nfunction sum = (node: T): Integer => when node is {\nT.Leaf as { value } => value\nT.Pair as { left, right } => sum(left) + sum(right)\nelse => 0\n}\nlet tree = build(16)\nassert isPair(tree)\nassert sum(tree) == 458752\nconsole.log(\"wide tags\")\n");
		let ir = ir_with("wide-relative-tags", &source, false);
		assert!(ir.contains("@fa_arena_alloc_near"), "{ir}");
		assert!(ir.contains("and i16") && ir.contains("-16385"), "{ir}");
		let binary = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join("wide-relative-tags");
		let output = std::process::Command::new(binary)
			.output()
			.expect("the wide regression runs");
		assert!(
			output.status.success(),
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);
		assert_eq!(output.stdout, b"wide tags\n");
	}

	#[test]
	fn mixed_root_edges_keep_scalar_alignment_and_copy_the_links() {
		let ir = ir_with(
			"mixed-root-edges",
			include_str!("programs/region-mixed-edges.fa"),
			false,
		);
		assert!(
			ir.contains("{ i32, i32, i32, i8, i32, double, i128 }"),
			"{ir}"
		);
		assert!(ir.contains("load i128") && ir.contains("align 16"), "{ir}");
		assert!(
			ir.contains("getelementptr") && ir.contains("fa.copy"),
			"{ir}"
		);
		let small = ir_with(
			"small-mixed-root-edges",
			r#"type T = Union { Leaf = { value: Int32 }, Pair = { left: T, marker: Byte, right: T } }
function build = (depth: Integer): T => if depth == 0 {
 T.Leaf { value = 7 }
} else {
 T.Pair { left = build(depth - 1), marker = 231, right = build(depth - 1) }
}
console.log(build(3))
"#,
			false,
		);
		assert!(small.contains("{ i32, i32, i8, i32 }"), "{small}");
		assert!(!small.contains("@fa_arena_alloc_near"), "{small}");
	}

	#[test]
	fn a_scalar_union_leaf_preserves_the_throwing_result_abi() {
		let ir = ir_with(
			"throwing-union-leaves",
			include_str!("programs/throwing-union-leaves.fa"),
			false,
		);
		assert!(ir.contains("union_leaf"), "{ir}");
		assert!(ir.contains("union_base_or_call = phi { i64, ptr }"), "{ir}");
		assert!(
			ir.contains("insertvalue { i64, ptr } zeroinitializer"),
			"{ir}"
		);
	}

	#[test]
	fn throwing_union_leaves_keep_effects_and_checked_return_ranges_in_calls() {
		let effectful = ir_with(
			"effectful-union-leaf",
			r#"type E = Errors { Bad }
type T = Union { Leaf = { value: Int32 }, One = { child: T } }
mutable count = 0
function effect = (value: Integer): Integer mutates count => {
 count += 1
 value
}
function read = (node: T): Integer throws E mutates count => when node is {
 T.Leaf as { value } => effect(value)
 T.One as { child } => {
  if count < 0 { throw E.Bad }
  try read(child)
 }
}
console.log(try read(T.One { child = T.Leaf { value = 7 } }), count)
"#,
			false,
		);
		assert!(!effectful.contains("union_leaf"), "{effectful}");
		let ranged = ir_of(
			"ranged-union-leaf",
			r#"type E = Errors { Bad }
type T = Union { Leaf = { value: Int32 }, One = { child: T } }
function read = (node: T, fail: Boolean): Integer<min = -10, max = 10> throws E => when node is {
 T.Leaf => 0
 T.One as { child } => {
  if fail { throw E.Bad }
  try read(child, false)
 }
}
console.log(try read(T.One { child = T.Leaf { value = 7 } }, false))
"#,
		);
		assert!(!ranged.contains("union_leaf"), "{ranged}");
	}

	#[test]
	fn recursive_union_arrays_use_region_buffer_cleanups() {
		let ir = ir_of(
			"region-child-arrays",
			"type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } }\nfunction build = (depth: Integer): Tree => if depth > 0 {\n mutable items: Array<Tree> = []\n items.push(build(depth - 1))\n Tree.List { items }\n} else { Tree.Leaf { value = 7 } }\nfunction count = (tree: Tree): Integer => when tree is {\nTree.Leaf as { value } => value\nTree.List as { items } => {\n mutable total = 0\n for child in items { total += count(child) }\n total\n}\n}\nlet roots: Array<Tree> = [build(1), build(2)]\nfor root in roots { console.log(count(root)) }\n",
		);
		assert!(ir.contains("@fa_arena_enter("), "{ir}");
		assert!(ir.contains("@fa_array_init_region_local("), "{ir}");
		assert!(ir.contains("@fa_array_into_region("), "{ir}");
		assert!(
			ir.contains("freeze_children") && ir.contains("children_view"),
			"{ir}"
		);
		assert!(
			ir.contains("getelementptr inbounds i32, ptr %items"),
			"{ir}"
		);
	}

	#[test]
	fn region_string_slices_only_borrow_stable_immediate_sources() {
		let prefix =
			"type Tree = Union { Leaf = { text: String }, Branch = { left: Tree, right: Tree } }\n";
		let suffix = "\nfunction count = (tree: Tree): Integer => when tree is {\nTree.Leaf as { text } => text.length\nTree.Branch as { left, right } => count(left) + count(right)\n}\nconsole.log(count(build(\"hello\".repeat(10))))\n";
		for (name, expression, extra, borrowed) in [
			("stable", "text[0..4]", "", true),
			("temporary", "\"hello\".repeat(10)[0..4]", "", false),
			("temporary-slice", "text[0..30][0..4]", "", false),
			(
				"called-bound",
				"text[start()..4]",
				"function start = (): Integer => 0\n",
				false,
			),
		] {
			let source = format!(
				"{prefix}{extra}function build = (text: String): Tree => Tree.Leaf {{ text = {expression} catch error => \"\" }}{suffix}"
			);
			let ir = ir_of(&format!("region-string-slice-{name}"), &source);
			assert!(
				ir.contains("freeze_text") && ir.contains("text_view"),
				"{ir}"
			);
			assert_eq!(ir.contains("@fa_string_slice_region("), borrowed, "{ir}");
		}
	}

	#[test]
	fn compact_region_tags_fall_back_when_the_program_has_too_many_members() {
		let source = "type Tree = Union { Leaf = { value: Int32 }, Branch = { left: Tree, right: Tree } }\nfunction build = (value: Int32): Tree => Tree.Leaf { value }\nfunction read = (tree: Tree): Int32 => when tree is {\nTree.Leaf as { value } => value\nTree.Branch as { left, right } => read(left) + read(right)\n}\nconsole.log(read(build(7)))\n";
		let compact = ir_of("compact-region-tags-small", source);
		assert!(
			compact.contains("store i16") && compact.contains("@fa_arena_enter("),
			"{compact}"
		);
		let mut large = String::from("type Noise = Enum {\n");
		for index in 0..32752 {
			large.push_str(&format!(" M{index}\n"));
		}
		large.push_str("}\n");
		large.push_str(source);
		let general = ir_of("compact-region-tags-large", &large);
		assert!(!general.contains("store i16"), "{general}");
		assert!(general.contains("@fa_arena_enter("), "{general}");
	}

	#[test]
	fn region_payloads_and_rejected_member_subsets_keep_separate_array_layouts() {
		let active = ir_of(
			"region-small-payloads-active",
			include_str!("programs/region-small-payloads.fa"),
		);
		assert!(
			active.contains("@fa_arena_enter(") && active.contains("store i16"),
			"{active}"
		);
		let ir = ir_of(
			"region-partial-members",
			"type Tree = Union { Number = { value: Int32 }, Text = { text: String }, List = { items: Array<Tree> } }\nfunction number = (value: Int32): Tree.Number => Tree.Number { value }\nfunction text = (value: String): Tree.Text => Tree.Text { text = value }\ntype Leaf = Tree.Number | Tree.Text\nmutable leaves: Array<Leaf> = []\nleaves.push(number(9))\nleaves.push(text(\"eleven\"))\nfor leaf in leaves { console.log(\"{leaf}\") }\n",
		);
		assert!(!ir.contains("@fa_arena_enter("), "{ir}");
		assert!(ir.contains("@fa_packed_new("), "{ir}");
	}

	#[test]
	fn pure_nonempty_split_filters_are_fused_and_other_callbacks_are_not() {
		let ir = ir_of(
			"split-nonempty",
			"function run = (text: String): Integer => {\nmutable source = text\nlet parts = source.split(\" \").filter((part) => not part.isEmpty())\nparts.length\n}\nconsole.log(run(\"a b\"))\n",
		);
		assert!(ir.contains("@fa_string_split_nonempty_views("), "{ir}");
		assert!(!ir.contains("filter_keep"), "{ir}");
		for (name, callback) in [
			("index", "(part, index) => index == 1"),
			("body", "(part) => { not part.isEmpty() }"),
			("capture", "(part) => not text.isEmpty()"),
		] {
			let ir = ir_of(
				&format!("split-filter-{name}"),
				&format!(
					"function run = (text: String): Integer => {{\nlet parts = text.split(\" \").filter({callback})\nparts.length\n}}\nconsole.log(run(\"a b\"))\n"
				),
			);
			assert!(!ir.contains("@fa_string_split_nonempty"), "{ir}");
			assert!(ir.contains("filter_keep"), "{ir}");
		}
	}
}

/// A resource moved on every path or on none has no drop flag left in the
/// optimized IR: only a move on some paths keeps one (`design/decisions.md`,
/// "A type holding a task is move-only").
#[cfg(fa_llvm)]
mod drop_flags {
	use std::path::PathBuf;
	use std::sync::Arc;

	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{OptLevel, Options, Target, compile};

	/// The optimized IR of `main` of a foreign file.
	fn foreign_main(name: &str, source: &str) -> String {
		let mut session = Session::new();
		let file = session.set_source("main.foreign", Arc::from(source));
		session.analyze_file(file);
		let options = Options {
			emit_ir: true,
			opt_level: OptLevel::O3,
			..Options::default()
		};
		let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-checks")
			.join(name);
		let artifact = compile(
			&session,
			file,
			"test.foreign.fa",
			Target::Native,
			&options,
			&output,
		)
		.unwrap_or_else(|error| panic!("{name}: {error}"));
		let ir = std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file");
		let start = ir.find(" @main(").expect("main");
		let start = ir[..start].rfind("define").expect("main");
		let end = ir[start..].find("\n}\n").expect("the end of main");
		ir[start..start + end].to_string()
	}

	#[test]
	fn only_a_move_on_some_paths_keeps_a_drop_flag() {
		// `destroy` calls `usleep` with the descriptor: one call per drop
		let head = "function release: (microseconds: UInt32) => Int32 = @extern(\"usleep\")\ntype Handle = {\n\tdescriptor: UInt32\n\tfunction destroy = (self) => {\n\t\trelease(self.descriptor)\n\t}\n}\nfunction close = (handle: Handle) => console.log(\"closing\")\n";
		let cases = [
			// moved on every path: destroyed by `close` only, no flag
			(
				"moved",
				"do {\n\tlet handle = Handle { descriptor = 1 }\n\tclose(handle)\n\tconsole.log(\"after\")\n}\n",
				1,
				false,
			),
			// never moved: destroyed at the end of its block, no flag
			(
				"kept",
				"do {\n\tlet handle = Handle { descriptor = 2 }\n\tconsole.log(\"after\")\n}\n",
				1,
				false,
			),
			// moved on one path: destroyed by `close` or at the end, by its flag
			(
				"maybe",
				"do {\n\tlet handle = Handle { descriptor = 3 }\n\tif arguments.length > 1 {\n\t\tclose(handle)\n\t}\n\tconsole.log(\"after\")\n}\n",
				2,
				true,
			),
			// a field moved out, then assigned again: no flag either
			(
				"field",
				"type Holder = { label: String, handle: Handle }\ndo {\n\tmutable holder = Holder { label = \"h\", handle = Handle { descriptor = 4 } }\n\tclose(holder.handle)\n\tholder.handle = Handle { descriptor = 5 }\n\tconsole.log(holder.label)\n}\n",
				2,
				false,
			),
		];
		for (name, body, drops, flagged) in cases {
			let ir = foreign_main(name, &format!("{head}{body}"));
			assert_eq!(
				ir.matches("call i32 @usleep(").count(),
				drops,
				"{name}:\n{ir}"
			);
			assert_eq!(ir.contains("label %drop_owned"), flagged, "{name}:\n{ir}");
		}
	}
}
