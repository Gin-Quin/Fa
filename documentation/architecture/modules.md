# Modules

Fa organizes code by the file system rather than by import statements. This page explains how code is imported and exported.

## Core principles

1. **Single export**: each file exports exactly one entity (a value, a function, a type, a namespace...).
2. **Implicit namespaces**: each folder is a namespace.
3. **Implicit imports**: no import statement is needed for sibling files or root files.
4. **Grouping without namespacing**: a folder written `(myFolderName)` groups files without creating a namespace.

## Importing code

In Fa, importing code is much simpler than in languages like JavaScript: code becomes available based on where it lives in the file structure.

### Example file structure

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

In JavaScript, explicit imports are required:

```js
import myOtherFile from "./myOtherFile.js"
import mySubFile from "./mySubFolder/mySubFile.js"
import myRootFile from "../myRootFile.js"

console.log(myOtherFile, mySubFile, myRootFile)
```

In Fa, you write nothing: you use the namespaces available to the file.

```fa
console.log(
	myOtherFile -- sibling file: available directly
	mySubFolder.mySubFile -- through the sibling folder's namespace
	myRootFile -- root file: available everywhere in the same root
)
```

Organize the root of your `library/` and `application/` directories into folders, so the global namespace stays small.

### Import rules

- **Sibling files**: available directly, by filename without the extension.
- **Sibling folders**: available directly, as namespaces.
- **Nested files**: available through the folder namespaces (`folder.file`).
- **Root files**: available from anywhere in the same root (`library/` or `application/`).
- **Library files from the application**: available through the `library` namespace (`library.myFolder.myFile`).
- **Parent directory files**: not directly accessible; reach them from the root.

:::tip
**Editor support**: ideally, the editor highlights implicitly available names with a distinct color, to tell them apart from locally declared ones.
:::

### Shortening paths with destructuring

Since files and folders are available through namespaces, you never need to import them. But you can create aliases of nested values by [destructuring](/operations/destructuring) them with `use`, to avoid repeating long paths:

```fa
-- create aliases of three values of the `utils.text` namespace
use { formatDate, validateEmail as isEmail, slugify } = utils.text

-- use them directly
console.log(formatDate(today))
console.log(isEmail("user@example.com"))

-- other values are still accessible through the namespace
utils.dialogs.alert("Hello World")
```

A single value can also be aliased on its own:

```fa
use Component = react.Component
```

`use` never copies anything: it only gives a new name to an existing value.

## Exporting code

Every file in Fa exports exactly one entity. The exported entity is automatically named after the file itself.

### Exporting a value

```fa
export = 42
```

The value is available through the filename:

```fa
-- if the code above is in "answer.fa"
console.log(answer) -- 42
```

An export can have a type annotation, like any declaration:

```fa
export: Integer = 42
```

### Exporting a function

```fa
export function = (x: Integer, y: Integer) => x + y
```

### Exporting a type

```fa
export type = {
	name: String
	age: Integer
	isActive: Boolean
}
```

It can then be used in type annotations:

```fa
-- if the type above is in "User.fa"
function describe = (user: User): String => "{user.name}, {user.age} years old"

function createUser = (name: String, age: Integer) => User { name = name, age = age, isActive = true }
```

### Exporting a namespace

To export several related values, export a namespace:

```fa
export namespace = {
	let defaultValue = 100

	function increment = (x: Integer) => x + 1

	function decrement = (x: Integer) => x - 1

	namespace constants = {
		let pi = 3.14159
		let maximumUsers = 1000
	}
}
```

Its members are accessible through the filename:

```fa
-- if the namespace above is in "numbers.fa"
console.log(numbers.defaultValue) -- 100
console.log(numbers.increment(5)) -- 6
console.log(numbers.constants.pi) -- 3.14159
```

### Private members

A member whose name starts with `#` is private: it can be used inside the namespace, but it is not exported.

```fa
export namespace = {
	function parse = (input: String): Ast throws ParseError => {
		try #parseInternal(input, 0)
	}

	function #parseInternal = (input: String, depth: Integer): Ast throws ParseError => ...
}
```

```fa
-- if the namespace above is in "parser.fa"
parser.parse("1 + 2") -- ok
parser.#parseInternal("1 + 2", 0) -- error: `#parseInternal` is private
```

## Advanced features

### Parenthesized folders

Folders with parentheses `(utils)` group files without creating a namespace:

```
library/
  └── (utils)/
      ├── format.fa
      └── validate.fa
```

Files in parenthesized folders are accessed directly:

```fa
-- no need to write utils.format
console.log(format("Hello"))
console.log(validate("email@example.com"))
```

### Re-exporting

To create an aggregated export, re-export values from other files:

```fa
-- in "index.fa"
export namespace = {
	-- re-export other files (the names of the namespace shadow the sibling files)
	use User = User -- from User.fa
	use createUser = createUser -- from createUser.fa

	-- add new exports
	let version = "1.0.0"
}
```

## Building a library

When building a library, all files and folders of the `library/` directory are available to the outside world.

Let's say you're building a library called `farm` that exports utilities to manage a farm:

```
farm/
  ├── package.fa  <-- your configuration file
  ├── dependencies.fa  <-- your dependencies
  └── library/
      ├── (animals)/  <-- because of the parentheses, this folder is not a namespace
      │   ├── cows.fa
      │   ├── cows.test.fa  <-- test files are colocated with the source files
      │   └── pigs.fa
      └── machines/
          ├── tractor.fa
          ├── tractor.test.fa
          ├── harvester.fa
          └── irrigator.fa
```

In this case, the `farm` library exports the following namespaces:

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
These are the exact same namespaces that the library's own files see: there is no separate public API to declare. Use `#` members to keep implementation details private.
:::

In Fa, libraries are shared unbundled and uncompiled. Push your source code to a Git repository with its configuration file. See [deploying a library](/guides/deploy-a-library).

## Importing a library

To use a library, declare it in the `dependencies.fa` file at the root of your package (or of your workspace, to share it between all packages):

```fa
-- dependencies.fa
export = {
	farm = "github:farmers/farm@4"
}
```

The library is then available as a namespace in every file of the package: `farm.cows`, `farm.machines.tractor`...

Libraries can also be loaded from the web, or imported directly in the code. See [dependencies](/architecture/dependencies).

## Compiling an application

The same way your library files live in the `library/` directory, your application files live in the `application/` directory.

```
my-app/
  ├── package.fa
  ├── library/
  │   └── math.fa
  └── application/
      └── main.fa
```

In your `package.fa`, you can specify the targets you want to compile for (`web`, `bun`, `node`, `deno`, `native`, `wasm`, `rust`). See [configuration](/architecture/configuration#targets).

### The entry point

`application/main.fa` is the entry point of the application (another file can be chosen with the `entry` field of the [configuration](/architecture/configuration#entry)). It exports a function, which is called when the program starts:

```fa
-- application/main.fa
export function = () => {
	console.log("Hello, World!")
}
```

The entry function may take the command-line arguments as `arguments: Array(String)`, and may return an exit code as an `Integer`. The errors it can fail with are declared with `throws`, like for any other function:

```fa
-- application/main.fa
export function = (arguments: Array(String)): Integer throws ConfigError => {
	let configuration = try library.loadConfiguration(arguments)
	library.runServer(configuration)
	0
}
```

If an error escapes the entry function, the program stops with a non-zero exit code and the error is reported.

### Development

To start a development session, use the `fa develop` command. It starts a development server that automatically compiles your application and re-runs the tests when you make changes:

```bash
fa develop
```

By default, your code is compiled to JavaScript and executed by the [Bun](https://bun.sh) runtime. This allows much faster iteration and debugging than traditional low-level languages. See [hot reloading](/architecture/hot-reloading).

### Compilation

To compile your application, use the `fa build` command. It compiles your application for the selected targets:

```bash
fa build
fa build --target=native  # <-- only one target
```

## Workspaces

To work on several packages at once, group them in a [workspace](/architecture/workspaces).
