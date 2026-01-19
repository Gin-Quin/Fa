# String operations

Fa strings use the same API as JavaScript strings.

## Concatenation

Use `+` to add values to a string.

```fa
let greeting = "Hello" + " " + "world"
let label = "Score: " + 42
```

## Common operations

```fa
let name = "Fa"
let upper = name.toUpperCase()
let lower = name.toLowerCase()
let length = name.length
```

## Slicing and searching

```fa
let text = "hello world"
let part = text.slice(0, 5)
let hasWorld = text.includes("world")
let index = text.indexOf("world")
```

## Replace and split

```fa
let clean = "hello-world".replace("-", " ")
let parts = "a,b,c".split(",")
```
