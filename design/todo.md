# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **Beat Goose on the complete suite**, in synchronous release builds, using the same inputs and checking every output. The ports, Goose column, scoped alias metadata, parallel unit compilation and bounded full LTO are implemented. The remaining representation and CPU gaps are assigned to the native performance engineer; keep CPU time separate from instruction counts. Current improvements and their measurements belong in `benchmarks/README.md`, not in a stale baseline here.
- **Analyzer passes**: fuse the ordinary-file checks into the intended single walk, retaining on-demand handling of cycles and generic specialization. Measure CPU, wall time, peak memory and deterministic diagnostics on the same 1,000-file package. Assigned to the analyzer engineer after the missing language rules.

## Bugs

- **JavaScript worker errors**: stable declaration identities, class reconstruction and typed displays are being validated on lead. The reproduction with two files declaring `Problem.Bad` used to choose `left` for both errors, directly and through a worker. The native and wasm test runner work is integrated and validated.
- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.

## Performance (Goose suite, native, vs Rust)

- **Strings up to 64 bytes**: inline strings, UTF-8 character views, capacity/reserve APIs, constant folding and helper inlining are implemented. The native performance engineer is validating frame buffers for nonescaping strings of 24–64 bytes, including deep copies when they escape. No unvalidated layout change is integrated.

- **Remaining native layouts**: compact map slots (with wasm size-overflow fixes), frame strings, `sexp` recursive payloads and regions, `words` string keys, `scene` transforms, and the CPU cost of the `binary-trees` walk. Recursive return-path and empty-leaf optimizations are integrated; `binary-trees` now wins in instructions but its CPU win remains open.
- **Open from the review of the Goose work**: a conservative whole-program arena/recursive-collection proof is being built by the proofs engineer and consumed by the native performance engineer. `benchmarks/audit/compare.py` and the first audit's shell scripts still overlap (one driver would do).
- **Split aggregates without fragments (native, proposed by the native-goose work, not implemented)**: a `let parts = text.split(sep)` read only through `parts.length` and `parts.join(literal).length` (string-building), or the chained `text.split(sep).length`, needn't build the array. Sound plan: the uses are syntactic (no other read, no capture, no alias, no `use`), the text and separator are evaluated once at the declaration; at run time, when the text and the separator are simple (ASCII, no `\r`) and the separator isn't empty, store the count of non-overlapping byte matches + 1 and the byte length of the text instead of the array (graphemes are bytes there, and a simple `literal` can't form a cluster across a boundary); otherwise build the array as today (a null-or-array slot, dropped when built). `.length` is then the count, `.join(literal).length` is `bytes - (count - 1) * sep.length + (count - 1) * literal.length`. Tests: mutated sources and separators after the declaration, empty texts and separators, non-simple texts and literals, evaluation order, leak checks. `string-building` is already 0.88x Goose (2026-11-04), so the gain is small. Since the split views (`src/lower/views.rs`), a split into views costs one count of the separator byte and three stores per part: still not done, the gain is smaller still.
- **`lru` index proofs**: the port now uses the same explicit widths as Goose and Rust (`Int32` keys/values, `UInt32` links). Removing the remaining index checks still needs a relational invariant: every value stored into the links, `free` and the values of `positions` designates an element of `nodes`, and `nodes` never shrinks. Ordinary integer intervals alone cannot establish that relationship. Automatic narrowing of wider user-written fields is a separate future use of the same facts. `unlink` is already inlined.
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.


## Features still missing (from the engineers' reports)

- **Bun generic pointer capabilities**: lead's separate branch is frozen for its full suite, after analyzer, JS codegen, runtime, website and pointer regressions on all relevant targets. It covers generic function values, copies, resources, task destruction and references to `pointer.value`. Remaining combinations with generic method owners must coalesce with the analyzer's origin metadata.
- **Analyzer language rules**: `with` and its backends, the documented diagnostic rules still without an emitter, `TypeFromSchema` inside a generic type other than as its value, and interpretation on an explicit continuation stack. The callback error-set instances and exact `throws`/`mutates`, including exact variants, are integrated. Method selection must preserve the written origin, also inside generic owners and their destructors. Assigned to the analyzer engineer.
- **Cross-file generic interfaces**: `Pick`/`Omit` derived parameters, key-set applications and `EF004` writes through `Type<parameter>` are implemented in the metadata branch and running the full suite before integration. Assigned to the proofs engineer.
- Native: compile-time named union members (`Schema` values).
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.

## Language decisions awaiting the user

- Copy requirements on generic parameters: infer a copyable requirement from the body and check concrete calls/function values, or require an explicit guarantee at the definition. Known move-only `Pointer.read`/`toArray` calls are rejected already.
- Joining distinct method origins without an annotation: require an explicit written type when methods/destruction depend on it, or retain each branch's origin for runtime dispatch. Independent work continues while these questions are pending.
