//! Strings and printing (values brief §2.8, §7; advanced brief D39):
//! interpolation, the members of strings, `console.log` and `console.error`.
//!
//! Interpolation is a template literal. A `String`, a `Character`, an integer,
//! a `Float` or a `Boolean` goes in directly (JavaScript's `ToString` is Fa's
//! `toString` for them, D-5); a `Float32`, a `Percentage`, an optional or a
//! composite value goes through a formatter of the runtime.
//! `console.log(a, b)` prints one template literal: the values separated by
//! a space.

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::{NodeId, TypeId};
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
		match self.kind(ty) {
			Kind::Boolean if text == "true" || text == "false" => Some(text.to_string()),
			Kind::Int(storage) if !storage.is_js_bigint() && digits(text) => Some(text.to_string()),
			Kind::Int(_) | Kind::BigInt => text
				.strip_suffix('n')
				.filter(|number| digits(number))
				.map(str::to_string),
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
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => {
				return Some(Ok(self.helper("faQuote").to_string()));
			}
			TypeKind::None => "() => \"none\"".to_string(),
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
			TypeKind::Union(list) => {
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
					Some(Some(first))
						if formats.iter().all(|format| format.as_ref() == Some(first)) =>
					{
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
				format!("(value) => {helper}(value, {inner})")
			}
			TypeKind::Object(_) => {
				let mut fields: Vec<(String, TypeId)> = self
					.types
					.fields_of(ty)
					.iter()
					.map(|field| (self.types.syms.resolve(field.name).to_string(), field.ty))
					.collect();
				// the declaration order, when the type is declared, or the
				// order an inline member's data is written in; otherwise
				// literals of the type may write the fields in any order, which
				// their properties keep: printed from the representation
				let order: Vec<String> =
					match self.analysis.type_decls.values().find(|decl| decl.ty == ty) {
						Some(decl) => decl
							.fields
							.iter()
							.map(|field| self.types.syms.resolve(field.name).to_string())
							.collect(),
						None => self
							.analysis
							.inline_field_orders
							.get(&ty)?
							.iter()
							.map(|&name| self.types.syms.resolve(name).to_string())
							.collect(),
					};
				fields.sort_by_key(|(name, _)| order.iter().position(|field| field == name));
				// printing omits relation sides (D21)
				let relation_sides = self.relation_properties(ty);
				let related = !relation_sides.is_empty();
				fields.retain(|(name, _)| !relation_sides.contains(&Self::property(name)));
				let _ = related;
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
				format!("(value) => {helper}(value, [{}])", entries.join(", "))
			}
			_ => return None,
		};
		Some(Err(code))
	}

	/// `console.log(values…)` and `console.error(values…)`: one line, the
	/// values separated by a space.
	pub(crate) fn console(&mut self, method: &str, values: &[NodeId]) -> Expr {
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
				View::Spread(_) => {
					pieces.push(Piece::Value(
						self.unsupported(value, "spreading into `console.log`"),
					));
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

	/// A method of a string: `text.includes("a")` (arguments in parameter
	/// order, `None` for an omitted one).
	pub(crate) fn string_method(
		&mut self,
		n: NodeId,
		path: &str,
		receiver: Expr,
		arguments: Vec<Option<Expr>>,
	) -> Expr {
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
		let with_receiver = |receiver: Expr, given: Vec<String>| {
			let mut all = vec![receiver.text];
			all.extend(given);
			all
		};
		match path {
			"String#isEmpty" => {
				let length = Expr::primary(format!("{}.length", receiver.primary_text()));
				js::binary(
					&length,
					"===",
					&Expr::constant("0", Prec::Primary),
					Prec::Equality,
				)
			}
			"String#toUpperCase" | "String#toLowerCase" | "String#trim" | "String#trimStart"
			| "String#trimEnd" | "String#repeat" => {
				let name = path.trim_start_matches("String#");
				method(name, &receiver, &given)
			}
			"String#toString" => receiver,
			// read in place: the bytes of the last text are kept
			"String#bytes" if self.bytes_only_read(n) => {
				let helper = self.helper("faStringBytesView");
				js::call(helper, &with_receiver(receiver, given))
			}
			// simple texts (ASCII without `\r`): a character is a code unit
			"String#padStart" | "String#padEnd" if self.simple_padding(n) => {
				let name = path.trim_start_matches("String#");
				method(name, &receiver, &given)
			}
			_ => {
				let helper = match path {
					"String#get" => "faStringGet",
					"String#includes" => "faStringIncludes",
					"String#startsWith" => "faStringStartsWith",
					"String#endsWith" => "faStringEndsWith",
					"String#indexOf" => "faStringIndexOf",
					"String#lastIndexOf" => "faStringLastIndexOf",
					"String#padStart" => "faStringPadStart",
					"String#padEnd" => "faStringPadEnd",
					"String#replace" => "faStringReplace",
					"String#replaceAll" => "faStringReplaceAll",
					"String#split" => "faStringSplit",
					"String#characters" => "faCharacters",
					"String#codePoints" => "faStringCodePoints",
					"String#bytes" => "faStringBytes",
					_ => return self.unsupported(n, &format!("`{path}`")),
				};
				let helper = self.helper(helper);
				js::call(helper, &with_receiver(receiver, given))
			}
		}
	}
}

impl Generator<'_> {
	/// Whether the receiver and the filler of a `padStart`/`padEnd` call are
	/// proven simple texts (`faIsSimpleText`), where a character is one code
	/// unit and JavaScript's method pads as Fa does.
	fn simple_padding(&self, call: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let View::Access { target, .. } = self.view(callee) else {
			return false;
		};
		let filler = match self
			.analysis
			.call(call)
			.and_then(|info| info.arguments.get(1))
		{
			Some(Argument::Given(filler)) => Some(*filler),
			Some(Argument::Default) | None => None,
			Some(_) => return false,
		};
		self.is_simple_text(target) && filler.is_none_or(|filler| self.is_simple_text(filler))
	}

	/// Whether the array of `text.bytes()` (the call `n`) is only read where
	/// it is computed: indexed, read with `get`, measured, or iterated.
	fn bytes_only_read(&mut self, n: NodeId) -> bool {
		let parents = self.parents();
		let Some(parent) = parents[n.index()] else {
			return false;
		};
		match self.view(parent) {
			View::Index { target, .. } if target == n => match parents[parent.index()] {
				Some(grand) => match self.view(grand) {
					View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
						target != parent
					}
					View::Use { .. } | View::Reuse { .. } | View::UpdateFields { .. } => false,
					_ => true,
				},
				None => true,
			},
			View::Access { target, members } if target == n && members.len() == 1 => {
				let name = members.first().map(|member| self.text(member).to_string());
				matches!(name.as_deref(), Some("length" | "get"))
			}
			View::For { iterable, .. } => iterable == n,
			_ => false,
		}
	}

	/// Whether a string value is proven to hold ASCII characters other than
	/// `\r` only: a literal of them, a number or a boolean converted
	/// (`toString()`, interpolation), or a template of these.
	pub(crate) fn is_simple_text(&self, node: NodeId) -> bool {
		let node = self.strip_groups(node);
		let simple = |text: &str| {
			text.bytes()
				.all(|byte| byte < 0x80 && byte != b'\r' && byte != b'\\')
		};
		match self.view(node) {
			View::String(text) => simple(text),
			View::Template { parts } => parts.iter().all(|part| match part {
				TemplatePart::Text(text) => simple(text),
				TemplatePart::Expression(value) => {
					self.is_printed_simply(value) || self.is_simple_text(value)
				}
			}),
			View::Call { callee, arguments } if arguments.is_empty() => {
				let Some(Callee::Builtin(builtin)) =
					self.analysis.call(node).map(|info| info.callee)
				else {
					return false;
				};
				let path = fa_analyzer::builtins::get(builtin).path;
				matches!(
					path,
					"Integer#toString" | "Float#toString" | "BigInt#toString"
				) && matches!(self.view(callee), View::Access { target, .. } if self.is_printed_simply(target))
			}
			_ => false,
		}
	}

	/// Whether a value prints as ASCII: an integer, a float or a boolean.
	fn is_printed_simply(&self, node: NodeId) -> bool {
		matches!(
			self.kind(self.type_of(node)),
			Kind::Int(_) | Kind::BigInt | Kind::Float(FloatBits::F64) | Kind::Boolean
		)
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
