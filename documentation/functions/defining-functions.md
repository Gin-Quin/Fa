# Defining functions

A function is an ordinary value. It is written with the fat arrow `=>`, and declared with the same syntax as any other value.

```fa
let myFunction = (): ReturnType => { }
```

The `function` keyword declares a function that is immutable and **hoisted**, so it can be used before it is declared:

```fa
function myFunction = (): ReturnType => { }
```

You can also use `let` or `mutable`. In that case the function is not hoisted, and follows the ordinary rules of the keyword you used:

```fa
let myImmutableFunction = (): ReturnType => { }

mutable myMutableFunction = (): ReturnType => { }
```

A single parameter may drop its parentheses:

```fa
function increment = value => value + 1
```

## Return type

The return type may be omitted when the body is a single expression, or when the function returns nothing. Otherwise it is required.

```fa
-- valid: the type of a single expression is inferred
function addOne = (x: Int) => x + 1

-- valid: returns nothing
function logIt = (x: Int) => {
  console.log(x)
}

-- invalid: a block that returns a value must declare its type
function addOne = (x: Int) => {
  return x + 1
}

-- valid
function addOne = (x: Int): Int => {
  return x + 1
}
```

## Returning a value

The value of a function is the value of its last expression. `return` is available for returning early.

```fa
function classify = (value: Int): String => {
  if value < 0 {
    return "negative"
  }

  if value == 0 {
    return "zero"
  }

  "positive" -- the last expression is the returned value
}
```

## Parameters

Function parameters are **immutable references**. They are never copied when passed.

```fa
function increaseFoo = (input: { foo: Int, bar: Int }) => {
  -- error: cannot mutate an immutable reference
  input.foo += input.bar
}
```

Use `mutable` to declare a mutable reference:

```fa
function increaseFoo = (mutable input: { foo: Int, bar: Int }) => {
  input.foo += input.bar -- works
}
```

You cannot pass an immutable value to a mutable parameter:

```fa
let myValue = { foo = 12, bar = 2 }

increaseFoo(myValue) -- error: `myValue` is immutable

mutable myMutableValue = { foo = 12, bar = 2 }

increaseFoo(myMutableValue) -- works
```

A mutable parameter allows **interior mutability** only: you can change the fields of an object or the items of a collection, but you cannot replace the whole thing.

```fa
function increaseFoo = (mutable input: { foo: Int, bar: Int }) => {
  -- error: cannot reassign a parameter, even a mutable one
  input = { foo = input.foo + input.bar, bar = input.bar }

  -- works
  input.foo += input.bar
}
```

### Default values

```fa
function greet = (name: String, greeting: String = "Hello") =>
  console.log("{greeting}, {name}!")

greet("Ana")
greet("Ana", "Hi")
```

### Named arguments

Any parameter can be passed by name. Positional arguments come first, named arguments after.

```fa
function move = (x: Int, y: Int, speed: Int = 1) => ...

move(1, 2)
move(1, 2, speed = 3)
move(x = 1, y = 2, speed = 3)
move(1, speed = 3, y = 2)
```

### Variadic parameters

A parameter prefixed with `...` collects the remaining arguments into an array. There can be at most one, and it must come last.

```fa
function sum = (...values: Int): Int => {
  mutable total = 0
  for values >> value {
    total += value
  }
  return total
}

sum(1, 2, 3)
```

Spread an existing collection into a variadic parameter with `...`:

```fa
let numbers = [1, 2, 3]
sum(...numbers)
```

## Generic functions

A parameter annotated `Type` is a type parameter. Type parameters are inferred, or passed by name — never positionally.

```fa
function first = (Item: Type, items: Array(Item)): Item? => {
  if items.length == 0 { return none }
  return items[0]
}

first([1, 2, 3])                 -- Item inferred as Int
first(Item = Decimal, [1, 2, 3]) -- Item given explicitly
```

Constrain a type parameter by applying `Type` to a shape:

```fa
function readBar = (Item: Type({ bar: Int }), item: Item): Int => item.bar
```

See [generics](../types/generics) for the full picture, including compile-time value parameters.

## Destructuring parameters

Extract fields from a parameter with `>>`:

```fa
type SomeObject = {
  foo: String
  bar: Int
  baz: Boolean
}

function someFunction = (SomeObject >> { foo, bar }) => {
  console.log(foo, bar)
}

someFunction({ foo = "Hello", bar = 42 })
```

This works for several parameters at once:

```fa
function someFunction = (
  SomeObject >> { foo, bar }
  SomeObject >> { baz }
) => {
  console.log(foo, bar, baz)
}

someFunction({ foo = "Hello", bar = 42 }, { baz = true })
```

You can also declare an anonymous object as a parameter:

```fa
function someFunction = ({ foo: String, bar: Int = 12 }) => {
  console.log(foo, bar)
}

someFunction({ foo = "Hello", bar = 42 })
```

## Function types

The type of a function is written with `=>`:

```fa
type Transform = (Int, Int) => Int
type Handler = (Request) => Response
type Callback = () => None
```

Parameter names may be included for documentation, and are ignored when comparing types:

```fa
type Transform = (left: Int, right: Int) => Int
```

## Methods and lambdas

Inside an object, a function can be declared in two ways. The only difference is the `=`:

```fa
type Counter = {
  count: Int = 0

  -- an associated function: no `self`
  zero(): Counter => Counter { count = 0 }

  -- a method: reads `self`
  isEmpty(self): Boolean => self.count == 0

  -- a mutable method: writes through `self`
  increment(mutable self) => {
    self.count += 1
  }

  -- a lambda field: reassignable, stored per instance
  onChange = (self) => console.log(self.count)
}
```

Methods cost no memory per instance and cannot be overridden. Lambdas can be replaced instance by instance. Prefer methods. See [objects](../values/objects#methods-and-lambdas).

## Closures

A function can use values from the scope that surrounds it.

```fa
function makeCounter = (start: Int) => {
  mutable count = start
  return () => {
    count += 1
    return count
  }
}
```

A closure **captures by copy**, consistent with the rest of Fa: taking a value into a closure copies it, exactly like assigning it.

```fa
mutable total = 0

let addToTotal = (value: Int) => {
  total += value -- error: `total` is captured by copy and cannot be written
}
```

To share state, put it where sharing is safe: return it, pass it as a mutable parameter, or store it in a collection.

```fa
function makeAccumulator = () => {
  mutable total = 0            -- owned by the closure
  return (value: Int): Int => {
    total += value
    return total
  }
}
```

Capturing a **reference** with `use` is allowed only when the compiler can prove the closure does not outlive it — in practice, when the closure is called immediately or passed to a function that does not store it:

```fa
mutable numbers = [1, 2, 3]

use first = numbers[0]
numbers.forEach((value: Int) => console.log(value + first)) -- fine

let escaping = () => first -- error: this reference would outlive `numbers`
```

## Function factories

A function that returns a function is written directly:

```fa
function makeAdd = (y: Int) => {
  return (x: Int) => x + y
}

-- or on one line
function makeAdd = (y: Int) => (x: Int) => x + y

let add3 = makeAdd(3)
let add5 = makeAdd(5)
```

Use `let` for the result, unless the factory's arguments are known at compile time — a hoisted `function` must be resolvable at compile time.

## Function aliases

```fa
function myFunctionClone = myFunction
```

This is most useful when composing:

```fa
function addAndMultiply = add ||> multiply
```

## Binding parameters

`with` produces a new function with some parameters already supplied:

```fa
function xyz = (x: Int, y: Int, z: Int): Int => x + y + z

function xz = xyz with (y = 0)

xz(1, 2) -- 3
```

## Recursion

A hoisted `function` can call itself:

```fa
function factorial = (n: UInt): UInt => {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}
```

A function declared with `let` cannot, because its name is not in scope inside its own body.

## Async functions

A function marked `async` can be called synchronously or asynchronously. See [concurrency](../advanced/concurrency).

```fa
async function fetchData = (url: String): Response | HttpError => {
  return await httpClient.get(url)
}
```
