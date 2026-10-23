# Syntax basics

This page describes the mechanical rules of Fa source code: how files are laid out, how comments work, how statements are separated and how names are written. Everything else in this documentation assumes these rules.

## Source files

A Fa source file has the `.fa` extension and is encoded in UTF-8. Code is indented with tabs.

## Statements and newlines

Statements are separated by **newlines**. Fa has no statement terminator: there are no semicolons.

```fa
let a = 1
let b = 2
```

An expression may span several lines when it is unambiguously incomplete: after an opening bracket (`(`, `[`, `{`), after a binary operator, or after a comma.

```fa
let total =
	firstValue +
	secondValue

let point: Point = {
	x = 1
	y = 2
}

let sum = add(
	1
	2
)
```

Inside brackets, both newlines and commas separate items. Pick one style per literal and stay with it.

```fa
let a = [1, 2, 3]

let b = [
	1
	2
	3
]
```

## Comments

Fa has two kinds of comments:

```fa
-- an inline comment, running to the end of the line

---
A block comment.
It runs until the closing marker.
---
```

A block comment can also fit on a single line: `--- like this ---`.

### Documentation comments

A comment placed **immediately above a declaration**, with no blank line in between, is the documentation of that declaration. It is attached to it and available to the compiler, the language server and the tooling. There is no separate syntax for documentation.

```fa
---
Adds two integers together.
Returns their sum.
---
function add = (a: Integer, b: Integer): Integer => a + b

-- The number of retries before giving up.
let maximumRetries = 3
```

To write a comment that is *not* documentation, leave a blank line between the comment and the declaration:

```fa
-- This note is for maintainers, not for the API documentation.

function add = (a: Integer, b: Integer): Integer => a + b
```

:::tip
`--` is the comment marker, so `//` is free to mean **floor division**. See [arithmetic](/operations/arithmetic).
:::

Because `-` also means subtraction, always put spaces around a binary minus: `a - b` subtracts, while `a --b` is `a` followed by a comment.

## Strings

Strings are written with double quotes only. Values are interpolated with braces, and literal braces are doubled:

```fa
let name = "Ada"
let greeting = "Hello {name}" -- "Hello Ada"
let braces = "{{ not interpolated }}"
```

See [strings](/values/strings).

## Identifiers

An identifier starts with a letter or `_`, followed by letters, digits or `_`.

Two prefixes are significant:

- `#` marks a **private** member: `#internalCount`. See [modules](/architecture/modules).
- `@` marks a **compile-time operation**: `@readFile("version.txt")`. See [metaprogramming](/advanced/metaprogramming).

## Naming conventions

The standard library and this documentation follow these conventions, and you should too:

| Kind | Convention | Example |
| --- | --- | --- |
| Values, functions, methods, fields | `camelCase` | `userCount`, `loadScore` |
| Types | `PascalCase` | `User`, `ApiError` |
| Members of an `Enum` or of `Fields` | `camelCase` | `red`, `read` |
| Members of a `Union` or of `Errors` | `PascalCase` | `Circle`, `NotFound` |
| Type parameters | `PascalCase` | `Item`, `NumberType` |
| Compile-time value parameters | `camelCase` | `rows`, `min` |
| Files | mirror what they export | `User.fa`, `loadScore.fa` |

Outside of generic parameter lists, naming is a **convention only**: whether something is a type or a value is decided by its declaration.

In a **generic parameter list**, capitalization carries meaning: a capitalized parameter is a type parameter, and a lowercase parameter is a compile-time value parameter.

```fa
type Matrix(Element, rows: Integer, columns: Integer) = {
	values: Array(Element)
}

let identity: Matrix(Float, 4, 4) = ...
let percentage: Integer(min = 0, max = 100) = 50
```

See [generics](/types/generics).

## Declarations

Every declaration in Fa has the same shape:

```
<keyword> <name>[(<generic parameters>)][: <type>] = <value>
```

The declaration keywords are:

| Keyword | Declares |
| --- | --- |
| `let` | a deeply immutable value |
| `mutable` | a mutable value |
| `use` | an alias of an existing value (no copy) |
| `static` | a compile-time value |
| `function` | a function (`async function` for an asynchronous one) |
| `type` | a type |
| `namespace` | a namespace |
| `reactive` | a mutable value that can be observed |
| `derived` | a read-only value computed from reactive values |
| `export` | the single exported entity of a file |

```fa
let answer: Integer = 42
mutable count = 0
use street = user.address.street
static version = @readFile("version.txt")
function double = (value: Integer): Integer => value * 2
type Point(NumberType: Integer | Float) = { x: NumberType, y: NumberType }
```

There is no way to introduce a name without one of these keywords. A bare `name = value` is always an **assignment** to an existing mutable value, never a declaration:

```fa
mutable count = 0 -- declaration
count = 1 -- assignment
total = 1 -- error: `total` is not declared
```

Shadowing a name of an enclosing scope is allowed; declaring the same name twice in the same scope is an error.

See [declaring values](/values/declaring-values) and [defining functions](/functions/defining-functions).

## Blocks and expressions

Braces introduce a block. Most blocks are also expressions: the value of a block is its **last expression**.

```fa
let value = do {
	let a = 2
	let b = 3
	a * b -- the block evaluates to 6
}
```

`if` (with an `else`), `when`, `do` and `loop` can all be used as expressions. `for` and `while` cannot. See [conditionals](/control-flow/conditionals) and [loops](/control-flow/loops).

Braces are used for blocks and for objects only: collections are written with brackets (`[1, 2, 3]`).

## Reserved keywords

```
and          as           assert       async        await
break        catch        continue     derived      do
else         export       false        for          function
if           in           invalidates  is           let
loop         mutable      mutates      namespace    none
not          or           parallel     provide      reactive
readonly     requires     return       self         start
static       test         then         throw        throws
true         try          type         use          watch
when         while        with
```

## What the compiler enforces

A few rules shape every example in this documentation:

1. **Values are immutable by default.** `let` gives deep immutability; `mutable` is opt-in, and a function that modifies a parameter declares it with `mutates`.
2. **There is no truthiness.** Conditions only accept `Boolean` values: optionals are checked with `is none` and `is not none`.
3. **Errors are part of the signature.** A function that can fail declares it with `throws`; callers propagate with `try` or handle with `catch`. See [errors](/values/errors).
4. **Memory is managed by the compiler.** Values are destroyed at the end of their scope, and collections own the heap. See [memory management](/advanced/memory-management).
