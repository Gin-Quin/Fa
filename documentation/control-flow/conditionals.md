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

The condition needs no parentheses. The braces are always required, even for a single statement.

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
- every branch must produce a compatible type; the result type is the union of the branches.

```fa
let value = if condition { 1 } else { "one" } -- Integer | String
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

### Narrowing

Testing a value with `is` narrows its type inside the branch:

```fa
function describe = (value: String | Integer): String => {
	if value is String {
		return "text of length {value.length}" -- `value` is a String here
	}

	return "number {value}" -- `value` is an Integer here
}
```

`is` accepts the same patterns as the branches of `when`: types, literals and [ranges](/values/ranges).

```fa
if status is "ready" {
	start()
}

if score is 0..60 {
	console.log("fail")
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

A branch body is either a single expression or a block. The first branch whose pattern matches is the one that runs.

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
	Segment = Tuple(Position, Position)
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

`when` must handle every possible value. A missing case is a compile error:

```fa
let foo: Integer | String = 3

let result = when foo is {
	Integer => "integer"
	-- error: `String` is not handled
}
```

Add the missing branch, or an `else`. When matching numbers or strings with literals or ranges, an `else` branch is usually needed, unless the type of the subject is a [ranged integer](/values/numbers#minimum-and-maximum-values) whose values are all covered.

:::tip
For a named union, prefer listing every member over writing `else`. When a member is added later, the compiler shows you every `when` that needs to change.
:::

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
