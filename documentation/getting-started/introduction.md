# Introduction

:::caution
This documentation is a work in progress, and Fa is in active development. Expect breaking changes.
:::

Fa is a programming language designed for **both AI agents and humans**.

Its syntax sits between TypeScript and Rust: the ergonomics of TypeScript, with the performance and the guarantees of Rust.

## One language, every target

Fa is a universal language: write your code once, then compile it to **JavaScript** (for the browser, or for the server with Bun, Node.js or Deno), **native** code, **WebAssembly** or **Rust** source code. See [targets](/architecture/packages#targets).

The same language can be used for tooling, application development and game development.

## Built for AI agents

An agent writes code in a loop: write, compile, read the errors, fix. Fa is designed to make this loop as short and reliable as possible.

- **100% type safety.** Like Rust, Fa has no escape hatch: no `any`, no unchecked casts, no implicit conversions. If it compiles, it works.
- **Effects in the syntax.** Inspired by the [Effect](https://effect.website) library, a function signature tells everything the function can do: what it returns, how it can fail (`throws`), what it needs to run (`requires`) and what it modifies (`mutates`, `invalidates`). This is part of the language, not a library.
- **Clear error messages.** Diagnostics explain what went wrong, where, and how to fix it.
- **Simple, unambiguous syntax.** Every construct has one meaning and, as much as possible, one way to be written. There are no truthy or falsy values, no implicit conversions and no hidden rules.
- **Documentation in code.** Doc comments are attached to the declarations they describe, and are available to the compiler, the language server and the tooling.

## Built for humans

- **Code that reads naturally.** Fa favors words over symbols: `and`, `or`, `not`, `is`, `then`.
- **One language for everything.** No need to switch languages between your web frontend, your server, your tools and your game engine.

## Core features

### Simple, safe memory management

Fa is 100% memory-safe, with no garbage collector. The compiler enforces safety, as in Rust, but the model is small enough to learn in an afternoon.

Fa has no:

- pointers or smart pointers (`Box`, `Cow`, `Rc`, `RefCell`, `Cell`, `Weak`),
- manual allocators,
- borrow checker or lifetime annotations.

Instead, values belong to a scope or to a collection (arrays, sets, maps, bags, columnars), and are destroyed when it ends. Each collection uses the allocation strategy suited to its use case, so you never choose an allocator or call `free`.

In Fa, you will not ask yourself *"When should I free memory?"* nor *"What allocator should I use?"* but rather *"Where should I put this object?"*.

Two concepts make this possible:

- **Where data is stored is part of the type system.** A reference to an element of a collection has a type like `Person in persons`. The compiler tracks the operations that move or remove elements: after one of them, a reference must be checked or set again before use. See [owned references](/advanced/owned-references).
- **Safe, explicit relations instead of weak pointers.** A field that references another object names its collection (`bestFriend: (Person in people)?`). The compiler keeps both sides of the relation in sync, and deleting the target updates the field: a relation never dangles. See [relations](/advanced/relations).

Read more in [memory management](/advanced/memory-management).

### Typed errors

There are no exceptions. A function that can fail declares its errors in its signature with `throws`, and the caller either propagates them with `try` or handles them with `catch`:

```fa
type ApiError = Errors {
	NotFound = { id: String }
	Timeout
}

function loadScore = (id: String): Integer throws ApiError => {
	let response = try fetchScore(id) -- propagates the error
	response.score
}

let score = loadScore("42") catch error => 0 -- handles it
```

An error cannot be ignored, and cannot be silently caught somewhere up the stack. Read more in [errors](/values/errors).

The same idea applies to dependencies: a function declares what it needs with `requires`, and the caller provides it with `provide`. See [requirements](/advanced/requirements).

### Built-in reactivity

The rise of frontend development has led to many frameworks trying to solve the issue of reactivity. Most of them, written in JavaScript or TypeScript, had to work around the language:

- React added a virtual DOM and complex rules about hooks,
- Svelte, Vue and Solid added signals and a compilation step on top of the language,
- all of them invented their own template language, like JSX.

**This is because programming languages were not designed for user interfaces.** In Fa, reactivity is part of the type system:

```fa
reactive count = 0
derived doubled = count * 2

count = 5 -- `doubled` is now 10
```

Read more in [reactivity](/advanced/reactivity).

### Metaprogramming

Fa evaluates Fa at compile time, with the same syntax you write at runtime. No macro language, no code generator:

```fa
static routes = @readDirectory("routes")
let result = @add(2, 4) -- compiled to `let result = 6`
```

This gives you compile-time computation, reflection over types, and code generation from data you already have.

TypeScript cannot do this, because it is a superset of JavaScript. Rust can, but only through a separate macro language.

In Fa, `@` marks a compile-time operation, and any pure function can be called at compile time. Read more in [metaprogramming](/advanced/metaprogramming).

### Consistent syntax

Every declaration in Fa has the same shape:

```
<keyword> <name>[(<generic parameters>)][: <type>] = <value>
```

```fa
let answer: Integer = 42
mutable count = 0
type Point(NumberType: Integer | Float) = { x: NumberType, y: NumberType }
function first(Item) = (items: Array(Item)): Item? => items.get(0)
```

Functions are declared like values, generic parameters are applied like calls (`Array(Integer)`, `Point(Float)`), and there is one way to write a function. A name without a keyword is never a declaration: `count = 1` assigns an existing mutable value. See [syntax basics](/getting-started/syntax-basics).

## Start here

- [Installation](/getting-started/installation)
- [Syntax basics](/getting-started/syntax-basics): comments, newlines, naming, declarations
- [Hello world](/guides/hello-world)
- [Declaring values](/values/declaring-values)
- [Language reference](/reference/language-reference): the whole language on one page

If you already know another language, the [Fa for…](/fa-for/fa-for-typescript-developers) guides map what you know onto Fa.
