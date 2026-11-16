//! `std.time`, `std.fs` and `std.json` (advanced brief D9; runtime
//! `system.js`): `now()` is `Date.now()`, `File.read(path)` is
//! `faReadFile(path)`, `Json.stringify(value)` is `faJsonStringify(value)`,
//! and `Json.parse(text)` is `faJsonParse(text, schema)` with the schema of
//! the expected type (the call's type), written as a JavaScript literal.

use fa_analyzer::ids::{NodeId, NominalId, TypeId};
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
			"File.readText" | "File.read" | "File.writeText" | "File.write" => {
				let values = values(self);
				let helper = self.helper(match path {
					"File.readText" => "faReadFile",
					"File.read" => "faReadBytes",
					"File.write" => "faWriteBytes",
					_ => "faWriteFile",
				});
				js::call(helper, &values)
			}
			"Json.stringify" => {
				// written by the schema of the value's type (enumerations,
				// errors, the declaration order of fields)
				let argument = self.analysis.call(n).and_then(|call| {
					call.arguments.iter().find_map(|argument| match argument {
						fa_analyzer::analysis::Argument::Given(value) => Some(*value),
						_ => None,
					})
				});
				let schema = argument.and_then(|argument| {
					let ty = self.type_of(argument);
					self.json_schema(ty, 0)
				});
				let mut values = values(self);
				values.push(schema.unwrap_or_else(|| "null".to_string()));
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
				let argument = self
					.analysis
					.call(n)
					.and_then(|call| call.arguments.first())
					.and_then(|arg| match arg {
						fa_analyzer::analysis::Argument::Given(node) => Some(*node),
						_ => None,
					});
				if argument.is_some_and(|node| {
					self.types
						.union_members(&self.type_of(node))
						.contains(&TypeId::BYTES)
				}) {
					values.push(self.helper("faJsonReadBytes").to_string());
				}
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
			"File.readText" => ("faReadFileAsync", false),
			"File.read" => ("faReadBytesAsync", false),
			"File.writeText" => ("faWriteFileAsync", false),
			"File.write" => ("faWriteBytesAsync", false),
			"TcpListener.accept" => ("faAccept", true),
			"TcpStream.connect" => ("faConnect", false),
			"TcpStream.read" => ("faSocketRead", true),
			"TcpStream.write" => ("faSocketWrite", true),
			_ => return None,
		};
		Some((self.helper(helper), receiver))
	}

	/// A member of a named union in the schema of `faJsonParse`.
	fn json_member(&mut self, nominal: NominalId, index: u16, depth: u32) -> Option<String> {
		let name = js::string_literal(self.types.member_name(nominal, index));
		let tag = js::string_literal(&self.member_qualified_name(nominal, index));
		let payload = self.types.payload(nominal, index);
		// a member of an `Errors` type: an instance of its class, built from
		// its fields in declaration order
		if self.is_errors(nominal) {
			let class = self.error_class(nominal, index);
			let fields = self.error_fields(nominal, index);
			let names: Vec<String> = fields
				.iter()
				.map(|field| js::string_literal(field))
				.collect();
			let data = if payload.is_unset() {
				"null".to_string()
			} else {
				self.json_schema(payload, depth + 1)?
			};
			let kind = if fields.iter().any(|field| field == "$v" || field == "type") {
				"V"
			} else {
				"x"
			};
			return Some(format!(
				"[{name}, {tag}, \"{kind}\", {data}, {class}, [{}]]",
				names.join(", ")
			));
		}
		if payload.is_unset() {
			return Some(format!("[{name}, {tag}, \"e\", null]"));
		}
		if let Some(literal) = self.literal_of(payload) {
			return Some(format!("[{name}, {tag}, \"l\", {}]", literal.text));
		}
		let data = self.json_schema(payload, depth + 1)?;
		let kind = match self.types.kind(payload) {
			TypeKind::Object(_)
				if self
					.types
					.fields_of(payload)
					.iter()
					.any(|field| self.types.syms.resolve(field.name) == "type") =>
			{
				"w"
			}
			TypeKind::Object(_) => "r",
			_ => "v",
		};
		Some(format!("[{name}, {tag}, \"{kind}\", {data}]"))
	}

	/// A member of an enumeration in the schema of `faJsonParse`.
	fn json_enum_member(&mut self, nominal: NominalId, index: u16) -> String {
		let name = js::string_literal(self.types.member_name(nominal, index));
		let value = self.enum_value(nominal, index).text;
		let pinned = self
			.types
			.as_int(self.types.payload(nominal, index))
			.is_some_and(|int| int.min == int.max);
		format!("[{name}, {value}, {pinned}]")
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
			TypeKind::String => "\"s\"".to_string(),
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
			// `["N", [[name, tag, kind, data], …]]`: a member without data
			// (`"e"`), a literal (`"l"`, its value), an object (`"r"`: its fields
			// next to `"type"`), an object with a `type` field (`"w"`) or other
			// data (`"v"`): under `"value"`
			TypeKind::NamedUnion(nominal, mask) => {
				let count = self
					.types
					.sym_list(self.types.nominal(nominal).members)
					.len() as u16;
				let mut parts = Vec::new();
				for index in (0..count).filter(|&index| self.types.mask_has(mask, index)) {
					parts.push(self.json_member(nominal, index, depth)?);
				}
				format!("[\"N\", [{}]]", parts.join(", "))
			}
			TypeKind::Member(nominal, index) => {
				format!("[\"N\", [{}]]", self.json_member(nominal, index, depth)?)
			}
			// `["E", [[name, value, pinned], …]]`: written as its pinned value,
			// else as its name
			TypeKind::Enum(nominal, mask) => {
				let count = self
					.types
					.sym_list(self.types.nominal(nominal).members)
					.len() as u16;
				let mut parts = Vec::new();
				for index in (0..count).filter(|&index| self.types.mask_has(mask, index)) {
					parts.push(self.json_enum_member(nominal, index));
				}
				format!("[\"E\", [{}]]", parts.join(", "))
			}
			TypeKind::EnumMember(nominal, index) => {
				format!("[\"E\", [{}]]", self.json_enum_member(nominal, index))
			}
			// `["F", [path, …]]`: the dotted paths of the flags (fields.rs)
			TypeKind::Fields(_) => {
				let accepted: Vec<String> = self
					.types
					.fields_names(ty)
					.iter()
					.map(|(name, _)| js::string_literal(name))
					.collect();
				format!(
					"[\"F\", {}, [{}]]",
					self.fields_names(ty),
					accepted.join(", ")
				)
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
					// an object written inline as a member's data: as written
					.or_else(|| {
						self.analysis.inline_field_orders.get(&ty).map(|names| {
							names
								.iter()
								.map(|&name| self.types.syms.resolve(name).to_string())
								.collect()
						})
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
						// a type of another file: the constant its file evaluated
						_ => {
							let sym = fields_of_sym(self, ty, &name);
							match sym.and_then(|sym| self.analysis.json_defaults.get(&(ty, sym))) {
								Some(&constant) => {
									let value = self
										.analysis
										.field_constants
										.get(constant as usize)?
										.clone();
									let value =
										self.constant_value(NodeId::new(0), &value, field_ty);
									format!("() => ({})", value.text)
								}
								None => "null".to_string(),
							}
						}
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

/// The name of a field of an object type, as a symbol.
fn fields_of_sym(g: &Generator<'_>, ty: TypeId, name: &str) -> Option<fa_analyzer::ids::Sym> {
	g.types
		.fields_of(ty)
		.iter()
		.find(|field| g.types.syms.resolve(field.name) == name)
		.map(|field| field.name)
}
