# Requirements

Requirements let a function use values from its caller's environment (a database connection, a logger, the current user session, a configuration, ...) without passing them explicitly as arguments through every call.

Unlike global variables, requirements are **typed** and **checked at compile time**: calling a function without providing what it requires is a compile error.

:::tip
This is the "R" channel of the [Effect](https://effect.website) library, built into the language: a function signature tells what the function returns (`: Type`), how it can fail (`throws`), what it needs to run (`requires`) and what it modifies (`mutates`).
:::

## Declaring requirements

A function declares its requirements after its return type and its errors, with the `requires` keyword. A requirement is declared like a parameter, with a name and a type. Several requirements are separated by commas.

```fa
type Session = {
	user: {
		name: String
		email: String
	}
}

type Logger = {
	info: (message: String) => Void
}

function greet = () requires session: Session, logger: Logger => {
	logger.info("Hello, {session.user.name}!")
}
```

Inside the function, requirements are used like parameters.

Any type can be required: a plain object, or a type whose fields are function pointers, describing a [service](#services). The value provided only needs to match the type [structurally](/advanced/structural-typing).

## Providing requirements

Use the `provide` keyword to provide a value for a required type. It is available to every function called afterwards in the same block, directly or indirectly.

```fa
function main = () => {
	provide Session = {
		user = {
			name = "John Doe"
			email = "john.doe@example.com"
		}
	}
	provide Logger = {
		info = (message) => console.log(message)
	}

	greet() -- no need to pass the session and the logger explicitly
}
```

Requirements are never passed explicitly at the call site.

## Requirements are matched by type

Requirements are keyed by **type**, not by name: the name used in `requires` is local to the function. Two functions can name the same requirement differently, and still receive the same value.

Because they are keyed by type, unrelated requirements coexist without interfering:

```fa
type UserSession = { user: User }
type RequestInfo = { requestId: String }

function handle = () => {
	provide UserSession = { user = currentUser }
	provide RequestInfo = { requestId = "abc-123" }

	logRequest() -- receives the `RequestInfo`
	greetUser() -- receives the `UserSession`
}

function logRequest = () requires request: RequestInfo => {
	console.log("Request {request.requestId}")
}

function greetUser = () requires session: UserSession => {
	console.log("Hello, {session.user.name}!")
}
```

Providing a type that is already provided in the same block replaces it for the rest of the block.

## Requirements propagate

If a function calls another function that has requirements, it must either provide them, or declare them in its own signature:

```fa
-- `welcome` requires `Session` and `Logger` because it calls `greet`
function welcome = () requires session: Session, logger: Logger => {
	greet()
	logger.info("Welcome to Fa!")
}
```

Like the return type and the errors, the requirements of a function are inferred when its body is a single expression, and must be declared when its body is a block. This way, the signature of a function always tells everything it needs.

Calling a function without providing its requirements is a compile error, reported at the call site and never at runtime:

```fa
function main = () => {
	greet() -- error: `greet` requires `Session` and `Logger`, which are not provided. Use `provide Session = ...` or declare `requires session: Session` in `main`.
}
```

## Scoped requirements

A provided value is available until the end of the block where `provide` is used. To provide a different value for part of the code only, use a nested block, for example a [`do`](/control-flow/do) block:

```fa
function main = () => {
	provide Logger = {
		info = (message) => console.log(message)
	}
	provide Session = {
		user = {
			name = "John Doe"
			email = "john.doe@example.com"
		}
	}

	do {
		-- override the session for this block only
		provide Session = {
			user = {
				name = "Jane Smith"
				email = "jane.smith@example.com"
			}
		}

		greet() -- "Hello, Jane Smith!"
	}

	greet() -- "Hello, John Doe!"
}
```

## Requirements are references

A requirement is passed like a parameter: as an immutable reference. It is never copied down the call chain, however deep it goes.

To modify a requirement, a function declares the effect `mutates` on it, like it would for a parameter. The provided value must then be `mutable`:

```fa
type Trace = {
	breadcrumbs: Array(String)
}

function addBreadcrumb = (label: String) requires trace: Trace mutates trace => {
	trace.breadcrumbs.push(label)
}

function main = () => {
	mutable trace: Trace = { breadcrumbs = [] }
	provide Trace = trace

	addBreadcrumb("start")
	addBreadcrumb("loaded")

	console.log(trace.breadcrumbs) -- ["start", "loaded"]
}
```

A function calling `addBreadcrumb` without providing `Trace` itself must declare both `requires trace: Trace` and `mutates trace`.

## Services

A requirement whose fields are function pointers describes a **service**. It is the idiomatic way to inject dependencies, and to replace them in tests:

```fa
type Database = {
	findUser: (id: String) => User? throws DatabaseError
}

type UserError = Errors {
	NotFound = { id: String }
}

function getUser = (id: String): User throws DatabaseError | UserError requires database: Database => {
	let user = try database.findUser(id)

	if user is none {
		throw UserError.NotFound { id = id }
	}

	user
}
```

```fa
function main = () throws DatabaseError | UserError => {
	provide Database = postgresDatabase(url = "postgres://localhost/app")

	let user = try getUser("42")
	console.log(user.name)
}
```

In a test, provide an in-memory implementation instead:

```fa
-- getUser.test.fa
test "getUser returns the user" {
	provide Database = {
		findUser = (id) => User { id = id, name = "Test user" }
	}

	let user = try getUser("42")
	assert user.name == "Test user"
}
```

See [testing](/architecture/testing) for more about tests.

## Requirements and concurrency

Requirements follow the code started with [`start`](/advanced/concurrency#calling-async-functions): a task started in the background runs on the same thread, and sees the values provided where it was started.

Requirements do **not** cross worker boundaries. A [worker](/advanced/concurrency#multithreading) runs on another thread, and only receives copies of the arguments it is given: a value it needs has to be passed explicitly, or provided inside the worker.

## When to use them

Requirements suit values that genuinely belong to the environment of a whole call chain:

- services: a database, an HTTP client, a logger, a clock,
- the configuration of the application,
- the current user or session,
- a request id, a database transaction,
- a locale,
- a cancellation token.

:::note
Use requirements sparingly. They move information implicitly, which is exactly what makes them convenient, and exactly what makes code harder to follow. Regular data should be passed as arguments: if only two functions need a value, pass it explicitly.
:::
