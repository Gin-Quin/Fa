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
//!    store and publishes the interfaces ([`Shared`]), then hands the
//!    session to the [`UnitSink`] (diagnostics, code generation) and drops
//!    it: its trees, side tables and local types.
//! 4. **Program**: the initialization order and `MOD007`, the entry
//!    function (`MOD015`) and the problems of the scan, from the interfaces.
//!
//! Outputs don't depend on the threads: each module's diagnostics are
//! sorted, the report lists the modules in path order, and nothing is
//! ordered by a store id (P3). `FA_SCHEDULE_SEED` shuffles the order of
//! ready units of equal priority (the determinism test, P10).

use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostics::{Diagnostic, Location, SourceSpan};
use crate::ids::{FileId, ModuleId, NodeId, TypeId};
use crate::modules::{FileRole, Interface};
use crate::scan::FileScan;
use crate::session::{Parsed, Session, parse_module};

/// What the scan of a module gives: its scan, its tree when it is kept
/// (the tree budget), its source bytes.
type ScanResult = (Arc<FileScan>, Option<Parsed>, usize);

/// A module scanned, its tree taken by its unit.
type Scanned = (Arc<FileScan>, Mutex<Option<Parsed>>, usize);

/// The time spent in the sinks of the runs of this process (`--timings`).
static SINK_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What the units of a run share: the published interfaces, the scans,
/// the declarations of the global types (`Session::type_origins`).
#[derive(Default)]
pub struct Shared {
	/// The interface of each module, once its unit is published.
	pub(crate) interfaces: FxHashMap<ModuleId, OnceLock<Arc<Interface>>>,
	/// The scan of each module, by path.
	pub(crate) scans: FxHashMap<String, Arc<FileScan>>,
	/// The declarations of each global type of a published module.
	pub(crate) origins: std::sync::RwLock<FxHashMap<TypeId, Vec<crate::ids::DeclKey>>>,
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
	fn keep(&self, session: Session) {
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

	/// The report of a module.
	pub fn module(&self, module: &str) -> Option<&ModuleReport> {
		self.modules
			.binary_search_by(|report| report.module.as_str().cmp(module))
			.ok()
			.map(|index| &self.modules[index])
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
	/// The time spent in the sink (code generation), summed over the threads.
	pub sink: Duration,
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

/// Analyzes every module of the package of `root` (its resolver, target and
/// configuration set) in parallel, unit by unit; each unit goes to `sink`.
pub fn analyze_package(
	root: &mut Session,
	options: &Options,
	previous: Option<&Report>,
	sink: &dyn UnitSink,
) -> Report {
	let Some(resolver) = root.resolver_arc() else {
		return Report {
			modules: Vec::new(),
			init_order: Vec::new(),
			entry: None,
			tests: Vec::new(),
			shared: Arc::new(Shared::default()),
			cancelled: false,
			stats: Stats::default(),
		};
	};
	let mut stats = Stats::default();
	// 1. scan, in parallel: the modules of the package, then the modules
	// they read that it doesn't list (its dependencies' files), until none
	// is left; the trees kept within the budget
	let started = Instant::now();
	let budget = options.tree_budget.unwrap_or(128 << 20);
	let kept_bytes = AtomicUsize::new(0);
	let dirty = options.dirty.as_ref();
	let reuse = (options.retarget || dirty.is_some())
		.then_some(previous)
		.flatten();
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
		let parsed = parse_module(FileId::new(0), path, text, Some(&*resolver));
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
	let mut paths: Vec<String> = Vec::with_capacity(found.len());
	let mut scanned: Vec<Scanned> = Vec::with_capacity(found.len());
	for (path, (scan, parsed, bytes)) in found {
		paths.push(path);
		scanned.push((scan, Mutex::new(parsed), bytes));
	}
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
	let edges: Vec<Vec<usize>> = scanned
		.iter()
		.map(|(scan, _, _)| {
			scan.dependencies
				.iter()
				.filter_map(|dependency| index.get(dependency.as_str()).copied())
				.collect()
		})
		.collect();
	let components = crate::graph::components(&edges);
	let mut unit_of = vec![0; paths.len()];
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
	let mut pending: Vec<usize> = vec![0; units.len()];
	for (unit, modules) in components.iter().enumerate() {
		let mut dependencies: Vec<usize> = modules
			.iter()
			.flat_map(|&module| edges[module].iter().map(|&other| unit_of[other]))
			.filter(|&other| other != unit)
			.collect();
		dependencies.sort_unstable();
		dependencies.dedup();
		pending[unit] = dependencies.len();
		units[unit].dependencies = dependencies.clone();
		for dependency in dependencies {
			units[dependency].dependents.push(unit);
		}
	}
	// the components come dependencies first: their dependents are later
	for unit in (0..units.len()).rev() {
		let own: u64 = units[unit]
			.modules
			.iter()
			.map(|&module| scanned[module].2 as u64)
			.sum();
		let longest = units[unit]
			.dependents
			.iter()
			.map(|&dependent| units[dependent].critical)
			.max()
			.unwrap_or(0);
		units[unit].critical = own + longest;
	}
	stats.units = units.len();
	if let Some(largest) = units.iter().max_by_key(|unit| unit.modules.len()) {
		stats.largest = (paths[largest.modules[0]].clone(), largest.modules.len());
	}

	// what the units share; the interfaces of the modules not analyzed again
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
		let origins = previous
			.shared
			.origins
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.clone();
		*shared
			.origins
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner()) = origins;
	}
	let shared = Arc::new(shared);
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
		pending: pending.into_iter().map(AtomicUsize::new).collect(),
		ready: Mutex::new(BinaryHeap::new()),
		in_flight: AtomicUsize::new(0),
		peak: AtomicUsize::new(0),
		parsed_again: AtomicUsize::new(0),
		seed: options.seed,
		reuse,
		dirty,
		cancelled: AtomicBool::new(false),
		changed: (0..units.len()).map(|_| AtomicBool::new(false)).collect(),
		module_ids: &module_ids,
		sink,
	};
	match options.threads {
		Some(threads) => {
			let pool = rayon::ThreadPoolBuilder::new()
				.num_threads(threads)
				.stack_size(crate::parallel::STACK_SIZE)
				.build()
				.expect("the worker threads can be started");
			pool.in_place_scope(|scope| runner.launch(scope));
		}
		None => crate::parallel::pool().in_place_scope(|scope| runner.launch(scope)),
	}
	let Runner {
		reports,
		peak,
		parsed_again,
		cancelled,
		..
	} = runner;
	let cancelled = cancelled.into_inner();
	stats.units_time = started.elapsed();
	stats.peak_in_flight = peak.load(Ordering::Relaxed);
	stats.parsed_again = parsed_again.load(Ordering::Relaxed);
	stats.sink = Duration::from_nanos(SINK_NANOS.swap(0, Ordering::Relaxed));

	// the modules not analyzed again keep their previous reports
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
	let entry_id = entry
		.as_ref()
		.map(|entry| module_ids[index[entry.as_str()]]);
	let test_ids: Vec<ModuleId> = tests
		.iter()
		.map(|test| module_ids[index[test.as_str()]])
		.collect();
	let program_ids: Vec<ModuleId> = paths
		.iter()
		.enumerate()
		.filter(|(_, path)| resolver.role(path) != FileRole::Test)
		.map(|(position, _)| module_ids[position])
		.collect();
	// a fresh view: the structures of the root's own view (its prelude,
	// its configuration) would not be the published ones
	let root_view = std::mem::replace(&mut root.types, crate::types::TypeStore::view(&global));
	let (order, cycles) = root.initialization_order_of(&program_ids, entry_id, &test_ids);
	let mut extra: FxHashMap<String, Vec<Diagnostic>> = FxHashMap::default();
	for (module, diagnostic) in root.initializer_cycle_diagnostics(&cycles) {
		extra.entry(module).or_default().push(diagnostic);
	}
	if let Some(entry_id) = entry_id
		&& let Some(entry) = &entry
		&& let Some(diagnostic) = root.entry_function_diagnostic(entry_id, true)
	{
		extra.entry(entry.clone()).or_default().push(diagnostic);
	}
	for (module, code, message) in resolver.scan_diagnostics() {
		let code = match code {
			"MOD001" => &crate::diagnostics::codes::advanced::MOD001,
			"MOD002" => &crate::diagnostics::codes::advanced::MOD002,
			_ => &crate::diagnostics::codes::advanced::MOD003,
		};
		let diagnostic = Diagnostic::new(
			code,
			Location {
				file: FileId::new(0),
				span: SourceSpan::new(0, 0),
			},
			message,
		);
		extra.entry(module).or_default().push(diagnostic);
	}
	for report in &mut modules {
		if let Some(found) = extra.remove(&report.module) {
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
	}
	let init_order = order
		.into_iter()
		.filter_map(|key| {
			let (module, decl) = root.external_node(crate::symbols::Resolution::External(key))?;
			Some((root.types.module_path(module).to_string(), decl))
		})
		.collect();
	stats.global_types = global.types.len();
	root.types = root_view;
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
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
	mutex
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Runner<'_> {
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

	/// A task: runs the best ready unit, then spawns a task per unit it
	/// made ready.
	fn task<'s>(&'s self, scope: &rayon::Scope<'s>) {
		let Some(next) = lock(&self.ready).pop() else {
			return;
		};
		let unit = next.unit;
		let flying = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
		self.peak.fetch_max(flying, Ordering::Relaxed);
		// stopped (the language server got an edit): the rest is not analyzed
		let cancelled = self.cancelled.load(Ordering::Relaxed)
			|| self.base.cancel_flag().load(Ordering::Relaxed);
		if cancelled {
			self.cancelled.store(true, Ordering::Relaxed);
		} else if !self.skipped(unit) {
			let modules = &self.units[unit].modules;
			let inputs = modules
				.iter()
				.map(|&module| {
					let parsed = lock(&self.scanned[module].1).take();
					if parsed.is_none() {
						self.parsed_again.fetch_add(1, Ordering::Relaxed);
					}
					(self.paths[module].as_str(), parsed)
				})
				.collect();
			let analyzed = run_unit(
				self.base,
				self.shared,
				self.prelude,
				self.resolver,
				inputs,
				self.sink,
			);
			for report in analyzed {
				if let Ok(position) = self.paths.binary_search(&report.module) {
					*lock(&self.reports[position]) = Some(report);
				}
			}
			self.note_changes(unit);
		}
		self.in_flight.fetch_sub(1, Ordering::Relaxed);
		for &dependent in &self.units[unit].dependents {
			if self.pending[dependent].fetch_sub(1, Ordering::AcqRel) == 1 {
				self.make_ready(dependent);
				scope.spawn(move |scope| self.task(scope));
			}
		}
	}
}

/// Analyzes one unit in a session of its own, publishes its interfaces,
/// hands it to the sink, and reports its modules.
fn run_unit(
	base: &Session,
	shared: &Arc<Shared>,
	prelude: &[Option<TypeId>],
	resolver: &Arc<dyn crate::modules::ModuleResolver>,
	modules: Vec<(&str, Option<Parsed>)>,
	sink: &dyn UnitSink,
) -> Vec<ModuleReport> {
	let mut session = base.unit_session(Arc::clone(shared), prelude.to_vec());
	let mut files = Vec::with_capacity(modules.len());
	for (path, parsed) in modules {
		let file = FileId::new(session.file_count());
		let parsed = match parsed {
			Some(mut parsed) => {
				for diagnostic in &mut parsed.diagnostics {
					diagnostic.set_file(file);
				}
				parsed
			}
			None => match resolver.source(path) {
				Some(text) => parse_module(file, path, text, Some(&**resolver)),
				None => continue,
			},
		};
		files.push(session.add_module(path, parsed));
	}
	// the files the unit may read, for the names of global types (types/store.rs)
	let visible = session.visible_modules(&files);
	session.types.set_visible(Some(visible));
	session.settle_unit(&files);
	session.publish_unit(&files);
	let started = Instant::now();
	sink.unit(&mut session, &files);
	SINK_NANOS.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
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
	sink.keep(session);
	reports
}
