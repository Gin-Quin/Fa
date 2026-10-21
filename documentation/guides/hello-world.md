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

- a function is declared with `function`, like any other value is declared with its keyword,
- `{name}` interpolates: every string in Fa is a template string (write `{{` and `}}` for literal braces),
- the return type could have been left out, because the body is a single expression.

## Reading an argument

Standalone files receive their command-line arguments in `arguments`, an `Array(String)`:

```fa
let name = arguments.get(1) ?? "World"

console.log("Hello, {name}!")
```

```bash
fa run hello.fa Ada
-- Hello, Ada!
```

The compiler cannot know how many arguments there will be, so `arguments.get(1)` returns an optional `String?`, and `??` supplies the fallback. Fa will not let you read a value that might not be there without saying what happens if it is not.

## Growing into a package

When one file is no longer enough:

```bash
fa new package hello
```

which gives you:

```
hello/
├── package.fa
├── library/
│   └── greet.fa
└── application/
    └── main.fa
```

`package.fa` is the configuration of the package. It is a regular Fa file that exports an object:

```fa
-- package.fa
export = {
	name = "hello"
	version = "0.1.0"
	targets = ["web", "bun", "native"]
}
```

Each file exports exactly one value, named after the file:

```fa
-- library/greet.fa
export function = (name: String): String => "Hello, {name}!"
```

The entry point is the function exported by `application/main.fa`:

```fa
-- application/main.fa
export function = () => {
	console.log(library.greet("World"))
}
```

There is no import statement. `greet.fa` lives in `library/`, and the application reaches the library through the `library` namespace, so `library.greet` is in scope. See [modules](/architecture/modules).

Run it:

```bash
fa develop
```

## Adding a test

Tests live next to the code they test, in files ending in `.test.fa`:

```fa
-- library/greet.test.fa
test "greets by name" {
	assert greet("Ada") == "Hello, Ada!"
}
```

`greet.test.fa` sits next to `greet.fa`, so `greet` is in scope.

```bash
fa test
```

See [testing](/architecture/testing).

## Building it

```bash
fa build
```

This compiles for the targets listed in `package.fa`. To pick one:

```bash
fa build --target=native
```

## Where to go next

- [Syntax basics](/getting-started/syntax-basics): the mechanical rules
- [Declaring values](/values/declaring-values): `let`, `mutable`, `use`
- [Errors](/values/errors): how Fa handles failure
- [Memory management](/advanced/memory-management): where your objects live
- [Build your application](/guides/build-your-application): a longer walkthrough
