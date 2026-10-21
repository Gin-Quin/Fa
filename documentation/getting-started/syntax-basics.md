# Syntax basics

This page describes the mechanical rules of Fa source code: how files are laid out, how comments work, and how statements are separated. Everything else in this documentation assumes these rules.

## Source files

A Fa source file has the `.fa` extension and is encoded in UTF-8.

## Statements and newlines

Statements are separated by **newlines**. Fa has no statement terminator — there are no semicolons.

```fa
let a = 1
let b = 2
```

An expression may span several lines when it is unambiguously incomplete: after an opening bracket (`(`, `[`, `{`), after a binary operator, or after a comma.

```fa
let total =
  firstValue +
  secondValue

let point = {
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

Fa has three kinds of comments.

```fa
-- an inline comment, running to the end of the line

---
A block comment.
It runs until the next `---`.
---
```

Any comment placed **immediately before a declaration** is a documentation comment. There is no separate syntax for documentation.

```fa
-- Adds two integers together.
-- Returns their sum.
function add = (a: Int, b: Int): Int => a + b
```

To write a comment that is *not* documentation, leave a blank line between the comment and the declaration.

```fa
-- This note is for maintainers, not for the API documentation.

function add = (a: Int, b: Int): Int => a + b
```

:::tip
`--` is the comment marker, so `//` is free to mean **integer division**. See [arithmetic](../operations/arithmetic).
:::

Because `-` also means subtraction, always put a space around a binary minus. `a - b` subtracts; `a --b` is `a` followed by a comment.

## Identifiers

An identifier starts with a letter or `_`, followed by letters, digits, or `_`.

Two prefixes are significant:

- `#` marks a **private** member of an object or namespace: `#internalCount`.
- `@` marks **compile-time** evaluation: `@readFile("version.txt")`.

## Naming conventions

Fa's compiler does not enforce naming, but the whole standard library and this documentation follow these conventions, and you should too:

| Kind | Convention | Example |
| --- | --- | --- |
| Values, functions, methods | `camelCase` | `userCount`, `loadScore` |
| Types, type functions, union members | `PascalCase` | `User`, `Partial`, `ApiError` |
| Enum and field members | `camelCase` | `read`, `write` |
| Files | mirror what they export | `User.fa`, `loadScore.fa` |

:::caution
Naming is a **convention only**. Whether something is a type or a value is decided by its declaration and its type annotation, never by its capitalization. See [generics](../types/generics).
:::

## Declarations

Every declaration in Fa has the same shape:

```
<keyword> <name> [: <type>] = <value>
```

The keywords are:

| Keyword | Declares |
| --- | --- |
| `let` | an immutable value |
| `mutable` | a mutable value |
| `use` | a reference to an existing value (no copy) |
| `static` | a compile-time-only value |
| `function` | an immutable, hoisted function |
| `type` | a type |
| `namespace` | a namespace |
| `reactive` | a mutable value that can be observed |
| `derived` | a read-only value computed from reactive values |
| `export` | the single exported entity of a file |

There is no way to introduce a name without one of these keywords. A bare `name = value` is always an **assignment** to an existing mutable value, never a declaration.

```fa
mutable count = 0   -- declaration
count = 1           -- assignment
total = 1           -- error: `total` is not declared
```

## Blocks and expressions

Braces introduce a block. Most blocks are also expressions: the value of a block is its **last expression**.

```fa
let value = do {
  let a = 2
  let b = 3
  a * b        -- the block evaluates to 6
}
```

`if`, `when`, `do` and `loop` can all be used as expressions. `for` and `while` cannot — see [loops](../control-flow/loops).

## Reserved keywords

```
and       async     await     break     continue  derived   do
else      enum      errors    export    false     fields    for
function  if         is       let       loop      modulo    mutable
namespace none      not       reactive  return    static    true
type      union     use       when      while     yield
```

## What the compiler enforces

Three rules are worth knowing up front, because they shape every example in this documentation:

1. **Values are immutable by default.** `let` gives deep immutability; `mutable` is opt-in.
2. **Errors are values.** There is no `throw`, no `try`, and no `catch`. See [errors](../values/errors).
3. **Memory is managed by the compiler.** Objects live on the stack, collections own the heap. See [memory management](../advanced/memory-management).
