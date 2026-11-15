//! Key sets as compile-time values (types brief §3.9, `types/fields.md`,
//! "Key sets as compile-time parameters"): `Analysis::key_set_params`,
//! `key_loops`, `key_reads`, `key_loop_bodies`, `key_set_instances`.
//!
//! - `keys: Keys<Value>` / `DeepKeys<Value>`, a compile-time value parameter
//!   whose subject is a type parameter of the same declaration: an
//!   application binds it to `Tuple(Fields type, bits)`, the key set
//!   checked against the keys of the subject's argument (`TF010`). In the
//!   body, it is the iterable of an `@for` or the keys of `pick`/`omit`
//!   (a derived generic parameter, `type_ops.rs`).
//! - `@for key in keys` over such a parameter or a `static` key set is
//!   unrolled: its body is checked once per key, `key` the dotted path of
//!   the key (a `String`) and `value[key]` the field at that path, of the
//!   field's type. An error there is `TF011`, naming the key (and the
//!   application). Over a `static` key set, the keys are known at once;
//!   over a parameter, the body is walked once quietly in its function
//!   (for what the function's summary needs), then checked for each key of
//!   each application of the file, once the module is walked, in the
//!   context saved at the loop. Each check leaves a snapshot of the file's
//!   analysis that the backends use to lower the body for that key.

use std::sync::Arc;

use ethnum::I256;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::KeyLoopInstance;
use crate::check::{BodyCx, Walker};
use crate::diagnostics::codes::types::{TF010, TF011};
use crate::diagnostics::{Diagnostic, Location, Severity, SourceSpan};
use crate::ids::{ModuleId, NodeId, ParamId, Sym, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, Name, View};
use crate::types::{FieldEntry, FieldFlags, TypeKind};

/// The key sets of a walk.
#[derive(Default)]
pub(crate) struct KeySetState {
	/// Local and imported key-set signatures, independent of local symbols.
	pub(super) metadata: FxHashMap<ParamId, crate::modules::GenericKeySet>,
	/// The compile-time value parameters holding a key set, by symbol.
	params: FxHashMap<SymbolId, KeySetParam>,
	/// The variables of the key loops (by pattern node): what they go through.
	loop_variables: FxHashMap<NodeId, KeyLoop>,
	/// The key each loop variable stands for while its body is checked, and
	/// for a key-set parameter, the type argument of its subject.
	current: FxHashMap<NodeId, (String, Option<TypeId>)>,
	/// The iterable of the `@for` being checked: a key-set parameter is
	/// read there only.
	iterable: Option<NodeId>,
	/// The type whose keys a key set being checked names (`TF010` messages).
	pub(super) owner: Option<TypeId>,
	/// The order the fields of anonymous object types were written in
	/// (`Keys<T>` lists them in that order).
	field_orders: FxHashMap<TypeId, Vec<Sym>>,
	/// The applications of each key-set parameter in the file.
	instances: FxHashMap<ParamId, Vec<KeySetInstance>>,
	/// The loops over a key-set parameter, checked per key once the module
	/// is walked.
	deferred: Vec<DeferredLoop>,
	/// Local key-set parameters iterated by a loop, even without a local call.
	pub(super) loop_params: FxHashSet<ParamId>,
	/// Checking the loops per key: the applications met are not recorded.
	unrolling: bool,
}

/// A compile-time value parameter holding a key set (`keys: DeepKeys<Value>`).
#[derive(Copy, Clone, Debug)]
pub(crate) struct KeySetParam {
	/// The parameter.
	pub param: ParamId,
	/// The type parameter whose keys it holds.
	pub subject: ParamId,
	/// `DeepKeys`: the keys of nested objects too.
	pub deep: bool,
}

/// An application of a key-set parameter.
#[derive(Clone, Debug)]
struct KeySetInstance {
	/// The type argument of the subject.
	subject: TypeId,
	/// The binding: `Tuple(Fields type, bits)`.
	key_set: TypeId,
	/// The keys bound, as dotted paths.
	keys: Vec<String>,
	/// The application node (for the notes of its errors).
	caller: ModuleId,
	span: SourceSpan,
}

/// What the variable of a key loop goes through.
#[derive(Clone, Debug)]
enum KeyLoop {
	/// The keys of a key-set parameter: of a value of its subject.
	Param(ParamId),
	/// The keys of a `static` key set.
	Keys,
}

/// A loop over a key-set parameter, checked per key later.
struct DeferredLoop {
	node: NodeId,
	pattern: Option<NodeId>,
	index: Option<Name>,
	iterable: NodeId,
	body: NodeId,
	/// The key-set parameter.
	param: ParamId,
	/// The context at the loop.
	context: BodyCx,
	rigid_params: Vec<ParamId>,
}

impl Walker<'_> {
	/// `Keys<Value>`, `DeepKeys<Value>` as the type of a compile-time value
	/// parameter, `Value` a type parameter: the subject and whether it is
	/// deep. The nodes are typed with the placeholder of a key set.
	fn key_set_annotation(&mut self, annotation: NodeId) -> Option<(ParamId, bool)> {
		let tree = Arc::clone(&self.tree);
		let View::Call { callee, arguments } = syntax::view(&tree, annotation) else {
			return None;
		};
		let View::Name(function) = syntax::view(&tree, callee) else {
			return None;
		};
		let deep = match tree.name(function) {
			"Keys" => false,
			"DeepKeys" => true,
			_ => return None,
		};
		if self.is_declared(function) || arguments.len() != 1 {
			return None;
		}
		let subject = arguments.first()?;
		let View::Name(name) = syntax::view(&tree, subject) else {
			return None;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		if self.a.symbols.symbol(symbol).kind != SymbolKind::TypeParameter {
			return None;
		}
		let subject_ty = self.a.symbols.symbol(symbol).ty;
		let TypeKind::Param(param) = self.s.types.kind(subject_ty) else {
			return None;
		};
		self.set_resolution(subject, Resolution::Local(symbol));
		let placeholder = self.s.types.fields_type(&[]);
		for (node, ty) in [
			(subject, subject_ty),
			(callee, placeholder),
			(annotation, placeholder),
		] {
			self.set_flag(node, crate::analysis::NodeFlags::TYPE_EXPR);
			self.memo(node, |_| ty);
		}
		Some((param, deep))
	}

	/// Declares a compile-time value parameter whose type is a key set of a
	/// type parameter. `false` when its annotation is not one.
	pub(super) fn declare_key_set_param(
		&mut self,
		param: ParamId,
		generic: NodeId,
		target: NodeId,
		name: Name,
		annotation: NodeId,
	) -> bool {
		let Some((subject, deep)) = self.key_set_annotation(annotation) else {
			return false;
		};
		// checked at each application (`key_set_argument`)
		self.s
			.types
			.set_param_bounds(param, TypeId::UNKNOWN, TypeId::UNSET);
		let ty = self.s.types.fields_type(&[]);
		let symbol = self.declare(generic, name, SymbolKind::Static);
		let data = self.a.symbols.symbol_mut(symbol);
		data.ty = ty;
		data.state = DeclState::Done;
		self.set_type(target, ty);
		self.key_sets.params.insert(
			symbol,
			KeySetParam {
				param,
				subject,
				deep,
			},
		);
		self.a.key_set_params.insert(param, subject);
		self.key_sets
			.metadata
			.insert(param, crate::modules::GenericKeySet { subject, deep });
		true
	}

	/// The key-set parameter a name node reads (`keys` in `Pick<Entity,
	/// keys>`, `pick(entity, keys)`), resolved; `None` for another node.
	pub(super) fn key_set_param_named(&mut self, n: NodeId) -> Option<KeySetParam> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, self.strip_groups(n)) else {
			return None;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		let entry = *self.key_sets.params.get(&symbol)?;
		self.set_resolution(n, Resolution::Local(symbol));
		let ty = self.a.symbols.symbol(symbol).ty;
		self.memo(n, |_| ty);
		Some(entry)
	}

	/// The key-set parameter declared by `param`, if it is one.
	fn key_set_param(&self, param: ParamId) -> Option<KeySetParam> {
		self.key_sets.metadata.get(&param).map(|info| KeySetParam {
			param,
			subject: info.subject,
			deep: info.deep,
		})
	}

	/// The argument of a key-set parameter at an application: its key set
	/// checked against the keys of its subject's argument (`TF010`); the
	/// literal it is bound to, `Tuple(Fields type, bits)`. `None` when
	/// `param` is not a key-set parameter.
	pub(super) fn key_set_argument(
		&mut self,
		param: ParamId,
		argument: NodeId,
		params: &[ParamId],
		given: &[TypeId],
	) -> Option<TypeId> {
		let entry = self.key_set_param(param)?;
		self.set_flag(argument, crate::analysis::NodeFlags::TYPE_EXPR);
		let subject = params
			.iter()
			.position(|&other| other == entry.subject)
			.and_then(|index| given.get(index).copied());
		let Some(subject) = subject.filter(|ty| !ty.is_unset()) else {
			let diagnostic = self.diagnostic(
				&TF010,
				argument,
				"the keys of a key set are known once the type before it is given: give the type first",
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		if self.s.types.is_recovering(subject) {
			return Some(TypeId::UNKNOWN);
		}
		let fields = self.keys_type(argument, subject, entry.deep);
		if !self.is_fields(fields) {
			return Some(TypeId::UNKNOWN);
		}
		let tree = Arc::clone(&self.tree);
		let View::Object { items } = syntax::view(&tree, self.strip_groups(argument)) else {
			let diagnostic = self.diagnostic(
				&TF010,
				argument,
				"a key set is written in braces, ex: `{ id, name }`",
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		let saved = self.key_sets.owner.replace(subject);
		let bits = self.fields_items_bits(fields, items, &mut Vec::new(), &TF010);
		self.key_sets.owner = saved;
		let Some(bits) = bits else {
			return Some(TypeId::UNKNOWN);
		};
		self.memo(argument, |_| fields);
		let literal = self.s.types.int_literal(I256::from(bits));
		let key_set = self.s.types.tuple(&[fields, literal]);
		// an application: its keys are checked in the loops over it
		if !self.key_sets.unrolling {
			let keys = self.key_paths(fields, I256::from(bits));
			let instances = self.key_sets.instances.entry(param).or_default();
			if !instances
				.iter()
				.any(|other| other.subject == subject && other.key_set == key_set)
			{
				instances.push(KeySetInstance {
					subject,
					key_set,
					keys,
					caller: self.s.module_id(self.file),
					span: SourceSpan::new(
						self.tree.spans[argument.index()].start as u32,
						self.tree.spans[argument.index()].end as u32,
					),
				});
			}
		}
		Some(key_set)
	}

	/// What foreign declaring units need after this caller's tree is dropped.
	pub(super) fn foreign_key_set_applications(&self) -> Vec<crate::modules::KeySetApplication> {
		let own = self.s.module(self.file);
		self.key_sets
			.instances
			.iter()
			.filter(|(param, _)| {
				let module = self
					.s
					.types
					.decl_key_data(self.s.types.param(**param).owner)
					.module;
				self.s.types.module_path(module) != own
					&& self
						.s
						.interface(module)
						.is_some_and(|interface| interface.key_loop_params.contains(param))
			})
			.flat_map(|(&param, instances)| {
				instances
					.iter()
					.map(move |instance| crate::modules::KeySetApplication {
						param,
						subject: instance.subject,
						key_set: instance.key_set,
						caller: instance.caller,
						span: instance.span,
					})
			})
			.collect()
	}

	/// A read of a name: a key-set parameter is only the iterable of an
	/// `@for` (`AN001` elsewhere).
	pub(super) fn check_key_set_read(&mut self, n: NodeId, symbol: SymbolId) {
		if !self.key_sets.params.contains_key(&symbol) || self.key_sets.iterable == Some(n) {
			return;
		}
		self.unsupported_message(n, "a key set parameter outside of the iterable of `@for`");
	}

	/// The fields of an object type in declaration order: as declared, as
	/// written for an anonymous type, else in the order of its shape.
	pub(super) fn ordered_fields(&mut self, object: TypeId) -> Vec<FieldEntry> {
		let mut fields = self.s.types.fields_of(object).to_vec();
		let order: Option<Vec<Sym>> = match self.declared_fields(object) {
			Some((_, declared)) if !declared.is_empty() => {
				Some(declared.iter().map(|field| field.name).collect())
			}
			_ => self.key_sets.field_orders.get(&object).cloned(),
		};
		if let Some(order) = order {
			fields.sort_by_key(|field| {
				order
					.iter()
					.position(|&name| name == field.name)
					.unwrap_or(usize::MAX)
			});
		}
		fields
	}

	/// Records the order the fields of an anonymous object type are written in.
	pub(super) fn record_field_order(&mut self, ty: TypeId, names: Vec<Sym>) {
		self.key_sets.field_orders.entry(ty).or_insert(names);
	}

	/// The keys of a `Fields` value: the dotted paths of its flags.
	fn key_paths(&self, ty: TypeId, bits: I256) -> Vec<String> {
		self.s
			.types
			.fields_leaves(ty)
			.into_iter()
			.enumerate()
			.filter(|(index, _)| (bits >> *index as u32) & I256::ONE == I256::ONE)
			.map(|(_, path)| path)
			.collect()
	}

	/// The keys of a `Fields` value known at compile time, as the items of
	/// an `@for`.
	pub(super) fn key_items(&self, ty: TypeId, bits: I256) -> Vec<fa_interpreter::Value> {
		self.key_paths(ty, bits)
			.iter()
			.map(|path| fa_interpreter::Value::string(path))
			.collect()
	}

	/// `@for key in keys` over a key set (a key-set parameter, a `static`
	/// key set, a literal of a `Fields` type). `None` when the iterable is
	/// not a key set.
	pub(super) fn check_key_loop(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		iterable: NodeId,
		body: NodeId,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let parameter = match syntax::view(&tree, iterable) {
			View::Name(name) => {
				let sym = self.s.types.syms.intern(tree.name(name));
				self.a
					.symbols
					.lookup(self.body.scope, sym)
					.and_then(|symbol| self.key_sets.params.get(&symbol).copied())
			}
			_ => None,
		};
		self.a.key_loops.insert(n);
		match parameter {
			Some(entry) => {
				self.key_sets.loop_params.insert(entry.param);
				let saved = self.key_sets.iterable.replace(iterable);
				self.check(iterable, None);
				self.key_sets.iterable = saved;
				if let Some(pattern) = pattern {
					self.key_sets
						.loop_variables
						.insert(pattern, KeyLoop::Param(entry.subject));
				}
				// checked per key once the applications are known
				if !self.key_sets.unrolling {
					self.key_sets.deferred.push(DeferredLoop {
						node: n,
						pattern,
						index,
						iterable,
						body,
						param: entry.param,
						context: self.body.clone(),
						rigid_params: self.rigid_params.clone(),
					});
				}
				// walked quietly for the function's summary
				self.quiet += 1;
				let ty = self.check_for_items(n, pattern, index, (iterable, TypeId::STRING), body);
				self.quiet -= 1;
				Some(ty)
			}
			None => {
				let ty = self.check(iterable, None);
				if !self.is_fields(ty) {
					self.a.key_loops.remove(&n);
					return None;
				}
				self.evaluate_comptime_for(n, iterable);
				let keys: Vec<String> = self
					.a
					.unrolled
					.get(&n)
					.map(|items| {
						items
							.iter()
							.filter_map(|item| item.as_str().map(str::to_string))
							.collect()
					})
					.unwrap_or_default();
				if let Some(pattern) = pattern {
					self.key_sets.loop_variables.insert(pattern, KeyLoop::Keys);
				}
				let analyses =
					self.check_for_each_key(n, pattern, index, (iterable, body), &keys, None);
				self.a.key_loop_bodies.insert(
					n,
					vec![KeyLoopInstance {
						param: None,
						subject: TypeId::UNSET,
						key_set: TypeId::UNSET,
						keys,
						analyses,
					}],
				);
				Some(TypeId::VOID)
			}
		}
	}

	/// Checks the body of a key loop once per key (`subject`: the type
	/// argument of a key-set parameter's subject and the application), and
	/// returns the analysis of the file for each. Without key, the body is
	/// walked quietly once.
	fn check_for_each_key(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		(iterable, body): (NodeId, NodeId),
		keys: &[String],
		subject: Option<(TypeId, ModuleId, SourceSpan)>,
	) -> Vec<Arc<crate::analysis::Analysis>> {
		if keys.is_empty() {
			self.quiet += 1;
			self.check_for_items(n, pattern, index, (iterable, TypeId::STRING), body);
			self.quiet -= 1;
			return Vec::new();
		}
		let mut nodes = Vec::new();
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			nodes.push(node);
			stack.extend(syntax::children(&self.tree, node));
		}
		let mut seen: FxHashSet<(String, u32, u32, String)> = FxHashSet::default();
		let mut analyses = Vec::with_capacity(keys.len());
		let unrolling = std::mem::replace(&mut self.key_sets.unrolling, true);
		for key in keys {
			self.forget_body(&nodes);
			if let Some(pattern) = pattern {
				self.key_sets
					.current
					.insert(pattern, (key.clone(), subject.map(|(ty, _, _)| ty)));
			}
			let output = self.output.len();
			let buffered = self
				.body
				.flow_buffers
				.last()
				.map_or(0, |buffer| buffer.diagnostics.len());
			self.check_for_items(n, pattern, index, (iterable, TypeId::STRING), body);
			// the errors of this key name it (and the application)
			let note = subject.and_then(|(_, module, span)| {
				let path = self.s.types.module_path(module).to_string();
				self.s
					.loaded_module(&path)
					.map(|file| Location { file, span })
			});
			let foreign_call = subject.and_then(|(_, module, span)| {
				let path = self.s.types.module_path(module);
				if path == self.s.module(self.file) {
					return None;
				}
				let text = self.s.resolver()?.source(path)?;
				let (line, column) =
					crate::diagnostics::LineIndex::new(&text).line_column(span.start);
				Some(format!("{path}.fa:{line}:{column}"))
			});
			let mut produced: Vec<Diagnostic> = self.output.split_off(output);
			if let Some(buffer) = self.body.flow_buffers.last_mut()
				&& buffer.diagnostics.len() > buffered
			{
				produced.extend(buffer.diagnostics.split_off(buffered));
			}
			for mut diagnostic in produced {
				let identity = (
					diagnostic.code.id.to_string(),
					diagnostic.location.span.start,
					diagnostic.location.span.end,
					diagnostic.message.clone(),
				);
				if !seen.insert(identity) {
					continue;
				}
				if diagnostic.severity == Severity::Error && !std::ptr::eq(diagnostic.code, &TF011)
				{
					diagnostic.message = format!(
						"for the key `{key}` ({}): {}",
						diagnostic.code.id, diagnostic.message
					);
					diagnostic.code = &TF011;
				}
				if let Some(call) = &foreign_call {
					diagnostic
						.message
						.push_str(&format!(" (key set applied at {call})"));
				}
				if let Some(location) = note {
					diagnostic
						.related
						.push((location, format!("the key `{key}` comes from this key set")));
				}
				self.output.push(diagnostic);
			}
			if let Some(pattern) = pattern {
				self.key_sets.current.remove(&pattern);
			}
			// the analysis with the body checked for this key
			let nested = std::mem::take(&mut self.a.key_loop_bodies);
			analyses.push(Arc::new(self.a.clone()));
			self.a.key_loop_bodies = nested;
		}
		self.key_sets.unrolling = unrolling;
		analyses
	}

	/// Forgets what the check of a body recorded on its nodes, before it is
	/// checked for another key.
	fn forget_body(&mut self, nodes: &[NodeId]) {
		for &node in nodes {
			let index = node.index();
			if let Some(slot) = self.a.types.get_mut(index) {
				*slot = TypeId::UNSET;
			}
			if let Some(slot) = self.a.flags.get_mut(index) {
				*slot = crate::analysis::NodeFlags::empty();
			}
			self.a.coercions.remove(&node);
			self.a.arith.remove(&node);
			self.a.intervals.remove(&node);
			self.a.instantiations.remove(&node);
			self.a.instantiated_methods.remove(&node);
			self.a.instantiation_origins.remove(&node);
			self.a.written_types.remove(&node);
			self.a.method_instantiations.remove(&node);
			self.a.callback_types.remove(&node);
			self.a.calls.remove(&node);
			self.a.objects.remove(&node);
			self.a.patterns.remove(&node);
			self.a.key_reads.remove(&node);
			self.a.moved_reads.remove(&node);
			self.a.task_arguments.remove(&node);
			self.a.thread_moves.remove(&node);
			self.a.thread_views.remove(&node);
			self.a.thread_origins.remove(&node);
			self.a.owned_projections.remove(&node);
			self.a.constants.remove(&node);
			self.a.comptime_branches.remove(&node);
			self.a.unrolled.remove(&node);
			self.a.unrolled_items.remove(&node);
			self.a.key_loop_bodies.remove(&node);
			self.comptime_evaluated.remove(&node);
		}
	}

	/// The loops over key-set parameters, checked for each key of each
	/// application of the file (once the module is walked), in the context
	/// saved at each loop.
	pub(super) fn check_deferred_key_loops(&mut self) {
		// The same declaration gets fresh local parameter ids when a unit
		// is analyzed again. Match published requests by owner and position.
		let mut applications = if self.key_sets.params.is_empty() {
			Vec::new()
		} else {
			self.s.key_set_applications()
		};
		// Equal instances from several callers share one diagnostic. Its
		// application note must not depend on interface publication order.
		applications.sort_by(|left, right| {
			self.s
				.types
				.module_path(left.caller)
				.cmp(self.s.types.module_path(right.caller))
				.then(left.span.cmp(&right.span))
		});
		let params: Vec<_> = self
			.key_sets
			.params
			.values()
			.map(|entry| entry.param)
			.collect();
		for param in params {
			let info = self.s.types.param(param);
			for application in &applications {
				let requested = self.s.types.param(application.param);
				if info.owner != requested.owner || info.index != requested.index {
					continue;
				}
				self.a.foreign_key_sets = true;
				let instances = self.key_sets.instances.entry(param).or_default();
				if !instances.iter().any(|instance| {
					instance.subject == application.subject
						&& instance.key_set == application.key_set
				}) {
					let TypeKind::Tuple(list) = self.s.types.kind(application.key_set) else {
						continue;
					};
					let [fields, bits] = *self.s.types.list_items(list) else {
						continue;
					};
					let Some(bits) = self.s.types.as_int(bits).and_then(|int| int.literal()) else {
						continue;
					};
					let keys = self.key_paths(fields, bits);
					self.key_sets
						.instances
						.entry(param)
						.or_default()
						.push(KeySetInstance {
							subject: application.subject,
							key_set: application.key_set,
							keys,
							caller: application.caller,
							span: application.span,
						});
				}
			}
		}
		// Publication order and worker scheduling must not order generated
		// specializations. Concrete type hashes ignore local/global ids.
		for instances in self.key_sets.instances.values_mut() {
			instances.sort_by_cached_key(|instance| {
				(
					self.s.types.stable_hash(instance.subject),
					self.s.types.stable_hash(instance.key_set),
				)
			});
		}
		// Every application, also of a parameter no loop goes through.
		for (&param, instances) in &self.key_sets.instances {
			self.a.key_set_instances.insert(
				param,
				instances
					.iter()
					.map(|instance| (instance.subject, instance.key_set))
					.collect(),
			);
		}
		let deferred = std::mem::take(&mut self.key_sets.deferred);
		for entry in deferred {
			let subject = self.a.key_set_params[&entry.param];
			let instances = self
				.key_sets
				.instances
				.get(&entry.param)
				.cloned()
				.unwrap_or_default();
			self.a.key_set_instances.insert(
				entry.param,
				instances
					.iter()
					.map(|instance| (instance.subject, instance.key_set))
					.collect(),
			);
			let saved_body = std::mem::replace(&mut self.body, entry.context);
			let saved_rigid = std::mem::replace(&mut self.rigid_params, entry.rigid_params);
			let mut bodies = Vec::with_capacity(instances.len());
			for instance in instances {
				let analyses = self.check_for_each_key(
					entry.node,
					entry.pattern,
					entry.index,
					(entry.iterable, entry.body),
					&instance.keys,
					Some((instance.subject, instance.caller, instance.span)),
				);
				bodies.push(KeyLoopInstance {
					param: Some(subject),
					subject: instance.subject,
					key_set: instance.key_set,
					keys: instance.keys,
					analyses,
				});
			}
			self.body = saved_body;
			self.rigid_params = saved_rigid;
			self.a.key_loop_bodies.insert(entry.node, bodies);
		}
	}

	/// `value[key]`, `key` the variable of a key loop: the field of `value`
	/// at the key being checked, of the field's type. `None` for another
	/// index.
	pub(super) fn check_key_read(
		&mut self,
		n: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Option<TypeId> {
		let name = self.strip_groups(index);
		let View::Name(variable) = self.view(name) else {
			return None;
		};
		let sym = self.s.types.syms.intern(self.tree.name(variable));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		let decl = self.a.symbols.symbol(symbol).decl_node;
		let keys = self.key_sets.loop_variables.get(&decl).cloned()?;
		let current = self.key_sets.current.get(&decl).cloned();
		self.check(index, None);
		let target_ty = self.check(target, None);
		self.a.key_reads.insert(n);
		Some(self.memo(n, |w| {
			// the quiet walk of a loop over a key-set parameter
			let Some((key, subject)) = current else {
				return TypeId::UNKNOWN;
			};
			if w.s.types.is_recovering(target_ty) {
				return TypeId::UNKNOWN;
			}
			let object = match (&keys, subject) {
				(KeyLoop::Param(param), Some(subject)) => {
					if !matches!(w.s.types.kind(target_ty), TypeKind::Param(own) if own == *param) {
						let name =
							w.s.types
								.syms
								.resolve(w.s.types.param(*param).name)
								.to_string();
						let message = format!(
							"the keys go through the fields of a `{name}`: `{}` is a `{}`",
							w.tree.text(target.index()),
							w.display(target_ty)
						);
						let diagnostic = w.diagnostic(&TF011, n, message);
						w.report(diagnostic);
						return TypeId::UNKNOWN;
					}
					subject
				}
				_ => target_ty,
			};
			match w.field_at_path(object, &key) {
				Some(ty) => ty,
				None => {
					let message = format!(
						"for the key `{key}`: a `{}` has no field `{key}`",
						w.display(object)
					);
					let diagnostic = w.diagnostic(&TF011, n, message);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}
			}
		}))
	}

	/// The type of the field of an object type at a dotted path.
	pub(super) fn field_at_path(&mut self, ty: TypeId, path: &str) -> Option<TypeId> {
		let mut current = ty;
		for name in path.split('.') {
			let object = self.s.types.unfold(current);
			let sym = self.s.types.syms.intern(name);
			current = self
				.s
				.types
				.fields_of(object)
				.iter()
				.find(|field| field.name == sym && !field.flags.contains(FieldFlags::PRIVATE))?
				.ty;
		}
		Some(current)
	}
}
