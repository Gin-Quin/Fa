//! `fa-codegen-js`: Fa's JavaScript backend.
//!
//! [`generate`] turns an analyzed file into one ES2022 module: readable
//! JavaScript (`const`/`let`, plain functions, `if`/`while`/`for`, template
//! literals) with only the runtime helpers it uses inlined at its start. The
//! analyzer decides every runtime check (`Analysis::flags_of`): the
//! generator emits exactly those (overflow, sink ranges, divisors,
//! conversions), each stop carrying its `file:line:column`.
//!
//! The runtime helpers are plain ES2022 modules under `runtime/`, embedded
//! here, and extracted one by one with their dependencies: see [`runtime`].
//!
//! ```
//! use fa_codegen_js::{Options, Target, generate};
//!
//! let (mut session, file) = fa_analyzer::analyze_source("let a = 1\nconsole.log(a + 1)\n");
//! let output = generate(&mut session, file, &Options::new(Target::Bun, "main.fa")).unwrap();
//! assert!(output.program.contains("console.log(String(a + 1));"));
//! ```

mod declarations;
mod generate;
pub mod js;
pub mod names;
pub mod runtime;

use std::collections::{BTreeSet, HashMap};

use fa_analyzer::ids::FileId;
use fa_analyzer::{Diagnostic, Program, Session, Severity};

pub use runtime::{collect_helpers, collect_helpers_inline, helper_dependencies, runtime_helper};

/// Where the generated JavaScript runs.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Target {
	/// Bun (`fa run`).
	Bun,
	/// Node.js.
	Node,
	/// Deno.
	Deno,
	/// A browser (an ES module for `<script type="module">`).
	Web,
}

impl Target {
	/// The target named on the command line (`bun`, `node`, `deno`, `web`).
	pub fn from_name(name: &str) -> Option<Self> {
		match name {
			"bun" => Some(Target::Bun),
			"node" => Some(Target::Node),
			"deno" => Some(Target::Deno),
			"web" => Some(Target::Web),
			_ => None,
		}
	}

	/// The name of the target.
	pub fn name(self) -> &'static str {
		match self {
			Target::Bun => "bun",
			Target::Node => "node",
			Target::Deno => "deno",
			Target::Web => "web",
		}
	}
}

/// The options of [`generate`].
#[derive(Clone, Debug)]
pub struct Options {
	/// The target runtime.
	pub target: Target,
	/// The path of the file as shown in the locations of stops (`main.fa`).
	pub path: String,
	/// Whether to generate the TypeScript declarations of the exports.
	pub declarations: bool,
	/// Whether to check integer overflows at runtime (`build.overflowChecks`,
	/// `--overflow-checks`; `design/decisions.md`, "Overflow checks"). When
	/// off, `OVERFLOW_JS`, `OVERFLOW_NATIVE` and `SINK_CHECK` are ignored:
	/// the operations are plain number (or bigint) operations. Divisor,
	/// index, shift and conversion checks stay.
	pub overflow_checks: bool,
}

impl Options {
	/// Options for a target, with locations shown relative to `path`, and
	/// overflow checks (the development default).
	pub fn new(target: Target, path: impl Into<String>) -> Self {
		Self {
			target,
			path: path.into(),
			declarations: false,
			overflow_checks: true,
		}
	}
}

/// A generated module.
#[derive(Clone, Debug)]
pub struct JsOutput {
	/// The ES module, runtime helpers included.
	pub code: String,
	/// The TypeScript declarations of its exports (`.d.ts`), when asked for.
	pub declarations: Option<String>,
	/// The runtime helpers inlined in `code`.
	pub helpers: Vec<&'static str>,
	/// The program without the runtime helpers (what golden tests compare).
	pub program: String,
	/// The warnings of the analysis.
	pub warnings: Vec<Diagnostic>,
}

/// Why a file couldn't be generated: its errors (from the analysis, or from
/// the JavaScript backend: `E-NUM-19`, constructs not supported yet), which
/// `Session::render_diagnostics`-style rendering can print.
#[derive(Clone, Debug)]
pub struct GenerateError {
	/// The errors, sorted by position.
	pub diagnostics: Vec<Diagnostic>,
}

impl std::fmt::Display for GenerateError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for diagnostic in &self.diagnostics {
			writeln!(f, "error[{}]: {}", diagnostic.code.id, diagnostic.message)?;
		}
		Ok(())
	}
}

impl std::error::Error for GenerateError {}

/// The code of runtime helpers without their documentation comments (the
/// `/** … */` blocks starting a line): the runtime documents them, the
/// generated program stays small.
fn without_documentation(code: &str) -> String {
	let mut out = String::with_capacity(code.len());
	let mut in_comment = false;
	for line in code.split_inclusive('\n') {
		let trimmed = line.trim();
		if in_comment {
			in_comment = !trimmed.ends_with("*/");
			continue;
		}
		if trimmed.starts_with("/**") {
			in_comment = !trimmed.ends_with("*/");
			continue;
		}
		out.push_str(line);
	}
	out
}

/// Generates the JavaScript module of a file of a session (analyzing it
/// first if needed). A file with errors gives its errors instead.
pub fn generate(
	session: &mut Session,
	file: FileId,
	options: &Options,
) -> Result<JsOutput, GenerateError> {
	session.analyze_file(file);
	let session: &Session = session;
	let analysis_errors: Vec<Diagnostic> = session
		.diagnostics(file)
		.iter()
		.filter(|diagnostic| diagnostic.severity == Severity::Error)
		.cloned()
		.collect();
	if !analysis_errors.is_empty() {
		return Err(GenerateError {
			diagnostics: analysis_errors,
		});
	}
	let generated = generate::generate_file(session, file, options);
	if !generated.errors.is_empty() {
		let mut diagnostics = generated.errors;
		fa_analyzer::diagnostics::sort(&mut diagnostics);
		return Err(GenerateError { diagnostics });
	}
	let mut code = format!(
		"// Generated by fa from {}: edit the Fa source instead.\n",
		options.path
	);
	let helpers = without_documentation(&collect_helpers_inline(&generated.helpers));
	if !helpers.is_empty() {
		code.push('\n');
		code.push_str(&helpers);
	}
	if !generated.code.trim().is_empty() {
		code.push('\n');
		code.push_str(&generated.code);
	}
	let declarations = options
		.declarations
		.then(|| declarations::declarations(session, file));
	let warnings = session
		.diagnostics(file)
		.iter()
		.filter(|diagnostic| diagnostic.severity == Severity::Warning)
		.cloned()
		.collect();
	Ok(JsOutput {
		code,
		declarations,
		program: generated.code.clone(),
		helpers: runtime::resolve_helpers(&generated.helpers)
			.into_iter()
			.map(|helper| helper.name)
			.collect(),
		warnings,
	})
}

/// The JavaScript of a program of several files: one ES module per Fa file
/// (at its module path: `library/math.js`), the runtime helpers they use in
/// `$runtime.js`, and the entry module `main.js`, which initializes the
/// module-level values in the analyzer's order, then runs the program
/// (advanced brief §15.7, D36).
#[derive(Clone, Debug)]
pub struct ProgramOutput {
	/// The modules: `(path relative to the output folder, code)`, `main.js` first.
	pub modules: Vec<(String, String)>,
	/// The warnings of the analysis.
	pub warnings: Vec<Diagnostic>,
}

/// What the entry module of a program runs.
#[derive(Clone, Debug, Default)]
pub struct EntryOptions {
	/// The test files whose tests the entry module runs instead of the
	/// program (`fa test`): their modules export `tests`.
	pub tests: bool,
}

/// Generates the modules of an analyzed program (see [`ProgramOutput`]). The
/// errors of every file come first.
pub fn generate_program(
	session: &mut Session,
	program: &Program,
	options: &Options,
	entry: &EntryOptions,
) -> Result<ProgramOutput, GenerateError> {
	let files: Vec<FileId> = if entry.tests {
		program
			.files
			.iter()
			.chain(&program.tests)
			.copied()
			.collect()
	} else {
		program.files.clone()
	};
	let mut errors = Vec::new();
	for &file in &files {
		errors.extend(
			session
				.diagnostics(file)
				.iter()
				.filter(|diagnostic| diagnostic.severity == Severity::Error)
				.cloned(),
		);
	}
	if !errors.is_empty() {
		return Err(GenerateError {
			diagnostics: errors,
		});
	}
	let ids: HashMap<FileId, fa_analyzer::ids::ModuleId> = files
		.iter()
		.map(|&file| {
			let module = session.module(file).to_string();
			(file, session.types.module(&module))
		})
		.collect();
	let session: &Session = session;
	let mut contexts = generate::modules::program_contexts(session, &files, &ids);
	for context in &mut contexts {
		context.tests = entry.tests;
	}
	let mut entry_has_main = false;
	let mut modules = Vec::new();
	let mut helpers: BTreeSet<&'static str> = BTreeSet::new();
	let mut warnings = Vec::new();
	for (&file, context) in files.iter().zip(&contexts) {
		let options = Options {
			path: session.display_path(file),
			..options.clone()
		};
		let generated = generate::generate_module(session, file, &options, Some(context));
		if Some(file) == program.entry {
			entry_has_main = generated.has_main;
		}
		if !generated.errors.is_empty() {
			let mut diagnostics = generated.errors;
			fa_analyzer::diagnostics::sort(&mut diagnostics);
			return Err(GenerateError { diagnostics });
		}
		helpers.extend(generated.helpers.iter().copied());
		let mut code = format!(
			"// Generated by fa from {}: edit the Fa source instead.\n",
			options.path
		);
		if !generated.helpers.is_empty() {
			code.push_str(&format!(
				"import {{ {} }} from \"{}\";\n",
				generated.helpers.join(", "),
				generate::modules::relative(&context.path, "$runtime.js")
			));
		}
		code.push_str(&generated.code);
		modules.push((context.path.clone(), code));
		warnings.extend(
			session
				.diagnostics(file)
				.iter()
				.filter(|diagnostic| diagnostic.severity == Severity::Warning)
				.cloned(),
		);
	}
	// the entry module
	let mut main = String::from("// Generated by fa: the entry of the program.\n");
	let mut used = vec!["faRun"];
	if entry.tests {
		used.push("faRunTests");
	}
	helpers.extend(used.iter().copied());
	main.push_str(&format!(
		"import {{ {} }} from \"./$runtime.js\";\n",
		used.join(", ")
	));
	let mut init_calls = Vec::new();
	for (index, &(file, decl)) in program.init_order.iter().enumerate() {
		if !files.contains(&file) {
			continue;
		}
		let path = generate::modules::js_path(session.module(file));
		main.push_str(&format!(
			"import {{ $init{} as $i{index} }} from \"./{path}\";\n",
			decl.index()
		));
		init_calls.push(format!("$i{index}();"));
	}
	let arguments = match options.target {
		Target::Web => "[]",
		Target::Deno => "Deno.args",
		Target::Bun | Target::Node => "process.argv.slice(2)",
	};
	if entry.tests {
		let mut lists = Vec::new();
		for (index, &file) in program.tests.iter().enumerate() {
			let path = generate::modules::js_path(session.module(file));
			main.push_str(&format!(
				"import {{ tests as $tests{index} }} from \"./{path}\";\n"
			));
			lists.push(format!("...$tests{index}"));
		}
		main.push_str(&format!(
			"\nfunction $init() {{\n{}}}\n\nfaRun(() => faRunTests([{}], $init));\n",
			init_calls
				.iter()
				.map(|call| format!("\t{call}\n"))
				.collect::<String>(),
			lists.join(", ")
		));
	} else {
		let mut body = init_calls
			.iter()
			.map(|call| format!("\t{call}\n"))
			.collect::<String>();
		if let Some(entry_file) = program.entry {
			let path = generate::modules::js_path(session.module(entry_file));
			if entry_has_main {
				main.push_str(&format!("import {{ $main }} from \"./{path}\";\n"));
				body.push_str("\t$main();\n");
			}
			let export = session.source_file(entry_file).interface.export;
			if let Some(export) =
				export.filter(|export| export.kind == fa_analyzer::symbols::SymbolKind::Function)
			{
				let name = generate::modules::export_names(session, entry_file)
					.get(&export.decl)
					.cloned()
					.unwrap_or_default();
				main.push_str(&format!(
					"import {{ {name} as $entry }} from \"./{path}\";\n"
				));
				body.push_str(&format!("\treturn $entry({arguments});\n"));
			}
		}
		main.push_str(&format!("\nfaRun(() => {{\n{body}}});\n"));
	}
	modules.insert(0, ("main.js".to_string(), main));
	let runtime = without_documentation(&collect_helpers(helpers.iter()));
	modules.insert(1, ("$runtime.js".to_string(), runtime));
	Ok(ProgramOutput { modules, warnings })
}

#[cfg(test)]
mod tests {
	use super::*;

	const ARITHMETIC: &str = "function add = (a: Int8, b: Int8): Int8 => a + b\nfunction inc = (x: Integer) => x + 1\nfunction big = (x: Int64) => x * 3\nconsole.log(add(1, 2), inc(3), big(4))\n";

	fn program(overflow_checks: bool) -> String {
		let (mut session, file) = fa_analyzer::analyze_source(ARITHMETIC);
		let mut options = Options::new(Target::Bun, "main.fa");
		options.overflow_checks = overflow_checks;
		generate(&mut session, file, &options).unwrap().program
	}

	#[test]
	fn overflow_checks_are_on_by_default() {
		let program = program(true);
		assert!(program.contains("faCheckRange("), "{program}");
		assert!(program.contains("faCheckInteger("), "{program}");
		assert!(program.contains("faCheckWidth("), "{program}");
	}

	#[test]
	fn overflow_checks_can_be_turned_off() {
		let program = program(false);
		assert!(!program.contains("faCheck"), "{program}");
		assert!(program.contains("return a + b;"), "{program}");
		assert!(program.contains("return x * 3n;"), "{program}");
	}
}
