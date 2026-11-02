//! The memory and the time of the formatter stay linear in the size of the code, whatever its
//! shape: a counting allocator measures the peak of the bytes allocated while formatting
//! generated code (deep nesting, long lines, long lists, many comments, a large file).
//!
//! The formatter once copied the arguments of a call into both of its layouts, which doubled the
//! document at each level of `f({ a = g({ … }) })`: 20 levels took gigabytes. `fa format --check`,
//! `fa check` and the language server must stay well below 512 MB.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use fa_formatter::format;

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let pointer = unsafe { System.alloc(layout) };
		if !pointer.is_null() {
			let current = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
			PEAK.fetch_max(current, Ordering::Relaxed);
		}
		pointer
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		unsafe { System.dealloc(pointer, layout) };
		CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
	}

	unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
		let new = unsafe { System.realloc(pointer, layout, size) };
		if !new.is_null() {
			if size > layout.size() {
				let grown = size - layout.size();
				let current = CURRENT.fetch_add(grown, Ordering::Relaxed) + grown;
				PEAK.fetch_max(current, Ordering::Relaxed);
			} else {
				CURRENT.fetch_sub(layout.size() - size, Ordering::Relaxed);
			}
		}
		new
	}
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

// the tests run one at a time, since the allocations of the others would count
static MEASURING: Mutex<()> = Mutex::new(());

fn one_at_a_time() -> MutexGuard<'static, ()> {
	MEASURING.lock().unwrap_or_else(|error| error.into_inner())
}

/// The bytes allocated at the peak of `run`, above what was allocated before, and its time.
fn measure<T>(run: impl FnOnce() -> T) -> (T, usize, Duration) {
	let before = CURRENT.load(Ordering::Relaxed);
	PEAK.store(before, Ordering::Relaxed);
	let started_at = Instant::now();
	let result = run();
	let elapsed = started_at.elapsed();
	(result, PEAK.load(Ordering::Relaxed) - before, elapsed)
}

/// Format `source` and check that it allocates at most `bytes_per_byte` times its size (plus
/// 1 MB), in at most `seconds`, and that the result is formatted.
fn assert_bounded(name: &str, source: &str, bytes_per_byte: usize, seconds: u64) {
	let (formatted, peak, elapsed) = measure(|| format(source));
	let formatted = formatted.unwrap_or_else(|errors| {
		panic!("{name}: refused: {}", errors[0].message);
	});
	println!(
		"{name}: {} KB of code, peak {} KB ({:.0} bytes per byte), {elapsed:.2?}",
		source.len() / 1000,
		peak / 1000,
		peak as f64 / source.len() as f64
	);
	let limit = bytes_per_byte * source.len() + 1_000_000;
	assert!(
		peak <= limit,
		"{name}: formatting {} bytes allocated {peak} bytes at the peak (more than {limit})",
		source.len()
	);
	// a debug build is slow, but not that slow: a quadratic or exponential layout is
	assert!(
		elapsed <= Duration::from_secs(seconds),
		"{name}: formatting took {elapsed:?}"
	);
	assert_eq!(
		format(&formatted).ok().as_deref(),
		Some(formatted.as_str()),
		"{name}: not idempotent"
	);
}

/// `depth` levels of `open` … `close` around `x`.
fn nested(open: &str, close: &str, depth: usize) -> String {
	format!("let a = {}x{}\n", open.repeat(depth), close.repeat(depth))
}

/// Deeply nested calls whose last argument hugs the parentheses: the document of each level is
/// in both layouts of its call.
#[test]
fn nested_hugged_arguments_stay_linear() {
	let _guard = one_at_a_time();
	// the parser refuses code nested deeper than 128 levels (each level here counts for more)
	for depth in [20, 30, 40] {
		assert_bounded(
			&format!("{depth} objects"),
			&nested("f({ a = ", " })", depth),
			3000,
			10,
		);
		assert_bounded(
			&format!("{depth} lists"),
			&nested("f([", "])", depth),
			3000,
			10,
		);
		assert_bounded(
			&format!("{depth} lambdas"),
			&nested("f((x) => { g(", ") })", depth),
			3000,
			10,
		);
		assert_bounded(
			&format!("{depth} calls"),
			&nested("f(1, ", ")", depth),
			3000,
			10,
		);
		assert_bounded(
			&format!("{depth} objects"),
			&nested("{ a = ", " }", depth),
			3000,
			10,
		);
	}
}

#[test]
fn long_lines_and_lists_stay_linear() {
	let _guard = one_at_a_time();
	let names: Vec<String> = (0..20_000).map(|index| format!("x{index}")).collect();
	assert_bounded(
		"call of 20,000 arguments",
		&format!("let a = f({})\n", names.join(", ")),
		200,
		20,
	);
	assert_bounded(
		"20,000 additions",
		&format!("let a = {}\n", names.join(" + ")),
		200,
		20,
	);
	assert_bounded(
		"20,000 `and`",
		&format!("let a = {}\n", names.join(" and ")),
		200,
		20,
	);
	let numbers: Vec<String> = (0..10_000).map(|index| index.to_string()).collect();
	assert_bounded(
		"list of 10,000 items",
		&format!("let a = [{}]\n", numbers.join(", ")),
		200,
		20,
	);
	// each link of a chain counts in the nesting of the parser
	let calls: String = (0..50).map(|index| format!(".m{index}(y)")).collect();
	assert_bounded(
		"1,000 chains of 50 calls",
		&format!("let a = x{calls}\n").repeat(1_000),
		200,
		20,
	);
	assert_bounded(
		"string of 1 MB",
		&format!("let a = \"{}\"\n", "a".repeat(1_000_000)),
		20,
		20,
	);
}

#[test]
fn comments_stay_linear() {
	let _guard = one_at_a_time();
	let source: String = (0..20_000)
		.map(|index| format!("let a{index} = {index} -- comment {index}\n-- line {index}\n"))
		.collect();
	assert_bounded("20,000 comments", &source, 200, 20);
}

/// A file of several megabytes, made of the kinds of statements of the documentation.
#[test]
fn a_large_file_stays_linear() {
	let _guard = one_at_a_time();
	let mut source = String::new();
	let mut index = 0;
	while source.len() < 2_000_000 {
		source.push_str(&format!(
			"let a{index} = {index} + 3 -- the first\n\
			type Point{index} = {{\n\tx: Integer\n\ty: Integer\n}}\n\
			function f{index} = (point: Point{index}): Integer => {{\n\
			\tlet values = [point.x, point.y, {index}].map((value) => value * 2)\n\
			\tif values.length > 2 {{ values[0] }} else {{ 0 }}\n}}\n\
			console.log(f{index}({{ x = 1, y = 2 }}), \"done {index}\")\n\n"
		));
		index += 1;
	}
	assert_bounded("2 MB file", &source, 200, 60);
}
