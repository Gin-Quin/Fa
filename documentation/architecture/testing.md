# Testing

Tests live next to the code they test, in files ending in `.test.fa`:

```
library/
├── math.fa
└── math.test.fa
```

Test files are excluded from builds, so test-only code never reaches your output.

Run them with:

```bash
fa test  # <-- every test of the package or workspace
fa test myPackage  # <-- the tests of one package of the workspace
fa test --watch  # <-- re-run the tests on change
```

## Writing a test

A test is declared with the `test` keyword, followed by its name and its body:

```fa
-- math.test.fa
export namespace = {
	test "adds two numbers" {
		assert math.add(2, 3) == 5
	}

	test "handles negative numbers" {
		assert math.add(-2, 3) == 1
		assert math.add(-2, -3) == -5
	}
}
```

The code under test is available implicitly, by the usual [module](/architecture/modules) rules: `math.test.fa` sits next to `math.fa`, so `math` is in scope.

## Assertions

`assert` is a keyword followed by a condition, without parentheses. The condition must be a `Boolean`:

```fa
assert value == 42
```

A failing assertion reports the expression, the values of its operands, and the source location. There is nothing to learn beyond `assert`, because Fa's operators already express what other frameworks need matchers for:

```fa
assert result is none
assert result is not none
assert value is String
assert userId is in allowedIds
assert 0 < score <= 100
assert users.length == 3
assert point == Point { x = 1, y = 2 } -- deep structural equality
assert tags == Set.from(["a", "b"]) -- sets compare without order
```

## Testing errors

A test body can propagate errors with `try`: if an error escapes the test, the test fails and the error is reported.

```fa
test "reads a valid port" {
	let port = try readPort("8080")

	assert port == 8080
}
```

To check that a function fails, handle the error with `catch`:

```fa
test "rejects an invalid port" {
	let port = readPort("not a number") catch error => when error is {
		PortError.InvalidNumber => -1
		else => throw error -- any other error fails the test
	}

	assert port == -1
}
```

## Providing requirements

When the code under test has [requirements](/advanced/requirements), the test provides them with `provide`, exactly like application code does:

```fa
test "greets the current user" {
	provide Session = { user = { id = "42", name = "Ada" } }

	assert greeting() == "Hello, Ada!"
}
```

This is how dependencies are replaced by test doubles: a requirement whose type is a service (a type whose fields are function pointers) can be provided with a fake implementation.

```fa
test "loads the score from the API" {
	provide ScoreApi = {
		fetchScore = (id: String) => 42
	}

	let score = try loadScore("ada")

	assert score == 42
}
```

## Grouping

Nest namespaces to group related tests:

```fa
export namespace = {
	namespace addition = {
		test "adds positives" {
			assert math.add(2, 3) == 5
		}
	}

	namespace division = {
		test "divides evenly" {
			let result = try math.divide(6, 3)

			assert result == 2
		}

		test "reports a division by zero" {
			let result = math.divide(6, 0) catch error => when error is {
				DivisionByZero => -1
				else => throw error
			}

			assert result == -1
		}
	}
}
```

## Setup and teardown

A test body is an ordinary block, so setup is just code, and teardown happens by scope: values are destroyed at the end of the block.

```fa
test "writes to the database" {
	mutable database = testDatabase()

	database.insert(User { name = "Ana" })

	assert database.count() == 1
} -- `database` is destroyed here
```

For setup shared by several tests, write a function:

```fa
export namespace = {
	function makeUser = (name: String) => User { name = name, active = true }

	test "a new user is active" {
		assert makeUser("Ana").active
	}
}
```

## Asynchronous tests

A test body can use `await`, without any special syntax:

```fa
test "fetches a profile" {
	let profile = try await loadProfile(42)

	assert profile.id == 42
}
```

## Compile-time tests

A test declared with `@test` runs during compilation. Use it for pure functions and type-level code: a failure is a compile error, and the test costs nothing at runtime.

```fa
@test "the version parses" {
	let version = try parseVersion("1.2.3")

	assert version.major == 1
}
```

## Skipping and focusing

```fa
test skip "not ready yet" {
	...
}

test only "the one I am debugging" {
	...
}
```

`test skip` declares a test that is not run. When a `test only` exists, only the focused tests are run.

`fa test` fails if a `test only` reaches the main branch, so a focused test cannot be committed by accident.

## Coverage

```bash
fa test --coverage
```

Reports line and branch coverage per file, and writes an LCOV report to `build/coverage`.

## Testing across targets

Tests run on the `bun` target by default, which is the fastest to start. Run them on another target when the behavior of a target matters:

```bash
fa test --target=native
fa test --target=web
```

## Configuration

The files to include and the timeout of each test are set in the `test` field of the [configuration](/architecture/configuration#test).
