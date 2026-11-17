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

An error payload is data: it cannot own a Task, a resource with `destroy`, a built-in lifetime owner such as a socket or Worker, or any composite owning one. The declaration is rejected with `MEM005`: keep an identifier or a description of the resource instead. Shared synchronization handles (`Mutex`, `Channel` and atomics) are allowed; they keep their usual sharing contract.

```fa
type Failure = Errors { Bad = { task: Task<Integer> } } -- error: MEM005
-- Keep a task identifier or a description in the payload; keep ownership of the Task outside it.
```

The rule follows owned data through fields, tuples, optionals, unions and collections. A generic payload must satisfy the same restriction at each concrete instantiation. A concrete error member keeps its payload's [ownership category](/advanced/memory-management#structures): ordinary payloads keep value semantics, while collection or recursive payloads require `copy` when keeping two owners.

## The `Error` type

The open `Error` type is conservatively a structure, because its payload may own a collection, and is never unique. Its last use moves it; `copy error` is allowed, and storing it while the source is still used requires `copy` (`MEM004`). Narrowing to a concrete member uses that member's payload category.

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

| Error | Data | Thrown by |
| --- | --- | --- |
| `IndexError` | `index: Integer`, `length: Integer` | indexing or slicing a collection with an index the compiler cannot prove in bounds (`items[index]`) |
| `CapacityError` | none (`InlineFull`) | appending to an [InlineArray](/collections/inline-arrays) when room is not proven |
| `KeyNotFound` | `key: String`, the `toString()` of the key | reading a map with a key the compiler cannot prove present (`scores["ana"]`) |
| `DivisionByZero` | — | integer `//` and `%%` when the divisor is not provably non-zero (`/` never throws) |
| `ArithmeticError` | `message: String` | `checked*` arithmetic methods, like `checkedAdd`, when the result does not fit; a conversion to a smaller integer type (`Int8(value)`, `Integer(bigInteger)`) or from a float (`Integer.round(value)`) that is not proven to fit; an integer `**` with an exponent not proven non-negative; a shift by an unproven amount |
| `ParseError` | `input: String`, `message: String` | parsing text, like `Integer.parse(text)` |
| `Utf8Error` | `position: Integer`, the index of the first invalid byte | decoding bytes that are not valid UTF-8 with [`String.fromUtf8(bytes)`](/values/bytes#text-and-bytes) |
| `Cancelled` | — | awaiting a [cancelled task](/advanced/concurrency#cancelling-a-task) |

Each of them is an `Errors` type with a single member (`CapacityError.InlineFull`; the same name as the type for the others), so `IndexError` is both the type to declare (`throws IndexError`) and the pattern to match (`when error is { IndexError => ... }`).

What the compiler can prove is defined by the rules of [range analysis](/values/numbers#range-analysis).

These operations are handled like any call that can fail:

```fa
function third = (items: Array<Integer>): Integer throws IndexError => {
	return try items[2]
}

let firstOrZero = items[0] catch error => 0
```

When you do not want an error, use the methods that return an [optional](/values/optionals) instead: `items.get(index)` and `scores.get(key)`. See [arrays](/collections/arrays), [maps](/collections/maps) and [arithmetic](/operations/arithmetic).

## Throwing errors

A function that can fail declares its errors after its return type, with `throws`. Several error types are separated by `|`.

Use `throw` to fail with an error. A `throw` written inside `do … catch` is caught by that lexical `catch`, including a direct `throw` in the block. Without an enclosing handler in the function, the error leaves the function. A handler can throw again to propagate it. A `return` still exits the function.

```fa
type Failure = Errors { Bad }
function handled = (): Integer => do {
	throw Failure.Bad
} catch _error => 7
console.log(handled()) -- 7
```

Ownership and mutable state on entering the handler are those at the failure point: a value moved before the throw stays moved. Error cleanup destroys only the values still owned on that path.

```fa
function loadScore = (): Integer throws ApiError.AuthenticationError => {
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

A defined function must declare exactly the errors that can escape its body. Declaring an error that the body never propagates is an error, including when a `catch` handles every occurrence of that error. Throwing it again from the handler makes it part of the function's signature. This rule also applies to functions with single-expression bodies and explicitly typed declarations.

The set is exact down to each variant. With `type E = Errors { Bad, Other }`, a function that can only propagate `E.Bad` declares `throws E.Bad`; `throws E` would also declare `E.Other` and is an error. A union lists exactly the possible variants.

A value that throws fewer errors satisfies a type that throws more: `Task<T>` satisfies `Task<T> throws E`, and `(x: Integer) => Integer` satisfies `(x: Integer) => Integer throws E`. This applies to variables, collection items, fields, arguments and return values.

Such a function type describes the effects a value is allowed to have. Assigning an existing function with fewer effects to it remains valid; defining a function whose declared effects exceed its body's effects does not.

## Propagating errors with `try`

Calling a function that can fail requires an explicit choice: propagate the error, or handle it.

To propagate the error to the caller, use `try`. If the call fails, the current function fails with the same error. Otherwise, `try` returns the value.

```fa
function printScore = () throws ApiError.AuthenticationError => {
	let score = try loadScore()
	console.log("Your score is: {score}")
}
```

`try` can only be used in a function that declares the propagated errors, at the top level of a standalone file (an error that escapes ends the program, see [standalone files](/architecture/overview#standalone-files)), or in a test (the test fails). Calling a function that can fail without `try` or `catch` is a compile error:

```fa
function printScore = () => {
	let score = loadScore() -- error: `loadScore` can fail with `ApiError.AuthenticationError`. Use `try` to propagate the error, or `catch` to handle it.
}
```

Unlike Rust, there is no postfix `?` operator: `loadScore()?` is an error, write `try loadScore()`.

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

`catch` covers every failing call of its left operand: in `parse(read(path)) catch error => fallback`, it handles the errors of `read` and of `parse`, and `error` has the union of their error types. The handler extends as far right as possible, like the body of a lambda; a handler written with braces is a block. The handler's value has the type of the left operand when it fits (`text[index] catch error => "?"` is a `String`), like the branches of an [`if`](/control-flow/conditionals). Writing `try` and `catch` on the same expression is an error, and so is a `try` inside the left operand of a `catch`: the `catch` would handle the errors that `try` means to propagate.

A `try` or a `catch` on an expression that can't fail is a warning, not an error, so that code keeps compiling when a function it calls stops failing.

A handler is not a function: `return`, `throw`, `break` and `continue` inside it act on the enclosing function or loop, as in the example below. The name bound by `catch` only exists inside the handler. See [order of operations](/operations/arithmetic#order-of-operations).

For the next examples, `fetchScore` returns an `Integer` and can fail with any variant of `ApiError`. To handle each kind of error differently, use a `when` expression. `when` is exhaustive: the compiler checks that every error is handled, and tells you if you forgot one. Members are always written with their type, like `ApiError.TimeoutError`, and their data is bound with `as`.

```fa
function printScore = () => {
	let score = fetchScore() catch error => when error is {
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
function printScore = () throws ApiError.AuthenticationError | ApiError.NetworkError => {
	let score = fetchScore() catch error => when error is {
		ApiError.TimeoutError => 0
		else => throw error
	}
	console.log("Your score is: {score}")
}
```

Errors are not "falsy": a condition only accepts a `Boolean`, so the result of a call is never tested with `if result { ... }` to detect a failure. A failure is handled with `try` or `catch`.

### Effects before an error

An error never rolls back work already done: writes, removals and moves stay applied. A collection method runs its callback as it goes. If the callback throws, its own mutations remain and the method leaves the collection in its specified valid state. In `deleteWhere`, earlier matches stay removed, while the throwing item and the unvisited items remain in order. A failure does not restore a moved value.

When an operation has no specified intermediate state, failure preserves its original arrangement instead. A failing `sortInPlace` keeps the original item order, while mutations by its comparator remain. The compiler stages the order only when the comparator's type can throw; a comparator that cannot throw sorts in place without that buffer. The same distinction applies to any in-place operation: a specified prefix keeps completed effects; otherwise an operation that may fail applies its arrangement only on success.

## Stops

Some failures are not errors: they mean that the program is wrong, and there is nothing sensible for its caller to do. When a runtime check that the compiler couldn't prove away fails, the program **stops**:

- an integer operation overflows (see [overflow](/operations/arithmetic#overflow)),
- an `assert` is false outside of a test,
- the call stack overflows, or memory runs out.

`assert` is always evaluated, in every build: it is not removed in release builds. After it, the rest of the block knows that its condition is true, like after an early exit.

A stop prints `stop: ` followed by the reason, then the source location, on the standard error output:

```text
stop: integer overflow: 300 doesn't fit in UInt8
  at main.fa:12:7
```

Then it ends the program with the exit code `101`. It is not an error: Fa code can't catch it, throw it or declare it, and there is no panic that can be caught. In a test, a stop fails the test, and the test runner goes on with the next test file.

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

- Every task promise has an error slot, null for a task that cannot fail, so widening its error type keeps its frame. A function value widened to a throwing type uses an adapter holding a copy of the function.

### JavaScript

- A Fa error is thrown as a JavaScript exception: each member is a class extending a runtime `FaError` class, not `Error`, so a failure doesn't capture a stack (a debug build may). `try` is the default propagation of exceptions, and `catch` compiles to a JavaScript `try`/`catch` around the covered calls, which rethrows everything that is not a `FaError`: a `FaStop`, or a JavaScript exception thrown by foreign code.
- An error escaping the top level of a program prints `error: ` and its `toString()`, and exits with code 1.
- A JavaScript `RangeError` for a call-stack overflow is converted into a stop by the entry point.
- A stop throws a `FaStop` instance, which no compiled `catch` intercepts. The entry point catches it at the top, prints the message and the location, and exits with code 101 (`process.exit(101)` on server runtimes; on the web, the error is reported in the console and the program stops running).
:::
