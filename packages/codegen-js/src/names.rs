//! JavaScript names of Fa symbols.
//!
//! A Fa name keeps its spelling in JavaScript, except when that would change
//! its meaning:
//!
//! - a JavaScript reserved word, a global the generated code uses (`Math`,
//!   `BigInt`, `process`…) or a runtime helper name gets a `$` suffix
//!   (`class$`);
//! - a name declared while another name with the same spelling is visible
//!   (Fa shadowing, which JavaScript would turn into a temporal dead zone
//!   error: `let x = x + 1` in an inner block) gets a numbered suffix (`x$1`).
//!
//! Fa identifiers never contain `$`, so the suffixed names and the
//! compiler's temporaries (`$t0`) never collide with user names.

use std::collections::HashSet;

/// JavaScript reserved words, strict-mode and module restrictions included,
/// and the globals the generated code relies on.
const RESERVED: &[&str] = &[
	// keywords and reserved words
	"arguments",
	"await",
	"break",
	"case",
	"catch",
	"class",
	"const",
	"continue",
	"debugger",
	"default",
	"delete",
	"do",
	"else",
	"enum",
	"eval",
	"export",
	"extends",
	"false",
	"finally",
	"for",
	"function",
	"if",
	"implements",
	"import",
	"in",
	"instanceof",
	"interface",
	"let",
	"new",
	"null",
	"package",
	"private",
	"protected",
	"public",
	"return",
	"static",
	"super",
	"switch",
	"this",
	"throw",
	"true",
	"try",
	"typeof",
	"var",
	"void",
	"while",
	"with",
	"yield",
	// globals used by the generated code and the runtime
	"Array",
	"BigInt",
	"Boolean",
	"Error",
	"Infinity",
	"Intl",
	"JSON",
	"Map",
	"Math",
	"NaN",
	"Number",
	"Object",
	"RangeError",
	"Set",
	"String",
	"Symbol",
	"TypeError",
	"console",
	"globalThis",
	"process",
	"undefined",
];

/// Whether a name can't be used as is.
pub fn is_reserved(name: &str) -> bool {
	RESERVED.contains(&name) || crate::runtime::helper(name).is_some()
}

/// The names visible at a point of the generated code, by scope.
#[derive(Debug, Default)]
pub struct Scopes {
	scopes: Vec<HashSet<String>>,
}

impl Scopes {
	/// One open scope: the module.
	pub fn new() -> Self {
		Self {
			scopes: vec![HashSet::new()],
		}
	}

	/// Opens a scope.
	pub fn push(&mut self) {
		self.scopes.push(HashSet::new());
	}

	/// Closes the innermost scope.
	pub fn pop(&mut self) {
		if self.scopes.len() > 1 {
			self.scopes.pop();
		}
	}

	/// Whether a name is visible in an open scope.
	pub fn is_visible(&self, name: &str) -> bool {
		self.scopes.iter().any(|scope| scope.contains(name))
	}

	/// Declares a name in the module scope (a module-level constant made
	/// while generating an inner scope).
	pub fn declare_global(&mut self, wanted: &str) -> String {
		let mut name = wanted.to_string();
		let mut index = 1;
		while self.is_visible(&name) || is_reserved(&name) {
			name = format!("{wanted}${index}");
			index += 1;
		}
		self.scopes[0].insert(name.clone());
		name
	}

	/// Declares a name in the innermost scope: the Fa spelling, or a
	/// suffixed one when it is reserved or already visible.
	pub fn declare(&mut self, wanted: &str) -> String {
		let base = if wanted.is_empty() {
			"$unnamed".to_string()
		} else if is_reserved(wanted) {
			format!("{wanted}$")
		} else {
			wanted.to_string()
		};
		let mut name = base.clone();
		let mut index = 1;
		while self.is_visible(&name) {
			name = format!("{base}${index}");
			index += 1;
		}
		self.scopes
			.last_mut()
			.expect("the module scope")
			.insert(name.clone());
		name
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reserved_names_get_a_suffix() {
		let mut scopes = Scopes::new();
		assert_eq!(scopes.declare("count"), "count");
		assert_eq!(scopes.declare("class"), "class$");
		assert_eq!(scopes.declare("Math"), "Math$");
		assert_eq!(scopes.declare("faLog"), "faLog$");
	}

	#[test]
	fn shadowing_gets_a_number() {
		let mut scopes = Scopes::new();
		assert_eq!(scopes.declare("x"), "x");
		scopes.push();
		assert_eq!(scopes.declare("x"), "x$1");
		assert_eq!(scopes.declare("y"), "y");
		scopes.pop();
		scopes.push();
		assert_eq!(scopes.declare("y"), "y");
		scopes.pop();
	}
}
