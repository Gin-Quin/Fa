//! Spawns `n` tasks (default 100 000) that each yield once to the scheduler
//! then return `2 * index`, awaits them all in order, and prints the sum
//! (`n * (n - 1)`) and the time taken, on a single-threaded runtime
//! (`current`, the model of Fa's event loop) or the work-stealing one
//! (`multi`).
//!
//! ```bash
//! cargo run --release -- [n] [current|multi]
//! ```

use std::time::Instant;

async fn spawn_all(n: u64) -> u64 {
	let mut handles = Vec::with_capacity(n as usize);
	for index in 0..n {
		handles.push(tokio::spawn(async move {
			tokio::task::yield_now().await;
			index * 2
		}));
	}
	let mut sum = 0;
	for handle in handles {
		sum += handle.await.expect("the task panicked");
	}
	sum
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let n: u64 = arguments
		.next()
		.map_or(100_000, |argument| argument.parse().expect("n is a number"));
	let flavor = arguments.next().unwrap_or_else(|| "current".to_string());
	let runtime = match flavor.as_str() {
		"current" => tokio::runtime::Builder::new_current_thread().build(),
		"multi" => tokio::runtime::Builder::new_multi_thread().build(),
		_ => panic!("the flavor is `current` or `multi`"),
	}
	.expect("the runtime starts");
	let start = Instant::now();
	let sum = runtime.block_on(spawn_all(n));
	let elapsed = start.elapsed();
	println!("checksum {sum}");
	println!(
		"tokio {flavor}: {n} tasks in {:.2} ms ({:.0} ns per task)",
		elapsed.as_secs_f64() * 1e3,
		elapsed.as_nanos() as f64 / n as f64
	);
}
