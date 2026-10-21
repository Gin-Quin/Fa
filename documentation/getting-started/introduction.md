# Introduction

:::caution
This documentation is work in progress.
:::

Fa is a programming language designed for **both AI agents and humans**.

Its syntax sits between TypeScript and Rust: the ergonomics of TypeScript, with the performance and the guarantees of Rust.

## One language, every target

Fa is a universal language: write your code once, then compile it to **JavaScript** (for the browser, or for the server with Bun, Node.js or Deno), **native**, **WebAssembly** or **Rust** source code. See [targets](/architecture/packages#targets).

The same language can be used for tooling, application development and game development.

## Built for AI agents

An agent writes code in a loop: write, compile, read the errors, fix. Fa is designed to make this loop as short and reliable as possible.

- **100% type safety.** Like Rust, Fa has no escape hatch: no `any`, no unchecked casts, no implicit conversions. If it compiles, it works.
- **Typed errors and requirements.** Inspired by the [Effect](https://effect.website) library, a function signature tells everything the function can do: what it returns, how it can fail, and what it needs to run. This is part of the language syntax, not a library.
- **Clear error messages.** Diagnostics explain what went wrong, where, and how to fix it.
- **Simple, unambiguous syntax.** Every construct has one meaning and, as much as possible, one way to be written. There are no truthy or falsy values, no implicit conversions and no hidden rules.
- **Documentation in code.** Documentation is part of the language and is available to the compiler, the language server and the tooling.

## Built for humans

- **Code that reads naturally.** Fa favors words over symbols: `and`, `or`, `not`, `is`, `then`.
- **One language for everything.** No need to switch languages between your web frontend, your server, your tools and your game engine.

## Core features

### Simple, safe memory management

Fa has no garbage collector and no borrow checker, yet it is 100% memory-safe.

Instead of manual allocators, Fa uses collections (arrays, sets, maps, bags, columnars). Each collection uses the most optimized allocator for its use case.

In Fa, you will not ask yourself *"When should I free memory?"* nor *"What allocator should I use?"* but rather *"Where should I put this object?"*.

Two concepts make this possible:

- **Where data is stored is part of the type system.** A reference to an element of a collection has a type like `Person in persons`. The compiler tracks mutations: after a collection is mutated, references to its elements must be revalidated. See [owned references](/advanced/owned-references).
- **Safe, explicit relations instead of weak pointers.** Relations between objects are declared in the types, and the compiler keeps both sides in sync. See [relationships](/advanced/relationships).

### Built-in reactivity

The rise of frontend development in the web has led to many new frameworks emerging, trying to solve the issue of reactivity.

Most of them, written in JavaScript or TypeScript, had to hack the language itself:

- React introduces a heavy virtual DOM with complex hook rules,
- Svelte, Vue and Solid opted for implementing signals and added a compilation layer on top of the language,
- all of these frameworks had to invent their own template language, like JSX.

**This is because programming languages were not designed for frontend development.**

But what if this extra compilation step wasn't necessary? What if you could naturally rely on your programming language to build your UI?

In Fa, reactivity is a core feature of the type system. See [reactivity](/advanced/reactivity).

### Metaprogramming

Metaprogramming brings powerful features to the language, such as:

- Performance: running code at compile time.
- Reflection: inspect the structure of your program.
- Code generation: generate code based on input data or other code.

TypeScript, being a superset of JavaScript, misses all these features. Rust, on the other hand, had to invent a separate syntax to write and run macros.

In Fa, you can simply call any pure function at compile time using the `@` prefix operator. See [metaprogramming](/advanced/metaprogramming).

## Start here

This documentation will walk you through the language, its philosophy, and the tools around it.

If you want one language for everything you build, start here.
