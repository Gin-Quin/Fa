# Configuration

Packages and workspaces are configured with Fa files: `package.fa`, `workspace.fa` and `dependencies.fa`.

Configuration files are **regular Fa files**. There is no separate configuration language to learn: you can use values, functions and libraries. Since the type of the configuration is known, the compiler and the editor check it: a typo in a field name or an invalid value is an error, with autocompletion.

## How configuration files are executed

Configuration files are executed before the package is compiled, by the same interpreter that runs [compile-time code](/advanced/metaprogramming). The value they export is the configuration used by `fa`.

Like compile-time code, they are deterministic: they can't read the clock, random numbers or the environment variables, and they can't reach the network. They can read the files of the package or of the workspace with `@readFile`, and these files become inputs of the build. The only remote code they can use comes from `@import`, which goes through the [lockfile](/architecture/dependencies#lockfile), like every dependency.

They are evaluated in this order:

1. `dependencies.fa` of the workspace, then of the package: they can only use `@import`, sibling configuration files and the `workspace` namespace, never the dependencies they declare;
2. `workspace.fa`, then `package.fa`: they can use `@import` too, but not the dependencies of the package.

## Syntax

Like every Fa file, a configuration file [exports](/architecture/modules#exporting-code) exactly one value: here, an object.

```fa
-- package.fa
export = {
	name = "farm"
	version = "2.1.0"
	targets = ["web", "bun", "native"]
}
```

The export of a configuration file is typed by the file it is in, with no annotation to write:

| File | Type of its export |
| --- | --- |
| `package.fa` | `PackageConfiguration` |
| `workspace.fa` | `WorkspaceConfiguration` |
| `dependencies.fa` | an object with one field per dependency: the name of the field is the name of the dependency, and its value a `DependencySource` |

These types, and `Target`, are available in every configuration file:

```fa
type Target = "web" | "bun" | "node" | "deno" | "native" | "wasm" | "rust"

type PackageConfiguration = {
	name: String
	version: String?
	targets: Array(Target) = ["web", "bun", "native"]
	entry: String = "application/main.fa"

	description: String?
	license: String?
	repository: String?
	authors: Array(String) = []

	build: BuildOptions = {}
	develop: DevelopOptions = {}
	test: TestOptions = {}
	overrides: TargetOverrides = {}
}

type BuildOptions = {
	optimize: Boolean = true
	overflowChecks: Boolean? -- default: checked in development, unchecked in `fa build`
	sourceMaps: Boolean = false
	output: String = "build"
	minify: Boolean = true
	link: Array(String) = []
	libraryPaths: Array(String) = []
	cLibrary: CLibraryOptions?
}

type CLibraryOptions = {
	header: String
	prefix: String
	api: String
}

type DevelopOptions = {
	port: Integer(min = 1, max = 65535) = 3000
	open: Boolean = false
	hotReload: Boolean = true
}

type TestOptions = {
	include: Array(String) = ["**/*.test.fa"]
	timeout: Integer(min = 1) = 5000
}

type TargetOverride = {
	entry: String?
	build: BuildOptions?
	develop: DevelopOptions?
	test: TestOptions?
}

type TargetOverrides = {
	web: TargetOverride?
	bun: TargetOverride?
	node: TargetOverride?
	deno: TargetOverride?
	native: TargetOverride?
	wasm: TargetOverride?
	rust: TargetOverride?
}
```

`WorkspaceConfiguration` has the same fields as `PackageConfiguration`, all optional. `DependencySource` is described in [dependencies](/architecture/dependencies#sources).

Values and functions can be used to build the configuration:

```fa
let version = "2.1.0"

function withServerTargets = (targets: Array(Target)): Array(Target) => [...targets, "bun", "node"]

export = {
	name = "farm"
	version = version
	description = "Farm utilities, version {version}"
	targets = withServerTargets(["web"])
}
```

## Sharing values between configuration files

Configuration files follow the same rules as code: the other `.fa` files next to them are available implicitly through their names, and the `.fa` files at the root of the workspace are available through the `workspace` namespace. The configuration files themselves (`package.fa`, `workspace.fa`, `dependencies.fa`) are never names, and the folders next to them (`library/`, `application/`, `scripts/`, `packages/`) are not visible to configuration files.

```fa
-- myWorkspace/shared.fa
export = {
	defaultTargets = ["web", "native"]
	license = "MIT"
}
```

```fa
-- myWorkspace/packages/farm/package.fa
use { defaultTargets, license } = workspace.shared

export = {
	name = "farm"
	license = license
	targets = [...defaultTargets, "wasm"]
}
```

## Package configuration

All the fields of a package configuration, at a glance:

```fa
-- package.fa
export = {
	name = "farm"
	version = "4.2.1"
	targets = ["web", "native"]
	entry = "application/main.fa"

	description = "Utilities to manage a farm"
	license = "MIT"
	repository = "github:farmers/farm"
	authors = ["Ada Lovelace"]

	build = {
		optimize = true
		sourceMaps = false
		output = "build"
		minify = true
	}

	develop = {
		port = 3000
		open = true
		hotReload = true
	}

	test = {
		include = ["library/**/*.test.fa"]
		timeout = 5000
	}

	overrides = {
		web = {
			build = { minify = true }
		}
	}
}
```

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Name of the package, unique in its workspace. Required. |
| `version` | `String` | [Semantic version](https://semver.org) of the package. Required for a published library. |
| `targets` | `Array(Target)` | Targets the application is compiled for. Defaults to `["web", "bun", "native"]`. |
| `entry` | `String` | Entry file of the application. Defaults to `"application/main.fa"`. |
| `description` | `String` | Short description of a published library. |
| `license` | `String` | License of a published library. |
| `repository` | `String` | Where the source code of a published library lives. |
| `authors` | `Array(String)` | Authors of a published library. |
| `build` | `BuildOptions` | Options of `fa build`. |
| `develop` | `DevelopOptions` | Options of `fa develop`. |
| `test` | `TestOptions` | Options of `fa test`. |
| `overrides` | `TargetOverrides` | Per-target overrides. |

Every field of `build`, `develop`, `test` and `overrides` is optional.

### `targets`

The targets to build for: any of `web`, `bun`, `node`, `deno`, `native`, `wasm` and `rust`. See [targets](/architecture/packages#targets) for what each one produces.

```fa
export = {
	name = "farm"
	targets = ["web", "native"]
}
```

When `targets` is not set, the package is built for `web`, `bun` and `native`.

:::tip
Some features exist only on some targets. Code using a feature that a selected target does not support fails to compile, rather than degrading silently. See [concurrency](/advanced/concurrency).
:::

### `entry`

The entry file of the application, which exports the function called when the program starts (see [modules](/architecture/modules#the-entry-point)). Defaults to `application/main.fa`.

```fa
export = {
	name = "server"
	entry = "application/server.fa"
}
```

### `build`

Options of `fa build`:

```fa
export = {
	name = "farm"
	build = {
		optimize = true -- optimize for speed; defaults to true for `fa build`
		overflowChecks = true -- keep the overflow checks in `fa build` too (default: checked in development only)
		sourceMaps = false -- emit source maps
		output = "build" -- output directory, defaults to "build"; each target writes into `<output>/<target>/`
		minify = true -- JavaScript and WebAssembly targets only
		link = ["sqlite3"] -- C libraries to link, for the foreign files
		libraryPaths = [] -- extra folders where the C libraries are searched
	}
}
```

`overflowChecks` says whether possible integer overflows are checked at runtime in the code of the package (not of its dependencies). By default, like Rust, they are checked in development (`fa run`, `fa test`, `fa develop`), where an overflow stops the program, and not in a release build (`fa build`), where it wraps around on `native` and `wasm` and loses precision beyond 2^53 on the JavaScript targets. Set it to force one behaviour everywhere. Everything else is always checked: a certain overflow is a compile error, and divisions by zero, indexes and conversions keep their checks. `fa build`, `fa run` and `fa test` take the same option as `--overflow-checks=true|false`, which is how a standalone file sets it. Prefer `checkedAdd` and the other explicit variants when an overflow must be handled in every build (see [overflow](/values/numbers#overflow)).

`cLibrary` builds the package as a C library, with a generated header: `header` is the header's file name, `prefix` the prefix of the C names, and `api` the foreign file whose export is the C interface. See [foreign code](/advanced/foreign-code#exposing-a-library-to-c).

### `develop`

Options of `fa develop` (see [hot reloading](/architecture/hot-reloading)):

```fa
export = {
	name = "farm"
	develop = {
		port = 3000
		open = true -- open a browser on start
		hotReload = true -- reload the changed modules instead of restarting
	}
}
```

### `test`

Options of `fa test` (see [testing](/architecture/testing)):

```fa
export = {
	name = "farm"
	test = {
		include = ["library/**/*.test.fa"] -- defaults to every test file of the package
		timeout = 5000 -- in milliseconds, for each test
	}
}
```

## Per-target overrides

The `overrides` object overrides fields for one target. Its keys are target names, and it can override `entry`, `build`, `develop` and `test`:

```fa
export = {
	name = "farm"
	targets = ["web", "native"]

	overrides = {
		web = {
			build = { minify = true }
		}
		native = {
			build = { optimize = true }
		}
	}
}
```

## Workspace configuration

`workspace.fa` accepts the same fields as `package.fa`. They are the **defaults** of every package of the workspace, and each package overrides them field by field in its own `package.fa`:

```fa
-- workspace.fa
export = {
	name = "myWorkspace"
	version = "1.0.0"
	license = "MIT"
	targets = ["web", "native"]
}
```

```fa
-- packages/tool/package.fa
export = {
	name = "tool"
	targets = ["native"] -- this package is native-only
}
```

Here, the `tool` package is version `1.0.0`, licensed under MIT, and built only for `native`.

Nested objects are merged the same way: a package that sets `build = { minify = false }` keeps the other `build` options of the workspace.

## How the configuration is merged

The configuration used to compile a package for a target is built from the fields **written** in each file, in this order, each step overriding the previous one:

1. `workspace.fa`;
2. `package.fa`;
3. the entry of `overrides` for the target being compiled, from `workspace.fa` then from `package.fa`;
4. the defaults of `PackageConfiguration`, for the fields that are still missing.

Objects are merged field by field, at every depth. Any other value, including an array, replaces the previous one: a package that sets `targets = ["native"]` builds only for `native`.

## Configuration at compile time

The target being compiled is available at compile time as the static value `target`, and the configuration merged for that target as the static value `configuration`, of type `PackageConfiguration`. Use them with `@if` to branch, and let the compiler remove the dead code:

```fa
function setUpStorage = () => {
	@if target is "native" {
		useFileStorage()
	} @else {
		useBrowserStorage()
	}
}

function run = () => {
	@if configuration.build.optimize {
		useFastPath()
	} @else {
		useDebugPath()
	}
}
```

See [metaprogramming](/advanced/metaprogramming).

## Dependencies

The `dependencies.fa` file declares the dependencies of a package or a workspace. See [dependencies](/architecture/dependencies).

:::implementation
## Implementation details

- The interpreter keeps the omitted fields absent in a value typed by a configuration type (`PackageConfiguration`, `WorkspaceConfiguration` and their nested types), instead of filling in their defaults at construction: the merge sees only the fields written, and the defaults are applied after it.
- A configuration file exporting a literal (strings, numbers, booleans, arrays, objects, and `let` literals of the same file) is read without running it; any other export is checked like a program file, then evaluated on the compile-time interpreter, and its value checked against the configuration type.
:::
