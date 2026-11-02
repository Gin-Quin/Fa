//! `--timings` on `fa check` and `fa build` (design/parallel-pipeline.md,
//! step 0): the time of each phase, the units, the size of the type store
//! and of the retained trees, printed to stderr at the end of the command.
//!
//! The phases are recorded through [`phase`], a no-op unless [`enable`] was
//! called; a phase run several times (`fa check` analyzes the program again
//! for each target when it reads `target`) adds up, with its count.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use fa_analyzer::Session;
use fa_analyzer::ids::FileId;

/// What one analyzed program held (the largest, when several were loaded).
#[derive(Default)]
struct SessionStats {
	/// Analysis units: files, until the analysis is scheduled by strongly
	/// connected components of the module graph.
	units: usize,
	/// The largest unit: its path, nodes and source bytes.
	largest: Option<(String, usize, usize)>,
	/// Types in the type store.
	types: usize,
	/// Nodes of every tree.
	nodes: usize,
	/// Bytes of the trees (nodes, spans, sources).
	tree_bytes: usize,
	/// Bytes of the per-node side tables of the analyses (types,
	/// resolutions, flags); the maps keyed by node are not counted.
	side_table_bytes: usize,
}

/// A run of the pipeline: its sizes and times, and its files.
type PipelineStats = (fa_analyzer::pipeline::Stats, usize);

struct Timings {
	start: Instant,
	/// The phases in their first order: name, total time, count.
	phases: Vec<(&'static str, Duration, usize)>,
	stats: SessionStats,
	/// Programs analyzed (one per target for `fa check`).
	sessions: usize,
	/// The runs of the parallel pipeline.
	pipeline: Vec<PipelineStats>,
}

thread_local! {
	static TIMINGS: RefCell<Option<Timings>> = const { RefCell::new(None) };
}

/// Starts recording.
pub(crate) fn enable() {
	TIMINGS.with(|timings| {
		*timings.borrow_mut() = Some(Timings {
			start: Instant::now(),
			phases: Vec::new(),
			stats: SessionStats::default(),
			sessions: 0,
			pipeline: Vec::new(),
		});
	});
}

/// Whether recording.
pub(crate) fn enabled() -> bool {
	TIMINGS.with(|timings| timings.borrow().is_some())
}

/// Runs `run`, adding its time to the phase `name` when recording.
pub(crate) fn phase<T>(name: &'static str, run: impl FnOnce() -> T) -> T {
	if !enabled() {
		return run();
	}
	let start = Instant::now();
	let result = run();
	let elapsed = start.elapsed();
	TIMINGS.with(|timings| {
		if let Some(timings) = timings.borrow_mut().as_mut() {
			match timings.phases.iter_mut().find(|phase| phase.0 == name) {
				Some(phase) => {
					phase.1 += elapsed;
					phase.2 += 1;
				}
				None => timings.phases.push((name, elapsed, 1)),
			}
		}
	});
	result
}

fn vec_bytes<T>(vector: &Vec<T>) -> usize {
	vector.capacity() * size_of::<T>()
}

/// Records the sizes of an analyzed program: its files are the units.
pub(crate) fn record_session(session: &Session, files: &[FileId]) {
	if !enabled() {
		return;
	}
	let mut stats = SessionStats {
		units: files.len(),
		types: session.types.len(),
		..SessionStats::default()
	};
	for &file in files {
		let tree = session.tree(file);
		let nodes = tree.nodes.len();
		stats.nodes += nodes;
		stats.tree_bytes += vec_bytes(&tree.nodes) + vec_bytes(&tree.spans) + tree.source.len();
		let analysis = session.analysis(file);
		stats.side_table_bytes += vec_bytes(&analysis.types)
			+ vec_bytes(&analysis.resolutions)
			+ vec_bytes(&analysis.flags);
		if stats
			.largest
			.as_ref()
			.is_none_or(|(_, largest, _)| nodes > *largest)
		{
			stats.largest = Some((session.display_path(file), nodes, tree.source.len()));
		}
	}
	TIMINGS.with(|timings| {
		if let Some(timings) = timings.borrow_mut().as_mut() {
			timings.sessions += 1;
			if stats.nodes >= timings.stats.nodes {
				timings.stats = stats;
			}
		}
	});
}

/// Records the sizes of a run of the parallel pipeline (`streaming.rs`).
pub(crate) fn record_pipeline(stats: &fa_analyzer::pipeline::Stats, files: usize) {
	if !enabled() {
		return;
	}
	TIMINGS.with(|timings| {
		if let Some(timings) = timings.borrow_mut().as_mut() {
			timings.sessions += 1;
			timings.pipeline.push((stats.clone(), files));
			// reading, parsing and scanning the files; the code generation
			// (fused into the units: its time summed over the threads)
			for (name, elapsed) in [
				("parse", stats.scan),
				("analysis", stats.units_time),
				("generation", stats.sink),
			] {
				if elapsed.is_zero() {
					continue;
				}
				match timings.phases.iter_mut().find(|other| other.0 == name) {
					Some(other) => {
						other.1 += elapsed;
						other.2 += 1;
					}
					None => timings.phases.push((name, elapsed, 1)),
				}
			}
		}
	});
}

/// The peak resident set size of the process, in bytes (Linux).
fn peak_rss() -> Option<usize> {
	let status = std::fs::read_to_string("/proc/self/status").ok()?;
	let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
	let kilobytes: usize = line
		.trim_start_matches("VmHWM:")
		.trim()
		.trim_end_matches("kB")
		.trim()
		.parse()
		.ok()?;
	Some(kilobytes * 1024)
}

fn megabytes(bytes: usize) -> String {
	format!("{:.1} MB", bytes as f64 / 1e6)
}

/// The use of the worker threads by the units of a run: busy (and in
/// what), idle; and in the units, the time on a core, waiting for one and
/// blocked, when the system's scheduler told.
fn workers(out: &mut String, stats: &fa_analyzer::pipeline::Stats) {
	let capacity = stats.units_time.as_secs_f64() * stats.threads as f64;
	if capacity <= 0.0 {
		return;
	}
	let share = |time: Duration| time.as_secs_f64() / capacity * 100.0;
	let _ = writeln!(
		out,
		"  workers {}: busy {:.0}% (setup {:.0}%, analysis {:.0}%, merge {:.0}%, sink {:.0}%, teardown {:.0}%), idle {:.0}%",
		stats.threads,
		share(stats.busy),
		share(stats.setup),
		share(stats.analysis),
		share(stats.merge),
		share(stats.sink),
		share(stats.teardown),
		(100.0 - share(stats.busy)).max(0.0)
	);
	if let Some((on_cpu, runnable)) = stats.scheduled {
		let busy = stats.busy.as_secs_f64().max(1e-9);
		let part = |time: Duration| time.as_secs_f64() / busy * 100.0;
		let _ = writeln!(
			out,
			"    in units: on a core {:.0}%, waiting for a core {:.0}%, blocked {:.0}% (locks, evaluation threads, page faults)",
			part(on_cpu),
			part(runnable),
			(100.0 - part(on_cpu) - part(runnable)).max(0.0)
		);
	}
}

/// Prints what was recorded to stderr (nothing unless recording).
pub(crate) fn print(command: &str) {
	let Some(timings) = TIMINGS.with(|timings| timings.borrow_mut().take()) else {
		return;
	};
	let mut out = format!("timings: fa {command}\n");
	for (name, elapsed, count) in &timings.phases {
		let repeated = if *count > 1 {
			format!(" (×{count})")
		} else {
			String::new()
		};
		let _ = writeln!(
			out,
			"  phase {name:<13} {:>10.1} ms{repeated}",
			elapsed.as_secs_f64() * 1000.0
		);
	}
	let _ = writeln!(
		out,
		"  phase {:<13} {:>10.1} ms",
		"total",
		timings.start.elapsed().as_secs_f64() * 1000.0
	);
	let stats = &timings.stats;
	if let Some((first, files)) = timings.pipeline.first() {
		let _ = writeln!(
			out,
			"  units {} (strongly connected components of {files} files; programs analyzed: {})",
			first.units, timings.sessions
		);
		let _ = writeln!(
			out,
			"  largest unit {} ({} files)",
			first.largest.0, first.largest.1
		);
		let _ = writeln!(
			out,
			"  types {} (global store: what the interfaces reach)",
			first.global_types
		);
		let _ = writeln!(
			out,
			"  trees streamed: dropped with their unit, {} parsed again (tree budget); peak units in flight {}",
			first.parsed_again, first.peak_in_flight
		);
		for (stats, _) in &timings.pipeline {
			workers(&mut out, stats);
		}
		if let Some(bytes) = peak_rss() {
			let _ = writeln!(out, "  peak rss {}", megabytes(bytes));
		}
		eprint!("{out}");
		return;
	}
	let _ = writeln!(
		out,
		"  units {} (files; programs analyzed: {})",
		stats.units, timings.sessions
	);
	if let Some((path, nodes, bytes)) = &stats.largest {
		let _ = writeln!(
			out,
			"  largest unit {path} ({nodes} nodes, {:.1} KB)",
			*bytes as f64 / 1e3
		);
	}
	let _ = writeln!(out, "  types {}", stats.types);
	let _ = writeln!(
		out,
		"  trees {} nodes, {} retained (nodes {} B + spans {} B each, and the sources); side tables {}",
		stats.nodes,
		megabytes(stats.tree_bytes),
		size_of::<fa_parser::Node>(),
		size_of::<fa_parser::SourceSpan>(),
		megabytes(stats.side_table_bytes)
	);
	if let Some(bytes) = peak_rss() {
		let _ = writeln!(out, "  peak rss {}", megabytes(bytes));
	}
	eprint!("{out}");
}
