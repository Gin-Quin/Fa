# Deploy a library

Fa libraries are shared **unbundled and uncompiled**. There is no publishing step, no registry account, and no build artefact: you push source to a Git repository and tag it.

## What gets published

Everything under `library/`. Your `application/` directory, your tests and your scripts stay private.

```
farm/
├── package.fa        <- required
├── dependencies.fa
└── library/          <- this is the public API
    ├── (animals)/
    │   ├── cows.fa
    │   ├── cows.test.fa
    │   └── pigs.fa
    └── machines/
        ├── tractor.fa
        └── harvester.fa
```

The namespaces consumers see are exactly the ones your own files see. There is no separate export list to maintain. See [modules](/architecture/modules#building-a-library).

## Prepare the package

`package.fa` is a regular Fa file that exports the configuration of the package. A library needs a name and a version, and should describe itself:

```fa
-- package.fa
export = {
	name = "farm"
	version = "4.2.1"
	description = "Utilities to manage a farm"
	license = "MIT"
	repository = "github:farmers/farm"
	targets = ["web", "bun", "native", "wasm"]
}
```

List every target your library supports. A consumer building for a target you have not listed gets a compile error naming your library, rather than a confusing failure deep in their build.

Check that it builds and passes everywhere before tagging:

```bash
fa check
fa test
fa test --target=native
fa test --target=web
```

## Version it

Fa uses semantic versioning, and consumers pin by range:

- `@4`: the latest `4.x.y`
- `@4.2`: the latest `4.2.x`
- `@4.2.1`: exactly that version

Anything reachable from `library/` is public API. Because Fa is [structurally typed](/advanced/structural-typing), changing a field's type, removing a field, or narrowing a parameter is a breaking change even if the names stay the same.

The signature of a function also lists its effects: the errors it `throws`, the values it `requires`, and what it `mutates` or `invalidates`. They are part of the public API too.

| Change | Bump |
| --- | --- |
| Adding a file or a namespace member | minor |
| Adding an optional field or a defaulted parameter | minor |
| Adding a member to a union | **major**: it breaks exhaustive `when` |
| Adding an error to `throws`, or a requirement to `requires` | **major**: callers must handle or provide it |
| Adding a `mutates` or `invalidates` effect | **major**: callers must pass a `mutable` value |
| Removing or renaming anything public | major |
| Changing a type, narrowing a parameter, widening a return | major |
| Fixing behaviour without changing types or effects | patch |

Use `#` to keep something out of the public surface:

```fa
-- library/parser.fa
export namespace = {
	function parse = (input: String): Ast throws ParseError => try #parseInternal(input, 0)

	function #parseInternal = (input: String, depth: Integer): Ast throws ParseError => ...
}
```

## Publish

Tag and push:

```bash
git tag v4.2.1
git push origin main --tags
```

That is the whole publishing process.

## Consume it

In the consuming package, declare the dependency in `dependencies.fa`. Like `package.fa`, it is a regular Fa file that exports an object:

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
}
```

The library is then available as the `farm` namespace in every file of the package. Destructure it with `use` to create short aliases:

```fa
use { cows, pigs } = farm

cows.milk()
```

The key of the dependency is the name of the namespace. To use the library under a different name, pick another key:

```fa
-- dependencies.fa
export = {
	farming = "github:farmers/farm@4"
}
```

The exact versions that were resolved are recorded in `dependencies.lock`, with a hash of their content, so every build uses the same code. Run `fa update` to move to the latest versions allowed by your ranges. See [dependencies](/architecture/dependencies).

## Why source, not binaries

Shipping source means:

- the compiler sees your generic and compile-time code, and can specialise it for the consumer's target,
- one library works for JavaScript, native and WebAssembly with no per-target artefact,
- consumers can read and step through the code they depend on,
- there is nothing to rebuild when a new target appears.

The trade-off is compile time: a consumer compiles your library the first time they build. Fa caches compiled dependencies, so this is a one-off cost per version.

## Continuous integration

A minimal GitHub Actions workflow:

```yaml
name: test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: fa check
      - run: fa test
      - run: fa test --target=native
```

:::caution
An official CI action is not published yet. Until it is, install the compiler as part of the workflow.
:::
