//! Patterns: destructuring declarations, object, array and tuple patterns
//! (`when` patterns are handled by `when`, with exhaustiveness).

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::types::TY057;
use crate::diagnostics::codes::values::{
	E_OBJ_5, E_RNG_4, E_WHEN_1, E_WHEN_3, E_WHEN_5, E_WHEN_8, W_WHEN_1, W_WHEN_2,
};
use crate::flow::FlowState;
use crate::ids::{NodeId, TypeId};
use crate::symbols::{ScopeKind, SymbolKind};
use crate::syntax::{self, BindingKind, Name, NodeList, PatternFields, View, WhenArms};
use crate::types::TypeKind;

impl Walker<'_> {
	/// `let { a, b } = object` (destructuring.rs).
	pub(super) fn check_destructuring(
		&mut self,
		n: NodeId,
		kind: BindingKind,
		pattern: NodeId,
		value: NodeId,
	) -> TypeId {
		self.check_destructuring_declaration(n, kind, pattern, value)
	}

	/// The names a pattern binds, in source order.
	pub(super) fn pattern_names(&self, pattern: NodeId, names: &mut Vec<Name>) {
		match self.view(pattern) {
			View::Name(name) => names.push(name),
			View::ObjectPattern { fields, rest } => {
				for field in fields.iter() {
					match field.nested {
						Some(nested) => self.pattern_names(nested, names),
						None => names.push(field.alias.unwrap_or(field.name)),
					}
				}
				if let Some(rest) = rest {
					self.pattern_names(rest, names);
				}
			}
			View::ArrayPattern { items, rest } => {
				for item in items.iter() {
					self.pattern_names(item, names);
				}
				if let Some(rest) = rest {
					self.pattern_names(rest, names);
				}
			}
			View::TuplePattern { items } | View::Tuple { items } => {
				for item in items.iter() {
					self.pattern_names(item, names);
				}
			}
			View::Spread(inner) | View::Group(inner) => self.pattern_names(inner, names),
			_ => {}
		}
	}

	/// `{ a, b as c, d = 4, nested { e }, ...rest }`
	pub(super) fn check_object_pattern(
		&mut self,
		n: NodeId,
		_fields: PatternFields<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "object patterns")
	}

	/// `[first, second, ...others]`
	pub(super) fn check_array_pattern(
		&mut self,
		n: NodeId,
		_items: NodeList<'_>,
		_rest: Option<NodeId>,
	) -> TypeId {
		self.unsupported(n, "array patterns")
	}

	/// `(number, name)`
	pub(super) fn check_tuple_pattern(&mut self, n: NodeId, _items: NodeList<'_>) -> TypeId {
		self.unsupported(n, "tuple patterns")
	}
}

// ── `when` (values brief §17.2): typing of the arms and of their patterns ──

/// A `when` being checked: what its arms cover, for exhaustiveness.
#[derive(Default)]
struct WhenCoverage {
	/// The type of every pattern (alternatives flattened).
	covered: Vec<TypeId>,
	/// Whether an arm is `else`.
	has_else: bool,
	/// The covered types, for `W-WHEN-1` in linear time: each one exactly,
	/// and the members of named unions and enumerations by mask.
	exact: rustc_hash::FxHashSet<TypeId>,
	masks: Vec<(crate::ids::NominalId, crate::ids::MaskId)>,
}

impl Walker<'_> {
	/// `when subject is { pattern [as binding] => value … }` (called by
	/// `control.rs`): each arm in its own scope (the binding), from the flow
	/// state before the `when`; the value is the join of the arms (as `if`),
	/// `Void` in statement position. Exhaustiveness: [`Walker::check_when_exhaustive`].
	pub(super) fn check_when_value(
		&mut self,
		n: NodeId,
		subject: NodeId,
		arms: WhenArms<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let subject_ty = self.check(subject, None);
		// the values the arms must cover: a local narrowed to a single member
		// is that member (its facts, values brief D-55), and so is a local
		// declared with a member type (`Shape - Shape.Point`)
		let narrowed = self
			.a
			.flags_of(self.strip_groups(subject))
			.contains(NodeFlags::NARROWED);
		let declared_member = match self.a.resolution(self.strip_groups(subject)) {
			crate::symbols::Resolution::Local(symbol) => {
				self.a.symbols.symbol(symbol).ty == subject_ty
			}
			_ => false,
		};
		// a member of a named union is a value of the whole union (D53)
		let joined = self.join_member(subject_ty);
		let covered_ty = if narrowed || declared_member {
			subject_ty
		} else {
			joined
		};
		let subject_ty = joined;
		let statement = self.is_statement(n);
		let mut entry = self.body.flow.fork();
		// the state the previous arms leave (narrowing.rs)
		let mut remaining = entry.fork();
		let mut integer_patterns: Vec<NodeId> = Vec::new();
		let mut ends: Vec<FlowState> = Vec::new();
		let mut branches: Vec<(NodeId, TypeId)> = Vec::new();
		let mut coverage = WhenCoverage::default();
		let count = arms.len();
		for (position, arm) in arms.iter().enumerate() {
			self.body.flow = remaining.fork();
			let matched = match arm.pattern {
				// a value of a set of flags: compared, covers no type (fields.rs)
				Some(pattern)
					if let Some(fields) = self.fields_when_pattern(pattern, subject_ty) =>
				{
					fields
				}
				Some(pattern) => {
					let ty = self.check_when_pattern(pattern, subject_ty);
					// the subject narrowed in the arm, and what is left (narrowing.rs)
					self.when_arm(n, subject, pattern, &mut remaining);
					// `W-WHEN-1`: by the types matched above (members, literals),
					// or by the values left (integers)
					if self.covered_above(ty, &coverage) {
						self.report_unreachable_arm(pattern);
					} else if !self.body.flow.reachable
						&& entry.reachable && self
						.s
						.types
						.union_members(&subject_ty)
						.iter()
						.all(|&part| matches!(self.s.types.kind(part), TypeKind::Int(_)))
					{
						self.unreachable_arm(n, subject, pattern, &mut entry);
					}
					integer_patterns.push(pattern);
					let members: Vec<TypeId> = self.s.types.union_members(&ty).to_vec();
					for member in members {
						self.note_covered(&mut coverage, member);
					}
					ty
				}
				None => {
					coverage.has_else = true;
					// the arms above handle every value of the subject's type,
					// whatever is known of it (`W-WHEN-2`)
					if position > 0
						&& entry.reachable && self.covers_every_member(subject_ty, &coverage.covered)
					{
						let diagnostic = self.diagnostic(
							&W_WHEN_2,
							arm.value,
							"this `else` never runs: the other arms handle every value. Remove it",
						);
						self.report_flow(diagnostic);
					}
					if position + 1 != count {
						let diagnostic = self.diagnostic(
							&E_WHEN_8,
							arm.value,
							"`else` is the last arm of a `when`",
						);
						self.report_once(arm.value, diagnostic);
					}
					subject_ty
				}
			};
			let parent = self.body.scope;
			let scope = self
				.a
				.symbols
				.scope_for(arm.value, Some(parent), ScopeKind::Block);
			self.body.scope = scope;
			// `else as x` is `E-WHEN-6`, reported by the parser
			if let Some(binding) = arm.binding
				&& arm.pattern.is_some()
			{
				self.bind_pattern(binding, matched);
			}
			if statement {
				self.check_stmt(arm.value);
			} else {
				let (ty, _) = self.check_block_value(arm.value, expected);
				branches.push((arm.value, ty));
				if let Some(expected) = expected
					&& ty != TypeId::NEVER
				{
					let value = match self.view(arm.value) {
						View::Block { statements } => statements.last().unwrap_or(arm.value),
						_ => arm.value,
					};
					self.sink(value, ty, expected);
				}
			}
			self.body.scope = parent;
			if self.body.flow.reachable {
				ends.push(self.body.flow.fork());
			}
		}
		// the flow after the `when`: the join of the arms that reach its end
		let mut exit = if coverage.has_else || !statement {
			FlowState::unreachable()
		} else {
			remaining.fork()
		};
		// integers covered by literals and ranges, with the facts (narrowing.rs)
		if !coverage.has_else {
			let saved = std::mem::replace(&mut self.body.flow, entry.fork());
			self.integer_coverage(n, subject, &integer_patterns);
			self.body.flow = saved;
		}
		for end in &ends {
			exit = if exit.reachable {
				self.join_states(&exit, end)
			} else {
				end.clone()
			};
		}
		self.body.flow = exit;
		self.memo(n, |w| {
			w.check_when_exhaustive(n, covered_ty, &coverage.covered, coverage.has_else);
			if statement {
				return TypeId::VOID;
			}
			w.branches_type(n, &branches, expected)
		})
	}

	/// The type a pattern matches (type part of the pattern): a literal, a
	/// type, a member of a named union or an enumeration, alternatives
	/// `A | B`. `E-WHEN-3` for a pattern that can never match the subject.
	fn check_when_pattern(&mut self, pattern: NodeId, subject: TypeId) -> TypeId {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, pattern) {
			View::Union { operands } => {
				let members: Vec<TypeId> = operands
					.iter()
					.map(|operand| self.check_when_pattern(operand, subject))
					.collect();
				self.memo(pattern, |w| {
					if members
						.iter()
						.any(|&member| w.s.types.is_recovering(member))
					{
						return TypeId::UNKNOWN;
					}
					w.s.types.union(&members)
				})
			}
			View::Integer(_) | View::String(_) | View::Boolean(_) | View::None | View::Float(_) => {
				let literal = self.check(pattern, Some(subject));
				self.pattern_overlap(pattern, literal, subject);
				literal
			}
			View::Range { .. } => {
				self.check(pattern, None);
				// an integer, or the integers of an optional one (`Integer?`)
				match self.int_target(subject) {
					Some(integer) => integer,
					None if self.s.types.is_recovering(subject) => subject,
					None => {
						let diagnostic = self.diagnostic(
							&E_RNG_4,
							pattern,
							"a range pattern matches integers only",
						);
						self.report_once(pattern, diagnostic);
						subject
					}
				}
			}
			View::Name(name) if self.unqualified_member(name, subject).is_some() => {
				let text = self.tree.name(name).to_string();
				let qualified = self.unqualified_member(name, subject).unwrap_or_default();
				let diagnostic = self.diagnostic(
					&TY057,
					pattern,
					format!("members are always qualified: write `{qualified}.{text}`"),
				);
				self.report_once(pattern, diagnostic);
				TypeId::UNKNOWN
			}
			View::Name(_)
			| View::Access { .. }
			| View::Call { .. }
			| View::Optional(_)
			| View::ErrorType => {
				let ty = self.check_type_expr(pattern);
				self.pattern_overlap(pattern, ty, subject);
				ty
			}
			View::ImplicitMember(name) => {
				let ty = self.check_implicit_member(pattern, name, Some(subject));
				self.pattern_overlap(pattern, ty, subject);
				ty
			}
			View::Tuple { .. } | View::TuplePattern { .. } | View::Object { .. } => {
				let diagnostic = self.diagnostic(
					&E_WHEN_5,
					pattern,
					"`when` matches a type, a member or a literal: bind the value with `as` and test its parts inside the arm",
				);
				self.report_once(pattern, diagnostic);
				TypeId::UNKNOWN
			}
			View::Error => TypeId::UNKNOWN,
			_ => self.unsupported(pattern, "this pattern"),
		}
	}

	/// The name of the subject's named union or enumeration when `name`, not
	/// declared, is one of its members (`TY057`: members are qualified).
	fn unqualified_member(&mut self, name: Name, subject: TypeId) -> Option<String> {
		if self.is_declared(name) {
			return None;
		}
		let text = self.tree.name(name).to_string();
		self.s
			.types
			.union_members(&subject)
			.to_vec()
			.into_iter()
			.find_map(|member| {
				let nominal = match self.s.types.kind(member) {
					TypeKind::NamedUnion(nominal, _)
					| TypeKind::Member(nominal, _)
					| TypeKind::Enum(nominal, _)
					| TypeKind::EnumMember(nominal, _) => nominal,
					_ => return None,
				};
				self.s.types.member_index(nominal, &text)?;
				Some(
					self.s
						.types
						.decl_name(self.s.types.nominal(nominal).key)
						.to_string(),
				)
			})
	}

	/// `E-WHEN-3` when no value of the subject's type matches the pattern's type.
	fn pattern_overlap(&mut self, pattern: NodeId, ty: TypeId, subject: TypeId) {
		if self.s.types.is_recovering(ty) || self.s.types.is_recovering(subject) {
			return;
		}
		let members: Vec<TypeId> = self.s.types.union_members(&subject).to_vec();
		let overlaps = members.iter().any(|&member| {
			self.s.types.satisfies(ty, member) || self.s.types.satisfies(member, ty)
		}) || self.s.types.satisfies(ty, subject)
			|| self.s.types.satisfies(subject, ty);
		if overlaps {
			return;
		}
		let message = format!(
			"a `{}` never matches `{}`",
			self.display(subject),
			self.display(ty)
		);
		let diagnostic = self.diagnostic(&E_WHEN_3, pattern, message);
		self.report_once(pattern, diagnostic);
	}

	/// Whether patterns cover every member of a union-like subject (named
	/// unions, enumerations, `Boolean`, literal unions; never integers).
	fn covers_every_member(&mut self, subject: TypeId, covered: &[TypeId]) -> bool {
		if self.s.types.is_recovering(subject)
			|| covered.iter().any(|&ty| self.s.types.is_recovering(ty))
			|| self
				.members_of(subject)
				.iter()
				.any(|&member| self.s.types.as_int(member).is_some())
		{
			return false;
		}
		if !matches!(
			self.s.types.kind(subject),
			TypeKind::NamedUnion(..) | TypeKind::Enum(..) | TypeKind::Boolean | TypeKind::Union(_)
		) {
			return false;
		}
		let mut missing = Vec::new();
		self.when_missing(subject, covered, &mut missing, true);
		missing.is_empty()
	}

	/// `W-WHEN-1`: an arm no value reaches. Reported when the earlier arms
	/// match all its values (not when the facts known before the `when`
	/// exclude them).
	fn unreachable_arm(
		&mut self,
		n: NodeId,
		subject: NodeId,
		pattern: NodeId,
		entry: &mut FlowState,
	) {
		let saved = std::mem::replace(&mut self.body.flow, entry.fork());
		let (matching, _) = self.pattern_test(n, subject, pattern, None);
		self.body.flow = saved;
		if matching.reachable {
			self.report_unreachable_arm(pattern);
		}
	}

	/// Records a type a pattern matches.
	fn note_covered(&mut self, coverage: &mut WhenCoverage, ty: TypeId) {
		coverage.covered.push(ty);
		coverage.exact.insert(ty);
		if let Some((nominal, mask, _)) = self.s.types.nominal_mask(ty) {
			match coverage
				.masks
				.iter_mut()
				.find(|(other, _)| *other == nominal)
			{
				Some((_, union)) => *union = self.s.types.mask_union(*union, mask),
				None => coverage.masks.push((nominal, mask)),
			}
		}
	}

	/// Whether the arms above match every value of a pattern's type (a
	/// literal, a member, a subset of members; not integers).
	fn covered_above(&mut self, ty: TypeId, coverage: &WhenCoverage) -> bool {
		if coverage.covered.is_empty() || ty.is_unset() || self.s.types.is_recovering(ty) {
			return false;
		}
		let parts: Vec<TypeId> = self.s.types.union_members(&ty).to_vec();
		// integers: by the values left (`unreachable_arm`)
		if parts
			.iter()
			.any(|&part| self.s.types.as_int(part).is_some())
		{
			return false;
		}
		parts.into_iter().all(|part| {
			// a literal matches one value (a pattern on a `Character` is typed
			// `Character`: it matches one character, not every one)
			if matches!(
				self.s.types.kind(part),
				TypeKind::StringLit(_) | TypeKind::BoolLit(_) | TypeKind::None
			) {
				return coverage.exact.contains(&part);
			}
			// a pattern's type may be wider than what it matches (a
			// character literal is a `Character`): only exact types and
			// members count
			match self.s.types.nominal_mask(part) {
				Some((nominal, mask, _)) => coverage.masks.iter().any(|&(other, union)| {
					other == nominal && self.s.types.mask_subset(mask, union)
				}),
				None => false,
			}
		})
	}

	/// Reports `W-WHEN-1` on an arm's pattern.
	fn report_unreachable_arm(&mut self, pattern: NodeId) {
		let text = self.tree.text(pattern.index()).to_string();
		let diagnostic = self.diagnostic(
			&W_WHEN_1,
			pattern,
			format!("this arm never runs: the arms above already match every value of `{text}`. Remove it, or move it up"),
		);
		self.report_flow(diagnostic);
	}

	/// Reports a type diagnostic attached to a node checked several times
	/// (a pattern in a loop): once.
	pub(super) fn report_once(&mut self, n: NodeId, diagnostic: crate::diagnostics::Diagnostic) {
		if self.reported_once.insert((n, diagnostic.code.id)) {
			self.report(diagnostic);
		}
	}

	/// `as name` or `as { a, b }`: binds the matched value in the arm's scope.
	fn bind_pattern(&mut self, binding: NodeId, matched: TypeId) {
		// a member whose data is not an object binds its data (types brief D4)
		let matched = match self.member_data(matched) {
			Some(data) if !matches!(self.s.types.kind(data), TypeKind::Object(_)) => data,
			_ => matched,
		};
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, binding) {
			View::Name(name) => {
				self.memo(binding, |w| {
					if !name.is_empty() {
						let symbol = w.declare(binding, name, SymbolKind::PatternBinding);
						w.a.symbols.symbol_mut(symbol).ty = matched;
					}
					matched
				});
			}
			View::ObjectPattern { fields, rest: None } => {
				self.memo(binding, |w| {
					for field in fields.iter() {
						if field.nested.is_some() || field.default.is_some() {
							let message = "nested patterns and defaults in `as` are not supported yet by the analyzer";
							let diagnostic = w.diagnostic(&AN001, binding, message);
							w.report(diagnostic);
							continue;
						}
						let text = w.tree.name(field.name).to_string();
						let ty = w.pattern_field_type(binding, matched, &text);
						let bound = field.alias.unwrap_or(field.name);
						let symbol = w.declare(binding, bound, SymbolKind::PatternBinding);
						w.a.symbols.symbol_mut(symbol).ty = ty;
					}
					matched
				});
			}
			_ => {
				self.unsupported(binding, "this binding");
			}
		}
	}

	/// The type of the field `name` of every member of a matched type (`E-OBJ-5`).
	fn pattern_field_type(&mut self, at: NodeId, matched: TypeId, name: &str) -> TypeId {
		if self.s.types.is_recovering(matched) {
			return TypeId::UNKNOWN;
		}
		let members: Vec<TypeId> = self.s.types.union_members(&matched).to_vec();
		let mut types = Vec::new();
		for member in members {
			let object = match self.s.types.kind(member) {
				TypeKind::Member(nominal, index) => self.s.types.payload(nominal, index),
				_ => member,
			};
			let found = self
				.s
				.types
				.fields_of(object)
				.iter()
				.find(|field| self.s.types.syms.resolve(field.name) == name)
				.map(|field| field.ty);
			match found {
				Some(ty) => types.push(ty),
				None => {
					let message = format!("a `{}` has no field `{name}`", self.display(member));
					let diagnostic = self.diagnostic(&E_OBJ_5, at, message);
					self.report(diagnostic);
					return TypeId::UNKNOWN;
				}
			}
		}
		self.s.types.union(&types)
	}

	/// `E-WHEN-1` for a `when` without `else` that doesn't cover every value
	/// of its subject: the members of named unions, `Errors` types and
	/// enumerations, `true` and `false`, the members of anonymous unions.
	/// Minimal (E1): integers by ranges and facts are E2's (values brief
	/// D-55); other types need an `else`. Type part.
	pub(super) fn check_when_exhaustive(
		&mut self,
		n: NodeId,
		subject: TypeId,
		covered: &[TypeId],
		has_else: bool,
	) {
		if has_else || self.s.types.is_recovering(subject) {
			return;
		}
		if covered.iter().any(|&ty| self.s.types.is_recovering(ty)) {
			return;
		}
		let mut missing: Vec<String> = Vec::new();
		self.when_missing(subject, covered, &mut missing, true);
		if missing.is_empty() {
			return;
		}
		let message = format!("this `when` doesn't handle {}", missing.join(", "));
		let diagnostic = self.diagnostic(&E_WHEN_1, n, message);
		self.report(diagnostic);
	}

	/// The values of `subject` no pattern of `covered` handles, for `E-WHEN-1`
	/// (`top`: the subject itself, not a member of it).
	fn when_missing(
		&mut self,
		subject: TypeId,
		covered: &[TypeId],
		missing: &mut Vec<String>,
		top: bool,
	) {
		let covers = |w: &mut Self, ty: TypeId| {
			covered
				.iter()
				.any(|&pattern| w.s.types.satisfies(ty, pattern))
		};
		if covers(self, subject) {
			return;
		}
		match self.s.types.kind(subject) {
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				let enumeration = matches!(self.s.types.kind(subject), TypeKind::Enum(..));
				let count = self
					.s
					.types
					.sym_list(self.s.types.nominal(nominal).members)
					.len() as u16;
				// the members the patterns of this type cover, as one mask
				let empty = self.s.types.mask(&[]);
				let mut handled = empty;
				for &pattern in covered {
					if let Some((other, pattern_mask, _)) = self.s.types.nominal_mask(pattern)
						&& other == nominal
					{
						handled = self.s.types.mask_union(handled, pattern_mask);
					}
				}
				for index in 0..count {
					if !self.s.types.mask_has(mask, index) || self.s.types.mask_has(handled, index)
					{
						continue;
					}
					let member = if enumeration {
						let mask = self.s.types.single_mask(index);
						self.s.types.enumeration(nominal, mask)
					} else {
						self.s.types.member(nominal, index)
					};
					if !covers(self, member) {
						missing.push(format!("`{}`", self.display(member)));
					}
				}
			}
			TypeKind::Boolean => {
				for literal in [TypeId::TRUE, TypeId::FALSE] {
					if !covers(self, literal) {
						missing.push(format!("`{}`", self.display(literal)));
					}
				}
			}
			TypeKind::Union(list) => {
				let members = self.s.types.list_items(list).to_vec();
				for member in members {
					// integers covered by literals and ranges: E2 (facts)
					if self.s.types.as_int(member).is_some() {
						continue;
					}
					// a named union, an enumeration or `Boolean` inside: by member
					if matches!(
						self.s.types.kind(member),
						TypeKind::NamedUnion(..) | TypeKind::Enum(..) | TypeKind::Boolean
					) {
						self.when_missing(member, covered, missing, false);
						continue;
					}
					if !covers(self, member) {
						// the value `none`, not its type `None`
						let text = if member == TypeId::NONE {
							"none".to_string()
						} else {
							self.display(member)
						};
						missing.push(format!("`{text}`"));
					}
				}
			}
			// integers by literals and ranges: E2
			TypeKind::Int(_) => {}
			_ if top => missing.push("the other values (add an `else` arm)".to_string()),
			_ => missing.push(format!("`{}`", self.display(subject))),
		}
	}
}
