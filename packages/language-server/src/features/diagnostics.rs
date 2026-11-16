//! Conversion of the backend's diagnostics (byte spans) to LSP diagnostics.

use tower_lsp::lsp_types::{
	CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, DiagnosticRelatedInformation,
	DiagnosticSeverity, DiagnosticTag, Location, NumberOrString, Range, TextEdit, Url,
	WorkspaceEdit,
};

use crate::backend::{FileDiagnostic, Severity};
use crate::document::Document;
use crate::line_index::PositionEncoding;

/// Convert a diagnostic of `document`. The related information in other documents is kept
/// only for the documents `find_document` knows (their positions need their text).
pub fn to_lsp_diagnostic<'a>(
	diagnostic: &FileDiagnostic,
	document: &Document,
	encoding: PositionEncoding,
	find_document: impl Fn(&Url) -> Option<&'a Document>,
) -> Diagnostic {
	let related: Vec<DiagnosticRelatedInformation> = diagnostic
		.related
		.iter()
		.filter_map(|related| {
			let related_document = find_document(&related.uri)?;
			Some(DiagnosticRelatedInformation {
				location: Location::new(
					related.uri.clone(),
					related_document.range(related.span, encoding),
				),
				message: related.message.clone(),
			})
		})
		.collect();
	Diagnostic {
		range: document.range(diagnostic.span, encoding),
		severity: Some(match diagnostic.severity {
			Severity::Error => DiagnosticSeverity::ERROR,
			Severity::Warning => DiagnosticSeverity::WARNING,
			Severity::Information => DiagnosticSeverity::INFORMATION,
			Severity::Hint => DiagnosticSeverity::HINT,
		}),
		code: diagnostic.code.clone().map(NumberOrString::String),
		source: Some("fa".to_string()),
		message: diagnostic.message.clone(),
		related_information: if related.is_empty() {
			None
		} else {
			Some(related)
		},
		tags: diagnostic
			.unnecessary
			.then(|| vec![DiagnosticTag::UNNECESSARY]),
		..Default::default()
	}
}

/// Checker-provided fixes only; the editor applies all edits as one action.
pub fn code_actions<'a>(
	diagnostic: &FileDiagnostic,
	document: &Document,
	range: Range,
	encoding: PositionEncoding,
	find_document: impl Fn(&Url) -> Option<&'a Document>,
) -> Vec<CodeActionOrCommand> {
	let converted = to_lsp_diagnostic(diagnostic, document, encoding, &find_document);
	if converted.range.start > range.end || range.start > converted.range.end {
		return Vec::new();
	}
	diagnostic
		.fixes
		.iter()
		.filter_map(|fix| {
			let mut changes = std::collections::HashMap::<Url, Vec<TextEdit>>::new();
			for edit in &fix.edits {
				let document = find_document(&edit.uri)?;
				changes.entry(edit.uri.clone()).or_default().push(TextEdit {
					range: document.range(edit.span, encoding),
					new_text: edit.new_text.clone(),
				});
			}
			Some(CodeActionOrCommand::CodeAction(CodeAction {
				title: fix.title.clone(),
				kind: Some(CodeActionKind::QUICKFIX),
				diagnostics: Some(vec![converted.clone()]),
				edit: Some(WorkspaceEdit {
					changes: Some(changes),
					..Default::default()
				}),
				is_preferred: Some(true),
				..Default::default()
			}))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use fa_parser::SourceSpan;
	use tower_lsp::lsp_types::{Position, Range};

	use super::*;
	use crate::backend::RelatedInformation;

	fn uri(name: &str) -> Url {
		Url::parse(&format!("file:///project/{name}")).unwrap()
	}

	fn diagnostic(span: SourceSpan) -> FileDiagnostic {
		FileDiagnostic {
			severity: Severity::Error,
			code: None,
			message: "Unexpected token".to_string(),
			span,
			related: Vec::new(),
			unnecessary: false,
			fixes: Vec::new(),
		}
	}

	#[test]
	fn converts_the_span_with_the_encoding() {
		let document = Document::new("let s = \"😀\" )".to_string(), 1);
		let span = SourceSpan::new(15, 16);
		let converted = to_lsp_diagnostic(
			&diagnostic(span),
			&document,
			PositionEncoding::Utf16,
			|_| None,
		);
		assert_eq!(
			converted.range,
			Range::new(Position::new(0, 13), Position::new(0, 14))
		);
		assert_eq!(converted.severity, Some(DiagnosticSeverity::ERROR));
		assert_eq!(converted.source.as_deref(), Some("fa"));
		assert_eq!(converted.code, None);
		assert_eq!(converted.tags, None);
		let converted =
			to_lsp_diagnostic(&diagnostic(span), &document, PositionEncoding::Utf8, |_| {
				None
			});
		assert_eq!(
			converted.range,
			Range::new(Position::new(0, 15), Position::new(0, 16))
		);
	}

	#[test]
	fn severities_codes_and_tags() {
		let document = Document::new("let a = 1".to_string(), 1);
		let mut warning = diagnostic(SourceSpan::new(4, 5));
		warning.severity = Severity::Hint;
		warning.code = Some("W-DECL-1".to_string());
		warning.unnecessary = true;
		let converted = to_lsp_diagnostic(&warning, &document, PositionEncoding::Utf16, |_| None);
		assert_eq!(converted.severity, Some(DiagnosticSeverity::HINT));
		assert_eq!(
			converted.code,
			Some(NumberOrString::String("W-DECL-1".to_string()))
		);
		assert_eq!(converted.tags, Some(vec![DiagnosticTag::UNNECESSARY]));
	}

	#[test]
	fn related_information_in_known_documents_only() {
		let document = Document::new("let a = (\n".to_string(), 1);
		let other = Document::new("é\nb".to_string(), 1);
		let mut with_related = diagnostic(SourceSpan::new(10, 10));
		with_related.related = vec![
			RelatedInformation {
				uri: uri("a.fa"),
				span: SourceSpan::new(8, 9),
				message: "opened here".to_string(),
			},
			RelatedInformation {
				uri: uri("b.fa"),
				span: SourceSpan::new(3, 4),
				message: "declared here".to_string(),
			},
			RelatedInformation {
				uri: uri("closed.fa"),
				span: SourceSpan::new(0, 1),
				message: "unknown".to_string(),
			},
		];
		let converted = to_lsp_diagnostic(
			&with_related,
			&document,
			PositionEncoding::Utf16,
			|uri| match uri.path() {
				"/project/a.fa" => Some(&document),
				"/project/b.fa" => Some(&other),
				_ => None,
			},
		);
		let related = converted.related_information.unwrap();
		assert_eq!(related.len(), 2);
		assert_eq!(related[0].location.range.start, Position::new(0, 8));
		assert_eq!(related[1].location.uri, uri("b.fa"));
		assert_eq!(related[1].location.range.start, Position::new(1, 0));
		assert_eq!(converted.range.start, Position::new(1, 0));
	}

	#[test]
	fn a_copy_fix_uses_editor_positions_and_is_filtered_by_range() {
		use crate::backend::AnalysisBackend;
		use std::sync::Arc;
		let uri = uri("ownership.fa");
		let source = "let items = [1]\nlet duplicate = items\nconsole.log(items, duplicate)\n";
		let document = Document::new(source.into(), 1);
		let mut backend = crate::analyzer_backend::AnalyzerBackend::new();
		backend.set_document(&uri, Arc::from(source));
		let diagnostics = backend.diagnostics(&uri);
		let diagnostic = diagnostics
			.iter()
			.find(|diagnostic| diagnostic.code.as_deref() == Some("MEM004"))
			.unwrap();
		let range = Range::new(Position::new(1, 16), Position::new(1, 21));
		let actions = code_actions(
			diagnostic,
			&document,
			range,
			PositionEncoding::Utf16,
			|found| (found == &uri).then_some(&document),
		);
		let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
			panic!("{actions:?}")
		};
		let edits = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&uri];
		assert_eq!(
			edits,
			&[TextEdit {
				range: Range::new(range.start, range.start),
				new_text: "copy ".into()
			}]
		);
		let mut fixed = source.to_string();
		let start = document.offset(edits[0].range.start, PositionEncoding::Utf16);
		fixed.insert_str(start, &edits[0].new_text);
		backend.set_document(&uri, Arc::from(fixed));
		assert!(
			backend
				.diagnostics(&uri)
				.iter()
				.all(|diagnostic| diagnostic.severity != Severity::Error)
		);
		assert!(
			code_actions(
				diagnostic,
				&document,
				Range::default(),
				PositionEncoding::Utf16,
				|_| Some(&document)
			)
			.is_empty()
		);
		assert!(
			code_actions(
				diagnostic,
				&document,
				range,
				PositionEncoding::Utf16,
				|_| None
			)
			.is_empty()
		);
	}
}
