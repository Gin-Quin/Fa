//! The parallel and streaming analysis of a package
//! (design/parallel-pipeline.md §2–§3): `fa check` and `fa build`.
//!
//! 1. **Scan**: every module is read, parsed and scanned in parallel
//!    ([`crate::scan`]); a parsed tree is kept while the trees kept fit the
//!    tree budget, or parsed again when its unit runs.
//! 2. **Graph**: the strongly connected components of the module graph are
//!    the units ([`crate::graph`]); a unit is ready once the units it reads
//!    are published.
//! 3. **Units**, on the worker threads (Rayon, 64 MiB stacks), the ready
//!    unit with the longest chain of dependents first: a session of its own
//!    (a view of the global type store) analyzes the unit's files (rounds
//!    for a cycle), merges what their interfaces reach into the global
//!    store (merges run concurrently, `types/merge.rs`) and publishes the
//!    interfaces ([`Shared`]), which makes its dependents ready; then it
//!    hands the session to the [`UnitSink`] (diagnostics, code generation)
//!    and drops it: its trees, side tables and local types.
//! 4. **Program**: the initialization order and `MOD007`, the entry
//!    function (`MOD015`) and the problems of the scan, from the interfaces.
//!
//! Outputs don't depend on the threads: each module's diagnostics are
//! sorted, the report lists the modules in path order, and nothing is
//! ordered by a store id (P3). `FA_SCHEDULE_SEED` shuffles the order of
//! ready units of equal priority (the determinism test, P10).

use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostics::{Diagnostic, Location, SourceSpan};
use crate::ids::{DeclKey, FileId, ModuleId, NodeId, TypeId};
use crate::modules::{FileRole, Interface};
use crate::scan::FileScan;
use crate::session::{Parsed, Session, parse_module};

/// What the scan of a module gives: its scan, its tree when it is kept
/// (the tree budget), its source bytes.
type ScanResult = (Arc<FileScan>, Option<Parsed>, usize);

/// A module scanned, its tree taken by its unit.
type Scanned = (Arc<FileScan>, Mutex<Option<Parsed>>, usize);

/// What the units of a run share: the published interfaces, the scans,
/// the declarations of the global types (`Session::type_origins`).
#[derive(Default)]
pub struct Shared {
	/// The interface of each module, once its unit is published.
	pub(crate) interfaces: FxHashMap<ModuleId, OnceLock<Arc<Interface>>>,
	/// The scan of each module, by path.
	pub(crate) scans: FxHashMap<String, Arc<FileScan>>,
	/// The declarations of each global type of a published module.
	pub(crate) origins: Origins,
	/// The complete scanned graph, shared by closed whole-program proofs.
	pub(crate) region_modules: OnceLock<Arc<[String]>>,
}

/// The number of shards of [`Origins`].
const ORIGIN_SHARDS: usize = 32;

/// The declarations of the global types of the published modules
/// (`Session::type_origins`): for each type, its declarations in the order
/// of their module paths, so that a lookup reads them in that order and
/// stops at the first that fits. In shards: a unit publishing its types
/// locks one shard at a time, a lookup reads one shard's list and clones
/// it (an `Arc`), without waiting for a whole merge.
#[derive(Default)]
pub(crate) struct Origins {
	shards: [std::sync::RwLock<FxHashMap<TypeId, Arc<[DeclKey]>>>; ORIGIN_SHARDS],
}

impl Origins {
	fn shard(&self, ty: TypeId) -> &std::sync::RwLock<FxHashMap<TypeId, Arc<[DeclKey]>>> {
		&self.shards[ty.0 as usize % ORIGIN_SHARDS]
	}

	/// The declarations of a global type, in the order of their modules.
	pub(crate) fn get(&self, ty: TypeId) -> Option<Arc<[DeclKey]>> {
		self.shard(ty)
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.get(&ty)
			.cloned()
	}

	/// Adds declarations of a global type (`order`: the order of their
	/// modules, `Session::key_cmp`); a declaration known already is kept once.
	pub(crate) fn add(
		&self,
		ty: TypeId,
		keys: &[DeclKey],
		order: impl Fn(DeclKey, DeclKey) -> std::cmp::Ordering,
	) {
		let mut shard = self
			.shard(ty)
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		let mut list: Vec<DeclKey> = shard.get(&ty).map_or_else(Vec::new, |list| list.to_vec());
		for &key in keys {
			if let Err(position) = list.binary_search_by(|&other| order(other, key)) {
				list.insert(position, key);
			}
		}
		shard.insert(ty, list.into());
	}

	/// A copy (the origins of a previous run, for another one).
	fn snapshot(&self) -> Origins {
		let copy = Origins::default();
		for (shard, copied) in self.shards.iter().zip(&copy.shards) {
			let shard = shard
				.read()
				.unwrap_or_else(|poisoned| poisoned.into_inner());
			*copied
				.write()
				.unwrap_or_else(|poisoned| poisoned.into_inner()) = shard.clone();
		}
		copy
	}
}

impl Shared {
	/// The published interface of a module.
	pub fn interface(&self, module: ModuleId) -> Option<&Interface> {
		self.interfaces
			.get(&module)
			.and_then(OnceLock::get)
			.map(|interface| &**interface)
	}
}

/// What receives each analyzed unit, on the worker thread that analyzed
/// it: the unit's session (its files analyzed, their interfaces published)
/// and its files, in path order.
pub trait UnitSink: Sync {
	/// A unit is analyzed.
	fn unit(&self, session: &mut Session, files: &[FileId]);
	/// The session of a unit, once its modules are reported: dropped,
	/// unless the sink keeps it (the language server keeps the units of the
	/// open documents, their trees and analyses).
	fn keep(&self, session: Session, files: &[FileId]) {
		let _ = files;
		drop(session);
	}
}

/// A sink that does nothing (`fa check`).
pub struct NoSink;

impl UnitSink for NoSink {
	fn unit(&self, _session: &mut Session, _files: &[FileId]) {}
}

/// The options of a run.
#[derive(Clone, Debug, Default)]
pub struct Options {
	/// The module of the application's entry (`application/main`).
	pub entry: Option<String>,
	/// The number of threads (`None`: the worker pool's).
	pub threads: Option<usize>,
	/// Shuffles the ready units of equal priority (`FA_SCHEDULE_SEED`).
	pub seed: u64,
	/// The bytes of trees kept from the scan to their unit (beyond, a tree
	/// is parsed again). `None`: 128 MB.
	pub tree_budget: Option<usize>,
	/// An analysis for another target, after the run `previous` (§4 #19):
	/// a unit is analyzed again only when one of its files read the target,
	/// or when the interface of a unit it reads changed; the others keep
	/// the previous run's interfaces and diagnostics.
	pub retarget: bool,
	/// An incremental run after the run `previous` (the language server):
	/// these modules are read and analyzed again, a new module too; any
	/// other unit keeps the previous run's interfaces and diagnostics,
	/// unless the interface of a unit it reads changed.
	pub dirty: Option<FxHashSet<String>>,
	/// Measures the time of the units as the system's scheduler sees it
	/// ([`Stats::scheduled`]; `--timings`).
	pub measure: bool,
}

/// What a module of the run gave.
#[derive(Clone, Debug)]
pub struct ModuleReport {
	/// Its path (`library/math`).
	pub module: String,
	/// Its role.
	pub role: FileRole,
	/// Its diagnostics, sorted (their files are the unit's: irrelevant).
	pub diagnostics: Vec<Diagnostic>,
	/// Whether its analysis read `target` or `configuration`.
	pub reads_target: bool,
	/// Whether it was analyzed in this run (not kept from a previous one).
	pub analyzed: bool,
}

impl ModuleReport {
	/// Whether it has an error.
	pub fn has_errors(&self) -> bool {
		self.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.severity == crate::Severity::Error)
	}
}

/// What a run gives.
pub struct Report {
	/// Every module analyzed (configuration files excepted), in path order.
	pub modules: Vec<ModuleReport>,
	/// The module-level values to initialize, in order: `(module path,
	/// declaration)`.
	pub init_order: Vec<(String, NodeId)>,
	/// The entry module, if it exists.
	pub entry: Option<String>,
	/// The test files, in path order.
	pub tests: Vec<String>,
	/// What the units shared (the interfaces, for another target's run).
	pub shared: Arc<Shared>,
	/// Whether the run was stopped by the cancel flag (between units): its
	/// results are partial.
	pub cancelled: bool,
	/// Sizes and times, for `--timings`.
	pub stats: Stats,
}

impl Report {
	/// Whether a module has an error.
	pub fn has_errors(&self) -> bool {
		self.modules.iter().any(ModuleReport::has_errors)
	}

	/// The published interface of a module (`root`: the session of the run).
	pub fn interface<'r>(&'r self, root: &Session, module: &str) -> Option<&'r Interface> {
		let id = root.types.existing_module(module)?;
		self.shared.interface(id)
	}

	/// The name the entry function of a module is exported as, if it
	/// exports one (`main.js` calls it).
	pub fn entry_export(&self, root: &Session, module: &str) -> Option<String> {
		let interface = self.interface(root, module)?;
		let export = interface
			.export
			.filter(|export| export.kind == crate::symbols::SymbolKind::Function)?;
		Some(
			interface
				.export_names
				.get(&export.decl)
				.cloned()
				.unwrap_or_default(),
		)
	}

	/// The modules analyzed again for another target: those whose analysis
	/// read the target, and the modules reading them (directly or not).
	pub fn target_dependent(&self) -> FxHashSet<String> {
		let mut dirty: FxHashSet<String> = self
			.modules
			.iter()
			.filter(|report| report.reads_target)
			.map(|report| report.module.clone())
			.collect();
		if dirty.is_empty() {
			return dirty;
		}
		// the readers of each module
		let mut readers: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
		for (module, scan) in &self.shared.scans {
			for dependency in &scan.dependencies {
				readers.entry(dependency.as_str()).or_default().push(module);
			}
		}
		let mut stack: Vec<String> = dirty.iter().cloned().collect();
		while let Some(module) = stack.pop() {
			for &reader in readers.get(module.as_str()).map_or(&[][..], Vec::as_slice) {
				if dirty.insert(reader.to_string()) {
					stack.push(reader.to_string());
				}
			}
		}
		dirty
	}

	/// A session of modules of the run analyzed again, alone, against the
	/// published interfaces (deterministic, like `Session::rehydrate`): the
	/// LLVM backend lowers the units the entry reaches, and the instances of
	/// generic functions other units request (design/parallel-pipeline.md
	/// §3.3). `root`: the session of the run. The files of the modules
	/// found, in the order given.
	pub fn reanalyze(&self, root: &mut Session, modules: &[&str]) -> (Session, Vec<FileId>) {
		self.reanalyzer(root).analyze(modules)
	}

	/// What analyzes modules of the run again, from any thread
	/// ([`Report::reanalyze`]).
	pub fn reanalyzer<'r>(&'r self, root: &'r mut Session) -> Reanalyzer<'r> {
		let prelude = root.global_prelude();
		Reanalyzer {
			report: self,
			root,
			prelude,
		}
	}

	/// The report of a module.
	pub fn module(&self, module: &str) -> Option<&ModuleReport> {
		self.modules
			.binary_search_by(|report| report.module.as_str().cmp(module))
			.ok()
			.map(|index| &self.modules[index])
	}
}

/// Analyzes modules of a run again against its published interfaces
/// ([`Report::reanalyzer`]); shared by threads.
pub struct Reanalyzer<'r> {
	report: &'r Report,
	root: &'r Session,
	prelude: Vec<Option<TypeId>>,
}

impl Reanalyzer<'_> {
	/// A session of these modules analyzed again, and their files (in the
	/// order given, the modules found).
	pub fn analyze(&self, modules: &[&str]) -> (Session, Vec<FileId>) {
		let root = self.root;
		let mut session = root.unit_session(Arc::clone(&self.report.shared), self.prelude.clone());
		let mut files = Vec::with_capacity(modules.len());
		if let Some(resolver) = root.resolver_arc() {
			for &path in modules {
				let file = FileId::new(session.file_count());
				let Some(text) = resolver.source(path) else {
					continue;
				};
				let parsed = parse_module(file, path, text, Some(&*resolver));
				files.push(session.add_module(path, parsed));
			}
		}
		let visible = session.visible_modules(&files);
		session.types.set_visible(Some(visible));
		session.settle_unit(&files);
		(session, files)
	}
}

/// Sizes and times of a run.
#[derive(Clone, Debug, Default)]
pub struct Stats {
	/// The time of the scan (read, parse, scan every file).
	pub scan: Duration,
	/// The time of the units (analysis, merge, sink).
	pub units_time: Duration,
	/// The number of units.
	pub units: usize,
	/// The largest unit: its first module and its files.
	pub largest: (String, usize),
	/// The most units in flight at once.
	pub peak_in_flight: usize,
	/// The types of the global store at the end.
	pub global_types: usize,
	/// The trees parsed again when their unit ran (beyond the budget).
	pub parsed_again: usize,
	/// The worker threads of the units.
	pub threads: usize,
	/// The time the workers spent in units, summed over the threads: the
	/// rest of `threads × units_time` they were idle (no unit ready).
	pub busy: Duration,
	/// Of `busy`: the sessions made and the trees parsed again.
	pub setup: Duration,
	/// Of `busy`: the analyses.
	pub analysis: Duration,
	/// Of `busy`: the merges into the global store and the publications.
	pub merge: Duration,
	/// Of `busy`: the sinks (code generation).
	pub sink: Duration,
	/// Of `busy`: the reports of the modules and the drops of the sessions.
	pub teardown: Duration,
	/// Of `busy`, as the system's scheduler saw it (Linux, with
	/// [`Options::measure`]): the time on a core, and the time waiting for
	/// one (other processes); the rest, the workers were blocked (locks,
	/// evaluation threads, page faults).
	pub scheduled: Option<(Duration, Duration)>,
}

/// One unit of the graph.
struct UnitNode {
	/// Its modules (indices into the run's modules), in path order.
	modules: Vec<usize>,
	/// The units it reads.
	dependencies: Vec<usize>,
	/// The units that read it.
	dependents: Vec<usize>,
	/// The longest chain of source bytes from it through its dependents.
	critical: u64,
}

/// A ready unit in the queue: the longest critical chain first, then a
/// deterministic order (shuffled by the seed).
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Ready {
	critical: u64,
	order: std::cmp::Reverse<u64>,
	unit: usize,
}

impl Report {
	/// The report of a run that analyzed nothing (no module system).
	fn empty() -> Report {
		Report {
			modules: Vec::new(),
			init_order: Vec::new(),
			entry: None,
			tests: Vec::new(),
			shared: Arc::new(Shared::default()),
			cancelled: false,
			stats: Stats::default(),
		}
	}
}

/// Analyzes every module of the package of `root` (its resolver, target and
/// configuration set) in parallel, unit by unit; each unit goes to `sink`.
pub fn analyze_package(
	root: &mut Session,
	options: &Options,
	previous: Option<&Report>,
	sink: &dyn UnitSink,
) -> Report {
	let Some(resolver) = root.resolver_arc() else {
		return Report::empty();
	};
	root.set_entry(options.entry.as_deref());
	let mut stats = Stats::default();
	let reuse = (options.retarget || options.dirty.is_some())
		.then_some(previous)
		.flatten();

	// 1. the scan
	let started = Instant::now();
	let (paths, scanned) = scan_modules(&resolver, options, reuse);
	stats.scan = started.elapsed();
	// the modules, interned in the global store: their ids are global everywhere
	let global = Arc::clone(root.types.global());
	let module_ids: Vec<ModuleId> = paths
		.iter()
		.map(|path| crate::types::store::intern_module(&global, path))
		.collect();

	// 2. the graph and its units
	let index: FxHashMap<&str, usize> = paths
		.iter()
		.enumerate()
		.map(|(position, path)| (path.as_str(), position))
		.collect();
	let units = unit_graph(&index, &scanned);
	stats.units = units.len();
	if let Some(largest) = units.iter().max_by_key(|unit| unit.modules.len()) {
		stats.largest = (paths[largest.modules[0]].clone(), largest.modules.len());
	}
	let shared = share(&paths, &scanned, &module_ids, previous);
	let prelude = root.global_prelude();
	root.set_shared(Some(Arc::clone(&shared)));

	// 3. the units, in parallel
	let started = Instant::now();
	let runner = Runner {
		base: &*root,
		shared: &shared,
		prelude: &prelude,
		resolver: &resolver,
		paths: &paths,
		scanned: &scanned,
		units: &units,
		reports: paths.iter().map(|_| Mutex::new(None)).collect(),
		pending: units
			.iter()
			.map(|unit| AtomicUsize::new(unit.dependencies.len()))
			.collect(),
		ready: Mutex::new(BinaryHeap::new()),
		in_flight: AtomicUsize::new(0),
		peak: AtomicUsize::new(0),
		parsed_again: AtomicUsize::new(0),
		seed: options.seed,
		measure: options.measure,
		reuse,
		dirty: options.dirty.as_ref(),
		cancelled: AtomicBool::new(false),
		changed: (0..units.len()).map(|_| AtomicBool::new(false)).collect(),
		module_ids: &module_ids,
		sink,
		times: Times::default(),
	};
	stats.threads = runner.run(options.threads);
	stats.units_time = started.elapsed();
	let cancelled = runner.cancelled.load(Ordering::Relaxed);
	runner.record(&mut stats);
	let mut modules = module_reports(runner.reports, &paths, previous);

	// 4. the program, from the interfaces
	let entry = options
		.entry
		.as_ref()
		.filter(|entry| index.contains_key(entry.as_str()))
		.cloned();
	let tests: Vec<String> = paths
		.iter()
		.filter(|path| resolver.role(path) == FileRole::Test)
		.cloned()
		.collect();
	let program = Program {
		resolver: &*resolver,
		paths: &paths,
		index: &index,
		module_ids: &module_ids,
		entry: entry.as_deref(),
		tests: &tests,
	};
	let init_order = program.analyze(root, &mut modules);
	stats.global_types = global.types.len();
	root.set_shared(None);
	Report {
		modules,
		init_order,
		entry,
		tests,
		shared,
		cancelled,
		stats,
	}
}

/// Reads, parses and scans the modules of the package, in parallel: the
/// modules it lists, then the modules they read that it doesn't list (its
/// dependencies' files), until none is left. A tree is kept for its unit
/// while the trees kept fit the budget. Gives the modules in path order.
///
/// The units start once every module is scanned: the units are the
/// strongly connected components of the whole graph, and a unit can't be
/// known complete before the modules it reaches are scanned. The scan is
/// parallel and short next to the analysis (a twentieth of it for the
/// generated benchmark package), so the cores are busy throughout.
fn scan_modules(
	resolver: &Arc<dyn crate::modules::ModuleResolver>,
	options: &Options,
	reuse: Option<&Report>,
) -> (Vec<String>, Vec<Scanned>) {
	let budget = options.tree_budget.unwrap_or(128 << 20);
	let kept_bytes = AtomicUsize::new(0);
	let dirty = options.dirty.as_ref();
	let scan_one = |path: &String| -> ScanResult {
		// another target, or a module not changed: the scans are the same,
		// a unit parses its files
		if let Some(previous) = reuse
			&& !dirty.is_some_and(|dirty| dirty.contains(path))
			&& let Some(scan) = previous.shared.scans.get(path)
		{
			return (Arc::clone(scan), None, scan.bytes);
		}
		let Some(text) = resolver.source(path) else {
			return (Arc::new(FileScan::default()), None, 0);
		};
		let bytes = text.len();
		let parsed = parse_module(FileId::new(0), path, text, Some(&**resolver));
		let scan = Arc::new(parsed.scan.clone());
		let size = parsed.tree_bytes();
		let keep = kept_bytes.fetch_add(size, Ordering::Relaxed) + size <= budget;
		if !keep {
			kept_bytes.fetch_sub(size, Ordering::Relaxed);
		}
		(scan, keep.then_some(parsed), bytes)
	};
	let is_module = |path: &str| !matches!(resolver.role(path), FileRole::Configuration(_));
	let mut frontier: Vec<String> = resolver
		.modules()
		.into_iter()
		.filter(|module| is_module(module))
		.collect();
	frontier.sort();
	frontier.dedup();
	let mut found: FxHashMap<String, ScanResult> = FxHashMap::default();
	while !frontier.is_empty() {
		let batch: Vec<ScanResult> = crate::parallel::install(|| {
			use rayon::prelude::*;
			frontier.par_iter().map(scan_one).collect()
		});
		let mut next: Vec<String> = Vec::new();
		for (path, result) in frontier.drain(..).zip(batch) {
			for dependency in &result.0.dependencies {
				if !found.contains_key(dependency) && *dependency != path && is_module(dependency) {
					next.push(dependency.clone());
				}
			}
			found.insert(path, result);
		}
		next.sort();
		next.dedup();
		next.retain(|path| !found.contains_key(path));
		frontier = next;
	}
	let mut found: Vec<(String, ScanResult)> = found.into_iter().collect();
	found.sort_by(|a, b| a.0.cmp(&b.0));
	found
		.into_iter()
		.map(|(path, (scan, parsed, bytes))| (path, (scan, Mutex::new(parsed), bytes)))
		.unzip()
}

/// The units of the module graph (its strongly connected components,
/// dependencies first), with their dependencies, dependents and critical
/// chains. `index`: the position of each module path.
fn unit_graph(index: &FxHashMap<&str, usize>, scanned: &[Scanned]) -> Vec<UnitNode> {
	let edges: Vec<Vec<usize>> = scanned
		.iter()
		.map(|(scan, _, _)| {
			scan.dependencies
				.iter()
				.filter_map(|dependency| index.get(dependency.as_str()).copied())
				.collect()
		})
		.collect();
	let bytes: Vec<u64> = scanned.iter().map(|(_, _, bytes)| *bytes as u64).collect();
	units_of(&edges, &bytes)
}

/// The units of a graph (`edges[i]`: the nodes node `i` reads; `bytes[i]`:
/// its size, the estimate of its analysis time).
///
/// A unit's priority is its critical chain: its bytes plus the longest
/// critical chain among its dependents, so that the scheduler runs first
/// the units the most work waits for (the analysis time of a module is
/// roughly proportional to its bytes). The units stay the strongly
/// connected components: a file is the smallest piece a session analyzes
/// and publishes, and a cycle has to be analyzed in rounds within one
/// session.
fn units_of(edges: &[Vec<usize>], bytes: &[u64]) -> Vec<UnitNode> {
	let components = crate::graph::components(edges);
	let mut unit_of = vec![0; edges.len()];
	for (unit, modules) in components.iter().enumerate() {
		for &module in modules {
			unit_of[module] = unit;
		}
	}
	let mut units: Vec<UnitNode> = components
		.iter()
		.map(|modules| UnitNode {
			modules: modules.clone(),
			dependencies: Vec::new(),
			dependents: Vec::new(),
			critical: 0,
		})
		.collect();
	for (unit, modules) in components.iter().enumerate() {
		let mut dependencies: Vec<usize> = modules
			.iter()
			.flat_map(|&module| edges[module].iter().map(|&other| unit_of[other]))
			.filter(|&other| other != unit)
			.collect();
		dependencies.sort_unstable();
		dependencies.dedup();
		for &dependency in &dependencies {
			units[dependency].dependents.push(unit);
		}
		units[unit].dependencies = dependencies;
	}
	// the components come dependencies first: their dependents are later
	for unit in (0..units.len()).rev() {
		let own: u64 = units[unit]
			.modules
			.iter()
			.map(|&module| bytes[module])
			.sum();
		let longest = units[unit]
			.dependents
			.iter()
			.map(|&dependent| units[dependent].critical)
			.max()
			.unwrap_or(0);
		units[unit].critical = own + longest;
	}
	units
}

/// What the units share: a slot for the interface of each module, the
/// scans, and the origins of the global types of a previous run (the
/// modules it analyzed and this one doesn't are published again from it).
fn share(
	paths: &[String],
	scanned: &[Scanned],
	module_ids: &[ModuleId],
	previous: Option<&Report>,
) -> Arc<Shared> {
	let mut shared = Shared::default();
	for (position, path) in paths.iter().enumerate() {
		shared
			.interfaces
			.insert(module_ids[position], OnceLock::new());
		shared
			.scans
			.insert(path.clone(), Arc::clone(&scanned[position].0));
	}
	if let Some(previous) = previous {
		shared.origins = previous.shared.origins.snapshot();
	}
	Arc::new(shared)
}

/// The reports of the modules, in path order: those of this run, else the
/// previous run's (a module not analyzed again keeps its report).
fn module_reports(
	reports: Vec<Mutex<Option<ModuleReport>>>,
	paths: &[String],
	previous: Option<&Report>,
) -> Vec<ModuleReport> {
	let mut modules: Vec<ModuleReport> = Vec::with_capacity(paths.len());
	for (position, report) in reports.into_iter().enumerate() {
		let report = report
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		match report {
			Some(report) => modules.push(report),
			None => {
				if let Some(old) = previous.and_then(|previous| previous.module(&paths[position])) {
					let mut old = old.clone();
					old.analyzed = false;
					modules.push(old);
				}
			}
		}
	}
	modules
}

/// The analysis of the whole program, once its units are published: the
/// initialization order and its cycles (`MOD007`), the entry function
/// (`MOD015`), the problems of the scan (`MOD001`–`MOD003`).
struct Program<'p> {
	resolver: &'p dyn crate::modules::ModuleResolver,
	paths: &'p [String],
	index: &'p FxHashMap<&'p str, usize>,
	module_ids: &'p [ModuleId],
	entry: Option<&'p str>,
	tests: &'p [String],
}

impl Program<'_> {
	/// Adds the program's diagnostics to the reports of their modules, and
	/// gives the module-level values to initialize, in order.
	fn analyze(&self, root: &mut Session, modules: &mut [ModuleReport]) -> Vec<(String, NodeId)> {
		let entry_id = self.entry.map(|entry| self.module_ids[self.index[entry]]);
		let test_ids: Vec<ModuleId> = self
			.tests
			.iter()
			.map(|test| self.module_ids[self.index[test.as_str()]])
			.collect();
		let program_ids: Vec<ModuleId> = self
			.paths
			.iter()
			.enumerate()
			.filter(|(_, path)| self.resolver.role(path) != FileRole::Test)
			.map(|(position, _)| self.module_ids[position])
			.collect();
		// a fresh view: the structures of the root's own view (its prelude,
		// its configuration) would not be the published ones
		let global = Arc::clone(root.types.global());
		let root_view = std::mem::replace(&mut root.types, crate::types::TypeStore::view(&global));
		let (order, cycles) = root.initialization_order_of(&program_ids, entry_id, &test_ids);
		let mut extra: FxHashMap<String, Vec<Diagnostic>> = FxHashMap::default();
		for (module, diagnostic) in root.initializer_cycle_diagnostics(&cycles) {
			extra.entry(module).or_default().push(diagnostic);
		}
		for (module, diagnostic) in root.worker_diagnostics(&program_ids) {
			extra.entry(module).or_default().push(diagnostic);
		}
		// the API file of a C library is the entry of its build: no entry function
		let library_api = crate::c_library::configured(root.comptime.configuration.as_ref())
			.is_some_and(|library| Some(library.api_module().as_str()) == self.entry);
		if let (Some(entry_id), Some(entry)) = (entry_id, self.entry)
			&& !library_api
			&& let Some(diagnostic) = root.entry_function_diagnostic(entry_id, true)
		{
			extra.entry(entry.to_string()).or_default().push(diagnostic);
		}
		for (module, code, message) in self.resolver.scan_diagnostics() {
			let code = match code {
				"MOD001" => &crate::diagnostics::codes::advanced::MOD001,
				"MOD002" => &crate::diagnostics::codes::advanced::MOD002,
				_ => &crate::diagnostics::codes::advanced::MOD003,
			};
			let location = Location {
				file: FileId::new(0),
				span: SourceSpan::new(0, 0),
			};
			let diagnostic = Diagnostic::new(code, location, message);
			extra.entry(module).or_default().push(diagnostic);
		}
		for report in modules.iter_mut() {
			if let Some(found) = extra.remove(&report.module) {
				add_program_diagnostics(report, found);
			}
		}
		let init_order = order
			.into_iter()
			.filter_map(|key| {
				let (module, decl) =
					root.external_node(crate::symbols::Resolution::External(key))?;
				Some((root.types.module_path(module).to_string(), decl))
			})
			.collect();
		root.types = root_view;
		init_order
	}
}

/// Adds diagnostics of the program to a module's report, once each, and
/// sorts them.
fn add_program_diagnostics(report: &mut ModuleReport, found: Vec<Diagnostic>) {
	for diagnostic in found {
		let duplicate = report.diagnostics.iter().any(|other| {
			std::ptr::eq(other.code, diagnostic.code)
				&& other.location.span == diagnostic.location.span
				&& other.message == diagnostic.message
		});
		// `MOD007` once per file (the walk may have reported it)
		let cycle_known = std::ptr::eq(
			diagnostic.code,
			&crate::diagnostics::codes::advanced::MOD007,
		) && report
			.diagnostics
			.iter()
			.any(|other| std::ptr::eq(other.code, diagnostic.code));
		if !duplicate && !cycle_known {
			report.diagnostics.push(diagnostic);
		}
	}
	crate::diagnostics::sort(&mut report.diagnostics);
}

/// The times of the units of a run, summed over the worker threads (in
/// nanoseconds).
#[derive(Default)]
struct Times {
	busy: AtomicU64,
	setup: AtomicU64,
	analysis: AtomicU64,
	merge: AtomicU64,
	sink: AtomicU64,
	teardown: AtomicU64,
	on_cpu: AtomicU64,
	runnable: AtomicU64,
}

impl Times {
	/// Adds the time since `since` to a counter; gives the time now.
	fn add(counter: &AtomicU64, since: Instant) -> Instant {
		let now = Instant::now();
		counter.fetch_add((now - since).as_nanos() as u64, Ordering::Relaxed);
		now
	}
}

/// The time the current thread ran on a core and waited for one, in
/// nanoseconds (Linux: `/proc/thread-self/schedstat`).
fn schedstat() -> Option<(u64, u64)> {
	let text = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
	let mut fields = text
		.split_whitespace()
		.map(|field| field.parse::<u64>().ok());
	Some((fields.next()??, fields.next()??))
}

/// The state of the units of a run, shared by the worker threads.
struct Runner<'r> {
	base: &'r Session,
	shared: &'r Arc<Shared>,
	prelude: &'r [Option<TypeId>],
	resolver: &'r Arc<dyn crate::modules::ModuleResolver>,
	paths: &'r [String],
	scanned: &'r [Scanned],
	units: &'r [UnitNode],
	reports: Vec<Mutex<Option<ModuleReport>>>,
	/// The units each unit waits for.
	pending: Vec<AtomicUsize>,
	ready: Mutex<BinaryHeap<Ready>>,
	in_flight: AtomicUsize,
	peak: AtomicUsize,
	parsed_again: AtomicUsize,
	seed: u64,
	/// Whether the times as the system's scheduler saw them are measured.
	measure: bool,
	/// Another target's run, or an incremental one: the previous run.
	reuse: Option<&'r Report>,
	/// An incremental run: the modules analyzed again.
	dirty: Option<&'r FxHashSet<String>>,
	/// Whether the cancel flag stopped the run.
	cancelled: AtomicBool,
	/// Whether the interface of each unit changed from the previous run.
	changed: Vec<AtomicBool>,
	module_ids: &'r [ModuleId],
	sink: &'r dyn UnitSink,
	times: Times,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
	mutex
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Runner<'_> {
	/// Runs every unit on `threads` worker threads (`None`: the pool's);
	/// gives the number of threads.
	fn run(&self, threads: Option<usize>) -> usize {
		match threads {
			Some(threads) => {
				let pool = rayon::ThreadPoolBuilder::new()
					.num_threads(threads)
					.stack_size(crate::parallel::STACK_SIZE)
					.build()
					.expect("the worker threads can be started");
				pool.in_place_scope(|scope| self.launch(scope));
				threads
			}
			None => {
				let pool = crate::parallel::pool();
				pool.in_place_scope(|scope| self.launch(scope));
				pool.current_num_threads()
			}
		}
	}

	/// Copies the sizes and times of the run into `stats`.
	fn record(&self, stats: &mut Stats) {
		let time = |counter: &AtomicU64| Duration::from_nanos(counter.load(Ordering::Relaxed));
		stats.peak_in_flight = self.peak.load(Ordering::Relaxed);
		stats.parsed_again = self.parsed_again.load(Ordering::Relaxed);
		stats.busy = time(&self.times.busy);
		stats.setup = time(&self.times.setup);
		stats.analysis = time(&self.times.analysis);
		stats.merge = time(&self.times.merge);
		stats.sink = time(&self.times.sink);
		stats.teardown = time(&self.times.teardown);
		stats.scheduled = self
			.measure
			.then(|| (time(&self.times.on_cpu), time(&self.times.runnable)));
	}

	/// The order of units of equal priority: theirs, or shuffled by the seed.
	fn order(&self, unit: usize) -> u64 {
		if self.seed == 0 {
			return unit as u64;
		}
		// splitmix64 of the seed and the unit
		let mut x = self
			.seed
			.wrapping_add((unit as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
		x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
		x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
		x ^ (x >> 31)
	}

	/// Queues the units that depend on nothing, a task for each.
	fn launch<'s>(&'s self, scope: &rayon::Scope<'s>) {
		let initially: Vec<usize> = (0..self.units.len())
			.filter(|&unit| self.pending[unit].load(Ordering::Relaxed) == 0)
			.collect();
		for &unit in &initially {
			self.make_ready(unit);
		}
		for _ in &initially {
			scope.spawn(move |scope| self.task(scope));
		}
	}

	/// Queues a unit whose dependencies are done.
	fn make_ready(&self, unit: usize) {
		lock(&self.ready).push(Ready {
			critical: self.units[unit].critical,
			order: std::cmp::Reverse(self.order(unit)),
			unit,
		});
	}

	/// For another target's run: whether a unit keeps the results of the
	/// previous run (none of its files read the target, and the interfaces
	/// of the units it reads didn't change); then its interfaces are
	/// published again.
	fn skipped(&self, unit: usize) -> bool {
		let Some(previous) = self.reuse else {
			return false;
		};
		let modules = &self.units[unit].modules;
		// another target: the modules that read it; an incremental run: the
		// modules changed (and the new ones)
		let forced = modules.iter().any(|&module| {
			let path = &self.paths[module];
			let report = previous.module(path);
			match self.dirty {
				Some(dirty) => dirty.contains(path) || report.is_none(),
				None => report.is_none_or(|report| report.reads_target),
			}
		});
		let changed = self.units[unit]
			.dependencies
			.iter()
			.any(|&dependency| self.changed[dependency].load(Ordering::Acquire));
		if forced || changed {
			return false;
		}
		for &module in modules {
			let id = self.module_ids[module];
			if let (Some(interface), Some(slot)) = (
				previous.shared.interfaces.get(&id).and_then(OnceLock::get),
				self.shared.interfaces.get(&id),
			) {
				let _ = slot.set(Arc::clone(interface));
			}
		}
		true
	}

	/// For another target's run: whether the interfaces a unit published
	/// differ from the previous run's (its readers are analyzed again).
	fn note_changes(&self, unit: usize) {
		let Some(previous) = self.reuse else {
			return;
		};
		let types = crate::types::TypeStore::view(self.base.types.global());
		let changed = self.units[unit].modules.iter().any(|&module| {
			let id = self.module_ids[module];
			let hash = |interfaces: &FxHashMap<ModuleId, OnceLock<Arc<Interface>>>| {
				interfaces
					.get(&id)
					.and_then(OnceLock::get)
					.map(|interface| interface.hash(&types))
			};
			hash(&previous.shared.interfaces) != hash(&self.shared.interfaces)
		});
		self.changed[unit].store(changed, Ordering::Release);
	}

	/// A task: runs the best ready unit (the dependents it makes ready get
	/// a task each).
	fn task<'s>(&'s self, scope: &rayon::Scope<'s>) {
		let Some(next) = lock(&self.ready).pop() else {
			return;
		};
		let unit = next.unit;
		let started = Instant::now();
		let scheduled = if self.measure { schedstat() } else { None };
		let flying = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
		self.peak.fetch_max(flying, Ordering::Relaxed);
		// stopped (the language server got an edit): the rest is not analyzed
		let cancelled = self.cancelled.load(Ordering::Relaxed)
			|| self.base.cancel_flag().load(Ordering::Relaxed);
		if cancelled {
			self.cancelled.store(true, Ordering::Relaxed);
			self.release(unit, false, scope);
		} else if self.skipped(unit) {
			self.release(unit, false, scope);
		} else {
			for report in self.analyze_unit(unit, scope) {
				if let Ok(position) = self.paths.binary_search(&report.module) {
					*lock(&self.reports[position]) = Some(report);
				}
			}
		}
		self.in_flight.fetch_sub(1, Ordering::Relaxed);
		Times::add(&self.times.busy, started);
		if let Some(before) = scheduled
			&& let Some(after) = schedstat()
		{
			let times = &self.times;
			times
				.on_cpu
				.fetch_add(after.0 - before.0, Ordering::Relaxed);
			times
				.runnable
				.fetch_add(after.1 - before.1, Ordering::Relaxed);
		}
	}

	/// The interfaces of a unit are published (or kept from the previous
	/// run): the dependents it was the last wait of are ready, a task each.
	fn release<'s>(&'s self, unit: usize, analyzed: bool, scope: &rayon::Scope<'s>) {
		if analyzed {
			self.note_changes(unit);
		}
		for &dependent in &self.units[unit].dependents {
			if self.pending[dependent].fetch_sub(1, Ordering::AcqRel) == 1 {
				self.make_ready(dependent);
				scope.spawn(move |scope| self.task(scope));
			}
		}
	}

	/// Analyzes one unit in a session of its own, publishes its interfaces
	/// and releases its dependents, hands the session to the sink, and
	/// reports its modules.
	///
	/// The dependents start as soon as the interfaces are published: they
	/// read nothing else of the unit, so its sink (code generation), its
	/// reports and the drop of its session overlap with them.
	fn analyze_unit<'s>(&'s self, unit: usize, scope: &rayon::Scope<'s>) -> Vec<ModuleReport> {
		let started = Instant::now();
		let mut session = self
			.base
			.unit_session(Arc::clone(self.shared), self.prelude.to_vec());
		let files = self.add_files(&mut session, unit);
		// the files the unit may read, for the names of global types (types/store.rs)
		let visible = session.visible_modules(&files);
		session.types.set_visible(Some(visible));
		let started = Times::add(&self.times.setup, started);
		session.settle_unit(&files);
		let started = Times::add(&self.times.analysis, started);
		session.publish_unit(&files);
		self.release(unit, true, scope);
		let started = Times::add(&self.times.merge, started);
		self.sink.unit(&mut session, &files);
		let started = Times::add(&self.times.sink, started);
		let reports = files
			.iter()
			.map(|&file| ModuleReport {
				module: session.module(file).to_string(),
				role: session.role(file),
				diagnostics: session.diagnostics(file).to_vec(),
				reads_target: session.analysis(file).reads_target,
				analyzed: true,
			})
			.collect();
		self.sink.keep(session, &files);
		Times::add(&self.times.teardown, started);
		reports
	}

	/// Adds the modules of a unit to its session: the trees kept from the
	/// scan, the others parsed again.
	fn add_files(&self, session: &mut Session, unit: usize) -> Vec<FileId> {
		let mut files = Vec::with_capacity(self.units[unit].modules.len());
		for &module in &self.units[unit].modules {
			let path = self.paths[module].as_str();
			let file = FileId::new(session.file_count());
			let parsed = match lock(&self.scanned[module].1).take() {
				Some(mut parsed) => {
					for diagnostic in &mut parsed.diagnostics {
						diagnostic.set_file(file);
					}
					parsed
				}
				None => {
					self.parsed_again.fetch_add(1, Ordering::Relaxed);
					match self.resolver.source(path) {
						Some(text) => parse_module(file, path, text, Some(&**self.resolver)),
						None => continue,
					}
				}
			};
			files.push(session.add_module(path, parsed));
		}
		files
	}
}

#[cfg(test)]
mod tests;
