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
fa test              # every test in the package or project
fa test myPackage    # one package
fa test math         # every test file matching "math"
fa test --watch      # re-run on change
```

## Writing a test

A test file exports a namespace of test blocks. Each `test` takes a name and a body:

```fa
-- math.test.fa
export namespace = {
  test "adds two numbers" {
    assert(math.add(2, 3) == 5)
  }

  test "handles negative numbers" {
    assert(math.add(-2, 3) == 1)
    assert(math.add(-2, -3) == -5)
  }
}
```

The code under test is available implicitly, by the usual [module](./modules) rules: `math.test.fa` sits next to `math.fa`, so `math` is in scope.

## Assertions

`assert` takes a condition and an optional message:

```fa
assert(value == 42)
assert(value == 42, "the answer should be 42")
```

A failing assertion reports the expression, both operand values, and the source location. There is nothing to learn beyond `assert`, because Fa's operators already express what other frameworks need matchers for:

```fa
assert(result is none)
assert(result is not none)
assert(value is String)
assert(userId is in allowedIds)
assert(0 < score <= 100)
assert(users.length == 3)
assert(point == Point(1, 2))     -- structural equality, deep by default
```

For errors, match on the result:

```fa
test "rejects an invalid port" {
  let result = readPort("not a number")

  assert(result is ParseError)
}
```

## Grouping

Nest namespaces to group related tests:

```fa
export namespace = {
  namespace addition = {
    test "adds positives" {
      assert(math.add(2, 3) == 5)
    }
  }

  namespace division = {
    test "divides evenly" {
      assert(math.divide(6, 3)! == 2)
    }

    test "reports division by zero" {
      assert(math.divide(6, 0) is DivisionByZero)
    }
  }
}
```

## Setup and teardown

A test body is an ordinary block, so setup is just code, and teardown happens by scope:

```fa
test "writes to the database" {
  use database = testDatabase()

  database.insert(User { name = "Ana" })

  assert(database.count() == 1)
} -- `database` is released here
```

For setup shared by several tests, write a function:

```fa
export namespace = {
  function makeUser = (name: String): User => User { name, active = true }

  test "a new user is active" {
    assert(makeUser("Ana").active)
  }
}
```

## Asynchronous tests

A test body can `await`, without any special syntax:

```fa
test "fetches a profile" {
  let profile = await loadProfile(42)!

  assert(profile.id == 42)
}
```

## Compile-time tests

A test marked `@test` runs during compilation. Use it for pure functions and type-level code: a failure is a compile error, and the test costs nothing at runtime.

```fa
@test "the version parses" {
  assert(parseVersion("1.2.3").major == 1)
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

`fa test` fails if a `test only` reaches the main branch, so a focused test cannot be committed by accident.

## Coverage

```bash
fa test --coverage
```

Reports line and branch coverage per file, and writes an LCOV report to `build/coverage`.

## Testing across platforms

Tests run on the `bun` platform by default, which is the fastest to start. Run them on another target when platform behaviour matters:

```bash
fa test --platform native
fa test --platform web
```
