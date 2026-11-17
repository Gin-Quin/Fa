# Metaprogramming

Metaprogramming is code that runs over code. In Fa, it means one thing: **evaluating code at compile time**, with the same syntax you use at runtime.

There is no macro language, no separate syntax, and no external code generator. There is one declaration keyword, `static`, for compile-time values, and one prefix, `@`, for compile-time operations.

## Why it matters

- **Performance**: computations moved to compile time cost nothing at runtime.
- **Safety**: a schema, a query or a route can be validated while compiling.
- **Less boilerplate**: repetitive code is generated from data you already have, without external code generation tools.
- **Reflection**: the shape of a type becomes an ordinary value you can inspect.

TypeScript cannot do any of this, because it is a superset of JavaScript. Rust can, but only through a separate macro language. Fa runs Fa.

## Static values

A **static value** is computed by the compiler, and exists only at compile time. Declare it with `static`, and use it plainly, like any other value:

```fa
static maximumUsers = 1000
static version = @readFile("../version.txt")

console.log(version)
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

Some static values are provided by the compiler, like `target`, the target being compiled (`"web"`, `"native"`, ...), and `configuration`, the active [configuration](/architecture/configuration#configuration-at-compile-time).

## The `@` prefix

`@` marks a **compile-time operation**: a call evaluated by the compiler, or a control-flow construct resolved while compiling. It is never used on a value: a static value is used plainly.

| Syntax | Meaning |
| --- | --- |
| `@add(2, 4)` | call a function at compile time |
| `@if condition { } @else { }` | keep only one branch in the output |
| `@for item in items { }` | unroll a loop |
| `@readFile(path)`, `@readDirectory(path)` | read the file system while compiling |
| `@parseJson(text)` | parse JSON while compiling |
| `@typeToString(Type)`, `@typeToSchema(Type)` | reflection (use `Type<value>` to get the type of a value) |
| `@import("https://...")` | import a [remote module](/architecture/dependencies) |
| `@test "name" { }` | a [test](/architecture/testing) run while compiling |

## Compile-time function calls

Any pure function can be called at compile time, when its arguments are known at compile time. Prefix the call with `@`:

```fa
function add = (x: Integer, y: Integer): Integer => x + y

let result = @add(2, 4) -- compiled to `let result = 6`
```

The `@` is written on every call that the compiler evaluates, including inside a [type function](/types/type-functions) or the value of a `static`: it marks where compile-time evaluation starts. The function it calls, and every function that one calls, runs as usual, without `@`.

The arguments must be literals, static values, compile-time parameters, other compile-time operations, or the local values of a type function:

```fa
let count = readInput()
let total = @add(count, 4) -- error: `count` is not known at compile time
```

A function called at compile time must be [**pure**](/functions/defining-functions#pure-functions): it doesn't mutate or invalidate anything but its own local values, has no requirement, isn't `async`, doesn't read a module-level `mutable` or reactive value, and doesn't reach a primitive with outside effects (input and output, the clock, randomness, `console`). The compiler infers it: purity is never written. One exception to purity is allowed at compile time: the function may throw, and the error becomes a compile error at the `@` call.

### Determinism

Compile-time evaluation is **deterministic**: given the same inputs, it always produces the same output, so a build can be reproduced and cached. Reading the clock, the network or a random source is not allowed at compile time.

`String.compareLocale` is also a runtime-only operation: it depends on the locale data of the machine running the program. Call it at runtime rather than in a `static` or an `@` call.

```fa
use { now } = std.time

static timestamp = @now() -- error: `now` is not deterministic
static settingsText = @readFile("settings.json") -- fine: files are build inputs
```

Reading the file system is allowed, because the files read become part of the build inputs (see [the file system](#the-file-system)). The compile-time operations of the table above are provided by the compiler, and are the only way to read files at compile time.

Compile-time arithmetic follows the rules of the target being compiled: a value that fits on `native` may overflow when compiling for JavaScript, and an overflow at compile time is a compile error.

Compile-time evaluation is bounded, so that an endless computation is reported instead of hanging the build: an `@` call that runs more than 100 million evaluation steps or uses more than 256 MB of memory is a compile error, and so are more than 256 nested applications of [type functions](/types/type-functions).

## Compile-time parameters

A function or a type can take compile-time values as [generic parameters](/types/generics#type-parameters-and-compile-time-value-parameters). A lowercase generic parameter receives a value known at compile time, and is used like any other value:

```fa
function repeatText = <count: Integer<min = 0>>(value: String): String => {
	mutable result = ""
	@for 0..count {
		result += value
	}
	result
}

repeatText<3>("ab") -- the loop is unrolled into `"ab" + "ab" + "ab"`
```

There is no `@` on compile-time parameters: being a generic parameter is what makes them compile-time values.

A compile-time parameter is always given explicitly, never inferred from the arguments of the call: in `repeatText<3>("ab")`, `<3>` gives `count`, then `("ab")` calls the function. See [explicit parameters](/types/generics#explicit-parameters).

The body of an `@for` or an `@if` that depends on a compile-time parameter is checked once the parameter is known, for each value it receives: an error is reported at the call that gave the parameter.

## Compile-time conditions

`@if` evaluates its condition at compile time, and keeps only the matching branch in the output. The other branch is parsed, but neither checked nor compiled, so it may use features that don't exist on the current target:

```fa
@if target is "native" {
	for parallel item, index in items {
		results[index] = process(item)
	}
} @else {
	for item, index in items {
		results[index] = process(item)
	}
}
```

The condition must be a `Boolean` known at compile time: static values, literals and compile-time operations.

Since each branch is only checked for the targets that keep it, `fa check` checks the code for every target of the configuration.

## Compile-time loops

`@for` unrolls a loop at compile time. Its iterable must be a compile-time value. Over a [key set](/types/fields#key-sets-as-compile-time-parameters), it goes through its keys.

```fa
static fruits = ["apple", "banana", "cherry"]

@for fruit in fruits {
	console.log("Fruit: {fruit}")
}
```

produces exactly:

```fa
console.log("Fruit: apple")
console.log("Fruit: banana")
console.log("Fruit: cherry")
```

At the top level of a module, an `@for` can only contain declarations without a name, which would otherwise be declared once per item: `watch` blocks, and tests in a test file (whose names can interpolate the item).

## Reflection

Types can be turned into values:

```fa
let x = 12
console.log(@typeToString(Type<x>)) -- "Integer"

let name = @typeToString(User) -- "User"
let schema = @typeToSchema(User)
```

A schema can be turned back into a type, which lets you transform types programmatically. See [types at runtime](/types/types-at-runtime).

## The file system

The compiler can read files and directories while compiling:

```fa
static version = @readFile("../version.txt")
static routes = @readDirectory("../routes")
static settings = @parseJson(@readFile("settings.json"))

console.log(settings.port)
```

`@readFile` returns the content of a file as a `String`. `@readDirectory` returns the entries of a folder, sorted by name, as an `Array<DirectoryEntry>`, where a `DirectoryEntry` is a `{ name: String, path: String, isDirectory: Boolean }`. Paths are relative to the folder of the file that reads them.

Every file read this way becomes a **build dependency**: change it, and the code that depends on it is recompiled.

Combined with `@if`, a static value lets the compiler remove dead code:

```fa
@if settings.debug {
	enableVerboseLogs() -- removed entirely from the output when `debug` is false
}
```

## How it works

Fa ships an interpreter that executes Fa at compile time. It supports a subset of the standard library, including deterministic I/O, and runs Fa with the same semantics as every target, which is why compile-time code needs no separate dialect. It is only used while compiling: `fa run` compiles the program for `bun`, like `fa build`, and runs it.

It also runs the [configuration files](/architecture/configuration#how-configuration-files-are-executed) before the package is compiled, with the same rules: they are deterministic, can read the files of the package, which become build inputs, and only reach remote code through `@import` and the lockfile.

## What it is good for

- **Configuration**: read settings at build time, and let dead branches be removed.
- **API clients**: generate typed clients from a schema file.
- **Routing**: build a route table from the file system.
- **Query validation**: check SQL or GraphQL queries against a schema while compiling.
- **Component variants**: expand a set of variants into specialized code.
- **Domain-specific languages**: parse a small language into Fa code.

:::caution
Compile-time work happens on every build. Keep it proportionate: a type function that recurses deeply, or a `@for` over thousands of items, will be felt in your compile times.
:::

:::implementation
## Implementation details

- An evaluation step is one evaluated node of the checked tree; the limit of 100 million steps is counted per `@` call.
- Fa function calls have a default depth limit of 10 000 (`META003`), including calls from defaults and callbacks. Type-function applications have their own limit of 256 (`META004`). Heap continuations make these limits independent of the native stack size, thread count and address-space limits. Frames and continuations count toward the 256 MB evaluation heap budget; preparation and resumption do not add evaluation steps.
- The memory of an `@` call is checked before building a large value at once (a text, the items of a range), and every 1 024 steps against the heap of the compiler, which counts its allocations: the compiler stays below 512 MB. A value is copied on write: changing a collection that nothing else shares doesn't copy it.
- The branch of `@if` that is not taken is parsed but not analyzed, like Rust's `cfg`. The analysis is parameterized by the target: `fa check` analyzes each configured target, sharing the analysis of every declaration that doesn't read `target` or `configuration`.
- A named declaration inside a module-level `@for` is the error `MOD018`.
:::
