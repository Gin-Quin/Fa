//! `when` (values brief §17.2): the decision list of the analyzer
//! (`fa_analyzer::lower::when`) as an `if` / `else if` chain on the subject,
//! evaluated once. The last arm of an exhaustive `when` needs no test.

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::lower::when::{Decision, Literal, Test, decision_list};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{NominalKind, TypeKind};

use crate::generate::{Dest, Generator, Repr};
use crate::js::{self, Expr, Prec};

/// The number of arms from which a `when` is written without nesting
/// ([`Generator::flat_when`]).
const FLAT_WHEN_ARMS: usize = 64;

impl Generator<'_> {
	/// A `when` whose arms' values go to `dest`.
	pub(crate) fn when_into(&mut self, n: NodeId, dest: Dest) {
		let Ok(list) = decision_list(self.tree, self.analysis, n) else {
			self.internal_error(n, "the checked when has an invalid decision list");
			return;
		};
		let units = self.has(self.strip_groups(list.subject), NodeFlags::STRING_CODE_UNIT)
			&& list.decisions.iter().all(|decision| {
				decision.binding.is_none()
					&& decision.tests.iter().all(|test| match test {
						Test::Equals(Literal::String(text)) => text.encode_utf16().count() == 1,
						_ => false,
					})
			});
		let subject_value = if units {
			self.indexed_code_unit(list.subject)
		} else {
			self.expr(list.subject)
		};
		let subject = if subject_value.stable {
			subject_value
		} else {
			let temp = self.temp();
			self.out
				.line(format!("const {temp} = {};", subject_value.text));
			Expr::name(temp, true)
		};
		let subject_type = list.subject_type;
		let count = list.decisions.len();
		if count >= FLAT_WHEN_ARMS {
			self.flat_when(&list.decisions, &subject, subject_type, &dest, units);
			return;
		}
		let mut opened = false;
		for (position, decision) in list.decisions.iter().enumerate() {
			let last = position + 1 == count;
			let condition = if decision.is_else() || last {
				None
			} else {
				let mut tests = Vec::new();
				for test in &decision.tests {
					tests.push(if units {
						let Test::Equals(Literal::String(text)) = test else {
							unreachable!("single-unit cases")
						};
						js::binary(
							&subject,
							"===",
							&Expr::constant(
								text.encode_utf16().next().unwrap().to_string(),
								Prec::Primary,
							),
							Prec::Equality,
						)
					} else {
						self.when_test(&subject, subject_type, test)
					});
				}
				Some(tests)
			};
			match (&condition, opened) {
				(Some(tests), false) => {
					self.out.open(format!("if ({})", join_or(tests)));
					opened = true;
				}
				(Some(tests), true) => self.out.reopen(&format!("else if ({})", join_or(tests))),
				(None, true) => self.out.reopen("else"),
				(None, false) => self.out.open(""),
			}
			self.when_arm(decision, &subject, &dest);
		}
		if count > 0 {
			self.out.close("");
		}
	}

	/// The body of an arm: its bindings, then its value into `dest`.
	fn when_arm(&mut self, decision: &Decision<'_>, subject: &Expr, dest: &Dest) {
		self.push_scope();
		if let Some(binding) = decision.binding {
			self.bind_arm(binding, subject, decision.tests.first());
		}
		match self.view(decision.value) {
			View::Block { .. } => self.block_into(decision.value, dest.clone()),
			_ => self.into(decision.value, dest.clone()),
		}
		self.pop_scope();
	}

	/// A long `when` (at least [`FLAT_WHEN_ARMS`] arms), written without
	/// nesting: an `else if` chain is a nested statement for the JavaScript
	/// parsers, which overflow their stack on some thousands of arms. When
	/// every test compares the subject with a constant and no arm has a
	/// `break` (which would leave the `switch` instead of the loop), a
	/// `switch`; otherwise a labeled block of `if`s, each arm leaving the
	/// block when it is done.
	fn flat_when(
		&mut self,
		decisions: &[Decision<'_>],
		subject: &Expr,
		subject_type: TypeId,
		dest: &Dest,
		units: bool,
	) {
		let count = decisions.len();
		let cases: Option<Vec<Vec<Expr>>> = if decisions
			.iter()
			.any(|decision| self.has_break(decision.value))
		{
			None
		} else {
			decisions
				.iter()
				.enumerate()
				.map(|(position, decision)| {
					if decision.is_else() || position + 1 == count {
						return Some(Vec::new());
					}
					decision
						.tests
						.iter()
						.map(|test| {
							if units {
								let Test::Equals(Literal::String(text)) = test else {
									return None;
								};
								Some(Expr::constant(
									text.encode_utf16().next()?.to_string(),
									Prec::Primary,
								))
							} else {
								self.case_constant(subject_type, test)
							}
						})
						.collect()
				})
				.collect()
		};
		if let Some(cases) = cases {
			self.out.open(format!("switch ({})", subject.text));
			for (decision, values) in decisions.iter().zip(cases) {
				let header = if values.is_empty() {
					"default:".to_string()
				} else {
					values
						.iter()
						.map(|value| format!("case {}:", value.text))
						.collect::<Vec<_>>()
						.join(" ")
				};
				self.out.open(header);
				self.when_arm(decision, subject, dest);
				self.out.line("break;");
				self.out.close("");
			}
			self.out.close("");
			return;
		}
		let label = self.temp();
		self.out.open(format!("{label}:"));
		for (position, decision) in decisions.iter().enumerate() {
			if decision.is_else() || position + 1 == count {
				self.out.open("");
				self.when_arm(decision, subject, dest);
				self.out.close("");
				break;
			}
			let mut tests = Vec::new();
			for test in &decision.tests {
				tests.push(if units {
					let Test::Equals(Literal::String(text)) = test else {
						unreachable!("unit literals")
					};
					js::binary(
						subject,
						"===",
						&Expr::constant(
							text.encode_utf16().next().unwrap().to_string(),
							Prec::Primary,
						),
						Prec::Equality,
					)
				} else {
					self.when_test(subject, subject_type, test)
				});
			}
			self.out.open(format!("if ({})", join_or(&tests)));
			self.when_arm(decision, subject, dest);
			self.out.line(format!("break {label};"));
			self.out.close("");
		}
		self.out.close("");
	}

	/// Whether a node has a `break` outside of the loops it contains (the
	/// loops and functions it holds keep their own `break`s).
	fn has_break(&self, n: NodeId) -> bool {
		let mut pending = vec![n];
		while let Some(node) = pending.pop() {
			match self.view(node) {
				View::Break(_) => return true,
				View::For { .. }
				| View::While { .. }
				| View::WhileLet { .. }
				| View::Loop(_)
				| View::Function { .. }
				| View::Lambda { .. } => continue,
				_ => {}
			}
			pending.extend(fa_analyzer::syntax::children(self.tree, node));
		}
		false
	}

	/// The constant a test compares the subject with (`subject === value`),
	/// for a `case` of a `switch`; `None` for another test.
	fn case_constant(&mut self, subject_type: TypeId, test: &Test<'_>) -> Option<Expr> {
		match *test {
			Test::Equals(literal) => Some(self.literal_constant(subject_type, literal)),
			Test::IsType(ty) => match self.types.kind(ty) {
				TypeKind::EnumMember(nominal, index) => Some(self.enum_value(nominal, index)),
				TypeKind::None => Some(Expr::constant("undefined", Prec::Primary)),
				_ => None,
			},
			Test::InRange { .. } => None,
		}
	}

	/// The JavaScript constant of a literal compared with a subject.
	fn literal_constant(&mut self, subject_type: TypeId, literal: Literal<'_>) -> Expr {
		match literal {
			Literal::Integer(value) => {
				if self.repr(subject_type) == Repr::BigInt || self.union_has_bigint(subject_type) {
					Expr::constant(format!("{value}n"), Prec::Primary)
				} else {
					Expr::constant(value.to_string(), Prec::Primary)
				}
			}
			Literal::Float(value) => js::float_literal(value),
			Literal::String(text) => Expr::constant(js::string_literal(text), Prec::Primary),
			Literal::Boolean(value) => Expr::constant(value.to_string(), Prec::Primary),
			Literal::None => Expr::constant("undefined", Prec::Primary),
		}
	}

	/// The JavaScript test of one way to match an arm.
	fn when_test(&mut self, subject: &Expr, subject_type: TypeId, test: &Test<'_>) -> Expr {
		let equals = |value: Expr| js::binary(subject, "===", &value, Prec::Equality);
		match *test {
			Test::Equals(literal) => {
				let value = self.literal_constant(subject_type, literal);
				equals(value)
			}
			Test::InRange {
				start,
				end,
				inclusive,
			} => {
				let mut parts = Vec::new();
				if let Some(start) = start {
					let start = self.expr(start);
					parts.push(js::binary(&start, "<=", subject, Prec::Relational));
				}
				if let Some(end) = end {
					let end = self.expr(end);
					let operator = if inclusive { "<=" } else { "<" };
					parts.push(js::binary(subject, operator, &end, Prec::Relational));
				}
				let mut iter = parts.into_iter();
				let first = iter
					.next()
					.unwrap_or_else(|| Expr::constant("true", Prec::Primary));
				iter.fold(first, |a, b| js::binary(&a, "&&", &b, Prec::And))
			}
			Test::IsType(ty) => self.type_test(subject, subject_type, ty),
		}
	}

	/// `value is pattern` (`negated`: `is not`): a type, a literal, a range,
	/// alternatives `A | B`; the value evaluated once.
	pub(crate) fn is_expression(&mut self, value: NodeId, pattern: NodeId, negated: bool) -> Expr {
		let subject_type = self.type_of(value);
		let subject = self.expr(value);
		let subject = if subject.stable || !self.has_alternatives(pattern) {
			subject
		} else {
			let (first, later) = self.twice(subject);
			// the first test evaluates the value, the others read it again
			let test = self.pattern_test(&first, subject_type, pattern, Some(&later));
			return if negated {
				js::prefix("!", &test)
			} else {
				test
			};
		};
		let test = self.pattern_test(&subject, subject_type, pattern, None);
		if negated {
			js::prefix("!", &test)
		} else {
			test
		}
	}

	/// Whether a pattern tests the value several times.
	fn has_alternatives(&self, pattern: NodeId) -> bool {
		matches!(
			self.view(self.strip_groups(pattern)),
			View::Union { .. }
				| View::Range {
					start: Some(_),
					end: Some(_),
					..
				}
		)
	}

	/// The test of one pattern of `is`; `later` reads the value after the first test.
	fn pattern_test(
		&mut self,
		subject: &Expr,
		subject_type: TypeId,
		pattern: NodeId,
		later: Option<&Expr>,
	) -> Expr {
		let pattern = self.strip_groups(pattern);
		if self.analysis.fields_patterns.contains(&pattern) {
			let expected = self.expr(pattern);
			return js::binary(subject, "===", &expected, Prec::Equality);
		}
		match self.view(pattern) {
			View::Union { operands } => {
				let mut tests = Vec::new();
				for (index, operand) in operands.iter().enumerate() {
					let current = if index == 0 {
						subject
					} else {
						later.unwrap_or(subject)
					};
					tests.push(self.pattern_test(current, subject_type, operand, later));
				}
				let mut iter = tests.into_iter();
				let first = iter
					.next()
					.unwrap_or_else(|| Expr::constant("false", Prec::Primary));
				iter.fold(first, |a, b| js::binary(&a, "||", &b, Prec::Or))
			}
			View::Range {
				start,
				end,
				inclusive,
			} => {
				let mut parts = Vec::new();
				if let Some(start) = start {
					let start = self.expr(start);
					parts.push(js::binary(&start, "<=", subject, Prec::Relational));
				}
				if let Some(end) = end {
					let end = self.expr(end);
					let operator = if inclusive { "<=" } else { "<" };
					let current = if parts.is_empty() {
						subject
					} else {
						later.unwrap_or(subject)
					};
					parts.push(js::binary(current, operator, &end, Prec::Relational));
				}
				let mut iter = parts.into_iter();
				let first = iter
					.next()
					.unwrap_or_else(|| Expr::constant("true", Prec::Primary));
				iter.fold(first, |a, b| js::binary(&a, "&&", &b, Prec::And))
			}
			_ => {
				let ty = self.type_of(pattern);
				let literal = !matches!(
					self.types.kind(ty),
					TypeKind::Member(..) | TypeKind::EnumMember(..) | TypeKind::None
				) && self.types.is_literal(ty);
				if literal && let Some(value) = self.literal_of(ty) {
					let value = if self.repr(subject_type) == Repr::BigInt
						&& self.repr(ty) == Repr::Number
					{
						Expr::constant(format!("{}n", value.text), Prec::Primary)
					} else {
						value
					};
					return js::binary(subject, "===", &value, Prec::Equality);
				}
				self.type_test(subject, subject_type, ty)
			}
		}
	}

	/// Whether a union type has a bigint member (its integer literals are bigints).
	fn union_has_bigint(&self, ty: TypeId) -> bool {
		self.types
			.union_members(&ty)
			.iter()
			.any(|&member| self.repr(member) == Repr::BigInt)
	}

	/// `subject is ty` for a member of a named union or enumeration, or a
	/// member type of an anonymous union.
	fn type_test(&mut self, subject: &Expr, subject_type: TypeId, ty: TypeId) -> Expr {
		if let Some(test) = self.error_test(subject, ty) {
			return test;
		}
		let may_be_none = self
			.types
			.union_members(&subject_type)
			.contains(&TypeId::NONE);
		match self.types.kind(ty) {
			TypeKind::Member(nominal, index) => {
				let payload = self.types.payload(nominal, index);
				if let Some(literal) = self.literal_of(payload).filter(|_| !payload.is_unset()) {
					return js::binary(subject, "===", &literal, Prec::Equality);
				}
				let tag = js::string_literal(&self.member_qualified_name(nominal, index));
				let dot = if may_be_none { "?." } else { "." };
				Expr::new(
					format!("{}{dot}$t === {tag}", subject.primary_text()),
					Prec::Equality,
				)
			}
			TypeKind::EnumMember(nominal, index) => {
				let value = self.enum_value(nominal, index);
				js::binary(subject, "===", &value, Prec::Equality)
			}
			TypeKind::Bytes => {
				let helper = self.helper("faIsBytes");
				js::call(helper, std::slice::from_ref(&subject.text))
			}
			TypeKind::Collection(fa_analyzer::types::CollKind::Array, ..) | TypeKind::Tuple(_) => {
				js::call("Array.isArray", std::slice::from_ref(&subject.text))
			}
			TypeKind::None => js::binary(
				subject,
				"===",
				&Expr::constant("undefined", Prec::Primary),
				Prec::Equality,
			),
			TypeKind::NamedUnion(nominal, _)
				if self.types.nominal(nominal).kind == NominalKind::Union =>
			{
				let prefix = format!("{}.", self.types.decl_name(self.types.nominal(nominal).key));
				Expr::new(
					format!(
						"{}?.$t?.startsWith({}) === true",
						subject.primary_text(),
						js::string_literal(&prefix)
					),
					Prec::Equality,
				)
			}
			_ => {
				let kind = match self.repr(ty) {
					Repr::Number => "number",
					Repr::BigInt => "bigint",
					Repr::String => "string",
					Repr::Boolean => "boolean",
					_ => "object",
				};
				Expr::new(
					format!("typeof {} === \"{kind}\"", subject.at(Prec::Unary)),
					Prec::Equality,
				)
			}
		}
	}

	/// The names an arm binds with `as`: the subject (or the data of a
	/// member that holds no object), or its fields.
	fn bind_arm(&mut self, binding: NodeId, subject: &Expr, test: Option<&Test<'_>>) {
		// a member holding data that is not an object binds its data
		let data = match test {
			Some(Test::IsType(ty)) => match self.types.kind(*ty) {
				TypeKind::Member(nominal, index) => {
					let payload = self.types.payload(nominal, index);
					!payload.is_unset()
						&& !self.types.is_literal(payload)
						&& !matches!(self.types.kind(payload), TypeKind::Object(_))
				}
				_ => false,
			},
			_ => false,
		};
		let value = if data {
			format!("{}.$v", subject.primary_text())
		} else {
			subject.text.clone()
		};
		if !matches!(
			self.view(binding),
			View::Name(_) | View::ObjectPattern { .. }
		) {
			self.unsupported(binding, "this binding");
			return;
		}
		// paired by name (`fa_analyzer::lower::when::arm_bindings`)
		for bound in fa_analyzer::lower::when::arm_bindings(self.tree, self.analysis, binding) {
			let name = self.declare(bound.symbol);
			match bound.field {
				Some(field) => {
					let property = Self::property(self.tree.name(field));
					self.out.line(format!(
						"const {name} = {}.{property};",
						subject.primary_text()
					));
				}
				None => self.out.line(format!("const {name} = {value};")),
			}
		}
	}
}

/// Tests joined with `||`.
fn join_or(tests: &[Expr]) -> String {
	let mut iter = tests.iter();
	let Some(first) = iter.next() else {
		return "true".to_string();
	};
	let mut result = first.clone();
	for test in iter {
		result = js::binary(&result, "||", test, Prec::Or);
	}
	result.text
}
