# Configuration

A package is configured by `package.fab`, and a project by `project.fab`. Both use the same format and the same fields.

A `.fab` file is written in Fa's object syntax, without the outer braces:

```
name = "farm"
version = "4.2.1"
description = "Utilities to manage a farm"
platforms = ["web", "native"]
```

Because it is Fa, comments, nested objects and arrays all work as they do everywhere else:

```
name = "farm"

-- the platforms this package is built for
platforms = ["web", "native"]

build = {
  optimize = true
  sourceMaps = false
}
```

## Fields

### `name`

The package name. Required. Used as the namespace when the package is consumed as a library.

```
name = "farm"
```

### `version`

A semantic version. Required for a published library.

```
version = "4.2.1"
```

### `platforms`

The targets to build for. Any of `web`, `bun`, `native` and `wasm`. Defaults to all four.

```
platforms = ["web", "native"]
```

| Platform | Output | Used for |
| --- | --- | --- |
| `web` | JavaScript for the browser | web applications |
| `bun` | JavaScript for the Bun runtime | development, servers, scripts |
| `native` | a native binary | command-line tools, servers, games |
| `wasm` | a WebAssembly module | the browser, edge runtimes, plugins |

:::tip
Some features exist only on some platforms — `for parallel`, for instance, is native-only. Code using a feature the selected platform does not support fails to compile rather than degrading silently. See [concurrency](../advanced/concurrency#what-each-target-supports).
:::

### `entry`

The application's entry file. Defaults to `application/main.fa`.

```
entry = "application/server.fa"
```

### `description`, `license`, `repository`, `authors`

Metadata for a published library.

```
description = "Utilities to manage a farm"
license = "MIT"
repository = "github:farmers/farm"
authors = ["Ada Lovelace"]
```

### `build`

Build options. Every field is optional.

```
build = {
  optimize = true       -- optimise for speed; defaults to true for `fa build`
  sourceMaps = false    -- emit source maps
  output = "dist"       -- output directory, defaults to "build"
  minify = true         -- JavaScript and WebAssembly targets only
}
```

### `develop`

Options for `fa develop`.

```
develop = {
  port = 3000
  open = true      -- open a browser on start
  hotReload = true
}
```

### `test`

Options for `fa test`.

```
test = {
  include = ["library/**/*.test.fa"]
  timeout = 5000
}
```

## Per-platform overrides

A `platform` object overrides fields for one target:

```
platforms = ["web", "native"]

platform = {
  web = {
    build = { minify = true }
  }
  native = {
    build = { optimize = true }
  }
}
```

## Project-level configuration

`project.fab` accepts the same fields. They apply to every package, and a package's own `package.fab` overrides them field by field:

```
-- project.fab
name = "myProject"
version = "1.0.0"
platforms = ["web", "native"]
```

```
-- packages/tool/package.fab
name = "tool"
platforms = ["native"] -- this package is native-only
```

## Configuration at compile time

The active configuration is available while compiling, so you can branch on it and let dead code be removed:

```fa
@if @configuration.build.optimize {
  useFastPath()
} @else {
  useDebugPath()
}
```

See [metaprogramming](../advanced/metaprogramming).
