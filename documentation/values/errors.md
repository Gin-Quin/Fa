# Errors 

In Fa, there are no `try / throw / catch` expressions. Instead, functions must return an explicit type using a union.

Throws are error-prone because:

- the thrown value cannot be safely typed and retrieved with its type,
- it's possible to forget to catch a throw.

Using the `errors` type, error-checking safety is enforced at compilation time.

The `errors` type is exactly similar to an `union` (same syntax and internal usage), with the difference that errored values are considered falsy and react differently to `?` and `!` operators.

## Declaring error types

```fa
errors ApiError = {
	AuthenticationError
	TimeoutError
	NetworkError
}

function apiCall = (): void | ApiError => {
	let networkIssueHappened = true
	if networkIssueHappened {
		return ApiError.NetworkError
	}
}
```

Since it's similar to an `union`, you can define additional parameters to your errors:

```fa
errors ApiError = {
	AuthenticationError = {
		userId: String?
	}
	TimeoutError = {
		timeoutDuration: Number
	}
	NetworkError = {
		message: String
	}
}


function apiCall = (): void | ApiError => {
	let networkIssueHappened = true
	if networkIssueHappened {
		return ApiError.NetworkError("Serious network issue")
	}
}
```

## The `!` operator

You can use the `!` operator to unwrap the value if it is successful, otherwise it returns the value itself, narrowed as non-error.

Example:

```fa
function printScore = (): void | ApiError => {
  let score = loadScore()!
  console.log("Your score is: {score}")
}
```

This is the exact equivalent of the following function:

```fa
function printScore = () => {
  let score = loadScore()
  if score is Error {
    return score
  }
  log("Your score is: {score}")
}
```

## The `?` operator

Like the `!` operator, the `?` operator can be used on an error type.

It tries to unwrap the value as the first type of the union. If it fails, it unwraps as the other type(s) of the union and stops the current operation.

The `?` operator can be used in several places:

1. When accessing the field of an object: `object?.field`.
2. When calling a function: `myFunction?()`.
3. When accessing an element of a collection that may be in error: `array?[index]`.

Example

```fa
function printScore = () => {
  console.log("Your score is: {loadScore()?}")
  console.log("Done")
}
```

This is the exact equivalent of the following function:

```fa
function printScore = () => {
  let score = loadScore()
  if score is not Error {
    console.log("Your score is: {score}")
  }
  console.log("Done")
}
```
