# Generics

A generic is a value parameterised by a **type**. Fa has no separate generics system: a type parameter is an ordinary parameter whose type is `Type`, and applying type arguments is an ordinary call.

```fa
Array(Int)
Map(String, User)
Int(min = 0, max = 200)
```

This is why Fa uses parentheses and not angle brackets: `Array(Int)` really is a call to `Array`.

## Generic types

A generic type is a function from types to a type:

```fa
type Pair = (First: Type, Second: Type) => {
  first: First
  second: Second
}

type StringAndInt = Pair(String, Int)

let value = StringAndInt {
  first = "hello"
  second = 12
}
```

A parameter is a **type parameter** when its annotation is `Type`. Nothing else marks it — not its name, not its capitalization.

## Generic functions

Type parameters live in the ordinary parameter list:

```fa
function first = (Item: Type, items: Array(Item)): Item? => {
  if items.length == 0 {
    return none
  }
  return items[0]
}

let value = first([1, 2, 3]) -- Item is inferred as Int
```

Because they are ordinary parameters, generics work identically for hoisted functions, lambdas, methods and mutable function values:

```fa
function identity = (Value: Type, value: Value): Value => value
let identity      = (Value: Type, value: Value): Value => value
mutable identity  = (Value: Type, value: Value): Value => value
```

## Passing type arguments explicitly

**A type parameter is never positional.** It is either inferred, or passed by name:

```fa
first([1, 2, 3])                -- inferred
first(Item = Decimal, [1, 2, 3]) -- explicit
first(Decimal, [1, 2, 3])        -- error: type arguments must be named
```

This is what keeps argument positions stable. Positional arguments always line up with the runtime parameters, in order, whether or not any type parameter was given.

Pass a type argument explicitly when inference has nothing to work from, or when you want a wider type than the one that would be inferred:

```fa
let empty = Array(Int) {}                      -- nothing to infer from
let numbers = first(Item = Decimal, [1, 2, 3]) -- widen Int to Decimal
```

## Constraints

`Type` on its own accepts any type. Applying it to a constraint restricts what may be passed:

```fa
-- Item must be an object with a `bar` field holding an Int
function readBar = (Item: Type({ bar: Int }), item: Item): Int => item.bar
```

Constraints are [structural](../advanced/structural-typing): any type with the required shape is accepted, no declaration of intent required.

Combine constraints with the intersection operator:

```fa
function store = (Item: Type(Serializable & Identifiable), item: Item) => ...
```

## Default type arguments

Type parameters can have defaults, like any parameter:

```fa
type Collection = (Item: Type, Index: Type = Int) => {
  get(self, index: Index): Item?
}

type Names = Collection(String)              -- Index is Int
type NamesByKey = Collection(String, String) -- Index is String
```

## Compile-time value parameters

A parameter prefixed with `@` must be given a value the compiler can compute. Unlike type parameters, `@` parameters are never inferred, so they stay positional.

```fa
type Matrix = (Element: Type, @rows: UInt, @columns: UInt) => {
  values: Array(Element, length = rows * columns)
}

type Transform = Matrix(Decimal, 4, 4)
type Transform = Matrix(Element = Decimal, rows = 4, columns = 4)
```

Applied to a function, `@` parameters let the compiler specialise the generated code:

```fa
function repeat = (@count: UInt, value: String): String => {
  mutable result = ""
  @for 0..count {
    result += value
  }
  return result
}

repeat(3, "ab") -- compiled to `"ab" + "ab" + "ab"`
```

## Inference

Type arguments are inferred from the runtime arguments and from the expected return type. Inference never guesses between two equally valid answers: when a type parameter is ambiguous, the compiler reports it and asks you to name it.

```fa
function parse = (Output: Type, input: String): Output | ParseError => ...

let count: Int = parse("42")!            -- inferred from the expected type
let count = parse(Output = Int, "42")!   -- inferred from the argument
let count = parse("42")!                 -- error: `Output` is ambiguous
```

## Types versus values

`Array(Int)` produces a type, but `Point(3, 4)` produces a value. The rule is:

- calling a **type function** applies type arguments and yields a **type**,
- calling a **type** constructs a **value**.

```fa
type Point = { x: Int, y: Int }              -- a type
type Pair = (A: Type, B: Type) => { ... }    -- a type function

Point(3, 4)              -- a value: Point is a type
Pair(Int, String)        -- a type: Pair is a type function
Pair(Int, String) { ... } -- a value of that type
```

The callee's declaration decides, exactly as it does for any other call.

## What Fa deliberately leaves out

To keep the type system predictable, Fa has no:

- user-defined specialisation or overloading,
- higher-kinded types,
- variance annotations,
- implicit conversions used to satisfy a constraint,
- overload resolution based on the return type alone.

Generic types, generic functions, structural constraints, compile-time values, defaults and inference cover the cases these features are usually reached for.
