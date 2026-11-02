# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

## Bugs

- `Schema.Integer.signed` is `Boolean?` in the prelude (built-in types can't have defaults) while the docs say `Boolean = true`: the docs win; built-in object types must support field defaults (lead's decision).
- Native: pa-10 (a `Set` spread into a variadic parameter sums garbage), ft-1 (calling a `mutates` function value overflows the stack); `std.time`/`std.fs`/`std.json` not lowered natively; `while let` natively only for optional scalars.
- Analyzer: `TypeFromSchema` for named unions, enums, functions, recursive references and inside generic type functions; `@typeToString` prints the declared name instead of the spelling.
- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.
- JavaScript: `fa build --target=bun` of a 20,000-arm `when` makes Bun fail with "Maximum call stack size exceeded" (deeply nested `else if`): emit a flat `switch` or a lookup for long `when`s.
- Native: a 50,000-line program still takes 1.4 GB to build (huge top-level function): split top-level code into several functions, and step 6 of the parallel pipeline (one LLVM module per unit).

## Performance (Goose suite, native, vs Rust)

- blur 31x (bounds-checked loads don't vectorize), strlist 6.7x, sexp 5.7x, tree 3.8x, records 3.5x, calc 3.3x; string indexing is linear in the string's length natively (needs an ASCII/byte-index fast path or cached grapheme offsets); appending one character at a time is slow; a cheap byte/character-code access to a `String` is missing from the library (`codePoints()` allocates). Native string slicing `text[a..b]` is not implemented. To assign after the LLVM fourth shift.
- Bun: push and sieve of the Goose suite exceed 4 GB; the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun.

## Features still missing (from the engineers' reports)

- Native: relations, columnar per-field layout, reactivity, compile-time maps/sets/enums/ranges (*assigned to the LLVM fourth shift*); foreign code: `CArray` values, `CStruct` by value off x86-64, `Pointer.allocate`/`free`, WebAssembly imports, `build.cLibrary`; JS foreign code: `destroy`, structs by value. Concurrency: `parallel()` views, `WorkerPool`, `for parallel` over growing collections and columnars, requirements and `mutable use` in parallel bodies.
- Analyzer: relations across files, `@import`, `@extern` (*foreign code*); implied relation effects for reactivity; `TypeFromSchema` of named unions, enumerations, functions, recursive references and inside generic type functions; JSON default values and named unions; `@typeToString` spellings (D49: op-1, tf-1, rt-1).
- Native: `std.time`, `std.fs`, `std.json`; compile-time named union members (`Schema` values); `while let` over non-scalar optionals.
- Tooling: hot reloading by module swap (D43; `fa develop` restarts today), the language server resolving dependencies, `fa build` without `--target` building every configured target, `fa test` running test files in separate workers, `DependencySource.generate`, path dependencies outside a workspace (decided: allowed).
- JavaScript: `mutable use` of a whole primitive local, references to primitives returned by calls other than `get`/`getOrInsert`, variadic parameters with requirements.
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.
