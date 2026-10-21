# Types at runtime

Types live at compile time. To use type information at runtime, you have to turn a type into a value — which is what the functions on this page do.

They all run at compile time, so they can only be applied to types the compiler knows, and the value they produce is baked into the output.

## `@typeToString(Type): String`

Returns the name of a type as a string.

```fa
let name = @typeToString(String)
-- name is "String"

let numbers = [1, 2, 3]
let itemType = @typeToString(TypeOf(numbers[0]))
-- itemType is "Int"
```

## `@typeToSchema(Type): Schema`

Returns a description of a type as an ordinary object.

```fa
type MyType = { foo = "hello", bar: Int? }

let myTypeSchema = @typeToSchema(MyType)

assert(myTypeSchema == {
  type = "Object"
  properties = {
    foo = { definition = { type = "String" }, default = "hello" }
    bar = { definition = { type = "Int" }, optional = true }
  }
})
```

Because the schema is a normal value, you can serialise it, send it over the network, or use it to drive validation:

```fa
let schema = @typeToSchema(User)
let json = Json.stringify(schema)
```

## `TypeFromSchema(Schema): Type`

The inverse: builds a type from a schema value. Combined with `@typeToSchema`, it lets you transform types programmatically.

```fa
type CapitalizeSecondLetter = (Input: Type) => {
  let schema = @typeToSchema(Input)

  let capitalize = (text: String): String =>
    text[0] + text[1].toUpperCase() + text[2..]

  return when schema.type is {
    "StringLiteral" => TypeFromSchema({
      type = "StringLiteral"
      value = capitalize(schema.value)
    })

    "Union" => {
      mutable newItems = Array(Schema, capacity = schema.items.length) {}

      for schema.items >> item {
        if item.type is not "StringLiteral" {
          return Error("Every member must be a string literal")
        }

        newItems.push({
          type = "StringLiteral"
          value = capitalize(item.value)
        })
      }

      TypeFromSchema({
        type = "Union"
        items = newItems
      })
    }

    else => Error("Unsupported type argument: {@typeToString(Input)}")
  }
}

type Shouted = CapitalizeSecondLetter("hello" | "world")
-- Shouted is "hEllo" | "wOrld"
```

A type function that returns an `Error` reports it as a compile error at the point where the type function was applied.

## Keeping type information at runtime

Fa erases types when it compiles. If you need to know at runtime which member of a union a value holds, the union itself carries that information, so `is` and `when` work without any reflection:

```fa
when value is {
  String => console.log("a string")
  Int => console.log("an integer")
}
```

Reflection is only needed when you want the *shape* of a type as data — for schema validation, serialisation, or generating a form from a type. For that, use `@typeToSchema`.
