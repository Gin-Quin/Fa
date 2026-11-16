//! Ownership of the data a value stores, rather than all types it mentions.
//!
//! `documentation/advanced/memory-management.md` defines these categories.
//! Queries follow owned fields and selected union payloads. References,
//! synchronization handles, function signatures and phantom arguments do not
//! make their containing value own their targets. Late-bound payloads and
//! recursive targets are read on each query; interning flags are insufficient.

use rustc_hash::FxHashSet;

use super::{FieldFlags, SyncKind, TypeId, TypeKind, TypeStore};

mod projection;
pub use projection::StoreProjection;

/// The storage rule of a type. Unique ownership takes precedence over structure
/// ownership; neither is inferred from the implementation's copy/drop flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ownership {
	/// Bounded value data, including String and non-owning references/addresses.
	Ordinary,
	/// A collection, an owned recursive graph, or an owner of either.
	Structure,
	/// A Task, a destroy resource, a built-in lifetime owner, or an owner of one.
	Unique,
	/// Mutex, Channel or an atomic: copying the handle retains its identity.
	SharedHandle,
	/// An owned type parameter must be substituted before choosing a category.
	Dependent,
	/// Error recovery or an unfinished declaration: no ownership error is proven.
	Unresolved,
}

impl TypeStore {
	/// The canonical category of the value's owned data. Phantom arguments,
	/// references and callable signatures do not own the types they mention.
	/// Late payloads are read on each query, so no incomplete answer is cached.
	pub fn ownership(&self, ty: TypeId) -> Ownership {
		let mut root = ty;
		let mut aliases = FxHashSet::default();
		while !root.is_unset() {
			let next = match self.kind(root) {
				TypeKind::Rec(_) => match self.rec_target(root) {
					Some(target) => target,
					None => return Ownership::Unresolved,
				},
				TypeKind::Member(nominal, index) => {
					if !self.has_payloads(nominal) {
						return Ownership::Unresolved;
					}
					let payload = self.payload(nominal, index);
					if payload.is_unset() {
						return Ownership::Ordinary;
					}
					payload
				}
				_ => break,
			};
			if !aliases.insert(root) {
				return Ownership::Structure;
			}
			root = next;
		}
		self.ownership_leaf(root)
			.unwrap_or_else(|| OwnedGraph::default().classify(self, root))
	}

	/// A terminal ownership boundary, or None when owned children must be read.
	fn ownership_leaf(&self, ty: TypeId) -> Option<Ownership> {
		if ty.is_unset() {
			return Some(Ownership::Unresolved);
		}
		Some(match self.kind(ty) {
			TypeKind::Task(..) | TypeKind::Subscription => Ownership::Unique,
			TypeKind::Object(_) if self.is_resource(ty) => Ownership::Unique,
			TypeKind::Sync(kind, _) => match kind {
				SyncKind::Guard
				| SyncKind::Worker
				| SyncKind::WorkerPool
				| SyncKind::TcpListener
				| SyncKind::TcpStream => Ownership::Unique,
				SyncKind::AtomicInteger
				| SyncKind::AtomicBoolean
				| SyncKind::Mutex
				| SyncKind::Channel => Ownership::SharedHandle,
				SyncKind::Ordering => Ownership::Ordinary,
			},
			TypeKind::Param(param) if !self.param(param).value => Ownership::Dependent,
			TypeKind::ImplicitParam(..) => Ownership::Dependent,
			TypeKind::Unknown => Ownership::Unresolved,
			// Error payloads are data, never unique (lead, 2026-11-16).
			TypeKind::ErrorTop => Ownership::Structure,
			TypeKind::Object(_)
			| TypeKind::Collection(..)
			| TypeKind::CArray(..)
			| TypeKind::Tuple(_)
			| TypeKind::Union(_)
			| TypeKind::NamedUnion(..)
			| TypeKind::Member(..)
			| TypeKind::Rec(_) => return None,
			// Bytes needs an explicit collection arm when feature/strings lands.
			// This match is exhaustive so that the merge cannot silently omit it.
			TypeKind::Never
			| TypeKind::Void
			| TypeKind::None
			| TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::SymInt(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::Percentage
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::Fields(_)
			| TypeKind::CStruct(_)
			| TypeKind::Pointer(_)
			| TypeKind::CString
			| TypeKind::Function(_)
			| TypeKind::Iterator(..)
			| TypeKind::Ref(..)
			| TypeKind::Range
			| TypeKind::Const(_)
			| TypeKind::Param(_) => Ownership::Ordinary,
		})
	}

	/// A Task parameter transfers ownership, including a plain composite
	/// owning one. Resource and synchronization boundaries keep borrowing.
	pub fn takes_task_parameter(&self, ty: TypeId) -> bool {
		self.owned_any(ty, |types, ty| matches!(types.kind(ty), TypeKind::Task(..)))
	}

	/// Whether the current parameter contract transfers this type. Keep
	/// this policy separate from unique ownership: resources borrow, and a
	/// future explicit consuming contract may also permit other types.
	pub fn parameter_can_be_consumed(&self, ty: TypeId) -> bool {
		self.takes_task_parameter(ty)
	}

	/// Visits owned data, stopping at every terminal ownership boundary.
	pub(super) fn owned_any(&self, ty: TypeId, predicate: impl Fn(&Self, TypeId) -> bool) -> bool {
		let mut graph = OwnedGraph::default();
		graph.push(ty);
		while let Some((ty, _)) = graph.pending.pop() {
			if ty.is_unset() || !graph.visited.insert(ty) {
				continue;
			}
			if predicate(self, ty) {
				return true;
			}
			if self.ownership_leaf(ty).is_none() {
				graph.children(self, ty);
			}
		}
		false
	}

	pub fn is_structure(&self, ty: TypeId) -> bool {
		self.ownership(ty) == Ownership::Structure
	}

	pub fn is_unique(&self, ty: TypeId) -> bool {
		self.ownership(ty) == Ownership::Unique
	}

	pub fn is_shared_handle(&self, ty: TypeId) -> bool {
		self.ownership(ty) == Ownership::SharedHandle
	}

	pub fn is_ordinary(&self, ty: TypeId) -> bool {
		self.ownership(ty) == Ownership::Ordinary
	}
}

/// Iterative DFS: sharing a child is not recursion, and an ownership boundary
/// stops a cycle. Each owned type and edge is visited once, without a depth cap.
#[derive(Default)]
struct OwnedGraph {
	pending: Vec<(TypeId, bool)>,
	visited: FxHashSet<TypeId>,
	active: FxHashSet<TypeId>,
	structure: bool,
	dependent: bool,
	unresolved: bool,
}

impl OwnedGraph {
	fn classify(mut self, types: &TypeStore, root: TypeId) -> Ownership {
		self.pending.push((root, false));
		while let Some((ty, leaving)) = self.pending.pop() {
			if leaving {
				self.active.remove(&ty);
				continue;
			}
			if let Some(category) = types.ownership_leaf(ty) {
				match category {
					Ownership::Unique => return Ownership::Unique,
					Ownership::Dependent => self.dependent = true,
					Ownership::Unresolved => self.unresolved = true,
					Ownership::Structure => self.structure = true,
					Ownership::Ordinary | Ownership::SharedHandle => {}
				}
				continue;
			}
			if self.active.contains(&ty) {
				self.structure = true;
				continue;
			}
			if !self.visited.insert(ty) {
				continue;
			}
			self.active.insert(ty);
			self.pending.push((ty, true));
			self.children(types, ty);
		}
		if self.unresolved {
			Ownership::Unresolved
		} else if self.dependent {
			Ownership::Dependent
		} else if self.structure {
			Ownership::Structure
		} else {
			Ownership::Ordinary
		}
	}

	fn push(&mut self, ty: TypeId) {
		self.pending.push((ty, false));
	}

	fn payload(&mut self, types: &TypeStore, nominal: crate::ids::NominalId, index: u16) {
		self.unresolved |= !types.has_payloads(nominal);
		let payload = types.payload(nominal, index);
		if !payload.is_unset() {
			self.push(payload);
		}
	}

	fn children(&mut self, types: &TypeStore, ty: TypeId) {
		match types.kind(ty) {
			TypeKind::Object(shape) => {
				for field in types.shape_fields(shape) {
					if !field
						.flags
						.intersects(FieldFlags::RELATION | FieldFlags::METHOD)
					{
						self.push(field.ty);
					}
				}
			}
			TypeKind::Collection(_, item, key) => {
				self.structure = true;
				self.push(item);
				self.push(key);
			}
			TypeKind::CArray(item, _) => self.push(item),
			TypeKind::Tuple(list) | TypeKind::Union(list) => {
				for &item in types.list_items(list) {
					self.push(item);
				}
			}
			TypeKind::NamedUnion(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				self.unresolved |= !types.has_payloads(nominal);
				for index in 0..count {
					if types.mask_has(mask, index) {
						self.payload(types, nominal, index);
					}
				}
			}
			TypeKind::Member(nominal, index) => self.payload(types, nominal, index),
			TypeKind::Rec(_) => match types.rec_target(ty) {
				Some(target) => self.push(target),
				None => self.unresolved = true,
			},
			// All other kinds were handled by ownership_leaf.
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests;
