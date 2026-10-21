# Errors

In Fa, errors are values. A function declares the errors it can fail with in its signature, using the `throws` keyword, and the compiler ensures that every error is either propagated or handled.

## Why not exceptions?

Exceptions, as found in most languages, are error-prone:

- the thrown value is not typed, so it cannot be safely recovered with its type,
- nothing in the signature of a function says that it can throw, so it is easy to forget to catch an exception.

Fa keeps the familiar `throw`, `try` and `catch` keywords, but errors are part of the signature of a function, like its parameters and its return type. An error never escapes silently, it is always typed, and the compiler knows the exact list of errors each call can fail with.

## Declaring error types

Error types are declared with `type` and the `Errors` type constructor. An `Errors` type has the same syntax as a [union](/types/unions):

```fa
type ApiError = Errors {
	AuthenticationError
	TimeoutError
	NetworkError
}
```

Each error can carry data:

```fa
type ApiError = Errors {
	AuthenticationError = {
		userId: String?
	}
	TimeoutError = {
		duration: Integer
	}
	NetworkError = {
		message: String
	}
}
```

## The `Error` type

Every error type satisfies the built-in `Error` type. Use `is Error` to check that a value is an error, whatever its type:

```fa
let error = ApiError.NetworkError { message = "Connection lost" }

assert error is Error
assert error is ApiError
```

Fa also provides built-in error types for operations that can fail:

| Error | Thrown by |
| --- | --- |
| `IndexError` | indexing or slicing a collection with an index the compiler cannot prove in bounds (`items[index]`) |
| `KeyNotFound` | reading a map with a key the compiler cannot prove present (`scores["ana"]`) |
| `DivisionByZero` | `/`, `//` and `%%` when the divisor is not provably non-zero |
| `ArithmeticError` | `checked*` arithmetic methods, like `checkedAdd`, when the result does not fit |

These operations are handled like any call that can fail:

```fa
function third = (items: Array(Integer)): Integer throws IndexError => {
	return try items[2]
}

let firstOrZero = items[0] catch error => 0
```

When you do not want an error, use the methods that return an [optional](/values/optionals) instead: `items.get(index)` and `scores.get(key)`. See [arrays](/collections/arrays), [maps](/collections/maps) and [arithmetic](/operations/arithmetic).

## Throwing errors

A function that can fail declares its errors after its return type, with `throws`. Several error types are separated by `|`.

Use the `throw` keyword to fail with an error. `throw` exits the current function, like `return`.

```fa
function loadScore = (): Integer throws ApiError => {
	let response = request("/score")

	if response.status == 401 {
		throw ApiError.AuthenticationError { userId = none }
	}

	return response.body.score
}
```

A function that returns nothing omits the return type:

```fa
function saveScore = (score: Integer) throws ApiError | StorageError => {
	-- ...
}
```

Like the return type, the errors of a function are inferred when its body is a single expression, and must be declared when its body is a block.

Throwing an error that is not declared in the signature is a compile error:

```fa
function saveScore = (score: Integer) throws StorageError => {
	throw ApiError.TimeoutError { duration = 30 } -- error: `saveScore` does not declare `ApiError`
}
```

Errors are never part of the return type: a function does not return `Integer | ApiError`, it returns `Integer` and `throws ApiError`.

## Propagating errors with `try`

Calling a function that can fail requires an explicit choice: propagate the error, or handle it.

To propagate the error to the caller, use `try`. If the call fails, the current function fails with the same error. Otherwise, `try` returns the value.

```fa
function printScore = () throws ApiError => {
	let score = try loadScore()
	console.log("Your score is: {score}")
}
```

`try` can only be used in a function that declares the propagated errors. Calling a function that can fail without `try` or `catch` is a compile error:

```fa
function printScore = () => {
	let score = loadScore() -- error: `loadScore` can fail with `ApiError`. Use `try` to propagate the error, or `catch` to handle it.
}
```

With asynchronous functions, `try` is written before `await`:

```fa
let user = try await fetchUser(id)
```

## Handling errors with `catch`

To handle an error, use `catch`. It binds the error to a name and evaluates an expression that replaces the failed value:

```fa
function printScore = () => {
	let score = loadScore() catch error => 0
	console.log("Your score is: {score}")
}
```

To handle each kind of error differently, use a `when` expression. `when` is exhaustive: the compiler checks that every error is handled, and tells you if you forgot one.

```fa
function printScore = () => {
	let score = loadScore() catch error => when error is {
		AuthenticationError => 0
		TimeoutError => {
			console.log("Timeout after {error.duration} seconds")
			return
		}
		NetworkError => {
			console.log("Network error: {error.message}")
			return
		}
	}
	console.log("Your score is: {score}")
}
```

An error can also be handled partially. The remaining errors are thrown again and must be declared in the signature:

```fa
function printScore = () throws ApiError => {
	let score = loadScore() catch error => when error is {
		TimeoutError => 0
		else => throw error
	}
	console.log("Your score is: {score}")
}
```

Errors are not "falsy": a condition only accepts a `Boolean`, so the result of a call is never tested with `if result { ... }` to detect a failure. A failure is handled with `try` or `catch`.

## Errors and optionals

Errors and optional values are two different things:

- an **optional** value (`User?`) may be `none`, which is a normal situation,
- an **error** (`throws NotFound`) means that the operation failed.

The `?.` and `??` operators only apply to optional values. Errors are only handled with `try` and `catch`.

### Choosing between errors and optionals

- Use an **optional** when absence is a normal, expected outcome: a lookup that found nothing.
- Use an **error** when something went wrong and the caller may want to know why.

```fa
function findUser = (id: Integer): User? => ...              -- may simply not exist
function loadUser = (id: Integer): User throws ApiError => ... -- may fail, with a reason
```
