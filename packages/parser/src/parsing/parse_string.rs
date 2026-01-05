use crate::{
	context::Context,
	nodes::{Node, StringPart},
	parsing::{
		parse_escape_sequence::parse_escape_sequence,
		parse_template_expression::parse_template_expression,
	},
};

pub(crate) fn parse_string(context: &mut Context) -> Node {
	let raw = context.slice();
	if raw.len() < 2 {
		panic!("Invalid string literal");
	}

	let content = &raw[1..raw.len() - 1];
	let mut parts: Vec<StringPart> = Vec::new();
	let mut literal = String::new();
	let mut has_expression = false;
	let bytes = content.as_bytes();
	let mut index = 0;

	while index < bytes.len() {
		match bytes[index] {
			b'\\' => {
				let (escaped, new_index) = parse_escape_sequence(bytes, index);
				literal.push(escaped);
				index = new_index;
			}
			b'{' => {
				if bytes.get(index + 1) == Some(&b'{') {
					literal.push('{');
					index += 2;
					continue;
				}

				has_expression = true;
				if !literal.is_empty() {
					parts.push(StringPart::Literal(literal));
					literal = String::new();
				}

				let expression_start = index + 1;
				let mut expression_end = expression_start;
				while expression_end < bytes.len() && bytes[expression_end] != b'}' {
					expression_end += 1;
				}

				if expression_end >= bytes.len() {
					panic!("Missing closing `}}` in template string");
				}

				let expression_content = &content[expression_start..expression_end];
				if expression_content.trim().is_empty() {
					panic!("Empty expression in template string");
				}

				let expression = parse_template_expression(context, expression_content);
				parts.push(StringPart::Expression(expression));

				index = expression_end + 1;
			}
			b'}' => {
				if bytes.get(index + 1) == Some(&b'}') {
					literal.push('}');
					index += 2;
				} else {
					literal.push('}');
					index += 1;
				}
			}
			_ => {
				literal.push(bytes[index] as char);
				index += 1;
			}
		}
	}

	context.go_to_next_token();

	if has_expression {
		if !literal.is_empty() {
			parts.push(StringPart::Literal(literal));
		}
		Node::StringTemplate { parts }
	} else {
		Node::StringLiteral(literal)
	}
}
