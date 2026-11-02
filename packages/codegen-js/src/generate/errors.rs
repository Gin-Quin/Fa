//! Errors (values brief §15, D-24, D-92): every member of an `Errors` type is
//! a class extending the runtime's `FaError`, with a static `faName` (its
//! qualified name) and its fields in declaration order; the built-in errors
//! (`IndexError`…) are the runtime's classes. `throw` and `try` are plain
//! JavaScript; `catch` keeps only Fa errors:
//!
//! ```js
//! let $t0;
//! try {
//!     $t0 = load("");
//! } catch ($e) {
//!     const error = faCaught($e);
//!     $t0 = 0;
//! }
//! ```

use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{NominalKind, TypeKind};

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec};

/// The module of the prelude, where the built-in errors are declared.
const PRELUDE: &str = "prelude";

impl Generator<'_> {
	/// `throw error`.
	pub(crate) fn throw_statement(&mut self, value: NodeId) {
		let value = self.expr(value);
		self.out.line(format!("throw {};", value.text));
	}

	/// `expression catch error => handler`, its value going to `dest`.
	pub(crate) fn catch_into(
		&mut self,
		n: NodeId,
		expression: NodeId,
		handler: NodeId,
		dest: Dest,
	) {
		let ty = self.type_of(n);
		let dest = if ty == TypeId::VOID || ty == TypeId::NEVER {
			Dest::Discard
		} else {
			dest
		};
		self.out.open("try");
		self.into(expression, dest.clone());
		self.out.reopen("catch ($e)");
		let caught = self.helper("faCaught");
		self.push_scope();
		match self.analysis.symbols.symbol_of_decl(n) {
			Some(symbol) => {
				let name = self.declare(symbol);
				self.out.line(format!("const {name} = {caught}($e);"));
			}
			None => self.out.line(format!("{caught}($e);")),
		}
		match self.view(handler) {
			View::Block { .. } => self.block_into(handler, dest),
			_ => self.into(handler, dest),
		}
		self.pop_scope();
		self.out.close("");
	}

	/// Whether a nominal type is an `Errors` type.
	pub(crate) fn is_errors(&self, nominal: NominalId) -> bool {
		self.types.nominal(nominal).kind == NominalKind::Errors
	}

	/// The class of a member of an `Errors` type: a runtime class for a
	/// built-in error, a generated one (`ApiError$Timeout`) otherwise.
	pub(crate) fn error_class(&mut self, nominal: NominalId, index: u16) -> String {
		let info = self.types.nominal(nominal);
		let key = self.types.decl_key_data(info.key);
		let type_name = self.types.decl_name(info.key).to_string();
		if self.types.module_path(key.module) == PRELUDE {
			if crate::runtime::helper(&type_name).is_some() {
				return self.helper_name(&type_name);
			}
			return type_name;
		}
		let member = self.types.member_name(nominal, index).to_string();
		let qualified = format!("{type_name}.{member}");
		let wanted = format!("{type_name}${member}");
		if let Some(name) = self.classes.get(&qualified) {
			return name.clone();
		}
		let fields = self.error_fields(nominal, index);
		let base = self.helper("FaError");
		let mut code = vec![
			format!("class {{name}} extends {base} {{"),
			format!("\tstatic faName = {};", js::string_literal(&qualified)),
		];
		if !fields.is_empty() {
			code.push(String::new());
			code.push(format!("\tconstructor({}) {{", fields.join(", ")));
			code.push("\t\tsuper();".to_string());
			for field in &fields {
				code.push(format!("\t\tthis.{field} = {field};"));
			}
			code.push("\t}".to_string());
		}
		code.push("}".to_string());
		let name = self.class(&wanted, code);
		self.classes.insert(qualified, name.clone());
		name
	}

	/// The fields of a member of an `Errors` type, in declaration order (the
	/// order of its constructor's parameters).
	pub(crate) fn error_fields(&self, nominal: NominalId, index: u16) -> Vec<String> {
		let payload = self.types.payload(nominal, index);
		if payload.is_unset() {
			return Vec::new();
		}
		if !matches!(self.types.kind(payload), TypeKind::Object(_)) {
			return vec!["$v".to_string()];
		}
		// the declaration order, from the syntax of the type's declaration
		let declared = self
			.member_definition(nominal, index)
			.and_then(|definition| {
				let View::Object { items } = self.view(definition) else {
					return None;
				};
				let names: Vec<String> = items
					.iter()
					.filter_map(|item| match self.view(item) {
						View::Assignment { target, .. } => Some(Self::property(self.text(target))),
						View::Name(_) => Some(Self::property(self.text(item))),
						_ => None,
					})
					.collect();
				Some(names)
			});
		declared.unwrap_or_else(|| {
			self.types
				.fields_of(payload)
				.iter()
				.map(|field| Self::property(self.types.syms.resolve(field.name)))
				.collect()
		})
	}

	/// The definition of a member in its type's declaration
	/// (`Timeout = { seconds: Integer }` → the object type node).
	fn member_definition(&self, nominal: NominalId, index: u16) -> Option<NodeId> {
		let (&node, _) = self.analysis.type_decls.iter().find(|(_, decl)| {
			matches!(
				self.types.kind(decl.ty),
				TypeKind::NamedUnion(found, _) | TypeKind::Member(found, _) if found == nominal
			)
		})?;
		let View::TypeDecl { value, .. } = self.view(node) else {
			return None;
		};
		let members = match self.view(value) {
			View::TypedObject { members, .. } | View::Object { items: members } => members,
			_ => return None,
		};
		match self.view(members.get(index as usize)?) {
			View::Assignment {
				value: Some(definition),
				..
			} => Some(definition),
			_ => None,
		}
	}

	/// `new ApiError$Timeout(…)` with the values of the fields in order.
	pub(crate) fn new_error(
		&mut self,
		nominal: NominalId,
		index: u16,
		values: Vec<String>,
	) -> Expr {
		let class = self.error_class(nominal, index);
		Expr::primary(format!("new {class}({})", values.join(", ")))
	}

	/// The test that a value is a member (or any member) of an `Errors` type.
	pub(crate) fn error_test(&mut self, subject: &Expr, ty: TypeId) -> Option<Expr> {
		let classes: Vec<String> = match self.types.kind(ty) {
			TypeKind::Member(nominal, index) if self.is_errors(nominal) => {
				vec![self.error_class(nominal, index)]
			}
			TypeKind::NamedUnion(nominal, mask) if self.is_errors(nominal) => {
				let count = self
					.types
					.sym_list(self.types.nominal(nominal).members)
					.len();
				(0..count as u16)
					.filter(|&index| self.types.mask_has(mask, index))
					.map(|index| self.error_class(nominal, index))
					.collect()
			}
			_ => return None,
		};
		let tests: Vec<String> = classes
			.iter()
			.map(|class| format!("{} instanceof {class}", subject.primary_text()))
			.collect();
		Some(Expr::new(tests.join(" || "), Prec::Or))
	}

	/// The name under which a runtime helper is inlined (itself).
	fn helper_name(&mut self, name: &str) -> String {
		let helper = crate::runtime::helper(name).expect("a runtime helper").name;
		self.helper(helper).to_string()
	}
}
