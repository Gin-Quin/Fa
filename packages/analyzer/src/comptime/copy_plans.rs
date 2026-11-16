//! Type-directed copy graphs for compile-time evaluation.

use std::sync::Arc;

use fa_interpreter::ir::{CopyPlan, CopyStep, TypeTest};
use rustc_hash::FxHashMap;

use super::Lowerer;
use crate::ids::TypeId;
use crate::lower::copies::{copy_alternatives, copy_destination, preserves_copy_alternative};
use crate::types::{CollKind, FieldFlags, TypeKind};

struct Planner<'a, 's> {
	lower: &'a Lowerer<'s>,
	indices: FxHashMap<(TypeId, TypeId), u32>,
	steps: Vec<CopyStep>,
	pending: Vec<(TypeId, TypeId, u32)>,
}

impl Lowerer<'_> {
	pub(super) fn copy_plan(&self, source: TypeId, target: TypeId) -> Arc<CopyPlan> {
		let mut planner = Planner {
			lower: self,
			indices: FxHashMap::default(),
			steps: Vec::new(),
			pending: Vec::new(),
		};
		let root = planner.node(source, target);
		while let Some((source, target, index)) = planner.pending.pop() {
			planner.steps[index as usize] = planner.step(source, target);
		}
		Arc::new(CopyPlan {
			root,
			steps: planner.steps,
		})
	}
}

impl Planner<'_, '_> {
	fn node(&mut self, source: TypeId, target: TypeId) -> u32 {
		if let Some(&index) = self.indices.get(&(source, target)) {
			return index;
		}
		let index = self.steps.len() as u32;
		self.indices.insert((source, target), index);
		self.steps.push(CopyStep::Value);
		self.pending.push((source, target, index));
		index
	}

	fn step(&mut self, source: TypeId, target: TypeId) -> CopyStep {
		let types = self.lower.types();
		// A borrowed structural view may designate a larger object. An
		// owning value keeps its written fields even when its visible source
		// and target types are identical.
		if source == target
			&& !matches!(
				types.kind(target),
				TypeKind::Object(_) | TypeKind::Tuple(_) | TypeKind::Union(_)
			) {
			return CopyStep::Value;
		}
		let module = self.lower.session.module_id(self.lower.file);
		let from = types.rec_target(source).unwrap_or(source);
		let to = types.rec_target(target).unwrap_or(target);
		if (from, to) != (source, target) {
			return CopyStep::Alias(self.node(from, to));
		}
		if let TypeKind::Param(param) = types.kind(source) {
			let bound = types.param(param).constraint;
			if !bound.is_unset() && bound != TypeId::UNKNOWN && bound != source {
				return CopyStep::Alias(self.node(bound, target));
			}
		}
		if !matches!(
			types.kind(source),
			TypeKind::Object(_) | TypeKind::Tuple(_) | TypeKind::Union(_)
		) && preserves_copy_alternative(types, module, source, target)
		{
			return CopyStep::Value;
		}
		if matches!(
			types.kind(source),
			TypeKind::Union(_) | TypeKind::NamedUnion(..)
		) {
			let cases = copy_alternatives(types, module, source)
				.into_iter()
				.map(|case| {
					let chosen = copy_destination(types, module, case.ty, target).unwrap_or(target);
					(self.case_test(case.ty), self.node(case.ty, chosen))
				})
				.collect();
			return CopyStep::Select(cases);
		}
		if matches!(types.kind(target), TypeKind::Union(_)) {
			let chosen = copy_destination(types, module, source, target).unwrap_or(target);
			return if chosen == target {
				CopyStep::Value
			} else {
				CopyStep::Alias(self.node(source, chosen))
			};
		}
		if let TypeKind::Member(nominal, index) = types.kind(source)
			&& matches!(types.kind(target), TypeKind::Object(_))
		{
			return CopyStep::MemberData(self.node(types.payload(nominal, index), target));
		}
		match types.kind(target) {
			TypeKind::Int(_)
			| TypeKind::SymInt(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::Percentage => CopyStep::Convert(self.lower.num(target)),
			TypeKind::Object(_) => {
				let fields = types
					.fields_of(target)
					.to_vec()
					.into_iter()
					.filter(|field| {
						!field
							.flags
							.intersects(FieldFlags::METHOD | FieldFlags::RELATION)
					})
					.map(|field| {
						let actual = types
							.fields_of(source)
							.iter()
							.find(|actual| actual.name == field.name)
							.map_or(field.ty, |actual| actual.ty);
						(
							Arc::from(types.syms.resolve(field.name)),
							self.node(actual, field.ty),
						)
					})
					.collect();
				CopyStep::Object(fields)
			}
			TypeKind::Tuple(list) => {
				let source_items = match types.kind(source) {
					TypeKind::Tuple(list) => types.list_items(list),
					_ => &[],
				};
				CopyStep::Tuple(
					types
						.list_items(list)
						.iter()
						.enumerate()
						.map(|(index, &target)| {
							self.node(source_items.get(index).copied().unwrap_or(target), target)
						})
						.collect(),
				)
			}
			TypeKind::Collection(kind, item, key) => {
				let (actual, actual_key) = match types.kind(source) {
					TypeKind::Collection(_, item, key) => (item, key),
					_ => (item, key),
				};
				match kind {
					CollKind::Map | CollKind::GrowingMap => CopyStep::Map {
						key: self.node(actual_key, key),
						value: self.node(actual, item),
					},
					CollKind::Set | CollKind::GrowingSet => CopyStep::Set(self.node(actual, item)),
					_ => CopyStep::Array(self.node(actual, item)),
				}
			}
			_ => CopyStep::Value,
		}
	}

	fn case_test(&self, ty: TypeId) -> TypeTest {
		let types = self.lower.types();
		if matches!(types.kind(ty), TypeKind::Object(_)) {
			return TypeTest::Fields(
				types
					.fields_of(ty)
					.iter()
					.filter(|field| {
						!field
							.flags
							.intersects(FieldFlags::METHOD | FieldFlags::RELATION)
					})
					.map(|field| {
						let test = if types.is_literal(field.ty) {
							self.lower.type_test(field.ty)
						} else {
							TypeTest::Any
						};
						(Arc::from(types.syms.resolve(field.name)), test)
					})
					.collect(),
			);
		}
		self.lower.type_test(ty)
	}
}
