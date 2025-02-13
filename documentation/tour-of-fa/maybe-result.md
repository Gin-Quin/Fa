In Fa, there are no `try / throw / catch` expressions. Instead, every function can return a special error result.

Throws are error-prone since:

- the thrown value cannot be safely typed and retrieved with its type,
- it's possible to forget to catch a throw.

Using `Maybe`, error-checking safety is enforced at compilation time.

Example of use case:

```ts
// let's define a function that may return a number
loadScore = (): Maybe(Number) => {
  score = api.loadScore()
  if score is undefined {
    return Error("No score!")
  }
  return score
}

// then let's use it this way:
score = loadScore()
if score.error {
  print score.error.message
} else {
  print "Your score is: {score.value}"
}
```

## The exclamation mark shorthand

Always having to check if an error is returned can be cumbersome, especially if it is to return the error to a caller function, that will itself return the error to its caller function, etc...

That's why a logic has been put into the exclamation mark operator '!'. It is used this way:

```ts
printScore = () => {
  log("Your score is: {loadScore()!}")
}
```

This is the exact equivalent of the following function:

```ts
printScore = () => {
  score = loadScore()
  if score.error {
    return /* score.error */ // <- Since the function returns "Void", the exclamation mark operator will return void and not the error
  }
  log("Your score is: {score.value}")
}
```

In other words, the `!` operator unwraps the value if it is successful, otherwise it returns the error.
