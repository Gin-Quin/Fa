# Requirements

Requirements let a function use values from its caller's environment (a database connection, a logger, the current user session, a configuration, ...) without passing them explicitly as arguments through every call.

Unlike global variables, requirements are **typed** and **checked at compile time**: calling a function without providing what it requires is a compile error.

:::tip
This is the "R" channel of the [Effect](https://effect.website) library, built into the language: a function signature tells what the function returns (`: Type`), how it can fail (`throws`) and what it needs to run (`requires`).
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

Any type can be required: a plain object, or a type whose fields are function pointers, describing a service. The value provided only needs to match the type [structurally](/advanced/structural-typing).

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
	provide Logger = ConsoleLogger()

	greet() -- no need to pass the session and the logger explicitly
}
```

Requirements are matched by type: the name used in `requires` is local to the function.

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

Calling a function without providing its requirements is a compile error:

```fa
function main = () => {
	greet() -- error: `greet` requires `Session` and `Logger`, which are not provided. Use `provide Session = ...` or declare `requires session: Session` in `main`.
}
```

## Scoped requirements

A provided value is available until the end of the block where `provide` is used. To provide a different value for part of the code, use a nested block:

```fa
function main = () => {
	provide Logger = ConsoleLogger()
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

## Services

A requirement whose fields are function pointers describes a **service**. It is the idiomatic way to inject dependencies, and to replace them in tests:

```fa
type Database = {
	findUser: (id: String) => User? throws DatabaseError
}

errors UserError = {
	NotFound = { id: String }
}

function getUser = (id: String): User throws DatabaseError | UserError requires database: Database => {
	let user = try database.findUser(id)

	if user is none {
		throw UserError.NotFound(id)
	}

	user
}
```

```fa
function main = () throws DatabaseError | UserError => {
	provide Database = PostgresDatabase("postgres://localhost/app")

	let user = try getUser("42")
	console.log(user.name)
}
```

```fa
-- in a test, provide an in-memory implementation instead
function getUserReturnsTheUser = () throws DatabaseError | UserError => {
	provide Database = {
		findUser = (id) => User { id, name = "Test user" }
	}

	let user = try getUser("42")
	assert user.name == "Test user"
}
```

:::note
Requirements should be used for values that belong to the environment of a program (services, configuration, session). Regular data should be passed as arguments.
:::
