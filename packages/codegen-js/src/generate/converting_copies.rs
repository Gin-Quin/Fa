//! A converting copy uses the actual input type and copies only retained
//! destination data. Generated plans schedule children on a work stack, so
//! recursive values never recurse through the JavaScript call stack.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::types::{CollKind, TypeKind};

use super::Generator;
use crate::js::{self, Expr};

impl Generator<'_> {
	pub(super) fn copy_as(
		&mut self,
		node: NodeId,
		value: Expr,
		source: TypeId,
		target: TypeId,
	) -> Expr {
		if source == target {
			return self.copy_of(value, source);
		}
		let plan = self.copy_plan(node, source, target);
		let helper = self.helper("faCopyAs");
		js::call(helper, &[value.text, plan])
	}

	fn copy_plan(&mut self, node: NodeId, source: TypeId, target: TypeId) -> String {
		if let Some(name) = self.copy_plans.get(&(source, target)) {
			return name.clone();
		}
		let name = self.scopes.declare_global("copy$convert");
		self.copy_plans.insert((source, target), name.clone());
		let body = self.copy_plan_body(node, source, target);
		self.constants.push((
			name.clone(),
			format!("(value, target, key, work) => {{ {body} }}"),
		));
		name
	}

	fn copy_plan_body(&mut self, node: NodeId, source: TypeId, target: TypeId) -> String {
		if source == target {
			let copied = self.copy_of(Expr::primary("value"), source);
			return format!("target[key] = {};", copied.text);
		}
		let from = self.types.rec_target(source).unwrap_or(source);
		let to = self.types.rec_target(target).unwrap_or(target);
		if (from, to) != (source, target) {
			let plan = self.copy_plan(node, from, to);
			return format!("work.push([{plan}, value, target, key]);");
		}
		if matches!(
			self.types.kind(source),
			TypeKind::Union(_) | TypeKind::NamedUnion(..)
		) {
			let alternatives = fa_analyzer::lower::copies::copy_alternatives(
				self.types,
				self.session.module_id(self.file),
				source,
			);
			let mut branches = Vec::new();
			for alternative in alternatives {
				let alternative = alternative.ty;
				let chosen = self.copy_target(alternative, target);
				let test = self.copy_case_test(source, alternative);
				let plan = self.copy_plan(node, alternative, chosen);
				branches.push(format!(
					"if ({test}) {{ work.push([{plan}, value, target, key]); return; }}"
				));
			}
			return branches.join(" ");
		}
		if matches!(self.types.kind(target), TypeKind::Union(_)) {
			let chosen = self.copy_target(source, target);
			let plan = self.copy_plan(node, source, chosen);
			return format!("work.push([{plan}, value, target, key]);");
		}
		match (self.types.kind(source), self.types.kind(target)) {
			(TypeKind::Object(_), TypeKind::Object(_)) => {
				self.object_copy_plan(node, source, target)
			}
			(TypeKind::Tuple(a), TypeKind::Tuple(b)) => {
				let from = self.types.list_items(a).to_vec();
				let to = self.types.list_items(b).to_vec();
				let mut jobs = Vec::new();
				for (index, (source, target)) in from.into_iter().zip(to).enumerate().rev() {
					let plan = self.copy_plan(node, source, target);
					jobs.push(format!(
						"work.push([{plan}, value[{index}], result, {index}]);"
					));
				}
				format!(
					"const result = new Array(value.length); target[key] = result; {}",
					jobs.join(" ")
				)
			}
			(
				TypeKind::Collection(_, from_item, from_key),
				TypeKind::Collection(kind, item, key),
			) => {
				let plan = self.copy_plan(node, from_item, item);
				if matches!(
					kind,
					CollKind::Array | CollKind::GrowingArray | CollKind::OrderedColumnar
				) {
					return format!(
						"const result = new Array(value.length); target[key] = result; for (let index = value.length - 1; index >= 0; index--) work.push([{plan}, value[index], result, index]);"
					);
				}
				if matches!(kind, CollKind::Map | CollKind::GrowingMap) {
					let key_plan = self.copy_plan(node, from_key, key);
					let class = self.map_class(key);
					return format!(
						"const result = new {class}(); target[key] = result; const entries = Array.from(value); for (let index = entries.length - 1; index >= 0; index--) {{ const entry = {{}}; work.push([() => result.set(entry.key, entry.value)]); work.push([{plan}, entries[index][1], entry, 'value']); work.push([{key_plan}, entries[index][0], entry, 'key']); }}"
					);
				}
				let class = if matches!(kind, CollKind::Set | CollKind::GrowingSet) {
					self.set_class(item)
				} else if self.is_value_pool(target) {
					self.helper("FaValuePool").to_string()
				} else {
					"Set".to_string()
				};
				format!(
					"const result = new {class}(); target[key] = result; const entries = Array.from(value); for (let index = entries.length - 1; index >= 0; index--) {{ const entry = {{}}; work.push([() => result.add(entry.value)]); work.push([{plan}, entries[index], entry, 'value']); }}"
				)
			}
			(TypeKind::CArray(from, _), TypeKind::CArray(to, _)) => {
				let plan = self.copy_plan(node, from, to);
				format!(
					"const result = new Array(value.length); target[key] = result; for (let index = value.length - 1; index >= 0; index--) work.push([{plan}, value[index], result, index]);"
				)
			}
			(TypeKind::Member(nominal, index), TypeKind::Object(_)) => {
				let payload = self.types.payload(nominal, index);
				let plan = self.copy_plan(node, payload, target);
				format!("work.push([{plan}, value, target, key]);")
			}
			_ => {
				let converted =
					self.convert_representation(node, Expr::primary("value"), source, target);
				let copied = if self.is_composite(target) {
					self.copy_of(converted, target)
				} else {
					converted
				};
				format!("target[key] = {};", copied.text)
			}
		}
	}

	fn object_copy_plan(&mut self, node: NodeId, source: TypeId, target: TypeId) -> String {
		let fields = self.types.fields_of(target).to_vec();
		let mut slots = Vec::new();
		let mut jobs = Vec::new();
		for field in fields {
			let property = Self::property(self.types.syms.resolve(field.name));
			let actual = self
				.types
				.fields_of(source)
				.iter()
				.find(|actual| actual.name == field.name)
				.map_or(field.ty, |actual| actual.ty);
			let plan = self.copy_plan(node, actual, field.ty);
			slots.push(format!("{property}: undefined"));
			jobs.push(format!(
				"work.push([{plan}, value.{property}, result, {}]);",
				js::string_literal(self.types.syms.resolve(field.name))
			));
		}
		jobs.reverse();
		format!(
			"const result = {{ {} }}; target[key] = result; {}",
			slots.join(", "),
			jobs.join(" ")
		)
	}

	fn copy_target(&self, source: TypeId, target: TypeId) -> TypeId {
		fa_analyzer::lower::copies::copy_destination(
			self.types,
			self.session.module_id(self.file),
			source,
			target,
		)
		.unwrap_or(target)
	}

	fn copy_case_test(&mut self, source: TypeId, alternative: TypeId) -> String {
		let subject = Expr::primary("value");
		if matches!(self.types.kind(alternative), TypeKind::Object(_)) {
			let fields = self.types.fields_of(alternative).to_vec();
			let mut tests = vec!["value !== undefined && value !== null".to_string()];
			for field in fields {
				let name = js::string_literal(self.types.syms.resolve(field.name));
				tests.push(format!("{name} in value"));
				if let Some(literal) = self.literal_of(field.ty) {
					tests.push(format!("value[{name}] === {}", literal.text));
				}
			}
			return tests.join(" && ");
		}
		self.type_test(&subject, source, alternative).text
	}
}
