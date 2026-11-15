//! Foreign code (advanced brief §13): foreign files, `@extern` functions,
//! `Pointer(T)`, `CString`, `CStruct`, `CArray`, the types of `std.c`, the
//! passing rules of a C call and `destroy`.
//!
//! - A file whose module ends with `.foreign` (`Database.foreign.fa`) is a
//!   foreign file: only there can `@extern` be declared and called
//!   (`FFI001`), `Pointer` and `CString` be used (`FFI002`), and a type
//!   declare `destroy` (`FFI003`).
//! - Another file can't use a value whose public type contains `Pointer` or
//!   `CString` (`FFI002`, D29): the flag `HAS_FOREIGN` of its type. A
//!   private field doesn't count, since it is not part of the shape.
//! - `function name: (…) => R = @extern("symbol")`: the declared function
//!   type, recorded in `Analysis::externs`; a call is recorded in
//!   `Analysis::extern_calls`, its converted arguments in
//!   `Analysis::foreign_args` (§13.3, D50).
//! - `use c = std.c`: `c.Int`, `c.SizeT`… are the integer types of the
//!   target's C (LP64 on native and JavaScript hosts, ILP32 on wasm).
//! - A type with `destroy` is a [`NominalKind::Resource`]: move-only.

use std::sync::Arc;

use rustc_hash::FxHashSet;

use crate::analysis::{ExternCall, ForeignArg};
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{
	FFI001, FFI002, FFI003, FFI005, FFI006, FFI008, FFI009, FFI010, FFI011,
};
use crate::diagnostics::codes::types::{TY003, TY007};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, Name, NodeList, View};
use crate::types::{CollKind, FnSig, PathRoot, TypeFlags, TypeKind};

/// The foreign-code state of a walk.
#[derive(Default, Debug)]
pub(crate) struct ForeignState {
	/// The aliases of `std.c` (`use c = std.c`).
	pub std_c: FxHashSet<SymbolId>,
	/// The nodes already reported (`FFI002` once per node).
	pub reported: FxHashSet<NodeId>,
}

/// The integer types of `std.c`, as `(name, bits, signed)` for an LP64
/// target; `Long`, `UnsignedLong`, `SizeT`, `SSizeT`, `PtrDiffT`,
/// `IntPtrT` and `UIntPtrT` have the size of a pointer.
const C_INTEGERS: [(&str, u32, bool); 18] = [
	("Char", 8, true),
	("SignedChar", 8, true),
	("UnsignedChar", 8, false),
	("Short", 16, true),
	("UnsignedShort", 16, false),
	("Int", 32, true),
	("UnsignedInt", 32, false),
	("Long", 0, true),
	("UnsignedLong", 0, false),
	("LongLong", 64, true),
	("UnsignedLongLong", 64, false),
	("SizeT", 0, false),
	("SSizeT", 0, true),
	("PtrDiffT", 0, true),
	("IntPtrT", 0, true),
	("UIntPtrT", 0, false),
	("WChar", 32, true),
	("Bool", 1, false),
];

impl Walker<'_> {
	/// Whether the file being analyzed is a foreign file (`*.foreign.fa`).
	pub(crate) fn in_foreign_file(&self) -> bool {
		self.s.module(self.file).ends_with(".foreign")
	}

	/// `FFI002` outside foreign files: `what` (`Pointer`, `CString`) is used.
	fn require_foreign_file(&mut self, n: NodeId, what: &str) {
		if self.in_foreign_file() || !self.foreign.reported.insert(n) {
			return;
		}
		let message = format!(
			"`{what}` can only be used in a foreign file (`*.foreign.fa`): bind the C library there, and export a safe interface"
		);
		let diagnostic = self.diagnostic(&FFI002, n, message);
		self.report(diagnostic);
	}

	// ── types ────────────────────────────────────────────────────────────

	/// `CString`, and `Pointer`/`CArray`/`CStruct` without their arguments.
	pub(super) fn foreign_type_name(&mut self, n: NodeId, text: &str) -> Option<TypeId> {
		match text {
			// `double`, as the table of C types writes it
			"Float64" => Some(TypeId::FLOAT),
			"CString" => {
				self.require_foreign_file(n, "CString");
				Some(self.s.types.intern(TypeKind::CString))
			}
			"Pointer" | "CArray" => {
				let example = if text == "Pointer" {
					"Pointer<Int32>"
				} else {
					"CArray<Float32, 3>"
				};
				let message = format!("`{text}` needs its arguments, ex: `{example}`");
				let diagnostic = self.diagnostic(&TY003, n, message);
				self.report(diagnostic);
				Some(TypeId::UNKNOWN)
			}
			_ => None,
		}
	}

	/// `Pointer.allocate(value)` (foreign files only): the value moves into a
	/// new block of memory, and the call is a `Pointer(T)` to it, `T` the
	/// value's type (or the target of the expected pointer). `None` for
	/// another callee.
	pub(super) fn check_pointer_allocate(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: crate::syntax::NodeList<'_>,
		expected: Option<TypeId>,
	) -> Option<TypeId> {
		let tree = std::sync::Arc::clone(&self.tree);
		let View::Access { target, members } = crate::syntax::view(&tree, callee) else {
			return None;
		};
		let member = members.first()?;
		if members.len() != 1 || tree.text(member.index()) != "allocate" {
			return None;
		}
		let View::Name(name) = crate::syntax::view(&tree, target) else {
			return None;
		};
		if tree.name(name) != "Pointer" {
			return None;
		}
		let sym = self.s.types.syms.intern("Pointer");
		if self.a.symbols.lookup(self.body.scope, sym).is_some() {
			return None;
		}
		self.require_foreign_file(target, "Pointer");
		let allocate = crate::builtins::by_path("Pointer.allocate")?;
		self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
		let expected_target = expected.and_then(|expected| self.s.types.pointer_target(expected));
		let values: Vec<NodeId> = arguments.iter().collect();
		let [value] = values.as_slice() else {
			for &value in &values {
				self.check(value, None);
			}
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::values::E_TYPE_1,
				n,
				"`Pointer.allocate` takes the value to move into the new block: `Pointer.allocate(value)`",
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		let value = *value;
		let actual = self.check(value, expected_target);
		if let Some(target_ty) = expected_target {
			self.sink(value, actual, target_ty);
		}
		self.note_copy_site(value);
		let pointed = expected_target.unwrap_or_else(|| self.s.types.widen(actual));
		self.memo(member, |w| {
			w.set_resolution(member, Resolution::Builtin(allocate));
			TypeId::UNKNOWN
		});
		self.memo(callee, |_| TypeId::UNKNOWN);
		let ty = self.memo(n, |w| {
			w.a.calls.insert(
				n,
				crate::analysis::CallInfo {
					callee: crate::analysis::Callee::Builtin(allocate),
					arguments: vec![crate::analysis::Argument::Given(value)],
				},
			);
			w.s.types.pointer(pointed)
		});
		Some(ty)
	}

	/// `[a, b, c]` where a `CArray(T, n)` is expected: `n` items of type `T`
	/// (`E-TYPE-1` for another count, or a spread).
	pub(super) fn check_c_array_literal(
		&mut self,
		n: NodeId,
		items: crate::syntax::NodeList<'_>,
		expected: TypeId,
		item: TypeId,
		count: u32,
	) -> TypeId {
		let mut given = 0u32;
		let mut spreads = false;
		for value in items.iter() {
			if let View::Spread(spread) = self.view(value) {
				spreads = true;
				self.check(spread, None);
				let diagnostic = self.diagnostic(
					&crate::diagnostics::codes::values::E_TYPE_1,
					value,
					"a `CArray` is written with its items, without spreads",
				);
				self.report(diagnostic);
				continue;
			}
			let ty = self.check(value, Some(item));
			self.sink(value, ty, item);
			given += 1;
		}
		if given != count && !spreads {
			let message = format!(
				"a `{}` holds {count} items, not {given}",
				self.display(expected)
			);
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::values::E_TYPE_1, n, message);
			self.report(diagnostic);
		}
		self.memo(n, |_| expected)
	}

	/// `Pointer(T)` and `CArray(T, n)`.
	pub(super) fn foreign_type_application(
		&mut self,
		n: NodeId,
		callee: NodeId,
		text: &str,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, crate::analysis::NodeFlags::TYPE_EXPR);
		let tree = Arc::clone(&self.tree);
		let first = arguments.first();
		let item = first.map(|first| match syntax::view(&tree, first) {
			// `Pointer(Void)`: C's `void *`
			View::Name(name) if tree.name(name) == "Void" && !self.is_declared(name) => {
				TypeId::VOID
			}
			_ => self.check_type_expr(first),
		});
		if text == "Pointer" {
			self.require_foreign_file(n, "Pointer");
			return self.memo(n, |w| match (item, arguments.len()) {
				(Some(item), 1) if !w.s.types.is_recovering(item) => w.s.types.pointer(item),
				(Some(_), 1) => TypeId::UNKNOWN,
				_ => {
					let diagnostic = w.diagnostic(
						&TY003,
						n,
						"`Pointer` takes the type it points to, ex: `Pointer<Int32>`",
					);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}
			});
		}
		let count = arguments
			.get(1)
			.and_then(|count| self.constant_integer(count));
		self.memo(n, |w| {
			let (Some(item), Some(count)) = (item, count) else {
				let diagnostic = w.diagnostic(
					&TY003,
					n,
					"`CArray` takes the type of its items and their count, ex: `CArray<Float32, 3>`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if w.s.types.is_recovering(item) {
				return TypeId::UNKNOWN;
			}
			let Some(count) = u32::try_from(count).ok().filter(|&count| count > 0) else {
				let diagnostic = w.diagnostic(
					&TY003,
					n,
					"the count of a `CArray` is a positive integer known at compile time",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			w.s.types.c_array(item, count)
		})
	}

	/// `use c = std.c`: declares the alias. `None` when the value is not `std.c`.
	pub(super) fn check_std_c_use(
		&mut self,
		n: NodeId,
		name: Name,
		value: NodeId,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, value) else {
			return None;
		};
		let View::Name(root) = syntax::view(&tree, target) else {
			return None;
		};
		if tree.name(root) != "std" || self.is_declared(root) || members.len() != 1 {
			return None;
		}
		let member = members.first()?;
		let View::Name(module) = syntax::view(&tree, member) else {
			return None;
		};
		if tree.name(module) != "c" {
			return None;
		}
		Some(self.memo(n, |w| {
			if !name.is_empty() {
				let symbol = w.declare(n, name, SymbolKind::NameAlias);
				let data = w.a.symbols.symbol_mut(symbol);
				data.ty = TypeId::UNKNOWN;
				data.state = DeclState::Done;
				w.foreign.std_c.insert(symbol);
			}
			TypeId::VOID
		}))
	}

	/// `c.Int`: a type of `std.c` through its alias. `None` when the target
	/// is not an alias of `std.c`.
	pub(super) fn std_c_type_path(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, target) else {
			return None;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		if !self.foreign.std_c.contains(&symbol) {
			return None;
		}
		self.set_resolution(target, Resolution::Local(symbol));
		Some(self.memo(n, |w| {
			let member = members.first().filter(|_| members.len() == 1);
			let text = member.map(|member| w.tree.text(member.index()).to_string());
			match text.as_deref().and_then(|text| w.std_c_type(text)) {
				Some(ty) => ty,
				None => {
					let shown = text.unwrap_or_default();
					let message = format!(
						"`std.c` has no type `{shown}`: its types are `Char`, `Int`, `Long`, `SizeT`, `Float`, `Double`…"
					);
					let diagnostic = w.diagnostic(&TY007, n, message);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}
			}
		}))
	}

	/// A type of `std.c`, sized for the target.
	fn std_c_type(&mut self, text: &str) -> Option<TypeId> {
		match text {
			"Float" => return Some(TypeId::FLOAT32),
			"Double" => return Some(TypeId::FLOAT),
			_ => {}
		}
		let &(_, bits, signed) = C_INTEGERS.iter().find(|(name, ..)| *name == text)?;
		if bits == 1 {
			return Some(TypeId::BOOLEAN);
		}
		let pointer_bits = if self.s.target() == "wasm" { 32 } else { 64 };
		Some(
			match (if bits == 0 { pointer_bits } else { bits }, signed) {
				(8, true) => TypeId::INT8,
				(8, false) => TypeId::UINT8,
				(16, true) => TypeId::INT16,
				(16, false) => TypeId::UINT16,
				(32, true) => TypeId::INT32,
				(32, false) => TypeId::UINT32,
				(_, true) => TypeId::INT64,
				(_, false) => TypeId::UINT64,
			},
		)
	}

	/// Whether a type is a C type: what a `CStruct` field or a C parameter can be.
	fn is_c_type(&self, ty: TypeId) -> bool {
		let types = &self.s.types;
		match types.kind(ty) {
			TypeKind::Int(id) => {
				let int = types.int(id);
				int.is_full()
					&& ty != TypeId::INTEGER
					&& ty != TypeId::INT128
					&& ty != TypeId::UINT128
			}
			TypeKind::Float(_) | TypeKind::Boolean | TypeKind::Void => true,
			TypeKind::Pointer(_) | TypeKind::CString | TypeKind::CArray(..) => true,
			TypeKind::Object(shape) => types.shape(shape).nominal.is_some_and(|nominal| {
				types.nominal(nominal).kind == crate::types::NominalKind::CStruct
			}),
			TypeKind::Union(_) => {
				types.pointer_target(ty).is_some() || {
					// `CString?`
					let (inner, had_none) = self.strip_none_type(ty);
					had_none && types.kind(inner) == TypeKind::CString
				}
			}
			TypeKind::Unknown => true,
			_ => false,
		}
	}

	/// `T?` without its `none`.
	fn strip_none_type(&self, ty: TypeId) -> (TypeId, bool) {
		match self.s.types.kind(ty) {
			TypeKind::Union(list) => {
				let items = self.s.types.list_items(list);
				if items.len() == 2 && items.contains(&TypeId::NONE) {
					let other = if items[0] == TypeId::NONE {
						items[1]
					} else {
						items[0]
					};
					(other, true)
				} else {
					(ty, false)
				}
			}
			_ => (ty, false),
		}
	}

	/// `FFI009`: the fields of a `CStruct` have C types.
	pub(super) fn check_c_struct_fields(&mut self, fields: &[crate::analysis::FieldDecl]) {
		for field in fields {
			if self.s.types.is_recovering(field.ty) {
				continue;
			}
			let pointer = self
				.s
				.types
				.flags(field.ty)
				.contains(TypeFlags::HAS_FOREIGN);
			if !self.is_c_type(field.ty) || (pointer && !self.in_foreign_file()) {
				let message = format!(
					"a `CStruct` field has a C type (numbers, `std.c` types, `CStruct`, `CArray`, and `Pointer` in foreign files), not `{}`",
					self.display(field.ty)
				);
				let diagnostic = self.diagnostic(&FFI009, field.node, message);
				self.report(diagnostic);
			}
		}
	}

	// ── `@extern` ────────────────────────────────────────────────────────

	/// Whether a function value is `@extern("symbol")`.
	pub(super) fn is_extern_value(&self, value: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(value) else {
			return false;
		};
		matches!(self.view(callee), View::Name(name) if self.tree.name(name) == "@extern")
	}

	/// `function name: (…) => R = @extern("symbol")`: the declared type.
	pub(super) fn check_extern_decl(
		&mut self,
		n: NodeId,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		let symbol = self.a.symbols.symbol_of_decl(n);
		let declared = annotation.map(|annotation| self.check_type_expr(annotation));
		let tree = Arc::clone(&self.tree);
		let View::Call { callee, arguments } = syntax::view(&tree, value) else {
			return TypeId::VOID;
		};
		self.set_flag(callee, crate::analysis::NodeFlags::TYPE_EXPR);
		let name = arguments
			.first()
			.filter(|_| arguments.len() == 1)
			.and_then(|first| match syntax::view(&tree, first) {
				View::String(text) => Some(text.to_string()),
				_ => None,
			});
		let ty = self.memo(value, |w| {
			if !w.in_foreign_file() {
				let diagnostic = w.diagnostic(
					&FFI001,
					n,
					"C functions can only be declared in a foreign file (`*.foreign.fa`): move this declaration into one",
				);
				w.report(diagnostic);
			}
			let Some(name) = &name else {
				let diagnostic = w.diagnostic(
					&FFI001,
					value,
					"`@extern` takes the name of the C symbol, a string literal, ex: `@extern(\"strlen\")`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			let Some(declared) = declared else {
				let diagnostic = w.diagnostic(
					&FFI001,
					n,
					"a C function declares its function type, ex: `function length: (text: CString) => c.SizeT = @extern(\"strlen\")`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if w.s.types.is_recovering(declared) {
				return TypeId::UNKNOWN;
			}
			let Some(sig) = w.signature_of(declared) else {
				let message = format!("`{}` is not a function type", w.display(declared));
				let diagnostic = w.diagnostic(&FFI001, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if sig.throws != TypeId::NEVER || !w.s.types.requirements(sig.requires).is_empty() {
				let diagnostic = w.diagnostic(
					&FFI005,
					annotation.unwrap_or(n),
					"a C function can't declare `throws` or `requires`: C can't propagate a Fa error",
				);
				w.report(diagnostic);
			}
			w.check_c_signature(annotation.unwrap_or(n), sig);
			w.a.externs.insert(n, name.clone());
			declared
		});
		if let Some(symbol) = symbol {
			self.a.symbols.symbol_mut(symbol).ty = ty;
		}
		TypeId::VOID
	}

	/// The parameters and the result of a C function have C types (a
	/// function type for a callback, whose own parameters have C types).
	fn check_c_signature(&mut self, at: NodeId, sig: FnSig) {
		let params: Vec<TypeId> = self
			.s
			.types
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		for ty in params.into_iter().chain([sig.ret]) {
			let ok = match self.s.types.kind(ty) {
				TypeKind::Function(inner) => {
					let inner = self.s.types.sig(inner);
					let inner_params: Vec<TypeId> = self
						.s
						.types
						.params(inner.params)
						.iter()
						.map(|param| param.ty)
						.collect();
					inner_params
						.into_iter()
						.chain([inner.ret])
						.all(|ty| self.is_c_type(ty))
				}
				_ => self.is_c_type(ty),
			};
			if !ok {
				let message = format!(
					"a C function takes and returns C types (numbers, `std.c` types, `Pointer`, `CString`, `CStruct`, functions of C types), not `{}`",
					self.display(ty)
				);
				let diagnostic = self.diagnostic(&FFI009, at, message);
				self.report(diagnostic);
			}
		}
	}

	/// The `@extern` declaration a name or a member access designates.
	pub(super) fn extern_of(&self, callee: NodeId) -> Option<ExternCall> {
		let node = match self.view(callee) {
			View::Access { members, .. } => members.last()?,
			View::Group(inner) => return self.extern_of(inner),
			_ => callee,
		};
		let Resolution::Local(symbol) = self.a.resolution(node) else {
			return None;
		};
		let data = self.a.symbols.symbol(symbol);
		if data.kind != SymbolKind::Function {
			return None;
		}
		if let Some(name) = self.a.externs.get(&data.decl_node) {
			return Some(ExternCall {
				file: self.file,
				decl: data.decl_node,
				symbol: name.clone(),
			});
		}
		// a function of another file: its interface (check/interface.rs)
		let (module, decl) = self.proxy_decl(symbol)?;
		let name = self.s.interface(module)?.externs.get(&decl)?;
		Some(ExternCall {
			file: self.s.file_of(module)?,
			decl,
			symbol: name.clone(),
		})
	}

	/// A call of an `@extern` function: recorded; `FFI001` outside foreign
	/// files, `FFI008` on a target that can't call C.
	pub(super) fn note_extern_call(&mut self, n: NodeId, callee: NodeId) -> bool {
		let Some(call) = self.extern_of(callee) else {
			return false;
		};
		if self.a.extern_calls.contains_key(&n) {
			return true;
		}
		self.a.reads_target = true;
		if !self.in_foreign_file() {
			let diagnostic = self.diagnostic(
				&FFI001,
				callee,
				"C functions can only be called from a foreign file (`*.foreign.fa`): call them through the safe interface it exports",
			);
			self.report(diagnostic);
		}
		let target = self.s.target().to_string();
		if matches!(target.as_str(), "web" | "node") {
			let message = format!(
				"C functions can't be called on target `{target}`: branch with `@if target` around the code that uses the binding"
			);
			let diagnostic = self.diagnostic(&FFI008, n, message);
			self.report(diagnostic);
		}
		self.a.extern_calls.insert(n, call);
		true
	}

	// ── passing values to C ──────────────────────────────────────────────

	/// An argument of a call of a C function (advanced brief §13.3, D50).
	/// Returns its type; the conversion goes to `Analysis::foreign_args`.
	pub(super) fn check_c_argument(
		&mut self,
		value: NodeId,
		expected: TypeId,
		mutated: bool,
	) -> TypeId {
		let (param, optional) = self.strip_none_type(expected);
		let kind = self.s.types.kind(param);
		match kind {
			TypeKind::CString => {
				let actual = self.check(value, Some(TypeId::STRING));
				if matches!(
					self.s.types.kind(actual),
					TypeKind::String | TypeKind::StringLit(_)
				) {
					self.a.foreign_args.insert(value, ForeignArg::CString);
				} else {
					self.sink(value, actual, expected);
				}
				actual
			}
			TypeKind::Pointer(target) => {
				let actual = self.check(value, None);
				if self.s.types.is_recovering(actual) {
					return actual;
				}
				if self.s.types.satisfies(actual, expected)
					|| (optional && actual == TypeId::NONE)
					|| (target == TypeId::VOID && self.s.types.pointer_target(actual).is_some())
				{
					return actual;
				}
				let item = match self.s.types.kind(actual) {
					TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => {
						Some(item)
					}
					_ => None,
				};
				let conversion = match item {
					Some(item)
						if target == TypeId::VOID || self.s.types.satisfies(item, target) =>
					{
						ForeignArg::Array { mutated }
					}
					_ if target == TypeId::VOID || self.s.types.satisfies(actual, target) => {
						ForeignArg::Address { mutated }
					}
					_ => {
						self.sink(value, actual, expected);
						return actual;
					}
				};
				if mutated && !self.is_mutable_place(value) {
					let message = format!(
						"C writes through this parameter (`mutates`): pass a `mutable` value, not `{}`",
						self.tree.text(value.index())
					);
					let diagnostic = self.diagnostic(&FFI010, value, message);
					self.report(diagnostic);
				}
				self.a.foreign_args.insert(value, conversion);
				actual
			}
			TypeKind::Function(_) => {
				let actual = self.check(value, Some(expected));
				// C converts pointers: a callback may take `Pointer(Int32)`
				// where C gives a `void *`
				if !self.c_callback_compatible(actual, expected) {
					self.sink(value, actual, expected);
				}
				self.check_callback(value);
				actual
			}
			_ => {
				let actual = self.check(value, Some(expected));
				self.sink(value, actual, expected);
				actual
			}
		}
	}

	/// Whether a function satisfies a C function pointer type, pointer
	/// parameters being interchangeable (C converts `void *` implicitly).
	fn c_callback_compatible(&mut self, actual: TypeId, expected: TypeId) -> bool {
		let (Some(actual), Some(expected)) =
			(self.signature_of(actual), self.signature_of(expected))
		else {
			return false;
		};
		let actual_params: Vec<TypeId> = self
			.s
			.types
			.params(actual.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		let expected_params: Vec<TypeId> = self
			.s
			.types
			.params(expected.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		if actual_params.len() != expected_params.len() || actual.throws != TypeId::NEVER {
			return false;
		}
		let pointer = |w: &Self, ty: TypeId| w.s.types.pointer_target(ty).is_some();
		actual_params
			.iter()
			.zip(&expected_params)
			.all(|(&a, &e)| (pointer(self, a) && pointer(self, e)) || self.s.types.satisfies(e, a))
			&& self.s.types.satisfies(actual.ret, expected.ret)
	}

	/// Whether a value is a place rooted at a `mutable` local (or a
	/// mutable parameter, or `self` of a mutating method).
	fn is_mutable_place(&mut self, value: NodeId) -> bool {
		let value = self.strip_groups(value);
		let Some(place) = self.place_of(value) else {
			return false;
		};
		let Some(symbol) = self.place_symbol(place) else {
			return false;
		};
		matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Mutable
				| SymbolKind::MutableUse
				| SymbolKind::Parameter { .. }
				| SymbolKind::SelfParameter
		)
	}

	/// `FFI006`: a callback is a `function` known at compile time, which
	/// captures nothing and can't throw.
	fn check_callback(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		let symbol = match self.view(value) {
			View::Name(_) | View::Access { .. } => {
				let node = match self.view(value) {
					View::Access { members, .. } => members.last().unwrap_or(value),
					_ => value,
				};
				match self.a.resolution(node) {
					Resolution::Local(symbol)
						if self.a.symbols.symbol(symbol).kind == SymbolKind::Function =>
					{
						Some(symbol)
					}
					_ => None,
				}
			}
			_ => None,
		};
		let problem = match symbol {
			None => {
				Some("a C callback is a `function` declared with a name, known at compile time")
			}
			Some(symbol) => {
				let ty = self.a.symbols.symbol(symbol).ty;
				let throws = self
					.signature_of(ty)
					.is_some_and(|sig| sig.throws != TypeId::NEVER);
				let captures = self
					.direct_function(symbol)
					.and_then(|decl| self.a.functions.get(&decl))
					.is_some_and(|info| !info.captures.is_empty());
				if throws {
					Some("a C callback can't throw: C can't propagate a Fa error")
				} else if captures {
					Some(
						"a C callback can't capture local values: pass its data through a `Pointer<Void>?` parameter",
					)
				} else {
					None
				}
			}
		};
		match problem {
			Some(message) => {
				let diagnostic = self.diagnostic(&FFI006, value, message);
				self.report(diagnostic);
			}
			None => {
				self.a.foreign_args.insert(value, ForeignArg::Callback);
			}
		}
	}

	/// Whether parameter `index` of a signature is declared `mutates`.
	pub(super) fn param_mutated(&self, sig: FnSig, index: usize) -> bool {
		let (mutates, _) = self.s.types.effects(sig.effects);
		mutates.iter().any(
			|&path| matches!(self.s.types.path_data(path).0.root, PathRoot::Param(i) if i as usize == index),
		)
	}

	// ── values outside foreign files (D29) ───────────────────────────────

	/// `FFI002`: a value whose public type contains `Pointer` or `CString`,
	/// used outside foreign files.
	pub(super) fn check_foreign_value(&mut self, n: NodeId, ty: TypeId) {
		if !self.s.types.flags(ty).contains(TypeFlags::HAS_FOREIGN) || self.in_foreign_file() {
			return;
		}
		// a C function called from here is `FFI001` (calls.rs)
		if self.extern_of(n).is_some() || !self.foreign.reported.insert(n) {
			return;
		}
		let message = format!(
			"a `{}` holds a `Pointer` or a `CString`, which only foreign files can use: give it a safe interface in its foreign file",
			self.display(ty)
		);
		let diagnostic = self.diagnostic(&FFI002, n, message);
		self.report(diagnostic);
	}

	// ── destroy ───────────────────────────────────────────────────────────

	/// A type declaring `destroy`: `FFI003` outside foreign files, `FFI011`
	/// for a `destroy` that isn't `(self) => …` or that throws.
	pub(super) fn check_destroy_declaration(&mut self, function: NodeId) -> bool {
		let View::Function {
			value, is_async, ..
		} = self.view(function)
		else {
			return false;
		};
		if !self.in_foreign_file() {
			let diagnostic = self.diagnostic(
				&FFI003,
				function,
				"only a type of a foreign file (`*.foreign.fa`) can declare `destroy`",
			);
			self.report(diagnostic);
			return false;
		}
		let valid = match self.view(value) {
			View::Lambda { signature, .. } => {
				signature.throws.is_none()
					&& signature.parameters.len() == 1
					&& signature
						.parameters
						.first()
						.is_some_and(|first| super::type_expr::is_self_parameter(&self.tree, first))
			}
			_ => false,
		};
		if !valid || is_async {
			let diagnostic = self.diagnostic(
				&FFI011,
				function,
				"`destroy` takes `self` only, and can't throw or be `async`: `function destroy = (self) => { … }`",
			);
			self.report(diagnostic);
		}
		true
	}

	/// The move-only values a call moves into its callee (its arguments: the
	/// callee owns them), a literal into its fields, a closure into its
	/// environment.
	pub(crate) fn move_handles(&mut self, values: &[NodeId]) {
		for &value in values {
			self.consume_local(value, true);
		}
	}

	/// A copy of a move-only value (a declaration, an assignment, a returned
	/// value, an item pushed) moves it: a later use is `MEM002`.
	pub(crate) fn note_resource_move(&mut self, value: NodeId) {
		self.consume_local(value, false);
	}

	/// Consumes what `value` reads when its type is move-only: a local or a
	/// parameter as a whole, or a field of one (a partial move: the value
	/// can't be used whole until the field is assigned again). A value not
	/// owned here (a `use`, a loop variable, `self`) can't give a field up:
	/// `MEM002`, saying to move the whole value.
	fn consume_local(&mut self, value: NodeId, handles: bool) {
		let mut value = self.strip_groups(value);
		while let View::Try(inner) = self.view(value) {
			value = self.strip_groups(inner);
		}
		let branches: Option<Vec<NodeId>> = match self.view(value) {
			View::Do(inner) => Some(vec![inner]),
			View::Block { statements } => Some(statements.last().into_iter().collect()),
			View::If {
				condition,
				then_body,
				else_body,
				..
			} => Some(match self.s.types.kind(self.a.type_of(condition)) {
				crate::types::TypeKind::BoolLit(true) => vec![then_body],
				crate::types::TypeKind::BoolLit(false) => else_body.into_iter().collect(),
				_ => std::iter::once(then_body).chain(else_body).collect(),
			}),
			View::When { arms, .. } => Some(arms.iter().map(|arm| arm.value).collect()),
			View::Catch {
				expression,
				handler,
				..
			} => Some(vec![expression, handler]),
			View::Fallback { operands } => Some(operands.iter().collect()),
			_ => None,
		};
		if let Some(branches) = branches {
			for branch in branches {
				self.consume_local(branch, handles);
			}
			return;
		}
		let actual = self.a.type_of(value);
		let ty = self.a.coercion(value).unwrap_or(actual);
		if !self.body.flow.reachable {
			return;
		}
		if self.s.types.takes_task_parameter(ty) && self.s.types.is_resource(actual) {
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::advanced::MEM002,
				value,
				"this structural value consumes its Task fields, but the source has `destroy`; its fields can't be moved out of their owner",
			);
			self.report_flow(diagnostic);
			return;
		}
		if self.fresh_field_owner(value)
			&& ((!handles && self.s.types.is_move_only(ty))
				|| (handles && self.s.types.could_own_task(ty)))
		{
			self.mark_fresh_field_transfer(value, ty, handles);
			if let Some(resource_gate) = self.moved_field_gate(value) {
				self.defer_copy_condition(
					value,
					crate::analysis::CopyRequirement {
						subject: ty,
						explicit: false,
						task_transfer: false,
						task_only: handles,
						resource_gate: Some(resource_gate),
					},
				);
			}
			return;
		}
		if !self.s.types.is_move_only(ty) || (handles && !self.s.types.takes_task_parameter(ty)) {
			if self.s.types.could_own_task(ty) {
				self.consume_task_conditionally(value, ty);
			}
			return;
		}
		if !handles
			&& matches!(
				self.s.types.kind(ty),
				crate::types::TypeKind::Sync(crate::types::SyncKind::Worker, _)
			) {
			return;
		}
		let tree = std::sync::Arc::clone(&self.tree);
		let root = match syntax::view(&tree, value) {
			View::Name(_) => value,
			View::Access { target, members }
				if members
					.iter()
					.all(|member| matches!(self.a.resolution(member), Resolution::Field(..))) =>
			{
				self.strip_groups(target)
			}
			_ => {
				self.report_item_moved_out(value);
				return;
			}
		};
		if !matches!(syntax::view(&tree, root), View::Name(_)) {
			self.report_item_moved_out(value);
			return;
		}
		let Resolution::Local(symbol) = self.a.resolution(root) else {
			return;
		};
		let kind = self.a.symbols.symbol(symbol).kind;
		let owned = (matches!(kind, SymbolKind::Let | SymbolKind::Mutable)
			|| (matches!(kind, SymbolKind::Parameter { .. })
				&& self
					.s
					.types
					.could_own_task(self.a.symbols.symbol(symbol).ty)))
			&& self.owns_task_symbol_at(symbol, value);
		if !owned {
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::advanced::MEM002,
				value,
				"this value is borrowed here and can't be moved or copied; transfer an owned value instead",
			);
			self.report_flow(diagnostic);
			return;
		}
		// the backends make the place give the value up (`moved_reads`)
		if owned && self.s.types.is_move_only(ty) {
			self.a.moved_reads.insert(value);
		}
		if root == value {
			if owned
				&& !self
					.body
					.flow
					.consumed
					.iter()
					.any(|&(other, _)| other == symbol)
			{
				self.body.flow.consumed.push((symbol, value));
			}
			return;
		}
		// a field moved out
		if let View::Access { members, .. } = syntax::view(&tree, value) {
			let count = members.len();
			let holders = std::iter::once(root).chain(members.iter().take(count - 1));
			let resource = holders
				.into_iter()
				.find(|&holder| self.s.types.is_resource(self.a.type_of(holder)));
			if let Some(resource) = resource {
				let text = tree.text(value.index()).to_string();
				let name = tree.text(resource.index()).to_string();
				let message = format!(
					"`{text}` can't be moved out of `{name}`: its type has `destroy`, which uses every field; move the whole value instead"
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::MEM002, value, message);
				self.report_flow(diagnostic);
				return;
			}
		}
		if let Some(place) = self.place_of(value)
			&& !self
				.body
				.flow
				.moved_fields
				.iter()
				.any(|&(other, _)| other == place)
		{
			self.body.flow.moved_fields.push((place, value));
		}
	}

	/// A move-only value read out of an item of a collection (`items[0]`,
	/// `items[0].task`): it can't leave a hole in the collection (MEM002).
	fn report_item_moved_out(&mut self, value: NodeId) {
		let tree = std::sync::Arc::clone(&self.tree);
		let mut current = value;
		let collection = loop {
			match syntax::view(&tree, current) {
				View::Group(inner) | View::Try(inner) => current = inner,
				View::Access { target, members } => {
					if members.last().is_some_and(|member| {
						matches!(self.a.resolution(member), Resolution::Builtin(builtin) if crate::builtins::get(builtin).path.ends_with("#value"))
					}) {
						let diagnostic = self.diagnostic(&crate::diagnostics::codes::advanced::MEM002, value,
							"this value is borrowed from another owner and can't be moved or copied");
						self.report_flow(diagnostic);
						return;
					}
					current = target;
				}
				View::Index { target, .. } => break target,
				View::Call { .. } => {
					match crate::lower::copies::element_view(&tree, &self.a, current) {
						Some((target, false)) => break target,
						_ => return,
					}
				}
				_ => return,
			}
		};
		let text = tree.text(value.index()).to_string();
		let name = tree.text(collection.index()).to_string();
		let message = format!(
			"`{text}` can't be moved out of `{name}`: its value can't be copied, and a collection can't keep a hole; take it out with `{name}.take(…)`, or move the whole collection"
		);
		let diagnostic =
			self.diagnostic(&crate::diagnostics::codes::advanced::MEM002, value, message);
		self.report_flow(diagnostic);
	}

	/// `MEM002` when an access reads a field moved out of its local, or
	/// below one (flow part; `written`: the whole access is assigned, which
	/// gives the field a value again).
	pub(crate) fn check_moved_fields(&mut self, n: NodeId, target: NodeId, written: bool) {
		if self.body.flow.moved_fields.is_empty() || !self.body.flow.reachable {
			return;
		}
		let tree = std::sync::Arc::clone(&self.tree);
		let View::Access { members, .. } = syntax::view(&tree, n) else {
			return;
		};
		let Some(mut place) = self.place_of(target) else {
			return;
		};
		let count = members.len();
		for (position, member) in members.iter().enumerate() {
			let Some(next) = self.member_step(place, member) else {
				return;
			};
			place = next;
			if written && position + 1 == count {
				return;
			}
			if let Some(&(_, at)) = self
				.body
				.flow
				.moved_fields
				.iter()
				.find(|&&(other, _)| other == place)
			{
				let text = tree.text(n.index()).to_string();
				let line = self.line_of(at);
				let message = format!(
					"`{}` was moved out on line {line}: assign it again before using `{text}`",
					tree.text(at.index())
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::MEM002, n, message);
				self.report_flow(diagnostic);
				return;
			}
		}
	}

	/// A field assigned again (`holder.task = …`), or its local as a whole:
	/// its moved fields are back.
	pub(crate) fn restore_moved_fields(&mut self, target: NodeId) {
		if let Some(place) = self.place_of(target) {
			self.body
				.flow
				.conditional_consumed
				.retain(|entry| !self.places.is_at_or_below(entry.0, place));
			if self.places.parent(place).is_none()
				&& let Some(symbol) = self.place_symbol(place)
			{
				self.reset_consumed(symbol);
			}
		}
		if self.body.flow.moved_fields.is_empty() {
			return;
		}
		let Some(place) = self.place_of(target) else {
			return;
		};
		let places = std::mem::take(&mut self.body.flow.moved_fields);
		let kept = places
			.into_iter()
			.filter(|&(moved, _)| !self.places.is_at_or_below(moved, place))
			.collect();
		self.body.flow.moved_fields = kept;
	}

	/// The node at which a field of `symbol` was moved out, if one was.
	pub(crate) fn moved_field_of(&self, symbol: SymbolId) -> Option<NodeId> {
		self.body
			.flow
			.moved_fields
			.iter()
			.find(|&&(place, _)| self.place_symbol(place) == Some(symbol))
			.map(|&(_, at)| at)
	}
}
