In Fa, every file and every folder is a `namespace`.

# export

To make a value from a module accessible from other modules, one must "export" it.

It is done via the special `export` (or `main`) variable name. It's a variable name that is reserved for this purpose.

Since this is a variable name, you can only have one `export` per module.

You can export anything that you can assign to a variable, like:

- variables
- functions
- types
- namespaces
- singletons
- ...

```ts
export = 12

// but you can add `const` when you want to create a constant expression
export = (x: Integer, y: Integer) => x + y

export = type {
  value: Number
}
```

Syntax proposal, using `main` instead of `export`:

```ts
main = 12

main = (x: Integer, y: Integer) => x + y

main = type {
  value: Number
}
```

## namespace export

It's possible to export multiple values at once by using a namespace.

```ts
export = namespace {
  #private = 121
  public = 2132

  somePublicFunction = (x: Integer, y: Integer) => x + y
  
  SomeType = type {
    value: Number
  }
}
```

If the main export of a module is a type, the first letter of the module name must be uppercase.
Otherwise, the first letter of the module name must be lowercase.

# import

Fa's import system relies on the "use" keyword and relies on the file system to find the module.

Unlike Typescript, you don't have to explicitly import a module. It's available by default if you follow the file system structure. You can "use" values from a module to extract specific values into your scope.

The type system rules are the following:

- A directory is considered a namespace,
- A directory with parentheses is ignored, and all its files can be accessed directly,
- A file have one and only one main export,
- (to validate) A namespace can have a "main" value, and so a folder can have a "main.fa" file.
