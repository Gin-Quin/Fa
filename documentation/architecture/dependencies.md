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

Each dependency is then available as a namespace in every file of the package, in `library/`, in `application/`, in `scripts/` and in the tests:

```fa
use { cows, pigs } = farm

function loadForecast = (): weather.Forecast throws HttpError => {
	try fetchForecast("Paris")
}
```

The key is the namespace under which the dependency is available. It must be a valid identifier, and must not be the name of a root entry of the package (see [name lookup](/architecture/modules#name-lookup)). You can name a dependency as you like, for example to use two major versions side by side:

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
	legacyFarm = "github:farmers/farm@3"
}
```

Only the dependencies declared by a package are visible in it: the dependencies of a dependency are not.

### Sources

| Source | Example |
|---|---|
| Git repository | `"github:farmers/farm@4"`, `"gitlab:farmers/farm@4"`, `"git+https://example.com/farm.git@4"` |
| Local package (workspace sibling or external folder) | `"../shared"` |
| Remote file | `"https://api.weather.com/fa/types.fa"` |
| Remote folder | `"https://cdn.example.com/ui@2.1.0/"` |

Use versioned sources whenever possible: a version range for Git repositories (`@4`), a version in the URL for remote files. An unversioned URL is allowed, but `fa` emits a warning.

A source is written as a string, or as a `DependencySource` object that generates the Fa code of the dependency from a fetched file (see [loading types from an API](#loading-types-from-an-api)):

```fa
type DependencySource = String | {
	source: String -- fetched and locked like any other source
	generate: (content: String) => String -- turns the fetched content into the Fa source of the dependency
}
```

`file://`, local paths and local Git repositories (`git+file://…`, `git+/srv/x.git`) are allowed only from the root package, workspace packages and their explicitly named local dependencies. A fetched dependency declaring or importing one is `DEP008`. An `http://` source gets `DEP004` because its first content is unauthenticated. Only the URL path identifies a version (`/v3/`, `@2.1.0`, `-1.2`), never its host.

### Git repositories

A Git repository is a Fa package: its `library/` directory is what the dependency exports, its `dependencies.fa` declares its own dependencies, and its `application/`, `scripts/` and test files are ignored.

The versions of a repository are its Git tags that are semantic versions, written `4.2.1` or `v4.2.1`. The `version` field of the `package.fa` of the tagged commit must be the same version, otherwise the version is rejected with an error.

### Remote files and folders

A remote file is a module on its own: it can't see other files, and uses other remote code only with `@import`.

A remote folder must serve a `manifest.fa` file at its root, which exports the paths of its `.fa` files, relative to the folder:

```fa
-- https://cdn.example.com/ui@2.1.0/manifest.fa
export = ["button.fa", "card.fa", "layout/grid.fa"]
```

The folder is then a root of its own, with the usual [module rules](/architecture/modules): `ui.button`, `ui.layout.grid`. It has no dependencies, and uses other remote code only with `@import`.

### Version ranges

Versions of Git repositories follow [semantic versioning](https://semver.org) (`major.minor.patch`). The version after `@` is a range:

| Range | Resolves to |
|---|---|
| `@4` | the latest `4.x.y` version |
| `@4.2` | the latest `4.2.x` version |
| `@4.2.1` | exactly the version `4.2.1` |
| `@0` | the latest `0.x.y` version: `0` follows the same rule as any other major version |

A pre-release version (`5.0.0-beta.1`) is only chosen when the range names it exactly. The version that was actually resolved is recorded in the [lockfile](#lockfile).

### Resolving versions

A library can be required by several packages of the workspace and by several dependencies, with different ranges. A library is identified by its repository, never by the key that names it, and `fa` resolves it once per major version:

- for each repository and each major version, `fa` chooses the latest version that satisfies every range that asks for this major version;
- if no version satisfies them all, for example `@4.2.1` and `@4.3`, resolution fails with an error that names the packages asking for each range;
- different major versions coexist: `farm@3` and `farm@4` are two distinct libraries in the same program. A value of one can be used where the other expects a type of the same shape, by [structural typing](/advanced/structural-typing).

### Sibling packages

In a [workspace](/architecture/workspaces), a package depends on another package of the same workspace with a relative path instead of a repository:

```fa
-- packages/api/dependencies.fa
export = {
	shared = "../shared"
}
```

The path goes from the folder of the package to the folder of the other package. Sibling packages are always built from source: a change in `shared` is picked up immediately by `fa develop`, with no publishing step and no version bump. They are not recorded in the lockfile.

Outside a workspace, a package can depend on a local package folder, including a relative path such as `shared = "../shared"`. Unlike workspace siblings, it is locked by the hash of `package.fa`, `dependencies.fa` and `library/`; changed contents require `fa update shared`.

### Shared dependencies

A `dependencies.fa` file at the root of a workspace applies to every package of the workspace. A package can still declare its own dependencies: a key of the package replaces the same key of the workspace, so a package can use a different version of a shared dependency.

There are no test-only dependencies: the dependencies of a package are available to its tests, and a dependency used only by the tests is never compiled into the application.

## Inline imports

A remote module can also be imported directly in the code, with the `@import` intrinsic. The source is resolved at compile time, exactly like a source declared in `dependencies.fa`, and locked in the same lockfile:

```fa
use weather = @import("https://api.weather.com/fa/types.fa")

function loadForecast = (): weather.Forecast throws HttpError => {
	try fetchForecast("Paris")
}
```

The argument of `@import` is a string literal, written like a source of `dependencies.fa`: a Git repository, a remote file or a remote folder. `@import` is only used for non-local sources: local files are available through [implicit namespaces](/architecture/modules), and sibling packages through `dependencies.fa`.

:::tip
Prefer `dependencies.fa` for dependencies used in several files: all the external sources of the package are then visible in one place.
:::

## Lockfile

The first time a dependency is resolved, `fa` writes its exact version and the hash of its content into the `dependencies.lock` file. This file should be committed.

There is one lockfile per workspace, at its root, or per package outside a workspace, or per standalone file, next to it. It records every remote source of the program, including the dependencies of the dependencies and the sources of `@import`. Its content is Fa syntax, generated by `fa` and sorted, so that its diffs are easy to review:

```fa
-- dependencies.lock: generated by `fa`, do not edit
export = [
	{
		source = "github:farmers/farm"
		version = "4.2.1"
		commit = "9f2c1e7a4b0d3c8e6f5a2b1c0d9e8f7a6b5c4d3e"
		hash = "sha256:3a7bd3e2360a3d29eea436fcfb7e44c735d117c42d1c1835420b6b9942dd4f1b"
	}
	{
		source = "https://api.weather.com/fa/types.fa"
		hash = "sha256:ef2d127de37b942baad06145e54b0c619a1f22327b2ebbcfbec78f5564afe39d"
	}
]
```

The hash is the SHA-256 of the content: the bytes of a remote file, the files listed by the manifest of a remote folder, or the files of a Git repository that the dependency uses (`package.fa`, `dependencies.fa` and `library/`). Several files are hashed together, sorted by path, each one as its path, a zero byte, its length in bytes, a zero byte, then its bytes.

On the next builds, the content of every dependency is checked against its hash. If a remote file changed, the build fails with a clear error, instead of silently changing the behavior of your program:

```
error: the content of `weather` changed since it was locked
  source: https://api.weather.com/fa/types.fa
  help: run `fa update weather` to accept the new version
```

A source added to `dependencies.fa` or to an `@import` is resolved and added to the lockfile by the next command that needs it. A locked version is never changed implicitly; to update dependencies deliberately:

```bash
fa update  # <-- resolve every range again, to the latest allowed versions
fa update weather  # <-- only the dependency named `weather`
```

A hash is `sha256:` followed by 64 lowercase hexadecimal digits; a Git commit has 40 or 64 hexadecimal digits. Other forms make the lockfile invalid. Remote manifests and Git trees use relative `/` paths without empty segments, `.`, `..`, backslashes, colons or control characters (`DEP007`).

## Cache

Resolved dependencies are stored in a local cache, so builds work offline and are reproducible, for example in continuous integration. A build whose dependencies are all locked and cached needs no network.

The cache is shared by all the projects of the user. It lives in `~/.cache/fa` on Linux, `~/Library/Caches/fa` on macOS and `%LOCALAPPDATA%\fa` on Windows, or in the folder named by the `FA_CACHE` environment variable. Its entries are addressed by their hash, so it can be deleted at any time.

## Loading types from an API

Most APIs don't publish Fa files, but describe themselves with a schema (OpenAPI, JSON Schema, GraphQL, ...). A library can turn a schema into Fa types, with a `DependencySource` that has a `generate` function:

```fa
-- dependencies.fa
use { openapi } = @import("github:example/fa-openapi@1")

export = {
	stripe = openapi("https://api.stripe.com/openapi.json")
}
```

Here, `openapi` returns `{ source = url, generate = ... }`. `fa` fetches the schema and locks it like any other source, then runs `generate` on its content, with the compile-time interpreter: the Fa source it returns becomes the module `stripe`. `generate` must be pure, so the generated code only changes when the locked schema changes.

```fa
function loadCustomer = (): stripe.Customer throws stripe.ApiError => {
	try stripe.customers.retrieve("cus_123")
}
```

## Security

Dependencies are trusted, like in npm or Cargo: a dependency can contain code that runs at compile time (see [metaprogramming](/advanced/metaprogramming)) or at build time (in configuration files). Only add dependencies from sources you trust.
