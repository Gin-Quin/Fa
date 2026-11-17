# Destructuring

Destructuring declares several values at once from an object, an array, a tuple or a namespace. The syntax is close to TypeScript's, with `as` to rename a field and `=` for a default value.

```fa
let point = { x = 1, y = 2, z = 3 }

let { x, y } = point
```

## Declaring values

Destructuring works with the three declaration keywords:

- `let` to store the extracted values immutably under the [copy and move rules](/advanced/memory-management#copies-moves-unique-values-and-shared-handles),
- `mutable` to store them mutably under the same rules,
- `use` to create [aliases](/advanced/memory-management#aliases), without copying.

Pattern bindings in `when` and destructuring use the same ownership rule as a field in the same position: `use` borrows, an ordinary stored binding copies, and a stored structure moves at last use or needs an explicit source copy. A `mutable use` can't be destructured: declare each reference on its own line.

```fa
let object = { a = 1, b = 2, nested = { c = 3 } }

let { a, b } = object          -- immutable copies
mutable { a, b } = object      -- or mutable copies
use { a, b } = object          -- or aliases of `object.a` and `object.b`

let { nested { c } } = object  -- nested destructuring
```

### Default values

A default value is used when the field is missing from the type of the value, or when it holds `none`; it is only evaluated in that case. The binding is then never optional. Defaults only exist in the object patterns of `let` and `mutable`: a `use` alias must designate a field, and an array item has no `none` to replace.

```fa
type Options = { retries: Integer?, verbose: Boolean }

let options: Options = { retries = none, verbose = true }

let { retries = 3, verbose, timeout = 30 } = options
-- `retries` is an Integer: 3, since the field holds `none`
-- `timeout` is an Integer: 30, since `Options` has no `timeout` field
```

### Rest

`...rest` collects the remaining fields into a new object, or remaining items into a new array. It owns those values, even with `use`. Ordinary data copies; structures must move from an exclusive last-use owner or be explicitly copied, and unique values cannot be duplicated. Write `copy` on the source when keeping it usable and an extracted/rest value is a structure:

```fa
let { name, ...details } = copy person -- `details` has every field of `person` except `name`
let [first, ...others] = copy items    -- `others` is a new array without the first item
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

Renaming works the same way in every destructuring: declarations, imports, parameters, loops and `when` branches.

:::note
Unlike TypeScript or Rust, `as` never converts a value to another type in Fa: it only gives a name. Conversions are explicit function calls, like `Float(count)`.

Unlike TypeScript, `:` never renames a field either: in Fa, `:` is always followed by a type.
:::

### Arrays and tuples

Arrays are destructured with brackets, tuples with parentheses:

```fa
let array = [1, 2, 3]
let [first, second] = array

let tuple = (1, "one")
let (number, name) = tuple
```

The extra items of an array are ignored: `[first, second]` takes the first two items of `[1, 2, 3]`. An array pattern reads items by index, so it follows the rules of [indexing](/collections/arrays#smart-bound-checks): when the compiler can't prove that the array has enough items, the destructuring `throws IndexError`. Like an assignment whose target can fail, the declaration is then prefixed with `try`; to handle the error instead, check the length first:

```fa
function firstTwo = (values: Array<Integer>): Tuple<Integer, Integer> throws IndexError => {
	try let [first, second] = values
	(first, second)
}

if values.length >= 2 {
	let [first, second] = values -- proven: no error possible
}
```

`try` is only written before an array pattern: before any other declaration, it is an error (write `let item = try items[index]`, not `try let item = items[index]`).

A tuple pattern must have exactly the number of elements of the tuple, and has no rest: name every element.

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

Imported members can be renamed with `as`, and nested namespaces can be destructured at once:

```fa
use { Button as UIButton, Card } = ui.components

use {
	cows
	machines { tractor, harvester }
} = farm
```

See [modules](/architecture/modules) for how namespaces are created from files and folders.

## Loops

In a `for` loop, each element of a collection can be destructured with the same patterns:

```fa
for { name, email } in users {
	console.log("{name} <{email}>")
}

for { name as userName, age } in people {
	console.log("{userName} is {age}")
}

for (key, value) in scores {
	console.log("{key}: {value}")
}
```

A map is iterated as `(key, value)` tuples. See [loops](/control-flow/loops). Array patterns can't be used in a `for` loop, since they could fail on each element: destructure inside the body.

As without destructuring, the names are references to the parts of each element: over a `mutable` collection, writing to them writes into the collection (except for the keys of a map, which are read-only).

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

See [conditionals](/control-flow/conditionals).

## Function parameters

A parameter can be destructured by writing the pattern, then the type:

```fa
function greet = (prefix: String, { name, email }: User) => {
	console.log("{prefix} {name} <{email}>")
}
```

When the type of the parameter is anonymous, write an anonymous structure parameter instead, with the type of each field:

```fa
function greet = ({ name: String, greeting: String = "Hello" }) => {
	console.log("{greeting}, {name}!")
}
```

See [defining functions](/functions/defining-functions#destructuring-parameters).

## Picking fields of a type

Destructuring extracts values. To build a type from some fields of another type, use the `Pick` and `Omit` [type functions](/types/type-functions):

```fa
type Contact = Pick<User, { name, email }>
type PublicUser = Omit<User, { password }>
```

:::implementation
LLVM binds the analyzer's `PatternInfo` in source order after evaluating the
source once. Destructured function parameters bind their patterns from the
received argument before later parameters and the body use those names. Object rest projects the fields of the analyzer's rest type in
their declared layout, using the existing value-copy helpers. Array rest uses
the same owned slice operation as range indexing, after the pattern's single
length check. Rest bindings participate in the ordinary local or module cleanup,
including in loop bodies and async frames. A loop establishes its iteration
cleanup scope before creating a rest binding, so owned fields are released on
fallthrough, `continue` and `break`.
An optional field with a default branches on presence and joins in storage of
the binding's nonoptional type, including for a nested pattern. Only the absent
branch evaluates the default; both branches use the ordinary ownership helpers.
JavaScript modules collect named, nested and rest bindings when declaring module
values and exporting their shared initialization function.
:::
