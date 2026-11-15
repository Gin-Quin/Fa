# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **Beat Goose on the complete suite**, in synchronous release builds, using the same inputs and checking every output. The ports, Goose column, scoped alias metadata, parallel unit compilation and bounded full LTO are implemented. The remaining representation and CPU gaps are assigned to the native performance engineer; keep CPU time separate from instruction counts. Current improvements and their measurements belong in `benchmarks/README.md`, not in a stale baseline here.
- **Analyzer passes**: fuse the ordinary-file checks into the intended single walk, retaining on-demand handling of cycles and generic specialization. Measure CPU, wall time, peak memory and deterministic diagnostics on the same 1,000-file package. Assigned to the analyzer engineer after the missing language rules.

## Bugs

- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.

## Performance (Goose suite, native, vs Rust)

- **Remaining native layouts**: further compact `sexp` recursive payloads, `words` string keys, `scene` transforms, and traversal costs. Frame strings, physical root regions, compact child arrays, closed whole-program proofs and unobserved-capacity facts are integrated and pass the complete LLVM suite. Further gains must be measured against the actual original 16 programs with matched compilers; keep CPU, instructions and child-process RSS separate.
- **Benchmark drivers**: `benchmarks/audit/compare.py` and the first audit's shell scripts still overlap (one driver would do). A separately validated metrology lot adds manifests, the actual original 16 programs and checked reuse.
- **Split aggregates without fragments (native, proposed by the native-goose work, not implemented)**: a `let parts = text.split(sep)` read only through `parts.length` and `parts.join(literal).length` (string-building), or the chained `text.split(sep).length`, needn't build the array. Sound plan: the uses are syntactic (no other read, no capture, no alias, no `use`), the text and separator are evaluated once at the declaration; at run time, when the text and the separator are simple (ASCII, no `\r`) and the separator isn't empty, store the count of non-overlapping byte matches + 1 and the byte length of the text instead of the array (graphemes are bytes there, and a simple `literal` can't form a cluster across a boundary); otherwise build the array as today (a null-or-array slot, dropped when built). `.length` is then the count, `.join(literal).length` is `bytes - (count - 1) * sep.length + (count - 1) * literal.length`. Tests: mutated sources and separators after the declaration, empty texts and separators, non-simple texts and literals, evaluation order, leak checks. `string-building` is already 0.88x Goose (2026-11-04), so the gain is small. Since the split views (`src/lower/views.rs`), a split into views costs one count of the separator byte and three stores per part: still not done, the gain is smaller still.
- **`lru` index proofs**: the port now uses the same explicit widths as Goose and Rust (`Int32` keys/values, `UInt32` links). Removing the remaining index checks still needs a relational invariant: every value stored into the links, `free` and the values of `positions` designates an element of `nodes`, and `nodes` never shrinks. Ordinary integer intervals alone cannot establish that relationship. Automatic narrowing of wider user-written fields is a separate future use of the same facts. `unlink` is already inlined.
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.


## Features still missing (from the engineers' reports)

- **Native self-replacement moves**: the JavaScript gap audit reproduced quadratic construction of `list = List { head, tail = list }` and consumption of `cursor = cursor.tail`. JavaScript now consumes a single root read when replacing a mutable optional local without intervening calls, closures or suspension. Publish the corresponding shared analyzer proof for native lowering; the new `js-list-prepend.fa` and `js-million-copy.fa` fixtures also run in the LLVM harness.

- **Bun generic pointer capabilities**: lead's full suite exposed synthesized generic descriptor name and owner-argument order failures. Fix them against the new metadata/origin summaries before integration. It covers generic function values, copies, resources, task destruction and references to `pointer.value`. Remaining combinations with generic method owners must coalesce with the analyzer's origin metadata.
- **Analyzer language rules**: `with` and its backends, the documented diagnostic rules still without an emitter, `TypeFromSchema` inside a generic type other than as its value, and interpretation on an explicit continuation stack. The callback error-set instances and exact `throws`/`mutates`, including exact variants, are integrated. Method selection must preserve the written origin, also inside generic owners and their destructors. Assigned to the analyzer engineer.
- **Key-loop effects**: inferred expression bodies must refresh their thrown effects and callers (`modules/generic-key-loop-inferred-errors.fa`). Analyzer owns this after the origin metadata lot. Pick/Omit, key-set interfaces and EF004 are integrated and validated.
- Native: compile-time named union members (`Schema` values).
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.


## Tooling decisions waiting for an engineer

- **JIT for native tests and `fa run --target=native` (decided 2026-11-14, see `design/decisions.md`; in progress on `tooling/native-jit`)**: the module compiled in memory with ORC instead of a linked executable, one forked child per test, the `dev` profile. Measure the wall time of the native test fixtures before and after.

## Latest language decisions being implemented

- Task and every type declaring `destroy` are noncopyable, including composites that own one. Resource parameters are borrowed; a second resource requires a new instance (`open`, `create` or constructor). Ordinary collections and their owners require `copy value` when both values remain usable; implicit moves at the last use are allowed. Explicit synchronization handles such as Mutex and Channel retain their sharing semantics. Update both backends, generic capabilities, pointer reads, tests and documentation; the root lot remains unintegrated.
- Set uses standard value hashing and equality, without a key callback. Use Map for a separate key. The root implementation and generated codecs are being validated together.
- Written types with identical fields and different methods/destructors retain their alternatives in an inferred union. Dispatch uses the actual member. Analyzer owns origin unions after its frozen single-origin lot.
- `with` uses the parameter names of the visible signature, including parameters and fields holding function values. Analyzer owns the complete feature.
