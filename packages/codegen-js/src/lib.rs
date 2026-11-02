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

use fa_analyzer::ids::FileId;
use fa_analyzer::{Diagnostic, Session, Severity};

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
}

impl Options {
	/// Options for a target, with locations shown relative to `path`.
	pub fn new(target: Target, path: impl Into<String>) -> Self {
		Self {
			target,
			path: path.into(),
			declarations: false,
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
