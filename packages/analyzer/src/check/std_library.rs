//! The modules of `std` (`builtins/std_library.rs`): `std.concurrency`,
//! `std.time`, `std.fs`, `std.json`.
//!
//! - **Visibility**: their names are not in the prelude. A file sees them
//!   through `use { Mutex } = std.concurrency` ([`StdState`]: the `use`s of
//!   the file are scanned once, so a type declared above the `use` sees its
//!   names too), or names them in full (`std.time.now()`,
//!   `std.concurrency.AtomicInteger`).
//! - **`use { … } = std.module`**: its members must exist (`E-DECL-1`).
//! - **Targets**: `File.read` doesn't exist on `web` (`AN004`).
//! - **`Json.parse(text)`** returns the expected type, which must be a JSON
//!   value type (`AN005`): booleans, numbers, strings, `none`, objects, arrays,
//!   maps with `String` keys, and anonymous unions of them. Without an
//!   expected type, it is `E-DECL-11`. `Json.stringify(value)` takes the
//!   same types.

use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::builtins::{self, std_library};
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::{AN004, AN005};
use crate::diagnostics::codes::types::TY008;
use crate::diagnostics::codes::values::{E_DECL_1, E_DECL_11};
use crate::ids::{BuiltinId, NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, View};
use crate::types::{CollKind, TypeKind};

/// The names of `std` a file brings with `use`.
#[derive(Default, Debug)]
pub(crate) struct StdState {
	/// The names, from the scan of the file, on the first lookup of a name of `std`.
	imports: Option<FxHashMap<String, BuiltinId>>,
}

impl Walker<'_> {
	/// The built-in a name designates: the prelude's, or a name of `std`
	/// brought by a `use` of this file.
	pub(super) fn lookup_builtin(&mut self, text: &str) -> Option<BuiltinId> {
		builtins::lookup(text).or_else(|| self.std_import(text))
	}

	/// Whether a name of `std` (`Mutex`) is brought by a `use` of this file;
	/// `true` for a name that is not one of `std`'s.
	pub(super) fn std_name_visible(&mut self, text: &str) -> bool {
		!std_library::gated(text) || self.std_import(text).is_some()
	}

	/// `E-DECL-1` for a name of `std` that no `use` brings: says which `use`
	/// brings it. `false` when `text` is not a name of `std`.
	pub(super) fn report_std_name(&mut self, n: NodeId, text: &str) -> bool {
		let Some(module) = std_library::MODULES
			.iter()
			.find(|module| module.members.iter().any(|(name, _)| *name == text))
		else {
			return false;
		};
		let message = format!(
			"`{text}` is a name of `std.{0}`: bring it with `use {{ {text} }} = std.{0}`, or write `std.{0}.{text}`",
			module.name
		);
		let diagnostic = self.diagnostic(&E_DECL_1, n, message);
		self.report(diagnostic);
		true
	}

	/// The name of `std` that a `use` of this file brings as `text` (the
	/// `use`s were found by the scan of the file, `scan.rs`).
	fn std_import(&mut self, text: &str) -> Option<BuiltinId> {
		if self.std_library.imports.is_none() {
			let imports = self
				.s
				.source_file(self.file)
				.scan
				.std_uses
				.iter()
				.map(|(name, id)| (name.to_string(), *id))
				.collect();
			self.std_library.imports = Some(imports);
		}
		self.std_library.imports.as_ref()?.get(text).copied()
	}

	/// `concurrency` when `value` is `std.concurrency` (any module name).
	fn std_module_name(&self, value: NodeId) -> Option<String> {
		let value = self.strip_groups(value);
		let View::Access { target, members } = self.view(value) else {
			return None;
		};
		let View::Name(root) = self.view(self.strip_groups(target)) else {
			return None;
		};
		if self.tree.name(root) != "std" || members.len() != 1 {
			return None;
		}
		match self.view(members.first()?) {
			View::Name(name) => Some(self.tree.name(name).to_string()),
			_ => None,
		}
	}

	/// The member a full path designates, `std.concurrency.Mutex` (`std` not
	/// being declared); sets the resolutions of its names.
	pub(super) fn std_qualified(&mut self, n: NodeId) -> Option<BuiltinId> {
		let n = self.strip_groups(n);
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, n) else {
			return None;
		};
		let root = self.strip_groups(target);
		let View::Name(root_name) = syntax::view(&tree, root) else {
			return None;
		};
		if tree.name(root_name) != "std" || members.len() != 2 || self.is_declared(root_name) {
			return None;
		}
		let names: Vec<NodeId> = members.iter().collect();
		let (View::Name(module), View::Name(name)) =
			(syntax::view(&tree, names[0]), syntax::view(&tree, names[1]))
		else {
			return None;
		};
		let module_text = tree.name(module).to_string();
		let found = std_library::member(&module_text, tree.name(name))?;
		if let Some(std) = builtins::by_path("std") {
			self.set_resolution(root, Resolution::Builtin(std));
		}
		if let Some(namespace) = builtins::by_path(&format!("std.{module_text}")) {
			self.set_resolution(names[0], Resolution::Builtin(namespace));
		}
		self.set_resolution(names[1], Resolution::Builtin(found));
		self.set_flag(n, crate::analysis::NodeFlags::TYPE_EXPR);
		Some(found)
	}

	/// Whether a `use` value is a module of `std` (`std` not being declared).
	pub(super) fn is_std_path(&self, value: NodeId) -> bool {
		let Some(module) = self.std_module_name(value) else {
			return false;
		};
		// `use c = std.c`: an alias (foreign.rs)
		if module == "c" {
			return false;
		}
		let sym = self.s.types.syms.get("std");
		sym.is_none_or(|sym| self.a.symbols.lookup(self.body.scope, sym).is_none())
	}

	/// `use { Mutex, Channel } = std.concurrency`: the module and its members exist.
	pub(super) fn check_std_use(&mut self, n: NodeId, pattern: NodeId, value: NodeId) -> TypeId {
		self.memo(n, |w| {
			let tree = Arc::clone(&w.tree);
			let value = w.strip_groups(value);
			if let View::Access { target, members } = syntax::view(&tree, value) {
				let root = w.strip_groups(target);
				if let Some(std) = builtins::by_path("std") {
					w.set_resolution(root, Resolution::Builtin(std));
				}
				w.set_type(root, TypeId::UNKNOWN);
				if let Some(member) = members.first()
					&& let Some(module) =
						builtins::by_path(&format!("std.{}", tree.text(member.index())))
				{
					w.set_resolution(member, Resolution::Builtin(module));
				}
			}
			let known = std_library::MODULES
				.iter()
				.map(|module| format!("`std.{}`", module.name))
				.collect::<Vec<_>>()
				.join(", ");
			let Some(module) = w
				.std_module_name(value)
				.and_then(|name| std_library::module(&name))
			else {
				let path: String = tree
					.text(value.index())
					.chars()
					.filter(|c| !c.is_whitespace())
					.collect();
				let message = format!(
					"`{path}` is not a module of the standard library the compiler knows: {known} and `std.c` are"
				);
				let diagnostic = w.diagnostic(&E_DECL_1, value, message);
				w.report(diagnostic);
				return TypeId::VOID;
			};
			let View::ObjectPattern { fields, .. } = syntax::view(&tree, pattern) else {
				let message = format!(
					"a module is destructured with braces, ex: `use {{ {} }} = std.{}`",
					module.members[0].0, module.name
				);
				let diagnostic = w.diagnostic(&TY008, pattern, message);
				w.report(diagnostic);
				return TypeId::VOID;
			};
			for field in fields.iter() {
				let text = tree.name(field.name);
				if std_library::member(module.name, text).is_none() {
					let message = format!(
						"`{text}` is not a member of `std.{}`: it has {}",
						module.name,
						module
							.members
							.iter()
							.map(|(name, _)| format!("`{name}`"))
							.collect::<Vec<_>>()
							.join(", ")
					);
					let diagnostic =
						w.diagnostic_at(&E_DECL_1, syntax::name_span(field.name), message);
					w.report(diagnostic);
				}
			}
			TypeId::VOID
		})
	}

	/// The type of a call of a built-in, `ret` by its signature: the
	/// expected type for `Json.parse`; checks the target (`AN004`) and the
	/// value of `Json.stringify`. Type part.
	pub(super) fn std_call_type(
		&mut self,
		n: NodeId,
		builtin: BuiltinId,
		ret: TypeId,
		expected: Option<TypeId>,
	) -> TypeId {
		self.check_std_target(n, builtin);
		match builtins::get(builtin).path {
			"Json.parse" => self.json_parse_type(n, expected),
			"Json.stringify" => {
				let value = self
					.a
					.calls
					.get(&n)
					.and_then(|call| call.arguments.first().cloned());
				if let Some(crate::analysis::Argument::Given(value)) = value {
					let ty = self.type_of(value);
					self.check_json_type(value, ty, "Json.stringify");
				}
				ret
			}
			_ => ret,
		}
	}

	/// A call of a built-in of `std` that doesn't exist on the target of the
	/// analysis (`File.read` on `web`): `AN004`. Type part.
	pub(super) fn check_std_target(&mut self, n: NodeId, builtin: BuiltinId) {
		let path = builtins::get(builtin).path;
		let missing = std_library::missing_on(path);
		if missing.is_empty() {
			return;
		}
		self.a.reads_target = true;
		let target = match self.s.freestanding() {
			true => "native-freestanding".to_string(),
			false => self.s.target().to_string(),
		};
		if missing.contains(&target.as_str()) {
			let message = if self.s.freestanding() {
				format!(
					"`{path}` doesn't exist without a C library (`native-freestanding`): build for `native`"
				)
			} else {
				format!(
					"`{path}` doesn't exist on target `{target}`: branch with `@if target != \"{target}\"`, or build for another target"
				)
			};
			let diagnostic = self.diagnostic(&AN004, n, message);
			self.report(diagnostic);
		}
	}

	/// The type `Json.parse(text)` returns: the expected type, which must be
	/// a JSON value type. Type part.
	pub(super) fn json_parse_type(&mut self, n: NodeId, expected: Option<TypeId>) -> TypeId {
		let Some(expected) = expected.filter(|ty| !ty.is_unset() && *ty != TypeId::UNKNOWN) else {
			let diagnostic = self.diagnostic(
				&E_DECL_11,
				n,
				"`Json.parse` reads the type you expect: write it, ex: `let user: User = Json.parse(text)`",
			);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		};
		self.check_json_type(n, expected, "Json.parse");
		self.note_json_defaults(expected, &mut Vec::new());
		expected
	}

	/// The default values of the fields of the object types declared in
	/// other files that `Json.parse` reads (`Analysis::json_defaults`): a
	/// missing field takes its default value.
	fn note_json_defaults(&mut self, ty: TypeId, seen: &mut Vec<TypeId>) {
		if seen.contains(&ty) || seen.len() > 64 {
			return;
		}
		seen.push(ty);
		match self.s.types.kind(ty) {
			TypeKind::Object(shape) => {
				let fields: Vec<TypeId> = self
					.s
					.types
					.shape_fields(shape)
					.iter()
					.map(|field| field.ty)
					.collect();
				if self.declared_fields(ty).is_none()
					&& let Some(defaults) = self.external_field_defaults(ty)
				{
					for (name, value) in defaults {
						if let Some(value) = value
							&& !self.a.json_defaults.contains_key(&(ty, name))
						{
							let index = self.a.field_constants.len() as u32;
							self.a.field_constants.push(value);
							self.a.json_defaults.insert((ty, name), index);
						}
					}
				}
				for field in fields {
					self.note_json_defaults(field, seen);
				}
			}
			TypeKind::Rec(_) => {
				if let Some(target) = self.s.types.rec_target(ty) {
					self.note_json_defaults(target, seen);
				}
			}
			TypeKind::Union(list) | TypeKind::Tuple(list) => {
				for member in self.s.types.list_items(list).to_vec() {
					self.note_json_defaults(member, seen);
				}
			}
			TypeKind::Collection(_, item, _) => self.note_json_defaults(item, seen),
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				let count = self
					.s
					.types
					.sym_list(self.s.types.nominal(nominal).members)
					.len() as u16;
				for index in 0..count {
					let payload = self.s.types.payload(nominal, index);
					if !payload.is_unset() {
						self.note_json_defaults(payload, seen);
					}
				}
			}
			_ => {}
		}
	}

	/// `AN005` when values of `ty` have no JSON form.
	pub(super) fn check_json_type(&mut self, n: NodeId, ty: TypeId, what: &str) {
		if let Some(part) = self.not_json(ty, 0) {
			let message = if part == ty {
				format!(
					"`{what}` reads and writes JSON values, and a `{}` is not one",
					self.display(ty)
				)
			} else {
				format!(
					"`{what}` reads and writes JSON values, and a `{}` (in `{}`) is not one",
					self.display(part),
					self.display(ty)
				)
			};
			let diagnostic = self.diagnostic(&AN005, n, message);
			self.report(diagnostic);
		}
	}

	/// The part of `ty` that has no JSON form, if any.
	fn not_json(&mut self, ty: TypeId, depth: u32) -> Option<TypeId> {
		if depth > 32 || self.s.types.is_recovering(ty) {
			return None;
		}
		match self.s.types.kind(ty) {
			TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::None
			| TypeKind::Rec(_) => None,
			TypeKind::Object(shape) => {
				let fields: Vec<TypeId> = self
					.s
					.types
					.shape_fields(shape)
					.iter()
					.map(|field| field.ty)
					.collect();
				fields
					.into_iter()
					.find_map(|field| self.not_json(field, depth + 1))
			}
			TypeKind::Union(list) => {
				let members = self.s.types.list_items(list).to_vec();
				members
					.into_iter()
					.find_map(|member| self.not_json(member, depth + 1))
			}
			// a `Union` type: its members tagged by `"type"` (design/decisions.md,
			// "JSON form of named union members")
			// an enumeration: its pinned values, else its member names
			TypeKind::Enum(..) | TypeKind::EnumMember(..) => None,
			// a set of flags: the array of their paths (fields.rs)
			TypeKind::Fields(_) => None,
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _)
				if matches!(
					self.s.types.nominal(nominal).kind,
					crate::types::NominalKind::Union | crate::types::NominalKind::Errors
				) =>
			{
				let count = self
					.s
					.types
					.sym_list(self.s.types.nominal(nominal).members)
					.len() as u16;
				let payloads: Vec<TypeId> = (0..count)
					.map(|index| self.s.types.payload(nominal, index))
					.filter(|payload| !payload.is_unset())
					.collect();
				payloads
					.into_iter()
					.find_map(|payload| self.not_json(payload, depth + 1))
			}
			TypeKind::Collection(
				CollKind::Array | CollKind::InlineArray | CollKind::GrowingArray,
				item,
				_,
			) => self.not_json(item, depth + 1),
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, value, key) => {
				if !matches!(self.s.types.kind(key), TypeKind::String) {
					return Some(ty);
				}
				self.not_json(value, depth + 1)
			}
			_ => Some(ty),
		}
	}
}
