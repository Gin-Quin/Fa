# Errors

Fa has no `try`, no `throw` and no `catch`. A function that can fail says so in its return type, and returns the error as an ordinary value.

Exceptions are error-prone because:

- the thrown value cannot be safely typed and recovered with its type,
- it is possible to forget to catch a throw.

Errors in Fa are checked at compile time: a value that may be an error cannot be used as if it were not.

## Declaring errors

An error type is declared with `type` and the `Errors` constructor. It behaves exactly like a [union](../types/unions), with two differences: its members are considered falsy, and they participate in the `!` operator.

```fa
type ApiError = Errors {
  AuthenticationError
  TimeoutError
  NetworkError
}

function apiCall = (): None | ApiError => {
  let networkIssueHappened = true

  if networkIssueHappened {
    return ApiError.NetworkError
  }
}
```

Like union members, error members can carry data:

```fa
type ApiError = Errors {
  AuthenticationError = {
    userId: String?
  }
  TimeoutError = {
    timeoutDuration: Int
  }
  NetworkError = {
    message: String
  }
}

function apiCall = (): None | ApiError => {
  if networkIssueHappened {
    return ApiError.NetworkError { message = "Serious network issue" }
  }
}
```

Every error type is a subtype of the built-in `Error` type, so `value is Error` tests whether a value is any error at all.

## Returning errors

A fallible function returns a union of the success type and the error type:

```fa
function loadScore = (): Int | ApiError => {
  let response = apiCall()

  if response is Error {
    return response
  }

  return response.score
}
```

## The `!` operator

`!` unwraps a value if it succeeded, and returns the error from the **enclosing function** if it did not. The expression's type is narrowed to the non-error case.

```fa
function printScore = (): None | ApiError => {
  let score = loadScore()!
  console.log("Your score is: {score}")
}
```

This is exactly equivalent to:

```fa
function printScore = (): None | ApiError => {
  let score = loadScore()

  if score is Error {
    return score
  }

  console.log("Your score is: {score}")
}
```

`!` can only be used inside a function whose return type includes the error being propagated. Otherwise the compiler rejects it:

```fa
function printScore = (): None => {
  let score = loadScore()! -- error: `printScore` cannot return `ApiError`
}
```

:::caution
`!` never aborts the program. Fa has no panics on errors: `!` is a *return*, and the caller must handle the value it returns.
:::

## Handling errors

To deal with an error instead of propagating it, match on it:

```fa
let score = when loadScore() is {
  Int >> value => value
  ApiError.TimeoutError => 0
  ApiError >> error => {
    console.log("Could not load the score: {error}")
    0
  }
}
```

`when` is exhaustive: if you forget an error member, the compiler tells you.

## Errors are falsy

Errors are falsy, so a truthiness test is enough when you do not care which error occurred:

```fa
let score = loadScore()

if not score {
  console.log("No score available")
}
```

## Errors and composition

When you compose functions with `||>`, an error returned by any step short-circuits the rest of the chain, and the composed function's return type is the union of every step's error type:

```fa
function loadUser = (id: Int): User | DatabaseError => ...
function loadScore = (user: User): Int | ApiError => ...

-- type: (Int) => Int | DatabaseError | ApiError
function loadUserScore = loadUser ||> loadScore
```

See [function composition](../functions/function-composition).

## Choosing between errors and optionals

- Use an **optional** when absence is a normal, expected outcome: a lookup that found nothing.
- Use an **error** when something went wrong and the caller may want to know why.

```fa
function findUser = (id: Int): User?             -- may simply not exist
function loadUser = (id: Int): User | ApiError   -- may fail, with a reason
```
