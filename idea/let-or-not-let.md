Should I use let or not let before declaring a variable?


## Let

```coffee
let myVariable = 123

let method(x = 12) = x * 2

let method(x, y): number => {
  ...
}

set myVariable = 456

export = 456
```

### Advantages

- People from Typescript and Rust are used to it
- Shorter syntax for functions that is ISO with methods (can be used to define pure functions as well)
- Clearer intent
- Easier destructuring

## Not let

```coffee
myVariable = constant(123)

method = (x = 12) => x * 2

method = (x, y): number => {
  ...
}

set myVariable = 456

export = 456
```

### Advantages

- Don't have to write all these "let" (declaring values is one of the most common things to do in a program)
- Explicit shadowing with "set"
- Same syntax as objects and modules

## Conclusion

I should stick to `let`. Innovations should be preferred only when the added value is significant. In this case, the added value is not significant enough to justify changing people's usual patterns.
