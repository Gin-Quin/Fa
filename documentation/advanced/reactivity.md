# Reactivity

Reactivity is part of the Fa language, not a library or a compilation step bolted on top. A reactive value remembers who depends on it, and updates them when it changes.

```fa
let multiplier = 2
reactive number = 3
derived multiplied = number * multiplier

watch {
	console.log("Multiplied is {multiplied}")
}

number = 5 -- `multiplied` is now 10, and the `watch` block runs again after this statement
```

## The three keywords

| Keyword | Declares |
| --- | --- |
| `reactive` | a mutable value that can be observed |
| `derived` | a read-only value computed from reactive values |
| `watch` | a block that runs again when what it reads changes |

`derived` and `watch` do not declare their dependencies: each time they run, Fa records the reactive values they read, and depends on exactly those. The reads made inside the functions they call count too, at any depth.

```fa
reactive firstName = "Ada"
reactive lastName = "Lovelace"
reactive showFull = true

derived label = if showFull { "{firstName} {lastName}" } else { firstName }
```

When `showFull` is `false`, the last computation of `label` did not read `lastName`, so changing `lastName` recomputes nothing.

`reactive` and `derived` values are declared in a function body or at the top of a module. They can't be exported in this version.

:::note
A `watch` block is code that reacts to changes. Not to be confused with the effects of a [function signature](/functions/defining-functions) (`throws`, `requires`, `mutates`, `invalidates`).
:::

## Reactive collections and objects

Any type can be reactive, including objects and collections:

```fa
reactive numbers = [1, 2, 3]

watch {
	console.log("Numbers are {numbers}")
}

numbers.push(4) -- runs the `watch` block again
```

Reactivity is fine-grained: a read depends on the path it reads.

- Reading `numbers.length` depends on the length, `numbers[2]` on that element, `settings.database.connections` on that field.
- Reading a whole value (iterating over it, interpolating it, copying it) depends on all of it.
- A write changes the path it writes and everything below it. Adding or removing elements also changes the length (or the size).

So a `watch` block reading `numbers.length` does not run again when the value of an element changes, only when the length does. Assigning a flat value (a number, a boolean, a union of literals) that is equal to the current one changes nothing.

Being reactive is a property of a place, not of a type: `reactive count = 0` is an `Integer`. What depends on where the value is used:

- A copy is a plain value: `let snapshot = count` reads `count` once, and `snapshot` is not reactive.
- A reference keeps the place reactive: a `use` alias of a reactive value, and a parameter that receives one, are tracked when they are read, and notify when they are written. A function declared with `mutates numbers` that pushes into a reactive array runs the `watch` blocks that read it.

## Reactive fields

A field of a type can be reactive on its own:

```fa
type Counter = {
	label: String
	reactive count: Integer
}

mutable counter = Counter { label = "clicks", count = 1 }

derived next = counter.count + 1

counter.count = 5
assert next == 6
```

The object must be `mutable`: a `reactive` field is still a mutable field, and `let` values are deeply immutable without exception.

`reactive` is part of the field, like `readonly`: `{ reactive count: Integer }` and `{ count: Integer }` are not the same type. A value with reactive fields is still accepted where the same fields without `reactive` are expected, and the reads and writes made through that view are tracked. A copy of an object has the same reactive fields, with no dependents.

## Derived values

A `derived` value is computed when it is read, and is **always up to date**: reading it right after a change gives the new value. It is recomputed only when one of the values it read last time has changed since, and at most once per change, however many times it is read.

A `derived` value refers to the reactive values it reads. The other local values it uses are copied when it is declared, like the values used by an [escaping closure](/functions/defining-functions#closures): a later change to them is never seen.

## Watch blocks

A `watch` block runs once when it is declared, and again after a value it read has changed.

```fa
reactive query = ""

watch {
	console.log("Searching for {query}")
}
```

### When watch blocks run

Changing a reactive value doesn't run the `watch` blocks right away: they become pending, and run:

- when the current statement of the outermost running code ends: a statement of the entry function, of a task, of a test, of a `watch` block, of a callback called by the platform, or a top-level statement of a standalone file,
- and before the current task suspends at an `await`.

Each pending `watch` block runs once, however many of its dependencies changed, in the order the blocks were declared. When it runs, every `derived` value it reads is up to date: a `watch` block never sees a mix of old and new values.

A `watch` block can write reactive values: the blocks that depend on them become pending, and run after it. A block that writes a value it reads itself is a compile error. If the pending blocks keep scheduling each other more than 100 times in a row, the program stops.

A `watch` block is not an async context, and can't throw: handle errors with `catch` inside it. To wait for something, `start` a task inside the block: the task belongs to this run of the block, and is cancelled when the block runs again or is destroyed.

Like a `derived` value, a `watch` block refers to the reactive values it reads, and copies the other local values it uses when it is declared. A function it calls with [requirements](/advanced/requirements) receives the values provided where the block is declared, and the block can't outlive the block of code that provides them.

### Cleanup

A `watch` block can end with a cleanup function: when its last expression is a lambda, that lambda is the cleanup. The cleanup function runs before the next execution of the block, and when the block is destroyed:

```fa
reactive channelName = "general"

watch {
	joinChannel(channelName)

	() => leaveChannel(channelName) -- cleanup: leaves the previous channel
}
```

Like every escaping closure, the cleanup function captures `channelName` by copy: it leaves the channel that was joined, even after `channelName` has changed.

### Lifetime

A `watch` block lives as long as the scope that declared it: like every value, it is destroyed at the end of its scope, and a `watch` block declared at the top of a module lives as long as the program. To stop it earlier, keep its subscription and stop it:

```fa
reactive count = 0

let subscription = watch {
	console.log(count)
}

subscription.stop() -- runs the cleanup; `subscription` can't be used afterwards
```

The subscription is a `Subscription`, from the prelude. It can't be copied, only moved, and `stop()` consumes it.

## Batching

The changes made by one statement are applied together: the `watch` blocks that depend on them run once, after the statement. A `do` block is one statement, so it groups several changes:

```fa
reactive width = 10
reactive height = 20
derived area = width * height

watch {
	console.log("Area: {area}")
}

do {
	width = 30
	height = 40
	assert area == 1200 -- `area` is always up to date
}
-- the `watch` block runs once, here, and logs "Area: 1200"
```

The same holds for a function that changes several values: its call is part of one statement of the caller.

## Cycles

A cycle between derived values is a compile error, not an infinite loop:

```fa
derived a = b + 1
derived b = a + 1 -- error: cyclic dependency between `a` and `b`
```

The compiler checks every branch of a `derived` expression, even a branch that could not create the cycle at runtime.

## Reactivity and purity

A `derived` expression must be [pure](/functions/defining-functions#pure-functions): the compiler may recompute it, skip it, or evaluate it in a different order. So it can't log, throw, use requirements, modify anything but its own local values, or read a module-level `mutable` value that isn't reactive. Put side effects in a `watch` block, which is the only place where they are guaranteed to run exactly once per change.

```fa
derived total = items.length -- fine

derived loggedTotal = do { -- error: a derived value must be pure
	console.log("computing the total")
	items.length
}
```

## Reactivity and concurrency

Reactive values, `derived` values and `watch` blocks belong to the thread that declares them. The [tasks](/advanced/concurrency#tasks) of that thread can read and write them, and the pending `watch` blocks run before a task suspends. A reactive value can't be read or written in a parallel loop, and a worker only receives a plain copy of it.

## Where to use it

The obvious place is a user interface, where reactivity replaces the virtual DOM, hooks and signals that JavaScript frameworks had to invent because the language did not provide them.

But nothing about it is specific to the frontend:

```fa
reactive settings = loadSettings()

derived poolSize = settings.database.connections

watch {
	connectionPool.resize(poolSize)
}
```

:::info
Because reactivity is part of the language, you can use it on the backend too, to keep derived state, caches or connection pools in sync declaratively rather than imperatively.
:::

:::implementation
## Implementation details

Reactivity is push-pull signals, implemented by a small runtime library, the same on every target. Values that are not reactive never carry reactive metadata, and code that never touches a reactive place is compiled as if reactivity did not exist.

### The graph

- **Source**: each reactive place has a version number and a list of observers. A reactive collection or object has one source for the whole value and, created on the first read that needs them, sources for its length or size, for each element or key read individually, and for each field. A write bumps the version of the source it writes, of the sources below it, of the length when it adds or removes elements, and of the whole value.
- **Derived**: a node with a cached value, a `dirty` flag and the list of the sources it read during its last computation, with their versions.
- **Watch**: a node with the same list of dependencies, a cleanup function, and the tasks started by its last run.
- **Tracking**: a thread-local "current observer". The compiler instruments every read of a reactive place with `track(source)`, which records the source in the current observer, and every write with `notify(source)`. Since functions are monomorphized, a function that receives a reactive place gets its own instance, instrumented for it; the instances for plain values are untouched. On JavaScript, where a function is otherwise emitted once, a function that receives a reactive place (or a value with reactive fields) is emitted once per combination of reactive parameters.

### Push, then pull

- **Push**: `notify` walks the observers transitively. A derived node is only marked `dirty` (its value is not computed); a watch node is added to the thread's pending queue, once.
- **Pull**: reading a derived value that is `dirty` checks the versions of its recorded sources (recursively pulling the derived ones); if one changed, it recomputes, recording a fresh dependency list. This is what makes `derived` values always fresh and computed at most once per change.
- **Flush**: the pending queue runs in declaration order (each node has a sequence number). Each run first calls the cleanup and cancels the tasks of the previous run, then runs the block under tracking. The flush repeats while blocks become pending, and stops the program after 100 rounds.

### When the flush happens

The compiler emits a `flush()` after each statement that may write a reactive place (known from the effects of the instances it calls), and before each `await`. A thread-local depth counter, incremented around the calls of instances that may write reactive places, makes `flush()` do nothing unless the current statement belongs to the outermost running code. The entry points of a run (entry function, task resumption, platform callback, test, watch run) start at depth zero.

### Scope cleanup

At the end of the scope of a `watch` block, or on `stop()`, the generated code calls `dispose(node)`: run the cleanup, cancel its tasks, remove the node from the observer lists of its sources. A derived node is disposed the same way. On native, this is the ordinary destruction at scope exit. On JavaScript, which has no destructors, the compiler emits the `dispose` calls explicitly at every exit of the scope (end of block, `return`, `break`, error propagation), and on the cancellation stack of the task that owns the scope.

### Compile-time checks

The cycle check on `derived` values and the check that a `watch` block doesn't write what it reads use the static "may read" and "may write" sets of each expression, over every branch and every function it may call.
:::
