# Project lead's to-do list

Open items that are not yet assigned to a running engineer, or that need a follow-up. Done items are removed (the history is in Git and `design/decisions.md`).

## Performance

- **nbody aliasing (native)**: LLVM can't tell that `body` and `other` never designate the same element, so their fields stay in memory (Rust works around it with `split_at_mut`). Use the analyzer's facts (distinct indices, the alias rule) to emit `noalias`/scoped alias metadata, or keep fields in registers when no write can reach them. *Assigned to the LLVM backend's fourth shift.*
- **Benchmarks against Goose**: add the benchmarks of https://aardappel.github.io/goose/bench/results.html and a Goose column next to Rust. *Assigned.*
- **Parallel and streaming pipeline** (`design/parallel-pipeline.md`): steps 1–5 *assigned*; step 6 (LLVM per module, ThinLTO) and step 7 (language server) to schedule after.

- **Caller destinations for native returns (user decision, 2026-11-15):** finish
  destination passing for fresh user-function collection results and object/tuple
  results, including direct calls, generics, closures/function values, branches,
  recursive forwarding, fields, collection slots, errors and async instances.
  Preserve analyzer moves and existing foreign `sret`, early recursive boxes and
  in-place pushed object construction. Do not use stack-to-heap copy-out as a
  substitute. Current support passes a bounded caller buffer to nonempty-separator
  `String.split` used by a proven non-escaping immutable local; larger results
  choose the heap before constructing any items. Extend this protocol to
  `map`, `filter`, `from`, empty-separator splits, and the other fresh collection
  constructors, including table indices and stable segments.
- **Immutable storage coverage (native, 2026-11-15):** add compact immutable
  object-columnar chunks that preserve the reference ABI, broader escape proofs
  for borrowed parameters/non-escaping captures/async intervals, and static
  lowering of interpreter-produced composites, defaulted objects, unions and
  composite table keys. Current unsupported cases keep ordinary heap storage;
  `Bytes` is not implemented in this branch and must adopt the same storage rules.

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

- **User decisions of 2026-11-03** (see `design/decisions.md` "Minimal representations"): text stored as UTF-8 bytes (sexp's text is walked in place; the `Character` views were superseded on 2026-11-15, see the strings section above), strings < 64 bytes never on the heap (inline ≤ 23 bytes, stack buffers ≤ 64 bytes when not escaping), `capacity`/`reserve` on strings and every collection (no-op on JavaScript), parameters never copied (always by reference), module `let` constants folded, small hot-path helpers inlined. To implement on resume: analyzer (builtins `capacity`, `reserve`, `from(capacity)` everywhere; escape facts for stack strings), native (layouts), JavaScript (no-op `reserve`), documentation (strings, collections, functions pages).

- **Goal (user, 2026-11-02): Fa faster than Goose on Goose's own benchmarks, in sync mode, release builds**, using the strict typing of the checker for optimizations (ranges, index proofs, alias rule → `noalias`, immutability, narrowed types → unboxed layouts, purity → `readnone`, non-escaping → stack allocation). *LLVM fifth shift: Fa/Goose 3.96 → 1.9 on the Goose suite (CPU time 1.94; ahead or level on `sum`, `push`, `tree`, `graph`, `particles`, `blur`); see "Against Goose on its own suite" in `benchmarks/README.md`.* Left, structural, by expected gain: inline strings (≤ 23 bytes) and one-unit strings compared as code units (`calc`, `sexp`, `strlist`, `words`, `records`, `respond`); `T?` of a reference as a nullable pointer (`tree`, `binary-trees`: nodes from 40 to 24 bytes); pools for the boxes of a recursive value freed at once (`binary-trees`, `interp`, `calc`); field layouts narrowed by ranges the analyzer proves for every store (`lru`: `Integer` fields that hold indices); map entries without a stored hash and 32-bit index slots (`lru`, `words`); `scene` (Transform copies by value).
- Bun: `push` exceeds a 4 GB cap through the generator's dead bigints, and the 64-bit generator compiled to `BigInt` dominates the Goose-suite times on Bun (inline shifts helped; see `design/decisions.md`). Next step: 64-bit values as two 32-bit numbers in JavaScript, at least in functions that only use bitwise operations.

## GrowingArray native layout (decision of 2026-11-15, `design/decisions.md`)

`GrowingArray` stays a segmented list; see "GrowingArray native layout" and the `:::implementation` section of `collections/growing-collections`. The first item is *in progress* (native Goose shift); the others are not assigned.

- **Segmented fast paths** (*in progress*): cached cursor and end pointer for `push`, block copies per segment, index checks hoisted out of loops, `x = []` reusing the segments.
- **Measure**: `benchmarks/string-building` with a `GrowingArray<Byte>` against `Array<Byte>`, `String` and Goose; add `benchmarks/growing-index` (random reads and writes by index, 16 and 1,000,000 items, against `Array<Integer>` and Goose's `T[>..]`). Record the ratios in `design/native-goose-performance.md` and apply the thresholds of the decision (closed at ≤ 1.10 times `Array` and not slower than Goose; otherwise profile).
- **No more flat views**: the read-only methods of a `GrowingArray` (`join`, `includes`, `indexOf`, `map`, `filter`, slices…) run on the segments instead of on a copy (`growing_view`, `fa_segmented_flatten` in `lower/growing.rs`); keep the flat copy only where an `Array` is produced.
- **`k` per array**: store the exponent of the first segment in the header (loaded once before a loop); lower `GrowingArray<T>.from(items, capacity = n)`, `capacity` and `reserve` (not lowered for `GrowingArray` today) so that the first segment holds the capacity; a copy of a `GrowingArray` takes one segment of its length. Check that `push` and indexing of default arrays don't get slower.
- **Smaller header**: 400 bytes today (48 inline segment pointers); keep a few inline and move the others to a table allocated on demand, or bound the count by the target (wasm32 needs at most 32).
- **`x = []` for the other growing collections**: `GrowingPool`, `GrowingColumnar`, `GrowingMap` and `GrowingSet` keep their chunks, pages and index when an empty collection is assigned to a `mutable` variable or field. Analyzer: check that `clear` on a growing collection says to assign `[]`.
- **Conditional, only if the thresholds trigger it**: a reserved first segment (1 GiB of address space, pages committed on demand, at most 64 per process, fallback to an ordinary segment) for the arrays the compiler sees as alone and long-lived, on 64-bit native targets only.

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
