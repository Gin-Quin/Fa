# Modules

Fa organizes code by the file system rather than by import statements. This page explains how code is imported and exported.

## Core principles

1. **Single export**: each file exports exactly one entity (a value, a function, a type, a namespace...). Test files are the one exception: they export nothing (see [testing](/architecture/testing)).
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
- **Root entries**: the files and folders at the root of `library/` or `application/` are available from anywhere in the same root.
- **Library files from the application**: available through the `library` namespace (`library.myFolder.myFile`).
- **Dependencies**: available from anywhere in the package, by the name given in `dependencies.fa` (see [dependencies](/architecture/dependencies)).
- **The prelude**: the built-in types and values, available everywhere (see [the prelude](#prelude)).
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
function welcome = (email: String): String => {
	if isEmail(email) {
		return "Welcome, {slugify(email)}! Today is {formatDate(today)}."
	}
	utils.dialogs.alert("Invalid email") -- other values are still accessible through the namespace
	""
}
```

A single value can also be aliased on its own:

```fa
use Component = react.Component
```

`use` never copies anything: it only gives a new name to an existing value. An alias of a file, of a member of a namespace or of a dependency is never invalidated, unlike a [reference](/values/declaring-values) to a place inside a value, such as `use first = people[0]`.

## Name lookup

A name is looked up in this order, and the first match wins:

1. the **local scopes**, from the innermost to the outermost: blocks, the function, then the module-level declarations of the file and the [file's own name](#the-files-own-name);
2. the **siblings** of the file: the files and folders of its folder, including the contents of its `(group)` folders;
3. the **root entries**: the files and folders at the root of `library/` or `application/` (with the contents of their `(group)` folders), and the `library` namespace in `application/`;
4. the **dependencies** of the package;
5. the **prelude**.

```
library/
  ├── format.fa
  └── reports/
      ├── format.fa
      └── summary.fa      <-- `format` is `reports/format.fa`, a sibling
```

Shadowing a name of a later level is allowed: a sibling `format.fa` shadows a root `format.fa`, and a local declaration shadows both. Shadowing a name of the prelude is a warning.

Two entries with the same name at the same level are a compile error, wherever the name is used:

- a file `report.fa` and a folder `report/`;
- a file and a file of a `(group)` folder in the same folder;
- a dependency key and a root entry, or a root entry named `library` in `application/`;
- a module-level declaration of a file and the file's own name.

File and folder names follow these rules:

- The name of a `.fa` file without its extension, and the name of a folder that contains `.fa` files, must be a valid [identifier](/getting-started/syntax-basics#identifiers). `my-file.fa` or `2d.fa` is a compile error.
- `*.test.fa` files are never names: they can't be used from other files.
- A [worker file](/advanced/concurrency#2-workers) `MathWorker.worker.fa` is named by the part before `.worker.fa`: `MathWorker`. So is a [foreign file](/advanced/foreign-code#foreign-files): `Database.foreign.fa` is `Database`.
- Two entries of the same folder whose names differ only by case (`User.fa` and `user.fa`) are a compile error, so a project behaves the same on every file system.
- Files that don't end in `.fa`, and folders that contain no `.fa` file, are ignored: they can be read at compile time (`@readFile`), and their names don't have to be identifiers. Names that start with a `.` are ignored.

The right-hand side of a declaration never sees the name being declared: in `use User = User`, the right-hand `User` is looked up as if the left-hand one didn't exist (see [re-exporting](#re-exporting)).

## Exporting code

Every file in Fa exports exactly one entity. The exported entity is automatically named after the file itself.

A file of `library/` or `application/` without an export is an error. Only a standalone file and the scripts, which are programs rather than modules, can have none.

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

An exported value is deeply immutable, like a value declared with `let`.

### Exporting a mutable value

`export mutable` exports a value that other files can modify:

```fa
export mutable: Pool<Person> = []
```

```fa
-- if the value above is in "people.fa"
function addAna = () mutates people => {
	use ana = people.add({ name = "Ana" })
	ana.name = "Anna"

	people = [] -- this works too: the value is replaced for every file
}
```

The value lives as long as the program, and its address is stable: every file sees the same value, whether it is modified field by field or replaced as a whole. Its elements can be linked with [relations](/advanced/relations). A function that modifies the value declares it in its effects, like `mutates people.size` for `people.add`. See [module-level values](/values/declaring-values#module-level-values).

A primitive can be exported as mutable too:

```fa
export mutable: Integer = 0
```

```fa
-- if the value above is in "visits.fa"
function countVisit = () mutates visits => {
	visits += 1
}
```

### Exporting a function

```fa
export function = (x: Integer, y: Integer) => x + y
```

An asynchronous function is exported with `export async function`:

```fa
export async function = (url: String): String throws HttpError => {
	let response = try await std.http.get(url)
	response.text()
}
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

### Exporting a generic

The generic parameters of an exported type or function come right after `type` or `function`, where the name would be:

```fa
-- in "Pair.fa"
export type<First, Second> = { first: First, second: Second }
```

```fa
-- in "first.fa"
export function = <Item>(items: Array<Item>): Item? => items.get(0)
```

They are used like any generic: `Pair<String, Integer>`, `first([1, 2, 3])`. See [generics](/types/generics).

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

A `mutable` member of an exported namespace can be read by every file, but only written from inside the file that declares it. Other files change it through the functions of the namespace that declare it in their effects:

```fa
-- in "counter.fa"
export namespace = {
	mutable count = 0

	function increment = () mutates count => {
		count += 1
	}
}
```

```fa
-- in another file
counter.increment() -- ok
counter.count = 10 -- error: `counter.count` can only be written from "counter.fa"
```

To let every file write a value directly, export it on its own with `export mutable`.

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

The module-level declarations of a file that are not exported are private to the file.

### The file's own name

Inside a file, the name of the file designates its export. This is how a type refers to itself from its own file, and how a file writes to its anonymous `export mutable` value:

```fa
-- in "people.fa"
export mutable: Pool<Person> = []

function clearPeople = () mutates people => {
	people.clear() -- `people` is the export of this file
}
```

```fa
-- in "Person.fa"
export type = {
	name: String
	bestFriend: (Person in people)? -- `Person` is the export of this file
	bestFriendOf: Pool<people->bestFriend>
}
```

A module-level declaration with the same name as the file is a compile error.

### Export forms

These are all the forms of `export`:

| Form | Exports |
| --- | --- |
| `export = value`, `export: Type = value` | a deeply immutable value |
| `export mutable = value`, `export mutable: Type = value` | a value that every file can modify |
| `export function = ...`, `export function = <Item>(...) => ...` | a function, generic or not |
| `export async function = ...` | an asynchronous function |
| `export type = ...`, `export type<First, Second> = ...` | a type, generic or not |
| `export namespace = { ... }` | a namespace |

`reactive`, `derived` and `static` values can't be exported in this version: declare them as members of an exported namespace, or as module-level values of the file that uses them.

## Module-level values and initialization

A module-level value is a value declared at the top of a file, or as a member of a namespace, exported or not. It lives as long as the program (see [module-level values](/values/declaring-values#module-level-values)).

The files of `library/` and `application/` contain only declarations: `let`, `mutable`, `use`, `static`, `function`, `type`, `namespace`, `reactive`, `derived`, `watch`, `export`, `@if` around declarations, and `@for` around declarations without a name (`watch` blocks, and tests in a test file). A `watch` at the top of a file lives as long as the program, like a module-level value. Any other statement at the top of these files, like a call or an assignment, is a compile error. Top-level statements are only allowed in [standalone files](/architecture/overview#standalone-files) and [scripts](/architecture/packages#what-each-part-is-for).

Module-level values are initialized before the program starts, once:

- A value whose initializer the compiler can evaluate (a literal, a `static` value, a constant object) needs no initialization at runtime.
- The other values are initialized in **dependency order**: a value is initialized after every module-level value its initializer reads, directly or through the functions it calls. Values that don't depend on each other are initialized in declaration order within a file, and files in the alphabetical order of their paths.
- Only the values the program uses are initialized: the ones reachable from the entry function, from the top-level statements of a standalone file or a script, or from the tests being run.
- A cycle between the initializers of values is a compile error. Cycles between types, and between functions, are allowed: `Person.fa` can use `people` while `people.fa` uses `Person`.

```fa
-- in "a.fa"
export = b + 1
```

```fa
-- in "b.fa"
export = a + 1 -- error: the initializers of `a` and `b` depend on each other
```

An initializer can't propagate an error with `try`, since there is no function to propagate it to: handle it with `catch`. An initializer can't use `await`.

When the program exits normally, module-level values are destroyed in the reverse order of their initialization.

The code of a `.worker.fa` file runs on the worker's thread: it can't use a module-level `mutable` value, unless the value's type is thread-safe (a `Mutex`, an atomic or a `Channel`, see [concurrency](/advanced/concurrency)).

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

A parenthesized folder can be at any level, and can contain other folders: its contents merge into its parent folder, as if they were written there. In `reports/(charts)/bar.fa`, `bar` is a sibling of the files of `reports/`, and other files reach it as `reports.bar`. A parenthesized folder at the root of `library/` adds root entries.

### Re-exporting

To create an aggregated export, re-export values from other files:

```fa
-- in "index.fa"
export namespace = {
	-- re-export other files: each right-hand side is looked up without the name being declared,
	-- so it designates the sibling file
	use User = User -- from User.fa
	use createUser = createUser -- from createUser.fa

	-- add new exports
	let version = "1.0.0"
}
```

## Prelude

Every file sees the **prelude**, the names that the compiler provides. It is the last level of the [name lookup](#name-lookup):

| Kind | Names |
| --- | --- |
| Types | `Boolean`, `String`, `Bytes`, `Integer`, `Float`, `BigInt`, `Percentage`, `Byte`, `Int8` to `Int128`, `UInt8` to `UInt128`, `Float32`, `Float64`, `None`, `Never`, `Void`, `Array`, `Set`, `Map`, `Pool`, `Columnar`, the `Growing` collections, `Tuple`, `Range`, `Task`, `Optional`, `Error`, `Schema`, `Subscription` |
| Type constructors and functions | `Enum`, `Union`, `Errors`, `Fields`, `CStruct`, `CArray`, `Pick`, `Omit`, `Type` and the other [type functions](/types/type-functions) |
| Values | `console` (`console.log`, and `console.error` for the error output), `Math`, `Task` (with `Task.all`, `Task.any` and `Task.settled`) |
| Errors | `IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`, `ParseError`, `Utf8Error`, `Cancelled` |
| Static values | `target` and `configuration` (see [configuration at compile time](/architecture/configuration#configuration-at-compile-time)) |
| Standard library | `std` |
| Standalone files and scripts | `arguments`, the command-line arguments |

`std` is a namespace provided by the compiler, which holds the standard library: `std.concurrency` (`Mutex`, `AtomicInteger`, `AtomicBoolean`, `Ordering`, `Channel`, `ChannelClosed`, `WorkerPool`, `WorkerTimeout`), `std.fs` (`File`, `FileError`), `std.json` (`Json`, `JsonError`), `std.http`, `std.time` (`now`, `sleep`), `std.c` (the C types, see [foreign code](/advanced/foreign-code#c-types)), and so on. In [foreign files](/advanced/foreign-code), the prelude also contains `Pointer` and `CString`. It is used like any namespace:

```fa
use { Channel, Mutex } = std.concurrency
```

Some parts of `std` exist only on some targets: using one on a target that lacks it is a compile error.

`std.fs` is unavailable on `web`, and `std.net` on `web` and `wasm`. Without a C library, `native-freestanding` has no `std.fs`, `std.net` or `std.json`; its `target` value remains `"native"`.

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

In your `package.fa`, you can specify the targets you want to compile for (`web`, `bun`, `node`, `native`, `wasm`). See [configuration](/architecture/configuration#targets).

### The entry point

`application/main.fa` is the entry point of the application (another file can be chosen with the `entry` field of the [configuration](/architecture/configuration#entry)). It exports a function, which is called when the program starts, after the module-level values are initialized:

```fa
-- application/main.fa
export function = () => {
	console.log("Hello, World!")
}
```

The entry function may take the command-line arguments as `arguments: Array<String>`, and may return an exit code as an `Integer`. The errors it can fail with are declared with `throws`, like for any other function:

```fa
-- application/main.fa
export function = (arguments: Array<String>): Integer throws ConfigError => {
	let configuration = try library.loadConfiguration(arguments)
	library.runServer(configuration)
	0
}
```

`arguments` holds the arguments that follow the program name, without the program name itself: `arguments.get(0)` is the first one. A standalone file or a script sees the same array as an implicit `arguments` at its top level. On the `web` target, `arguments` is empty and the exit code is ignored.

Nothing calls the entry function, so, like the top-level statements of a standalone file and the tests, its `mutates` and `invalidates` effects are inferred: it doesn't have to declare them.

The entry function may be asynchronous, to use `await`:

```fa
-- application/main.fa
export async function = (): Integer throws HttpError => {
	let page = try await std.http.get("https://example.com")
	console.log(page.text())
	0
}
```

The exit code of the program is:

- the `Integer` returned by the entry function, or `0` if it returns nothing;
- `1` if an error escapes the entry function: the error is reported on the error output;
- `101` if the program [stops](/values/errors) on a failed runtime check, like an integer overflow.

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

:::implementation
## Implementation details

### Name resolution

- Discovering the names of a root is a scan of the file system, before parsing: it builds one table per folder (siblings, with the contents of `(group)` folders merged in), one for the root entries, one for the dependencies and the prelude. Collisions and invalid names are reported during this scan, on the entries themselves.
- Adding, removing or renaming a file only invalidates the name tables of its folder and, for a root entry, of its root: the files whose resolution used a changed table are checked again.

### Native

- A module-level value is a global with a stable address. A value computed at compile time is emitted as constant data. The others are initialized by one generated function, called before the entry function, in the order defined above; another generated function destroys them in reverse order at normal exit.
- `export mutable` is replaced in place: an assignment destroys the old value and writes the new one at the same address.

- `Json.parse` checks a parsed tree against the expected type's schema, using the same messages as JavaScript, then a generated reader builds the value. Native integers accept all `Int64` values; JavaScript JSON integers stop at ±(2^53 − 1). On `wasm`, the generated JavaScript loader supplies file and clock operations (`node:fs` for files).

### JavaScript

- Each file becomes a JavaScript module. A module-level value is a module-level `let`; the initialization order above is made explicit with one generated initialization function per program rather than relying on the order of `import` evaluation.
- An `export mutable` value is exported as an ES module live binding (`export let x`) with a generated accessor (`export const x$ref = { get v() { … }, set v(value) { … } }`): other files read the binding directly, with no getter call, and write the value through the accessor (`x$ref.v = …`), so no value is wrapped in an object.
:::
