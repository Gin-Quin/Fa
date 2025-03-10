# Architecture overview

With Fa, you can work at three different levels:

- **Standalone file**: A standalone file is a single file that you can run directly.
- **Package**: A package is a collection of files that are related to each other.
- **Project**: A project is a collection of packages that are related to each other.

## Standalone files

Standalone files are the simplest form of Fa code. They are single files that you can run directly.

You can run a standalone file with the following command:

```bash
fa run myFile.fa  # <-- don't forget the .fa extension
fa build myFile.fa --target=native  # <-- compile the file
```

In standalone files, there is no implicit import system. You need to explicitly import the code you need.

```fa
use ./mySiblingFile.fa
use ../myParentFile.fa
use ../../myGrandParentFile.fa
```

Standalone files are useful for:

- Writing one-off scripts
- Writing small programs
- Playing around with ideas
- Learning Fa

## Packages

Standalone files are great, but when your project grows, you'll need a way to organize your code. This is where packages come in.

A **package** is a directory that contains a `package.fab` file, and optionally:

- a `dependencies.fa` file,
- a `library/` directory,
- an `application/` directory,
- a `scripts/` directory.

A package can be a library or an application.

When working with packages, you can use the following commands:

```bash
fa develop  # <-- start a development server with hot reloading
fa test  # <-- run the tests
fa build  # <-- compile the package (for applications only)
fa preview  # <-- preview the package (for applications only)
```

To scaffold a new package, you can use the following command:

```bash
fa new package
```

## Projects

A **project** is a collection of packages that are related to each other. Projects are useful for building multiple applications that are closely related. It makes sharing code between your libraries and applications easy.

A project is a directory containing a `project.fab` file. A project will also contain a `packages/` directory, which contains the packages for the project.

To scaffold a new project, you can use the following command:

```bash
fa new project
```

When working with projects, you will use the same commands as when working with packages. The commands will be executed for every package that implements it.

```bash
fa develop  # <-- start a development server with hot reloading for all packages
fa test  # <-- run the tests for all packages
fa build  # <-- compile all *application* packages (for applications only)
```

You can also run commands on a specific package:

```bash
fa develop myPackage  # <-- start a development server with hot reloading for myPackage
fa test myPackage  # <-- run the tests for myPackage
fa build myPackage  # <-- compile myPackage (for applications only)
fa preview myPackage  # <-- preview myPackage (for applications only)
```
