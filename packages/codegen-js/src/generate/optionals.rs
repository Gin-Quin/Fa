//! An optional guard encloses the whole remaining chain, including lifted
//! arguments and awaited/builtin calls. No runtime wrapper or closure.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{self, View};

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

#[derive(Clone, Debug)]
pub(super) struct ChainCall {
	pub root: NodeId,
	pub call: NodeId,
	pub callee: NodeId,
	pub member: Option<NodeId>,
	pub receiver: Expr,
	pub receiver_node: NodeId,
	pub receiver_type: TypeId,
	pub result: TypeId,
}

type Next<'a, 's> = dyn FnMut(&mut Generator<'s>, Expr, TypeId) -> Expr + 'a;

impl<'s> Generator<'s> {
	fn chain_syntax(&self, node: NodeId) -> bool {
		match syntax::view(self.tree, node) {
			View::OptionalAccess { .. } => true,
			View::Access { target, .. } | View::Index { target, .. } => self.chain_syntax(target),
			View::Call { callee, .. } => self.chain_syntax(callee),
			View::Await(value) => self.chain_syntax(value),
			_ => false,
		}
	}

	pub(super) fn chain_candidate(&self, node: NodeId) -> bool {
		!self.fcx.chain_active.contains(&node) && self.chain_syntax(node)
	}

	pub(super) fn chain_read(&mut self, node: NodeId) -> Option<Expr> {
		if let Some((_, value, _)) = self
			.fcx
			.chain_values
			.iter()
			.rev()
			.find(|(n, ..)| *n == node)
		{
			return Some(value.clone());
		}
		let call = self
			.fcx
			.chain_calls
			.iter()
			.rev()
			.find(|call| call.callee == node)?
			.clone();
		Some(match call.member {
			Some(member) => self.apply_member(call.receiver, member),
			None => call.receiver,
		})
	}

	pub(super) fn optional_chain(&mut self, node: NodeId, raw: bool) -> Expr {
		self.chain_value(node, raw.then_some(node), &mut |_, value, _| value)
	}

	/// Starting and detaching belong inside the same guard as the call.
	pub(super) fn optional_start(&mut self, node: NodeId, call: NodeId, callee: NodeId) -> Expr {
		self.chain_call(node, call, callee, None, &mut |_, value, _| value)
	}

	fn chain_value(&mut self, node: NodeId, raw: Option<NodeId>, next: &mut Next<'_, 's>) -> Expr {
		match syntax::view(self.tree, node) {
			View::Access { target, members } | View::OptionalAccess { target, members } => {
				let optional = matches!(syntax::view(self.tree, node), View::OptionalAccess { .. });
				let members: Vec<_> = members.iter().collect();
				self.chain_value(target, raw, &mut |g, value, ty| {
					g.chain_members(value, ty, &members, optional, &mut |g, value, ty| {
						let ty = g
							.analysis
							.optional_results
							.get(&node)
							.copied()
							.unwrap_or(ty);
						g.fcx.chain_values.push((node, value.clone(), ty));
						let value = g.instantiated_function_value(node, value);
						let value = g.reactive_expr(node, value);
						let value = g.sink(node, value);
						g.fcx.chain_values.pop();
						next(g, value, ty)
					})
				})
			}
			View::Call { callee, .. } => self.chain_call(node, node, callee, raw, next),
			View::Await(call) if self.chain_syntax(call) => {
				let call = self.strip_groups(call);
				let View::Call { callee, .. } = syntax::view(self.tree, call) else {
					return self.internal_error(node, "an optional await has no call");
				};
				self.chain_call(node, call, callee, raw, next)
			}
			View::Index { target, .. } if self.chain_syntax(target) => {
				self.chain_value(target, raw, &mut |g, value, ty| {
					g.fcx.chain_values.push((target, value, ty));
					g.fcx.chain_active.push(node);
					let value = g.expr(node);
					g.fcx.chain_active.pop();
					g.fcx.chain_values.pop();
					next(g, value, g.type_of(node))
				})
			}
			_ => {
				let value = self.expr(node);
				next(self, value, self.type_of(node))
			}
		}
	}

	fn chain_members(
		&mut self,
		value: Expr,
		ty: TypeId,
		members: &[NodeId],
		optional: bool,
		next: &mut Next<'_, 's>,
	) -> Expr {
		let Some((&member, rest)) = members.split_first() else {
			return next(self, value, ty);
		};
		let mut apply = |g: &mut Self, value, _| {
			let value = g.apply_member(value, member);
			g.chain_members(value, g.type_of(member), rest, optional, next)
		};
		if optional {
			self.chain_guard(value, ty, &mut apply)
		} else {
			apply(self, value, ty)
		}
	}

	fn chain_call(
		&mut self,
		root: NodeId,
		call: NodeId,
		callee: NodeId,
		raw: Option<NodeId>,
		next: &mut Next<'_, 's>,
	) -> Expr {
		let (target, members, optional) = match syntax::view(self.tree, callee) {
			View::Access { target, members } => (target, members, false),
			View::OptionalAccess { target, members } => (target, members, true),
			_ => {
				return self.chain_value(callee, raw, &mut |g, value, ty| {
					let binding = g.chain_binding((root, call, callee), None, value, ty);
					g.chain_invoke(binding, raw, next)
				});
			}
		};
		let members: Vec<_> = members.iter().collect();
		let Some((&member, prefix)) = members.split_last() else {
			return self.internal_error(call, "an optional call has no member");
		};
		self.chain_value(target, raw, &mut |g, value, ty| {
			g.chain_members(value, ty, prefix, optional, &mut |g, value, ty| {
				let mut invoke = |g: &mut Self, receiver, ty| {
					let binding = g.chain_binding((root, call, callee), Some(member), receiver, ty);
					g.chain_invoke(binding, raw, next)
				};
				if optional {
					g.chain_guard(value, ty, &mut invoke)
				} else {
					invoke(g, value, ty)
				}
			})
		})
	}

	fn chain_binding(
		&self,
		nodes: (NodeId, NodeId, NodeId),
		member: Option<NodeId>,
		receiver: Expr,
		ty: TypeId,
	) -> ChainCall {
		let (root, call, callee) = nodes;
		let receiver_node = match syntax::view(self.tree, callee) {
			View::Access { target, members } | View::OptionalAccess { target, members } => members
				.len()
				.checked_sub(2)
				.and_then(|index| members.get(index))
				.unwrap_or(target),
			_ => callee,
		};
		ChainCall {
			root,
			call,
			callee,
			member,
			receiver,
			receiver_node,
			receiver_type: member
				.and_then(|member| self.analysis.optional_receivers.get(&member).copied())
				.unwrap_or(ty),
			result: self
				.analysis
				.optional_results
				.get(&call)
				.copied()
				.unwrap_or_else(|| self.type_of(call)),
		}
	}

	fn chain_invoke(
		&mut self,
		binding: ChainCall,
		raw: Option<NodeId>,
		next: &mut Next<'_, 's>,
	) -> Expr {
		let ChainCall {
			root,
			call,
			callee,
			result,
			..
		} = binding;
		self.fcx.chain_values.push((
			binding.receiver_node,
			binding.receiver.clone(),
			binding.receiver_type,
		));
		self.fcx.chain_calls.push(binding);
		self.fcx.chain_active.extend([root, call]);
		let value = if raw == Some(call) {
			let View::Call { arguments, .. } = syntax::view(self.tree, call) else {
				unreachable!("a chain call")
			};
			self.call(call, callee, arguments)
		} else {
			self.expr(root)
		};
		self.fcx
			.chain_active
			.truncate(self.fcx.chain_active.len() - 2);
		self.fcx.chain_calls.pop();
		self.fcx.chain_values.pop();
		next(self, value, result)
	}

	fn chain_guard(&mut self, value: Expr, ty: TypeId, next: &mut Next<'_, 's>) -> Expr {
		if !self.types.union_members(&ty).contains(&TypeId::NONE) {
			return next(self, value, ty);
		}
		let value = self.keep_value(value);
		let (lines, result) =
			self.capture_at(self.out.indentation() + 1, |g| next(g, value.clone(), ty));
		let none = Expr::constant("undefined", Prec::Primary);
		let test = js::binary(&value, "===", &none, Prec::Equality);
		if lines.is_empty() {
			return js::conditional(&test, &none, &result);
		}
		let name = self.temp();
		self.out.line(format!("let {name};"));
		self.out.open(format!("if ({} !== undefined)", value.text));
		self.out.append(lines);
		self.out.line(format!("{name} = {};", result.text));
		self.out.close("");
		Expr::name(name, true)
	}
}
