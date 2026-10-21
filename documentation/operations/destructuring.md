# Destructuring

Destructuring declares several values at once from an object, an array, a tuple or a namespace. The syntax is the same as TypeScript's.

```fa
let point = { x = 1, y = 2, z = 3 }

let { x, y } = point
```

## Declaring values

Destructuring works with the three declaration keywords:

- `let` to copy the extracted values into immutable values,
- `mutable` to copy them into mutable values,
- `use` to create [aliases](/advanced/memory-management#aliases), without copying.

```fa
let object = { a = 1, b = 2, nested = { c = 3 } }

let { a, b } = object          -- immutable copies
mutable { a, b } = object      -- mutable copies
use { a, b } = object          -- aliases of `object.a` and `object.b`

let { nested { c } } = object  -- nested destructuring
let { a, d = 4 } = object      -- default value, if `d` is optional or missing
```

To get a single value, use regular field access: `let a = object.a`.

### Renaming fields

Use `as` to give a different name to an extracted field:

```fa
let { a as first, b as second } = object

use { Button as UIButton } = ui.components

function draw = ({ x as left, y as top }: Point) => {
	console.log(left, top)
}

for { name as userName } in users {
	console.log(userName)
}
```

Renaming works the same way in every destructuring: declarations, imports, parameters and loops.

:::note
Unlike TypeScript or Rust, `as` never converts a value to another type in Fa: it only gives a name. Conversions are explicit function calls, like `Float(count)`.
:::

### Arrays and tuples

```fa
let array = [1, 2, 3]
let [first, second] = array

let tuple = (1, "one")
let (number, name) = tuple
```

## When is the type written?

The type name is only written when it can't be inferred:

- in a declaration or a loop, the type comes from the value, so it's never written: `let { x, y } = getPoint()`,
- in a function parameter, the type has no other source, so it's written after the pattern, like in TypeScript: `({ x, y }: Point)`. If the type is anonymous, use an [anonymous structure parameter](/functions/defining-functions#anonymous-structure-parameter) instead: `({ x: Integer, y: Integer })`,
- in a `when` branch, the type name selects the union variant, and the value is bound with `as`: `Shape.Circle as { radius } =>`.

## Imports

Importing values from a namespace is destructuring it with `use`, since an import is an alias:

```fa
use { Button, Card, Modal } = ui.components

Button({ label = "Click me" })
```

Nested namespaces can be destructured at once:

```fa
use {
	cows
	machines { tractor, harvester }
} = farm
```

See [modules](/architecture/modules) for how namespaces are created from files and folders.

## Loops

In a `for` loop, each element of a collection can be destructured:

```fa
for { name, email } in users {
	console.log("{name} <{email}>")
}
```

See [loops](/control-flow/loops).

## Unions

In a `when` expression, the value of a branch is bound or destructured with `as`:

```fa
when shape is {
	Shape.Circle as { radius } => radius * radius * 3.14
	Shape.Rectangle as { width, height } => width * height
	Shape.Segment as (start, end) => 0.0
}

when value is {
	MyUnion.SomeNumber as number => "number: {number}"
	MyUnion.SomeString as string => "string: {string}"
}
```

See [conditionals](/control-flow/conditionals#binding-the-value-of-a-branch-with-as).

## Function parameters

A parameter can be destructured by writing the pattern, then the type:

```fa
function greet = (prefix: String, { name, email }: User) => {
	console.log("{prefix} {name} <{email}>")
}
```

See [defining functions](/functions/defining-functions#destructuring-parameters).
