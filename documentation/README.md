# About Fa

This directory holds the specification and the user documentation for the Fa language. It is the source of truth: where the documentation and the implementation disagree, the documentation is the intent.

## Reading order

- [Introduction](./getting-started/introduction) — what Fa is and why
- [Syntax basics](./getting-started/syntax-basics) — comments, statements, naming
- [Language reference](./reference/language-reference) — the whole language on one page

## What Fa is for

Fa's main advantages are:

- mixing interpreted and compiled code — *TypeScript at the speed of Rust*,
- a sound and simple memory model, built on collections, copyable objects, references, **owned references** and **relations**. Of these, owned references are Fa's own contribution.

The language aims to be:

- **readable**, so that code is understood before it is run,
- **minimalistic**, with one way to express each thing,
- **interpreted or natively compiled**, for fast iteration and fast execution,
- **close to its targets**, so that the generated JavaScript, Rust and WebAssembly stay clean. Fa's goal is not to reinvent what those ecosystems already do well.

And beyond that: to make **creative programming** enjoyable.

## On graphics and user interfaces

Most visual software falls into two groups:

- **interactive documents** — a user interface for creating or manipulating data, where most of the work is text, input handling and responsive layout,
- **intense graphical applications**, such as games, which need a rendering engine and physics.

The easiest way to build an interactive document today is web technology, and targeting it with Fa is sensible: much of what runs on the web now runs well beyond it.

So should Fa ship its own high-performance graphics library, tied to the language?

The answer is no, for two reasons:

- graphics APIs are mid-revolution, with Vulkan and Metal displacing OpenGL. It is better to wait for the next good universal framework than to bet on the current moment.
- there are many graphics frameworks, each specialised. Rather than spend enormous effort on a standard one, Fa can spend almost none on easy bindings, and let everyone use the framework they prefer.

Beyond that, the future of application development looks less like those frameworks and more like a webview:

- every client device has one, since it needs one to browse the web,
- a webview is less and less "web" and more and more a universal view; you can build any application in one,
- so an application is JavaScript or WebAssembly, connected directly to that view,
- and persistence happens through a remote server, or a local one in the Electron style.

The near future is JavaScript, and the one after that is WebAssembly. Those are the two targets to focus on — plus Rust, for its power and its ecosystem.

A graphics framework may come later, once WebAssembly is mature: Fa → WebAssembly → view API, possibly reusing a Rust framework.

## For contributors

See `AGENTS.md` at the repository root for build commands, code style and the parser architecture.

When changing the language, change the documentation in this directory first. Every code example is written in the canonical syntax described in [syntax basics](./getting-started/syntax-basics); keep new examples consistent with it.
