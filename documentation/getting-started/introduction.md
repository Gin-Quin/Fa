# Introduction

## One language, every target

Fa is a universal programming language designed to replace the divide between
fast, safe systems code and productive application development.

It brings the ergonomics of TypeScript together with the performance and
reliability of Rust.

Write once and target web apps, tooling, servers, and embedded systems without
switching languages or paradigms.

Fa can compile either to JavaScript (for the web), Rust, or WebAssembly using
the Rust backend.

## Performance without a GC

The core idea is simple: one coherent language, one mental model.

Fa favors clarity and correctness while still compiling down to efficient
low-level code.

There is no runtime garbage collector, no hidden costs, and no unpredictable
pauses.

## Simple, safe memory management

Memory management relies on a small set of simple yet powerful concepts, with
the compiler enforcing 100% type and memory safety.

Instead of manual allocators, Fa uses collections (arrays, sets, maps, bags)
that act as arenas with a bump allocator under the hood.

## Built for the future

Fa is also built for the future.

Reactivity is a first-class language feature, so building responsive,
data-driven systems is predictable and type-safe.

Metaprogramming is part of the language, which means you can generate and
transform programs without jumping into a separate DSL or a maze of syntax
trees.

## Start here

This documentation will walk you through the language, its philosophy, and the
tools around it.

If you want one language for everything you build, start here.
