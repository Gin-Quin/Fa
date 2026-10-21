# Functions

## Defining functions

In Fa, there is one way to declare a named function: the `function` keyword.

```fa
function add = (a: Integer, b: Integer): Integer => a + b
```

The syntax is close to TypeScript's arrow functions:

```ts
// in TypeScript
const add = (a: number, b: number): number => a + b
```

The complete form of a function declaration is:

```fa
[async] function name = (parameters): ReturnType throws Errors requires Requirements => body
```

- `async` marks a function that can be awaited. See [concurrency](/advanced/concurrency).
- `: ReturnType` is the type of the returned value.
- `throws Errors` lists the errors the function can fail with. See [errors](/values/errors).
- `requires Requirements` lists the values the function needs from its caller's environment. See [requirements](/advanced/requirements).

Every part except the name, the parameters and the body is optional, following the rules below.

### Return type, errors and requirements

If the body is a **single expression**, the return type, the errors and the requirements are inferred:

```fa
-- valid: the return type is inferred
function addOne = (x: Integer) => x + 1
```

If the body is a **block**, they must be declared explicitly. A block without a return type returns nothing.

```fa
-- valid: explicit return type
function addOne = (x: Integer): Integer => {
	let result = x + 1
	result
}

-- invalid: the block returns a value but has no return type
function addOne = (x: Integer) => {
	x + 1
}

-- valid: returns nothing
function greet = (name: String) => {
	console.log("Hello, {name}!")
}
```

This way, the signature of a function with a non-trivial body always documents everything the function can do.

### Parameters

Parameters are declared with a name and a type. They can have a default value:

```fa
function greet = (name: String, greeting: String = "Hello") => {
	console.log("{greeting}, {name}!")
}
```

Arguments can be passed by position or by name:

```fa
greet("John")
greet("John", "Hi")
greet(name = "John", greeting = "Hi")
```

Parameters are immutable references: they are never copied when passed to a function. A parameter declared `mutable` can be modified from inside the function (its fields or elements, but not the parameter itself). See [declaring values](/values/declaring-values#function-parameters).

### Hoisting

At the top level of a file and inside a type, the order of declarations does not matter: a function can be used before it is declared.

Inside a block, a function is available after its declaration, and it can use the values declared before it in the block.

```fa
function main = () => {
	let greeting = "Hello"

	function greet = (name: String) => console.log("{greeting}, {name}!")

	greet("John")
}
```

## Anonymous functions

An anonymous function (also called a lambda) uses the same syntax, without the `function` keyword and the name. Parameters always have parentheses.

Anonymous functions can only be used where a value is expected: as an argument, a return value or a field value.

```fa
let doubled = [1, 2, 3].map((x) => x * 2)

function makeAdd = (y: Integer) => (x: Integer) => x + y
```

When the types of the parameters are known from the context, they can be omitted.

A named function must always be declared with `function`. Assigning an anonymous function to a value is a compile error:

```fa
let addOne = (x: Integer) => x + 1 -- error: use `function addOne = (x: Integer) => x + 1`
```

## Function expressions

The right side of a `function` declaration can be any function that is known at compile time, not only an anonymous function.

An alias of another function:

```fa
function myFunctionClone = myFunction
```

A [composition](/functions/function-composition) of functions:

```fa
function addAndMultiply = add then multiply
```

A partial application, using the `with` keyword to bind some parameters:

```fa
function xyz = (x: Integer, y: Integer, z: Integer) => x + y + z

function xz = xyz with (y = 0)
```

The result of a function factory, when its arguments are known at compile time:

```fa
function makeAdd = (y: Integer) => (x: Integer) => x + y

function add3 = makeAdd(3)
```

If the arguments are only known at runtime, the result is a regular value:

```fa
let add = makeAdd(readNumber())
```

## Function types

The type of a function is written like its signature, without the body:

```fa
type Comparator = (a: Integer, b: Integer) => Integer
type Callback = (message: String) => Void
type Loader = (id: String) => User throws LoadError requires database: Database
```

Parameter names are optional in function types: `(Integer, Integer) => Integer`.

`Void` is the return type of a function that returns nothing. It's only written in function types: in a function declaration, a block without a return type already returns nothing.

## Methods and function pointers

Inside a type, the keyword makes the difference between methods and function pointers:

- `function` declares a function that **belongs to the type**. It is a **method** if its first parameter is `self`, and an **associated function** otherwise.
- `name: Type` declares a **field**, stored in each instance. A field can hold a function: it is then a **function pointer**.

```fa
type Counter = {
	value: Integer

	-- field: a function pointer, stored in each instance
	format: (value: Integer) => String

	-- associated function (no `self` parameter)
	function create = (value: Integer): Counter => Counter {
		value
		format = (value) => "{value}"
	}

	-- method
	function double = (self): Integer => self.value * 2

	-- mutating method (only callable on mutable instances)
	function increment = (mutable self) => {
		self.value += 1
	}

	-- method calling a function pointer
	function display = (self) => console.log(self.format(self.value))
}
```

```fa
mutable counter = Counter.create(12)

counter.increment()
counter.display() -- "13"

counter.format = (value) => "Value: {value}"
counter.display() -- "Value: 13"
```

Rules:

1. `self` must be the first parameter and has no type annotation.
2. Methods are not stored in instances and cannot be replaced. Function pointers are stored in each instance and can be set or replaced like any other field.
3. Methods can only be declared in `type` declarations. Anonymous objects only contain data, which can include function pointers.

## Destructuring parameters

A parameter can be destructured, like in TypeScript: write the extracted fields, then the type of the parameter.

```fa
type SomeObject = {
	foo: String
	bar: Integer
	baz: Boolean
}

function someFunction = (prefix: String, { foo, bar }: SomeObject) => {
	console.log(prefix, foo, bar)
}

someFunction("Values:", { foo = "Hello", bar = 42, baz = true })
```

### Anonymous structure parameter

When the type of the object is not declared elsewhere, write its fields and their types directly in the parameter. Each field becomes a local value, and fields can have default values:

```fa
function greet = ({ name: String, greeting: String = "Hello" }) => {
	console.log("{greeting}, {name}!")
}

greet({ name = "John" })
greet({ name = "Jane", greeting = "Hi" })
```

There is one form per situation:

- with a **named** type, destructure it: `({ x, y }: Point)`,
- with an **anonymous** type, use an anonymous structure parameter: `({ x: Integer, y: Integer })`.

Destructuring a parameter with an inline anonymous type is valid, but the compiler emits a warning, and the formatter rewrites it into an anonymous structure parameter:

```fa
-- warning: use an anonymous structure parameter
function greet = ({ name, greeting }: { name: String, greeting: String }) => {
	console.log("{greeting}, {name}!")
}

-- after formatting
function greet = ({ name: String, greeting: String }) => {
	console.log("{greeting}, {name}!")
}
```

:::note
Unlike TypeScript, `{ foo: String }` in a parameter never renames a field: in Fa, `:` is always followed by a type. Fields are renamed with `as`: `({ foo as bar }: SomeObject)`. See [destructuring](/operations/destructuring#renaming-fields).
:::

Destructured parameters can't keep a name for the whole value. If you need it, take the parameter as a whole and [destructure](/operations/destructuring) it in the body:

```fa
function readPoint = (point: Point) => {
	let { x, y } = point
	console.log(x, y, point)
}
```
