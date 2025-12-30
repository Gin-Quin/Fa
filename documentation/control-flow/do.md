
# Do
The `do` keyword is used to create a block of code and, if used as an expression, return a value.

```fa
let helloWorld = do {
	mutable hello = "Hello" -- create a scoped variable
	hello += " World"
	hello += "!"
	hello
}
```


```fa
let array = [1, 2, 3, 4, 5]

let sum = do {
	mutable total = 0
	for array >> value {
		total += value
	}
	total
}
```

## Use cases

1. Keep temporary variables out of the current scope
2. Replace **IIFE** (*immediately invoked function expression*)
3. Along with a `for` loop, serve as an alternative to the **reduce** method
