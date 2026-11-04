//! `fa bind <header.h> [--output <file>] [-- flags...]`: writes the foreign
//! file of a C header (documentation/advanced/foreign-code.md, "Generating
//! bindings").
//!
//! The header is read by a small parser of C declarations, without a C
//! compiler or libclang:
//!
//! - comments are removed and continued lines are joined;
//! - `#define NAME <integer expression>` becomes a constant, another object
//!   macro a comment; the other preprocessor lines are skipped, with the
//!   blocks of `#if 0` (every other branch of `#if`/`#ifdef` is read, and
//!   `#include`s are not followed);
//! - the declarations are split at `;`, and the bodies of inline functions
//!   are skipped; `extern "C" {` wrappers, attributes (`__attribute__((...))`,
//!   `__declspec(...)`) and the qualifiers that don't change a type
//!   (`restrict`, `volatile`, `inline`...) are dropped;
//! - unknown words before a type, like `SQLITE_API` in `SQLITE_API int`, are
//!   dropped: when a declaration names several types, a C keyword type wins,
//!   then a type declared by the header or a standard one, then the last one.
//!
//! What can't be read or translated (unions, bit fields, variadic functions,
//! variables, macros that aren't integer constants) becomes a
//! `-- not supported` comment. The parser never panics: every recursion is
//! bounded, and every index is checked.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use crate::USAGE;

/// The deepest nesting of declarators, parameters and records read.
const MAX_DEPTH: usize = 24;
/// The most pointer levels, or array dimensions, of one declarator.
const MAX_LEVELS: usize = 12;
/// The longest C declaration copied into a comment.
const MAX_COMMENT: usize = 240;

/// The C types with a fixed Fa spelling, written as names in C.
const STANDARD_TYPES: &[(&str, &str)] = &[
	("int8_t", "Int8"),
	("int16_t", "Int16"),
	("int32_t", "Int32"),
	("int64_t", "Int64"),
	("uint8_t", "UInt8"),
	("uint16_t", "UInt16"),
	("uint32_t", "UInt32"),
	("uint64_t", "UInt64"),
	("bool", "Boolean"),
	("size_t", "c.SizeT"),
	("ssize_t", "c.SSizeT"),
	("ptrdiff_t", "c.PtrDiffT"),
	("intptr_t", "c.IntPtrT"),
	("uintptr_t", "c.UIntPtrT"),
];

/// The words that can't name a Fa value: the keywords of
/// `packages/parser/src/tokenize.rs`, `self`, and `c` (the alias of `std.c`).
const RESERVED_VALUES: &[&str] = &[
	"let",
	"function",
	"mutable",
	"static",
	"type",
	"reactive",
	"derived",
	"namespace",
	"return",
	"if",
	"else",
	"for",
	"parallel",
	"while",
	"loop",
	"do",
	"watch",
	"when",
	"export",
	"true",
	"false",
	"none",
	"use",
	"reuse",
	"async",
	"await",
	"continue",
	"break",
	"and",
	"or",
	"not",
	"is",
	"in",
	"as",
	"then",
	"with",
	"throws",
	"throw",
	"try",
	"catch",
	"start",
	"provide",
	"assert",
	"requires",
	"mutates",
	"invalidates",
	"readonly",
	"self",
	"c",
];

/// The words that can't name a Fa type: the keyword `Error` and the types
/// a foreign file uses.
const RESERVED_TYPES: &[&str] = &[
	"Error", "Self", "Void", "Pointer", "CString", "CStruct", "CArray", "Boolean", "String",
	"Integer", "Array", "Int8", "Int16", "Int32", "Int64", "UInt8", "UInt16", "UInt32", "UInt64",
	"Float32", "Float64",
];

/// Words dropped wherever they appear: they don't change a type.
const NOISE_WORDS: &[&str] = &[
	"extern",
	"inline",
	"__inline",
	"__inline__",
	"__extension__",
	"restrict",
	"__restrict",
	"__restrict__",
	"volatile",
	"__volatile__",
	"register",
	"auto",
	"_Nullable",
	"__nullable",
	"_Null_unspecified",
	"__null_unspecified",
	"_Noreturn",
	"__cdecl",
	"__stdcall",
	"__fastcall",
	"_Thread_local",
	"thread_local",
	"__thread",
	"__THROW",
	"__wur",
];

/// Words dropped with the parenthesized arguments that follow them.
const NOISE_CALLS: &[&str] = &[
	"__attribute__",
	"__attribute",
	"__declspec",
	"__asm__",
	"__asm",
	"asm",
	"_Alignas",
	"alignas",
	"__nonnull",
	"_Static_assert",
	"static_assert",
];

/// `fa bind <header.h> [--output <file>] [-- flags...]`.
pub(crate) fn command(arguments: &[OsString]) -> i32 {
	let mut header: Option<PathBuf> = None;
	let mut output: Option<PathBuf> = None;
	let mut index = 0;
	while index < arguments.len() {
		let text = arguments[index].to_string_lossy();
		if text == "--" {
			// the flags of the C compiler: ignored for now
			break;
		}
		if text == "--output" || text == "-o" {
			let Some(value) = arguments.get(index + 1) else {
				eprintln!(
					"fa bind: `--output` needs a file, ex: `--output library/sqlite3.foreign.fa`"
				);
				return USAGE;
			};
			output = Some(PathBuf::from(value));
			index += 2;
			continue;
		}
		if let Some(value) = text.strip_prefix("--output=") {
			output = Some(PathBuf::from(value));
		} else if text.starts_with('-') && text.len() > 1 {
			eprintln!("fa bind: unknown option `{text}`");
			return USAGE;
		} else if header.is_some() {
			eprintln!("fa bind: unexpected argument `{text}`: give one header");
			return USAGE;
		} else {
			header = Some(PathBuf::from(&arguments[index]));
		}
		index += 1;
	}
	let Some(header) = header else {
		eprintln!(
			"fa bind: which header? ex: `fa bind sqlite3.h --output library/sqlite3.foreign.fa`"
		);
		return USAGE;
	};
	let bytes = match fs::read(&header) {
		Ok(bytes) => bytes,
		Err(error) => {
			eprintln!("fa bind: can't read `{}`: {error}", header.display());
			return 1;
		}
	};
	let source = String::from_utf8_lossy(&bytes);
	let shown = header
		.file_name()
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_else(|| header.display().to_string());
	let text = generate(&source, &shown);
	let Some(path) = output else {
		print!("{text}");
		return 0;
	};
	if let Some(parent) = path
		.parent()
		.filter(|parent| !parent.as_os_str().is_empty())
		&& let Err(error) = fs::create_dir_all(parent)
	{
		eprintln!("fa bind: can't create `{}`: {error}", parent.display());
		return 1;
	}
	match fs::write(&path, text) {
		Ok(()) => {
			eprintln!("fa bind: wrote `{}`", path.display());
			0
		}
		Err(error) => {
			eprintln!("fa bind: can't write `{}`: {error}", path.display());
			1
		}
	}
}

/// The foreign file of a C header; `shown` names the header in its first
/// comment.
pub(crate) fn generate(source: &str, shown: &str) -> String {
	let (code, defines) = preprocess(&strip_comments(source));
	let tokens = clean(tokenize(&code));
	let mut binder = Binder::new(&code);
	let constants = binder.define_constants(&defines);
	for declaration in split_declarations(&tokens) {
		binder.declaration(declaration);
	}
	Writer::new(&binder).write(shown, &constants)
}

// --- Reading the text ---------------------------------------------------

/// Replaces the comments by spaces, leaving strings and characters intact.
fn strip_comments(source: &str) -> String {
	let bytes = source.as_bytes();
	let length = bytes.len();
	let mut output = Vec::with_capacity(length);
	let mut index = 0;
	while index < length {
		let byte = bytes[index];
		let next = bytes.get(index + 1).copied();
		if byte == b'/' && next == Some(b'/') {
			while index < length && bytes[index] != b'\n' {
				index += 1;
			}
		} else if byte == b'/' && next == Some(b'*') {
			index += 2;
			while index < length && !(bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/')) {
				index += 1;
			}
			index = (index + 2).min(length);
			output.push(b' ');
		} else if byte == b'"' || byte == b'\'' {
			output.push(byte);
			index += 1;
			while index < length {
				let current = bytes[index];
				output.push(current);
				index += 1;
				if current == b'\\' {
					if let Some(&escaped) = bytes.get(index) {
						output.push(escaped);
						index += 1;
					}
				} else if current == byte || current == b'\n' {
					break;
				}
			}
		} else {
			output.push(byte);
			index += 1;
		}
	}
	match String::from_utf8(output) {
		Ok(text) => text,
		Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
	}
}

/// A `#define NAME value` line with a value.
struct Define {
	name: String,
	value: String,
	text: String,
}

/// Joins the continued lines and takes the preprocessor lines out of the
/// code: the code, and the object macros with a value.
fn preprocess(text: &str) -> (String, Vec<Define>) {
	let mut lines: Vec<String> = Vec::new();
	let mut current = String::new();
	for line in text.lines() {
		let trimmed = line.trim_end();
		if let Some(continued) = trimmed.strip_suffix('\\') {
			current.push_str(continued);
			current.push(' ');
		} else {
			current.push_str(line);
			lines.push(std::mem::take(&mut current));
		}
	}
	if !current.is_empty() {
		lines.push(current);
	}
	let mut code = String::with_capacity(text.len());
	let mut defines = Vec::new();
	// the nesting of `#if` inside a skipped `#if 0` block, plus one
	let mut skipping = 0usize;
	for line in &lines {
		let trimmed = line.trim_start();
		let Some(directive) = trimmed.strip_prefix('#') else {
			if skipping == 0 {
				code.push_str(line);
			}
			code.push('\n');
			continue;
		};
		code.push('\n');
		let directive = directive.trim_start();
		let word_end = directive
			.find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
			.unwrap_or(directive.len());
		let (word, rest) = directive.split_at(word_end);
		if skipping > 0 {
			match word {
				"if" | "ifdef" | "ifndef" => skipping += 1,
				"endif" => skipping -= 1,
				"else" | "elif" | "elifdef" | "elifndef" if skipping == 1 => skipping = 0,
				_ => {}
			}
			continue;
		}
		match word {
			"if" if rest.trim() == "0" => skipping = 1,
			"define" => {
				if let Some(define) = read_define(rest) {
					defines.push(define);
				}
			}
			_ => {}
		}
	}
	(code, defines)
}

/// Reads `NAME value` after `#define`; `None` for a function-like macro or a
/// macro without a value.
fn read_define(rest: &str) -> Option<Define> {
	let rest = rest.trim_start();
	let name_end = rest
		.find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
		.unwrap_or(rest.len());
	let (name, value) = rest.split_at(name_end);
	if name.is_empty() || name.starts_with(|character: char| character.is_ascii_digit()) {
		return None;
	}
	if value.starts_with('(') {
		return None;
	}
	let value = value.trim();
	if value.is_empty() {
		return None;
	}
	Some(Define {
		name: name.to_string(),
		value: value.to_string(),
		text: one_line(&format!("#define {name} {value}")),
	})
}

/// A C token, with its position in the code.
#[derive(Clone, Debug)]
struct Token {
	text: String,
	start: usize,
	end: usize,
}

/// Splits C code into tokens: names, numbers, strings, characters, `...`,
/// `<<`, `>>`, and single characters.
fn tokenize(code: &str) -> Vec<Token> {
	let bytes = code.as_bytes();
	let length = bytes.len();
	let mut tokens = Vec::new();
	let mut index = 0;
	while index < length {
		let byte = bytes[index];
		if byte.is_ascii_whitespace() {
			index += 1;
			continue;
		}
		let start = index;
		let next = bytes.get(index + 1).copied();
		if byte.is_ascii_alphabetic() || byte == b'_' {
			while index < length && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
				index += 1;
			}
		} else if byte.is_ascii_digit()
			|| (byte == b'.' && next.is_some_and(|next| next.is_ascii_digit()))
		{
			index += 1;
			while index < length {
				let current = bytes[index];
				let after_exponent = matches!(bytes[index - 1], b'e' | b'E' | b'p' | b'P');
				if current.is_ascii_alphanumeric()
					|| current == b'_'
					|| current == b'.'
					|| ((current == b'+' || current == b'-') && after_exponent)
				{
					index += 1;
				} else {
					break;
				}
			}
		} else if byte == b'"' || byte == b'\'' {
			index += 1;
			while index < length {
				let current = bytes[index];
				index += 1;
				if current == b'\\' {
					index += 1;
				} else if current == byte || current == b'\n' {
					break;
				}
			}
			index = index.min(length);
		} else if code[index..].starts_with("...") {
			index += 3;
		} else if code[index..].starts_with("<<") || code[index..].starts_with(">>") {
			index += 2;
		} else {
			index += 1;
		}
		while index < length && !code.is_char_boundary(index) {
			index += 1;
		}
		tokens.push(Token {
			text: code[start..index].to_string(),
			start,
			end: index,
		});
	}
	tokens
}

/// Drops `extern "C" {`, the attributes and the words that don't change a
/// type; `__nonnull` without arguments becomes `_Nonnull`.
fn clean(tokens: Vec<Token>) -> Vec<Token> {
	let mut cleaned = Vec::with_capacity(tokens.len());
	let mut index = 0;
	while index < tokens.len() {
		let text = tokens[index].text.as_str();
		let next = tokens.get(index + 1).map(|token| token.text.as_str());
		if text == "extern" && next.is_some_and(|next| next.starts_with('"')) {
			index += 2;
			if tokens.get(index).is_some_and(|token| token.text == "{") {
				// its `}` is dropped where a declaration would start
				index += 1;
			}
			continue;
		}
		if NOISE_CALLS.contains(&text) && next == Some("(") {
			index = matching(&tokens, index + 1).map_or(tokens.len(), |close| close + 1);
			continue;
		}
		if text == "__nonnull" {
			let mut token = tokens[index].clone();
			token.text = "_Nonnull".to_string();
			cleaned.push(token);
		} else if !NOISE_WORDS.contains(&text) && !NOISE_CALLS.contains(&text) {
			cleaned.push(tokens[index].clone());
		}
		index += 1;
	}
	cleaned
}

/// The index of the bracket closing the one at `open`, if it is closed.
fn matching(tokens: &[Token], open: usize) -> Option<usize> {
	let opening = tokens.get(open)?.text.as_str();
	let closing = match opening {
		"(" => ")",
		"[" => "]",
		"{" => "}",
		_ => return None,
	};
	let mut depth = 0usize;
	for (index, token) in tokens.iter().enumerate().skip(open) {
		if token.text == opening {
			depth += 1;
		} else if token.text == closing {
			depth -= 1;
			if depth == 0 {
				return Some(index);
			}
		}
	}
	None
}

/// A declaration of the header, without its `;`.
struct Declaration<'a> {
	tokens: &'a [Token],
	/// Whether it ended with `;` (the last one of a file may not).
	complete: bool,
}

/// Splits the tokens into declarations, at the `;` outside braces; the
/// definitions of functions (inline functions) are skipped.
fn split_declarations(tokens: &[Token]) -> Vec<Declaration<'_>> {
	let mut declarations = Vec::new();
	let mut start = 0;
	let mut braces = 0usize;
	let mut index = 0;
	while index < tokens.len() {
		match tokens[index].text.as_str() {
			"{" => {
				if braces == 0 && index > start && tokens[index - 1].text == ")" {
					// a function definition: skip its body, and the declaration
					index = matching(tokens, index).map_or(tokens.len(), |close| close + 1);
					start = index;
					continue;
				}
				braces += 1;
			}
			"}" => {
				if braces == 0 {
					// the end of `extern "C" {`, or a stray brace
					if index == start {
						start = index + 1;
					}
				} else {
					braces -= 1;
				}
			}
			";" if braces == 0 => {
				if index > start {
					declarations.push(Declaration {
						tokens: &tokens[start..index],
						complete: true,
					});
				}
				start = index + 1;
			}
			_ => {}
		}
		index += 1;
	}
	if start < tokens.len() {
		declarations.push(Declaration {
			tokens: &tokens[start..],
			complete: false,
		});
	}
	declarations
}

/// Splits tokens at a separator outside brackets.
fn split<'a>(tokens: &'a [Token], separator: &str) -> Vec<&'a [Token]> {
	let mut parts = Vec::new();
	let mut depth = 0usize;
	let mut start = 0;
	for (index, token) in tokens.iter().enumerate() {
		match token.text.as_str() {
			"(" | "[" | "{" => depth += 1,
			")" | "]" | "}" => depth = depth.saturating_sub(1),
			text if text == separator && depth == 0 => {
				parts.push(&tokens[start..index]);
				start = index + 1;
			}
			_ => {}
		}
	}
	parts.push(&tokens[start..]);
	parts
}

/// Whether a token appears outside brackets.
fn contains_at_top(tokens: &[Token], text: &str) -> bool {
	split(tokens, text).len() > 1
}

fn is_identifier(text: &str) -> bool {
	text.starts_with(|character: char| character.is_ascii_alphabetic() || character == '_')
}

/// A text on one line, with its spaces collapsed, shortened if it is long.
fn one_line(text: &str) -> String {
	let mut result = String::new();
	for word in text.split_whitespace() {
		if !result.is_empty() {
			result.push(' ');
		}
		result.push_str(word);
	}
	let result: String = result
		.chars()
		.filter(|character| !character.is_control())
		.collect();
	if result.chars().count() > MAX_COMMENT {
		let mut shortened: String = result.chars().take(MAX_COMMENT).collect();
		shortened.push('…');
		shortened
	} else {
		result
	}
}

// --- Constant expressions -------------------------------------------------

/// The value of an integer constant expression: literals, constants already
/// known, parentheses, casts to integer types, unary `-` and `+`, and the
/// binary `* / % + - << >> & ^ |`.
fn evaluate(tokens: &[Token], constants: &HashMap<String, i128>) -> Option<i128> {
	let mut position = 0;
	let value = expression(tokens, &mut position, constants, 0, 0)?;
	(position == tokens.len()).then_some(value)
}

fn expression(
	tokens: &[Token],
	position: &mut usize,
	constants: &HashMap<String, i128>,
	minimum: u8,
	depth: usize,
) -> Option<i128> {
	if depth > MAX_DEPTH * 2 {
		return None;
	}
	let mut left = unary(tokens, position, constants, depth + 1)?;
	while let Some(operator) = tokens.get(*position) {
		let precedence = match operator.text.as_str() {
			"|" => 1,
			"^" => 2,
			"&" => 3,
			"<<" | ">>" => 4,
			"+" | "-" => 5,
			"*" | "/" | "%" => 6,
			_ => break,
		};
		if precedence < minimum {
			break;
		}
		*position += 1;
		let right = expression(tokens, position, constants, precedence + 1, depth + 1)?;
		left = match operator.text.as_str() {
			"|" => left | right,
			"^" => left ^ right,
			"&" => left & right,
			"<<" => left.checked_mul(
				1i128.checked_shl(u32::try_from(right).ok().filter(|&shift| shift < 126)?)?,
			)?,
			">>" => left.checked_shr(u32::try_from(right).ok().filter(|&shift| shift < 127)?)?,
			"+" => left.checked_add(right)?,
			"-" => left.checked_sub(right)?,
			"*" => left.checked_mul(right)?,
			"/" => left.checked_div(right)?,
			_ => left.checked_rem(right)?,
		};
	}
	Some(left)
}

fn unary(
	tokens: &[Token],
	position: &mut usize,
	constants: &HashMap<String, i128>,
	depth: usize,
) -> Option<i128> {
	if depth > MAX_DEPTH * 2 {
		return None;
	}
	let token = tokens.get(*position)?;
	*position += 1;
	match token.text.as_str() {
		"-" => unary(tokens, position, constants, depth + 1)?.checked_neg(),
		"+" => unary(tokens, position, constants, depth + 1),
		"(" => {
			let mut end = *position;
			while tokens
				.get(end)
				.is_some_and(|token| is_integer_type_word(&token.text))
			{
				end += 1;
			}
			if end > *position && tokens.get(end).is_some_and(|token| token.text == ")") {
				// a cast to an integer type
				*position = end + 1;
				return unary(tokens, position, constants, depth + 1);
			}
			let value = expression(tokens, position, constants, 0, depth + 1)?;
			if tokens.get(*position)?.text != ")" {
				return None;
			}
			*position += 1;
			Some(value)
		}
		text if text.starts_with(|character: char| character.is_ascii_digit()) => {
			parse_integer(text)
		}
		text if text.len() == 3 && text.starts_with('\'') && text.ends_with('\'') => {
			let character = text.as_bytes()[1];
			(character != b'\\' && character.is_ascii()).then_some(i128::from(character))
		}
		text if is_identifier(text) => constants.get(text).copied(),
		_ => None,
	}
}

fn is_integer_type_word(text: &str) -> bool {
	matches!(
		text,
		"int" | "unsigned" | "signed" | "long" | "short" | "char"
	) || STANDARD_TYPES
		.iter()
		.any(|&(name, fa)| name == text && fa != "Boolean")
}

/// An integer literal: decimal, hexadecimal, octal or binary, with its `U`
/// and `L` suffixes.
fn parse_integer(text: &str) -> Option<i128> {
	let digits = text.trim_end_matches(['u', 'U', 'l', 'L']);
	let (radix, digits) = if let Some(rest) = digits
		.strip_prefix("0x")
		.or_else(|| digits.strip_prefix("0X"))
	{
		(16, rest)
	} else if let Some(rest) = digits
		.strip_prefix("0b")
		.or_else(|| digits.strip_prefix("0B"))
	{
		(2, rest)
	} else if digits.len() > 1 && digits.starts_with('0') {
		(8, &digits[1..])
	} else {
		(10, digits)
	};
	if digits.is_empty() || !digits.chars().all(|character| character.is_digit(radix)) {
		return None;
	}
	i128::from_str_radix(digits, radix).ok()
}

// --- Declarations -----------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
	Struct,
	Union,
	Enum,
}

impl Kind {
	fn key(self, tag: &str) -> String {
		let word = match self {
			Kind::Struct => "struct",
			Kind::Union => "union",
			Kind::Enum => "enum",
		};
		format!("{word} {tag}")
	}
}

/// A C type.
#[derive(Clone, Debug, PartialEq)]
enum CType {
	/// A type with a fixed Fa spelling.
	Builtin(&'static str),
	Void,
	/// A type name: a typedef of the header, a standard type, or unknown.
	Named(String),
	/// `struct tag`, `union tag`, `enum tag`, without a body.
	Tag(Kind, String),
	/// A struct, union or enum with a body (an index in `Binder::records`).
	Record(usize),
	Pointer {
		target: Box<CType>,
		/// Whether the designated value is `const`.
		const_target: bool,
		nonnull: bool,
	},
	/// An array; its size is `None` when it is missing or not a constant.
	Array(Box<CType>, Option<i128>),
	Function {
		parameters: Vec<Parameter>,
		result: Box<CType>,
		variadic: bool,
	},
	/// A type Fa doesn't have, like `long double`.
	Unsupported(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
struct Parameter {
	name: Option<String>,
	ty: CType,
}

/// A struct, union or enum with a body.
struct Record {
	kind: Kind,
	tag: Option<String>,
	text: String,
	/// The typedefs naming it directly, like `Point` in
	/// `typedef struct { ... } Point;`.
	typedef_names: Vec<String>,
	fields: Vec<(String, CType)>,
	members: Vec<Constant>,
	/// Why its fields can't be translated.
	problem: Option<String>,
}

/// An enum member or a `#define`: a C name and its value, if it is an
/// integer constant.
struct Constant {
	name: String,
	value: Option<i128>,
	text: String,
}

/// The declaration specifiers: `typedef`, `static`, `const` and the type.
struct Specifiers {
	typedef: bool,
	is_static: bool,
	is_const: bool,
	base: CType,
}

/// A declaration of the header, in order.
enum Item {
	Record(usize),
	/// The first mention of `struct tag`: an opaque type, unless a record
	/// defines it.
	Opaque(String),
	Typedef {
		name: String,
		ty: CType,
		text: String,
	},
	Function {
		name: String,
		parameters: Vec<Parameter>,
		result: CType,
		variadic: bool,
		text: String,
	},
	Unsupported {
		reason: String,
		text: String,
	},
}

/// Reads the declarations of a header.
struct Binder<'a> {
	code: &'a str,
	records: Vec<Record>,
	items: Vec<Item>,
	/// The integer constants: `#define`s and enum members, by C name.
	constants: HashMap<String, i128>,
	/// The typedefs, by C name.
	typedefs: HashMap<String, CType>,
	/// The typedefs naming a `struct tag` directly, by key.
	tag_typedefs: HashMap<String, Vec<String>>,
	/// The keys of the `struct tag`s mentioned.
	mentioned: HashSet<String>,
	/// The C names of the functions declared.
	functions: HashSet<String>,
}

impl<'a> Binder<'a> {
	fn new(code: &'a str) -> Self {
		Self {
			code,
			records: Vec::new(),
			items: Vec::new(),
			constants: HashMap::new(),
			typedefs: HashMap::new(),
			tag_typedefs: HashMap::new(),
			mentioned: HashSet::new(),
			functions: HashSet::new(),
		}
	}

	/// The source text of tokens, on one line.
	fn text(&self, tokens: &[Token]) -> String {
		match (tokens.first(), tokens.last()) {
			(Some(first), Some(last)) => self
				.code
				.get(first.start..last.end)
				.map(one_line)
				.unwrap_or_default(),
			_ => String::new(),
		}
	}

	/// Evaluates the `#define`s, in order.
	fn define_constants(&mut self, defines: &[Define]) -> Vec<Constant> {
		let mut constants = Vec::new();
		for define in defines {
			let value = evaluate(&tokenize(&define.value), &self.constants);
			if let Some(value) = value {
				self.constants.insert(define.name.clone(), value);
			}
			constants.push(Constant {
				name: define.name.clone(),
				value,
				text: define.text.clone(),
			});
		}
		constants
	}

	fn unsupported(&mut self, reason: &str, text: String) {
		self.items.push(Item::Unsupported {
			reason: reason.to_string(),
			text,
		});
	}

	/// Whether a name is a type: a typedef of the header, or a standard type.
	fn is_known_type(&self, name: &str) -> bool {
		self.typedefs.contains_key(name)
			|| STANDARD_TYPES.iter().any(|&(standard, _)| standard == name)
	}

	/// Notes the mention of `struct tag`: its first mention declares it.
	fn mention(&mut self, kind: Kind, tag: &str) {
		if kind == Kind::Struct && self.mentioned.insert(kind.key(tag)) {
			self.items.push(Item::Opaque(tag.to_string()));
		}
	}

	fn declaration(&mut self, declaration: Declaration<'_>) {
		let tokens = declaration.tokens;
		let text = self.text(tokens);
		if !declaration.complete {
			// the end of a file, often a lone macro like `__END_DECLS`
			if !tokens.iter().all(|token| is_identifier(&token.text)) {
				self.unsupported("not ended by `;`", text);
			}
			return;
		}
		let mut position = 0;
		let Some(specifiers) = self.specifiers(tokens, &mut position, 0) else {
			self.unsupported("not a declaration", text);
			return;
		};
		if let CType::Record(id) = specifiers.base {
			self.items.push(Item::Record(id));
		}
		if position == tokens.len() {
			// `struct tag;`, `struct tag { ... };`, `enum { ... };`
			return;
		}
		loop {
			let Some((name, ty)) = self.declarator(
				tokens,
				&mut position,
				specifiers.base.clone(),
				specifiers.is_const,
				0,
			) else {
				self.unsupported("not a declaration", text);
				return;
			};
			let last = match tokens.get(position) {
				None => true,
				Some(token) if token.text == "," => {
					position += 1;
					false
				}
				Some(_) => {
					self.unsupported("not a declaration", text);
					return;
				}
			};
			match name {
				None => self.unsupported("not a declaration", text.clone()),
				// a static declaration can't be linked
				Some(_) if specifiers.is_static => {}
				Some(name) if specifiers.typedef => self.typedef(name, ty, text.clone()),
				Some(name) => match ty {
					CType::Function {
						parameters,
						result,
						variadic,
					} => {
						if self.functions.insert(name.clone()) {
							self.items.push(Item::Function {
								name,
								parameters,
								result: *result,
								variadic,
								text: text.clone(),
							});
						}
					}
					_ => self.unsupported("variable", text.clone()),
				},
			}
			if last {
				break;
			}
		}
	}

	fn typedef(&mut self, name: String, ty: CType, text: String) {
		match &ty {
			CType::Record(id) => self.records[*id].typedef_names.push(name.clone()),
			CType::Tag(kind, tag) => {
				self.tag_typedefs
					.entry(kind.key(tag))
					.or_default()
					.push(name.clone());
				self.items.push(Item::Typedef {
					name: name.clone(),
					ty: ty.clone(),
					text,
				});
			}
			_ => self.items.push(Item::Typedef {
				name: name.clone(),
				ty: ty.clone(),
				text,
			}),
		}
		self.typedefs.insert(name, ty);
	}

	/// Reads the declaration specifiers: storage classes, qualifiers and the
	/// base type; `None` if there is no type.
	fn specifiers(
		&mut self,
		tokens: &[Token],
		position: &mut usize,
		depth: usize,
	) -> Option<Specifiers> {
		if depth > MAX_DEPTH {
			return None;
		}
		let mut typedef = false;
		let mut is_static = false;
		let mut is_const = false;
		let mut words: Vec<&str> = Vec::new();
		let mut long_count = 0;
		let mut names: Vec<String> = Vec::new();
		let mut tagged: Option<CType> = None;
		while let Some(token) = tokens.get(*position) {
			match token.text.as_str() {
				"typedef" => typedef = true,
				"static" => is_static = true,
				"const" => is_const = true,
				"_Nonnull" => {}
				"long" => {
					long_count += 1;
					words.push("long");
				}
				word @ ("unsigned" | "signed" | "short" | "int" | "char" | "float" | "double"
				| "void" | "_Bool" | "_Complex" | "__int128" | "_Float16" | "__fp16") => words.push(word),
				word @ ("struct" | "union" | "enum") => {
					let kind = match word {
						"struct" => Kind::Struct,
						"union" => Kind::Union,
						_ => Kind::Enum,
					};
					let keyword = *position;
					*position += 1;
					let tag = tokens
						.get(*position)
						.filter(|token| is_identifier(&token.text))
						.map(|token| token.text.clone());
					if tag.is_some() {
						*position += 1;
					}
					if tokens.get(*position).is_some_and(|token| token.text == "{") {
						let close = matching(tokens, *position)?;
						let text = self.text(&tokens[keyword..=close]);
						let body = &tokens[*position + 1..close];
						let id = self.record(kind, tag, body, text, depth + 1);
						*position = close + 1;
						tagged = Some(CType::Record(id));
					} else {
						let tag = tag?;
						self.mention(kind, &tag);
						tagged = Some(CType::Tag(kind, tag));
					}
					continue;
				}
				text if is_identifier(text) => {
					let next = tokens.get(*position + 1).map(|token| token.text.as_str());
					let has_type = !words.is_empty() || tagged.is_some() || !names.is_empty();
					let declarator_follows =
						matches!(next, None | Some("(" | "[" | ";" | "," | ")" | ":" | "="));
					if has_type && declarator_follows {
						break;
					}
					names.push(text.to_string());
				}
				_ => break,
			}
			*position += 1;
		}
		let base = if !words.is_empty() {
			builtin(&words, long_count)
		} else if let Some(tagged) = tagged {
			tagged
		} else {
			// several names: the unknown ones are macros, like `SQLITE_API`
			let name = names
				.iter()
				.find(|name| self.is_known_type(name))
				.or_else(|| names.last())?;
			CType::Named(name.clone())
		};
		Some(Specifiers {
			typedef,
			is_static,
			is_const,
			base,
		})
	}

	/// Reads a declarator applied to `base`: its name, if any, and its type.
	fn declarator(
		&mut self,
		tokens: &[Token],
		position: &mut usize,
		base: CType,
		base_const: bool,
		depth: usize,
	) -> Option<(Option<String>, CType)> {
		if depth > MAX_DEPTH {
			return None;
		}
		let mut ty = base;
		let mut target_const = base_const;
		let mut levels = 0;
		while let Some(token) = tokens.get(*position) {
			match token.text.as_str() {
				"*" => {
					levels += 1;
					if levels > MAX_LEVELS {
						return None;
					}
					ty = CType::Pointer {
						target: Box::new(ty),
						const_target: target_const,
						nonnull: false,
					};
					target_const = false;
				}
				"const" => target_const = true,
				"_Nonnull" => {
					if let CType::Pointer { nonnull, .. } = &mut ty {
						*nonnull = true;
					}
				}
				_ => break,
			}
			*position += 1;
		}
		let mut name = None;
		let mut inner = None;
		match tokens.get(*position) {
			Some(token)
				if token.text == "("
					&& tokens
						.get(*position + 1)
						.is_some_and(|next| next.text == "*") =>
			{
				let close = matching(tokens, *position)?;
				inner = Some((*position + 1, close));
				*position = close + 1;
			}
			Some(token) if is_identifier(&token.text) => {
				name = Some(token.text.clone());
				*position += 1;
			}
			_ => {}
		}
		enum Suffix {
			Array(Option<i128>),
			Function(Vec<Parameter>, bool),
		}
		let mut suffixes = Vec::new();
		while let Some(token) = tokens.get(*position) {
			let open = *position;
			match token.text.as_str() {
				"[" => {
					let close = matching(tokens, open)?;
					let size = &tokens[open + 1..close];
					suffixes.push(Suffix::Array(evaluate(size, &self.constants)));
					*position = close + 1;
				}
				"(" => {
					let close = matching(tokens, open)?;
					let (parameters, variadic) =
						self.parameters(&tokens[open + 1..close], depth + 1)?;
					suffixes.push(Suffix::Function(parameters, variadic));
					*position = close + 1;
				}
				_ => break,
			}
			if suffixes.len() > MAX_LEVELS {
				return None;
			}
		}
		for suffix in suffixes.into_iter().rev() {
			ty = match suffix {
				Suffix::Array(size) => CType::Array(Box::new(ty), size),
				Suffix::Function(parameters, variadic) => CType::Function {
					parameters,
					result: Box::new(ty),
					variadic,
				},
			};
		}
		if let Some((start, end)) = inner {
			let mut inner_position = start;
			let (inner_name, inner_ty) =
				self.declarator(&tokens[..end], &mut inner_position, ty, false, depth + 1)?;
			if inner_position != end {
				return None;
			}
			name = inner_name;
			ty = inner_ty;
		}
		Some((name, ty))
	}

	/// Reads the parameters of a function: the parameters, and whether it is
	/// variadic.
	fn parameters(&mut self, tokens: &[Token], depth: usize) -> Option<(Vec<Parameter>, bool)> {
		if tokens.is_empty() || (tokens.len() == 1 && tokens[0].text == "void") {
			return Some((Vec::new(), false));
		}
		let mut parameters = Vec::new();
		let mut variadic = false;
		for part in split(tokens, ",") {
			if variadic || part.is_empty() {
				return None;
			}
			if part.len() == 1 && part[0].text == "..." {
				variadic = true;
				continue;
			}
			let mut position = 0;
			let specifiers = self.specifiers(part, &mut position, depth)?;
			if specifiers.typedef || specifiers.is_static {
				return None;
			}
			let (name, ty) = self.declarator(
				part,
				&mut position,
				specifiers.base,
				specifiers.is_const,
				depth + 1,
			)?;
			if position != part.len() {
				return None;
			}
			// an array parameter is a pointer
			let ty = match ty {
				CType::Array(element, _) => CType::Pointer {
					target: element,
					const_target: specifiers.is_const,
					nonnull: false,
				},
				other => other,
			};
			parameters.push(Parameter { name, ty });
		}
		Some((parameters, variadic))
	}

	/// Reads the body of a struct, union or enum.
	fn record(
		&mut self,
		kind: Kind,
		tag: Option<String>,
		body: &[Token],
		text: String,
		depth: usize,
	) -> usize {
		let mut record = Record {
			kind,
			tag,
			text,
			typedef_names: Vec::new(),
			fields: Vec::new(),
			members: Vec::new(),
			problem: None,
		};
		if depth > MAX_DEPTH {
			record.problem = Some("too deeply nested".to_string());
		} else if kind == Kind::Enum {
			record.members = self.members(body);
		} else {
			match self.fields(body, depth) {
				Ok(fields) => record.fields = fields,
				Err(problem) => record.problem = Some(problem),
			}
		}
		self.records.push(record);
		self.records.len() - 1
	}

	fn fields(&mut self, body: &[Token], depth: usize) -> Result<Vec<(String, CType)>, String> {
		let mut fields = Vec::new();
		for part in split(body, ";") {
			if part.is_empty() {
				continue;
			}
			if contains_at_top(part, ":") {
				return Err(format!("bit field: {}", self.text(part)));
			}
			let mut position = 0;
			let Some(specifiers) = self.specifiers(part, &mut position, depth) else {
				return Err(format!("field: {}", self.text(part)));
			};
			if let CType::Record(id) = specifiers.base
				&& self.records[id].tag.is_some()
			{
				self.items.push(Item::Record(id));
			}
			if position == part.len() {
				return Err(format!("anonymous member: {}", self.text(part)));
			}
			loop {
				let Some((Some(name), ty)) = self.declarator(
					part,
					&mut position,
					specifiers.base.clone(),
					specifiers.is_const,
					depth,
				) else {
					return Err(format!("field: {}", self.text(part)));
				};
				fields.push((name, ty));
				match part.get(position) {
					None => break,
					Some(token) if token.text == "," => position += 1,
					Some(_) => return Err(format!("field: {}", self.text(part))),
				}
			}
		}
		Ok(fields)
	}

	fn members(&mut self, body: &[Token]) -> Vec<Constant> {
		let mut members = Vec::new();
		let mut next = Some(0i128);
		for part in split(body, ",") {
			let Some(first) = part.first() else {
				continue;
			};
			if !is_identifier(&first.text) {
				next = None;
				continue;
			}
			let value = match part.get(1) {
				None => next,
				Some(token) if token.text == "=" => evaluate(&part[2..], &self.constants),
				Some(_) => None,
			};
			if let Some(value) = value {
				self.constants.insert(first.text.clone(), value);
			}
			next = value.and_then(|value| value.checked_add(1));
			members.push(Constant {
				name: first.text.clone(),
				value,
				text: self.text(part),
			});
		}
		members
	}
}

/// The type of C keywords, like `unsigned long`.
fn builtin(words: &[&str], long_count: usize) -> CType {
	let has = |word: &str| words.contains(&word);
	let unsigned = has("unsigned");
	if has("_Complex") || has("__int128") || has("_Float16") || has("__fp16") {
		return CType::Unsupported("a type Fa doesn't have");
	}
	if has("void") {
		CType::Void
	} else if has("_Bool") {
		CType::Builtin("Boolean")
	} else if has("float") {
		CType::Builtin("Float32")
	} else if has("double") {
		if long_count > 0 {
			CType::Unsupported("`long double`")
		} else {
			CType::Builtin("Float64")
		}
	} else if has("char") {
		CType::Builtin(if unsigned {
			"c.UnsignedChar"
		} else if has("signed") {
			"c.SignedChar"
		} else {
			"c.Char"
		})
	} else if has("short") {
		CType::Builtin(if unsigned {
			"c.UnsignedShort"
		} else {
			"c.Short"
		})
	} else if long_count >= 2 {
		CType::Builtin(if unsigned {
			"c.UnsignedLongLong"
		} else {
			"c.LongLong"
		})
	} else if long_count == 1 {
		CType::Builtin(if unsigned { "c.UnsignedLong" } else { "c.Long" })
	} else {
		CType::Builtin(if unsigned { "c.UnsignedInt" } else { "c.Int" })
	}
}

// --- Names ------------------------------------------------------------------

/// A C name in camelCase, for a Fa value: `open_v2` is `openV2`, `SQLITE_OK`
/// is `sqliteOk`.
fn camel_case(name: &str) -> String {
	let mut result = String::new();
	for word in name.split('_').filter(|word| !word.is_empty()) {
		let word = if word.chars().any(|character| character.is_ascii_lowercase()) {
			word.to_string()
		} else {
			word.to_ascii_lowercase()
		};
		let mut characters = word.chars();
		let Some(first) = characters.next() else {
			continue;
		};
		if result.is_empty() {
			result.push(first.to_ascii_lowercase());
		} else {
			result.push(first.to_ascii_uppercase());
		}
		result.extend(characters);
	}
	if result.is_empty() {
		result.push_str("value");
	} else if result.starts_with(|character: char| character.is_ascii_digit()) {
		result.insert_str(0, "value");
	}
	if RESERVED_VALUES.contains(&result.as_str()) {
		result.push_str("Value");
	}
	result
}

/// A C name in PascalCase, for a Fa type, without a `_t` suffix: `point_t`
/// is `Point`, `sqlite3` is `Sqlite3`.
fn pascal_case(name: &str) -> String {
	let base = name
		.strip_suffix("_t")
		.filter(|base| {
			base.chars()
				.any(|character| character.is_ascii_alphanumeric())
		})
		.unwrap_or(name);
	let mut result = String::new();
	for word in base.split('_').filter(|word| !word.is_empty()) {
		let mut characters = word.chars();
		if let Some(first) = characters.next() {
			result.push(first.to_ascii_uppercase());
			result.extend(characters);
		}
	}
	if result.is_empty() {
		result.push_str("Type");
	} else if result.starts_with(|character: char| character.is_ascii_digit()) {
		result.insert(0, 'T');
	}
	if RESERVED_TYPES.contains(&result.as_str()) {
		result.push_str("Type");
	}
	result
}

/// The prefix ending with `_` that every name of a group of two or more
/// shares, if what follows it starts with a letter in every name.
fn common_prefix(names: &[&str]) -> Option<String> {
	let (first, rest) = names.split_first()?;
	if rest.is_empty() {
		return None;
	}
	let mut length = first.len();
	for name in rest {
		let shared = first
			.bytes()
			.zip(name.bytes())
			.take_while(|(left, right)| left == right)
			.count();
		length = length.min(shared);
	}
	let cut = first.get(..length)?.rfind('_')? + 1;
	let all_letters = names.iter().all(|name| {
		name.get(cut..)
			.and_then(|rest| rest.chars().next())
			.is_some_and(|character| character.is_ascii_alphabetic())
	});
	all_letters.then(|| first[..cut].to_string())
}

// --- Writing ----------------------------------------------------------------

/// A group of lines of the namespace.
struct Block {
	kind: &'static str,
	lines: Vec<String>,
}

impl Block {
	fn line(kind: &'static str, line: String) -> Self {
		Self {
			kind,
			lines: vec![line],
		}
	}

	fn comment(reason: &str, text: &str) -> Self {
		Self::line("comment", format!("-- not supported ({reason}): {text}"))
	}

	/// A comment kept with the functions around it.
	fn function_comment(reason: &str, text: &str) -> Self {
		Self {
			kind: "function",
			..Self::comment(reason, text)
		}
	}
}

/// Writes the foreign file of the declarations read.
struct Writer<'a> {
	binder: &'a Binder<'a>,
	/// The Fa names of the `struct tag`s and `enum tag`s, by key.
	tag_names: HashMap<String, String>,
	/// The Fa names of the typedefs, by C name.
	typedef_names: HashMap<String, String>,
	/// The Fa names of the structs and enums.
	record_names: Vec<Option<String>>,
	/// The record defining a key, the first one.
	definitions: HashMap<String, usize>,
	/// The fields of the structs, written, or why they can't be.
	struct_fields: Vec<Option<Result<Vec<String>, String>>>,
	types: HashSet<String>,
	values: HashSet<String>,
	emitted_opaque: HashSet<String>,
	emitted_typedefs: HashSet<String>,
	/// The names of the typedefs that aren't a `struct tag`: a tag without a
	/// typedef doesn't take them.
	alias_names: HashSet<String>,
	/// The prefix left out of the names of the functions.
	function_prefix: Option<String>,
}

impl<'a> Writer<'a> {
	fn new(binder: &'a Binder<'a>) -> Self {
		let function_names: Vec<&str> = binder
			.items
			.iter()
			.filter_map(|item| match item {
				Item::Function { name, .. } => Some(name.as_str()),
				_ => None,
			})
			.collect();
		Self {
			binder,
			tag_names: HashMap::new(),
			typedef_names: HashMap::new(),
			record_names: vec![None; binder.records.len()],
			definitions: HashMap::new(),
			struct_fields: Vec::new(),
			types: HashSet::new(),
			values: HashSet::new(),
			emitted_opaque: HashSet::new(),
			emitted_typedefs: HashSet::new(),
			alias_names: HashSet::new(),
			function_prefix: common_prefix(&function_names),
		}
	}

	fn unique_type(&mut self, name: String) -> String {
		let mut candidate = name.clone();
		let mut counter = 2;
		while self.types.contains(&candidate) {
			candidate = format!("{name}{counter}");
			counter += 1;
		}
		self.types.insert(candidate.clone());
		candidate
	}

	fn unique_value(&mut self, preferred: String, fallback: String) -> String {
		if !self.values.contains(&preferred) {
			self.values.insert(preferred.clone());
			return preferred;
		}
		let mut candidate = fallback.clone();
		let mut counter = 2;
		while self.values.contains(&candidate) {
			candidate = format!("{fallback}{counter}");
			counter += 1;
		}
		self.values.insert(candidate.clone());
		candidate
	}

	/// The Fa name of a key (`struct tag`), from the first typedef naming it.
	fn preferred_name(&self, key: &str, tag: &str, record: Option<&Record>) -> String {
		record
			.and_then(|record| record.typedef_names.first())
			.or_else(|| {
				self.binder
					.tag_typedefs
					.get(key)
					.and_then(|names| names.first())
			})
			.map(|name| pascal_case(name))
			.unwrap_or_else(|| {
				let name = pascal_case(tag);
				if self.alias_names.contains(&name) {
					format!("{name}Struct")
				} else {
					name
				}
			})
	}

	/// Names the types, in the order of the header.
	fn name_types(&mut self) {
		let binder = self.binder;
		for item in &binder.items {
			if let Item::Typedef { name, ty, .. } = item
				&& !matches!(ty, CType::Tag(..))
				&& !is_standard(name)
			{
				self.alias_names.insert(pascal_case(name));
			}
		}
		for item in &binder.items {
			match item {
				Item::Record(id) => {
					let record = &binder.records[*id];
					if record.kind == Kind::Union || self.record_names[*id].is_some() {
						continue;
					}
					let key = record.tag.as_ref().map(|tag| record.kind.key(tag));
					let name = match (&key, &record.tag) {
						(Some(key), Some(tag)) => {
							if let Some(name) = self.tag_names.get(key) {
								name.clone()
							} else {
								let name = self.preferred_name(key, tag, Some(record));
								let name = self.unique_type(name);
								self.tag_names.insert(key.clone(), name.clone());
								name
							}
						}
						_ => match record.typedef_names.first() {
							Some(typedef) => self.unique_type(pascal_case(typedef)),
							None => continue,
						},
					};
					if let Some(key) = key {
						self.definitions.entry(key).or_insert(*id);
					}
					for typedef in &record.typedef_names {
						self.typedef_names.insert(typedef.clone(), name.clone());
					}
					self.record_names[*id] = Some(name);
				}
				Item::Opaque(tag) => {
					let key = Kind::Struct.key(tag);
					if !self.tag_names.contains_key(&key) {
						let name = self.preferred_name(&key, tag, None);
						let name = self.unique_type(name);
						self.tag_names.insert(key, name);
					}
				}
				Item::Typedef { name, ty, .. } => {
					if is_standard(name) || self.typedef_names.contains_key(name) {
						continue;
					}
					if let CType::Tag(kind, tag) = ty
						&& let Some(target) = self.tag_names.get(&kind.key(tag))
					{
						self.typedef_names.insert(name.clone(), target.clone());
						continue;
					}
					let fa_name = self.unique_type(pascal_case(name));
					self.typedef_names.insert(name.clone(), fa_name);
				}
				_ => {}
			}
		}
	}

	/// Writes the fields of the structs, in the order of their definitions,
	/// so that a struct used by value is known before.
	fn write_struct_fields(&mut self) {
		let binder = self.binder;
		for record in &binder.records {
			let fields = if record.kind != Kind::Struct {
				None
			} else if let Some(problem) = &record.problem {
				Some(Err(problem.clone()))
			} else {
				let mut used = HashSet::new();
				let mut lines = Vec::new();
				let mut result = Ok(());
				for (name, ty) in &record.fields {
					if !self.is_complete(ty, 0) {
						result = Err(format!("the field `{name}` has an incomplete type"));
						break;
					}
					match self.render(ty) {
						Ok(rendered) => {
							let mut fa_name = camel_case(name);
							while !used.insert(fa_name.clone()) {
								fa_name.push('_');
							}
							lines.push(format!("{fa_name}: {rendered}"));
						}
						Err(reason) => {
							result = Err(format!("the field `{name}` is {reason}"));
							break;
						}
					}
				}
				Some(result.map(|()| lines))
			};
			self.struct_fields.push(fields);
		}
	}

	/// Whether a field type has a known layout: not an opaque struct.
	fn is_complete(&self, ty: &CType, depth: usize) -> bool {
		if depth > MAX_DEPTH {
			return false;
		}
		match ty {
			CType::Array(element, _) => self.is_complete(element, depth + 1),
			CType::Named(name) => self
				.binder
				.typedefs
				.get(name)
				.is_none_or(|ty| self.is_complete(ty, depth + 1)),
			CType::Tag(Kind::Struct, _) | CType::Record(_) => !self.is_opaque(ty, depth + 1),
			_ => true,
		}
	}

	/// Whether a type is a struct without fields known to Fa.
	fn is_opaque(&self, ty: &CType, depth: usize) -> bool {
		if depth > MAX_DEPTH {
			return false;
		}
		let fields = |id: usize| {
			self.struct_fields
				.get(id)
				.and_then(|fields| fields.as_ref())
		};
		match ty {
			CType::Tag(Kind::Struct, tag) => match self.definitions.get(&Kind::Struct.key(tag)) {
				Some(&id) => !matches!(fields(id), Some(Ok(_))),
				None => true,
			},
			CType::Record(id) => {
				self.binder.records[*id].kind == Kind::Struct && !matches!(fields(*id), Some(Ok(_)))
			}
			CType::Named(name) => self
				.binder
				.typedefs
				.get(name)
				.is_some_and(|ty| self.is_opaque(ty, depth + 1)),
			_ => false,
		}
	}

	/// Whether a parameter of this type is written by C: a pointer to data
	/// that isn't `const`, nor a function, nor an opaque struct.
	fn mutates(&self, ty: &CType, depth: usize) -> bool {
		if depth > MAX_DEPTH {
			return false;
		}
		match ty {
			CType::Named(name) => self
				.binder
				.typedefs
				.get(name)
				.is_some_and(|ty| self.mutates(ty, depth + 1)),
			CType::Pointer {
				target,
				const_target,
				..
			} => {
				!const_target
					&& !self.is_function(target, depth + 1)
					&& !self.is_opaque(target, depth + 1)
			}
			_ => false,
		}
	}

	fn is_function(&self, ty: &CType, depth: usize) -> bool {
		if depth > MAX_DEPTH {
			return false;
		}
		match ty {
			CType::Function { .. } => true,
			CType::Pointer { target, .. } => matches!(**target, CType::Function { .. }),
			CType::Named(name) => self
				.binder
				.typedefs
				.get(name)
				.is_some_and(|ty| self.is_function(ty, depth + 1)),
			_ => false,
		}
	}

	/// The Fa spelling of a type, or why it has none.
	fn render(&self, ty: &CType) -> Result<String, String> {
		match ty {
			CType::Builtin(name) => Ok((*name).to_string()),
			CType::Void => Ok("Void".to_string()),
			CType::Named(name) => Ok(self.named(name)),
			CType::Tag(Kind::Union, tag) => Err(format!("the union `{tag}`")),
			CType::Tag(kind, tag) => Ok(self
				.tag_names
				.get(&kind.key(tag))
				.cloned()
				.unwrap_or_else(|| {
					if *kind == Kind::Enum {
						"c.Int".to_string()
					} else {
						pascal_case(tag)
					}
				})),
			CType::Record(id) => match self.binder.records.get(*id) {
				Some(record) if record.kind == Kind::Union => Err("a union".to_string()),
				_ => self
					.record_names
					.get(*id)
					.cloned()
					.flatten()
					.ok_or_else(|| "an anonymous struct".to_string()),
			},
			CType::Pointer {
				target,
				const_target,
				nonnull,
			} => {
				if **target == CType::Builtin("c.Char") && *const_target {
					return Ok("CString".to_string());
				}
				if let CType::Function { .. } = **target {
					return self.render(target);
				}
				let inner = self.render(target)?;
				Ok(format!(
					"Pointer<{inner}>{}",
					if *nonnull { "" } else { "?" }
				))
			}
			CType::Array(element, Some(size)) if *size > 0 => {
				Ok(format!("CArray<{}, {size}>", self.render(element)?))
			}
			CType::Array(..) => Err("an array without a constant size".to_string()),
			CType::Function {
				parameters,
				result,
				variadic,
			} => {
				if *variadic {
					return Err("a variadic function".to_string());
				}
				let (parameters, _) = self.parameters(parameters)?;
				Ok(format!("({parameters}) => {}", self.result(result)?))
			}
			CType::Unsupported(reason) => Err((*reason).to_string()),
		}
	}

	/// The Fa name of a C type name.
	fn named(&self, name: &str) -> String {
		if let Some(&(_, fa)) = STANDARD_TYPES
			.iter()
			.find(|&&(standard, _)| standard == name)
		{
			return fa.to_string();
		}
		self.typedef_names.get(name).cloned().unwrap_or_else(|| {
			if RESERVED_TYPES.contains(&name) || RESERVED_VALUES.contains(&name) {
				format!("{name}Type")
			} else {
				name.to_string()
			}
		})
	}

	/// The result of a function.
	fn result(&self, ty: &CType) -> Result<String, String> {
		if self.is_function(ty, 0) {
			return Err("returns a function".to_string());
		}
		self.render(ty)
	}

	/// The parameters of a function, written, and their Fa names.
	fn parameters(&self, parameters: &[Parameter]) -> Result<(String, Vec<String>), String> {
		let mut names: Vec<String> = Vec::new();
		let mut written = Vec::new();
		for (index, parameter) in parameters.iter().enumerate() {
			if parameter.ty == CType::Void {
				return Err("a `void` parameter".to_string());
			}
			let mut name = parameter
				.name
				.as_deref()
				.map_or_else(|| format!("p{index}"), camel_case);
			if names.contains(&name) {
				name = format!("{name}{index}");
			}
			written.push(format!("{name}: {}", self.render(&parameter.ty)?));
			names.push(name);
		}
		Ok((written.join(", "), names))
	}

	fn write(mut self, shown: &str, defines: &[Constant]) -> String {
		self.name_types();
		self.write_struct_fields();
		let mut blocks = Vec::new();
		if !defines.is_empty() {
			let constants: Vec<&Constant> = defines.iter().collect();
			blocks.push(self.constants(&constants));
		}
		let binder = self.binder;
		for item in &binder.items {
			if let Some(block) = self.item(item) {
				blocks.push(block);
			}
		}
		let mut output = format!(
			"-- Generated by `fa bind {}`: edit it like any source file.\n",
			one_line(shown)
		);
		if let Some(prefix) = &self.function_prefix {
			output.push_str(&format!(
				"-- The prefix `{prefix}` of the C functions is left out of their names.\n"
			));
		}
		output.push_str("use c = std.c\n\nexport namespace = {\n");
		let mut previous: Option<&Block> = None;
		for block in &blocks {
			if let Some(previous) = previous
				&& (previous.kind != block.kind
					|| previous.lines.len() > 1
					|| block.lines.len() > 1)
			{
				output.push('\n');
			}
			for line in &block.lines {
				output.push('\t');
				output.push_str(line);
				output.push('\n');
			}
			previous = Some(block);
		}
		output.push_str("}\n");
		output
	}

	/// The static values of a group of constants, with their common prefix
	/// left out.
	fn constants(&mut self, constants: &[&Constant]) -> Block {
		let names: Vec<&str> = constants
			.iter()
			.filter(|constant| constant.value.is_some())
			.map(|constant| constant.name.as_str())
			.collect();
		let prefix = common_prefix(&names).unwrap_or_default();
		let mut lines = Vec::new();
		for constant in constants {
			match constant.value {
				Some(value) => {
					let short = constant
						.name
						.strip_prefix(prefix.as_str())
						.unwrap_or(&constant.name);
					let name = self.unique_value(camel_case(short), camel_case(&constant.name));
					lines.push(format!("static {name} = {value}"));
				}
				None => lines.push(format!(
					"-- not supported (not an integer constant): {}",
					constant.text
				)),
			}
		}
		Block {
			kind: "static",
			lines,
		}
	}

	fn item(&mut self, item: &Item) -> Option<Block> {
		let binder = self.binder;
		match item {
			Item::Record(id) => {
				let record = &binder.records[*id];
				match record.kind {
					Kind::Union => Some(Block::comment("union", &record.text)),
					Kind::Enum => {
						let mut lines = Vec::new();
						if let Some(name) = &self.record_names[*id] {
							lines.push(format!("type {name} = c.Int"));
						}
						let members: Vec<&Constant> = record.members.iter().collect();
						lines.extend(self.constants(&members).lines);
						if lines.is_empty() {
							return None;
						}
						Some(Block {
							kind: "enum",
							lines,
						})
					}
					Kind::Struct => {
						let Some(name) = self.record_names[*id].clone() else {
							return Some(Block::comment("anonymous struct", &record.text));
						};
						let key = record.tag.as_ref().map(|tag| Kind::Struct.key(tag));
						if key
							.as_ref()
							.is_some_and(|key| self.definitions.get(key) != Some(id))
						{
							return Some(Block::comment("defined twice", &record.text));
						}
						match self.struct_fields.get(*id).cloned().flatten() {
							Some(Ok(fields)) if fields.is_empty() => {
								Some(Block::line("type", format!("type {name} = CStruct {{ }}")))
							}
							Some(Ok(fields)) => {
								let mut lines = vec![format!("type {name} = CStruct {{")];
								lines.extend(fields.into_iter().map(|field| format!("\t{field}")));
								lines.push("}".to_string());
								Some(Block {
									kind: "struct",
									lines,
								})
							}
							Some(Err(reason)) => Some(Block::line(
								"type",
								format!(
									"type {name} = CStruct {{ }} -- not supported ({}): only usable behind a pointer",
									one_line(&reason)
								),
							)),
							None => None,
						}
					}
				}
			}
			Item::Opaque(tag) => {
				let key = Kind::Struct.key(tag);
				if self.definitions.contains_key(&key) || !self.emitted_opaque.insert(key.clone()) {
					return None;
				}
				let name = self.tag_names.get(&key)?;
				Some(Block::line(
					"type",
					format!(
						"type {name} = CStruct {{ }} -- an opaque C type, only used behind a pointer"
					),
				))
			}
			Item::Typedef { name, ty, text } => {
				if is_standard(name) {
					return None;
				}
				let fa_name = self.typedef_names.get(name)?.clone();
				if let CType::Tag(kind, tag) = ty
					&& self.tag_names.get(&kind.key(tag)) == Some(&fa_name)
				{
					return None;
				}
				if !self.emitted_typedefs.insert(name.clone()) {
					return Some(Block::comment("defined twice", text));
				}
				match self.render(ty) {
					Ok(rendered) if rendered == fa_name => None,
					Ok(rendered) => {
						Some(Block::line("type", format!("type {fa_name} = {rendered}")))
					}
					Err(reason) => Some(Block::comment(&reason, text)),
				}
			}
			Item::Function {
				name,
				parameters,
				result,
				variadic,
				text,
			} => {
				if *variadic {
					return Some(Block::function_comment("variadic", text));
				}
				let signature = self.result(result).and_then(|result| {
					let (written, names) = self.parameters(parameters)?;
					Ok((result, written, names))
				});
				let (result, written, names) = match signature {
					Ok(signature) => signature,
					Err(reason) => return Some(Block::function_comment(&reason, text)),
				};
				let mutated: Vec<&str> = parameters
					.iter()
					.zip(&names)
					.filter(|(parameter, _)| self.mutates(&parameter.ty, 0))
					.map(|(_, name)| name.as_str())
					.collect();
				let mutates = if mutated.is_empty() {
					String::new()
				} else {
					format!(" mutates {}", mutated.join(", "))
				};
				let short = self
					.function_prefix
					.as_deref()
					.and_then(|prefix| name.strip_prefix(prefix))
					.unwrap_or(name);
				let fa_name = self.unique_value(camel_case(short), camel_case(name));
				Some(Block::line(
					"function",
					format!(
						"function {fa_name}: ({written}) => {result}{mutates} = @extern(\"{name}\")"
					),
				))
			}
			Item::Unsupported { reason, text } => Some(Block::comment(reason, text)),
		}
	}
}

fn is_standard(name: &str) -> bool {
	STANDARD_TYPES.iter().any(|&(standard, _)| standard == name)
}

#[cfg(test)]
mod tests {
	use super::generate;

	/// Generates the bindings of a header, and checks that they parse.
	fn bind(header: &str) -> String {
		let output = generate(header, "test.h");
		let tree = fa_parser::parse(&output);
		assert!(
			tree.diagnostics.is_empty(),
			"the output doesn't parse: {:?}\n{output}",
			tree.diagnostics
		);
		output
	}

	fn assert_contains(output: &str, expected: &str) {
		assert!(
			output.contains(expected),
			"expected `{expected}` in:\n{output}"
		);
	}

	#[test]
	fn bind_sqlite_like_header() {
		let output = bind(
			r#"
#ifndef SQLITE3_H
#define SQLITE3_H
#include <stdarg.h>
#ifdef __cplusplus
extern "C" {
#endif
#define SQLITE_API
#define SQLITE_OK 0 /* Successful result */
#define SQLITE_ERROR 1
#define SQLITE_OPEN_READONLY 0x00000001UL
#define SQLITE_VERSION "3.45.0"
typedef struct sqlite3 sqlite3;
SQLITE_API int sqlite3_open(
  const char *filename,   /* Database filename (UTF-8) */
  sqlite3 **ppDb          /* OUT: SQLite db handle */
);
SQLITE_API int sqlite3_close(sqlite3*);
SQLITE_API const char *sqlite3_errmsg(sqlite3*);
SQLITE_API void sqlite3_free(void*);
SQLITE_API int sqlite3_exec(sqlite3*, const char *sql, int (*callback)(void*,int,char**,char**), void *, char **errmsg);
SQLITE_API char *sqlite3_mprintf(const char*,...);
static inline int sqlite3_helper(int x) { if (x) { return 1; } return 0; }
#ifdef __cplusplus
}
#endif
#endif
"#,
		);
		assert_contains(
			&output,
			"-- Generated by `fa bind test.h`: edit it like any source file.",
		);
		assert_contains(
			&output,
			"-- The prefix `sqlite3_` of the C functions is left out of their names.",
		);
		assert_contains(&output, "use c = std.c\n\nexport namespace = {\n");
		assert_contains(
			&output,
			"\tstatic ok = 0\n\tstatic error = 1\n\tstatic openReadonly = 1\n",
		);
		assert_contains(
			&output,
			"-- not supported (not an integer constant): #define SQLITE_VERSION \"3.45.0\"",
		);
		assert_contains(
			&output,
			"type Sqlite3 = CStruct { } -- an opaque C type, only used behind a pointer",
		);
		assert_contains(
			&output,
			"function open: (filename: CString, ppDb: Pointer<Pointer<Sqlite3>?>?) => c.Int mutates ppDb = @extern(\"sqlite3_open\")",
		);
		assert_contains(
			&output,
			"function close: (p0: Pointer<Sqlite3>?) => c.Int = @extern(\"sqlite3_close\")",
		);
		assert_contains(
			&output,
			"function errmsg: (p0: Pointer<Sqlite3>?) => CString = @extern(\"sqlite3_errmsg\")",
		);
		assert_contains(
			&output,
			"function free: (p0: Pointer<Void>?) => Void mutates p0 = @extern(\"sqlite3_free\")",
		);
		assert_contains(
			&output,
			"function exec: (p0: Pointer<Sqlite3>?, sql: CString, callback: (p0: Pointer<Void>?, p1: c.Int, p2: Pointer<Pointer<c.Char>?>?, p3: Pointer<Pointer<c.Char>?>?) => c.Int, p3: Pointer<Void>?, errmsg: Pointer<Pointer<c.Char>?>?) => c.Int mutates p3, errmsg = @extern(\"sqlite3_exec\")",
		);
		assert_contains(
			&output,
			"-- not supported (variadic): SQLITE_API char *sqlite3_mprintf(const char*,...)",
		);
		assert!(!output.contains("helper"), "{output}");
		assert!(!output.contains("SQLITE3_H"), "{output}");
	}

	#[test]
	fn bind_structs_with_arrays() {
		let output = bind(
			"#define NAME_LENGTH 32\n\
			struct point { int x; int y; };\n\
			typedef struct vertex_s { float position[3]; uint8_t color[4]; } vertex_t;\n\
			typedef struct { char name[NAME_LENGTH]; struct point origin; struct point *next; double matrix[2][2]; } shape_t;\n\
			struct flags { unsigned ready : 1; unsigned done : 1; };\n\
			union value { int i; float f; };\n\
			struct node;\n\
			void move_point(struct point *p, const struct point *by);\n\
			void reset(struct node *n);\n",
		);
		assert_contains(&output, "static nameLength = 32");
		assert_contains(
			&output,
			"\ttype Point = CStruct {\n\t\tx: c.Int\n\t\ty: c.Int\n\t}\n",
		);
		assert_contains(
			&output,
			"\ttype Vertex = CStruct {\n\t\tposition: CArray<Float32, 3>\n\t\tcolor: CArray<UInt8, 4>\n\t}\n",
		);
		assert_contains(&output, "\t\tname: CArray<c.Char, 32>\n");
		assert_contains(&output, "\t\torigin: Point\n");
		assert_contains(&output, "\t\tnext: Pointer<Point>?\n");
		assert_contains(&output, "\t\tmatrix: CArray<CArray<Float64, 2>, 2>\n");
		assert_contains(&output, "type Shape = CStruct {");
		assert_contains(
			&output,
			"type Flags = CStruct { } -- not supported (bit field: unsigned ready : 1)",
		);
		assert_contains(
			&output,
			"-- not supported (union): union value { int i; float f; }",
		);
		assert_contains(&output, "type Node = CStruct { } -- an opaque C type");
		assert_contains(
			&output,
			"function movePoint: (p: Pointer<Point>?, by: Pointer<Point>?) => Void mutates p = @extern(\"move_point\")",
		);
		assert_contains(
			&output,
			"function reset: (n: Pointer<Node>?) => Void = @extern(\"reset\")",
		);
	}

	#[test]
	fn bind_typedefs() {
		let output = bind(
			"typedef unsigned int id_t;\n\
			typedef long long big;\n\
			typedef unsigned char byte;\n\
			typedef signed char tiny;\n\
			typedef unsigned long long size64;\n\
			typedef uint32_t uint32_t;\n\
			typedef struct handle *handle_t;\n\
			typedef int (*compare_t)(const void *, const void *);\n\
			typedef int *_Nonnull counter;\n\
			typedef enum { RED, GREEN = 4, BLUE } color_t;\n\
			id_t next_id(handle_t handle, color_t color, compare_t compare, size_t size, bool flag);\n",
		);
		assert_contains(&output, "type Id = c.UnsignedInt");
		assert_contains(&output, "type Big = c.LongLong");
		assert_contains(&output, "type Byte = c.UnsignedChar");
		assert_contains(&output, "type Tiny = c.SignedChar");
		assert_contains(&output, "type Size64 = c.UnsignedLongLong");
		assert!(!output.contains("Uint32"), "{output}");
		assert_contains(
			&output,
			"type HandleStruct = CStruct { } -- an opaque C type",
		);
		assert_contains(&output, "type Handle = Pointer<HandleStruct>?");
		assert_contains(
			&output,
			"type Compare = (p0: Pointer<Void>?, p1: Pointer<Void>?) => c.Int",
		);
		assert_contains(&output, "type Counter = Pointer<c.Int>");
		assert_contains(
			&output,
			"\ttype Color = c.Int\n\tstatic red = 0\n\tstatic green = 4\n\tstatic blue = 5\n",
		);
		assert_contains(
			&output,
			"function nextId: (handle: Handle, color: Color, compare: Compare, size: c.SizeT, flag: Boolean) => Id = @extern(\"next_id\")",
		);
	}

	#[test]
	fn bind_enums_and_defines() {
		let output = bind(
			"#define FLAG_A (1 << 0)\n\
			#define FLAG_B (1 << 1)\n\
			#define FLAG_ALL (FLAG_A | FLAG_B)\n\
			#define FLAG_OFF -1\n\
			#define FLAG_MAX(a, b) ((a) > (b) ? (a) : (b))\n\
			enum level { LEVEL_LOW = -2, LEVEL_MID, LEVEL_HIGH = FLAG_ALL + 10, LEVEL_FOR };\n\
			enum { SINGLE };\n",
		);
		assert_contains(
			&output,
			"static a = 1\n\tstatic b = 2\n\tstatic all = 3\n\tstatic off = -1\n",
		);
		assert!(!output.contains("FLAG_MAX"), "{output}");
		assert_contains(
			&output,
			"type Level = c.Int\n\tstatic low = -2\n\tstatic mid = -1\n\tstatic high = 13\n\tstatic forValue = 14\n",
		);
		assert_contains(&output, "static single = 0");
	}

	#[test]
	fn bind_qsort_with_a_function_pointer() {
		let output = bind(
			"extern void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));\n\
			void (*signal(int sig, void (*func)(int)))(int);\n\
			int abs(int);\n",
		);
		assert_contains(
			&output,
			"function qsort: (base: Pointer<Void>?, nmemb: c.SizeT, size: c.SizeT, compar: (p0: Pointer<Void>?, p1: Pointer<Void>?) => c.Int) => Void mutates base = @extern(\"qsort\")",
		);
		assert_contains(
			&output,
			"-- not supported (returns a function): void (*signal(int sig, void (*func)(int)))(int)",
		);
		assert_contains(
			&output,
			"function abs: (p0: c.Int) => c.Int = @extern(\"abs\")",
		);
	}

	#[test]
	fn bind_variadic_printf() {
		let output = bind(
			"#include <stdio.h>\n\
			int printf(const char *restrict format, ...);\n\
			int puts(const char *s);\n\
			extern int errno_value;\n",
		);
		assert_contains(
			&output,
			"-- not supported (variadic): int printf(const char *restrict format, ...)",
		);
		assert_contains(
			&output,
			"function puts: (s: CString) => c.Int = @extern(\"puts\")",
		);
		assert_contains(&output, "-- not supported (variable): int errno_value");
	}

	#[test]
	fn bind_names_and_attributes() {
		let output = bind(
			"#if 0\nint hidden(void);\n#else\nint shown(int type, int for, char *in);\n#endif\n\
			__attribute__((visibility(\"default\"))) int shown_too(void) __attribute__((nonnull(1)));\n\
			int Error_count(int * _Nonnull count);\n\
			struct Error { int code; };\n",
		);
		assert!(!output.contains("hidden"), "{output}");
		assert_contains(
			&output,
			"function shown: (typeValue: c.Int, forValue: c.Int, inValue: Pointer<c.Char>?) => c.Int mutates inValue = @extern(\"shown\")",
		);
		assert_contains(
			&output,
			"function shownToo: () => c.Int = @extern(\"shown_too\")",
		);
		assert_contains(
			&output,
			"function errorCount: (count: Pointer<c.Int>) => c.Int mutates count = @extern(\"Error_count\")",
		);
		assert_contains(&output, "type ErrorType = CStruct {");
	}

	#[test]
	fn bind_garbage_without_panicking() {
		let inputs = [
			"",
			"}}}{{{",
			"int f(;",
			"struct { int a; ",
			"typedef",
			"#define\n#define 1\n#define X (\n#if 0\n",
			"int (*(*(*(*f)(int))(int))(int))(int);",
			"/* unclosed comment",
			"\"unclosed string",
			"int a[1 << 200];",
			"enum { A = 1 / 0, B, C = 'x', D = '\\n' };",
			"int f(int a, ...,);",
			"struct s { struct { int x; }; union { int y; } u; } ;",
			"é; int é(int);\u{0}\u{7}\\\n",
			"typedef struct a a; typedef struct a b; struct a { a *self; b *other; }; struct a { int z; };",
			"int ((((((((((((((((((((((((((((((((((((((((x))))))))))))))))))))))))))))))))))))))));",
			"SQLITE_API SQLITE_DEPRECATED",
			"void f(void (*)(void (*)(void (*)(int))));",
			"long double x(long double); _Complex float y(void);",
		];
		for input in inputs {
			bind(input);
		}
		let deep = format!("int {}x{};", "(*".repeat(5000), ")".repeat(5000));
		bind(&deep);
		let stars = format!("int {}x;", "*".repeat(5000));
		bind(&stars);
		let arrays = format!("struct s {{ int x{}; }};", "[1]".repeat(5000));
		bind(&arrays);
		let structs = format!("{}{}", "struct a { ".repeat(3000), "} b; ".repeat(3000));
		bind(&structs);
		let expression = format!("#define X {}1{}\n", "(".repeat(5000), ")".repeat(5000));
		bind(&expression);
		let mut garbage = String::new();
		let pieces = [
			"int", "(", ")", "*", "{", "}", ";", ",", "struct", "x", "[", "]", "...", ":", "=",
			"1", "typedef", "enum",
		];
		let mut state: u64 = 7;
		for _ in 0..4000 {
			state = state
				.wrapping_mul(6364136223846793005)
				.wrapping_add(1442695040888963407);
			garbage.push_str(pieces[(state >> 33) as usize % pieces.len()]);
			garbage.push(' ');
		}
		bind(&garbage);
	}
}
