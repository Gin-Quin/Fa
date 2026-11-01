# Packages

In Fa, a **package** is a directory of related files, marked by a `package.fa` file. A package can contain:

- a **library**: code that is shared with other packages, in the `library/` directory,
- an **application**: code that is compiled and run, in the `application/` directory,
- or both.

This is the file structure of a package:

```
myPackage/
├── package.fa
├── dependencies.fa
├── library/
│   ├── math.fa
│   ├── math.test.fa
│   └── .. your other library files ..
├── application/
│   ├── main.fa
│   └── .. your other application files ..
└── scripts/
    └── seed.fa
```

You can scaffold a package with the following command:

```bash
fa new package myPackage
```

## What each part is for

| Path | Purpose |
| --- | --- |
| `package.fa` | the [configuration](/architecture/configuration) of the package |
| `dependencies.fa` | the [dependencies](/architecture/dependencies) of the package |
| `library/` | code exported to the outside world when the package is published |
| `application/` | the runnable program; `application/main.fa` is its [entry point](/architecture/modules#the-entry-point) |
| `scripts/` | one-off scripts, run with `fa run` (for example `fa run scripts/seed.fa`) |

A script is run like a [standalone file](/architecture/overview#standalone-files): its top-level statements run in order, and the folder `scripts/` is its root. It also sees the `library` namespace and the dependencies of the package, like the files of `application/`.

:::tip
If you're writing a library, you can omit the `application/` folder.

If you're developing an application, the `library/` folder is optional, but recommended: it is your application's "local" library, holding the logic that the application and the tests use.
:::

## Libraries and applications

The difference is what a package produces.

A **library package** exposes everything under `library/`. It is published as source, unbundled and uncompiled, and consumed by other packages. See [deploying a library](/guides/deploy-a-library).

An **application package** is compiled into a program for the [targets](#targets) listed in its configuration. Its `application/` directory is private: nothing in it can be used by another package.

A package that has both is a common and useful shape: the library holds the logic, and the application is a thin entry point around it.

## Library and application roots

The `library/` and `application/` directories are two separate roots: the files of `application/` don't see the files of `library/` implicitly.

To use the library from the application, use the `library` namespace:

```fa
-- application/main.fa
export function = () => {
	let total = library.math.add(1, 2) -- uses `library/math.fa`
	console.log(total)
}
```

Inside each root, files are accessed through [implicit namespaces](/architecture/modules). The dependencies of the package are available in both roots. The files of `library/` never see the files of `application/`, and a package without a `library/` directory has no `library` namespace.

## Tests

Test files sit next to the code they test, and are named `*.test.fa`:

```
library/
├── math.fa
└── math.test.fa
```

Test files can live in `library/` and in `application/`, and see the names of their folder like any other file. They are excluded from builds. See [testing](/architecture/testing).

## Commands

Run these from anywhere inside the package:

```bash
fa develop  # <-- development server with hot reloading
fa test  # <-- run the tests
fa check  # <-- type-check without producing output
fa format  # <-- format every source file
fa build  # <-- compile the application
fa preview  # <-- run the compiled application
```

## Package configuration

The `package.fa` file contains the configuration of the package, which is a regular [Fa configuration file](/architecture/configuration):

```fa
export = {
	name = "farm"
	version = "1.0.0"
	targets = ["web", "bun", "native"]
}
```

See [configuration](/architecture/configuration) for every available field.

## Targets

An application can be compiled for several targets at once:

| Target | Output |
|---|---|
| `web` | JavaScript for the browser |
| `bun` | JavaScript for the server, running on [Bun](https://bun.sh) (the default server runtime) |
| `node` | JavaScript for the server, running on [Node.js](https://nodejs.org) |
| `deno` | JavaScript for the server, running on [Deno](https://deno.com) |
| `native` | a native binary |
| `wasm` | WebAssembly |
| `rust` | Rust source code, to be used in a Rust project |

When the configuration doesn't list any target, the application is compiled for `web`, `bun` and `native`.

:::tip
Depending on the chosen targets, some features may or may not be available. Using a feature that a selected target doesn't support is a compile error.
:::
