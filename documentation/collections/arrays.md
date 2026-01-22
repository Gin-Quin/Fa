
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


## Smart bound checks

Fa's compiler keeps track of as much information as possible, which means it can optimize most bounds checking away.

Look at this Rust code:

```rs
let mut array = vec![1, 2, 3];

// This returns an Option<int>
let first = array.get(0);

// This is unsafe and will make the compilation fail
let first = array.get_unchecked(0);

// This will cause bound check at runtime
let first = array[0];
```

As a human, we can easily see that the first element is always valid and does not need bounds checking, but the compiler cannot.

Fa's compiler will understand the size of the vector at this point in code and allow to get the first element without bounds checking:

```fa
mutable array = [1, 2, 3]

-- no bound checking
let first = array[0]
console.log(@type(first)) -- will print "Integer"

-- this will cause the compilation to fail
let fourth = array[3]
```
