# Maps

A map associates keys with values. Maps can be mutable or immutable.

```fa
let myMap = Map(String, Int) {
  ("foo", 12)
  ("bar", 13)
}
```

Entries are written as tuples. An empty map needs its type:

```fa
mutable inventory = Map(String, Int) {}
```

## Access and update

```fa
let scores = Map(String, Int) {
  ("ana", 12)
  ("bao", 17)
}

let anaScore = scores["ana"] -- Int | KeyNotFound
```

Reading a key the compiler cannot prove is present returns an error union, so a missing key can never be mistaken for a value. Handle it, or supply a fallback:

```fa
let anaScore = scores["ana"] ?? 0

let score = when scores["cleo"] is {
  Int >> value => value
  Error => 0
}
```

Use `get` when absence is expected rather than exceptional:

```fa
let maybeScore = scores.get("cleo") -- Int?
```

Writing is straightforward:

```fa
mutable counters = Map(String, Int) {
  ("visits", 1)
}

counters["visits"] = 2
```

## Adding and removing

```fa
mutable inventory = Map(String, Int) {}

inventory["apples"] = 3
inventory["oranges"] = 2
inventory.delete("apples")
inventory.clear()

inventory.has("oranges") -- true
inventory.length
inventory.isEmpty()
```

## Iteration

Iterating a map yields `(key, value)` tuples.

```fa
let usersById = Map(Int, String) {
  (1, "Ana")
  (2, "Bao")
}

for usersById >> (id, name) {
  console.log("{id} -> {name}")
}
```

Iteration order is not guaranteed. Keys and values can be iterated separately:

```fa
for usersById.keys() >> id { }
for usersById.values() >> name { }
```

## Default values

`getOrInsert` reads a key, inserting a default when it is absent:

```fa
mutable counts = Map(String, Int) {}

for words >> word {
  use count = counts.getOrInsert(word, 0)
  count += 1
}
```

## Keys

Any type with a well-defined equality can be a key: numbers, strings, booleans, enumerations, tuples and objects.

```fa
let byPoint = Map(Tuple(Int, Int), String) {
  ((0, 0), "origin")
}
```

## Stable references

Like a set, a map may rehash when modified, invalidating references into it. Use `getOrInsert` and short-lived references, or store the values in a [`Bag`](./bags) and map keys to references into that bag.

## Maps versus objects

An object has a fixed set of fields known at compile time; a map has keys discovered at runtime.

```fa
type Settings = { host: String, port: Int } -- known fields
let headers = Map(String, String) {}        -- arbitrary keys
```

If you find yourself writing `Map(String, T)` with a fixed set of keys, use an object or a [`Record`](../types/type-functions#recordkeys-item) instead — it is faster and type-checked.
