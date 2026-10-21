# Metaprogramming

Metaprogramming is code that runs over code. In Fa it means one thing: **evaluating code at compile time**, with the same syntax you use at runtime.

There is no macro language, no separate syntax, and no external code generator. There is one prefix operator, `@`, and one declaration keyword, `static`.

## Why it matters

- **Performance** — computations moved to compile time cost nothing at runtime.
- **Safety** — a schema, a query or a route can be validated while compiling.
- **Less boilerplate** — repetitive code is generated from data you already have.
- **Reflection** — a type's shape becomes an ordinary value you can inspect.

TypeScript cannot do any of this, because it is a superset of JavaScript. Rust can, but only by inventing a separate macro language. Fa runs Fa.

## Static values

A **static value** exists only at compile time. Declare one with `static`:

```fa
static maximumUsers = 1000
static version = @readFile("../version.txt")
```

A static value must be computable by the compiler. It is not present in the output: every use of it is replaced by its value.

```fa
static foo = 12
console.log(foo)
```

compiles to:

```js
console.log(12)
```

## The `@` operator

`@` forces an expression to be evaluated at compile time.

```fa
let foo = 12
console.log(@foo) -- compiled to console.log(12)
```

This works because `foo` is immutable and initialised with a value the compiler knows. Otherwise it is an error:

```fa
let foo = readInput()
console.log(@foo) -- error: `foo` is not known at compile time
```

`@` and `static` are two sides of the same thing: `static` declares a value that is *always* compile-time, `@` evaluates one expression at compile time.

## Compile-time functions

Any function whose arguments are all known at compile time can be evaluated at compile time:

```fa
function add = (x: Int, y: Int): Int => x + y

let result = @add(2, 4) -- compiled to `let result = 6`
```

The function must be **deterministic**: given the same inputs, it must produce the same output. Reading the file system is allowed, because the build inputs are part of the build; reading the clock, the network or a random source is not.

```fa
static timestamp = @now()   -- error: `now` is not deterministic
static config = @readFile("config.json") -- fine
```

A function can require compile-time arguments by prefixing its parameters with `@`:

```fa
function repeat = (@count: UInt, value: String): String => {
  mutable result = ""
  @for 0..count {
    result += value
  }
  return result
}

repeat(3, "ab") -- compiled to `"ab" + "ab" + "ab"`
```

See [generics](../types/generics#compile-time-value-parameters).

## Compile-time loops

`@for` unrolls at compile time. Its iterable must be a compile-time value.

```fa
static fruits = @["apple", "banana", "cherry"]

@for fruits >> fruit {
  console.log("Fruit: {fruit}")
}
```

produces exactly:

```fa
console.log("Fruit: apple")
console.log("Fruit: banana")
console.log("Fruit: cherry")
```

## Compile-time literals

| Syntax | Meaning |
| --- | --- |
| `@[...]` | a compile-time array |
| `@{...}` | a [key set](../types/fields#key-sets) |
| `@name` | evaluate `name` at compile time |
| `@name(...)` | call `name` at compile time |

## Reflection

Types can be turned into values:

```fa
let name = @typeToString(User)
let schema = @typeToSchema(User)
```

and values back into types:

```fa
type Generated = TypeFromSchema(schema)
```

See [types at runtime](../types/types-at-runtime).

## The file system

The compiler can read files and directories while compiling:

```fa
static version = @readFile("../version.txt")
static routes = @readDirectory("../routes")
static configuration = @parseJson(@readFile("config.json"))

console.log(configuration.port)
```

Every file read this way becomes a build dependency: change it, and the affected code is recompiled.

Two constants describe the current file:

```fa
static currentFile = @fileName
static directory = @directoryName
```

## Compile-time modules

A file whose name starts with `@` is evaluated at compile time, and everything it exports is a static value:

```fa
-- @configuration.fa
export = {
  port = 8080
  debug = false
  stage = "development"
}
```

```fa
-- anywhere else
if configuration.debug {
  enableVerboseLogs() -- removed entirely from the output when debug is false
}
```

## How it works

Fa ships an interpreter that executes Fa at compile time. It supports a subset of the standard library, including deterministic I/O. This is the same interpreter used by `fa run`, which is why compile-time code needs no separate dialect.

## What it is good for

- **Configuration** — read settings at build time, and let dead branches be removed.
- **API clients** — generate typed clients from a schema file.
- **Routing** — build a route table from the file system.
- **Query validation** — check SQL or GraphQL against a schema while compiling.
- **Component variants** — expand a set of variants into specialised code.
- **Domain-specific languages** — parse a small language into Fa code.

:::caution
Compile-time work happens on every build. Keep it proportionate: a type function that recurses deeply, or a `@for` over thousands of items, will be felt in your compile times.
:::
