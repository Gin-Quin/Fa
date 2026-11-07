# Compiler completion in progress (2026-11-06)

Integration: branch `compiler`, worktree `.claude/worktrees/lead`. The main checkout
is shared and dirty: preserve it. The user requested completion of all remaining
compiler work and explicitly authorized parallel agents.

## Integrated

All three original paused branches passed their complete LLVM workspace suites,
clippy and JS runtime tests before integration: initial proofs (`f2f43d6`), native
representations (`ca9f85a`), analyzer fixes (`91b749e`). Private package index proofs
and per-file proof caches followed in `e436568`. Validated recursive return-path
and empty-arena-leaf optimizations followed in `a39f3c4` (44 merged LLVM IR tests
and merged workspace clippy also pass). Binary-trees uses112.71M instructions vs
Goose128.15M; its CPU win is still open.

Sixteen clean integrated worktrees have been removed, branches preserved. Active,
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
  pointer programs pass on all three targets with leak checks. The full `/tmp/fa-generic-workspace-full.log` is now running after clean clippy
  and targeted checks. Signature-only generic ABI patch shared with analyzer-with.
- Analyzer, `analyzer-callbacks`, `d22ebd2`: native instance per callback error set
  and exact effects, including exact variants. Complete LLVM suite passes (exit0)
  in `/tmp/fa-exact-effects-workspace-fixed.log`, with clippy, formatting, runtime
  and website checks green. Merge checks pass too: workspace clippy, callback programs on Bun and all three
  native targets, plus all five C-library tests. Its old initial worktree was removed. `analyzer-with` separately owns with, missing documented
  rules, TypeFromSchema/named Schema, interpreter continuation stack, method origin
  and native method-specific instance keys, key-loop thrown summaries, pass fusion.
- Proofs, `cross-file-metadata`, `0d80b62`: complete LLVM workspace suite passes
  (exit0, `/tmp/fa-metadata-workspace-full.log`), with clippy and JS runtime green.
  Imported Pick/Omit, EF004, cross-file key loops, hidden generic ABI, concrete
  native field indices and JS generic copies are integrated. Merge checks pass:
  analyzer units/cases, pipeline (after migrating its new Integer<> source string),
  complete JS crate, generic programs on all three LLVM targets and all31 package
  tests. Runtime298 and website checks pass too. The new notes are
  applied to the rewritten documentation. Method origins remain analyzer-owned.
- Native performance, `c80d6d9`: compact 32-bit map slots, short append loops,
  reservation overflow guards and memoized recursive JSON writers passed the full
  LLVM workspace suite before integration. Merged checks pass:47 IR tests,
  object-cache invalidation, C libraries, ABI tests and reservation/map/append/JSON
  programs on native, freestanding and wasm, with representative AArch64 QEMU
  runs. C ASan/UBSan, JS runtime298, clippy and website checks pass. Next: stack strings
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

Root worker/Errors WIP is saved explicitly in stash c9a0034315b5c4bdb7f3da0ecd9e02516e1f1b05 while metadata merges. Do not lose or reapply it twice. Generic-pointers sources remain frozen in their own worktree for the queued full.

## Latest user decisions

Task and composites holding a Task remain noncopyable. Collections and ordinary objects
containing them require an explicit copy when both sources remain usable; an implicit
last-use move remains allowed. Lead/analyzer own that parser and semantic migration.
Resource copying and finalization are being clarified by lead; this physical lot makes
no change to Resource or Task ownership.
`with` uses parameter names from the visible function signature, including a
function-valued parameter or field. Analyzer owns this implementation.

Joining written types with distinct methods/destructors forms their union, even
when their fields have the same shape: runtime dispatch uses the actual member.
Analyzer owns this after the frozen single-origin validation.

## Native physical regions: validated delivery, 2026-11-07

This worktree is the frozen delivery of the physical root-region implementation, based
on 0290400. It includes the already delivered Frame changes (daa8ebc) and the frontend
ROOT_REGIONS/capacity APIs from proofs 1a0d6b/a9b1375. Lead merges these existing changes
once and preserves newer Resource/MOVE code. Task remains move-only; there are no Task
reference-count changes.

Implemented: child-array frame packets and buffers owned by a root region; compact String
and Array payloads; whole-module marked 16-bit ids and 32-bit union-array links; conservative
ABI/split/layout fallbacks; separately logged capacity-unobserved consumption; small decimal
pairs. A separate prototype remains at /tmp/fa-native-frame for the next measured changes.

The 1 KiB chunk is retained after 21 alternating pinned-CPU samples at default sizes. On
sexp N=20000, medians are 250.1 ms CPU / 57600 KiB RSS for 1 KiB, 306.7 ms / 101376 KiB for
4 KiB, and 175.0 ms / 26752 KiB for Goose built with GCC 13 (`/usr/bin/cc`, -O2),
not the LLVM/Clang 18 used for the historical instruction ratios. On binary-trees 17,
1 KiB costs 124.2 ms versus 120.3 ms for 4 KiB; that GCC-built Goose is 99.2 ms.
Preparation overlapped part of the sampling window;
retain CPU/instructions separately and repeat the final compiler comparison when quiet.
The original 16 Goose benchmarks remain the priority; no overall win is claimed.

Validation is complete: full workspace with LLVM exit 0
(/tmp/fa-region-frozen-workspace-full.log), fmt/clippy clean, JS runtime 298/0. All 54 IR
checks, 6 cache tests (including physical capacity invalidation), programs on native,
freestanding and wasm, C ASan/UBSan and representative AArch64 QEMU runs pass. The next lot
also tests pure split/nonempty-filter fusion and ids disjoint from compact markers after
late type interning. Neither later change is silently included in this tested snapshot.

## Follow-up: fused split filters and disjoint compact ids

Frozen on top of physical regions 100f4b9. The user has removed Set key callbacks:
Set uses standard value hashing/equality, and Map serves separate keys. No Set callback
prototype is included. Pure immediate callbacks of the
form `part => not part.isEmpty()` can run while split produces its parts; other callbacks
keep ordinary filtering. Raw split capacity and argument evaluation order are preserved,
including wasm, multi-byte separators, Unicode and returned borrowed views.

Ordinary union/type ids always skip the compact marker bit, with checked u64 arithmetic,
and Root member ids are reserved during preparation. This keeps late type interning
disjoint from the packed representation without changing errors or split-module ids.

Array assignment/copy check the compact Character form only at the String element width.
The other widths retain ordinary buffers, letting LLVM remove representation branches.
No String key-map layout or malloc-annotation experiment is retained: neither improved
the measured corpus.

Measurements at audit sizes: words 30.44M→28.53M, strlist 89.51M→86.70M instructions,
fannkuch 137.76M→136.67M. sexp 81.40M / Goose 77.53M, still 1.050x. These are historical
Clang 18 instruction measurements. The report previously labeled original16 had only
11 of the original Goose programs plus 5 secondary ports: blur, graph, push, sum and tree
were missing. It does not establish coverage of the original suite. A new comparison
must use exactly binary-trees/blur/calc/graph/interp/lru/particles/push/records/respond/
scene/sexp/strlist/sum/tree/words, Clang 18 on both sides and the default sizes, with the
9 secondary ports separate. A different fresh mixed corpus used GCC 13 and must stay
labeled as such. The legacy --reuse size/manifest bug is being corrected before any new
comparison. CPU medians remain a separate final validation task.

Targeted 55 IR checks + id-boundary unit, split/region/Character programs across three
targets, QEMU and C ASan/UBSan pass. Valgrind region/split examples report zero errors
and all heap blocks freed. The complete LLVM workspace passes (exit 0,
/tmp/fa-region-small-frozen-workspace-full.log), fmt/clippy pass and JS runtime is 298/0.
Parent has a separate unapplied NodeCopy buffer-reuse hunk at
/tmp/fa-copy-array-reuse.patch; it is not included here because this base predates Copy.
