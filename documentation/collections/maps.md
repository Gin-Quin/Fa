# Maps

Maps are collections that associate keys with values. They can be mutable or immutable.

A map is created from a list of `(key, value)` tuples, typed by the context, or with `Map.from` when there is no context (see [creating collections](/collections/arrays#creating-collections)).

```fa
let myMap: Map(String, Integer) = [
  ("foo", 12)
  ("bar", 13)
]
```

## Access and update

```fa
let scores: Map(String, Integer) = [
	("ana", 12)
	("bao", 17)
]

let anaScore = scores["ana"]

mutable counters: Map(String, Integer) = [
	("visits", 1)
]
counters["visits"] = 2
```

## Iteration

```fa
let usersById: Map(Integer, String) = [
	(1, "Ana")
	(2, "Bao")
]

for (id, name) in usersById {
	console.log("{id} -> {name}")
}
```

## Adding and removing

```fa
mutable inventory: Map(String, Integer) = []
inventory["apples"] = 3
inventory["oranges"] = 2
inventory.delete("apples")
```
