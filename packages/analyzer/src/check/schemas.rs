//! `TypeFromSchema(schema)` (types brief §3.14, D48): the type a `Schema`
//! known at compile time describes, the inverse of `@typeToSchema`.
//!
//! ```fa
//! type Shouted = TypeFromSchema(@capitalizeLiterals(@typeToSchema("hello" | "world")))
//! ```
//!
//! The argument is a `Schema` (`builtins/schema.rs`) evaluated on the
//! interpreter. A `Schema.Named` gives its definition (named for messages
//! when it is an object type); a `Schema.Reference` must name an enclosing
//! `Schema.Named` (`TF040`), and is not supported yet as a recursive type
//! (`AN001`); the properties of an object have distinct names (`TF041`);
//! bits other than 8–128 or 32/64 are `TF042` (`min > max` is `Never`); an
//! unknown collection kind is `TF043`. Named unions and enumerations, whose
//! identity is `(the applying declaration, the schema)`, are not built yet
//! (`AN001`), nor is a `TypeFromSchema` inside a generic type function,
//! whose argument is known per application only.

use ethnum::I256;
use fa_interpreter::Value;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::types::{TF040, TF041, TF042, TF043, TY003};
use crate::ids::{NodeId, TypeId};
use crate::syntax::NodeList;
use crate::types::{CollKind, FieldEntry, FieldFlags, IntStorage};

impl Walker<'_> {
	/// `TypeFromSchema(schema)` in a type.
	pub(super) fn type_from_schema(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let schema = crate::builtins::schema::schema(&mut self.s.types);
		let argument = arguments.first().filter(|_| arguments.len() == 1);
		if let Some(argument) = argument {
			let actual = self.check(argument, Some(schema));
			self.sink(argument, actual, schema);
		}
		self.memo(n, |w| {
			let Some(argument) = argument else {
				let diagnostic = w.diagnostic(
					&TY003,
					n,
					"`TypeFromSchema` takes one `Schema` known at compile time, ex: `TypeFromSchema(@typeToSchema(User))`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if !w.rigid_params.is_empty() {
				return w.unsupported(n, "`TypeFromSchema` inside a generic type function");
			}
			let Some(value) = w.evaluate_at(argument, argument) else {
				return TypeId::UNKNOWN;
			};
			let mut named = Vec::new();
			w.schema_type(n, &value, &mut named, 0)
		})
	}

	/// The type a schema value describes; `named`: the enclosing `Schema.Named`.
	fn schema_type(
		&mut self,
		n: NodeId,
		value: &Value,
		named: &mut Vec<String>,
		depth: u32,
	) -> TypeId {
		if depth > 64 {
			return self.invalid_schema(n, &TF042, "this schema is nested too deeply");
		}
		let Value::Member(member) = value else {
			return self.invalid_schema(n, &TF042, "this value is not a `Schema`");
		};
		let kind = member.name.strip_prefix("Schema.").unwrap_or(&member.name);
		let field = |name: &str| match &member.data {
			Some(Value::Object(object)) => object.get(name).cloned(),
			_ => None,
		};
		let items = |name: &str| -> Vec<Value> {
			field(name)
				.and_then(|value| value.items().map(<[Value]>::to_vec))
				.unwrap_or_default()
		};
		match kind {
			"Boolean" => TypeId::BOOLEAN,
			"String" => TypeId::STRING,
			"Character" => TypeId::CHARACTER,
			"Percentage" => TypeId::PERCENTAGE,
			"BigInt" => TypeId::BIG_INT,
			"None" => TypeId::NONE,
			"Never" => TypeId::NEVER,
			"Void" => TypeId::VOID,
			"BooleanLiteral" => match field("value").and_then(|value| value.as_bool()) {
				Some(true) => TypeId::TRUE,
				Some(false) => TypeId::FALSE,
				None => self.invalid_schema(n, &TF042, "a `BooleanLiteral` has a `Boolean` value"),
			},
			"StringLiteral" => match field("value") {
				Some(Value::String(text)) => self.s.types.string_literal(&text),
				_ => self.invalid_schema(n, &TF042, "a `StringLiteral` has a `String` value"),
			},
			"IntegerLiteral" => match field("value").and_then(|value| value.as_int()) {
				Some(value) => {
					let storage = IntStorage::Default;
					self.s.types.int_range(value, value, storage)
				}
				None => self.invalid_schema(n, &TF042, "an `IntegerLiteral` has an integer value"),
			},
			"FloatLiteral" | "Float" => {
				let bits = field("bits").and_then(|bits| bits.as_int());
				match bits.map(|bits| bits.as_i64()) {
					Some(32) => TypeId::FLOAT32,
					Some(64) | None => TypeId::FLOAT,
					Some(other) => {
						let message = format!("a float has 32 or 64 bits, not {other}");
						self.invalid_schema(n, &TF042, &message)
					}
				}
			}
			"Integer" => self.schema_integer(n, &field),
			"Named" => {
				let Some(name) = field("name").and_then(|name| name.as_str().map(str::to_string))
				else {
					return self.invalid_schema(n, &TF042, "a `Named` schema has a `name`");
				};
				let Some(definition) = field("definition") else {
					return self.invalid_schema(n, &TF042, "a `Named` schema has a `definition`");
				};
				named.push(name.clone());
				let ty = self.schema_type(n, &definition, named, depth + 1);
				named.pop();
				self.s.types.name_type(ty, &name);
				ty
			}
			"Reference" => {
				let name = field("name")
					.and_then(|name| name.as_str().map(str::to_string))
					.unwrap_or_default();
				if named.contains(&name) {
					self.unsupported(n, "a recursive `Schema.Reference`")
				} else {
					let message = format!(
						"`Schema.Reference {{ name = \"{name}\" }}` refers to no enclosing `Schema.Named`"
					);
					self.invalid_schema(n, &TF040, &message)
				}
			}
			"Object" => {
				let mut fields: Vec<FieldEntry> = Vec::new();
				for property in items("properties") {
					let Value::Object(property) = property else {
						continue;
					};
					let Some(name) = property.get("name").and_then(|name| name.as_str()) else {
						continue;
					};
					let name = self.s.types.syms.intern(name);
					if fields.iter().any(|field| field.name == name) {
						let message = format!(
							"two properties are named `{}`",
							self.s.types.syms.resolve(name)
						);
						return self.invalid_schema(n, &TF041, &message);
					}
					let Some(definition) = property.get("definition").cloned() else {
						continue;
					};
					let ty = self.schema_type(n, &definition, named, depth + 1);
					let readonly = property
						.get("isReadonly")
						.and_then(Value::as_bool)
						.unwrap_or(false);
					fields.push(FieldEntry {
						name,
						ty,
						flags: if readonly {
							FieldFlags::READONLY
						} else {
							FieldFlags::empty()
						},
					});
				}
				self.s.types.object(&fields, None)
			}
			"Union" | "Tuple" => {
				let members: Vec<TypeId> = items("items")
					.iter()
					.map(|item| self.schema_type(n, item, named, depth + 1))
					.collect();
				if kind == "Union" {
					self.s.types.union(&members)
				} else {
					self.s.types.tuple(&members)
				}
			}
			"Collection" => {
				let item = match field("item") {
					Some(item) => self.schema_type(n, &item, named, depth + 1),
					None => return self.invalid_schema(n, &TF042, "a collection has an `item`"),
				};
				let key = match field("key") {
					Some(Value::None) | None => TypeId::NONE,
					Some(key) => self.schema_type(n, &key, named, depth + 1),
				};
				let kind_name = field("kind")
					.and_then(|kind| kind.as_str().map(str::to_string))
					.unwrap_or_default();
				let kind = match kind_name.as_str() {
					"Array" => CollKind::Array,
					"GrowingArray" => CollKind::GrowingArray,
					"Set" => CollKind::Set,
					"GrowingSet" => CollKind::GrowingSet,
					"Map" => CollKind::Map,
					"GrowingMap" => CollKind::GrowingMap,
					"Pool" => CollKind::Pool,
					"GrowingPool" => CollKind::GrowingPool,
					_ => {
						let message = format!("`{kind_name}` is not a collection kind");
						return self.invalid_schema(n, &TF043, &message);
					}
				};
				self.s.types.collection(kind, item, key)
			}
			"NamedUnion" | "Enum" | "Fields" | "Function" => {
				self.unsupported(n, &format!("`TypeFromSchema` of a `Schema.{kind}`"))
			}
			_ => {
				let message = format!("`{}` is not a member of `Schema`", member.name);
				self.invalid_schema(n, &TF042, &message)
			}
		}
	}

	/// `Schema.Integer { bits, signed, min, max }`.
	fn schema_integer(&mut self, n: NodeId, field: &dyn Fn(&str) -> Option<Value>) -> TypeId {
		let bits = field("bits")
			.and_then(|bits| bits.as_int())
			.map(|bits| bits.as_i64());
		let signed = field("signed")
			.and_then(|signed| signed.as_bool())
			.unwrap_or(true);
		let storage = match (bits, signed) {
			(Some(8), true) => IntStorage::I8,
			(Some(16), true) => IntStorage::I16,
			(Some(32), true) => IntStorage::I32,
			(Some(64), true) => IntStorage::Default,
			(Some(128), true) => IntStorage::I128,
			(Some(8), false) => IntStorage::U8,
			(Some(16), false) => IntStorage::U16,
			(Some(32), false) => IntStorage::U32,
			(Some(64), false) => IntStorage::U64,
			(Some(128), false) => IntStorage::U128,
			(bits, _) => {
				let message = format!(
					"an integer has 8, 16, 32, 64 or 128 bits, not {}",
					bits.map_or("none".to_string(), |bits| bits.to_string())
				);
				return self.invalid_schema(n, &TF042, &message);
			}
		};
		let range = storage.range();
		let bound = |name: &str, default: I256| {
			field(name)
				.and_then(|value| value.as_int())
				.unwrap_or(default)
				.clamp(range.lo, range.hi)
		};
		let (min, max) = (bound("min", range.lo), bound("max", range.hi));
		if min > max {
			return TypeId::NEVER;
		}
		self.s.types.int_range(min, max, storage)
	}

	/// An invalid schema (`TF040`–`TF043`): `UNKNOWN`.
	fn invalid_schema(
		&mut self,
		n: NodeId,
		code: crate::diagnostics::Code,
		message: &str,
	) -> TypeId {
		let diagnostic = self.diagnostic(code, n, format!("invalid schema: {message}"));
		self.report(diagnostic);
		TypeId::UNKNOWN
	}
}
