//! In-process tests of the language server: JSON-RPC messages sent to the `LspService`, and the
//! notifications it sends back read from its `ClientSocket`.

use std::time::Duration;

use fa_language_server::FaLanguageServer;
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::LspService;
use tower_lsp::jsonrpc::Request;

const URI: &str = "file:///project/main.fa";

struct Client {
	service: LspService<FaLanguageServer>,
	// the notifications sent by the server
	notifications: UnboundedReceiver<Request>,
	next_id: i64,
}

impl Client {
	/// A server that is not initialized yet.
	fn new() -> Self {
		let (service, mut socket) = fa_language_server::service();
		let (sender, notifications) = unbounded_channel();
		tokio::spawn(async move {
			while let Some(message) = socket.next().await {
				if sender.send(message).is_err() {
					break;
				}
			}
		});
		Client {
			service,
			notifications,
			next_id: 0,
		}
	}

	/// An initialized server, with the capabilities of the client.
	async fn initialized(capabilities: Value) -> (Self, Value) {
		let mut client = Client::new();
		let result = client
			.request(
				"initialize",
				json!({ "processId": null, "rootUri": null, "capabilities": capabilities }),
			)
			.await
			.expect("initialize should succeed");
		client.notify("initialized", json!({})).await;
		(client, result)
	}

	/// A server initialized by a client that supports only UTF-16 (the LSP default).
	async fn utf16() -> Self {
		Client::initialized(json!({})).await.0
	}

	/// A server initialized by a client that prefers UTF-8.
	async fn utf8() -> Self {
		Client::initialized(json!({ "general": { "positionEncodings": ["utf-8", "utf-16"] } }))
			.await
			.0
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

	/// Send a request and return its result, or its error.
	async fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String> {
		self.next_id += 1;
		let mut request = Request::build(method).id(self.next_id);
		if !params.is_null() {
			request = request.params(params);
		}
		let request = request.finish();
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

	async fn open(&mut self, text: &str) {
		self.notify(
			"textDocument/didOpen",
			json!({ "textDocument": { "uri": URI, "languageId": "fa", "version": 1, "text": text } }),
		)
		.await;
	}

	async fn change(&mut self, version: i32, changes: Value) {
		self.notify(
			"textDocument/didChange",
			json!({ "textDocument": { "uri": URI, "version": version }, "contentChanges": changes }),
		)
		.await;
	}

	/// The next `publishDiagnostics` notification (the logs are skipped).
	async fn diagnostics(&mut self) -> Value {
		loop {
			let message = tokio::time::timeout(Duration::from_secs(5), self.notifications.recv())
				.await
				.expect("no diagnostics were published")
				.expect("the server stopped");
			if message.method() == "textDocument/publishDiagnostics" {
				return message.params().cloned().unwrap_or(Value::Null);
			}
		}
	}

	/// True if no diagnostics are published for a while.
	async fn no_more_diagnostics(&mut self, wait: Duration) -> bool {
		let deadline = tokio::time::Instant::now() + wait;
		loop {
			match tokio::time::timeout_at(deadline, self.notifications.recv()).await {
				Err(_) => return true,
				Ok(None) => return true,
				Ok(Some(message)) if message.method() == "textDocument/publishDiagnostics" => {
					return false;
				}
				Ok(Some(_)) => {}
			}
		}
	}

	async fn document_request(&mut self, method: &'static str, extra: Value) -> Value {
		let mut params = json!({ "textDocument": { "uri": URI } });
		if let (Some(params), Some(extra)) = (params.as_object_mut(), extra.as_object()) {
			params.extend(extra.clone());
		}
		self.request(method, params)
			.await
			.expect("the request should succeed")
	}

	async fn text(&mut self, position: (u32, u32), method: &'static str) -> Value {
		self.document_request(
			method,
			json!({ "position": { "line": position.0, "character": position.1 } }),
		)
		.await
	}
}

fn range(start: (u32, u32), end: (u32, u32)) -> Value {
	json!({
		"start": { "line": start.0, "character": start.1 },
		"end": { "line": end.0, "character": end.1 },
	})
}

fn edit(start: (u32, u32), end: (u32, u32), text: &str) -> Value {
	json!({ "range": range(start, end), "text": text })
}

fn messages(diagnostics: &Value) -> Vec<String> {
	diagnostics["diagnostics"]
		.as_array()
		.expect("diagnostics")
		.iter()
		.map(|diagnostic| diagnostic["message"].as_str().unwrap_or("").to_string())
		.collect()
}

/// Decode semantic tokens into (line, character, length, type index).
fn decode_tokens(data: &Value) -> Vec<(u32, u32, u32, u32)> {
	let numbers: Vec<u32> = data
		.as_array()
		.expect("semantic tokens")
		.iter()
		.map(|number| number.as_u64().unwrap() as u32)
		.collect();
	let mut tokens = Vec::new();
	let (mut line, mut character) = (0, 0);
	for chunk in numbers.chunks(5) {
		if chunk[0] > 0 {
			line += chunk[0];
			character = chunk[1];
		} else {
			character += chunk[1];
		}
		tokens.push((line, character, chunk[2], chunk[3]));
	}
	tokens
}

/* ------------------------------ Initialization ------------------------------ */

#[tokio::test]
async fn initialize_advertises_the_capabilities() {
	let (_, result) = Client::initialized(json!({})).await;
	let capabilities = &result["capabilities"];
	assert_eq!(capabilities["positionEncoding"], "utf-16");
	assert_eq!(
		capabilities["textDocumentSync"]["change"], 2,
		"incremental sync"
	);
	assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
	assert_eq!(capabilities["documentSymbolProvider"], true);
	assert_eq!(capabilities["foldingRangeProvider"], true);
	assert_eq!(capabilities["selectionRangeProvider"], true);
	assert_eq!(capabilities["hoverProvider"], true);
	assert_eq!(capabilities["definitionProvider"], true);
	assert_eq!(
		capabilities["completionProvider"]["triggerCharacters"],
		json!(["."])
	);
	let semantic_tokens = &capabilities["semanticTokensProvider"];
	assert_eq!(semantic_tokens["full"], true);
	assert!(
		semantic_tokens["legend"]["tokenTypes"]
			.as_array()
			.unwrap()
			.contains(&json!("keyword"))
	);
	assert_eq!(result["serverInfo"]["name"], "fa-language-server");
}

#[tokio::test]
async fn initialize_negotiates_utf8() {
	let (_, result) =
		Client::initialized(json!({ "general": { "positionEncodings": ["utf-16", "utf-8"] } }))
			.await;
	assert_eq!(result["capabilities"]["positionEncoding"], "utf-8");
}

#[tokio::test]
async fn initialize_falls_back_to_utf16() {
	let (_, result) =
		Client::initialized(json!({ "general": { "positionEncodings": ["utf-32"] } })).await;
	assert_eq!(result["capabilities"]["positionEncoding"], "utf-16");
	let (_, result) = Client::initialized(json!({ "general": {} })).await;
	assert_eq!(result["capabilities"]["positionEncoding"], "utf-16");
}

#[tokio::test]
async fn requests_before_initialize_are_rejected() {
	let mut client = Client::new();
	let result = client
		.request(
			"textDocument/documentSymbol",
			json!({ "textDocument": { "uri": URI } }),
		)
		.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn initialized_logs_a_message() {
	let mut client = Client::utf16().await;
	let message = tokio::time::timeout(Duration::from_secs(5), client.notifications.recv())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(message.method(), "window/logMessage");
}

#[tokio::test]
async fn shutdown_answers_null() {
	let mut client = Client::utf16().await;
	client.open("let a = 1").await;
	assert_eq!(
		client.request("shutdown", Value::Null).await,
		Ok(Value::Null)
	);
	// after `shutdown`, the requests are refused
	let result = client
		.request(
			"textDocument/documentSymbol",
			json!({ "textDocument": { "uri": URI } }),
		)
		.await;
	assert!(result.is_err());
}

/* -------------------------------- Diagnostics -------------------------------- */

#[tokio::test]
async fn open_publishes_the_syntax_errors() {
	let mut client = Client::utf16().await;
	client.open("let a = 1\nlet b = a ! 2").await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(diagnostics["uri"], URI);
	assert_eq!(diagnostics["version"], 1);
	let first = &diagnostics["diagnostics"][0];
	assert_eq!(first["range"], range((1, 10), (1, 11)));
	assert_eq!(first["severity"], 1);
	assert_eq!(first["source"], "fa");
	assert!(
		first["message"]
			.as_str()
			.unwrap()
			.contains("`!` is not an operator")
	);
}

#[tokio::test]
async fn open_valid_code_publishes_no_diagnostics() {
	let mut client = Client::utf16().await;
	client
		.open("function add = (a: Integer, b: Integer): Integer => a + b\nlet c = add(1, 2)")
		.await;
	assert!(messages(&client.diagnostics().await).is_empty());
}

#[tokio::test]
async fn incremental_change_fixes_an_error() {
	let mut client = Client::utf16().await;
	client.open("let a = (1").await;
	assert!(!messages(&client.diagnostics().await).is_empty());
	client.change(2, json!([edit((0, 10), (0, 10), ")")])).await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(diagnostics["version"], 2);
	assert!(messages(&diagnostics).is_empty());
}

#[tokio::test]
async fn incremental_change_introduces_an_error() {
	let mut client = Client::utf16().await;
	client.open("let a = 1\nlet b = 2").await;
	assert!(messages(&client.diagnostics().await).is_empty());
	client.change(2, json!([edit((1, 6), (1, 6), "& ")])).await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(
		diagnostics["diagnostics"][0]["range"],
		range((1, 6), (1, 7))
	);
}

#[tokio::test]
async fn error_after_an_accented_letter_in_utf16() {
	let mut client = Client::utf16().await;
	// `é` is 2 bytes but 1 UTF-16 unit: `!` is at byte 16, column 15
	client.open("let s = \"café\" ! x").await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(
		diagnostics["diagnostics"][0]["range"],
		range((0, 15), (0, 16))
	);

	let mut client = Client::utf16().await;
	client.open("let s = \"é\" ! x").await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(
		diagnostics["diagnostics"][0]["range"],
		range((0, 12), (0, 13))
	);
}

#[tokio::test]
async fn error_after_an_emoji_in_utf16_and_utf8() {
	// `😀` is 4 bytes, 2 UTF-16 units
	let text = "let s = \"😀\" ! x";
	let mut client = Client::utf16().await;
	client.open(text).await;
	assert_eq!(
		client.diagnostics().await["diagnostics"][0]["range"],
		range((0, 13), (0, 14))
	);
	let mut client = Client::utf8().await;
	client.open(text).await;
	assert_eq!(
		client.diagnostics().await["diagnostics"][0]["range"],
		range((0, 15), (0, 16))
	);
}

#[tokio::test]
async fn emoji_outside_of_a_string_is_one_error_of_two_utf16_units() {
	let mut client = Client::utf16().await;
	client.open("let a = 😀").await;
	let diagnostics = client.diagnostics().await;
	let ranges: Vec<&Value> = diagnostics["diagnostics"]
		.as_array()
		.unwrap()
		.iter()
		.map(|diagnostic| &diagnostic["range"])
		.collect();
	assert!(ranges.contains(&&range((0, 8), (0, 10))), "{ranges:?}");
}

#[tokio::test]
async fn error_after_cjk_text() {
	let mut client = Client::utf16().await;
	// each CJK character is 3 bytes and 1 UTF-16 unit
	client.open("let s = \"日本語\"\nlet t = s ! 1").await;
	assert_eq!(
		client.diagnostics().await["diagnostics"][0]["range"],
		range((1, 10), (1, 11))
	);
	client
		.change(
			2,
			json!([
				edit((0, 12), (0, 12), "😀"),
				edit((1, 0), (1, 0), "\"中\" ")
			]),
		)
		.await;
	// line 1 is now `"中" let t = s ! 1`
	let diagnostics = client.diagnostics().await;
	let ranges: Vec<&Value> = diagnostics["diagnostics"]
		.as_array()
		.unwrap()
		.iter()
		.map(|diagnostic| &diagnostic["range"])
		.collect();
	assert!(ranges.contains(&&range((1, 14), (1, 15))), "{ranges:?}");
}

#[tokio::test]
async fn edits_inside_multi_byte_text() {
	let mut client = Client::utf16().await;
	client.open("let s = \"a😀b\"").await;
	client.diagnostics().await;
	// replace the emoji (UTF-16 columns 10..12) with `é`, then add an error after it
	client
		.change(
			2,
			json!([edit((0, 10), (0, 12), "é"), edit((0, 13), (0, 13), " ! 1")]),
		)
		.await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(
		diagnostics["diagnostics"][0]["range"],
		range((0, 14), (0, 15))
	);
	let symbols = client
		.document_request("textDocument/documentSymbol", json!({}))
		.await;
	// the whole line is the declaration: `let s = "aéb" ! 1` is 17 UTF-16 units
	assert_eq!(symbols[0]["name"], "s");
}

#[tokio::test]
async fn edits_in_utf8_columns() {
	let mut client = Client::utf8().await;
	client.open("let s = \"é\"").await;
	client.diagnostics().await;
	// UTF-8 column 12 is after the closing quote (`é` is 2 bytes)
	client
		.change(2, json!([edit((0, 12), (0, 12), " & 1")]))
		.await;
	let diagnostics = client.diagnostics().await;
	assert_eq!(
		diagnostics["diagnostics"][0]["range"],
		range((0, 13), (0, 14))
	);
}

#[tokio::test]
async fn several_changes_in_one_notification() {
	let mut client = Client::utf16().await;
	client.open("let a = 1").await;
	client.diagnostics().await;
	client
		.change(
			2,
			json!([
				edit((0, 9), (0, 9), "\nlet b = 2"),
				edit((1, 8), (1, 9), "(3"),
				edit((0, 4), (0, 5), "first"),
			]),
		)
		.await;
	// `let first = 1\nlet b = (3`: the parenthesis is never closed
	let diagnostics = client.diagnostics().await;
	assert!(!messages(&diagnostics).is_empty());
	let symbols = client
		.document_request("textDocument/documentSymbol", json!({}))
		.await;
	assert_eq!(symbols[0]["name"], "first");
	assert_eq!(symbols[1]["name"], "b");
}

#[tokio::test]
async fn full_text_change() {
	let mut client = Client::utf16().await;
	client.open("let a = (").await;
	assert!(!messages(&client.diagnostics().await).is_empty());
	client.change(2, json!([{ "text": "let a = 1" }])).await;
	let diagnostics = client.diagnostics().await;
	assert!(messages(&diagnostics).is_empty());
	assert_eq!(diagnostics["version"], 2);
}

#[tokio::test]
async fn rapid_changes_are_debounced() {
	let mut client = Client::utf16().await;
	client.open("let a = 1").await;
	client.diagnostics().await;
	for (version, column) in (2..12).zip(9..) {
		client
			.change(version, json!([edit((0, column), (0, column), "1")]))
			.await;
	}
	// the ten edits arrive within the debounce delay: they are analyzed together (a very slow
	// machine could split them, never into one analysis per edit)
	let mut publications = 1;
	while client.diagnostics().await["version"] != 11 {
		publications += 1;
	}
	assert!(publications < 10, "{publications} publications");
	assert!(client.no_more_diagnostics(Duration::from_millis(200)).await);
}

#[tokio::test]
async fn close_clears_the_diagnostics() {
	let mut client = Client::utf16().await;
	client.open("let a = (").await;
	assert!(!messages(&client.diagnostics().await).is_empty());
	client
		.notify(
			"textDocument/didClose",
			json!({ "textDocument": { "uri": URI } }),
		)
		.await;
	let diagnostics = client.diagnostics().await;
	assert!(messages(&diagnostics).is_empty());
	assert_eq!(diagnostics["version"], Value::Null);
	// the document is gone
	let symbols = client
		.request(
			"textDocument/documentSymbol",
			json!({ "textDocument": { "uri": URI } }),
		)
		.await;
	assert_eq!(symbols, Ok(Value::Null));
}

#[tokio::test]
async fn several_documents() {
	let mut client = Client::utf16().await;
	for (index, text) in ["let a = (", "let b = 1"].iter().enumerate() {
		client
			.notify(
				"textDocument/didOpen",
				json!({ "textDocument": {
					"uri": format!("file:///project/{index}.fa"),
					"languageId": "fa",
					"version": 1,
					"text": text,
				} }),
			)
			.await;
	}
	let first = client.diagnostics().await;
	let second = client.diagnostics().await;
	assert_eq!(first["uri"], "file:///project/0.fa");
	assert!(!messages(&first).is_empty());
	assert_eq!(second["uri"], "file:///project/1.fa");
	assert!(messages(&second).is_empty());
}

#[tokio::test]
async fn crlf_document() {
	let mut client = Client::utf16().await;
	client.open("let a = 1\r\nlet b = a ! 2\r\n").await;
	assert_eq!(
		client.diagnostics().await["diagnostics"][0]["range"],
		range((1, 10), (1, 11))
	);
}

#[tokio::test]
async fn change_of_a_document_that_is_not_open_is_ignored() {
	let mut client = Client::utf16().await;
	client.change(2, json!([edit((0, 0), (0, 0), "x")])).await;
	assert!(client.no_more_diagnostics(Duration::from_millis(150)).await);
}

/* ----------------------------- Document symbols ----------------------------- */

#[tokio::test]
async fn document_symbols_are_nested() {
	let mut client = Client::utf16().await;
	client
		.open(
			"type User = {\n\tname: String\n\tfunction greet = (self): String => \"Hi {self.name}\"\n}\n\nnamespace utils = {\n\tlet x = 1\n}\n\nfunction main = () => {\n\tfunction helper = () => 1\n}",
		)
		.await;
	let symbols = client
		.document_request("textDocument/documentSymbol", json!({}))
		.await;
	let symbols = symbols.as_array().unwrap();
	assert_eq!(symbols.len(), 3);

	assert_eq!(symbols[0]["name"], "User");
	assert_eq!(symbols[0]["kind"], 23, "struct");
	assert_eq!(symbols[0]["range"], range((0, 0), (3, 1)));
	assert_eq!(symbols[0]["selectionRange"], range((0, 5), (0, 9)));
	assert_eq!(symbols[0]["children"][0]["name"], "name");
	assert_eq!(symbols[0]["children"][0]["kind"], 8, "field");
	assert_eq!(symbols[0]["children"][1]["name"], "greet");
	assert_eq!(symbols[0]["children"][1]["kind"], 6, "method");

	assert_eq!(symbols[1]["name"], "utils");
	assert_eq!(symbols[1]["kind"], 3, "namespace");
	assert_eq!(symbols[1]["children"][0]["name"], "x");
	assert_eq!(symbols[1]["children"][0]["kind"], 13, "variable");

	assert_eq!(symbols[2]["name"], "main");
	assert_eq!(symbols[2]["kind"], 12, "function");
	assert_eq!(symbols[2]["children"][0]["name"], "helper");
}

#[tokio::test]
async fn document_symbol_ranges_in_utf16() {
	let mut client = Client::utf16().await;
	client.open("let s = \"😀\"\nlet t = 1").await;
	let symbols = client
		.document_request("textDocument/documentSymbol", json!({}))
		.await;
	assert_eq!(symbols[0]["range"], range((0, 0), (0, 12)));
	assert_eq!(symbols[1]["selectionRange"], range((1, 4), (1, 5)));
}

#[tokio::test]
async fn document_symbols_follow_the_edits() {
	let mut client = Client::utf16().await;
	client.open("function a = () => 1").await;
	client
		.change(2, json!([edit((0, 9), (0, 10), "renamed")]))
		.await;
	// the query sees the edit even before the debounce delay is over
	let symbols = client
		.document_request("textDocument/documentSymbol", json!({}))
		.await;
	assert_eq!(symbols[0]["name"], "renamed");
}

#[tokio::test]
async fn requests_on_a_document_that_is_not_open() {
	let mut client = Client::utf16().await;
	for method in [
		"textDocument/documentSymbol",
		"textDocument/foldingRange",
		"textDocument/semanticTokens/full",
	] {
		let result = client
			.request(
				method,
				json!({ "textDocument": { "uri": "file:///missing.fa" } }),
			)
			.await;
		assert_eq!(result, Ok(Value::Null), "{method}");
	}
}

/* ------------------------------ Semantic tokens ------------------------------ */

#[tokio::test]
async fn semantic_tokens_of_a_declaration() {
	let (mut client, initialize) = Client::initialized(json!({})).await;
	let legend: Vec<String> =
		initialize["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
			.as_array()
			.unwrap()
			.iter()
			.map(|token_type| token_type.as_str().unwrap().to_string())
			.collect();
	client
		.open("function add = (a: Integer) => a + 1 -- sum")
		.await;
	let result = client
		.document_request("textDocument/semanticTokens/full", json!({}))
		.await;
	let tokens: Vec<(u32, u32, u32, String)> = decode_tokens(&result["data"])
		.into_iter()
		.map(|(line, character, length, token_type)| {
			(line, character, length, legend[token_type as usize].clone())
		})
		.collect();
	let expected = [
		(0, 0, 8, "keyword"),
		(0, 9, 3, "function"),
		(0, 13, 1, "operator"),
		(0, 16, 1, "parameter"),
		(0, 19, 7, "type"),
		(0, 28, 2, "operator"),
		(0, 31, 1, "variable"),
		(0, 33, 1, "operator"),
		(0, 35, 1, "number"),
		(0, 37, 6, "comment"),
	];
	let expected: Vec<(u32, u32, u32, String)> = expected
		.iter()
		.map(|(line, character, length, token_type)| {
			(*line, *character, *length, token_type.to_string())
		})
		.collect();
	assert_eq!(tokens, expected);
}

#[tokio::test]
async fn semantic_tokens_after_multi_byte_text() {
	let mut client = Client::utf16().await;
	client.open("let s = \"é😀\" + t\n-- 日本\nlet u = 2").await;
	let result = client
		.document_request("textDocument/semanticTokens/full", json!({}))
		.await;
	let tokens = decode_tokens(&result["data"]);
	// the string is 5 UTF-16 units: `"`, `é`, the emoji (2), `"`
	assert_eq!(tokens[3], (0, 8, 5, 12));
	assert_eq!((tokens[4].0, tokens[4].1), (0, 14));
	assert_eq!((tokens[5].0, tokens[5].1), (0, 16));
	// the comment is 5 UTF-16 units
	assert_eq!(tokens[6], (1, 0, 5, 11));
	assert_eq!((tokens[7].0, tokens[7].1), (2, 0));
}

#[tokio::test]
async fn semantic_tokens_in_utf8() {
	let mut client = Client::utf8().await;
	client.open("let s = \"é😀\" + t").await;
	let result = client
		.document_request("textDocument/semanticTokens/full", json!({}))
		.await;
	let tokens = decode_tokens(&result["data"]);
	assert_eq!(tokens[3], (0, 8, 8, 12));
	assert_eq!((tokens[4].0, tokens[4].1), (0, 17));
}

#[tokio::test]
async fn semantic_tokens_follow_incremental_edits() {
	let mut client = Client::utf16().await;
	client.open("let a = 1").await;
	client
		.change(2, json!([edit((0, 0), (0, 0), "\"😀\"\n")]))
		.await;
	let result = client
		.document_request("textDocument/semanticTokens/full", json!({}))
		.await;
	let tokens = decode_tokens(&result["data"]);
	assert_eq!(tokens[0], (0, 0, 4, 12));
	assert_eq!(tokens[1], (1, 0, 3, 10));
}

#[tokio::test]
async fn semantic_tokens_of_an_empty_document() {
	let mut client = Client::utf16().await;
	client.open("").await;
	let result = client
		.document_request("textDocument/semanticTokens/full", json!({}))
		.await;
	assert_eq!(result["data"], json!([]));
}

/* ---------------------------------- Folding ---------------------------------- */

#[tokio::test]
async fn folding_ranges_of_blocks_and_comments() {
	let mut client = Client::utf16().await;
	client
		.open("-- first\n-- second\nfunction main = () => {\n\tif ready {\n\t\tgo()\n\t}\n}\nlet list = [\n\t1,\n\t2,\n]")
		.await;
	let result = client
		.document_request("textDocument/foldingRange", json!({}))
		.await;
	let ranges: Vec<(u64, u64, Option<String>)> = result
		.as_array()
		.unwrap()
		.iter()
		.map(|range| {
			(
				range["startLine"].as_u64().unwrap(),
				range["endLine"].as_u64().unwrap(),
				range["kind"].as_str().map(str::to_string),
			)
		})
		.collect();
	assert_eq!(
		ranges,
		[
			(0, 1, Some("comment".to_string())),
			(2, 5, None),
			(3, 4, None),
			(7, 9, None),
		]
	);
}

#[tokio::test]
async fn folding_ranges_of_an_edited_document() {
	let mut client = Client::utf16().await;
	client.open("function main = () => { go() }").await;
	let result = client
		.document_request("textDocument/foldingRange", json!({}))
		.await;
	assert_eq!(result, json!([]));
	client
		.change(
			2,
			json!([edit((0, 23), (0, 24), "\n\t"), edit((1, 5), (1, 6), "\n")]),
		)
		.await;
	let result = client
		.document_request("textDocument/foldingRange", json!({}))
		.await;
	assert_eq!(result[0]["startLine"], 0);
	assert_eq!(result[0]["endLine"], 1);
}

/* ----------------------------- Selection ranges ----------------------------- */

#[tokio::test]
async fn selection_ranges_expand_outwards() {
	let mut client = Client::utf16().await;
	client.open("let s = \"😀\" + b * c").await;
	let result = client
		.document_request(
			"textDocument/selectionRange",
			json!({ "positions": [{ "line": 0, "character": 19 }] }),
		)
		.await;
	let mut ranges = Vec::new();
	let mut current = &result[0];
	while !current.is_null() {
		ranges.push(current["range"].clone());
		current = &current["parent"];
	}
	assert_eq!(
		ranges,
		[
			range((0, 19), (0, 20)),
			range((0, 15), (0, 20)),
			range((0, 8), (0, 20)),
			range((0, 0), (0, 20)),
		]
	);
}

#[tokio::test]
async fn one_selection_range_per_position() {
	let mut client = Client::utf16().await;
	client.open("let a = 1\nlet b = 2").await;
	let result = client
		.document_request(
			"textDocument/selectionRange",
			json!({ "positions": [{ "line": 0, "character": 4 }, { "line": 1, "character": 8 }] }),
		)
		.await;
	assert_eq!(result.as_array().unwrap().len(), 2);
	assert_eq!(result[0]["range"], range((0, 4), (0, 5)));
	assert_eq!(result[1]["range"], range((1, 8), (1, 9)));
}

/* ------------------------- Hover, definition, completion ------------------------- */

#[tokio::test]
async fn hover_shows_the_declaration_and_the_type() {
	let mut client = Client::utf16().await;
	client
		.open("let s = \"☃\"\nlet a = 1\nlet b = a\nconsole.log(\"😀\", s)")
		.await;
	let hover = client.text((2, 8), "textDocument/hover").await;
	assert_eq!(hover["contents"]["kind"], "markdown");
	assert_eq!(
		hover["contents"]["value"],
		"```fa\nlet a: Integer\n```\n\nhere `1`"
	);
	assert_eq!(hover["range"], range((2, 8), (2, 9)));
	// columns after a multi-byte name, in UTF-16
	let hover = client.text((3, 18), "textDocument/hover").await;
	assert_eq!(hover["contents"]["value"], "```fa\nlet s: String\n```");
	assert_eq!(hover["range"], range((3, 18), (3, 19)));
	let log = client.text((3, 9), "textDocument/hover").await;
	assert!(
		log["contents"]["value"]
			.as_str()
			.unwrap()
			.contains("console.log(...values): Void")
	);
	// nothing to say on a keyword
	assert_eq!(client.text((1, 1), "textDocument/hover").await, Value::Null);
}

#[tokio::test]
async fn definition_goes_to_the_declaration() {
	let mut client = Client::utf16().await;
	client.open("let a = 1\nlet b = a").await;
	let definition = client.text((1, 8), "textDocument/definition").await;
	assert_eq!(definition["uri"], URI);
	assert_eq!(definition["range"], range((0, 4), (0, 5)));
	// a built-in has no declaration in the file
	client
		.change(2, json!([edit((1, 8), (1, 9), "console")]))
		.await;
	assert_eq!(
		client.text((1, 9), "textDocument/definition").await,
		Value::Null
	);
}

#[tokio::test]
async fn diagnostics_of_the_analyzer_have_codes() {
	let mut client = Client::utf16().await;
	client
		.open("let small: Int8 = 300\nlet c: Character = \"ab\"\nlet x = [[1]]")
		.await;
	let diagnostics = client.diagnostics().await;
	let list = diagnostics["diagnostics"].as_array().unwrap();
	// the construct the analyzer doesn't support yet (AN001) is not published
	assert_eq!(list.len(), 2, "{list:?}");
	assert_eq!(list[0]["code"], "E-NUM-6");
	assert_eq!(list[0]["range"], range((0, 18), (0, 21)));
	assert_eq!(list[1]["code"], "E-STR-9");
	assert_eq!(list[1]["severity"], 1);
}

#[tokio::test]
async fn completion_offers_the_names_in_scope_and_the_members() {
	let mut client = Client::utf16().await;
	client.open("let total = 1\nmutable text = \"a\"\n").await;
	let result = client.text((2, 0), "textDocument/completion").await;
	let items = result.as_array().unwrap();
	let item = |label: &str| {
		items
			.iter()
			.find(|item| item["label"] == label)
			.unwrap_or_else(|| panic!("no `{label}`"))
			.clone()
	};
	assert_eq!(item("total")["kind"], 21, "constant");
	assert_eq!(item("total")["detail"], "Integer");
	assert_eq!(item("text")["kind"], 6, "variable");
	assert_eq!(item("console")["kind"], 9, "module");
	assert_eq!(item("let")["kind"], 14, "keyword");
	client
		.change(2, json!([edit((2, 0), (2, 0), "text.")]))
		.await;
	let result = client.text((2, 5), "textDocument/completion").await;
	let labels: Vec<&str> = result
		.as_array()
		.unwrap()
		.iter()
		.map(|item| item["label"].as_str().unwrap())
		.collect();
	assert!(labels.contains(&"length"), "{labels:?}");
	assert!(labels.contains(&"toUpperCase"));
	assert!(!labels.contains(&"let"));
}

#[tokio::test]
async fn completion_offers_the_keywords() {
	let mut client = Client::utf16().await;
	client.open("le").await;
	let result = client.text((0, 2), "textDocument/completion").await;
	let labels: Vec<&str> = result
		.as_array()
		.unwrap()
		.iter()
		.map(|item| item["label"].as_str().unwrap())
		.collect();
	assert!(labels.contains(&"let"));
	assert!(labels.contains(&"function"));
	let keyword = result
		.as_array()
		.unwrap()
		.iter()
		.find(|item| item["label"] == "let")
		.unwrap();
	assert_eq!(keyword["kind"], 14, "keyword");
}

#[tokio::test]
async fn completion_after_a_dot_offers_no_keyword() {
	let mut client = Client::utf16().await;
	client.open("let é = 1\nuser.").await;
	let result = client.text((1, 5), "textDocument/completion").await;
	assert_eq!(result, json!([]));
}
