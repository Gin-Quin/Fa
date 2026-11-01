//! Conversion of the backend's diagnostics (byte spans) to LSP diagnostics.

use tower_lsp::lsp_types::{
	Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, Location,
	NumberOrString, Url,
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
}
