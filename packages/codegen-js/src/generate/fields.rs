//! `Fields` values (types brief §3.9, `types/fields.md`): a JavaScript
//! number up to 32 flags, a `bigint` above, one bit per flag in declaration
//! order. The analysis gives the bits of every literal (`T { a }`, `T.a`, a
//! key set) as a compile-time value; the operations are bit operations:
//! `+` is `|`, `-` is `& ~`, `x is in s` is `(s & x) === x`. A 32-flag set
//! has its bit 31: the result of a bit operation goes through `>>> 0`.
//!
//! An `@for` over a key set is generated for each key with the analysis of
//! its body for that key (`Analysis::key_loop_bodies`): a `for…of` over
//! the keys when every key gives the same code (`value[key]` read with
//! `faFieldAt`), else unrolled, the fields read directly. A key-set
//! compile-time parameter is a hidden object holding its paths and a
//! stable instance tag, shared by local and imported applications; an
//! unrolled loop over it is a `switch` on that tag.

use fa_analyzer::analysis::{Analysis, KeyLoopInstance};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::ArithOp;
use fa_analyzer::types::TypeKind;

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec, Writer};

impl<'s> Generator<'s> {
	/// Whether a type is a `Fields` type.
	pub(crate) fn is_fields(&self, ty: TypeId) -> bool {
		matches!(self.types.kind(ty), TypeKind::Fields(_))
	}

	/// Whether the values of a `Fields` type are `bigint`s (more than 32 flags).
	pub(crate) fn fields_are_big(&self, ty: TypeId) -> bool {
		self.types.fields_width(ty) > 32
	}

	/// A number result of a bit operation, unsigned when bit 31 may be set.
	fn fields_unsigned(&self, ty: TypeId, value: Expr) -> Expr {
		if self.types.fields_width(ty) == 32 {
			js::binary(
				&value,
				">>>",
				&Expr::constant("0", Prec::Primary),
				Prec::Shift,
			)
		} else {
			value
		}
	}

	/// `a + b + …` (union) and `a - b - …` (difference) of field sets.
	pub(crate) fn fields_arithmetic(&mut self, n: NodeId, op: ArithOp, nodes: &[NodeId]) -> Expr {
		let ty = self.type_of(n);
		let values = self.in_order(nodes, |g, operand| g.expr(operand));
		let mut values = values.into_iter();
		let Some(mut result) = values.next() else {
			return self.internal_error(n, "the checked field-set operation has no operands");
		};
		for value in values {
			result = self.fields_step(ty, op, &result, &value);
		}
		result
	}

	/// One step of `+` (`|`) or `-` (`& ~`) on values of the `Fields` type `ty`.
	pub(crate) fn fields_step(
		&mut self,
		ty: TypeId,
		op: ArithOp,
		left: &Expr,
		right: &Expr,
	) -> Expr {
		let result = match op {
			ArithOp::Add => js::binary(left, "|", right, Prec::BitOr),
			_ => js::binary(left, "&", &js::prefix("~", right), Prec::BitAnd),
		};
		if self.fields_are_big(ty) {
			result
		} else {
			self.fields_unsigned(ty, result)
		}
	}

	/// `value is in set`: every flag of `value` is in `set`.
	pub(crate) fn fields_is_in(&mut self, set_ty: TypeId, set: &Expr, value: &Expr) -> Expr {
		// the value is used twice
		let (value, set) = (value.clone(), set.clone());
		let value = self.keep(value);
		let common = js::binary(&set, "&", &value, Prec::BitAnd);
		let common = if self.fields_are_big(set_ty) {
			common
		} else {
			self.fields_unsigned(set_ty, common)
		};
		js::binary(&common, "===", &value, Prec::Equality)
	}

	/// The module-level table of the dotted paths of the flags of a `Fields`
	/// type, by bit.
	pub(crate) fn fields_names(&mut self, ty: TypeId) -> String {
		let names: Vec<String> = self
			.types
			.fields_leaves(ty)
			.iter()
			.map(|path| js::string_literal(path))
			.collect();
		let wanted = match self.types.type_name(ty) {
			Some(name) => format!("{name}$flags"),
			None => "fields$flags".to_string(),
		};
		self.constant(&wanted, format!("Object.freeze([{}])", names.join(", ")))
	}

	/// `set.toStringArray()`.
	pub(crate) fn fields_to_string_array(&mut self, ty: TypeId, value: Expr) -> Expr {
		let names = self.fields_names(ty);
		let helper = self.helper("faFieldsToStringArray");
		js::call(helper, &[value.text, names])
	}

	/// The function printing the values of a `Fields` type: `{ a, g { b } }`.
	pub(crate) fn fields_formatter(&mut self, ty: TypeId) -> String {
		let names = self.fields_names(ty);
		let helper = self.helper("faFormatFields");
		format!("(value) => {helper}(value, {names})")
	}

	/// The paths and stable tag of an application, independent of a file's
	/// locally recorded order of instances. Forwarded parameters keep their
	/// enclosing function's hidden value.
	pub(crate) fn key_set_value(
		&mut self,
		binding: TypeId,
		subject: Option<TypeId>,
	) -> Option<String> {
		if let TypeKind::Param(param) = self.types.kind(binding) {
			return self
				.hidden_names
				.get(&super::generics::Hidden::Value(param))
				.cloned();
		}
		let TypeKind::Tuple(list) = self.types.kind(binding) else {
			return None;
		};
		let [fields, bits] = *self.types.list_items(list) else {
			return None;
		};
		let bits = self.types.as_int(bits)?.literal()?;
		let paths: Vec<_> = self
			.types
			.fields_leaves(fields)
			.iter()
			.enumerate()
			.filter(|(index, _)| (bits >> *index as u32) & ethnum::I256::ONE == ethnum::I256::ONE)
			.map(|(_, path)| js::string_literal(path))
			.collect();
		let tag = self.key_set_tag(subject?, binding)?;
		Some(self.constant(
			"keys",
			format!(
				"Object.freeze({{ tag: {tag}, paths: Object.freeze([{}]) }})",
				paths.join(", ")
			),
		))
	}

	/// The same subject and key set have the same tag in every unit. The
	/// subject is part of it: equal paths may need different generated code
	/// when the selected fields have different types.
	fn key_set_tag(&self, subject: TypeId, key_set: TypeId) -> Option<String> {
		let subject = fa_analyzer::stable_names::stable_type_name(self.types, subject)
			.unwrap_or_else(|| format!("hash:{:032x}", self.types.stable_hash(subject)));
		let TypeKind::Tuple(list) = self.types.kind(key_set) else {
			return None;
		};
		let [fields, bits] = *self.types.list_items(list) else {
			return None;
		};
		let bits = self.types.as_int(bits)?.literal()?;
		let paths: Vec<_> = self
			.types
			.fields_leaves(fields)
			.iter()
			.enumerate()
			.filter(|(index, _)| (bits >> *index as u32) & ethnum::I256::ONE == ethnum::I256::ONE)
			.map(|(_, path)| js::string_literal(path))
			.collect();
		Some(js::string_literal(&format!(
			"{subject}|[{}]",
			paths.join(",")
		)))
	}

	/// `@for key in keys` over a key set: the body generated for each key
	/// (of each application of a key-set parameter) with that key's
	/// analysis; a `for…of` over the keys when every key gives the same
	/// code, else unrolled (per application: a `switch` on the stable tag).
	pub(crate) fn key_loop(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		iterable: NodeId,
		body: NodeId,
	) {
		let main: &'s Analysis = self.analysis;
		let Some(instances) = main.key_loop_bodies.get(&n) else {
			// a key-set parameter that no call applies
			return;
		};
		if instances.is_empty() {
			return;
		}
		self.push_scope();
		let variable = match pattern.and_then(|pattern| main.symbols.symbol_of_decl(pattern)) {
			Some(symbol) => self.declare(symbol),
			None => self.temp(),
		};
		let indent = self.out.indentation() + 1;
		let mut texts: Vec<Writer> = Vec::new();
		for instance in instances {
			for analysis in &instance.analyses {
				texts.push(self.key_body(body, analysis, None, indent));
			}
		}
		let uniform = texts
			.first()
			.is_some_and(|first| texts.iter().all(|text| text.lines() == first.lines()));
		let parameter = instances.iter().any(|instance| instance.param.is_some());
		let path_array = |keys: &[String]| {
			let paths: Vec<String> = keys.iter().map(|key| js::string_literal(key)).collect();
			format!("Object.freeze([{}])", paths.join(", "))
		};
		match (parameter, uniform) {
			(false, true) if instances[0].keys.len() > 1 => {
				let keys = self.constant("keys", path_array(&instances[0].keys));
				self.out.open(format!("for (const {variable} of {keys})"));
				self.out.append(texts.swap_remove(0));
				self.out.close("");
			}
			(false, _) => self.unrolled_keys(&variable, body, &instances[0]),
			(true, true) => {
				let keys = self.expr(iterable);
				self.out.open(format!(
					"for (const {variable} of {}.paths)",
					keys.primary_text()
				));
				self.out.append(texts.swap_remove(0));
				self.out.close("");
			}
			(true, false) => {
				let index = self.expr(iterable);
				self.out
					.open(format!("switch ({}.tag)", index.primary_text()));
				for instance in instances {
					let Some(tag) = self.key_set_tag(instance.subject, instance.key_set) else {
						self.unsupported(n, "this key-set instance");
						continue;
					};
					self.out.open(format!("case {tag}:"));
					self.unrolled_keys(&variable, body, instance);
					self.out.line("break;");
					self.out.close("");
				}
				self.out.close("");
			}
		}
		self.pop_scope();
	}

	/// The body once per key, in a block declaring the key, the fields read
	/// directly.
	fn unrolled_keys(&mut self, variable: &str, body: NodeId, instance: &'s KeyLoopInstance) {
		for (key, analysis) in instance.keys.iter().zip(&instance.analyses) {
			self.out.open("");
			self.out
				.line(format!("const {variable} = {};", js::string_literal(key)));
			let indent = self.out.indentation();
			let text = self.key_body(body, analysis, Some(key.clone()), indent);
			self.out.append(text);
			self.out.close("");
		}
	}

	/// The code of the body of a key loop for one key, generated with that
	/// key's analysis (`direct`: the key, whose fields are read directly).
	fn key_body(
		&mut self,
		body: NodeId,
		analysis: &'s Analysis,
		direct: Option<String>,
		indent: usize,
	) -> Writer {
		let saved_out = std::mem::replace(&mut self.out, Writer::at(indent));
		let saved = std::mem::replace(&mut self.analysis, analysis);
		let saved_key = std::mem::replace(&mut self.direct_key, direct);
		self.push_scope();
		self.block_into(body, Dest::Discard);
		self.pop_scope();
		self.direct_key = saved_key;
		self.analysis = saved;
		std::mem::replace(&mut self.out, saved_out)
	}

	/// `value[key]`, `key` the dotted path of a key: the field at that path,
	/// read directly when the loop is unrolled.
	pub(crate) fn key_read(&mut self, target: NodeId, index: NodeId) -> Expr {
		if let Some(key) = self.direct_key.clone() {
			let value = self.expr(target);
			let path: Vec<String> = key.split('.').map(Self::property).collect();
			return Expr::primary(format!("{}.{}", value.primary_text(), path.join(".")));
		}
		let values = self.in_order(&[target, index], |g, node| g.expr(node));
		let helper = self.helper("faFieldAt");
		js::call(helper, &[values[0].text.clone(), values[1].text.clone()])
	}
}
