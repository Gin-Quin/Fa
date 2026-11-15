# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **Beat Goose on the complete suite**, in synchronous release builds, using the same inputs and checking every output. The ports, Goose column, scoped alias metadata, parallel unit compilation and bounded full LTO are implemented. The remaining representation and CPU gaps are assigned to the native performance engineer; keep CPU time separate from instruction counts. Current improvements and their measurements belong in `benchmarks/README.md`, not in a stale baseline here.
- **Analyzer passes**: fuse the ordinary-file checks into the intended single walk, retaining on-demand handling of cycles and generic specialization. Measure CPU, wall time, peak memory and deterministic diagnostics on the same 1,000-file package. Assigned to the analyzer engineer after the missing language rules.

- **Caller destinations for native returns (user decision, 2026-11-15):**
  extend the implemented synchronous collection buffer forwarding to remaining
  constructors and coroutine instances.
  Synchronous object/tuple calls now use output slots (direct calls, supported
  generic instances, function values/lambdas, blocks, `if`/`when`, nested fields,
  mapped array elements and eligible recursive early boxes), including immutable
  named locals when every successful exit returns the same proven movable owner.
  Still needed: coalesce mutable locals or different result owners; forward into
  ordinary push/insert slots and existing fields when alias/error rules allow it;
  cover structural/SoA conversions, optional/union/fallback/catch/loop results,
  coroutine promises and blocking finished-task result slots.
  Preserve analyzer moves and existing foreign `sret`, early recursive boxes and
  in-place pushed object construction. Never use stack-to-heap copy-out.
  Collection-returning user functions now forward bounded buffers through direct
  calls, function values/lambdas, branches and proven moved immutable locals.
  Literal arrays, growing arrays, tables and pools build directly in final storage;
  object columnars retain their larger heap chunks. Bounded buffers also cover
  nonempty-separator `String.split`,
  array `map`/`filter` and required copies of live borrowed `Array.from` without
  explicit hints. Extend to other collection transforms, `from` with capacity
  hints or conversions, empty-separator splits and other fresh constructors,
  including table indices and stable segments. A return containing type parameters
  inside a tuple (`Tuple<T, T>`) already fails outside its generic instance in
  the native baseline; resolve that layout instantiation independently.
- **Immutable storage coverage (native, 2026-11-15):** add compact immutable
  object-columnar chunks that preserve the reference ABI, broader escape proofs
  for borrowed parameters/non-escaping captures/async intervals, and static
  lowering of interpreter-evaluated initializers, defaulted objects, optional/enum/union
  values, and 128-bit or composite table keys. Unsupported read-only shapes keep
  the ordinary frame/heap construction path;
  `Bytes` is not implemented in this branch and must adopt the same storage rules.

- **Reviewed stack-storage follow-ups (Fable, 2026-11-15):** verify whether
  `wasm-ld --stack-first` is in effect and test that a deep shadow-stack overflow
  traps instead of overwriting data. Apply the async gate per function rather
  than per program (`collection_stack`, `uses_return_slot`,
  `uses_collection_return`); later allow coroutine collection ranges spanning
  no suspension. Measure indirect recursion before extending cycle detection.
  Frame grants currently follow lowering order; prefer loops if measurements
  justify it. Preserve the shared 128-byte recursive offer sub-budget when
  extending constructor destinations; the one/four-split relative depth gate
  uses a 256 KiB native stack and excludes exact literals. The `nbody` delta after the
  reviewed by-value rule (861.2 M → 867.2 M) is attributed to a loop-alignment
  NOP executed six million times; retain the profile for future code-layout work.

- **Follow-ups of the trunk integration (2026-11-15, "Trunk integration" in `design/decisions.md`)**:
  - *Recursion across units*: a unit indexes its own recursive functions, and a function that calls into another unit counts as recursive (512 bytes of frame collections instead of 4 KiB). Share the call graph through the program facts (`facts.rs`) so that such a function keeps the whole budget.
  - *Cleanup loads of growing collections*: the load of a header in a cleanup carries no `dereferenceable` metadata, since a last-use move zeroes the local. A flag of the move, or a proof that the local is never moved, would give it back.
  - *Read-only tables*: the compiler hashes integers up to 64 bits, booleans, floats and strings (`constant_key` in `lower/tables.rs`, kept in step with `build_hash` and `fa_hash_bytes`); a literal with another key (an enum, a 128-bit integer, a tuple, an object) is built on the heap. A test that compares the two hashes key by key would catch a divergence before a program does.
  - *Performance record*: `design/native-goose-performance.md` (in French) is kept for the sections written on `perf/native-goose` after the record was condensed into the English design records (`ff1af45f`). Condense those sections too (string building, bytes, stack collections, caller destinations, capacity, the `GrowingArray` audit), then remove the file and the four JSON files of its first rounds.
  - *Reserved tables*: `reserve` and `from(…, capacity = n)` now allocate the chunks of the entries (32 entries each) with the index; measure `lru` and `words` against the lazy chunks, and size the first chunk of a small reservation if it shows.

## Bugs

- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.

## Strings: UTF-16 semantics over UTF-8 storage (user decision, 2026-11-15)

See `design/decisions.md` ("Strings: UTF-16 semantics, UTF-8 storage") and the pages `values/strings`, `operations/string`, `values/bytes` (with their `:::implementation` sections). The documentation is done; nothing is implemented. Suggested order: analyzer, then the two backends and the interpreter together (the T-STR cases must agree on every target), then the fixtures.

- **Analyzer** (`packages/analyzer`, 38 files name `Character` or graphemes: `builtins/strings.rs`, `check/strings.rs`, `types/{mod,store,merge,relate,layer}.rs`, `flow/facts.rs`, `check/proofs.rs`, `diagnostics/codes/values.rs`…):
  - remove the `Character` type, its literal adaptation, `Character ≤ String`, `ConstValue::Character`, `Schema.Character`; retire `E-STR-8` and `E-STR-9`; an unknown `Character` gets a fix that names `String`;
  - `text[i]` and `get` give `String`/`String?`; the loop variable over a string is a `String` (one code point), its index a code unit position; `characters()` returns `Array<String>`;
  - length facts of literals in UTF-16 code units (D-97); index proofs unchanged otherwise;
  - new builtins: `codeUnitAt`, `isWellFormed`, `utf8Length`, `toUtf8`, `String.fromUtf8` (`throws Utf8Error`), `String.fromUtf8Lossy`; remove `bytes()`; recognize `codePoints().length` and an iteration of `toUtf8()` (no array built);
  - the prelude: the `Bytes` type (array rules for indexing, slices, `use` slices and item assignment; no structural mutation), `Bytes.from`, `Bytes.filled`, `toArray`, `Schema.Bytes`, `Utf8Error { position }`;
  - `std`: `File.read`/`File.write` on `Bytes`, `File.readText`/`File.writeText`, HTTP `body: Bytes` and `text()`, `Json.parse` accepting `Bytes`.
- **Native runtime and codegen** (`packages/codegen-llvm`, `runtime/fa_runtime.c` then `runtime/generate.sh`):
  - replace the grapheme scans (`length`, indexing, `faSliceBounds`, searches) by code unit arithmetic: cached UTF-16 length, breadcrumbs every 64th code unit, positions inside a four-byte sequence;
  - the ASCII flag (`SIMPLE`) no longer excludes `'\r'`; **slices carry the ASCII flag** of their string (`{ ptr, byte length }` has no flag today: take a bit of the length, or keep the flag statically);
  - WTF-8: the surrogates flag, cutting a pair (index, slice), joining a pair in concatenation and `+=`, `U+FFFD` in `toUtf8()` and in every output (`console.log`, files, `CString`);
  - `<` in code unit order (correction at the first differing byte; code unit comparison with the surrogates flag);
  - **searches report UTF-16 positions**: `indexOf`, `lastIndexOf`, `split`, `replace` search the bytes and count the code units of the prefix (identity with the ASCII flag); a needle with a lone surrogate at an end is searched by code unit; the same conversion for a future regular expression engine;
  - iteration by code point with a code unit index; `codeUnitAt`; a one-unit string compared with a literal without being built (replaces the planned `Character` scalar: `calc`, `sexp`, `strlist`, `words`, `records`, `respond`);
  - `Bytes` (layout of `Array<Byte>`), `toUtf8`/`fromUtf8` moving the block of a dead source, vectorized UTF-8 validation that computes the ASCII flag, `fromUtf8Lossy`;
  - UAX #29 tables only for `characters()`, linked only when it is called (check the binary sizes with the `report` example);
  - **wasm boundary**: strings cross to the JavaScript host as UTF-8 (`TextDecoder`/`TextEncoder` on the memory), lone surrogates as `U+FFFD`; the JS String Builtins were measured slower (2.4 ms per MB on Node, 10.5 ms on Bun), don't use them.
- **JavaScript codegen and runtime** (`packages/codegen-js`, `runtime/*.js`, then `bun test packages/codegen-js/runtime` and the golden files):
  - delete the grapheme helpers, the ASCII regular expression fast path and the code point comparison helper: `length`, indexing, slices (`slice`), searches and `<` are the plain JavaScript operations; keep the bound checks and `-1` → `undefined`;
  - iteration: `for…of`, or a `codePointAt` loop when the index is used; `characters()` through the shared `Intl.Segmenter`;
  - `utf8Length` (`Buffer.byteLength` on `bun`/`node`, a `charCodeAt` loop on `web`), shared `TextEncoder`, `TextDecoder("utf-8", { fatal: true, ignoreBOM: true })` and its lossy twin, `Utf8Error` with the position of the first invalid byte (found by a scan after the failure);
  - `Bytes` as `Uint8Array`: `slice()` for copies, `subarray()` for `use` slices, helpers for `==` and `toString`.
- **Compile-time interpreter** (`packages/interpreter`): the same rules on its own strings, so that constants agree with every target: UTF-16 length and positions, lone surrogates from a cut pair, code unit order of `<`, code point iteration, `Bytes` values and the conversions, `utf8Length`. A `static` string with a lone surrogate must be emitted as such by both backends.
- **Language server and CLI** (`packages/language-server`, `packages/cli`: one file each names `Character`): hovers and completions of the removed type; LSP positions are already UTF-16.
- **Formatter and parser**: not affected (no syntax change; `Character` was only a type name).
- **Tests and fixtures**: the T-STR cases of `design/implementation/values-and-control-flow.md` (T-STR-12 to 14, 20, 21, 26, 27, 31, 32, 46, 47 changed or new); the 11 `.fa` fixtures of `packages/` that use `Character`; `benchmarks/calc`, `benchmarks/sexp`, `benchmarks/string-building` and `benchmarks/README.md`; add cases with non-ASCII text on every target (breadcrumbs, cut pairs, order, search positions), since the benchmarks are all ASCII.
- **Other design records** still describe graphemes as history: `design/spec-review.md` (V21, V22, AR31), `design/minimalism-audit.md`, `design/analyzer-design.md`, `design/llvm-backend-plan.md`, `design/native-goose-performance.md`. Update them when their subject is touched.

## Performance (Goose suite, native, vs Rust)

- **Remaining native layouts**: further compact `sexp` recursive payloads, `words` string keys, `scene` transforms, and traversal costs. Frame strings, physical root regions, compact child arrays, closed whole-program proofs and unobserved-capacity facts are integrated and pass the complete LLVM suite. Further gains must be measured against the actual original 16 programs with matched compilers; keep CPU, instructions and child-process RSS separate.
- **Benchmark drivers**: `benchmarks/audit/compare.py` and the first audit's shell scripts still overlap (one driver would do). A separately validated metrology lot adds manifests, the actual original 16 programs and checked reuse.
- **Split aggregates without fragments (native, proposed by the native-goose work, not implemented)**: a `let parts = text.split(sep)` read only through `parts.length` and `parts.join(literal).length` (string-building), or the chained `text.split(sep).length`, needn't build the array. Sound plan: the uses are syntactic (no other read, no capture, no alias, no `use`), the text and separator are evaluated once at the declaration; at run time, when the text and the separator are simple (ASCII, no `\r`) and the separator isn't empty, store the count of non-overlapping byte matches + 1 and the byte length of the text instead of the array (graphemes are bytes there, and a simple `literal` can't form a cluster across a boundary); otherwise build the array as today (a null-or-array slot, dropped when built). `.length` is then the count, `.join(literal).length` is `bytes - (count - 1) * sep.length + (count - 1) * literal.length`. Tests: mutated sources and separators after the declaration, empty texts and separators, non-simple texts and literals, evaluation order, leak checks. `string-building` is already 0.88x Goose (2026-11-04), so the gain is small. Since the split views (`src/lower/views.rs`), a split into views costs one count of the separator byte and three stores per part: still not done, the gain is smaller still.
- **`lru` index proofs**: the port now uses the same explicit widths as Goose and Rust (`Int32` keys/values, `UInt32` links). Removing the remaining index checks still needs a relational invariant: every value stored into the links, `free` and the values of `positions` designates an element of `nodes`, and `nodes` never shrinks. Ordinary integer intervals alone cannot establish that relationship. Automatic narrowing of wider user-written fields is a separate future use of the same facts. `unlink` is already inlined.
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.

## GrowingArray native layout (decision of 2026-11-15, `design/decisions.md`)

`GrowingArray` stays a segmented list; see "GrowingArray native layout" and the `:::implementation` section of `collections/growing-collections`. The segmented fast paths are implemented; the capacity and header work is implemented (2026-11-15). The indexing audit and two-stage directory are done (2026-11-15); the remaining items are listed below.

- **Segmented fast paths** (*done*): cached cursor and end pointer for `push`, block copies per segment, index checks hoisted out of loops, `x = []` reusing the segments.
- **Measure** (*done, closed 2026-11-15*): the branch-free lookup and biased directory pass every indexing CPU gate; the layout is final and no reserved segment is implemented. String-building `GrowingArray<Byte>` against `Array<Byte>` is not resolved by the measurement (1.09× to 1.12×, noise of the host larger than the margin); see the closure in `design/decisions.md`.
- **`GrowingArray` on contiguous paths** (string-building residual, 12.2 M instructions at 300,000 entries: `appendInteger` +7.0 M, `main` +5.1 M): (1) `x = []` with items that need no drop emits the stores inline instead of calling `fa_segmented_reset`; (2) `for item in a { b.push(item) }` between growing arrays of plain items becomes one block append per segment; (3) check in the IR that the `k` load, the `k < 0` test and `segments <= 1` are hoisted out of the loop of indexed writes in `appendInteger`. Profile first. Done when `GrowingArray<Byte>` is at most 1.10× `Array<Byte>` in CPU on both measures, in a series with a same-binary control within 3%; instructions (target about 181 M) are a guide only; no program of the audit above +1% in instructions.
- **Benchmark harness**: add the same-binary control (one binary under two labels, interleaved) to the paired CPU series, and report quartiles of the paired ratios with the medians.
- **wasm at `-O0`, a failure that predates this work**: `tree-arena-large-payload` declares a function of 8,196 parameters, above Node's limit of 1,000 (`fa run --target=wasm -O0`; it passes at the default level, which the test harness uses). `floats_and_strings` passes on the trunk.
- **No more flat views**: the read-only methods of a `GrowingArray` (`join`, `includes`, `indexOf`, `map`, `filter`, slices…) run on the segments instead of on a copy (`growing_view`, `fa_segmented_flatten` in `lower/growing.rs`); keep the flat copy only where an `Array` is produced.
- **`k` per array** (*done*): exponent in the header; `from(items, capacity = n)` and empty `reserve` choose a contiguous first block; non-empty reservations preallocate later segments; copies start with one segment of their length. The default push cursor remains unchanged.
- **Smaller header** (*done*): 64 bytes on 64-bit native targets (40 on wasm32), with three segment pointers inline and a directory allocated on the fourth segment. Segments never move.
- **`x = []` for the other growing collections**: `GrowingPool`, `GrowingColumnar`, `GrowingMap` and `GrowingSet` keep their chunks, pages and index when an empty collection is assigned to a `mutable` variable or field. Analyzer: check that `clear` on a growing collection says to assign `[]`.
- **Recover the cost of the per-array `k`** (*done, 2026-11-15*): 224.199 → 188.226 M instructions on string-building, below 206.5 M and 1.073× the freshly measured 175.418 M of Array<Byte>. One-segment proofs, stable header loads, contiguous byte search/copies, default-k origin proofs and direct highest-bit lookups; every required bound check remains. The Array/String gap to Goose is shared code, followed under the Goose goal above.
- **Directory in two steps** (*done, 2026-11-15*): three inline pointers, eight overflow entries at segment four (eleven segments total), reallocated once to 64 entries on 64-bit or 32 on wasm32 at segment twelve. Directory reloads survive growth; item addresses stay stable. Runtime tests cover both sized frees, frame storage, reallocation and empty reservation rebuilds.
- **Branch-free unknown indexing** (*done, 2026-11-15*): four million excess conditional mispredictions at 16 items fall to about fourteen above Array. `clz(2 * (index >> k) + 1)` and a biased integer directory remove every index-dependent branch beyond the bounds check. The directory always points to the inline entries or their heap mirror; frame/static storage uses the same rule. The default first segment is unchanged.
- **Reserved first segment, amended by Fable** (2026-11-15): never for small arrays. The million-item row is 0.913× Goose in median CPU, so the amended large-array condition is not triggered. No reservation, promotion or inferred capacity is implemented. Stop after the authorized biased-directory step; further work on the remaining string-building CPU gate needs a new decision.

## Generic angle brackets (decisions of 2026-11-15, `design/decisions.md`)

- **Analyzer, `GN030`**: implemented for the positions that aren't a parameter (a type alias, a field, a collection item, a `let`, a return type, the annotation of a `function`; `generic-function-value-errors.fa`, `gn-angle-brackets.fa`). Left: refuse a generic function type as the type of a parameter of a lambda or of a function type (accepted today: `run: (transform: <Value>(value: Value) => Value) => Integer`), keeping it for the parameters of `function` declarations and methods; check the argument (a named function, a lambda written as the argument, or the caller's own parameter of a generic function type); cases for `static` and for forwarding; the message for a declaration says to use the function itself. A generic function type written as a return type without parentheses isn't parsed as one (`(): <T>(value: T) => T => …`), and `AN001` is reported for the grouped form: report `GN030` there too. The native check of a class of instances that keeps growing (`generic_values.rs`) can no longer be reached from a program: keep it as a guard, or remove it once the rule above is complete.
- **Analyzer, `E-DECL-6`**: `let f = <T>(…) => …` reports `E-DECL-6` alone; its message still says `function f = …` and should show `function f = <T>(…) => …`.
- **Parser, `<Item> async (…)`**: the error says to write `async <Item>(…) => …`. Tests for `async <Item>(…) => …` as a lambda and as a function type (parser, analyzer, a program on both targets).
- **Formatter, long generic lists**: break what follows the list first, then a last item between braces or parentheses, then the list: a line break after each comma, items indented once, no trailing comma, `>` on its own line at the indentation of the statement; never a line break after `<`. Corpus test: idempotence and tree preservation of the broken form (declarations, applications, nested lists).
- **Parser, recovery of an unclosed generic list**: one error, then go back to the `<` and read it as a comparison in the same expression context (`if count<limit { … }` gives one error and a checked block); drop the nodes and diagnostics of the abandoned list. After a capitalized name, the message is "this generic list is never closed: add `>`" (`let y: Array<Integer = []` says `Array < Integer` today). Tests in `recovery.rs`; `doc_snippets` keeps its two expected errors.

## Features still missing (from the engineers' reports)

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
