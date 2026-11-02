# Architecture overview

With Fa, you can work at three different levels:

- **Standalone file**: a single file that you can run directly.
- **Package**: a collection of related files: a library, an application, or both.
- **Workspace**: a collection of related packages.

Each level is a superset of the one before it. Start with a file, grow into a package when the file gets too big, and group packages into a workspace when they need to share code.

| Level | Marker file | Contains |
| --- | --- | --- |
| Standalone file | none | one `.fa` file, and the files and folders next to it |
| Package | `package.fa` | `dependencies.fa`, `library/`, `application/`, `scripts/` |
| Workspace | `workspace.fa` | `dependencies.fa`, `packages/` |

## Standalone files

Standalone files are the simplest form of Fa code. They are single files that you can run directly:

```bash
fa run myFile.fa  # <-- don't forget the .fa extension
fa build myFile.fa --target=native  # <-- compile the file
```

`fa run` compiles the file for the `bun` target and runs it with [Bun](https://bun.sh), which the toolchain needs (see [installation](/getting-started/installation)).

In a standalone file, the folder of the file acts as its root: sibling files and sibling folders are implicitly available, like in a package (see [modules](/architecture/modules)).

```fa
-- mySiblingFile.fa and the folder utils/ are next to this file
console.log(mySiblingFile)
console.log(utils.format("Hello"))
```

The file given to `fa run` is the program: its top-level statements run in order, after the module-level values they use are initialized. They can use `try` and `await`: an error that escapes them ends the program with the exit code `1`, like an error escaping an [entry function](/architecture/modules#the-entry-point).

The arguments given after the file name (`fa run myFile.fa first second`) are in `arguments`, an implicit `Array(String)` that the top level of a standalone file or a script sees: `arguments.get(0)` is the first one (`first`). If the file exports a function, it is called after the top-level statements, as an entry function: it receives the same arguments, and its result is the exit code.

The other files are modules: they contain only declarations, like the files of a package (see [module-level values and initialization](/architecture/modules#module-level-values-and-initialization)).

Files in parent directories are not accessible. To organize code across several folders, create a [package](#packages).

Remote modules can be imported with `@import`, see [dependencies](/architecture/dependencies). Their versions and hashes are locked in a `dependencies.lock` file next to the standalone file.

Standalone files are useful for:

- writing one-off scripts,
- writing small programs,
- playing around with ideas,
- learning Fa.

## Packages

Standalone files are great, but when your code grows, you'll need a way to organize it. This is where packages come in.

A **package** is a directory that contains a `package.fa` file, and optionally:

- a `dependencies.fa` file,
- a `library/` directory,
- an `application/` directory,
- a `scripts/` directory.

A package can contain a library, an application, or both. See [packages](/architecture/packages).

When working with packages, you can use the following commands:

```bash
fa develop  # <-- start a development server with hot reloading
fa test  # <-- run the tests
fa check  # <-- type-check without producing output
fa build  # <-- compile the application
fa preview  # <-- run the compiled application
```

To scaffold a new package, use the following command:

```bash
fa new package myPackage
```

## Workspaces

A **workspace** is a collection of related packages. Workspaces are useful for building several libraries and applications that are closely related, and make sharing code between them easy, without publishing it.

A workspace is a directory containing a `workspace.fa` file and a `packages/` directory, which contains the packages of the workspace. See [workspaces](/architecture/workspaces).

To scaffold a new workspace, use the following command:

```bash
fa new workspace
```

When working with workspaces, you use the same commands as when working with packages. The commands are executed for every package that implements them: `build`, `develop` and `preview` for the packages with an application, `test` for the packages with test files, `check` and `format` for all of them:

```bash
fa develop  # <-- start a development server with hot reloading for all packages
fa test  # <-- run the tests of all packages
fa build  # <-- compile all applications
```

You can also run a command on a specific package, named by the `name` field of its `package.fa`:

```bash
fa develop myPackage  # <-- start a development server with hot reloading for myPackage
fa test myPackage  # <-- run the tests of myPackage
fa build myPackage  # <-- compile the application of myPackage
fa preview myPackage  # <-- run the compiled application of myPackage
```

## Command line

Every command works from anywhere inside a package or a workspace: `fa` looks for the closest `package.fa`, then for the closest `workspace.fa`. In a workspace, `<package>` limits the command to one package; without it, the command runs for every package that implements it.

| Command | Description |
| --- | --- |
| `fa run [--target=<target>] <file.fa> [arguments...]` | run a standalone file or a [script](/architecture/packages#what-each-part-is-for): compile it for `bun` and run it with Bun, or for `<target>` and run the result; everything after the file goes to the program (a `--` right after the file is accepted and dropped) |
| `fa new package <name>` | scaffold a package in a new folder `<name>`: the package of the [hello world](/guides/hello-world#growing-into-a-package) guide, with a test |
| `fa new workspace [<name>]` | scaffold a workspace (`workspace.fa`, `dependencies.fa`, `packages/`), in a new folder `<name>` or in the current folder |
| `fa develop [<package>] [--target=<target>]` | compile, run, watch and reload; `bun` by default (see [hot reloading](/architecture/hot-reloading)) |
| `fa test [<package>] [--watch] [--coverage] [--target=<target>]` | run the tests; `bun` by default (see [testing](/architecture/testing)) |
| `fa check [<package>] [--format=json]` | check every file (library, application, scripts, tests) for every configured target, run the compile-time tests, produce no output; `--format=json` prints the diagnostics as JSON, one object per line (`file`, `range`, `severity`, `code`, `message`), for editors and CI |
| `fa format [<file or folder>...] [--check]` | format the source files (without a path, the closest package, else the workspace); `--check` only reports the files that are not formatted |
| `fa build [<package> \| <file.fa>] [--target=<target>] [--output=<directory>]` | check every file of the library and the application, and compile the application, or a standalone file, for its targets or for one target, into `<directory>/<target>/` (`build.output`, `build` by default) |
| `fa preview [<package>] [--target=<target>]` | run the output of `fa build`; without `--target`, the first target of the configuration that runs on this machine |
| `fa update [<dependency>...]` | resolve the dependencies again and rewrite the lockfile, for all of them or for the named ones (see [dependencies](/architecture/dependencies#lockfile)) |
| `fa bind <header.h> [--output <file>] [-- clang flags...]` | generate a [foreign file](/advanced/foreign-code#generating-bindings) from a C header |
| `fa help [<command>]` | describe the commands |
| `fa version` | print the version of `fa` |

`fa build` writes, in `build/<target>/`: an executable for `native`, a `.wasm` file and its JavaScript loader for `wasm`, JavaScript modules for `bun`, `node` and `web` (bundled into one `main.js` for `web`). `fa run`, `fa test` and `fa develop` run on `bun` by default: `fa` looks for `bun` on the `PATH`, then in `~/.bun/bin`.

The exit code of a command is `0` when it succeeds and `1` when it fails (a compile error, a failing test, a changed locked dependency, a wrong argument). `fa run` and `fa preview` exit with the exit code of the program: its returned value or `0`, `1` for an escaped error, `101` for a [stop](/values/errors#stops).

## Which level do I need?

- Writing a script, or trying an idea? A **standalone file**.
- Writing something you will publish, test, or come back to? A **package**.
- Building several applications and libraries that share code? A **workspace**.
