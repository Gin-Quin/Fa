# Fa for C and C++ developers

Fa compiles to a native binary with no runtime and no garbage collector, and gives you the control you expect over where data lives — while making dangling pointers, buffer overruns and use-after-free impossible to write.

## Syntax at a glance

| C / C++ | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const int x = 1;` | `let x = 1` |
| `int x = 1;` | `mutable x = 1` |
| `int add(int a) { return a; }` | `function add = (a: Int): Int => a` |
| `[](int x) { return x; }` | `(x: Int) => x` |
| `struct Point { int x, y; };` | `type Point = { x: Int, y: Int }` |
| `enum Color { RED };` | `type Color = Enum { red }` |
| `union` | `type S = Union { ... }` (tagged) |
| `std::vector<T>` | `Array(T)` |
| `std::unordered_map<K, V>` | `Map(K, V)` |
| `T*` (may be absent) | `T?` |
| `T&` | a parameter, or `use` |
| `NULL` / `nullptr` | `none` |
| `int64_t`, `uint8_t` | `Int64`, `UInt8` |
| `x % y` | `x %% y` |
| `x >> y` (bit shift) | `x.shiftRight(y)` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `switch` | `when` |
| `#include` | nothing — files see their siblings |
| `#ifdef` | `@if` |
| `#define` | `static` and `@` |

## Where memory lives

The model is close to how you already think, with the rules enforced:

1. **Objects are stack values.** Always. `type Point = { x: Int, y: Int }` is a `struct` with no hidden header, no vtable and no allocation.
2. **Collections own the heap.** `Array`, `Set`, `Map`, `Bag`, `Columnar` — each with the allocation strategy that suits it. There is no `malloc` and no `free`.
3. **Assignment copies.** Like a C struct assignment. The compiler turns a copy into a move when the source is dead.
4. **`use` takes a reference.** Like `T&`, but the compiler knows what it points into.
5. **Everything is released at the end of its scope**, in reverse order of declaration.

```fa
function process = () => {
  mutable buffer = Array(Byte, capacity = 4096) {}
  -- fill it, pass references around, take slices
} -- one deallocation, here
```

This is arena allocation as the default, which is what most careful C code ends up doing anyway.

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
use first = numbers[0]   -- type: `Int in numbers`
numbers.push(4)          -- may reallocate
console.log(first)       -- error: `first` was invalidated
```

For references that must survive growth, use a `Bag` — an arena whose elements keep stable addresses — or a `GrowingArray`, whose references are valid for the life of the collection. See [owned references](../advanced/owned-references).

## No buffer overruns

Indexing is checked by the type system, not by a runtime guard you pay for everywhere:

```fa
let array = [1, 2, 3]

let a = array[0]      -- Int: provably in bounds, no check emitted
let b = array[3]      -- compile error
let c = array[i]      -- Int | IndexError: handle it

let index: Int(min = 0, max = 2) = readIndex()
let d = array[index]  -- Int: the range proves it, no check emitted
```

Ranged integers are the mechanism. They also cover division:

```fa
function averageOf = (values: Array(Int), count: Int(min = 1)): Int =>
  sum(values) // count -- no zero check: the type rules it out
```

## Data-oriented design, built in

`Columnar` is a struct-of-arrays with the interface of an array-of-structs:

```fa
mutable people = Columnar(Person) {}

for people >> { age } {
  total += age -- reads one contiguous run of memory
}
```

See [columnars](../collections/columnars).

## Errors instead of return codes

```c
int load_score(int *out);  // 0 on success, error code otherwise
```

```fa
type ScoreError = Errors {
  NotFound
  Malformed = { line: Int }
}

function loadScore = (): Int | ScoreError => ...

let score = loadScore()!      -- propagate
let score = loadScore() ?? 0  -- default
```

The error carries data, cannot be ignored, and there is no `errno`. There are no exceptions either — nothing unwinds, so there is no `-fno-exceptions` decision and no hidden cost on a code path.

## Tagged unions

C unions are untagged and unsafe. Fa's are tagged and exhaustively matched:

```fa
type Value = Union {
  Integer = Int
  Text = String
  Pair = { left: Int, right: Int }
}

when value is {
  Integer >> n => n
  Text >> s => s.length
  Pair >> { left, right } => left + right
}
```

## The preprocessor, replaced

Fa executes Fa at compile time. There are no macros, no textual substitution and no separate include model:

```fa
static version = @readFile("../version.txt")
static config = @parseJson(@readFile("config.json"))
static tables = @buildLookupTables()

@if @target is "native" {
  useSimdPath()
} @else {
  usePortablePath()
}

@for 0..4 >> i {
  accumulate(i) -- unrolled at compile time
}
```

Everything is type-checked, and it debugs like ordinary code. See [metaprogramming](../advanced/metaprogramming).

## No headers, no build system

There are no declarations to duplicate, no include guards, no link order, and no `Makefile` or `CMakeLists.txt`. Files see their siblings and the package root, folders are namespaces, and each file exports one thing.

```bash
fa build --platform native
```

See [modules](../architecture/modules).

## Concurrency without data races

Values crossing a thread boundary are copied, and references cannot cross at all, so a data race does not typecheck:

```fa
for parallel items >> item, index {
  results[index] = expensiveComputation(item)
}
```

Native builds use Rayon for parallel iteration and Tokio for async. Mutexes and atomics are available when you genuinely need shared state, with the guard released by scope rather than by an explicit unlock.

## What you gain and what you give up

**Gain:** memory safety with no garbage collector, no undefined behaviour, no headers or build system, a real type system, compile-time execution, and the same source compiled to native, WebAssembly and JavaScript.

**Give up:** raw pointers and pointer arithmetic, manual allocator control, `unsafe` escape hatches (there are none), inline assembly, and direct access to the C and C++ ecosystems beyond the FFI.

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`; `%` builds a percentage |
| `//` comments | `--`; `//` is integer division |
| `>>` / `<<` for bit shifts | `.shiftRight()` / `.shiftLeft()`; `>>` and `<<` are extract and insert |
| `int` overflow wrapping | rejected at compile time, or explicit with `wrappingAdd` |
| Passing by pointer to avoid a copy | parameters are already references |
| `memcpy` of a struct | assignment already copies |
| Out parameters | return a tuple, or an error union |
| `goto cleanup` | scope-based release |
