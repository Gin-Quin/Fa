# Fa for TypeScript developers

Fa will feel familiar: structural typing, arrow functions, unions, generics. Its syntax sits between TypeScript and Rust. The differences are where Fa does things TypeScript cannot, because TypeScript has to remain a superset of JavaScript.

Fa is designed for AI agents as much as for humans: types are 100% safe — if the code compiles, it works — and everything a function does (the errors it throws, the values it requires, the data it mutates) is written in its signature, like with [Effect](https://effect.website), but in the syntax.

## Syntax at a glance

| TypeScript | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const x = 1` | `let x = 1` |
| `let x = 1` | `mutable x = 1` |
| `const f = (a: number) => a` | `function f = (a: Integer) => a` |
| `` `Hello ${name}` `` | `"Hello {name}"` |
| `number` | `Integer` or `Float` |
| `Number(count)` | `Float(count)` |
| `Array<string>` | `Array(String)` |
| `new Set([1, 2])` | `Set.from([1, 2])` |
| `x?.y` | `x?.y` |
| `x ?? y` | `x ?? y` (optionals only) |
| `x === y` | `x == y` (deep structural equality) |
| `x % y` | `x %% y` |
| `!x` | `not x` |
| `x && y`, `x \|\| y` | `x and y`, `x or y` |
| `switch` | `when` |
| `for (const x of xs)` | `for x in xs` |
| `for (const [k, v] of map)` | `for (k, v) in map` |
| `undefined` / `null` | `none` |
| `throw new Error()` | `throw ApiError.Network { message = "..." }` |
| `try { ... } catch (e) { ... }` | `try call()` to propagate, `call() catch error => ...` to handle |
| `Promise.all([a, b])` | `Task.all([a, b])` |
| `import { a } from "./b"` | nothing — or `use { a } = b` |
| `export const a = 1` | `export = 1` |

## Generics use parentheses

```ts
// TypeScript
function first<T>(items: T[]): T | undefined { ... }
```

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)
```

Generic parameters come right after the name. They are inferred from the arguments and the expected type; when you need to write them, they are applied first:

```fa
first([1, 2, 3])
first(Float)([1, 2, 3])
```

A constraint is a shape, like `T extends { id: number }`:

```fa
function readId(Item: { id: Integer }) = (item: Item): Integer => item.id
```

See [generics](/types/generics).

## Errors are typed values

Fa has `throw`, `try` and `catch`, but an error is not an exception: it is a typed value, and a function declares the errors it can fail with in its signature, using `throws`.

```ts
// TypeScript: nothing in the signature tells you this can throw
function loadScore(): number {
	throw new Error("boom")
}
```

```fa
type ApiError = Errors {
	Network = { message: String }
	Timeout
}

function loadScore = (): Integer throws ApiError => {
	throw ApiError.Network { message = "boom" }
}

let score = try loadScore()                -- propagate to the caller
let score = loadScore() catch error => 0   -- handle it here
```

`try` is not a block: it marks one call whose errors propagate to the caller, which must declare them too. `catch` handles the errors of one expression and gives a replacement value, and `when error is { ... }` handles each error differently, exhaustively. `??` only applies to optionals, never to errors. See [errors](/values/errors).

## `none`, not `null` and `undefined`

Fa has one absent value. `T?` is `T | None`.

```fa
let name: String? = none

if name is not none {
	console.log(name.length) -- narrowed to String
}
```

## No truthiness

Conditions only accept a `Boolean`. `if (name)` does not compile: write `if name is not none`, or `if name.length > 0`. `and`, `or` and `not` only take booleans as well, so `a || "default"` becomes `a ?? "default"` on an optional.

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

To share, create an alias:

```fa
use b = a
b.x = 2
console.log(a.x) -- 2
```

Function parameters and loop bindings are already references, so this only matters for explicit assignments. Because identity is not observable, `==` compares values deeply. See [memory management](/advanced/memory-management).

## Immutability is real and deep

`const` in TypeScript stops reassignment only. `let` in Fa stops everything:

```fa
let point = { x = 1 }
point.x = 2 -- error

mutable point = { x = 1 }
point.x = 2 -- fine
```

A function that modifies a parameter says so in its signature, with `mutates`:

```fa
function rename = (user: User, name: String) mutates user => {
	user.name = name
}
```

## Objects and methods

There are no classes. Methods are declared inside a type, and objects are built by naming their fields:

```fa
type User = {
	name: String
	age: Integer = 0

	function greet = (self): String => "Hello, {self.name}"
}

let user = User { name = "Ada", age = 30 }
let grace: User = { name = "Grace" } -- typed by context
```

Types compose with the spread: `type Admin = { ...User, permissions: Array(String) }`.

## Numbers are concrete

There is no single `number`. `Integer` and `Float` are different types, with no implicit conversion, and integers can carry a range:

```fa
let count: Integer = 42
let ratio: Float = 0.5
let percent: Integer(min = 0, max = 100) = 50

let scaled = Float(count) * ratio
```

`/` always returns a `Float`, and `//` is floor division, which returns an `Integer`. The range is checked at compile time, and lets the compiler remove bounds and division-by-zero checks. See [numbers](/values/numbers).

## No imports

Files see their siblings and the package root automatically. Folders are namespaces.

```fa
-- no import needed
console.log(myOtherFile.doSomething())

-- create aliases of names from a namespace
use { formatDate } = utils
```

Each file exports exactly one thing, named after the file. See [modules](/architecture/modules).

## What TypeScript cannot do

- **Metaprogramming** — run Fa at compile time: `@readFile`, `@for`, compile-time functions
- **Type reflection** — `@typeToSchema(User)` gives you a type's shape as a value
- **Built-in reactivity** — `reactive` and `derived` values, with no framework
- **True immutability** — deep, enforced, not just `readonly`
- **Native and WebAssembly output** — optimized binaries, no runtime, no garbage collector
- **Colourless async** — an `async` function can also be called synchronously
- **Function composition** — `a then b then c`, and slicing a composition (`migrate[1..]`), for plugins, middleware and migrations
- **Fields** — `Pick(User, { name, address { city } })` for GraphQL-like selection
- **Typed errors and requirements** — `throws` and `requires` in the signature, checked at compile time

Other things you will enjoy:

- **Type functions** — types computed from other types
- **Additional collections** — `Bag`, `Columnar`, and growing collections with stable references
- **Aliases** — `use` names a value without copying it
- **No `let` vs `const` war** — `let` is immutable, `mutable` is mutable

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `x % 2` | `x %% 2` (floor-based: `-7 %% 3 == 2`); `%` is the percentage suffix (`50%`) |
| `//` comments | `--`; `//` is floor division |
| Backtick templates | every string interpolates with `{}` |
| Single-quoted strings | double quotes only |
| Semicolons | none |
| `arr[i]` always works | `throws IndexError` unless provably in bounds; `.get(i)` returns `T?` |
| `map[key]` returns `undefined` | `throws KeyNotFound` unless the key is proven; `.get(key)` returns `V?` |
| Truthiness everywhere | conditions only accept a `Boolean`; use `is none` / `is not none` |
| `a === b` on objects (identity) | `==` compares structure; identity is not observable |
| `try` around a block | `try` or `catch` on each call that can fail |
| `value as User` (type assertion) | does not exist: narrow with `if value is User`; `as` only renames |
| `new User("Ada")` | `User { name = "Ada" }` |
| Spreading to copy (`{...a}`) | assignment already copies |
| `const f = () => ...` | `function f = () => ...`; lambdas are only written where a value is expected |
