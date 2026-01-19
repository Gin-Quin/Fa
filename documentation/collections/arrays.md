
# Arrays

Arrays are the most common collection for storing a list of values. They can be mutable or immutable.

```fa
let myArray = [1, 2, 3] // immutable array, you cannot modify, add, or remove elements
let myArray: Array(Integer) = [1, 2, 3]
let myArray = Array(Integer) [1, 2, 3]

mutable myMutableArray = [1, 2, 3] // mutable array, you can modify, add, and remove elements
mutable myMutableArray: Array(Integer) = [1, 2, 3]
mutable myMutableArray = Array(Integer) [1, 2, 3]
```

## Adding and removing items

```fa
mutable users = ["Ana", "Bao"]
users.push("Cleo")
users.insert(1, "Dan")
users.delete(0)
```

## Access and update

```fa
let numbers = [10, 20, 30]
let first = numbers[0]

mutable scores = [100, 90, 80]
scores[1] = 95
```

## Iteration

```fa
let items = ["a", "b", "c"]

for items >> item {
	console.log(item)
}

for items >> item, index {
	console.log("{index}: {item}")
}
```

## Slicing and copying

```fa
let data = [1, 2, 3, 4, 5]
let middle = data[1..4] -- copies [2, 3, 4]
```
