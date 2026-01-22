# Introduction

## One language, every target

Fa is a universal programming language designed to replace the divide between fast, safe and productive development.

It brings the ergonomics of TypeScript together with the performance and reliability of Rust.

Fa can compile either to JavaScript (for the web), Rust, or WebAssembly using the Rust backend.

## Core features

### Simple, safe memory management

Fa takes a unique approach to memory management. It's inspired by Rust's model where the compiler enforces memory safety. Like Rust, Fa is very fast and 100% memory-safe. But, unlike Rust, the compiler does not get on your way, and the language stays very simple.

For example, Fa does not have:

- pointers or smart pointers (Box, Cow, Rc, RefCell, Cell, Weak, etc.)
- allocators
- borrowing mechanisms

Instead of manual allocators, Fa uses collections (arrays, sets, maps, bags, columnars). Each collection uses the most optimized allocator for its use case.

In Fa, you will not ask yourself *"When should I free memory?"* nor *"What allocator should I use?"* but rather *"Where should I put this object?"*.

### Built-in reactivity

The rise of frontend development in the web has led to many new frameworks emerging, trying to solve the issue of reactivity.

Most of them, written in Javascript or Typescript, had to hack the language itself:

- React introduces an heavy and sub-optimized virtual DOM with complex hook rules,
- Svelte, Vue, Solid opted for implementing signals and added a compilation layer on top of the language,
- all of these frameworks had to invent their own template language, like JSX.

**This is because programming languages were not designed for frontend development.**

But what if this extra compilation step wasn't necessary? What if you could naturally rely on your programming language to build your UI?

Fa's reactivity is a core feature of the type system.

### Metaprogramming

Metaprogramming brings powerful features to the language, such as:

- Performance: running code at compile time.
- Reflection: inspect and modify the structure of your program at runtime.
- Code generation: generate code based on input data or other code.

Because Typescript is built as a superset of Javascript, it misses all these great features.

On the other hand, languages like Rust had to invent their own syntax to build and run macros.

In Fa, you can simply call any pure function at compile time using the `@` prefix operator.

## Start here

This documentation will walk you through the language, its philosophy, and the
tools around it.

If you want one language for everything you build, start here.
