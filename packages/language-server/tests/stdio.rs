//! Tests of the binary, over stdin and stdout.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

struct Process {
	child: Child,
	stdin: ChildStdin,
	stdout: BufReader<ChildStdout>,
}

impl Process {
	fn start() -> Self {
		let mut child = Command::new(env!("CARGO_BIN_EXE_fa-language-server"))
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.expect("the server should start");
		let stdin = child.stdin.take().unwrap();
		let stdout = BufReader::new(child.stdout.take().unwrap());
		Process {
			child,
			stdin,
			stdout,
		}
	}

	fn send(&mut self, message: Value) {
		let body = message.to_string();
		write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
		self.stdin.flush().unwrap();
	}

	fn receive(&mut self) -> Value {
		let mut length = 0;
		loop {
			let mut line = String::new();
			self.stdout.read_line(&mut line).unwrap();
			let line = line.trim();
			if line.is_empty() {
				break;
			}
			if let Some(value) = line.strip_prefix("Content-Length: ") {
				length = value.parse().unwrap();
			}
		}
		let mut body = vec![0; length];
		self.stdout.read_exact(&mut body).unwrap();
		serde_json::from_slice(&body).unwrap()
	}

	/// The next message with this method, or the response with this id.
	fn receive_until(&mut self, matches: impl Fn(&Value) -> bool) -> Value {
		loop {
			let message = self.receive();
			if matches(&message) {
				return message;
			}
		}
	}

	fn initialize(&mut self) -> Value {
		self.send(json!({
			"jsonrpc": "2.0", "id": 1, "method": "initialize",
			"params": { "capabilities": {} },
		}));
		let response = self.receive_until(|message| message["id"] == 1);
		self.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
		response
	}

	fn wait(mut self) -> i32 {
		drop(self.stdin);
		let started = Instant::now();
		loop {
			if let Some(status) = self.child.try_wait().unwrap() {
				return status.code().unwrap_or(-1);
			}
			if started.elapsed() > Duration::from_secs(10) {
				let _ = self.child.kill();
				panic!("the server didn't stop");
			}
			std::thread::sleep(Duration::from_millis(10));
		}
	}
}

#[test]
fn session_over_stdio() {
	let mut process = Process::start();
	let response = process.initialize();
	assert_eq!(
		response["result"]["capabilities"]["positionEncoding"],
		"utf-16"
	);

	let uri = "file:///project/main.fa";
	process.send(json!({
		"jsonrpc": "2.0", "method": "textDocument/didOpen",
		"params": { "textDocument": {
			"uri": uri, "languageId": "fa", "version": 1,
			"text": "function main = () => {\n\tlet a = (1\n}",
		} },
	}));
	let diagnostics =
		process.receive_until(|message| message["method"] == "textDocument/publishDiagnostics");
	assert_eq!(diagnostics["params"]["uri"], uri);
	assert!(
		!diagnostics["params"]["diagnostics"]
			.as_array()
			.unwrap()
			.is_empty()
	);

	process.send(json!({
		"jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol",
		"params": { "textDocument": { "uri": uri } },
	}));
	let symbols = process.receive_until(|message| message["id"] == 2);
	assert_eq!(symbols["result"][0]["name"], "main");

	process.send(json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }));
	let response = process.receive_until(|message| message["id"] == 3);
	assert_eq!(response["result"], Value::Null);
	process.send(json!({ "jsonrpc": "2.0", "method": "exit" }));
	assert_eq!(process.wait(), 0);
}

#[test]
fn exit_without_shutdown_is_an_error() {
	let mut process = Process::start();
	process.initialize();
	process.send(json!({ "jsonrpc": "2.0", "method": "exit" }));
	assert_eq!(process.wait(), 1);
}

#[test]
fn stops_at_the_end_of_stdin() {
	let mut process = Process::start();
	process.initialize();
	assert_eq!(process.wait(), 0);
}
