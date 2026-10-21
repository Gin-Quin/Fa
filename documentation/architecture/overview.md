# Architecture overview

With Fa, you can work at three different levels:

- **Standalone file**: a single file that you can run directly.
- **Package**: a collection of files that are related to each other: a library, an application, or both.
- **Workspace**: a collection of packages that are related to each other.

## Standalone files

Standalone files are the simplest form of Fa code. They are single files that you can run directly.

You can run a standalone file with the following commands:

```bash
fa run myFile.fa  # <-- don't forget the .fa extension
fa build myFile.fa --target=native  # <-- compile the file
```

In a standalone file, the folder of the file acts as its root: sibling files and sibling folders are implicitly available, like in a package (see [modules](/architecture/modules)).

```fa
-- mySiblingFile.fa and the folder utils/ are next to this file
console.log(mySiblingFile)
console.log(utils.format("Hello"))
```

Files in parent directories are not accessible. To organize code across several folders, create a [package](#packages).

Remote modules can be imported with `@import`, see [dependencies](/architecture/dependencies).

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
fa build  # <-- compile the application
fa preview  # <-- preview the application
```

To scaffold a new package, you can use the following command:

```bash
fa new package
```

## Workspaces

A **workspace** is a collection of packages that are related to each other. Workspaces are useful for building multiple libraries and applications that are closely related, and make sharing code between them easy.

A workspace is a directory containing a `workspace.fa` file and a `packages/` directory, which contains the packages of the workspace. See [workspaces](/architecture/workspaces).

To scaffold a new workspace, you can use the following command:

```bash
fa new workspace
```

When working with workspaces, you use the same commands as when working with packages. The commands are executed for every package that implements them:

```bash
fa develop  # <-- start a development server with hot reloading for all packages
fa test  # <-- run the tests of all packages
fa build  # <-- compile all applications
```

You can also run a command on a specific package:

```bash
fa develop myPackage  # <-- start a development server with hot reloading for myPackage
fa test myPackage  # <-- run the tests of myPackage
fa build myPackage  # <-- compile the application of myPackage
fa preview myPackage  # <-- preview the application of myPackage
```
