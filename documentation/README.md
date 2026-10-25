# Fa documentation

This folder contains the specification and the user documentation of the Fa language. Every Markdown file (except this README) is turned into a page of the website in `packages/website`.

The documentation is the **source of truth**: where the documentation and the implementation disagree, the documentation describes the intent, and the implementation has to catch up.

## Reading order

- [Introduction](/getting-started/introduction): what Fa is, and why.
- [Syntax basics](/getting-started/syntax-basics): comments, newlines, naming, declarations.
- [Language reference](/reference/language-reference): the whole language on one page.

## Goals of the language

Fa is a programming language designed for **both AI agents and humans**. Its syntax sits between TypeScript and Rust.

For AI agents:

- **100% type safety**: if it compiles, it works.
- **Effects in the syntax**, "à la Effect": typed errors (`throws`), typed requirements (`requires`) and typed mutations (`mutates`, `invalidates`) are part of every function signature.
- **Clear error messages**, written to be acted upon without guesswork.
- **A simple and unambiguous syntax**: one meaning per construct, one way to write it.
- **Documentation in code**: doc comments are attached to the declarations they describe.

For humans:

- **Code that is easy to read**, almost natural.
- **One language, many platforms**: write once, compile to the web, to server JavaScript runtimes (Bun, Node.js, Deno), to native code, to WebAssembly or to Rust. Fa is suited for tooling, application development and game development.

Memory management:

- **Where data is stored is part of the type system** (`Person in people`): operations that move or remove elements are tracked, and an invalidated reference must be declared again before use.
- **Safe, explicit relations** instead of weak pointers: a reference stored in a field names its collection and has a declared reverse side; the compiler keeps both sides in sync, and cleans them up when an object is deleted.
- No garbage collector, no borrow checker.

## On graphics and user interfaces

Most visual software falls into two groups:

- **interactive documents**: a user interface to create or manipulate data, where most of the work is text, input handling and responsive layout,
- **intense graphical applications**, such as games, which need a 2D/3D rendering engine and physics.

Today, the easiest way to build an interactive document is web technology (JavaScript and a modern framework like Svelte, Vue or React). Targeting it with Fa is sensible: "web" technologies are becoming less and less web-only.

So should Fa ship its own high-performance graphics library, closely tied to the language? The answer is no, for two reasons:

- graphics APIs are in the middle of a revolution, with Vulkan and Metal displacing OpenGL. It is better to wait for the next good universal framework (the next SDL or SFML) than to bet on the current moment,
- there are many graphics frameworks, each with its own specialisation. Rather than spending a lot of effort on a standard Fa framework, Fa can spend almost none on easy bindings, and let everyone use the framework they prefer.

Beyond that, the future of application development looks less like those frameworks and more like a webview:

- every client device has one, since it needs one to browse the web,
- a webview is less and less "web" and more and more a universal view: any application can be built in one,
- so an application is JavaScript or WebAssembly, connected directly to that view,
- and file system access goes through a remote server (web style) or a local one (Electron style).

The near future is JavaScript and the one after is WebAssembly: those are the two targets to focus on, plus Rust for its power and its ecosystem.

A graphics framework may come later, once WebAssembly is mature: Fa → WebAssembly → view API, possibly reusing a Rust framework.

## For contributors

See `AGENTS.md` at the repository root for build commands, code style and the parser architecture.

When changing the language, change the documentation in this folder first, then the implementation.

Writing rules for the documentation:

- every code example uses the canonical syntax described in [syntax basics](/getting-started/syntax-basics): `--` and `---` comments, double-quoted strings, tabs for indentation,
- internal links are absolute site paths without the `.md` extension, like `[errors](/values/errors)`,
- a page lives at `documentation/<folder>/<name>.md` and must be listed in `packages/website/src/lib/docs/navigation.ts`.
