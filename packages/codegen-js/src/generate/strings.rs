//! Strings and printing (values brief §2.8, §7; advanced brief D39):
//! interpolation, the members of strings, `console.log` and `console.error`.
//!
//! Interpolation is a template literal. A `String`, an integer,
//! a `Float` or a `Boolean` goes in directly (JavaScript's `ToString` is Fa's
//! `toString` for them, D-5); a `Float32`, a `Percentage`, an optional or a
//! composite value goes through a formatter of the runtime.
//! `console.log(a, b)` prints one template literal: the values separated by
//! a space.

use fa_analyzer::analysis::{Argument, NodeFlags};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::lower::index::IndexAccess;
use fa_analyzer::syntax::{TemplatePart, TemplateParts, View};
use fa_analyzer::types::{FloatBits, TypeKind};

use crate::generate::{Generator, Kind};
use crate::js::{self, Expr, Prec};

/// One part of a template literal being built.
enum Piece {
	/// Text, not escaped yet.
	Text(String),
	/// A value already formatted for the top level (`${…}`).
	Value(Expr),
}

impl Generator<'_> {
	/// A string with interpolations: a template literal.
	pub(crate) fn template(&mut self, parts: TemplateParts<'_>) -> Expr {
		let parts: Vec<TemplatePart<'_>> = parts.iter().collect();
		let nodes: Vec<NodeId> = parts
			.iter()
			.filter_map(|part| match part {
				TemplatePart::Expression(node) => Some(*node),
				TemplatePart::Text(_) => None,
			})
			.collect();
		let values = self.in_order(&nodes, |g, node| g.expr(node));
		let mut values = values.into_iter().zip(nodes);
		let mut pieces = Vec::new();
		for part in parts {
			match part {
				TemplatePart::Text(text) => pieces.push(Piece::Text(text.to_string())),
				TemplatePart::Expression(_) => {
					let (value, node) = values.next().expect("a value per expression");
					let ty = self.printed_type(node);
					pieces.push(self.printed_piece(node, value, ty));
				}
			}
		}
		template_of(pieces)
	}

	/// The value of a node formatted for the top level of a string: a
	/// compile-time range is its text (`static span = 1..5`: `1..5`).
	fn printed_piece(&mut self, node: NodeId, value: Expr, ty: TypeId) -> Piece {
		if let Some((start, end, inclusive)) = self.constant_range(node) {
			let start = start.map_or(String::new(), |start| start.to_string());
			let text = match end {
				None => format!("{start}.."),
				Some(end) if inclusive => format!("{start}..={end}"),
				Some(end) => format!("{start}..{end}"),
			};
			return Piece::Text(text);
		}
		self.top_level_piece(value, ty)
	}

	/// A value formatted for the top level of a string (`toString`).
	fn top_level_piece(&mut self, value: Expr, ty: TypeId) -> Piece {
		if let Some(text) = self.constant_text(&value, ty) {
			return Piece::Text(text);
		}
		match self.types.kind(ty) {
			TypeKind::None => Piece::Text("none".to_string()),
			_ => Piece::Value(self.format_top(value, ty)),
		}
	}

	/// The text a constant prints, when JavaScript prints it the same way:
	/// booleans, safe integers, plain decimal floats.
	fn constant_text(&self, value: &Expr, ty: TypeId) -> Option<String> {
		if !(value.pure && value.stable) {
			return None;
		}
		let text = value.text.as_str();
		let digits = |text: &str| {
			let unsigned = text.strip_prefix('-').unwrap_or(text);
			!unsigned.is_empty() && unsigned.bytes().all(|byte| byte.is_ascii_digit())
		};
		// an integer has no negative zero: `-0n` prints `0`
		let integer = |text: &str| {
			if text.bytes().all(|byte| byte == b'-' || byte == b'0') {
				"0".to_string()
			} else {
				text.to_string()
			}
		};
		match self.kind(ty) {
			Kind::Boolean if text == "true" || text == "false" => Some(text.to_string()),
			Kind::Int(storage) if !storage.is_js_bigint() && digits(text) => Some(integer(text)),
			Kind::Int(_) | Kind::BigInt => text
				.strip_suffix('n')
				.filter(|number| digits(number))
				.map(integer),
			Kind::Float(FloatBits::F64) => {
				let number: f64 = text.parse().ok()?;
				let plain = !text.contains('e') && text != "-0";
				(plain && (number == 0.0 || number.abs() >= 1e-6)).then(|| text.to_string())
			}
			_ => None,
		}
	}

	/// The expression whose `${…}` prints a value of type `ty` at the top level.
	pub(crate) fn format_top(&mut self, value: Expr, ty: TypeId) -> Expr {
		match self.kind(ty) {
			Kind::Text | Kind::Boolean | Kind::BigInt | Kind::Int(_) => value,
			Kind::Float(FloatBits::F64) => value,
			Kind::Float(FloatBits::F32) => {
				let helper = self.helper("faFloat32ToString");
				js::call(helper, &[value.text])
			}
			Kind::Percentage => {
				let helper = self.helper("faPercentageToString");
				js::call(helper, &[value.text])
			}
			Kind::Other => match self.types.kind(ty) {
				TypeKind::Enum(nominal, _) | TypeKind::EnumMember(nominal, _) => {
					let names = self.enum_names(nominal);
					Expr::primary(format!("{names}[{}]", value.text))
				}
				// a string prints raw at the top level, unlike nested: a union
				// holding one prints by its representation
				_ if self.holds_text(ty) => {
					let helper = self.helper("faFormat");
					js::call(helper, &[value.text])
				}
				_ => match self.formatter(ty) {
					Some(formatter) => js::call(&formatter, &[value.text]),
					None => {
						let helper = self.helper("faFormat");
						js::call(helper, &[value.text])
					}
				},
			},
		}
	}

	/// Whether a union type has a string member (`"a" | "b"`, `String?`).
	fn holds_text(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::Union(list) => self
				.types
				.list_items(list)
				.iter()
				.any(|&member| self.kind(member) == Kind::Text),
			_ => false,
		}
	}

	/// The expression whose `${…}` prints a value nested in a composite
	/// (strings quoted): the operands of a failed `assert`.
	pub(crate) fn format_nested(&mut self, value: &Expr, ty: TypeId) -> String {
		match self.kind(ty) {
			Kind::Boolean | Kind::BigInt | Kind::Int(_) | Kind::Float(FloatBits::F64) => {
				value.text.clone()
			}
			Kind::Text => {
				let helper = self.helper("faQuote");
				js::call(helper, std::slice::from_ref(&value.text)).text
			}
			_ => {
				if self.kind(ty) == Kind::Other {
					let formatter = self.nested_formatter(ty);
					return js::call(&formatter, std::slice::from_ref(&value.text)).text;
				}
				self.format_top(value.clone(), ty).text
			}
		}
	}

	/// The type a value prints as: its type, or for a `catch` the analyzer
	/// left `Unknown`, the type of its left operand (the handler's value
	/// has that type as a hint).
	fn printed_type(&self, n: NodeId) -> TypeId {
		let ty = self.type_of(n);
		if self.types.kind(ty) != TypeKind::Unknown {
			return ty;
		}
		match self.view(self.strip_groups(n)) {
			View::Catch { expression, .. } => self.printed_type(expression),
			_ => ty,
		}
	}

	/// The function printing a value of a type nested in a composite:
	/// [`Generator::formatter`], or the dynamic `faFormatNested`.
	pub(crate) fn nested_formatter(&mut self, ty: TypeId) -> String {
		match self.formatter(ty) {
			Some(formatter) => formatter,
			None => self.helper("faFormatNested").to_string(),
		}
	}

	/// The function printing the values of a type nested in a composite,
	/// chosen from the static type, so that a program only contains the
	/// formatters of the types it prints (`String` for a number, `faQuote`
	/// for a string, `faFormatArray(value, String)`…): a helper or a
	/// module-level constant. `None` where only the representation says how
	/// a value prints (a named union, a union of unrelated types, a generic
	/// or recursive type): the dynamic `faFormatNested` then.
	pub(crate) fn formatter(&mut self, ty: TypeId) -> Option<String> {
		// a recursive type prints dynamically below its first level
		if !self.formatting.insert(ty) {
			return None;
		}
		let code = self.formatter_code(ty);
		self.formatting.remove(&ty);
		match code? {
			Ok(helper) => Some(helper),
			Err(code) => Some(self.constant("format", code)),
		}
	}

	/// The formatter of a type: a helper (`Ok`), or the code of a function
	/// (`Err`) to declare as a constant.
	fn formatter_code(&mut self, ty: TypeId) -> Option<Result<String, String>> {
		let code = match self.types.kind(ty) {
			// JavaScript's `String(x)` is the Fa notation of these
			TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::Float(FloatBits::F64)
			| TypeKind::FloatLit(FloatBits::F64, _) => return Some(Ok("String".to_string())),
			TypeKind::String | TypeKind::StringLit(_) => {
				return Some(Ok(self.helper("faQuote").to_string()));
			}
			TypeKind::None => "() => \"none\"".to_string(),
			TypeKind::Bytes => return Some(Ok(self.helper("faFormatBytes").to_string())),
			TypeKind::Range => return Some(Ok(self.helper("faFormatRange").to_string())),
			TypeKind::Float(FloatBits::F32) | TypeKind::FloatLit(FloatBits::F32, _) => {
				return Some(Ok(self.helper("faFloat32ToString").to_string()));
			}
			TypeKind::Percentage => {
				return Some(Ok(self.helper("faPercentageToString").to_string()));
			}
			TypeKind::Enum(nominal, _) | TypeKind::EnumMember(nominal, _) => {
				let names = self.enum_names(nominal);
				format!("(value) => {names}[value]")
			}
			TypeKind::Fields(_) => self.fields_formatter(ty),
			TypeKind::Tuple(list) => {
				let items = self.types.list_items(list).to_vec();
				let formats: Vec<String> = items
					.into_iter()
					.map(|item| self.nested_formatter(item))
					.collect();
				let helper = self.helper("faFormatTuple");
				format!("(value) => {helper}(value, [{}])", formats.join(", "))
			}
			TypeKind::Collection(_, item, key) => {
				let item_format = self.nested_formatter(item);
				if key == TypeId::NONE {
					let helper = self.helper("faFormatArray");
					format!("(value) => {helper}(value, {item_format})")
				} else {
					let key_format = self.nested_formatter(key);
					let helper = self.helper("faFormatMap");
					format!("(value) => {helper}(value, {key_format}, {item_format})")
				}
			}
			TypeKind::Union(list) => return self.union_formatter(list),
			TypeKind::Object(_) => self.object_formatter(ty)?,
			_ => return None,
		};
		Some(Err(code))
	}

	/// The formatter of a union ([`Generator::formatter_code`]): of an
	/// optional, or of members that print alike; `None` for another.
	fn union_formatter(
		&mut self,
		list: fa_analyzer::ids::ListId,
	) -> Option<Result<String, String>> {
		let members: Vec<TypeId> = self.types.list_items(list).to_vec();
		let optional = members.contains(&TypeId::NONE);
		let values: Vec<TypeId> = members
			.into_iter()
			.filter(|&member| member != TypeId::NONE)
			.collect();
		let formats: Vec<Option<String>> = values
			.iter()
			.map(|&member| self.formatter(member))
			.collect();
		let tuples = values
			.iter()
			.all(|&member| matches!(self.types.kind(member), TypeKind::Tuple(_)));
		let inner = match formats.first() {
			// the same formatter for every member (`(1, 2)` or `(0, 0)`)
			Some(Some(first)) if formats.iter().all(|format| format.as_ref() == Some(first)) => {
				first.clone()
			}
			_ if tuples => {
				let helper = self.helper("faFormatTuple");
				let nested = self.helper("faFormatNested");
				let code = format!("(value) => {helper}(value, value.map(() => {nested}))");
				self.constant("format", code)
			}
			_ => return None,
		};
		if !optional {
			return Some(Ok(inner));
		}
		let helper = self.helper("faFormatOptional");
		Some(Err(format!("(value) => {helper}(value, {inner})")))
	}

	/// The code of the formatter of an object type: its fields in
	/// declaration order, or UTF-16 name order for an anonymous type,
	/// with relation sides and private fields left out.
	fn object_formatter(&mut self, ty: TypeId) -> Option<String> {
		let mut fields: Vec<(String, TypeId)> = self
			.types
			.fields_of(ty)
			.iter()
			.map(|field| (self.types.syms.resolve(field.name).to_string(), field.ty))
			.collect();
		// Declared and inline member types carry their field order;
		// anonymous shapes already store fields in UTF-16 name order.
		let order: Vec<String> = match self.analysis.type_decls.values().find(|decl| decl.ty == ty)
		{
			Some(decl) => decl
				.fields
				.iter()
				.map(|field| self.types.syms.resolve(field.name).to_string())
				.collect(),
			None => self
				.analysis
				.inline_field_orders
				.get(&ty)
				.map(|names| {
					names
						.iter()
						.map(|&name| self.types.syms.resolve(name).to_string())
						.collect()
				})
				.unwrap_or_else(|| fields.iter().map(|(name, _)| name.clone()).collect()),
		};
		fields.sort_by_key(|(name, _)| order.iter().position(|field| field == name));
		// printing omits relation sides (D21)
		let relation_sides = self.relation_properties(ty);
		fields.retain(|(name, _)| !relation_sides.contains(&Self::property(name)));
		let mut entries = Vec::new();
		for (name, field_ty) in fields {
			if name.starts_with('#') {
				continue;
			}
			let format = self.nested_formatter(field_ty);
			entries.push(format!(
				"[{}, {format}]",
				js::string_literal(&Self::property(&name))
			));
		}
		let helper = self.helper("faFormatObject");
		Some(format!(
			"(value) => {helper}(value, [{}])",
			entries.join(", ")
		))
	}

	/// `console.log(values…)` and `console.error(values…)`: one line, the
	/// values separated by a space.
	pub(crate) fn console(&mut self, method: &str, values: &[NodeId]) -> Expr {
		if values
			.iter()
			.any(|&node| matches!(self.view(self.strip_groups(node)), View::Spread(_)))
		{
			return self.console_spread(method, values);
		}
		// the text of literal and template arguments is inlined
		let mut nodes = Vec::new();
		for &value in values {
			match self.view(self.strip_groups(value)) {
				View::Template { parts } => {
					for part in parts.iter() {
						if let TemplatePart::Expression(node) = part {
							nodes.push(node);
						}
					}
				}
				View::String(_) | View::Spread(_) => {}
				_ => nodes.push(value),
			}
		}
		let computed = self.in_order(&nodes, |g, node| g.expr(node));
		let mut computed = computed.into_iter();
		let mut pieces = Vec::new();
		for (index, &value) in values.iter().enumerate() {
			if index > 0 {
				pieces.push(Piece::Text(" ".to_string()));
			}
			let inner = self.strip_groups(value);
			match self.view(inner) {
				View::String(text) => pieces.push(Piece::Text(text.to_string())),
				View::Template { parts } => {
					for part in parts.iter() {
						match part {
							TemplatePart::Text(text) => pieces.push(Piece::Text(text.to_string())),
							TemplatePart::Expression(node) => {
								let value = computed.next().expect("a value");
								let ty = self.printed_type(node);
								let piece = self.printed_piece(node, value, ty);
								pieces.push(piece);
							}
						}
					}
				}
				_ => {
					let value_expr = computed.next().expect("a value");
					let ty = self.printed_type(value);
					let piece = self.printed_piece(value, value_expr, ty);
					pieces.push(piece);
				}
			}
		}
		let argument = match pieces.as_slice() {
			[] => None,
			[Piece::Value(value)] if values.len() == 1 => {
				match self.kind(self.type_of(values[0])) {
					Kind::Text => Some(value.text.clone()),
					Kind::Int(_) | Kind::BigInt | Kind::Boolean | Kind::Float(FloatBits::F64) => {
						Some(js::call("String", std::slice::from_ref(&value.text)).text)
					}
					_ => Some(value.text.clone()),
				}
			}
			_ => Some(template_of(pieces).text),
		};
		Expr::primary(format!(
			"console.{method}({})",
			argument.unwrap_or_default()
		))
	}

	/// A spread can contain any number of arguments, so append its formatted
	/// items without spreading them onto the JavaScript call stack.
	fn console_spread(&mut self, method: &str, values: &[NodeId]) -> Expr {
		let parts = self.temp();
		self.out.line(format!("const {parts} = [];"));
		for &node in values {
			if let View::Spread(collection) = self.view(self.strip_groups(node)) {
				let value = self.expr(collection);
				let ty = match self.types.kind(self.type_of(collection)) {
					TypeKind::Collection(_, item, _) => item,
					_ => TypeId::UNKNOWN,
				};
				let item = self.temp();
				let formatted = self.format_top(Expr::name(item.clone(), true), ty);
				self.out
					.open(format!("for (const {item} of {})", value.text));
				self.out.line(format!("{parts}.push({});", formatted.text));
				self.out.close("");
			} else {
				let value = self.expr(node);
				let ty = self.printed_type(node);
				let formatted = self.format_top(value, ty);
				self.out.line(format!("{parts}.push({});", formatted.text));
			}
		}
		Expr::primary(format!("console.{method}({parts}.join(\" \"))"))
	}

	/// A method of a string: `text.includes("a")` (arguments in parameter
	/// order, `None` for an omitted one).
	pub(crate) fn string_method(
		&mut self,
		n: NodeId,
		path: &str,
		mut receiver: Expr,
		arguments: Vec<Option<Expr>>,
	) -> Expr {
		let mut arguments = arguments;
		if matches!(path, "String#replace" | "String#replaceAll") {
			let replacement = self
				.analysis
				.call(n)
				.and_then(|info| info.arguments.get(1))
				.and_then(|arg| match arg {
					Argument::Given(node) => Some(*node),
					_ => None,
				});
			let literal = replacement.and_then(|node| match self.view(self.strip_groups(node)) {
				View::String(text) => Some(text.to_string()),
				_ => match self.types.kind(self.type_of(node)) {
					TypeKind::StringLit(sym) => Some(self.types.syms.resolve(sym).to_string()),
					_ => None,
				},
			});
			if let Some(text) = literal.as_ref().filter(|text| text.contains('$')) {
				// Preserve evaluation of a nonliteral expression even if its
				// type determines the text it returns.
				if let Some(value) = arguments
					.get(1)
					.cloned()
					.flatten()
					.filter(|value| !value.pure)
				{
					receiver = self.keep(receiver);
					if let Some(Some(search)) = arguments.get_mut(0) {
						*search = self.keep(search.clone());
					}
					self.out.line(format!("{};", value.text));
				}
				arguments[1] = Some(Expr::constant(
					js::string_literal(&text.replace('$', "$$")),
					Prec::Primary,
				));
			} else if literal.is_none() {
				receiver = self.keep(receiver);
				if let Some(Some(search)) = arguments.get_mut(0) {
					*search = self.keep(search.clone());
				}
				if let Some(Some(replacement)) = arguments.get_mut(1) {
					let value = self.keep(replacement.clone());
					*replacement = Expr::new(format!("() => {}", value.text), Prec::Assign);
				}
			}
		}
		let mut given: Vec<String> = Vec::new();
		// omitted trailing arguments are left out; a middle one is `undefined`
		let last_given = arguments.iter().rposition(Option::is_some);
		for (index, argument) in arguments.into_iter().enumerate() {
			if last_given.is_some_and(|last| index <= last) {
				given.push(argument.map_or_else(|| "undefined".to_string(), |value| value.text));
			}
		}
		let method = |name: &str, receiver: &Expr, given: &[String]| {
			Expr::primary(format!(
				"{}.{name}({})",
				receiver.primary_text(),
				given.join(", ")
			))
		};
		match path {
			"String#toString" => receiver,
			"String#get" | "Bytes#get" => method("at", &receiver, &given),
			"String#isEmpty" | "Bytes#isEmpty" => Expr::new(
				format!("{}.length === 0", receiver.primary_text()),
				Prec::Equality,
			),
			"String#indexOf" | "String#lastIndexOf" => {
				let found = self.keep(method(
					path.trim_start_matches("String#"),
					&receiver,
					&given,
				));
				Expr::new(
					format!("{} === -1 ? undefined : {}", found.text, found.text),
					Prec::Conditional,
				)
			}
			"String#codeUnitAt" => {
				let index = match self
					.analysis
					.call(n)
					.and_then(|info| info.arguments.first())
				{
					Some(Argument::Given(index)) => *index,
					_ => return self.unsupported(n, "a missing code-unit index"),
				};
				self.code_unit(n, index, receiver, Expr::primary(given[0].clone()))
			}
			"String#isWellFormed" if self.options.target == crate::Target::Web => {
				let helper = self.helper("faIsWellFormed");
				js::call(helper, &[receiver.text])
			}
			"String#utf8Length" => {
				let helper = if self.options.target == crate::Target::Web {
					self.helper("faUtf8Length")
				} else {
					"Buffer.byteLength"
				};
				js::call(helper, &[receiver.text])
			}
			"String#toUtf8" => {
				let encoder = self.helper("faUtf8Encoder");
				js::call(&format!("{encoder}.encode"), &[receiver.text])
			}
			"String#characters" | "String#codePoints" => {
				let helper = self.helper(if path == "String#characters" {
					"faCharacters"
				} else {
					"faStringCodePoints"
				});
				js::call(helper, &[receiver.text])
			}
			"Bytes#toArray" => js::call("Array.from", &[receiver.text]),
			"Bytes#toString" => {
				let helper = self.helper("faFormatBytes");
				js::call(helper, &[receiver.text])
			}
			_ => method(path.trim_start_matches("String#"), &receiver, &given),
		}
	}

	/// Read a numeric code unit with exactly the analyzer's bounds check.
	pub(crate) fn code_unit(
		&mut self,
		n: NodeId,
		index: NodeId,
		text: Expr,
		position: Expr,
	) -> Expr {
		let access = self.index_access(n, index);
		let text = if access == IndexAccess::Direct {
			text
		} else {
			self.keep(text)
		};
		let position = match access {
			IndexAccess::Direct => position.text,
			IndexAccess::Checked => {
				let helper = self.helper("faNormalizeIndex");
				format!(
					"{helper}({}.length, {})",
					text.primary_text(),
					position.text
				)
			}
			IndexAccess::FromEnd => {
				let position = self.keep(position);
				format!(
					"{} < 0 ? {} + {}.length : {}",
					position.text,
					position.text,
					text.primary_text(),
					position.text
				)
			}
		};
		Expr::primary(format!("{}.charCodeAt({position})", text.primary_text()))
	}

	/// A string index may be read numerically when its consumer compares units.
	pub(crate) fn indexed_code_unit(&mut self, n: NodeId) -> Expr {
		let n = self.strip_groups(n);
		let View::Index { target, index } = self.view(n) else {
			unreachable!("STRING_CODE_UNIT is an index")
		};
		let values = self.in_order(&[target, index], |g, node| g.expr(node));
		self.code_unit(n, index, values[0].clone(), values[1].clone())
	}

	pub(crate) fn one_unit_literal(&self, node: NodeId) -> Option<u16> {
		let View::String(text) = self.view(self.strip_groups(node)) else {
			return None;
		};
		let mut units = text.encode_utf16();
		let first = units.next()?;
		units.next().is_none().then_some(first)
	}

	/// Direct code-point counting consumes the analyzer fact before building an array.
	pub(crate) fn code_point_count(&mut self, n: NodeId, target: NodeId) -> Option<Expr> {
		if !self.has(n, NodeFlags::CODE_POINT_COUNT) {
			return None;
		}
		let View::Call { callee, .. } = self.view(self.strip_groups(target)) else {
			return None;
		};
		let text = self.receiver(callee);
		let helper = self.helper("faCodePointCount");
		Some(js::call(helper, &[text.text]))
	}
}

/// A template literal of pieces; a single value piece that is already a
/// string stays itself.
fn template_of(pieces: Vec<Piece>) -> Expr {
	let mut text = String::from("`");
	let mut all_text = true;
	for piece in &pieces {
		match piece {
			Piece::Text(part) => text.push_str(&js::template_text(part)),
			Piece::Value(value) => {
				all_text = false;
				text.push_str("${");
				text.push_str(&value.text);
				text.push('}');
			}
		}
	}
	text.push('`');
	if all_text {
		let joined: String = pieces
			.iter()
			.map(|piece| match piece {
				Piece::Text(part) => part.as_str(),
				Piece::Value(_) => "",
			})
			.collect();
		return Expr::constant(js::string_literal(&joined), Prec::Primary);
	}
	Expr::primary(text)
}
