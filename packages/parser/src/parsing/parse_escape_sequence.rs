pub(crate) fn parse_escape_sequence(bytes: &[u8], index: usize) -> (char, usize) {
	let Some(&next) = bytes.get(index + 1) else {
		panic!("Unexpected end of escape sequence");
	};

	match next {
		b'\\' => ('\\', index + 2),
		b'"' => ('"', index + 2),
		b'n' => ('\n', index + 2),
		b'r' => ('\r', index + 2),
		b't' => ('\t', index + 2),
		b'0' => ('\0', index + 2),
		b'{' => ('{', index + 2),
		b'}' => ('}', index + 2),
		b'x' => {
			let Some(&first) = bytes.get(index + 2) else {
				panic!("Invalid \\x escape sequence");
			};
			let Some(&second) = bytes.get(index + 3) else {
				panic!("Invalid \\x escape sequence");
			};
			let value = (hex_value(first) << 4) | hex_value(second);
			(
				char::from_u32(value).expect("Invalid \\x escape sequence"),
				index + 4,
			)
		}
		b'u' => {
			if bytes.get(index + 2) != Some(&b'{') {
				panic!("Invalid \\u escape sequence");
			}
			let mut cursor = index + 3;
			let mut value: u32 = 0;
			let mut digits = 0;
			while cursor < bytes.len() && bytes[cursor] != b'}' {
				value = (value << 4) | hex_value(bytes[cursor]);
				cursor += 1;
				digits += 1;
			}
			if cursor >= bytes.len() || bytes[cursor] != b'}' || digits == 0 || digits > 6 {
				panic!("Invalid \\u escape sequence");
			}
			let Some(character) = char::from_u32(value) else {
				panic!("Invalid \\u escape sequence");
			};
			(character, cursor + 1)
		}
		_ => {
			panic!("Unknown escape sequence");
		}
	}
}

fn hex_value(byte: u8) -> u32 {
	match byte {
		b'0'..=b'9' => (byte - b'0') as u32,
		b'a'..=b'f' => (byte - b'a' + 10) as u32,
		b'A'..=b'F' => (byte - b'A' + 10) as u32,
		_ => panic!("Invalid hex digit in escape sequence"),
	}
}
