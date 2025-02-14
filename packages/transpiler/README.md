
# Transpiler

Fa transpiles to two languages: Javascript and Rust. Via these two languages, Fa intends to be an "universal language" as it can be used in any environment.

By transpiling to Javascript code, Fa can be used:

- in the browser,
- in Progressive Web Applications,
- in serverless platforms like Cloudflare or Vercel,
- as a Bun server.

Transpiling to Javascript also leverages the ability to have **hot reloading** during development (unlike other low-level languages like Rust or Zig).

By transpiling to Rust, Fa can be used:

- as a WebAssembly library or binary,
- for native tooling and applications,
- for the fastest web servers.

Since Fa can transpile to Javascript and Javascript is a **script language**, we use this feature to run code at compile-time. This happens when resolving type functions, and calling compile-time functions or computed values. To achieve this, Fa communicates privately with a Bun instance.

Transpiling to Javascript can happen at two places:

1. to generate compile-time Javascript,
2. to generate run-time Javascript.
