# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

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

## Bugs

- Async follow-ups (after J1/W1): async lambdas and awaits through function values are blocking calls (native and JS); `fa test` runs tests in blocking mode; detached tasks don't copy their arguments on JavaScript; `.d.ts` files don't declare the two instances of an async function; J2 (workers on JavaScript) not done.
- Async: the loop back-edge yield doesn't apply to the top level of a file or to tests (it broke narrowing of top-level loops): a long top-level loop never yields. Revisit when the native executor lands.
- `fa-interpreter` `the_budget_and_the_depth_are_bounded` fails under `ulimit -v 4000000` (the evaluation stack shrinks): make the test independent of the address-space limit.
- `Schema.Integer.signed` is `Boolean?` in the prelude (built-in types can't have defaults) while the docs say `Boolean = true`: the docs win; built-in object types must support field defaults (lead's decision).
- Native: pa-10 (a `Set` spread into a variadic parameter sums garbage), ft-1 (calling a `mutates` function value overflows the stack); `std.time`/`std.fs`/`std.json` not lowered natively; `while let` natively only for optional scalars.
- Analyzer: `TypeFromSchema` for named unions, enums, functions, recursive references and inside generic type functions; `@typeToString` prints the declared name instead of the spelling.
- `W-ERR-1` on a string slice (Goose ports): not reproduced (unproven slices are `E-ERR-3`, proven ones `W-ERR-1` by design); needs the snippet.
- Native: a 50,000-line program still takes 1.4 GB to build (huge top-level function): split top-level code into several functions, and step 6 of the parallel pipeline (one LLVM module per unit).

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

- **User decisions of 2026-11-03** (see `design/decisions.md` "Minimal representations"): text stored as UTF-8 bytes (sexp's text is walked in place; the `Character` views were superseded on 2026-11-15, see the strings section above), strings < 64 bytes never on the heap (inline ≤ 23 bytes, stack buffers ≤ 64 bytes when not escaping), `capacity`/`reserve` on strings and every collection (no-op on JavaScript), parameters never copied (always by reference), module `let` constants folded, small hot-path helpers inlined. The capacity API is implemented on every resizable collection and strings (2026-11-15), including native allocation, JavaScript no-ops, compile-time evaluation and documentation; fixed-length `Bytes` deliberately has no spare capacity. Still to implement: escape facts for stack strings, remaining layouts and calling conventions.

- **Goal (user, 2026-11-02): Fa faster than Goose on Goose's own benchmarks, in sync mode, release builds**, using the strict typing of the checker for optimizations (ranges, index proofs, alias rule → `noalias`, immutability, narrowed types → unboxed layouts, purity → `readnone`, non-escaping → stack allocation). *LLVM fifth shift: Fa/Goose 3.96 → 1.9 on the Goose suite (CPU time 1.94; ahead or level on `sum`, `push`, `tree`, `graph`, `particles`, `blur`); see "Against Goose on its own suite" in `benchmarks/README.md`.* Left, structural, by expected gain: inline strings (≤ 23 bytes) and one-unit strings compared as code units (`calc`, `sexp`, `strlist`, `words`, `records`, `respond`); `T?` of a reference as a nullable pointer (`tree`, `binary-trees`: nodes from 40 to 24 bytes); pools for the boxes of a recursive value freed at once (`binary-trees`, `interp`, `calc`); field layouts narrowed by ranges the analyzer proves for every store (`lru`: `Integer` fields that hold indices); map entries without a stored hash and 32-bit index slots (`lru`, `words`); `scene` (Transform copies by value).
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.

## GrowingArray native layout (decision of 2026-11-15, `design/decisions.md`)

`GrowingArray` stays a segmented list; see "GrowingArray native layout" and the `:::implementation` section of `collections/growing-collections`. The segmented fast paths are implemented; the capacity and header work is implemented (2026-11-15). The indexing audit and two-stage directory are done (2026-11-15); the remaining items are listed below.

- **Segmented fast paths** (*done*): cached cursor and end pointer for `push`, block copies per segment, index checks hoisted out of loops, `x = []` reusing the segments.
- **Measure** (*done, amended 2026-11-15*): string-building and unreserved growing-index at 16, 1,000 and 1,000,000 items, against Array, Goose and Rust. The branch-free lookup and biased integer directory pass every indexing CPU gate with 21 alternating CPU-1 runs and both ratios. String-building is 187.626 M instructions (cap 190.183 M), but 1.122× Array in median CPU (paired-ratio median 1.090×): the layout remains open on that gate. See the latest French section and `native-goose-growing-array-branchless-results.json`.
- **No more flat views**: the read-only methods of a `GrowingArray` (`join`, `includes`, `indexOf`, `map`, `filter`, slices…) run on the segments instead of on a copy (`growing_view`, `fa_segmented_flatten` in `lower/growing.rs`); keep the flat copy only where an `Array` is produced.
- **`k` per array** (*done*): exponent in the header; `from(items, capacity = n)` and empty `reserve` choose a contiguous first block; non-empty reservations preallocate later segments; copies start with one segment of their length. The default push cursor remains unchanged.
- **Smaller header** (*done*): 64 bytes on 64-bit native targets (40 on wasm32), with three segment pointers inline and a directory allocated on the fourth segment. Segments never move.
- **`x = []` for the other growing collections**: `GrowingPool`, `GrowingColumnar`, `GrowingMap` and `GrowingSet` keep their chunks, pages and index when an empty collection is assigned to a `mutable` variable or field. Analyzer: check that `clear` on a growing collection says to assign `[]`.
- **Recover the cost of the per-array `k`** (*done, 2026-11-15*): 224.199 → 188.226 M instructions on string-building, below 206.5 M and 1.073× the freshly measured 175.418 M of Array<Byte>. One-segment proofs, stable header loads, contiguous byte search/copies, default-k origin proofs and direct highest-bit lookups; every required bound check remains. The Array/String gap to Goose is shared code, followed under the Goose goal above.
- **Directory in two steps** (*done, 2026-11-15*): three inline pointers, eight overflow entries at segment four (eleven segments total), reallocated once to 64 entries on 64-bit or 32 on wasm32 at segment twelve. Directory reloads survive growth; item addresses stay stable. Runtime tests cover both sized frees, frame storage, reallocation and empty reservation rebuilds.
- **Branch-free unknown indexing** (*done, 2026-11-15*): four million excess conditional mispredictions at 16 items fall to about fourteen above Array. `clz(2 * (index >> k) + 1)` and a biased integer directory remove every index-dependent branch beyond the bounds check. The directory always points to the inline entries or their heap mirror; frame/static storage uses the same rule. The default first segment is unchanged.
- **Reserved first segment, amended by Fable** (2026-11-15): never for small arrays. The million-item row is 0.913× Goose in median CPU, so the amended large-array condition is not triggered. No reservation, promotion or inferred capacity is implemented. Stop after the authorized biased-directory step; further work on the remaining string-building CPU gate needs a new decision.

## Generic angle brackets (decisions of 2026-11-15, `design/decisions.md`)

- **Analyzer, `GN030`**: refuse a generic function type as the type of a parameter of a lambda or of a function type (accepted today: `run: (transform: <Value>(value: Value) => Value) => Integer`); keep it for the parameters of `function` declarations and methods. Check the argument: a named function, a lambda written as the argument, or the caller's own parameter of a generic function type (forwarding, already accepted). Cases in `gn-angle-brackets.fa` for `let`, `static`, return type, field, forwarding. The message for a declaration says to use the function itself.
- **Analyzer, `E-DECL-6`**: `let f = <T>(…) => …` reports `E-DECL-6` only (no `GN015` after it), and its message shows `function f = <T>(…) => …`.
- **Parser, `<Item> async (…)`**: the error says to write `async <Item>(…) => …`. Tests for `async <Item>(…) => …` as a lambda and as a function type (parser, analyzer, a program on both targets).
- **Formatter, long generic lists**: break what follows the list first, then a last item between braces or parentheses, then the list: a line break after each comma, items indented once, no trailing comma, `>` on its own line at the indentation of the statement; never a line break after `<`. Corpus test: idempotence and tree preservation of the broken form (declarations, applications, nested lists).
- **Parser, recovery of an unclosed generic list**: one error, then go back to the `<` and read it as a comparison in the same expression context (`if count<limit { … }` gives one error and a checked block); drop the nodes and diagnostics of the abandoned list. After a capitalized name, the message is "this generic list is never closed: add `>`" (`let y: Array<Integer = []` says `Array < Integer` today). Tests in `recovery.rs`; `doc_snippets` keeps its two expected errors.
- **Older bug met there**: a block-bodied lambda ending in a tuple is typed `Void`.

## Features still missing (from the engineers' reports)

- Native: relations, columnar per-field layout, reactivity, compile-time maps/sets/enums/ranges (*assigned to the LLVM fourth shift*); foreign code: `CArray` values, `CStruct` by value off x86-64, `Pointer.allocate`/`free`, WebAssembly imports, `build.cLibrary`; JS foreign code: structs by value, `destroy` of resources inside values, collections or replaced locals. Concurrency: `parallel()` views, `WorkerPool`, `for parallel` over growing collections and columnars, requirements and `mutable use` in parallel bodies.
- Analyzer: relations across files, `@import`, `@extern` (*foreign code*); implied relation effects for reactivity; `TypeFromSchema` of named unions, enumerations, functions, recursive references and inside generic type functions; JSON default values and named unions; `@typeToString` spellings (D49: op-1, tf-1, rt-1).
- Native: `std.time`, `std.fs`, `std.json`; compile-time named union members (`Schema` values); `while let` over non-scalar optionals.
- Tooling: hot reloading by module swap (D43; `fa develop` restarts today), the language server resolving dependencies, `fa build` without `--target` building every configured target, `fa test` running test files in separate workers, `DependencySource.generate`, path dependencies outside a workspace (decided: allowed).
- JavaScript: a reference to a primitive element of a `Pool` (`pool.find(…)`), the guard of an awaited `lock()` used as a temporary. (User functions returning a reference to a primitive are rejected by the analyzer, `REF004`.)
- Reviews to schedule: a second analyzer review (M2/M3 features), a review of the tooling (CLI, formatter, workspaces), a final review of both backends.
