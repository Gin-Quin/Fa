# Conditionals

## `if`

Syntax: `if <expression> { <code> } [else if <expression> { <code> }] [else { <code> }]`

### `if` expression

## `when`

The `when` keyword is used to create a conditional expression that evaluates to one of several possible values based on the result of an expression.

Syntax: `when <expression> { <branches> }`

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
let foo: Number | String = 3

let result = when foo is {
	Number => "number"
	String => "string"
}

console.log(result) -- "number"
```

And with explicit unions:

```fa
union MyUnion = {
	SomeString = String
	SomeNumber = Number
	SomeObject = {
		baz: String
		qux: Number
	}
}

let foo = MyUnion.SomeObject {
	baz = "hello"
	qux = 42
}

let result = when foo is {
	MyUnion.SomeNumber >> value => "number: {value}"
	MyUnion.SomeString >> value => "string: {value}"
	MyUnion.SomeObject >> { baz, qux } => {
		"object: {{ baz: {baz}, qux: {qux} }}"
	}
}
```
