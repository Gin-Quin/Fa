
# Arrays

Arrays are the most common collection for storing a list of values. They can be mutable or immutable.

A list literal `[...]` is an array, unless the context expects another collection (see [creating collections](#creating-collections)).

```fa
let myArray = [1, 2, 3] -- immutable array, you cannot modify, add, or remove elements
let myArray: Array(Integer) = [1, 2, 3]

mutable myMutableArray = [1, 2, 3] -- mutable array, you can modify, add, and remove elements
mutable myMutableArray: Array(Integer) = [1, 2, 3]
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

for item in items {
	console.log(item)
}

for item, index in items {
	console.log("{index}: {item}")
}
```

## Slicing and copying

```fa
let data = [1, 2, 3, 4, 5]
let middle = data[1..4] -- copies the elements at indices 1, 2 and 3: [2, 3, 4]
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

## Creating collections

All collections (`Array`, `Set`, `Map`, `Bag`, `Columnar`, ...) are created from a list literal `[...]`. Braces `{ ... }` are only used for objects.

When the type of the collection is known from the context (a declaration, a parameter, a return type, a field), the literal takes this type:

```fa
let numbers: Set(Integer) = [1, 2, 3]
let scores: Map(String, Integer) = [("ana", 12), ("bao", 17)]
mutable humans: Bag(Human) = []

function total = (values: Set(Integer)): Integer => values.sum()
total([1, 2, 3]) -- the literal is a `Set(Integer)`
```

When there is no context, use the `from` function of the collection type. The type of the elements is inferred from the list, or can be given explicitly:

```fa
let distinctCount = Set.from([1, 2, 2, 3]).size() -- 3
let floats = Set(Float).from([1, 2, 3])
```

`from` also accepts the options of the collection:

```fa
let users = Set.from(userList, compare = (a, b) => a.id == b.id)
```

There is one form per situation: using `from` where the context already gives the type is a warning, and the formatter rewrites it into a plain literal.

