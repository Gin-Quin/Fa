# Fa for Rust developers

Fa shares Rust's goals — memory safety without a garbage collector, errors as values, zero-cost abstractions — and reaches them with a much smaller model. There is no borrow checker, no lifetime syntax, and no smart pointer zoo.

## Syntax at a glance

| Rust | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `let x = 1;` | `let x = 1` |
| `let mut x = 1;` | `mutable x = 1` |
| `fn add(a: i64) -> i64 { a }` | `function add = (a: Int): Int => a` |
| `\|x\| x + 1` | `(x: Int) => x + 1` |
| `Vec<T>` | `Array(T)` |
| `HashMap<K, V>` | `Map(K, V)` |
| `Option<T>` / `None` | `T?` / `none` |
| `Result<T, E>` | `T \| E` |
| `?` | `!` |
| `match x { }` | `when x is { }` |
| `for x in xs` | `for xs >> x` |
| `while let Some(x) = it.next()` | `while it.next() >> x` |
| `x % y` | `x %% y` |
| `!x` | `not x` |
| `&&`, `\|\|` | `and`, `or` |
| `impl Trait for T` | structural types; no traits |
| `use std::collections::HashMap;` | nothing — files see their siblings |

## No borrow checker

Fa's memory model has five rules instead of ownership and borrowing:

1. Objects live on the **stack**; collections own the **heap**.
2. Assignment **copies** (the compiler turns it into a move when the source is dead).
3. `use` makes a **reference**, which cannot outlive what it points into.
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
console.log(first) -- error: `first` was invalidated by the growth
```

The same error, for the same reason — but the type is `Int in v`, not `&'a i64`, and you never write it. There is one rule (a reference dies when its container changes shape) rather than a system of aliasing rules.

## No smart pointers

Fa has no `Box`, `Rc`, `Arc`, `RefCell`, `Cell`, `Cow` or `Weak`. Recursive and shared structures are built two ways.

**A union**, when only one variant exists at a time:

```fa
type Node = Union {
  Text = { value: String }
  Parent = { child: Node }
}
```

**A collection plus references**, when nodes are shared:

```fa
mutable nodes = Bag(Node) {}

type Node = {
  next: (Node in nodes)?
}
```

`Bag` is an arena: elements keep stable addresses as it grows, so references into it stay valid. This is the `Vec<T>`-plus-index pattern you already write in Rust for graphs, except the index is typed and checked.

## Relations replace `Weak`

A back-reference in Rust means `Weak<RefCell<T>>` and manual upgrade calls. In Fa you declare the relation, and the compiler maintains both sides:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Bag(Self->bestFriend)
}

ana.bestFriend = bao
-- bao.bestFriendOf now contains ana; nothing was written by hand
```

Deleting a related object clears the references to it. See [relations](../advanced/relations).

## Errors

`Result<T, E>` is spelled `T | E`, and `?` is spelled `!`:

```fa
type ApiError = Errors {
  Timeout = { after: Int }
  Network = { message: String }
}

function load = (): Int | ApiError => ...

function caller = (): Int | ApiError => {
  let value = load()!
  return value * 2
}
```

There is no `panic!` path for errors and no `unwrap`. Indexing out of bounds is not a panic either — it is in the type:

```fa
let value = array[index]      -- Int | IndexError
let value = array[index] ?? 0
let value = array[0]          -- Int: provably in bounds, no check emitted
```

## Structural, not nominal

Two types with the same fields are the **same type**:

```fa
type Human = { name: String }
type Animal = { name: String }

let human: Human = Animal { name = "Zeus" } -- valid
```

There are no traits and no `impl` blocks. Generic constraints are shapes:

```fa
function readId = (Item: Type({ id: Int }), item: Item): Int => item.id
```

Anything with an `id: Int` satisfies it, with nothing to implement. When you need two distinct types with the same shape, give them different fields — see [structural typing](../advanced/structural-typing#when-you-want-types-to-be-distinct).

## Generics without turbofish

```fa
function first = (Item: Type, items: Array(Item)): Item? => ...

first([1, 2, 3])
first(Item = Decimal, [1, 2, 3])
```

Type parameters are ordinary parameters annotated `Type`. Because they are named rather than positional, `::<>` is unnecessary.

Compile-time value parameters replace const generics:

```fa
type Matrix = (Element: Type, @rows: UInt, @columns: UInt) => { ... }
```

## Metaprogramming without macros

Fa runs Fa at compile time. There is no `macro_rules!`, no proc-macro crate, and no separate token model:

```fa
static routes = @readDirectory("../routes")
static config = @parseJson(@readFile("config.json"))

@for routes >> route {
  server.register(route)
}
```

Derive-style code generation is a type function over `@typeToSchema`. See [metaprogramming](../advanced/metaprogramming).

## Async without colour

An `async` function in Fa is callable from synchronous code. There is no `block_on`, no runtime to choose, and no `Send`/`Sync` bounds to satisfy — values crossing a thread boundary are copied, which is Fa's ordinary assignment rule.

```fa
let data = fetch(url)!         -- blocking
let data = await fetch(url)!   -- awaited
let task = start fetch(url)    -- spawned
```

Native builds use Tokio underneath, and Rayon for `for parallel`.

## What you gain and what you give up

**Gain:** no lifetimes, no borrow-checker fights, no smart-pointer selection, structural types, built-in reactivity, compile-time execution in the same language, and one source shipped to native, WebAssembly and JavaScript.

**Give up:** traits and their coherence guarantees, free nominal newtypes, `unsafe` (there is none), fine control over allocation, and the crate ecosystem.

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`; `%` builds a percentage |
| `//` comments | `--`; `//` is integer division |
| `Option<T>` combinators | `?.`, `??`, and `when` |
| `unwrap()` | there is none; handle it, or propagate with `!` |
| `&T` / `&mut T` | parameters are already references; `mutable` for mutable ones |
| `clone()` | assignment already copies |
| `impl Trait` | a structural constraint: `Type({ field: T })` |
| `Vec::with_capacity(n)` | `Array(Int, capacity = n) {}` |
