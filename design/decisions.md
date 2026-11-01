# Implementation decisions

Points the documentation left open or contradictory, and how the compiler resolves them. Each entry says where the question came from. When a decision changes what a user writes, the documentation is updated to match.

## Programs and the command line

- **Arguments of a standalone file**: the top level of a standalone file sees `arguments: Array(String)`, where index `0` is the program (the file) and index `1` the first user argument, as in the hello world guide. An exported entry function receives only the user arguments, as in the architecture overview. (benchmarks)
- **`fa build` output**: `fa build <file.fa | package> --target=<target> [--output=<directory>]`; the default directory is `build/<target>/`. `native` writes an executable, `wasm` a `.wasm` file and a JavaScript loader, `bun`/`node`/`deno`/`web` a `.js` module. (benchmarks)

## Standard library

- **Parsing a number** is `Integer.parse(text)` (`throws ParseError`), as on the type conversion page. `Integer.fromString` in the guides was stale and is renamed. (benchmarks)

## Proofs

- **Index proofs**: a loop `for i in a..b` proves `a <= i < b`; with `b` being `array.length` (or `array.length - k`) and `a >= 0` provable, `array[i]` (and `array[i + k]` within bounds) needs no check. A `while i < array.length` guard proves the same inside the loop body until `i` or the array is mutated. (benchmarks)
