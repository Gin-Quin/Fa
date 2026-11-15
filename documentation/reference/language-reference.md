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
| Number literals | `42`, `12.5`, `1.5e9`, `1_000_000`, `0b1010`, `0o17`, `0xFF`, `12n`, `0xFFn`, `50%`, `12.5%` (prefixes and exponent in lowercase) |
| Private member | `#name` |
| Compile-time operation | `@name(...)` |
| Naming | `camelCase` values, `PascalCase` types |

Naming is a convention, with one exception: in the parameters of a generic declaration, a capitalized parameter is a type parameter and a lowercase parameter is a compile-time value parameter (see [generics](#generics)).

See [syntax basics](/getting-started/syntax-basics).

## Declarations

Every declaration is `<keyword> <name>[: <type>] = <value>`. A type declares its generic parameters after its name (`type Pair<Left, Right> = ...`), a function at the start of its signature (`function first = <Item>(items: Array<Item>) => ...`). `reuse` and the `export` forms are statements outside this shape.

```fa
let total = 1                              -- deeply immutable
mutable count = 0                          -- mutable
use address = user.address                 -- alias, no copy
mutable use best = players[0]              -- alias that can designate another element
static version = @readFile("version.txt")  -- compile-time value, used plainly: `version`
function add = (a: Integer, b: Integer): Integer => a + b
type Point = { x: Integer, y: Integer }
namespace utils = { ... }
reactive score = 0
derived doubled = score * 2
export = total
```

- A bare `name = value` assigns an existing `mutable` value. Assigning to an undeclared name is an error.
- A module-level value (declared at the top of a module, or as a namespace member) lives as long as the program, at a stable address. Unlike a parameter, a `mutable` one can be replaced, even if it is a primitive. A function that modifies it declares it in its effects (`mutates people.size`).
- Shadowing a name of an enclosing scope is allowed; redeclaring a name in the same scope is an error, except for an invalidated (or maybe invalidated) `use` reference, which is declared again. A redeclaration inside an inner block only lasts until the end of that block. The right-hand side of a declaration never sees the name being declared.
- An unannotated binding has the widened type of its value (`let x = 42` is an `Integer`, `let s = "ready"` a `String`); the compiler also keeps what it knows about the value (its exact literal, its range, its narrowed member) until the place is mutated. An annotated binding has the annotated type. `static` values keep their literal type.
- A `use` alias designates the same place for its whole life. A `mutable use` alias can be moved to another place of the same type with `reuse best = player`: `reuse` is a statement, not a declaration, and `=` still writes into the designated value. A `mutable use` is moved with `reuse` after an invalidation, instead of being declared again.
- Every value is destroyed at the end of its scope. `use` only creates an alias: it never copies, and it does not change when the value is destroyed.
- `use` takes a place (`use a = user.address`) or a call that returns a reference (`use p = people.add(...)`, `use s = scores.get("ana")`, an optional reference). Any other expression is an error (`use x = 5`). `use` on a place that holds a reference designates its target, not the field: `use next = node.next` is not changed by a later write to `node.next`.

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
Boolean  String  Bytes
Integer  Float  BigInt  Percentage  Byte
Int8  Int16  Int32  Int64  Int128  UInt8  UInt16  UInt32  UInt64  UInt128  Float32  Float64
None  Never  Void
Array<T>  Set<T>  Map<K, V>  Pool<T>  Columnar<T>  OrderedColumnar<T>
GrowingArray<T>  GrowingSet<T>  GrowingMap<K, V>  GrowingPool<T>  GrowingColumnar<T>
Tuple<A, B>  Range  Task<T>  Optional<T>  T?
(name: String) => Boolean  (message: String) => Void
```

- `Integer` has the range and native storage of `Int64` (two types, not interchangeable inside collections), and `Float` is `Float64`; `Byte` is `UInt8`. A `Float32` is accepted where a `Float` is expected, but arithmetic, comparisons and equality never mix the two. `T?` is `Optional<T>`, which is `T | None`, and `T??` is `T?`.
- Every integer type is a range of `Integer` values with a storage width: `Int8` is `Integer<min = -128, max = 127>` stored in 8 bits.
- A `String` is a sequence of UTF-16 code units on every target, exactly like a JavaScript string: `length`, `text[i]` (a `String` of length 1), slices, `indexOf` and `<` work on code units. `for character in text` visits the code points, `text.characters()` gives the grapheme clusters, and there is no character type.
- `Bytes` is a fixed-length sequence of bytes (a `Uint8Array` on JavaScript). `text.toUtf8()` encodes a string, `String.fromUtf8(bytes)` decodes one (`throws Utf8Error`), and `text.utf8Length()` counts the bytes without encoding. Input and output give and take `Bytes`; their `Text` variants (`File.readText`) decode UTF-8.
- `let middle = text[1..3]` copies the code units; `use middle = text[1..3]` is a slice of type `String in text`, a reference into the string, invalidated by any mutation of the string.
- `Void` is the type of "no value": the return type of a function that returns nothing, also usable as a type argument (`Task<Void>`). It can't be bound with `let`, and it is not `None`.
- `Never` is the bottom type: the type of `throw`, `return`, `break`, `continue`, of a `loop` without `break` and of a call to a function returning `Never`.
- There is no `Any` type: use generics or unions.
- `Type<value>` is the inferred type of a value, without what the compiler knows about its value; `value` is not evaluated. For a parameter (and a `use` alias or a loop variable), it is the type of what it designates: `(human: Human): Type<human>` returns the caller's full type.

Declaring types:

```fa
type Point = { x: Integer, y: Integer }
type Color = Enum { red, green, blue }
type Permissions = Fields { read, write, admin }
type Shape = Union { Circle = { radius: Float }, Square = { side: Float }, Point }
type ApiError = Errors { NotFound = { id: String }, Timeout }
```

There are no `enum`, `union`, `errors` or `fields` keywords: `Enum`, `Union`, `Errors`, `Fields` and `CStruct` (the layout of a C `struct`, see [foreign code](#foreign-code)) are type constructors used with `type`. A bare member (`Point` in `Shape`, `Timeout`) carries no data; a member whose data is not an object is built with parentheses (`UserId.User(12)`). Named unions, their members and enumerations are nominal; `Fields` types are structural. Enumerations are not ordered and have no arithmetic: compare `Integer(a) < Integer(b)`.

A type annotation is a **constraint**: `value: Type` means that `value` satisfies `Type`. Typing is [structural](/advanced/structural-typing):

- A value with more fields is accepted where fewer are expected. A parameter or a `use` alias sees it through the declared type, without copying; a copy annotated with the narrower type (`let human: Human = hero`) drops the extra fields.
- An object literal can't have fields that the expected type lacks.
- A parameter keeps the type of its argument, `Type<parameter>`: an unannotated copy of it keeps every field, and `(human: Human): Type<human>` returns the caller's full type.
- A function that mutates a parameter only receives a value whose written fields have exactly the declared types. A collection of supersets (`Array<Hero>` for `Array<Human>`) is accepted only by a parameter that is not mutated; otherwise the function must be generic. `Set` elements and `Map` keys never accept supersets.
- Compatibility and identity compare the public data fields only. Methods and defaults resolve statically, from the type written at the use site. A type with `#private` fields is nominal.
- An anonymous union must be discriminable at runtime: its members are different kinds of values (number, string, boolean, `none`, collection, object), or objects that differ by a required field or a literal field. Otherwise, use a named `Union`, whose members carry a tag.

Type operations:

| Operation | Meaning |
| --- | --- |
| `A \| B` | union |
| `T?` | optional (`T \| None`) |
| `{ ...User, permissions: Permissions }` | composition by spread (order matters, conflicting field types are an error) |
| `Shape - Shape.Point` | removes members from a union |
| `Pick<User, { name, address { city } }>` | keeps the keys of a key set (nested keys allowed) |
| `Omit<User, { password }>` | removes the keys of a key set |
| `Keys<User>`, `DeepKeys<User>`, `KeyType<Map<String, User>>` | the key set of an object type, and the key type of a map |

A type cannot contain itself directly (its size would be infinite). Recursion through a named declaration and a union, an optional (`T?`) or a collection is allowed: the recursive values are stored on the heap automatically, and each value owns its children. Polymorphic recursion (`List<T> = { tail: List<Array<T>>? }`) is an error.

See [declaring types](/types/declaring-types), [enumerations](/types/enumerations), [unions](/types/unions), [fields](/types/fields), [type operations](/types/operations).

### Numbers

```fa
let count = 42                                -- Integer
let ratio = 12.5                              -- Float
let big = 12n                                 -- BigInt
let half = 50%                                -- Percentage
let score: Integer<min = 0, max = 100> = 87   -- ranged integer
let precise: Float = 3                        -- literals adapt to the expected type
let scaled = Float(count) * ratio             -- conversions are explicit calls
```

- Integers are compared by range: an integer value is accepted wherever its range fits the expected one (`let c: Int16 = a + b` with `Int8` operands), with no conversion. A narrower range needs a conversion call (`Int8(x)`). Conversions between `Integer`, `Float`, `BigInt` and `Percentage` are always explicit. `as` never converts. An arithmetic expression is exact: only its result is checked, where it goes; any other value must be proven to fit, or converted. Integer equality compares exact values across widths.
- Prefer `Integer<min = 0>` to an unsigned type for a non-negative value.
- Overflow: a certain overflow (like a divisor proven zero or an index proven out of bounds) is a compile error, an impossible one generates no check, a possible one is checked at runtime and stops the program in development builds (`fa run`, `fa test`, `fa develop`), and wraps around in release builds (`fa build`), like Rust; `overflowChecks` in the build options changes the default. Explicit variants: `wrappingAdd`, `saturatingAdd`, `checkedAdd` (and `Sub`, `Mul`, `Pow`); `checked*` methods `throws ArithmeticError`.
- An integer divisor must be provably non-zero (a literal, a ranged type, or narrowed with `if divisor != 0`); otherwise `//` and `%%` `throws DivisionByZero`. Floats follow IEEE 754: `/` never throws, `NaN == NaN` is false.
- Index and divisor proofs follow fixed rules (ranges of types and narrowing, loop bounds, `index < collection.length` guards): what they don't prove needs `try`. See [ranges and proofs](/values/numbers).
- On JavaScript targets, an `Integer` is a JavaScript number and its overflow check uses `Integer.jsMaxSafeInteger` (2^53 − 1): the program stops rather than losing precision. `Int64`, `UInt64`, the 128-bit types and `BigInt` are JavaScript `BigInt`s. A certain overflow of 2^53 − 1 is the one compile error that depends on the target.
- Constants: `Int8.minValue`, `Integer.maxValue` (on every integer type), `Float.maxValue`, `Float.infinity`, `Float.nan`, `Float.epsilon`, `Integer.jsMaxSafeInteger`.

See [numbers](/values/numbers).

### Objects and collections

```fa
let point: Point = { x = 1, y = 2 }          -- typed by context
let other = Point { x = 1, y = 2 }           -- no context
let precise = Point<Float> { x = 1, y = 2 }  -- generic type

let numbers: Set<Integer> = [1, 2, 3]
let ages: Map<String, Integer> = [("Ana", 30), ("Bao", 25)]
mutable humans: Pool<Human> = []
let unique = Set.from([1, 2, 2])
let users = Set.from(userList, key = (user) => user.id)
mutable buffer = Array<Integer>.from([], capacity = 1000)

let last = items[-1]       -- negative index
let middle = items[1..4]   -- slice (a copy)
let maybe = items.get(10)  -- optional
```

- There are no positional constructors: always `Point { x = 1, y = 2 }`. Writing the type twice is a warning. `{ id }` is short for `{ id = id }`.
- In expression position, braces are an object, except after `=>`, `if`, `else`, `do`, `loop`, `for`, `while`, `watch`, `test` and `namespace`, where they open a block. A lambda body `=> { ... }` is a block: return an object with its type name, `=> Point { x = 1, y = 2 }`. Where a key set is expected (`Pick<User, { name }>`), braces are a key set. Anonymous objects contain only data and function pointers.
- Indexing that the compiler cannot prove `throws IndexError`; an unproven map key `throws KeyNotFound`. `.get(index)` and `.get(key)` return an optional.
- Methods: `length` (a property: `size` for the collections without positions, `Set`, `Map`, `Pool` and `Columnar`), `includes`, `delete`, `take` (returns the removed element; an optional on `Set` and `Map`), `deleteWhere`, `sort()` (a copy, stable) and `sortInPlace()`, `reverse()` and `reverseInPlace()`, `pop()` (an optional), `getOrInsert`, `sum()`. `Set.delete` and `Map.delete` return whether something was removed. `find` returns an optional reference, which `let` copies.
- A comparator returns a number whose sign gives the order: `sort((a, b) => b - a)`. Callbacks receive `(item, index)` on the collections with positions, `(item)` on the others, `(key, value)` on maps. `filter` keeps the kind of collection; `map` returns an `Array` (a `Map` on maps).
- Chained methods (`filter`, `map`, `reduce`...) run step by step, as in JavaScript; the compiler fuses a chain into one pass when its callbacks are pure. `values()` gives an iterator, which runs element by element: `items.values().filter(f).firstValues(3).toArray()`. `firstValues(count = 1)` returns an iterator, never an element.
- `pool.add({ … })` builds the element directly in the collection and returns a reference: bind it with `use`, since `let` would copy it.

See [objects](/values/objects), [arrays](/collections/arrays).

## Generics

Generic parameters come right after the declared name, for types and for functions:

```fa
type Pair<First, Second> = { first: First, second: Second }
type Point<NumberType: Integer | Float> = { x: NumberType, y: NumberType }
type Matrix<Element, rows: Integer, columns: Integer> = { ... }
type Collection<Item, Index = Integer> = { ... }

function first = <Item>(items: Array<Item>): Item? => items.get(0)
function sum = <Value: Integer | Float>(values: Array<Value>): Value => ...
```

Generic parameters and arguments are written between angle brackets; parentheses are only calls and conversions:

```fa
Array<Integer>
Map<String, User>
Integer<min = 0, max = 200>
Matrix<Float, 4, 4>

first([1, 2, 3])                -- `Item` is inferred as `Integer`
sum<Float>([1, 2, 3])           -- explicit
Point<Float> { x = 1, y = 2 }
Set<Float>.from([1, 2])
```

- A **capitalized** parameter is a type parameter; a **lowercase** parameter is a compile-time value parameter. No `@` on compile-time parameters.
- Constraints use `:`. A union means "one of these types"; an object type means "has at least these fields".
- Parameters can have defaults: `Index = Integer`.
- Generic arguments are inferred from the arguments and from the expected type. An ambiguity is an error. Compile-time value parameters are never inferred.
- Explicit generic arguments come before the call: `repeatText<3>("ab")`, `sum<Float>([1, 2])`, `makeAdder<Float>(5)(3)`. On a built-in scalar type, parentheses convert (`Int8(x)`) and angle brackets refine (`Integer<min = 0>`).
- `<` followed by whitespace (space, tab, line break) is the comparison operator; any other `<` opens a generic list. Inside an open generic list, `>` closes it; outside, `>` is the comparison operator. A comparison inside a generic list goes in parentheses (`Choose<(count > 255), UInt16, UInt8>`), which suspend the list until they close. `<=`, `>=`, `=>` and `->` are separate tokens, never generic brackets (`Pool<people->bestFriend>`). So `count<limit` is not a comparison: write `count < limit`.
- Generic parameters are part of a function's signature: a lambda can declare them (`<Value>(value: Value) => value`), and a function type can have them (`(transform: <Value>(value: Value) => Value) => ...`). A value of a generic function type must be known at compile time (a named function or a lambda), and the function receiving it is compiled for each one. Generic function types are never inferred.
- A generic body is checked once, against the constraints; a union constraint binds the parameter to exactly one member. Only `@for`, `@if` and type functions are checked after substitution. A method called on a type parameter resolves on the actual type, and a constraint can require a method.
- Deliberately left out: specialisation and overloading, higher-kinded types, variance annotations, implicit conversions to satisfy a constraint, overload resolution on the return type.

See [generics](/types/generics).

## Operators

### Arithmetic

| | | | |
| --- | --- | --- | --- |
| `+` add | `-` subtract | `*` multiply | `**` power (right-associative) |
| `/` divide (always a `Float`) | `//` floor division (an `Integer`) | `%%` remainder (floor-based) | `50%`, `(x + y)%` percentage |

Compound assignment: `+=` `-=` `*=` `**=` `/=` `//=` `%%=`, and `.=` to assign several fields of a `mutable` object at once (`settings .= { port = 8080, secure = true }`, shallow: every value is evaluated first, then the fields are assigned in order, and only these fields are mutated). Assignments don't chain: `a = b = c` is an error.

`%` is only the percentage suffix. Bitwise operations are methods: `bitAnd`, `bitOr`, `bitXor`, `bitNot`, `shiftLeft`, `shiftRight`.

### Comparison and logic

| | |
| --- | --- |
| `==` `!=` | deep structural equality of the data fields (sets, pools and maps compare without order; relation and private fields are not compared; values containing functions can't be compared) |
| `is` `is not` | pattern, type or literal match, with narrowing: `value is Animal`, `status is "ready"`, `score is 0..60`, `answer is "y" \| "yes"` |
| `is in` `is not in` | membership |
| `<` `<=` `>` `>=` | ordering, chainable: `0 < x <= 100` evaluates each operand once; `<` is followed by whitespace (otherwise it opens a [generic list](#generics)) |
| `and` `or` | short-circuiting; `and` binds tighter than `or` |
| `not` | negation, binds tighter than `and`: `not a and b` is `(not a) and b` |

There is no truthiness: `if`, `while`, `and`, `or` and `not` only accept a `Boolean`. Optionals are checked with `is none` and `is not none`. Identity is not observable, except that a copy of a thread-safe value (a mutex, an atomic, a channel) designates the same value.

`is`, `is not`, and `==` or `!=` against a literal, a `static` value or `none`, narrow the type of local values, parameters and immutable field paths of them; the narrowing ends with an assignment or a call that may mutate the place. `is <Type>` narrows a union, and comparing a discriminant field (`shape.kind == "circle"`) narrows a union of objects; on any other value `is <Type>` is decided at compile time. See [narrowing](/control-flow/conditionals#narrowing).

### Other

| | |
| --- | --- |
| `?.` | safe access on an optional (short-circuits the whole chain on its right) |
| `??` | fallback for an optional: `user?.name ?? "anonymous"` (never for errors) |
| `then` | function composition: `parse then validate then save` |
| `with` | partial application: `xyz with (y = 0)` |
| `..` `..=` | range, exclusive / inclusive: `a..b`, `a..=b`, `a..`, `..b`, `..=b`, `..` |
| `...` | spread, variadic parameter |
| `->` | reverse side of a relation: `collection->field`, as in `Pool<people->bestFriend>` |
| `as` | gives a name (destructuring, `when` bindings); never converts |
| `try` `catch` | propagate or handle an error |
| `await` `start` | wait for or start an asynchronous call |
| `@` | compile-time operation |

### Precedence

From the tightest to the loosest:

1. access, call and partial application: `.` `?.` `()` `[]` `with`
2. postfix: `%`
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
13. `??`
14. `then`
15. `try`, `await`, `start`: they cover the whole expression on their right, up to a `catch`
16. `catch`: its handler extends as far right as possible, and it covers every failing call of its left operand
17. `return`, `throw`, `break`: they cover everything on their right
18. assignments: `=`, `+=` and the other compound assignments, `.=`

`try` and `catch` on the same expression is an error. In `when` patterns, `|` is the loosest, and `as` applies to the whole alternative.

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

- `when` branches match literals, types, ranges and alternatives (`|`), with `else =>` as the fallback. Union members are always qualified (`Shape.Circle`, `ApiError.NotFound`). Bindings use `as`: `as circle`, `as { radius }`, `as (a, b)`.
- `when` is exhaustive. There are no guards: put an `if` inside the branch. There is no `when` without a subject.
- Without an expected type, the type of an `if` or `when` expression is the union of the widened types of its branches; a literal branch first adapts to the type of the others (`if c { a } else { 1 }` with `a: Int8` is an `Int8`).

```fa
for item in items { }
for item, index in items { }        -- `index` for `Array`, `GrowingArray`, `OrderedColumnar` and `String`
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

- `break` leaves the innermost loop and `continue` skips to its next iteration. There are no labeled loops. A `loop` expression has the union type of its `break` values, and `Never` without `break`.
- `return` inside a `do` block returns from the enclosing function.
- `if`, `when`, `do` and `loop` are expressions; `for` and `while` are not.
- Adding or removing elements of a collection while iterating over it is an error, including through the implied effects of a relation. Over a `mutable` collection, the loop variable is a mutable reference: writing to it writes into the collection (except for `Set` elements and `Map` keys, which are read-only).

See [conditionals](/control-flow/conditionals), [loops](/control-flow/loops), [do](/control-flow/do).

## Functions

There is one form:

```
[async] function name = [<GenericParameters>](parameters): ReturnType throws Errors requires name: Type mutates path invalidates path => body
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

- The right side of `=` can be any function known at compile time: a lambda, an alias, a `then` composition, a `with` partial application (it binds parameters by name only), the result of a pure factory called with compile-time arguments.
- In a `then` composition, only the first step may take several parameters, and only it may mutate them.
- In a `function` declaration with a block body, the return type and every effect (`throws`, `requires`, `mutates`, `invalidates`) must be declared; a block body without a return type returns nothing, and its last value is dropped. With a single-expression body, they are inferred. The last expression of a block is its value; `return` exits early.
- Lambda parameters are always parenthesized. A lambda is accepted wherever a value is expected, except as the value of a declaration: `let f = (x) => x + 1` is an error, use `function` (`function f: Comparator = (a, b) => a - b` gives it a declared type). `let f = add` copies a plain function value. `Human.describe` is a function value taking the receiver first; `human.describe` without a call is an error: write a lambda. A lambda takes its parameter types, return type and effects from the expected function type, and infers the rest from its body. It may have fewer parameters than the expected type.
- One variadic parameter at most, in last position.
- Parameters are immutable references. A parameter declared with `mutates` is only modified from the inside: it can't be replaced, and a parameter of a primitive type cannot be mutated.
- The compiler knows whether a function stores (lets escape) each function parameter; this is part of its interface. Passing a callback to a parameter that doesn't escape adds the callback's effects to the call: `items.map(parse)` throws what `parse` throws.
- A closure that doesn't escape captures by reference, like in JavaScript. A closure that escapes (stored, returned, started) captures a copy of each value it uses, taken when it is created, including the values designated by parameters and `use` aliases. A closure can't capture a `mutable use`.
- A function type declares `async` and its effects (`async (request: Request) => Response throws HttpError`, `(player: Player) => Void mutates player`), and a function can only be stored in a function type that declares all of its effects. Parameters are matched by position (effect paths are renamed accordingly), parameters are contravariant and returns covariant, and a `=> Void` type accepts any return value. Named arguments and defaults don't go through function values.
- A place passed to a `mutates` or `invalidates` parameter must not overlap another argument of the call, a requirement the callee uses, a module-level value it uses, or a reference captured by a callback argument.
- A function is **pure** when it has no `throws`, no `requires`, no `mutates` or `invalidates` outside its own locals, is not `async`, reads no module-level `mutable` or reactive value, and calls no impure platform primitive (I/O, clock, randomness, `console`). The compiler infers it. See [pure functions](/functions/defining-functions).
- Functions are hoisted at the top level and in types; inside a block, a function is visible after its declaration.

### Effects

| Effect | Meaning |
| --- | --- |
| `throws ErrorA \| ErrorB` | the errors the function can fail with |
| `requires name: Type` | a value the function needs from its caller's environment |
| `mutates self`, `mutates input`, `mutates self.balance`, `mutates self[index]`, `mutates self.size`, `mutates players[..].score` | what the function mutates: a whole parameter, requirement or module-level value (`mutates people.size`), a field, an index given by a parameter, a field of every element (`[..]`). The place stays valid; the local references into the memory its old value owned are invalidated (none for a flat value) |
| `invalidates self[key]`, `invalidates self[index..]`, `invalidates element` | the function can remove precise elements, or shift them: the local references to one element or a range of elements are invalidated (it implies `mutates`). Paths are evaluated with the values before the call |

Paths are `root (.field | [index] | [range] | [..])*`, separated by commas; an index is a literal, a parameter, `self.length` or `self.size`, possibly plus or minus a constant. A declared effect covers its sub-paths. Calling a mutating function requires a `mutable` value. See [reference checking](/advanced/reference-checking).

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
- Errors are never part of the return type, and are never falsy. There is no `!` (only `!=`) and no postfix `?`.
- The built-in errors are `Errors` types with one member of the same name: `IndexError { index, length }`, `KeyNotFound { key }`, `DivisionByZero`, `ArithmeticError { message }`, `ParseError { input, message }`, `Utf8Error { position }` and `Cancelled`. `Error` only exposes `toString()`.
- An optional is an absence; an error is a failure. `?.` and `??` only apply to optionals.
- `try` also prefixes an assignment whose target can fail: `try scores[index] = 0`.
- A runtime check that the compiler could not prove away (an integer overflow, a failed `assert` outside tests), a stack overflow or an exhausted memory **stops the program**: `stop: <reason>` and the source location on the error output, and the exit code 101. An `assert` is always evaluated, and narrows the code after it. A stop can't be caught or thrown; in a test, it fails the test.

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

- Requirements are matched by the exact shape of their type (aliases are transparent, no subtype lookup): two named types with the same shape are the same requirement. A provided value is available from the `provide` statement to the end of its block; a nested block can provide another one. Providing a place is an alias; providing an expression owns the value until the end of the block.
- Requirements are resolved at the call site, never captured by a closure. A `watch` or a task keeps what was provided when it was created, and can't outlive the block that provided it. A `derived` can't use requirements (it would not be pure). The entry function, the top level of a standalone file and tests provide requirements; they don't declare them.
- A function that calls a function with requirements must provide them or declare them.
- A requirement can be mutated with the effect `mutates name`.
- A type whose fields are function pointers describes a service.
- Requirements cross `start`, but not worker boundaries.

See [requirements](/advanced/requirements).

## Memory

1. Assignment copies; the compiler turns a copy into a move when the source isn't used afterwards. A move is never observable.
2. `use` creates an alias; function parameters and iteration give references.
3. Where data lives is part of the type: a reference into `people` has the type `Person in people`. It is inferred for a `use`, and written in full in parameters, return types and relation fields.
4. A reference cannot outlive what it points into. After an operation that invalidates it (a mutation of what it points into, or an invalidation of its place), a local reference must be declared again before it is used (`use someone = people[0]`, `use score = scores.get("ana")`); this redeclaration is allowed in the same scope, and one inside an inner block ends with that block. There is no runtime check and no generation counter.
5. A field typed `T` or `T?` holds a copy (recursion through a union is stored on the heap). A field holding a reference is one side of a relation.
6. Values are destroyed at the end of their scope, in reverse order.

There is no garbage collector and no borrow checker.

```fa
mutable people: Pool<Person> = []

use ada = people.add(Person { name = "Ada" })  -- a reference: `Person in people`
let copy = ada                                 -- a copy
```

| Collection | Operation | Effect |
| --- | --- | --- |
| `Array` | `array[index] = value` | `mutates self[index]` |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` |
| | `pop` | `invalidates self[self.length - 1]` |
| | `push`, `insert`, `deleteWhere`, `clear`, `sortInPlace`, `reverseInPlace` | `mutates self` |
| `Map` | `map[key] = value` | `mutates self[key], self.size` |
| | `getOrInsert` | `mutates self.size` |
| | `delete(key)`, `take(key)` | `invalidates self[key]` |
| | `deleteWhere`, `clear` | `mutates self` |
| `Set` | `add`, `delete`, `take`, `deleteWhere`, `clear` | `mutates self` |
| `OrderedColumnar` | `add` | `mutates self` |
| | `[index] = value` | `mutates self[index]` |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` |
| | `pop` | `invalidates self[self.length - 1]` |
| | `deleteWhere`, `clear` | `mutates self` |
| `Pool`, `Columnar` | `add` | `mutates self.size` |
| | `delete(element)`, `take(element)` | `invalidates element` |
| | `deleteWhere`, `clear` | `mutates self` |
| `GrowingArray` | `push` | `mutates self.length` |
| | `array[index] = value` | `mutates self[index]` |
| `GrowingSet`, `GrowingMap`, `GrowingPool`, `GrowingColumnar` | `add` | `mutates self.size` |
| `GrowingMap` | `map[key] = value` | `mutates self[key], self.size` |

`reserve` declares `mutates self` on `Array`, `Set` and `OrderedColumnar`, and `mutates self.capacity` on every other collection, where it moves no item. An effect on `length` or `size` implies the same effect on `capacity`. See [capacity and reservation](/collections/arrays#capacity-and-reservation).

A mutation gives a new value to a place, which stays valid; it invalidates the local references into the memory the old value owned, deduced from its type: nothing for a flat value (a primitive, a union of literals, a relation reference, an object of flat fields), and the elements, string slices, union payload or recursive children otherwise. Mutating a whole collection invalidates every reference into it. An invalidation means the place itself may have disappeared, or may designate another element; it always names precise places. Both target every local reference that may designate the affected elements. The compiler keeps the exact key or index of each reference when it is known at compile time (like the range of an integer), and knows that a reference returned by `add` is distinct from every reference that existed before: the other references stay valid. The facts it uses are fixed (see [reference checking](/advanced/reference-checking)).

`Pool.add`, `GrowingPool.add` and `Columnar.add` return a reference. Outside the implementation of a collection, its `length` or `size` is read-only. Mutating a collection or its length or size while iterating over it is a compile error. `invalidates element` on an element of `collection` implies `mutates collection.size`.

After branches, a reference invalidated in any branch is invalidated; loops are checked until nothing changes. To repair a reference inside a branch or a loop, use a `mutable use` and `reuse`. A reference type names a path (`Person in town.people`): at a call, the parameter names of the callee's reference types are replaced by the arguments. A function returns a reference only when its return type says `in`. Local collections of references (`Array<Person in people>`) are not allowed: use indices or a relation. The exact rules are on the [reference checking](/advanced/reference-checking) page.

A stored reference is always one side of a relation, and the other side is always declared: a direct side (a reference field) and a reverse side `collection->field`, read as "the elements of `collection` whose `field` points at me". The collection is a field of the containing type or a module-level value. Both sides are elements of stable-address collections (`Pool`, `Columnar`, `GrowingPool`, `GrowingColumnar`; never `Array`, `Set`, `Map`). A direct side is optional or a `Pool`.

```fa
type Town = {
	people: Pool<Person> = []

	type Person = {
		name: String
		bestFriend: (Person in people)?
		bestFriendOf: Pool<people->bestFriend>
	}
}
```

| Relation | Direct side | Reverse side |
| --- | --- | --- |
| one-to-many | `bestFriend: (Person in people)?` | `bestFriendOf: Pool<people->bestFriend>` |
| one-to-one | `bestFriend: (Person in people)?` | `bestFriendOf: (people->bestFriend)?` |
| many-to-many | `friends: Pool<Person in people>` | `friendOf: Pool<in people->friends>` |
| symmetric | `spouse: (people->spouse)?` | itself |
| two types | `author: (User in users)?` (in `Post`) | `articles: Pool<posts->author>` (in `User`) |

- The reverse side is read-only. A write to the direct side detaches from the previous target, writes, and attaches to the new target, in O(1). One-to-one: the previous holder of the new target becomes `none`. Symmetric: `ana.spouse = bao` also sets `bao.spouse`, and detaches previous spouses.
- `mutates a.bestFriend` implies the related reverse sides, and the direct side of the previous holder for one-to-one and symmetric relations.
- Deleting an object eagerly sets to `none` (or removes from pools) every direct side that pointed at it, and removes it from the reverse sides of its targets: O(k) for k linked objects. No stale reference ever exists. These writes are implied effects of `delete`, `take`, `clear`, `deleteWhere` and of replacing the collection: they count for the iteration rule and for the overlap of arguments.
- A direct side is written only through a reference into the collection named by its reverse side, with a target of the same owner. A copy, or an element of another collection, has its relation fields set to `none` (or empty), and they are read-only. Copies keep their links only between collections copied together inside one owner.
- The pools of a relation hold links: `add(target)` and `delete(target)` return a `Boolean`, and a target is linked at most once.
- Deleting counts its implied effects: `for friend in ana.friends { town.people.delete(friend) }` is an error.
- A collection with owner fields is modified only through a path from its owner.
- An owner field (a field of the type that owns the collection, pointing into it, like a linked list's `firstChild`) has a single holder and no reverse side; it becomes `none` when its target is deleted.
- Copying an owner copies its internal relations, pointing at the copied objects. An object copied out of its collection, taken with `take` or added to another collection has no links.

See [memory management](/advanced/memory-management), [owned references](/advanced/owned-references), [relations](/advanced/relations), [reference checking](/advanced/reference-checking).

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

Dependencies are tracked automatically, at runtime. A `derived` expression must be pure; it is computed when it is read, and is always up to date. `watch` blocks run after the current statement of the running code and before each `await`, each at most once per change; a watch can't throw, and can't write a value it reads. Reactiveness belongs to places, not types: a copy of a reactive value is a plain value. `reactive` is part of a field, like `readonly`. Reactive and derived values can't be exported.

See [reactivity](/advanced/reactivity).

## Concurrency

```fa
async function fetchUser = (id: String): User throws HttpError => {
	let response = try await std.http.get("/users/{id}")
	...
}

let user = try fetchUser(id)          -- blocking
let user = try await fetchUser(id)    -- awaited, inside an async function
let task = start fetchUser(id)        -- started: a `Task<User>`
let user = try await task
task.cancel()

start do {
	-- runs in the background
}

for parallel item, index in items {
	results[index] = compute(item)    -- each iteration writes to its own index
}
```

- Async is colorless: an `async function` can be called blocking, awaited or started. On JavaScript targets, it is compiled lazily into `name___sync` and `name___async` versions. Calling an async-only primitive synchronously on a target is a compile error, and so is a blocking call that reaches a channel, a lock or a started task on JavaScript. Whether a function can be called blocking on a target is part of its interface.
- `async` is part of a function type; a lambda is async when its expected type is, and a non-async function can be stored in an async function type. `await` is only allowed in an `async function`, an async lambda, a `start do` block, a test and the top level of a standalone file. `await` takes an async call or a `Task`.
- A started task begins at the starter's next `await` (or when it returns to the event loop). Inside a blocking call, on every target, `await` blocks and `start` runs eagerly to a finished task.
- A `Task<T> throws E` declares its errors like a function. Tasks are combined with `Task.all`, `Task.any` and `Task.settled`, plain functions that return a `Task`.
- Tasks are structured: a `Task` can't be copied, and leaving its scope cancels it and waits for it. Cancellation happens at the next suspension point, `cancel()` waits for the task to stop, and awaiting a task that may have been cancelled can throw `Cancelled`. A `start` whose task is not kept is **detached**: it copies its arguments and what it captures, can't have `throws`, and the program waits for it before exiting.
- A started function receives references: until its task is awaited or cancelled, the values it reads can't be modified, and the values it modifies can't be used. This covers its arguments, the values a `start do` captures, the module-level values of its effects and its requirements. A task can't outlive the values it uses. At each `await`, what is known about module-level mutable values is forgotten.
- Tasks run on one thread, as in JavaScript; threads come from workers and `for parallel`.
- Mutexes, atomics and channels are declared `mutable`. They are thread-safe handles: their methods don't declare `mutates self`, so several tasks and threads can use them at once, and a copy shares the same value. The thread-safe types are `Mutex<T>`, `AtomicInteger`, `AtomicBoolean`, `Channel<T>` (from `std.concurrency`), and objects or tuples whose fields are all thread-safe. Only thread-safe types can be shared mutably between threads.
- Workers copy arguments and results across threads: references and relations cannot cross worker boundaries, thread-safe handles can. A worker runs one call at a time, and its handle can't be copied. `for parallel` and `parallel()` are native only.

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

Type<value>                          -- the type of a value
@typeToString(User)
@typeToSchema(User)

use weather = @import("https://example.com/weather/types.fa")
```

- `target` is a static value.
- `@` is written on every compile-time call, including inside type functions and `static` values. A compile-time call is pure, except that it may throw: the error becomes a compile error. Evaluation is bounded (100 million steps per call, 256 nested type functions).
- Compile-time evaluation is deterministic: no clock, no network, no randomness. The files it reads become build dependencies. Configuration files run on the same interpreter; only dependency sources and `@import` reach the network, through the lockfile.
- `@import` is only for remote modules.

See [metaprogramming](/advanced/metaprogramming).

## Foreign code

```fa
-- sqlite3.foreign.fa
use c = std.c

export namespace = {
	type Sqlite3 = CStruct { }
	function open: (path: CString, database: Pointer<Pointer<Sqlite3>?>) => c.Int mutates database = @extern("sqlite3_open")
	function close: (database: Pointer<Sqlite3>) => c.Int = @extern("sqlite3_close")
}
```

- Only `*.foreign.fa` files can declare C functions (`@extern("symbol")`), call them, use `Pointer<T>` and `CString`, and declare `destroy` in a type. A value whose type contains `Pointer` or `CString` can't be used outside foreign files. They are the only code the compiler trusts instead of proving.
- C types: the numbers, `Boolean`, `c.Int`, `c.Long`, `c.SizeT`... (`std.c`), `Pointer<T>` (never `NULL`) and `Pointer<T>?` (`none` is `NULL`), `CString`, `CStruct { ... }` (nominal, C layout; usable in every file), `CArray<T, n>`.
- For the duration of a call, an `Array<T>` is passed as a `Pointer<T>`, a `mutable` value as its address (declared with `mutates`), a `String` as a `CString`. C must not keep these pointers.
- A type with `destroy` (and every type containing one) is move-only; `destroy` runs when the value is destroyed.
- A C function pointer parameter accepts a function known at compile time that captures nothing and can't throw.
- `fa bind header.h` generates a foreign file once; builds never read C headers. Libraries are linked with `build.link`.
- `build.cLibrary` builds the package as a C library with a generated header: the functions of a foreign file's export become `prefix_snake_case` C functions, errors become status codes.
- Rust code is used through the C ABI: a crate built as a `staticlib` with `#[unsafe(no_mangle)] pub extern "C"` functions, declared with `@extern` and linked with `build.link` and `build.libraryPaths`. There is no `rust` target.
- Targets: `native`, `wasm`, `bun` call C; `web` and `node` don't (a compile error).

See [foreign code](/advanced/foreign-code).

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

- Tests live in `*.test.fa` files, next to the code they test, and are grouped with `namespace`. A test file exports nothing. Each test starts with freshly initialized module-level values, and a test body can `await`.
- `assert` is a keyword, without parentheses.
- `fa test` fails on a `test only` when the `CI` environment variable is set or on the default Git branch.
- Tests and `@test` blocks only exist in `*.test.fa` files. The tests of a file run in order; files may run in parallel. `@test` blocks run during `fa check` and `fa test`.
- Requirements are given to the code under test with `provide`.

See [testing](/architecture/testing).

## Modules

```fa
export = 42
export mutable: Pool<Person> = []  -- other files can modify it
export type = { ... }
export function = (x: Integer): Integer => x
export namespace = { ... }

use { formatDate } = utils
use { Button as UIButton, Card } = ui.components
library.greet("Ada")  -- from a file of `application/`
```

- One export per file, named after the file; inside the file, its own name designates its export. A file of `library/` or `application/` without an export is an error. An exported value is deeply immutable, unless it is exported with `export mutable`. The export forms are `export =`, `export mutable`, `export type`, `export function`, `export async function` and `export namespace`.
- A file name must be a valid identifier; names that differ only by case are an error.
- Sibling files and folders are available with no declaration; folders are namespaces, and `(folder)` groups files without creating a namespace. Files of a parent folder are not accessible.
- Root files and folders are available everywhere in their own root (`library/` or `application/`). `application/` sees `library/` only through the `library` namespace.
- Names are looked up in the local scopes, then among the siblings (including the contents of `(group)` folders), then among the root entries, then in the dependencies, then in the prelude (`console` with `log` and `error`, `Task`, `Math`, the built-in types and errors, and the `std` namespace). A `use` of a module path (`use { formatDate } = utils`) aliases a name and is never invalidated. Two entries with the same name at the same level are an error.
- Module-level values are initialized in dependency order, and destroyed in reverse order at exit; a cycle between their initializers is an error. Statements other than declarations are only allowed at the top of standalone files and scripts.
- `#name` members are private.
- `*.test.fa`, `*.worker.fa` and `*.foreign.fa` files are named by the part before their suffix; test files are never names.
- `application/main.fa` exports the entry function. It may be `async`, take `arguments: Array<String>` (empty on `web`; `arguments.get(0)` is the first argument) and return an exit code (`Integer`, ignored on `web`); its errors are declared with `throws`, its other effects are inferred. The top level of a standalone file or a script sees the same implicit `arguments`.
- A `mutable` member of an exported namespace can be read by every file but written only by its own file. `reactive`, `derived` and `static` values can't be exported. A generic export is written `export type<First, Second> = ...` or `export function = <Item>(...) => ...`.
- `(group)` folders can appear at any level: their contents merge into the parent folder. See [name lookup](/architecture/modules#name-lookup) and the [prelude](/architecture/modules#prelude).

See [modules](/architecture/modules).

### Projects

| Level | Contents |
| --- | --- |
| Standalone file | a `.fa` file; sibling files and folders are available, parent files are not |
| Package | `package.fa`, `dependencies.fa`, `library/`, `application/`, `scripts/` |
| Workspace | `workspace.fa` and `packages/` (scanned recursively) |

Configuration files are regular Fa files, executed at build time by the compile-time interpreter, that export an object of a declared type (`PackageConfiguration`, `WorkspaceConfiguration`):

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

- Targets: `web`, `bun`, `node`, `native`, `wasm`. Default: `web`, `bun`, `native`.
- The workspace configuration provides defaults that packages override field by field.
- Versions: `@4`, `@4.2`, `@4.2.1`, matched against the semver Git tags of the repository. One version is chosen per repository and major version; the dependencies of a dependency are not visible. Resolved versions are locked in one `dependencies.lock` per workspace, package or standalone file, with SHA-256 content hashes.
- A configuration file never uses the dependencies it declares. A dependency is a `DependencySource`: a string, or `{ source, generate }`.

See [packages](/architecture/packages), [workspaces](/architecture/workspaces), [configuration](/architecture/configuration), [dependencies](/architecture/dependencies).

### Command line

| Command | |
| --- | --- |
| `fa run [--target=...] <file.fa> [arguments...]` | compile a standalone file or a script for `bun` (or the target) and run it; everything after the file goes to the program |
| `fa new package <name>`, `fa new workspace [<name>]` | scaffold a package or a workspace |
| `fa develop [<package>] [--target=...]` | run with hot reloading |
| `fa test [<package>] [--watch] [--coverage] [--target=...]` | run the tests |
| `fa check [<package>]` | check every file for every configured target and run the compile-time tests |
| `fa format [<files>...] [--check]` | format the code |
| `fa build [<package> \| <file.fa>] [--target=...] [--output=<directory>]` | compile into `<directory>/<target>/` (`build/` by default) |
| `fa preview [<package>] [--target=...]` | run the compiled output |
| `fa update [<dependency>...]` | update the locked dependencies |
| `fa bind <header.h> [--output <file>]` | generate a foreign file from a C header |
| `fa help [<command>]`, `fa version` | help and version |

Commands exit with `0` on success and `1` on failure; `fa run` and `fa preview` exit with the program's code (its returned value or `0`, `1` for an escaped error, `101` for a stop). See [command line](/architecture/overview#command-line).

## Keywords

```
and          as           assert       async        await        break
catch        continue     derived      do           else         export
false        for          function     if           in           invalidates
is           let          loop         mutable      mutates      namespace
none         not          or           parallel     provide      reactive
readonly     requires     return       reuse        start        static
test         then         throw        throws       true         try
type         use          watch        when         while        with
```

Contextual keywords: `skip` and `only` (after `test`). Special names: `self` (the receiver of a method) and `Self` (the type being declared).
