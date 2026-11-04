//! `timers` (design/async.md §6): n tasks (default 1 000 000), the i-th
//! sleeping (i * 7919) % 1001 ms, plus one task sleeping 1 ms in a loop until
//! they all ended and recording how late each wake is; prints the total time
//! and the lateness percentiles. Runs on a single-threaded runtime
//! (`current`) or the work-stealing one (`multi`).
//!
//! ```bash
//! cargo run --release --bin timers -- [n] [current|multi]
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn percentile(sorted: &[Duration], p: f64) -> u128 {
	let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1;
	sorted[rank].as_micros()
}

async fn run(n: u64) -> (Duration, Vec<Duration>) {
	let start = Instant::now();
	let done = Arc::new(AtomicBool::new(false));
	let ticker = tokio::spawn({
		let done = done.clone();
		async move {
			let mut lateness = Vec::new();
			while !done.load(Ordering::Relaxed) {
				let before = Instant::now();
				tokio::time::sleep(Duration::from_millis(1)).await;
				lateness.push(before.elapsed().saturating_sub(Duration::from_millis(1)));
			}
			lateness
		}
	});
	let mut handles = Vec::with_capacity(n as usize);
	for index in 0..n {
		handles.push(tokio::spawn(async move {
			tokio::time::sleep(Duration::from_millis((index * 7919) % 1001)).await;
		}));
	}
	for handle in handles {
		handle.await.expect("the sleeper ended");
	}
	let total = start.elapsed();
	done.store(true, Ordering::Relaxed);
	(total, ticker.await.expect("the ticker ended"))
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let n: u64 = arguments
		.next()
		.map_or(1_000_000, |argument| argument.parse().expect("n is a number"));
	let flavor = arguments.next().unwrap_or_else(|| "current".to_string());
	let runtime = match flavor.as_str() {
		"current" => tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build(),
		"multi" => tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build(),
		_ => panic!("the flavor is `current` or `multi`"),
	}
	.expect("the runtime starts");
	let (total, mut lateness) = runtime.block_on(run(n));
	lateness.sort();
	println!(
		"total_ms={} lateness_p99_us={} lateness_max_us={}",
		total.as_millis(),
		percentile(&lateness, 0.99),
		lateness.last().expect("one tick at least").as_micros()
	);
}
