# Language reference

The whole language on one page. Each section links to the page that explains it.

## Lexical

| | |
| --- | --- |
| Inline comment | `-- comment` |
| Block comment | `--- ... ---` |
| Documentation | any comment directly above a declaration |
| Statement separator | newline (no semicolons) |
| Private prefix | `#name` |
| Compile-time prefix | `@name` |
| Naming | `camelCase` values, `PascalCase` types — convention only |

See [syntax basics](../getting-started/syntax-basics).

## Declarations

Every declaration is `<keyword> <name>[: <type>] = <value>`.

```fa
let value = 1              -- immutable
mutable value = 1          -- mutable
use alias = collection[0]  -- reference, no copy
static value = 1           -- compile-time only
function add = (a: Int, b: Int): Int => a + b  -- immutable + hoisted
type MyType = { foo: Int }
namespace utils = { ... }
reactive count = 0
derived doubled = count * 2
export = value
```

A bare `name = value` is an assignment, never a declaration.

See [declaring values](../values/declaring-values).

## Types

```fa
Boolean  String  Character
Int  UInt  Int8..Int128  UInt8..UInt128  BigInt
Decimal  Float32  Float64  Percentage  Byte
None  Any  Object  Never  Type  Error
Array(T)  Set(T)  Map(K, V)  Bag(T)  Columnar(T)
GrowingArray(T)  GrowingSet(T)  GrowingMap(K, V)  GrowingBag(T)  GrowingColumnar(T)
Tuple(A, B)  Range  Optional(T)  T?
(A, B) => C
```

Type constructors:

```fa
type Color = Enum { red, green, blue }
type Permissions = Fields { read, write, admin }
type Shape = Union { Circle = { radius: Decimal }, Rect = { w: Decimal, h: Decimal } }
type ApiError = Errors { Timeout = { after: Int }, Network = { message: String } }
```

Type operators:

| Operator | Meaning |
| --- | --- |
| `\|` | union |
| `&` | intersection |
| `-` | difference |
| `...` | spread |
| `>>` | pick fields, with a key set |
| `?` | optional (`T \| None`) |

See [declaring types](../types/declaring-types), [type operations](../types/operations).

## Generics

Type application is a call. A type parameter is a parameter annotated `Type`.

```fa
Array(Int)
Map(String, User)
Int(min = 0, max = 200)

type Pair = (First: Type, Second: Type) => { first: First, second: Second }

function first = (Item: Type, items: Array(Item)): Item? => ...

first([1, 2, 3])                  -- inferred
first(Item = Decimal, [1, 2, 3])  -- explicit: type arguments are always named
```

- Constraints: `(Item: Type({ bar: Int }), ...)`
- Compile-time values: `(@count: UInt, ...)` — positional, never inferred
- Calling a **type function** yields a type; calling a **type** constructs a value

See [generics](../types/generics).

## Operators

### Arithmetic

| | | | |
| --- | --- | --- | --- |
| `+` add | `-` subtract | `*` multiply | `**` power |
| `/` divide | `//` integer divide | `%%` remainder | `50%` percentage suffix |

Compound: `+=` `-=` `*=` `/=` `//=` `%%=` `**=`

Bitwise are methods: `.bitAnd()` `.bitOr()` `.bitXor()` `.bitNot()` `.shiftLeft()` `.shiftRight()`

### Logical

| | |
| --- | --- |
| `==` `!=` | value equality (deep, structural) |
| `is` `is not` | pattern / type / literal match, narrows |
| `is in` `is not in` | membership |
| `<` `<=` `>` `>=` | ordering, chainable (`0 < a < 10`) |
| `and` `or` `not` | boolean logic, short-circuiting |
| `??` | fallback for `none` and errors |

### Other

| | |
| --- | --- |
| `>>` | extract |
| `<<` | insert |
| `\|>` | pipe a value: `x \|> f` is `f(x)` |
| `\|\|>` | compose functions |
| `->` | relation |
| `!` | propagate an error to the caller |
| `?.` `?[]` `?()` | safe navigation |
| `..` `..=` | range, exclusive / inclusive |
| `...` | spread, variadic |
| `@` | evaluate at compile time |

Precedence, tightest first: `**` · `*` `/` `//` `%%` · `+` `-` · comparisons · `and` · `or`.

See [arithmetic](../operations/arithmetic), [logical](../operations/logical).

## Control flow

```fa
if condition { } else if other { } else { }

let value = if condition { 1 } else { 2 }   -- expression; `else` required

when value is {
  1 => "one"
  Int >> n if n < 0 => "negative"
  0..60 => "low"
  "a" | "b" => "letter"
  else => "other"
}

when {                    -- no subject: a condition ladder
  score >= 90 => "A"
  else => "F"
}

for 0..10 >> index { }
for items >> item, index { }
for map >> (key, value) { }
for people >> { name, age } { }
for parallel items >> item, index { }
@for staticItems >> item { }     -- unrolled at compile time

while condition { }
while queue.pop() >> item { }

loop { break }
let found = loop { break value }  -- expression

outer: for rows >> row {
  break outer
  continue outer
}

let value = do { 1 + 1 }
```

`when` is exhaustive. `for` and `while` are not expressions; `if`, `when`, `do` and `loop` are.

See [conditionals](../control-flow/conditionals), [loops](../control-flow/loops), [do](../control-flow/do).

## Functions

```fa
function add = (a: Int, b: Int): Int => a + b
function addOne = x => x + 1
let lambda = (): Result => { ... }

function greet = (name: String, greeting: String = "Hello") => ...
function sum = (...values: Int): Int => ...

greet("Ana", greeting = "Hi")
sum(...numbers)

function bound = xyz with (y = 0)
```

- Parameters are immutable references; `mutable` makes them mutable references with interior mutability only.
- The return type is required for a block body that returns a value.
- The last expression is the returned value; `return` exits early.
- Closures capture by copy.

Inside an object:

```fa
type Counter = {
  count: Int = 0
  zero(): Counter => Counter { count = 0 }   -- associated function
  isEmpty(self): Boolean => self.count == 0  -- method
  increment(mutable self) => { self.count += 1 }
  onChange = (self) => ...                   -- lambda field (has `=`)
}
```

See [defining functions](../functions/defining-functions).

## Errors and optionals

```fa
type ApiError = Errors { Timeout, Network = { message: String } }

function load = (): Int | ApiError => ...

let value = load()!                -- propagate to the caller
let value = load() ?? 0            -- fallback
let value = when load() is { Int >> v => v, Error => 0 }

let name: String? = none
let city = user?.address?.city ?? "unknown"

if value is none { }
if value is not none { }
```

- No `throw`, no `try`, no `catch`. `!` returns, it never panics.
- `?` is for absence; `!` is for failure.

See [errors](../values/errors), [optionals](../values/optionals).

## Extract and insert

```fa
object >> let { a, b = 4 }    -- declare immutable copies
object >> mutable { a }       -- declare mutable copies
object >> use { a }           -- declare references
object >> { a }               -- reassign existing values
object >> let { a: renamed }
array  >> let [first, second]
tuple  >> let (a, b)

object << { a, b }            -- write into existing fields
array  << [a, b]              -- replace the first items
```

See [extract and insert](../operations/extract-insert).

## Memory

1. Objects live on the stack; collections own the heap.
2. Assignment copies; the compiler turns a copy into a move when it can.
3. `use` makes a reference; function parameters and iteration are references.
4. A reference cannot outlive what it points into.
5. Values are destroyed at the end of their scope, in reverse order.

```fa
let copy = people[1]        -- copy
use reference = people[1]   -- reference: type `Person in people`
let moved = people.take(1)  -- move out of the collection
```

Growth invalidates references into `Array`, `Set` and `Map`, but not into `Bag`, `Columnar` or any `Growing*` collection.

Relations are compiler-maintained weak references:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Bag(Self->bestFriend)
}
```

See [memory management](../advanced/memory-management), [owned references](../advanced/owned-references), [relations](../advanced/relations).

## Reactivity

```fa
reactive count = 0
derived doubled = count * 2

effect {
  console.log(count)
}

let subscription = effect { ... }
subscription.stop()
```

Dependencies are tracked automatically. A `derived` expression must be pure.

See [reactivity](../advanced/reactivity).

## Concurrency

```fa
async function fetch = (url: String): Response | HttpError => await client.get(url)!

let data = fetch(url)!            -- blocking
let data = await fetch(url)!      -- awaited
let task = start fetch(url)       -- started; `await task!` later

let all = await [start a(), start b()]

for parallel items >> item, index { }   -- native only
```

See [concurrency](../advanced/concurrency).

## Metaprogramming

```fa
static version = @readFile("../version.txt")
let result = @add(2, 4)
static items = @[1, 2, 3]
let keys = @{ foo, bar }

@for items >> item { }

@typeToString(User)
@typeToSchema(User)
TypeFromSchema(schema)
TypeOf(value)
```

A file named `@something.fa` is evaluated at compile time.

See [metaprogramming](../advanced/metaprogramming).

## Modules

```fa
export = 42
export type = { ... }
export function = (x: Int): Int => x
export namespace = { ... }

utils >> use { formatDate }
use Component = react.Component
use farm = "github:farmers/farm@4"   -- in dependencies.fa
```

- One export per file, named after the file.
- Sibling files and root files are available with no declaration.
- Folders are namespaces; `(folder)` groups without a namespace.
- `application/main.fa` exports the entry function.

See [modules](../architecture/modules).

## Reserved keywords

```
and       async     await     break     continue  derived   do
else      enum      errors    export    false     fields    for
function  if        is        let       loop      mutable   namespace
none      not       or        reactive  return    static    true
type      union     use       when      while     yield
```

## Truthiness

Falsy: `false`, `none`, `0`, `""`, empty collections, errors. Everything else is truthy.

Prefer `is none` / `is not none` when a value could legitimately be `0`, `""` or empty.
