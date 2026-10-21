# Fa for Go developers

Go and Fa share a taste for small languages, one obvious way to write things, and explicit error handling. Fa keeps all three, adds a real type system, and removes the garbage collector.

Fa is designed for AI agents as much as for humans: everything a function does — the errors it throws, the values it requires, the data it mutates — is written in its signature, and if the code compiles, it works.

## Syntax at a glance

| Go | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `x := 1` | `let x = 1` (immutable) / `mutable x = 1` |
| `func add(a int) int { return a }` | `function add = (a: Integer): Integer => a` |
| `int64`, `float64`, `byte` | `Integer`, `Float`, `Byte` |
| `float64(count)` | `Float(count)` |
| `[]T` | `Array(T)` |
| `map[K]V` | `Map(K, V)` |
| `Point{X: 1, Y: 2}` | `Point { x = 1, y = 2 }` |
| `*T` (optional) | `T?` |
| `nil` | `none` |
| `func load() (T, error)` | `function load = (): T throws LoadError => ...` |
| `return nil, err` | `throw err` |
| `if err != nil { return err }` | `try` |
| `switch` | `when` |
| `for i, x := range xs` | `for x, i in xs` |
| `for i := 0; i < n; i++` | `for i in 0..n` |
| `for { }` | `loop { }` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `x & y`, `x << n` | `x.bitAnd(y)`, `x.shiftLeft(n)` |
| `go f()` | `start f()` |
| `chan T` | `Channel(T)` |
| `make(chan T, 10)` | `Channel(T) { capacity = 10 }` |
| `interface{ Read() []byte }` | a structural constraint: `{ read: () => Array(Byte) }` |
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
let data = try load()
```

The signature carries the same information, and more of it — the error type is specific, not a bare `error`:

```fa
type ApiError = Errors {
	Timeout = { after: Integer }
	Network = { message: String }
}

function load = (): Data throws ApiError => ...
```

Handling instead of propagating is exhaustive, so a new error member breaks compilation until you deal with it:

```fa
let data = load() catch error => when error is {
	Timeout => cachedData
	Network as { message } => {
		report(message)
		return
	}
}
```

Errors are values, returned to the caller as in Go — `throw` exits the function like `return`, and `try` propagates at each level. There is no `panic`/`recover` for ordinary failure, and no way to ignore an error with `_`. See [errors](/values/errors).

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

## No zero values

A value is always initialised. A field with a default declares it, and a field that may be absent is optional:

```fa
type Config = {
	host: String
	port: Integer(min = 1, max = 65535) = 8080
	proxy: String?
}
```

## No garbage collector

Objects live on the stack; collections own the heap; every value is released at the end of its scope. There is no GC, no pauses, and no runtime.

The trade-off is that assignment copies:

```fa
mutable a = Point { x = 1, y = 2 }
mutable b = a
b.x = 3
console.log(a.x) -- 1
```

Create an alias when you want sharing:

```fa
use b = a
```

Function parameters and loop bindings are already references, so idiomatic Go code translates without extra copies. A function that modifies a parameter declares it with `mutates` — the Go pointer receiver, made explicit in the signature:

```fa
function rename = (user: User, name: String) mutates user => {
	user.name = name
}
```

See [memory management](/advanced/memory-management).

## Structural typing, everywhere

Go's implicit interfaces are the feature Fa generalises. In Go, a type satisfies an interface by having the methods. In Fa, *every* type relationship works that way, for fields as well as functions:

```fa
function readId(Item: { id: Integer }) = (item: Item): Integer => item.id
```

There is no `interface` keyword because there is nothing to declare. See [structural typing](/advanced/structural-typing).

## Generics use parentheses

```go
func First[T any](items []T) (T, bool)
```

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)

first([1, 2, 3])
first(Float)([1, 2, 3])
```

Generic parameters come right after the name, like Go's square brackets. They are inferred from the arguments and the expected type. See [generics](/types/generics).

## Concurrency

Goroutines and channels map closely:

```fa
use { Channel } = std.concurrency

async function producer = (channel: Channel(String)) mutates channel => {
	for index in 1..=10 {
		await channel.send("item {index}")
	}
	channel.close()
}

async function main = () => {
	mutable channel = Channel(String) { capacity = 10 }

	await Task.all([start producer(channel), start consumer(channel)])
}
```

`start` is `go`, and `Channel(T) { capacity = n }` is a buffered channel. Ranging over a channel until it closes:

```fa
while let item = await channel.receive() {
	console.log(item)
}
```

Data races are impossible by construction: values crossing a thread boundary are copied, references cannot cross at all, and only thread-safe types (channels, mutexes, atomics) can be shared mutably. `go vet` has nothing left to find.

For CPU-bound work, `for parallel` beats hand-rolled worker goroutines:

```fa
for parallel item, index in items {
	results[index] = expensiveComputation(item)
}
```

See [concurrency](/advanced/concurrency).

## Modules

Go's package system is close to Fa's, minus the import block. Files see their siblings and the package root automatically, folders are namespaces, and each file exports exactly one thing:

```fa
export function = (x: Integer): Integer => x + 1
```

A `#` prefix is Fa's lowercase: `#helper` is private to its namespace. To shorten a long path, create aliases: `use { formatDate } = utils`. See [modules](/architecture/modules).

## What you gain

- A type system that models your domain: unions, ranged integers, optionals, exhaustive matching
- Typed errors and requirements, written in the signature
- No `nil` panics, no unhandled errors, no data races
- No GC pauses, and a binary with no runtime
- Compile-time execution instead of `go generate`
- Built-in reactivity
- One source compiled to native, WebAssembly and JavaScript

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`, floor-based: `-7 %% 3 == 2` |
| `/` between integers truncates | `/` always returns a `Float`; `//` is floor division |
| `//` comments | `--`; `//` is floor division |
| Zero values | there are none; a value is initialised, has a default, or is optional |
| `_` to ignore an error | not allowed; handle it with `catch` or propagate it with `try` |
| `defer` | scope-based release: every value is freed at the end of its block |
| `panic`/`recover` | errors are typed values: `throws`, `try`, `catch` |
| Slices sharing an array | slicing copies (`xs[1..4]`); assignment copies |
| `append` in a loop | reserve with `Array(T).from([], capacity = n)` |
| Labeled `break` | there are no labeled loops |
