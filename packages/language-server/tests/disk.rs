//! The language server and the disk: the file watchers it registers, the
//! files changed outside the editor (`workspace/didChangeWatchedFiles`),
//! and the dependencies fetched in the background, which never hold the
//! first diagnostics (a remote `file://` source that is a named pipe: its
//! read blocks until the test writes it, like a slow network).
//!
//! `FA_CACHE` is set once for the whole process, before any test reads it.

use std::path::Path;
use std::sync::Once;
use std::time::{Duration, Instant};

use fa_language_server::FaLanguageServer;
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::LspService;
use tower_lsp::jsonrpc::Request;
use tower_lsp::lsp_types::Url;

static CACHE: Once = Once::new();

/// Sets the cache of the dependencies of the process (once, in a folder of
/// its own).
fn set_cache() {
	CACHE.call_once(|| {
		let cache = tempfile::TempDir::new().unwrap().into_path();
		// SAFETY: every test calls this first: no thread reads it meanwhile
		unsafe { std::env::set_var("FA_CACHE", cache) };
	});
}

struct Client {
	service: LspService<FaLanguageServer>,
	// the messages sent by the server
	messages: UnboundedReceiver<Request>,
	next_id: i64,
}

impl Client {
	/// An initialized server, with the capabilities of the client.
	async fn initialized(capabilities: Value) -> Self {
		set_cache();
		let (service, mut socket) = fa_language_server::service();
		let (sender, messages) = unbounded_channel();
		tokio::spawn(async move {
			while let Some(message) = socket.next().await {
				if sender.send(message).is_err() {
					break;
				}
			}
		});
		let mut client = Client {
			service,
			messages,
			next_id: 0,
		};
		client
			.request(
				"initialize",
				json!({ "processId": null, "rootUri": null, "capabilities": capabilities }),
			)
			.await
			.expect("initialize should succeed");
		client.notify("initialized", json!({})).await;
		client
	}

	async fn call(&mut self, request: Request) -> Option<tower_lsp::jsonrpc::Response> {
		self.service
			.ready()
			.await
			.expect("the service should be ready")
			.call(request)
			.await
			.expect("the service should answer")
	}

	async fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String> {
		self.next_id += 1;
		let request = Request::build(method)
			.id(self.next_id)
			.params(params)
			.finish();
		let response = self.call(request).await.expect("a request has a response");
		let (_, result) = response.into_parts();
		result.map_err(|error| error.message.to_string())
	}

	async fn notify(&mut self, method: &'static str, params: Value) {
		let response = self
			.call(Request::build(method).params(params).finish())
			.await;
		assert!(response.is_none(), "a notification has no response");
	}

	async fn open(&mut self, path: &Path) {
		let text = std::fs::read_to_string(path).unwrap();
		self.notify(
			"textDocument/didOpen",
			json!({ "textDocument": { "uri": uri(path), "languageId": "fa", "version": 1, "text": text } }),
		)
		.await;
	}

	/// Tells the server that files changed on the disk (1: created, 2:
	/// changed, 3: deleted).
	async fn changed_on_disk(&mut self, changes: &[(&Path, u32)]) {
		let changes: Vec<Value> = changes
			.iter()
			.map(|(path, kind)| json!({ "uri": uri(path), "type": kind }))
			.collect();
		self.notify(
			"workspace/didChangeWatchedFiles",
			json!({ "changes": changes }),
		)
		.await;
	}

	/// The next message of the server with this method.
	async fn next(&mut self, method: &str, wait: Duration) -> Option<Request> {
		let deadline = tokio::time::Instant::now() + wait;
		loop {
			match tokio::time::timeout_at(deadline, self.messages.recv()).await {
				Ok(Some(message)) if message.method() == method => return Some(message),
				Ok(Some(_)) => {}
				_ => return None,
			}
		}
	}

	/// The next diagnostics published for a file: (code or severity, message).
	async fn diagnostics(&mut self, path: &Path) -> Vec<(String, String)> {
		loop {
			let message = self
				.next("textDocument/publishDiagnostics", Duration::from_secs(20))
				.await
				.expect("no diagnostics were published");
			let params = message.params().cloned().unwrap_or(Value::Null);
			if params["uri"] != uri(path) {
				continue;
			}
			return params["diagnostics"]
				.as_array()
				.unwrap()
				.iter()
				.map(|diagnostic| {
					let code = match diagnostic["code"].as_str() {
						Some(code) => code.to_string(),
						None => format!("severity {}", diagnostic["severity"]),
					};
					(code, diagnostic["message"].as_str().unwrap().to_string())
				})
				.collect();
		}
	}
}

fn uri(path: &Path) -> String {
	Url::from_file_path(path).unwrap().to_string()
}

/// Writes files below a folder.
fn write_files(root: &Path, files: &[(&str, &str)]) {
	for (path, text) in files {
		let path = root.join(path);
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(path, text).unwrap();
	}
}

fn codes(diagnostics: &[(String, String)]) -> Vec<&str> {
	diagnostics.iter().map(|(code, _)| code.as_str()).collect()
}

const MAIN: &str =
	"export function = () => {\n\tlet value: Integer = library.answer()\n\tconsole.log(value)\n}\n";

#[tokio::test]
async fn the_watchers_are_registered_when_the_client_can() {
	let mut client = Client::initialized(
		json!({ "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } } }),
	)
	.await;
	let request = client
		.next("client/registerCapability", Duration::from_secs(5))
		.await
		.expect("the watchers are registered");
	let registration = &request.params().unwrap()["registrations"][0];
	assert_eq!(registration["method"], "workspace/didChangeWatchedFiles");
	let globs: Vec<&str> = registration["registerOptions"]["watchers"]
		.as_array()
		.unwrap()
		.iter()
		.map(|watcher| watcher["globPattern"].as_str().unwrap())
		.collect();
	assert_eq!(globs, ["**/*.fa", "**/dependencies.lock"]);
	// a client that can't: no registration
	let mut client = Client::initialized(json!({})).await;
	assert!(
		client
			.next("client/registerCapability", Duration::from_millis(300))
			.await
			.is_none()
	);
}

/// A file changed on the disk (a `git pull`) updates the diagnostics of the
/// open documents that use it; a file created then deleted too; the open
/// document keeps the editor's text.
#[tokio::test]
async fn files_changed_on_the_disk_update_the_open_documents() {
	let directory = tempfile::TempDir::new().unwrap();
	let root = directory.path().canonicalize().unwrap();
	write_files(
		&root,
		&[
			("package.fa", "export = { name = \"p\" }\n"),
			("library/answer.fa", "export function = (): Integer => 42\n"),
			("application/main.fa", MAIN),
		],
	);
	let main = root.join("application/main.fa");
	let answer = root.join("library/answer.fa");
	let mut client = Client::initialized(json!({})).await;
	client.open(&main).await;
	assert!(client.diagnostics(&main).await.is_empty());
	// changed on the disk
	std::fs::write(&answer, "export function = (): String => \"x\"\n").unwrap();
	client.changed_on_disk(&[(&answer, 2)]).await;
	assert_eq!(codes(&client.diagnostics(&main).await), ["E-TYPE-1"]);
	// deleted: the name is unknown
	std::fs::remove_file(&answer).unwrap();
	client.changed_on_disk(&[(&answer, 3)]).await;
	let diagnostics = client.diagnostics(&main).await;
	assert!(!diagnostics.is_empty() && !codes(&diagnostics).contains(&"E-TYPE-1"));
	// created again
	std::fs::write(&answer, "export function = (): Integer => 42\n").unwrap();
	client.changed_on_disk(&[(&answer, 1)]).await;
	assert!(client.diagnostics(&main).await.is_empty());
	// the open document changed on the disk: the editor's text stays
	std::fs::write(&main, "let broken = (\n").unwrap();
	client.changed_on_disk(&[(&main, 2)]).await;
	let symbols = client
		.request(
			"textDocument/documentSymbol",
			json!({ "textDocument": { "uri": uri(&main) } }),
		)
		.await
		.unwrap();
	assert!(
		!symbols.to_string().contains("broken"),
		"the disk's text replaced the editor's: {symbols}"
	);
}

/// A dependency that takes long to fetch doesn't hold the first diagnostics
/// nor the queries: the package is analyzed without it at once, then again
/// when it arrives.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_slow_dependency_does_not_block_the_first_diagnostics() {
	let directory = tempfile::TempDir::new().unwrap();
	let root = directory.path().canonicalize().unwrap();
	std::fs::create_dir_all(root.join("served")).unwrap();
	let pipe = root.join("served/slow.fa");
	let made = std::process::Command::new("mkfifo").arg(&pipe).status();
	if !made.is_ok_and(|status| status.success()) {
		eprintln!("`mkfifo` failed: the test is skipped");
		return;
	}
	let package = root.join("app");
	write_files(
		&package,
		&[
			("package.fa", "export = { name = \"app\" }\n"),
			(
				"dependencies.fa",
				&format!("export = {{\n\tslow = \"file://{}\"\n}}\n", pipe.display()),
			),
			(
				"application/main.fa",
				"export function = () => {\n\tlet value: Integer = slow()\n\tconsole.log(value)\n}\n",
			),
		],
	);
	let main = package.join("application/main.fa");
	let mut client = Client::initialized(json!({})).await;
	let started = Instant::now();
	client.open(&main).await;
	// at once: a note, and no error about the names of the dependency
	let diagnostics = client.diagnostics(&main).await;
	assert_eq!(codes(&diagnostics), ["severity 3"], "{diagnostics:?}");
	assert!(diagnostics[0].1.contains("being fetched"));
	// the queries are answered meanwhile
	let symbols = client
		.request(
			"textDocument/documentSymbol",
			json!({ "textDocument": { "uri": uri(&main) } }),
		)
		.await;
	assert!(symbols.is_ok());
	assert!(started.elapsed() < Duration::from_secs(10));
	// the dependency arrives: analyzed with it
	let writer = std::thread::spawn(move || {
		std::fs::write(&pipe, "export function = (): Integer => 42\n").unwrap();
	});
	let diagnostics = client.diagnostics(&main).await;
	assert!(diagnostics.is_empty(), "{diagnostics:?}");
	writer.join().unwrap();
	// a type error with the dependency
	let wrong =
		"export function = () => {\n\tlet value: String = slow()\n\tconsole.log(value)\n}\n";
	client
		.notify(
			"textDocument/didChange",
			json!({ "textDocument": { "uri": uri(&main), "version": 2 }, "contentChanges": [{ "text": wrong }] }),
		)
		.await;
	assert_eq!(codes(&client.diagnostics(&main).await), ["E-TYPE-1"]);
}
