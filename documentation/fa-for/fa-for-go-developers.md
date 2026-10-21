# Fa for Go developers

Go and Fa share a taste for small languages and explicit error handling. Fa keeps both, adds a real type system, and removes the garbage collector.

## Syntax at a glance

| Go | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `x := 1` | `let x = 1` (immutable) / `mutable x = 1` |
| `func add(a int) int { return a }` | `function add = (a: Int): Int => a` |
| `[]T` | `Array(T)` |
| `map[K]V` | `Map(K, V)` |
| `*T` (optional) | `T?` |
| `nil` | `none` |
| `(T, error)` | `T \| SomeError` |
| `if err != nil { return err }` | `!` |
| `switch` | `when` |
| `for i := range xs` | `for xs >> x, i` |
| `for { }` | `loop { }` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `go f()` | `start f()` |
| `chan T` | `Channel(T)` |
| `interface{ Read() }` | `Type({ read: () => Bytes })` |
| `import "fmt"` | nothing — files see their siblings |

## Errors, without the ceremony

Go's `if err != nil` is the right idea; Fa keeps the semantics and removes the repetition.

```go
// Go
data, err := load()
if err != nil {
	return nil, err
}
```

```fa
let data = load()!
```

The signature carries the same information, and more of it — the error type is specific, not a bare `error`:

```fa
type ApiError = Errors {
  Timeout = { after: Int }
  Network = { message: String }
}

function load = (): Data | ApiError => ...
```

Handling instead of propagating is exhaustive, so a new error member breaks compilation until you deal with it:

```fa
when load() is {
  Data >> data => use(data)
  Timeout => retry()
  Network >> error => report(error.message)
}
```

There is no `panic`/`recover` for ordinary failure, and no way to ignore an error with `_`.

## No nil pointer dereferences

Go's `nil` inhabits every pointer, map, slice and interface. Fa's `none` inhabits only types that opted in with `?`:

```fa
let user: User = loadUser()   -- cannot be none
let user: User? = findUser()  -- may be none

console.log(user.name)        -- error: `user` may be none

if user is not none {
  console.log(user.name)      -- fine, narrowed
}
```

## No garbage collector

Objects live on the stack; collections own the heap; everything is released at the end of its scope. There is no GC, no pauses, and no runtime.

The trade-off is that assignment copies:

```fa
mutable a = Point { x = 1 }
mutable b = a
b.x = 2
console.log(a.x) -- 1
```

Take a reference when you want sharing:

```fa
use b = a
```

Function parameters and loop bindings are already references, so idiomatic Go code translates without extra copies. See [memory management](../advanced/memory-management).

## Structural typing, everywhere

Go's implicit interfaces are the feature Fa generalises. In Go, a type satisfies an interface by having the methods. In Fa, *every* type relationship works that way, for fields as well as methods:

```fa
function readId = (Item: Type({ id: Int }), item: Item): Int => item.id
```

There is no `interface` keyword because there is nothing to declare.

## Generics use parentheses

```go
func First[T any](items []T) (T, bool)
```

```fa
function first = (Item: Type, items: Array(Item)): Item? => ...

first([1, 2, 3])
first(Item = Decimal, [1, 2, 3])
```

## Concurrency

Goroutines and channels map closely:

```fa
use { Channel } = std.concurrency

async function producer = (mutable channel: Channel(String)) => {
  for 1..=10 >> i {
    await channel.send("item {i}")!
  }
  channel.close()
}

async function main = () => {
  mutable channel = Channel(String, capacity = 10)

  await [
    start producer(channel)
    start consumer(channel)
  ]
}
```

`start` is `go`, and `Channel(T, capacity = n)` is a buffered channel. Ranging over a channel until it closes:

```fa
while await channel.receive() >> item {
  console.log(item)
}
```

Data races are impossible by construction: values crossing a thread boundary are copied, and references cannot cross at all. `go vet` has nothing left to find.

For CPU-bound work, `for parallel` beats hand-rolled worker goroutines:

```fa
for parallel items >> item, index {
  results[index] = expensiveComputation(item)
}
```

## Modules

Go's package system is close to Fa's, minus the import block. Files see their siblings and the package root automatically, folders are namespaces, and each file exports exactly one thing:

```fa
export function = (x: Int): Int => x + 1
```

A `#` prefix is Fa's lowercase: `#helper` is private to its namespace.

## What you gain

- A type system that models your domain: unions, ranged integers, optionals, exhaustive matching
- No `nil` panics, no unhandled errors, no data races
- No GC pauses, and a binary with no runtime
- Compile-time execution instead of `go generate`
- Built-in reactivity
- One source compiled to native, WebAssembly and JavaScript

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%` |
| `//` comments | `--`; `//` is integer division |
| Zero values | there are none; a value is initialised or optional |
| `_` to ignore an error | not allowed; handle it or propagate with `!` |
| `defer` | scope-based release; `use` values are freed at the end of the block |
| `panic`/`recover` | errors are values |
| Slices sharing an array | assignment copies; slicing copies |
| `append` in a loop | reserve with `Array(T, capacity = n) {}` |
