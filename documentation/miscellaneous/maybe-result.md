In Fa, there are no `try / throw / catch` expressions. Instead, every function can return a special error result.

Throws arre error-prone since:

- the thrown value cannot be safely typed and retrieved with its type,
- it's possible to forget to catch a throw.

Using `Maybe`, error-checking safety is enforced at compilation time.

Example of use case:

```coffee
# let's define a function that may return a number
const loadScore = (): Maybe<Number> ->
  const score = api.loadScore()
  if (score = undefined)
    return Error("No score!")
  return score

# then let's use it this way:
const score = loadScore()
if (score.error)
  print score.error.message
else
  print "Your score is: {score.value}"
```

## The exclamation mark shorthand

Always having to check if an error is returned can be cumbersome, especially if it is to return the error to a caller function, that will itself return the error to its caller function, etc...

That's why a logic has been put into the exclamation mark operator '!'. It is used this way:

```coffee
printScore = (): Maybe ->
  print "Your score is: {loadScore()!}"
```

This is the exact equivalent of the following function:

```coffee
printScore = (): Maybe ->
  const score = loadScore()
  if (score.error)
    return score.error
  print "Your score is: {score.value}"
```

In other words, the `!` operator unwraps the value if it is successful, otherwise it returns the error.
