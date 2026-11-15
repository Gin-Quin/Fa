//! Structural copies must be explicit when they duplicate a collection.
//! Conditions depending on a rigid type parameter are inferred once from
//! the body, substituted at applications, and published without runtime ABI.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{Callee, CopyRequirement, NodeFlags};
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{MEM002, MEM003, MEM004};
use crate::ids::{DeclKey, ListId, NodeId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;
use crate::types::{CollKind, TypeKind};

#[derive(Clone, Copy)]
enum Function {
	Local(NodeId),
	External(DeclKey),
}

enum ResourceGate {
	Active,
	Inactive,
	Deferred(TypeId, ListId),
}

impl Walker<'_> {
	pub(super) fn finish_copy_requirements(&mut self) {
		let mut summaries: FxHashMap<NodeId, Vec<CopyRequirement>> = FxHashMap::default();
		let mut sites: Vec<_> = self
			.copy_sites
			.iter()
			.copied()
			.map(|n| (n, false))
			.collect();
		sites.extend(self.explicit_copy_sites.iter().copied().map(|n| (n, true)));
		sites.sort_unstable();
		for (site, explicit) in sites {
			let value = match self.view(site) {
				View::Copy(inner) if explicit => inner,
				_ => site,
			};
			if !explicit && !self.copy_reads_place(value) {
				continue;
			}
			let subject = self
				.a
				.coercion(value)
				.unwrap_or_else(|| self.a.type_of(value));
			// Concrete Tasks are already consumed by note_copy_site and their
			// later use is MEM002. Do not replace that diagnostic with a second
			// copy error here; only latent copies of a rigid T need this finish.
			let task_transfer = !explicit
				&& self.copy_source(value).is_some_and(|(symbol, read)| {
					matches!(
						self.a.symbols.symbol(symbol).kind,
						SymbolKind::Parameter { .. }
					) && self.copy_owner(self.a.symbols.symbol(symbol).decl_node)
						== self.copy_owner(site)
						&& self.source_is_owned(value)
						&& self.source_is_last_use(symbol, read, false)
				});
			if !explicit
				&& (task_transfer
					|| self.a.has_flags(value, NodeFlags::MOVE)
					|| self.a.collection_moves.contains_key(&value))
				&& let Some(resource_gate) = self.moved_field_gate(value)
			{
				self.add_copy_requirement(
					site,
					CopyRequirement {
						subject,
						explicit: false,
						task_transfer: false,
						resource_gate: Some(resource_gate),
					},
					self.copy_owner(site),
					&mut summaries,
				);
			}
			if !explicit
				&& (self.s.types.is_move_only(subject)
					|| self.a.has_flags(value, NodeFlags::MOVE)
					|| self.a.collection_moves.contains_key(&value))
			{
				continue;
			}
			let requirement = CopyRequirement {
				subject,
				explicit,
				task_transfer,
				resource_gate: None,
			};
			self.add_copy_requirement(site, requirement, self.copy_owner(site), &mut summaries);
		}
		self.capture_copy_requirements(&mut summaries);
		self.builtin_copy_requirements(&mut summaries);
		for (site, requirement) in self.copy_conditions.clone() {
			self.add_copy_requirement(site, requirement, self.copy_owner(site), &mut summaries);
		}

		while self.finish_callable_copy_requirements(&mut summaries) {}
		// Imported callers need the completed summaries, including conditions
		// discovered through a call whose body was checked later in this file.
		self.s
			.source_file_mut(self.file)
			.interface
			.copy_requirements = summaries.clone();
		self.a.copy_requirements = summaries;
	}

	fn finish_callable_copy_requirements(
		&mut self,
		summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	) -> bool {
		for (&node, &key) in &self.a.callable_keys {
			if let Some(summary) = self.a.callables.get_mut(&key)
				&& let Some(requirements) = summaries.get(&node)
			{
				summary.copy_requirements = requirements.clone();
			}
		}
		let mut calls: Vec<_> = self
			.a
			.calls
			.keys()
			.filter_map(|site| {
				let value = self.a.value_origins.get(site)?;
				let crate::provenance::OriginKind::Call(call) = &value.kind else {
					return None;
				};
				Some((*site, call.clone(), self.copy_owner(*site)))
			})
			.collect();
		calls.sort_unstable_by_key(|(site, ..)| *site);
		let mut changed = false;
		for (site, call, owner) in calls {
			changed |= self.check_callable_copy_application(
				site,
				&call,
				owner,
				&mut FxHashSet::default(),
				summaries,
			);
		}
		let values: Vec<_> = self
			.copy_instantiations
			.keys()
			.chain(
				self.a
					.method_instantiations
					.keys()
					.filter(|site| self.callable_materialization(**site)),
			)
			.filter(|site| !self.a.calls.contains_key(site))
			.filter_map(|site| {
				Some((
					*site,
					self.a.value_origins.get(site)?.clone(),
					self.copy_owner(*site),
				))
			})
			.collect();
		for (site, callee, owner) in values {
			let call = crate::provenance::Invocation {
				callee,
				arguments: Vec::new(),
				bindings: Vec::new(),
			};
			changed |= self.check_callable_copy_application(
				site,
				&call,
				owner,
				&mut FxHashSet::default(),
				summaries,
			);
		}
		let callables = self.a.callables.clone();
		self.s.source_file_mut(self.file).interface.callables = callables;
		changed
	}

	fn check_callable_copy_application(
		&mut self,
		site: NodeId,
		call: &crate::provenance::Invocation,
		owner: Option<NodeId>,
		active: &mut FxHashSet<std::sync::Arc<crate::provenance::ValueOrigin>>,
		summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	) -> bool {
		let mut changed = false;
		for callee in self.resolve_value_origin(
			std::sync::Arc::clone(&call.callee),
			&mut FxHashSet::default(),
		) {
			let crate::provenance::OriginKind::Function {
				declaration,
				captures,
				..
			} = &callee.kind
			else {
				continue;
			};
			let Some(summary) = self.callable_summary(*declaration) else {
				continue;
			};
			if !active.insert(std::sync::Arc::clone(&callee)) {
				continue;
			}
			let bindings = self.invocation_bindings(&callee, call, &summary);
			for requirement in &summary.copy_requirements {
				let requirement = CopyRequirement {
					subject: self.s.types.substitute(requirement.subject, &bindings),
					resource_gate: requirement.resource_gate.map(|(receiver, path)| {
						(self.s.types.substitute(receiver, &bindings), path)
					}),
					..*requirement
				};
				changed |= self.add_copy_requirement(site, requirement, owner, summaries);
			}
			let replacements: Vec<_> = summary
				.captures
				.iter()
				.cloned()
				.zip(captures.iter().cloned())
				.collect();
			let arguments = vec![(*declaration, call.arguments.clone())];
			for application in &summary.applications {
				let application = application.replacing(&replacements).substituted(
					&mut self.s.types,
					&bindings,
					&arguments,
				);
				changed |= self.check_callable_copy_application(
					site,
					&application,
					owner,
					active,
					summaries,
				);
			}
			active.remove(&callee);
		}
		changed
	}

	pub(super) fn defer_copy_condition(&mut self, site: NodeId, requirement: CopyRequirement) {
		if self.quiet > 0 {
			return;
		}
		let conditions = match self.body.flow_buffers.last_mut() {
			Some(buffer) => &mut buffer.copy_conditions,
			None => &mut self.copy_conditions,
		};
		if !conditions.contains(&(site, requirement)) {
			conditions.push((site, requirement));
		}
	}

	pub(super) fn consume_task_conditionally(&mut self, value: NodeId, ty: TypeId) {
		if !self.copy_reads_place(value) {
			return;
		}
		let source = self.copy_source(value);
		let owned = source.is_some_and(|(symbol, _)| {
			matches!(
				self.a.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. }
			) && self.owns_task_symbol_at(symbol, value)
				&& self.source_is_owned(value)
		});
		if !owned {
			self.defer_copy_condition(
				value,
				CopyRequirement {
					subject: ty,
					explicit: true,
					task_transfer: false,
					resource_gate: None,
				},
			);
			return;
		}
		let Some(place) = self.place_of(value) else {
			return;
		};
		self.a.task_moves.insert(value, ty);
		if !self
			.body
			.flow
			.conditional_consumed
			.iter()
			.any(|&(known, subject, _)| known == place && subject == ty)
		{
			self.body.flow.conditional_consumed.push((place, ty, value));
		}
	}

	/// Implicit returns are processed after their FnCx has been popped. Use
	/// lexical scopes, so an outer capture never looks like a local owner.
	pub(super) fn owns_task_symbol_at(&self, symbol: crate::ids::SymbolId, value: NodeId) -> bool {
		let function_scope = |mut scope: crate::ids::ScopeId| {
			loop {
				let data = self.a.symbols.scope(scope);
				if data.kind == crate::symbols::ScopeKind::Function {
					return Some(scope);
				}
				scope = data.parent?;
			}
		};
		let owner = function_scope(self.a.symbols.symbol(symbol).scope);
		let mut node = Some(value);
		while let Some(current) = node {
			if let Some(info) = self.a.function(current) {
				return owner == Some(info.scope);
			}
			if matches!(self.view(current), View::Lambda { .. }) {
				return owner == function_scope(self.body.scope);
			}
			node = self.parent(current);
		}
		owner.is_none()
	}

	pub(super) fn is_direct_replacement(&self, mut node: NodeId) -> bool {
		loop {
			let Some(parent) = self.parent(node) else {
				return false;
			};
			match self.view(parent) {
				View::Group(inner) if inner == node => node = parent,
				View::Assignment {
					target,
					value: Some(_),
					..
				} if target == node => return true,
				_ => return false,
			}
		}
	}

	pub(super) fn check_conditional_task_read(&mut self, n: NodeId) {
		if !self.body.flow.reachable || self.body.flow.conditional_consumed.is_empty() {
			return;
		}
		let Some(place) = self.place_of(n) else {
			return;
		};
		let replaced = self.is_direct_replacement(n);
		let root_of_access =
			self.access_roots.contains(&n) && matches!(self.view(n), View::Name(_));
		let conditions: Vec<_> = self
			.body
			.flow
			.conditional_consumed
			.iter()
			.filter_map(|&(consumed, ty, _)| {
				if replaced && self.places.is_at_or_below(consumed, place) {
					return None;
				}
				if root_of_access && self.places.parent(consumed).is_some() {
					return None;
				}
				self.may_overlap(consumed, place).then_some(ty)
			})
			.collect();
		for subject in conditions {
			self.defer_copy_condition(
				n,
				CopyRequirement {
					subject,
					explicit: true,
					task_transfer: false,
					resource_gate: None,
				},
			);
		}
	}

	fn capture_copy_requirements(
		&mut self,
		summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	) {
		let mut creations: Vec<_> = self
			.capture_creations
			.iter()
			.map(|(&site, &function)| {
				(
					site,
					function,
					self.capture_lists
						.get(&function)
						.cloned()
						.unwrap_or_default(),
				)
			})
			.collect();
		for (&site, start) in &self.a.starts {
			if start.detached && start.block.is_some() {
				let function = match self.view(site) {
					View::Start(operand) => operand,
					_ => site,
				};
				creations.retain(|(existing, ..)| *existing != function);
				creations.push((site, function, start.captures.clone()));
			}
		}
		creations.sort_unstable_by_key(|(site, ..)| *site);
		for (site, function, captures) in creations {
			let owner = self.parent(site).and_then(|parent| self.copy_owner(parent));
			let mut moved = Vec::new();
			let mut conditional = Vec::new();
			for symbol in captures {
				let data = self.a.symbols.symbol(symbol);
				if matches!(data.kind, SymbolKind::Reactive | SymbolKind::Derived) {
					continue;
				}
				let subject = data.ty;
				let can_move = self.capture_is_last_use(symbol, site, function);
				// A Task can never satisfy a capture-copy, even when an ordinary
				// collection's copy can be eliminated by moving its last use.
				self.add_copy_requirement(
					site,
					CopyRequirement {
						subject,
						explicit: can_move,
						task_transfer: false,
						resource_gate: None,
					},
					owner,
					summaries,
				);
				if can_move && !self.s.types.is_move_only(subject) {
					if self.s.types.requires_explicit_copy(subject) {
						moved.push(symbol);
					} else if self
						.s
						.types
						.flags(subject)
						.contains(crate::types::TypeFlags::HAS_PARAM)
					{
						conditional.push((symbol, subject));
					}
				}
			}
			if !moved.is_empty() {
				self.a.moved_captures.insert(site, moved);
			}
			if !conditional.is_empty() {
				self.a.collection_moved_captures.insert(site, conditional);
			}
		}
	}

	fn builtin_copy_requirements(
		&mut self,
		summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	) {
		let calls: Vec<_> = self
			.a
			.calls
			.iter()
			.filter_map(|(&site, call)| {
				let Callee::Builtin(builtin) = call.callee else {
					return None;
				};
				Some((
					site,
					crate::builtins::get(builtin).path,
					call.arguments.clone(),
				))
			})
			.collect();
		for (site, path, arguments) in calls {
			let View::Call { callee, .. } = self.view(site) else {
				continue;
			};
			let receiver = self.callee_receiver_type(callee);
			let name = path.split_once('#').map(|(_, name)| name);
			let callback = matches!(
				name,
				Some(
					"map"
						| "filter" | "find"
						| "reduce" | "every"
						| "some" | "deleteWhere"
						| "sort" | "sortInPlace"
				)
			);
			let collection_copy = matches!(
				name,
				Some(
					"filter"
						| "sort" | "reverse"
						| "union" | "intersection"
						| "difference" | "symmetricDifference"
				)
			);
			let pointer_copy = matches!(path, "Pointer#read" | "Pointer#toArray");
			let from = path.ends_with(".from");
			let mut subjects = Vec::new();
			if collection_copy
				&& let Some(receiver) = receiver
				&& let Some(item) = self.iteration_item(receiver)
			{
				subjects.push(item);
			}
			if callback && let Some(receiver) = receiver {
				let arity = arguments
					.first()
					.and_then(|argument| match argument {
						crate::analysis::Argument::Given(value) => {
							Some(self.copy_callback_arity(*value))
						}
						_ => None,
					})
					.unwrap_or(0);
				let offset = usize::from(name == Some("reduce"));
				match self.s.types.kind(receiver) {
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, item, key) => {
						if arity > offset {
							subjects.push(key);
						}
						if arity > offset + 1 {
							subjects.push(item);
						}
					}
					_ if arity > offset => {
						if let Some(item) = self.iteration_item(receiver) {
							subjects.push(item);
						}
					}
					_ => {}
				}
			}
			if name == Some("toArray")
				&& !pointer_copy
				&& let Some(receiver) = receiver
				&& !self.iterator_produces_owned(callee, &mut FxHashSet::default())
				&& let Some(item) = self.iteration_item(receiver)
			{
				subjects.push(item);
			}
			if pointer_copy
				&& let Some(receiver) = receiver
				&& let Some(target) = self.s.types.pointer_target(receiver)
			{
				subjects.push(target);
			}
			if from
				&& let Some(crate::analysis::Argument::Given(source)) = arguments.first()
				&& !self.owned_iterator_value(*source, &mut FxHashSet::default())
				&& let Some(item) = self.iteration_item(self.a.type_of(*source))
			{
				subjects.push(item);
			}
			let owner = self.copy_owner(site);
			for subject in subjects {
				self.add_copy_requirement(
					site,
					CopyRequirement {
						subject,
						explicit: true,
						task_transfer: false,
						resource_gate: None,
					},
					owner,
					summaries,
				);
			}
		}
		let slices: Vec<_> = self
			.copy_slices
			.iter()
			.copied()
			.filter_map(|site| {
				let View::Index { target, index } = self.view(site) else {
					return None;
				};
				if !matches!(self.s.types.kind(self.a.type_of(index)), TypeKind::Range) {
					return None;
				}
				Some((site, self.a.type_of(target)))
			})
			.collect();
		for (site, collection) in slices {
			if let Some(subject) = self.iteration_item(collection) {
				self.add_copy_requirement(
					site,
					CopyRequirement {
						subject,
						explicit: true,
						task_transfer: false,
						resource_gate: None,
					},
					self.copy_owner(site),
					summaries,
				);
			}
		}
	}

	fn copy_callback_arity(&self, value: NodeId) -> usize {
		match self.copy_function(value, &mut FxHashSet::default()) {
			Some(Function::Local(function)) => self
				.a
				.function(function)
				.map_or(0, |info| info.params.len()),
			Some(Function::External(key)) => self
				.s
				.interface(self.s.types.decl_key_data(key).module)
				.and_then(|interface| {
					interface
						.keys
						.get(&key)
						.and_then(|decl| interface.parameters.get(&decl.node))
				})
				.map_or(0, Vec::len),
			None => match self.s.types.kind(self.a.type_of(value)) {
				TypeKind::Function(sig) => self.s.types.params(self.s.types.sig(sig).params).len(),
				_ => 0,
			},
		}
	}

	fn iterator_produces_owned(&self, callee: NodeId, seen: &mut FxHashSet<NodeId>) -> bool {
		let receiver = match self.view(callee) {
			View::Access { target, .. } => target,
			_ => return false,
		};
		self.owned_iterator_value(receiver, seen)
	}

	fn owned_iterator_value(&self, value: NodeId, seen: &mut FxHashSet<NodeId>) -> bool {
		if !seen.insert(value) {
			return false;
		}
		match self.view(value) {
			View::Group(inner) => self.owned_iterator_value(inner, seen),
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(value) else {
					return false;
				};
				let decl = self.a.symbols.symbol(symbol).decl_node;
				match self.view(decl) {
					View::Declaration {
						value: Some(value), ..
					} => self.owned_iterator_value(value, seen),
					_ => false,
				}
			}
			View::Call { callee, .. } => {
				let Some(call) = self.a.calls.get(&value) else {
					return false;
				};
				let Callee::Builtin(builtin) = call.callee else {
					return false;
				};
				match crate::builtins::get(builtin).path {
					"Iterator#map" => true,
					"Iterator#firstValues" => self.iterator_produces_owned(callee, seen),
					_ => false,
				}
			}
			_ => false,
		}
	}

	fn capture_is_last_use(
		&self,
		symbol: crate::ids::SymbolId,
		site: NodeId,
		function: NodeId,
	) -> bool {
		if !matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Let | SymbolKind::Mutable
		) {
			return false;
		}
		let Some(uses) = self.move_uses.get(&symbol) else {
			return false;
		};
		if !uses.iter().any(|read| read.node == site && read.movable) {
			return false;
		}
		let start = self.tree.spans[site.index()].start;
		!uses.iter().any(|read| {
			if read.node == site || self.tree.spans[read.node.index()].start <= start {
				return false;
			}
			let mut node = Some(read.node);
			while let Some(current) = node {
				if current == function {
					return false;
				}
				node = self.parent(current);
			}
			!self.exclusive_reads(site, read.node)
		})
	}

	/// Normalize conditions to rigid parameter leaves. This both avoids
	/// redundant container conditions and makes recursive propagation finite:
	/// `copy Array<T>` needs exactly the same Task-free condition as `copy T`.
	fn add_copy_requirement(
		&mut self,
		site: NodeId,
		requirement: CopyRequirement,
		owner: Option<NodeId>,
		summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	) -> bool {
		let mut pending = vec![requirement];
		let mut seen = FxHashSet::default();
		let mut changed = false;
		while let Some(requirement) = pending.pop() {
			if !seen.insert(requirement) {
				continue;
			}
			let ty = requirement.subject;
			if let Some((receiver, path)) = requirement.resource_gate {
				match self.resolve_resource_gate(receiver, path) {
					ResourceGate::Inactive => continue,
					ResourceGate::Active => {}
					ResourceGate::Deferred(receiver, path) => {
						if let Some(owner) = owner {
							let requirement = CopyRequirement {
								resource_gate: Some((receiver, path)),
								..requirement
							};
							changed |= push_requirement(summaries, owner, requirement);
						}
						continue;
					}
				}
			}
			if requirement.task_transfer && self.s.types.is_move_only(ty) {
				continue;
			}
			if (!requirement.task_transfer && self.s.types.is_move_only(ty))
				|| (!requirement.explicit && self.s.types.requires_explicit_copy(ty))
			{
				self.report_copy_requirement(site, requirement);
				continue;
			}
			let mut children = Vec::new();
			let mut child_requirement = requirement;
			match self.s.types.kind(ty) {
				TypeKind::Param(param) => {
					if let Some(owner) = owner {
						let owner = self.copy_parameter_owner(owner, param);
						changed |= push_requirement(summaries, owner, requirement);
					}
				}
				TypeKind::Rec(_) => children.extend(self.s.types.rec_target(ty)),
				TypeKind::Object(shape) => {
					if self.s.types.is_resource(ty) {
						if requirement.task_transfer {
							continue;
						}
						child_requirement.explicit = true;
					}
					children.extend(
						self.s
							.types
							.shape_fields(shape)
							.iter()
							.map(|field| field.ty),
					);
				}
				TypeKind::Collection(kind, item, key)
					if !matches!(kind, CollKind::RelationPool(_)) =>
				{
					children.extend([item, key]);
				}
				TypeKind::CArray(item, _) => children.push(item),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					children.extend(self.s.types.list_items(list))
				}
				TypeKind::NamedUnion(nominal, mask) => {
					for index in 0..self
						.s
						.types
						.sym_list(self.s.types.nominal(nominal).members)
						.len() as u16
					{
						if self.s.types.mask_has(mask, index) {
							children.push(self.s.types.payload(nominal, index));
						}
					}
				}
				TypeKind::Member(nominal, index) => {
					children.push(self.s.types.payload(nominal, index))
				}
				_ => {}
			}
			pending.extend(children.into_iter().map(|subject| CopyRequirement {
				subject,
				..child_requirement
			}));
		}
		changed
	}

	/// A field moved from a generic view may actually belong to a Resource.
	/// Its destructor still needs that field; a concrete shared cell never
	/// becomes an owned structural value merely because a name is last-used.
	fn moved_field_gate(&mut self, value: NodeId) -> Option<(TypeId, ListId)> {
		let mut current = self.strip_groups(value);
		let mut fields = Vec::new();
		loop {
			match self.view(current) {
				View::Access { target, members } => {
					let mut names: Vec<_> = members
						.iter()
						.map(|member| self.tree.text(member.index()).to_string())
						.collect();
					names.extend(fields);
					fields = names;
					current = self.strip_groups(target);
				}
				View::Name(_) => break,
				_ => return None,
			}
		}
		fields.pop()?;
		let receiver = self.a.type_of(current);
		let names: Vec<_> = fields
			.iter()
			.map(|name| self.s.types.string_literal(name))
			.collect();
		let path = self.s.types.list(&names);
		Some((receiver, path))
	}

	fn resolve_resource_gate(&mut self, receiver: TypeId, path: ListId) -> ResourceGate {
		let names = self.s.types.list_items(path).to_vec();
		let mut pending = vec![(receiver, 0usize)];
		let mut seen = FxHashSet::default();
		let mut deferred = None;
		while let Some((ty, position)) = pending.pop() {
			if !seen.insert((ty, position)) {
				continue;
			}
			if self.s.types.is_resource(ty) {
				return ResourceGate::Active;
			}
			match self.s.types.kind(ty) {
				TypeKind::Param(_) => {
					deferred = Some((ty, position));
				}
				TypeKind::Rec(_) => {
					if let Some(target) = self.s.types.rec_target(ty) {
						pending.push((target, position));
					}
				}
				TypeKind::Union(list) => pending.extend(
					self.s
						.types
						.list_items(list)
						.iter()
						.map(|&member| (member, position)),
				),
				TypeKind::Member(nominal, member) => {
					pending.push((self.s.types.payload(nominal, member), position))
				}
				TypeKind::NamedUnion(nominal, mask) => {
					for member in 0..self
						.s
						.types
						.sym_list(self.s.types.nominal(nominal).members)
						.len() as u16
					{
						if self.s.types.mask_has(mask, member) {
							pending.push((self.s.types.payload(nominal, member), position));
						}
					}
				}
				TypeKind::Object(shape) if position < names.len() => {
					if let TypeKind::StringLit(name) = self.s.types.kind(names[position])
						&& let Some(field) = self
							.s
							.types
							.shape_fields(shape)
							.iter()
							.find(|field| field.name == name)
					{
						pending.push((field.ty, position + 1));
					}
				}
				_ => {}
			}
		}
		match deferred {
			Some((ty, position)) if seen.len() == position + 1 => {
				ResourceGate::Deferred(ty, self.s.types.list(&names[position..]))
			}
			Some(_) => ResourceGate::Deferred(receiver, path),
			None => ResourceGate::Inactive,
		}
	}

	/// A nested function is checked under its enclosing rigid parameters.
	/// Its condition on an outer T belongs to that outer instantiation, even
	/// when the nested function is returned through an annotated value.
	fn copy_parameter_owner(&self, owner: NodeId, param: crate::ids::ParamId) -> NodeId {
		let mut node = Some(owner);
		while let Some(current) = node {
			if let Some(info) = self.a.function(current)
				&& info.generics.contains(&param)
			{
				return info.declaration.unwrap_or(info.lambda);
			}
			node = self.parent(current);
		}
		owner
	}

	fn copy_owner(&self, mut node: NodeId) -> Option<NodeId> {
		loop {
			if let Some(info) = self.a.function(node) {
				return Some(info.declaration.unwrap_or(info.lambda));
			}
			node = self.parent(node)?;
		}
	}

	fn copy_function(&self, node: NodeId, seen: &mut FxHashSet<NodeId>) -> Option<Function> {
		if !seen.insert(node) {
			return None;
		}
		if let Some(info) = self.a.function(node) {
			return Some(Function::Local(info.declaration.unwrap_or(info.lambda)));
		}
		match self.a.resolution(node) {
			Resolution::External(key) => return Some(Function::External(key)),
			Resolution::Local(symbol) => {
				if let Some(function) = self.direct_function(symbol) {
					return Some(Function::Local(function));
				}
				let declaration = self.a.symbols.symbol(symbol).decl_node;
				if let View::Declaration {
					value: Some(value), ..
				}
				| View::Function { value, .. } = self.view(declaration)
				{
					return self.copy_function(value, seen);
				}
			}
			_ => {}
		}
		match self.view(node) {
			View::Group(inner) => self.copy_function(inner, seen),
			View::Call { callee, .. } if !self.a.calls.contains_key(&node) => {
				self.copy_function(callee, seen)
			}
			View::Access { members, .. } => self.copy_function(members.last()?, seen),
			_ => None,
		}
	}

	fn copy_reads_place(&self, node: NodeId) -> bool {
		match self.view(node) {
			View::Group(inner) | View::Try(inner) => self.copy_reads_place(inner),
			View::Name(_) => matches!(
				self.a.resolution(node),
				Resolution::Local(_) | Resolution::External(_)
			),
			View::Access { members, .. } | View::OptionalAccess { members, .. } => members
				.last()
				.is_some_and(|member| match self.a.resolution(member) {
					Resolution::Builtin(builtin) => {
						crate::builtins::get(builtin).path.ends_with("#value")
					}
					_ => true,
				}),
			View::Index { index, .. } => {
				!matches!(self.s.types.kind(self.a.type_of(index)), TypeKind::Range)
			}
			View::Call { .. } => crate::lower::copies::element_view(&self.tree, &self.a, node)
				.is_some_and(|(_, collection)| !collection),
			_ => false,
		}
	}

	fn report_copy_requirement(&mut self, site: NodeId, requirement: CopyRequirement) {
		let code = if self.s.types.is_move_only(requirement.subject) && !requirement.task_transfer {
			if requirement.resource_gate.is_some() {
				&MEM002
			} else {
				&MEM003
			}
		} else if !requirement.explicit && self.s.types.requires_explicit_copy(requirement.subject)
		{
			&MEM004
		} else {
			return;
		};
		// This finish runs outside the body's flow buffers. Preserve one
		// diagnostic per site, including recursive call-graph iterations.
		let span = self.s.span(self.file, site);
		if self
			.output
			.iter()
			.any(|d| std::ptr::eq(d.code, code) && d.location.span == span)
		{
			return;
		}
		let ty = self.display(requirement.subject);
		let message = if std::ptr::eq(code, &MEM002) {
			format!(
				"`{ty}` can't be moved out of a Resource: its destructor still needs every field; move the complete Resource instead"
			)
		} else if std::ptr::eq(code, &MEM003) {
			format!("`{ty}` contains a Task and can't be copied; move it instead")
		} else {
			format!(
				"copying `{ty}` duplicates a collection: write `copy value`, or move a local at its last use"
			)
		};
		let diagnostic = self.diagnostic(code, site, message);
		self.report(diagnostic);
	}
}

fn push_requirement(
	summaries: &mut FxHashMap<NodeId, Vec<CopyRequirement>>,
	owner: NodeId,
	requirement: CopyRequirement,
) -> bool {
	let requirements = summaries.entry(owner).or_default();
	if requirements.contains(&requirement) {
		return false;
	}
	requirements.push(requirement);
	true
}

#[cfg(test)]
mod tests {
	use crate::analyze_source;

	fn codes(source: &str) -> Vec<String> {
		let (session, file) = analyze_source(source);
		session
			.diagnostics(file)
			.iter()
			.filter(|d| d.severity == crate::diagnostics::Severity::Error)
			.map(|d| d.code.id.to_string())
			.collect()
	}

	#[test]
	fn collections_require_copy_but_their_last_local_use_moves() {
		assert_eq!(
			codes("let a = [1]\nlet b = a\nconsole.log(a, b)\n"),
			["MEM004"]
		);
		assert!(codes("let a = [1]\nlet b = copy a\nconsole.log(a, b)\n").is_empty());
		assert!(
			codes(
				"function make = (): Array<Integer> => { let a = [1]\na }\nconsole.log(make())\n"
			)
			.is_empty()
		);
		assert_eq!(
			codes("function borrowed = (a: Array<Integer>): Array<Integer> => a\n"),
			["MEM004"]
		);
		assert!(
			codes("function copied = (a: Array<Integer>): Array<Integer> => copy a\n").is_empty()
		);
	}

	#[test]
	fn conditional_last_reads_and_fields_move_owned_values() {
		assert!(codes("function choose = (test: Boolean): Array<Integer> => { let a = [1]\nif test { a } else { a } }\n").is_empty());
		assert!(
			codes(
				"function field = (): Array<Integer> => { let holder = { items = [1] }\nholder.items }\n"
			)
			.is_empty()
		);
		assert_eq!(
			codes(
				"function field = (holder: { items: Array<Integer> }): Array<Integer> => holder.items\n"
			),
			["MEM004"]
		);
	}

	#[test]
	fn copies_in_literals_obey_the_same_rule() {
		assert_eq!(
			codes("let a = [1]\nlet b = { items = a }\nconsole.log(a, b)\n"),
			["MEM004"]
		);
		assert_eq!(
			codes("let a = [1]\nlet b = [a]\nconsole.log(a, b)\n"),
			["MEM004"]
		);
		assert!(
			codes("function store = () => { let a = [1]\nlet b = [a]\nconsole.log(b) }\n")
				.is_empty()
		);
	}

	#[test]
	fn generic_copy_conditions_are_checked_after_inference() {
		let identity = "function identity = <Item>(item: Item): Item => item\n";
		assert!(codes(&format!("{identity}console.log(identity(1))\n")).is_empty());
		assert_eq!(
			codes(&format!("{identity}console.log(identity([1]))\n")),
			["MEM004"]
		);
		let clone = "function clone = <Item>(item: Item): Item => copy item\n";
		assert!(codes(&format!("{clone}console.log(clone([1]))\n")).is_empty());
		assert_eq!(
			codes(&format!(
				"{clone}function invalid = (task: Task<Integer>): Task<Integer> => clone(task)\n"
			)),
			["MEM003"]
		);
	}

	#[test]
	fn generic_conditions_propagate_through_later_bodies() {
		let source = "function relay = <Item>(item: Item): Item => identity(item)\nfunction identity = <Item>(item: Item): Item => item\nconsole.log(relay([1]))\n";
		assert_eq!(codes(source), ["MEM004"]);
	}

	#[test]
	fn an_explicit_copy_never_consumes_a_task() {
		let source = "function invalid = (task: Task<Integer>): Task<Integer> => copy task\n";
		assert_eq!(codes(source), ["MEM003"]);
	}

	#[test]
	fn escaped_captures_reject_tasks_and_keep_the_outer_handle() {
		let source = "type Action = () => Void\nfunction invalid = (task: Task<Integer>): Action => { function f: Action = () => task.cancel()\nf }\n";
		assert_eq!(codes(source), ["MEM003"]);
		let generic = "type Reader<Item> = () => Item\nfunction retain = <Item>(item: Item): Reader<Item> => () => item\nfunction invalid = (task: Task<Integer>): Reader<Task<Integer>> => retain(task)\n";
		assert!(codes(generic).iter().any(|code| code == "MEM003"));
	}

	#[test]
	fn collections_can_move_into_an_immutable_environment() {
		let source = "type Reader = () => Integer\nfunction reader = (input: Array<Integer>): Reader => { let owned = copy input\n() => owned.length }\nconsole.log(reader([1])())\n";
		assert!(codes(source).is_empty(), "{:?}", codes(source));
		let source = "type Reader = () => Integer\nfunction reader = (input: Array<Integer>): Reader => () => input.length\n";
		assert_eq!(codes(source), ["MEM004"]);
	}

	#[test]
	fn generic_field_moves_keep_actual_resource_boundaries() {
		let source = "type Held = { task: Task<Integer>, function destroy = (self) => console.log(\"destroy\") }\nfunction take = <Holder: {task: Task<Integer>}>(holder: Holder): Task<Integer> => holder.task\nfunction invalid = (value: Held): Task<Integer> => take(value)\n";
		let mut session = crate::Session::new();
		let file = session.set_source("main.foreign", std::sync::Arc::from(source));
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|d| d.severity == crate::diagnostics::Severity::Error)
			.map(|d| (d.code.id, d.message.clone()))
			.collect();
		assert_eq!(
			errors.iter().map(|(code, _)| *code).collect::<Vec<_>>(),
			["MEM002"],
			"{errors:?}"
		);
	}

	#[test]
	fn generic_call_transfers_are_conditional_on_task_ownership() {
		let prefix = "function sink = <Item>(_item: Item): Void => {}\nfunction twice = <Item>(item: Item): Void => { sink(item)\nsink(item) }\n";
		assert!(codes(&format!("{prefix}twice(1)\ntwice([1])\n")).is_empty());
		let errors = codes(&format!(
			"{prefix}function invalid = (task: Task<Integer>): Void => twice(task)\n"
		));
		assert_eq!(errors, ["MEM003"]);
	}

	#[test]
	fn forall_parameters_preserve_actual_copy_conditions() {
		let source = "type Identity = <Item>(value: Item) => Item\nfunction clone = <Item>(item: Item): Item => copy item\nfunction invoke = <Item>(transform: Identity, value: Item): Item => transform(value)\nfunction invalid = (task: Task<Integer>): Task<Integer> => invoke(clone, task)\n";
		assert_eq!(codes(source), ["MEM003"]);
	}

	#[test]
	fn factories_preserve_returned_generic_copy_conditions() {
		let source = "type Identity = <Item>(value: Item) => Item\nfunction factory = (): Identity => <Item>(item: Item): Item => copy item\nfunction invalid = (task: Task<Integer>): Task<Integer> => { let transform = factory()\ntransform(task) }\n";
		assert_eq!(codes(source), ["MEM003"]);
	}

	#[test]
	fn returned_factories_rebind_captured_callable_parameters() {
		let source = "type Identity = <Item>(value: Item) => Item\nfunction clone = <Item>(item: Item): Item => copy item\nfunction wrap = (transform: Identity): Identity => <Item>(item: Item): Item => transform(item)\nfunction invalid = (task: Task<Integer>): Task<Integer> => { let wrapped = wrap(clone)\nwrapped(task) }\n";
		assert_eq!(codes(source), ["MEM003"]);
		let nested = source.replace("wrap(clone)", "wrap(wrap(clone))");
		assert_eq!(codes(&nested), ["MEM003"]);
	}

	#[test]
	fn function_fields_keep_generic_copy_conditions() {
		let source = "type Identity = <Item>(value: Item) => Item\nfunction clone = <Item>(item: Item): Item => copy item\nfunction invalid = (task: Task<Integer>): Task<Integer> => { let holder = { transform = clone }\nholder.transform(task) }\n";
		assert_eq!(codes(source), ["MEM003"]);
	}

	#[test]
	fn generic_function_materialization_checks_copy_conditions() {
		let prefix = "function clone = <Item>(item: Item): Item => copy item\n";
		assert_eq!(
			codes(&format!(
				"{prefix}let invalid: (value: Task<Integer>) => Task<Integer> = clone\n"
			)),
			["MEM003"]
		);
		assert_eq!(
			codes(&format!("{prefix}let invalid = clone<Task<Integer>>\n")),
			["MEM003"]
		);
		let valid = "type Identity = <Element>(value: Element) => Element\nfunction make = <Item>(state: Item): Identity => { let local = copy state\n<Element>(item: Element): Element => item }\nfunction valid = (task: Task<Integer>): Task<Integer> => { let transform = make(1)\ntransform(task) }\n";
		let (session, file) = crate::analyze_source(valid);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|error| error.severity == crate::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:#?}");
	}

	#[test]
	fn shared_captures_keep_their_lexical_owner() {
		let source = "type Token = { value: Integer, function destroy = (self) => {} }\nfunction outer = () => { let token = Token { value = 1 }\ndo { let holder = { action = () => token.value }\nconsole.log(holder.action()) } }\n";
		let mut session = crate::Session::new();
		let file = session.set_source("main.foreign", std::sync::Arc::from(source));
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|error| error.severity == crate::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:?}");
		assert!(session.analysis(file).moved_captures.is_empty());
		let generic = "type Reader<Item> = () => Item\nfunction hold = <Item>(value: Item): Reader<Item> => { let owned = copy value\n() => owned }\n";
		let (session, file) = crate::analyze_source(generic);
		assert!(codes(generic).is_empty(), "{:?}", codes(generic));
		assert!(session.analysis(file).moved_captures.is_empty());
		assert_eq!(session.analysis(file).collection_moved_captures.len(), 1);
	}

	#[test]
	fn hash_collections_share_only_their_items() {
		let mut types = crate::types::TypeStore::new();
		for kind in [
			crate::types::CollKind::Set,
			crate::types::CollKind::GrowingSet,
			crate::types::CollKind::Map,
			crate::types::CollKind::GrowingMap,
			crate::types::CollKind::Pool,
			crate::types::CollKind::GrowingPool,
		] {
			let plain =
				types.collection(kind, crate::ids::TypeId::STRING, crate::ids::TypeId::NONE);
			assert!(!types.contains_shared_handle(plain));
			assert!(
				!types
					.flags(plain)
					.contains(crate::types::TypeFlags::HAS_SHARED)
			);
			assert!(types.requires_explicit_copy(plain));
			let mutex = types.intern(crate::types::TypeKind::Sync(
				crate::types::SyncKind::Mutex,
				crate::ids::TypeId::INTEGER,
			));
			let shared = types.collection(kind, mutex, crate::ids::TypeId::NONE);
			assert!(types.contains_shared_handle(shared));
		}
	}

	#[test]
	fn methods_keep_their_owner_copy_conditions() {
		let source = "type Cloner<Item> = { function clone = (self, value: Item): Item => copy value }\nfunction invalid = (task: Task<Integer>): Task<Integer> => Cloner<Task<Integer>> {}.clone(task)\n";
		assert_eq!(codes(source), ["MEM003"]);
		let materialized = "type Cloner<Item> = { function clone = (self, value: Item): Item => copy value }\nlet invalid = Cloner<Task<Integer>>.clone\n";
		assert_eq!(codes(materialized), ["MEM003"]);
		let valid = "type Cloner<Item> = { function clone = (self, value: Item): Item => copy value }\nconsole.log(Cloner<Integer> {}.clone(1))\n";
		assert!(codes(valid).is_empty(), "{:?}", codes(valid));
	}

	#[test]
	fn task_holding_self_is_borrowed_by_a_static_method_call() {
		let source = "type Holder = { task: Task<Integer>, function inspect = (self): Integer => 1 }\nfunction observe = (holder: Holder): Integer => { Holder.inspect(holder)\nholder.inspect() }\n";
		assert!(codes(source).is_empty(), "{:?}", codes(source));
	}

	#[test]
	fn generic_task_transfers_in_a_loop_are_not_repeated() {
		let prefix = "function sink = <Item>(_item: Item): Void => {}\nfunction repeat = <Item>(item: Item): Void => { for 0..2 { sink(item) } }\n";
		assert!(codes(&format!("{prefix}repeat([1])\n")).is_empty());
		assert_eq!(
			codes(&format!(
				"{prefix}function invalid = (task: Task<Integer>): Void => repeat(task)\n"
			)),
			["MEM003"]
		);
	}
}
