//! Strings and printing (values brief §2.8, §7; advanced brief D39):
//! interpolation, the members of strings, `console.log` and `console.error`.
//!
//! Interpolation is a template literal. A `String`, a `Character`, an integer,
//! a `Float` or a `Boolean` goes in directly (JavaScript's `ToString` is Fa's
//! `toString` for them, D-5); a `Float32`, a `Percentage`, an optional or a
//! composite value goes through a formatter of the runtime.
//! `console.log(a, b)` prints one template literal: the values separated by
//! a space.

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
					let ty = self.type_of(node);
					pieces.push(self.top_level_piece(value, ty));
				}
			}
		}
		template_of(pieces)
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
				_ => {
					let helper = self.helper("faFormat");
					js::call(helper, &[value.text])
				}
			},
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
				let formatted = self.format_top(value.clone(), ty);
				match self.kind(ty) {
					Kind::Other => {
						let helper = self.helper("faFormatNested");
						js::call(helper, std::slice::from_ref(&value.text)).text
					}
					_ => formatted.text,
				}
			}
		}
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
								let ty = self.type_of(node);
								let piece = self.top_level_piece(value, ty);
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
					let ty = self.type_of(value);
					let piece = self.top_level_piece(value_expr, ty);
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
					_ => return self.unsupported(n, &format!("`{path}`")),
				};
				let helper = self.helper(helper);
				js::call(helper, &with_receiver(receiver, given))
			}
		}
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
