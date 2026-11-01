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

Any type can be required: a plain object, or a type whose fields are function pointers, describing a [service](#services).

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

- A provided value is visible **from the `provide` statement to the end of its block**. A call made before the `provide` statement doesn't see it.
- `provide Type = place` (a value, a field, an element) provides an **alias** of the place, like [`use`](/values/declaring-values): nothing is copied, and the provided value is `mutable` if the place is.
- `provide Type = expression` (an object literal, a call) provides a **temporary**, owned by the block and destroyed at its end.
- The value only needs to satisfy the provided type [structurally](/advanced/structural-typing): a value with more fields is seen through the provided type.

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

The key is the **exact shape** of the type, since Fa's typing is [structural](/advanced/structural-typing):

- An alias is the same key: after `type AppLogger = Logger`, `provide AppLogger` satisfies `requires logger: Logger`.
- Two named types with the same shape are the same key: providing one provides the other.
- There is no lookup by subtype: `requires logger: Logger` is only satisfied by a provided `Logger` (or a type of exactly the same shape), never by a provided type with more fields. Provide the value as a `Logger` instead: `provide Logger = verboseLogger`.
- A generic type is keyed with its arguments: `Cache(User)` and `Cache(Post)` are two keys.

To keep two requirements of the same shape apart, give them different shapes, or use single-member unions, which are distinct by name:

```fa
type Source = { path: String }
type Destination = { path: String } -- same shape: providing `Destination` replaces `Source`

type InputFile = Union { Input = { path: String } }
type OutputFile = Union { Output = { path: String } } -- distinct keys
```

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

The entry function of an application can't declare requirements, since nothing calls it: it provides them. So do the top-level statements of a standalone file, and [tests](/architecture/testing).

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

A `provide` inside a branch of an `if` or a `when` only lasts until the end of that branch.

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
	provide Trace = trace -- an alias of `trace`

	addBreadcrumb("start")
	addBreadcrumb("loaded")

	console.log(trace.breadcrumbs) -- ["start", "loaded"]
}
```

A function calling `addBreadcrumb` without providing `Trace` itself must declare both `requires trace: Trace` and `mutates trace`.

A call that mutates a requirement has the same consequences as a call that mutates an argument: it invalidates the local references into the provided value (`use first = trace.breadcrumbs[0]` must be declared again after `addBreadcrumb`), and the place passed to a `mutates` parameter of the call can't overlap a requirement the callee uses.

## Requirements and function values

Requirements are resolved **where a function is called**, and never captured by a closure:

- A lambda passed to a function that only calls it (like `map`) has its requirements resolved where the lambda is written: they become requirements of the call, like its other [effects](/functions/defining-functions#inferred-effects).
- A function that is stored (in a field, a collection, a returned closure) gets its requirements where it is eventually called: its [function type](/functions/defining-functions#function-types) must declare them, like every other effect.

```fa
type Loader = (id: String) => User throws LoadError requires database: Database

function loadAll = (ids: Array(String), load: Loader): Array(User) throws LoadError requires database: Database => {
	try ids.map((id) => try load(id)) -- each call of `load` receives the `Database` that `loadAll` receives
}
```

A `watch` block and a [task](/advanced/concurrency#tasks) are the exception, since they run later: they keep the values provided where they are created, and can't outlive the block that provides them. A `derived` value can't use requirements at all, since it must be [pure](/functions/defining-functions#pure-functions).

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

Requirements follow the code started with [`start`](/advanced/concurrency#calling-async-functions): a task started in the background runs on the same thread, and sees the values provided where it was started. A task uses its requirements like its arguments: a task that is kept can't outlive the block that provides them, and while it runs, a requirement it modifies can't be used by other code. A detached task (a `start` whose task is not kept) can't use requirements.

The body of a [parallel loop](/advanced/concurrency#1-parallel-iterations) can read the requirements, but can't modify them unless they are thread-safe.

Requirements do **not** cross worker boundaries. A [worker](/advanced/concurrency#multithreading) runs on another thread, and only receives copies of the arguments it is given: a value it needs has to be passed explicitly. The methods of a worker type can't declare requirements.

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

:::implementation
## Implementation details

Requirements are resolved entirely at compile time: there is no lookup at runtime.

- **Keys**: the key of a requirement is the interned shape of its type (the same identity as [structural typing](/advanced/structural-typing) uses), with its generic arguments.
- **Passing**: each requirement is a hidden pointer parameter, appended after the regular parameters in a canonical order (by key). The checker resolves each call to the innermost `provide` of that key visible at the call, or to the caller's own hidden parameter.
- **`provide`**: providing a place passes its address; providing an expression allocates a stack slot (on native) or a local (on JavaScript) for the temporary, destroyed at the end of the block.
- **Function values**: a function type that declares requirements has the same hidden parameters, so they are part of the ABI of a function value. A non-escaping lambda receives them from the frame that creates it.
- **Watch blocks and tasks**: the provided pointers are stored in the watch node or the task frame when it is created; the checker's rule that they can't outlive the providing block keeps the pointers valid.
- **JavaScript**: the hidden parameters are extra trailing arguments, in the same canonical order. A provided value is a local variable; an alias of a primitive place is passed through the same wrapper as a mutated `export mutable` value.
:::
