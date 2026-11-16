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
test "adds two numbers" {
	assert math.add(2, 3) == 5
}

test "handles negative numbers" {
	assert math.add(-2, 3) == 1
	assert math.add(-2, -3) == -5
}
```

Tests are written at the top level of a `*.test.fa` file, or in its namespaces (see [grouping](#grouping)). A test file exports nothing: it is the one exception to the rule of [one export per file](/architecture/modules#core-principles). It is only read by `fa test` and `fa check`, and its name can't be used by other files. Besides tests, it can declare values, functions and types for its tests, but it can't contain other top-level statements.

Tests can only be declared in test files. Two tests with the same name in the same namespace are a compile error.

The code under test is available implicitly, by the usual [module](/architecture/modules) rules: `math.test.fa` sits next to `math.fa`, so `math` is in scope.

## Assertions

`assert` is a keyword followed by a condition, without parentheses. The condition must be a `Boolean`:

```fa
assert value == 42
```

A failing assertion reports the expression, the values of its operands, and the source location, and fails the test. There is nothing to learn beyond `assert`, because Fa's operators already express what other frameworks need matchers for:

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

Outside tests, `assert` checks the condition at runtime: when it is false, the program [stops](/values/errors), with the same report on the error output and the exit code `101`.

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
function makeUser = (name: String) => User { name = name, active = true }

test "a new user is active" {
	assert makeUser("Ana").active
}
```

## Asynchronous tests

A test body is an asynchronous context, like the body of an `async function`: it can use `await`, without any special syntax:

```fa
test "fetches a profile" {
	let profile = try await loadProfile(42)

	assert profile.id == 42
}
```

## Compile-time tests

A test declared with `@test` runs during compilation, on the compile-time interpreter, when `fa check` or `fa test` checks the test file (test files are not part of `fa build`). Use it for pure functions and type-level code: a failure is a compile error, and the test costs nothing at runtime. Like any compile-time code, its body must be [pure](/advanced/metaprogramming).

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

`fa test` fails if a `test only` exists when it runs in continuous integration (the `CI` environment variable is set) or on the default branch of the Git repository (the branch of `origin/HEAD`, else the `init.defaultBranch` of Git, else `main`), so a focused test cannot be merged by accident. Outside a Git repository, only the `CI` variable is checked.

## How tests run

- Each test runs with freshly initialized [module-level values](/architecture/modules#module-level-values-and-initialization): a test never sees what another test wrote into an `export mutable` value.
- The tests of a file run one after the other, in the order of the file. Different test files may run in parallel.
- A test passes when its body ends. It fails when an assertion is false, when an error escapes its body, or when it takes longer than the `timeout` of the [configuration](/architecture/configuration#test).
- When a test [stops](/values/errors) on a failed runtime check (an integer overflow, for example), the test fails, the remaining tests of its file are reported as not run, and the runner goes on with the next test file.
- `fa test` exits with the code `0` when every test that ran passed, and `1` otherwise.

- A test file still running after `FA_TEST_TIMEOUT` seconds (300 by default) is stopped and reported as failed with that reason.

## Coverage

```bash
fa test --coverage
```

Reports line and branch coverage per file, and writes an LCOV report to `build/coverage`.

## Testing across targets

Tests run on the `bun` target by default, which is the fastest to start, even when `bun` is not one of the targets of the package. Run them on another target when the behavior of a target matters:

```bash
fa test --target=native
fa test --target=wasm
fa test --target=wasm --watch
```

The watch mode keeps the selected target. WebAssembly tests run through their generated loader on Node.js or Bun.

## Configuration

The files to include and the timeout of each test are set in the `test` field of the [configuration](/architecture/configuration#test).

:::implementation
## Implementation details

- Each test file is compiled once. Bun runs a process per file and resets the module bindings before each test. Native tests compile at `-O0`, load the module through LLVM ORC in `fa-native`, and fork a fresh child for each test body. `FA_JIT=0` and programs the JIT cannot load use a linked executable. Native selects a body by an argument; WebAssembly runs a fresh process per body using the `fa.testIndex` import.
- A failed check or timeout marks the remaining tests of that file as not run; other files continue. The native and WebAssembly runner kills a process that exceeds the configured timeout.
:::
