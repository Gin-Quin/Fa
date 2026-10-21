# Modules

Fa organises code by the file system rather than by import statements. Four principles cover the whole system:

1. **Single export** — each file exports exactly one entity.
2. **Implicit namespaces** — each folder is a namespace.
3. **Implicit imports** — sibling files and root files need no declaration.
4. **Grouping without namespacing** — a folder written `(likeThis)` groups files without adding a namespace.

## Importing

Code becomes available based on where it lives.

Take this structure:

```
library/
  ├── myRootFile.fa
  └── myFolder/
      ├── myFile.fa        <-- we are in this file
      ├── myOtherFile.fa
      └── mySubFolder/
          └── mySubFile.fa
```

In JavaScript you would write:

```js
import myOtherFile from "./myOtherFile.js"
import mySubFile from "./mySubFolder/mySubFile.js"
import myRootFile from "../myRootFile.js"

console.log(myOtherFile, mySubFile, myRootFile)
```

In Fa you write nothing:

```fa
console.log(
  myOtherFile            -- sibling file
  mySubFolder.mySubFile  -- through the sibling folder's namespace
  myRootFile             -- root file, available everywhere
)
```

### Rules

- **Sibling files** — available directly, by filename without the extension.
- **Nested files** — available through the folder namespace, `folder.file`.
- **Root files** — available everywhere in the package.
- **Parent files** — not available directly; reach them from the root.

Organise the root of a package into folders, so the global namespace stays small.

:::tip
Ideally your editor colours implicitly available names differently from locally declared ones.
:::

### Extracting from a namespace

`>>` pulls specific names out of a namespace, so you do not have to repeat the path:

```fa
utils >> use { formatDate, validateEmail, parseJson }

formatDate(now())
validateEmail("user@example.com")
```

It works with nested namespaces:

```fa
ui.components >> use { Button, Card, Modal }

Button({ label = "Click me" })
```

And composes with ordinary namespace access:

```fa
ui >> use { Button, Card }

ui.Modal.show("Hello World")
```

To rename something, declare a reference:

```fa
use Component = react.Component
```

See [extract and insert](../operations/extract-insert).

## Exporting

Every file exports exactly one entity, named after the file.

### A value

```fa
export = 42
```

```fa
-- if the file above is answer.fa
console.log(answer) -- 42
```

### A function

```fa
export function = (x: Int, y: Int): Int => x + y
```

### A type

```fa
export type = {
  name: String
  age: Int
  isActive: Boolean
}
```

```fa
-- if the file above is User.fa
function createUser = (data: User): User => ...
```

### A namespace

```fa
export namespace = {
  defaultValue = 100

  function increment = (x: Int): Int => x + 1
  function decrement = (x: Int): Int => x - 1

  namespace constants = {
    pi = 3.14159
    maximumUsers = 1000
  }
}
```

```fa
-- if the file above is numbers.fa
console.log(numbers.defaultValue)          -- 100
console.log(numbers.increment(5))          -- 6
console.log(numbers.constants.pi)          -- 3.14159
```

A member prefixed with `#` is private to the namespace and is not exported.

### With a type annotation

```fa
export: Int = 42
export function: (Int) => Int = increment
```

## Parenthesised folders

A folder named `(likeThis)` groups files without creating a namespace:

```
library/
  └── (utils)/
      ├── format.fa
      └── validate.fa
```

Its files are reached directly:

```fa
console.log(format("Hello"))
console.log(validate("email@example.com"))
```

## Re-exporting

Aggregate several files into one namespace:

```fa
-- in index.fa
export namespace = {
  User = User             -- from User.fa
  createUser = createUser -- from createUser.fa

  version = "1.0.0"
}
```

## Publishing a library

Everything under `library/` is visible to the outside world.

Say you are building a library called `farm`:

```
farm/
  ├── package.fab       <- configuration
  ├── dependencies.fa   <- dependencies
  └── library/
      ├── (animals)/       <-- parenthesised: not a namespace
      │   ├── cows.fa
      │   ├── cows.test.fa <-- tests sit next to the code
      │   └── pigs.fa
      └── machines/
          ├── tractor.fa
          ├── tractor.test.fa
          ├── harvester.fa
          └── irrigator.fa
```

The library exports:

```fa
use cows = farm.cows
use pigs = farm.pigs
use tractor = farm.machines.tractor

-- or, extracting several at once
farm >> use {
  cows
  pigs
  machines { tractor, harvester, irrigator }
}
```

:::tip
These are exactly the namespaces the library's own files see, so there is no separate public API to declare.
:::

Fa libraries are shared **unbundled and uncompiled**: push your source to a Git repository along with its configuration file. See [deploying a library](../guides/deploy-a-library).

## Depending on a library

Declare dependencies in `dependencies.fa`, at the root of your package, with an ordinary `use` declaration:

```fa
-- the library `farm` from GitHub, major version 4
use farm = "github:farmers/farm@4"

-- the same library under a different name
use farming = "github:farmers/farm@4"
```

Then extract from it wherever you need it:

```fa
farm >> use { cows, pigs }
```

Version ranges follow semantic versioning: `@4` means the latest `4.x.y`, `@4.2` the latest `4.2.x`, and `@4.2.1` exactly that version.

## Applications

Application files live in `application/`, next to `library/`:

```
my-app/
  ├── package.fab
  ├── library/
  │   └── math.fa
  └── application/
      └── main.fa
```

`application/main.fa` is the entry point. It exports a function, which is called when the program starts:

```fa
-- application/main.fa
export function = () => {
  console.log("Hello, World!")
}
```

The entry function may take the command-line arguments, and may return an error or an exit code:

```fa
export function = (arguments: Array(String)): Int | ConfigError => {
  let config = loadConfig(arguments)!
  runServer(config)
  return 0
}
```

Choose your targets in `package.fab` — see [configuration](./configuration).
