//! Printing types as a Fa programmer writes them, for messages and hover.
//!
//! Spellings (aliases as written, `UserId` for `Integer`) are applied by the
//! callers that have them; this module prints the structure.

use std::fmt::Write;

use ethnum::I256;

use crate::ids::TypeId;
use crate::types::store::TypeStore;
use crate::types::{CollKind, ConstValue, FieldFlags, FloatBits, IntType, TypeKind};

/// The brackets around the arguments of a type application (`Array<Integer>`,
/// `Point<Integer>`): every printer of types writes them through
/// [`open_arguments`] and [`close_arguments`], so that the spelling with
/// angle brackets is a change of this one constant.
pub const ARGUMENT_BRACKETS: (char, char) = ('<', '>');

/// Opens the arguments of a type application.
pub fn open_arguments(out: &mut String) {
	out.push(ARGUMENT_BRACKETS.0);
}

/// Closes the arguments of a type application.
pub fn close_arguments(out: &mut String) {
	out.push(ARGUMENT_BRACKETS.1);
}

/// A type application spelled from its name and its spelled arguments:
/// `Point<Integer>`.
pub fn type_application(name: &str, arguments: &[String]) -> String {
	let mut out = String::from(name);
	open_arguments(&mut out);
	out.push_str(&arguments.join(", "));
	close_arguments(&mut out);
	out
}

impl TypeStore {
	/// The type as written in Fa: `Integer<min = 0>`, `String?`, `"a" | "b"`,
	/// `{ name: String, age: Integer }`, `(Integer) => String`.
	pub fn display(&self, ty: TypeId) -> String {
		let mut out = String::new();
		self.write_type(&mut out, ty, false);
		out
	}

	/// Writes a type; `nested` parenthesizes unions and functions.
	fn write_type(&self, out: &mut String, ty: TypeId, nested: bool) {
		if ty.is_unset() {
			out.push_str("<unset>");
			return;
		}
		match self.kind(ty) {
			TypeKind::Never => out.push_str("Never"),
			TypeKind::Void => out.push_str("Void"),
			TypeKind::None => out.push_str("None"),
			TypeKind::ErrorTop => out.push_str("Error"),
			TypeKind::Unknown => out.push_str("Unknown"),
			TypeKind::Boolean => out.push_str("Boolean"),
			TypeKind::BoolLit(value) => out.push_str(if value { "true" } else { "false" }),
			TypeKind::Int(id) => write_int(out, self.int(id)),
			TypeKind::SymInt(_) => out.push_str("Integer(…)"),
			TypeKind::Float(FloatBits::F64) => out.push_str("Float"),
			TypeKind::Float(FloatBits::F32) => out.push_str("Float32"),
			TypeKind::FloatLit(_, value) | TypeKind::BigIntLit(value) | TypeKind::Const(value) => {
				self.write_const(out, value)
			}
			TypeKind::BigInt => out.push_str("BigInt"),
			TypeKind::Percentage => out.push_str("Percentage"),
			TypeKind::String => out.push_str("String"),
			TypeKind::StringLit(sym) => write_quoted(out, self.syms.resolve(sym)),
			TypeKind::Character => out.push_str("Character"),
			TypeKind::Object(shape) => {
				if let Some(nominal) = self.shape(shape).nominal {
					out.push_str(self.decl_name(self.nominal(nominal).key));
					return;
				}
				if let Some(name) = self.type_name(ty) {
					out.push_str(name);
					return;
				}
				let fields = self.shape_fields(shape);
				if fields.is_empty() {
					out.push_str("{}");
					return;
				}
				out.push_str("{ ");
				for (index, field) in fields.iter().enumerate() {
					if index > 0 {
						out.push_str(", ");
					}
					if field.flags.contains(FieldFlags::READONLY) {
						out.push_str("readonly ");
					}
					if field.flags.contains(FieldFlags::REACTIVE) {
						out.push_str("reactive ");
					}
					out.push_str(self.syms.resolve(field.name));
					out.push_str(": ");
					self.write_type(out, field.ty, false);
				}
				out.push_str(" }");
			}
			TypeKind::Tuple(list) => {
				out.push_str("Tuple");
				open_arguments(out);
				self.write_list(out, self.list_items(list));
				close_arguments(out);
			}
			TypeKind::Union(list) => {
				let members = self.list_items(list);
				let others: smallvec::SmallVec<[TypeId; 8]> = members
					.iter()
					.copied()
					.filter(|&member| member != TypeId::NONE)
					.collect();
				let optional = others.len() < members.len();
				let parenthesize =
					(nested && !optional && others.len() > 1) || (optional && others.len() > 1);
				if parenthesize {
					out.push('(');
				}
				for (index, &member) in others.iter().enumerate() {
					if index > 0 {
						out.push_str(" | ");
					}
					self.write_type(out, member, true);
				}
				if parenthesize {
					out.push(')');
				}
				if optional {
					out.push('?');
				}
			}
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				let info = self.nominal(nominal);
				let names = self.sym_list(info.members);
				let full = (0..names.len()).all(|index| self.mask_has(mask, index as u16));
				let name = self.decl_name(info.key);
				if full {
					out.push_str(name);
					self.write_args(out, info.args);
					return;
				}
				if nested {
					out.push('(');
				}
				let mut first = true;
				for (index, &member) in names.iter().enumerate() {
					if self.mask_has(mask, index as u16) {
						if !first {
							out.push_str(" | ");
						}
						first = false;
						let _ = write!(out, "{name}.{}", self.syms.resolve(member));
					}
				}
				if nested {
					out.push(')');
				}
			}
			TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) => {
				let info = self.nominal(nominal);
				let member = self
					.sym_list(info.members)
					.get(index as usize)
					.map_or("?", |&sym| self.syms.resolve(sym));
				let _ = write!(out, "{}.{member}", self.decl_name(info.key));
			}
			TypeKind::Fields(_) => out.push_str("Fields<…>"),
			TypeKind::CStruct(nominal) => out.push_str(self.decl_name(self.nominal(nominal).key)),
			TypeKind::Function(sig) => {
				let sig = self.sig(sig);
				if nested {
					out.push('(');
				}
				if sig.is_async {
					out.push_str("async ");
				}
				// a generic function type: `<T, Value: Integer | Float>(…) => …`
				let generics = self.list_items(sig.generics);
				if !generics.is_empty() {
					out.push('<');
					for (index, &generic) in generics.iter().enumerate() {
						if index > 0 {
							out.push_str(", ");
						}
						self.write_type(out, generic, false);
						if let TypeKind::Param(param) = self.kind(generic) {
							let constraint = self.param(param).constraint;
							if constraint != TypeId::UNKNOWN && !constraint.is_unset() {
								out.push_str(": ");
								self.write_type(out, constraint, false);
							}
						}
					}
					out.push('>');
				}
				out.push('(');
				for (index, param) in self.params(sig.params).iter().enumerate() {
					if index > 0 {
						out.push_str(", ");
					}
					// a variadic parameter's type is `Array(T)`, written `...T`
					let ty = match (param.variadic, self.kind(param.ty)) {
						(true, TypeKind::Collection(CollKind::Array, item, _)) => {
							out.push_str("...");
							item
						}
						_ => param.ty,
					};
					self.write_type(out, ty, false);
				}
				out.push_str(") => ");
				self.write_type(out, sig.ret, true);
				if sig.throws != TypeId::NEVER {
					out.push_str(" throws ");
					self.write_type(out, sig.throws, true);
				}
				if nested {
					out.push(')');
				}
			}
			TypeKind::Collection(kind, item, key) => {
				out.push_str(kind.name());
				open_arguments(out);
				if key != TypeId::NONE {
					self.write_type(out, key, false);
					out.push_str(", ");
				}
				self.write_type(out, item, false);
				close_arguments(out);
			}
			TypeKind::Iterator(item, _) => {
				out.push_str("Iterator");
				open_arguments(out);
				self.write_type(out, item, false);
				close_arguments(out);
			}
			TypeKind::Ref(target, _) => {
				self.write_type(out, target, true);
				out.push_str(" in …");
			}
			TypeKind::Range => out.push_str("Range"),
			TypeKind::Subscription => out.push_str("Subscription"),
			TypeKind::Pointer(target) => {
				out.push_str("Pointer");
				open_arguments(out);
				self.write_type(out, target, false);
				close_arguments(out);
			}
			TypeKind::CString => out.push_str("CString"),
			TypeKind::CArray(item, count) => {
				out.push_str("CArray");
				open_arguments(out);
				self.write_type(out, item, false);
				out.push_str(&format!(", {count}"));
				close_arguments(out);
			}
			// a worker is shown as its type
			TypeKind::Sync(crate::types::SyncKind::Worker, item) => {
				self.write_type(out, item, false)
			}
			TypeKind::Sync(kind, item) => {
				out.push_str(kind.name());
				if kind.is_generic() {
					open_arguments(out);
					self.write_type(out, item, false);
					close_arguments(out);
				}
			}
			TypeKind::Task(result, throws) => {
				out.push_str("Task");
				open_arguments(out);
				self.write_type(out, result, false);
				close_arguments(out);
				if throws != TypeId::NEVER {
					out.push_str(" throws ");
					self.write_type(out, throws, true);
				}
			}
			TypeKind::Param(param) => out.push_str(self.syms.resolve(self.param(param).name)),
			TypeKind::ImplicitParam(..) => out.push_str("Type(…)"),
			TypeKind::Rec(rec) => {
				let info = self.rec_info(rec);
				out.push_str(self.syms.resolve(info.name));
				let args = self.list_items(info.args);
				if !args.is_empty() {
					open_arguments(out);
					self.write_list(out, args);
					close_arguments(out);
				}
			}
		}
	}

	fn write_list(&self, out: &mut String, items: &[TypeId]) {
		for (index, &item) in items.iter().enumerate() {
			if index > 0 {
				out.push_str(", ");
			}
			self.write_type(out, item, false);
		}
	}

	fn write_args(&self, out: &mut String, args: crate::ids::ListId) {
		let args = self.list_items(args);
		if !args.is_empty() {
			open_arguments(out);
			self.write_list(out, args);
			close_arguments(out);
		}
	}

	fn write_const(&self, out: &mut String, value: crate::ids::ConstId) {
		match self.const_value(value) {
			ConstValue::Bool(value) => out.push_str(if value { "true" } else { "false" }),
			ConstValue::Int(value) => {
				let _ = write!(out, "{value}");
			}
			ConstValue::Float(bits) => {
				let _ = write!(out, "{:?}", f64::from_bits(bits));
			}
			ConstValue::String(sym) | ConstValue::Character(sym) => {
				write_quoted(out, self.syms.resolve(sym))
			}
			ConstValue::BigInt(sym) => {
				let _ = write!(out, "{}n", self.syms.resolve(sym));
			}
			ConstValue::Binding(index) => out.push_str(&format!("<local {index}>")),
			ConstValue::EnumMember(nominal, index) => {
				let info = self.nominal(nominal);
				let member = self
					.sym_list(info.members)
					.get(index as usize)
					.map_or("?", |&sym| self.syms.resolve(sym));
				let _ = write!(out, "{}.{member}", self.decl_name(info.key));
			}
		}
	}
}

/// `Integer`, `Int8<min = 0>`, `Integer<min = 0, max = 10>`, or a literal `42`.
fn write_int(out: &mut String, int: &IntType) {
	if let Some(value) = int.literal() {
		let _ = write!(out, "{value}");
		return;
	}
	out.push_str(int.storage.name());
	let range = int.storage.range();
	let bounds: [(bool, &str, I256); 2] = [
		(int.min != range.lo, "min", int.min),
		(int.max != range.hi, "max", int.max),
	];
	let mut first = true;
	for (shown, name, value) in bounds {
		if shown {
			out.push_str(if first { "<" } else { ", " });
			first = false;
			let _ = write!(out, "{name} = {value}");
		}
	}
	if !first {
		out.push('>');
	}
}

/// A string with Fa's escapes, in double quotes.
fn write_quoted(out: &mut String, text: &str) {
	out.push('"');
	for character in text.chars() {
		match character {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\t' => out.push_str("\\t"),
			'{' => out.push_str("\\{"),
			other => out.push(other),
		}
	}
	out.push('"');
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::{ListId, Sym};
	use crate::types::{CollKind, FieldEntry, IntStorage, NominalInfo, NominalKind};

	fn n(value: i128) -> I256 {
		I256::from(value)
	}

	#[test]
	fn scalars() {
		let mut store = TypeStore::new();
		assert_eq!(store.display(TypeId::INTEGER), "Integer");
		assert_eq!(store.display(TypeId::INT64), "Int64");
		assert_eq!(store.display(TypeId::BYTE), "UInt8");
		assert_eq!(store.display(TypeId::FLOAT32), "Float32");
		assert_eq!(store.display(TypeId::TRUE), "true");
		assert_eq!(store.display(TypeId::UNSET), "<unset>");
		let natural = store.int_range(n(0), IntStorage::Default.range().hi, IntStorage::Default);
		assert_eq!(store.display(natural), "Integer<min = 0>");
		let score = store.int_range(n(0), n(10), IntStorage::Default);
		assert_eq!(store.display(score), "Integer<min = 0, max = 10>");
		let small = store.int_range(n(-128), n(5), IntStorage::I8);
		assert_eq!(store.display(small), "Int8<max = 5>");
		let answer = store.int_literal(n(-42));
		assert_eq!(store.display(answer), "-42");
		let hello = store.string_literal("say \"hi\" {x}");
		assert_eq!(store.display(hello), r#""say \"hi\" \{x}""#);
		let half = store.float_literal(FloatBits::F64, 0.5);
		assert_eq!(store.display(half), "0.5");
	}

	#[test]
	fn unions_and_optionals() {
		let mut store = TypeStore::new();
		let maybe = store.optional(TypeId::STRING);
		assert_eq!(store.display(maybe), "String?");
		let a = store.string_literal("a");
		let b = store.string_literal("b");
		let ab = store.union(&[a, b]);
		assert_eq!(store.display(ab), r#""a" | "b""#);
		let maybe_ab = store.optional(ab);
		assert_eq!(store.display(maybe_ab), r#"("a" | "b")?"#);
		let array = store.array(ab);
		assert_eq!(store.display(array), r#"Array<"a" | "b">"#);
	}

	#[test]
	fn structures() {
		let mut store = TypeStore::new();
		let name = FieldEntry {
			name: store.syms.intern("name"),
			ty: TypeId::STRING,
			flags: FieldFlags::READONLY,
		};
		let age = FieldEntry {
			name: store.syms.intern("age"),
			ty: TypeId::INTEGER,
			flags: FieldFlags::empty(),
		};
		let person = store.object(&[name, age], None);
		assert_eq!(
			store.display(person),
			"{ age: Integer, readonly name: String }"
		);
		let empty = store.object(&[], None);
		assert_eq!(store.display(empty), "{}");
		let tuple = store.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		assert_eq!(store.display(tuple), "Tuple<Integer, String>");
		let map = store.collection(CollKind::Map, TypeId::INTEGER, TypeId::STRING);
		assert_eq!(store.display(map), "Map<String, Integer>");
		let sig = store.simple_sig(
			&[TypeId::INTEGER, TypeId::STRING],
			TypeId::BOOLEAN,
			TypeId::NEVER,
		);
		let function = store.function(sig);
		assert_eq!(store.display(function), "(Integer, String) => Boolean");
		let maybe_function = store.optional(function);
		assert_eq!(
			store.display(maybe_function),
			"((Integer, String) => Boolean)?"
		);
	}

	#[test]
	fn nominal_types() {
		let mut store = TypeStore::new();
		let module = store.module("main");
		let key = store.decl_key(module, &["Shape"], 0);
		let names: Vec<Sym> = ["Circle", "Square", "Point"]
			.iter()
			.map(|name| store.syms.intern(name))
			.collect();
		let members = store.intern_sym_list(&names);
		let shape = store.intern_nominal(NominalInfo {
			key,
			args: ListId::EMPTY,
			kind: NominalKind::Union,
			members,
		});
		let all = store.nominal_type(shape);
		assert_eq!(store.display(all), "Shape");
		let circle = store.member(shape, 0);
		assert_eq!(store.display(circle), "Shape.Circle");
		let point = store.member(shape, 2);
		let some = store.union(&[circle, point]);
		assert_eq!(store.display(some), "Shape.Circle | Shape.Point");
		let maybe = store.optional(some);
		assert_eq!(store.display(maybe), "(Shape.Circle | Shape.Point)?");
	}
}
