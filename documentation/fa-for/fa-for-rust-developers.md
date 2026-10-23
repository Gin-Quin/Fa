# Fa for Rust developers

Fa shares Rust's goals — memory safety without a garbage collector, errors as values, zero-cost abstractions — and reaches them with a much smaller model. There is no borrow checker, no lifetime syntax, and no smart pointer zoo. Its syntax sits between Rust and TypeScript.

Fa is designed for AI agents as much as for humans: like Rust, if the code compiles, it works — and everything a function does (the errors it throws, the values it requires, the data it mutates) is written in its signature.

## Syntax at a glance

| Rust | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `let x = 1;` | `let x = 1` |
| `let mut x = 1;` | `mutable x = 1` |
| `fn add(a: i64) -> i64 { a }` | `function add = (a: Integer): Integer => a` |
| `\|x\| x + 1` | `(x) => x + 1` |
| `i64`, `f64`, `u8` | `Integer`, `Float`, `Byte` |
| `x as f64` | `Float(x)` |
| `Vec<T>` | `Array(T)` |
| `HashMap<K, V>` | `Map(K, V)` |
| `Option<T>` / `None` | `T?` / `none` |
| `fn load() -> Result<T, E>` | `function load = (): T throws E => ...` |
| `return Err(e)` | `throw e` |
| `load()?` | `try load()` |
| `load().unwrap_or(0)` | `load() catch error => 0` |
| `match x { }` | `when x is { }` |
| `for x in xs` | `for x in xs` |
| `for (i, x) in xs.iter().enumerate()` | `for x, i in xs` |
| `while let Some(x) = stack.pop()` | `while let x = stack.pop()` |
| `1..10`, `1..=10` | `1..10`, `1..=10` |
| `x % y` | `x %% y` |
| `x & y`, `x >> n` | `x.bitAnd(y)`, `x.shiftRight(n)` |
| `!x` | `not x` |
| `&&`, `\|\|` | `and`, `or` |
| `&mut T` parameter | a parameter declared with `mutates` |
| `Box<T>` | nothing — recursion through a union is stored on the heap automatically |
| `impl Trait for T` | structural types; no traits |
| `assert_eq!(x, y)` | `assert x == y` |
| `use std::collections::HashMap;` | nothing — files see their siblings |

## No borrow checker

Fa's memory model has five rules instead of ownership and borrowing:

1. Objects live on the **stack**; collections own the **heap**.
2. Assignment **copies** (the compiler turns it into a move when the source is dead).
3. `use` creates an **alias**, a reference which cannot outlive what it points into.
4. Function parameters and loop bindings are **already references**.
5. Values are dropped at the end of scope, in reverse order.

```rust
// Rust
let mut v = vec![1, 2, 3];
let first = &v[0];
v.push(4);        // error: cannot borrow `v` as mutable
println!("{first}");
```

```fa
mutable v = [1, 2, 3]
use first = v[0]
v.push(4)
console.log(first) -- error: `first` was invalidated by `v.push`: declare it again with `use first = v[...]`
```

The same error, for the same reason — but the type is `Integer in v`, not `&'a i64`, and you never write it. There is one rule rather than a system of aliasing rules: a reference into a collection is invalidated by any operation that declares `invalidates` on it (`push`, `insert`, `delete`, `clear`...). After that, you check it with `is not none` (bags, sets, maps) or, for an array, declare it again. See [owned references](/advanced/owned-references#after-invalidation).

## `&mut` becomes an effect

Parameters are immutable references by default. A function that modifies one declares it in its signature with `mutates`, and the call site needs a `mutable` value — but no `&mut` marker:

```fa
function addBonus = (scores: Array(Integer), bonus: Integer) mutates scores => {
	for score in scores {
		score += bonus -- writes into the array
	}
}

mutable scores = [10, 20, 30]
addBonus(scores, 5)
```

Methods are declared inside their type, and a method that changes its object declares `mutates self`:

```fa
type Counter = {
	value: Integer = 0

	function increment = (self) mutates self => {
		self.value += 1
	}
}
```

## No smart pointers

Fa has no `Box`, `Rc`, `Arc`, `RefCell`, `Cell`, `Cow` or `Weak`. Recursive and shared structures are built two ways.

**A union**, including an optional, when each value owns its children. Recursion through a union is stored on the heap automatically, so there is no `Box` to write:

```fa
type Tree = Union {
	Leaf = { value: Integer }
	Branch = { left: Tree, right: Tree }
}

type ListNode = {
	value: Integer
	next: ListNode?
}
```

Direct recursion (`next: ListNode`) is an error: the type would have an infinite size.

**A collection plus references**, when nodes are shared:

```fa
mutable nodes: Bag(Node) = []

type Node = {
	next: (Node in nodes)?
}
```

`Bag` is an arena: elements keep stable addresses as it grows, so references into it stay valid. This is the `Vec<T>`-plus-index pattern you already write in Rust for graphs, except the index is typed and checked, and deleting a node sets every `next` that pointed at it to `none`.

## Relations replace `Weak`

A back-reference in Rust means `Weak<RefCell<T>>` and manual upgrade calls. In Fa you declare the relation, and the compiler maintains both sides:

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(Self->bestFriend)
}

ana.bestFriend = bao
-- bao.bestFriendOf now contains ana; nothing was written by hand
```

The related persons live in a stable-address collection (a `Bag`, `GrowingBag` or `Columnar`), named in the type. Deleting a related object updates every field that pointed at it: a relation never dangles. See [relations](/advanced/relations).

## Errors

`Result<T, E>` becomes `throws E` in the signature, and `?` becomes a prefix `try`:

```fa
type ApiError = Errors {
	Timeout = { after: Integer }
	Network = { message: String }
}

function load = (): Integer throws ApiError => ...

function caller = (): Integer throws ApiError => {
	let value = try load()
	return value * 2
}
```

An error is handled with `catch`, which replaces the failed value — matching on the error is exhaustive:

```fa
function printValue = () => {
	let value = load() catch error => when error is {
		Timeout => 0
		Network as { message } => {
			console.log(message)
			return
		}
	}
	console.log(value)
}
```

There is no `panic!` path for errors and no `unwrap`. Indexing out of bounds is not a panic either — it is in the type:

```fa
let array = [1, 2, 3]

let value = array[0]               -- Integer: provably in bounds, no check emitted
let value = try array[index]       -- throws IndexError
let value = array.get(index) ?? 0  -- `get` returns an optional
```

`??` only applies to optionals, never to errors. See [errors](/values/errors).

## Structural, not nominal

Two types with the same fields are the **same type**:

```fa
type Human = { name: String }
type Animal = { name: String }

let human: Human = Animal { name = "Zeus" } -- valid
```

There are no traits and no `impl` blocks. Generic constraints are shapes:

```fa
function readId(Item: { id: Integer }) = (item: Item): Integer => item.id
```

Anything with an `id: Integer` satisfies it, with nothing to implement. When you need two distinct types with the same shape, give them different fields — see [structural typing](/advanced/structural-typing#when-you-want-types-to-be-distinct).

## Generics without turbofish

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)

first([1, 2, 3])
first(Float)([1, 2, 3])
```

Generic parameters are declared right after the name, and inferred from the arguments and the expected type. When they must be written, they are applied before the arguments, so `::<>` is unnecessary.

A capitalized parameter is a type parameter; a lowercase one is a compile-time value, which replaces const generics:

```fa
type Matrix(Element, rows: Integer, columns: Integer) = ...

let transform: Matrix(Float, 4, 4) = ...
```

There is no specialization and no overloading. See [generics](/types/generics).

## Metaprogramming without macros

Fa runs Fa at compile time. There is no `macro_rules!`, no proc-macro crate, and no separate token model:

```fa
static routes = @readDirectory("../routes")
static config = @parseJson(@readFile("config.json"))

@for route in routes {
	server.register(route)
}
```

`@` marks compile-time operations; `static` values are then used like any other value. Derive-style code generation is a type function over `@typeToSchema`. See [metaprogramming](/advanced/metaprogramming).

## Async without colour

An `async` function in Fa can also be called synchronously. There is no `block_on` and no runtime to choose:

```fa
let data = try fetch(url)         -- called synchronously
let data = try await fetch(url)   -- awaited, inside an async function
let task = start fetch(url)       -- spawned
```

Calling synchronously a primitive that is only asynchronous on a target (like network access on `web`) is a compile error.

There are no `Send`/`Sync` bounds to write either: values crossing a thread boundary are copied, which is Fa's ordinary assignment rule, references cannot cross, and the compiler knows which types (mutexes, atomics, channels) can be shared mutably between threads.

Native builds use Tokio underneath, and Rayon for `for parallel`.

## What you gain and what you give up

**Gain:** no lifetimes, no borrow-checker fights, no smart-pointer selection, structural types, typed requirements, built-in reactivity, compile-time execution in the same language, and one source shipped to native, WebAssembly and JavaScript.

**Give up:** traits and their coherence guarantees, free nominal newtypes, `unsafe` (there is none), fine control over allocation, and the crate ecosystem.

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`, floor-based like `//`: `-7 %% 3 == 2`; `%` is the percentage suffix (`50%`) |
| `/` between integers truncates | `/` always returns a `Float`; `//` is floor division |
| `//` comments | `--`; `//` is floor division |
| `as` for numeric casts | conversions are calls (`Float(x)`); `as` only renames |
| `Option<T>` combinators | `?.`, `??`, and `when` |
| `unwrap()` | there is none; handle with `catch`, or propagate with `try` |
| `&T` / `&mut T` | parameters are already references; declare `mutates name` for a mutable one |
| `clone()` | assignment already copies |
| `impl Trait` | a structural constraint: `Item: { field: Type }` |
| `Box<T>` | not needed: recursion through a union or an optional is automatic |
| `Vec::with_capacity(n)` | `Array(Integer).from([], capacity = n)` |
| `'outer: loop` | there are no labeled loops |
| `#[derive(PartialEq)]` | `==` is always a deep structural comparison |
