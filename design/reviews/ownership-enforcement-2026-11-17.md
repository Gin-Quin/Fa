# Ownership enforcement (`feature/ownership`): design review, verdict and slicing plan

Review of `feature/ownership` at `0807b0de` (2026-11-17), 118 commits on top of `main` `5e3b602d` (the merge base: nothing to merge). Written for an architect who has not seen the branch. The branch was not changed by this review; its worktree is clean.

Method: reading the branch's diff (`git diff main...HEAD`), small probes with the branch's `fa` binary against a trunk binary (the build of the `ownership/set` worktree: `main` plus 8 Set commits; Bun and native JIT), and three delegated read-only reviews (interpreter, native and JavaScript generators, fixture migration) whose findings are folded in below (their full texts are kept outside the repository, in `/home/claude-dev/fa-lead/reports/ownership-2-stage1-appendix-{interpreter,backends,migration}.md`). No complete suite result is available: the takeover run was stopped on the lead's instruction after `c_library` 5/5, `foreign` 3/3, `format_memory` 2/2. The last complete-ish run is the previous engineer's at `32cf081b` (two commits earlier): CLI programs 11 passed / 5 failed (causes in §7), analyzer units 615/615, analyzer cases 3/3; the native and interpreter suites were not reached.

## 1. Verdict

1. **Do not merge the branch, and do not try to finish it.** Its core (a control-flow graph per body and a backward liveness that sets the `MOVE` flag the engines already read) is the right mechanism and gives correct answers on every local-variable probe I ran. But it was bolted on **next to** the mechanisms it should have replaced, and more than half of the analyzer code (about 2,900 of 5,500 lines, tests excluded) is an interprocedural machinery for closures, module owners and tasks that a simpler rule makes unnecessary.
2. **It is not one general mechanism.** After the branch there are four answers to "is this the last use?" and two to "was this used after a move?", selected by type category and by backend (§2.2). The user's requirement of 2026-11-17 (one move fact for every value that owns storage) is not met by construction: the new pass only looks at whole local names whose type is a structure or unique (`check/moves/concrete.rs:47-55`).
3. **Port about a third, redo the rest smaller.** Worth porting as is: the type categories, `MEM005`, the graph, the diagnostics with their machine fixes, the conversion rule, the specification pages and most fixtures as acceptance tests. To redo: everything about closures, module owners and tasks (about 2,900 lines of analyzer code → about 400 with the "borrower" rule of §4), and most engine changes, which are a second project (a loan-based interpreter, new closure environments, a default-argument ABI) hidden inside this one. The slicing plan (§6) brings the enforcement to `main` in 23 branches of a day or less; the first seven (A1–A4, C3, E1, F5) add no ownership error on existing code.

**A bug of `main` found on the way** (independent of this branch): the textual last-use rule of `main` moves a module-level array that a function still reads through a stored function value; the program of probe `g2` (§8) prints `4 4` on Bun instead of `3 4`. Slice A2 fixes it; until then it is a miscompilation on the trunk.

## 2. What the pass computes and how the engines consume it

### 2.1 The pass

Entry: `Walker::finish_moves` (`packages/analyzer/src/check/moves.rs:167`), called at the end of a file's walk.

- During the walk, the walker records per node: the locals a node reads (`move_reads`), the places it reads (`move_places`), whole-slot replacements (`move_writes`), store positions (`copy_sites`: `note_copy_site` from declarations, assignments, returns, `push`/`add` arguments, detached-start arguments), ends of unique lifetimes (`lifetime_ends`), and several origin tables for closures (`ownership_read_origins`, `ownership_capture_origins`, `callable_reads`, `callable_captures`, `capture_places`, `callable_calls`). 16 new fields on the walker (`check/mod.rs`).
- At the end, `finish_concrete_moves` (`moves/concrete.rs:14`): (1) adds the store positions that the walk does not see (object fields, array and tuple items, `throw`, exported values, defaults: `collect_store_values`, a scan of every node); (2) builds the complete read table `ownership_reads` (`concrete.rs:210-438`: a fixpoint that adds, at each call, the outer places the callee reads, transitively, with defaults, callbacks and callable values); (3) for each body (module root, function bodies, defaults, `start` blocks, watches, tests) builds a `Graph` (`moves/graph.rs`: one event per node that reads, defines or can fail; edges for sequence, branches, loop back edges, `catch` failure edges, `return`/`break`/`continue`); (4) for each stored local, one backward traversal (`Graph::later_uses`) that says, for each store event, whether a later read exists: if none and the source is an owned local of this body, the read gets `NodeFlags::MOVE` (and `moved_reads` when unique); (5) checks escaping closures' captures on the same graph (`moves/captures.rs`), (6) reports `MEM002` with a forward traversal from each move (`moves/validity.rs`).
- `finish_copy_checks` (`moves/copies.rs:203`) then walks the store positions: a structure or unique read that is a place read (`copy_reads_place`) and has no `MOVE` gets `MEM004` (structure) or `MEM002` (unique), with the machine-applicable `copy` edit; `copy` of a unique value gets `MEM003`.
- Last, the **old** loop of `main` still runs (`moves.rs:170-188`): the textual last use (`last_uses`, walk order, same loop and function depth) sets `MOVE` for ordinary values that own memory (strings, plain objects, tuples).

Facts published in `Analysis` (`analysis.rs`): `NodeFlags::MOVE`, `moved_reads`, `capture_moves`, `NodeFlags::BORROWED_OWNER`, `argument_ownership`, `copy_inputs`, `capture_types`, `named_function_dependencies`, `function_capture_states`, `named_function_readiness`, `ownership_sources`, `ownership_source_graph`, `kept_parameters`, `unknown_kept_parameters`.

### 2.2 Is it one general mechanism? No

"Is this read the last use?" has four implementations after the branch:

| # | Mechanism | Where | Applies to |
| --- | --- | --- | --- |
| 1 | Textual last use, walk order, loop/function depth (from `main`) | `check/moves.rs:34-73`, `:170-188`, `owns_memory` `:193-207` | ordinary values owning memory (String, plain objects, tuples) |
| 2 | Backward liveness on the control-flow graph (new) | `check/moves/graph.rs`, `concrete.rs:44-93` | whole local **names** of structure or unique type only (`concrete.rs:47-55`) |
| 3 | Textual last use in the native backend (from `main`, kept) | `codegen-llvm/src/lower/ownership.rs:2274-2281` (`is_returned_local`, `is_last_use` `:2444`), `collection_storage.rs:330` | any local slot when the `MOVE` flag is absent |
| 4 | A whole-program predicate in the native backend that decides whether mechanism 2 may be trusted for module slots (new) | `codegen-llvm/src/lower/ownership.rs` `closed_module_transfers` (about `:2285-2340`), used `:2249` | module-level owners: only when the program has no escaping capturing function, no `start`, no watch, no type declaration, no `use`, no `await`, no namespace export… |

"Was this value used after a move?" has two: the forward flow state of `main` (`FlowState::consumed`, `flow/mod.rs:285-290`, read by `check_consumed` in `check/reactivity.rs:264-345`, written by `check/foreign.rs:935-1023`, `check/tasks.rs:1350`, `check/concurrency.rs:1141`; it already does partial field moves for unique values) and the new forward traversal of the graph (`moves/validity.rs:43-106`). Both report `MEM002`; duplicates are removed by searching the diagnostics already emitted for the same span or for a message containing the variable's name (`validity.rs:119-129`).

Other signs that the pass is an accumulation rather than one mechanism:

- **Deduplication by scanning the output**: eight sites test `self.output.iter().any(|d| d.code.id == … && d.location.span == …)` before reporting (`moves/copies.rs:39, 254, 267, 329, 411, 425, 512`; `validity.rs:119`). Several emitters reach the same store; none owns it.
- **Three veto tables around the liveness** instead of events in it: tasks that still borrow a source (`tasks.ownership_loans`, `check/tasks.rs:698-785`, which also clones the set of live tasks at every name read, `:698-706`), exported functions that read a module owner (`module_owner_loans`, `moves/globals.rs`), captures of function values (`pinned_captures`, `moves.rs:157`). Each clears `MOVE` after the fact and sets `BORROWED_OWNER`.
- **The same computation twice, with different inputs**: "which outer places does this body read" exists as the `free` map (`concrete.rs:311-435`), as the published `ownership_source_graph` (`moves/readiness_sources.rs`), as `unconditional_borrowed_sources` (`moves/borrowed_calls.rs:78`) and as `callable_captures` (`check/callable_values.rs`); `concrete.rs:316-346` reads the source graph when its entry is settled and otherwise falls back to the symbol table, with the same 15 lines written twice (`:328-343` and `:356-370`).
- **Three alias tables**: `self.aliases`, the reference checker's `ref_state(...).places`, and the new `FlowState::ownership_aliases` (`flow/ownership.rs:120-172`); `moves/origins.rs:11-31` consults them in that priority order.
- **Lists of builtin names in the pass**: receivers consumed by `Subscription#stop`, `Worker#stop`, `WorkerPool#shutdown`, `TcpListener#close`, `TcpStream#close` (`concrete.rs:287-293`); callable results of `Array#get`/`find`/`pop` (`callable_values.rs:464-479`); retention by `Channel#send`/`WorkerPool#submit` (`retention.rs:65`). These are properties of the builtins' signatures, not of the pass.
- **Per-form syntax tests** decide what a "place read" is (`copies.rs:465-486`: names, accesses, non-range indexes, calls that return an element view), with an exception for unique fields of fresh owners (`:474`, `:488-505`).
- **Cost**: `Graph::build` has five call sites (`concrete.rs:45`, `readiness.rs:63`, `readiness_calls.rs:378`, `versions.rs:72` once per replaced captured variable, `globals.rs:106`); the liveness is one traversal per stored local rather than one bit-vector pass; eight loops over every node of the tree run at the end of each file (`concrete.rs:37, 147`, `copies.rs:205`, `globals.rs:31`, `readiness.rs:22`, `readiness_calls.rs:209`, `source_facts.rs:70`, `retention.rs:337`); `TypeStore::ownership` re-traverses the type graph at every query (`types/ownership.rs`: "no incomplete answer is cached"). None of this was measured; Fa's second goal is a fast checker.
- **Ordering hazard**: compile-time evaluation runs during the walk, before the move facts exist, so the lowered compile-time code is invalidated and lowered again at the end (`check/mod.rs`: `walker.s.comptime.invalidate_functions()`, "Demand-time code precedes these final ownership facts").

### 2.3 What is right in it

- **The categories** (`types/ownership.rs`, 288 lines, plus `conversions.rs` 286 and `projection.rs` 178, tests 652): one query `ownership(ty)` → Ordinary / Structure / Unique / SharedHandle / Dependent / Unresolved, an iterative traversal of owned data that stops at references, synchronization handles and function signatures. Clear, total, tested. Keep (add a cache once declarations are complete).
- **The graph** (`moves/graph.rs`, 478 lines): branches, loop back edges, failure edges to the lexical `catch`, `return`/`break`/`continue`, store-commit events placed after the stored value's evaluation. On my probes (store then read, store in a loop, exclusive branches, replacement inside a loop, failure after a move, a function declared before or after the store, a callback passed to another function, an alias of a field, a module owner read by a function stored in an array) the branch refused or accepted correctly, and accepted programs printed the same on Bun and natively. The trunk has an unsound move that the graph fixes: a module array stored into a `mutable` while a function kept in an array still reads it prints `4 4` on trunk's Bun instead of `3 4` (probe `g2`, §8).
- **The diagnostics and fixes** (`copies.rs:507-568`, `explicit_copy_edits` `:65-156`, `capture_fixes.rs`): the message names the later use and its line, the fix is machine-applicable and tested by applying it and checking again.
- **The specification work** (documentation +361 lines, 17 pages) and the fixtures (74 new `.fa` files).

## 3. File by file

Sizes are code lines (tests excluded) / total lines on the branch.

### 3.1 Analyzer

| File | Size | Verdict | Reason, target size |
| --- | --- | --- | --- |
| `types/ownership.rs`, `ownership/conversions.rs`, `ownership/projection.rs`, `ownership/tests.rs` | 750 / 1,400 | **Keep as is** | The one category query; add a cache. |
| `check/moves/graph.rs` | 478 / 478 | **Keep, generalize** | Right mechanism. Events must carry places (owner + field path), not symbols, and every storage-owning type; add events for borrowers (§4). About 550. |
| `check/moves/validity.rs` | 139 / 256 | **Keep, then make it the only one** | Forward pass for `MEM002`; must replace `FlowState::consumed`, not sit next to it. About 150. |
| `check/moves/copies.rs` | 571 / 1,114 | **Rewrite smaller** | Keep `report_structure_copy`, the copy edits, `MEM003`; one emitter per store with a `Form`, no output scanning; move conversions to their own file. About 350. |
| `check/moves/concrete.rs` | 680 / 1,049 | **Rewrite smaller** | `finish_concrete_moves` (85 lines) is the pass; the 230-line `ownership_reads` fixpoint and its helpers go with the closure machinery. About 200. |
| `check/moves.rs` | 208 | **Rewrite smaller** | Recording helpers stay; the textual `last_uses` loop and `owns_memory` go when the graph covers ordinary values. About 100. |
| `check/moves/captures.rs` | 293 / 609 | **Rewrite smaller** | Capture stores are stores at the creation event; `MEM003`/`MEM004` messages stay. About 150. |
| `check/moves/capture_fixes.rs` | 123 / 200 | **Keep** | The snapshot fix (`let snapshot = copy value` before the function). |
| `check/moves/readiness.rs`, `readiness_calls.rs`, `readiness_sources.rs`, `source_facts.rs` | 943 / 1,590 | **Drop** | "Has execution reached the declaration of this local function, in borrowed or owned mode, per caller position", published in `Analysis` and read by no engine (`ownership_sources`, `named_function_readiness`: analyzer and tests only). Replaced by the syntactic hoisting rule the lead already decided plus the borrower rule. |
| `check/moves/origins.rs`, `flow/ownership.rs` | 320 / 600 | **Drop** | A third alias table. Use the reference checker's designations. |
| `check/moves/versions.rs` | 112 / 142 | **Drop** | "A function that replaces a captured variable before reading it does not observe the incoming owner": precision for one shape (direct assignment, `mutable`, block body), one graph build per variable. |
| `check/moves/borrowed_calls.rs` | 180 / 180 | **Drop** | Witnesses for one diagnostic of the capture check; subsumed by borrowers. |
| `check/moves/globals.rs` | 129 / 129 | **Drop** | Module end becomes an event that reads what exported and escaping functions read. |
| `check/callable_values.rs` | 688 / 829 | **Drop** | Tracks which function bodies a value may call through fields, arrays and returns, to place reads at indirect calls. With "a function value borrows its captures while it is live", the value's own liveness answers. |
| `check/capture_handles.rs` | 152 / 378 | **Fold into the closure-environment slice** | Which named functions an environment must carry; an engine concern (native environments), about 100. |
| `check/retention.rs` | 459 / 657 | **Keep, own slice** | "May this callee keep its callback argument": a per-parameter summary published in interfaces. Independent of moves; it refines `escapes`. Replace the builtin-name list by a flag on the builtin signatures. About 350. |
| `check/tasks.rs` (+200 −75) | — | **Rewrite smaller** | Task loans (`:698-785`) become borrower events; the Task identity rewrite of `0807b0de` (+331) is unfinished (its own probe is still accepted). |
| `check/foreign.rs`, `reactivity.rs`, `concurrency.rs` (consumed values) | — | **Fold into `validity.rs`** | The old `consumed` flow state and the new pass must become one. |
| `lower/copies.rs` (+247), `comptime/copy_plans.rs` (199), `comptime/borrows.rs` (119), `comptime/lower.rs` (+421 −140), `comptime/mod.rs` (+382) | — | see §3.2 | |
| `analysis.rs` (+199) | — | **Rewrite smaller** | Publish `MOVE`, `capture_moves`, `copy_inputs`, `argument_ownership`, retention flags. Remove `ownership_sources`, `ownership_source_graph`, `named_function_readiness`, `function_capture_states`, `BORROWED_OWNER`. About 60. |

### 3.2 Engines

`N:` is `packages/codegen-llvm/src/lower/`, `J:` is `packages/codegen-js/src/generate/`. The engine reviews are from reading only (nothing was built for them); I checked the citations marked ✓ myself.

**How a move is consumed today.**
- Native: `take` moves when `movable_local` answers, else calls `emit_copy` (`N:ownership.rs:1636-1644` ✓). `movable_local` accepts the analyzer's `MOVE` flag and, without it, its own textual test (`N:ownership.rs:2274-2281` ✓). The primitive is general: `move_owned_slot` (`N:ownership.rs:1648`) zeroes the slot and clears its drop flag. Around it there are about ten transfer paths with their own guards: `take`, `take_local`, `give_owned`/`give_up_place`, the `moved_reads` reads (`N:objects.rs:1850`, `N:mod.rs:3149`), the capture transfer (`N:closures.rs:322-374`), map entries (`N:tables.rs:1640`), defaults (`N:defaults.rs:155-180`), fields of temporary objects, compositions, the return slot. Three end in a per-case `unsupported(...)`.
- JavaScript: `is_move` skips the copier; `moved_reads` in `J:resources.rs:434, 471`; `capture_moves` in `J:references.rs:1929`.
- **A store with no move fact still deep-copies silently on both backends, through the same path as on `main`.** Neither backend asserts that the analyzer refused it. The task's goal "no engine copies a structure implicitly" is therefore a property of the analyzer's errors only; the only copy path removed is the in-place item copier of array assignment (`fa_array_assign` is now unreferenced in `runtime/fa_runtime.c` ✓).
- Explicit `copy` is a pass-through in both generators (`N:mod.rs:3068` ✓, `J:expressions.rs:247` ✓): the copy is made at the result leaves that have a `copy_inputs` entry. A leaf without an entry would yield an alias with no check.
- Facts that no backend reads: `capture_types` (documented as required for capture materialization; both backends use the declared type), `named_function_readiness`, `ownership_sources`, `kept_parameters`, `unknown_kept_parameters`. `BORROWED_OWNER` exists only to veto the native backend's own last-use test (`N:ownership.rs:2228`). `argument_ownership` is true only for Task-owning parameters (`check/calls.rs:403` ✓), so the `Structure` arm added to `give_owned` is dead.

**What the native additions are** (about 2,900 lines added):

| Share | Lines | Work | Needed by moves? | Verdict |
| --- | --- | --- | --- | --- |
| 39% | ~1,120 | Converting copies between layouts (`converting_copies.rs` 855 and hooks) | no (the conversion rule) | **Rewrite smaller**: about 350 lines folded into the existing copy generator; `build_converting_copy` is 238 lines enumerating every representation, next to the existing copier (two copy generators per backend). |
| 20% | ~580 | Counted closure environments bound at the declaration (`environments.rs` 174, `closures.rs`, `compose.rs`, `generic_values.rs`, task lease) | no (the closure decision) | **Keep**: it deletes about 340 lines of the old scheme; land alone. Cost to measure: every indirect call of a stored closure now retains and releases its environment (`N:calls.rs:129-135` ✓). Calling an escaping local function before its declaration is `unsupported` (`N:closures.rs:113-116` ✓). |
| 12% | 353 | `sorting.rs` (about 250 moved, 100 new: staging when the comparator may throw) | no | **Keep**, with slice F5. |
| 10.5% | ~305 | Default-argument binding (`defaults.rs` 257 and hooks in `expr`, `own`, `owned_as`: `N:mod.rs:2726-2741`, `N:ownership.rs:1688, 2198`, with a compile-time counter of emitted selections to detect skipped leaves) | no | **Drop and redo**: about 80 lines at the parameter prologue. The notes record a double finalization found the same day. |
| 6.5% | ~185 | Comparison without conversion (pair comparators) | no | **Keep** after checking that a lookup hashes the needle like the table does (`N:tables.rs:1257` against `:779-804`: sets with different key widths may compare unequal). |
| 4% | ~110 | Reference and callback fixes | no | Keep. |
| 8% | ~230 | **Move consumption** | yes | **Redo**: `closed_module_transfers` and `binding_may_move` (`N:ownership.rs:2289-2343`, whole-file scans at each call) are dropped; the capture transfer becomes about 15 lines through `take` (today it skips `escape_owned_storage`, so a moved `Bytes` view may enter a heap environment without being owned: to verify); catch bindings and `same_layout` lines are kept. |

JavaScript additions (about 445 lines in the generator, 257 in the runtime): converting copies 66% (`converting_copies.rs` 205 → about 100; `runtime/copy.js` allocates a job array per item and per field, primitives included: back to `main`'s loop), default resource flags 17% (redo with F2), sort variants 8% (F5), captures 5% (captured immutable ordinary composites are now always snapshotted, `J:references.rs:1926-1934`: a cost to measure).

**Minimal generator change for whole-value moves and explicit `copy`**, with the analyzer as the only authority: native 100–200 lines and net negative once `is_last_use` and `BORROWED_OWNER` go (plus about 350 if `copy` converts layouts); JavaScript 20–40 lines (plus about 100 for conversions).

Overall for the two generators: about 45% is worth porting as is, nearly all of it independent of enforcement (closure environments, sorting, comparisons); about 45% is to redo smaller (converting copies, defaults, move consumption); 10% to drop.

**Interpreter (compile-time evaluation).** 22 files, +5,278 −1,283, of which about 3,450 lines of code. Paths under `packages/interpreter/src/` and `packages/analyzer/src/comptime/`.

- On `main`, values are `Arc` composites with copy on write, places exist as `Value::Reference { frame, generation, slot, keys }` (18 uses; the reference work that was reviewed and landed), arguments are passed by reference, callbacks run on a snapshot of their receiver, and `copy` is a plain read. In such a machine **a move is not observable**: it only avoids a later copy.
- The branch **deletes `Value::Reference`** (0 uses ✓) and replaces it with its own place model: every result is `Out = Value | Loan` (`loans.rs:296`), loans carry a path with 6 step kinds and 6 view kinds, new operations `Move`, `Copy(plan)`, `Own`, `Lend`, `Bind`, `CurrentFunction`, loops and callbacks lend the original items, callbacks became position-based with "effects before errors", and every public result goes through a publication walk (`publication.rs`) because closures' environments now hold loans inside values. This was built in parallel with `main`'s reference landing and then merged over it. `machine.rs` went from 1,530 to 2,832 lines ✓, with 33 `"internal: …"` stops against 0 on `main` ✓.
- The move itself is `Op::Move`, a type-blind `mem::replace` (`machine.rs:416-424` ✓), read from `MOVE`/`moved_reads` at one place (`comptime/lower.rs:594-600` ✓); `capture_moves` at `lower.rs:1559`; `copy_inputs` at `lower.rs:788-805`.

| Part | Code lines | Share | Verdict |
| --- | --- | --- | --- |
| `Move` and capture transfers | ~70 | 2% | **Keep** (about 45 lines on today's `main`). |
| Converting `copy` (`copies.rs` 244, `comptime/copy_plans.rs` 199, IR) | ~405 | 12% | **Keep**, with slice C1. To fix: the planner maps Pool and Columnar to `CopyStep::Array` (`copy_plans.rs:161-168`) while `copies.rs:103-106` rejects `Value::Pool`. |
| Shared helpers in `analyzer/src/lower/copies.rs` (result paths, place roots) | ~177 | 5% | **Keep**: used by the checker and the three engines. |
| Loans replacing `Value::Reference` (`loans.rs` 331, most of `machine.rs`, `comptime/borrows.rs` 119) | ~1,500 | 43% | **Drop from this work.** One place model must exist, and `main` has one. If loans are better, that is a proposal of its own, replacing `Value::Reference` in one reviewed branch. Today both lowerings coexist in `comptime/lower.rs` (`borrow_value`/`mark_reference_result`/`receiver_place` at `:1278-1462` and `bound`/`borrowable_place`/`borrowed_receiver` in `borrows.rs`), and three operation pairs are identical in the machine (`Lend` = `Borrow(_, false)` `machine.rs:769` ✓, `Own` = `Dereference` `:428` ✓, `Bind` = `Let`). |
| Callbacks and loops over original storage, failure effects, sort staging (`callbacks.rs` 432) | ~750 | 22% | **Land separately** (slice F5): it is the "an error never rolls back completed effects" decision, not ownership. |
| Publication walk, written argument order, `CurrentFunction`, projection copies, the revisioned code cache, live ranges | ~550 | 16% | **Drop** `publication.rs` (177; it exists only because loans sit inside closure values) and the wrappers without callers outside tests (`builtins::call`/`mutate`, `Machine::mutate_builtin`); the argument-order fix is a small change of its own. |

Other findings of the interpreter review, by reading only: `BorrowGlobal` pushes a root that is never popped (`machine.rs:432-450`) while `release_root` demands last-in-first-out order (`:2047-2052`): a module-value lookup inside a `for` should stop with an internal error; every expression of an ordinary object, tuple or union type is wrapped in a `Copy` (`comptime/lower.rs:802-812` ✓), so `point.x` rebuilds `point` at each read; special cases by builtin name in the machine (`Pool#delete/take`, `Columnar#delete/take` `machine.rs:912-956`, `Array#reverseInPlace` `:1033`, `Json.parse` `:1392, 1415`, `#sort/#filter/#map` `:1431-1446`); `Container::{Array, Set, Map, Pool}` switched on in seven places (`:1391-1685`); functions of 261, 161, 156 and 160 lines (`enter`, `resume`, `finish`, `View::project`).

**Minimal interpreter change on today's `main`**: about 30 lines for whole-value moves, 15 for capture moves, nothing for `copy` without conversion (it works), about 400 for converting copies. That is 50 lines for moves and 450 with conversions, against 3,450 on the branch: **about 20% of the interpreter work is worth porting; 80% is a redesign of the machine's place model and of the builtins' failure semantics that should not land under this name.**

## 4. What a clean design would be

**One fact.** `MOVE` on a read (and `capture_moves` for a capture, which has no read node): *this read transfers its place*. A place is a local, or a field/tuple path of a local or of a temporary. It is computed for every type that owns storage; the category only decides what a read that is **not** a move means at a store: an ordinary value is copied (a cost, never an error), a structure is `MEM004` unless `copy` is written, a unique value is `MEM002`. This is the user's requirement of 2026-11-17 stated as a design rule: the liveness never looks at the category.

**One place where it is computed.** After the walk of a file, per body: the graph of `graph.rs` (built from the tree and from the per-node reads the walker recorded), one backward bit-vector liveness over the body's places, one forward may-be-moved pass for `MEM002` and for the drop obligations of partial moves. No other code decides a move: the textual loops of `moves.rs` and of the native backend are deleted.

**One rule for everything that reads later: borrowers.** A value that keeps reading other places is a *borrower*: a `use` alias, a reference result, a function value that captures without escaping, a kept task (it reads its arguments and captures until awaited or cancelled), an iteration over a collection. The rule: **a borrower's places are read wherever the borrower is read, and at the end of its own life**. Liveness of the borrower (which the same pass computes, since a function value or a task is a local like another) then gives the liveness of what it borrows. This replaces, with one table `borrows: local → places` filled during the walk:
- the transitive call fixpoint, the callable-value tracking, the readiness propagation, the source graph and the version proofs (a local function is live until its last call or use: its captures are read there);
- the task loans (a task is live until its `await`);
- the module-owner loans (an exported or escaping function is live at the module's end event);
- the three alias tables (an alias is a borrower).
It is less precise than the branch in one case — a function value that is still live pins its captures even if it will not be called again — and the diagnostic can say so ("`reader`, declared on line 3, still reads `items`"). An **escaping** function is not a borrower: its creation is a store of each capture (move at a last use, otherwise `MEM004` with the snapshot fix, `MEM003` for a unique capture), after which it owns its environment.

**One way for an engine to consume it.** At a store: `MOVE` → transfer the owner and cancel the source's drop; a `copy` node → deep copy (with the conversion of `copy_inputs`); otherwise → the value copy of an ordinary type. For an enforced form the third case cannot be reached with a structure, and the engine asserts it. Partial moves add one thing: the forward pass publishes, per owner, the paths that are moved out at each exit, so that the native target drops the rest exactly once.

**One switch.** `enum Form { LocalOwner, BorrowedSource, Field, Element, Pattern, Capture, ModuleOwner, Default, BuiltinStore, Conversion, Generic }` with `const fn enforced(self) -> bool` in one file, read by the single function that emits `MEM004`/`MEM002` at a store. Moves are computed whatever the switch says: a proven move is always valid. Turning a form on is a one-line change plus the migration of the fixtures that it breaks.

**Compared with the branch.**

| Part | On the branch (code lines) | Port as is | Redo smaller | Drop |
| --- | --- | --- | --- | --- |
| Analyzer: categories, conversions rule | 750 | 750 | — | — |
| Analyzer: pass (`check/moves*`, `flow/ownership.rs`, `callable_values.rs`, `capture_handles.rs`, `retention.rs`) | 5,500 | about 1,500 (graph, validity, diagnostics and fixes, snapshot fix, retention) | about 1,100 → 600 (stores, captures, liveness loop, task and module rules as borrowers) | about 2,900 (readiness, source facts and graph, origins, versions, borrowed calls, callable values, module loans) |
| Native lowering and runtime | 2,900 | about 1,300 (environments, sorting, comparisons: none needed by moves) | about 1,400 → 600 (converting copies, defaults, move consumption) | about 200 |
| JavaScript generator and runtime | 700 | about 100 | about 500 → 250 | about 100 |
| Interpreter and compile-time lowering | 3,450 | about 700 | about 750 (callbacks and failure effects, as their own branch) | about 2,000 (loans, publication) |

About 4,300 of 13,300 lines of compiler code (a third) are worth porting as they are, and half of that third is not ownership enforcement at all (closure environments, sorting, comparisons, retention) and can land first. The move enforcement proper — facts, switch, diagnostics, consumption by three engines — is about 2,500 lines in the clean design.

**Size.** Analyzer: 5 files, about 1,500 lines of code (graph 550, liveness and validity 350, stores and diagnostics 350, borrowers 200, switch 50), plus the categories (750) and retention (350) kept. The branch has about 5,500 lines of code in 19 files for the same job, plus about 3,300 lines of tests of which the diagnostic tests port and the tests of dropped tables do not.

## 5. The 367 touched files

107 added, 259 modified, 1 deleted (`packages/interpreter/src/references.rs`, the reference model of `main`: a source file, not a test).

| Class | Files | Lines |
| --- | --- | --- |
| Compiler sources (`packages/*/src`) | 131 (33 new, 97 modified, 1 deleted) | +20,175 −2,649, of which about 4,000 are test modules |
| Runtimes (C, checked-in IR, JavaScript) | 10 | +456 −183 |
| New `.fa` fixtures | 74 | +2,531 |
| Existing `.fa` fixtures migrated | 123 | +245 −228 |
| Rust test files | 7 | +674 −20 |
| Documentation | 17 | +361 −28 |
| Design records, benchmarks, other | 5 | +383 −13 |

By area: analyzer sources 74 files +12,765; interpreter 14 files +3,896; native lowering 25 files +2,899; JavaScript generator 12 files +379; language server and CLI 6 files +236.

**New fixtures** (74): closures, environments and callbacks 28; moves and borrows 10; `copy` and conversions 9; compile-time evaluation 8; callback-failure semantics 6; repros 5; module owners 4; pending tasks 2; defaults 2. Only 9 are negative: **the basic `MEM004` store forms have no negative `.fa` fixture**; they exist only as Rust unit tests in `check/moves/`. Each enforcement slice must add them.

**The migration: 215 `copy` inserted in 109 existing files** (198 in 101 `.fa` files, 3 of them in 2 benchmarks; 15 in Rust tests; 2 in a generator). No expected output changed, no test was deleted, no `-- pending:` was added (13 were removed, for cases that now pass).

| Pattern | Count | Is `copy` what a programmer would accept? |
| --- | --- | --- |
| A whole value kept twice (`let duplicate = copy values`), a parameter copied to mutate it, `duplicate` helpers, a live element or field kept next to its owner, detached-call arguments | 157 (73%) | **Yes**: two owners are needed. (The share is inflated: 93 of the 95 whole-binding copies assign to names like `duplicate`, `copied`, `snapshot`: tests of copy independence.) |
| A parameter stored, returned or projected: a constructor or `push` (10), a pass-through return (8), object shorthand (5), a pattern binding (4), a getter, an element | 29 | **No — forced whatever the implementation**: `packages/analyzer/tests/cases/…/mixed-recursive-storage.fa:20` `push(copy child)` on a temporary argument; `comptime/schemas.fa:31` `else => copy schema`; `returned-values.fa:19` `return copy box.items`. Language question Q1. |
| A part of a dying local or of a temporary: field, element and pattern reads (11), walks of recursive lists (3), a temporary (1) | 15 | **No — disappears with field moves** (slices D1–D3): `modules/boolean-fill.fa:7` `copy bucket.flags`; `arena-escapes.fa:25` `copy Tree.build(6).left`; `js-list-prepend.fa:11` `cursor = copy cursor.tail` (quadratic, times out; `memory-management.md:340-343` shows this very loop without `copy` as valid, and the unit test `partial_structure_moves_wait_for_field_support`, `copies.rs:762`, pins the opposite). These copies must not land. |
| Item-type conversions | 7 | **Doubtful**: `adv-10.8-07.fa:9` `logNames(copy superHumans)` deep-copies an array to pass it to a function that only reads it. Language question Q6. One looks like a checker defect: `recursive-collection-aliases.fa:54` copies a literal, `[copy (2, [])]`. |
| Compile-time constants (`let colors = copy Palette.Color.members`) | 5 | **No**: language question Q7. |
| `MEM004` on a line that already has a type error (`bytes-errors.fa:12-13`) | 2 | A cascade: suppress the ownership diagnostic where the store has a type error. |

The count is a lower bound of what full enforcement costs: construction stores (`slice`, `filter`, spreads…), patterns and generic instantiations are not enforced on the branch. No loop variable is stored anywhere in the corpus (0 occurrences).

**Changed expectations** (not code): three old codes became `MEM004` (`cv-11.fa:1` `CV005`, `ma-12.fa:1` `CL010`, `adv-10.8-08.fa:6` `STR006`: legitimate consequences of the conversion rule); `adv-10.8-07.fa:14` now expects two codes for one mistake (`MEM004, STR005`); one positive case became negative (a `Map<String, Hero>` passed to a reading `Map<String, Human>` parameter, `check/collection_tests.rs:47`); `task-moves-errors.fa:14-27` changed subject (tasks captured by stored closures became tasks passed to stored callbacks, since the capture is now `MEM003`); `std-io.fa` lost `stream.close()` in a handler and gained `closeBorrowed -- error MEM002`: **a function can no longer close a socket it receives** (Q1). Two tests were weakened without saying so: `flags-shift-and-move.fa:8` and `move-through-function-value.fa:7` keep `-- no-check: MOVE` on a line that now says `copy`, which no longer tests anything.

**Performance of the migration**: `js-list-prepend.fa` (above); the same shape on tiny lists in `golden/recursive.fa:10` and `while-let-values.fa:26`; `benchmarks/fannkuch-redux/main.fa:26` `current = copy permutation` runs once per permutation while the branch removes the in-place array assignment (`fa_array_assign`), so the hot loop now allocates and frees instead of copying into the existing block (not measured: time it before any landing).

**The documentation diff changes rules**, it does not only clarify: collection item types are invariant except primitive literal/range widening (the read-only "collection of supersets" view is removed); Set/Map key conversion by `copy` (what happens when converted keys collide is not specified: Q8); resources cannot be narrowed implicitly and `copy resource` is always `MEM003`; `MEM005`; closures never consume captures, escaping closures cannot capture unique values and capture field paths; the local-function declaration rules; errors never roll back collection effects. Each rule should land with the slice that implements it, not ahead of it.

## 6. Slicing plan

Rules for every slice: a branch from `main`, one engineer, a day at most, about 1,500 lines at most (fixtures excluded), green on every target by itself, with its specification text and its `:::implementation` notes. "Port" means: take the code from `feature/ownership` by path (not by cherry-pick: the commits mix topics), read it, and keep only what the slice needs. Group A enforces nothing; from group B on, each slice switches one `Form` on and migrates only the fixtures that this form breaks.

### A. Foundations: facts only, no new error on existing code

| # | Branch | Scope | Port from `feature/ownership` | Do not port | Acceptance | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | `ownership/categories` | The category query in the type store; `MEM005` (an error payload that owns a unique value). About 900 lines with tests. | `types/ownership.rs`, `types/ownership/tests.rs`, `check/error_payload_tests.rs`, the `MEM005` part of `check/errors.rs` (commits `4699df0f`, `46adafe1`, `6d46ba4f`). Add a cache. | `ownership/conversions.rs`, `projection.rs` (slice C1). | The 652 lines of category tests; `values/errors.md` example; no existing fixture changes. | — |
| A2 | `ownership/liveness` | The graph and one backward liveness set `MOVE` for whole local bindings of **every type that owns storage**; the textual loop of `check/moves.rs:170-188` is deleted. Borrowers in their simplest sound form (aliases, function values that capture, kept tasks, the module's end for what exported or escaping functions read). No diagnostic. About 1,300 lines with tests. If it does not fit in a day: A2a the graph and locals with the trunk's rule for captures (`pinned_captures`), A2b the borrowers. | `check/moves/graph.rs` whole; the `transfers(...)` tests of `concrete.rs:680-1049`; the liveness loop of `concrete.rs:44-93`. | `ownership_reads` (`concrete.rs:210-438`), `readiness*.rs`, `source_facts.rs`, `origins.rs`, `versions.rs`, `borrowed_calls.rs`, `globals.rs`, `callable_values.rs`, `flow/ownership.rs`. | Every suite green on every target **without changing a fixture**. Flag tests. New all-target fixture from probe `g2` (prints `3 4`; trunk's Bun prints `4 4`). The benchmark report (`--example report`) shows no lost move: list every benchmark read that had `MOVE` on trunk and lost it. | A1 |
| A3 | `ownership/engines-trust-the-flag` | The native backend stops deciding moves: delete `is_last_use`, `is_returned_local`, `reassigned_after`, `exclusive` (`codegen-llvm/src/lower/ownership.rs:2274-2281`, `:2444+`; `collection_storage.rs:330`); negative size. The interpreter takes the slot at a `MOVE` read (`Op::Move`, about 30 lines; JavaScript already reads the shared `lower::copies::is_move`). | `Op::Move` from commit `6da54e78` (`interpreter/src/machine.rs:416-424`, `comptime/lower.rs:592-600`). | `closed_module_transfers` (it must never exist). | IR suite, leak checks, benchmark report unchanged within noise; the IR assertions "no copier in …" of the branch's `tests/` for locals. | A2 |
| A4 | `ownership/one-use-after-move-pass` | One forward may-be-moved pass on the graph emits every `MEM002` for consumed values; `FlowState::consumed` and `check_consumed` are removed (`check/reactivity.rs:256-345`, `check/foreign.rs:935-1147`, `check/tasks.rs:1335-1360`, `check/concurrency.rs:1120-1145`). Graph events carry **places** (owner and field path) from here on, since unique fields already move partially today. Same messages as today. About 900 lines. | `check/moves/validity.rs` and its tests. | The output-scanning deduplication. | Every existing `MEM002` fixture unchanged (codes, positions, messages). | A2 |

### B. Whole bindings: the first forms switched on

| # | Branch | Scope | Port | Acceptance | Depends on |
| --- | --- | --- | --- | --- | --- |
| B1 | `ownership/enforce-local-stores` | The switch (`Form`, one file). Form `LocalOwner`: a structure local stored whole while it is read later is `MEM004` with the machine fix `copy`; `copy` of a unique value is `MEM003`. Fix edits in `fa check --format=json` and in the language server. Migration of the fixtures that this form breaks (pattern (a) of §5). | `copies.rs:507-568` (`report_structure_copy`), `:65-156` (copy edits), `:203-215` (`MEM003`) and their tests; `cli/src/report.rs` (+56); `language-server/src/features/diagnostics.rs` (+110). | `memory-management.md` §Diagnostics and fixes examples 1 and 3 turn on; fix-then-recheck tests; expected outputs unchanged. | A2 |
| B2 | `ownership/enforce-module-owners` | Form `ModuleOwner`: the same error for module-level bindings (most fixtures are top-level scripts, so this is the large migration). Diagnostic names the function or export that still reads the owner. | The messages of `copies.rs:263-298`; fixtures `ownership-private-global-moves.fa`, `ownershipGlobalEntryCopy.fa`, the export/version module fixtures. | Probes `g1`, `g2`, `s11`, `s13` (§8) as fixtures. | B1 |
| B3 | `ownership/enforce-borrowed-sources` | Form `BorrowedSource`: a structure read through a borrowed parameter, a `use` alias, a loop variable or a capture of a nonescaping function cannot be stored without `copy`. **Waits for language question Q1** (how a function takes ownership of a structure argument): without it, every constructor-like function deep-copies. | `copies.rs:326-340`; tests of `copies.rs` on parameters and aliases. | `memory-management.md` §Partial field moves "borrowed parameter" example; probes `s7`, `p8`. | B1, Q1 |

### C. Conversions (the lead's rule: a conversion never converts the inside of a structure)

| # | Branch | Scope | Port | Acceptance | Depends on |
| --- | --- | --- | --- | --- | --- |
| C1 | `ownership/structure-conversions` | Form `Conversion`: storing a structure into a type whose items or recursive parts are laid out differently is `MEM004` unless `copy`; `copy_inputs`; the converting copy on JavaScript and in the interpreter; primitive literal/range widening stays storage-preserving at any depth. | `types/ownership/conversions.rs`, `projection.rs`; `check_structure_conversion` (`copies.rs:15-63`); `codegen-js/src/generate/converting_copies.rs`; `interpreter/src/copies.rs`; `comptime/copy_plans.rs` (commits `462cb51c`, `67c2ce68`, `3205a86f`, `bb51377c`, the widening part of `16790cb0`/`f910cd33`). | `ownership-converting-copy.fa`, `ownership-primitive-widening.fa`, `collections/ownership-nested-item-widening.fa`; the three old `CV005`/`CL010` cases become `MEM004`. | B1 |
| C2 | `ownership/native-converting-copies` | The same on native, wasm and freestanding. | `codegen-llvm/src/lower/converting_copies.rs` (commit `ed9844cb`), reduced from 855 to about 350 lines by folding it into the existing copy generator (§3.2). | The same fixtures on the three LLVM targets; IR: no copier outside `copy`. | C1 |
| C3 | `ownership/inspecting-comparisons` | Equality and lookups between different layouts read both sides in place (no conversion, no copy). Independent of enforcement; can land any time. | `0e76f438`, the equality part of `16790cb0`. | `ownership-inspecting-tables.fa`, the union inspection fixture, their IR checks. | — |

### D. Fields: the general move of every value that owns storage (user, 2026-11-17)

| # | Branch | Scope | Acceptance | Depends on |
| --- | --- | --- | --- | --- |
| D1 | `ownership/field-liveness` | Analyzer only: a field or tuple-position read of an owned local or temporary gets `MOVE` when nothing reads that path later, for **every** type that owns storage (a `String` field as much as an array field); the forward pass publishes, per owner, the paths moved out at each exit and the "owner dies here" bit. No enforcement. | Flag tests: last-use field read, sibling reads after, conditional partial move, replacement of a moved field, `cursor = cursor.tail`, `head = List { tail = head }`, a temporary's field (`make().items`). | A4 |
| D2 | `ownership/native-field-moves` | Native consumes it: the field is transferred, the unused siblings of a dying owner are dropped once, a live owner with a hole is not dropped twice (static when the pass proves it, a drop flag otherwise). Per-unit and async builds. | Leak checks; list prepend of 20,000 and traversal of 200,000 nodes in linear time (2.8 s and 329 s before); the `respond` field-return and `records` temporary-push programs of the string-performance work. | D1 |
| D3 | `ownership/js-interpreter-field-moves` | JavaScript (a plain assignment, no copier call) and the interpreter (take the slot). | `js-list-prepend.fa` **as on `main`** (without the `copy` the migration inserted) within the harness timeout on Bun, Node, web. | D1 |
| D4 | `ownership/enforce-field-stores` | Forms `Field`, `Element`, `Pattern` on: a structure field, element or pattern binding stored while its owner is still read is `MEM004`; an element leaves a live collection only by `take`/`pop`; a whole use after a partial move is `MEM002` naming the path and the line. | The six negative blocks of `memory-management.md` §Partial field moves; probe `h9` (destructuring, accepted silently today). | D2, D3, B1 |

Files: D1 `check/moves/graph.rs` and the liveness (places instead of symbols), `analysis.rs` (moved-out paths); D2 `codegen-llvm/src/lower/ownership.rs` and `objects.rs` (the general `move_owned_slot` applied to a field, the drop of the remaining siblings); D3 `codegen-js/src/generate/` and `interpreter/src/machine.rs`. Nothing to port from `feature/ownership` except acceptance programs: the branch refuses these moves (a field access is a place read that never gets `MOVE`: `copies.rs:465-486`, `:326-340`; the test `partial_structure_moves_wait_for_field_support`, `copies.rs:762`) and its recorded decision "general field transfers (lead, 2026-11-17)" in `design/decisions.md` states the requirement without implementing it. The partial moves of unique fields that `main` already has (`check/foreign.rs:958-1050`) are the model to generalize.

### E. Closures

| # | Branch | Scope | Port | Depends on |
| --- | --- | --- | --- | --- |
| E1 | `ownership/callback-retention` | "May this callee keep its callback": per-parameter summary, published in interfaces, used to decide `escapes`. No new error: fewer functions escape. | `check/retention.rs` and its tests (commits `fa8184cb`, `3b338703`, `db63aff0`, `beb1267d`), with a flag on builtin signatures instead of the name list. | — |
| E2 | `ownership/closure-environments` | The decided semantics, in the engines: one immutable environment per escaping local function, created at its declaration; the hoisting rule (before its declaration a capturing local function can only be called directly). Two branches if needed (native; interpreter and JavaScript). | Native: `environments.rs`, the rewiring of `closures.rs`, `compose.rs`, `generic_values.rs` and the task lease (about 580 lines, deleting about 340; commits `81f6607a`, `260e4bf2`, `4ce65c9b`, `1ea96381`, `53697d3e`, `d2540870`), with the retain/release per indirect call measured. `check/capture_handles.rs` reduced to about 100 lines. Interpreter: environments on `main`'s `Value::Reference` model, not on loans. | — |
| E3 | `ownership/enforce-captures` | Form `Capture`: creating an escaping function stores each capture (move at a last use: `capture_moves`; otherwise `MEM004` with the snapshot fix; a unique capture is `MEM003`). | `captures.rs` messages and tests, `capture_fixes.rs`; fixtures `ownership-structure-capture-*.fa`, `ownership-heap-capture-moves.fa`, `ownership-unique-captures-errors.fa`, `ownership-capture-creation*.fa`. | A2, E2 |
| E4 | `ownership/path-captures` | The lead's rule of 2026-11-16: a closure captures only the paths it uses. | — | D1, E3 |

### F. Tasks, defaults, builtin stores, generics

| # | Branch | Scope | Depends on |
| --- | --- | --- | --- |
| F1 | `ownership/task-borrowers` | A kept task borrows what its call reads until it is awaited or cancelled: stores of those sources are `MEM004`/`MEM002` naming the task. With tasks as borrowers, the unfinished "flow-dependent Task identity" work of `0807b0de` is not needed (moving the handle moves the borrow). Fixtures `ownership-pending-task-*.fa`; the previous engineer's accepted-but-invalid probe (two tasks, a reassigned handle) as a negative test. Needs Q2 for detached calls on a unique handle. | A2 |
| F2 | `ownership/default-arguments` | Defaults follow the rule of written arguments: an omitted default that names an existing value borrows it; a consuming parameter's default must build its value (`MEM002`). Native default binding redone at the parameter prologue (about 80 lines instead of `defaults.rs` and its hooks in `expr`/`own`/`owned_as`). Port `copies.rs:344-389` and `ownership-default-*.fa`. | B3 |
| F3 | `ownership/builtin-stores` | Point 5: `slice`, `filter`, `map`, spreads, rest patterns, `from`, `toArray`, set algebra with structure items are stores (form `BuiltinStore`). Acceptance: the eight positive fixes of `arrays.md` §Construction with structure items and their eight negative twins. | B3 |
| F4 | `ownership/generic-conditions` | Copy conditions inferred on type parameters and checked at each instantiation (form `Generic`); `MEM005` for generic error payloads. | B3 |
| F5 | `effects/failures-keep-completed-effects` | Not ownership: an error never rolls back completed effects (`deleteWhere`, callbacks over collections, `sortInPlace` staging only when the comparator may throw). Port `2bfa709e`, `9058e6ba`, `e02ec3ce` (`sorting.rs`), `ownership-callback-failure-*.fa`, `ownership-sort-failure.fa`. | — |

**What to do with `feature/ownership`**: keep it as a read-only reference (an archive ref) until slices E3 and F2 have landed, since the slices port code from it by path; do not merge `main` into it any more, and do not land its migration of the fixtures (each slice migrates for its own form, and 15 of the 215 copies must never land: §5).

**Two decisions for the lead that are not language questions**: (1) the interpreter keeps the place model of `main` (`Value::Reference`); loans, if wanted, are a separate proposal that replaces it in one branch (§3.2); (2) every enforcement slice lands its negative `.fa` fixtures on every target, not only Rust unit tests (§5).

Order: A1 → A2 → (A3, A4, E1, C3, F5 in parallel) → B1 → B2 → C1 → C2 → D1 → (D2, D3) → D4 → E2 → E3 → F1 → (Q1) → B3 → F2 → F3 → F4 → E4.

## 7. Known failures at takeover

From the previous engineer's run at `32cf081b` (`/tmp/fa-ownership-workspace-current.log`), CLI programs suite, 5 tests failed for 4 causes:

1. `codegen-llvm/tests/programs/async-workers.fa` (Bun, Node, web): `MEM002` "`worker` was moved" on `start worker.squares([index])` in a loop followed by `await worker.sum()`. Cause: a detached start registers its receiver as a store (`check/tasks.rs:1803-1815`, as on `main`), and the branch made `Worker` unique, so the store moves it. A regression of the branch; see Q2.
2. `cli/tests/programs/js-list-prepend.fa` (Bun, Node, web): timeout. The migration replaced `cursor = cursor.tail` by `cursor = copy cursor.tail`: a deep copy of the remaining list at each step (quadratic for 20,000 nodes). The faithful fix is the field move of D1–D3, not `copy`.
3. `division-boundaries-bigint.fa`, `division-boundaries-int128.fa`: timeouts under machine load (20-second limit); not attributed.
4. String differential, `json-composites-00/01` on native and wasm at O0: "`Json.stringify` of this value is not supported by the native backend yet" for `(value: Forward | Reverse) => Json.stringify(value)`. **A regression of the branch**: the same generated file runs with the trunk binary (`fa run --target=native`, checked 2026-11-17) and fails with the branch's.

## 8. Probes

The probe programs are in [`repros/ownership-enforcement/`](repros/ownership-enforcement/) (the names below are their file names).

- `g2` — trunk is unsound, the branch refuses correctly:
  ```fa
  let original = [1, 2, 3]
  function total = (): Integer => original.length
  let handlers = [total]
  mutable target = original
  target.push(4)
  console.log(handlers[0](), target.length)
  ```
  Trunk on Bun prints `4 4`; the branch reports `MEM004` on line 4 ("used again on line 6").
- Correctly refused by the branch (`MEM004` naming the later use): a store followed by a read (`l1`), a store in a loop (`l2`, `p4`), a store in one branch followed by a read (`p5`), a store before a call of a local function that reads the source (`s1`, `s14`), before a callback passed to another function (`s4`), an alias of a field (`s6`), a loop variable (`s7`), a field of a borrowed parameter returned (`p8`), a module owner mutated or read by a function called later (`s11`, `g1`), a default that names a module owner (`s13`), a field assignment and an indexed assignment followed by a read (`s27`, `s28`), a store followed by a read in a string interpolation (`h6`), a replacement inside a loop (`h12`), an escaping function whose capture is read again (`s2`, `c3`, `c4`).
- Correctly accepted, same output on Bun and native: exclusive branches (`c6`), a failure after a move caught by `catch` (`c8`).
- Findings:
  - `h9`: `mutable (left, right) = pair` followed by `pair[0]` is accepted silently (a copy on every engine): destructuring is not a store position for the pass.
  - `h13`: `let holder = { items, count = items.length }` is `MEM004` ("used again on line 3"): see Q3.

## 9. Language questions

Questions that the work revealed and that the pages do not settle (or settle with a rule whose cost only shows now). Each has my recommendation; none was applied.

**Decided by the lead on 2026-11-17: every recommendation below is adopted** (Q1 (b), Q2 (a), Q3 (b), Q4 liveness, Q5 confirmed, Q6 invariance kept, Q7 fresh value, Q8 as `from`), recorded in [`decisions.md`](../decisions.md) under "Ownership enforcement: verdict on `feature/ownership` and language decisions". No page was changed yet: each rule enters the pages with the slice that implements it.

**Q1. How does a function take ownership of a structure argument?** `memory-management.md` §Parameters are references: structure parameters always borrow; "only the explicit Task-consuming contract transfers ownership"; the documented idiom is `function reader = (input: Array<Integer>) => { let snapshot = copy input … }` called as `reader([1, 2])`: the temporary array is built, deep-copied, then destroyed. Every constructor-like function (`(items) => Holder { items = copy items }`), every `add(collection, item)` helper and every function that stores its argument pays a deep copy that no last-use analysis in the caller can remove. In the analyzer this is one line (`TypeStore::parameter_can_be_consumed` = `takes_task_parameter`, `types/ownership.rs`), and the migration shows the pattern (§5).
- (a) Keep the rule: storing a parameter always needs `copy`.
- (b) A parameter that the body stores is a consuming parameter, shown in the signature like `mutates` (the lead's decision of 2026-11-16 already says "a function storing a parameter declares `invalidates <parameter>`, and function types carry it; absence promises borrowing"): the argument is then a store position at the call (moved at its last use, a temporary is moved, otherwise `MEM004` at the call: `make(copy items)`).
- (c) A keyword on the parameter.
- **Recommendation: (b)**, with the effect written in the signature (inferred for local functions, as `mutates` is). It is the same contract as Task parameters, generalized, so it adds no concept. Slice B3 should not migrate "store a parameter" fixtures before this is decided. The same contract answers the unique case that the migration hit: a function can no longer close a socket it receives (`std-io.fa`: `closeBorrowed -- error MEM002`), because resource parameters borrow.

**Q2. A detached started call on a unique handle.** `concurrency.md:443`: "a started call uses the handle: the worker can't be stopped or moved until the task is awaited or cancelled". `concurrency.md:183`: a detached `start` copies what it uses, like an escaping closure. A detached `start worker.squares([index])` is never awaited, and a unique handle cannot be copied: the page gives no answer, the branch answers "the worker is moved" (`MEM002` on the next use: failure 1 of §7), and the fixture `async-workers.fa` on `main` expects the calls to be queued and the worker to stop at the end of its handle's scope.
- (a) A detached call on a worker (or pool) queues the call and releases the handle at once: the queue belongs to the worker; stopping the worker (explicitly or at scope end) runs or cancels what is queued, as the page already says for `stop`.
- (b) Refuse detached calls on unique handles.
- **Recommendation: (a)**: it is what the engines do and what the fixture tests; write it in `concurrency.md` §Workers.

**Q3. Sibling operands of one construction.** `let holder = { items, count = items.length }` is `MEM004` on the branch ("`items` is used again on line 3", probe `h13`), while `{ count = items.length, items }` is accepted: the order of the fields of a literal changes whether the code compiles. The same holds for the items of an array or tuple literal and for the arguments of a call. The page says a last use is a control-flow property; it does not say when a store inside a literal takes effect.
- (a) Keep: operands are evaluated and stored left to right.
- (b) The stores of one literal or one argument list take effect when the whole construction has been evaluated: a sibling may still **read** the source (`items.length`); two sibling **stores** of the same source still conflict (`[items, items]`).
- **Recommendation: (b)**. It costs nothing at run time (a move is "do not drop the source later"; on JavaScript an assignment) and removes an error that no programmer would understand. In the graph it is where the store-commit events are placed (`graph.rs:209-216` already separates the commit from the read).

**Q4. Is a function value in use only when it is called, or while it is live?** The page describes the precise rule: a nonescaping closure's captures are read when it is called or passed. The branch implements it with the interprocedural machinery of §2.2 (about 2,900 lines). The clean design of §4 uses "a function value borrows its captures for as long as the value is live": identical except for a function value that is kept (in a local, a field, a collection) and never called again, whose captures stay pinned.
- **Recommendation: adopt the liveness rule as the language rule.** It is simpler to state ("a function that reads `items` keeps `items` in use until the function's last use"), the diagnostic can name the function, and it removes about half of the analyzer code.

**Q5. Stores made by patterns.** Settled by the page (patterns follow the rules of field reads), but worth the lead's attention because it is silent today: `let (left, right) = pair` followed by a use of `pair` copies on every engine without a diagnostic (probe `h9`), and with field moves (D1) the natural reading is "`pair` is consumed by the pattern". No question, a confirmation: destructuring an owned local at its last use moves each bound part; destructuring a live owner needs `copy pair` or `use` bindings.

**Q6. Passing a collection where a collection of a wider item type is read.** The lead decided that a conversion never converts the inside of a structure (item types are invariant except primitive widening). The migration shows the price: `logNames(copy superHumans)`: a deep copy to call a function that only reads; a `Map<String, Hero>` is no longer accepted by a parameter `Map<String, Human>`. The diagnostic already suggests making the function generic (`<Item: Human>(items: Array<Item>)`).
- **Recommendation: keep the invariance** (a view over items laid out differently cannot be free on native), and make the generic signature the machine-applicable fix when the callee is in the program, instead of `copy` at the call. Confirm that the read-only superset view of `main` is given up on purpose.

**Q7. Storing a compile-time constant.** `let colors = copy Palette.Color.members` and stores of statics now need `copy` (5 sites). A constant has no owner to protect.
- **Recommendation: reading a compile-time constant yields a fresh value**: it is a temporary, so it moves into its destination and never needs `copy`; an engine may share the static storage for an immutable destination.

**Q8. Key collisions after a converting `copy` of a Set or a Map.** `copy` may convert keys (for example to a narrower record); two keys can become equal. The pages do not say what happens.
- **Recommendation**: the result is what building the collection from the converted entries in order gives (a Set keeps the first, a Map keeps the last value), as for `from`.

Also revealed, not questions: the diagnostic cascade (`MEM004` on a store that already has a type error, and two codes for one mistake in `adv-10.8-07.fa:14`); `-- no-check: MOVE` lines that test nothing after the migration.
