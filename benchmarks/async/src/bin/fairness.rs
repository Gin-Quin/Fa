//! `fairness` (design/async.md §6): a TCP server on 0.0.0.0:<port> (backlog
//! 4096) reading 64-byte requests; one starting with `l` first runs 20 ms of
//! CPU (a busy loop until the monotonic clock says 20 ms passed), calling
//! `yield_now().await` every 1 ms in the `yield` variant and never in
//! `spin`; then every request is written back. Runs on a single-threaded
//! runtime (`current`) or the work-stealing one (`multi`).
//!
//! ```bash
//! cargo run --release --bin fairness -- <port> <yield|spin> [current|multi]
//! ```

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;

async fn work(yield_: bool) {
	let start = Instant::now();
	let mut last_yield = start;
	while start.elapsed() < Duration::from_millis(20) {
		if yield_ && last_yield.elapsed() >= Duration::from_millis(1) {
			tokio::task::yield_now().await;
			last_yield = Instant::now();
		}
	}
}

async fn serve(port: u16, yield_: bool) {
	let socket = TcpSocket::new_v4().expect("a socket");
	socket.set_reuseaddr(true).expect("SO_REUSEADDR");
	socket
		.bind(SocketAddr::from(([0, 0, 0, 0], port)))
		.expect("the port is free");
	let listener = socket.listen(4096).expect("listening");
	loop {
		let Ok((mut stream, _)) = listener.accept().await else {
			continue;
		};
		tokio::spawn(async move {
			let mut request = [0u8; 64];
			loop {
				if stream.read_exact(&mut request).await.is_err() {
					return;
				}
				if request[0] == b'l' {
					work(yield_).await;
				}
				if stream.write_all(&request).await.is_err() {
					return;
				}
			}
		});
	}
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let port: u16 = arguments
		.next()
		.expect("the port")
		.parse()
		.expect("the port is a number");
	let yield_ = match arguments.next().as_deref() {
		Some("yield") => true,
		Some("spin") => false,
		_ => panic!("the variant is `yield` or `spin`"),
	};
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
	runtime.block_on(serve(port, yield_));
}
