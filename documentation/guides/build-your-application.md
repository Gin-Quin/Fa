# Build your application

This guide walks through a small but complete application: a command-line tool that reads a file of scores and reports statistics. It touches the pieces you will use in any Fa program: packages, errors, collections and tests.

## Scaffold

```bash
fa new package scores
cd scores
```

```
scores/
├── package.fa
├── dependencies.fa
├── library/
└── application/
    └── main.fa
```

Set the targets in `package.fa`:

```fa
-- package.fa
export = {
	name = "scores"
	version = "0.1.0"
	targets = ["native", "bun"]
}
```

## Model the data

Put the logic in `library/`, so it can be tested and reused. Start with the types. Each file exports exactly one value, named after the file.

```fa
-- library/Score.fa
export type = {
	name: String
	value: Integer(min = 0, max = 100)
}
```

The range on `value` is not documentation: the compiler rejects any code that could put a number outside it into this field.

Errors are declared as types, with the `Errors` type constructor. In `ScoreError.fa`, `export type = Errors { ... }` declares the `ScoreError` type:

```fa
-- library/ScoreError.fa
export type = Errors {
	FileNotFound = { path: String }
	MalformedLine = { line: Integer, content: String }
	OutOfRange = { line: Integer, value: Integer }
}
```

## Parse the input

```fa
-- library/parseScores.fa
export function = (contents: String): Array(Score) throws ScoreError => {
	mutable scores: Array(Score) = []
	let lines = contents.split("\n")

	for line, index in lines {
		if line.trim() == "" {
			continue
		}

		let parts = line.split(",")

		if parts.length != 2 {
			throw ScoreError.MalformedLine { line = index, content = line }
		}

		let value = Integer.fromString(parts[1].trim()) catch error => throw ScoreError.MalformedLine { line = index, content = line }

		if value is not 0..=100 {
			throw ScoreError.OutOfRange { line = index, value }
		}

		scores.push({
			name = parts[0].trim()
			value
		})
	}

	return scores
}
```

A few things worth pointing out.

The signature says everything the function can do: it returns an `Array(Score)`, and it can fail with a `ScoreError`. `throw` exits the function with an error, like `return` exits it with a value.

After `if parts.length != 2 { throw ... }`, the compiler knows that `parts` has exactly two items, so `parts[0]` and `parts[1]` cannot fail. Without the check, indexing would `throws IndexError`, and you would have to handle it.

`Integer.fromString` fails when the text is not a number. `catch` handles that failure by throwing a `ScoreError` instead, so the caller only ever deals with one error type.

After `if value is not 0..=100 { throw ... }`, the compiler has narrowed `value` to `Integer(min = 0, max = 100)`, so it fits the `Score.value` field with no conversion.

`scores.push(...)` receives an object typed by context: the array holds `Score` values, so there is no need to write the type again.

## Compute the statistics

The summary is its own type, in its own file:

```fa
-- library/Summary.fa
export type = {
	count: Integer
	average: Float
	highest: Score
	lowest: Score
}
```

```fa
-- library/summarise.fa
export function = (scores: Array(Score)): Summary? => {
	if scores.length == 0 {
		return none
	}

	mutable total = 0
	mutable highest = scores[0]
	mutable lowest = scores[0]

	for score in scores {
		total += score.value
		if score.value > highest.value {
			highest = score
		}
		if score.value < lowest.value {
			lowest = score
		}
	}

	return {
		count = scores.length
		average = total / scores.length
		highest
		lowest
	}
}
```

After the early return, the compiler knows `scores` is not empty: `scores[0]` cannot fail, and the division cannot throw `DivisionByZero`. `/` always returns a `Float`, which is the type of `average`.

An empty input is an absence, not a failure, so this returns an optional rather than throwing an error. See [errors and optionals](/values/errors).

## Wire up the entry point

The entry point is the function exported by `application/main.fa`. It receives the command-line arguments and returns an exit code.

The application sees the library only through the `library` namespace. Destructuring it with `use` creates short aliases:

```fa
-- application/main.fa
use { parseScores, summarise, ScoreError } = library

export function = (arguments: Array(String)): Integer throws ScoreError => {
	let path = arguments.get(1) ?? "scores.csv"

	let contents = File.read(path) catch error => throw ScoreError.FileNotFound { path }

	let scores = try parseScores(contents)

	let summary = summarise(scores)

	if summary is none {
		console.log("No scores found.")
		return 0
	}

	console.log("Scores:    {summary.count}")
	console.log("Average:   {summary.average.round()}")
	console.log("Highest:   {summary.highest.name} ({summary.highest.value})")
	console.log("Lowest:    {summary.lowest.name} ({summary.lowest.value})")

	return 0
}
```

`try` propagates the errors of `parseScores` to the caller. After `if summary is none { return 0 }`, `summary` is narrowed to `Summary`, so its fields can be read directly.

The entry function returns an `Integer` exit code and `throws ScoreError`. If an error reaches it, Fa prints the error and exits with a non-zero code, so you never write that plumbing yourself.

## Test it

```fa
-- library/parseScores.test.fa
test "parses well-formed input" {
	let scores = parseScores("Ana, 90\nBao, 70") catch error => []

	assert scores == [
		{ name = "Ana", value = 90 }
		{ name = "Bao", value = 70 }
	]
}

test "skips blank lines" {
	let scores = parseScores("Ana, 90\n\nBao, 70") catch error => []

	assert scores.length == 2
}

test "reports a malformed line" {
	let scores = parseScores("Ana") catch error => {
		assert error is ScoreError.MalformedLine
		[]
	}

	assert scores.length == 0
}

test "reports an out-of-range score" {
	let scores = parseScores("Ana, 150") catch error => {
		assert error is ScoreError.OutOfRange
		[]
	}

	assert scores.length == 0
}
```

`==` compares deeply, so a whole array of scores can be checked in one assertion.

```bash
fa test
```

## Run and build

```bash
fa develop               # run with hot reloading
fa build                 # compile for every configured target
fa build --target=native
fa preview
```

The native binary is written to `build/native/scores`.

## What to add next

- **Configuration**: read defaults at compile time with `static` values and `@if` (see [metaprogramming](/advanced/metaprogramming)), so unused branches disappear from the binary.
- **Concurrency**: process several files at once with `for parallel` (see [concurrency](/advanced/concurrency)).
- **Performance**: if the input is large, a [`Columnar`](/collections/columnars) will beat an array of objects for column-wise statistics. Measure first; see [performance](/guides/performance).
