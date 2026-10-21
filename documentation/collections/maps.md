# Maps

A map associates keys with values. Maps can be mutable or immutable.

A map is written as a list of `(key, value)` [tuples](/values/tuples). Like every collection, it is typed by the context, or created with `Map.from` when there is no context (see [creating collections](/collections/arrays#creating-collections)).

```fa
let myMap: Map(String, Integer) = [
	("foo", 12)
	("bar", 13)
]

let sameMap = Map.from([("foo", 12), ("bar", 13)]) -- no context: a `Map(String, Integer)`
```

An empty map needs its type:

```fa
mutable inventory: Map(String, Integer) = []
```

## Access and update

Reading a key with `map[key]` is checked at compile time when the compiler can prove the key is present. Otherwise, it `throws KeyNotFound`, so a missing key can never be mistaken for a value. Propagate the error with `try`, or handle it with `catch` (see [errors](/values/errors)):

```fa
let scores: Map(String, Integer) = [
	("ana", 12)
	("bao", 17)
]

function scoreOf = (scores: Map(String, Integer), name: String): Integer throws KeyNotFound => {
	let score = try scores[name] -- propagates `KeyNotFound` to the caller
	score
}

let cleoScore = scores["cleo"] catch error => 0
```

Use `get` when absence is expected rather than exceptional. It returns an [optional](/values/optionals):

```fa
let maybeScore = scores.get("cleo") -- Integer?
let score = scores.get("cleo") ?? 0

if maybeScore is not none {
	console.log("Cleo scored {maybeScore}") -- `maybeScore` is narrowed to `Integer`
}
```

Writing a key replaces its value, or adds the key when it is absent:

```fa
mutable counters: Map(String, Integer) = [
	("visits", 1)
]

counters["visits"] = 2
```

## Adding and removing

```fa
mutable inventory: Map(String, Integer) = []

inventory["apples"] = 3
inventory["oranges"] = 2
inventory.delete("apples")

inventory.includes("oranges") -- true: `includes` checks the keys
inventory.length              -- 1
inventory.isEmpty()           -- false

inventory.clear()
```

## Iteration

Iterating a map yields `(key, value)` tuples, which the loop destructures:

```fa
let usersById: Map(Integer, String) = [
	(1, "Ana")
	(2, "Bao")
]

for (id, name) in usersById {
	console.log("{id} -> {name}")
}
```

Iteration order is not guaranteed. Keys and values can be iterated separately:

```fa
for id in usersById.keys() {
	console.log(id)
}

for name in usersById.values() {
	console.log(name)
}
```

Adding or removing keys of a map while iterating over it is a compile error.

## Default values

`getOrInsert` returns a reference to the value of a key, inserting a default value first when the key is absent:

```fa
mutable counts: Map(String, Integer) = []

for word in words {
	use count = counts.getOrInsert(word, 0)
	count += 1
}
```

Since `getOrInsert` returns a reference, it is bound with `use`: `let` would copy the value, and the map would not be updated.

## Keys

Any type with a well-defined equality can be a key: numbers, strings, booleans, enumerations, tuples and objects. Keys are compared with `==`, which is a deep structural equality.

```fa
let byPoint: Map(Tuple(Integer, Integer), String) = [
	((0, 0), "origin")
]
```

## Stable references

Like a set, a map may rehash when a key is added or removed, which moves its entries. Writing a key (`map[key] = value`), `getOrInsert`, `delete` and `clear` declare the `invalidates self` effect, and the compiler rejects any use of a reference into the map obtained before one of these calls.

Keep references into a map short-lived, as in the `getOrInsert` example above. When you need long-lived references, store the values in a [`Bag`](/collections/bags) instead.

## Maps versus objects

An object has a fixed set of fields known at compile time; a map has keys discovered at runtime.

```fa
type Settings = { host: String, port: Integer } -- known fields
let headers: Map(String, String) = []           -- arbitrary keys
```

If you find yourself writing `Map(String, Item)` with a fixed set of keys, use an object or a [`Record`](/types/type-functions#recordkeys-item) instead: it is faster and type-checked.
