# Hello world

## A standalone file

The shortest Fa program is one line. Put it in `hello.fa`:

```fa
console.log("Hello, World!")
```

and run it:

```bash
fa run hello.fa
```

In a standalone file, top-level statements run in order, top to bottom.

## Adding a function

```fa
function greet = (name: String): String => "Hello, {name}!"

console.log(greet("World"))
```

Three things to notice:

- a function is a value, declared like any other value,
- `"{name}"` interpolates — every string in Fa is a template string,
- the return type could have been left out, because the body is a single expression.

## Reading an argument

Standalone files receive their command-line arguments in `arguments`:

```fa
let name = arguments[1] ?? "World"

console.log("Hello, {name}!")
```

```bash
fa run hello.fa Ada
-- Hello, Ada!
```

`arguments[1]` returns `String | IndexError`, because the compiler cannot know how many arguments there will be. `??` supplies the fallback. Fa will not let you read a value that might not be there without saying what happens if it is not.

## Growing into a package

When one file is no longer enough:

```bash
fa new package
```

which gives you:

```
myPackage/
├── package.fab
├── library/
│   └── greet.fa
└── application/
    └── main.fa
```

```fa
-- library/greet.fa
export function = (name: String): String => "Hello, {name}!"
```

```fa
-- application/main.fa
export function = () => {
  console.log(greet("World"))
}
```

There is no import statement: `greet.fa` lives in `library/`, so `greet` is in scope. See [modules](../architecture/modules).

Run it:

```bash
fa develop
```

## Adding a test

```fa
-- library/greet.test.fa
export namespace = {
  test "greets by name" {
    assert(greet("Ada") == "Hello, Ada!")
  }
}
```

```bash
fa test
```

See [testing](../architecture/testing).

## Building it

```bash
fa build
```

This compiles for the platforms listed in `package.fab`. To pick one:

```bash
fa build --platform native
```

## Where to go next

- [Syntax basics](../getting-started/syntax-basics) — the mechanical rules
- [Declaring values](../values/declaring-values) — `let`, `mutable`, `use`
- [Errors](../values/errors) — how Fa handles failure
- [Memory management](../advanced/memory-management) — where your objects live
- [Build your application](./build-your-application) — a longer walkthrough
