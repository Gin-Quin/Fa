# Packages

A **package** is a directory of related files, marked by a `package.fab` file.

A package can be a library, an application, or both.

```
myPackage/
├── package.fab
├── dependencies.fa
├── library/
│   ├── math.fa
│   └── ...
└── application/
    ├── main.fa
    └── ...
```

Scaffold one with:

```bash
fa new package
```

## What each part is for

| Path | Purpose |
| --- | --- |
| `package.fab` | the package's [configuration](./configuration) |
| `dependencies.fa` | the libraries this package depends on |
| `library/` | code, exported to the outside world if the package is published |
| `application/` | the runnable program; `application/main.fa` is its entry point |
| `scripts/` | one-off scripts, run with `fa run` |

:::tip
A library package can leave out `application/`.

An application package should still have `library/` — it is the local library the application is built from, and it is what the tests exercise.
:::

## Libraries and applications

The difference is what a package produces.

A **library package** exposes everything under `library/`. It is published as source, uncompiled, and consumed by other packages.

An **application package** is compiled into a program for the platforms listed in its configuration. Its `application/` directory is private: nothing there can be imported by another package.

A package that has both is a common and useful shape — the library holds the logic, and the application is a thin entry point around it.

## Tests

Test files sit next to the code they test, named `*.test.fa`:

```
library/
├── math.fa
└── math.test.fa
```

They are excluded from builds. See [testing](./testing).

## Commands

Run these from anywhere inside the package:

```bash
fa develop   # development server with hot reloading
fa test      # run the tests
fa build     # compile (applications only)
fa preview   # run the compiled application
fa check     # type-check without producing output
fa format    # format every source file
```

## Package configuration

`package.fab` declares the package's name, version, and targets:

```
name = "farm"
version = "4.2.1"
platforms = ["web", "native"]
```

See [configuration](./configuration) for every available field.
