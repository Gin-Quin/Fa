# Workspaces

A **workspace** is a collection of [packages](/architecture/packages) that are related to each other. Workspaces exist so that several applications and libraries can share code without publishing it.

A workspace is a directory that contains a `workspace.fa` file and a `packages/` directory.

This is a typical workspace file structure:

```
myWorkspace/
├── workspace.fa
├── dependencies.fa
└── packages/
    ├── shared/
    │   ├── package.fa
    │   └── library/
    │       └── models.fa
    ├── api/
    │   ├── package.fa
    │   ├── dependencies.fa
    │   ├── library/
    │   └── application/
    │       └── main.fa
    └── web/
        ├── package.fa
        ├── dependencies.fa
        ├── library/
        └── application/
            └── main.fa
```

You can scaffold a workspace with the following command:

```bash
fa new workspace
```

## Nested packages

The `packages/` directory is scanned recursively to find all the folders that contain a `package.fa` file, so packages can be grouped in subdirectories:

```
myWorkspace/
├── workspace.fa
└── packages/
    ├── shared/
    │   └── package.fa
    └── applications/
        ├── api/
        │   └── package.fa
        └── web/
            └── package.fa
```

The intermediate `applications/` directory is organisational only: it doesn't affect the names of the packages, which come from the `name` field of their `package.fa` and must be unique in the workspace. The scan doesn't enter the folder of a package: a package can't contain another package.

## Depending on a sibling package

A package of the workspace depends on another one the same way it depends on any library, but with a relative path instead of a repository:

```fa
-- packages/api/dependencies.fa
export = {
	shared = "../shared"
}
```

The path goes from the folder of the package to the folder of the other package, so it follows the intermediate directories: from `packages/applications/api`, the package `shared` of the example above is `"../../shared"`.

```fa
-- packages/api/application/main.fa
export function = () => {
	let user: shared.models.User = { name = "Ada" }
	console.log("Hello {user.name}")
}
```

Sibling packages are always built from source: a change in `shared` is picked up immediately by `fa develop`, with no publishing step and no version bump.

## Shared dependencies

A `dependencies.fa` file at the root of the workspace applies to every package:

```fa
-- myWorkspace/dependencies.fa
export = {
	farm = "github:farmers/farm@4"
}
```

A package can still declare its own dependencies: a key of the package replaces the same key of the workspace, so a package can use a different version of a shared dependency. See [dependencies](/architecture/dependencies).

The whole workspace has one lockfile, `dependencies.lock`, at its root.

## Workspace configuration

`workspace.fa` holds the settings shared by every package. They are defaults: each package overrides them field by field in its own `package.fa`.

```fa
-- myWorkspace/workspace.fa
export = {
	name = "myWorkspace"
	version = "1.0.0"
	targets = ["web", "native"]
}
```

The other `.fa` files at the root of the workspace are available to the configuration files of the packages through the `workspace` namespace. See [configuration](/architecture/configuration#sharing-values-between-configuration-files).

## Commands

Commands run at the root of a workspace are executed for every package that implements them: `build`, `develop` and `preview` for the packages with an application, `test` for the packages with test files, `check` and `format` for every package:

```bash
fa develop  # <-- development server for every package
fa test  # <-- tests of every package
fa check  # <-- type-check everything
fa build  # <-- compile every application
```

Name a package to target only that one:

```bash
fa develop api
fa test shared
fa build web
fa preview web
```

See the [overview](/architecture/overview#workspaces).
