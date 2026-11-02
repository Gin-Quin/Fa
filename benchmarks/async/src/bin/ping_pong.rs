//! `ping-pong` (design/async.md §6): two tasks exchange `n` messages
//! (default 10 000 000, `n / 2` round trips) over two channels of capacity
//! 1, on a single-threaded runtime (`current`) or the work-stealing one
//! (`multi`), and print the last ball and the time taken.
//!
//! ```bash
//! cargo run --release --bin ping_pong -- [n] [current|multi]
//! ```

use std::time::Instant;

use tokio::sync::mpsc;

async fn play(messages: u64) -> u64 {
	let (ping_sender, mut ping_receiver) = mpsc::channel::<u64>(1);
	let (pong_sender, mut pong_receiver) = mpsc::channel::<u64>(1);
	let player = tokio::spawn(async move {
		while let Some(ball) = ping_receiver.recv().await {
			if pong_sender.send(ball + 1).await.is_err() {
				return;
			}
		}
	});
	let mut ball = 0;
	for _ in 0..messages / 2 {
		ping_sender.send(ball).await.expect("the player is there");
		ball = pong_receiver.recv().await.expect("a ball");
	}
	drop(ping_sender);
	player.await.expect("the player ended");
	ball
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let n: u64 = arguments
		.next()
		.map_or(10_000_000, |argument| argument.parse().expect("n is a number"));
	let flavor = arguments.next().unwrap_or_else(|| "current".to_string());
	let runtime = match flavor.as_str() {
		"current" => tokio::runtime::Builder::new_current_thread().build(),
		"multi" => tokio::runtime::Builder::new_multi_thread().build(),
		_ => panic!("the flavor is `current` or `multi`"),
	}
	.expect("the runtime starts");
	let start = Instant::now();
	let ball = runtime.block_on(play(n));
	let elapsed = start.elapsed();
	println!("ball {ball}");
	println!(
		"tokio {flavor}: {n} messages in {:.2} ms ({:.1} ns per message)",
		elapsed.as_secs_f64() * 1e3,
		elapsed.as_nanos() as f64 / n as f64
	);
}
