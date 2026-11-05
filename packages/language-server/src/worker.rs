//! The analysis thread.
//!
//! One thread owns the documents and the `AnalysisBackend`. The LSP handlers send it the edits
//! and the queries as messages, in the order they arrive: the backend is never shared, never
//! locked, and a query always sees every edit received before it.
//!
//! Edits are debounced: the backend analyzes a changed document once no edit has arrived for
//! `Options::debounce` (50 ms). A query on a document with pending edits analyzes it first.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fa_parser::TypedSyntaxTree;
use tokio::sync::mpsc::UnboundedSender;
use tower_lsp::lsp_types::{Diagnostic, MessageType, TextDocumentContentChangeEvent, Url};

use crate::backend::AnalysisBackend;
use crate::document::Document;
use crate::features::diagnostics::to_lsp_diagnostic;
use crate::line_index::PositionEncoding;

/// The settings of the analysis thread.
#[derive(Debug, Clone, Copy)]
pub struct Options {
	/// How long to wait after an edit before analyzing, so that typing doesn't trigger an
	/// analysis per keystroke.
	pub debounce: Duration,
}

impl Default for Options {
	fn default() -> Self {
		Options {
			debounce: Duration::from_millis(50),
		}
	}
}

/// A query run on the analysis thread, with the document it is about (`None` if the document
/// is not open).
pub type Query = Box<dyn for<'a> FnOnce(Option<QueryContext<'a>>) + Send>;

/// A message to the analysis thread.
pub enum Message {
	/// The position encoding negotiated by `initialize`.
	Configure {
		encoding: PositionEncoding,
	},
	Open {
		uri: Url,
		text: String,
		version: i32,
	},
	Change {
		uri: Url,
		version: i32,
		changes: Vec<TextDocumentContentChangeEvent>,
	},
	Close {
		uri: Url,
	},
	/// The editor saved a document (a configuration file is read again).
	Save {
		uri: Url,
	},
	Query {
		uri: Url,
		run: Query,
	},
	Shutdown,
}

/// A message from the analysis thread to the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum Outgoing {
	Diagnostics {
		uri: Url,
		diagnostics: Vec<Diagnostic>,
		version: Option<i32>,
	},
	Log {
		kind: MessageType,
		message: String,
	},
}

/// What a query can read: the document, its tree, the backend, and the other documents.
pub struct QueryContext<'a> {
	pub uri: &'a Url,
	pub document: &'a Document,
	pub tree: Option<&'a TypedSyntaxTree>,
	pub backend: &'a dyn AnalysisBackend,
	pub documents: &'a HashMap<Url, Document>,
	pub encoding: PositionEncoding,
}

/// Start the analysis thread. It stops after `Message::Shutdown`, or when every sender is
/// dropped.
pub fn spawn<B: AnalysisBackend>(
	backend: B,
	options: Options,
	outgoing: UnboundedSender<Outgoing>,
) -> (Sender<Message>, JoinHandle<()>) {
	let (sender, receiver) = std::sync::mpsc::channel();
	let cancel = backend.cancel_flag();
	let worker = Worker {
		backend,
		documents: HashMap::new(),
		dirty: Vec::new(),
		deadline: None,
		encoding: PositionEncoding::default(),
		options,
		outgoing,
		cancel,
	};
	let handle = std::thread::Builder::new()
		.name("fa-analysis".to_string())
		.spawn(move || worker.run(receiver))
		.expect("the analysis thread should start");
	(sender, handle)
}

struct Worker<B> {
	backend: B,
	documents: HashMap<Url, Document>,
	// the documents edited since their last analysis, in the order of their first edit
	dirty: Vec<Url>,
	// when to analyze the dirty documents
	deadline: Option<Instant>,
	encoding: PositionEncoding,
	options: Options,
	outgoing: UnboundedSender<Outgoing>,
	cancel: Option<Arc<AtomicBool>>,
}

impl<B: AnalysisBackend> Worker<B> {
	fn run(mut self, receiver: Receiver<Message>) {
		loop {
			let message = match self.deadline {
				None => match receiver.recv() {
					Ok(message) => message,
					Err(_) => return,
				},
				Some(deadline) => {
					let timeout = deadline.saturating_duration_since(Instant::now());
					match receiver.recv_timeout(timeout) {
						Ok(message) => message,
						Err(RecvTimeoutError::Timeout) => {
							self.guarded(|worker| worker.flush_all());
							continue;
						}
						Err(RecvTimeoutError::Disconnected) => return,
					}
				}
			};
			if matches!(message, Message::Shutdown) {
				return;
			}
			self.guarded(|worker| worker.handle(message));
		}
	}

	/// Run an operation, and report a panic of the backend instead of stopping the thread.
	fn guarded(&mut self, operation: impl FnOnce(&mut Self)) {
		if let Err(panic) = catch_unwind(AssertUnwindSafe(|| operation(self))) {
			let reason = panic
				.downcast_ref::<&str>()
				.map(|reason| reason.to_string())
				.or_else(|| panic.downcast_ref::<String>().cloned())
				.unwrap_or_else(|| "unknown reason".to_string());
			self.log(
				MessageType::ERROR,
				format!("The analysis of Fa failed: {reason}"),
			);
		}
	}

	fn handle(&mut self, message: Message) {
		match message {
			Message::Configure { encoding } => self.encoding = encoding,
			Message::Open { uri, text, version } => {
				self.documents
					.insert(uri.clone(), Document::new(text, version));
				// a document just opened is analyzed right away
				self.flush(&uri);
			}
			Message::Change {
				uri,
				version,
				changes,
			} => {
				let Some(document) = self.documents.get_mut(&uri) else {
					self.log(
						MessageType::WARNING,
						format!("Ignored a change of {uri}, which is not open"),
					);
					return;
				};
				document.apply_changes(changes, version, self.encoding);
				self.mark_dirty(uri);
			}
			Message::Close { uri } => {
				self.documents.remove(&uri);
				self.dirty.retain(|dirty| *dirty != uri);
				let changed = self.backend.close_document(&uri);
				// clear the diagnostics of the closed document
				self.send(Outgoing::Diagnostics {
					uri,
					diagnostics: Vec::new(),
					version: None,
				});
				for uri in changed {
					self.publish(&uri);
				}
			}
			Message::Save { uri } => {
				for uri in self.backend.saved(&uri) {
					self.publish(&uri);
				}
			}
			Message::Query { uri, run } => {
				if self.dirty.contains(&uri) && !self.flush(&uri) {
					// a newer edit stopped the analysis: the kept tree is
					// older than the text, and the answer would be outdated
					run(None);
					return;
				}
				let context = self.documents.get(&uri).map(|document| QueryContext {
					uri: &uri,
					document,
					tree: self.backend.tree(&uri),
					backend: &self.backend,
					documents: &self.documents,
					encoding: self.encoding,
				});
				run(context);
			}
			Message::Shutdown => {}
		}
	}

	fn mark_dirty(&mut self, uri: Url) {
		if !self.dirty.contains(&uri) {
			self.dirty.push(uri);
		}
		self.deadline = Some(Instant::now() + self.options.debounce);
	}

	fn flush_all(&mut self) {
		self.deadline = None;
		for uri in std::mem::take(&mut self.dirty) {
			self.flush(&uri);
		}
	}

	/// Analyze a document now, and publish the diagnostics that changed;
	/// `false` when a newer edit cancelled the analysis.
	fn flush(&mut self, uri: &Url) -> bool {
		self.dirty.retain(|dirty| dirty != uri);
		if self.dirty.is_empty() {
			self.deadline = None;
		}
		let Some(document) = self.documents.get(uri) else {
			return true;
		};
		if let Some(cancel) = &self.cancel {
			cancel.store(false, Ordering::SeqCst);
		}
		let changed = self
			.backend
			.set_document(uri, Arc::from(document.text.as_str()));
		let cancelled = self
			.cancel
			.as_ref()
			.is_some_and(|cancel| cancel.load(Ordering::SeqCst));
		if cancelled {
			// a newer edit stopped the analysis: analyze again after it
			let uri = uri.clone();
			self.mark_dirty(uri);
			return false;
		}
		for uri in changed {
			self.publish(&uri);
		}
		true
	}

	fn publish(&self, uri: &Url) {
		let Some(document) = self.documents.get(uri) else {
			return;
		};
		let diagnostics = self
			.backend
			.diagnostics(uri)
			.iter()
			.map(|diagnostic| {
				to_lsp_diagnostic(diagnostic, document, self.encoding, |uri| {
					self.documents.get(uri)
				})
			})
			.collect();
		self.send(Outgoing::Diagnostics {
			uri: uri.clone(),
			diagnostics,
			version: Some(document.version),
		});
	}

	fn log(&self, kind: MessageType, message: String) {
		self.send(Outgoing::Log { kind, message });
	}

	fn send(&self, message: Outgoing) {
		// the receiver is gone only when the server stops
		let _ = self.outgoing.send(message);
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Mutex;

	use fa_parser::SourceSpan;
	use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
	use tower_lsp::lsp_types::{Position, Range};

	use super::*;
	use crate::backend::{FileDiagnostic, ParseBackend, Severity};

	fn uri(name: &str) -> Url {
		Url::parse(&format!("file:///project/{name}")).unwrap()
	}

	fn options(milliseconds: u64) -> Options {
		Options {
			debounce: Duration::from_millis(milliseconds),
		}
	}

	fn open(uri: &Url, text: &str) -> Message {
		Message::Open {
			uri: uri.clone(),
			text: text.to_string(),
			version: 1,
		}
	}

	fn insert(uri: &Url, version: i32, at: (u32, u32), text: &str) -> Message {
		let position = Position::new(at.0, at.1);
		Message::Change {
			uri: uri.clone(),
			version,
			changes: vec![TextDocumentContentChangeEvent {
				range: Some(Range::new(position, position)),
				range_length: None,
				text: text.to_string(),
			}],
		}
	}

	fn next(receiver: &mut UnboundedReceiver<Outgoing>) -> Outgoing {
		let started = Instant::now();
		loop {
			if let Ok(message) = receiver.try_recv() {
				return message;
			}
			assert!(
				started.elapsed() < Duration::from_secs(5),
				"no message from the worker"
			);
			std::thread::sleep(Duration::from_millis(1));
		}
	}

	fn diagnostics_of(message: Outgoing) -> (Url, usize, Option<i32>) {
		match message {
			Outgoing::Diagnostics {
				uri,
				diagnostics,
				version,
			} => (uri, diagnostics.len(), version),
			Outgoing::Log { message, .. } => panic!("unexpected log: {message}"),
		}
	}

	/// The text of a document, read on the analysis thread.
	fn text_of(sender: &Sender<Message>, uri: &Url) -> Option<String> {
		let (reply, answer) = std::sync::mpsc::channel();
		sender
			.send(Message::Query {
				uri: uri.clone(),
				run: Box::new(move |context| {
					let _ = reply.send(context.map(|context| context.document.text.clone()));
				}),
			})
			.unwrap();
		answer.recv().unwrap()
	}

	#[test]
	fn open_publishes_diagnostics_right_away() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(10_000), outgoing);
		sender.send(open(&uri("a.fa"), "let a = (1")).unwrap();
		let (published, count, version) = diagnostics_of(next(&mut receiver));
		assert_eq!(published, uri("a.fa"));
		assert!(count > 0);
		assert_eq!(version, Some(1));
	}

	#[test]
	fn edits_are_debounced() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(100), outgoing);
		let a = uri("a.fa");
		sender.send(open(&a, "let a = 1")).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)).1, 0);
		for (version, character) in [(2, "("), (3, "2"), (4, " ")] {
			sender.send(insert(&a, version, (0, 8), character)).unwrap();
		}
		let started = Instant::now();
		let (_, count, version) = diagnostics_of(next(&mut receiver));
		assert!(started.elapsed() >= Duration::from_millis(80));
		assert_eq!(version, Some(4));
		assert!(count > 0);
		// a single publication for the three edits
		std::thread::sleep(Duration::from_millis(150));
		assert!(receiver.try_recv().is_err());
	}

	#[test]
	fn query_sees_every_previous_edit() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(10_000), outgoing);
		let a = uri("a.fa");
		sender.send(open(&a, "let a = 1")).unwrap();
		sender.send(insert(&a, 2, (0, 9), "\nlet b = 2")).unwrap();
		assert_eq!(
			text_of(&sender, &a).as_deref(),
			Some("let a = 1\nlet b = 2")
		);
		// the query analyzed the document: its diagnostics were published without waiting
		next(&mut receiver);
		assert_eq!(diagnostics_of(next(&mut receiver)).2, Some(2));
	}

	#[test]
	fn query_on_an_unknown_document() {
		let (outgoing, _receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(10), outgoing);
		assert_eq!(text_of(&sender, &uri("missing.fa")), None);
	}

	#[test]
	fn close_clears_the_diagnostics() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(10), outgoing);
		let a = uri("a.fa");
		sender.send(open(&a, "let a = (")).unwrap();
		assert!(diagnostics_of(next(&mut receiver)).1 > 0);
		sender.send(Message::Close { uri: a.clone() }).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)), (a.clone(), 0, None));
		assert_eq!(text_of(&sender, &a), None);
	}

	#[test]
	fn change_of_an_unknown_document_is_logged() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(ParseBackend::default(), options(10), outgoing);
		sender.send(insert(&uri("a.fa"), 2, (0, 0), "x")).unwrap();
		assert!(matches!(
			next(&mut receiver),
			Outgoing::Log {
				kind: MessageType::WARNING,
				..
			}
		));
	}

	#[test]
	fn shutdown_stops_the_thread() {
		let (outgoing, _receiver) = unbounded_channel();
		let (sender, handle) = spawn(ParseBackend::default(), options(10), outgoing);
		sender.send(Message::Shutdown).unwrap();
		handle.join().unwrap();
	}

	#[test]
	fn dropping_the_sender_stops_the_thread() {
		let (outgoing, _receiver) = unbounded_channel();
		let (sender, handle) = spawn(ParseBackend::default(), options(10), outgoing);
		drop(sender);
		handle.join().unwrap();
	}

	/// A backend whose analysis of `a.fa` also changes the diagnostics of `b.fa`, which can
	/// panic, and which records the texts it was given.
	#[derive(Default)]
	struct DependentBackend {
		texts: HashMap<Url, Arc<str>>,
		log: Arc<Mutex<Vec<String>>>,
		cancel: Arc<AtomicBool>,
		/// How many of the next analyses are cancelled.
		cancels: usize,
	}

	impl AnalysisBackend for DependentBackend {
		fn set_document(&mut self, uri: &Url, text: Arc<str>) -> Vec<Url> {
			if text.contains("panic") {
				panic!("the backend panicked");
			}
			if self.cancels > 0 {
				self.cancels -= 1;
				self.cancel.store(true, Ordering::SeqCst);
			}
			self.log
				.lock()
				.unwrap()
				.push(format!("{} {text}", uri.path()));
			self.texts.insert(uri.clone(), text);
			if uri.path().ends_with("a.fa") {
				vec![uri.clone(), super::tests::uri("b.fa")]
			} else {
				vec![uri.clone()]
			}
		}

		fn close_document(&mut self, uri: &Url) -> Vec<Url> {
			self.texts.remove(uri);
			vec![super::tests::uri("b.fa")]
		}

		fn tree(&self, _uri: &Url) -> Option<&TypedSyntaxTree> {
			None
		}

		fn diagnostics(&self, uri: &Url) -> Vec<FileDiagnostic> {
			// one diagnostic per document known to the backend
			vec![
				FileDiagnostic {
					severity: Severity::Warning,
					code: Some("TEST".to_string()),
					message: format!("{} documents", self.texts.len()),
					span: SourceSpan::new(0, 1),
					related: Vec::new(),
					unnecessary: false,
				};
				usize::from(self.texts.contains_key(uri))
			]
		}

		fn cancel_flag(&self) -> Option<Arc<AtomicBool>> {
			Some(self.cancel.clone())
		}
	}

	#[test]
	fn dependents_are_republished() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(DependentBackend::default(), options(10), outgoing);
		sender.send(open(&uri("b.fa"), "b")).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)).0, uri("b.fa"));
		sender.send(open(&uri("a.fa"), "a")).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)).0, uri("a.fa"));
		assert_eq!(diagnostics_of(next(&mut receiver)).0, uri("b.fa"));
		sender.send(Message::Close { uri: uri("a.fa") }).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)), (uri("a.fa"), 0, None));
		assert_eq!(diagnostics_of(next(&mut receiver)).0, uri("b.fa"));
	}

	#[test]
	fn panic_of_the_backend_is_logged_and_the_thread_continues() {
		let (outgoing, mut receiver) = unbounded_channel();
		let (sender, _) = spawn(DependentBackend::default(), options(10), outgoing);
		sender.send(open(&uri("b.fa"), "panic")).unwrap();
		match next(&mut receiver) {
			Outgoing::Log { kind, message } => {
				assert_eq!(kind, MessageType::ERROR);
				assert!(message.contains("the backend panicked"), "{message}");
			}
			other => panic!("expected a log, got {other:?}"),
		}
		sender.send(open(&uri("b.fa"), "fine")).unwrap();
		assert_eq!(diagnostics_of(next(&mut receiver)).0, uri("b.fa"));
	}

	#[test]
	fn cancelled_analysis_is_retried_and_not_published() {
		let (outgoing, mut receiver) = unbounded_channel();
		let backend = DependentBackend {
			cancels: 1,
			..Default::default()
		};
		let log = backend.log.clone();
		let (sender, _) = spawn(backend, options(20), outgoing);
		sender.send(open(&uri("b.fa"), "b")).unwrap();
		// the first analysis is cancelled: the second one, after the debounce, is published
		let (published, count, _) = diagnostics_of(next(&mut receiver));
		assert_eq!((published, count), (uri("b.fa"), 1));
		assert_eq!(log.lock().unwrap().len(), 2);
	}

	#[test]
	fn a_query_after_a_cancelled_analysis_gets_no_document() {
		let (outgoing, _receiver) = unbounded_channel();
		let backend = DependentBackend {
			cancels: 2,
			..Default::default()
		};
		let (sender, _) = spawn(backend, options(10_000), outgoing);
		sender.send(open(&uri("b.fa"), "b")).unwrap();
		// the query's analysis is cancelled too: its tree would be older than the text
		assert_eq!(text_of(&sender, &uri("b.fa")), None);
		// the next one is current
		assert_eq!(text_of(&sender, &uri("b.fa")).as_deref(), Some("b"));
	}
}
