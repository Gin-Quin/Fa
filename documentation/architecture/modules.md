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

```
source/
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
// In Fa, no import statements are needed
// Instead, you should use the namespaces available to the file
console.log(
  myOtherFile,             // Sibling file - automatically available
  mySubFolder.mySubFile,   // Accessed through sibling folder
  myRootFile,              // Root file - automatically available
)
```

Beware of organizing carefully the root of your project into folders to avoid polluting your global namespace.

### Import Rules

- **Sibling files**: Directly accessible by filename (without extension)
- **Nested files**: Accessed through namespace paths (folder.file)
- **Root files**: Directly accessible from anywhere in the project
- **Parent directory files**: Not directly accessible (must be accessed through root)

> **IDE Support**: Ideally, the IDE highlights imported values with a distinct color to improve readability.

### Import Aliases with the Extract Operator

Fa provides a convenient way to extract specific values from namespaces using the `>>` extract operator. This is particularly useful when you want to use specific parts of a namespace without referencing the full path each time.

#### Basic Extraction

```fa
// Extract Component from the react namespace
use react.Component

// Now Component can be used directly
class MyComponent extends Component {
  // ...
}
```

#### Multiple Extractions

You can extract multiple values at once:

```fa
// Extract multiple items from a namespace
use utils >> { formatDate, validateEmail, parseJSON }

// Use them directly
formatDate(new Date())
validateEmail("user@example.com")
```

#### Nested Extraction

You can extract from nested namespaces:

```fa
// Extract from nested namespaces
use ui.components >> { Button, Card, Modal }

// Use components directly
Button({ label: "Click me" })
```

#### Combining with Regular Imports

The extraction operator can be combined with regular namespace access:

```fa
// Extract some components
use ui >> { Button, Card }

// Still access other components through the namespace
ui.Modal.show("Hello World")
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
// If the above code is in "answer.fa"
console.log(answer) // Outputs: 42
```

### Exporting a Function

To export a function:

```fa
export function = (x: Number, y: Number) => x + y
```

### Exporting a Type

To export a type definition:

```fa
export type = {
  name: string
  age: number
  isActive: boolean
}
```

This can be used in type annotations:

```fa
// If the above type is in "User.fa"
function createUser = (data: User): User => {
  // ...
}
```

### Exporting a Namespace

To export multiple related values as a namespace:

```fa
export namespace = {
  defaultValue = 100
  
  function increment = (x: number) = x + 1
  
  function decrement = (x: number) = x - 1
  
  namespace constants = {
    #pi = 3.14159
    #maxUsers = 1000
  }
}
```

When imported, all properties are accessible through the filename:

```fa
// If the above namespace is in "Math.fa"
console.log(Math.defaultValue)      // 100
console.log(Math.increment(5))      // 6
console.log(Math.Constants.PI)      // 3.14159
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
// No need to use utils.format
console.log(format("Hello"))
console.log(validate("email@example.com"))
```

### Re-exporting

To create an aggregated export, you can re-export values from other files:

```fa
// In "index.fa"
export namespace = {
  // Re-export other files
  User = User           // from User.fa
  createUser = createUser  // from createUser.fa
  
  // Add new exports
  version = "1.0.0"
}
```

## Building a library

When building a library, all files and folders in the `library/` directory will be available to the outside world.

Let's say you're building a library called `farm`, that export utilities to manage a farm.

```
farm/
  ├── project.fab  <- your configuration file
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

In this case, the `farm` library will export the following namespaces

```fa
use farm.cows
use farm.pigs
use farm.machines.tractor
use farm.machines.harvester
use farm.machines.irrigator

// or:

use farm >> {
  cows
  pigs
  machines {
    tractor
    harvester
    irrigator
  }
}
```

:::tip
These are the exact same namespaces that are exported by the `farm` library.
:::

In Fa, libraries are shared unbundled and uncompiled. Just push your source code to a git repository with a configuration file.

:::warning
TODO: Guide to deploy with Github Actions / CircleCI.
:::

## Importing a library

To import a library, simply export the library name with its version range to the `/dependencies.fa` file in the root of your project.

```
// import the library "farm" from github
use github:farmers/farm@4

// import the library "farm" but alias it as "farming"
use github:farmers/farm@4 >> farming

// import specific values from a library
use github:farmers/farm@4 >> { cows, pigs, machines }
```

## Compiling an application

The same way your library files live in the `library/` directory, your application files live in the `application/` directory.

```
my-app/
  ├── project.fab
  └── application/
      └── main.fa
```

In your `project.fab`, you might specify which platform you want to compile for among `web`, `bun`, `native` and `wasm`. You can select multiple platforms at once. By default, all platforms are selected.

```
platforms = [ "web", "native" ]
```

:::tip
Depending on the chosen platforms, some features may or may not be available.
:::

### Development

To start a development session, you can use the `fa develop` command. This will start a development server that will automatically compile your application and re-run tests when you make changes.

```bash
fa develop
```

By default, and if you have the `bun` platform selected, your code will be compiled to Javascript and executed via the `bun` runtime.

This allows much faster iteration and debugging than traditional low-level languages.

### Compilation

To compile your application, you can use the `fa build` command. This will compile your application for the selected platforms.

```bash
fa build
```

## Workspaces

You can also have multiple packages in the same project. In this case, you should create a `packages/` directory that contains all your sub-packages.

These packages can be organized in subdirectories. The `packages/` directory will be scanned to find all folders that contain a `project.fab` file.

```
my-app/
  ├── project.fab
  └── packages/
      ├── myPackageA/
          ├── project.fab
          └── library/
              └── foo.fa
      └── nested-packages/
          ├── myPackageB/
              ├── project.fab
              └── application/
                  └── bar.fa
          └── myPackageC/
              ├── project.fab
              └── application/
                  └── baz.fa
```
