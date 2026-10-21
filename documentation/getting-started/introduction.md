# Introduction

:::caution
This documentation is a work in progress, and Fa is in active development.
:::

## One language, every target

Fa is a general-purpose programming language that removes the choice between *fast*, *safe* and *productive*.

It brings the ergonomics of TypeScript together with the performance and reliability of Rust, and compiles to JavaScript, to native code, or to WebAssembly.

## Core features

### Simple, safe memory management

Fa is memory-safe, with no garbage collector. The compiler enforces safety, as in Rust — but the model is small enough to learn in an afternoon.

Fa has no:

- pointers or smart pointers (`Box`, `Cow`, `Rc`, `RefCell`, `Cell`, `Weak`),
- allocators,
- borrow checker or lifetime annotations.

Instead, objects live on the stack, and collections own the heap. Each collection uses the allocation strategy suited to it, so you never choose an allocator or call `free`.

The question in Fa is not *"when should I free this?"* but *"where should this object live?"*.

Two ideas make this work: [**owned references**](../advanced/owned-references), which point into a collection and are tracked by the type system, and [**relations**](../advanced/relations), weak references the compiler keeps consistent on both sides.

Read more in [memory management](../advanced/memory-management).

### Explicit error handling

There is no `throw`, no `try` and no `catch`. A function that can fail says so in its return type and returns the error as a value:

```fa
function loadScore = (): Int | ApiError => ...

let score = loadScore()!
```

An error cannot be ignored, and cannot be silently caught somewhere up the stack.

Read more in [errors](../values/errors).

### Built-in reactivity

Frontend frameworks have spent a decade working around languages that were not designed for user interfaces:

- React added a virtual DOM and rules about hooks,
- Svelte, Vue and Solid added signals and a compilation step,
- all of them invented a template language.

**These are workarounds for something the language should provide.** In Fa, reactivity is part of the type system:

```fa
reactive count = 0
derived doubled = count * 2

effect {
  console.log("count is {count}")
}
```

Read more in [reactivity](../advanced/reactivity).

### Metaprogramming

Fa evaluates Fa at compile time, using the same syntax you write at runtime. No macro language, no code generator:

```fa
static routes = @readDirectory("../routes")
let result = @add(2, 4) -- compiled to `let result = 6`
```

This gives you compile-time computation, reflection over types, and code generation from data you already have.

TypeScript cannot do this, because it is a superset of JavaScript. Rust can, but only through a separate macro language.

Read more in [metaprogramming](../advanced/metaprogramming).

### Consistent syntax

Everything in Fa is declared the same way:

```
<keyword> <name>[: <type>] = <value>
```

Functions are values, types are values, and generics are ordinary calls. There is one way to declare something, one way to apply a parameterised thing, and one way to write a function body.

## Start here

- [Installation](./installation)
- [Syntax basics](./syntax-basics) — comments, statements, naming
- [Hello world](../guides/hello-world)
- [Declaring values](../values/declaring-values)
- [Language reference](../reference/language-reference) — the whole language on one page

If you already know another language, the [Fa for…](../fa-for/fa-for-typescript-developers) guides map what you know onto Fa.
