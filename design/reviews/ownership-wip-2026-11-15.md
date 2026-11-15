# WIP assessment — 2026-11-15

Assessment branch: `audit/wip-assessment`, HEAD `9cbb7558fe00c0f08426fbcba3e9c8583e272535` (equal to `compiler`). **Land focused slices, never one of these six snapshots wholesale.** Start with the Sync remap and written fingerprints; establish one ownership/capability contract; port Copy, provenance and final unique-resource rules; then finish the affected backends. Set can proceed separately. Implement monomorphic `with` after counted Function environments. Use the pointer and transport work as references. Reconcile the November 15 generic-value restriction before taking any forall machinery. Measure pass fusion last, independently of semantic changes.

This is a read-only review, not a delivery or validation of compiler code. I read the lead brief, this worktree's AGENTS.md, all six NEXT.md files, decision records, relevant documentation, diffs and implementation/test sources. No build, test, formatter, checkout, merge, commit, stash, branch or worktree change was performed. The only file written is this report. The explicitly requested `merge-tree --write-tree` inspections create Git tree/blob objects, but do not alter refs, indexes or worktrees. Historical test results below are **claims in checkpoint notes**, not results reproduced in this assessment.

Pinned comparison inputs:

| Label | Snapshot / parent | Snapshot size relative to parent | Actual merge base with `compiler` |
|---|---|---|---|
| P: analyzer-partial | `22cd8d298a9cefdd8cffb31e72f950bd6a0dcbdf` / `aa3e05083c900b6d91c93a2445ae6bd9f9347542` | 231 files, +16,406/−1,275 | `aa3e0508` |
| F: analyzer-pass-fusion | `706f8f7a731edd5503bcad243c1b2ec9bc293d0e` / `aa3e0508` | 201 files, +15,778/−1,317 | `aa3e0508` |
| R: analyzer-rules | `c4eeeeecf45492890bfd44e4cb6a4590ce39df3d` / `ecda9e58` | 100 files, +6,478/−291 | `ef777e454cea7e61722c6e4b370261f4fb12a618` |
| W: analyzer-with | `5259a8b817ae858e6131799a391c70f06c2aa18d` / `ecda9e58` | 1 file, +115 | `ef777e454cea7e61722c6e4b370261f4fb12a618` |
| H: shared-handles | `3c8cc4d026c328a73ab2d78288b24311cf2cb2af` / `64a31a84` | 141 files, +16,484/−6,735 | `0290400797f9bc9ab8d684923dd1cb064f858804` |
| G: generic-pointers | `54039eedf36da739277e9efb1bb47e5376b61e6c` / `0256c70d` | 27 files, +1,115/−74 | `a39f3c4285cb81924172847880dabeb17ef90708` |

All six tracked worktrees matched these snapshot commits when inspected. All six snapshot messages explicitly say the state was left on November 7, archived by the lead on November 15, and never validated as a whole. Their common Git author/coauthor is archival attribution, not proof of who made a language decision.

The other line and main checkout HEAD were `d256f11a40fc60e34e896e161c37a3bbd6c961c9` (`perf/native-goose`); its on-disk `design/decisions.md` exactly matched that commit. Comparisons to that line use `f75ce8a4..d256f11a`, as requested. I did not assess an eventual merged trunk or the ongoing merge's uncommitted resolutions. Paths below are repository-relative; WIP worktrees are under `/home/claude-dev/projects/Fa/.claude/worktrees/<name>/`.

## 1. Decisions, authority and contradictions

### 1.1 What the records actually establish

**None of the six WIP commits changes `design/decisions.md` or any `documentation/` page relative to its own parent.** Differences in those files between worktrees are inherited base differences. In particular, a later NEXT claim does not imply the worktree updated the specification. P and F have exactly compiler's documentation, despite implementing materially new Copy/Set/function behavior. R/W have older pages (metaprogramming, maps and strings); H has an older metaprogramming page; G has twelve older documentation pages, including foreign code and generics. Do not merge any older page to obtain an implementation.

| Decision | Exact authority/date available | Where recorded and caveat |
|---|---|---|
| Task and owning composites are noncopyable | User, November 7, per compiler heading and P/F/R/H NEXT | Compiler decisions §“Explicit copies, shared resources and written type unions”; P/F inherit it. H's decision record still states the revoked universal-copy policy. |
| Every `destroy` type and its owners are noncopyable; Resource parameters borrow, including Resource-with-Task | Final user choice reported in November 7 checkpoints; exact conversation/time not recoverable here | Compiler **todo** §“Latest language decisions being implemented”; P/F current checkpoint, R “FINAL Resource choice”, H opening policy. **Not reflected in the compiler/P/F decision section, which still specifies Resource RC.** Foreign-code documentation already specifies move-only Resources, but does not completely specify the new parameter/erasure ABI. |
| Collection copies require `copy value`; last-use moves remain allowed and invalidate aliases | User, November 7 | Compiler/P/F decision section and todo. Spaced contextual prefix and preserving identifiers named `copy` are recorded in P/F/R NEXT and code, not in the decision section. |
| Sync handles share identity, Function environments are counted; Task never retained | Sync sharing is existing specification; Function-environment protocol is engineer implementation work, November 6–7 checkpoints | Concurrency/memory pages; H/P NEXT and runtime code. Do not interpret Resource RC prototypes as authorization. |
| Worker transfer is an independent copy, common Mutex or exclusive transfer; Task stays executor-affine | User answer reported in November 7 inherited NEXT text | Compiler/P/F decision section; P/F/H NEXT. The Resource-cell explanation in the older text is superseded by unique Resource ownership; protection and alias invalidation still apply. |
| Set standard hash/equality; no `key` callback; use Map for separate keys | User final choice reported in R/H November 7 checkpoints | Compiler todo, R/H NEXT. **Absent from every audited decision record and from the Set/Array documentation.** |
| Equal-field written types with different methods/destructors retain alternatives at a join | User, H decision heading November 6; compiler/P/F restate November 7 | Compiler/P/F §“Explicit copies…” and H §“Universal copyability…”; W/R NEXT explicitly leave joins unimplemented. No complete documentation rule for indistinguishable origin alternatives. |
| `with` takes parameter names from the visible signature, including fields and parameters | User, same November 6–7 records | Same decision sections. Defining-functions page describes with binding/defaults/copy behavior; visible names are more explicit in decisions/NEXT than the page. |
| JS operations generated at build time; no runtime schema interpreter/function compiler; no JSON transport | User requirements reported in P/F NEXT, November 7 | Build-time operations are in compiler/P/F §“Explicit copies…”. No-JSON and Codec work are explicit in NEXT; concurrency implementation documentation still prescribes JSON. |
| Generic TypeInfo and sparse NeedsCodec, interface hash tags, counted environment layout, portable value origins and SignatureNames | Engineer implementation choices, November 6–7 checkpoints | NEXT/code only. Not additional language decisions. The universal TypeInfo “semantic key” note is itself superseded by H's later “no universal .codec or canonical .key” note. |
| Function factories/aliases, defaults, effects and return-to-Void | Existing specification; WIP implements it, with engineer ABI choices | `documentation/functions/defining-functions.md`, `function-composition.md`, `types/generics.md`, `advanced/requirements.md`; compiler generic decision below. Factory semantic instances are implementation work, not a separately approved new runtime. |

The following are verbatim operative records, quoted once where several worktrees share them.

**Compiler, P and F — `design/decisions.md:626`, “Explicit copies, shared resources and written type unions (user decisions, 2026-11-07)”**:

> Task and composites owning a Task are noncopyable and transfer ownership. This
> revokes the intermediate universal-copyability decision; no Task reference count
> is introduced. Collections and ordinary objects owning collections require
> `copy value` to keep both values usable. An implicit last-use move invalidates
> the source, including its aliases. Other ordinary objects retain value semantics.
>
> Foreign resources and synchronization handles share their instance and state
> through counted local handles. The last owner finalizes a resource. A resource
> containing a collection is a sharing boundary: copying its handle does not copy
> that collection. A resource containing a Task remains noncopyable.
>
> Across workers, ordinary values require an independent copy or an exclusive move
> that invalidates the source and aliases. Mutable shared state requires a common
> Mutex. A Resource handle retains shared state, so it crosses this boundary only
> under that Mutex or by exclusive transfer. Old unprotected aliases and handles
> copied from Guard.value must not escape the protection; Task stays with its own
> executor. This extends the existing thread-safety rule, without hidden automatic
> field locking. Frontend cell/loan proofs and backend ownership support are in progress.
>
> The JavaScript backend will generate copy/read/write/drop operations during
> `fa build` from analyzed types. Erased generics receive concrete TypeInfo
> operations, including Task ownership. The Fa Codec runtime supplies binary
> primitives and fromFunctions(write,read); it neither compiles functions nor
> interprets schemas while the program runs.
>
> A conditional joining written Person and Greeting types with different methods
> or destructors has type Person | Greeting, even if their fields are structurally
> identical. The actual union member selects its method or destructor. A `with`
> application uses names in the visible function signature, including the signature
> of a function-valued parameter or field.

**Compiler `design/todo.md`, “Latest language decisions being implemented”** overrides that section's Resource paragraph:

> Task and every type declaring `destroy` are noncopyable, including composites that own one. Resource parameters are borrowed; a second resource requires a new instance (`open`, `create` or constructor). Ordinary collections and their owners require `copy value` when both values remain usable; implicit moves at the last use are allowed. Explicit synchronization handles such as Mutex and Channel retain their sharing semantics.

**H — `design/decisions.md:625`, “Universal copyability and written type unions (2026-11-06, user decision)”** is historical and revoked:

> All types are copyable, including a Task, a foreign resource and composites that
> hold them. This supersedes the earlier move-only policy. Copies of a Task or
> resource share that instance and its state through reference counting. Releasing
> the last owner destroys the resource or cancels the still-running Task. Ordinary
> values retain structural value-copy semantics; generic functions do not need a
> Copyable constraint. Implementation and lifetime regressions are in progress.
>
> A conditional joining written Person and Greeting types with different methods
> or destructors has type Person | Greeting, even if their fields are structurally
> identical. The actual union member selects its method or destructor. A `with`
> application uses names in the visible function signature, including the signature
> of a function-valued parameter or field.

**P/F current checkpoint and R/H final policy** supersede that history. P/F say:

> Final ownership policy: Resource/destroy and Task are unique/noncopyable;
> Resource parameters borrow, even when containing Task. Task-consuming ABI
> stops at Resource/Sync. Collections need explicit copy unless a proved move.
> Escaping captures cannot own any unique value.

R's exact Set record is:

> FINAL Set choice: standard hash/equality, separate key uses Map. Root owns
> docs + JS; native_perf LLVM. /tmp/fa-set-standard-frontend.patch (347 lines)
> removes key option/slot, stored-key provenance and artificial HAS_SHARED;
> [source, capacity] is now the CallInfo ABI for Set/GrowingSet.from. FN023 for
> key=.

**R and W** have no new ownership section in `design/decisions.md`; their records end at “Generic metadata across files (2026-11-06)”. **G** ends at “Recursive integer returns and empty arena leaves (2026-11-06)”. All retain the earlier “Foreign code” move-only rule and “Minimal representations” rules. Relevant exact inherited statements are:

> **Move-only**: a type with `destroy` is a `NominalKind::Resource`; a copy of a local holding one moves it (`MEM002` on a later use, like a stopped `Subscription`).

> **Parameters are never copied.** Every parameter is passed by reference (a pointer natively), mutated or not; only scalars that fit in a register are passed by value, which is the same thing for them. A copy happens only when the callee stores the value (an escaping use) and the analyzer can't move it.

> **`WorkerPool` state** (lead, question of the async engineer): a pool's options take `state: T`, the initial state of each worker, copied into every worker of the pool (`WorkerPool(DataProcessor) { count = 4, state = DataProcessor { factor = 2 } }`); it may be left out when every field of `T` has a default (then `T {}`). A worker restarted after a `timeout` starts again from a fresh copy of `state`. The concurrency page gets the field in its options example on resume.

That last paragraph is a lead resolution, not attributed to the user, under the November 3 heading. P's refusal of Resource pool state implements this existing repeated-copy contract; it does not invent a “one-time move” pool.

**Compiler and all six worktrees — `design/decisions.md`, “Generics with angle brackets (user, 2026-11-04)”**, particularly the user bullet “Generic functions carry their list on the function value” and the lead's Phase 2 entry:

> The list belongs to the function expression, so lambdas use the same syntax (`<T>(value: T) => value`), methods too (`function map = <Output>(self, transform: (Item) => Output) => …`), and function types can be generic: `type Identity = <T>(value: T) => T`, then `function f: Identity = <T>(value: T): T => value`.

> `let f = <T>(…) => …` stays `E-DECL-6` (a function is declared with `function`); generic values are stored in fields, collections and other `let`s, and returned.

These broader value rules are explicitly contradicted by the November 15 decision below. R/W/G's older records do not authorize adding the newer WIP factories or adapters independently of that arbitration.

### 1.2 Specification status on compiler

| Page / section | What it specifies; missing update |
|---|---|
| `advanced/memory-management.md:9,101,248,281`; `values/declaring-values.md:145,400,408` | Assignments implicitly copy, with last-use moves; only Sync sharing is described as RC. **No `copy value` syntax/requirement.** Also “only reference counting” needs qualification for Function environments. |
| `advanced/foreign-code.md:162–167,327,334–345` | `destroy` types and containing composites are move-only, released once; pointer reads copy and `.value` borrows. The last implementation paragraph still rejects unresolved generic targets. Neither WIP pointer capabilities nor final borrowed-Resource generic ABI is documented. |
| `advanced/concurrency.md:151,166,436–475,582,744–765` | Structured Task cancellation; pool state copied per worker/restart; Sync copies share state. JS implementation still says channels store JSON and mutexes use the same scheme. Worker handle stop/copy semantics need coalescing with H. |
| `functions/defining-functions.md:386–445,475–531` | Escaping captures copy at creation, cannot own Task/Resource and are immutable; function aliases/factories must be known at compile time; with binds once by name, retains residual defaults/effects/generics and cannot bind mutated or variadic parameters. This provides the contract implemented by P, but doesn't settle `start` default timing. |
| `functions/defining-functions.md` function types; `advanced/requirements.md:294–295` | Effects/requires and stable requirement-key ABI; P extends the implementation to full paths. |
| `types/generics.md:351` | Generic function values work at every call; the old compiler allows storing/returning them. Must be reconciled with perf's narrower November 15 rule. |
| `collections/sets.md:48,226,236`; `collections/arrays.md:56` | Explicitly teaches Set's `key` callback. **Contradicts the final Set decision; no WIP fixes these pages.** |
| `types/unions.md:41` | Indistinguishable members are an error; no implemented exception/representation for equal-field written-origin alternatives. Origin-union decision needs a precise public rule and implementation note. |
| `types/types-at-runtime.md:160`; `types/type-functions.md` | Describes Schema/type-function behavior more fully than the implementation, including the special “value of a generic type function” supported case. Arbitrary nested TypeFromSchema and native named Schema values remain gaps. |

No documentation change exists only in a WIP snapshot delta. The WIP-only records are NEXT's final ownership clarification, Set decision, contextual Copy grammar, transport/AOT details, exact ABI/cache choices and open issues; most semantic contracts otherwise predate the snapshots. Older copied NEXT sections are not newer decisions.

### 1.3 November 15 perf/native-goose decisions to preserve

Source: `/home/claude-dev/projects/Fa/design/decisions.md`, pinned `d256f11a`. The date is November 15. “Fable for the user” denotes an engineer arbitration, distinguished below from the user rule.

| Section and operative quotation | Authority | Effect on WIP landing |
|---|---|---|
| “Generic angle brackets” / “Open points of the implementation”: “**A generic function type is written only as the type of a parameter of a `function` declaration** (named functions and methods).” “Everywhere else it is `GN030` … return types, fields, collection elements, and parameters of lambdas or of function types…” | Original angle syntax: user November 4; narrowed scope: Fable for user November 15 | Drop general stored/returned forall values, runtime forall class/table adapters and factories returning forall. Keep static named generics, permitted callback parameters/forwarding, and generic-to-concrete instantiation. Rewrite positive WIP tests that now describe forbidden programs into GN030 cases; don't retain both semantics. |
| Same subsection: “**`let f = <T>(…) => …` is `E-DECL-6` only**”; “**`async` comes before the generic parameters**”; generic lists break after commas with first item beside `<`; unclosed expression lists rewind and parse as comparisons | Fable for user | Port contextual Copy into the new tokenizer/recovery. Never overwrite parser or formatter files from P/R/H. Preserve new async ordering and diagnostic/recovery choices. |
| “Strings: UTF-16 semantics, UTF-8 storage”: “**Semantics: a `String` is a JavaScript string on every target**, a sequence of UTF-16 code units.” “**`Character` is removed.**” “**`Bytes`** is a new prelude type: a fixed-length sequence of bytes…” | User; details Fable | All WIPs still use Character/grapheme-era type cases. Extend Copy/codec/pointer operations to Bytes; preserve lone surrogates, code-unit order/indexes, UTF-8/WTF-8 output rules. Remove obsolete Character schema/layout cases rather than copying them. The string decision is a specification, not proof every path at d256 is implemented. |
| Same section: “**`bytes(): Array<Byte>` is removed** in favour of `toUtf8(): Bytes`”; “**`Schema.Character` becomes `Schema.Bytes`.**” | Fable details | Update generated wire string/byte cases and Schema tests when porting; don't transplant old schema registries. Raw FFI memory remains bytes, string semantic indexes do not. |
| “GrowingArray native layout”: “**Decision: `GrowingArray` stays a segmented list on every native target**”; closure: “**`GrowingArray` is a segmented list with the branch-free lookup on every target; this is final.**” | Fable for user | Retain per-array k, 64/40-byte header, inline directory, stable item addresses and branch-free/biased directory lookup. WIP native runtime/IR blobs predate these changes. No reserved-address-space redesign. |
| Same section: “**No `clear()` (user decision).**” Empty assignment drops items and retains storage | User | Preserve reset/reference semantics when adding shared item drops or copy/move handling. Don't restore generic cleanup code that frees frame/static or retained segments. |
| “Local immutable collections on the stack”: “**User rule:** every local immutable collection uses preallocated frame storage when its lifetime permits it.” “**Escapes (user decision): favor stack over heap, but heap over copy.**” “Never construct in a frame and copy out to rescue an escaping value.” | User; budgets/layouts Fable, reviewed same date | P's ownership, adapters, temporaries and argument conversions need new destination/return ABI integration. Retain static no-op drops, frame item drops without freeing storage, completed-prefix cleanup and independent payloads for mutable copies. |
| Same section and review: “**Return destinations (user decision)** … returned aggregates must not be copied through memory”; small aggregate SSA threshold, 4 KiB frame/512-byte recursive budget, 128-byte recursive offer sub-budget | User rule; thresholds Fable | Reimplement P wrappers against `return_destinations.rs`, `collection_storage.rs` and current result ABI; do not restore old all-by-value aggregate returns or introduce output aliasing. Preserve native/free/wasm selection and noalias invariant. |
| “Capacity and reserve on every collection”: “`reserve(additional)` follows Rust's `Vec::reserve`: reserve for current length/size plus the argument; `from(source, capacity = n)` requests total capacity.” | Fable confirms user's November 3 wording | Keep [source, capacity] when removing Set key; evaluate hints once in source order even on JS, keep source cleanup until later hints succeed. Native strings count capacity in bytes while length counts UTF-16 units. |
| Same section: “Copies preserve contents, not spare capacity.” “**Rule added: an effect on `length` or `size` implies the same effect on `capacity`**” | Fable for user | Merge effects with P's function-type path checks; do not reinstate older reserve effects or capacity-preserving copies. Generic ownership metadata must remain independent of spare capacity. |

There is also a wording conflict: the November 15 capacity review justifies the copy choice with “copies are implicit and frequent in Fa”, whereas compiler todo and the final November 7 policy require explicit collection copies. The storage rule (contents, not spare capacity) can stand under either policy. Recommend retaining explicit `copy` as the last clearly attributed user choice until the lead confirms whether that newer phrase was intended to revoke it; do not infer revocation from explanatory prose.

## 2. Work by topic: implementation, validation claims and quality

### 2.1 Which snapshot is most advanced?

**P is the best broad semantic reference, not a superset or integration candidate.** `git diff F P` changes 68 files (+1,047/−377). Of F's 201 snapshot-modified files, 166 are byte-identical in P and 35 differ. The common core includes `check/copying.rs`, moves/value_state, named signatures, partial application, function constants, semantic VM instances, most function adapters/partial native lowering, parser Copy and interpreter support. F uniquely has shared scan parent edges, reference-source collection, sparse read-based nonescape checking and lazy region metadata. P reverts those optimizations and adds later correctness work.

P-only/later work includes grouped forall identity; qualified function/worker aliases; generic value defaults; inline named-union payload defaults (analyzer, interfaces and both backends); Mutex stores and WorkerPool copyability; Sync remapping; caught failure flow states and lexical direct throw; removal of 19 additional pending markers and three ft-9 native skip-list entries. The differing production files are:

`analyzer/src/analysis.rs`, `check/{calls,concurrency,errors,functions,generics,interface,locals,mod,objects,provenance,references,type_decls,type_expr,value_params}.rs`, `modules.rs`, `proofs.rs`, `provenance.rs`, `regions.rs`, `scan.rs`, `session.rs`, `types/{merge,subst}.rs`; JS `generate/{expressions,functions,mod,objects}.rs`; LLVM `lower/{objects,program,values}.rs`, `src/units.rs`. The rest of the 68 are NEXT, tests, fixtures and expected-skip lists. Exact filename inventories are reproducible with `git diff --name-status 706f8f7a 22cd8d29`.

**P is not a superset of R either.** Of R's 100 changed paths, 72 match P and 28 differ/are absent; the total tree diff is 207 paths because bases also differ (regions, continuations, compiler/performance deliveries). R retains `consume_worker_argument`, `thread_moves`, `thread_views`, `thread_origins`, transmitted structural-view provenance, a JS-local-handle affinity category and two fixtures absent in P: `modules/resource-worker-transfers.fa` and `modules/js-local-worker-handles.fa`. P instead adds partial/capability/default/factory provenance and the later borrowing/type compatibility fixes. R's blanket JS-local refusal conflicts with the no-blanket-rejection transport requirement; selectively reuse its exclusive-transfer proofs, not its entire policy. R's `conditional_consumed` has a fourth, task-only discriminator; P uses a three-tuple specialized to Task consumption. Blind union of those code paths is unsafe.

The 28 differing R snapshot paths comprise NEXT; analyzer `analysis.rs`, `check/{calls,comptime,concurrency,copying,decl,effects,errors,flow_tests,functions,generics,key_sets,mod,objects,provenance,references}.rs`, `comptime/lower.rs`, `flow/mod.rs`, `{lib,modules,provenance,session}.rs`, `types/store.rs`; the two worker fixture files above; JS `generate/expressions.rs`; and LLVM `lower/mod.rs`. These are semantic/ABI differences rather than R simply lacking the new P files.

**W is a small distinct delta**, not the implementation of `with`: only semantic fingerprints in analysis.rs. Its NEXT describes already delivered written-origin work at ecda and remaining work. P does **not** contain W's `semantic_fingerprint` methods. Much of W/R's inherited written-origin work was already ported onto compiler; their old Git bases still make merge-tree conflict. H has the largest shared-runtime/codec changes, but older ownership/provenance. G is the only generic-pointer implementation and has the oldest incompatible capability ABI.

### 2.2 Explicit Copy syntax and CopyCore

Best syntax source: P/F/R/H, effectively the same Copy patch. `parser/src/nodes.rs` adds `Node::Copy { right }`, children/kind/rendering; tokens/`tokenize.rs` add contextual Copy resolution; `parsing/parse_expression.rs` uses prefix precedence and preserves `if copy { … }`/loop identifiers. Spacing distinguishes prefix `copy values[0]` from `copy(value)` and `copy[0]`. `is_stop_token.rs`, formatter printer, analyzer syntax `View::Copy` and LSP semantic tokens have adapters. Parser precedence tests cover nested prefix, field/index precedence, ordinary identifiers/control conditions.

Analyzer collects explicit sites and uses the operand's projected target type. Compile-time lowering maps Copy into existing value/COW semantics, not a new interpreter heap model. JS expr calls generated `copy_value`; LLVM expr calls `emit_copy` and marks the result fresh. These are real paths, but **the complete root CopyCore/AOT/unique-Resource lot discussed in NEXT is not present in any of these six snapshots**. P/F use a deliberately minimal Function-only bridge; R has almost no backend integration; H has the obsolete Resource-sharing queries. None documents the syntax. Port syntax and tests; rework backend Copy through the single new typed operation contract and new native destination/storage rules.

Checkpoint evidence: parser and LSP contextual tests reported green in P/F's inherited root notes, along with explicit-copy and then-obsolete shared-resources probes. No final six-snapshot build or total CopyCore validation exists. The old Resource-sharing probes are not final-policy acceptance tests.

### 2.3 MEM003/MEM004, moves, capture and builtin copy conditions

Best semantic source: P/F `check/copying.rs` (1,842 lines including tests), with R's separate worker transfer extension (R 1,901 lines). `CopyRequirement { subject, explicit, task_transfer, task_only, resource_gate }` is separate from runtime capabilities. Finishing gathers copy sinks/captures/builtin element operations, normalizes to rigid parameter leaves, propagates callable applications to a fixpoint and publishes interface summaries. This handles local/imported calls without rechecking bodies. MEM003 rejects explicit/required copies of Task, Resource and owning composites; MEM004 rejects implicit structural collection copies. Resource/Sync boundaries, generic field gates and post-coercion target types matter.

`check/moves.rs` uses lexical function identity and evaluation-order reads, aliases/captured named functions, exclusive branches and same-function replacement sinks. It supports a concrete data constructor replacing its former owner even inside a loop, but excludes later RHS reads/failing or calling RHSs and unsafe shared/generic/Task cases. `lower/copies.rs` unwraps Try/Group. Move metadata distinguishes `moved_reads`, `collection_moves`, `task_moves`, `task_arguments`, `owned_projections` and capture-site maps. Conflating them changes ownership.

Captures of escaping closures cannot take unique values even on last use (no invented FnOnce). Collection captures may move a proven owned last-use local; borrowed collections require an earlier explicit copy. `moved_captures` is keyed by environment **creation expression**, including a named-function read, not only the lambda declaration. Builtin checks cover copying slice/filter/reverse/sort/from/toArray/Pointer results and Task-consuming callback inputs; borrowed iteration and mapped fresh Task results remain distinct.

This is substantive implementation with adversarial unit tests, not just flags. The notes still acknowledge incomplete .push/.set provenance, loop/builtin-produced origins, key-loop-specialized copy conditions and opaque-capture source conflation. Consequently it is not a proof that erased callbacks cannot hide forbidden copies. Port the final predicates and core conditions, then finish these holes before enabling the complete policy. Schema fixtures also require explicit copy; runtime methods that intentionally create a new collection need their operation-specific copy contract rather than blindly rejecting every owned result.

R reports 385 analyzer units plus cases/corpus/pipeline/docs for value state, 387 for Set, 391 for unique Resource and 393 for borrow/default/place followups, with targeted clippy; P later reports cumulative 457 complete analyzer checks and clippy. These are different staged sources. Fixture migration counts are in §4.1.

### 2.4 Callable provenance and evaluation-time value states

Best general source: P's `provenance.rs` (564 lines) plus `check/provenance.rs` (1,408) and `check/value_state.rs` (220). R is a useful earlier independent-transfer reference; F adds shared parent traversal without later P fixes. Portable `ValueOrigin` represents opaque sources, formal parameters by DeclKey/position, functions with captures/exact bindings, aggregates, projections, calls and alternatives. `CallableSummary` carries result, applications, capture sources, copy and thread requirements. P adds `CallableCapabilities` for nested written origins/method selections and defaults. Interface hashing/remapping preserves them when provider ASTs are discarded.

Flow `callable_values` snapshots mutable slots/field paths at each read and environment creation; branch joins preserve possible sources. Later assignments must not rewrite a previously copied capture. Tests compare reassigned callable/holder/branch/named-function worker affinity and saved-before-reassignment cases. P's factory test explicitly compares equal structural Person/Greeting types with distinct method/capability hashes.

Strength: no backend guesses an owned closure's captures from its visible signature. Risk: recursive symbolic expansion and structural equality of opaque origins can merge distinct sources; NEXT explicitly admits this. No test run here establishes termination/memory behavior for all recursive factories. This graph's `Alternatives` are **analysis possibilities**, not a runtime written-origin union or a Send protocol.

### 2.5 Noncopyable Resource/Task, borrowed parameters and projections

Best frontend: P/F final `types/store.rs`, `check/{copying,foreign,tasks,references}.rs`; R adds transfer work. `is_move_only` covers destroy/Task ownership; `takes_task_parameter` controls the consuming ABI and stops at Resource/Sync; `contains_shared_handle` covers counted Sync/Function; executor affinity traverses real nested Sync/Resource/Pointer/Ref payloads. Phantom type arguments must not imply owned Task fields. `owns_unique` is an old ABI alias, **not** the definition of noncopyability.

P records explicit Task argument ownership in source order, includes fresh/variadic/generic arguments, keeps values registered until all arguments succeed, preserves borrowed self including imported namespace calls, and refuses Task field extraction through a Resource structural view. `owned_projections.rs` in LLVM transfers kept fields and destroys dropped extras. Resource ancestors cannot be partially moved because destroy can read them; guard-backed borrows cannot leave their guard's lexical scope. Tests exercise default ownership, invalidated aliases, fresh container fields and structural Task views.

**Backend completion is absent.** P's JS `resources.rs:347` still categorizes Resource parameters as owners, contrary to the final borrowed-parameter rule. H's TypeStore says “Resources without tasks are copyable shared handles”; its C runtime still adds `fa_resource_new` on a counted `FaShared` header. P lacks the production ResourceDrop/typed-copy/TypeInfo integration mentioned in NEXT. There is no `Analysis.resource_drops` or `Interface.written_parameters` delivery in these six. H's later generic-destructor/conditional-borrow notes describe separate `/tmp` snapshots/patches, not the checked-in H implementation. Do not credit these six with those finished finalizers.

Thus reimplement the final ownership protocol on trunk using the frontend and separate-delivery descriptions as reference. Preserve zero-valued valid C resources with ownership flags, not zero sentinels; drop Resource exactly once, never retain it. Preserve Task cancellation and executor identity without RC. Reuse selected-method/written-owner information at finalizer construction, not a structural TypeId scan at destruction.

### 2.6 Mutex payload stores and WorkerPool

Best source: P (later than F/R/H) `check/concurrency.rs:143–210,1357+`, `types/merge.rs`, CLI/analyzer fixtures. A Mutex owns its payload; its value expression enters normal copy/store checking. Tests prove erased Task payload transfer, MEM003 for a borrowed Resource store and MEM004 for a borrowed Array store. A locally owning Resource can move into a Mutex; copying a Mutex remains handle sharing. This must not permit unprotected copies from `Guard.value`.

WorkerPool has two independent checks: its state expression is an owning sink, and state type must support repeated independent copies for workers/restarts. P defers an explicit copyability condition on the item type; `Mutex<Resource>` can be a shared handle, Resource-containing state cannot be duplicated. `modules/worker-pool-unique-state.fa` is the negative fixture. This corrects the tempting but wrong inference that last use of state makes a unique state reusable.

P also ports `K::Sync(kind,item) => K::Sync(kind,self.ty(item))` into Merger. Otherwise a forall returning Mutex<T> leaks local TypeId into global class keys and can panic. H independently has that fix plus tests. P's native same_layout treats equal global Sync payloads without prematurely ending a coinductive representation check. Notes report targeted FE3, remap, pool negative, factory/forall on all three LLVM targets/leaks and cumulative analyzer 457 green. JS TypeInfo execution was validated in another production checkpoint, explicitly not P's minimal backend.

### 2.7 Partial application `with` and named signatures

Best source: P/F; P has later surrounding generic/default/alias fixes. Parser already had with syntax; W adds none. Analyzer `check/partial.rs` (527) checks names (FN100–102), mutated/invalidated and variadic binders (FN103–104), unique captures (FN072/conditional requirements), inference/fixed/residual generics, parameter order, effects and requires remapping. `PartialApplication` stores bound values in source order and residual positions in signature order. Index-dependent effects currently widen conservatively to All.

`signatures.rs` / `check/named_signatures.rs` preserve parameter spelling/defaults separately from structural FnSig equality. Reference(DeclKey) makes recursive named graphs finite; functions/objects/tuples/collections forward visible metadata through annotations/imported aliases. Interface tag109 hashes names/defaults/bound names. Two signatures with the same structure but different parameter names must remain type-compatible while with chooses only the visible names. Defaults belong to declarations, not structural function type identity; a plain function value erases them, a named residual declaration retains them.

Interpreter IR `Expr::Partial` → code `Op::Partial`, Gather/Finish continuations and closure-bound groups evaluate target/arguments once, preserve order and original defaults. Three interpreter tests include chained rebinding. This extends the already integrated heap-continuation VM; do not reimport the old VM base wholesale.

JS `generate/partial.rs` (149) captures/copies target and bound values under temporary guards, creates faOwnShared, releases in reverse order, forwards hidden arguments/requires, omission markers, variadics and async calls via faCallValue. It uses P's Function-only cleanup bridge; no full production typed-copy integration. It does not consume `function_constants` to emit evaluated factory closures.

LLVM `lower/partial.rs` (728), `default_values.rs`, calls/closures/generic_values/units implement counted `{drop,copy,refs,target,bound…}` environments, mono/residual-forall producers, a separate default-aware entry and source capture order. Async wrappers retain environment/input owners until invocation, own the child before suspension, and cancel the child before destroying its borrowed environment/parameters. Requirement forwarding and mutable argument writeback use the function-value ABI.

Notes report 5 initial targeted analyzer tests, three interpreter tests, Bun printing 5/5, then ten/eleven with fixtures on native/freestanding/wasm with leak checks and analyzer 441–447/clippy. **The full feature remains unvalidated after coalescing.** Production JS operation integration, exact default start timing and newer native return storage are unresolved. General residual forall values now conflict with perf's restricted GN030 rule; implement legal monomorphic and statically named cases without reviving prohibited storage/returns.

### 2.8 Function-value ABI, adapters, forall and function constants

P/F are most advanced. Native `function_adapters.rs` (626) generalizes error adapters to numeric/optional/structural input and output conversion, sync-to-async lifts, return-to-Void drops, requirement-key forwarding, counted target environments and mutation writeback on success/error/cancellation. `value_borrows_parameter` makes indirect aggregate borrowing independent of whether unrelated code makes the whole program async; source thunks adapt direct conventions. Deep object conversion checks child representations coinductively—equal-sized Function pairs may need different entries. Wasm low-bit tagging applies only to async source tables, since sync table indices can be odd. These fixes address real lifetime/ABI issues; they are not scaffolding.

Generic native adapter tables include exact type/value bindings and selected methods, not only Repr. Producer roles/default entries survive split instance waves. However, the broad generic-to-generic runtime table machinery, stored/returned forall factories and containers are no longer requested semantics after November 15. **Drop those portions**; preserve exact identity when specializing the allowed named generic callback parameters and generic-to-concrete values.

`check/function_constants.rs` (534), `constants.rs`, `comptime/instances.rs` (502), host/lower and Session add semantic FnKeys/contexts with exact type/value bindings and methods. VM factories can invoke constraint methods, forward generics, retain phantom values and reify closure captures/defaults through typed AOT templates. Native `lower/function_constants.rs` (385) emits those constants. Named aliases track computed locals vs hoisted static aliases; aliases of runtime let/callback values get FN110; imported namespace/type aliases keep names/defaults and initialize in Program.init_order.

Gaps: the Rust reifier recursively descends Function-containing composites/closures; a finite-depth/stack-safe reifier and constant-composition factories are still listed as pending. It can report AN001 when it cannot recover an exact template/captured type. P/F's JS expr path has no `analysis.function_constants` consumer, so “factory works in VM/native” is not “factory works on Bun”. P explicitly leaves co-11 pending for that reason. The JS numeric/Void adapters described in NEXT are external patches, not applied here; erased callable-constrained numeric conversion and deep Function fields remain open. Do not publish a second runtime schema walker to solve them.

Notes: analyzer 439 VM instances; 442 + 47 LLVM checks, 8 function-value programs, 11 with and 2 factory programs (some historical platform skips); 444 forall and per-unit conversions; 445 deep/Void conversions; 447 function-type effects + alias delta; 448 alias hoisting; P 450 generic defaults/qualified aliases, then 453 union/worker fixes and 457 latest cumulative analyzer/clippy. The multiple native-only `tests/programs/function-*` fixtures need equivalent Bun coverage before cross-target completion is claimed.

### 2.9 Shared Guard/Worker/Pool/Socket/Listener/Subscription and Function RC

Best source H, with P/F's later Function lifetime/partial consumers. JS `runtime/ownership.js` has faOwnShared/faRetainShared/faReleaseShared and fallback faDropOwned; constructors in concurrency/workers/net/reactive/tasks acquire counted owners. Native C adds `FaEnvironment {drop,copy,count}`, retain/release, Subscription owners/disposal, and **a distinct counted Guard acquisition** that unlocks only when its last copy dies. A Guard must own the acquisition, not simply another mutex reference. Native lowering updates copy/drop for shared handles, watches, captured environments and source-selected moves. WorkerPool queued work and detached tasks must keep required environments alive.

P/F `runtime/function-tasks.test.js` has six added lifetime regressions: faStartValue retains before queueMicrotask, pre-entry cancellation releases without calling, running cancellation releases after the real promise/finally, faCallValue retains through result, and blocking/reentrant dispatch retains until it returns. Task is not retained. H notes 27 task runtime tests green and an isolated Function RC copy 28/28 JS lib; no aggregate H full suite. Guard/Subscription/closure fixtures exercise copied handles, captures and last-owner release; module-watch-cleanup and module-owned-detached cover module/realm lifetime.

Quality is uneven: counted Function/Guard design and cancellation tests are reusable, while H still includes Resource-sharing source and object-content fallback destruction. Its notes explicitly say “Never merge this worktree wholesale over root's unique-resource/AOT work.” Preserve genuine Sync/Function counting, remove Resource counting and fold fallback graph traversal into the final static operations. `faReactiveRelease` on explicit realm-worker close remains a named missing integration point. A copied local Socket/Worker handle is not automatically transferable across realms; lifetime and transfer capabilities are separate.

### 2.10 NeedsCodec, generated wire operations and transport

H alone has `check/wire.rs` (590), wire uses/requirements and `GenericRequirement::Codec(TypeId)`. The “NeedsCodec” name in notes describes this feature, not a universal TypeInfo field. Stable generic containers compose leaf operations; a union whose normalization depends on substitution requires one exact concrete codec. SCC growth analysis rejects expanding structural substitutions rather than imposing an arbitrary instance count. Interface tag110/remap/hash includes add/remove needs; tests change needs without structural signature changes and expect transitive invalidation. Unbound generic wire callable values are explicitly AN001 until a static adapter; ordinary generic calls request no codec.

H `generate/codecs.rs` (1,130) builds static typed writer/reader functions and explicit worklists for recursive values; `runtime/{codec,wire}.js` expose AOT primitives/fromFunctions. There are tests for 16k recursive objects, 12k nominal arrays, numeric union wire shape, byte ownership after packet reuse, optional-array capabilities, separate modules communicating through a real worker, and protected Resource DATA traversal. Ordinary Resource reference encoding is refused; protected DATA is distinct from sharing a Resource handle. Category references to Sync/Function need a realm protocol.

H `runtime/transport.js` (469) is a GSAB arena with synchronized allocator, stable IDs, atomic owner counts, instance directory, lifecycle/publication state and packets; transport tests exercise growth, reference churn and a worker fixture. **It is not connected to production Mutex/Channel/Worker serialization.** Runtime concurrency still calls JSON.stringify/parse, workers still use faShare/faRevive and pool callback dispatch; `FaTransportMemory` is not a production worker helper dependency. H's TypeInfo is only `{ ownsTask, hasCollection }`; P has none. The promised `.layout`, `.isUnique`, `.assign/.fillOwned` and root typed read/write/copy/drop contract are not delivered by these snapshots.

Notes report analyzer 386/386, three wire tests, JS codecs 13/13 (an earlier 9-codec checkpoint), with final whole JS lib/clippy repeat still pending. The external Codec repository's 129 tests/690 assertions and tsc/Biome refer to separate code, not validation of Fa transport. Keep the AOT portion, reimplement integration and review old Category/Resource assumptions. Update string/Bytes encoding for November 15 semantics; don't silently replace lone surrogates or use old Character cases.

Archived stash `c73686379d49beac2baab07553a9dd4d3b39c0a2` has untracked parent `59e9fc536d663281707bdeb92dbffefb3f7c1616`. `…^3:packages/codegen-js/runtime/managed.js` is 22 lines/1,000 bytes: a WeakMap of remote mutable facades and mutation hooks. `transport.worker.js` is 21 lines/760 bytes: an arena retain/release/create worker test. These are not the missing final production protocol. H now has `runtime/test-fixtures/transport-worker.js`. The stash also has an older 58,473-byte codec and 9,794-byte transport; H's checked-in AOT codec is the newer direction. Preserve archive refs, **do not resurrect managed Resource facades**.

Archived rules stash `fa81e961d12a3b339ad83bc888a9e37bff596b1e` (base `ef777e4`, untracked parent `efd44b283457630e39d68ac93fbb8dfa979ba360`) predates adoption of validated origins and has a broad 96-path tracked diff (+3,596/−475). It is historical reference, not an additional ordered commit to stack after R; doing so duplicates older origin/copy changes.

### 2.11 Generic Pointer<T> on Bun

Only G implements it. JS `generate/foreign.rs` adds layout_descriptor/composed_layout and generic allocate/read/write/free/value/cell/offset/toArray; runtime foreign.js adds descriptor caching and object/tuple/collection/fixed-array/optional factories. A descriptor supplies C spec, alignment, bigint conversion, copy and drop; opaque Fa blocks remain rooted by C address. `GenericRequirement::Layout(ParamId)` is carried in analyzer interfaces, remapped/hashed, forwarded through local/imported/qualified/higher-order applications; `instantiation_parameters` records exact binder order. Names.rs sanitizes synthesized JS identifiers. LLVM ownership queries use `concrete_type` before Resource checks.

Tests: CLI `pointer-generic.foreign.fa`, `pointer-references.foreign.fa`, `pointer-tasks.foreign.fa`; analyzer pointer-copy errors, generic-hidden-arguments, qualified-generic-arguments; unit substitution of nested nominal unions. Notes in G are stale (full suite “running”); P/F's later notes and compiler todo say full found invalid synthesized descriptor names and owner-capability argument order. G contains a name sanitization fix and an owner/order mapping fallback, but reading cannot establish that the failures were fixed and revalidated.

Do not port the separate Layout ABI or runtime schema-shaped copy loops as a second operation system. Unify layout with final TypeInfo/AOT copy/drop, selected written method/destructor identity, and final unique-resource/Task restrictions. G's move-only pointer reads report MEM002; final copy policy may want the common MEM003 emitter. Generic pointer reads must not duplicate Task/Resource through erasure; `.value` borrows and cannot outlive/free its block. Composed generic Functions currently have descriptor cases without full counted drop/copy, another reason not to claim completeness. Bun is the execution target for new pointer capabilities; native/free/wasm and C library ABI are regressions to retain, not evidence from this read-only review.

### 2.12 Set hashing and equality

Best complete reference P/F, frontend also R/H. Builtins `array.rs` / checker collections remove key parameter and artificial Set HAS_SHARED, use CallInfo `[source,capacity]`, reject `key=` with FN023. JS P/F removes FaSet.key and associated callback ownership/equality/copy paths; uses ordinary Fa hashing/equality for nonprimitive elements and built-in Set where valid, preserving insertion order. H's runtime **still has** `this.key`, keyOf and retained key callbacks: its notes warn not to restore it.

LLVM P/F `tables.rs:1947` removes unsupported-key handling and changes capacity slot from 2 to 1; normal value hash/equality already provides the desired behavior. Interpreter Set.from already deduplicates by normal value equality; no new key callback semantics needed. Neither parser nor formatter needs a new construct. Documentation is missing. Test `collections/set-standard-equality.fa` checks equal objects and hint side effects once; `set-key-options.fa` is negative; se-3 uses Map for separate keys. Reconcile new capacity/source-cleanup and frame table storage from perf before taking the native hunk. This is a good independent port, provided docs and both backends change together.

### 2.13 Written-origin unions and semantic fingerprints

Unique written origins were already delivered/ported into compiler before these snapshots; W's NEXT attributes complete full LLVM/clippy/runtime validation to that earlier lot. `WrittenType` carries declaration, exact arguments, nested origins and selected methods; GenericMethod carries owner selections. Containers retain a common nested origin only when all literals agree (`check/structural.rs:492–498`). **No snapshot implements the new runtime union of equal-field Person/Greeting origins**. TypeStore structural unions still collapse equal structures, and ValueOrigin::Alternatives does not tag runtime members. Fresh implementation must preserve dispatch/destruction/defaults through joins, arrays, imports and generic calls without changing ordinary structural assignability.

W's entire 115-line delta adds `WrittenType::semantic_fingerprint`, recursive canonicalization of nested origins and method-order sorting, plus GenericMethod fingerprint and one unit test. It preserves phantom arguments and nested method choices; permutation should not change identity. No consumer is added by W and no other snapshot includes the method. It is a small, reviewable port for descriptor/finalizer caches, **after actual substitutions**; it does not create a universal descriptor key or canonicalize unresolved types by magic. Add cross-store/remap/order/nested-depth coverage; the existing one test is unvalidated snapshot code.

### 2.14 Pass fusion and remaining analyzer rules

F is the only source of fusion work. `scan.rs` collects Arc parent_edges during its existing scan; Analysis borrows them, `check/mod.rs` and ProofFile consume them, use/reuse/provide record reference roots during checking, nonescaping checks reuse sparse move reads rather than scan every AST node. RegionFile lazily constructs owners/function-name maps using the shared edges only if proof needs them. A new `parent_edges_are_shared_and_keep_nested_function_boundaries` regression checks Arc sharing and nested function boundaries. This is a useful reduction in redundant traversals; it is **not** complete single-walk analysis. Copy/provenance/implicit-write/effect/cycle finishing passes remain. There is no measured 1,000-file CPU/wall/RSS result in these notes, and no final fusion-specific suite. Reimplement a minimal optimized delta after semantic fixes, retaining compiler's later region capacity/physical-layout facts.

P adds FN086 for unbound method values, FN110 for invalid function declarations/factories, full function-type mutates/invalidates/requires scopes, requirement-key comparison independent of local names, return-to-Void, generic value default GN003/GN005/GN008 and alias hoisting. Many removed pending markers simply activate existing functionality, rather than correspond to a new implementation. P removes 21 pending markers total vs compiler; F removes two and R/H one. Co-11/co-12 and key-loop inferred effects remain pending. `schemas.rs` is unchanged in the WIP delta; arbitrary TypeFromSchema inside generic types, native Schema nominal constants, implicit default/destructor effect summaries and several documented diagnostics remain unfinished.

Static inventory: P has no production reference outside the diagnostic registry to these 14 type-code constants: CL023, CV003, EF020, FN094, GN017, GN030, TF002, TY001, TY012, TY043, TY044, TY072, TY073, WFN002. This is **not** fourteen proven missing behaviors (another diagnostic/alias may cover one; CL023 is obsolete with Set keys). GN030 specifically needs the new narrowed scope. Lexical/CLI codes absent from analyzer need no analyzer emitter. Audit meaningful remaining cases against documentation, not only registry occurrence counts.

P also changes direct throw/catch semantics in `check/errors.rs:369+`, adds failure FlowState joins and `packages/cli/tests/programs/lexical-throws.fa`. This is absent in F and contradicts the “no guessed semantic edit” NEXT text. See §4.2 before landing that hunk. Qualified worker constructor identity and named-union defaults are useful P-only fixes described as separate slices below; neither is origin-union completion.

## 3. Landing plan

### 3.1 Disposition by topic

A = port a focused WIP file/hunk with its tests onto the merged trunk; B = reimplement on trunk, using the WIP as a reference; C = drop the implementation/behavior from the landing scope. “A” never means cherry-pick the archival snapshot or overwrite a touched trunk file.

| Topic | Disposition | Reason / source |
|---|---|---|
| Sync payload global remap | **A** | P/H types/merge.rs, small independent correctness fix and regression. |
| Written semantic fingerprints | **A** | W analysis.rs only; add consumers deliberately and test remapped identity. |
| Contextual Copy grammar/formatter/LSP | **A** | Common P/F/R/H hunks, transplanted into perf's parser and formatter. |
| Final ownership predicates, MEM003/MEM004 and move/capture conditions | **A/B** | Port P predicates/tests; integrate checks and runtime ownership afresh. R transfer extension is separate, H policy is obsolete. |
| Portable callable provenance and value state | **A**, then finish holes | P structs/finishes/interfaces and R transfer reference; no alternate graph per backend. |
| Resource/Task backend ownership, generic defaults and written finalizers | **B** | Complete root integration isn't in these six; H Resource RC must be removed, P bridge isn't final. |
| Function environment/Guard/Subscription counting | **A/B** | Reuse H/P runtime protocol and regression scenarios, integrate with current static operations and native ABI. |
| Mutex store and WorkerPool repeated state copy | **A** | Latest P analyzer tests/contract, then backend acceptance with unified capabilities. |
| Monomorphic with, visible names/defaults | **A/B** | Port P analyzer/interpreter and JS structure; rewrite native entry/return integration and production cleanup. |
| Ordinary concrete function adapters and aliases/factories | **A/B** | P has useful conversion/reification algorithms; production JS and native destination ABI need new work. |
| General stored/returned forall, runtime forall class adapters | **C** | Explicitly conflicts with perf November 15 GN030 rule. Keep only allowed static parameter specialization/generic-to-concrete parts. |
| Generic Pointer<T> on Bun | **B**, small A fixes | Unify G Layout into final TypeInfo; preserve names/parameter-order fixes and regressions, not the old parallel ABI/runtime copy engine. |
| Set standard hashing/equality | **A** | P/F across analyzer/JS/LLVM + documentation. H old key runtime is C. |
| Written-origin unions | **B** | No implementation to port. Reuse existing single-origin metadata and method keys. |
| Sparse NeedsCodec + AOT codecs | **A/B** | H sparse inference/generator is substantial; rebase metadata and new strings/Bytes; do not universally request codecs. |
| Production binary Mutex/Channel/Worker/Pool protocol | **B** | Arena/codec components exist; production hook-up, transfer/finalization and shared category refs are missing. |
| Resource GSAB facade / dynamic schema codecs / Task RC | **C** | Revoked or superseded. Keep archival refs only. |
| Analyzer pass fusion | **B** | Reapply only F's optimization delta after semantic work; new tests and measurements required. |
| Remaining diagnostics, TypeFromSchema, key-loop effects, destruction effects | **B** | WIP notes acknowledge gaps; no complete delivery in the six. |
| Inline union payload defaults, qualified worker/function identity | **A** | Latest P isolated fixes with cross-file tests; keep distinct from origin unions. |
| Direct lexical throw/catch change | **Hold for lead**, then A/B | Actual P semantic change lacks recorded decision and contradicts its own checkpoint; do not smuggle it in with flow fixes. |

### 3.2 Actual compiler-side merge conflicts

Commands used, sequentially, against the pinned refs:

```text
git -C /home/claude-dev/projects/Fa -c gc.auto=0 merge-tree --write-tree --name-only compiler <WIP>
git -C /home/claude-dev/projects/Fa diff --stat f75ce8a4 d256f11a -- <paths>
```

Every merge-tree exited **1** (conflicts); no result was checked out. Counts are conflicting paths, not hunk counts. Lowercase paths in this table are literal.

| WIP | Conflicts | Paths | Inspection result tree |
|---|---:|---|---|
| P `22cd8d29` | 2 | `packages/codegen-llvm/src/lower/mod.rs`; `packages/codegen-llvm/src/units.rs` | `e713d5503d20e5cb8e98679b46a6a25c5994a48e` |
| F `706f8f7a` | 3 | P's two plus `packages/analyzer/src/regions.rs` | `425d518011762938ac241aa1f0540147a8b78c4a` |
| R `c4eeeeec` | 7 | `NEXT.md`; analyzer `src/{analysis,lib,modules}.rs`, `src/check/structural.rs`, `src/types/store.rs`; LLVM `src/units.rs` | `2b71e298c90bf55836261255d306c2fbadd57627` |
| W `5259a8b8` | 3 | `NEXT.md`; analyzer `src/analysis.rs`; LLVM `src/units.rs` | `cb30778f5aaec381960ba5e62fa741c3ed173f40` |
| H `3c8cc4d0` | 12 | `NEXT.md`; analyzer `src/{analysis,lib,modules}.rs`, `src/check/{key_sets,structural}.rs`, `src/types/store.rs`; JS `src/generate/generics.rs`; all three LLVM runtime `.ll` files; LLVM `src/units.rs` | `32a45746e8558131e9795a8b8f097b08071c4257` |
| G `54039eed` | 24 | `NEXT.md`; `design/{decisions,pending-documentation}.md`, `design/implementation/advanced-and-architecture.md`; documentation `advanced/{foreign-code,structural-typing}.md`, `collections/maps.md`, `types/{fields,generics}.md`, `values/strings.md`; analyzer `src/{analysis,modules,session}.rs`, `src/check/{calls,generics,interface,key_sets}.rs`, `tests/cases/modules/generic-hidden-arguments.fa`; CLI `tests/programs/pointer-allocate-c.foreign.fa`; JS `runtime/foreign.js`, `src/generate/{foreign,functions,generics}.rs`; LLVM `src/lower/ownership.rs` | `a9bb212b0ed2616a2f98c06d394bacde4168ec7d` |

W only has one changed file, yet a whole-branch merge also conflicts with NEXT and LLVM units: this is ancestry/base divergence, not a dependency of fingerprints on LLVM. Compiler has ported earlier work without retaining all of these exact parents. Extract **parent→WIP hunks** for intended changes; distinguish that from compiler→WIP whole-file replacement. The same applies to G's old metadata/docs. P's two textual conflicts are misleadingly small: semantic ownership integration and the other line's ABI changes remain substantial.

### 3.3 Other-line impact and conflict map

`f75ce8a4..d256f11a` changes 701 paths overall. Relevant measured stats:

| Conflict label | Paths | Diff stat | What must survive |
|---|---|---|---|
| **S** syntax | `packages/parser`; plus formatter/LSP | Parser 21 files +613/−115; parser+formatter+semantic_tokens 24 files +702/−167 | New angle parser/recovery and formatting decisions. |
| **A** checker | `packages/analyzer/src/check` | 27 files +487/−224 | Generic syntax/restriction work, capacity/reference/effect changes; functions/generics/type_expr are shared hot spots. |
| **B** builtins | `packages/analyzer/src/builtins` | 4 files +84/−12 | Capacity signatures/effects, current strings definitions. |
| **N** native code | `packages/codegen-llvm` | 81 files +21,098/−7,591 (includes generated IR/tests) | Native storage/return/collection/performance changes. |
| **N-call** calls/ownership | LLVM `lower/{calls,closures,ownership,values,mod}.rs` | 5 files +496/−34 | Indirect/direct result ABI, frame destinations, current cleanup. |
| **N-obj** construction | LLVM `lower/{objects,patterns}.rs` | 2 files +82/−8 | Early boxes, completed fields, caller output alias invariants. |
| **N-table** tables/runtime | LLVM `lower/tables.rs`, runtime C | 2 files +943/−51 | Set/Map frame storage, reserve ownership and new GrowingArray runtime. Regenerate IR from merged C; never take H's old blobs. |
| **J** JS | `packages/codegen-js` | 13 files +99/−30 | Capacity evaluation/cleanup and generated fixtures. Existing JS function emit file alone +25. |
| **J-set** JS Set overlap | JS `runtime/{collections,equality}.js`, `generate/collections.rs` | 1 file +27/−2 | Capacity/source cleanup while simplifying key paths. |
| **I** interpreter | `packages/interpreter`; analyzer comptime | 4 files +50/−32 combined; interpreter alone 1 file +26/−6 | Capacity handling and existing integrated continuation VM. |
| **D** specification | `documentation` | 58 files +1,177/−754 | New scope/strings/Bytes/capacity/storage rules. |

The other line leaves analyzer analysis.rs, types/merge.rs, scan.rs, check/locals.rs, proofs.rs and regions.rs unchanged relative to f75. Likewise the selected JS generic/foreign/names files, native generic_values/program/units/split group and JS runtime concurrency group have no diff on that line. That lowers **textual** conflict risk, not compiler-side divergence or semantic dependencies. New native files `lower/{collection_storage,copies,return_destinations}.rs` (+1,058/+558/+279 lines) are essential integration context even when a WIP file has no conflict marker. The reviewed small-result return thresholds must be preserved as well as the new output-slot paths.

### 3.4 Ordered, focused slices

Each row is a separate reviewable commit/session scope, not one engineer's assignment to merge a branch. Rows with no dependency can proceed independently after the merge settles; ordering is for landing. P/F/R/H/G/W refer to the exact commits in the opening table. Conflict labels refer to §3.3; compiler-side conflict paths are in §3.2. If a row grows beyond its stated case, split it before expanding. Some features intentionally require several sessions.

| # | Scope and source | Depends on / expected conflicts | Done when (future validation, not run here) |
|---:|---|---|---|
| 1 | **Reconcile specification and generic scope.** Record final unique Resource/borrowed ABI, explicit Copy and Set; preserve November 15 restricted generic values. Use compiler todo, P/R/H NEXT and perf records; don't merge old decision files. Add narrowly scoped GN030 tests. | Lead answers §4.2 where needed; D/A/S. No other row should silently decide throw/default timing. | Spec and implementation notes agree; old stored/returned forall positives have explicit GN030 expectations, allowed named callbacks/forwarding and mono instantiation still pass. Parser doc snippets/website checks later. |
| 2 | **Global Sync payload remap.** Port P `types/merge.rs` case + `shared_handles_remap_generic_payloads_before_publishing`, compare H's independent regression. | Independent; no perf diff in file. | Local/global generic Mutex/Guard/Channel/Worker/Pool payloads contain no local TypeIds; substitution and cross-file factory regression pass without panic. Analyzer pipeline + native per-unit case. |
| 3 | **Written fingerprint API.** Port W's 115-line delta only. Add consumer-independent tests for ordering/phantom/nested selections and cross-store remap. | Independent of Copy/backend; compiler analysis.rs has textual ancestry conflict, no perf change. | Existing fingerprint test plus permutation and remap tests pass; substituted Person/Greeting, phantom values and selected destroy choices remain distinct. |
| 4 | **Copy syntax and tooling.** Transplant common P parser/token/node/render hunk, formatter printer, syntax View and LSP arm. Include Copy expression lowering to existing copy operation without enforcing new global rules yet. | S/I/N-call; preserve parser generics/recovery. | P precedence/context cases, doc parse corpus, formatter preservation/idempotence and semantic-token test pass; `copy(x)`/identifier copy remain calls/names; explicit ordinary Array copy runs Bun + all LLVM targets. |
| 5 | **One ownership predicate contract.** Port P/R TypeStore final `is_move_only` / `takes_task_parameter` / collection/shared/affinity queries and regression tests. Keep these separate; establish TypeInfo shape + ordered hidden ABI using current metadata. | 1; A/B; R/H analysis/store/interface conflicts. | Resource-with-Task borrows, Task composites consume, Sync boundaries share, phantom args don't own, empty Array<Task> and unions retain static ownership facts. Analyzer/interface remap/hash tests pass. |
| 6 | **Portable callable facts.** Port P provenance data structures, stable merge/hash/substitution and interface/session publication. Exclude partial/factory consumers initially. | 3/5; analysis/modules/session integration; no competing runtime graph. | Direct/imported callable facts survive provider AST disposal; same structural type/different method choices differ; changing only summaries invalidates importers. |
| 7 | **Evaluation-time value state.** Port common `check/value_state.rs`, flow callable slots/joins, read/capture hooks and P provenance builder's matching consumers. | 6; A; retain current loop/control flow. | Both value_state test groups pass, including hidden callable assignment/conditional/holder/named capture and saved-before-later-write cases. Add .push/.set and loop producer cases to close explicitly noted holes. |
| 8 | **Concrete moves and collection Copy checks.** Port P moves/copy-site core and MEM003/004 emitters for non-generic locals, aliases, fields and exclusive branches. Migrate existing fixtures with semantics review. | 4/5; A; do not turn on unfinished generic propagation. | Explicit-structural-copy, replacement-moves, branch/alias/failing-RHS tests pass; no test weakened/skipped. New error messages suggest copy/new instance/take as appropriate. Bun + native/free/wasm execution of positive migrations. |
| 9 | **Generic and higher-order copy summaries/captures.** Complete P copying finish on 6–8; fold R worker-transfer discriminator only in its later transfer slice. | 6–8; A and interface maps. | `generic-copy-conditions.fa`, `callable-copy-conditions.fa`, resource-copy-contracts, fresh Task fields and escaping-capture cases work locally/imported; no unique capture sneaks through a factory/field; deferred key-loop copies remain explicitly assigned to 33. |
| 10 | **Counted Function environments on JS.** Port H/P ownership primitives, function materialization/copy/drop and six start/call lifetime regressions. Exclude Resource counting and static-type-independent graph scanning. | 5/6/9; J; merge closure temporaries with current emitter, one operation system. | Runtime function-tasks tests, callback-owners and closure capture scenarios pass; exactly-one last-owner cleanup on normal/error/reentrant/cancel-before-entry/running cancel; Task never retained. JS helper dependency isolation and golden checks pass. |
| 11 | **Counted Function environments on native.** Port H/P environment header/retain/release and capture order; adapt closures to current direct/indirect result convention. Regenerate all three IR files. | 5/6/9; N-call/N-table; exclude `fa_resource_new`. | Closure shared captures/detached env programs pass native/free/wasm with leaks; unchanged ABI/C library checks pass; byte-sized/small aggregate vs output-slot returns respect perf thresholds. |
| 12 | **Unique Resource/Task ownership on JS.** Reimplement borrowed Resource parameters, owning sinks, replacement/default temporaries and Task consumption using 5/9/10. Use P/R frontend metadata, not H Resource hooks. | 5/9/10; J. | resource-holders, resource-borrow-errors/defaults, task-moves, task-argument-views and owned-task-parameters scenarios pass; provided later arguments cleaned when an earlier default fails; borrowed Resource never finalized by callee. Run full relevant Bun programs/runtime. |
| 13 | **Unique Resource conditional/projection ownership on native.** Reimplement branch flags/backing temporaries and owned projections against current storage/returns. P `owned_projections.rs`, H conditional-borrow description are references. | 5/9/11; N-call/N-obj; preserve early-box frame locality and noalias. | if/when/catch/fallback/loop Resource borrowed results never become owners, Task keeps consuming ABI, extras dropped once, guard-local escapes rejected by analyzer. resource/Task conversions on all LLVM targets/leaks, whole and split. |
| 14 | **Written generic Resource finalizers.** Add ResourceDrop/written-parameter publication and selected finalizer capabilities using current single-origin data. H NEXT describes the separate delivery; no wholesale H source exists to take. | 2/3/12/13; N-obj/J/interface hashes; honor tag111 allocation. | Ticket<Integer>/String, Person/Greeting same layout, phantom labels, mutable self, aliases/imports and named-union generic payload destruction match on Bun + all LLVM targets. Incremental imported parameter change alters selected destroy without structural FnSig change. |
| 15 | **Guard and Subscription local lifetime.** Port H distinct acquisition control block, subscription owner/dispose and module cleanup; include worker realm-close reactive release. | 10–14 as relevant; N-table/J; current C runtime and module lifecycle. | guard-copies, guard-array-copies, detached-guard-capture, subscription-copies, module-watch-cleanup/module-owned-detached pass relevant targets with exact unlock/dispose counts and leaks. Explicit stop behavior documented. No blanket assumption of inter-realm Send. |
| 16 | **Mutex stores and WorkerPool state contract.** Port P concurrency sink/copyability hunks and tests; wire to 5/12/13 operations. | 2/5/9/12/13; A/J/N. | Three P stored_payload_tests and worker-pool-unique-state pass; Mutex<Resource> can share locally, borrowed Resource/Array store rejected, erased Task store moves once; state replay/restart independent copies validated on supported worker targets. |
| 17 | **Set standard equality frontend/docs/JS.** Port P/R builtins/checker, P runtime/generation/copy/equality changes; replace docs key example with Map. | Can land independently of 6–16 after 1; B/A/J-set/D. Preserve perf hint evaluation/cleanup. | set-key-options negative, set-standard-equality and se-3 Map positive; hint evaluated once even on JS, throws clean source; runtime sets/maps suite, Bun programs and reviewed goldens green. |
| 18 | **Set native constructor ABI.** Port P tables.rs slot-one hunk onto perf's storage/from cleanup; remove stale unsupported-key path. | 17; N-table. | Same Set equality/hint programs on native/free/wasm/leaks, capacity/overflow/stack-set tests unchanged; interpreter has normal Set equality and side effects. No revived key callback. |
| 19 | **Unified Bun pointer capabilities.** Reimplement G layout_descriptor/foreign operations using final TypeInfo and AOT copy/drop; retain names.rs sanitization and exact parameter identities. No separate Layout-only hidden ABI. | 3/5/12/14; G's 24 compiler conflicts, J/A; no direct perf foreign diff. | pointer-generic/references/tasks, pointer-copy-errors, qualified-generic-arguments and generic-hidden-arguments pass on Bun; C view writes/offsets/ownership correct. Empty/noncopyable generic reads rejected or borrowed, Function-containing targets retain/drop correctly. |
| 20 | **Generic owner/callback capability forwarding.** Close G full-suite failures with current metadata/origins; static generic callbacks only per 1. | 19/6; generic_functions, instantiation_parameters, JS names/generics and native concrete ownership. | Owner phantom/type/value order, imported/qualified methods, callback instantiations and same-layout selected methods pass Bun + native/free/wasm/C-library regressions. A complete suite must finally demonstrate resolution of the historical name/order failures. |
| 21 | **Visible signature metadata + monomorphic with frontend/VM.** Port P signatures/named_signatures/PartialApplication, names/defaults/bound diagnostics and interpreter Op/Gather/Finish. Defer unsupported residual forall. | 4/6/9; A/I/interface tag109. | Visible differently named equal signatures, imported aliases/fields, duplicate bound name, mutated/variadic and unique capture cases pass; interpreter's three partial tests and runtime order/default cases pass. New generic scope diagnostics retained. |
| 22 | **Monomorphic with JS consumer.** Port P partial.rs onto 10/12 typed captures, source-order guards, omission/default/requires ABI. | 21/10/12/20; J. | with-runtime/defaults/mutable-arguments/requirements/imported-named-storage fixtures pass Bun and Node where supported, no retained mutable/unique bound value, creation errors release earlier captures. |
| 23 | **Monomorphic with native entries/defaults.** Reimplement P partial/default_values against current return destinations, small returns and counted env protocol. | 21/11/13; N-call/N-obj. | Same sync with fixtures native/free/wasm/leaks including imports and per-unit compilation; fresh collection results build into legal destinations, defaults retain correct declared origin. |
| 24 | **Async with and permitted static generic specialization.** Port P cancellation/default-entry producer scenarios, avoid generalized stored forall tables. Implement only legal named/static generic cases approved by 1. | 22/23; lead default-timing answer if observable tests require it; N/J generic dispatch and tasks. | with-async, with-async-values, with-task-arguments, with-resource-defaults pass Bun + all supported LLVM targets/leaks, including cancel before first turn and after child starts, requires and error propagation. |
| 25 | **Concrete Function coercions.** Port P assignability/deep representation rules; rewrite native adapters against new result ABI and JS through static templates. Include value→Void disposal and mutation writeback. | 10–14/20/23; N-call/N-obj/J. | function-value/deep/owned/void-conversions and function-type-clauses scenarios match Bun/native/free/wasm, return/error/cancellation writeback/drop checked. Add Bun equivalents of native-only fixtures. Erased numeric callable constraint in §4.2 covered. |
| 26 | **Hoisted and qualified function aliases.** Port P static_function_aliases/computed-functions/init-order and FN110 changes; retain mono generic-to-concrete aliases. | 21/25; A/J/N generic values/program; no general forall storage. | Early local/module alias, namespace/type method alias, imports/defaults/captures in function-alias-owners and units-function-aliases work Bun and all LLVM targets, split not fallback; runtime aliases rejected with FN110. |
| 27 | **Compile-time instances and mono factory constants.** Port P comptime/instances + exact typed constant model; make reifier iterative/budgeted and add JS constant consumer. Separate native constant emitter uses current ABI. | 6/14/25/26; I/J/N; no returning forall. | function-factories/imported, phantom values, selected methods/defaults and factory errors run VM/Bun/all LLVM targets/split. co-11 activated only when every supported target works; deep finite closures don't overflow host stack. Constant compositions only if included with explicit tests, else separate followup. |
| 28 | **Inline named-union payload defaults.** Port P member_fields/member_defaults, object lookup and both backend binder forwarding, hash tag112. | 2/3; independent of with for basic constants; A/N-obj/J/interfaces. | modules/union-payload-defaults and native union-payload-defaults pass Bun + all LLVM targets/split/leaks; equal-shaped variants keep distinct defaults; recursive Tree and empty generic collections fresh each construction. Do not claim portable runtime defaults until §4.2 is resolved. |
| 29 | **Qualified worker constructors.** Port P written exported-identity lookup/aliases without making nested types workers. | 2/current written origins; A, worker dispatch in both backends. | qualified-worker-constructors/fields tests and aliased .worker exports work on supported native/Bun package paths; field access CONC008 remains, wasm platform limitation explicitly retained. |
| 30 | **NeedsCodec inference/interface.** Port H wire.rs/maps/hash/remap to P-compatible provenance with permitted static callback scope. | 5–7/20; A/modules/session/GenericRequirement, tag110. | Three wire tests, add/remove same-signature invalidation across three modules, exact union normalization and expanding SCC refusal pass; ordinary generic calls request no codec. Forbidden generic value forms get GN030, not a competing AN001 rule. |
| 31 | **AOT DATA wire generator.** Port H codecs.rs, codec/wire runtime with current strings/Bytes/Set and final owner operations; complete exact concrete leaf categories. | 14/17/19/30; J/D; old Character cases superseded. | H's 13 codec tests + protected Resource DATA, lone-surrogate strings/Bytes, separate-module wire compatibility, long recursive worklists, packet reuse independence. Runtime helper isolation; no schema interpretation/eval/new Function in Fa execution. |
| 32a | **Shared transport arena/category lifetime.** Reuse H transport allocator only after API/lifetime review; integrate Sync/Function category refs and explicit realm close. Exclude Resource facades. | 15/31; JS runtime. | Real-worker growth/identity/retain/release/error/realm-shutdown tests pass; no unfinished records/retained environments leak, release exactly once. Shared owner and packet failure paths covered. |
| 32b | **Mutex and Channel binary payloads.** Replace production JSON calls with typed codecs/ownership under lock; preserve Guard loans. | 16/31/32a; JS concurrency/generation. | No JSON serialization remains in these production paths; independent DATA vs common Mutex state, timeout/cancel/closed/throw cleanup and non-ASCII/Bytes roundtrips run actual Bun/Node workers and web where supported. Thread Task rejection persists. |
| 32c | **Worker/Pool argument/result transport and exclusive transfer.** Integrate binary state/jobs/results/errors with R's transferred-view/thread-move proofs and final unique policy. No general JS-local ban. | 7/14/16/29–32b; analyzer concurrency/provenance and JS workers. | resource-worker-transfers, worker-callable/guard-affinity, hidden-worker-task, copied state/restarts and result errors pass real supported workers; old alias invalidation proven; native worker regressions/leaks pass. No JSON, unprotected Resource alias, or Task crossing. |
| 33 | **Deferred key-loop effects/copies and missing diagnostic rules.** Reimplement specialized summaries/caller refresh, default/destruction effect sites and prioritized remaining documented diagnostics. Split by rule if needed; TypeFromSchema/native Schema is a separate focused followup. | 9/current effects; A/B, preserve capacity implication. | generic-key-loop-inferred-errors loses pending only after correct local/imported callers; wrong copy/effects after specialization diagnosed at application; full exact-throws corpus unchanged. Schema work has explicit positive/negative fixtures on VM/Bun/LLVM before considered done. |
| 34a | **Origin alternatives in analyzer/interface.** Fresh design/implementation of equal-field written origin joins and union identity; common single-origin code only as reference. | 3/6/14/28; lead public union rule agreement; A/types/interfaces. | Person/Greeting conditional, array, annotation/import and generic return alternatives retained; structural assignability and ordinary union diagnostics stay coherent, stable incremental hash changes. |
| 34b | **Origin union dispatch/destruction in both backends.** Runtime tag/dispatch for actual written alternative through copy/move/erase, sharing native layouts only when semantically sound. | 34a/12–14/25; N-obj/J/codec keys. | Same-field different-method AND different-destroy programs execute correct branch on Bun/native/free/wasm/split/leaks; union copy/noncopyability follows members; no arbitrary first equal-TypeId method/default/finalizer. |
| 35 | **Measured pass reduction.** Reimplement only F→P optimization differences: shared scan parents, read/reference reuse, lazy region facts; preserve latest semantic code/closed facts. | All relevant semantic slices stable; regions compiler conflict but no perf file diff. | New Arc/nested-function test, deterministic diagnostics, proof invalidation/regions/corpus/full suites pass. Same frozen 1,000-file package: CPU, wall, peak RSS and diagnostic equality before/after, no claimed gain without measurements. |

The rows represent a sequence, not permission to retain a half-integrated policy on the release branch. For interdependent ownership changes, keep an integration branch while reviewing small commits, then run the full acceptance suite when the contract is coherent. Independent Set, fingerprints and Sync remap can land earlier. Schema completion and remaining diagnostic audits should be broken into one rule or schema feature per session rather than bundled into a miscellaneous “finish analyzer” commit.

Future per-slice checks are proportionate; before final compiler integration run the repository's required fmt, clippy workspace/all-targets, full workspace tests **with LLVM enabled**, JS runtime/helper/golden checks and website check/snippets/build. Printing fixtures must execute on Bun and native/native-freestanding/wasm where supported, leak-checking the LLVM runs. Multi-file generic/alias/default/finalizer cases must run per-unit as well as whole-module; a silent fallback to one module does not prove the split ABI. Keep genuine platform limitations explicitly distinguished from newly hidden failures. Do not delete/weaken/skip tests to make the integration green.

## 4. Risks, fixture migration and open questions

### 4.1 Quantified migration and behavior risks

**Minimum confirmed Copy migration in the snapshots:** compared with compiler, P/F add **28 Copy-bearing changed lines in 22 existing `.fa` fixtures**: **23 lines in 19 existing analyzer case files**, one JS golden fixture line, and four lines across two existing LLVM program fixtures. R/H carry the same 19 analyzer fixture changes (23 lines), without the additional golden/native migrations. This is a static diff count of changed source lines containing prefix copies, not number of diagnostics or every required migration. An additional existing LLVM Rust test string adds `copy child`; parser Copy test additions are new grammar tests, not migration sites. The notes' “Twenty historical analyzer sites” is not a verified complete repository-wide count.

Existing analyzer fixture paths (under `packages/analyzer/tests/cases/`) are:

| Group | Files | Added Copy lines |
|---|---|---:|
| Reference/relations | `advanced/04-reference-types-use-mutable-use-reuse/adv-4.5-01.fa`; `advanced/08-relations/adv-8.10-12.fa`, `adv-8.10-19.fa` | 3 |
| Foreign | `advanced/13-foreign-code/adv-13.9-07.fa` | 1 |
| Async/parallel | `advanced/14-async/detached-and-shared.fa` (2), `doc-16-worker-pool.fa`; `advanced/14-concurrency-without-async/adv-14.6-01.fa` | 4 |
| Adversarial | `adversarial/async-11.fa`, `flags-shift-and-move.fa`, `rea-01.fa`, `tgt-01.fa` | 4 |
| Callback collections | `collections/throwing-callbacks.fa` | 3 |
| Schemas | `comptime/schema-types.fa`, `comptime/schemas.fa`; `types/3.12-type-functions/type-from-schema-generic-errors.fa`, `type-from-schema-generic.fa` | 4 |
| Imported generic copy | `modules/generic-copies.fa` | 1 |
| Function captures | `review/move-through-function-value.fa` | 2 |
| Optional holder | `values/14-narrowing/optional-fields.fa` | 1 |

Additional existing fixtures: `packages/codegen-js/tests/golden/check-bits.fa` (1), LLVM `tests/programs/passive-tree-arenas.fa` (2) and `return-local-ownership.fa` (2). `packages/codegen-llvm/tests/checks.rs` adds one Copy in an embedded program. These confirm that recursive returns, pool captures and Schema transformations rely on the old implicit policy, not just simple array assignment tests. Compiler documentation and perf's new stack/copy fixtures add more potential sites; **their complete number cannot be established without checking the merged analyzer**, forbidden in this task. The 22 is a lower bound, not a forecast of only 22 affected files.

Other material risks:

- **Spec/backend mismatch already inside snapshots:** P/R frontend borrow Resources but old backend parameter-owner cleanup persists; H final NEXT says unique but source still shares Resources. Don't treat a passing frontend or standalone runtime test as end-to-end ownership proof.
- **Task ABI vs noncopyability vs Send:** three distinct questions. Empty static Task containers still have ownership behavior; Sync<Resource<Task>> can be a copyable local handle but cannot transfer Task to a different executor. Phantom type parameters must not imply ownership. Resource containing Task still borrows at ordinary calls.
- **Observable last-owner lifetime:** a last textual read of Guard/Subscription/Socket/Function cannot silently end a lexical owner when release/unlock matters. Environment creation and cancellation must retain actual owners until real invocation completes. No content scan can replace the static generic contract.
- **Runtime result layout:** P's adapters/partial/functions predate the stack/result ABI. Restoring their aggregate convention can break both small SSA results and caller slots; a new pointer argument shifts hidden/default/environment positions. Output destinations must not overlap arguments/captures/globals. Frame/static storage never freed, but owned items must still drop on errors.
- **Cache identity/invalidation:** merge tags109 names,110 wire,111 written parameters (external delivery),112 member defaults without collision; public map removal must invalidate too. Type layout alone loses phantom values, methods/destructors and equal-shaped defaults. Fresh native cache versions should describe the merged contract, not select one old suffix from each worktree by hand.
- **Origin unions:** an analysis alternatives list isn't a runtime discriminant. Structural TypeId equality must not select a destructor/default/method. Reusing “same LLVM size” is insufficient for nested Function ABI or Task projection.
- **Silent test activation/semantic shifts:** P removes 21 pending markers against compiler, F two and R/H one; P removes ft-9 native skip entries after claimed validation. Port activation only after each target executes the now-supported behavior. Co-11 remains pending because JS factory constants are absent; co-12/other composition cases are not complete. The new lexical-throws fixture is a new semantic choice, not merely test cleanup.
- **Native-only tests:** most new conversion fixtures live under LLVM's program directory. Their successful LLVM historical runs cannot demonstrate JS support; add common analyzer/CLI printing cases or explicit Bun variants.
- **Historical external patches:** H's generic finalizers and conditional ownership have good claimed test counts, but those deliveries are outside the six snapshots. NEXT references are pointers, not proof the corresponding WIP contains those fixes. Similarly P's JS adapters and production TypeInfo/AOT changes are external.
- **No performance evidence for fusion:** F still has multiple summary passes and a broad WIP; assess only its isolated optimization against an unchanged semantic baseline. Keep same-binary/noise controls for native performance work; perf already closed layout questions that the old NEXT still calls experimental.

### 4.2 Verbatim unresolved questions, options and recommendations

These are the distinct open **language/design** issues in the six NEXT files, including implementation design choices that were not settled in the stored snapshots. Repeated inherited paragraphs are quoted once. Older “OPEN with work” lists are tracked separately below because newer checkpoints solve parts of them. Options/recommendations here are this assessment's advice; the lead decides language questions. None is implemented by this report.

**Q1 — When are defaults evaluated for a started function?** P NEXT:70–76, F:23–29:

> OPEN language-order question sent to Root: Native source start evaluates
> parameter defaults immediately; JS source start waits for its first turn.
> The docs specify arguments now/body later, but not default binding explicitly.
> Do not change JS globally until answered. Repro saved outside corpus at
> /tmp/fa-with-start-default-timing.fa. Corpus tests independent of this choice
> cover actual default values and cancellation after the child has started.

Options: (a) bind/evaluate defaults with explicit arguments at start creation; (b) bind defaults on the task's first turn with the body. **Recommend (a)** for consistency with arguments and deterministic source-order construction, but specify cancellation/throw/side-effect ownership explicitly. Existing native behavior favors it; JS must change intentionally with tests. Keep unrelated with work independent until a timing-sensitive acceptance test is required.

**Q2 — Resource result narrowing that would need to duplicate owned fields.** P:190–197, F:121–128:

> OPEN language questions sent to Root (still errored at this checkpoint):
> (1) start/default timing above; (2) adapting Resource-with-Array returns to
> Plain-with-Array would need a structural copy before destroy can observe its
> fields: restriction vs implicit copy contract; (3) direct throw in do/catch:
> analyzer ignores catch, both backends use it, errors.md says throw exits the
> function like return. Repro /tmp/fa-split-do-catch-repro/main.fa. No guessed
> semantic edit was made. Local methods capturing DATA also need constructor
> provenance/environment transport (Projection::Method like Finalizer), no new
> language prohibition; coordinated with proofs.

Q2 options as recorded: restriction vs implicit copy contract. **Recommend restriction/explicit copy at a visible conversion site**, consistent with unique Resource and explicit collection copy, rather than an invisible copy inside a function adapter. A narrower view can borrow for the duration of a call; owning returned Plain data must be independent before destroy sees the Resource. Give a diagnostic pointing to an explicit constructor/copy projection or API returning the intended Plain value. Do not move fields out from under destroy.

**Q3 — Does a direct throw inside do enter lexical catch?** Same verbatim paragraph above. Options: (a) lexical catch handles it, matching both backends; (b) throw exits the function regardless, matching the cited wording, and backend behavior changes. **Recommend (a)** and clarify errors.md that throw leaves the current successful expression and searches the function's enclosing catch, while uncaught errors leave the function. P already made this change (`errors.rs`, lexical-throws.fa) and tracks failure-state joins, but no approval is recorded; F retains the old behavior. The lead must accept or remove the semantic hunk explicitly, including compile-time behavior, rethrow, moves and mutable state at failure.

**Q4 — Local methods capturing DATA.** Same quoted paragraph: “Local methods capturing DATA also need constructor provenance/environment transport (Projection::Method like Finalizer), no new language prohibition…” Options: carry the lexical environment in written method/finalizer capabilities, or reject methods that use such DATA. **Recommend carry provenance/environment**, as the note directs; no evidence authorizes a new restriction. Add same-layout local/foreign generic method tests, including after the constructor returns and under cancellation. This is implementation work once the existing method semantics is confirmed.

**Q5 — Generic exact mutation with resource sharing.** P:514–518, F:445–449:

> Latest open semantic clarification: user asks what “shared resource” means in the
> generic exact-mutation question. Root explained a type with destroy and copies
> sharing one counted instance/state, as previously selected. No choice of generic
> instantiation restrictions vs conditional effects has been made; do not assume
> an elapsed-time answer. Agents continue concrete cell/loan proofs independently.

Options explicitly recorded: instantiation restrictions or conditional effects. **Recommend reframe the question around final unique Resources and explicit Sync handles first.** The “destroy copies share a counted instance” premise was revoked by the current checkpoint. For still-relevant generic Sync payload mutation, prefer portable conditional effect summaries at specialization (consistent with existing semantic copy conditions) if exactness can be retained; otherwise restrict only the unsupported instantiation with a specific diagnostic. Do not impose a language-wide generic restriction based on the abandoned Resource-cell premise. No answer is inferable from elapsed time.

**Q6 — Portable defaults and anonymous nested default owners.** P:45–49 only (later than F):

> Still to audit: defaults depending on a generic value across files need a
> portable recipe (the existing constant-default interface rejects them as
> runtime defaults). Anonymous nested object annotations also lack a written
> default owner; do not infer their defaults from structural TypeId equality.
> Implicit default/destructor effects must contribute at their evaluation sites.

Options: publish a stable owner/template plus exact generic type/value recipe, or keep cross-file use rejected unless explicitly supplied; inventing a structural-type default owner is a third, unsafe possibility. **Recommend a portable owner recipe for written declarations; keep the current explicit-argument diagnostic where no owner exists.** Any broader anonymous-type default policy needs a language rule; don't select arbitrary same-shaped defaults. Effects at actual default/destructor evaluation sites are required by existing exact-effect semantics, not an optional optimization.

**Q7 — Static callable adaptation under erasure.** P:184–189 / F:115–120:

> OPEN before final full/integration: JS erased F:(UInt32)=>UInt128 needs a
> static call capability when concrete F
> actually accepts UInt128/returns UInt32; concrete numeric adapters alone do not
> cover that generic constraint. Also audit deep structural Function fields.
> Field/index effects/requires and native returns-as-Void are validated above;
> JS adapters still need the production-template consumer and its full run.

Options: generate a static typed call-adapter capability, respecialize the permitted named callback at compile time, or narrow assignability on JS. **Recommend compile-time specialization/static adapter for the narrowed November 15 allowed callback scope**, with the same assignability on all targets. General runtime stored forall adaptation is dropped under that decision. No dynamic schema walker or target-specific silent narrowing.

**Q8 — How to represent exact destructor identity.** W:68–71 and R:168–171 (shared older checkpoint):

> Generic destroy selection/invocation remains separate: scope drops and storage
> drops need the same static owner capabilities, including through composites.
> Preserve the lead's separate storage_drop_function and detached-task behavior.
> No runtime native dictionary is introduced by this lot.

H:53–56 gives the later prescription:

> Next: audit generic destructor lookup and finalizer identity in another snapshot
> from current Root. Lookup by decl.ty==substituted ty fails; finalizers must bind
> WrittenType+owner args/caps/method provenance at creation. No universal TypeInfo
> key exists yet; use WrittenType.semantic_fingerprint and captured capabilities.

Options: bind exact written owner/capabilities when constructing a finalizer, or find a destructor later by equal structural TypeId. **Recommend the first, as the later checkpoint does.** This is settled implementation direction, but not delivered in the six. Don't invent a runtime native dictionary or universal codec/key to fill the gap.

**Q9 — Resource origins through callable types and named-union payloads.** H:125–130:

> Known follow-up coordinated, don't duplicate graphs: analyzer-partial with has
> SignatureNames tag109 following aliases/function types, but not per-parameter
> WrittenType. Enrich this graph after Root integrates with for bare Resource
> literals passed through callable TYPE aliases/parameters. written_parameters111
> covers declared functions/methods. Also audit named-union Resource payload origins
> separately; no repro yet, don't claim support merely from TypeId.

Options: extend portable visible callable metadata with written-parameter owner information, or infer by structural TypeId. **Recommend metadata with exact owner identity.** H's later final delivered note says a named-union generic Resource DATA test was added to the separate delivery; it does not settle callable TYPE alias parameter origins in these six. Use one graph/owner protocol and verify both constructions; no new language prohibition needed.

**Q10 — Inter-worker ownership/protection and handle transfer.** P/F's inherited “Inter-worker resource rule (latest answer)” says:

> The user requires independent copies, a common Mutex, or an exclusive move
> invalidating the source and aliases. Resource handles still share cells locally;
> at a worker boundary they therefore require a common Mutex or exclusive transfer.
> No automatic synchronization of Resource fields. Analyzer must track the cell
> aliases and Guard loans: neither an old alias outside a Mutex nor a Resource
> handle copied from Guard.value may bypass the protection. Task remains affine
> to its executor and never transferable between workers.

The copy/Mutex/exclusive-transfer alternatives and Task affinity are user choices, not open for an engineer to reverse. The local counted-Resource premise is superseded. Implementation options are static proven exclusive transfer + category handle protocol, or rejecting only unproven transfers; automatic Resource synchronization and blanket JS transfer refusal are not authorized. **Recommend final unique owner/loan proofs and binary transport of protected DATA**, with stable Sync/Function references where a supported remote operation exists. Decide handle category semantics explicitly if it requires a new remote-operation contract; local retain is not enough.

### 4.3 Open implementation work, superseded checkpoint questions and evidence limits

For completeness, the other “open” paragraphs in NEXT are quoted here. They are mostly implementation followups or older checkpoints, not additional undecided language choices.

P/F's older with section says:

> OPEN with work: retained defaults (FnSig deliberately has no default flags;
> need a secondary default-aware entry/portable visible contract, preserving
> ordinary first-class Function ABI), true async native wrapper retaining env
> before initial suspension (proofs supplies runtime/RC helpers; current partial
> calls target sync), compiletime factory binding FN110 and emitted constant
> closures, imported/split partial declarations, requires forwarding, callback
> error specializations, explicit Copy/capture/provenance/TypeInfo integration.
> Do not remove co-9..12 pending or claim the whole with feature complete yet.

Newer P/F current checkpoints implement retained defaults, native async wrappers, split/requires handling and factory semantic instances. They do not finish production JS constant emission/AOT/unique integration; the November 15 generic scope drops some older use cases. Preserve a separate declaration-default entry (recommendation); do not add default flags to structural FnSig identity. Earlier notes also say:

> Index-in-effect bindings currently conservatively All;
> precise constant indices can be retained later.

Conservative All is a safe implementation fallback, not a new language choice. Prefer precision as a focused followup with effect coverage tests, keeping existing diagnostics sound.

P:199–202 and F:130–133:

> Pending after these: constant-composition factories/finite-depth reifier,
> union written-origin joins, deferred generic keyloop effect signatures,
> documented rules/Schema/native nominal cases, destruction effect summaries,
> and measured analyzer pass fusion. No final fullLLVM/commit yet; coalesce
> Root AOT/TypeInfo/Resource and proofs wire/Function templates before full.

Options are completing each contract or retaining a specific supported diagnostic; claiming completion from the broad snapshot is not an option. Recommend the focused rows 27,33–35 and separate Schema/rule sessions. Keep a stack-safe budgeted reifier rather than introducing an arbitrary new language recursion cap. Stackless interpreter continuations themselves are already integrated at compiler; old R/W ownership assignments are stale, not unfinished interpreter runtime work.

R:43–46:

> Remaining provenance holes: builtin .push/.set storage, loop variables and
> builtin-producing calls; capture replacement by equal opaque origin can
> conflate distinct captures. Worker/Susbcription/Guard transport protocol needs
> conservatism; do not equate local shared copying with Send.

Recommend exact flow source identity and storage/producer hooks (rows7/9/32), conservative explicit refusal only where an actual required proof is absent. This does not authorize treating all shared handles as Send or blanket JS rejection.

R/W's key-loop checkpoint (W:72–75, R:172–175):

> The generic expression-body key-loop inference hole remains assigned here:
> `/tmp/fa-key-error-expression.fa`, mirrored by proofs' pending
> generic-key-loop-inferred-errors.fa. Concrete deferred loops currently throw
> away inferred summaries; exact checks/callers must run after specialization.

Recommend recomputing/publishing exact specialized summaries and invalidating callers; don't relax exact throws or leave a passing body with a stale signature. Existing pending fixture is the acceptance seed. Their remaining-work lists name with/factories/TypeFromSchema/origin joins/pass fusion and duplicate the items above; no alternative language arbitration is recorded in them.

H's conditional borrow checkpoint explicitly described a failing native case:

> Root fixture unique-resource-temporaries.foreign.fa passes Bun, fails3LLVM because if/block/region/fallback currently return owned/fresh even for borrowed Resource. Fix conditional ownership/cleanup across if/when/catch/fallback/blocks, preserve consumed Task ABI.

Later H notes say the separate unique-borrows patch passed LLVM lib7/7 and all Resource programs on native/free/wasm/leaks; clippy had one inherited counter-loop warning. The generic-destructors separate delivery reports analyzer399, pipeline6, resource_origins1, JS lib41/41, four Bun programs, resource suites all three LLVM targets, package Bun/Node equality, and inherited clippy warnings. **These counts do not validate `3c8cc4d0` or show those files in it.** Recommend preserve lexical lifetime and path ownership in the current trunk implementation; never extend a local Guard lifetime to make a bad borrow escape. The named-union origin warning is handled in Q9.

H's production transport open point is explicit:

> Production Mutex/Channel/Worker binary hookup is STILL OPEN; existing JSON exchanges must ultimately disappear.

Recommend rows30–32; no design decision remains permitting JSON or dynamic runtime Fa schemas. Its “unbound forall wire value … unsupported AN001 until it has a static adapter” is superseded in forbidden value positions by the November 15 GN030 rule; permitted static callbacks still need capability forwarding.

G NEXT contains no new explicit language question; its unfinished work is generic-pointer completion and eventual full validation. Later P/F notes establish the descriptor-name/owner-order failures. The source includes candidate fixes, so **failure status after those last edits is unknown**, not proven still failing or proven green. R's latest unique-borrow validation note also has an “in progress” line superseded by its later delivered green paragraph; do not count the older line as a known current failure.

Inherited native NEXT questions about frame strings, physical regions, 4 KiB experimental chunks and beating Goose are stale relative to compiler/perf: compiler includes later 1 KiB compact-region work and perf November 15 fixes the stack collection/GrowingArray layout rules and reports residual code-generation gates separately. Do not reopen those old experiments to land shared handles. Generic pointer capabilities still need current layout metadata; they do not determine native GrowingArray layout.

Limits of this assessment:

1. I did not run any code, build, tests, format checks, benchmarks, leak checks or patch application. There is **no newly established passing test count**. Static merge-tree conflicts establish only merge mechanics.
2. NEXT is a chronological working log with copied stale sections and claims about separate `/tmp` sources. I read the six tracked snapshots and archive refs, not all external patch files, old log files, the separate Codec repository, root coalescing worktree or every former engineer conversation. Missing approval/validation was not inferred from silence or time.
3. No complete enumeration of future merged-trunk Copy diagnostics, latent races, ABI failures or missing diagnostic behaviors is possible from reading. Counts in §4.1 are confirmed changed fixtures only; positive snapshots are not total proof of safety.
4. The merge in progress can alter conflict counts/paths and implementations. Re-run the same pinned comparisons against the actual merged HEAD before starting a slice; do not rely on these inspection tree hashes as deliverables.
5. Exact individual authorship and exact time of the final Resource/Set/no-JSON answers are only attributed by checkpoint headings/text. They are not independently recoverable conversations in these sources. Record the final lead arbitration in decisions.md when landing.

Report output only: `/home/claude-dev/fa-lead/reports/wip-assessment.md`. Assessment branch/HEAD remain `audit/wip-assessment` / `9cbb7558`; no source commit was made.
