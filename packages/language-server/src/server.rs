//! The LSP handlers: they forward the notifications to the analysis thread, and turn the
//! requests into queries run there.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use fa_parser::SourceSpan;
use tokio::sync::{mpsc, oneshot};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
	CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
	DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams,
	DocumentSymbolResponse, FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability,
	GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
	HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
	MarkupContent, MarkupKind, MessageType, OneOf, Range, SelectionRange, SelectionRangeParams,
	SelectionRangeProviderCapability, SemanticTokens, SemanticTokensFullOptions,
	SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
	SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
	TextDocumentSyncKind, TextDocumentSyncOptions, Url, WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer, LspService};

use crate::backend::{AnalysisBackend, ParseBackend};
use crate::features::{completion, folding, selection, semantic_tokens, symbols};
use crate::line_index::{LineIndex, PositionEncoding};
use crate::worker::{self, Message, Options, Outgoing, QueryContext};

/// The language server of Fa.
pub struct FaLanguageServer {
	client: Client,
	worker: Sender<Message>,
	cancel: Option<Arc<AtomicBool>>,
}

/// The service to serve over a transport, with the parse-only backend.
pub fn service() -> (LspService<FaLanguageServer>, tower_lsp::ClientSocket) {
	LspService::new(FaLanguageServer::new)
}

impl FaLanguageServer {
	/// A server that only parses (until the analyzer is plugged in).
	pub fn new(client: Client) -> Self {
		Self::with_backend(client, ParseBackend::default(), Options::default())
	}

	/// A server whose analysis thread runs `backend`. Must be called inside a Tokio runtime.
	pub fn with_backend<B: AnalysisBackend>(client: Client, backend: B, options: Options) -> Self {
		let cancel = backend.cancel_flag();
		let (outgoing, receiver) = mpsc::unbounded_channel();
		let (worker, _) = worker::spawn(backend, options, outgoing);
		tokio::spawn(forward(client.clone(), receiver));
		FaLanguageServer {
			client,
			worker,
			cancel,
		}
	}

	fn send(&self, message: Message) {
		// fails only after `shutdown`
		let _ = self.worker.send(message);
	}

	/// Run `query` on the analysis thread, after the edits received before it. `None` when the
	/// document is not open.
	async fn query<R: Send + 'static>(
		&self,
		uri: Url,
		query: impl for<'a> FnOnce(QueryContext<'a>) -> R + Send + 'static,
	) -> Option<R> {
		let (reply, answer) = oneshot::channel();
		self.send(Message::Query {
			uri,
			run: Box::new(move |context| {
				let _ = reply.send(context.map(query));
			}),
		});
		answer.await.ok().flatten()
	}
}

/// Send the messages of the analysis thread to the editor, in order.
async fn forward(client: Client, mut receiver: mpsc::UnboundedReceiver<Outgoing>) {
	while let Some(message) = receiver.recv().await {
		match message {
			Outgoing::Diagnostics {
				uri,
				diagnostics,
				version,
			} => client.publish_diagnostics(uri, diagnostics, version).await,
			Outgoing::Log { kind, message } => client.log_message(kind, message).await,
		}
	}
}

pub fn capabilities(encoding: PositionEncoding) -> ServerCapabilities {
	ServerCapabilities {
		position_encoding: Some(encoding.kind()),
		text_document_sync: Some(TextDocumentSyncCapability::Options(
			TextDocumentSyncOptions {
				open_close: Some(true),
				change: Some(TextDocumentSyncKind::INCREMENTAL),
				..Default::default()
			},
		)),
		document_symbol_provider: Some(OneOf::Left(true)),
		folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
		selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
		semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
			SemanticTokensOptions {
				work_done_progress_options: WorkDoneProgressOptions::default(),
				legend: semantic_tokens::legend(),
				range: Some(false),
				full: Some(SemanticTokensFullOptions::Bool(true)),
			},
		)),
		hover_provider: Some(HoverProviderCapability::Simple(true)),
		definition_provider: Some(OneOf::Left(true)),
		completion_provider: Some(CompletionOptions {
			trigger_characters: Some(vec![".".to_string()]),
			..Default::default()
		}),
		..Default::default()
	}
}

/// The range of a span of any document: an open one, or else the file on disk.
fn range_in(context: &QueryContext<'_>, uri: &Url, span: SourceSpan) -> Option<Range> {
	if let Some(document) = context.documents.get(uri) {
		return Some(document.range(span, context.encoding));
	}
	let text = std::fs::read_to_string(uri.to_file_path().ok()?).ok()?;
	Some(LineIndex::new(&text).range(&text, span, context.encoding))
}

#[tower_lsp::async_trait]
impl LanguageServer for FaLanguageServer {
	async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
		let supported = params
			.capabilities
			.general
			.as_ref()
			.and_then(|general| general.position_encodings.as_deref());
		let encoding = PositionEncoding::negotiate(supported);
		self.send(Message::Configure { encoding });
		Ok(InitializeResult {
			capabilities: capabilities(encoding),
			server_info: Some(ServerInfo {
				name: "fa-language-server".to_string(),
				version: Some(env!("CARGO_PKG_VERSION").to_string()),
			}),
		})
	}

	async fn initialized(&self, _: InitializedParams) {
		self.client
			.log_message(MessageType::INFO, "The Fa language server is ready")
			.await;
	}

	async fn shutdown(&self) -> Result<()> {
		self.send(Message::Shutdown);
		Ok(())
	}

	// the notifications are sent to the analysis thread before any `.await`, so that they keep
	// the order in which they arrived

	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		let document = params.text_document;
		self.send(Message::Open {
			uri: document.uri,
			text: document.text,
			version: document.version,
		});
	}

	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		if let Some(cancel) = &self.cancel {
			cancel.store(true, Ordering::SeqCst);
		}
		self.send(Message::Change {
			uri: params.text_document.uri,
			version: params.text_document.version,
			changes: params.content_changes,
		});
	}

	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		self.send(Message::Close {
			uri: params.text_document.uri,
		});
	}

	async fn document_symbol(
		&self,
		params: DocumentSymbolParams,
	) -> Result<Option<DocumentSymbolResponse>> {
		Ok(self
			.query(params.text_document.uri, |context| {
				let tree = context.tree?;
				Some(DocumentSymbolResponse::Nested(symbols::document_symbols(
					tree,
					context.document,
					context.encoding,
				)))
			})
			.await
			.flatten())
	}

	async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
		Ok(self
			.query(params.text_document.uri, |context| {
				Some(folding::folding_ranges(context.tree?, context.document))
			})
			.await
			.flatten())
	}

	async fn semantic_tokens_full(
		&self,
		params: SemanticTokensParams,
	) -> Result<Option<SemanticTokensResult>> {
		Ok(self
			.query(params.text_document.uri, |context| {
				let data = semantic_tokens::semantic_tokens(
					context.tree?,
					context.document,
					context.encoding,
				);
				Some(SemanticTokensResult::Tokens(SemanticTokens {
					result_id: None,
					data,
				}))
			})
			.await
			.flatten())
	}

	async fn selection_range(
		&self,
		params: SelectionRangeParams,
	) -> Result<Option<Vec<SelectionRange>>> {
		let positions = params.positions;
		Ok(self
			.query(params.text_document.uri, move |context| {
				let tree = context.tree?;
				Some(
					positions
						.iter()
						.map(|position| {
							let offset = context.document.offset(*position, context.encoding);
							selection::selection_range(
								tree,
								context.document,
								offset,
								context.encoding,
							)
						})
						.collect(),
				)
			})
			.await
			.flatten())
	}

	async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
		let position = params.text_document_position_params;
		Ok(self
			.query(position.text_document.uri, move |context| {
				let offset = context.document.offset(position.position, context.encoding);
				let hover = context.backend.hover(context.uri, offset)?;
				Some(Hover {
					contents: HoverContents::Markup(MarkupContent {
						kind: MarkupKind::Markdown,
						value: hover.markdown,
					}),
					range: hover
						.span
						.map(|span| context.document.range(span, context.encoding)),
				})
			})
			.await
			.flatten())
	}

	async fn goto_definition(
		&self,
		params: GotoDefinitionParams,
	) -> Result<Option<GotoDefinitionResponse>> {
		let position = params.text_document_position_params;
		Ok(self
			.query(position.text_document.uri, move |context| {
				let offset = context.document.offset(position.position, context.encoding);
				let target = context.backend.definition(context.uri, offset)?;
				let range = range_in(&context, &target.uri, target.name_span)?;
				Some(GotoDefinitionResponse::Scalar(Location::new(
					target.uri, range,
				)))
			})
			.await
			.flatten())
	}

	async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
		let position = params.text_document_position;
		Ok(self
			.query(position.text_document.uri, move |context| {
				let offset = context.document.offset(position.position, context.encoding);
				let names = context.backend.completions(context.uri, offset);
				CompletionResponse::Array(completion::completion_items(
					&context.document.text,
					offset,
					names,
				))
			})
			.await)
	}
}
