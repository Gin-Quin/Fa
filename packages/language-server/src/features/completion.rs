//! `textDocument/completion`: the keywords, plus the names the backend knows.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::backend::CompletionInfo;

/// The keywords of Fa, offered where a name can be written.
pub const KEYWORDS: &[&str] = &[
	"and",
	"as",
	"assert",
	"async",
	"await",
	"break",
	"catch",
	"continue",
	"derived",
	"do",
	"else",
	"export",
	"false",
	"for",
	"function",
	"if",
	"in",
	"invalidates",
	"is",
	"let",
	"loop",
	"mutable",
	"mutates",
	"namespace",
	"none",
	"not",
	"or",
	"parallel",
	"provide",
	"reactive",
	"readonly",
	"requires",
	"return",
	"reuse",
	"start",
	"static",
	"test",
	"then",
	"throw",
	"throws",
	"true",
	"try",
	"type",
	"use",
	"watch",
	"when",
	"while",
	"with",
];

/// True when the cursor is right after a `.` (and the name being written after it): only the
/// members of the value before it make sense there, not keywords.
pub fn is_after_dot(text: &str, offset: usize) -> bool {
	let before = text.get(..offset).unwrap_or("");
	let before = before
		.trim_end_matches(|character: char| character.is_ascii_alphanumeric() || character == '_');
	before.ends_with('.') && !before.ends_with("..")
}

/// The completion items at an offset: the backend's names, then the keywords (unless after a
/// `.`).
pub fn completion_items(
	text: &str,
	offset: usize,
	names: Vec<CompletionInfo>,
) -> Vec<CompletionItem> {
	let mut items: Vec<CompletionItem> = names
		.into_iter()
		.map(|name| CompletionItem {
			label: name.label,
			kind: Some(name.kind),
			detail: name.detail,
			..Default::default()
		})
		.collect();
	if !is_after_dot(text, offset) {
		items.extend(KEYWORDS.iter().map(|keyword| CompletionItem {
			label: keyword.to_string(),
			kind: Some(CompletionItemKind::KEYWORD),
			..Default::default()
		}));
	}
	items
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn keywords_are_sorted_and_unique() {
		let mut sorted = KEYWORDS.to_vec();
		sorted.sort_unstable();
		sorted.dedup();
		assert_eq!(sorted, KEYWORDS);
	}

	#[test]
	fn every_keyword_is_a_keyword_of_the_tokenizer() {
		use fa_parser::tokens::TokenKind;
		for keyword in KEYWORDS {
			let tokens = fa_parser::tokenize::tokenize(&format!("{keyword} x"));
			// `test` is recognized by the parser, the others by the tokenizer
			if *keyword != "test" {
				assert_ne!(tokens[0].kind, TokenKind::Identifier, "{keyword}");
			}
		}
	}

	#[test]
	fn after_a_dot() {
		assert!(is_after_dot("user.", 5));
		assert!(is_after_dot("user.na", 7));
		assert!(is_after_dot("user?.na", 8));
		assert!(!is_after_dot("user", 4));
		assert!(!is_after_dot("1..", 3));
		assert!(!is_after_dot("", 0));
		assert!(!is_after_dot("let a = b", 9));
	}

	#[test]
	fn keywords_are_offered_at_the_start_of_a_statement() {
		let items = completion_items("le", 2, Vec::new());
		assert!(items.iter().any(|item| item.label == "let"));
		assert!(
			items
				.iter()
				.all(|item| item.kind == Some(CompletionItemKind::KEYWORD))
		);
	}

	#[test]
	fn no_keywords_after_a_dot() {
		let names = vec![CompletionInfo {
			label: "name".to_string(),
			kind: CompletionItemKind::FIELD,
			detail: Some("String".to_string()),
		}];
		let items = completion_items("user.", 5, names);
		assert_eq!(items.len(), 1);
		assert_eq!(items[0].label, "name");
		assert_eq!(items[0].detail.as_deref(), Some("String"));
	}

	#[test]
	fn names_come_before_keywords() {
		let names = vec![CompletionInfo {
			label: "total".to_string(),
			kind: CompletionItemKind::VARIABLE,
			detail: None,
		}];
		let items = completion_items("t", 1, names);
		assert_eq!(items[0].label, "total");
		assert_eq!(items.len(), 1 + KEYWORDS.len());
	}
}
