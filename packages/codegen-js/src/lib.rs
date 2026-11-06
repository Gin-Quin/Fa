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

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Mutex;

use rayon::prelude::*;

use fa_analyzer::ids::{FileId, NodeId};
use fa_analyzer::{Diagnostic, Program, Session, Severity};

pub use runtime::{collect_helpers, collect_helpers_inline, helper_dependencies, runtime_helper};

/// Where the generated JavaScript runs.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Target {
	/// Bun (`fa run`).
	Bun,
	/// Node.js.
	Node,
	/// A browser (an ES module for `<script type="module">`).
	Web,
}

impl Target {
	/// The target named on the command line (`bun`, `node`, `web`).
	pub fn from_name(name: &str) -> Option<Self> {
		match name {
			"bun" => Some(Target::Bun),
			"node" => Some(Target::Node),
			"web" => Some(Target::Web),
			_ => None,
		}
	}

	/// The name of the target.
	pub fn name(self) -> &'static str {
		match self {
			Target::Bun => "bun",
			Target::Node => "node",
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
	/// The C libraries of foreign code (`build.link`: names, or paths), opened
	/// with `bun:ffi` before the C library.
	pub libraries: Vec<String>,
	/// The asynchronous mode (`design/async.md` A5–A7): `None` detects it
	/// for a standalone file (its top level starts a task or awaits
	/// something that may suspend); a program takes `Program::asynchronous`.
	pub asynchronous: Option<bool>,
	/// A development build for hot reloading (`fa develop`, decision D43):
	/// the declarations of other modules are read through the registry of
	/// the module swapped last (`faHotModule`: `$h0.ns.name`), and `main.js`
	/// listens for swaps (`faHotListen`).
	pub hot: bool,
	/// What the index proofs of the analysis (`fa_analyzer::proofs`) may
	/// rely on: [`ProgramProofs::None`] in a build that doesn't see every
	/// file first (the streaming build of a package).
	pub proofs: ProgramProofs,
}

/// What the index proofs of a module see (`fa_analyzer::proofs`).
#[derive(Clone, Debug, Default)]
pub enum ProgramProofs {
	/// Nothing: every index keeps its checks.
	#[default]
	None,
	/// The module is the whole program: the index proofs and the
	/// never-negative places, over its own stores.
	Alone,
	/// A module of a program: private-array index proofs and never-negative
	/// places, by the failed claims of the whole program ([`program_claims`]).
	Claims(std::sync::Arc<BTreeSet<String>>),
}

/// The failed claims of the proofs of a program's files: each file's part
/// (`fa_analyzer::proofs::unit_claims`), merged and resolved, as natively
/// the facts of the units of a build per unit.
pub fn program_claims(
	session: &Session,
	files: &[FileId],
	overflow_checks: bool,
) -> std::sync::Arc<BTreeSet<String>> {
	let types = session.types();
	let proof_files: Vec<fa_analyzer::proofs::ProofFile<'_>> = files
		.iter()
		.map(|&file| {
			fa_analyzer::proofs::ProofFile::new(session.tree(file), session.analysis(file), types)
				.with_module(session.module(file))
		})
		.collect();
	let edges = fa_analyzer::proofs::unit_claims_of_goal(&proof_files, overflow_checks);
	std::sync::Arc::new(fa_analyzer::proofs::resolve_claims(&edges))
}

/// Whether the JavaScript of a file of a program of several files depends
/// on the program's claims ([`ProgramProofs::Claims`]): an item index whose
/// bounds check or count from the end disappears when no claim fails.
/// Failed claims only take proofs away, so
/// a file without one is generated the same way whatever the claims: a
/// streaming build writes it before every unit's claims are known.
pub fn reads_program_claims(session: &Session, file: FileId, overflow_checks: bool) -> bool {
	use fa_analyzer::lower::index::{IndexAccess, index_access};
	use fa_analyzer::proofs::{ProofScope, index_facts};
	use fa_analyzer::syntax::{View, view};
	let tree = session.tree(file);
	let analysis = session.analysis(file);
	let types = session.types();
	let indices: Vec<(NodeId, NodeId, IndexAccess)> = (0..tree.nodes.len())
		.map(NodeId::new)
		.filter_map(|n| match view(tree, n) {
			View::Index { index, .. } => match index_access(tree, analysis, types, n, index) {
				IndexAccess::Direct => None,
				access => Some((n, index, access)),
			},
			_ => None,
		})
		.collect();
	if indices.is_empty() {
		return false;
	}
	let nothing_failed = |_: &str| false;
	let scope = ProofScope::Unit {
		module: session.module(file),
		failed: &nothing_failed,
	};
	let facts = index_facts(tree, analysis, types, scope, overflow_checks);
	indices
		.into_iter()
		.any(|(node, index, access)| match access {
			IndexAccess::Checked => facts.proven_index(node),
			IndexAccess::FromEnd => facts.proven_non_negative(index),
			IndexAccess::Direct => false,
		})
}

/// Whether a file reads no other file of the session: alone, it is the
/// whole program.
fn alone(session: &Session, file: FileId) -> bool {
	session
		.dependency_closure(file)
		.iter()
		.all(|&module| session.file_of(module).is_none_or(|other| other == file))
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
			libraries: Vec::new(),
			asynchronous: None,
			hot: false,
			proofs: ProgramProofs::None,
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
			match trimmed.find("*/") {
				// a cast starting a line (`/** @type {T} */ (value),`): code
				Some(end) if !trimmed[end + 2..].trim().is_empty() => {}
				Some(_) => continue,
				None => {
					in_comment = true;
					continue;
				}
			}
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
	let mut options = options.clone();
	if options.asynchronous.is_none() {
		options.asynchronous = Some(detect_asynchronous(session, file));
	}
	if alone(session, file) {
		options.proofs = ProgramProofs::Alone;
	}
	let options = &options;
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

/// The asynchronous mode of a standalone file (A6): its top level or its
/// exported function starts a task, or awaits something that may suspend.
pub fn detect_asynchronous(session: &Session, file: FileId) -> bool {
	let interface = &session.source_file(file).interface;
	// the key of the top level's summary (`check/tasks.rs`)
	let top_level = fa_analyzer::ids::NodeId(u32::MAX);
	let mut keys = vec![top_level];
	if let Some(export) = &interface.export {
		keys.push(export.decl);
	}
	keys.iter().any(|key| {
		interface
			.async_summaries
			.get(key)
			.is_some_and(|summary| summary.starts || summary.suspends)
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

/// Where the modules of a program go as they are generated
/// ([`generate_program_into`]): each module is written by the thread that
/// generated it, and never held.
pub trait ModuleSink: Sync {
	/// Writes a module at its path relative to the output folder.
	fn write(&self, path: &str, code: &str) -> std::io::Result<()>;
}

/// The modules written by [`generate_program_into`].
#[derive(Clone, Debug)]
pub struct WrittenProgram {
	/// The paths of the modules written, `main.js` first.
	pub paths: Vec<String>,
	/// The warnings of the analysis.
	pub warnings: Vec<Diagnostic>,
}

/// Why [`generate_program_into`] failed.
#[derive(Debug)]
pub enum WriteError {
	/// The program has errors (see [`GenerateError`]).
	Generate(GenerateError),
	/// A module couldn't be written.
	Io(std::io::Error),
}

impl From<GenerateError> for WriteError {
	fn from(error: GenerateError) -> Self {
		WriteError::Generate(error)
	}
}

/// Generates the modules of an analyzed program (see [`ProgramOutput`]). The
/// errors of every file come first.
pub fn generate_program(
	session: &mut Session,
	program: &Program,
	options: &Options,
	entry: &EntryOptions,
) -> Result<ProgramOutput, GenerateError> {
	/// Keeps the modules, by path.
	struct Collect(Mutex<Vec<(String, String)>>);
	impl ModuleSink for Collect {
		fn write(&self, path: &str, code: &str) -> std::io::Result<()> {
			let mut modules = self.0.lock().unwrap_or_else(|error| error.into_inner());
			modules.push((path.to_string(), code.to_string()));
			Ok(())
		}
	}
	let sink = Collect(Mutex::new(Vec::new()));
	let written = match generate_program_into(session, program, options, entry, &sink) {
		Ok(written) => written,
		Err(WriteError::Generate(error)) => return Err(error),
		Err(WriteError::Io(_)) => unreachable!("collecting modules can't fail"),
	};
	let mut collected: HashMap<String, String> = sink
		.0
		.into_inner()
		.unwrap_or_else(|error| error.into_inner())
		.into_iter()
		.collect();
	let modules = written
		.paths
		.into_iter()
		.map(|path| {
			let code = collected.remove(&path).unwrap_or_default();
			(path, code)
		})
		.collect();
	Ok(ProgramOutput {
		modules,
		warnings: written.warnings,
	})
}

/// Generates the modules of an analyzed program in parallel, each written
/// into `sink` by the thread that generated it; `main.js` and
/// `$runtime.js` are written last. The errors of every file come first:
/// nothing is written then.
pub fn generate_program_into(
	session: &mut Session,
	program: &Program,
	options: &Options,
	entry: &EntryOptions,
	sink: &dyn ModuleSink,
) -> Result<WrittenProgram, WriteError> {
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
		}
		.into());
	}
	let mut options = options.clone();
	if options.asynchronous.is_none() {
		// for `fa test`, the mode of the tests (`Session::configure_asynchronous`)
		options.asynchronous = Some(program.asynchronous);
	}
	// every file is analyzed: the proofs see the whole program
	options.proofs = match files[..] {
		[file] if alone(session, file) => ProgramProofs::Alone,
		_ => ProgramProofs::Claims(program_claims(session, &files, options.overflow_checks)),
	};
	let options = &options;
	let session: &Session = session;
	let done: Vec<std::io::Result<Result<UnitModule, Vec<Diagnostic>>>> =
		fa_analyzer::parallel::install(|| {
			files
				.par_iter()
				.map(|&file| generate_unit_module(session, file, options, entry.tests, sink))
				.collect()
		});
	let mut helpers: BTreeSet<&'static str> = BTreeSet::new();
	let mut entry_has_main = false;
	let mut entry_asynchronous = false;
	let mut paths = vec!["main.js".to_string(), "$runtime.js".to_string()];
	for (&file, done) in files.iter().zip(done) {
		match done.map_err(WriteError::Io)? {
			Ok(module) => {
				helpers.extend(module.helpers);
				if Some(file) == program.entry {
					entry_has_main = module.has_main;
					entry_asynchronous = module.asynchronous;
				}
				paths.push(module.path);
				paths.extend(module.typings);
			}
			Err(diagnostics) => return Err(GenerateError { diagnostics }.into()),
		}
	}
	let warnings = files
		.iter()
		.flat_map(|&file| {
			session
				.diagnostics(file)
				.iter()
				.filter(|diagnostic| diagnostic.severity == Severity::Warning)
				.cloned()
		})
		.collect();
	// the entry module
	let file_set: HashSet<FileId> = files.iter().copied().collect();
	let inputs = EntryInputs {
		init_order: program
			.init_order
			.iter()
			.filter(|(file, _)| file_set.contains(file))
			.map(|&(file, decl)| (session.module(file).to_string(), decl))
			.collect(),
		entry: program.entry.map(|file| EntryModule {
			module: session.module(file).to_string(),
			has_main: entry_has_main,
			export: entry_export(session, session.module_id(file)),
			asynchronous: entry_asynchronous,
		}),
		tests: program
			.tests
			.iter()
			.map(|&file| session.module(file).to_string())
			.collect(),
	};
	let main = entry_module(&inputs, options, entry, &mut helpers);
	if let Some(worker) = worker_module(&inputs, &mut helpers) {
		sink.write(WORKER_MODULE, &worker).map_err(WriteError::Io)?;
		paths.push(WORKER_MODULE.to_string());
	}
	sink.write("$runtime.js", &runtime_module(&helpers))
		.map_err(WriteError::Io)?;
	sink.write("main.js", &main).map_err(WriteError::Io)?;
	if options.declarations {
		// the entry module runs the program: it exports nothing
		sink.write("main.d.ts", ENTRY_DECLARATIONS)
			.map_err(WriteError::Io)?;
		paths.push("main.d.ts".to_string());
	}
	Ok(WrittenProgram { paths, warnings })
}

/// The `.d.ts` of the entry module of a program (`main.d.ts`): it runs the
/// program, and exports nothing.
pub const ENTRY_DECLARATIONS: &str =
	"// Generated by fa: the entry module runs the program.\nexport {};\n";

/// The `.d.ts` of a module of a program, next to it (`library/math.d.ts`).
pub fn declarations_path(js_path: &str) -> String {
	format!("{}.d.ts", js_path.strip_suffix(".js").unwrap_or(js_path))
}

/// The module a worker's thread runs (step J2, runtime `workers.js`).
pub const WORKER_MODULE: &str = "$worker.js";

/// `$worker.js`, when a module uses workers (`FaWorker` among `helpers`):
/// the thread of a worker initializes the module-level values of the
/// program (its own copies), then serves the calls it receives
/// (`faWorkerServe`, added to `helpers`).
pub fn worker_module(inputs: &EntryInputs, helpers: &mut BTreeSet<&'static str>) -> Option<String> {
	if !helpers.contains("FaWorker") {
		return None;
	}
	helpers.insert("faWorkerServe");
	let mut module = String::from(
		"// Generated by fa: the thread of a worker.\nimport { faWorkerServe } from \"./$runtime.js\";\n",
	);
	let mut calls = String::new();
	for (index, (path, decl)) in inputs.init_order.iter().enumerate() {
		let path = generate::modules::js_path(path);
		module.push_str(&format!(
			"import {{ $init{} as $i{index} }} from \"./{path}\";\n",
			decl.index()
		));
		calls.push_str(&format!("\t$i{index}();\n"));
	}
	module.push_str(&format!("\nfaWorkerServe(() => {{\n{calls}}});\n"));
	Some(module)
}

/// What [`generate_unit_module`] leaves of a module it wrote.
#[derive(Clone, Debug)]
pub struct UnitModule {
	/// Its path relative to the output folder (`library/math.js`).
	pub path: String,
	/// The runtime helpers it imports.
	pub helpers: Vec<&'static str>,
	/// Whether it exports `$main` (its top-level statements).
	pub has_main: bool,
	/// Whether it was generated in asynchronous mode (its `$main` is an
	/// async function taking its task).
	pub asynchronous: bool,
	/// The path of its TypeScript declarations (`library/math.d.ts`), when
	/// asked for (`Options::declarations`).
	pub typings: Option<String>,
}

/// Generates the JavaScript module of one analyzed file of a program and
/// writes it into `sink`; its errors (constructs the backend doesn't
/// support) instead, sorted. The other modules are read through their
/// interfaces only: this runs on the thread that analyzed the file
/// (`fa_analyzer::pipeline`), with any other file of the program dropped.
pub fn generate_unit_module(
	session: &Session,
	file: FileId,
	options: &Options,
	tests: bool,
	sink: &dyn ModuleSink,
) -> std::io::Result<Result<UnitModule, Vec<Diagnostic>>> {
	let mut context = generate::modules::program_context(session, file);
	context.tests = tests;
	let options = Options {
		path: session.display_path(file),
		..options.clone()
	};
	let generated = generate::generate_module(session, file, &options, Some(&context));
	if !generated.errors.is_empty() {
		let mut diagnostics = generated.errors;
		fa_analyzer::diagnostics::sort(&mut diagnostics);
		return Ok(Err(diagnostics));
	}
	let mut code = String::with_capacity(generated.code.len() + 160);
	code.push_str("// Generated by fa from ");
	code.push_str(&options.path);
	code.push_str(": edit the Fa source instead.\n");
	if !generated.helpers.is_empty() {
		code.push_str("import { ");
		code.push_str(&generated.helpers.join(", "));
		code.push_str(" } from \"");
		code.push_str(&generate::modules::relative(&context.path, "$runtime.js"));
		code.push_str("\";\n");
	}
	code.push_str(&generated.code);
	drop(generated.code);
	sink.write(&context.path, &code)?;
	let typings = match &generated.typings {
		Some(typings) => {
			let path = declarations_path(&context.path);
			sink.write(&path, typings)?;
			Some(path)
		}
		None => None,
	};
	Ok(Ok(UnitModule {
		path: context.path,
		helpers: generated.helpers,
		has_main: generated.has_main,
		asynchronous: generated.asynchronous,
		typings,
	}))
}

/// The entry module of a program, by module path.
#[derive(Clone, Debug)]
pub struct EntryModule {
	/// Its module path (`application/main`).
	pub module: String,
	/// Whether it exports `$main` (its top-level statements).
	pub has_main: bool,
	/// The name its exported entry function is exported as, if any.
	pub export: Option<String>,
	/// Whether it was generated in asynchronous mode (`UnitModule::asynchronous`).
	pub asynchronous: bool,
}

/// What `main.js` needs of the program.
#[derive(Clone, Debug, Default)]
pub struct EntryInputs {
	/// The module-level values to initialize, in order: `(module path,
	/// declaration)`.
	pub init_order: Vec<(String, fa_analyzer::ids::NodeId)>,
	/// The entry module, if any.
	pub entry: Option<EntryModule>,
	/// The test files' modules.
	pub tests: Vec<String>,
}

/// The export name of a module's exported function, from its interface.
pub fn entry_export(session: &Session, module: fa_analyzer::ids::ModuleId) -> Option<String> {
	let interface = session.interface(module)?;
	let export = interface
		.export
		.filter(|export| export.kind == fa_analyzer::symbols::SymbolKind::Function)?;
	Some(
		interface
			.export_names
			.get(&export.decl)
			.cloned()
			.unwrap_or_default(),
	)
}

/// `main.js`: the entry of a program of several modules (it initializes the
/// module-level values in order, then runs the program or its tests). The
/// helpers it imports are added to `helpers`.
pub fn entry_module(
	inputs: &EntryInputs,
	options: &Options,
	entry: &EntryOptions,
	helpers: &mut BTreeSet<&'static str>,
) -> String {
	let mut main = String::from("// Generated by fa: the entry of the program.\n");
	let asynchronous = !entry.tests
		&& inputs
			.entry
			.as_ref()
			.is_some_and(|entry_module| entry_module.asynchronous);
	// a test that awaits: the test modules use `faTestTask` (modules.rs)
	let asynchronous_tests = entry.tests && helpers.contains("faTestTask");
	let mut used = vec![if asynchronous || asynchronous_tests {
		"faRunAsync"
	} else {
		"faRun"
	}];
	if entry.tests {
		used.push("faRunTests");
	}
	if asynchronous_tests {
		used.push("faTestTask");
	}
	// a hot build (D43): the modules with values registered, the swaps listened for
	let hot = options.hot && !entry.tests;
	if hot {
		used.push("faHotModule");
		used.push("faHotListen");
	}
	helpers.extend(used.iter().copied());
	main.push_str(&format!(
		"import {{ {} }} from \"./$runtime.js\";\n",
		used.join(", ")
	));
	if hot {
		let mut registered: Vec<&String> = inputs
			.init_order
			.iter()
			.map(|(module, _)| module)
			.chain(inputs.entry.as_ref().map(|entry| &entry.module))
			.collect();
		registered.sort();
		registered.dedup();
		for (index, module) in registered.into_iter().enumerate() {
			let path = generate::modules::js_path(module);
			main.push_str(&format!(
				"import * as $m{index} from \"./{path}\";\nfaHotModule({}, $m{index});\n",
				crate::js::string_literal(&path)
			));
		}
		main.push_str("faHotListen(import.meta.url);\n");
	}
	let mut init_calls = Vec::new();
	for (index, (module, decl)) in inputs.init_order.iter().enumerate() {
		let path = generate::modules::js_path(module);
		main.push_str(&format!(
			"import {{ $init{} as $i{index} }} from \"./{path}\";\n",
			decl.index()
		));
		init_calls.push(format!("$i{index}();"));
	}
	let arguments = match options.target {
		Target::Web => "[]",
		Target::Bun | Target::Node => "process.argv.slice(2)",
	};
	if entry.tests {
		let mut lists = Vec::new();
		for (index, module) in inputs.tests.iter().enumerate() {
			let path = generate::modules::js_path(module);
			main.push_str(&format!(
				"import {{ tests as $tests{index} }} from \"./{path}\";\n"
			));
			// by module: `fa test` runs each file in a process of its own
			// (`--fa-test-file=<module>`)
			lists.push(format!(
				"[{}, $tests{index}]",
				crate::js::string_literal(module)
			));
		}
		let run = if asynchronous_tests {
			format!(
				"faRunAsync(async () => await faRunTests([{}], $init, faTestTask, {arguments}));",
				lists.join(", ")
			)
		} else {
			format!(
				"faRun(() => faRunTests([{}], $init, undefined, {arguments}));",
				lists.join(", ")
			)
		};
		main.push_str(&format!(
			"\nfunction $init() {{\n{}}}\n\n{run}\n",
			init_calls
				.iter()
				.map(|call| format!("\t{call}\n"))
				.collect::<String>(),
		));
	} else {
		let mut body = init_calls
			.iter()
			.map(|call| format!("\t{call}\n"))
			.collect::<String>();
		if let Some(entry_module) = &inputs.entry {
			let path = generate::modules::js_path(&entry_module.module);
			if entry_module.has_main {
				main.push_str(&format!("import {{ $main }} from \"./{path}\";\n"));
				if asynchronous {
					body.push_str("\tawait $main($task);\n");
				} else {
					body.push_str("\t$main();\n");
				}
			}
			if let Some(name) = &entry_module.export {
				main.push_str(&format!(
					"import {{ {name} as $entry }} from \"./{path}\";\n"
				));
				if asynchronous {
					body.push_str(&format!("\treturn await $entry({arguments});\n"));
				} else {
					body.push_str(&format!("\treturn $entry({arguments});\n"));
				}
			}
		}
		if asynchronous {
			main.push_str(&format!("\nfaRunAsync(async ($task) => {{\n{body}}});\n"));
		} else {
			main.push_str(&format!("\nfaRun(() => {{\n{body}}});\n"));
		}
	}
	main
}

/// The path of a Fa module's JavaScript module, relative to the output
/// folder (`library/math` → `library/math.js`).
pub fn js_path(module: &str) -> String {
	generate::modules::js_path(module)
}

/// `$runtime.js`: the runtime helpers the modules of a program import.
pub fn runtime_module(helpers: &BTreeSet<&'static str>) -> String {
	without_documentation(&collect_helpers(helpers.iter()))
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

	/// A package whose files read each other in every way: namespaces with
	/// types, methods, `static`s and `mutable` members, values initialized
	/// from other files' values, a cycle between namespace files, and two
	/// files declaring object types of the same structure.
	const PACKAGE: &[(&str, &str)] = &[
		(
			"package.fa",
			"export = {\n\tname = \"p\"\n\tversion = \"0.1.0\"\n}\n",
		),
		(
			"library/shapes.fa",
			"export namespace = {\n\ttype Item = {\n\t\tid: Integer\n\t\tfunction score = (self): Integer => self.id * 2\n\t}\n\tstatic base = 3\n\tmutable count = 0\n\tfunction make = (seed: Integer, step: Integer = 1): Item => Item { id = seed + base + step }\n\tfunction bump = () mutates count => { count += 1 }\n}\n",
		),
		(
			"library/other.fa",
			"export namespace = {\n\ttype Item = {\n\t\tid: Integer\n\t\tfunction score = (self): Integer => self.id * 5\n\t}\n\tfunction make = (seed: Integer): Item => Item { id = seed }\n}\n",
		),
		("library/total.fa", "export = values.first + shapes.base\n"),
		(
			"library/values.fa",
			"export namespace = {\n\tlet first = 10\n}\n",
		),
		(
			"library/ringA.fa",
			"export namespace = {\n\tfunction call = (x: Integer): Integer => if x <= 0 { 0 } else { ringB.call(x - 1) + 1 }\n}\n",
		),
		(
			"library/ringB.fa",
			"export namespace = {\n\tfunction call = (x: Integer): Integer => if x <= 0 { 0 } else { ringA.call(x - 1) + 2 }\n}\n",
		),
		(
			"application/main.fa",
			"export function = () => {\n\tlibrary.shapes.bump()\n\tconsole.log(library.shapes.make(4, step = 2).score() + library.other.make(1).score() + library.total + library.ringA.call(3) + library.shapes.count + library.shapes.base)\n}\n",
		),
	];

	/// The package above, analyzed.
	fn analyzed_package() -> (Session, Program) {
		let files: Vec<(String, String)> = PACKAGE
			.iter()
			.map(|(path, text)| (path.to_string(), text.to_string()))
			.collect();
		let mut session = Session::new();
		session.set_resolver(Box::new(fa_analyzer::CodebaseResolver::from_texts(
			&files, None,
		)));
		let program = session.analyze_package(Some("application/main"));
		(session, program)
	}

	#[test]
	fn a_package_proves_private_indices_after_collecting_every_units_claims() {
		for (other, checked) in [
			("export function = (): UInt32 => 0\n", false),
			(
				"export function = (): UInt32 => {\n\tlet wide = { next = UInt32(1000000), extra = 1 }\n\twide.next\n}\n",
				true,
			),
		] {
			let files = [
				("package.fa".to_string(), "export = { name = \"indices\" }\n".to_string()),
				("library/chain.fa".to_string(), "type Link = { next: UInt32 }\nmutable links: Array<Link> = [{ next = 0 }]\nfunction hop = (items: Array<Link>, from: UInt32): UInt32 throws IndexError => try items[try items[from].next].next\nexport function = (): UInt32 => hop(links, 0) catch error => 99\n".to_string()),
				("library/other.fa".to_string(), other.to_string()),
				("application/main.fa".to_string(), "export function = () => console.log(library.chain(), library.other())\n".to_string()),
			];
			let mut session = Session::new();
			session.set_resolver(Box::new(fa_analyzer::CodebaseResolver::from_texts(
				&files, None,
			)));
			let program = session.analyze_package(Some("application/main"));
			let links = program
				.files
				.iter()
				.copied()
				.find(|&file| session.module(file) == "library/chain")
				.expect("links module");
			// The first streaming pass must defer this module, including
			// when a foreign wider object eventually prevents the proof.
			assert!(reads_program_claims(&session, links, true));
			let output = generate_program(
				&mut session,
				&program,
				&Options::new(Target::Bun, "indices"),
				&EntryOptions::default(),
			)
			.expect("the package generates");
			let code = &output
				.modules
				.iter()
				.find(|(path, _)| path == "library/chain.js")
				.expect("links output")
				.1;
			assert_eq!(code.contains("faIndex("), checked, "{code}");
		}
	}

	/// Modules are read only through their interfaces: with the trees and
	/// analyses of every other file dropped, a file generates the same
	/// module, and the program the same initialization order
	/// (design/parallel-pipeline.md, step 2).
	#[test]
	fn a_module_reads_the_others_through_their_interfaces_only() {
		let (mut session, program) = analyzed_package();
		for &file in &program.files {
			let diagnostics = session.diagnostics(file);
			assert!(
				diagnostics.is_empty(),
				"{}: {diagnostics:?}",
				session.module(file)
			);
		}
		let options = Options::new(Target::Bun, "p");
		let full = generate_program(&mut session, &program, &options, &EntryOptions::default())
			.expect("the package generates");
		for &kept in &program.files {
			let (mut session, program) = analyzed_package();
			let module = session.module(kept).to_string();
			for &file in &program.files {
				if file != kept {
					session.forget_tree(file);
				}
			}
			let again = session.assemble_again(&program);
			assert_eq!(again.init_order, program.init_order, "{module}");
			assert_eq!(again.files, program.files, "{module}");
			let context = generate::modules::program_context(&session, kept);
			let options = Options {
				path: session.display_path(kept),
				..options.clone()
			};
			let generated = generate::generate_module(&session, kept, &options, Some(&context));
			assert!(
				generated.errors.is_empty(),
				"{module}: {:?}",
				generated.errors
			);
			let path = generate::modules::js_path(&module);
			let (_, code) = full
				.modules
				.iter()
				.find(|(other, _)| *other == path)
				.unwrap();
			assert!(
				code.ends_with(&generated.code),
				"{module}:\n{code}\n---\n{}",
				generated.code
			);
		}
	}

	/// `--declarations` on a program: a `.d.ts` next to each module, which
	/// declares what the module exports (its export names), and the entry's.
	#[test]
	fn a_program_declares_the_exports_of_each_module() {
		let (mut session, program) = analyzed_package();
		let options = Options {
			declarations: true,
			..Options::new(Target::Bun, "p")
		};
		let output = generate_program(&mut session, &program, &options, &EntryOptions::default())
			.expect("the package generates");
		let typings = |path: &str| {
			output
				.modules
				.iter()
				.find(|(other, _)| other == path)
				.map(|(_, code)| code.clone())
				.unwrap_or_else(|| panic!("no {path}"))
		};
		for (path, _) in &output.modules {
			if let Some(module) = path.strip_suffix(".js")
				&& path != "$runtime.js"
			{
				typings(&format!("{module}.d.ts"));
			}
		}
		let shapes = typings("library/shapes.d.ts");
		assert!(
			shapes.contains(
				"export declare function shapes$make(seed: number, step?: number): { id: number };"
			),
			"{shapes}"
		);
		assert!(
			shapes.contains("export declare let shapes$count: number;")
				&& shapes.contains("export declare const shapes$count$ref: { v: number };"),
			"{shapes}"
		);
		assert!(
			shapes.contains("export declare function Item$score(self: { id: number }): number;"),
			"{shapes}"
		);
		let values = typings("library/values.d.ts");
		assert!(
			values.contains("export declare const values$first: number;"),
			"{values}"
		);
		assert!(typings("main.d.ts").ends_with("export {};\n"));
	}

	#[test]
	fn overflow_checks_can_be_turned_off() {
		let program = program(false);
		assert!(!program.contains("faCheck"), "{program}");
		// an `Int8` and an `Int64` wrap like native code; `Integer` doesn't
		// (design/decisions.md, "After the backends review")
		assert!(program.contains("return a + b << 24 >> 24;"), "{program}");
		assert!(program.contains("return x + 1;"), "{program}");
		assert!(
			program.contains("return BigInt.asIntN(64, x * 3n);"),
			"{program}"
		);
	}
}
