use std::collections::HashMap;

use crate::{
	nodes::Node,
	type_error::TypeError,
	typed_syntax_tree::TypedSyntaxTree,
	types::{NumberType, Type},
};
use NumberType::*;

pub fn infer_node_type(tree: &TypedSyntaxTree, node_index: usize) -> Type {
	match tree.at(node_index) {
		Node::Integer(_) => Type::Number(Integer(32)),
		Node::StringLiteral(_) => Type::String,
		Node::StringTemplate { .. } => Type::String,
		Node::Boolean(value) => {
			if *value {
				Type::True
			} else {
				Type::False
			}
		}
		Node::Null => Type::Null,
		Node::Percentage { value } => {
			let inner_type = infer_node_type(tree, *value);
			match inner_type {
				Type::Number(inner_number_type) => Type::Percentage(inner_number_type),
				_ => Type::Errored(TypeError::PercentageMustBeNumber),
			}
		}
		Node::List { items } => Type::Array {
			items: Box::new(merge_inferred_type_list(tree, items)),
		},
		Node::Members { .. } => Type::Object {
			properties: HashMap::new(),
		},
		_ => Type::Any,
	}
}

/// Merge a list of types into a single type.
/// Used for type inference of arrays, tuples, sets and maps.
pub fn merge_inferred_type_list(tree: &TypedSyntaxTree, nodes: &[usize]) -> Type {
	let mut merged_type = Type::Any;
	for node_index in nodes {
		let node_type = infer_node_type(tree, *node_index);
		merged_type = merge_inferred_types(&merged_type, &node_type);
	}
	merged_type
}

/// Merge two inferred types into a single type.
/// For numbers, merge the precision of the two numbers.
/// Other merges should not be allowed.
pub fn merge_inferred_types(type_a: &Type, type_b: &Type) -> Type {
	use std::cmp::max;

	if type_a == type_b {
		return type_a.clone();
	}

	match (type_a, type_b) {
		(Type::Any, other) | (other, Type::Any) => return other.clone(),
		(Type::Never, _) | (_, Type::Never) => return Type::Never,

		// percent is considered as a float
		(Type::Percentage(percent_number_type), other_type)
		| (other_type, Type::Percentage(percent_number_type)) => {
			let sub_type: Type = match percent_number_type {
				BigInteger => Type::Number(Float(64)),
				IntegerLiteral(_) => Type::Number(Float(64)),
				FloatLiteral(_) => Type::Number(Float(64)),
				UnsignedFloat(bits) => Type::Number(UnsignedFloat(*bits)),
				Float(bits) => Type::Number(Float(*bits)),
				Integer(bits) => Type::Number(Float(*bits)),
				UnsignedInteger(bits) => Type::Number(UnsignedFloat(*bits)),
			};
			merge_inferred_types(&sub_type, other_type)
		}

		// 1. INTEGER
		// integer + unsigned integer = integer
		(Type::Number(Integer(a_bits)), Type::Number(UnsignedInteger(b_bits))) => {
			Type::Number(Integer(max(*a_bits, *b_bits)))
		}
		(Type::Number(UnsignedInteger(a_bits)), Type::Number(Integer(b_bits))) => {
			Type::Number(Integer(max(*a_bits, *b_bits)))
		}

		// integer + float = float
		(Type::Number(Integer(a_bits)), Type::Number(Float(b_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}
		(Type::Number(Float(b_bits)), Type::Number(Integer(a_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}

		// integer + unsigned float = float
		(Type::Number(Integer(a_bits)), Type::Number(UnsignedFloat(b_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}
		(Type::Number(UnsignedFloat(b_bits)), Type::Number(Integer(a_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}

		// 2. UNSIGNED INTEGER
		// unsigned integer + float = float
		(Type::Number(UnsignedInteger(a_bits)), Type::Number(Float(b_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}
		(Type::Number(Float(b_bits)), Type::Number(UnsignedInteger(a_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}

		// unsigned integer + unsigned float = unsigned float
		(Type::Number(UnsignedInteger(a_bits)), Type::Number(UnsignedFloat(b_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}
		(Type::Number(UnsignedFloat(b_bits)), Type::Number(UnsignedInteger(a_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}

		// 3. FLOAT
		// float + unsigned float = float
		(Type::Number(Float(a_bits)), Type::Number(UnsignedFloat(b_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}
		(Type::Number(UnsignedFloat(b_bits)), Type::Number(Float(a_bits))) => {
			Type::Number(Float(max(*a_bits, *b_bits)))
		}

		// Other types are considered too distant to be inferred and should be explicit instead
		// Example: string + number = errored
		// Should use an explicit union `string | number`
		_ => Type::Errored(TypeError::TypesTooDistantToBeMergedAndInferred(
			Box::new(type_a.clone()),
			Box::new(type_b.clone()),
		)),
	}
}
