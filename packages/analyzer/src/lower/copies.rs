//! Copy decisions (advanced brief §6.2, §6.5, decision D15): when a value
//! stored somewhere must be copied, so that two places never share an
//! object or a collection.
//!
//! A value read from a place is copied when it is stored into a mutable
//! destination (a `mutable` binding, an assignment, a field, an item), or
//! when the place itself may change later (a `mutable` root, a `for`
//! variable over a collection). Fresh values (literals, call results) are
//! never copied, and neither is the last use of a local (`NodeFlags::MOVE`).
//! A conditional value (`if`, `when`, `catch`, `??`, `do`) is read from a
//! place when one of its branches is.
//!
//! Which types own memory is the backend's business (JavaScript: objects,
//! collections, tuples, unions holding them; native code: strings too):
//! [`copies`] takes the answer for the value's type.

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::ids::{ModuleId, NodeId, NominalId, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, View};
use crate::types::{Mode, NominalKind, TypeKind, TypeStore};

/// Immediate values contributing to an expression's result. Constructor
/// fields, collection items and call arguments compute their own stores;
/// they do not inherit the surrounding result's Copy boundary.
pub fn result_inputs(tree: &TypedSyntaxTree, node: NodeId) -> Option<Vec<NodeId>> {
	Some(match syntax::view(tree, node) {
		View::Group(inner) | View::Do(inner) | View::Try(inner) => vec![inner],
		View::If {
			then_body,
			else_body,
			..
		} => std::iter::once(then_body).chain(else_body).collect(),
		View::When { arms, .. } => arms.iter().map(|arm| arm.value).collect(),
		View::Catch {
			expression,
			handler,
			..
		} => vec![expression, handler],
		View::Fallback { operands } => operands.iter().collect(),
		View::Block { statements } => statements.last().into_iter().collect(),
		_ => return None,
	})
}

/// A runtime union alternative. A named member's semantic type is published
/// in the shared store; its original nominal still identifies the source
/// layout and tag in the unit being lowered.
#[derive(Clone, Copy, Debug)]
pub struct CopyAlternative {
	pub ty: TypeId,
	pub member: Option<(NominalId, u16)>,
}

pub fn copy_alternatives(
	types: &TypeStore,
	module: ModuleId,
	source: TypeId,
) -> Vec<CopyAlternative> {
	let source = types.rec_target(source).unwrap_or(source);
	let mut alternatives = Vec::new();
	for &ty in types.union_members(&source) {
		if let TypeKind::NamedUnion(nominal, mask) = types.kind(ty) {
			let count = types.sym_list(types.nominal(nominal).members).len() as u16;
			let mut merger = types.merger(Some(module));
			let global = merger.ty(ty);
			drop(merger);
			let mut scratch = TypeStore::view(types.global());
			let TypeKind::NamedUnion(global_nominal, _) = scratch.kind(global) else {
				unreachable!()
			};
			for index in 0..count {
				if types.mask_has(mask, index) {
					let member = scratch.member(global_nominal, index);
					alternatives.push(CopyAlternative {
						ty: scratch.merger(Some(module)).ty(member),
						member: Some((nominal, index)),
					});
				}
			}
		} else {
			let member = match types.kind(ty) {
				TypeKind::Member(nominal, index) => Some((nominal, index)),
				_ => None,
			};
			alternatives.push(CopyAlternative { ty, member });
		}
	}
	alternatives
}

/// Whether copying to the target retains a written member's identity rather
/// than projecting its data. Compare in one store: local and published ids
/// can describe the same declaration and arguments.
pub fn preserves_copy_alternative(
	types: &TypeStore,
	module: ModuleId,
	source: TypeId,
	target: TypeId,
) -> bool {
	let mut merger = types.merger(Some(module));
	let source = merger.ty(source);
	let target = merger.ty(target);
	drop(merger);
	let scratch = TypeStore::view(types.global());
	scratch.union_members(&target).iter().any(|&alternative| {
		source == alternative
			|| match (scratch.kind(source), scratch.kind(alternative)) {
				(TypeKind::Member(a, index), TypeKind::NamedUnion(b, mask)) => {
					a == b && scratch.mask_has(mask, index)
				}
				(TypeKind::NamedUnion(a, from), TypeKind::NamedUnion(b, to)) if a == b => {
					let count = scratch.sym_list(scratch.nominal(a).members).len() as u16;
					(0..count)
						.all(|index| !scratch.mask_has(from, index) || scratch.mask_has(to, index))
				}
				(
					TypeKind::Member(nominal, _) | TypeKind::NamedUnion(nominal, _),
					TypeKind::ErrorTop,
				) => scratch.nominal(nominal).kind == NominalKind::Errors,
				_ => false,
			}
	})
}

/// The compatible destination for a copied alternative, retaining an exact
/// written alternative before considering structural projections.
pub fn copy_destination(
	types: &TypeStore,
	module: ModuleId,
	source: TypeId,
	target: TypeId,
) -> Option<TypeId> {
	destination(types, module, source, target, true)
}

/// A union destination for a move or borrowed view. Unlike a converting
/// copy, it cannot change collection items or recursive parts.
pub fn implicit_destination(
	types: &TypeStore,
	module: ModuleId,
	source: TypeId,
	target: TypeId,
) -> Option<TypeId> {
	destination(types, module, source, target, false)
}

fn destination(
	types: &TypeStore,
	module: ModuleId,
	source: TypeId,
	target: TypeId,
	copy_inside: bool,
) -> Option<TypeId> {
	let target = types.rec_target(target).unwrap_or(target);
	let choices = types.union_members(&target).to_vec();
	if let Some(&choice) = choices
		.iter()
		.find(|&&choice| preserves_copy_alternative(types, module, source, choice))
	{
		return Some(choice);
	}
	let mut merger = types.merger(Some(module));
	let source = merger.ty(source);
	let global: Vec<_> = choices.iter().map(|&ty| merger.ty(ty)).collect();
	drop(merger);
	let mut scratch = TypeStore::view(types.global());
	choices
		.into_iter()
		.zip(global)
		.find_map(|(original, target)| {
			(scratch.assignable(source, target, Mode::Copy).is_ok()
				&& (copy_inside || scratch.structure_conversion(source, target).is_none()))
			.then_some(original)
		})
}

/// The local symbol at the root of a place expression (`a`, `a.b`,
/// `a[i].b`), if the node reads a place.
pub fn place_root(tree: &TypedSyntaxTree, analysis: &Analysis, n: NodeId) -> Option<SymbolId> {
	match syntax::view(tree, n) {
		View::Group(inner) => place_root(tree, analysis, inner),
		View::Name(_) => match analysis.resolution(n) {
			Resolution::Local(symbol) => Some(symbol),
			_ => None,
		},
		View::Access { target, .. } | View::Index { target, .. } => {
			place_root(tree, analysis, target)
		}
		_ => None,
	}
}

/// The root of the place a value may be read from: [`place_root`], or for
/// a conditional value, the place of one of its branches, a mutable one
/// first.
pub fn value_place_root(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	n: NodeId,
) -> Option<SymbolId> {
	let branches: Vec<NodeId> = match syntax::view(tree, n) {
		View::Group(inner) | View::Do(inner) => vec![inner],
		View::If {
			then_body,
			else_body,
			..
		} => std::iter::once(then_body).chain(else_body).collect(),
		View::When { arms, .. } => arms.iter().map(|arm| arm.value).collect(),
		View::Catch {
			expression,
			handler,
			..
		} => vec![expression, handler],
		View::Fallback { operands } => operands.iter().collect(),
		View::Block { statements } => statements.iter().last().into_iter().collect(),
		// `items.get(0)`, `scores.values()`: elements of the receiver's place
		View::Call { .. } => match element_view(tree, analysis, n) {
			Some((receiver, _)) => vec![receiver],
			None => return None,
		},
		_ => return place_root(tree, analysis, n),
	};
	let roots: Vec<SymbolId> = branches
		.into_iter()
		.filter_map(|branch| value_place_root(tree, analysis, branch))
		.collect();
	roots
		.iter()
		.copied()
		.find(|&root| is_mutable_place(analysis, root))
		.or(roots.first().copied())
}

/// A built-in call whose result holds elements of its receiver as they are
/// stored, not copies, where a collection holds its elements by reference
/// (JavaScript): one element (`items.get(0)`, `scores.get(name)`,
/// `find`), or a new collection of them (`filter`, `values`, `keys`, the
/// non-in-place `sort` and `reverse`, set operations). Returns the node the
/// receiver is read from (its place root is the receiver's) and whether the
/// result is a collection of elements.
pub fn element_view(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	n: NodeId,
) -> Option<(NodeId, bool)> {
	let View::Call { callee, .. } = syntax::view(tree, n) else {
		return None;
	};
	let crate::analysis::Callee::Builtin(id) = analysis.call(n)?.callee else {
		return None;
	};
	let View::Access { target, .. } = syntax::view(tree, callee) else {
		return None;
	};
	let path = crate::builtins::get(id).path;
	let (owner, method) = path.split_once('#')?;
	if !matches!(
		owner,
		"Array"
			| "GrowingArray"
			| "Map" | "GrowingMap"
			| "Set" | "GrowingSet"
			| "Pool" | "GrowingPool"
			| "Columnar"
			| "OrderedColumnar"
			| "Iterator"
	) {
		return None;
	}
	match method {
		"get" | "find" => Some((target, false)),
		"filter" | "values" | "keys" | "sort" | "reverse" | "union" | "intersection"
		| "difference" | "firstValues" | "toArray" => Some((target, true)),
		_ => None,
	}
}

/// Whether a symbol may be assigned or reseated after its declaration
/// (`mutable`, `mutable use`, `reactive`).
pub fn is_reassignable(analysis: &Analysis, symbol: SymbolId) -> bool {
	matches!(
		analysis.symbols.symbol(symbol).kind,
		SymbolKind::Mutable | SymbolKind::MutableUse | SymbolKind::Reactive
	)
}

/// Whether the place rooted at a symbol may change later: a reassignable
/// symbol, or a `for` variable designating an item of a collection.
pub fn is_mutable_place(analysis: &Analysis, root: SymbolId) -> bool {
	is_reassignable(analysis, root)
		|| matches!(
			analysis.symbols.symbol(root).kind,
			SymbolKind::LoopVariable { copy: false }
		)
}

/// Whether the read at a node is the last use of a local, in a copy
/// position (`NodeFlags::MOVE`, parentheses skipped).
pub fn is_move(tree: &TypedSyntaxTree, analysis: &Analysis, n: NodeId) -> bool {
	let mut n = n;
	while let View::Group(inner) = syntax::view(tree, n) {
		n = inner;
	}
	analysis.has_flags(n, NodeFlags::MOVE)
}

/// Whether a value stored into a destination (`destination_mutable`: a
/// `mutable` binding, an assignment, a field or an item) must be copied;
/// `owns_memory` says whether values of its type own memory on the target.
pub fn copies(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	value: NodeId,
	destination_mutable: bool,
	owns_memory: bool,
) -> bool {
	if !owns_memory || is_move(tree, analysis, value) {
		return false;
	}
	let mut borrowed = value;
	while let View::Try(inner) | View::Group(inner) = syntax::view(tree, borrowed) {
		borrowed = inner;
	}
	if matches!(syntax::view(tree, borrowed), View::Call { .. })
		&& analysis.has_flags(borrowed, NodeFlags::PLACE)
	{
		return true;
	}
	let Some(root) = value_place_root(tree, analysis, value) else {
		return false;
	};
	destination_mutable || is_mutable_place(analysis, root)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_implicit_union_destination_never_converts_collection_items() {
		for (target, expected) in [
			("Array<Integer> | String", Some("Array<Integer>")),
			("Array<Int64> | String", None),
			("Array<Integer | String> | String", None),
		] {
			let source = format!(
				"let narrow: Array<Integer<min = 0, max = 10>> = [1]\nlet wide: {target} = copy narrow\nconsole.log(narrow.length, wide)\n"
			);
			let (session, file) = crate::analyze_source(&source);
			assert!(
				session.diagnostics(file).is_empty(),
				"{:?}",
				session.diagnostics(file)
			);
			let input = session
				.analysis(file)
				.copy_inputs
				.values()
				.find(|input| input.produces)
				.unwrap();
			let types = session.types();
			let module = session.module_id(file);
			let selected = implicit_destination(types, module, input.source, input.target);
			assert_eq!(
				selected.map(|ty| types.display(ty).to_string()).as_deref(),
				expected
			);
			assert!(copy_destination(types, module, input.source, input.target).is_some());
		}
	}

	#[test]
	fn converting_union_copies_retain_written_members_across_store_views() {
		let (session, file) = crate::analyze_source(
			"type Status = Union { Ready, Value = { count: Int8 } }\nlet statuses: Array<Status | String> = [Status.Value { count = 1 }, Status.Ready]\nlet copied: Array<Status | String | Integer> = copy statuses\nconsole.log(statuses.length, copied.length)\n",
		);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let input = session
			.analysis(file)
			.copy_inputs
			.values()
			.find(|input| input.produces)
			.unwrap();
		let types = session.types();
		let TypeKind::Collection(_, source, _) = types.kind(input.source) else {
			panic!("an array")
		};
		let TypeKind::Collection(_, target, _) = types.kind(input.target) else {
			panic!("an array")
		};
		let module = session.module_id(file);
		let members = copy_alternatives(types, module, source);
		assert_eq!(
			members.iter().filter(|case| case.member.is_some()).count(),
			2
		);
		for member in members.into_iter().filter(|case| case.member.is_some()) {
			let selected = copy_destination(types, module, member.ty, target).unwrap();
			assert!(matches!(types.kind(selected), TypeKind::NamedUnion(..)));
			assert!(preserves_copy_alternative(
				types, module, member.ty, selected
			));
		}
	}

	/// Whether the value of the last declaration of a source is copied.
	fn copied(source: &str) -> bool {
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let (value, mutable) = (0..tree.nodes.len())
			.rev()
			.map(NodeId::new)
			.find_map(|n| match syntax::view(tree, n) {
				View::Declaration {
					value: Some(value),
					kind,
					..
				} => Some((value, kind == syntax::BindingKind::Mutable)),
				_ => None,
			})
			.expect("a declaration");
		copies(tree, analysis, value, mutable, true)
	}

	#[test]
	fn places_are_copied_fresh_values_and_moves_are_not() {
		// read again after the copy: a copy
		assert!(copied(
			"let a = [1, 2]\nmutable b = a\nb.push(3)\nconsole.log(a, b)\n"
		));
		// a fresh value
		assert!(!copied("mutable b = [1, 2]\nb.push(3)\nconsole.log(b)\n"));
		// two immutable sides
		assert!(!copied("let a = [1, 2]\nlet b = a\nconsole.log(a, b)\n"));
		// a conditional value with a branch read from a mutable place
		assert!(copied(
			"mutable a = [1]\nlet c = true\nlet b = if c { a } else { [2] }\na.push(2)\nconsole.log(a, b)\n"
		));
	}

	#[test]
	fn the_last_use_of_a_local_is_moved() {
		let source = "function f = () => {\n\tmutable a = [1, 2]\n\ta.push(3)\n\tmutable b = a\n\tb.push(4)\n\tconsole.log(b)\n}\nf()\n";
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let moved = (0..tree.nodes.len())
			.map(NodeId::new)
			.any(|n| is_move(tree, analysis, n));
		assert!(moved, "the read of `a` in `mutable b = a` is a move");
	}
	#[test]
	fn explicit_copy_is_fresh_and_preserves_type_and_length() {
		let source = "let original = [1, 2]\nmutable duplicate = copy original\nconsole.log(original[0], duplicate[1])\n";
		let (session, file) = crate::analyze_source(source);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let mut found = false;
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			if let View::Copy(inner) = syntax::view(tree, node) {
				found = true;
				assert_eq!(analysis.type_of(node), analysis.type_of(inner));
				assert!(!is_move(tree, analysis, inner));
				assert!(!copies(tree, analysis, node, true, true));
				assert!(place_root(tree, analysis, node).is_none());
			}
			if let View::Index { .. } = syntax::view(tree, node) {
				assert!(!analysis.has_flags(node, NodeFlags::INDEX_CHECK));
			}
		}
		assert!(found);
	}
}
