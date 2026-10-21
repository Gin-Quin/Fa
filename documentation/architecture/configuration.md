# Configuration

Packages and workspaces are configured with Fa files: `package.fa`, `workspace.fa` and `dependencies.fa`.

Configuration files are **regular Fa files**. There is no separate configuration language to learn: you can use values, functions and libraries. Since the type of the configuration is known, the compiler and the editor check it: a typo in a field name or an invalid value is an error, with autocompletion.

## How configuration files are executed

Configuration files are compiled to JavaScript and executed at build time (just-in-time). The value they export is the configuration used by `fa`.

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

Values and functions can be used to build the configuration:

```fa
let version = "2.1.0"

function withServerTargets = (targets: Array(Target)) => [...targets, "bun", "node"]

export = {
	name = "farm"
	version = version
	description = "Farm utilities, version {version}"
	targets = withServerTargets(["web"])
}
```

## Sharing values between configuration files

Configuration files follow the same rules as code: sibling files are available implicitly through their names, and the files at the root of the workspace are available through the `workspace` namespace.

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
| `name` | `String` | Name of the package. Required. |
| `version` | `String` | [Semantic version](https://semver.org) of the package. Required for a published library. |
| `targets` | `Array(Target)` | Targets the application is compiled for. Defaults to `["web", "bun", "native"]`. |
| `entry` | `String` | Entry file of the application. Defaults to `"application/main.fa"`. |
| `description` | `String` | Short description of a published library. |
| `license` | `String` | License of a published library. |
| `repository` | `String` | Where the source code of a published library lives. |
| `authors` | `Array(String)` | Authors of a published library. |
| `build` | object | Options of `fa build`. |
| `develop` | object | Options of `fa develop`. |
| `test` | object | Options of `fa test`. |
| `overrides` | object | Per-target overrides. |

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
		sourceMaps = false -- emit source maps
		output = "build" -- output directory, defaults to "build"
		minify = true -- JavaScript and WebAssembly targets only
	}
}
```

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
		include = ["library/**/*.test.fa"]
		timeout = 5000 -- in milliseconds, for each test
	}
}
```

## Per-target overrides

The `overrides` object overrides fields for one target. Its keys are target names:

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

## Configuration at compile time

The target being compiled is available at compile time as the static value `target`, and the active configuration as the static value `configuration`. Use them with `@if` to branch, and let the compiler remove the dead code:

```fa
@if target is "native" {
	useFileStorage()
} @else {
	useBrowserStorage()
}

@if configuration.build.optimize {
	useFastPath()
} @else {
	useDebugPath()
}
```

See [metaprogramming](/advanced/metaprogramming).

## Dependencies

The `dependencies.fa` file declares the dependencies of a package or a workspace. See [dependencies](/architecture/dependencies).
