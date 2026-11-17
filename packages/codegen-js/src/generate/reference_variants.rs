//! Static representation variants. Ordinary generic functions have one body.
use fa_analyzer::ids::{NodeId, ParamId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;

use crate::generate::Generator;
use crate::js::Expr;

pub(super) type Variant = Vec<(ParamId, bool)>;

impl Generator<'_> {
	pub(super) fn reference_variants(&self, declaration: NodeId) -> Vec<Variant> {
		let params = self
			.analysis
			.reference_sensitive
			.get(&declaration)
			.cloned()
			.unwrap_or_default();
		let mut variants = vec![Vec::new()];
		for param in params {
			variants = variants
				.into_iter()
				.flat_map(|variant| {
					[false, true].map(|object| {
						let mut with = variant.clone();
						with.push((param, object));
						with
					})
				})
				.collect();
		}
		variants
	}

	pub(super) fn reference_suffix(&self, generics: &[ParamId], variant: &Variant) -> String {
		let mut positions = Vec::new();
		for &(param, object) in variant {
			if object
				&& let Some(index) = generics
					.iter()
					.position(|&candidate| self.same_reference_parameter(candidate, param))
			{
				positions.push(index.to_string());
			}
		}
		if positions.is_empty() {
			String::new()
		} else {
			format!("$r{}", positions.join("_"))
		}
	}

	pub(super) fn same_reference_parameter(&self, left: ParamId, right: ParamId) -> bool {
		left == right || {
			let (left, right) = (self.types.param(left), self.types.param(right));
			left.owner == right.owner && left.index == right.index
		}
	}

	pub(super) fn reference_parameter_object(&self, param: ParamId) -> bool {
		self.reference_bindings
			.iter()
			.find_map(|(&candidate, &object)| {
				self.same_reference_parameter(candidate, param)
					.then_some(object)
			})
			.unwrap_or(false)
	}

	pub(super) fn reference_selection(
		&self,
		site: NodeId,
		generics: &[ParamId],
		sensitive: &[ParamId],
	) -> String {
		let (bindings, _) = self.generic_call_bindings(site, generics);
		let variant = sensitive
			.iter()
			.filter_map(|&param| {
				generics
					.iter()
					.position(|&candidate| self.same_reference_parameter(candidate, param))
					.and_then(|index| bindings.get(index))
					.map(|&ty| (param, self.is_object_reference(ty)))
			})
			.collect();
		self.reference_suffix(generics, &variant)
	}

	pub(super) fn reference_call_variant_suffix(&self, call: NodeId) -> String {
		let View::Call { callee, .. } = self.view(call) else {
			return String::new();
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return String::new();
		};
		let generics = self.reference_call_generics(call, self.types.sig(sig));
		let sensitive = match self
			.analysis
			.resolution(self.named_reference_callee(callee))
		{
			Resolution::Local(symbol) => self
				.analysis
				.reference_sensitive
				.get(&self.analysis.symbols.symbol(symbol).decl_node)
				.cloned()
				.unwrap_or_default(),
			resolution @ Resolution::External(_) => self
				.session
				.external_node(resolution)
				.and_then(|(module, decl)| {
					self.session
						.interface(module)?
						.reference_sensitive
						.get(&decl)
				})
				.cloned()
				.unwrap_or_default(),
			_ => Vec::new(),
		};
		self.reference_selection(call, &generics, &sensitive)
	}

	fn named_reference_callee(&self, node: NodeId) -> NodeId {
		let node = self.strip_groups(node);
		match self.view(node) {
			View::Call { callee, .. }
				if self
					.analysis
					.has_flags(node, fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION) =>
			{
				self.named_reference_callee(callee)
			}
			View::Access { members, .. } | View::OptionalAccess { members, .. } => {
				members.last().unwrap_or(node)
			}
			_ => node,
		}
	}

	/// A generic function value names its representation body directly.
	pub(super) fn reference_function_value(&mut self, node: NodeId, value: Expr) -> Expr {
		if self.is_callee(node)
			|| matches!(self.analysis.resolution(node), Resolution::Local(symbol) if self.tasks.instances.contains_key(&self.analysis.symbols.symbol(symbol).decl_node))
		{
			return value;
		}
		self.select_reference_body(node, node, value, false)
	}

	pub(super) fn select_reference_call(
		&mut self,
		call: NodeId,
		callee: NodeId,
		value: Expr,
	) -> Expr {
		self.select_reference_body(call, callee, value, true)
	}

	pub(super) fn select_reference_body_value(
		&mut self,
		site: NodeId,
		callee: NodeId,
		value: Expr,
	) -> Expr {
		self.select_reference_body(site, callee, value, false)
	}

	pub(super) fn select_reference_body(
		&mut self,
		site: NodeId,
		callee: NodeId,
		value: Expr,
		direct: bool,
	) -> Expr {
		let named = self.named_reference_callee(callee);
		let layout = if direct {
			self.reference_call_layout(site)
		} else {
			fa_analyzer::analysis::ReferenceLayout::General
		};
		let layout_suffix = Self::reference_layout_suffix(layout);
		match self.analysis.resolution(named) {
			Resolution::Local(symbol)
				if self.analysis.symbols.symbol(symbol).kind == SymbolKind::Function =>
			{
				let declaration = self.analysis.symbols.symbol(symbol).decl_node;
				let generics = self.function_generics(declaration);
				let sensitive = self
					.analysis
					.reference_sensitive
					.get(&declaration)
					.cloned()
					.unwrap_or_default();
				let suffix = self.reference_selection(site, &generics, &sensitive);
				if !suffix.is_empty() || !layout_suffix.is_empty() {
					Expr::name(format!("{}{suffix}{layout_suffix}", value.text), true)
				} else {
					value
				}
			}
			resolution @ Resolution::External(_) => {
				let Some((module, declaration)) = self.session.external_node(resolution) else {
					return value;
				};
				let Some(interface) = self.session.interface(module) else {
					return value;
				};
				let sensitive = interface
					.reference_sensitive
					.get(&declaration)
					.cloned()
					.unwrap_or_default();
				let mut generics = match resolution {
					Resolution::External(key) => self.external_owner_parameters(key),
					_ => Vec::new(),
				};
				if let Some(function) = interface.generic_functions.get(&declaration) {
					generics.extend(self.types.list_items(function.generics).iter().filter_map(
						|&ty| match self.types.kind(ty) {
							TypeKind::Param(param) => Some(param),
							_ => None,
						},
					));
				}
				let suffix = self.reference_selection(site, &generics, &sensitive);
				if suffix.is_empty() && layout_suffix.is_empty() {
					return value;
				}
				let Some(external) = self.external(resolution) else {
					return value;
				};
				Expr::name(
					self.import(
						&external.path,
						&format!("{}{suffix}{layout_suffix}", external.export),
					),
					true,
				)
			}
			Resolution::Local(_)
				if direct
					&& self
						.analysis
						.instantiation(self.generic_application_site(site))
						.is_some() =>
			{
				// A generic callable parameter can contain any representation family.
				// Each edge is a property of an existing function, without a wrapper,
				// a representation argument, a table object or dynamic key allocation.
				let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
					return value;
				};
				let generics: Vec<_> = self
					.types
					.list_items(self.types.sig(sig).generics)
					.iter()
					.filter_map(|&ty| match self.types.kind(ty) {
						TypeKind::Param(param) => Some(param),
						_ => None,
					})
					.collect();
				if generics.is_empty() {
					return value;
				}
				let (bindings, _) = self.generic_call_bindings(site, &generics);
				let mut selected = value.primary_text();
				for (index, ty) in bindings.iter().enumerate() {
					selected.push_str(&format!(
						"?.$ref{index}{}",
						if self.is_object_reference(*ty) {
							"o"
						} else {
							"p"
						}
					));
				}
				Expr::primary(format!("({selected} ?? {})", value.primary_text()))
			}
			_ => value,
		}
	}

	/// Edges use the generic callable's own positions; fixed owner binders
	/// have already selected the starting body when a method becomes a value.
	pub(super) fn emit_reference_family(&mut self, declaration: NodeId, base: &str) {
		let Some(info) = self.analysis.function(declaration) else {
			return;
		};
		if !self.analysis.reference_sensitive.contains_key(&declaration) || info.generics.is_empty()
		{
			return;
		}
		let own = info.generics.clone();
		let generics = self.function_generics(declaration);
		for variant in self.reference_variants(declaration) {
			let name = format!("{base}{}", self.reference_suffix(&generics, &variant));
			for (index, &param) in own.iter().enumerate() {
				for object in [false, true] {
					let mut target = variant.clone();
					if let Some(binding) = target
						.iter_mut()
						.find(|entry| self.same_reference_parameter(entry.0, param))
					{
						binding.1 = object;
					}
					let target = format!("{base}{}", self.reference_suffix(&generics, &target));
					self.out.line(format!(
						"{name}.$ref{index}{} = {target};",
						if object { "o" } else { "p" }
					));
				}
			}
		}
	}
}
