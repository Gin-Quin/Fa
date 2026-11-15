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
