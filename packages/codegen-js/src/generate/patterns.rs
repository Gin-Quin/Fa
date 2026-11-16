//! Destructuring (values brief §16, `Analysis::patterns`): `let { a, b } =
//! o`, `let [first, ...rest] = items`, `let (x, y) = pair`, nested
//! patterns, `for (key, value) in …` and destructured parameters.
//!
//! The value is evaluated once (a temporary unless it is a stable name),
//! then each element is bound in order by its own declaration: a field is
//! `value.field`, an item `value[index]`, a default `read ?? default` (none
//! is `undefined`, so `??` is exactly "when the field holds `none`"). An
//! array pattern whose length isn't proven checks it once and throws
//! `IndexError`. A composite element is copied when the binding is
//! `mutable` or the value is read from a mutable place, like a plain
//! binding.

use fa_analyzer::analysis::{NodeFlags, PatternBinding};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};

use crate::generate::Generator;
use crate::js::{Expr, Prec};

impl Generator<'_> {
	/// The names a pattern declares, nested patterns included, in order.
	pub(crate) fn pattern_symbols(&self, pattern: NodeId) -> Vec<SymbolId> {
		let mut symbols = Vec::new();
		let mut pending = vec![pattern];
		while let Some(pattern) = pending.pop() {
			let Some(info) = self.analysis.pattern(pattern) else {
				continue;
			};
			for element in info.elements.iter().rev() {
				match element.binding {
					PatternBinding::Symbol(symbol) => symbols.push(symbol),
					PatternBinding::Nested(nested) => pending.push(nested),
					PatternBinding::None => {}
				}
			}
		}
		symbols
	}

	/// Binds the names of a pattern to the parts of `value`: `const` (or
	/// `let` when `mutable`) declarations. `copy` copies the composite
	/// parts (a mutable binding, or a value read from a mutable place).
	pub(crate) fn bind_pattern(&mut self, pattern: NodeId, value: Expr, mutable: bool, copy: bool) {
		let Some(info) = self.analysis.pattern(pattern).cloned() else {
			self.unsupported(pattern, "this pattern");
			return;
		};
		let value = if value.stable {
			value
		} else {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", value.text));
			Expr::name(temp, true)
		};
		let target = value.primary_text();
		let items = info
			.elements
			.iter()
			.filter(|element| element.field.is_none())
			.count();
		if self.has(pattern, NodeFlags::INDEX_CHECK) && items > 0 {
			let error = self.helper("IndexError");
			self.out.line(format!(
				"if ({target}.length < {items}) throw new {error}({}, {target}.length);",
				items - 1
			));
		}
		let mut position = 0;
		for element in &info.elements {
			let read = match element.field {
				Some(field) => {
					let property = Self::property(self.types.syms.resolve(field));
					format!("{target}.{property}")
				}
				None => {
					position += 1;
					format!("{target}[{}]", position - 1)
				}
			};
			let read = Expr::new(read, Prec::Primary).with_purity(true, value.stable);
			match element.binding {
				PatternBinding::Symbol(symbol) => {
					self.bind_element(
						symbol,
						read,
						element.ty,
						element.default,
						element.absent,
						mutable,
						copy,
					);
				}
				PatternBinding::Nested(nested) => {
					let read = match element.default {
						Some(default) if !element.absent => self.with_default(read, default),
						Some(default) => self.expr(default),
						None => read,
					};
					self.bind_pattern(nested, read, mutable, copy);
				}
				PatternBinding::None => {}
			}
		}
		if let Some((symbol, ty)) = info.rest {
			let rest = if info.elements.iter().all(|element| element.field.is_none()) {
				// the items after the named ones: a new array
				Expr::primary(format!("{target}.slice({items})"))
			} else {
				self.object_rest(&target, ty, info.ty)
			};
			self.bind_element(symbol, rest, ty, None, false, mutable, copy);
		}
	}

	/// One name of a pattern: `const name = read` (or its default).
	#[allow(clippy::too_many_arguments)]
	fn bind_element(
		&mut self,
		symbol: SymbolId,
		read: Expr,
		ty: TypeId,
		default: Option<NodeId>,
		absent: bool,
		mutable: bool,
		copy: bool,
	) {
		let module_value = self.is_module_value(symbol);
		let name = self.declare(symbol);
		let keyword = if module_value {
			""
		} else if mutable {
			"let "
		} else {
			"const "
		};
		let value = match default {
			Some(default) if absent => self.expr(default),
			Some(default) => {
				let indent = self.out.indentation() + 1;
				let (lines, fallback) = self.capture_at(indent, |g| g.expr(default));
				if !lines.is_empty() {
					// a default that needs statements: assigned when the field is `none`
					let declared = if module_value || self.guard_binding(&name) {
						name.clone()
					} else {
						format!("let {name}")
					};
					self.out.line(format!("{declared} = {};", read.text));
					self.out.open(format!("if ({name} === undefined)"));
					self.out.append(lines);
					self.out.line(format!("{name} = {};", fallback.text));
					self.out.close("");
					return;
				}
				crate::js::binary(&read, "??", &fallback, Prec::Nullish)
			}
			None => read,
		};
		let value = if copy && self.is_composite(ty) && default.is_none() {
			self.copy_value(value, ty)
		} else {
			value
		};
		let keyword = if !module_value && self.guard_binding(&name) {
			""
		} else {
			keyword
		};
		self.out.line(format!("{keyword}{name} = {};", value.text));
	}

	/// `read ?? default` for a nested pattern's value.
	fn with_default(&mut self, read: Expr, default: NodeId) -> Expr {
		let fallback = self.expr(default);
		crate::js::binary(&read, "??", &fallback, Prec::Nullish)
	}

	/// `...rest` of an object pattern: a new object with the other fields,
	/// in the order of the value's type.
	fn object_rest(&mut self, target: &str, rest: TypeId, value: TypeId) -> Expr {
		let wanted: Vec<String> = self
			.types
			.fields_of(rest)
			.iter()
			.map(|field| self.types.syms.resolve(field.name).to_string())
			.collect();
		let order: Vec<String> = match self
			.analysis
			.type_decls
			.values()
			.find(|decl| decl.ty == value)
		{
			Some(decl) => decl
				.fields
				.iter()
				.map(|field| self.types.syms.resolve(field.name).to_string())
				.collect(),
			None => self
				.types
				.fields_of(value)
				.iter()
				.map(|field| self.types.syms.resolve(field.name).to_string())
				.collect(),
		};
		let parts: Vec<String> = order
			.iter()
			.filter(|name| wanted.contains(name))
			.map(|name| {
				let property = Self::property(name);
				format!("{property}: {target}.{property}")
			})
			.collect();
		if parts.is_empty() {
			return Expr::primary("{}");
		}
		Expr::primary(format!("{{ {} }}", parts.join(", ")))
	}

	/// `let { a, b } = value`, `mutable [x, y] = value`.
	pub(crate) fn destructuring(&mut self, pattern: NodeId, value: NodeId, mutable: bool) {
		// parts of a fresh value are not shared: only a place's are copied
		let copy = self
			.place_root(value)
			.is_some_and(|root| mutable || self.is_mutable_place(root));
		let value = self.expr(value);
		self.bind_pattern(pattern, value, mutable, copy);
	}
}
