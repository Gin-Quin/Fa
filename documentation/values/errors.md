# Errors

In Fa, errors are values. A function declares the errors it can fail with in its signature, using the `throws` keyword, and the compiler ensures that every error is either propagated or handled.

There are no exceptions: an error never escapes silently, and it is always typed.

## Declaring error types

Error types are declared with the `errors` keyword. An `errors` type has the same syntax as a [union](/types/unions):

```fa
errors ApiError = {
	AuthenticationError
	TimeoutError
	NetworkError
}
```

Each error can carry data:

```fa
errors ApiError = {
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

## Throwing errors

A function that can fail declares its errors after its return type, with `throws`. Several error types are separated by `|`.

Use the `throw` keyword to fail with an error. `throw` exits the current function, like `return`.

```fa
function loadScore = (): Integer throws ApiError => {
	let response = request("/score")

	if response.status == 401 {
		throw ApiError.AuthenticationError(userId = none)
	}

	response.body.score
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
	throw ApiError.TimeoutError(duration = 30) -- error: `saveScore` does not declare `ApiError`
}
```

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

To handle each kind of error differently, use a `when` expression. The compiler checks that every error is handled:

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

## Errors and optionals

Errors and optional values are two different things:

- an **optional** value (`User?`) may be `none`, which is a normal situation,
- an **error** (`throws NotFound`) means that the operation failed.

The `?.` operator only applies to optional values. Errors are only handled with `try` and `catch`.
