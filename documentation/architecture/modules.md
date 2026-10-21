# Modules

Fa uses a file system-based approach to code organization that differs significantly from traditional module systems. This document explains how code is imported and exported in Fa.

## Core Principles

1. **Single Export Rule**: Each file exports exactly one value (which can be a function, type, namespace, etc.)
2. **Implicit Namespaces**: Folders automatically create namespaces
3. **Implicit Imports**: No explicit import statements are needed for sibling files or root files
4. **Grouping Without Namespacing**: Folders can be parenthesized `(myFolderName)` to group files without creating a namespace

## Importing Code

In Fa, importing code is much simpler than in languages like JavaScript. The system automatically makes code available based on its location in the file structure.

### Example File Structure

Each package has two roots: `library/` and `application/` (see [packages](/architecture/packages)). In this example, we are in the `library/` root:

```
library/
  ├── myRootFile.fa
  └── myFolder/
      ├── myFile.fa        <-- we are here
      ├── myOtherFile.fa
      └── mySubFolder/
          └── mySubFile.fa
```

#### Comparison with JavaScript

**JavaScript approach:**

```js
// In JavaScript, explicit imports are required
import myOtherFile from "./myOtherFile.fa"
import mySubFile from "./mySubFolder/mySubFile.fa"
import myRootFile from "../myRootFile.fa"

console.log(
  myOtherFile,
  mySubFile,
  myRootFile,
)
```

**Fa approach:**

```fa
-- In Fa, no import statements are needed
-- Instead, you should use the namespaces available to the file
console.log(
  myOtherFile,             -- Sibling file - automatically available
  mySubFolder.mySubFile,   -- Accessed through sibling folder
  myRootFile,              -- Root file - automatically available
)
```

Organize the root of your `library/` and `application/` directories into folders to avoid polluting the global namespace.

### Import Rules

- **Sibling files**: Directly accessible by filename (without extension)
- **Sibling folders**: Directly accessible as namespaces
- **Nested files**: Accessed through namespace paths (folder.file)
- **Root files**: Directly accessible from anywhere in the same root (`library/` or `application/`)
- **Library files from the application**: Accessed through the `library` namespace (`library.myFolder.myFile`)
- **Parent directory files**: Not directly accessible (must be accessed through the root)

:::tip
**IDE Support**: Ideally, the IDE highlights imported values with a distinct color to improve readability.
:::

### Shortening paths with destructuring

Since files and folders are available through namespaces, you never need to import them. But you can create aliases of nested values by [destructuring](/operations/destructuring) them with `use`, to avoid repeating long paths:

```fa
-- create aliases of three values of the `ui.components` namespace
use { Button, Card, Modal } = ui.components

-- use them directly
Button({ label = "Click me" })

-- other values are still accessible through the namespace
ui.dialogs.Alert.show("Hello World")
```

## Exporting Code

Every file in Fa exports exactly one entity. The exported entity is automatically named after the file itself.

### Exporting a Value

To export a simple value (number, string, function, etc.):

```fa
export = 42
```

When imported, this value will be available using the filename:

```fa
-- If the above code is in "answer.fa"
console.log(answer) -- Outputs: 42
```

### Exporting a Function

To export a function:

```fa
export function = (x: Integer, y: Integer) => x + y
```

### Exporting a Type

To export a type definition:

```fa
export type = {
  name: String
  age: Integer
  isActive: Boolean
}
```

This can be used in type annotations:

```fa
-- If the above type is in "User.fa"
function createUser = (data: User): User => {
  -- ...
}
```

### Exporting a Namespace

To export multiple related values as a namespace:

```fa
export namespace = {
  defaultValue = 100
  
  function increment = (x: Integer) => x + 1
  
  function decrement = (x: Integer) => x - 1
  
  namespace constants = {
    pi = 3.14159
    maxUsers = 1000
  }
}
```

When imported, all properties are accessible through the filename:

```fa
-- If the above namespace is in "Math.fa"
console.log(Math.defaultValue)      -- 100
console.log(Math.increment(5))      -- 6
console.log(Math.constants.pi)      -- 3.14159
```

## Advanced Features

### Parenthesized Folders

Folders with parentheses `(utils)` group files without creating a namespace:

```
source/
  └── (utils)/
      ├── format.fa
      └── validate.fa
```

Files in parenthesized folders can be accessed directly:

```fa
-- No need to use utils.format
console.log(format("Hello"))
console.log(validate("email@example.com"))
```

### Re-exporting

To create an aggregated export, you can re-export values from other files:

```fa
-- In "index.fa"
export namespace = {
  -- Re-export other files
  User = User           -- from User.fa
  createUser = createUser  -- from createUser.fa
  
  -- Add new exports
  version = "1.0.0"
}
```

## Building a library

When building a library, all files and folders in the `library/` directory will be available to the outside world.

Let's say you're building a library called `farm` that exports utilities to manage a farm.

```
farm/
  ├── package.fa  <- your configuration file
  ├── dependencies.fa  <- your dependencies
  └── library/
      ├── (animals)/  <-- because of the parentheses, this folder is not a namespace
          ├── cows.fa
          ├── cows.test.fa  <-- test files are colocated with the source files
          └── pigs.fa
      └── machines/
          ├── tractor.fa
          ├── tractor.test.fa
          ├── harvester.fa
          └── irrigator.fa
```

In this case, the `farm` library will export the following namespaces:

```fa
farm.cows
farm.pigs
farm.machines.tractor
farm.machines.harvester
farm.machines.irrigator
```

Which can be aliased at once:

```fa
use {
	cows
	pigs
	machines { tractor, harvester, irrigator }
} = farm
```

:::tip
These are the exact same namespaces that are exported by the `farm` library.
:::

In Fa, libraries are shared unbundled and uncompiled. Just push your source code to a Git repository with a configuration file.

:::warning
TODO: Guide to deploying with GitHub Actions / CircleCI.
:::

## Importing a library

To use a library, declare it in the `dependencies.fa` file at the root of your package (or of your workspace, to share it between all packages):

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
}
```

The library is then available as a namespace in every file of the package: `farm.cows`, `farm.machines.tractor`, ...

Libraries can also be loaded from the web, or imported directly in the code. See [dependencies](/architecture/dependencies).

## Compiling an application

The same way your library files live in the `library/` directory, your application files live in the `application/` directory.

```
my-app/
  ├── package.fa
  └── application/
      └── main.fa
```

In your `package.fa`, you can specify the targets you want to compile for (`web`, `bun`, `node`, `deno`, `native`, `wasm`, `rust`). See [package configuration](/architecture/packages#package-configuration).

### Development

To start a development session, you can use the `fa develop` command. This will start a development server that will automatically compile your application and re-run tests when you make changes.

```bash
fa develop
```

By default, and if you have the `bun` target selected, your code will be compiled to JavaScript and executed via the `bun` runtime.

This allows much faster iteration and debugging than traditional low-level languages.

### Compilation

To compile your application, you can use the `fa build` command. This will compile your application for the selected targets.

```bash
fa build
```

## Workspaces

To work on several packages at once, group them in a [workspace](/architecture/workspaces).
