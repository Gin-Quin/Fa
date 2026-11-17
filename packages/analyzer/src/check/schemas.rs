//! `TypeFromSchema(schema)` (types brief §3.14, D48): the type a `Schema`
//! known at compile time describes, the inverse of `@typeToSchema`.
//!
//! ```fa
//! type Shouted = TypeFromSchema<@capitalizeLiterals(@typeToSchema("hello" | "world"))>
//! ```
//!
//! The argument is a `Schema` (`builtins/schema.rs`) evaluated on the
//! interpreter. A `Schema.Named` gives its definition (named for messages
//! when it is an object type); a `Schema.Reference` names an enclosing
//! `Schema.Named` (`TF040`): a recursive type (`TypeKind::Rec`, or the
//! named union itself). A `Schema.NamedUnion` or `Schema.Enum` gives a
//! nominal type, named by its `Schema.Named` (else by the applying
//! declaration), whose identity is the applying declaration and the schema
//! (equal schemas applied by the same declaration give the same type). A `Schema.Function`
//! gives a function type (no effects). The properties of an object, the
//! members of a union have distinct names (`TF041`); bits other than 8–128
//! or 32/64 are `TF042` (`min > max` is `Never`); an unknown collection kind
//! is `TF043`.
//!
//! In a generic type function, `TypeFromSchema` as its value (or as a result
//! of its logic, `type_logic.rs`) is built per application: its `@` calls
//! are not evaluated at the declaration (`deferred_comptime`), and each
//! application evaluates them with `@typeToSchema` of the bound types; an
//! error the code throws is `TF004` at the application.

use ethnum::I256;
use fa_interpreter::Value;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::types::{TF040, TF041, TF042, TF043, TY003};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{self, NodeList, View};
use crate::types::{
	Bindings, CollKind, FieldEntry, FieldFlags, IntStorage, NominalInfo, NominalKind,
};

pub(super) const LONE_SURROGATE_TYPE: &str =
	"a literal type cannot hold a lone surrogate: use `String`, or make the value well-formed";

impl Walker<'_> {
	/// Check the known payload at its written field, rather than blaming the
	/// whole constructor. Unknown runtime payloads are ordinary schema data.
	pub(super) fn check_literal_schema_constructor(&mut self, n: NodeId, member: TypeId) {
		use crate::types::TypeKind;
		let TypeKind::Member(nominal, index) = self.s.types.kind(member) else {
			return;
		};
		if self.schema_input_depth > 0
			|| crate::builtins::schema::MEMBERS.get(index as usize) != Some(&"StringLiteral")
		{
			return;
		}
		let schema = crate::builtins::schema::schema(&mut self.s.types);
		if !matches!(self.s.types.kind(schema), TypeKind::NamedUnion(owner, _) if owner == nominal)
		{
			return;
		}
		if let Some((field, Value::String(text))) = self.known_schema_field(n, "value", 0)
			&& !text.is_well_formed()
		{
			let diagnostic = self.diagnostic(&TF042, field, LONE_SURROGATE_TYPE);
			self.report(diagnostic);
			self.rejected_literal_schemas.insert(n);
		}
	}

	/// Follow the analyzer's resolved field sources, including spreads, rather
	/// than guessing which written initializer supplies the final value.
	fn known_schema_field(&mut self, n: NodeId, name: &str, depth: u32) -> Option<(NodeId, Value)> {
		use crate::analysis::FieldValue;
		if depth > 64 {
			return None;
		}
		let n = self.strip_groups(n);
		if let Some(Value::Object(object)) = self.a.constants.get(&n) {
			return object.get(name).cloned().map(|value| (n, value));
		}
		let source = self
			.a
			.objects
			.get(&n)?
			.fields
			.iter()
			.find(|(field, _)| self.s.types.syms.resolve(*field) == name)?
			.1;
		match source {
			FieldValue::Given(value) | FieldValue::Default(value) => {
				let value = self.strip_groups(value);
				let known = self.a.constants.get(&value).cloned().or_else(|| {
					if !self.is_constant_expression(value, 0) {
						return None;
					}
					// This is a known-data check, not a new compile-time
					// requirement on otherwise runtime schema construction.
					self.quiet += 1;
					let computed = self.evaluate_at(value, value);
					self.quiet -= 1;
					computed
				})?;
				Some((value, known))
			}
			FieldValue::Spread(value) => self.known_schema_field(value, name, depth + 1),
			FieldValue::Constant(index) => self
				.a
				.field_constants
				.get(index as usize)
				.cloned()
				.map(|value| (n, value)),
			FieldValue::None => None,
		}
	}

	/// Known schema construction is checked too; runtime schema data with an
	/// unknown payload remains legal. Do not turn ordinary malformed Strings
	/// into literal types, or diagnose them merely for being values.
	pub(super) fn check_literal_schema_value(&mut self, n: NodeId, value: &Value) {
		if self.schema_input_depth == 0
			&& malformed_literal_schema(value)
			&& !self.literal_schema_rejected(n)
		{
			let diagnostic = self.diagnostic(&TF042, n, LONE_SURROGATE_TYPE);
			self.report(diagnostic);
			self.rejected_literal_schemas.insert(n);
		}
	}

	/// Follow constant names as well as syntax so reconstructing a schema
	/// already rejected at its declaration produces no duplicate diagnostic.
	pub(super) fn literal_schema_rejected(&self, n: NodeId) -> bool {
		let mut pending = vec![n];
		let mut seen = rustc_hash::FxHashSet::default();
		while let Some(node) = pending.pop() {
			if !seen.insert(node) {
				continue;
			}
			if self.rejected_literal_schemas.contains(&node) {
				return true;
			}
			if let View::Name(_) = self.view(node)
				&& let crate::symbols::Resolution::Local(symbol) = self.a.resolution(node)
				&& (self.a.static_values.contains_key(&symbol)
					|| self.a.module_constants.contains_key(&symbol))
			{
				pending.push(self.a.symbols.symbol(symbol).decl_node);
			}
			pending.extend(syntax::children(&self.tree, node));
		}
		false
	}

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
		// the value of a generic type function: evaluated per application
		let deferred = !self.rigid_params.is_empty() && self.schema_logic.contains(&n);
		if let Some(argument) = argument {
			if deferred {
				self.deferred_comptime += 1;
			}
			self.schema_input_depth += 1;
			let actual = self.check(argument, Some(schema));
			self.sink(argument, actual, schema);
			self.schema_input_depth -= 1;
			if deferred {
				self.deferred_comptime -= 1;
			}
		}
		self.memo(n, |w| {
			let Some(argument) = argument else {
				let diagnostic = w.diagnostic(
					&TY003,
					n,
					"`TypeFromSchema` takes one `Schema` known at compile time, ex: `TypeFromSchema<@typeToSchema(User)>`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if deferred {
				return TypeId::UNKNOWN;
			}
			if !w.rigid_params.is_empty() {
				return w.unsupported(
					n,
					"`TypeFromSchema` inside a generic type function, but as its value",
				);
			}
			if w.literal_schema_rejected(argument) {
				return TypeId::UNKNOWN;
			}
			let Some(value) = w.evaluate_at(argument, argument) else {
				return TypeId::UNKNOWN;
			};
			let owner = w
				.decl_stack
				.last()
				.map(|&symbol| {
					w.s.types
						.syms
						.resolve(w.a.symbols.symbol(symbol).name)
						.to_string()
				})
				.unwrap_or_default();
			w.build_schema_type(n, owner, &value)
		})
	}

	/// The `TypeFromSchema` applications of a generic type declaration's
	/// value that are built per application: the value, or a result of its
	/// logic.
	pub(super) fn note_schema_logic(&mut self, value: NodeId) {
		let tree = std::sync::Arc::clone(&self.tree);
		match syntax::view(&tree, value) {
			View::Call { callee, .. } if matches!(syntax::view(&tree, callee), View::Name(name) if tree.name(name) == "TypeFromSchema") =>
			{
				self.schema_logic.insert(value);
			}
			View::If {
				then_body,
				else_body,
				..
			} => {
				self.note_schema_logic(then_body);
				if let Some(else_body) = else_body {
					self.note_schema_logic(else_body);
				}
			}
			View::When { arms, .. } => {
				for arm in arms.iter() {
					self.note_schema_logic(arm.value);
				}
			}
			View::Do(body) => self.note_schema_logic(body),
			View::Block { statements } => {
				if let Some(last) = statements.last() {
					self.note_schema_logic(last);
				}
			}
			_ => {}
		}
	}

	/// `TypeFromSchema` as the value of a generic type function, for the
	/// bindings of an application (`at`): `@typeToSchema` of the bound
	/// types, the other `@` calls evaluated again.
	pub(super) fn apply_type_from_schema(
		&mut self,
		at: NodeId,
		arguments: NodeList<'_>,
		bindings: &Bindings,
	) -> TypeId {
		let Some(argument) = arguments.first() else {
			return TypeId::UNKNOWN;
		};
		let tree = std::sync::Arc::clone(&self.tree);
		// the `@` calls of the argument
		let mut calls = Vec::new();
		let mut pending = vec![argument];
		while let Some(node) = pending.pop() {
			if let View::Call { callee, arguments } = syntax::view(&tree, node)
				&& let View::Name(name) = syntax::view(&tree, callee)
				&& tree.name(name).starts_with('@')
			{
				calls.push((node, tree.name(name) == "@typeToSchema", arguments.first()));
			}
			pending.extend(syntax::children(&tree, node));
		}
		let mut saved = Vec::new();
		for &(node, to_schema, first) in &calls {
			saved.push((node, self.a.constants.remove(&node)));
			self.comptime_evaluated.remove(&node);
			if to_schema && let Some(first) = first {
				let ty = self.type_of(first);
				let ty = self.s.types.substitute(ty, bindings);
				let schema = crate::comptime::schema::schema(&self.s.types, ty);
				let (defaults, order) = self.schema_defaults(ty);
				let schema = crate::comptime::schema::with_defaults(schema, &defaults, &order);
				self.a.constants.insert(node, schema);
			}
		}
		let saved_code = std::mem::replace(&mut self.in_type_function_code, true);
		let value = self.evaluate_at(argument, at);
		self.in_type_function_code = saved_code;
		for (node, previous) in saved {
			match previous {
				Some(previous) => self.a.constants.insert(node, previous),
				None => self.a.constants.remove(&node),
			};
		}
		let Some(value) = value else {
			return TypeId::UNKNOWN;
		};
		let owner = self
			.schema_identity
			.as_ref()
			.map(|(owner, _)| owner.clone())
			.unwrap_or_default();
		self.build_schema_type(at, owner, &value)
	}

	/// The type of a schema value applied by the declaration `owner`.
	fn build_schema_type(&mut self, n: NodeId, owner: String, value: &Value) -> TypeId {
		use std::hash::{Hash, Hasher};
		let mut hasher = rustc_hash::FxHasher::default();
		value.to_text().hash(&mut hasher);
		let saved = self.schema_identity.replace((owner, hasher.finish()));
		let mut named = Vec::new();
		let ty = self.schema_type(n, value, &mut named, 0);
		self.schema_identity = saved;
		ty
	}

	/// The identity of a nominal or recursive type named `name` built by
	/// `TypeFromSchema`: the applying declaration, the schema, the name.
	fn schema_key(&mut self, name: &str) -> crate::ids::DeclKey {
		let (owner, hash) = self.schema_identity.clone().unwrap_or_default();
		let module_name = self.s.module(self.file).to_string();
		let module = self.s.types.module(&module_name);
		let hash = format!("#{hash:016x}");
		self.s.types.decl_key(module, &[&owner, &hash, name], 0)
	}

	/// The type a schema value describes; `named`: the enclosing
	/// `Schema.Named`s, and what a reference to each is (its recursive
	/// reference, created at the first one).
	fn schema_type(
		&mut self,
		n: NodeId,
		value: &Value,
		named: &mut Vec<NamedSchema>,
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
			"Bytes" => TypeId::BYTES,
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
				Some(Value::String(text)) => match text.as_str() {
					Some(text) => self.s.types.string_literal(text),
					None => self.invalid_schema(n, &TF042, LONE_SURROGATE_TYPE),
				},
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
				// a named union or an enumeration: a nominal type of this name
				if let Value::Member(inner) = &definition {
					let inner_kind = inner.name.strip_prefix("Schema.").unwrap_or(&inner.name);
					if matches!(inner_kind, "NamedUnion" | "Enum") {
						return self.schema_nominal(n, &name, &definition, named, depth);
					}
				}
				let object =
					matches!(&definition, Value::Member(inner) if inner.name.ends_with("Object"));
				named.push(NamedSchema {
					name: name.clone(),
					reference: None,
					object,
				});
				let ty = self.schema_type(n, &definition, named, depth + 1);
				let entry = named.pop().expect("the entry pushed");
				// a recursive type: its references stand for it
				if let Some(reference) = entry.reference
					&& !self.s.types.is_recovering(ty)
				{
					self.s.types.set_rec_target(reference, ty);
				}
				self.s.types.name_type(ty, &name);
				ty
			}
			"Reference" => {
				let name = field("name")
					.and_then(|name| name.as_str().map(str::to_string))
					.unwrap_or_default();
				let Some(position) = named.iter().rposition(|entry| entry.name == name) else {
					let message = format!(
						"`Schema.Reference {{ name = \"{name}\" }}` refers to no enclosing `Schema.Named`"
					);
					return self.invalid_schema(n, &TF040, &message);
				};
				if let Some(reference) = named[position].reference {
					return reference;
				}
				let key = self.schema_key(&name);
				let object = named[position].object;
				let reference = self.s.types.recursive_reference(key, &name, &[], object);
				named[position].reference = Some(reference);
				reference
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
			"InlineArray" => {
				let Some(Value::Integer(capacity)) = field("capacity") else {
					return self.invalid_schema(
						n,
						&TF042,
						"an inline array needs an integer `capacity` from 0 through 4294967295",
					);
				};
				if capacity < I256::ZERO || capacity > I256::from(u32::MAX) {
					return self.invalid_schema(
						n,
						&TF042,
						"an inline-array capacity must be from 0 through 4294967295: choose a capacity in that range",
					);
				}
				let item = match field("item") {
					Some(item) => self.schema_type(n, &item, named, depth + 1),
					None => return self.invalid_schema(n, &TF042, "an inline array has an `item`"),
				};
				if item == TypeId::VOID {
					return self.invalid_schema(
						n,
						&TF042,
						"`Void` has no value: use an item type that can be stored in the inline array",
					);
				}
				let capacity = self.s.types.int_literal(capacity);
				self.s
					.types
					.collection(CollKind::InlineArray, item, capacity)
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
			// without a `Schema.Named`: named by the applying declaration
			"NamedUnion" | "Enum" => {
				let name = self
					.schema_identity
					.as_ref()
					.map(|(owner, _)| owner.clone())
					.filter(|owner| !owner.is_empty())
					.unwrap_or_else(|| kind.to_string());
				self.schema_nominal(n, &name, value, named, depth)
			}
			"Function" => {
				let parameters: Vec<TypeId> = items("parameters")
					.iter()
					.filter_map(|parameter| match parameter {
						Value::Object(parameter) => parameter.get("definition").cloned(),
						_ => None,
					})
					.collect::<Vec<Value>>()
					.iter()
					.map(|definition| self.schema_type(n, definition, named, depth + 1))
					.collect();
				let returns = match field("returns") {
					Some(returns) => self.schema_type(n, &returns, named, depth + 1),
					None => return self.invalid_schema(n, &TF042, "a function has `returns`"),
				};
				let throws = match field("throws") {
					Some(Value::None) | None => TypeId::NEVER,
					Some(throws) => self.schema_type(n, &throws, named, depth + 1),
				};
				let is_async = field("async")
					.and_then(|value| value.as_bool())
					.unwrap_or(false);
				let mut sig = self.s.types.simple_sig(&parameters, returns, throws);
				sig.is_async = is_async;
				self.s.types.function(sig)
			}
			// its flags in order, a group with its own flags (fields.rs)
			"Fields" => {
				let flags = items("flags");
				match self.schema_flags(n, &flags, 0) {
					Some(ty) if self.s.types.fields_width(ty) <= 128 => ty,
					Some(_) => self.invalid_schema(
						n,
						&TF042,
						"a `Fields` type has at most 128 flags, counting the members of its groups",
					),
					None => TypeId::UNKNOWN,
				}
			}
			_ => {
				let message = format!("`{}` is not a member of `Schema`", member.name);
				self.invalid_schema(n, &TF042, &message)
			}
		}
	}

	/// The `Fields` type of a list of `SchemaFlag`s: `None` when one is
	/// invalid (reported).
	fn schema_flags(&mut self, n: NodeId, flags: &[Value], depth: u32) -> Option<TypeId> {
		if depth > 16 {
			self.invalid_schema(
				n,
				&TF042,
				"the groups of a `Schema.Fields` are nested too deeply",
			);
			return None;
		}
		let mut keys: Vec<(String, TypeId)> = Vec::with_capacity(flags.len());
		for flag in flags {
			let Value::Object(flag) = flag else {
				self.invalid_schema(n, &TF042, "a flag is a `SchemaFlag`");
				return None;
			};
			let Some(name) = flag
				.get("name")
				.and_then(|name| name.as_str().map(str::to_string))
			else {
				self.invalid_schema(n, &TF042, "a `SchemaFlag` has a `name`");
				return None;
			};
			if keys.iter().any(|(other, _)| *other == name) {
				let message = format!("two flags are named `{name}`");
				self.invalid_schema(n, &TF041, &message);
				return None;
			}
			let members: Vec<Value> = flag
				.get("flags")
				.and_then(|members| members.items().map(<[Value]>::to_vec))
				.unwrap_or_default();
			let children = if members.is_empty() {
				TypeId::UNSET
			} else {
				self.schema_flags(n, &members, depth + 1)?
			};
			keys.push((name, children));
		}
		Some(self.s.types.fields_type(&keys))
	}

	/// A `Schema.Named` whose definition is a `Schema.NamedUnion` or a
	/// `Schema.Enum`: a nominal type named `name`, which its members' data
	/// may refer to.
	fn schema_nominal(
		&mut self,
		n: NodeId,
		name: &str,
		definition: &Value,
		named: &mut Vec<NamedSchema>,
		depth: u32,
	) -> TypeId {
		let Value::Member(member) = definition else {
			return TypeId::UNKNOWN;
		};
		let is_enum = member.name.ends_with("Enum");
		let field = |field: &str| match &member.data {
			Some(Value::Object(object)) => object.get(field).cloned(),
			_ => None,
		};
		let members: Vec<Value> = field("members")
			.and_then(|value| value.items().map(<[Value]>::to_vec))
			.unwrap_or_default();
		let mut names = Vec::new();
		let mut definitions = Vec::new();
		for (index, member) in members.iter().enumerate() {
			let Value::Object(member) = member else {
				continue;
			};
			let Some(member_name) = member.get("name").and_then(|name| name.as_str()) else {
				return self.invalid_schema(n, &TF042, "a member has a `name`");
			};
			let sym = self.s.types.syms.intern(member_name);
			if names.contains(&sym) {
				let message = format!("two members are named `{member_name}`");
				return self.invalid_schema(n, &TF041, &message);
			}
			names.push(sym);
			if is_enum {
				// a value other than the position is pinned (its literal payload)
				let value = member.get("value").and_then(Value::as_int);
				match value {
					Some(value) if value != I256::from(index as u64) => {
						if value < 0 {
							return self.invalid_schema(
								n,
								&TF042,
								"the value of a member is a non-negative integer",
							);
						}
						definitions.push(Some(Value::Integer(value)));
					}
					_ => definitions.push(None),
				}
			} else {
				definitions.push(match member.get("definition") {
					Some(Value::None) | None => None,
					Some(definition) => Some(definition.clone()),
				});
			}
		}
		let errors = field("errors")
			.and_then(|value| value.as_bool())
			.unwrap_or(false);
		let kind = if is_enum {
			NominalKind::Enum
		} else if errors {
			NominalKind::Errors
		} else {
			NominalKind::Union
		};
		let key = self.schema_key(name);
		let args = self.s.types.list(&[]);
		let member_list = self.s.types.intern_sym_list(&names);
		let nominal = self.s.types.intern_nominal(NominalInfo {
			key,
			args,
			kind,
			members: member_list,
		});
		let ty = self.s.types.nominal_type(nominal);
		// its members' data may refer to it
		named.push(NamedSchema {
			name: name.to_string(),
			reference: Some(ty),
			object: false,
		});
		let payloads: Vec<TypeId> = definitions
			.iter()
			.map(|definition| match definition {
				// a pinned value of an enumeration: its literal
				Some(Value::Integer(value)) if is_enum => {
					let storage = IntStorage::Default;
					self.s.types.int_range(*value, *value, storage)
				}
				Some(definition) => self.schema_type(n, definition, named, depth + 1),
				None => TypeId::UNSET,
			})
			.collect();
		named.pop();
		self.s.types.set_payloads(nominal, payloads);
		ty
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

/// An enclosing `Schema.Named` of the schema being built.
struct NamedSchema {
	name: String,
	/// What a `Schema.Reference` to it is: its recursive reference (created
	/// at the first one), or the named union itself.
	reference: Option<TypeId>,
	/// Whether its definition is an object type.
	object: bool,
}

/// Only schema literal payloads are restricted; strings elsewhere in schema
/// data (for example a default value) are not themselves literal types.
fn malformed_literal_schema(value: &Value) -> bool {
	let mut pending = vec![value];
	while let Some(value) = pending.pop() {
		match value {
			Value::Member(member) => {
				if member.name.as_ref() == "Schema.StringLiteral"
					&& let Some(Value::Object(data)) = &member.data
					&& let Some(Value::String(text)) = data.get("value")
					&& !text.is_well_formed()
				{
					return true;
				}
				pending.extend(member.data.iter());
			}
			Value::Object(object) => pending.extend(object.fields.iter().map(|(_, value)| value)),
			Value::Array(items) | Value::Tuple(items) | Value::Set(items) => {
				pending.extend(items.iter())
			}
			Value::Map(entries) => pending.extend(entries.iter().flat_map(|(k, v)| [k, v])),
			_ => {}
		}
	}
	false
}
