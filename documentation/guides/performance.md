# Performance

Fa is designed so that the straightforward version of your code is already fast. This guide covers the cases where it is not, and what to do about them.

## Measure first

Development builds are unoptimised and keep debug information. Never draw conclusions from `fa develop`.

```bash
fa build --target=native
fa preview --profile
```

`--profile` reports where time and allocations go, per function.

## Where the costs are

Fa's memory model means performance work is mostly about **where your data lives**, not about micro-optimising expressions.

| Cost | Cause | Fix |
| --- | --- | --- |
| Copying | assigning a large object | create an alias with `use` |
| Reallocation | growing a collection past its capacity | reserve capacity up front |
| Cache misses | reading one field across many objects | use a [`Columnar`](/collections/columnars) |
| Rehashing | inserting into a `Set` or `Map` in a loop | reserve, or use a [`Bag`](/collections/bags) |
| Gather | reading whole objects out of a columnar | use an `Array` if you read whole objects |

## Avoid copies you did not mean

Assignment copies. That is cheap for a small object and not for a large one.

```fa
-- copies the whole report
let report = company.annualReport

-- no copy: `report` is an alias of `company.annualReport`
use report = company.annualReport
```

Function parameters and iteration already give you references, so this only matters for explicit assignments. See [memory management](/advanced/memory-management).

## Reserve capacity

A collection that grows past its capacity allocates a larger block and moves its contents. When you know the size, say so with the `capacity` option of `from`:

```fa
mutable results = Array(Integer).from([], capacity = items.length)

for item in items {
	results.push(transform(item))
}
```

## Pick the right collection

| Access pattern | Collection |
| --- | --- |
| Ordered, indexed | `Array` |
| Membership tests | `Set` |
| Key lookup | `Map` |
| Many objects, stable references | `Bag` |
| One field across many objects | `Columnar` |
| Only ever added to | `Growing*` |

The columnar case is the one with the largest effect. A loop reading a single field across a million objects reads one contiguous run of memory instead of striding through whole objects:

```fa
mutable total = 0

for { age } in people {
	total += age
}
```

See [columnars](/collections/columnars).

## Let chained methods fuse

Chained collection methods are compiled into a single pass, with one allocation for the result:

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

Writing this as an explicit loop buys nothing. Breaking the chain across statements does prevent fusion, because each intermediate result becomes a real collection:

```fa
let evens = numbers.filter((n) => n %% 2 == 0) -- allocates
let doubled = evens.map((n) => n * 2)          -- allocates again
```

## Move work to compile time

Anything the compiler can compute costs nothing at runtime. Declare compile-time values with `static`, and use them plainly:

```fa
static lookupTable = @buildLookupTable()
static configuration = @parseJson(@readFile("config.json"))

@if configuration.debug {
	verboseLog() -- removed entirely when `configuration.debug` is false
}
```

`@if` is evaluated by the compiler: the branch that is not taken is not compiled at all. See [metaprogramming](/advanced/metaprogramming).

## Help the compiler remove checks

Ranged integers let the compiler prove an index is valid or a divisor is non-zero, removing bounds checks and division-by-zero handling entirely:

```fa
function averageOf = (values: Array(Integer), count: Integer(min = 1)): Integer =>
	sum(values) // count -- cannot throw `DivisionByZero`: `count` is at least 1
```

Without the range, `//` would `throws DivisionByZero`, and every caller would have to handle it. See [smart bound checks](/collections/arrays#smart-bound-checks).

## Parallelise computation

On the native target, `for parallel` spreads a loop across cores:

```fa
for parallel item, index in items {
	results[index] = expensiveComputation(item)
}
```

Each iteration writes only to its own index, so the compiler can prove that the writes never overlap, and no synchronisation is needed.

This pays off when each iteration does real work. For short bodies, the coordination costs more than it saves. See [concurrency](/advanced/concurrency).

## Per-target notes

**Native**: the fastest target. Use `Columnar`, `for parallel` and reserved capacity freely.

**WebAssembly**: no parallel iteration unless shared memory is available. Keep the module small: avoid pulling in large parts of the standard library you do not need.

**JavaScript**: collections compile to native JavaScript structures. Integers above `Integer.JS_MAX_SAFE_INTEGER` lose precision unless you use `BigInt`; see [numbers](/values/numbers).

## A checklist

1. Build with `fa build`, and profile the real thing.
2. Find the hot loop; ignore everything else.
3. Check what it copies, and whether `use` removes the copy.
4. Check whether the collection matches the access pattern.
5. Reserve capacity where the size is known.
6. Move constants to compile time.
7. Parallelise last, and only if the body is expensive.
