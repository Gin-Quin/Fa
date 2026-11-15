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
		let ir = std::fs::read_to_string(artifact.ir.expect("the IR")).expect("the IR file");
		// keep the program's own functions
		let mut kept = String::new();
		let mut keep = false;
		for line in ir.lines() {
			if line.starts_with("define ") {
				keep = line.contains("@main(") || line.contains("@fa.");
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

	fn count(ir: &str, pattern: &str) -> usize {
		ir.matches(pattern).count()
	}

	#[test]
	fn immutable_literals_use_readonly_data_for_every_collection() {
		for (kind, literal, measure) in [
			("Array<Integer>", "[2, 3, 5]", "length"),
			("GrowingArray<Integer>", "[2, 3, 5]", "length"),
			("Map<String, Integer>", "[(\"abcdefghijk\", 3)]", "size"),
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
		assert_eq!(count(fill, "@fa_array_reserve_additional("), 1, "{fill}");
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
		// (`irce`), and the sums of bytes are computed on 16-bit lanes
		let ir = optimized_ir(
			"blur",
			"function blur = (source: Array<UInt8>, destination: Array<UInt8>, width: Integer) throws IndexError mutates destination => {\n\tfor y in 1..width - 1 {\n\t\tfor x in 1..width - 1 {\n\t\t\tlet i = y * width + x\n\t\t\tlet sum: Integer = try source[i - width] + source[i - 1] * 2 + source[i] * 4 + source[i + 1] * 2 + source[i + width]\n\t\t\ttry destination[i] = sum // 16\n\t\t}\n\t}\n}\nlet width = arguments.length + 40\nmutable a = Array<UInt8>.from([], capacity = width * width)\nmutable b = Array<UInt8>.from([], capacity = width * width)\nfor index in 0..width * width {\n\ta.push(index %% 256)\n\tb.push(0)\n}\nfor 0..4 {\n\ttry blur(a, b, width)\n\ttry blur(b, a, width)\n}\nconsole.log(a.get(width + 1) ?? 0)\n",
		);
		let blur = function(&ir, "fa.blur");
		assert!(blur.contains("load <16 x i8>"), "{blur}");
		assert!(blur.contains("<16 x i16>"), "{blur}");
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
		assert!(ir.contains("i64 6, i64 0 }"), "{ir}");
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
		// only the cold paths that grow the array read it again
		let pushes: String = (0..12)
			.map(|index| format!("\ttarget.push({index})\n"))
			.collect();
		let source = format!(
			"function many = (target: Array<Byte>) mutates target => {{\n{pushes}}}\nmutable a: Array<Byte> = []\nmutable b: Array<Byte> = []\nmany(a)\nmany(b)\nmany(a)\nconsole.log(a, b)\n"
		);
		let ir = optimized_ir("consecutive-pushes", &source);
		let many = function(&ir, "fa.many");
		assert_eq!(count(many, "@fa_array_grow("), 12, "{many}");
		let hot: usize = many
			.split("\n\n")
			.filter(|block| !block.starts_with("push_grow"))
			.map(|block| count(block, " = load "))
			.sum();
		// the slot, the length, the capacity and the items, once
		assert_eq!(hot, 4, "{many}");
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
		let pushes: String = (0..12)
			.map(|index| format!("\ttarget.push({index})\n"))
			.collect();
		let source = format!(
			"function many = (target: GrowingArray<Byte>) mutates target => {{\n{pushes}}}\nmutable a: GrowingArray<Byte> = []\nmutable b: GrowingArray<Byte> = []\nmany(a)\nmany(b)\nmany(a)\nconsole.log(a, b)\n"
		);
		let ir = optimized_ir("growing-pushes", &source);
		let many = function(&ir, "fa.many");
		assert_eq!(count(many, "@fa_segmented_next("), 12, "{many}");
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
			"let letters = \"abc\"\nfunction letter = (i: Integer): Character throws IndexError => try letters[i]\nconsole.log(try letter(arguments.length))\n",
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
	fn injective_integer_hashes_need_no_key_comparator() {
		let ir = ir_of(
			"injective-integer-hash",
			"mutable items: Map<Integer, Integer> = [(1, 2)]\nitems[3] = 4\nconsole.log(items.get(1), items.get(3), items.get(5))\n",
		);
		assert!(!ir.contains("@fa.key_equals("), "{ir}");
		assert!(ir.contains("@fa_table_find_growing("), "{ir}");
	}

	#[test]
	fn single_byte_string_comparisons_do_not_call_the_general_byte_loop() {
		let ir = optimized_ir(
			"single-byte-equality",
			"let word = arguments.get(0) ?? \"\"\nconsole.log(word == \" \", word != \" \", \" \" == word)\n",
		);
		assert!(!ir.contains("@string_equal_bytes("), "{ir}");
		assert!(!ir.contains("@fa_string_equal("), "{ir}");
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
}
