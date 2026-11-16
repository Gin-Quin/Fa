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
	// globals used by the generated code and the runtime (the test
	// `runtime_globals_are_reserved` checks the runtime's)
	"Array",
	"ArrayBuffer",
	"Atomics",
	"BigInt",
	"Buffer",
	"BigInt64Array",
	"BigUint64Array",
	"Boolean",
	"Bun",
	"DataView",
	"Deno",
	"Error",
	"File",
	"Float32Array",
	"Float64Array",
	"Infinity",
	"Int16Array",
	"Int32Array",
	"Int8Array",
	"Intl",
	"JSON",
	"Map",
	"Math",
	"MessageChannel",
	"NaN",
	"Number",
	"Object",
	"Promise",
	"Proxy",
	"Reflect",
	"RangeError",
	"Set",
	"SharedArrayBuffer",
	"String",
	"Symbol",
	"TextDecoder",
	"TextEncoder",
	"TypeError",
	"SyntaxError",
	"URL",
	"Uint16Array",
	"Uint32Array",
	"Uint8Array",
	"WeakMap",
	"Worker",
	"clearTimeout",
	"console",
	"fetch",
	"globalThis",
	"performance",
	"process",
	"queueMicrotask",
	"setImmediate",
	"setTimeout",
	"structuredClone",
	"undefined",
];

/// The properties of `Object.prototype`: a field of that name would be
/// read through the prototype, or set it (`__proto__`), so its JavaScript
/// property gets a `$` suffix (`constructor$`). The printers and JSON strip
/// the suffix back (`format.js`, `system.js`).
const OBJECT_PROPERTIES: &[&str] = &[
	"__defineGetter__",
	"__defineSetter__",
	"__lookupGetter__",
	"__lookupSetter__",
	"__proto__",
	"constructor",
	"hasOwnProperty",
	"isPrototypeOf",
	"propertyIsEnumerable",
	"toLocaleString",
	"toString",
	"valueOf",
];

/// Whether a field name is a property of `Object.prototype`.
pub fn is_object_property(name: &str) -> bool {
	OBJECT_PROPERTIES.contains(&name)
}

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
		// Synthesized names may include a displayed type (`Wrapper<String>`).
		// Keep them readable while ensuring the spelling is one JS identifier.
		let mut base: String = wanted
			.chars()
			.map(|character| {
				if character.is_ascii_alphanumeric() || matches!(character, '_' | '$') {
					character
				} else {
					'_'
				}
			})
			.collect();
		if !base.starts_with(|character: char| {
			character.is_ascii_alphabetic() || matches!(character, '_' | '$')
		}) {
			base.insert(0, '$');
		}
		let mut name = base.clone();
		let mut index = 1;
		while self.is_visible(&name) || is_reserved(&name) {
			name = format!("{base}${index}");
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
		assert_eq!(scopes.declare("faFormat"), "faFormat$");
	}

	/// The identifiers of JavaScript code, without its comments and strings
	/// (nor the properties after a `.`).
	fn test_identifiers(code: &str) -> HashSet<String> {
		let mut names = HashSet::new();
		let bytes = code.as_bytes();
		let mut index = 0;
		let mut previous = b' ';
		while index < bytes.len() {
			let byte = bytes[index];
			if code[index..].starts_with("//") {
				index = code[index..]
					.find('\n')
					.map_or(bytes.len(), |end| index + end);
			} else if code[index..].starts_with("/*") {
				index = code[index + 2..]
					.find("*/")
					.map_or(bytes.len(), |end| index + end + 4);
			} else if matches!(byte, b'"' | b'\'' | b'`') {
				index += 1;
				while index < bytes.len() && bytes[index] != byte {
					index += if bytes[index] == b'\\' { 2 } else { 1 };
				}
				index += 1;
				previous = byte;
			} else if byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$' {
				let start = index;
				while index < bytes.len()
					&& (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'$'))
				{
					index += 1;
				}
				if previous != b'.' {
					names.insert(code[start..index].to_string());
				}
				previous = b'a';
			} else {
				if !byte.is_ascii_whitespace() {
					previous = byte;
				}
				index += 1;
			}
		}
		names
	}

	/// The output of a script run by Bun; `None` when Bun is missing.
	fn test_bun_output(script: &str) -> Option<String> {
		let bun = std::env::var("HOME")
			.map(|home| std::path::PathBuf::from(home).join(".bun/bin/bun"))
			.ok()
			.filter(|path| path.exists())
			.unwrap_or_else(|| std::path::PathBuf::from("bun"));
		// a file, not `bun -e`, which also exposes the `node:` modules
		let name = format!("fa-names-{}-{}.js", std::process::id(), script.len());
		let file = std::env::temp_dir().join(name);
		std::fs::write(&file, script).ok()?;
		let output = std::process::Command::new(bun).arg(&file).output();
		let _ = std::fs::remove_file(&file);
		let output = output.ok()?;
		Some(String::from_utf8_lossy(&output.stdout).into_owned())
	}

	/// Every global of Bun that the runtime names is reserved, so that a
	/// module-level value never shadows it (backends review, finding 12).
	#[test]
	fn runtime_globals_are_reserved() {
		let Some(output) =
			test_bun_output("console.log(Object.getOwnPropertyNames(globalThis).join(' '))")
		else {
			eprintln!("skipped: Bun is missing");
			return;
		};
		let globals: HashSet<&str> = output.split_whitespace().collect();
		let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
		let mut missing = Vec::new();
		for entry in std::fs::read_dir(folder).expect("the runtime") {
			let path = entry.expect("a runtime file").path();
			let name = path.file_name().unwrap().to_string_lossy().into_owned();
			if !name.ends_with(".js") || name.ends_with(".test.js") {
				continue;
			}
			let code = std::fs::read_to_string(&path).expect("a runtime file");
			for identifier in test_identifiers(&code) {
				// `self`: only a parameter of the runtime (and the receiver
				// of every Fa method: reserving it would rename them all)
				if globals.contains(identifier.as_str())
					&& identifier != "self"
					&& !RESERVED.contains(&identifier.as_str())
					&& !missing.contains(&identifier)
				{
					missing.push(identifier);
				}
			}
		}
		missing.sort();
		assert!(missing.is_empty(), "add to `RESERVED`: {missing:?}");
	}

	/// `OBJECT_PROPERTIES` lists the properties of `Object.prototype`.
	#[test]
	fn object_properties_are_those_of_bun() {
		let Some(output) = test_bun_output(
			"console.log(Object.getOwnPropertyNames(Object.prototype).sort().join(' '))",
		) else {
			eprintln!("skipped: Bun is missing");
			return;
		};
		let found: Vec<&str> = output.split_whitespace().collect();
		assert_eq!(found, OBJECT_PROPERTIES);
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
