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
			"std.time.monotonic" => {
				let helper = self.helper("faMonotonic");
				js::call(helper, &[])
			}
			"File.read" | "File.readBytes" | "File.write" => {
				let values = values(self);
				let helper = self.helper(match path {
					"File.read" => "faReadFile",
					"File.readBytes" => "faReadBytes",
					_ => "faWriteFile",
				});
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

	/// A call of a method of a socket of `std.net` (`net.js`) that is not
	/// awaited: `close()`; the waits called blocking are `CONC010` on
	/// JavaScript (the analyzer reports them). `None` for another built-in.
	pub(crate) fn net_call(&mut self, n: NodeId, callee: NodeId, path: &str) -> Option<Expr> {
		Some(match path {
			"TcpListener#close" | "TcpStream#close" => {
				let receiver = self.receiver(callee);
				let helper = self.helper("faSocketClose");
				js::call(helper, &[receiver.text])
			}
			"TcpListener#accept" | "TcpStream.connect" | "TcpStream#read" | "TcpStream#write" => {
				self.unsupported(n, "a blocking call of a socket (await it)")
			}
			_ => return None,
		})
	}

	/// The awaited form of a primitive of `std.fs` or `std.net` (`await
	/// File.read(path)`): its helper, which takes the task first, then the
	/// receiver (for a socket's method) and the arguments. `None` for
	/// another primitive.
	pub(crate) fn io_await_helper(&mut self, name: &str) -> Option<(&'static str, bool)> {
		let (helper, receiver) = match name {
			"File.read" => ("faReadFileAsync", false),
			"File.readBytes" => ("faReadBytesAsync", false),
			"File.write" => ("faWriteFileAsync", false),
			"TcpListener.accept" => ("faAccept", true),
			"TcpStream.connect" => ("faConnect", false),
			"TcpStream.read" => ("faSocketRead", true),
			"TcpStream.write" => ("faSocketWrite", true),
			_ => return None,
		};
		Some((self.helper(helper), receiver))
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
				// the default values of the declared fields: a function giving a
				// new one for each value read without the field
				let defaults: Vec<(String, Option<NodeId>)> = self
					.analysis
					.type_decls
					.values()
					.find(|decl| decl.ty == ty)
					.map(|decl| {
						decl.fields
							.iter()
							.map(|field| {
								(
									self.types.syms.resolve(field.name).to_string(),
									field.default,
								)
							})
							.collect()
					})
					.unwrap_or_default();
				let mut parts = Vec::new();
				for name in order {
					let Some(&(_, field_ty)) = fields.iter().find(|(field, _)| *field == name)
					else {
						continue;
					};
					let optional = self.types.union_members(&field_ty).contains(&TypeId::NONE);
					let schema = self.json_schema(field_ty, depth + 1)?;
					let default = match defaults.iter().find(|(field, _)| *field == name) {
						Some((_, Some(node))) => {
							let indent = self.out.indentation();
							let (lines, value) = self.capture_at(indent, |g| g.expr(*node));
							if !lines.is_empty() {
								return None;
							}
							format!("() => ({})", value.text)
						}
						_ => "null".to_string(),
					};
					parts.push(format!(
						"[{}, {schema}, {optional}, {default}]",
						js::string_literal(&name)
					));
				}
				format!("[\"o\", [{}]]", parts.join(", "))
			}
			_ => return None,
		})
	}
}
