# Journal

The lead's log: one entry per large step, newest last. Decisions are in `design/decisions.md`, open work in `design/todo.md`.

## 2026-11-15 — One trunk, a clean repository, the inventory

### The mandate

The user handed the project to the lead (Claude Fable 5.2): architecture, language design and orchestration, every remaining decision recorded in `design/decisions.md`, a report after each large step. Goal of the phase, the base of the language: the native compiler and the JavaScript compiler complete; Fa ahead of Goose (or within 10%) on every Goose benchmark and ahead of Rust; clean and optimized JavaScript output; clear `fa check` errors; standalone file, package and workspace all working. Engineers are GPT 6.1 Sol agents ("high" for short tasks, "xhigh" for longer or harder ones), Claude only where needed.

### What was found

Two lines had diverged since 2026-11-04: `compiler` (515 commits: analyzer rules, AArch64 and wasm libraries, proofs, native layouts, the parallel pipeline) and `perf/native-goose` (41 commits of 11-14/15: string building, `GrowingArray`, capacity, frame and static collections, caller destinations, and the user's decisions on strings and generic angle brackets). A trial merge conflicted in 103 files (about 230 real hunks; the rest in generated IR). Both lines had implemented the angle-bracket generics. About 70 branches, 11 stashes and six worktrees with uncommitted work (the only copy of the explicit `copy`, provenance, unique resources, `with`, shared handles and generic pointer work) surrounded them.

### What was done

- **Trunk** (`integrate/trunk`, merge `109864c2` and eleven follow-ups up to `09db93f0`): `compiler` is the base of the parser, the analyzer, the interpreter, the formatter and the JavaScript backend, with the other line's behaviors and tests ported; the native backend keeps both sets of lowerings (static/frame storage ported onto the compact table layout, result slots and caller buffers through units, compositions, C wrappers and generic instances). Seven contradictions were settled by date, the decisions of 2026-11-15 winning (generic function types only as parameters of `function` declarations: `GN030`; no line break as a separator in a generic list; `reserve` on maps and sets reserves entries and index; capacity hints evaluated on JavaScript). A miscompilation present on both lines was fixed (`pairWith(1)(2)` on a generic function). See "Trunk integration" in `design/decisions.md`.
- **Validation of the trunk**: fmt, clippy (0 warnings), `cargo test --workspace` with LLVM (1,348 passed, 8 ignored as before), 86 IR checks, native programs 789/769/701 passed on native/wasm/freestanding with 0 failed, 300 runtime tests, website checks, ASan/UBSan of the runtime, 29 of 29 benchmark outputs equal to Rust's. No timing yet.
- **Repository**: 70 branches down to the trunk lines plus `pending/*` (validated work to integrate: the native JIT, map probes and frame map headers, split streams and two soundness fixes) and `wip/*` (the six snapshots, committed as found); superseded branches and every stash kept under `refs/archive/*`.
- **Inventory** (static audit): a support matrix per feature and target and fifteen work packages. Main gaps: 19 native expected skips and 5 compile-time cases skipped natively; 116 pending analyzer cases and 38 diagnostic codes without emitter; `with` implemented nowhere; the string redesign (UTF-16 semantics, `Bytes`, no `Character`) documented but not implemented; `std.http` documented only; workspaces and dependencies untested on native and wasm.
- **Assessment of the unfinished ownership work**: no snapshot can be merged wholesale; 35 focused slices; stored or returned generic function values are dropped (`GN030`); the specification was never updated by that work.
- **JavaScript** (two branches ready for the trunk): `js/gaps` (guards of temporaries released on every exit, deep recursive values off the call stack, detached task arguments copied, cancelled worker calls skipped, quadratic list prepend fixed, optional enum comparisons, console spreads, tuple pools) and `js/node-web-harness` (every program runs on Bun, Node and a web sandbox; ratchets: 4 failures on Node, 9 on web).

### Decisions of the lead

Recorded in `design/decisions.md` ("Lead decisions, 2026-11-15"): the trunk and the branch namespaces; the web sandbox; the ten open questions of the ownership work (defaults of a started function evaluated at creation; `throw` inside `do … catch` is caught by it; no implicit copy in a function adapter; conditional effects at specialization; finalizers bind their written owner); engine-independent JSON errors; `Pointer.allocate` refused at build time on Node and web.

### Next

1. Review of the merge by a second engineer; integration of `js/*` and `pending/*`; the trunk becomes `main`.
2. A full baseline against Goose and Rust on the trunk (CPU time and instructions).
3. Language semantics before optimization: strings, then ownership (specification first), each on its integration branch.
4. Completeness (native skips to zero, analyzer pending cases, interpreter), then performance, then `fa check` quality and speed and the three project shapes on every target.

## 2026-11-16 and 17 — Strings land, ownership is specified, the reviews earn their keep

### Landed on `main`

- **The string redesign** (`af334be5`): strings have UTF-16 semantics (lengths, indices, slices and search positions in code units) over UTF-8 storage natively (WTF-8 with a lone surrogate), `Character` is gone, `Bytes` holds binary data with explicit UTF-8 conversions. Five engineers (analyzer, JavaScript, interpreter, native with its C runtime, integration), one review, one fix round. ASCII pays nothing: every benchmark stays within +1% of the trunk's instruction counts. A generated differential corpus now compares every string and `Bytes` operation on seven engines (the compile-time interpreter, Bun, Node, the web sandbox, native, wasm, freestanding) against JavaScript's own answers: a quick tier in the regular suite, the full tier (122 shards) before any change to the string code.
- **`InlineArray<Item, capacity>`** (`caeec90c`): a bounded list stored in its owner, on every target, with a generated matrix of item kinds × positions × operations. Two reviews, two fix rounds: the matrix found 33 failing programs out of 39 at its first run.
- **Native map optimizations** (`71af4fd0`): probe and slot reuse, frame headers of private maps, inline 64-bit slots. `lru` measured at 0.986× Goose with a passing control.
- **The ownership specification and `copy`** (`0505d967`): collections and recursive values are never copied silently (a move at the last use, otherwise `copy`); tasks, resources and built-in lifetime owners are unique; `Mutex`, `Channel` and atomics share; `copy` is a reserved prefix operator. The enforcement is in progress on `feature/ownership`.
- **The measurement driver and the baseline** against Goose and Rust (one driver, generated tables, a same-binary control), the audit of the sixteen Goose ports with the faithful unreserved, direct-output and open-table ports, and the first optimizations of recursive aggregates.

### Validated, waiting to land in order

`links/spec-references` (the specification of links, `REF010` everywhere, reference parameters and results of user functions on every target, a generated matrix of reference programs; two reviews, two fix rounds), then `perf/arrays` (constant remainder, boolean block fills, `GrowingArray` caches, shape and alias facts for `nbody`; one review, one fix round, which also fixed a wrong remainder of narrow unsigned integers that was on the trunk).

### What the days taught

- **Every compiler branch is reviewed before it lands.** Five reviews, five times critical defects in branches whose whole suite passed (a compiler stop on a common async pattern after the trunk merge, memory corruption in generic uses of `InlineArray`, a generic function receiving a map with a layout it didn't expect, references that outlived their collection, a double release of an empty string with reserved capacity).
- **Generated matrices and differential corpora beat hand-written reproductions.** They are kept in the repository as tests.
- **Several Fa ports of Goose's benchmarks were not Goose's programs** (recursive owning values where Goose links the nodes of one pool, three recursive walks where Goose scans the pool). Fa could not express that structure; it now has a decided design for it.

### Decisions of the lead (all in `design/decisions.md`)

Strings: literal replacements in `replace`, thirteen edge cases, well-formed literal types, lossy decoding of external text, `\uXXXX` for lone surrogates in JSON, the native view and compact formats. Ownership: `String` is an ordinary value and `Bytes` a collection; recursive values are structures; partial moves of field paths; common members of unions without narrowing; unique built-in handles; error payloads are data; a closure never consumes what it captures and captures only the paths it uses; one environment per declaration; conversions never convert the inside of a structure; `copy` reaches only its operand; an error never rolls back completed effects, and a failed sort keeps the original order; the enumeration order of fields depends on the static type alone. Planned for the next ownership step: a function takes ownership of a parameter through the `invalidates` effect. Collections: **pool links** (`link` fields into growing pools inside a complete owner, `InlineArray`, an `Index` parameter on the target collection), and the rule that a port of a Goose benchmark uses the corresponding Fa collection and does the same passes. Process: one CPU reserved for measurements, reviews before merges, landings one at a time.

### State against the goals

Goose's suite on the trunk before these optimizations: 8 met, 6 behind, 2 unresolved (`scene` 2.76×, `sexp` 1.46×, `interp` 1.43×, `respond` 1.29×, `records` 1.19×). Since: `lru` met; `scene` near 1.6× with its recursive port. The seven pool-shaped benchmarks wait for links; the string benchmarks can now be optimized on the final representation (`string-building` about 1.55× Goose). Against Rust, `nbody`, `sieve` and `mandelbrot` remain behind by a few percent to 12%. The machine's noise leaves many CPU verdicts unresolved; hardware counters are not available.

### Next

Land links 1a and the array optimizations; finish the ownership enforcement (moves, `MEM004`, partial moves, the repository migration), then its second step (generic conditions, owning parameters, unique resources on both backends, `with`); implement the `link` contract, the compact native pools and the seven pool ports; string construction and comparison on the new representation; the native expected skips and the analyzer's pending cases; `fa check` quality and speed; the three project shapes on every target.
