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

Every member of an `Errors` type satisfies the built-in `Error` type: a value typed `Error` accepts any error. `Error` only exposes `toString(): String`, which gives the qualified name of the member and its data, like `ApiError.NetworkError { message = "Connection lost" }`; to read the data of an error, narrow it to its member first. Use `is` to tell error types apart in a union:

```fa
function describe = (error: ApiError | StorageError): String => {
	if error is ApiError {
		return "the API failed"
	}
	"the storage failed"
}
```

As with every `is` on a type, a test that the compiler can decide (like `is Error` on a value that is always an error) is a warning when it is always true, and an error when it is never true.

Fa also provides built-in error types for operations that can fail:

| Error | Thrown by |
| --- | --- |
| `IndexError` | indexing or slicing a collection with an index the compiler cannot prove in bounds (`items[index]`) |
| `KeyNotFound` | reading a map with a key the compiler cannot prove present (`scores["ana"]`) |
| `DivisionByZero` | integer `//` and `%%` when the divisor is not provably non-zero (`/` never throws) |
| `ArithmeticError` | `checked*` arithmetic methods, like `checkedAdd`, when the result does not fit; a conversion to a smaller integer type (`Int8(value)`) or from a float (`Integer.round(value)`) that is not proven to fit; an integer `**` with an exponent not proven non-negative; a shift by an unproven amount |

What the compiler can prove is defined by the rules of [range analysis](/values/numbers#range-analysis).

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

Like the return type, the errors of a function declared with `function` are inferred when its body is a single expression, and must be declared when its body is a block. A lambda takes its errors from its expected function type (see [defining functions](/functions/defining-functions)).

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

`try` covers the whole expression on its right: `try parse(read(path))` propagates the errors of `read` and of `parse`. It also prefixes an assignment whose target can fail, like an index that is not proven in bounds, and then covers the whole assignment:

```fa
try scores[index] = 0
try counts[key] += 1
```

## Handling errors with `catch`

To handle an error, use `catch`. It binds the error to a name and evaluates an expression that replaces the failed value:

```fa
function printScore = () => {
	let score = loadScore() catch error => 0
	console.log("Your score is: {score}")
}
```

`catch` covers every failing call of its left operand: in `parse(read(path)) catch error => fallback`, it handles the errors of `read` and of `parse`, and `error` has the union of their error types. The handler extends as far right as possible, like the body of a lambda; a handler written with braces is a block. Writing `try` and `catch` on the same expression is an error.

A handler is not a function: `return`, `throw`, `break` and `continue` inside it act on the enclosing function or loop, as in the example below. The name bound by `catch` only exists inside the handler. See [order of operations](/operations/arithmetic#order-of-operations).

To handle each kind of error differently, use a `when` expression. `when` is exhaustive: the compiler checks that every error is handled, and tells you if you forgot one. Members are always written with their type, like `ApiError.TimeoutError`, and their data is bound with `as`.

```fa
function printScore = () => {
	let score = loadScore() catch error => when error is {
		ApiError.AuthenticationError => 0
		ApiError.TimeoutError as { duration } => {
			console.log("Timeout after {duration} seconds")
			return
		}
		ApiError.NetworkError as { message } => {
			console.log("Network error: {message}")
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
		ApiError.TimeoutError => 0
		else => throw error
	}
	console.log("Your score is: {score}")
}
```

Errors are not "falsy": a condition only accepts a `Boolean`, so the result of a call is never tested with `if result { ... }` to detect a failure. A failure is handled with `try` or `catch`.

## Stops

Some failures are not errors: they mean that the program is wrong, and there is nothing sensible for its caller to do. When a runtime check that the compiler couldn't prove away fails, the program **stops**:

- an integer operation overflows (see [overflow](/operations/arithmetic#overflow)),
- an `assert` is false outside of a test.

A stop prints a message with the source location on the standard error output, and ends the program with the exit code `101`. It is not an error: Fa code can't catch it, throw it or declare it, and there is no panic that can be caught. In a test, a stop fails the test, and the test runner goes on with the next test file.

```fa
let byte: UInt8 = x + y -- stops the program if the result exceeds 255
```

Use the explicit variants to turn a possible stop into an error or a defined result: `checkedAdd` throws `ArithmeticError`, `wrappingAdd` wraps around. See [numbers](/values/numbers#overflow).

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

:::implementation
## Implementation details

### Native

- A function that `throws` returns its error through a separate return slot (a tagged result: the error tag and payload next to the return value), never by unwinding. `try` is a branch on the tag; `catch` is a branch to the handler.
- A stop calls a runtime routine that prints the message and the source location to stderr, then exits with code 101, without unwinding or running destructors.

### JavaScript

- A Fa error is thrown as a JavaScript exception carrying the error value; `try` is the default propagation of exceptions, and `catch` compiles to a JavaScript `try`/`catch` around the covered calls, which rethrows `FaStop`.
- A stop throws a `FaStop` instance, which no compiled `catch` intercepts. The entry point catches it at the top, prints the message and the location, and exits with code 101 (`process.exit(101)` on server runtimes; on the web, the error is reported in the console and the program stops running).
:::
