# Conditionals

## `if`

```
if <expression> { <code> } [else if <expression> { <code> }] [else { <code> }]
```

```fa
if temperature > 30 {
  console.log("hot")
} else if temperature > 15 {
  console.log("mild")
} else {
  console.log("cold")
}
```

The condition needs no parentheses. The braces are always required, even for a single statement.

A condition can be any value, not only a `Boolean` — see [truthiness](../operations/logical#truthiness).

### `if` as an expression

`if` evaluates to the value of the branch that ran:

```fa
let label = if score >= 50 { "pass" } else { "fail" }
```

Two rules apply when `if` is used as an expression:

- an `else` branch is required, so that the expression always has a value,
- every branch must produce a compatible type; the result type is the union of the branches.

```fa
let value = if condition { 1 } else { "one" } -- Int | String
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

Testing a value narrows its type inside the branch:

```fa
function describe = (value: String | Int): String => {
  if value is String {
    return "text of length {value.length}" -- `value` is a String here
  }

  return "number {value}" -- `value` is an Int here
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

A branch body is either a single expression or a block.

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
let foo: Int | String = 3

let result = when foo is {
  Int => "number"
  String => "string"
}

console.log(result) -- "number"
```

And with named unions:

```fa
type MyUnion = Union {
  SomeString = String
  SomeNumber = Int
  SomeObject = {
    baz: String
    qux: Int
  }
}

let foo = MyUnion.SomeObject {
  baz = "hello"
  qux = 42
}

let result = when foo is {
  SomeNumber >> value => "number: {value}"
  SomeString >> value => "string: {value}"
  SomeObject >> { baz, qux } => "object: baz={baz}, qux={qux}"
}
```

Members of a named union may be written unqualified inside `when`. Use `MyUnion.SomeNumber` when the union is not clear from context.

### Extracting with `>>`

`>>` binds the matched value, or destructures it:

```fa
when shape is {
  Circle >> circle => circle.radius
  Rect >> { width, height } => width * height
}
```

### Matching ranges and multiple patterns

```fa
let label = when score is {
  0..60    => "fail"
  60..80   => "pass"
  80..=100 => "excellent"
  else     => "out of range"
}
```

Combine patterns with `|`:

```fa
when key is {
  "y" | "yes" => true
  "n" | "no" => false
  else => none
}
```

### Guards

Add a condition to a branch with `if`:

```fa
when value is {
  Int >> n if n < 0 => "negative"
  Int => "positive or zero"
  String => "text"
}
```

A branch with a guard does not count towards exhaustiveness, because the compiler cannot know whether the guard holds.

### Exhaustiveness

`when` must handle every possible value. A missing case is a compile error:

```fa
let foo: Int | String = 3

let result = when foo is {
  Int => "number"
  -- error: `String` is not handled
}
```

Add the missing branch, or an `else`.

:::tip
For a named union, prefer listing every member over writing `else`. When a member is added later, the compiler shows you every `when` that needs to change.
:::

### `when` without a subject

Omitting the subject turns `when` into a chain of conditions, which reads better than a long `else if` ladder:

```fa
let label = when {
  score >= 90 => "A"
  score >= 80 => "B"
  score >= 70 => "C"
  else => "F"
}
```
