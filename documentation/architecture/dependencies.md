# Dependencies

A Fa package can use code that is not on your machine: libraries hosted on Git, or files published on the web. A typical use case is to load the types of a published API.

## Declaring dependencies

Dependencies are declared in the `dependencies.fa` file, at the root of a package (or of a workspace, to share them between all its packages). Like every [configuration file](/architecture/configuration), it exports an object: each key is the name of a dependency, and each value is its source.

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
	weather = "https://api.weather.com/fa/types.fa"
	ui = "https://cdn.example.com/ui@2.1.0/"
}
```

Each dependency is then available as a namespace in every file of the package:

```fa
let forecast: weather.Forecast = try fetchForecast("Paris")

use { cows, pigs } = farm
```

### Sources

| Source | Example |
|---|---|
| Git repository | `"github:farmers/farm@4"` |
| Remote file | `"https://api.weather.com/fa/types.fa"` |
| Remote folder | `"https://cdn.example.com/ui@2.1.0/"` |

Use versioned sources whenever possible: a version range for Git repositories (`@4`), a version in the URL for remote files. An unversioned URL is allowed, but `fa` emits a warning.

## Inline imports

A remote module can also be imported directly in the code, with the `@import` intrinsic. The source is resolved at compile time, exactly like a source declared in `dependencies.fa`:

```fa
use weather = @import("https://api.weather.com/fa/types.fa")

let forecast: weather.Forecast = try fetchForecast("Paris")
```

`@import` is only used for non-local sources: local files are available through [implicit namespaces](/architecture/modules).

:::tip
Prefer `dependencies.fa` for dependencies used in several files: all the external sources of the package are then visible in one place.
:::

## Lockfile

The first time a dependency is resolved, `fa` writes its exact version and the hash of its content into the `dependencies.lock` file. This file should be committed.

On the next builds, the content of every dependency is checked against its hash. If a remote file changed, the build fails with a clear error, instead of silently changing the behavior of your program:

```
error: the content of `weather` changed since it was locked
  source: https://api.weather.com/fa/types.fa
  help: run `fa update weather` to accept the new version
```

To update dependencies deliberately:

```bash
fa update  # <-- update all dependencies
fa update weather  # <-- update a single dependency
```

## Cache

Resolved dependencies are stored in a local cache, so builds work offline and are reproducible, for example in continuous integration.

## Loading types from an API

Most APIs don't publish Fa files, but describe themselves with a schema (OpenAPI, JSON Schema, GraphQL, ...). Since configuration files are regular Fa code, executed at build time, a library can turn a schema into Fa types:

```fa
-- dependencies.fa
use { openapi } = @import("github:example/fa-openapi@1")

export = {
	stripe = openapi("https://api.stripe.com/openapi.json")
}
```

```fa
let customer: stripe.Customer = try stripe.customers.retrieve("cus_123")
```

The fetched schema is locked and cached like any other source.

## Security

Dependencies are trusted, like in npm or Cargo: a dependency can contain code that runs at compile time (see [metaprogramming](/advanced/metaprogramming)) or at build time (in configuration files). Only add dependencies from sources you trust.
