# Projects

A **project** is a group of related packages, marked by a `project.fab` file and a `packages/` directory.

Projects exist so several applications and libraries can share code without publishing it.

```
myProject/
├── project.fab
├── dependencies.fa
└── packages/
    ├── shared/
    │   ├── package.fab
    │   └── library/
    │       └── models.fa
    ├── api/
    │   ├── package.fab
    │   ├── library/
    │   └── application/
    │       └── main.fa
    └── web/
        ├── package.fab
        ├── library/
        └── application/
            └── main.fa
```

Scaffold one with:

```bash
fa new project
```

## Nested packages

`packages/` is scanned recursively for directories containing a `package.fab`, so you can group packages in subdirectories:

```
myProject/
├── project.fab
└── packages/
    ├── shared/
    │   └── package.fab
    └── applications/
        ├── api/
        │   └── package.fab
        └── web/
            └── package.fab
```

The intermediate `applications/` directory is organisational only: it does not affect package names or how they refer to each other.

## Depending on a sibling package

A package in a project depends on another the same way it depends on any library, but with a path instead of a repository:

```fa
-- packages/api/dependencies.fa
use shared = "../shared"
```

Sibling packages are always built from source, so a change in `shared` is picked up immediately by `fa develop`, with no publishing step and no version bump.

## Shared dependencies

A `dependencies.fa` at the project root applies to every package:

```fa
-- dependencies.fa
use farm = "github:farmers/farm@4"
```

A package can still declare its own, and can pin a different version if it needs to.

## Commands

Run from the project root, commands apply to every package that implements them:

```bash
fa develop   # development server for every package
fa test      # tests for every package
fa build     # compile every application package
fa check     # type-check everything
```

Name a package to target it:

```bash
fa develop api
fa test shared
fa build web
fa preview web
```

## Project configuration

`project.fab` holds the settings shared by every package. A package's own `package.fab` overrides them:

```
name = "myProject"
version = "1.0.0"
platforms = ["web", "native"]
```

See [configuration](./configuration).
