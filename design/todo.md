# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

## Bugs

- Analyzer issues from the Goose ports (`c is ")"` on a `Character` always true, `while let`, `(A, B)` tuple types, guards forgotten after a push, narrowing gaps, misleading W-NUM-1/W-ERR-1): *assigned to the analyzer-gaps engineer*.
- JavaScript: `fa build --target=bun` of a 20,000-arm `when` makes Bun fail with "Maximum call stack size exceeded" (deeply nested `else if`): emit a flat `switch` or a lookup for long `when`s.
- Native: a 50,000-line program still takes 1.4 GB to build (huge top-level function): split top-level code into several functions, and step 6 of the parallel pipeline (one LLVM module per unit).
- The names of `std.concurrency` are visible without `use { … } = std.concurrency` (the concurrency engineer's shortcut): they must only be visible through the `use`.

## Performance (Goose suite, native, vs Rust)

- Done by the fifth native shift (blur, tree, records at Rust's speed; strings flagged ASCII; slicing): see `benchmarks/README.md`. Left: `calc` and `sexp` (about 2x Rust: allocations of union boxes and small strings — a small-string optimization, and pools for boxes of one statement), `strlist` (1.5x: one `codePoints()` array per word, and one heap string per word), cached grapheme offsets for non-ASCII strings; a cheap byte/character-code access to a `String` is still missing from the library (a language decision).
- Bun: push and sieve of the Goose suite exceed 4 GB; the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun.

## Features still missing (from the engineers' reports)

- Native: relations, columnar per-field layout, reactivity, compile-time maps/sets/enums/ranges (*assigned to the LLVM fourth shift*); foreign code: `CArray` values, `CStruct` by value off x86-64, `Pointer.allocate`/`free`, WebAssembly imports, `build.cLibrary`; JS foreign code: `destroy`, structs by value. Concurrency: `parallel()` views, `WorkerPool`, `for parallel` over growing collections and columnars, requirements and `mutable use` in parallel bodies.
- Analyzer: implied effects of relation writes and deletes (§8.6, D47), relations across files, `@import`, `@extern` (*foreign code*), `@for` bodies analyzed per item, `Schema`/`TypeFromSchema` in the prelude, `std.time`.
- Tooling: hot reloading by module swap (D43; `fa develop` restarts today), the language server resolving dependencies, `fa build` without `--target` building every configured target, `fa test` running test files in separate workers, `DependencySource.generate`, path dependencies outside a workspace (decided: allowed).
- JavaScript: `mutable use` of a whole primitive local, references to primitives returned by calls other than `get`/`getOrInsert`, variadic parameters with requirements.
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.
