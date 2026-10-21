# Build your application

This guide walks through a small but complete application: a command-line tool that reads a file of scores and reports statistics. It touches the pieces you will use in any Fa program — packages, errors, collections and tests.

## Scaffold

```bash
fa new package scores
cd scores
```

```
scores/
├── package.fab
├── dependencies.fa
├── library/
└── application/
    └── main.fa
```

Set the targets in `package.fab`:

```
name = "scores"
version = "0.1.0"
platforms = ["native", "bun"]
```

## Model the data

Put the logic in `library/`, so it can be tested and reused. Start with the types.

```fa
-- library/Score.fa
export type = {
  name: String
  value: Int(min = 0, max = 100)
}
```

The range on `value` is not documentation: the compiler will reject any code that could put a number outside it into this field.

```fa
-- library/ScoreError.fa
export type = Errors {
  FileNotFound = { path: String }
  MalformedLine = { line: Int, content: String }
  OutOfRange = { line: Int, value: Int }
}
```

## Parse the input

```fa
-- library/parseScores.fa
export function = (contents: String): Array(Score) | ScoreError => {
  mutable scores = Array(Score) {}

  for contents.split("\n") >> line, index {
    if line.trim() is "" {
      continue
    }

    let parts = line.split(",")

    if parts.length != 2 {
      return ScoreError.MalformedLine { line = index, content = line }
    }

    let value = when Int.fromString(parts[1]!.trim()) is {
      Int >> parsed => parsed
      Error => return ScoreError.MalformedLine { line = index, content = line }
    }

    if value < 0 or value > 100 {
      return ScoreError.OutOfRange { line = index, value }
    }

    scores.push(Score {
      name = parts[0]!.trim()
      value
    })
  }

  return scores
}
```

Two things worth pointing out.

`parts[1]!` propagates the index error rather than crashing — the compiler knows `parts` has at least two entries only after the length check, and `!` covers the rest.

After `if value < 0 or value > 100 { return ... }`, the compiler has narrowed `value` to `Int(min = 0, max = 100)`, so it fits the `Score.value` field with no conversion.

## Compute the statistics

```fa
-- library/summarise.fa
export type Summary = {
  count: Int
  average: Decimal
  highest: Score
  lowest: Score
}

export function = (scores: Array(Score)): Summary? => {
  if scores.length == 0 {
    return none
  }

  mutable total = 0
  use highest = scores[0]
  use lowest = scores[0]

  for scores >> score {
    total += score.value
    if score.value > highest.value { highest = score }
    if score.value < lowest.value { lowest = score }
  }

  return Summary {
    count = scores.length
    average = total / scores.length
    highest
    lowest
  }
}
```

An empty input is an absence, not a failure, so this returns an optional rather than an error. See [choosing between errors and optionals](../values/errors#choosing-between-errors-and-optionals).

## Wire up the entry point

```fa
-- application/main.fa
export function = (arguments: Array(String)): Int | ScoreError => {
  let path = arguments[1] ?? "scores.csv"

  let contents = when File.read(path) is {
    String >> text => text
    Error => return ScoreError.FileNotFound { path }
  }

  let scores = parseScores(contents)!

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

The entry function returns `Int | ScoreError`. Fa prints the error and exits non-zero if one comes back, so you never write that plumbing yourself.

## Test it

```fa
-- library/parseScores.test.fa
export namespace = {
  test "parses well-formed input" {
    let scores = parseScores("Ana, 90\nBao, 70")!

    assert(scores.length == 2)
    assert(scores[0]!.name == "Ana")
    assert(scores[0]!.value == 90)
  }

  test "skips blank lines" {
    let scores = parseScores("Ana, 90\n\nBao, 70")!

    assert(scores.length == 2)
  }

  test "reports a malformed line" {
    let result = parseScores("Ana")

    assert(result is ScoreError.MalformedLine)
  }

  test "reports an out-of-range score" {
    let result = parseScores("Ana, 150")

    assert(result is ScoreError.OutOfRange)
  }
}
```

```bash
fa test
```

## Run and build

```bash
fa develop           # run with hot reloading
fa build             # compile for every configured platform
fa build --platform native
fa preview
```

The native binary is written to `build/native/scores`.

## What to add next

- **Configuration** — read defaults at compile time with [metaprogramming](../advanced/metaprogramming), so unused branches disappear from the binary.
- **Concurrency** — process several files at once with [`for parallel`](../advanced/concurrency#parallel-iteration).
- **Performance** — if the input is large, a [`Columnar`](../collections/columnars) will beat an array of objects for column-wise statistics. Measure first; see [performance](./performance).
