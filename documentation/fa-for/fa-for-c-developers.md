# Fa for C and C++ developers

Fa compiles to a native binary with no runtime and no garbage collector, and gives you the control you expect over where data lives — while making dangling pointers, buffer overruns and use-after-free impossible to write.

Fa is designed for AI agents as much as for humans: everything a function does — the errors it throws, the values it requires, the data it mutates — is written in its signature, and if the code compiles, it works.

## Syntax at a glance

| C / C++ | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const int x = 1;` | `let x = 1` |
| `int x = 1;` | `mutable x = 1` |
| `int add(int a) { return a; }` | `function add = (a: Integer): Integer => a` |
| `[](int x) { return x; }` | `(x) => x` |
| `struct Point { int x, y; };` | `type Point = { x: Integer, y: Integer }` |
| `Point p = { 1, 2 };` | `let p: Point = { x = 1, y = 2 }` |
| `enum Color { RED };` | `type Color = Enum { red }` |
| `union` | `type S = Union { ... }` (tagged) |
| `std::vector<T>` | `Array(T)` |
| `std::unordered_map<K, V>` | `Map(K, V)` |
| `T*` (may be absent) | `T?` |
| `const T&` | a parameter, or `use` |
| `T&` (mutated) | a parameter declared with `mutates` |
| `NULL` / `nullptr` | `none` |
| `int64_t`, `double` | `Integer`, `Float` |
| `int32_t`, `uint8_t` | `Int32`, `UInt8` (or `Byte`) |
| `(double)count` | `Float(count)` |
| `a / b` (integers) | `a // b` (`/` always returns a `Float`) |
| `x % y` | `x %% y` |
| `x >> y` (bit shift) | `x.shiftRight(y)` |
| `x & y`, `x \| y`, `x ^ y`, `~x` | `x.bitAnd(y)`, `x.bitOr(y)`, `x.bitXor(y)`, `x.bitNot()` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `switch` | `when` |
| `for (int i = 0; i < n; i++)` | `for i in 0..n` |
| `assert(x == y)` | `assert x == y` |
| `#include` | nothing — files see their siblings |
| `#ifdef` | `@if` |
| `#define` | `static` and `@` |

## Where memory lives

The model is close to how you already think, with the rules enforced:

1. **Objects are stack values.** Always. `type Point = { x: Integer, y: Integer }` is a `struct` with no hidden header, no vtable and no allocation.
2. **Collections own the heap.** `Array`, `Set`, `Map`, `Bag`, `Columnar` — each with the allocation strategy that suits it. There is no `malloc` and no `free`.
3. **Assignment copies.** Like a C struct assignment. The compiler turns a copy into a move when the source is dead.
4. **`use` creates an alias.** Like `T&`, but the compiler knows what it points into.
5. **Everything is released at the end of its scope**, in reverse order of declaration.

```fa
function process = () => {
	mutable buffer = Array(Byte).from([], capacity = 4096)
	-- fill it, pass references around, take slices
} -- one deallocation, here
```

This is arena allocation as the default, which is what most careful C code ends up doing anyway.

## Mutation is declared

A parameter is an immutable reference by default. A function that modifies one of its parameters declares it in its signature, as an effect:

```c
// C: nothing tells the caller that `scores` is modified
void add_bonus(int *scores, size_t length, int bonus);
```

```fa
function addBonus = (scores: Array(Integer), bonus: Integer) mutates scores => {
	for score in scores {
		score += bonus -- writes into the array
	}
}

mutable scores = [10, 20, 30]
addBonus(scores, 5)
```

Calling a mutating function requires a `mutable` value. An operation that can move or remove the elements of a collection, like `push`, declares `invalidates` instead — which is how the compiler tracks references.

## No pointers, no dangling references

A reference into a collection carries the collection in its type, so the compiler knows when it dies:

```c
// C: compiles, and is undefined behaviour
int *first = &vec.data[0];
vector_push(&vec, 4);  // may realloc
printf("%d", *first);  // use-after-free
```

```fa
mutable numbers = [1, 2, 3]
use first = numbers[0]   -- type: `Integer in numbers`
numbers.push(4)          -- may reallocate: `push` invalidates `numbers`
console.log(first)       -- error: `first` was invalidated by `numbers.push`: declare it again with `use first = numbers[...]`
```

For references that must survive growth, use a `Bag` — an arena whose elements keep stable addresses — or a `GrowingArray`, whose references are valid for the life of the collection. See [owned references](/advanced/owned-references). A pointer stored in a struct field becomes a [relation](/advanced/relations): `next: (Node in nodes)?`, set to `none` when its target is deleted.

## No buffer overruns

Indexing is checked by the type system, not by a runtime guard you pay for everywhere:

```fa
let array = [1, 2, 3]

let a = array[0]          -- Integer: provably in bounds, no check emitted
let b = array[3]          -- compile error
let c = try array[i]      -- throws IndexError: propagate it, or `catch` it
let d = array.get(i)      -- Integer?: `none` when out of bounds

let index: Integer(min = 0, max = 2) = readIndex()
let e = array[index]      -- Integer: the range proves it, no check emitted
```

Ranged integers are the mechanism. They also cover division:

```fa
function averageOf = (total: Integer, count: Integer(min = 1)): Integer =>
	total // count -- no zero check: the type rules it out
```

When the divisor is not provably non-zero, the division `throws DivisionByZero`, and the compiler makes you handle it.

## Data-oriented design, built in

`Columnar` is a struct-of-arrays with the interface of an array-of-structs:

```fa
mutable people: Columnar(Person) = []
mutable total = 0

for { age } in people {
	total += age -- reads one contiguous run of memory
}
```

See [columnars](/collections/columnars).

## Errors instead of return codes

```c
int load_score(int *out);  // 0 on success, error code otherwise
```

```fa
type ScoreError = Errors {
	NotFound
	Malformed = { line: Integer }
}

function loadScore = (): Integer throws ScoreError => ...

let score = try loadScore()                -- propagate to the caller
let score = loadScore() catch error => 0   -- handle it here
```

The error is a typed value declared in the signature with `throws`. It carries data, cannot be ignored — every call that can fail is marked with `try` or `catch` — and there is no `errno`. `throw` exits the function like `return` does: an error is not an exception that silently crosses code that did not declare it, so there is no `-fno-exceptions` decision and no hidden cost on a code path. See [errors](/values/errors).

## Tagged unions

C unions are untagged and unsafe. Fa's are tagged and exhaustively matched:

```fa
type Value = Union {
	Count = Integer
	Text = String
	Pair = { left: Integer, right: Integer }
}

let size = when value is {
	Count as count => count
	Text as text => text.length
	Pair as { left, right } => left + right
}
```

A recursive type is built through a union or an optional (`next: Node?`), and stored on the heap automatically. The child is still a copy, owned by its parent: to share nodes, use a `Bag` and [relations](/advanced/relations). See [unions](/types/unions).

## The preprocessor, replaced

Fa executes Fa at compile time. There are no macros, no textual substitution and no separate include model. `@` marks the compile-time operations, and `static` declares a compile-time value, used like any other value:

```fa
static version = @readFile("../version.txt")
static config = @parseJson(@readFile("config.json"))
static tables = @buildLookupTables()

@if target is "native" {
	useSimdPath()
} @else {
	usePortablePath()
}

@for index in 0..4 {
	accumulate(index) -- unrolled at compile time
}
```

Everything is type-checked, and it debugs like ordinary code. Compile-time evaluation is deterministic: no clock, no network, no randomness. See [metaprogramming](/advanced/metaprogramming).

## No headers, no build system

There are no declarations to duplicate, no include guards, no link order, and no `Makefile` or `CMakeLists.txt`. Files see their siblings and the package root, folders are namespaces, and each file exports one thing.

```bash
fa build --target=native
```

See [modules](/architecture/modules).

## Concurrency without data races

Values crossing a thread boundary are copied, references cannot cross at all, and only thread-safe types (mutexes, atomics, channels) can be shared mutably — so a data race does not typecheck:

```fa
for parallel item, index in items {
	results[index] = expensiveComputation(item) -- each iteration writes to its own index
}
```

Native builds use Rayon for parallel iteration and Tokio for async. Mutexes and atomics are available when you genuinely need shared state, with the guard released by scope rather than by an explicit unlock. See [concurrency](/advanced/concurrency).

## What you gain and what you give up

**Gain:** memory safety with no garbage collector and no borrow checker, no undefined behaviour, no headers or build system, a real type system with typed errors and declared mutations, compile-time execution, and the same source compiled to native, WebAssembly and JavaScript.

**Give up:** raw pointers and pointer arithmetic, manual allocator control, `unsafe` escape hatches (there are none), inline assembly, and direct access to the C and C++ ecosystems beyond the FFI.

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`, floor-based like `//`: `-7 %% 3 == 2`; `%` is the percentage suffix (`50%`) |
| `/` between integers truncates | `/` always returns a `Float`; `//` is floor division: `-7 // 2 == -4` |
| `//` comments | `--`; `//` is floor division |
| `>>` / `<<` for bit shifts | `.shiftRight()` / `.shiftLeft()`; there are no bitwise operators |
| Implicit numeric conversions | none: `Float(count) * ratio` |
| `int` overflow wrapping | a compile error when certain, a runtime stop when possible, or explicit with `wrappingAdd`, `saturatingAdd` or `checkedAdd` |
| `if (count)`, `if (ptr)` | no truthiness: `if count != 0`, `if value is not none` |
| Passing by pointer to avoid a copy | parameters are already references |
| Passing by pointer to modify | declare `mutates name` in the signature |
| `memcpy` of a struct | assignment already copies |
| Out parameters | return a tuple, or declare `throws` |
| `goto cleanup` | scope-based release |
| Labeled `break` out of nested loops | there are no labeled loops |
