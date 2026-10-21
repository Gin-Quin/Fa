# Fa for TypeScript developers

Fa will feel familiar: structural typing, arrow functions, unions, generics. The differences are where Fa does things TypeScript cannot, because TypeScript has to remain a superset of JavaScript.

## Syntax at a glance

| TypeScript | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const x = 1` | `let x = 1` |
| `let x = 1` | `mutable x = 1` |
| `const f = (a: number) => a` | `function f = (a: Int) => a` |
| `` `Hello ${name}` `` | `"Hello {name}"` |
| `Array<string>` | `Array(String)` |
| `x?.y` | `x?.y` |
| `x ?? y` | `x ?? y` |
| `x === y` | `x == y` |
| `x % y` | `x %% y` |
| `!x` | `not x` |
| `x && y`, `x \|\| y` | `x and y`, `x or y` |
| `switch` | `when` |
| `for (const x of xs)` | `for xs >> x` |
| `undefined` / `null` | `none` |
| `throw` / `try` / `catch` | error unions and `!` |
| `import { a } from "./b"` | nothing — or `b >> use { a }` |
| `export const a = 1` | `export = 1` |

## Generics use parentheses

```ts
// TypeScript
function first<T>(items: T[]): T | undefined { ... }
```

```fa
function first = (Item: Type, items: Array(Item)): Item? => ...
```

A type parameter is a parameter whose type is `Type`. It is inferred, or passed by name — never positionally:

```fa
first([1, 2, 3])
first(Item = Decimal, [1, 2, 3])
```

See [generics](../types/generics).

## Errors are values

There is no `throw`. A function that can fail says so:

```ts
// TypeScript: nothing in the signature tells you this can throw
function loadScore(): number {
	throw new Error("boom")
}
```

```fa
function loadScore = (): Int | ApiError => ApiError.Network { message = "boom" }

let score = loadScore()!  -- propagate
let score = loadScore() ?? 0
```

`!` is not a non-null assertion — it propagates the error to the caller. See [errors](../values/errors).

## `none`, not `null` and `undefined`

Fa has one absent value. `T?` is `T | None`.

```fa
let name: String? = none

if name is not none {
  console.log(name.length) -- narrowed to String
}
```

## Assignment copies

This is the biggest change. TypeScript objects are references; Fa objects are values on the stack.

```ts
const a = { x: 1 }
const b = a
b.x = 2
console.log(a.x) // 2 — same object
```

```fa
mutable a = { x = 1 }
mutable b = a
b.x = 2
console.log(a.x) -- 1 — b is a copy
```

To share, take a reference:

```fa
use b = a
b.x = 2
console.log(a.x) -- 2
```

Function parameters and loop bindings are already references, so this only matters for explicit assignments. See [memory management](../advanced/memory-management).

## Immutability is real and deep

`const` in TypeScript stops reassignment only. `let` in Fa stops everything:

```fa
let point = { x = 1 }
point.x = 2 -- error

mutable point = { x = 1 }
point.x = 2 -- fine
```

## Numbers are concrete

There is no single `number`. Integers and decimals are different types, and integers can carry a range:

```fa
let count: Int = 42
let ratio: Decimal = 0.5
let percent: Int(min = 0, max = 100) = 50
```

The range is checked at compile time, and lets the compiler remove bounds and division-by-zero checks. See [numbers](../values/numbers).

## No imports

Files see their siblings and the package root automatically. Folders are namespaces.

```fa
-- no import needed
console.log(myOtherFile.doSomething())

-- pull names out of a namespace
utils >> use { formatDate }
```

Each file exports exactly one thing, named after the file. See [modules](../architecture/modules).

## What TypeScript cannot do

- **Metaprogramming** — run Fa at compile time: `@readFile`, `@for`, compile-time functions
- **Type reflection** — `@typeToSchema(User)` gives you a type's shape as a value
- **Built-in reactivity** — `reactive`, `derived`, `effect`, with no framework
- **True immutability** — deep, enforced, not just `readonly`
- **Native and WebAssembly output** — no runtime, no garbage collector
- **Colourless async** — an `async` function is callable from anywhere
- **Composition slicing** — `pipeline[1..]` reuses part of a composition
- **Field sets** — `@{ foo, nested { bar } }` for GraphQL-like selection
- **Typed contexts** — implicit parameters, checked at compile time

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `x % 2` | `x %% 2` — `%` builds a percentage |
| `//` comments | `--`; `//` is integer division |
| Backtick templates | every string interpolates with `{}` |
| Semicolons | none |
| `arr[i]` always works | returns `T \| IndexError` unless provably in bounds |
| `map[key]` returns `undefined` | returns `V \| KeyNotFound`; use `.get()` for `V?` |
| Truthiness everywhere | works, but prefer `is none` when `0` or `""` are valid |
| Spreading to copy (`{...a}`) | assignment already copies |
