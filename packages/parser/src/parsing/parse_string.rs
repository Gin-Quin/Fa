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
				let first = bytes[index];
				if first < 0x80 {
					literal.push(first as char);
					index += 1;
					continue;
				}

				// Fast UTF-8 decode without iterator overhead.
				let (codepoint, len) = if first < 0xE0 {
					if index + 1 >= bytes.len() {
						panic!("Invalid UTF-8 in string literal");
					}
					let b1 = unsafe { *bytes.get_unchecked(index + 1) };
					let value = (((first & 0x1F) as u32) << 6) | ((b1 & 0x3F) as u32);
					(value, 2)
				} else if first < 0xF0 {
					if index + 2 >= bytes.len() {
						panic!("Invalid UTF-8 in string literal");
					}
					let b1 = unsafe { *bytes.get_unchecked(index + 1) };
					let b2 = unsafe { *bytes.get_unchecked(index + 2) };
					let value = (((first & 0x0F) as u32) << 12)
						| (((b1 & 0x3F) as u32) << 6)
						| ((b2 & 0x3F) as u32);
					(value, 3)
				} else {
					if index + 3 >= bytes.len() {
						panic!("Invalid UTF-8 in string literal");
					}
					let b1 = unsafe { *bytes.get_unchecked(index + 1) };
					let b2 = unsafe { *bytes.get_unchecked(index + 2) };
					let b3 = unsafe { *bytes.get_unchecked(index + 3) };
					let value = (((first & 0x07) as u32) << 18)
						| (((b1 & 0x3F) as u32) << 12)
						| (((b2 & 0x3F) as u32) << 6)
						| ((b3 & 0x3F) as u32);
					(value, 4)
				};

				let character = unsafe { char::from_u32_unchecked(codepoint) };
				literal.push(character);
				index += len;
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
