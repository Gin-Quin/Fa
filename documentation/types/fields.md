# Fields

Fields are like enums, but they can be combined with the `+` and `-` operators.

```ts
fields MyFields = { foo, bar, baz }

myFields = MyFields.foo + MyFields.bar

myFields = MyFields { foo }
myFields = MyFields { foo, bar }

myFields: MyFields = { foo }
myFields: MyFields = { foo, bar }
```

Under the hood, fields are stored as integers with size depending on the number of fields.

Number of bits in a field structure = number of fields.

For example, a field with 12 fields will be stored as a 12-bit integer, i.e. two bytes.

## Nested fields

Fields can have nested values:

```ts
type MyFields = Fields { foo, bar, nested { baz_1, baz_2 } }

myFields = MyFields { foo, bar }
myFields = MyFields { foo, bar, nested }
myFields = MyFields { foo, bar, nested { baz1 } }
myFields = MyFields { foo, bar, nested { baz1, baz2 } }

myFields: MyFields = { foo, bar }
myFields: MyFields = { foo, bar, nested { baz1 } }
myFields: MyFields = { foo, bar, nested { baz1, baz2 } }

// indicating a nested value is the same as indicating all its values
assert MyFields { nested } == MyFields { nested { baz1, baz2 } }
```

## Get the string representation of a field

A field is internally stored as an unsigned integer.

But you can retrieve the string representation of a field by using the `Fields.toStringArray` method:

```fa
let myFields = MyFields { foo, bar, nested { baz1, baz2 } }

assert myFields == 0b1111 // 15 in decimal

let myFieldsString = Fields.toStringArray(myFields)

assert myFieldsString == ["foo", "bar", "nested.baz1", "nested.baz2"]
```
