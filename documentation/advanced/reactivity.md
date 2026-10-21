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
| `effect` | a block that re-runs when what it reads changes |

`derived` and `effect` do not declare their dependencies: the compiler sees which reactive values the expression reads, and tracks exactly those.

```fa
reactive firstName = "Ada"
reactive lastName = "Lovelace"
reactive showFull = true

derived label = if showFull { "{firstName} {lastName}" } else { firstName }
```

When `showFull` is `false`, `label` does not depend on `lastName`, so changing `lastName` recomputes nothing.

## Reactive collections and objects

Any type can be reactive, including collections:

```fa
reactive array = [1, 2, 3]

effect {
  console.log("Array is {array}")
}

array.push(4) -- triggers the effect
```

Reactivity is fine-grained: an effect reading `array.length` does not re-run when an element's value changes, only when the length does.

## Reactive fields

A field of an object can be reactive on its own:

```fa
mutable object = {
  foo = 1
  reactive bar = 2
}

derived baz = object.bar + 1

object.bar = 5
assert(baz == 6)
```

The object must be `mutable`: a `reactive` field is still a mutable field, and Fa's deep immutability applies to `let` bindings without exception.

## Effects

An effect runs once when it is declared, and again whenever a value it read has changed.

```fa
reactive query = ""

effect {
  console.log("Searching for {query}")
}
```

An effect can return a cleanup function, which runs before the next execution and when the effect is destroyed:

```fa
reactive channelName = "general"

effect {
  use connection = openChannel(channelName)

  return () => connection.close()
}
```

An effect lives as long as the scope that declared it. To stop one early, declare it as a value and destroy it:

```fa
let subscription = effect {
  console.log(count)
}

subscription.stop()
```

## Batching

Several assignments in the same statement block are applied together: dependents recompute once, after the last one.

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

A `derived` expression must be pure: the compiler may recompute it, skip it, or evaluate it in a different order. Put side effects in an `effect`, which is the only place they are guaranteed to run exactly once per change.

```fa
derived total = items.length          -- fine
derived total = do { log("!"); 1 }    -- error: a derived value must be pure
```

## Where to use it

The obvious place is a user interface, where reactivity replaces the virtual DOM, hooks and signals that JavaScript frameworks had to invent because the language did not provide them.

But nothing about it is frontend-specific:

```fa
reactive configuration = loadConfiguration()

derived poolSize = configuration.database.connections

effect {
  connectionPool.resize(poolSize)
}
```

:::info
Because reactivity is part of the language, you can use it on the backend too — to keep derived state, caches or connection pools in sync declaratively rather than imperatively.
:::
