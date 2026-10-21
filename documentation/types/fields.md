# Fields

A `Fields` type is a set of named flags. Unlike an [enumeration](/types/enumerations), a value can hold any combination of them, including none at all.

```fa
type Permissions = Fields { read, write, admin }
```

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

A field set is stored as an unsigned integer with one bit per flag. A type with 12 flags occupies 12 bits, so two bytes.

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

## String representation

```fa
let myFields = MyFields { foo, bar, nested { baz1, baz2 } }

assert Integer(myFields) == 0b1111

assert Fields.toStringArray(myFields) == ["foo", "bar", "nested.baz1", "nested.baz2"]
```

## Key sets

The `{ foo, nested { bar } }` notation is not specific to `Fields` types: it is how Fa writes a **set of keys**, and it is what the key-taking [type functions](/types/type-functions) expect.

```fa
type MyType = { foo: String, bar: Integer }

type Picked = Pick(MyType, { foo }) -- { foo: String }
type Omitted = Omit(MyType, { foo }) -- { bar: Integer }
```

Key sets can reach into nested objects:

```fa
type MyType = {
	foo: {
		bar: Integer
		baz: Integer
	}
}

type Trimmed = Omit(MyType, { foo { baz } }) -- { foo: { bar: Integer } }
```

`Keys` produces the key set of a type, as a `Fields` type, and `DeepKeys` produces it recursively:

```fa
type MyType = { foo: String, bar: Integer }

type MyKeys = Keys(MyType) -- Fields { foo, bar }
```

### Key sets as compile-time parameters

Because a key set is a compile-time value, a function can take one as a [compile-time value parameter](/types/generics#type-parameters-and-compile-time-value-parameters), and use it to generate code with `@for`:

```fa
function logProperties(Value, keys: DeepKeys(Value)) = (value: Value) => {
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

logProperties(Settings, { foo, baz, nested { foo } })(settings)
```

`@for` unrolls the loop at compile time: the call above compiles to three `console.log` lines, with no runtime lookup at all.

A key set declared with `static` works the same way:

```fa
static loggedKeys: DeepKeys(Settings) = { foo, nested { bar } }

@for key in loggedKeys {
	console.log("{key} = {settings[key]}")
}
```
