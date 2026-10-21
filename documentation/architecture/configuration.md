# Configuration

Packages and workspaces are configured with Fa files: `package.fa` and `workspace.fa`.

Configuration files use a **subset of Fa**, restricted to static data. There is no separate configuration language to learn, and since the type of the configuration is known, the compiler and the editor check it: a typo in a field name or an invalid value is an error, with autocompletion.

## Syntax

Like every Fa file, a configuration file [exports](/architecture/modules#exporting-code) exactly one value: here, an object literal.

```fa
-- package.fa
export = {
	name = "farm"
	version = "2.1.0"
	targets = ["web", "bun", "native"]
}
```

A configuration file can only contain:

- `let` declarations of static values (strings, numbers, booleans, arrays, objects),
- imports of other configuration files with `@import`,
- comments,
- one `export`.

Functions, mutable values and side effects are not allowed.

## Variables

Use `let` to avoid repeating values:

```fa
let version = "2.1.0"

export = {
	name = "farm"
	version = version
	description = "Farm utilities, version {version}"
}
```

## Importing other configuration files

Use `@import` with [destructuring](/operations/destructuring) to reuse values from another configuration file:

```fa
-- shared.fa
export = {
	defaultTargets = ["web", "native"]
	license = "MIT"
}
```

```fa
-- packages/farm/package.fa
use { defaultTargets, license } = @import("../../shared.fa")

export = {
	name = "farm"
	license = license
	targets = [...defaultTargets, "wasm"]
}
```

## Package configuration

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Name of the package. |
| `version` | `String` | Version of the package. |
| `targets` | `Array(Target)` | Targets the application is compiled for. See [targets](/architecture/packages#targets). |

## Workspace configuration

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Name of the workspace. |
