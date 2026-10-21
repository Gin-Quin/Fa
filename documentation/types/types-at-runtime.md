# Types at runtime

Types live at compile time. To use type information at runtime, you have to turn a type into a value, which is what the operations on this page do.

They run at compile time, so they can only be applied to types the compiler knows, and the value they produce is baked into the output. Like every compile-time operation, they are deterministic.

## `@typeToString(Type): String`

Returns the name of a type as a string.

```fa
let name = @typeToString(String)
-- name is "String"

let numbers = [1, 2, 3]
let itemType = @typeToString(Type(numbers[0]))
-- itemType is "Integer"
```

`Type(value)` returns the type of a value: see [type functions](/types/type-functions#typevalue).

## `@typeToSchema(Type): Schema`

Returns a description of a type as an ordinary value.

```fa
type MyType = { foo: String = "hello", bar: Integer? }

let myTypeSchema = @typeToSchema(MyType)

assert myTypeSchema == {
	type = "Object"
	properties = {
		foo = { definition = { type = "String" }, default = "hello" }
		bar = { definition = { type = "Integer" }, optional = true }
	}
}
```

Because the schema is a normal value, you can serialize it, send it over the network, or use it to drive validation:

```fa
let schema = @typeToSchema(User)
let json = Json.stringify(schema)
```

## `TypeFromSchema(Schema): Type`

The inverse: a type function that builds a type from a schema known at compile time. Combined with `@typeToSchema` and an ordinary function called at compile time, it lets you transform types programmatically.

```fa
type SchemaError = Errors {
	Unsupported = { message: String }
}

function capitalizeSecondLetter = (text: String) => "{text[0]}{text[1].toUpperCase()}{text[2..]}"

function capitalizeLiterals = (schema: Schema): Schema throws SchemaError | IndexError => {
	when schema.type is {
		"StringLiteral" => {
			return {
				type = "StringLiteral"
				value = try capitalizeSecondLetter(schema.value)
			}
		}

		"Union" => {
			mutable newItems = Array(Schema).from([], capacity = schema.items.length)

			for item in schema.items {
				if item.type is not "StringLiteral" {
					throw SchemaError.Unsupported { message = "Every member must be a string literal" }
				}

				newItems.push({
					type = "StringLiteral"
					value = try capitalizeSecondLetter(item.value)
				})
			}

			return {
				type = "Union"
				items = newItems
			}
		}

		else => throw SchemaError.Unsupported { message = "Unsupported type: {schema.type}" }
	}
}

type CapitalizeSecondLetter(Input) = TypeFromSchema(@capitalizeLiterals(@typeToSchema(Input)))

type Shouted = CapitalizeSecondLetter("hello" | "world")
-- Shouted is "hEllo" | "wOrld"
```

`@capitalizeLiterals(...)` calls the function at compile time. If it throws, the error is reported as a compile error at the place where the type function was applied:

```fa
type Invalid = CapitalizeSecondLetter(Integer)
-- compile error: Unsupported type: Integer
```

## Keeping type information at runtime

Fa erases types when it compiles. If you need to know at runtime which member of a union a value holds, the union itself carries that information, so `is` and `when` work without any reflection:

```fa
when value is {
	String => console.log("a string")
	Integer => console.log("an integer")
}
```

Reflection is only needed when you want the *shape* of a type as data: for schema validation, serialization, or generating a form from a type. For that, use `@typeToSchema`.
