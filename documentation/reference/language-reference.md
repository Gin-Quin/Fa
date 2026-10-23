# Language reference

The whole language on one page. Each section links to the page that explains it.

## Lexical

| | |
| --- | --- |
| Inline comment | `-- comment` |
| Block comment | `--- ... ---` |
| Documentation | any comment directly above a declaration |
| Statement separator | newline (no semicolons) |
| String | double quotes only: `"Hello"` |
| Interpolation | `"Hello {name}"`, literal braces `{{ }}` |
| Number literals | `42`, `12.5`, `1_000_000`, `0b1010`, `0o17`, `0xFF`, `12n`, `50%` |
| Private member | `#name` |
| Compile-time operation | `@name(...)` |
| Naming | `camelCase` values, `PascalCase` types |

Naming is a convention, with one exception: in the parameters of a generic declaration, a capitalized parameter is a type parameter and a lowercase parameter is a compile-time value parameter (see [generics](#generics)).

See [syntax basics](/getting-started/syntax-basics).

## Declarations

Every declaration is `<keyword> <name>[: <type>] = <value>`.

```fa
let total = 1                              -- deeply immutable
mutable count = 0                          -- mutable
use address = user.address                 -- alias, no copy
static version = @readFile("version.txt")  -- compile-time value, used plainly: `version`
function add = (a: Integer, b: Integer): Integer => a + b
type Point = { x: Integer, y: Integer }
namespace utils = { ... }
reactive score = 0
derived doubled = score * 2
export = total
```

- A bare `name = value` assigns an existing `mutable` value. Assigning to an undeclared name is an error.
- Shadowing a name of an enclosing scope is allowed; redeclaring a name in the same scope is an error.
- Every value is destroyed at the end of its scope. `use` only creates an alias: it never copies, and it does not change when the value is destroyed.

See [declaring values](/values/declaring-values).

### Destructuring

```fa
let { a, b as c, d = 4, nested { e } } = object  -- rename with `as`, default with `=`
mutable { a } = object
use { a } = object                               -- aliases
let [first, second] = array
let (number, name) = tuple
use { Button as UIButton, Card } = ui.components  -- aliases of namespace members
```

Destructuring also works in loops (`for { name, age } in people`) and in parameters (`({ x, y }: Point)`).

See [destructuring](/operations/destructuring).

## Types

```
Boolean  String  Character
Integer  Float  BigInt  Percentage  Byte
Int8  Int16  Int32  Int64  Int128  UInt8  UInt16  UInt32  UInt64  UInt128  Float32  Float64
None  Any  Never
Array(T)  Set(T)  Map(K, V)  Bag(T)  Columnar(T)
GrowingArray(T)  GrowingSet(T)  GrowingMap(K, V)  GrowingBag(T)  GrowingColumnar(T)
Tuple(A, B)  Range  Task(T)  Optional(T)  T?
(name: String) => Boolean  (message: String) => Void
```

- `Integer` is `Int64` and `Float` is `Float64`; `Byte` is `UInt8`. `T?` is `Optional(T)`, which is `T | None`.
- `Character` is one grapheme cluster, and the `length` of a string counts graphemes.
- `Void` is the return type, in a function type, of a function that returns nothing.
- `Type(value)` is the type of a value.

Declaring types:

```fa
type Point = { x: Integer, y: Integer }
type Color = Enum { red, green, blue }
type Permissions = Fields { read, write, admin }
type Shape = Union { Circle = { radius: Float }, Square = { side: Float }, Point }
type ApiError = Errors { NotFound = { id: String }, Timeout }
```

There are no `enum`, `union`, `errors` or `fields` keywords: `Enum`, `Union`, `Errors` and `Fields` are type constructors used with `type`.

A type annotation is a **constraint**: `value: Type` means that `value` satisfies `Type`. Typing is [structural](/advanced/structural-typing).

Type operations:

| Operation | Meaning |
| --- | --- |
| `A \| B` | union |
| `T?` | optional (`T \| None`) |
| `{ ...User, permissions: Permissions }` | composition by spread (order matters, conflicting field types are an error) |
| `Shape - Shape.Point` | removes members from a union |
| `Pick(User, { name, address { city } })` | keeps the keys of a key set |
| `Omit(User, { password })` | removes the keys of a key set |

A type cannot contain itself directly (its size would be infinite). Recursion through a union, including `T?`, is allowed: the recursive values are stored on the heap automatically, and each value owns its children.

See [declaring types](/types/declaring-types), [enumerations](/types/enumerations), [unions](/types/unions), [fields](/types/fields), [type operations](/types/operations).

### Numbers

```fa
let count = 42                                -- Integer
let ratio = 12.5                              -- Float
let big = 12n                                 -- BigInt
let half = 50%                                -- Percentage
let score: Integer(min = 0, max = 100) = 87   -- ranged integer
let precise: Float = 3                        -- literals adapt to the expected type
let scaled = Float(count) * ratio             -- conversions are explicit calls
```

- No implicit numeric conversions. `as` never converts.
- Prefer `Integer(min = 0)` to an unsigned type for a non-negative value.
- Overflow: a certain overflow is a compile error, an impossible one generates no check, a possible one is checked at runtime and stops the program. Explicit variants: `wrappingAdd`, `saturatingAdd`, `checkedAdd` (and `Sub`, `Mul`, `Pow`); `checked*` methods `throws ArithmeticError`.
- A divisor must be provably non-zero (a literal, a ranged type, or narrowed with `if divisor != 0`); otherwise `/`, `//` and `%%` `throws DivisionByZero`.
- On JavaScript targets, `Integer.JS_MAX_SAFE_INTEGER` is the largest exact integer.

See [numbers](/values/numbers).

### Objects and collections

```fa
let point: Point = { x = 1, y = 2 }          -- typed by context
let other = Point { x = 1, y = 2 }           -- no context
let precise = Point(Float) { x = 1, y = 2 }  -- generic type

let numbers: Set(Integer) = [1, 2, 3]
let ages: Map(String, Integer) = [("Ana", 30), ("Bao", 25)]
mutable humans: Bag(Human) = []
let unique = Set.from([1, 2, 2])
let users = Set.from(userList, compare = (a, b) => a.id == b.id)
mutable buffer = Array(Integer).from([], capacity = 1000)

let last = items[-1]       -- negative index
let middle = items[1..4]   -- slice (a copy)
let maybe = items.get(10)  -- optional
```

- There are no positional constructors: always `Point { x = 1, y = 2 }`. Writing the type twice is a warning.
- Braces are only for objects. Anonymous objects contain only data and function pointers.
- Indexing that the compiler cannot prove `throws IndexError`; an unproven map key `throws KeyNotFound`. `.get(index)` and `.get(key)` return an optional.
- Methods: `length` (a property), `includes`, `delete`, `deleteWhere`, `sort()` (a copy) and `sortInPlace()`, `reverse()` and `reverseInPlace()`, `pop()` (an optional), `getOrInsert`.

See [objects](/values/objects), [arrays](/collections/arrays).

## Generics

Generic parameters come right after the declared name, for types and for functions:

```fa
type Pair(First, Second) = { first: First, second: Second }
type Point(NumberType: Integer | Float) = { x: NumberType, y: NumberType }
type Matrix(Element, rows: Integer, columns: Integer) = { ... }
type Collection(Item, Index = Integer) = { ... }

function first(Item) = (items: Array(Item)): Item? => items.get(0)
function sum(Value: Integer | Float) = (values: Array(Value)): Value => ...
```

Applying a generic is a call:

```fa
Array(Integer)
Map(String, User)
Integer(min = 0, max = 200)
Matrix(Float, 4, 4)

first([1, 2, 3])                -- `Item` is inferred as `Integer`
sum(Float)([1, 2, 3])           -- explicit
Point(Float) { x = 1, y = 2 }
Set(Float).from([1, 2])
```

- A **capitalized** parameter is a type parameter; a **lowercase** parameter is a compile-time value parameter. No `@` on compile-time parameters.
- Constraints use `:`. A union means "one of these types"; an object type means "has at least these fields".
- Parameters can have defaults: `Index = Integer`.
- Generic arguments are inferred from the arguments and from the expected type. An ambiguity is an error.
- Deliberately left out: specialisation and overloading, higher-kinded types, variance annotations, implicit conversions to satisfy a constraint, overload resolution on the return type.

See [generics](/types/generics).

## Operators

### Arithmetic

| | | | |
| --- | --- | --- | --- |
| `+` add | `-` subtract | `*` multiply | `**` power (right-associative) |
| `/` divide (always a `Float`) | `//` floor division (an `Integer`) | `%%` remainder (floor-based) | `50%`, `(x + y)%` percentage |

Compound assignment: `+=` `-=` `*=` `**=` `/=` `//=` `%%=`

`%` is only the percentage suffix. Bitwise operations are methods: `bitAnd`, `bitOr`, `bitXor`, `bitNot`, `shiftLeft`, `shiftRight`.

### Comparison and logic

| | |
| --- | --- |
| `==` `!=` | deep structural equality (sets and bags compare without order) |
| `is` `is not` | pattern, type or literal match, with narrowing: `value is Animal`, `status is "ready"`, `score is 0..60` |
| `is in` `is not in` | membership |
| `<` `<=` `>` `>=` | ordering, chainable: `0 < x <= 100` evaluates each operand once |
| `and` `or` | short-circuiting; `and` binds tighter than `or` |
| `not` | negation |

There is no truthiness: `if`, `while`, `and`, `or` and `not` only accept a `Boolean`. Optionals are checked with `is none` and `is not none`. Identity is not observable.

### Other

| | |
| --- | --- |
| `?.` | safe access on an optional |
| `??` | fallback for an optional: `user?.name ?? "anonymous"` (never for errors) |
| `then` | function composition: `parse then validate then save` |
| `with` | partial application: `xyz with (y = 0)` |
| `..` `..=` | range, exclusive / inclusive: `a..b`, `a..=b`, `a..`, `..b`, `..=b`, `..` |
| `...` | spread, variadic parameter |
| `->` | reverse side of a relation, in a type: `Bag(Self->bestFriend)` |
| `as` | gives a name (destructuring, `when` bindings); never converts |
| `try` `catch` | propagate or handle an error |
| `await` `start` | wait for or start an asynchronous call |
| `@` | compile-time operation |

### Precedence

From the tightest to the loosest:

1. access and call: `.` `?.` `()` `[]`
2. postfix: `?` `%`
3. prefix: `-`
4. `**` (right-associative)
5. `*` `/` `//` `%%`
6. `+` `-`
7. ranges: `..` `..=`
8. comparisons: `<` `<=` `>` `>=`
9. `==` `!=` `is` `is not` `is in` `is not in`
10. `not`
11. `and`
12. `or`
13. `then`

:::note
The precedence of `??`, `with` and `catch` relative to these operators is not fixed yet. Use parentheses when combining them with other operators.
:::

See [arithmetic](/operations/arithmetic), [logical](/operations/logical).

## Control flow

```fa
if condition { } else if other { } else { }

let value = if condition { 1 } else { 2 }  -- expression: `else` required

let grade = when score is {
	90..=100 => "A"
	60..90 => "B"
	else => "C"
}

let kind = when letter is {
	"a" | "e" | "i" | "o" | "u" => "vowel"
	else => "consonant"
}

let area = when shape is {
	Shape.Circle as { radius } => 3.14159 * radius * radius
	Shape.Square as square => square.side * square.side
	Shape.Point => 0.0
}
```

- `when` branches match literals, types, ranges and alternatives (`|`), with `else =>` as the fallback. Bindings use `as`: `as circle`, `as { radius }`, `as (a, b)`.
- `when` is exhaustive. There are no guards: put an `if` inside the branch. There is no `when` without a subject.

```fa
for item in items { }
for item, index in items { }
for { name, age } in people { }
for (key, value) in map { }
for index in 0..10 { }
for 1..10 { }                        -- no binding
for parallel item in items { }
for parallel(threads = 2) item in items { }
@for item in staticItems { }         -- unrolled at compile time

while condition { }
while let value = stack.pop() { }

loop { break }
let found = loop { break value }     -- expression

let result = do { 1 + 1 }            -- expression and scope
```

- `break` leaves the innermost loop and `continue` skips to its next iteration. There are no labeled loops.
- `return` inside a `do` block returns from the enclosing function.
- `if`, `when`, `do` and `loop` are expressions; `for` and `while` are not.
- Adding or removing elements of a collection while iterating over it is an error. Over a `mutable` collection, the loop variable is a mutable reference: writing to it writes into the collection.

See [conditionals](/control-flow/conditionals), [loops](/control-flow/loops), [do](/control-flow/do).

## Functions

There is one form:

```
[async] function name = (parameters): ReturnType throws Errors requires name: Type mutates path invalidates path => body
```

```fa
function add = (a: Integer, b: Integer): Integer => a + b
function greet = (name: String, greeting: String = "Hello"): String => "{greeting}, {name}!"
function sum = (...values: Integer): Integer => ...
function distance = ({ x, y }: Point): Float => ...

greet("Ana", greeting = "Hi")  -- named arguments come after positional ones
sum(...numbers)

function increment = add with (b = 1)   -- partial application
function process = parse then validate  -- composition
numbers.map((x) => x + 1)               -- lambda
```

- The right side of `=` can be any function known at compile time: a lambda, an alias, a `then` composition, a `with` partial application, the result of a factory.
- With a block body, the return type and every effect (`throws`, `requires`, `mutates`, `invalidates`) must be declared; a block body without a return type returns nothing. With a single-expression body, they are inferred. The last expression of a block is its value; `return` exits early.
- Lambda parameters are always parenthesized, and lambdas are only written where a value is expected: `let f = (x) => x + 1` is an error, use `function`.
- One variadic parameter at most, in last position.
- Parameters are immutable references. A parameter of a primitive type cannot be mutated.
- Closures capture by copy. Capturing a reference with `use` is allowed only if the closure provably does not escape.
- Functions are hoisted at the top level and in types; inside a block, a function is visible after its declaration.

### Effects

| Effect | Meaning |
| --- | --- |
| `throws ErrorA \| ErrorB` | the errors the function can fail with |
| `requires name: Type` | a value the function needs from its caller's environment |
| `mutates self`, `mutates input`, `mutates self.balance`, `mutates self[index]` | what the function mutates: a whole parameter, a field, an index given by a parameter |
| `invalidates self` | the function can move or remove elements of a collection (it implies `mutates`) |
| `invalidates item` | the function only invalidates the reference `item` |

Calling a mutating function requires a `mutable` value.

### Methods

Methods are only declared inside `type` declarations:

```fa
type Counter = {
	value: Integer
	readonly name: String
	onChange: (value: Integer) => Void  -- function pointer

	function isZero = (self): Boolean => self.value == 0
	function increment = (self) mutates self => {
		self.value += 1
	}
}
```

See [defining functions](/functions/defining-functions), [function composition](/functions/function-composition).

## Errors and optionals

```fa
type ApiError = Errors { NotFound = { id: String }, Timeout }

function load = (id: String): User throws ApiError => {
	let user = users.get(id)
	if user is none {
		throw ApiError.NotFound { id }
	}
	user
}

let user = try load(id)                   -- propagate: the function must declare `throws ApiError`
let user = load(id) catch error => guest  -- handle
let user = load(id) catch error => when error is {
	ApiError.NotFound => guest
	else => throw error
}
let data = try await fetch(url)

let name: String? = none
let city = user?.address?.city ?? "unknown"

if name is none { }
if name is not none { }                   -- narrows `name` to `String`
```

- `throw` exits the function with an error. A call that can fail must be preceded by `try` or followed by `catch`.
- Errors are never part of the return type, and are never falsy. There is no `!`.
- An optional is an absence; an error is a failure. `?.` and `??` only apply to optionals.

See [errors](/values/errors), [optionals](/values/optionals).

## Requirements

```fa
type Logger = { info: (message: String) => Void }

function greet = (name: String) requires logger: Logger => {
	logger.info("Hello, {name}!")
}

do {
	provide Logger = ConsoleLogger { }
	greet("Ana")  -- the logger is not passed at the call site
}
```

- Requirements are matched by type. A provided value is available until the end of its block; a nested block can provide another one.
- A function that calls a function with requirements must provide them or declare them.
- A requirement can be mutated with the effect `mutates name`.
- A type whose fields are function pointers describes a service.
- Requirements cross `start`, but not worker boundaries.

See [requirements](/advanced/requirements).

## Memory

1. Assignment copies; the compiler turns a copy into a move when it can.
2. `use` creates an alias; function parameters and iteration give references.
3. Where data lives is part of the type: a reference into `people` has the type `Person in people`, always written with its collection.
4. A reference cannot outlive what it points into. After an operation that invalidates it, a local reference must be checked with `is not none` (`Bag`, `Columnar`, `Growing*`, `Set`, `Map`) or, for an `Array`, declared again with `use`, before it is used.
5. A field typed `T` or `T?` holds a copy (recursion through a union is stored on the heap). A field holding a reference is a relation.
6. Values are destroyed at the end of their scope, in reverse order.

There is no garbage collector and no borrow checker.

```fa
mutable people: Bag(Person) = []

use ada = people.add(Person { name = "Ada" })  -- a reference: `Person in people`
let copy = ada                                 -- a copy
```

| Collection | Invalidating operations |
| --- | --- |
| `Array` | `push`, `insert`, `delete`, `pop`, `clear`, `sortInPlace` (`invalidates self`) |
| `Set`, `Map` | `add`, `delete`, `clear` (`invalidates self`) |
| `Bag`, `Columnar` | `delete(item)` only invalidates `item` |
| `Growing*` | none: they never delete |

`Bag.add`, `GrowingBag.add` and `Columnar.add` return a reference.

A stored reference is a relation. It names its collection, which is a field of the containing type or a module-level value, and must have stable addresses (`Bag`, `GrowingBag`, `Columnar`; never `Array`, `Set`, `Map`). It is always optional or held in a `Bag`. Declaring the reverse side is not required; it is declared from the "one" side:

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(Self->bestFriend)
}
```

| Relation | Forward side | Reverse side |
| --- | --- | --- |
| one-to-one | `bestFriend: (Person in people)?` | `bestFriendOf: (Self->bestFriend)?` |
| one-to-many | `bestFriend: (Person in people)?` | `bestFriendOf: Bag(Self->bestFriend)` |
| many-to-many | `bestFriends: Bag(Person in people)` | `bestFriendOf: Bag(in Self->bestFriends)` |

Deleting a target updates every field that pointed at it (an optional becomes `none`, a bag drops the reference), through the reverse side or a hidden reverse index: a relation never dangles.

See [memory management](/advanced/memory-management), [owned references](/advanced/owned-references), [relations](/advanced/relations).

## Reactivity

```fa
reactive count = 0
derived doubled = count * 2

watch {
	console.log(count)
}

let subscription = watch { ... }
subscription.stop()
```

Dependencies are tracked automatically. A `derived` expression must be pure.

See [reactivity](/advanced/reactivity).

## Concurrency

```fa
async function fetchUser = (id: String): User throws HttpError => {
	let response = try await http.get("/users/{id}")
	...
}

let user = try fetchUser(id)          -- blocking
let user = try await fetchUser(id)    -- awaited, inside an async function
let task = start fetchUser(id)        -- started: a `Task(User)`
let user = try await task
task.cancel()

start do {
	-- runs in the background
}

for parallel item, index in items {
	results[index] = compute(item)    -- each iteration writes to its own index
}
```

- Async is colorless: an `async function` can be called blocking, awaited or started. On JavaScript targets, it is compiled lazily into `name___sync` and `name___async` versions. Calling an async-only primitive synchronously on a target is a compile error.
- `await` is only allowed inside an `async function`.
- Tasks are combined with `Task.all`, `Task.any` and `Task.settled`.
- Mutexes, atomics and channels are declared `mutable`. Only thread-safe types can be shared mutably between threads.
- Workers copy arguments and results across threads: references and relations cannot cross worker boundaries.

See [concurrency](/advanced/concurrency).

## Metaprogramming

`@` marks compile-time operations only. Compile-time values are declared with `static`, and used plainly.

```fa
static version = @readFile("version.txt")
static configuration = @parseJson(@readFile("config.json"))
static routes = @readDirectory("routes")
static six = @add(2, 4)              -- a pure function called at compile time

@if target is "native" {
	...
} @else {
	...
}

@for route in routes {
	...
}

Type(value)                          -- the type of a value
@typeToString(User)
@typeToSchema(User)

use weather = @import("https://example.com/weather/types.fa")
```

- `target` is a static value.
- Compile-time evaluation is deterministic: no clock, no network, no randomness. The files it reads become build dependencies.
- `@import` is only for remote modules.

See [metaprogramming](/advanced/metaprogramming).

## Tests

```fa
-- math.test.fa
test "adds two numbers" {
	assert add(2, 3) == 5
}

namespace division = {
	test "divides evenly" {
		assert 6 // 3 == 2
	}
}

test skip "not ready yet" { }
test only "the one I am debugging" { }

@test "runs at compile time" {
	assert add(2, 3) == 5
}
```

- Tests live in `*.test.fa` files, next to the code they test, and are grouped with `namespace`.
- `assert` is a keyword, without parentheses.
- `fa test` fails if a `test only` reaches the main branch.
- Requirements are given to the code under test with `provide`.

See [testing](/architecture/testing).

## Modules

```fa
export = 42
export type = { ... }
export function = (x: Integer): Integer => x
export namespace = { ... }

use { formatDate } = utils
use { Button as UIButton, Card } = ui.components
library.greet("Ada")  -- from a file of `application/`
```

- One export per file, named after the file.
- Sibling files and folders are available with no declaration; folders are namespaces, and `(folder)` groups files without creating a namespace. Files of a parent folder are not accessible.
- Root files are available everywhere in their own root (`library/` or `application/`). `application/` sees `library/` only through the `library` namespace.
- `#name` members are private.
- `application/main.fa` exports the entry function. It may take `arguments: Array(String)` and return an exit code (`Integer`); its errors are declared with `throws`.

See [modules](/architecture/modules).

### Projects

| Level | Contents |
| --- | --- |
| Standalone file | a `.fa` file; sibling files and folders are available, parent files are not |
| Package | `package.fa`, `dependencies.fa`, `library/`, `application/`, `scripts/` |
| Workspace | `workspace.fa` and `packages/` (scanned recursively) |

Configuration files are regular Fa files, executed at build time, that export an object:

```fa
-- package.fa
export = {
	name = "farm"
	version = "1.0.0"
	targets = ["web", "native"]
	entry = "application/main.fa"
}
```

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
	shared = "../shared"
}
```

- Targets: `web`, `bun`, `node`, `deno`, `native`, `wasm`, `rust`. Default: `web`, `bun`, `native`.
- The workspace configuration provides defaults that packages override field by field.
- Versions: `@4`, `@4.2`, `@4.2.1`. Resolved versions are locked in `dependencies.lock`, with content hashes.

See [packages](/architecture/packages), [workspaces](/architecture/workspaces), [configuration](/architecture/configuration), [dependencies](/architecture/dependencies).

### Command line

| Command | |
| --- | --- |
| `fa run file.fa` | run a standalone file |
| `fa new package <name>`, `fa new workspace` | scaffold a package or a workspace |
| `fa develop` | run with hot reloading |
| `fa test [--watch] [--coverage] [--target=...]` | run the tests |
| `fa check` | type-check |
| `fa format` | format the code |
| `fa build [--target=...]` | compile |
| `fa preview` | run the compiled output |
| `fa update` | update the locked dependencies |

## Keywords

```
and          as           assert       async        await        break
catch        continue     derived      do           else         export
false        for          function     if           in           invalidates
is           let          loop         mutable      mutates      namespace
none         not          or           parallel     provide      reactive
readonly     requires     return       start        static       test
then         throw        throws       true         try          type
use          watch        when         while        with
```

Contextual keywords: `skip` and `only` (after `test`). Special names: `self` (the receiver of a method) and `Self` (the type being declared).
