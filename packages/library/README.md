# Library

This is the core library that implements the Fa language.

It's a collection of Zig and Typescript modules that are used to implement the Fa language depending on the target platform.

Very few Typescript modules will be necessary as the Fa language is very close to Javascript.

## Rust modules

The necessary Rust modules to implement the Fa language are:

- An http server: using [Ntex](https://ntex.rs/docs/whatis) (fastest) or [Axum](https://docs.rs/axum/latest/axum/) (most used)
- An http client: [reqwest](https://docs.rs/reqwest/latest/reqwest/) is the standard
- A websocket client: [tokio-tungstenite](https://github.com/snapview/tungstenite-rs) seems reliable and quite performant
- A string module with rope,
- An colorblind async filesystem module: see [async-std](https://github.com/async-rs/async-std) or **tokio**,
- A child process module,
- A console module,

Later:

- A gRPC client (ntex-grpc if ntex is used for the server)
- Coroutines / generators (they actually are the same thing, already implemented in Rust standard library)
- A path module,
- A thread / thread pool module (not sure it's necessary with the coroutines),
- A temporal module,
- A crypto module,
- ...

## Multi-platform modules

Fa can be used on multiple platforms:

- Browser,
- Bun (Typescript),
- Rust,
- Wasm (via Rust, C or Zig).

To create a multi-platform module, you have to create a folder with a `.fa` extension. Then, inside the folder, you can have the following files:

- `interface.fa`: mandatory, a Fa file without the implementation.
- `module.ts`: the code used for Browser, Bun and Node platforms.
- `module.bun.ts`: the code used for the Bun platform only (overwrite `module.ts` if it exists when compiling for the Bun platform).
- `module.browser.ts`: the code used for the browser only (overwrite `module.ts` if it exists when compiling for the browser).
- `module.rs`: the code used to compile Fa to native (via Rust) or Wasm.
