//! `std.time`, `std.fs` and `std.json` (advanced brief D9; runtime
//! `system.js`): `now()` is `Date.now()`, `File.read(path)` is
//! `faReadFile(path)`, `Json.stringify(value)` is `faJsonStringify(value)`,
//! and `Json.parse(text)` is `faJsonParse(text, schema)` with the schema of
//! the expected type (the call's type), written as a JavaScript literal.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::{Generator, Repr};
use crate::js::{self, Expr};

impl Generator<'_> {
	/// A call of a built-in of `std.time`, `std.fs` or `std.json`; `None`
	/// for another built-in.
	pub(crate) fn system_call(
		&mut self,
		n: NodeId,
		path: &str,
		values: impl FnOnce(&mut Self) -> Vec<String>,
	) -> Option<Expr> {
		Some(match path {
			"std.time.now" => Expr::primary("Date.now()".to_string()),
			"File.read" => {
				let values = values(self);
				let helper = self.helper("faReadFile");
				js::call(helper, &values)
			}
			"Json.stringify" => {
				let values = values(self);
				let helper = self.helper("faJsonStringify");
				js::call(helper, &values)
			}
			"Json.parse" => {
				let ty = self.type_of(n);
				let Some(schema) = self.json_schema(ty, 0) else {
					return Some(self.unsupported(n, "`Json.parse` of this type"));
				};
				let mut values = values(self);
				values.push(schema);
				let helper = self.helper("faJsonParse");
				js::call(helper, &values)
			}
			_ => return None,
		})
	}

	/// The schema `faJsonParse` reads a value of `ty` with (see `system.js`).
	fn json_schema(&mut self, ty: TypeId, depth: u32) -> Option<String> {
		if depth > 32 {
			return None;
		}
		let bound = |value: Option<String>| value.unwrap_or_else(|| "null".to_string());
		Some(match self.types.kind(ty) {
			TypeKind::Boolean => "\"b\"".to_string(),
			TypeKind::BoolLit(value) => format!("[\"l\", {value}]"),
			TypeKind::String | TypeKind::Character => "\"s\"".to_string(),
			TypeKind::StringLit(sym) => {
				format!(
					"[\"l\", {}]",
					js::string_literal(self.types.syms.resolve(sym))
				)
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) => "\"f\"".to_string(),
			TypeKind::None => "\"n\"".to_string(),
			TypeKind::Int(id) => {
				let int = *self.types.int(id);
				let range = int.storage.range();
				let min = (int.min > range.lo).then(|| format!("\"{}\"", int.min));
				let max = (int.max < range.hi).then(|| format!("\"{}\"", int.max));
				let kind = if self.repr(ty) == Repr::BigInt {
					"I"
				} else {
					"i"
				};
				if int.min == int.max && kind == "i" {
					format!("[\"l\", {}]", int.min)
				} else {
					format!("[\"{kind}\", {}, {}]", bound(min), bound(max))
				}
			}
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => {
				format!("[\"a\", {}]", self.json_schema(item, depth + 1)?)
			}
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, item, _) => {
				format!("[\"m\", {}]", self.json_schema(item, depth + 1)?)
			}
			TypeKind::Union(list) => {
				let members = self.types.list_items(list).to_vec();
				let mut parts = Vec::new();
				for member in members {
					parts.push(self.json_schema(member, depth + 1)?);
				}
				format!("[\"u\", [{}]]", parts.join(", "))
			}
			TypeKind::Object(_) => {
				let fields: Vec<(String, TypeId)> = self
					.types
					.fields_of(ty)
					.iter()
					.map(|field| (self.types.syms.resolve(field.name).to_string(), field.ty))
					.collect();
				// the declaration order, when the type is declared
				let order: Vec<String> = self
					.analysis
					.type_decls
					.values()
					.find(|decl| decl.ty == ty)
					.map(|decl| {
						decl.fields
							.iter()
							.map(|field| self.types.syms.resolve(field.name).to_string())
							.collect()
					})
					.unwrap_or_else(|| fields.iter().map(|(name, _)| name.clone()).collect());
				let mut parts = Vec::new();
				for name in order {
					let Some(&(_, field_ty)) = fields.iter().find(|(field, _)| *field == name)
					else {
						continue;
					};
					let optional = self.types.union_members(&field_ty).contains(&TypeId::NONE);
					let schema = self.json_schema(field_ty, depth + 1)?;
					parts.push(format!(
						"[{}, {schema}, {optional}]",
						js::string_literal(&name)
					));
				}
				format!("[\"o\", [{}]]", parts.join(", "))
			}
			_ => return None,
		})
	}
}
