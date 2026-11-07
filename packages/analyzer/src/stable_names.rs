//! Names of types that are the same in the session of every unit of a
//! program (builds per unit: the keys of the whole-program facts, `fa-codegen-llvm`'s
//! `facts.rs`, and of the proofs' claims, `proofs.rs`).

use crate::ids::TypeId;
use crate::types::{CollKind, TypeKind, TypeStore};

/// The name of a type that is the same in the session of every unit, if it
/// has one: a canonical spelling built from declaration identities
/// (`DeclKey`: module path, enclosing names, disambiguator, as the symbols of
/// units are named) and type arguments, independent of how a unit imported
/// or aliased the type. Anonymous structural types are spelled from their
/// parts (an object's fields, already sorted by name; a union's members,
/// sorted); a recursive reference is `^n` (n levels up). Built-in scalars are
/// spelled as Fa writes them. A function type is spelled from its signature,
/// a type parameter by its position (`$i`) and constraint, a task by its
/// result and errors. `None` for a type without a spelling that is stable
/// (references, iterators…).
pub fn stable_type_name(types: &TypeStore, ty: TypeId) -> Option<String> {
	let mut out = String::new();
	let mut stack = Vec::new();
	write_stable(types, ty, &mut stack, &mut out).then_some(out)
}

/// A declaration's identity: `fa.<module path>.<names>[#<n>]`.
pub fn stable_decl_name(types: &TypeStore, key: crate::ids::DeclKey) -> String {
	let data = types.decl_key_data(key);
	let mut name = format!("fa.{}", types.module_path(data.module));
	for &sym in types.sym_list(data.names) {
		name.push('.');
		name.push_str(types.syms.resolve(sym));
	}
	if data.disambiguator != 0 {
		name.push_str(&format!("#{}", data.disambiguator));
	}
	name
}

/// Writes the spelling of a nominal type: its declaration, then its type
/// arguments.
fn write_nominal(
	types: &TypeStore,
	nominal: crate::ids::NominalId,
	stack: &mut Vec<TypeId>,
	out: &mut String,
) -> bool {
	let info = types.nominal(nominal);
	out.push_str(&stable_decl_name(types, info.key));
	let args = types.list_items(info.args).to_vec();
	if !args.is_empty() {
		out.push('<');
		for (index, arg) in args.into_iter().enumerate() {
			if index > 0 {
				out.push_str(", ");
			}
			if !write_stable(types, arg, stack, out) {
				return false;
			}
		}
		out.push('>');
	}
	true
}

fn write_stable(types: &TypeStore, ty: TypeId, stack: &mut Vec<TypeId>, out: &mut String) -> bool {
	// Recursive references are transparent graph edges. Unfold before the
	// ancestor lookup, so an object's self edge gets the same back-reference
	// in a source session and after merging its type into another unit.
	let mut ty = ty;
	if matches!(types.kind(ty), TypeKind::Rec(_)) {
		let mut aliases = rustc_hash::FxHashSet::default();
		while matches!(types.kind(ty), TypeKind::Rec(_)) {
			if !aliases.insert(ty) {
				return false;
			}
			let Some(target) = types.rec_target(ty) else {
				return false;
			};
			ty = target;
		}
	}
	if let Some(depth) = stack.iter().rev().position(|&outer| outer == ty) {
		out.push_str(&format!("^{depth}"));
		return true;
	}
	stack.push(ty);
	let written = match types.kind(ty) {
		TypeKind::Boolean
		| TypeKind::BoolLit(_)
		| TypeKind::Int(_)
		| TypeKind::Float(_)
		| TypeKind::FloatLit(..)
		| TypeKind::BigInt
		| TypeKind::BigIntLit(_)
		| TypeKind::Percentage
		| TypeKind::String
		| TypeKind::StringLit(_)
		| TypeKind::Character
		| TypeKind::None
		| TypeKind::Void
		| TypeKind::Never
		| TypeKind::Range => {
			out.push_str(&types.display(ty));
			true
		}
		TypeKind::Object(shape) => match types.shape(shape).nominal {
			Some(nominal) => write_nominal(types, nominal, stack, out),
			None => {
				let fields: Vec<(String, TypeId)> = types
					.shape_fields(shape)
					.iter()
					.map(|field| (types.syms.resolve(field.name).to_string(), field.ty))
					.collect();
				out.push('{');
				let mut written = true;
				for (index, (name, field)) in fields.into_iter().enumerate() {
					if index > 0 {
						out.push_str(", ");
					}
					out.push_str(&name);
					out.push_str(": ");
					written &= write_stable(types, field, stack, out);
				}
				out.push('}');
				written
			}
		},
		TypeKind::Tuple(list) => {
			let items = types.list_items(list).to_vec();
			out.push_str("Tuple<");
			let mut written = true;
			for (index, item) in items.into_iter().enumerate() {
				if index > 0 {
					out.push_str(", ");
				}
				written &= write_stable(types, item, stack, out);
			}
			out.push('>');
			written
		}
		TypeKind::Union(list) => {
			let mut members = Vec::new();
			let mut written = true;
			for member in types.list_items(list).to_vec() {
				let mut spelled = String::new();
				written &= write_stable(types, member, stack, &mut spelled);
				members.push(spelled);
			}
			members.sort();
			out.push('(');
			out.push_str(&members.join(" | "));
			out.push(')');
			written
		}
		TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
			let written = write_nominal(types, nominal, stack, out);
			// the members it holds, by index (a mask's id is the session's)
			let count = types.sym_list(types.nominal(nominal).members).len() as u16;
			let held: Vec<String> = (0..count)
				.filter(|&index| types.mask_has(mask, index))
				.map(|index| index.to_string())
				.collect();
			out.push_str(&format!("/{}", held.join(",")));
			written
		}
		TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) => {
			let written = write_nominal(types, nominal, stack, out);
			out.push_str(&format!(".{index}"));
			written
		}
		TypeKind::CStruct(nominal) => write_nominal(types, nominal, stack, out),
		TypeKind::Function(sig) => write_signature(types, sig, stack, out),
		TypeKind::Task(result, error) => {
			out.push_str("Task<");
			let mut written = write_stable(types, result, stack, out);
			out.push_str(", ");
			written &= write_stable(types, error, stack, out);
			out.push('>');
			written
		}
		// a type parameter by its position, with its constraint: types equal
		// up to the renaming of their parameters share a name
		TypeKind::Param(param) => {
			let info = types.param(param);
			out.push_str(&format!("${}", info.index));
			if info.value {
				out.push_str(" value");
			}
			if info.constraint != TypeId::UNKNOWN {
				out.push_str(": ");
				if !write_stable(types, info.constraint, stack, out) {
					stack.pop();
					return false;
				}
			}
			true
		}
		TypeKind::ImplicitParam(_, index) => {
			out.push_str(&format!("$implicit{index}"));
			true
		}
		TypeKind::Collection(kind, item, key) => {
			out.push_str(&format!("{kind:?}<"));
			let mut written = true;
			if matches!(kind, CollKind::Map | CollKind::GrowingMap) {
				written &= write_stable(types, key, stack, out);
				out.push_str(", ");
			}
			written &= write_stable(types, item, stack, out);
			out.push('>');
			written
		}
		_ => false,
	};
	stack.pop();
	written
}

/// Writes a function type: its requirements, parameters (variadic marked),
/// result, errors, effects (paths rooted at parameters by position), the
/// callbacks it stores, and whether it is asynchronous.
fn write_signature(
	types: &TypeStore,
	sig: crate::ids::SigId,
	stack: &mut Vec<TypeId>,
	out: &mut String,
) -> bool {
	let sig = types.sig(sig);
	let mut written = true;
	if sig.is_async {
		out.push_str("async ");
	}
	out.push('(');
	let params = types.params(sig.params).to_vec();
	for (index, param) in params.into_iter().enumerate() {
		if index > 0 {
			out.push_str(", ");
		}
		if param.variadic {
			out.push_str("...");
		}
		written &= write_stable(types, param.ty, stack, out);
	}
	out.push_str(") => ");
	written &= write_stable(types, sig.ret, stack, out);
	if sig.throws != TypeId::NEVER {
		out.push_str(" throws ");
		written &= write_stable(types, sig.throws, stack, out);
	}
	let requirements = types.requirements(sig.requires).to_vec();
	if !requirements.is_empty() {
		out.push_str(" requires ");
		for (index, requirement) in requirements.into_iter().enumerate() {
			if index > 0 {
				out.push_str(", ");
			}
			out.push_str(types.syms.resolve(requirement.name));
			out.push_str(": ");
			written &= write_stable(types, requirement.ty, stack, out);
		}
	}
	let (mutates, invalidates) = types.effects(sig.effects);
	for (word, paths) in [("mutates", mutates), ("invalidates", invalidates)] {
		if paths.is_empty() {
			continue;
		}
		let mut texts: Vec<String> = paths
			.iter()
			.map(|&path| crate::effects::path_text(types, path, &|_| None))
			.collect();
		texts.sort();
		out.push_str(&format!(" {word} {}", texts.join(", ")));
	}
	if sig.stored != 0 {
		out.push_str(&format!(" stores {:#x}", sig.stored));
	}
	written
}
