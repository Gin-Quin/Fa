# Reactivity

You can declare reactive variables using the `reactive` and `derived` keywords, and call effects with the  `effect` statement:

```fa
let multiplier = 2
reactive number = 3
derived multiplied = number * multiplier

effect {
	console.log("Multiplied is {multiplied}")
}

number = 5 -- this will update `multiplied` and trigger the effect
```

All types can be reactive (even arrays, objects and other collections):

```fa
reactive array = [1, 2, 3]

effect {
	console.log("Array is {array}")
}

array.push(4) -- this will trigger the effect
```

You can have nested reactive values inside objects:

```fa
let object = {
	foo = 1
	reactive bar = 2
}

derived baz = object.bar + 1
```

:::info
Since reactivity is baked into the language, it also means you can use reactive programming in your backend 😱 For example, to manage the state of your server declaratively rather than imperatively.
:::
