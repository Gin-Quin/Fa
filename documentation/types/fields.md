# Fields

A `Fields` type is a set of named flags. Unlike an [enumeration](./enumerations), a value can hold any combination of them — including none at all.

```fa
type MyFields = Fields { foo, bar, baz }
```

## Field sets

A value of a `Fields` type is written with the key-set literal `@{ ... }`:

```fa
let myFields = MyFields @{ foo }
let myFields = MyFields @{ foo, bar }

-- the type name can be left out when it is known
let myFields: MyFields = @{ foo }
let myFields: MyFields = @{ foo, bar }
```

Combine and remove flags with `+` and `-`:

```fa
let readWrite = MyFields.foo + MyFields.bar
let readOnly  = readWrite - MyFields.bar

let all   = @{ foo, bar, baz }
let some  = all - @{ baz }
```

Test membership with `is in`:

```fa
if MyFields.foo is in myFields {
  console.log("foo is set")
}
```

## Representation

A field set is stored as an unsigned integer with one bit per flag. A type with 12 flags occupies 12 bits, so two bytes.

```fa
type MyFields = Fields { foo, bar, baz }

assert(MyFields @{ foo } == 0b001)
assert(MyFields @{ foo, bar } == 0b011)
```

## Nested fields

Flags can be grouped:

```fa
type MyFields = Fields { foo, bar, nested { baz1, baz2 } }

let a = MyFields @{ foo, bar }
let b = MyFields @{ foo, bar, nested }
let c = MyFields @{ foo, bar, nested { baz1 } }
let d = MyFields @{ foo, bar, nested { baz1, baz2 } }
```

Naming a group is the same as naming all of its members:

```fa
assert(MyFields @{ nested } == MyFields @{ nested { baz1, baz2 } })
```

## String representation

```fa
let myFields = MyFields @{ foo, bar, nested { baz1, baz2 } }

assert(myFields == 0b1111)

assert(Fields.toStringArray(myFields) == ["foo", "bar", "nested.baz1", "nested.baz2"])
```

## Key sets

The `@{ ... }` literal is not specific to `Fields` types: it is Fa's way of writing a **set of keys**, and it is what the key-taking [type functions](./type-functions) expect.

```fa
type MyType = { foo: String, bar: Int }

type Picked  = Pick(MyType, @{ foo })   -- { foo: String }
type Omitted = Omit(MyType, @{ foo })   -- { bar: Int }
```

Key sets can reach into nested objects:

```fa
type MyType = {
  foo: {
    bar: Int
    baz: Int
  }
}

type Trimmed = Omit(MyType, @{ foo { baz } }) -- { foo: { bar: Int } }
```

`Keys` produces the key set of a type, and `DeepKeys` produces it recursively:

```fa
type MyType = { foo: String, bar: Int }

type MyKeys = Keys(MyType) -- Fields { foo, bar }
```

### Key sets as parameters

Because a key set is a compile-time value, a function can take one and use it to generate code:

```fa
function logProperties = (object: Object, @keys: DeepKeys(object)) => {
  @for keys >> key {
    console.log("{key} = {object[key]}")
  }
}

let myObject = {
  foo = 12
  bar = 13
  baz = 14
  nested = {
    foo = 15
    bar = 16
  }
}

logProperties(myObject, @{ foo, baz, nested { foo } })
```

The call above compiles to three `console.log` lines with no runtime lookup at all.
