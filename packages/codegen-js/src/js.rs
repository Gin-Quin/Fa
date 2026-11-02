//! JavaScript building blocks: expressions with their precedence, literals
//! and the indented writer of statements.

use std::fmt::Write as _;

/// The precedence of a JavaScript expression (higher binds tighter), after
/// the operator precedence table of ECMAScript.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Prec {
	/// `a = b`, arrow functions
	Assign = 2,
	/// `c ? a : b`
	Conditional = 3,
	/// `a ?? b` (never mixed with `||` or `&&` without parentheses)
	Nullish = 4,
	/// `a || b`
	Or = 5,
	/// `a && b`
	And = 6,
	/// `a | b`
	BitOr = 7,
	/// `a ^ b`
	BitXor = 8,
	/// `a & b`
	BitAnd = 9,
	/// `a === b`
	Equality = 10,
	/// `a < b`
	Relational = 11,
	/// `a << b`, `a >> b`, `a >>> b`
	Shift = 12,
	/// `a + b`
	Additive = 13,
	/// `a * b`
	Multiplicative = 14,
	/// `a ** b` (right-associative; its left operand can't be a unary expression)
	Exponent = 15,
	/// `-a`, `!a`, `typeof a`
	Unary = 16,
	/// `f(a)`, `a.b`, literals, names, parenthesized expressions
	Primary = 20,
}

/// A JavaScript expression being built: its text, its precedence and what
/// the statement lifting needs to know about it.
#[derive(Clone, Debug)]
pub struct Expr {
	/// The code.
	pub text: String,
	/// How tightly it binds.
	pub prec: Prec,
	/// Evaluating it again later gives the same value and has no effect: a
	/// literal, a temporary, a name that is never reassigned. An unstable
	/// operand evaluated before a lifted construct is stored in a temporary.
	pub stable: bool,
	/// Evaluating it has no effect at all (a dropped pure value is not emitted).
	pub pure: bool,
}

impl Expr {
	/// An expression of some precedence, unstable and with effects.
	pub fn new(text: impl Into<String>, prec: Prec) -> Self {
		Self {
			text: text.into(),
			prec,
			stable: false,
			pure: false,
		}
	}

	/// A primary expression (call, member, parenthesized…).
	pub fn primary(text: impl Into<String>) -> Self {
		Self::new(text, Prec::Primary)
	}

	/// A literal: stable and pure.
	pub fn constant(text: impl Into<String>, prec: Prec) -> Self {
		Self {
			text: text.into(),
			prec,
			stable: true,
			pure: true,
		}
	}

	/// A read of a name: pure, and stable when the name is never reassigned.
	pub fn name(text: impl Into<String>, stable: bool) -> Self {
		Self {
			text: text.into(),
			prec: Prec::Primary,
			stable,
			pure: true,
		}
	}

	/// Sets whether it is pure and stable.
	pub fn with_purity(mut self, pure: bool, stable: bool) -> Self {
		self.pure = pure;
		self.stable = stable;
		self
	}

	/// The text, parenthesized when it binds less tightly than `min`.
	pub fn at(&self, min: Prec) -> String {
		if self.prec < min {
			format!("({})", self.text)
		} else {
			self.text.clone()
		}
	}

	/// The text, parenthesized unless it is a primary expression: the target
	/// of a member access or of a call.
	pub fn primary_text(&self) -> String {
		self.at(Prec::Primary)
	}

	/// Whether it is a literal `undefined`.
	pub fn is_undefined(&self) -> bool {
		self.text == "undefined"
	}
}

/// A binary operation `left op right`.
pub fn binary(left: &Expr, op: &str, right: &Expr, prec: Prec) -> Expr {
	let left_text = match prec {
		// `-a ** b` is a syntax error: the left operand must bind tighter than unary
		Prec::Exponent => left.at(Prec::Primary),
		Prec::Nullish => left.at(Prec::Equality),
		_ => left.at(prec),
	};
	let right_text = match prec {
		Prec::Exponent => right.at(Prec::Exponent),
		Prec::Nullish => right.at(Prec::Equality),
		_ => right.at(next(prec)),
	};
	Expr {
		text: format!("{left_text} {op} {right_text}"),
		prec,
		stable: false,
		pure: left.pure && right.pure,
	}
}

/// A prefix operation `op operand`.
pub fn prefix(op: &str, operand: &Expr) -> Expr {
	let text = operand.at(Prec::Unary);
	// `- -a` must not become `--a`
	let space = if op == "-" && text.starts_with('-') {
		" "
	} else {
		""
	};
	Expr {
		text: format!("{op}{space}{text}"),
		prec: Prec::Unary,
		stable: false,
		pure: operand.pure,
	}
}

/// The precedence just above `prec`.
fn next(prec: Prec) -> Prec {
	match prec {
		Prec::Assign => Prec::Conditional,
		Prec::Conditional => Prec::Nullish,
		Prec::Nullish => Prec::Or,
		Prec::Or => Prec::And,
		Prec::And => Prec::BitOr,
		Prec::BitOr => Prec::BitXor,
		Prec::BitXor => Prec::BitAnd,
		Prec::BitAnd => Prec::Equality,
		Prec::Equality => Prec::Relational,
		Prec::Relational => Prec::Shift,
		Prec::Shift => Prec::Additive,
		Prec::Additive => Prec::Multiplicative,
		Prec::Multiplicative => Prec::Exponent,
		Prec::Exponent => Prec::Unary,
		Prec::Unary | Prec::Primary => Prec::Primary,
	}
}

/// A call `callee(arguments)`.
pub fn call(callee: &str, arguments: &[String]) -> Expr {
	Expr::primary(format!("{callee}({})", arguments.join(", ")))
}

/// A conditional `condition ? yes : no`.
pub fn conditional(condition: &Expr, yes: &Expr, no: &Expr) -> Expr {
	Expr {
		text: format!(
			"{} ? {} : {}",
			condition.at(Prec::Nullish),
			yes.at(Prec::Assign),
			no.at(Prec::Assign)
		),
		prec: Prec::Conditional,
		stable: false,
		pure: condition.pure && yes.pure && no.pure,
	}
}

/// A JavaScript string literal in double quotes.
pub fn string_literal(text: &str) -> String {
	let mut out = String::with_capacity(text.len() + 2);
	out.push('"');
	let mut chars = text.chars().peekable();
	while let Some(c) = chars.next() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			'\0' if !chars.peek().is_some_and(char::is_ascii_digit) => out.push_str("\\0"),
			'\u{2028}' => out.push_str("\\u2028"),
			'\u{2029}' => out.push_str("\\u2029"),
			c if (c as u32) < 0x20 || c as u32 == 0x7f => {
				let _ = write!(out, "\\x{:02x}", c as u32);
			}
			c => out.push(c),
		}
	}
	out.push('"');
	out
}

/// The text of a template literal part (between backticks), escaped.
pub fn template_text(text: &str) -> String {
	let mut out = String::with_capacity(text.len());
	let mut chars = text.chars().peekable();
	while let Some(c) = chars.next() {
		match c {
			'`' => out.push_str("\\`"),
			'\\' => out.push_str("\\\\"),
			'$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
			'\r' => out.push_str("\\r"),
			'\0' => out.push_str("\\x00"),
			'\u{2028}' => out.push_str("\\u2028"),
			'\u{2029}' => out.push_str("\\u2029"),
			c if ((c as u32) < 0x20 && c != '\n' && c != '\t') || c as u32 == 0x7f => {
				let _ = write!(out, "\\x{:02x}", c as u32);
			}
			c => out.push(c),
		}
	}
	out
}

/// A JavaScript number literal for a `f64`, as short as possible and exact.
pub fn float_literal(value: f64) -> Expr {
	if value.is_nan() {
		return Expr::constant("NaN", Prec::Primary);
	}
	if value.is_infinite() {
		return if value > 0.0 {
			Expr::constant("Infinity", Prec::Primary)
		} else {
			Expr::constant("-Infinity", Prec::Unary)
		};
	}
	// Rust's shortest round-trip digits, in the shorter of the two notations
	let text = if value == 0.0 && value.is_sign_negative() {
		"-0".to_string()
	} else {
		let plain = format!("{value}");
		let exponent = format!("{value:e}");
		if exponent.len() < plain.len() {
			exponent
		} else {
			plain
		}
	};
	let prec = if text.starts_with('-') {
		Prec::Unary
	} else {
		Prec::Primary
	};
	Expr::constant(text, prec)
}

/// Lines of JavaScript code with their indentation.
#[derive(Default, Debug, Clone)]
pub struct Writer {
	lines: Vec<String>,
	indent: usize,
}

impl Writer {
	/// An empty writer at indentation 0.
	pub fn new() -> Self {
		Self::default()
	}

	/// An empty writer at an indentation.
	pub fn at(indent: usize) -> Self {
		Self {
			lines: Vec::new(),
			indent,
		}
	}

	/// Writes a line at the current indentation.
	pub fn line(&mut self, text: impl AsRef<str>) {
		let text = text.as_ref();
		let mut line = "\t".repeat(self.indent);
		line.push_str(text);
		self.lines.push(line);
	}

	/// Writes an empty line (never two in a row, never first).
	pub fn blank(&mut self) {
		if self.lines.last().is_some_and(|line| !line.is_empty()) {
			self.lines.push(String::new());
		}
	}

	/// Opens a block: writes `header {` and indents.
	pub fn open(&mut self, header: impl AsRef<str>) {
		let header = header.as_ref();
		if header.is_empty() {
			self.line("{");
		} else {
			self.line(format!("{header} {{"));
		}
		self.indent += 1;
	}

	/// Closes a block: dedents and writes `}`, followed by `after` when not
	/// empty (`} else {` is `close("else {")` with an indent).
	pub fn close(&mut self, after: &str) {
		self.indent = self.indent.saturating_sub(1);
		if after.is_empty() {
			self.line("}");
		} else {
			self.line(format!("}} {after}"));
		}
	}

	/// Closes a block and opens the next one on the same line: `} else {`.
	pub fn reopen(&mut self, header: &str) {
		self.indent = self.indent.saturating_sub(1);
		self.line(format!("}} {header} {{"));
		self.indent += 1;
	}

	/// The current indentation.
	pub fn indentation(&self) -> usize {
		self.indent
	}

	/// Whether nothing was written.
	pub fn is_empty(&self) -> bool {
		self.lines.is_empty()
	}

	/// Appends the lines of another writer (already indented).
	pub fn append(&mut self, other: Writer) {
		self.lines.extend(other.lines);
	}

	/// Inserts lines (already indented) at a position.
	pub fn insert(&mut self, at: usize, lines: Vec<String>) {
		let tail = self.lines.split_off(at.min(self.lines.len()));
		self.lines.extend(lines);
		self.lines.extend(tail);
	}

	/// The number of lines written.
	pub fn len(&self) -> usize {
		self.lines.len()
	}

	/// The code, one line per line, ending with a newline.
	pub fn finish(self) -> String {
		let mut out = String::new();
		for line in self.lines {
			out.push_str(&line);
			out.push('\n');
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn strings_are_escaped() {
		assert_eq!(string_literal("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
		assert_eq!(string_literal("\0"), r#""\0""#);
		assert_eq!(string_literal("\u{0}1"), r#""\x001""#);
		assert_eq!(string_literal("é☃"), "\"é☃\"");
		assert_eq!(template_text("a`b${c}\\"), "a\\`b\\${c}\\\\");
	}

	#[test]
	fn floats_are_short() {
		assert_eq!(float_literal(0.5).text, "0.5");
		assert_eq!(float_literal(1e21).text, "1e21");
		assert_eq!(float_literal(2.0).text, "2");
		assert_eq!(float_literal(-1.5).text, "-1.5");
		assert_eq!(float_literal(1e-7).text, "1e-7");
		assert_eq!(float_literal(0.1).text, "0.1");
		assert_eq!(float_literal(f64::INFINITY).text, "Infinity");
	}

	#[test]
	fn precedence_adds_parentheses() {
		let a = Expr::name("a", true);
		let b = Expr::name("b", true);
		let sum = binary(&a, "+", &b, Prec::Additive);
		let product = binary(&sum, "*", &a, Prec::Multiplicative);
		assert_eq!(product.text, "(a + b) * a");
		let difference = binary(&a, "-", &sum, Prec::Additive);
		assert_eq!(difference.text, "a - (a + b)");
		let negative = prefix("-", &a);
		assert_eq!(
			binary(&negative, "**", &b, Prec::Exponent).text,
			"(-a) ** b"
		);
		let power = binary(&a, "**", &b, Prec::Exponent);
		assert_eq!(binary(&a, "**", &power, Prec::Exponent).text, "a ** a ** b");
		assert_eq!(
			binary(&power, "**", &a, Prec::Exponent).text,
			"(a ** b) ** a"
		);
		assert_eq!(prefix("-", &negative).text, "- -a");
		let comparison = binary(&a, "<", &b, Prec::Relational);
		assert_eq!(
			binary(&comparison, "===", &a, Prec::Equality).text,
			"a < b === a"
		);
	}
}
