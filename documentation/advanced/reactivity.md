# Reactivity

Reactivity is part of Fa's type system, not a library or a compilation step bolted on top. A reactive value remembers who depends on it, and updates them when it changes.

```fa
let multiplier = 2
reactive number = 3
derived multiplied = number * multiplier

effect {
	console.log("Multiplied is {multiplied}")
}

number = 5 -- recomputes `multiplied`, then runs the effect
```

## The three keywords

| Keyword | Declares |
| --- | --- |
| `reactive` | a mutable value that can be observed |
| `derived` | a read-only value computed from reactive values |
| `effect` | a block that runs again when what it reads changes |

`derived` and `effect` do not declare their dependencies: the compiler sees which reactive values the expression reads, and tracks exactly those.

```fa
reactive firstName = "Ada"
reactive lastName = "Lovelace"
reactive showFull = true

derived label = if showFull { "{firstName} {lastName}" } else { firstName }
```

When `showFull` is `false`, `label` does not depend on `lastName`, so changing `lastName` recomputes nothing.

:::note
An `effect` block is not the same thing as the effects of a [function signature](/functions/defining-functions) (`throws`, `requires`, `mutates`, `invalidates`). An `effect` block is code that reacts to changes.
:::

## Reactive collections and objects

Any type can be reactive, including objects and collections:

```fa
reactive numbers = [1, 2, 3]

effect {
	console.log("Numbers are {numbers}")
}

numbers.push(4) -- runs the effect again
```

Reactivity is fine-grained: an effect reading `numbers.length` does not run again when the value of an element changes, only when the length does.

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

## Effects

An effect runs once when it is declared, and again whenever a value it read has changed.

```fa
reactive query = ""

effect {
	console.log("Searching for {query}")
}
```

### Cleanup

An effect can end with a cleanup function. The cleanup function runs before the next execution of the effect, and when the effect is destroyed:

```fa
reactive channelName = "general"

effect {
	joinChannel(channelName)

	() => leaveChannel(channelName) -- cleanup: leaves the previous channel
}
```

Like every closure, the cleanup function captures `channelName` by copy: it leaves the channel that was joined, even after `channelName` has changed.

### Lifetime

An effect lives as long as the scope that declared it: like every value, it is destroyed at the end of its scope. To stop it earlier, store it in a value and stop it:

```fa
reactive count = 0

mutable subscription = effect {
	console.log(count)
}

subscription.stop()
```

## Batching

Several assignments in the same block are applied together: the dependents are recomputed once, after the last assignment.

```fa
reactive width = 10
reactive height = 20
derived area = width * height

do {
	width = 30
	height = 40
}
-- `area` is recomputed once, to 1200
```

## Cycles

A cycle between derived values is a compile error, not an infinite loop:

```fa
derived a = b + 1
derived b = a + 1 -- error: cyclic dependency between `a` and `b`
```

## Reactivity and purity

A `derived` expression must be pure: the compiler may recompute it, skip it, or evaluate it in a different order. Put side effects in an `effect`, which is the only place where they are guaranteed to run exactly once per change.

```fa
derived total = items.length -- fine

derived loggedTotal = do { -- error: a derived value must be pure
	console.log("computing the total")
	items.length
}
```

## Where to use it

The obvious place is a user interface, where reactivity replaces the virtual DOM, hooks and signals that JavaScript frameworks had to invent because the language did not provide them.

But nothing about it is specific to the frontend:

```fa
reactive settings = loadSettings()

derived poolSize = settings.database.connections

effect {
	connectionPool.resize(poolSize)
}
```

:::info
Because reactivity is part of the language, you can use it on the backend too, to keep derived state, caches or connection pools in sync declaratively rather than imperatively.
:::
