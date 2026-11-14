# Fields

A `Fields` type is a set of named flags. Unlike an [enumeration](/types/enumerations), a value can hold any combination of them, including none at all.

```fa
type Permissions = Fields { read, write, admin }
```

`Fields` types are [structural](/advanced/structural-typing): two `Fields` types with the same flags, nested the same way and in the same order, are the same type (the order gives the bits, see [representation](#representation)). This is what lets a [key set](#key-sets) literal be passed wherever a `Fields` type is expected.

## Field sets

A value of a `Fields` type is written with braces, listing the flags it holds:

```fa
let reader = Permissions { read }
let editor = Permissions { read, write }

-- the type name can be left out when it is known
let guest: Permissions = {}
let permissions: Permissions = { read, write }
```

Combine and remove flags with `+` and `-`:

```fa
let readWrite = Permissions.read + Permissions.write
let readOnly = readWrite - Permissions.write

let all: Permissions = { read, write, admin }
let some = all - { admin }
```

Test membership with `is in`:

```fa
if Permissions.write is in permissions {
	console.log("write access granted")
}
```

## Representation

A field set is stored as an unsigned integer with one bit per flag. A type with 12 flags occupies 12 bits, so two bytes. A `Fields` type has at most 128 flags, counting the members of its groups.

```fa
type MyFields = Fields { foo, bar, baz }

assert Integer(MyFields { foo }) == 0b001
assert Integer(MyFields { foo, bar }) == 0b011
```

## Nested fields

Flags can be grouped:

```fa
type MyFields = Fields { foo, bar, nested { baz1, baz2 } }

let a = MyFields { foo, bar }
let b = MyFields { foo, bar, nested }
let c = MyFields { foo, bar, nested { baz1 } }
let d = MyFields { foo, bar, nested { baz1, baz2 } }
```

Naming a group is the same as naming all of its members:

```fa
assert MyFields { nested } == MyFields { nested { baz1, baz2 } }
```

A set is in another when each of its flags is: `MyFields { nested } is in c` is `true` only when both `baz1` and `baz2` are in `c`.

Bits are given in declaration order, the members of a group taking the place of the group.

## String representation

```fa
let myFields = MyFields { foo, bar, nested { baz1, baz2 } }

assert Integer(myFields) == 0b1111

assert myFields.toStringArray() == ["foo", "bar", "nested.baz1", "nested.baz2"]
```

## Key sets

The `{ foo, nested { bar } }` notation is not specific to `Fields` types: it is how Fa writes a **set of keys**, and it is what the key-taking [type functions](/types/type-functions) expect.

Braces are read as a key set only where a `Fields` type is expected. Everywhere else, `{ foo }` is an object literal, short for `{ foo = foo }`.

```fa
type MyType = { foo: String, bar: Integer }

type Picked = Pick<MyType, { foo }> -- { foo: String }
type Omitted = Omit<MyType, { foo }> -- { bar: Integer }
```

Key sets can reach into nested objects:

```fa
type MyType = {
	foo: {
		bar: Integer
		baz: Integer
	}
}

type Trimmed = Omit<MyType, { foo { baz } }> -- { foo: { bar: Integer } }
```

Naming every member of a group is the same as naming the group: `Omit<MyType, { foo { bar, baz } }>` removes the field `foo`, and `Pick<MyType, { foo { bar, baz } }>` keeps the whole field.

`Keys` produces the key set of a type, as a `Fields` type, and `DeepKeys` produces it recursively:

```fa
type MyType = { foo: String, bar: Integer }

type MyKeys = Keys<MyType> -- Fields { foo, bar }
```

Only public data fields are keys: methods and `#private` fields are not. `DeepKeys` stops at a [recursive](/types/unions#unions-and-recursion) field: the field is a key, its own fields are not.

### Key sets as compile-time parameters

Because a key set is a compile-time value, a function can take one as a [compile-time value parameter](/types/generics#type-parameters-and-compile-time-value-parameters), and use it to generate code with `@for`:

```fa
function logProperties = <Value, keys: DeepKeys<Value>>(value: Value) => {
	@for key in keys {
		console.log("{key} = {value[key]}")
	}
}

type Settings = {
	foo: Integer
	bar: Integer
	baz: Integer
	nested: {
		foo: Integer
		bar: Integer
	}
}

let settings: Settings = {
	foo = 12
	bar = 13
	baz = 14
	nested = {
		foo = 15
		bar = 16
	}
}

logProperties<Settings, { foo, baz, nested { foo } }>(settings)
```

`@for` unrolls the loop at compile time: the call above compiles to three `console.log` lines, with no runtime lookup at all.

`@for` over a key set goes through its leaves, in declaration order: a group stands for all of its members. Each `key` is a compile-time path, printed with dots (`"nested.foo"`), and `value[key]` reads the field at that path, with its own type. The body is checked once per key, after the loop is unrolled: an error names the key and the call that caused it.

The key set is a compile-time value parameter, so it is never inferred: `logProperties` always receives its parameters explicitly, before the runtime call. See [explicit parameters](/types/generics#explicit-parameters).

A key set declared with `static` works the same way:

```fa
static loggedKeys: DeepKeys<Settings> = { foo, nested { bar } }

@for key in loggedKeys {
	console.log("{key} = {settings[key]}")
}
```

:::implementation
## Implementation details

### JavaScript

- A field set is a JavaScript number up to 32 flags, and a `BigInt` above, so that the bit operations stay exact.
:::
