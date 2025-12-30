# Types at runtime

## Types to values

Some functions allow you to create runtime values from types.

These functions must be compile-time compatible, i.e. either **pure** or relying on external compile-time values only.

### @typeToSchema(Type): Schema

Returns the schema of a type.

```ts
type MyType = { foo = "hello", bar?: Integer }

myTypeSchema = @typeToSchema(MyType)

assert myTypeSchema == {
  type: "Object",
  properties: {
    foo: { definition: { type: "String" }, default: "hello" },
    bar: { definition: { type: "Integer" }, optional: true }
  }
}
```

In conjunction with the `TypeFromSchema` type function, it can be used to generate complex types:

```ts
type CapitalizeSecondLetter = (Type) => {
  schema = @typeToSchema(Type)

  capitalizeSecondLetter = (string: String) => string[0] + string[1].uppercase() + string[2..]

  when schema.type is {
    "StringLiteral" => TypeFromSchema({
      type = "StringLiteral"
      value = capitalizeSecondLetter(schema.value)
    })

    "Union" => {
      newItems = Array(Schema::StringLiteral, capacity = schema.items.length)
      
      for schema.items as item {
        assert item.type is "StringLiteral"
        
        newItems.push({
          type = "StringLiteral"
          value = capitalizeSecondLetter(item.value)
        })
      }

      return TypeFromSchema({
        type = "Union"
        items = newItems
      })
    }

    else => {
      return Error("Unsupported type argument: {Type}")
    }
  }
}
```

### @typeToString(Type): String

Returns the string representation of a type.

```ts
type MyType = @typeToString(String)
// MyType is "String"
```
