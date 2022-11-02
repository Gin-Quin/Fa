# export

To make a value from a module accessible from other modules, one must "export" it.

It is done via the `export` keyword. You can export:

- variables
- functions
- types

```coffee
# `let` keyword is not necessary
export x = 12

# but you can add `const` when you want to create a constant expression
export const add(x: Integer, y: Integer) => x + y

export type Foo =
  value: Number
```

## main export

You can export a `main` value, which is the export that will mostly be used from your module.

Usually, a module always has a `main` export, along with other secondary exports.

When imported, the `main` exported will the take the module file name.

```coffee
export main = 2132
export const main = 64351
export type main =
  value: Number
```

If the main export of a module is a type, the first letter of the module name must be uppercase.
Otherwise, the first letter of the module name must be lowercase.

# import
Fa has two syntaxes to import values:
- main module import (import the main exported value)
- module extraction (import specific fields of the given module)

Imports have the same syntax as variable declarations with `let` or `import`.

When importing a module, you just have to import the path of the. The file extension is mandatory.

```coffee
# main import (`myModule` is imported)
import ./path/to/myModule.fa

# main import (`myModule`, `submoduleA` and `submoduleB` are imported)
import ./path/to/myModule.fa >>
  main
  submoduleA
  submoduleB

# main import (`myAwesomeModule`, `A` and `submoduleB` are imported)
import ./path/to/myModule.fa >>
  main as myAwesomeModule
  submoduleA as A
  submoduleB
```


## Directory as module

It is also possible to import a directory. In this case:

- The `main.fa` file will be treated as the main module export
  - if `main.fa` does not exist the first file matching the `main.*` expression will be treated as the main module
- All the files of the directory will be treated as other exported values
