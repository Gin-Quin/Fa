//! `starve` (design/async.md §6): 1000 tasks spin forever on their own
//! always-ready channel of capacity 1 (send, then receive: Tokio's coop
//! budget makes them yield now and then), one task sleeps 1 ms <ticks> times
//! (default 3000) and records how late each wake is, and an accept loop on
//! 0.0.0.0:<port> (backlog 4096) writes the byte `x` to each connection then
//! closes it. Once the ticks are done, prints the lateness percentiles and
//! exits. Runs on a single-threaded runtime (`current`) or the work-stealing
//! one (`multi`).
//!
//! ```bash
//! cargo run --release --bin starve -- <port> [ticks] [current|multi]
//! ```

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;
use tokio::sync::mpsc;

fn percentile(sorted: &[Duration], p: f64) -> u128 {
	let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1;
	sorted[rank].as_micros()
}

async fn run(port: u16, ticks: usize) -> Vec<Duration> {
	let socket = TcpSocket::new_v4().expect("a socket");
	socket.set_reuseaddr(true).expect("SO_REUSEADDR");
	socket
		.bind(SocketAddr::from(([0, 0, 0, 0], port)))
		.expect("the port is free");
	let listener = socket.listen(4096).expect("listening");
	for _ in 0..1000 {
		tokio::spawn(async {
			let (sender, mut receiver) = mpsc::channel::<u64>(1);
			let mut value = 0;
			loop {
				sender.send(value).await.expect("the receiver is here");
				value = receiver.recv().await.expect("the sender is here") + 1;
			}
		});
	}
	tokio::spawn(async move {
		loop {
			if let Ok((mut stream, _)) = listener.accept().await {
				let _ = stream.write_all(b"x").await;
			}
		}
	});
	tokio::spawn(async move {
		let mut lateness = Vec::with_capacity(ticks);
		for _ in 0..ticks {
			let start = Instant::now();
			tokio::time::sleep(Duration::from_millis(1)).await;
			lateness.push(start.elapsed().saturating_sub(Duration::from_millis(1)));
		}
		lateness
	})
	.await
	.expect("the timer task ended")
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let port: u16 = arguments
		.next()
		.expect("the port")
		.parse()
		.expect("the port is a number");
	let ticks: usize = arguments
		.next()
		.map_or(3000, |argument| argument.parse().expect("ticks is a number"));
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
	let mut lateness = runtime.block_on(run(port, ticks));
	lateness.sort();
	println!(
		"lateness_p50_us={} lateness_p99_us={} lateness_max_us={}",
		percentile(&lateness, 0.5),
		percentile(&lateness, 0.99),
		lateness.last().expect("ticks > 0").as_micros()
	);
	std::process::exit(0);
}
