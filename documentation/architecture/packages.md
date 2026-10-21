# Packages

In Fa, a **package** is a collection of files that are related to each other. A package can contain:

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
│   └── .. your other library files ..
└── application/
    ├── main.fa
    └── .. your other application files ..
```

:::tip
If you're writing a library, you can omit the `application/` folder.

If you're developing an application, the `library/` folder is optional: it's your application's "local" library.
:::

You can scaffold a package with the following command:

```bash
fa new package
```

## Library and application

The `library/` and `application/` directories are two separate roots: the files of `application/` don't see the files of `library/` implicitly.

To use the library from the application, use the `library` namespace:

```fa
-- application/main.fa
let total = library.math.add(1, 2) -- uses `library/math.fa`
```

Inside each root, files are accessed through [implicit namespaces](/architecture/modules).

## Package configuration

The `package.fa` file contains the configuration of the package, which is a regular [Fa configuration file](/architecture/configuration):

```fa
export = {
	name = "my-package"
	version = "1.0.0"
	targets = ["web", "bun", "native"]
}
```

## Targets

An application can be compiled for several targets at once:

| Target | Output |
|---|---|
| `web` | JavaScript for the browser |
| `bun` | JavaScript for the server, running on [Bun](https://bun.sh) (the default server runtime) |
| `node` | JavaScript for the server, running on [Node.js](https://nodejs.org) |
| `deno` | JavaScript for the server, running on [Deno](https://deno.com) |
| `native` | A native binary |
| `wasm` | WebAssembly |
| `rust` | Rust source code, to be used in a Rust project |

:::tip
Depending on the chosen targets, some features may or may not be available.
:::
