//! `echo` (design/async.md §6): a TCP echo server on 0.0.0.0:<port> (backlog
//! 4096, like Go), one task per connection, which reads up to 64 bytes and
//! writes them back until the end of the stream, on a single-threaded
//! runtime (`current`) or the work-stealing one (`multi`).
//!
//! ```bash
//! cargo run --release --bin echo -- <port> [current|multi]
//! ```

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;

async fn serve(port: u16) {
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
			let mut buffer = [0u8; 64];
			loop {
				match stream.read(&mut buffer).await {
					Ok(0) | Err(_) => return,
					Ok(read) => {
						if stream.write_all(&buffer[..read]).await.is_err() {
							return;
						}
					}
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
	runtime.block_on(serve(port));
}
