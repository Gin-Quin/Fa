# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

## Bugs

- The names of `std.concurrency` are visible without `use { … } = std.concurrency` (the concurrency engineer's shortcut): they must only be visible through the `use`.

## Features still missing (from the engineers' reports)

- Native: relations, columnar per-field layout, reactivity, compile-time maps/sets/enums/ranges (*assigned to the LLVM fourth shift*); foreign code: `CArray` values, `CStruct` by value off x86-64, `Pointer.allocate`/`free`, WebAssembly imports, `build.cLibrary`; JS foreign code: `destroy`, structs by value. Concurrency: `parallel()` views, `WorkerPool`, `for parallel` over growing collections and columnars, requirements and `mutable use` in parallel bodies.
- Analyzer: implied effects of relation writes and deletes (§8.6, D47), relations across files, `@import`, `@extern` (*foreign code*), `@for` bodies analyzed per item, `Schema`/`TypeFromSchema` in the prelude, `std.time`.
- Tooling: hot reloading by module swap (D43; `fa develop` restarts today), the language server resolving dependencies, `fa build` without `--target` building every configured target, `fa test` running test files in separate workers, `DependencySource.generate`, path dependencies outside a workspace (decided: allowed).
- JavaScript: `mutable use` of a whole primitive local, references to primitives returned by calls other than `get`/`getOrInsert`, variadic parameters with requirements.
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.
