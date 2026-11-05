use std::task::{Context, Poll};

use tower::Service;
use tower_lsp::Server;
use tower_lsp::jsonrpc::Request;

/// Counts the heap in use, so that compile-time evaluation stays within its
/// memory budget (`fa_interpreter::memory`).
#[global_allocator]
static ALLOCATOR: fa_interpreter::memory::CountingAllocator<mimalloc::MiMalloc> =
	fa_interpreter::memory::CountingAllocator(mimalloc::MiMalloc);

/// Serve the language server over stdin and stdout.
#[tokio::main]
async fn main() {
	let stdin = tokio::io::stdin();
	let stdout = tokio::io::stdout();

	let (service, socket) = fa_language_server::service();
	let service = ExitOnExit {
		inner: service,
		shut_down: false,
	};
	Server::new(stdin, stdout, socket).serve(service).await;

	// the end of stdin: don't wait for the thread that reads it
	std::process::exit(0);
}

/// Stop the process on the `exit` notification, with the exit code of LSP: 0 after a
/// `shutdown` request, 1 otherwise. tower-lsp would only stop at the next message or at the
/// end of stdin.
struct ExitOnExit<S> {
	inner: S,
	shut_down: bool,
}

impl<S: Service<Request>> Service<Request> for ExitOnExit<S> {
	type Response = S::Response;
	type Error = S::Error;
	type Future = S::Future;

	fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(context)
	}

	fn call(&mut self, request: Request) -> Self::Future {
		match request.method() {
			"shutdown" => self.shut_down = true,
			"exit" => std::process::exit(if self.shut_down { 0 } else { 1 }),
			_ => {}
		}
		self.inner.call(request)
	}
}
