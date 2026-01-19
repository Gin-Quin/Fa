# Fa
Fa is a programming language that aims to be as fast as low-level languages like C++ and Rust and as easy to use as high-level languages like Python and JavaScript.

## Status

Fa is in active development. Expect breaking changes.

## Prerequisites

To test Fa locally, you need Rust installed. Install it with `rustup` from https://rustup.rs.

## Build the parser

The parser project lives in `packages/parser` and builds as the `fa-parser` crate.

From the repository root:

```bash
cargo build -p fa-parser
```

## Run the parser

The parser binary expects a file path and prints the parsed tree.

```bash
cargo run -p fa-parser -- path/to/file.fa
```

## Hello world

```fa
console.log("Hello, world")
```
