# Syntax basics

This page describes the mechanical rules of Fa source code: how files are laid out, how comments work, how statements are separated and how names are written. Everything else in this documentation assumes these rules.

## Source files

A Fa source file has the `.fa` extension and is encoded in UTF-8. Code is indented with tabs. Indentation is a style rule, enforced by `fa format`: it never changes the meaning of the code.

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

A line that starts with `.`, `?.` or a binary operator other than `-` continues the expression of the previous line, which is how long method chains are written. A line that starts with `-`, `(` or `[` always starts a new statement, since these can also begin an expression.

```fa
let names = users
	.filter((user) => user.isActive)
	.map((user) => user.name)

let isAllowed = isAdmin
	or isOwner
```

`else` is written on the same line as the `}` that closes the `if` block: `} else {`. An `else` at the start of a line is an error.

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

Because `-` also means subtraction, always put spaces around a binary minus: `a - b` subtracts, while `a --b` is `a` followed by a comment. A minus glued to a number is a negative literal only where an expression starts (`f(-1)`, `x = -1`); after a value, it is a subtraction (`a-1` is `a - 1`, which the formatter spaces out).

## Strings

Strings are written with double quotes only. Values are interpolated with braces, and literal braces are doubled:

```fa
let name = "Ada"
let greeting = "Hello {name}" -- "Hello Ada"
let braces = "{{ not interpolated }}"
```

See [strings](/values/strings).

## Identifiers

An identifier starts with an ASCII letter or `_`, followed by ASCII letters, digits or `_`: `_1` is a name, not a number. Keywords can't be used as identifiers. A name starting with `_` marks a value that is unused on purpose (see [unused values](/values/declaring-values#unused-values)).

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
type Matrix<Element, rows: Integer, columns: Integer> = {
	values: Array<Element>
}

let identity: Matrix<Float, 4, 4> = ...
let percentage: Integer<min = 0, max = 100> = 50
```

See [generics](/types/generics).

## Declarations

Every declaration in Fa has the same shape:

```
<keyword> <name>[: <type>] = <value>
```

A type declares its generic parameters between angle brackets after its name, and a function at the start of its signature: `type Pair<Left, Right> = ...`, `function first = <Item>(items: Array<Item>) => ...`. See [angle brackets](#angle-brackets).

The declaration keywords are:

| Keyword | Declares |
| --- | --- |
| `let` | a deeply immutable value |
| `mutable` | a mutable value |
| `use` | an alias of an existing value (no copy) |
| `mutable use` | an alias that can later designate another value, with `reuse` |
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
type Point<NumberType: Integer | Float> = { x: NumberType, y: NumberType }
```

There is no way to introduce a name without one of these keywords. A bare `name = value` is always an **assignment** to an existing mutable value, never a declaration:

```fa
mutable count = 0 -- declaration
count = 1 -- assignment
total = 1 -- error: `total` is not declared
```

Shadowing a name of an enclosing scope is allowed; declaring the same name twice in the same scope is an error, except for a reference invalidated by a mutation, which is declared again (see [shadowing](/values/declaring-values#shadowing)).

A few statements look like declarations but don't follow this shape: `reuse best = player` moves a [`mutable use`](/values/declaring-values#changing-what-a-reference-designates) reference, and the export of a file is written `export = value`, `export mutable: Type = value`, `export function = ...`, `export type = ...` or `export namespace = ...`, since it is named after the file (see [modules](/architecture/modules)).

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

## Braces

Braces are used for blocks and for objects only: collections are written with brackets (`[1, 2, 3]`).

Where a `{` appears decides what it opens:

- after `=>`, `if`, `else`, `do`, `loop`, `for`, `while`, `watch`, `test` or `namespace`, it opens a **block** (or the body of that construct),
- after a type name, it opens an **object literal** of that type: `Point { x = 1, y = 2 }`,
- everywhere else in an expression, it opens an **object literal**: `let point: Point = { x = 1, y = 2 }`.

The body of a lambda, of a `catch` handler or of a `when` branch, written after `=>`, is therefore always a block when it starts with `{`. To return an object literal from it, write the type name:

```fa
let points = values.map((value) => Point { x = value, y = 0 })

let origin = readPoint() catch error => Point { x = 0, y = 0 }
```

Inside an object literal, a field written alone is a shorthand: `{ id }` means `{ id = id }`. Where a [key set](/types/fields#key-sets) is expected, as in `Pick<User, { name, email }>`, `{ name, email }` is a key set. See [objects](/values/objects#field-shorthand).

## Angle brackets

Generic parameters and arguments are written between angle brackets: `Array<Integer>`, `first<Float>([1, 2])`. Since `<` and `>` are also comparison operators, the tokenizer tells them apart with these rules:

1. `<` followed by whitespace (a space, a tab or a line break) is the comparison operator. Any other `<` opens a generic list, whatever comes before it.
2. Inside an open generic list, `>` always closes it. Outside one, `>` is the comparison operator.
3. A comparison inside a generic list goes in parentheses: `Choose<(count > 255), UInt16, UInt8>`.
4. `<=`, `>=`, `=>` and `->` are tokens of their own, so they are never generic brackets: in `Pool<people->bestFriend>`, `->` doesn't close the list.

```fa
if count < limit { ... } -- a comparison
let names: Array<String> = [] -- a generic list
if count<limit { ... } -- error: `<limit` opens a generic list; write `count < limit`
```

See [angle brackets and comparisons](/types/generics#angle-brackets-and-comparisons).

## Reserved keywords

```
and          as           assert       async        await
break        catch        continue     derived      do
else         export       false        for          function
if           in           invalidates  is           let
loop         mutable      mutates      namespace    none
not          or           parallel     provide      reactive
readonly     requires     return       reuse        start
static       test         then         throw        throws
true         try          type         use          watch
when         while        with
```

`skip` and `only` are keywords only after `test`. `self` (the receiver of a method) and `Self` (the type being declared) are special names, not keywords.

## What the compiler enforces

A few rules shape every example in this documentation:

1. **Values are immutable by default.** `let` gives deep immutability; `mutable` is opt-in, and a function that modifies a parameter declares it with `mutates`.
2. **There is no truthiness.** Conditions only accept `Boolean` values: optionals are checked with `is none` and `is not none`.
3. **Errors are part of the signature.** A function that can fail declares it with `throws`; callers propagate with `try` or handle with `catch`. See [errors](/values/errors).
4. **Memory is managed by the compiler.** Values are destroyed at the end of their scope, and collections own the heap. See [memory management](/advanced/memory-management).
