//! Closed proofs for root-owned regions of recursive values.
//!
//! Each analyzed unit publishes semantic claims with stable identities. Their
//! union is resolved only after every module of the scanned graph contributed.
//! Native lowering retains its target-dependent size/layout guards; it may not
//! enable a region from a missing fact or from a file-local approximation.

use std::collections::{BTreeMap, BTreeSet};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{Analysis, Argument, Callee, FnInfo};
use crate::ids::{FileId, NodeId, NominalId, TypeId};
use crate::session::Session;
use crate::stable_names::{stable_decl_name, stable_type_name};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};
use crate::types::{CollKind, FieldFlags, NominalKind, PathRoot, TypeFlags, TypeKind, TypeStore};

/// Parts of a portable claim. Stable type/declaration spellings escape controls.
pub const SEPARATOR: char = '\u{1f}';
const OPAQUE: &str = "opaque";

/// The fact selecting a root type after the complete program was checked.
pub fn type_key(types: &TypeStore, root: TypeId) -> Option<String> {
	stable_type_name(types, root).map(|root| format!("type{SEPARATOR}{root}"))
}

/// A function's declaration identity, independent of local node/symbol ids.
pub fn function_key(session: &Session, file: FileId, function: NodeId) -> Option<String> {
	let analysis = session.analysis(file);
	let info = analysis.function(function)?;
	let key = info
		.symbol
		.and_then(|symbol| analysis.symbols.symbol(symbol).key)
		.or_else(|| {
			let declaration = info.declaration.unwrap_or(function);
			session
				.source_file(file)
				.interface
				.keys
				.iter()
				.filter(|(_, keyed)| keyed.node == declaration)
				.min_by(|(left, _), (right, _)| session.key_cmp(**left, **right))
				.map(|(&key, _)| key)
		})?;
	Some(stable_decl_name(session.types(), key))
}

/// The fact selecting this builder for this root type.
pub fn builder_key(function: &str, root: &str) -> String {
	format!("builder{SEPARATOR}{function}{SEPARATOR}{root}")
}

/// Fresh local arrays of this type, constructed in this builder, belong to
/// its region. Ordinary root arrays retain independent ownership per element.
pub fn child_array_key(function: &str, array: &str) -> String {
	format!("child-array{SEPARATOR}{function}{SEPARATOR}{array}")
}

/// A proven root array whose capacity is never observed anywhere in the
/// complete program. Missing facts always preserve the original capacity.
pub fn capacity_unobserved_key(types: &TypeStore, array: TypeId) -> Option<String> {
	matches!(types.kind(array), TypeKind::Collection(CollKind::Array, ..))
		.then(|| stable_type_name(types, array))
		.flatten()
		.map(|array| format!("capacity-unobserved{SEPARATOR}{array}"))
}

/// Recursive root shapes mentioned by this file, including imported types.
/// These are only candidates: lowering must require the resolved type fact.
pub fn candidate_types(session: &Session, file: FileId) -> Vec<TypeId> {
	let view = RegionFile::new(session, file);
	let mut candidates: Vec<_> = view
		.mentioned()
		.into_iter()
		.filter(|&ty| view.shape(ty).is_some())
		.collect();
	candidates.sort_by_cached_key(|&ty| stable_type_name(session.types(), ty));
	candidates
}

/// Unit-local claims. The graph manifest is emitted by its first module only;
/// every unit names that required manifest. This keeps total work linear in
/// modules, while a missing first module or any other module rejects the proof.
pub fn unit_claims(session: &Session, files: &[FileId]) -> BTreeSet<String> {
	let mut claims = crate::closed_world::claims(session, files);
	for &file in files {
		if !crate::closed_world::checked(session, file) {
			claims.insert(OPAQUE.to_string());
			continue;
		}
		RegionFile::new(session, file).collect(&mut claims);
	}
	claims
}

/// Resolves a union of unit contributions. Positive facts are emitted only
/// for a complete graph, a valid shape, at least one constructor, and no
/// forbidden use or constructor outside an admitted builder in any unit.
pub fn resolve_claims(claims: BTreeSet<String>) -> BTreeSet<String> {
	if claims.contains(OPAQUE) || !crate::closed_world::complete(&claims) {
		return BTreeSet::new();
	}
	let mut candidates = BTreeSet::new();
	let mut rejected = BTreeSet::new();
	let mut capacity_observed = BTreeSet::new();
	let mut constructed = BTreeSet::new();
	let mut builders = BTreeMap::<String, BTreeSet<String>>::new();
	let mut arrays = BTreeMap::<String, BTreeSet<String>>::new();
	let mut children = Vec::new();
	for claim in claims {
		let parts: Vec<_> = claim.split(SEPARATOR).collect();
		match parts.as_slice() {
			["requires", _] | ["module", _] | ["manifest", _, _] => {}
			["candidate", root] => {
				candidates.insert((*root).to_string());
			}
			["reject", root] => {
				rejected.insert((*root).to_string());
			}
			["capacity-observed", root] => {
				capacity_observed.insert((*root).to_string());
			}
			["constructed", root] => {
				constructed.insert((*root).to_string());
			}
			["builder", function, root] => {
				builders
					.entry((*root).to_string())
					.or_default()
					.insert((*function).to_string());
			}
			["array", root, array] => {
				arrays
					.entry((*root).to_string())
					.or_default()
					.insert((*array).to_string());
			}
			["child-array", root, function, array] => {
				children.push((
					(*root).to_string(),
					(*function).to_string(),
					(*array).to_string(),
				));
			}
			_ => return BTreeSet::new(),
		}
	}
	let mut resolved = BTreeSet::new();
	for root in candidates.intersection(&constructed) {
		if rejected.contains(root) {
			continue;
		}
		resolved.insert(format!("type{SEPARATOR}{root}"));
		if let Some(functions) = builders.get(root) {
			resolved.extend(functions.iter().map(|function| builder_key(function, root)));
		}
		if let Some(types) = arrays.get(root) {
			resolved.extend(types.iter().map(|array| format!("array{SEPARATOR}{array}")));
			if !capacity_observed.contains(root) {
				resolved.extend(
					types
						.iter()
						.map(|array| format!("capacity-unobserved{SEPARATOR}{array}")),
				);
			}
		}
		for (owner, function, array) in &children {
			if owner == root && builders.get(root).is_some_and(|all| all.contains(function)) {
				resolved.insert(child_array_key(function, array));
			}
		}
	}
	resolved
}

struct Root {
	ty: TypeId,
	name: String,
	nominal: Option<NominalId>,
	shapes: FxHashSet<TypeId>,
}

struct RegionFile<'a> {
	session: &'a Session,
	file: FileId,
	tree: &'a fa_parser::TypedSyntaxTree,
	analysis: &'a Analysis,
	types: &'a TypeStore,
	parents: Vec<Option<NodeId>>,
	owners: Vec<Option<NodeId>>,
	function_names: FxHashMap<NodeId, String>,
}

impl<'a> RegionFile<'a> {
	fn new(session: &'a Session, file: FileId) -> Self {
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let mut parents = vec![None; tree.nodes.len()];
		for (index, node) in tree.nodes.iter().enumerate() {
			node.for_each_child(|child| parents[child] = Some(NodeId::new(index)));
		}
		let bodies: FxHashMap<_, _> = analysis
			.functions
			.iter()
			.map(|(&key, info)| (info.body, key))
			.collect();
		let mut owners = vec![None; tree.nodes.len()];
		let mut stack: Vec<_> = parents
			.iter()
			.enumerate()
			.filter(|(_, parent)| parent.is_none())
			.map(|(index, _)| (NodeId::new(index), None))
			.collect();
		while let Some((node, owner)) = stack.pop() {
			let owner = bodies.get(&node).copied().or(owner);
			owners[node.index()] = owner;
			tree.nodes[node.index()]
				.for_each_child(|child| stack.push((NodeId::new(child), owner)));
		}
		let function_names = analysis
			.functions
			.keys()
			.filter_map(|&function| {
				function_key(session, file, function).map(|name| (function, name))
			})
			.collect();
		Self {
			session,
			file,
			tree,
			analysis,
			types: session.types(),
			parents,
			owners,
			function_names,
		}
	}

	fn view(&self, node: NodeId) -> View<'a> {
		syntax::view(self.tree, node)
	}

	fn contains(&self, ty: TypeId, root: TypeId) -> bool {
		let mut seen = FxHashSet::default();
		let mut pending = vec![ty];
		while let Some(ty) = pending.pop() {
			if ty == root
				|| matches!((self.types.kind(ty), self.types.kind(root)), (TypeKind::Member(a, _) | TypeKind::NamedUnion(a, _), TypeKind::NamedUnion(b, _)) if a == b)
			{
				return true;
			}
			if !seen.insert(ty) {
				continue;
			}
			match self.types.kind(ty) {
				TypeKind::Rec(_) => pending.extend(self.types.rec_target(ty)),
				TypeKind::Object(_) => {
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty))
				}
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.types.list_items(list))
				}
				TypeKind::Member(nominal, index) => {
					pending.push(self.types.payload(nominal, index))
				}
				TypeKind::NamedUnion(nominal, _) => {
					pending.extend(
						(0..self
							.types
							.sym_list(self.types.nominal(nominal).members)
							.len())
							.map(|index| self.types.payload(nominal, index as u16)),
					);
				}
				TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
					pending.extend([item, key])
				}
				TypeKind::Pointer(item)
				| TypeKind::Sync(_, item)
				| TypeKind::Ref(item, _)
				| TypeKind::CArray(item, _)
				| TypeKind::Iterator(item, _) => pending.push(item),
				_ => {}
			}
		}
		false
	}

	fn root_array(&self, ty: TypeId, root: TypeId) -> bool {
		matches!(self.types.kind(ty), TypeKind::Collection(CollKind::Array, item, _) if item == root || self.types.rec_target(item) == Some(root))
	}

	fn direct_root(&self, ty: TypeId, root: TypeId) -> bool {
		ty == root
			|| self.types.rec_target(ty) == Some(root)
			|| matches!((self.types.kind(ty), self.types.kind(root)), (TypeKind::Member(a, _) | TypeKind::NamedUnion(a, _), TypeKind::NamedUnion(b, _)) if a == b)
	}

	fn passive(&self, ty: TypeId, owning: bool) -> bool {
		let mut seen = FxHashSet::default();
		let mut pending = vec![ty];
		while let Some(ty) = pending.pop() {
			if !seen.insert(ty) {
				continue;
			}
			let flags = self.types.flags(ty);
			if flags.intersects(
				TypeFlags::HAS_PARAM
					| TypeFlags::HAS_UNKNOWN
					| TypeFlags::MOVE_ONLY
					| TypeFlags::RECURSIVE,
			) {
				return false;
			}
			match self.types.kind(ty) {
				TypeKind::Never
				| TypeKind::Void
				| TypeKind::None
				| TypeKind::Boolean
				| TypeKind::BoolLit(_)
				| TypeKind::Int(_)
				| TypeKind::Float(_)
				| TypeKind::FloatLit(..)
				| TypeKind::Percentage
				| TypeKind::Enum(..)
				| TypeKind::EnumMember(..)
				| TypeKind::Fields(_)
				| TypeKind::Range
				| TypeKind::Pointer(_)
				| TypeKind::CString => {}
				TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character if owning => {}
				TypeKind::Object(_) if !self.types.is_resource(ty) => {
					if self.types.fields_of(ty).iter().any(|field| {
						field
							.flags
							.intersects(FieldFlags::RELATION | FieldFlags::REACTIVE)
					}) {
						return false;
					}
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty));
				}
				TypeKind::Tuple(list) => pending.extend(self.types.list_items(list)),
				TypeKind::Union(list)
					if self.types.list_items(list).len() == 2
						&& self.types.list_items(list).contains(&TypeId::NONE) =>
				{
					pending.extend(self.types.list_items(list));
				}
				TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _)
					if owning =>
				{
					pending.push(item)
				}
				_ => return false,
			}
		}
		true
	}

	fn shape(&self, ty: TypeId) -> Option<Root> {
		if self
			.types
			.flags(ty)
			.intersects(TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN | TypeFlags::MOVE_ONLY)
		{
			return None;
		}
		let mut recursive = false;
		let mut shapes = FxHashSet::default();
		let nominal = match self.types.kind(ty) {
			TypeKind::Object(_) if !self.types.is_resource(ty) => {
				shapes.insert(ty);
				for field in self.types.fields_of(ty) {
					if field
						.flags
						.intersects(FieldFlags::RELATION | FieldFlags::REACTIVE)
					{
						return None;
					}
					let optional = match self.types.kind(field.ty) {
						TypeKind::Union(list) => {
							let members = self.types.list_items(list);
							members.len() == 2
								&& members.contains(&TypeId::NONE)
								&& members.iter().any(|&member| self.direct_root(member, ty))
						}
						_ => false,
					};
					if optional || self.root_array(field.ty, ty) {
						recursive = true;
					} else if !self.passive(field.ty, false) {
						return None;
					}
				}
				None
			}
			TypeKind::NamedUnion(nominal, _)
				if self.types.nominal(nominal).kind == NominalKind::Union =>
			{
				for index in 0..self
					.types
					.sym_list(self.types.nominal(nominal).members)
					.len()
				{
					let data = self.types.payload(nominal, index as u16);
					if data.is_unset() || data == TypeId::VOID {
						continue;
					}
					if !matches!(self.types.kind(data), TypeKind::Object(_))
						|| self.types.is_resource(data)
					{
						return None;
					}
					shapes.insert(data);
					for field in self.types.fields_of(data) {
						if field
							.flags
							.intersects(FieldFlags::RELATION | FieldFlags::REACTIVE)
						{
							return None;
						}
						if self.direct_root(field.ty, ty) || self.root_array(field.ty, ty) {
							recursive = true;
						} else if self.contains(field.ty, ty) || !self.passive(field.ty, true) {
							return None;
						}
					}
				}
				Some(nominal)
			}
			_ => return None,
		};
		if !recursive {
			return None;
		}
		Some(Root {
			ty,
			name: stable_type_name(self.types, ty)?,
			nominal,
			shapes,
		})
	}

	fn mentioned(&self) -> FxHashSet<TypeId> {
		let mut seen = FxHashSet::default();
		let mut pending: Vec<_> = self
			.analysis
			.types
			.iter()
			.copied()
			.chain(self.analysis.coercions.values().copied())
			.chain(
				self.analysis
					.symbols
					.symbols()
					.iter()
					.map(|symbol| symbol.ty),
			)
			.chain(self.analysis.type_decls.values().map(|decl| decl.ty))
			.collect();
		while let Some(ty) = pending.pop() {
			if !seen.insert(ty) {
				continue;
			}
			match self.types.kind(ty) {
				TypeKind::Rec(_) => pending.extend(self.types.rec_target(ty)),
				TypeKind::Object(_) => {
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty))
				}
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.types.list_items(list))
				}
				TypeKind::Member(nominal, _) | TypeKind::NamedUnion(nominal, _) => {
					pending.extend(
						(0..self
							.types
							.sym_list(self.types.nominal(nominal).members)
							.len())
							.map(|index| self.types.payload(nominal, index as u16)),
					);
				}
				TypeKind::Collection(_, item, key) | TypeKind::Task(item, key) => {
					pending.extend([item, key])
				}
				TypeKind::Pointer(item)
				| TypeKind::Sync(_, item)
				| TypeKind::Ref(item, _)
				| TypeKind::CArray(item, _)
				| TypeKind::Iterator(item, _) => pending.push(item),
				TypeKind::Function(sig) => {
					let sig = self.types.sig(sig);
					pending.extend([sig.ret, sig.throws]);
					pending.extend(self.types.params(sig.params).iter().map(|param| param.ty));
				}
				_ => {}
			}
		}
		seen
	}

	fn builder(&self, function: NodeId, info: &FnInfo, root: &Root) -> bool {
		let Some(sig) = info.sig.map(|sig| self.types.sig(sig)) else {
			return false;
		};
		let returns_root = if root.nominal.is_some() {
			self.direct_root(sig.ret, root.ty)
		} else {
			sig.ret == root.ty
		};
		if !returns_root
			|| sig.is_async
			|| (!matches!(self.types.kind(root.ty), TypeKind::NamedUnion(..))
				&& sig.throws != TypeId::NEVER)
			|| info.params.iter().any(|param| param.default.is_some())
			|| !self.function_names.contains_key(&function)
			|| matches!(
				info.declaration.map(|node| self.view(node)),
				None | Some(View::Lambda { .. })
			) {
			return false;
		}
		let mut stack = vec![info.body];
		while let Some(node) = stack.pop() {
			if matches!(self.view(node), View::Function { .. } | View::Lambda { .. }) {
				return false;
			}
			self.tree.nodes[node.index()].for_each_child(|child| stack.push(NodeId::new(child)));
		}
		true
	}

	fn place_contains(&self, node: NodeId, root: &Root) -> bool {
		let mut seen = FxHashSet::default();
		let mut stack = vec![node];
		while let Some(node) = stack.pop() {
			if !seen.insert(node) {
				continue;
			}
			let ty = self.analysis.type_of(node);
			if root.shapes.contains(&ty) || self.contains(ty, root.ty) {
				return true;
			}
			if let Resolution::Local(symbol) = self.analysis.resolution(node) {
				let symbol = self.analysis.symbols.symbol(symbol);
				if matches!(symbol.kind, SymbolKind::Use | SymbolKind::MutableUse) {
					stack.push(symbol.decl_node);
				}
			}
			self.tree.nodes[node.index()].for_each_child(|child| stack.push(NodeId::new(child)));
		}
		false
	}

	fn array_literal(&self, mut node: NodeId) -> bool {
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		matches!(self.view(node), View::Array { .. })
	}

	fn argument(&self, argument: &Argument) -> Option<NodeId> {
		match *argument {
			Argument::Given(node) => Some(node),
			Argument::Receiver(node) => match self.view(node) {
				View::Access { target, members } if members.len() == 1 => Some(target),
				_ => Some(node),
			},
			_ => None,
		}
	}

	fn builtin_receiver(&self, node: NodeId) -> Option<NodeId> {
		let View::Call { callee, .. } = self.view(node) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		match members.len() {
			0 => None,
			1 => Some(target),
			length => members.iter().nth(length - 2),
		}
	}

	/// A direct child-array field of an independent builder local. Parameters,
	/// aliases, globals and deeper projections cannot publish new region edges.
	fn builder_array_field(&self, call: NodeId, owner: Option<NodeId>, root: &Root) -> bool {
		let Some(owner) = owner else { return false };
		if root.nominal.is_some() {
			return false;
		}
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let View::Access { target, members } = self.view(callee) else {
			return false;
		};
		if members.len() != 2 || !self.direct_root(self.analysis.type_of(target), root.ty) {
			return false;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(target) else {
			return false;
		};
		let symbol = self.analysis.symbols.symbol(symbol);
		matches!(symbol.kind, SymbolKind::Mutable | SymbolKind::Let)
			&& self.owners[symbol.decl_node.index()] == Some(owner)
	}

	/// Writing finite, non-owning data cannot detach or invalidate a region
	/// edge. Union payloads retain their immutable compact-layout contract.
	fn passive_write(&self, target: NodeId, root: &Root) -> bool {
		let ty = self.analysis.type_of(target);
		root.nominal.is_none() && !self.contains(ty, root.ty) && self.passive(ty, false)
	}

	fn unproven_conversion(&self, node: NodeId, root: &Root) -> bool {
		let Some(&target) = self.analysis.coercions.get(&node) else {
			return false;
		};
		if root.nominal.is_some() {
			return false;
		}
		let source = self.analysis.type_of(node);
		let from_root = self.contains(source, root.ty);
		let to_root = self.contains(target, root.ty);
		if from_root {
			return !to_root;
		}
		to_root
			&& !matches!(self.types.kind(source), TypeKind::None | TypeKind::Never)
			&& !self
				.analysis
				.object(node)
				.is_some_and(|object| object.ty == root.ty)
			&& !(self.root_array(target, root.ty)
				&& matches!(self.types.kind(source), TypeKind::Collection(CollKind::Array, item, _) if item == TypeId::NEVER))
	}

	fn array_projection(&self, node: NodeId, root: &Root) -> bool {
		let mut pending = vec![node];
		while let Some(node) = pending.pop() {
			if self.root_array(self.analysis.type_of(node), root.ty)
				&& let View::Access { target, .. } = self.view(node)
				&& self.contains(self.analysis.type_of(target), root.ty)
			{
				return true;
			}
			self.tree.nodes[node.index()].for_each_child(|child| pending.push(NodeId::new(child)));
		}
		false
	}

	fn borrowed_pattern(&self, node: NodeId, function: NodeId, root: &Root) -> bool {
		let Some(info) = self.analysis.function(function) else {
			return false;
		};
		let mut current = node;
		while let Some(parent) = self.parents[current.index()] {
			if self.owners[parent.index()] != Some(function) {
				break;
			}
			let subject = match self.view(parent) {
				View::When { subject, .. } | View::Destructuring { value: subject, .. } => subject,
				_ => {
					current = parent;
					continue;
				}
			};
			// A conditional expression may mix borrowed and owned branches;
			// only an actual parameter place proves the payload stays borrowed.
			return !crate::lower::copies::is_move(self.tree, self.analysis, subject)
				&& crate::lower::copies::place_root(self.tree, self.analysis, subject)
					.is_some_and(|symbol| {
						info.params.iter().any(|param| param.symbol == symbol)
							&& self.contains(self.analysis.type_of(subject), root.ty)
					});
		}
		false
	}

	fn root_constructor(&self, mut node: NodeId, root: &Root) -> bool {
		loop {
			if self.analysis.objects.get(&node).is_some_and(|object| {
				object
					.member
					.is_some_and(|member| self.direct_root(member, root.ty))
			}) || self.analysis.call(node).is_some_and(
				|call| matches!(call.callee, Callee::Member(member) if self.direct_root(member, root.ty)),
			) {
				return true;
			}
			let Some(parent) = self.parents[node.index()] else {
				return false;
			};
			// The data literal of a member constructor is transparent. Do not
			// accept arbitrary objects merely nested somewhere in its fields.
			if !matches!(self.view(parent), View::TypedObject { .. } | View::Group(_)) {
				return false;
			}
			node = parent;
		}
	}

	fn child_array_origin(&self, mut node: NodeId, function: NodeId, root: &Root) -> bool {
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		if !self.root_array(self.analysis.type_of(node), root.ty)
			|| self.owners[node.index()] != Some(function)
		{
			return false;
		}
		if self.array_literal(node) {
			return true;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			return false;
		};
		let symbol = self.analysis.symbols.symbol(symbol);
		matches!(self.view(symbol.decl_node), View::Declaration { value: Some(value), .. } if self.array_literal(value) && self.owners[value.index()] == Some(function))
	}

	fn embedded(&self, at: TypeId, root: &Root) -> bool {
		let contains = |ty| self.contains(ty, root.ty);
		match self.types.kind(at) {
			TypeKind::Object(_) if !root.shapes.contains(&at) => self
				.types
				.fields_of(at)
				.iter()
				.any(|field| contains(field.ty)),
			TypeKind::Collection(_, item, key) if !self.root_array(at, root.ty) => {
				contains(item) || contains(key)
			}
			TypeKind::Tuple(list) => self.types.list_items(list).iter().any(|&ty| contains(ty)),
			TypeKind::Pointer(item)
			| TypeKind::Sync(_, item)
			| TypeKind::Task(item, _)
			| TypeKind::Ref(item, _)
			| TypeKind::CArray(item, _)
			| TypeKind::Iterator(item, _) => contains(item),
			TypeKind::Member(nominal, index) if Some(nominal) != root.nominal => {
				contains(self.types.payload(nominal, index))
			}
			TypeKind::NamedUnion(nominal, _) if Some(nominal) != root.nominal => (0..self
				.types
				.sym_list(self.types.nominal(nominal).members)
				.len())
				.any(|index| contains(self.types.payload(nominal, index as u16))),
			_ => false,
		}
	}

	fn collect(&self, claims: &mut BTreeSet<String>) {
		if !self.analysis.extern_calls.is_empty()
			|| !self
				.session
				.source_file(self.file)
				.interface
				.externs
				.is_empty()
			|| !self.analysis.key_loop_bodies.is_empty()
			|| self.analysis.functions.values().any(|info| {
				!info.generics.is_empty()
					|| info.sig.is_some_and(|sig| self.types.sig(sig).is_async)
			}) || self
			.analysis
			.types
			.iter()
			.any(|&ty| matches!(self.types.kind(ty), TypeKind::Task(..)))
		{
			claims.insert(OPAQUE.to_string());
			return;
		}
		let mentioned = self.mentioned();
		for root in mentioned.iter().filter_map(|&ty| self.shape(ty)) {
			claims.insert(format!("candidate{SEPARATOR}{}", root.name));
			let builders: FxHashSet<_> = self
				.analysis
				.functions
				.iter()
				.filter(|(key, info)| self.builder(**key, info, &root))
				.map(|(&key, _)| key)
				.collect();
			for function in &builders {
				claims.insert(builder_key(&self.function_names[function], &root.name));
			}
			if mentioned.iter().any(|&ty| self.embedded(ty, &root))
				|| self.analysis.functions.values().any(|info| {
					info.captures.iter().any(|&symbol| {
						self.contains(self.analysis.symbols.symbol(symbol).ty, root.ty)
					}) || info.sig.is_some_and(|sig| {
						let ret = self.types.sig(sig).ret;
						self.root_array(ret, root.ty)
							|| root.nominal.is_some()
								&& root.shapes.contains(&ret)
								&& self.contains(ret, root.ty)
					})
				}) {
				claims.insert(format!("reject{SEPARATOR}{}", root.name));
			}
			for &ty in &mentioned {
				if self.root_array(ty, root.ty)
					&& let Some(array) = stable_type_name(self.types, ty)
				{
					claims.insert(format!("array{SEPARATOR}{}{SEPARATOR}{array}", root.name));
				}
			}
			self.collect_uses(&root, &builders, claims);
		}
	}

	fn collect_uses(
		&self,
		root: &Root,
		builders: &FxHashSet<NodeId>,
		claims: &mut BTreeSet<String>,
	) {
		let reject = |claims: &mut BTreeSet<String>| {
			claims.insert(format!("reject{SEPARATOR}{}", root.name));
		};
		for symbol in self.analysis.symbols.symbols() {
			if !self.root_array(symbol.ty, root.ty)
				|| !matches!(
					symbol.kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::PatternBinding
				) {
				continue;
			}
			let owner = self.owners[symbol.decl_node.index()];
			match self.view(symbol.decl_node) {
				View::Declaration {
					value: Some(value), ..
				} => {
					if let Some(function) = owner.filter(|function| builders.contains(function)) {
						if !self.array_literal(value) {
							reject(claims);
						} else if let Some(array) = stable_type_name(self.types, symbol.ty) {
							claims.insert(format!(
								"child-array{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{array}",
								root.name, self.function_names[&function]
							));
						}
					} else if self.array_projection(value, root) {
						reject(claims);
					}
				}
				_ => {
					// A reader may borrow a payload array through a pattern. An
					// owned/temporary subject or a builder needs cloning support.
					let borrowed_reader = owner
						.filter(|function| !builders.contains(function))
						.is_some_and(|function| {
							self.borrowed_pattern(symbol.decl_node, function, root)
						});
					if !borrowed_reader {
						reject(claims);
					}
				}
			}
		}
		for (&node, object) in &self.analysis.objects {
			if root.nominal.is_some()
				&& object.member.is_none()
				&& root.shapes.contains(&object.ty)
				&& self.contains(object.ty, root.ty)
				&& !self.root_constructor(node, root)
			{
				reject(claims);
			}
			let constructs = match (root.nominal, object.member) {
				(Some(nominal), Some(member)) => {
					matches!(self.types.kind(member), TypeKind::Member(actual, _) if actual == nominal)
				}
				(None, _) => object.ty == root.ty,
				_ => false,
			};
			if !constructs {
				continue;
			}
			claims.insert(format!("constructed{SEPARATOR}{}", root.name));
			let Some(function) =
				self.owners[node.index()].filter(|function| builders.contains(function))
			else {
				reject(claims);
				continue;
			};
			for (name, value) in &object.fields {
				let crate::analysis::FieldValue::Given(value) = *value else {
					if root.nominal.is_none()
						&& self
							.types
							.fields_of(object.ty)
							.iter()
							.any(|field| field.name == *name && self.root_array(field.ty, root.ty))
					{
						reject(claims);
					}
					continue;
				};
				if self.root_array(self.analysis.type_of(value), root.ty) {
					if !self.child_array_origin(value, function, root) {
						reject(claims);
					} else if let Some(array) =
						stable_type_name(self.types, self.analysis.type_of(value))
					{
						claims.insert(format!(
							"child-array{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{array}",
							root.name, self.function_names[&function]
						));
					}
				}
			}
		}
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			let ty = self.analysis.type_of(node);
			if self.unproven_conversion(node, root) {
				// A structural conversion can manufacture this layout without
				// entering its builder. Its owning fields have no region proof.
				reject(claims);
			}
			let owner = self.owners[index].filter(|owner| builders.contains(owner));
			if let View::Access { target, members } | View::OptionalAccess { target, members } =
				self.view(node)
			{
				let mut receiver = self.analysis.type_of(target);
				for member in members.iter() {
					if matches!(self.analysis.resolution(member), Resolution::Builtin(id) if crate::builtins::get(id).path == "Array#capacity")
						&& self.contains(receiver, root.ty)
					{
						claims.insert(format!("capacity-observed{SEPARATOR}{}", root.name));
					}
					receiver = self.analysis.type_of(member);
				}
			}
			let constant_roots = self.analysis.constant(node).is_some_and(|value| {
				!matches!(value, fa_interpreter::Value::None)
					&& !matches!(value, fa_interpreter::Value::Member(member) if member.data.is_none())
					&& !matches!(value, fa_interpreter::Value::Array(items) if items.is_empty())
			});
			if constant_roots && self.contains(ty, root.ty) {
				reject(claims);
			}
			if self.root_array(ty, root.ty)
				&& let View::Access { .. } = self.view(node)
				&& (crate::lower::copies::value_place_root(self.tree, self.analysis, node)
					.is_none() || crate::lower::copies::is_move(self.tree, self.analysis, node))
			{
				reject(claims);
			}
			if matches!(
				self.view(node),
				View::Use { .. } | View::Reuse { .. } | View::Provide { .. }
			) && self.place_contains(node, root)
			{
				reject(claims);
			}
			if ty == TypeId::VOID {
				if let View::Assignment { target, value, .. } = self.view(node) {
					let own_local = owner.is_some()
						&& matches!(self.analysis.resolution(target), Resolution::Local(symbol) if self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable && self.owners[self.analysis.symbols.symbol(symbol).decl_node.index()] == owner)
						&& matches!(self.view(target), View::Name(_));
					if self.place_contains(target, root)
						&& !self.passive_write(target, root)
						&& (!own_local
							|| self.root_array(self.analysis.type_of(target), root.ty)
								&& value.is_some_and(|value| {
									!self.child_array_origin(
										value,
										owner.expect("local builder"),
										root,
									)
								})) {
						reject(claims);
					}
				} else if let View::CompoundAssignment { target, .. }
				| View::UpdateFields { target, .. } = self.view(node)
					&& self.place_contains(target, root)
					&& !self.passive_write(target, root)
				{
					reject(claims);
				}
			}
			if let Some(call) = self.analysis.call(node) {
				if let Callee::Member(member) = call.callee
					&& self.direct_root(member, root.ty)
				{
					claims.insert(format!("constructed{SEPARATOR}{}", root.name));
					if owner.is_none() {
						reject(claims);
					}
				}
				let sig = match call.callee {
					Callee::Function(function) => {
						self.analysis.function(function).and_then(|info| info.sig)
					}
					_ => match self.view(node) {
						View::Call { callee, .. } => {
							match self.types.kind(self.analysis.type_of(callee)) {
								TypeKind::Function(sig) => Some(sig),
								_ => None,
							}
						}
						_ => None,
					},
				};
				let builtin = match call.callee {
					Callee::Builtin(id) => Some(crate::builtins::get(id).path),
					_ => None,
				};
				let reads_array = builtin == Some("Array#get")
					&& self.builtin_receiver(node).is_some_and(|argument| {
						self.root_array(self.analysis.type_of(argument), root.ty)
					});
				if builtin.is_some() && self.contains(ty, root.ty) && !reads_array {
					reject(claims);
				}
				let Some(sig) = sig else {
					continue;
				};
				let (mutates, invalidates) = self.types.effects(self.types.sig(sig).effects);
				for path in mutates.iter().chain(invalidates) {
					let (path, _) = self.types.path_data(*path);
					let argument = match path.root {
						PathRoot::SelfParam if builtin.is_some() => self.builtin_receiver(node),
						PathRoot::SelfParam => call
							.arguments
							.first()
							.and_then(|argument| self.argument(argument)),
						PathRoot::Param(index) => call
							.arguments
							.get(index as usize)
							.and_then(|argument| self.argument(argument)),
						_ => continue,
					};
					let Some(value) = argument else {
						reject(claims);
						continue;
					};
					if !self.place_contains(value, root) {
						continue;
					}
					let allowed_array =
						self.root_array(self.analysis.type_of(value), root.ty)
							&& matches!(self.view(value), View::Name(_))
							&& matches!(self.analysis.resolution(value), Resolution::Local(symbol) if {
								let symbol = self.analysis.symbols.symbol(symbol);
								matches!(symbol.kind, SymbolKind::Mutable | SymbolKind::Let)
									&& self.owners[symbol.decl_node.index()] == self.owners[index]
							}) && matches!(builtin, Some("Array#push" | "Array#reserve"));
					let builder_field = self.root_array(self.analysis.type_of(value), root.ty)
						&& self.builder_array_field(node, owner, root)
						&& matches!(builtin, Some("Array#push" | "Array#reserve"));
					// Every body contributes its stores to this same closed proof.
					// An indirect/imported function value has no such body identity.
					let checked_call =
						root.nominal.is_none() && matches!(call.callee, Callee::Function(_));
					if !allowed_array && !builder_field && !checked_call {
						reject(claims);
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const TREE: &str = r#"
type Tree = Union {
	Leaf = { text: String }
	Branch = { left: Tree, right: Tree }
}
function build = (depth: Integer): Tree => if depth == 0 {
	Tree.Leaf { text = "leaf" }
} else {
	Tree.Branch { left = build(depth - 1), right = build(depth - 1) }
}
"#;

	fn facts(source: &str) -> (BTreeSet<String>, BTreeSet<String>) {
		facts_at("main", source)
	}

	fn facts_at(module: &str, source: &str) -> (BTreeSet<String>, BTreeSet<String>) {
		let mut session = Session::new();
		let file = session.set_source(module, source.into());
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|diagnostic| diagnostic.severity == crate::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:?}");
		let claims = unit_claims(&session, &[file]);
		let resolved = resolve_claims(claims.clone());
		(claims, resolved)
	}

	#[test]
	fn direct_recursive_builders_have_stable_positive_facts() {
		let (claims, resolved) =
			facts(&format!("{TREE}\nlet root = build(2)\nconsole.log(root)\n"));
		assert!(
			resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
		assert!(
			resolved
				.iter()
				.any(|fact| fact.starts_with("builder\u{1f}")),
			"{claims:#?}"
		);
	}

	#[test]
	fn a_missing_module_or_manifest_rejects_every_region() {
		let (claims, resolved) = facts(TREE);
		assert!(!resolved.is_empty(), "{claims:#?}");
		for prefix in ["module\u{1f}", "manifest\u{1f}", "requires\u{1f}"] {
			let incomplete = claims
				.iter()
				.filter(|claim| !claim.starts_with(prefix))
				.cloned()
				.collect();
			assert!(resolve_claims(incomplete).is_empty(), "missing {prefix:?}");
		}
		assert!(resolve_claims(BTreeSet::new()).is_empty());
	}

	#[test]
	fn json_producers_and_payload_alias_mutations_reject_regions() {
		for suffix in [
			r#"use { Json } = std.json
let root: Tree = Json.parse("{{}}") catch _error => build(0)
console.log(root)"#,
			r#"function change = (label: { text: String }) mutates label => { label.text += "!" }
mutable root = build(0)
if root is Tree.Leaf {
	use alias = root
	change(alias)
}
console.log(root)"#,
		] {
			let (claims, resolved) = facts(&format!("{TREE}\n{suffix}\n"));
			assert!(resolved.is_empty(), "{claims:#?}");
		}
	}

	#[test]
	fn root_arrays_and_child_arrays_have_distinct_ownership_roles() {
		let source = r#"
type Tree = Union {
	Leaf = { value: Integer }
	List = { items: Array<Tree> }
}
function build = (depth: Integer): Tree => {
	if depth == 0 { return Tree.Leaf { value = 7 } }
	mutable items: Array<Tree> = []
	items.push(build(depth - 1))
	Tree.List { items }
}
function count = (tree: Tree): Integer => when tree is {
	Tree.Leaf => 1
	Tree.List as { items } => {
		mutable result = 0
		for child in items { result += count(child) }
		result
	}
}
mutable roots: Array<Tree> = []
roots.push(build(2))
roots.push(build(3))
for root in roots { console.log(count(root)) }
"#;
		let (claims, resolved) = facts(source);
		assert!(
			resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
		assert!(
			resolved
				.iter()
				.any(|fact| fact.starts_with("child-array\u{1f}")),
			"{claims:#?}"
		);
		assert!(
			resolved.iter().any(|fact| fact.starts_with("array\u{1f}")),
			"{claims:#?}"
		);
	}

	#[test]
	fn capacity_observers_disable_compaction_without_disabling_regions() {
		let tree = r#"
type Tree = Union {
	Leaf = { value: Integer }
	List = { items: Array<Tree> }
}
function build = (depth: Integer): Tree => {
	if depth == 0 { return Tree.Leaf { value = 7 } }
	mutable items: Array<Tree> = []
	items.push(build(depth - 1))
	Tree.List { items }
}
mutable roots: Array<Tree> = []
roots.push(build(2))
"#;
		for (suffix, unobserved) in [
			("console.log(roots.length)", true),
			("console.log([1, 2].capacity)", true),
			("console.log(roots.capacity)", false),
			("let alias = roots\nconsole.log(alias.capacity)", false),
			(
				"function read = (items: Array<Tree>): Integer => items.capacity\nlet getter = read\nconsole.log(getter(roots))",
				false,
			),
			(
				"function read = (tree: Tree): Integer => when tree is { Tree.Leaf => 0, Tree.List as { items } => items.capacity }\nconsole.log(read(build(2)))",
				false,
			),
			(
				"function read = (tree: Tree): Integer => if tree is Tree.List { tree.items.capacity } else { 0 }\nconsole.log(read(build(2)))",
				false,
			),
		] {
			let (claims, resolved) = facts(&format!("{tree}\n{suffix}\n"));
			assert!(
				resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
				"{suffix}: {claims:#?}"
			);
			assert_eq!(
				resolved
					.iter()
					.any(|fact| fact.starts_with("capacity-unobserved\u{1f}")),
				unobserved,
				"{suffix}: {claims:#?}"
			);
		}
		// A member-only array is an unsupported container: it must not
		// silently acquire the full union array's compaction permission.
		let (claims, resolved) = facts(&format!(
			"{tree}\nfunction read = (items: Array<Tree.Leaf>): Integer => items.capacity\nconsole.log(read([Tree.Leaf {{ value = 1 }}]))\n"
		));
		assert!(resolved.is_empty(), "{claims:#?}");
	}

	#[test]
	fn a_remote_capacity_read_invalidates_the_positive_fact_in_every_mode() {
		use crate::pipeline::{self, UnitSink};
		use std::sync::Mutex;

		struct Claims(Mutex<Vec<BTreeSet<String>>>);
		impl UnitSink for Claims {
			fn unit(&self, session: &mut Session, files: &[FileId]) {
				self.0.lock().unwrap().push(unit_claims(session, files));
			}
		}
		for reads_capacity in [false, true] {
			let getter = if reads_capacity {
				"items.capacity"
			} else {
				"items.length"
			};
			let files: Vec<(String, String)> = [
				("package.fa".into(), "export = { name = \"capacityProof\" }".into()),
				("library/model.fa".into(), "export namespace = { type Tree = Union { Leaf = { value: Integer }, List = { items: Array<Tree> } } }".into()),
				("library/build.fa".into(), "export function = (depth: Integer): model.Tree => { if depth == 0 { return model.Tree.Leaf { value = 1 } }\nmutable items: Array<model.Tree> = []\nitems.push(build(depth - 1))\nmodel.Tree.List { items } }".into()),
				("library/read.fa".into(), format!("export function = (tree: model.Tree): Integer => when tree is {{ model.Tree.Leaf => 1, model.Tree.List as {{ items }} => {getter} }}")),
				("application/main.fa".into(), "export function = () => console.log(library.read(library.build(2)))".into()),
			].into();
			let mut session = Session::new();
			session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
			let program = session.analyze_package(Some("application/main"));
			assert!(
				program.files.iter().all(|&file| session
					.diagnostics(file)
					.iter()
					.all(|diagnostic| diagnostic.severity != crate::Severity::Error)),
				"{:?}",
				program
					.files
					.iter()
					.map(|&file| session.diagnostics(file))
					.collect::<Vec<_>>()
			);
			let expected = resolve_claims(unit_claims(&session, &program.files));
			assert!(expected.iter().any(|fact| fact.starts_with("type\u{1f}")));
			assert_eq!(
				expected
					.iter()
					.any(|fact| fact.starts_with("capacity-unobserved\u{1f}")),
				!reads_capacity
			);
			let mut parallel = Session::new();
			parallel.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
			let sink = Claims(Mutex::new(Vec::new()));
			let report = pipeline::analyze_package(
				&mut parallel,
				&pipeline::Options {
					entry: Some("application/main".into()),
					threads: Some(4),
					..Default::default()
				},
				None,
				&sink,
			);
			assert!(!report.has_errors(), "{:?}", report.modules);
			let contributions = sink.0.into_inner().unwrap();
			let claims = contributions.iter().flatten().cloned().collect();
			assert_eq!(resolve_claims(claims), expected);
			for missing in 0..contributions.len() {
				let incomplete = contributions
					.iter()
					.enumerate()
					.filter(|(index, _)| *index != missing)
					.flat_map(|(_, claims)| claims.iter().cloned())
					.collect();
				assert!(resolve_claims(incomplete).is_empty());
			}
		}
	}

	#[test]
	fn every_units_contribution_is_required_and_publication_order_is_irrelevant() {
		use crate::pipeline::{self, UnitSink};
		use std::sync::Mutex;

		struct Claims(Mutex<Vec<BTreeSet<String>>>);
		impl UnitSink for Claims {
			fn unit(&self, session: &mut Session, files: &[FileId]) {
				self.0.lock().unwrap().push(unit_claims(session, files));
			}
		}
		let mut files: Vec<(String, String)> = [
			("package.fa", "export = { name = \"regions\" }"),
			("library/model.fa", "export namespace = { type Tree = Union { Leaf = { text: String }, Branch = { left: Tree, right: Tree } } }"),
			("library/build.fa", "export function = (depth: Integer): model.Tree => if depth == 0 { model.Tree.Leaf { text = \"leaf\" } } else { model.Tree.Branch { left = build(depth - 1), right = build(depth - 1) } }"),
			("application/main.fa", "export function = () => console.log(library.build(2))"),
		].into_iter().map(|(path, source)| (path.into(), source.into())).collect();
		let mut expected = None;
		for (threads, seed) in [(1, 0), (2, 13), (8, 67)] {
			let mut session = Session::new();
			session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
			let sink = Claims(Mutex::new(Vec::new()));
			let report = pipeline::analyze_package(
				&mut session,
				&pipeline::Options {
					entry: Some("application/main".into()),
					threads: Some(threads),
					seed,
					..Default::default()
				},
				None,
				&sink,
			);
			assert!(!report.has_errors(), "{:?}", report.modules);
			let contributions = sink.0.into_inner().unwrap();
			assert_eq!(contributions.len(), 3);
			let claims: BTreeSet<_> = contributions.iter().flatten().cloned().collect();
			let resolved = resolve_claims(claims.clone());
			assert!(!resolved.is_empty(), "{claims:#?}");
			if let Some(expected) = &expected {
				assert_eq!(expected, &resolved);
			}
			expected = Some(resolved);
			for missing in 0..contributions.len() {
				let incomplete = contributions
					.iter()
					.enumerate()
					.filter(|(index, _)| *index != missing)
					.flat_map(|(_, claims)| claims.iter().cloned())
					.collect();
				assert!(
					resolve_claims(incomplete).is_empty(),
					"unit {missing} is missing"
				);
			}
		}
		let mut sequential = Session::new();
		sequential.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
		let program = sequential.analyze_package(Some("application/main"));
		let resolved = resolve_claims(unit_claims(&sequential, &program.files));
		assert_eq!(expected.as_ref(), Some(&resolved));
		files.push((
			"library/escape.fa".into(),
			"export function = (tree: model.Tree) => {\nlet box = { tree }\nconsole.log(box)\n}"
				.into(),
		));
		let mut session = Session::new();
		session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
		let program = session.analyze_package(Some("application/main"));
		assert!(program.files.iter().all(|&file| {
			session
				.diagnostics(file)
				.iter()
				.all(|diagnostic| diagnostic.severity != crate::Severity::Error)
		}));
		let claims = unit_claims(&session, &program.files);
		assert!(resolve_claims(claims.clone()).is_empty(), "{claims:#?}");
	}

	#[test]
	fn recursive_union_forests_of_the_parser_use_case_are_closed() {
		let (claims, resolved) = facts(include_str!("../../../benchmarks/sexp/main.fa"));
		assert!(
			resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
	}

	#[test]
	fn pattern_arrays_must_borrow_the_actual_subject_parameter() {
		let tree = r#"
type Tree = Union { Leaf, List = { items: Array<Tree> } }
function build = (depth: Integer): Tree => if depth == 0 {
	Tree.Leaf
} else {
	Tree.List { items = [build(depth - 1)] }
}
"#;
		for subject in ["build(1)", "if true { tree } else { build(1) }"] {
			let source = format!(
				"{tree}\nfunction count = (tree: Tree): Integer => when {subject} is {{\nTree.Leaf => 0\nTree.List as {{ items }} => items.length\n}}\nconsole.log(count(build(0)))\n"
			);
			let (claims, resolved) = facts(&source);
			assert!(resolved.is_empty(), "{claims:#?}");
		}
	}

	#[test]
	fn a_payload_shape_outside_its_member_is_an_escape() {
		for suffix in [
			"type Holder = { left: Tree, right: Tree }\nlet holder = Holder { left = build(0), right = build(0) }\nconsole.log(holder.left)",
			"function pair = (tree: Tree): { left: Tree, right: Tree } => ({ left = tree, right = tree })\nconsole.log(pair(build(0)))",
		] {
			let (claims, resolved) = facts(&format!("{TREE}\n{suffix}\n"));
			assert!(resolved.is_empty(), "{claims:#?}");
		}
	}

	#[test]
	fn unresolved_payloads_and_foreign_entries_have_no_region_proof() {
		let generic = "type Tree<T> = Union { Leaf = { value: T }, Branch = { left: Tree<T>, right: Tree<T> } }";
		let (claims, resolved) = facts(generic);
		assert!(resolved.is_empty(), "{claims:#?}");
		assert!(
			!claims
				.iter()
				.any(|claim| claim.starts_with("candidate\u{1f}")),
			"{claims:#?}"
		);
		let (claims, resolved) = facts_at(
			"entry.foreign",
			&format!(
				"{TREE}\nnamespace foreign = {{\nfunction absolute: (value: Int32) => Int32 = @extern(\"abs\")\n}}\nconsole.log(foreign.absolute(2), build(1))"
			),
		);
		assert!(resolved.is_empty(), "{claims:#?}");
		assert!(claims.contains(OPAQUE), "{claims:#?}");
	}

	#[test]
	fn a_builder_cannot_publish_children_into_an_outer_root_array() {
		for (writes, helper) in [
			("saved.push(child)", ""),
			(
				"save(child)",
				"function save = (tree: Tree) mutates saved => { saved.push(tree) }",
			),
			(
				"saveInto(saved, child)",
				"function saveInto = (array: Array<Tree>, tree: Tree) mutates array => { array.push(tree) }",
			),
		] {
			let source = format!(
				r#"
type Tree = Union {{ Leaf, Branch = {{ left: Tree, right: Tree }} }}
mutable saved: Array<Tree> = []
{helper}
function build = (depth: Integer): Tree mutates saved => {{
	if depth == 0 {{
		return Tree.Leaf
	}}
	let child = build(depth - 1)
	{writes}
	Tree.Branch {{ left = child, right = Tree.Leaf }}
}}
console.log(build(2))
console.log(saved)
"#
			);
			let (claims, resolved) = facts(&source);
			assert!(resolved.is_empty(), "{claims:#?}");
		}
	}

	#[test]
	fn constant_members_without_data_do_not_own_a_region() {
		let (claims, resolved) = facts(
			"type List = Union { Empty, Cell = { tail: List } }\nfunction build = (n: Integer): List => if n == 0 { List.Empty } else { List.Cell { tail = build(n - 1) } }\nlet empty = List.Empty\nconsole.log(empty, build(2))\n",
		);
		assert!(
			resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
	}

	#[test]
	fn recursive_object_arrays_keep_edges_closed_while_passive_fields_change() {
		let (claims, resolved) = facts(include_str!("../../../benchmarks/scene/main.fa"));
		assert!(
			resolved.iter().any(|key| key.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
		assert!(
			resolved
				.iter()
				.any(|key| key.starts_with("child-array\u{1f}")),
			"{claims:#?}"
		);
	}

	#[test]
	fn recursive_object_array_edges_cannot_be_mutated_or_aliased_by_readers() {
		let tree = "type Tree = { value: Integer, children: Array<Tree> }\nfunction build = (depth: Integer): Tree => { mutable tree = Tree { value = depth, children = [] }\nif depth > 0 { for index in 0..2 { tree.children.push(build(depth - 1)) } }\ntree }\n";
		for suffix in [
			"mutable tree = build(2)\ntree.children.push(build(0))\nconsole.log(tree)",
			"function grow = (tree: Tree) mutates tree => { tree.children.push(build(0)) }\nmutable tree = build(2)\ngrow(tree)",
			"mutable tree = build(2)\nuse children = tree.children\nconsole.log(children)",
			"mutable tree = build(2)\ntry tree.children[0] = build(0)\nconsole.log(tree)",
			"let holder = { tree = build(2) }\nconsole.log(holder)",
		] {
			let (claims, resolved) = facts(&format!("{tree}\n{suffix}\n"));
			assert!(resolved.is_empty(), "{suffix}: {claims:#?}");
		}
	}

	#[test]
	fn structural_root_conversions_keep_ordinary_ownership() {
		let (claims, resolved) = facts(include_str!(
			"../../codegen-llvm/tests/programs/region-object-coercion-fallback.fa"
		));
		assert!(resolved.is_empty(), "{claims:#?}");
	}

	#[test]
	fn recursive_object_array_defaults_keep_ordinary_ownership() {
		let (claims, resolved) = facts(
			"type Tree = { value: Integer, children: Array<Tree> = [] }\nfunction build = (value: Integer): Tree => Tree { value }\nconsole.log(build(1))\n",
		);
		assert!(resolved.is_empty(), "{claims:#?}");
	}

	#[test]
	fn recursive_objects_with_empty_optional_leaves_keep_their_regions() {
		let (claims, resolved) = facts(
			"type Tree = { left: Tree?, right: Tree? }\nfunction build = (n: Integer): Tree => if n > 0 { Tree { left = build(n - 1), right = build(n - 1) } } else { Tree {} }\nlet n = try Integer.parse(arguments.get(0) ?? \"5\")\nconsole.log(build(n))\n",
		);
		assert!(
			resolved.iter().any(|fact| fact.starts_with("type\u{1f}")),
			"{claims:#?}"
		);
	}
}
