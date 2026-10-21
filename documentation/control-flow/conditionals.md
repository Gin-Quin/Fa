# Conditionals

## `if`

Syntax: `if <expression> { <code> } [else if <expression> { <code> }] [else { <code> }]`

The condition must be a `Boolean`: see [logical operators](/operations/logical).

```fa
let temperature = readTemperature()

if temperature < 0 {
	console.log("Freezing")
} else if temperature < 20 {
	console.log("Cold")
} else {
	console.log("Warm")
}
```

### `if` expression

`if` can be used as an expression. Each branch evaluates to its last expression:

```fa
let count = items.length
let label = if count == 0 { "empty" } else { "not empty" }
```

## `when`

The `when` keyword is used to create a conditional expression that evaluates to one of several possible values based on the result of an expression.

Syntax: `when <expression> is { <branches> }`

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

You can use `when` as an expression:

```fa
let foo = 3 

let result = when foo is {
	1 => "one"
	2 => "two"
	3 => "three"
	else => {
		"other"
	}
}

console.log(result) -- "three"
```

When working with unions, you can use a type to check if a value is of a certain type.

It works with implicit unions:

```fa
let foo: Integer | String = 3

let result = when foo is {
	Integer => "integer"
	String => "string"
}

console.log(result) -- "integer"
```

And with explicit unions:

```fa
union MyUnion = {
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
	MyUnion.SomeObject as { baz, qux } => {
		"object: {{ baz: {baz}, qux: {qux} }}"
	}
}
```

### Binding the value of a branch with `as`

Use `as` after the type of a branch to give a name to the matched value, or to [destructure](/operations/destructuring) it:

```fa
when shape is {
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
