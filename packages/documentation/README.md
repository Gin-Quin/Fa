# Fa

**A programming language designed for both AI agents and humans.**

**If it compiles, it works.**

Agents get a language they can't get wrong without the compiler noticing: strict types, typed errors, requirements and mutations, clear diagnostics and an unambiguous grammar.
Humans get code that reads almost like natural language, and that runs everywhere: web, server, native, WebAssembly.

Syntax-wise, Fa sits between Rust and TypeScript.

This package is the Fa documentation site. Its pages are generated from the Markdown files of the top-level [`documentation/`](../../documentation) directory.

---

## Built for AI agents

- **100% type safety**: no `any`, no unchecked casts, no `null` surprises. The compiler is the feedback loop.
- **Effects in the syntax, "à la Effect"**: every signature tells what a function can do.
  - Typed errors: `throws`, propagated with `try`, handled with `catch`.
  - Typed requirements: `requires`, given with `provide`.
  - Typed mutations: `mutates` and `invalidates`.
- **Clear error messages**: diagnostics explain what went wrong, where, and how to fix it.
- **Simple, unambiguous syntax**: one way to write each thing.
- **Documentation in code**: a comment right above a declaration is its documentation.

---

## Built for humans

- **Code that reads naturally**: words and simple operators over symbols.
- **One language, many platforms**: compile to JavaScript (browser, Bun, Node.js, Deno), native, WebAssembly or Rust source code.
- **Tooling, applications and games** from the same code base.

---

## A new take on memory management

No garbage collector and no borrow checker, yet 100% memory-safe.

- **Where data lives is part of the type**: a reference to an element of `persons` has the type `Person in persons`. After an operation that can move or remove elements, references must be revalidated.
- **Relations instead of weak pointers**: declare relations between fields; the compiler keeps both sides in sync and a relation never dangles.

---

## And more

- **Built-in reactivity**: `reactive` and `derived` values, kept in sync by the language.
- **Metaprogramming**: compile-time operations are marked with `@` and are plain Fa.

---

## Running the site

From this directory:

```bash
bun install
bun run dev   # starts the site and regenerates the routes when the documentation changes
bun run build # produces a static build
```

---

## Status

Fa is in active development. Expect breaking changes.
