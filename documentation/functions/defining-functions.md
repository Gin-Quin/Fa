# Functions

Functions are values declared with the `=>` syntax.

```ts
add = (a: Number, b: Number) => a + b

// or

add = (a: Number, b: Number): Number => {
  return a + b
}
```

If it returns a value, it must be specified in the type declaration, unless the function directly returns a value (i.e. no brackets after the `=>`).

```ts
add = (a: Number, b: Number) => a + b // OK, the return type is inferred

add = (a: Number, b: Number) => {  // WRONG, we must specify the return type
  return a + b
}

add = (a: Number, b: Number): Number => {  // OK, we specified the return type
  return a + b
}

logSomething = () => { // OK, the function returns no value
  log("something")
}
```

