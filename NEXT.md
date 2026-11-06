# Compiler completion in progress (2026-11-06)

Integration: branch `compiler`, worktree `.claude/worktrees/lead`. The main checkout
is shared and dirty: preserve it. The user requested completion of all remaining
compiler work and explicitly authorized parallel agents.

## Written methods (2026-11-06, active in analyzer-with)

- Branch `codex/analyzer-with` is now based on validated `8c92474`, with
  uncommitted method-origin work only. Do not use its target from another worktree.
- `WrittenType` separates the named owner, its type arguments, their nested
  written origins (shared by Arc), and required methods from structural TypeId.
  `GenericMethod` carries parameter position/name, function DeclKey, owner args
  and nested method selections. Bindings keep these until declaration_order.
- Analyzer publishes sparse origins of values, returns and fields; generic
  projections preserve Holder<Person> vs Holder<Greeting>. Imported interfaces
  carry/remap/hash these facts. `function_owners` keeps type owner binders
  separate from the method's own binders; `method_instantiations` records the
  written receiver application. Selected methods contribute declaration uses.
- JS passes selected methods instead of choosing the first equal TypeId.
  Signature-only forall-value dispatch was shared by the lead in
  `/tmp/fa-generic-value-signature-abi.patch` and applied. Type-owner hidden
  parameters, including phantom values, are passed/bound at method calls.
  copy_function no longer selects an empty alias's field order and drops data.
- Native instances/requests/function-value tables include method selections;
  nested selections and exact type bindings survive closures and callbacks.
  Cache version is split-6 (reconcile with lead/platform/proofs on merge).
  The identical metadata engineer patch `/tmp/fa-generic-concrete-fields.patch`
  is applied here; do not duplicate it on integration. Mixed local/global
  shape copies compare field names textually. Chained `x.value.describe`
  dispatches from the prefix type, not just the root.
- Bun and Node E2E and all three LLVM targets pass for explicit aliases,
  imports, returns, exported values, forall values, Holder/Speaker and phantom
  owners. The multifile fixture requires per-unit compilation: fallback to a
  merged module is a failure. Logs: `/tmp/fa-method-origin-container-*`.
- Built-in containers retain nested written origins (annotations, homogeneous
  array literals, tuples, indices, loops, values/get/filter/reverse). Distinct
  origins at a join remain a language question owned by the lead; do not infer
  an arbitrary branch's methods from equal fields.
- Nominal objects now retain/substitute/infer phantom arguments, with HAS_PARAM
  through wrappers. Their phantom values do not confer ownership flags. CStruct
  substitution preserves field order; private access uses the declaration key
  across instances. Unit regression for phantom objects and incremental alias
  edits pass. Incomplete written applications cannot form invalid Bindings.
- Async selected methods retain their sync/async pair and bind owner capabilities
  in both ABIs. This new case, the complete analyzer, the historical structural
  exclusions (removed), and workspace clippy are being retested after the ff to
  `8c92474` in `/tmp/fa-method-origin-integrated-*`.
- Before committing: merge the validated metadata branch, remove its now-covered
  generic-hidden-arguments pending, and run strict full workspace LLVM, clippy,
  JS runtime. Then freeze this lot and continue in a separate worktree.
- Generic destroy selection/invocation remains separate: scope drops and storage
  drops need the same static owner capabilities, including through composites.
  Preserve the lead's separate storage_drop_function and detached-task behavior.
  No runtime native dictionary is introduced by this lot.
- The generic expression-body key-loop inference hole remains assigned here:
  `/tmp/fa-key-error-expression.fa`, mirrored by proofs' pending
  generic-key-loop-inferred-errors.fa. Concrete deferred loops currently throw
  away inferred summaries; exact checks/callers must run after specialization.
- Afterwards: with/factories/remaining documented rules and TypeFromSchema,
  analyzer pass fusion with 1000-file measurements, and stackless interpreter
  continuations for finding 42. Do not document a new metaprogram recursion cap.

## Integrated

All three original paused branches passed their complete LLVM workspace suites,
clippy and JS runtime tests before integration: initial proofs (`f2f43d6`), native
representations (`ca9f85a`), analyzer fixes (`91b749e`). Private package index proofs
and per-file proof caches followed in `e436568`. Validated recursive return-path
and empty-arena-leaf optimizations followed in `a39f3c4` (44 merged LLVM IR tests
and merged workspace clippy also pass). Binary-trees uses112.71M instructions vs
Goose128.15M; its CPU win is still open.

Twelve clean integrated worktrees have been removed, branches preserved. Active,
dirty, locked or unmerged trees are preserved; some locks belong to another live
session. Records: `/tmp/fa-resume-cleaned-worktrees.json`.

## Lead changes validated

The delivered documentation rewrite is merged with the later compiler notes.
Stage2 rejects old generics (GN031), keeping recovery only for the migration tool.
Fields is/is-not and compile-time evaluation, Color.members.length (also nested),
and known-layout Bun pointers (C memory, opaque Fa values, copies, drops, writable
C views) are implemented. Common fixtures run on Bun/native/freestanding/wasm.

The first combined full suite found two newly merged proof fixtures still using
old generics, one memory-test generator using Array(Integer), the obsolete
formatter transition test, and a Bun async proof fixture depending on timer order.
All are corrected; the failed test targets pass again. The async fixture now
synchronizes reader/writer phases and also passed under QEMU. No failed program
was skipped. Clippy,298 JS runtime tests and website check/snippets/build passed.
The combined complete workspace suite, including LLVM and the traversal merge,
passes (exit0): `/tmp/fa-lead-combined-workspace-full.log`. Final clippy,298 JS
runtime tests and website check/snippets/build pass too.

## Active ownership

- Lead platform support is integrated at `4a6ef8e`: Linux AArch64 runtime and
  freestanding, wasm C libraries with importable factory/dispose, wasm tests,
  timeouts and watch target retention. Complete suite with LLVM, clippy, website
  and merge checks pass. The clean worktree was removed. Regenerate all three
  runtime IR files after performance C changes.
- Lead, `generic-pointers`: generic Pointer<T> on Bun using published runtime type
  capabilities; direct/higher-order calls, composites, resources and task destruction
  pass on Bun. Generic native ownership now uses concrete resource types; all
  pointer programs pass on all three targets with leak checks. No full validation yet. Signature-only generic value ABI patch shared with analyzer-with.
- Analyzer, `analyzer-callbacks`, `d22ebd2`: native instance per callback error set
  and exact effects, including exact variants. Complete LLVM suite passes (exit0)
  in `/tmp/fa-exact-effects-workspace-fixed.log`, with clippy, formatting, runtime
  and website checks green. Merge checks pass too: workspace clippy, callback programs on Bun and all three
  native targets, plus all five C-library tests. Its old initial worktree was removed. `analyzer-with` separately owns with, missing documented
  rules, TypeFromSchema/named Schema, interpreter continuation stack, method origin
  and native method-specific instance keys, key-loop thrown summaries, pass fusion.
- Proofs, `cross-file-metadata`: imported Pick/Omit derived parameters, EF004,
  cross-file @for, generic hidden ABI and cache hashes. Fixing contract-field indices
  against concrete wider layouts before its final full. Its old integrated tree
  has been removed. Method-origin semantics remain assigned to analyzer-with.
- Native performance, `agent-a375af73b4e738f36`: compact map slots, append reserve
  paths and newly found wasm size-conversion overflow holes. Next: stack strings
 24–64bytes, recursive collections/sexp packing, remaining Goose gaps. No untested
  optimization is treated as integrated; program semantics remain unchanged.

## User effect decisions

An explicitly throws-typed callback can be caught normally. Its receiver declares
only errors that really escape, including manual rethrows. Additional errors of a
callback accepted by a non-stored parameter without throws remain transparent.
Overdeclared throws and mutates are errors everywhere. Throws is exact by variant:
only E.Bad can escape ⇒ throws E.Bad, not E={Bad,Other}. Function-type constraints
retain fewer-effects assignability; foreign declarations have no body to check.
Existing mutation-path coverage remains. Documentation records these decisions.

## Validation discipline

LLVM_SYS_181_PREFIX=/home/claude-dev/.local/llvm18/usr/lib/llvm-18, its bin and Bun
on PATH. Cargo jobs1–2, tests2 threads, ulimit-v8000000 for full suites. Dedicated
ASan runs outside the virtual-memory cap. Full suites serialize with
`flock --close /tmp/fa-compiler-full-suite.lock ...`; never pass its FD to children.
Targets are never shared between preparation and frozen validation worktrees.

Root stashes, including cba6f6a030c0cb2d1dc7105c74574fca83600a34, are already
applied: do not reapply them. The documentation rewrite backup is
`/tmp/fa-resume-rewritten-documentation.patch`. Git auto-gc cannot update the
shared perf/native-goose ref; use `git -c gc.auto=0 commit`, preserve permissions.

After integration, audit `design/todo.md` against actual regressions and completed
features, apply the remaining metaprogramming note, measure the analyzer on the
1,000-file package and all Goose programs, then run the final combined checks.
Do not remove unresolved items or mark the overall goal complete early.
