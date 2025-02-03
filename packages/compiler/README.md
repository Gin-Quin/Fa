
## Type functions

To handle any code that is called at build time, Fa uses a JavaScript compiler. The JS compiler is:

- Bun if found,
- Node if found and Bun is not found,
- Deno if found and Bun and Node are not found.

The compiler is used to compile the Fa code and generate the JavaScript code.

The Fa compiler is not embedded in the Fa compiler. It must be installed separately as a peer dependency.
