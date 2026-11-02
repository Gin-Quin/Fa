# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

## Bugs

- Async: the loop back-edge yield doesn't apply to the top level of a file or to tests (it broke narrowing of top-level loops): a long top-level loop never yields. Revisit when the native executor lands.
- `fa-interpreter` `the_budget_and_the_depth_are_bounded` fails under `ulimit -v 4000000` (the evaluation stack shrinks): make the test independent of the address-space limit.
- `Schema.Integer.signed` is `Boolean?` in the prelude (built-in types can't have defaults) while the docs say `Boolean = true`: the docs win; built-in object types must support field defaults (lead's decision).
- Native: pa-10 (a `Set` spread into a variadic parameter sums garbage), ft-1 (calling a `mutates` function value overflows the stack); `std.time`/`std.fs`/`std.json` not lowered natively; `while let` natively only for optional scalars.
- Analyzer: `TypeFromSchema` for named unions, enums, functions, recursive references and inside generic type functions; `@typeToString` prints the declared name instead of the spelling.
- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.
- Native: a 50,000-line program still takes 1.4 GB to build (huge top-level function): split top-level code into several functions, and step 6 of the parallel pipeline (one LLVM module per unit).

## Performance (Goose suite, native, vs Rust)

- **Goal (user, 2026-11-02): Fa faster than Goose on Goose's own benchmarks, in sync mode, release builds**, using the strict typing of the checker for optimizations (ranges, index proofs, alias rule → `noalias`, immutability, narrowed types → unboxed layouts, purity → `readnone`, non-escaping → stack allocation). *LLVM fifth shift: Fa/Goose 3.96 → 1.9 on the Goose suite (CPU time 1.94; ahead or level on `sum`, `push`, `tree`, `graph`, `particles`, `blur`); see "Against Goose on its own suite" in `benchmarks/README.md`.* Left, structural, by expected gain: inline strings (≤ 23 bytes) and `Character` as a scalar (`calc`, `sexp`, `strlist`, `words`, `records`, `respond`); `T?` of a reference as a nullable pointer (`tree`, `binary-trees`: nodes from 40 to 24 bytes); pools for the boxes of a recursive value freed at once (`binary-trees`, `interp`, `calc`); field layouts narrowed by ranges the analyzer proves for every store (`lru`: `Integer` fields that hold indices); map entries without a stored hash and 32-bit index slots (`lru`, `words`); `scene` (Transform copies by value).
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.

## Features still missing (from the engineers' reports)

- Native: relations, columnar per-field layout, reactivity, compile-time maps/sets/enums/ranges (*assigned to the LLVM fourth shift*); foreign code: `CArray` values, `CStruct` by value off x86-64, `Pointer.allocate`/`free`, WebAssembly imports, `build.cLibrary`; JS foreign code: structs by value, `destroy` of resources inside values, collections or replaced locals. Concurrency: `parallel()` views, `WorkerPool`, `for parallel` over growing collections and columnars, requirements and `mutable use` in parallel bodies.
- Analyzer: relations across files, `@import`, `@extern` (*foreign code*); implied relation effects for reactivity; `TypeFromSchema` of named unions, enumerations, functions, recursive references and inside generic type functions; JSON default values and named unions; `@typeToString` spellings (D49: op-1, tf-1, rt-1).
- Native: `std.time`, `std.fs`, `std.json`; compile-time named union members (`Schema` values); `while let` over non-scalar optionals.
- Tooling: hot reloading by module swap (D43; `fa develop` restarts today), the language server resolving dependencies, `fa build` without `--target` building every configured target, `fa test` running test files in separate workers, `DependencySource.generate`, path dependencies outside a workspace (decided: allowed).
- JavaScript: a reference to a primitive element of a `Pool` (`pool.find(…)`), the guard of an awaited `lock()` used as a temporary. (User functions returning a reference to a primitive are rejected by the analyzer, `REF004`.)
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.
