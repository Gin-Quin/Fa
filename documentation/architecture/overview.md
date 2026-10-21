# Architecture overview

Fa code can be organised at three levels:

- a **standalone file**, run directly,
- a **package**, a library or an application,
- a **project**, a group of related packages.

Each level is a superset of the one before it. Start with a file, and grow into a package when the file gets too big.

| Level | Marker file | Contains |
| --- | --- | --- |
| Standalone file | none | one `.fa` file |
| Package | `package.fab` | `library/`, `application/`, `scripts/` |
| Project | `project.fab` | `packages/` |

## Standalone files

A standalone file is a single `.fa` file you can run directly.

```bash
fa run myFile.fa                    # run it
fa build myFile.fa --target native  # compile it
```

In a standalone file there is no implicit module system, so you state what you need:

```fa
use mySiblingFile = "./mySiblingFile.fa"
use myParentFile = "../myParentFile.fa"
```

Standalone files are good for one-off scripts, small programs, experiments and learning.

## Packages

A **package** is a directory containing a `package.fab` file, and optionally:

- a `dependencies.fa` file,
- a `library/` directory,
- an `application/` directory,
- a `scripts/` directory.

A package is a library, an application, or both. Inside a package, files see each other automatically — see [modules](./modules).

```bash
fa new package   # scaffold one
fa develop       # start a development server with hot reloading
fa test          # run the tests
fa build         # compile it (applications only)
fa preview       # run the compiled application
```

See [packages](./packages).

## Projects

A **project** is a directory containing a `project.fab` file and a `packages/` directory. Projects exist so several closely related packages can share code without publishing it.

```bash
fa new project
```

The same commands work at project level, and apply to every package that implements them:

```bash
fa develop   # every package
fa test      # every package
fa build     # every application package
```

Add a package name to target one of them:

```bash
fa develop myPackage
fa test myPackage
fa build myPackage
fa preview myPackage
```

See [projects](./projects).

## Which level do I need?

- Writing a script, or trying an idea? A **standalone file**.
- Writing something you will publish, test, or come back to? A **package**.
- Building several applications and libraries that share code? A **project**.
