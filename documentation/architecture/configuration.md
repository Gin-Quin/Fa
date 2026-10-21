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

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Name of the package. |
| `version` | `String` | Version of the package. |
| `targets` | `Array(Target)` | Targets the application is compiled for. See [targets](/architecture/packages#targets). |

## Workspace configuration

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Name of the workspace. |

## Dependencies

The `dependencies.fa` file declares the dependencies of a package or a workspace. See [dependencies](/architecture/dependencies).
