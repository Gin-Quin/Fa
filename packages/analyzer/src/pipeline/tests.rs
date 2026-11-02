//! The scheduler of the pipeline: its units, their priorities and order,
//! what a run publishes and reports.

use std::sync::Mutex;

use super::*;
use crate::CodebaseResolver;

/// The units of a graph as their sorted modules.
fn modules_of(units: &[UnitNode]) -> Vec<Vec<usize>> {
	units.iter().map(|unit| unit.modules.clone()).collect()
}

#[test]
fn a_cycle_is_one_unit_after_its_dependencies() {
	// 0 reads 1, 1 reads 0 (a cycle), 2 reads 0, 3 reads nothing
	let edges = vec![vec![1], vec![0], vec![0], vec![]];
	let units = units_of(&edges, &[1, 1, 1, 1]);
	assert_eq!(modules_of(&units), vec![vec![0, 1], vec![2], vec![3]]);
	assert_eq!(units[1].dependencies, vec![0]);
	assert_eq!(units[0].dependents, vec![1]);
	assert!(units[0].dependencies.is_empty());
	assert!(units[2].dependencies.is_empty() && units[2].dependents.is_empty());
}

#[test]
fn the_critical_chain_is_the_longest_chain_of_dependents() {
	// 1 reads 0, 2 reads 1, 3 reads 0; 4 alone
	let edges = vec![vec![], vec![0], vec![1], vec![0], vec![]];
	let units = units_of(&edges, &[10, 100, 5, 50, 60]);
	let critical = |module: usize| {
		units
			.iter()
			.find(|unit| unit.modules == [module])
			.unwrap()
			.critical
	};
	assert_eq!(critical(2), 5);
	assert_eq!(critical(1), 105);
	assert_eq!(critical(3), 50);
	// through 1 and 2 (105), not 3 (50)
	assert_eq!(critical(0), 115);
	assert_eq!(critical(4), 60);
}

#[test]
fn ready_units_come_by_critical_chain_then_order() {
	let mut ready = BinaryHeap::new();
	for (critical, order, unit) in [(5, 1, 0), (90, 3, 1), (90, 2, 2), (40, 0, 3)] {
		ready.push(Ready {
			critical,
			order: std::cmp::Reverse(order),
			unit,
		});
	}
	let popped: Vec<usize> = std::iter::from_fn(|| ready.pop().map(|ready| ready.unit)).collect();
	assert_eq!(popped, vec![2, 1, 3, 0]);
}

/// A package: a chain of large files `library/z1` ← `z2` ← `z3`, and a
/// small lone file `library/a0` (first in path order).
fn chain_package() -> Vec<(String, String)> {
	let padding: String = (0..40)
		.map(|index| format!("\tfunction f{index} = (x: Integer): Integer => x + {index}\n"))
		.collect();
	let file = |reads: Option<&str>| {
		let call = reads.map_or_else(|| "0".to_string(), |other| format!("{other}.f1(x)"));
		format!(
			"export namespace = {{\n{padding}\tfunction g = (x: Integer): Integer => {call}\n}}\n"
		)
	};
	vec![
		(
			"package.fa".to_string(),
			"export = {\n\tname = \"p\"\n\tversion = \"0.1.0\"\n}\n".to_string(),
		),
		(
			"library/a0.fa".to_string(),
			"export namespace = {\n\tfunction one = (): Integer => 1\n}\n".to_string(),
		),
		("library/z1.fa".to_string(), file(None)),
		("library/z2.fa".to_string(), file(Some("z1"))),
		("library/z3.fa".to_string(), file(Some("z2"))),
	]
}

/// Records the modules of the units, in the order they are analyzed.
struct Order(Mutex<Vec<String>>);

impl UnitSink for Order {
	fn unit(&self, session: &mut Session, files: &[FileId]) {
		let mut order = self.0.lock().unwrap();
		order.extend(files.iter().map(|&file| session.module(file).to_string()));
	}
}

fn run(files: &[(String, String)], threads: usize, seed: u64) -> (Session, Report, Vec<String>) {
	let mut session = Session::new();
	session.set_resolver(Box::new(CodebaseResolver::from_texts(files, None)));
	let options = Options {
		threads: Some(threads),
		seed,
		measure: true,
		..Options::default()
	};
	let order = Order(Mutex::new(Vec::new()));
	let report = analyze_package(&mut session, &options, None, &order);
	(session, report, order.0.into_inner().unwrap())
}

#[test]
fn one_thread_runs_the_longest_chain_first() {
	for seed in [0, 3, 99] {
		let (_, _, order) = run(&chain_package(), 1, seed);
		assert_eq!(
			order,
			["library/z1", "library/z2", "library/z3", "library/a0"],
			"seed {seed}"
		);
	}
}

#[test]
fn a_run_publishes_every_interface_and_reports_in_path_order() {
	let (session, report, order) = run(&chain_package(), 4, 0);
	let modules: Vec<&str> = report
		.modules
		.iter()
		.map(|module| module.module.as_str())
		.collect();
	assert_eq!(
		modules,
		["library/a0", "library/z1", "library/z2", "library/z3"]
	);
	assert_eq!(order.len(), 4);
	for module in &report.modules {
		assert!(
			module.analyzed && module.diagnostics.is_empty(),
			"{module:?}"
		);
		assert!(report.interface(&session, &module.module).is_some());
	}
	let stats = &report.stats;
	assert_eq!((stats.units, stats.threads), (4, 4));
	assert!(stats.busy >= stats.analysis && stats.analysis > Duration::ZERO);
	if cfg!(target_os = "linux") {
		assert!(stats.scheduled.is_some());
	}
}

#[test]
fn the_report_is_the_same_whatever_the_threads_and_seed() {
	let render = |report: &Report| -> Vec<String> {
		report
			.modules
			.iter()
			.flat_map(|module| {
				module
					.diagnostics
					.iter()
					.map(|diagnostic| format!("{} {}", module.module, diagnostic.message))
			})
			.collect()
	};
	let mut files = chain_package();
	// errors, and a cycle
	files.push((
		"library/b1.fa".to_string(),
		"export namespace = {\n\tfunction f = (x: Integer): Integer => b2.f(x) + missing\n}\n"
			.to_string(),
	));
	files.push((
		"library/b2.fa".to_string(),
		"export namespace = {\n\tfunction f = (x: Integer): Integer => if x > 0 { b1.f(x - 1) } else { \"no\" }\n}\n"
			.to_string(),
	));
	let (_, expected, _) = run(&files, 1, 0);
	assert!(!render(&expected).is_empty());
	assert_eq!(expected.stats.largest.1, 2, "the cycle is one unit");
	for threads in [2, 8] {
		for seed in [0, 5, 77] {
			let (_, found, _) = run(&files, threads, seed);
			assert_eq!(
				render(&expected),
				render(&found),
				"{threads} threads, seed {seed}"
			);
			assert_eq!(expected.init_order, found.init_order);
		}
	}
}

#[test]
fn the_origins_of_a_type_are_kept_sorted_once_each() {
	let origins = Origins::default();
	let ty = TypeId(77);
	let order = |a: DeclKey, b: DeclKey| a.0.cmp(&b.0);
	origins.add(ty, &[DeclKey(5), DeclKey(1)], order);
	origins.add(ty, &[DeclKey(3), DeclKey(5)], order);
	let found: Vec<u32> = origins.get(ty).unwrap().iter().map(|key| key.0).collect();
	assert_eq!(found, vec![1, 3, 5]);
	assert!(origins.get(TypeId(78)).is_none());
	let copy = origins.snapshot();
	assert_eq!(copy.get(ty).unwrap().len(), 3);
}
