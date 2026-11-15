# Types at runtime

Types live at compile time. To use type information at runtime, you have to turn a type into a value, which is what the operations on this page do.

They run at compile time, so they can only be applied to types the compiler knows, and the value they produce is baked into the output. Like every compile-time operation, they are deterministic.

## `@typeToString(Type): String`

Returns the name of a type as a string.

```fa
let name = @typeToString(String)
-- name is "String"

let numbers = [1, 2, 3]
let itemType = @typeToString(Type<numbers[0]>)
-- itemType is "Integer"
```

`Type<value>` returns the type of a value: see [type functions](/types/type-functions#typevalue).

Since types are [structural](/advanced/structural-typing), two names can designate the same type. The name returned is the one written where the type is used:

```fa
type UserId = Integer

assert @typeToString(UserId) == "UserId"
assert @typeToString(Point<Integer>) == "Point<Integer>"
assert @typeToString({ x: Integer, y: Integer }) == "{ x: Integer, y: Integer }"
```

For `Type<value>`, it is the name of the type the value was declared or created with, and the base type of a literal otherwise.

An anonymous type is printed as it is spelled: the fields of an object in declaration order, the members of a union in the order they are written, a ranged integer with the bounds that differ from its base type (`Integer<min = 0>`), and a function type with its parameter names and its effects.

## `@typeToSchema(Type): Schema`

Returns a description of a type as an ordinary value, of the built-in type `Schema`.

```fa
type MyType = { foo: String = "hello", bar: Integer? }

let myTypeSchema = @typeToSchema(MyType)

assert myTypeSchema == Schema.Named {
	name = "MyType"
	definition = Schema.Object {
		properties = [
			{ name = "foo", definition = Schema.String, isReadonly = false, default = Schema.StringLiteral { value = "hello" } }
			{ name = "bar", definition = Schema.Union { items = [Schema.Integer { bits = 64 }, Schema.None] }, isReadonly = false }
		]
	}
}
```

Because the schema is a normal value, you can serialize it, send it over the network, or use it to drive validation:

```fa
use { Json } = std.json

let schema = @typeToSchema(User)
let json = Json.stringify(schema)
```

### The `Schema` type

`Schema` is a [named union](/types/unions#named-unions), with one member per kind of type:

| Member | Data | Describes |
| --- | --- | --- |
| `Schema.Named` | `name: String`, `definition: Schema` | a type used through a name, like `MyType` or `Point<Integer>` |
| `Schema.Reference` | `name: String` | a recursive use of the enclosing `Schema.Named` with this name |
| `Schema.Boolean`, `Schema.String`, `Schema.Bytes`, `Schema.Percentage`, `Schema.BigInt`, `Schema.None`, `Schema.Never`, `Schema.Void` | none | the type of the same name |
| `Schema.Integer` | `bits: 8 \| 16 \| 32 \| 64 \| 128`, `signed: Boolean = true`, `min: BigInt?`, `max: BigInt?` | an integer type, with its range when it is narrower than its storage |
| `Schema.Float` | `bits: 32 \| 64` | a float type |
| `Schema.BooleanLiteral`, `Schema.IntegerLiteral`, `Schema.FloatLiteral`, `Schema.StringLiteral` | `value`, of the matching type (`BigInt` for integers) | a literal type |
| `Schema.Object` | `properties: Array<SchemaProperty>` | an object type, its fields in declaration order |
| `Schema.Union` | `items: Array<Schema>` | an anonymous union |
| `Schema.NamedUnion` | `errors: Boolean`, `members: Array<SchemaMember>` | a `Union` or an `Errors` type |
| `Schema.Enum` | `members: Array<{ name: String, value: Integer }>` | an enumeration |
| `Schema.Fields` | `flags: Array<SchemaFlag>` | a `Fields` type |
| `Schema.Tuple` | `items: Array<Schema>` | a tuple |
| `Schema.Collection` | `kind: String`, `key: Schema?`, `item: Schema` | a collection: `kind` is its name (`"Array"`, `"Map"`, ...), and `key` is set for maps |
| `Schema.Function` | `parameters: Array<SchemaProperty>`, `returns: Schema`, `throws: Schema?`, `async: Boolean` | a function type |

A `Schema.Function` describes the parameters, the result, the errors and `async`, but not the `requires`, `mutates` and `invalidates` effects.

With:

```fa
type SchemaProperty = { name: String, definition: Schema, isReadonly: Boolean, default: Schema? }
type SchemaMember = { name: String, definition: Schema? }
type SchemaFlag = { name: String, flags: Array<SchemaFlag> }
```

The `default` of a property is the literal type of its default value when that value is a literal, and `none` otherwise. A recursive type is described once: inside its own definition, it is a `Schema.Reference` to its name.

A named union or enumeration is described by `Schema.Named`, whose definition is `Schema.NamedUnion` or `Schema.Enum`. Function parameter names are positional (`parameter1`, `parameter2`, …): a function type keeps no parameter names.

## `TypeFromSchema<Schema>: Type`

The inverse: a type function that builds a type from a schema known at compile time. A `Schema.Named` gives a type distinct from every other one, like any named type; applying the same declaration to equal schemas gives the same type, so builds stay deterministic. Combined with `@typeToSchema` and an ordinary function called at compile time, it lets you transform types programmatically.

```fa
type SchemaError = Errors {
	Unsupported = { message: String }
}

function capitalizeSecondLetter = (text: String): String throws IndexError => try "{text[0]}{text[1].toUpperCase()}{text[2..]}"

function capitalizeLiterals = (schema: Schema): Schema throws SchemaError | IndexError => when schema is {
	Schema.StringLiteral as { value } => Schema.StringLiteral { value = try capitalizeSecondLetter(value) }
	Schema.Union as { items } => Schema.Union { items = try items.map(capitalizeLiterals) }
	else => throw SchemaError.Unsupported { message = "Only string literals are supported" }
}

type CapitalizeSecondLetter<Input> = TypeFromSchema<@capitalizeLiterals(@typeToSchema(Input))>

type Shouted = CapitalizeSecondLetter<"hello" | "world">
-- Shouted is "hEllo" | "wOrld"
```

`@capitalizeLiterals(...)` calls the function at compile time. If it throws, the error is reported as a compile error at the place where the type function was applied:

```fa
type Invalid = CapitalizeSecondLetter<Integer>
-- compile error: Only string literals are supported
```

## Keeping type information at runtime

Fa erases types when it compiles. If you need to know at runtime which member of a union a value holds, the union itself carries that information, so `is` and `when` work without any reflection:

```fa
when value is {
	String => console.log("a string")
	Integer => console.log("an integer")
}
```

This works because the members of a union are always [distinguishable at runtime](/types/unions#members-must-be-distinguishable): by their kind of value, by their literal values, by their fields, or by the name of a named union member. On a value that is not a union, there is nothing to test at runtime: `is` with a type is decided while compiling.

Reflection is only needed when you want the *shape* of a type as data: for schema validation, serialization, or generating a form from a type. For that, use `@typeToSchema`.

:::implementation
## Implementation details

### JavaScript

- An anonymous union carries no tag: `is` tests the runtime kind (`typeof`, `Array.isArray`, `instanceof` for the other collections, `=== undefined` for `none`), then the literal value, the range, the presence of a distinguishing field (`"age" in value`) or the value of the discriminant. The checker has proven that these tests decide the member. The kinds of [distinguishable members](/types/unions#members-must-be-distinguishable) are chosen so that each test is one `typeof`, `Array.isArray` or `instanceof`: enumerations and `Fields` of up to 32 flags are numbers; `Int64`, `UInt64`, the 128-bit integers and larger `Fields` are `bigint`s.
- A named `Union` or `Errors` member carries a tag: an object member gets a hidden tag property (`$t`), and a member whose data is not an object is wrapped in a small object holding the tag and the value. Tags are global ids, one per member, assigned per program, so that members of different named unions in one anonymous union (`ApiError | StorageError`) never collide.
- A member whose definition is a literal is the literal itself (`MyUnion.Hello` is `"hello"`): two members can't accept the same literal, so the literal decides the member, and the value stays readable in JSON.

### Native

- Every union is a tag and a payload; see `design/compilation-with-llvm.md`. A member's tag is its index in its named union, so an anonymous union of several named unions adds its own tag around them.

### Type reconstruction

- `TypeFromSchema` builds every schema member except `Schema.Fields`; pinned enumeration values other than their positions remain unsupported. A named union or enumeration takes the enclosing `Schema.Named` name, or the name of the applying declaration. A reference to an enclosing named schema builds a recursive type. A schema that is the value of a generic type function is evaluated for each application; an error is `TF004` at that application, with its `message`.

:::
