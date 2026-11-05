//! The C header of a package built as a C library (`build.cLibrary`,
//! foreign-code.md, "Exposing a library to C"), generated from the
//! interface of its API file: plain text, available without LLVM.
//!
//! - Integers are the exact-width types of `<stdint.h>` (`c.Int` is an
//!   `int32_t`, the same type on every target Fa supports), `Boolean` is a
//!   `bool`, `Float32` and `Float64` are `float` and `double`, a `CString`
//!   is a `const char *`.
//! - A `CStruct` is a `struct` defined with its fields in order
//!   (`typedef struct farm_Point { … } farm_Point;`), an empty one or a Fa
//!   value behind a `Pointer` an opaque one (`typedef struct farm_Farm
//!   farm_Farm;`): `prefix_` and the type's name (its path joined:
//!   `Farm.Error` is `FarmError`).
//! - An `Errors` type is an `enum` of its members, from 1:
//!   `FARM_FARM_ERROR_EMPTY = 1`. A function that throws returns an `int`
//!   (0 on success), its result through a last parameter, `result`.
//! - `<prefix>_last_error_message()` describes the last error of the
//!   calling thread.

use fa_analyzer::c_library::{self, BoundaryType};
use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::modules::Interface;
use fa_analyzer::types::{NominalKind, TypeKind, TypeStore};

use crate::options::CLibrary;

/// The words C reserves: a parameter named so gets a `_` after its name.
const C_KEYWORDS: &[&str] = &[
	"auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
	"enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
	"restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
	"union", "unsigned", "void", "volatile", "while", "bool", "true", "false",
];

/// The header being written: the declarations of the types, in an order
/// where a `struct` used by value is defined before its users.
struct Header<'a> {
	types: &'a TypeStore,
	prefix: &'a str,
	/// The opaque types (`typedef struct x x;`), by name.
	opaque: Vec<String>,
	/// The `struct` definitions, in dependency order.
	structs: Vec<String>,
	/// The `enum`s of the error types.
	enums: Vec<String>,
	/// The names declared so far (each type once).
	declared: Vec<String>,
}

/// `FarmError` → `FARM_ERROR`.
fn upper_snake(name: &str) -> String {
	c_library::snake_case(name).to_uppercase()
}

/// A name C can use as an identifier (a keyword gets a `_`).
fn identifier(name: &str) -> String {
	if C_KEYWORDS.contains(&name) {
		format!("{name}_")
	} else {
		name.to_string()
	}
}

impl Header<'_> {
	/// The C name of a declared type.
	fn type_name(&self, name: &str) -> String {
		format!("{}_{name}", self.prefix)
	}

	/// The scalar C type of a C type of Fa, if it is one.
	fn scalar(&self, ty: TypeId) -> Option<String> {
		Some(match c_library::boundary_type(self.types, ty, true)? {
			BoundaryType::Void => "void".to_string(),
			BoundaryType::Bool => "bool".to_string(),
			BoundaryType::Int { bits, signed } => {
				format!("{}int{bits}_t", if signed { "" } else { "u" })
			}
			BoundaryType::Float32 => "float".to_string(),
			BoundaryType::Float64 => "double".to_string(),
			BoundaryType::CString { .. } => "const char *".to_string(),
			_ => return None,
		})
	}

	/// The C type of a parameter, a result or a field (`Pointer(T)` is `T *`).
	fn c_type(&mut self, ty: TypeId) -> Result<String, String> {
		if let Some(scalar) = self.scalar(ty) {
			return Ok(scalar);
		}
		if let Some(target) = self.types.pointer_target(ty) {
			let pointee = self.pointee(target)?;
			return Ok(if pointee.ends_with('*') {
				format!("{pointee}*")
			} else {
				format!("{pointee} *")
			});
		}
		if let Some(nominal) = self.c_struct(ty) {
			return self.declare_struct(ty, nominal);
		}
		Err(format!(
			"a `{}` can't be a C type of the header",
			self.types.display(ty)
		))
	}

	/// The type a pointer designates: a C type, or an opaque `struct` for a
	/// Fa value.
	fn pointee(&mut self, ty: TypeId) -> Result<String, String> {
		if ty == TypeId::VOID {
			return Ok("void".to_string());
		}
		if let TypeKind::CArray(item, _) = self.types.kind(ty) {
			return self.pointee(item);
		}
		if self.scalar(ty).is_some()
			|| self.types.pointer_target(ty).is_some()
			|| self.c_struct(ty).is_some()
		{
			return self.c_type(ty);
		}
		// a Fa value: opaque for C
		let name = match self.types.kind(ty) {
			TypeKind::Object(shape) => match self.types.shape(shape).nominal {
				Some(nominal) => Some(c_library::nominal_name(self.types, nominal)),
				None => self.types.type_name(ty).map(str::to_string),
			},
			TypeKind::NamedUnion(nominal, _) | TypeKind::Enum(nominal, _) => {
				Some(c_library::nominal_name(self.types, nominal))
			}
			_ => Some(self.types.display(ty)),
		};
		let Some(name) = name.filter(|name| {
			!name.is_empty()
				&& name
					.chars()
					.all(|character| character.is_alphanumeric() || character == '_')
		}) else {
			return Err(format!(
				"a pointer to a `{}`, a type without a name: give it one (`type Name = …`), which names its opaque `struct` in the header",
				self.types.display(ty)
			));
		};
		let name = self.type_name(&name);
		if !self.declared.contains(&name) {
			self.declared.push(name.clone());
			self.opaque.push(name.clone());
		}
		Ok(name)
	}

	/// The `CStruct` nominal of a type, if it is one.
	fn c_struct(&self, ty: TypeId) -> Option<NominalId> {
		let TypeKind::Object(shape) = self.types.kind(ty) else {
			return None;
		};
		let nominal = self.types.shape(shape).nominal?;
		(self.types.nominal(nominal).kind == NominalKind::CStruct).then_some(nominal)
	}

	/// Declares a `CStruct` (its fields' types first), and gives its name.
	fn declare_struct(&mut self, ty: TypeId, nominal: NominalId) -> Result<String, String> {
		let name = self.type_name(&c_library::nominal_name(self.types, nominal));
		if self.declared.contains(&name) {
			return Ok(name);
		}
		self.declared.push(name.clone());
		let fields = self.types.fields_of(ty).to_vec();
		if fields.is_empty() {
			self.opaque.push(name.clone());
			return Ok(name);
		}
		let mut body = String::new();
		for field in fields {
			let field_name = identifier(self.types.syms.resolve(field.name));
			let (item, length) = match self.types.kind(field.ty) {
				TypeKind::CArray(item, length) => (item, Some(length)),
				_ => (field.ty, None),
			};
			let c = self.c_type(item)?;
			let separator = if c.ends_with('*') { "" } else { " " };
			match length {
				Some(length) => {
					body.push_str(&format!("\t{c}{separator}{field_name}[{length}];\n"))
				}
				None => body.push_str(&format!("\t{c}{separator}{field_name};\n")),
			}
		}
		self.structs
			.push(format!("typedef struct {name} {{\n{body}}} {name};\n"));
		Ok(name)
	}

	/// Declares the `enum` of an `Errors` type, and gives its name.
	fn declare_errors(&mut self, errors: NominalId) -> String {
		let type_name = c_library::nominal_name(self.types, errors);
		let name = self.type_name(&type_name);
		if self.declared.contains(&name) {
			return name;
		}
		self.declared.push(name.clone());
		let constant = format!("{}_{}", self.prefix.to_uppercase(), upper_snake(&type_name));
		let members: Vec<String> = c_library::error_members(self.types, errors)
			.iter()
			.enumerate()
			.map(|(index, member)| format!("\t{constant}_{} = {}", upper_snake(member), index + 1))
			.collect();
		self.enums.push(format!(
			"/* The errors of `{}`: the status of a function that throws them. */\ntypedef enum {name} {{\n{}\n}} {name};\n",
			type_name,
			members.join(",\n")
		));
		name
	}
}

/// The C header of a C library: its API file's interface, the C names
/// starting with `options.prefix`. `Err`: why a declaration can't be
/// written in C.
pub fn c_header(
	types: &TypeStore,
	interface: &Interface,
	options: &CLibrary,
) -> Result<String, String> {
	let Some(functions) = c_library::api_functions(types, interface, &options.prefix) else {
		return Err(format!(
			"the API of the C library (`{}`) doesn't export a namespace of functions",
			options.api
		));
	};
	let mut header = Header {
		types,
		prefix: &options.prefix,
		opaque: Vec::new(),
		structs: Vec::new(),
		enums: Vec::new(),
		declared: Vec::new(),
	};
	let mut declarations = Vec::new();
	for function in &functions {
		let TypeKind::Function(sig) = types.kind(function.ty) else {
			continue;
		};
		let sig = types.sig(sig);
		let names = interface.parameters.get(&function.decl);
		let mut params = Vec::new();
		let mut taken = Vec::new();
		for (position, param) in types.params(sig.params).iter().enumerate() {
			let name = names.and_then(|names| names.get(position)).map_or_else(
				|| format!("parameter{}", position + 1),
				|parameter| identifier(types.syms.resolve(parameter.name)),
			);
			let c = header
				.c_type(param.ty)
				.map_err(|error| format!("`{}`: {error}", function.name))?;
			let separator = if c.ends_with('*') { "" } else { " " };
			params.push(format!("{c}{separator}{name}"));
			taken.push(name);
		}
		let errors = c_library::error_type(types, sig.throws)
			.map_err(|_| format!("`{}` throws errors of several types", function.name))?;
		let result = if sig.ret == TypeId::NEVER {
			"void".to_string()
		} else {
			header
				.c_type(sig.ret)
				.map_err(|error| format!("`{}`: {error}", function.name))?
		};
		let mut comment = format!("/* `{}` */", function.name);
		let returned = match errors {
			Some(errors) => {
				let enumeration = header.declare_errors(errors);
				if result != "void" {
					let mut out = "result".to_string();
					while taken.contains(&out) {
						out.push('_');
					}
					let separator = if result.ends_with('*') { "" } else { " " };
					params.push(format!("{result}{separator}*{out}"));
					comment = format!(
						"/* `{}`: 0 on success (its result in `*{out}`), else a `{enumeration}`. */",
						function.name
					);
				} else {
					comment = format!(
						"/* `{}`: 0 on success, else a `{enumeration}`. */",
						function.name
					);
				}
				"int".to_string()
			}
			None => result,
		};
		if params.is_empty() {
			params.push("void".to_string());
		}
		let separator = if returned.ends_with('*') { "" } else { " " };
		declarations.push(format!(
			"{comment}\n{returned}{separator}{}({});\n",
			function.symbol,
			params.join(", ")
		));
	}
	let guard: String = options
		.header
		.chars()
		.map(|character| {
			if character.is_ascii_alphanumeric() {
				character.to_ascii_uppercase()
			} else {
				'_'
			}
		})
		.collect();
	let mut text = format!(
		"/* {}: the C interface of a Fa library (`{}`), generated by `fa build`. */\n#ifndef {guard}\n#define {guard}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n#ifdef __cplusplus\nextern \"C\" {{\n#endif\n\n",
		options.header, options.api
	);
	for name in &header.opaque {
		text.push_str(&format!("typedef struct {name} {name};\n"));
	}
	if !header.opaque.is_empty() {
		text.push('\n');
	}
	for definition in header.structs.iter().chain(&header.enums) {
		text.push_str(definition);
		text.push('\n');
	}
	for declaration in declarations {
		text.push_str(&declaration);
		text.push('\n');
	}
	text.push_str(&format!(
		"/* The message of the last error of the calling thread (\"\" before the first one). */\nconst char *{}_last_error_message(void);\n\n#ifdef __cplusplus\n}}\n#endif\n\n#endif\n",
		options.prefix
	));
	Ok(text)
}
