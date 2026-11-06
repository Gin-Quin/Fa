# Conditionals

## `if`

```
if <expression> { <code> } [else if <expression> { <code> }] [else { <code> }]
```

```fa
let temperature = readTemperature()

if temperature > 30 {
	console.log("hot")
} else if temperature > 15 {
	console.log("mild")
} else {
	console.log("cold")
}
```

The condition needs no parentheses. The braces are always required, even for a single statement, and `else` is written on the same line as the `}` that closes the previous branch: an `else` at the start of a line is an error.

The condition must be a `Boolean`: Fa has no truthiness, so a number, a string or an optional value is never accepted as a condition. Optional values are checked explicitly with `is none` or `is not none`. See [logical operators](/operations/logical#no-truthiness).

```fa
let user = findUser(id) -- User?

if user is not none {
	console.log(user.name)
}

if items.length > 0 {
	console.log("not empty")
}
```

### `if` as an expression

`if` evaluates to the value of the branch that ran:

```fa
let label = if score >= 50 { "pass" } else { "fail" }
```

Two rules apply when `if` is used as an expression:

- an `else` branch is required, so that the expression always has a value,
- with an expected type (an annotation, a parameter, a return type), every branch is checked against it. Without one, the type of the expression is the union of the types of the branches, each widened to its base type (`"pass"` is a `String`, `1` is an `Integer`).

```fa
let value = if condition { 1 } else { "one" } -- Integer | String
let ratio: Float = if condition { 1 } else { 0.5 } -- Float: the literal `1` adapts to the expected type
let mixed = if condition { 1 } else { 0.5 } -- error: an `Integer` and a `Float` literal, with no expected type
```

Without an expected type, a literal branch first adapts to the type of the other branches when it fits: with `level: Int8`, `if valid { level } else { 0 }` is an `Int8`. The union must be a valid type: two integer types whose ranges overlap (`Int8 | UInt8`) or indistinguishable members are an error, which an annotation solves.

A branch that leaves with `return`, `throw`, `break` or `continue` has the type `Never`, which adds nothing to the union:

```fa
let size = if text is not none { text.length } else { return 0 } -- Integer
```

Multi-line branches work the same way, with the last expression of each block being its value:

```fa
let clamped = if value > maximum {
	console.log("clamping")
	maximum
} else {
	value
}
```

Used as a statement, an `if` drops the values of its branches, and needs no `else`.

## Narrowing

Testing a value narrows its type where the test is known to be true, and where it is known to be false:

```fa
function describe = (value: String | Integer): String => {
	if value is String {
		return "text of length {value.length}" -- `value` is a String here
	}

	return "number {value}" -- `value` is an Integer here
}
```

Narrowing is a fact that the compiler keeps about a place, as long as nothing can change it. It never declares a new value: the narrowed value is used under its own name.

### What can be narrowed

- a local value (declared with `let`, `mutable` or `use`) or a parameter, including `self`,
- a path of fields of one of them, like `user.address` or `self.owner`, as long as nothing in between can modify the path.

A module-level value is narrowed like a local value, but the narrowing is also dropped at each `await` (another task may modify it).

### Tests that narrow

| Test | Narrows to |
| --- | --- |
| `value is Type`, `value is not Type` | the member(s) of the union that match, or the others |
| `value is none`, `value == none` | `None`, and the base type in the other branch |
| `value is "ready"`, `value == "ready"` | the literal, and the union without it in the other branch |
| `value is 0..60` | the range: an `Integer<min = 0, max = 59>` |
| `value != 0` | an integer that excludes `0` |
| `value is in ["admin", "owner"]` | the literals of the array (a literal array only) |
| `value is Shape.Circle` | the member of the named union |
| `shape.kind == "circle"` | the members of an anonymous union of objects whose `kind` field is `"circle"` |

`==` and `!=` narrow only when one side is a literal, a [`static`](/values/declaring-values#compile-time-values) value (which keeps its literal type) or `none`. An optional compared with a literal is narrowed to that literal: after `if maybe == 42`, `maybe` is `42`, not `Integer?`. `is` accepts every pattern of [`when`](#when): types, literals, union members, [ranges](/values/ranges) and alternatives (`value is "y" | "yes"`).

```fa
if status is "ready" {
	start()
}

if score is 0..60 {
	console.log("fail")
}

if divisor != 0 {
	let share = total // divisor -- can't fail: `divisor` excludes `0`
}
```

An integer keeps a range and a set of excluded values. When the false branch of a range test is not a single range, the compiler keeps the excluded values: after `if score is 0..60 { return }`, a `score: Integer<min = 0, max = 100>` is an `Integer<min = 60, max = 100>`; after `if divisor == 0 { return }`, `divisor` is an `Integer` that excludes `0`.

`is Type` narrows a union. On a value whose type is not a union, the compiler already knows the answer: a test that is always true is a warning, a test that is never true is an error. Testing the members of an anonymous union at runtime requires that they can be told apart: see [unions](/types/unions).

### Combining tests

- `a and b`: `b` is checked with `a` narrowed as true, and so is the body of the `if`.
- `a or b`: `b` is checked with `a` narrowed as false, and so is the `else` branch.
- `not a` swaps the true and the false narrowings of `a`.

```fa
if user is not none and user.age >= 18 {
	console.log("{user.name} is an adult")
}

if user is none or user.name == "" {
	return
}

console.log(user.name) -- `user` is a `User` here
```

### Leaving a branch early

When a branch ends with `return`, `throw`, `break` or `continue`, the code after the `if` is narrowed as if the test was false:

```fa
function greet = (user: User?) => {
	if user is none {
		return
	}

	console.log(user.name) -- `user` is a `User` here
}
```

`assert` narrows the rest of the block the same way: after `assert user is not none`, `user` is a `User`.

The condition of a `while` narrows its body:

```fa
mutable current: Integer? = readNumber()

while current is not none {
	console.log(current + 1) -- `current` is an `Integer` here
	current = readNumber() -- the narrowing ends here
}
```

### When narrowing ends

A narrowing is dropped:

- by an assignment of the place, or of a prefix of its path (`user = other` ends the narrowing of `user.address`),
- by a call that may replace the place: a function that declares `mutates user` may assign `user.address`, so it ends the narrowings of the paths of `user`; a parameter typed `User?` may stand for the optional itself, so passing `user` to it ends the narrowing of `user`,
- at an `await`, for a module-level value.

Changing the narrowed value itself keeps the narrowing: a method that mutates it (`items.push(1)`), a function that mutates a parameter of its narrowed type, or a write of one of its fields can't make it `none` or another member.

```fa
type Order = { lines: Array<String>?, note: String? }

function addLine = (order: Order, line: String) mutates order => {
	if order.lines is none {
		order.lines = [line] -- the assignment narrows `order.lines` too
		return
	}

	order.lines.push(line)
	order.lines.push("(end)") -- still narrowed: `push` changes the array, not its presence
}
```

A closure sees the narrowings of the places it captures, as they are when it is created. Inside the closure, a write to the place drops its narrowing, as does a write by another callback given to the same call.

```fa
mutable user: User? = findUser(id)

if user is not none {
	user = findUser(otherId) -- `user` is `User?` again
	console.log(user.name) -- error: `user` may be `none`
}
```

## `when`

`when` matches a value against a series of patterns.

```
when <expression> is { <branches> }
```

```fa
let foo = randomNumber()

when foo is {
	1 => "one"
	2 => "two"
	3 => "three"
	else => {
		"other"
	}
}
```

The subject is evaluated once. Branches are separated by newlines (commas are accepted too, and the formatter turns them into newlines). A branch body is either a single expression or a block: after `=>`, a `{` always opens a block (see [braces](/getting-started/syntax-basics#braces)). The first branch whose pattern matches is the one that runs.

### `when` as an expression

```fa
let foo = 3

let result = when foo is {
	1 => "one"
	2 => "two"
	3 => "three"
	else => "other"
}

console.log(result) -- "three"
```

The type of a `when` expression follows the rules of [`if` as an expression](#if-as-an-expression): every branch is checked against the expected type, or the result is the union of the widened types of the branches. A branch of type `Never` (one that ends with `return` or `throw`) adds nothing. Used as a statement, a `when` drops the values of its branches.

### Patterns

| Pattern | Example |
| --- | --- |
| a literal | `1`, `"yes"`, `true`, `none` |
| a type | `Integer`, `String`, `User` |
| a member of a named union, an enumeration or an error type, always qualified | `Shape.Circle`, `Color.red`, `ApiError.NotFound` |
| a whole named union or error type | `ApiError` |
| a range, with any integer expressions as bounds | `0..60`, `90..`, `..=0`, `0..limit` |
| alternatives | `"y" \| "yes"`, `Shape.Circle \| Shape.Square` |

A range pattern is written in place. A `Range` stored in a value is tested with `is in`, not `is`: `score is in passingRange` (a name after `is` is a type). Only ranges with literal bounds count for exhaustiveness.

Members of a named union are always written with the name of the union, even when the type of the subject makes it obvious: `ApiError.NotFound =>`, never `NotFound =>`.

There are no tuple patterns, no nested patterns and no guards: match on the type or the member, then destructure it with `as` and test the rest with an `if`.

### Matching types

`when` matches against types as readily as against values. This works with anonymous unions:

```fa
let foo: Integer | String = 3

let result = when foo is {
	Integer => "integer"
	String => "string"
}

console.log(result) -- "integer"
```

And with named [unions](/types/unions):

```fa
type MyUnion = Union {
	SomeString = String
	SomeNumber = Integer
	SomeObject = {
		baz: String
		qux: Integer
	}
}

let foo = MyUnion.SomeObject {
	baz = "hello"
	qux = 42
}

let result = when foo is {
	MyUnion.SomeNumber as number => "number: {number}"
	MyUnion.SomeString as string => "string: {string}"
	MyUnion.SomeObject as { baz, qux } => "object: {{ baz: {baz}, qux: {qux} }}"
}
```

Inside the branch, the bound value has the narrowed type of the member.

### Binding the value of a branch with `as`

Use `as` after the pattern of a branch to give a name to the matched value, or to [destructure](/operations/destructuring) it. `as` never converts a value: it only gives it a name.

```fa
type Position = { x: Float, y: Float }

type Shape = Union {
	Point
	Circle = { radius: Float }
	Rectangle = { width: Float, height: Float }
	Segment = Tuple<Position, Position>
}

let area = when shape is {
	-- no binding
	Shape.Point => 0.0

	-- the whole value, with its narrowed type
	Shape.Circle as circle => circle.radius * circle.radius * 3.14

	-- destructuring an object
	Shape.Rectangle as { width, height } => width * height

	-- destructuring a tuple
	Shape.Segment as (start, end) => 0.0
}
```

A name bound with `as` exists only in its branch, and may shadow a name of the enclosing scope.

The subject itself is not narrowed by a branch: the narrowed value comes from `as`. The one exception is a subject that is a plain local name (a local value or a parameter): it is also [narrowed](#narrowing) inside each branch, and to the remaining members in the `else` branch.

```fa
let message = when error is {
	ApiError.Timeout => "timeout"
	ApiError.NotFound => "missing {error.id}" -- `error` is a local name: it is narrowed to `ApiError.NotFound`
}

let label = when loadShape() is {
	Shape.Circle as circle => "circle of radius {circle.radius}" -- the subject is a call: use `as`
	else => "another shape"
}
```

`as` bindings only exist in `when`. In an `if`, the narrowed value is used under its own name.

### Matching ranges and alternatives

[Ranges](/values/ranges) are patterns. `..` excludes the upper bound, `..=` includes it:

```fa
let label = when score is {
	0..60    => "fail"
	60..80   => "pass"
	80..=100 => "excellent"
	else     => "out of range"
}
```

Combine several patterns with `|`. The branch runs when any of them matches:

```fa
let answer = when key is {
	"y" | "yes" => "accepted"
	"n" | "no" => "refused"
	else => "unknown"
}
```

Alternatives work with types and union members too:

```fa
let hasArea = when shape is {
	Shape.Circle | Shape.Rectangle => true
	Shape.Point | Shape.Segment => false
}
```

`|` is the loosest part of a pattern: `as` applies to the whole alternative. The bound value has the union of the alternatives, and destructuring it with `as { ... }` only accepts the fields that every alternative has:

```fa
let name = when shape is {
	Shape.Circle | Shape.Rectangle as surface => describeSurface(surface) -- `Shape.Circle | Shape.Rectangle`
	else => "no surface"
}
```

### No guards

A branch has no extra condition: there is no `if` after a pattern. To test a condition, bind the value and use an `if` inside the branch:

```fa
let description = when value is {
	Integer as number => if number < 0 { "negative" } else { "positive or zero" }
	String => "text"
}
```

This keeps exhaustiveness simple: every branch counts, and the compiler always knows whether a case is handled.

### Exhaustiveness

`when` must handle every possible value, as an expression and as a statement. A missing case is a compile error:

```fa
let foo: Integer | String = 3

let result = when foo is {
	Integer => "integer"
	-- error: `String` is not handled
}
```

Add the missing branch, or an `else`. Without `else`, the branches must cover:

| Type of the subject | Covered by |
| --- | --- |
| a named union or an error type | every member (or the whole type) |
| an enumeration | every member |
| an anonymous union, an optional | every member type, `none` included |
| `Boolean` | `true` and `false` |
| an integer type | literals and ranges that cover its whole range: `..0` and `0..` cover `Integer`, and `0..=100` covers an `Integer<min = 0, max = 100>` (see [ranged integers](/values/numbers#minimum-and-maximum-values)) |
| a literal union, like `"north" \| "south"` | every literal |
| `Float`, `String` and the other types | an `else` is always needed |

What the compiler knows about the subject counts: after `if count < 0 { return }`, the branches `0` and `1..` cover `count`. Float literals can be used as patterns: they match with `==`, so a `NaN` pattern never matches.

A branch whose patterns are all matched by earlier branches can never run: it is a warning, and so is an `else` when the other branches already cover every value. A pattern that can never match the subject, like `"yes"` on an `Integer` or a type that is not one of its members, is an error.

:::tip
For a named union, prefer listing every member over writing `else`. When a member is added later, the compiler shows you every `when` that needs to change.
:::

Unreachable-arm warnings consider only earlier arms, not facts known before the `when`. After `if x > 10 { return }`, an arm matching `20` is therefore not warned about. A redundant `else` is warned about when earlier arms cover every member of a named union, enumeration, Boolean or literal union.

### Chains of conditions

`when` always has a subject. To choose between unrelated conditions, use `if` / `else if`:

```fa
let grade = if score >= 90 {
	"A"
} else if score >= 80 {
	"B"
} else if score >= 70 {
	"C"
} else {
	"F"
}
```

When the conditions are ranges of a single value, match the value instead:

```fa
let grade = when score is {
	90.. => "A"
	80..90 => "B"
	70..80 => "C"
	else => "F"
}
```
