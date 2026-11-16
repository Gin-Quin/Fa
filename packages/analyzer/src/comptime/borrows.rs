//! Borrowing result paths stop at constructors and calls: their operands
//! keep their own store and evaluation rules.

use std::sync::Arc;

use fa_interpreter::ir::{Expr, Place, Step};

use super::Lowerer;
use crate::ids::NodeId;
use crate::symbols::Resolution;
use crate::syntax::{self, View};
use crate::types::{CollKind, TypeKind};

impl Lowerer<'_> {
	pub(super) fn bound_argument(&mut self, node: NodeId) -> Expr {
		let consumes = self
			.a
			.argument_ownership
			.get(&node)
			.copied()
			.unwrap_or_else(|| self.types().parameter_can_be_consumed(self.a.type_of(node)));
		if consumes {
			self.expr(node)
		} else {
			self.bound(node)
		}
	}

	pub(super) fn bound(&mut self, node: NodeId) -> Expr {
		let mut pending = vec![node];
		while let Some(value) = pending.pop() {
			if self
				.a
				.copy_inputs
				.get(&value)
				.is_some_and(|input| input.produces)
			{
				continue;
			}
			if let Some(inputs) = crate::lower::copies::result_inputs(self.tree, value) {
				pending.extend(inputs);
			} else {
				self.borrowed_results.insert(value);
			}
		}
		self.expr(node)
	}

	/// Stored fields/items only. Computed members, string characters and
	/// slices are values rather than addressable payloads in the VM.
	pub(super) fn borrowable_place(&mut self, node: NodeId) -> Option<Place> {
		match syntax::view(self.tree, node) {
			View::Group(inner) => self.borrowable_place(inner),
			View::Name(_) => self.place(node),
			View::Access { target, members } => {
				let mut place = self.borrowable_place(target)?;
				for member in members.iter() {
					if !matches!(self.a.resolution(member), Resolution::Field(..)) {
						return None;
					}
					let View::Name(name) = syntax::view(self.tree, member) else {
						return None;
					};
					place
						.steps
						.push(Step::Field(Arc::from(self.tree.name(name))));
				}
				Some(place)
			}
			View::Index { target, index } => {
				let mut ty = self.a.type_of(target);
				while let TypeKind::Ref(inner, _) = self.types().kind(ty) {
					ty = inner;
				}
				if !matches!(
					self.types().kind(ty),
					TypeKind::Collection(
						CollKind::Array
							| CollKind::GrowingArray
							| CollKind::Map | CollKind::GrowingMap,
						..
					) | TypeKind::Tuple(_)
				) || matches!(self.types().kind(self.a.type_of(index)), TypeKind::Range)
				{
					return None;
				}
				let mut place = self.borrowable_place(target)?;
				place.steps.push(Step::Index(self.expr(index)));
				Some(place)
			}
			_ => None,
		}
	}

	pub(super) fn borrowed_receiver(&mut self, callee: NodeId) -> Option<Place> {
		let View::Access { target, members } = syntax::view(self.tree, callee) else {
			return None;
		};
		let mut place = self.borrowable_place(target)?;
		for member in members.iter().take(members.len().saturating_sub(1)) {
			if !matches!(self.a.resolution(member), Resolution::Field(..)) {
				return None;
			}
			let View::Name(name) = syntax::view(self.tree, member) else {
				return None;
			};
			place
				.steps
				.push(Step::Field(Arc::from(self.tree.name(name))));
		}
		Some(place)
	}
}
