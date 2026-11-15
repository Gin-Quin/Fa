//! Type tests on values (`is`, `when`).

use ethnum::I256;

use crate::ir::TypeTest;
use crate::value::{MemberKind, Value, equals};

/// Whether a value passes a type test.
pub fn test(test: &TypeTest, value: &Value) -> bool {
	match test {
		TypeTest::Any => true,
		TypeTest::None => matches!(value, Value::None),
		TypeTest::Boolean => matches!(value, Value::Boolean(_)),
		TypeTest::Integer { min, max } => {
			matches!(value, Value::Integer(integer) if min <= integer && integer <= max)
		}
		TypeTest::Float => matches!(value, Value::Float(_)),
		TypeTest::String => matches!(value, Value::String(_)),
		TypeTest::Array => matches!(value, Value::Array(_)),
		TypeTest::Map => matches!(value, Value::Map(_)),
		TypeTest::Set => matches!(value, Value::Set(_)),
		TypeTest::Tuple(length) => matches!(value, Value::Tuple(items) if items.len() == *length),
		TypeTest::Object => matches!(value, Value::Object(_)),
		TypeTest::Function => matches!(value, Value::Function(_)),
		TypeTest::Range => matches!(value, Value::Range { .. }),
		TypeTest::Literal(literal) => equals(literal, value),
		TypeTest::Member(name) => matches!(value, Value::Member(member) if member.name == *name),
		TypeTest::Error => {
			matches!(value, Value::Member(member) if member.kind == MemberKind::Error)
		}
		TypeTest::Union(tests) => tests.iter().any(|one| self::test(one, value)),
	}
}

/// The test of an integer range.
pub fn integer(min: I256, max: I256) -> TypeTest {
	TypeTest::Integer { min, max }
}
