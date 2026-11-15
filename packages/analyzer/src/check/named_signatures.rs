//! Names come from the signature visible at the expression, never from an
//! arbitrary structurally equal function type in the type interner.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::check::Walker;
use crate::ids::{DeclKey, NodeId, Sym};
use crate::signatures::SignatureNames;
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};

impl Walker<'_> {
	pub(super) fn named_signature(&mut self, node: NodeId) -> Option<Arc<SignatureNames>> {
		let node = self.strip_groups(node);
		if let Some(known) = self.named_signatures.get(&node) {
			return known.clone();
		}
		self.named_signatures.insert(node, None);
		let names = self.named_signature_inner(node);
		self.named_signatures.insert(node, names.clone());
		names
	}

	pub(super) fn resolved_signature_names(
		&mut self,
		mut names: Arc<SignatureNames>,
	) -> Option<Arc<SignatureNames>> {
		let mut seen = FxHashSet::default();
		while let SignatureNames::Reference(key) = &*names {
			if !seen.insert(*key) {
				return None;
			}
			names = self.signature_reference(*key)?;
		}
		Some(names)
	}

	fn signature_reference(&mut self, key: DeclKey) -> Option<Arc<SignatureNames>> {
		if let Some(&symbol) = self.modules.keyed.get(&key) {
			return self.named_signature(self.a.symbols.symbol(symbol).decl_node);
		}
		let interface = self.s.interface(self.s.types.decl_key_data(key).module)?;
		let decl = interface.keys.get(&key)?.node;
		interface.signature_names.get(&decl).cloned()
	}

	fn signature_resolution(&mut self, resolution: Resolution) -> Option<Arc<SignatureNames>> {
		match resolution {
			Resolution::Local(symbol) => {
				if let Some(Resolution::External(key)) = self.external_resolution(symbol) {
					return Some(Arc::new(SignatureNames::Reference(key)));
				}
				let data = self.a.symbols.symbol(symbol).clone();
				if data.kind == SymbolKind::Type {
					return Some(Arc::new(SignatureNames::Reference(self.symbol_key(symbol))));
				}
				self.named_signature(data.decl_node)
			}
			Resolution::External(key) => Some(Arc::new(SignatureNames::Reference(key))),
			Resolution::Builtin(builtin) => {
				let signature = crate::builtins::get(builtin).signature()?;
				let parameters = signature
					.params
					.iter()
					.map(|param| Some(self.s.types.syms.intern(param.name)))
					.collect();
				Some(Arc::new(SignatureNames::Function {
					parameters,
					defaults: signature.params.iter().map(|param| param.default).collect(),
					bound: None,
					returns: None,
				}))
			}
			_ => None,
		}
	}

	fn named_signature_inner(&mut self, node: NodeId) -> Option<Arc<SignatureNames>> {
		if let Some(partial) = self.a.partial_applications.get(&node).cloned() {
			let returns = self
				.named_signature(partial.function)
				.and_then(|names| self.resolved_signature_names(names))
				.and_then(|names| match &*names {
					SignatureNames::Function { returns, .. } => returns.clone(),
					_ => None,
				});
			return Some(Arc::new(SignatureNames::Function {
				parameters: partial.names,
				defaults: partial
					.remaining
					.iter()
					.map(|&index| partial.defaults[index as usize])
					.collect(),
				bound: Some(partial.bound_names),
				returns,
			}));
		}
		if let Some(info) = self.a.function(node).cloned() {
			let parameters = info
				.params
				.iter()
				.map(|param| Some(self.a.symbols.symbol(param.symbol).name))
				.collect();
			let returns = match self.view(info.lambda) {
				View::Lambda { signature, .. } => {
					signature.returns.and_then(|ty| self.named_signature(ty))
				}
				_ => None,
			}
			.or_else(|| self.named_signature(info.body));
			return Some(Arc::new(SignatureNames::Function {
				parameters,
				defaults: info
					.params
					.iter()
					.map(|param| param.default.is_some())
					.collect(),
				bound: None,
				returns,
			}));
		}
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, node) {
			View::Name(_) => self.signature_resolution(self.a.resolution(node)),
			View::Lambda { signature, body } => {
				let parameters = signature
					.parameters
					.iter()
					.map(|parameter| match self.view(parameter) {
						View::Assignment {
							target,
							annotation: Some(_),
							..
						} => match self.view(target) {
							View::Name(name) => Some(self.s.types.syms.intern(tree.name(name))),
							_ => None,
						},
						_ => None,
					})
					.collect();
				let returns = self.named_signature(body);
				Some(Arc::new(SignatureNames::Function {
					parameters,
					defaults: vec![],
					bound: None,
					returns,
				}))
			}
			View::Declaration {
				annotation, value, ..
			}
			| View::Assignment {
				annotation, value, ..
			} => {
				let names = match annotation {
					Some(ty) => self.named_signature(ty),
					None => value.and_then(|value| self.named_signature(value)),
				}?;
				let names = self.resolved_signature_names(names)?;
				Some(match &*names {
					SignatureNames::Function {
						parameters,
						returns,
						..
					} => Arc::new(SignatureNames::Function {
						parameters: parameters.clone(),
						defaults: vec![false; parameters.len()],
						bound: None,
						returns: returns.clone(),
					}),
					_ => names,
				})
			}
			View::Function {
				annotation, value, ..
			}
			| View::ExportFunction {
				annotation, value, ..
			} => match annotation {
				Some(ty) => self.named_signature(ty),
				None => self.named_signature(value),
			},
			View::TypeDecl { value, .. }
			| View::ExportType { value, .. }
			| View::ExportValue { value, .. } => self.named_signature(value),
			View::Optional(value)
			| View::Do(value)
			| View::Try(value)
			| View::Return(Some(value)) => self.named_signature(value),
			View::Block { statements } => statements
				.last()
				.and_then(|value| self.named_signature(value)),
			View::Access { target, members } | View::OptionalAccess { target, members } => {
				if let Some(last) = members.last()
					&& matches!(
						self.a.resolution(last),
						Resolution::Local(_) | Resolution::External(_)
					) {
					return self.signature_resolution(self.a.resolution(last));
				}
				let mut names = self.named_signature(target)?;
				for member in members.iter() {
					names = self.resolved_signature_names(names)?;
					let SignatureNames::Object(fields) = &*names else {
						return None;
					};
					let name = self.s.types.syms.intern(tree.text(member.index()));
					names = fields.iter().find(|(field, _)| *field == name)?.1.clone();
				}
				Some(names)
			}
			View::Call { callee, .. } => {
				let names = self.named_signature(callee)?;
				if !self.a.calls.contains_key(&node) {
					return Some(names);
				}
				let names = self.resolved_signature_names(names)?;
				match &*names {
					SignatureNames::Function { returns, .. } => returns.clone(),
					_ => None,
				}
			}
			View::Object { items } | View::TypedObject { members: items, .. } => {
				let mut fields = Vec::new();
				if let View::TypedObject { target, .. } = self.view(node)
					&& let Some(names) = self.named_signature(target)
					&& let Some(names) = self.resolved_signature_names(names)
					&& let SignatureNames::Object(known) = &*names
				{
					fields = known.clone();
				}
				for field in items.iter() {
					let View::Assignment {
						target,
						annotation,
						value,
					} = self.view(field)
					else {
						continue;
					};
					let View::Name(name) = self.view(target) else {
						continue;
					};
					let name = self.s.types.syms.intern(tree.name(name));
					if fields.iter().any(|(known, _)| *known == name) {
						continue;
					}
					let names = match annotation {
						Some(ty) => self.named_signature(ty),
						None => value.and_then(|value| self.named_signature(value)),
					};
					if let Some(names) = names {
						fields.push((name, names));
					}
				}
				(!fields.is_empty()).then(|| Arc::new(SignatureNames::Object(fields)))
			}
			View::Tuple { items } => {
				let items: Vec<_> = items
					.iter()
					.map(|value| self.named_signature(value))
					.collect();
				items
					.iter()
					.any(Option::is_some)
					.then(|| Arc::new(SignatureNames::Tuple(items)))
			}
			_ => None,
		}
	}

	pub(super) fn interface_signature_names(&mut self) -> FxHashMap<NodeId, Arc<SignatureNames>> {
		let mut pending: Vec<_> = self
			.modules
			.keyed
			.values()
			.map(|&symbol| self.a.symbols.symbol(symbol).decl_node)
			.collect();
		let mut seen = FxHashSet::default();
		let mut result = FxHashMap::default();
		while let Some(node) = pending.pop() {
			if !seen.insert(node) {
				continue;
			}
			let Some(names) = self.named_signature(node) else {
				continue;
			};
			let mut references = Vec::new();
			names.references(&mut references);
			for key in references {
				if let Some(&symbol) = self.modules.keyed.get(&key) {
					pending.push(self.a.symbols.symbol(symbol).decl_node);
				}
			}
			result.insert(node, names);
		}
		result
	}

	pub(super) fn visible_parameter_defaults(&mut self, node: NodeId) -> Vec<bool> {
		self.named_signature(node)
			.and_then(|names| self.resolved_signature_names(names))
			.and_then(|names| match &*names {
				SignatureNames::Function { defaults, .. } => Some(defaults.clone()),
				_ => None,
			})
			.unwrap_or_default()
	}

	pub(super) fn visible_parameter_names(&mut self, node: NodeId) -> Option<Vec<Option<Sym>>> {
		let names = self.named_signature(node)?;
		let names = self.resolved_signature_names(names)?;
		match &*names {
			SignatureNames::Function { parameters, .. } => Some(parameters.clone()),
			_ => None,
		}
	}
}
