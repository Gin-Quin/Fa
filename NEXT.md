# Native String length soundness checkpoint (2026-11-07)

Based on validated `6401446`; independent of KEY32/String16 prototypes. The
nonnegative-length LLVM assumption now follows inline decoding. In the inline
representation the raw second word contains UTF-8 bytes, which can set its sign
bit; only `select(inline, tagLength, rawLength)` is a valid nonnegative length.
The object cache contract advances with `string-length-1` so old assumptions are
not retained in cached objects. The three-word String ABI is unchanged.

The IR regression checks that the assumed operand is decoded first. The new
`inline-utf8-length.fa` fixture covers both 11-byte and 23-byte inline layouts,
concatenation, equality, grapheme length, case conversion and split/join. Its
baseline640 execution succeeds; this fixes an invalid IR contract, without a
claim of an observed baseline output mismatch.

Validated: full workspace with LLVM EXIT0 (`/tmp/fa-inline-utf8-length-workspace-full.log`),
workspace clippy/fmt, 300 JS runtime tests, targeted native/freestanding/wasm and
AArch64 QEMU execution. No runtime C changes or representation changes. The
performance prototypes remain separate, unretained; their CPU confidence intervals
include parity and they are not part of this checkpoint.

# Compiler completion in progress (2026-11-07)

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

Seventeen clean integrated worktrees have been removed, branches preserved. Active,
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
  and targeted checks. Signature-only generic ABI patch shared with analyzer-with. The full found
  invalid synthesized descriptor parameter names and owner-capability argument
  order failures; these must be corrected against metadata/origins, alongside the
  new universal-copy lifetime rules, before integration.
- Analyzer, `analyzer-callbacks`, `d22ebd2`: native instance per callback error set
  and exact effects, including exact variants. Complete LLVM suite passes (exit0)
  in `/tmp/fa-exact-effects-workspace-fixed.log`, with clippy, formatting, runtime
  and website checks green. Merge checks pass too: workspace clippy, callback programs on Bun and all three
  native targets, plus all five C-library tests. Its old initial worktree was removed. `analyzer-with` separately owns with, missing documented
  rules, TypeFromSchema/named Schema, method origin
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

Root worker/Errors WIP from stash c9a0034315b5c4bdb7f3da0ecd9e02516e1f1b05 is now restored on lead; do not reapply it. Its stable error identities and Error.toString additions await final validation. Generic-pointers full found hidden generic ABI/name failures; fix them against the integrated metadata/origin work before delivery.

## Latest user decisions (supersede earlier copy notes)

- Task is NONCOPYABLE, including composites holding one. The short-lived decision
  to make Task copyable was revoked; no Task RC implementation is to be delivered.
- Implicit moves remain allowed when the source (including aliases) is not used
  afterwards. A structural copy of a collection, or a plain object owning one,
  requires `copy value`. Syntax is a spaced contextual prefix; names `copy` remain.
- Every type declaring `destroy` is NONCOPYABLE. Resource parameters are borrowed,
  including resources containing a Task. A second resource requires a new instance
  (`open`, `create`, constructor). General Resource reference counting is revoked.
  Explicit Mutex/Channel/atomic handles retain their sharing semantics.
- Set uses standard value hashing and equality, without a key callback. Use Map
  for a separate key. The root frontend, backends and codecs implement this rule.
- `with` uses the names of the visible signature, also on parameters and fields.
- Joining written Person/Greeting alternatives with distinct methods/destructors
  gives Person | Greeting, preserving runtime dispatch to the actual member.
- Escaping captures are copies and immutable by the existing docs; Task captures
  cannot acquire an invented FnOnce semantics. Analyzer and proofs coordinate
  callable provenance across annotations, aliases, imports and generic values.

JS transport decision RESOLVED: the user requires no JSON transport; use
structuredClone and/or Codec, and explicitly authorized fetching and modifying
Codec locally. Implement the inter-realm resource protocol, preserving shared
mutable fields and global ownership. No blanket JS transfer rejection.

Codec is cloned at /home/claude-dev/projects/codec, branch codex/fa-worker-codecs,
commits f5d5e48 + 37006b0 (upstream997650b). Shared byte-window encoding, named
references, named value transforms, arbitrary-key Map, nested host discovery and
CSP-compatible dynamic codecs are implemented.129 tests/690 assertions, tsc and
Biome pass, including a real worker, GSAB growth, aliases, -0 and BigInt1000bits.
The user then required BUILD-TIME generation from static Fa types, including
copy functions. The old dynamic facade is superseded for Fa: minimal AOT runtime
now exposes binary primitives + fromFunctions(write,read), with neither runtime
function compilation nor schema interpretation.129 tests/tsc/Biome pass after
this change; /tmp/fa-codec-aot-runtime.js is the new artifact. Root must generate
typed write/read/copy/drop functions in the Rust JS backend and carry them in
TypeInfo. Generic factories emit precompiled operations parameterized by concrete
type descriptors. Proofs accepts a compiled codec in its Resource registry.

Generic runtime metadata now uses universal TypeInfo(ParamId), tag4, with at
least ownsTask and semantic key; Bun layout joins the same descriptor. Do not
apply the obsolete OwnsTask-only patch. /tmp/fa-type-info-metadata.patch is
validated; root has applied it to /tmp/fa-shared-values. No content scan for
Task ownership: empty Array<Task> and Union<Task|Array<Integer>> must transfer
according to the static type. Origins/method choices are part of semantic keys.

## Live worktrees after these decisions

- Lead0290400 includes c80d6d9 maps/append guards/JSON writer memo, all3 runtime IR
  regenerated and merged checks/QEMU/ASan/runtime/site green. Worker error identity
  and Error.toString WIP restored from c9a0034, targeted tests and JS crate green;
  first full failed only the builtins unit count (a method is not an Error type).
  Unit fixed; corrected complete LLVM workspace full EXIT0, session49643, in
  /tmp/fa-worker-display-workspace-fixed-full.log. Analyzer/clippy/style/site and
  JS runtime300 pass. Ready to commit/integrate the other green lots.
- /tmp/fa-shared-values, codex/shared-values, base0290400, owns Node::Copy parser,
  formatter, syntax adapter, JS/LLVM copy, resource cells and generic ownership.
  Full parser and LSP Copy contextual tests pass. New explicit-copy.fa and
  shared-resources.foreign.fa pass Bun and LLVM native/free/wasm. Runtime306 pass
  after unique Task outcome cleanup (NO Task retain/copy). Source remains WIP:
  generic Task parameter conditional cleanup, default owning parameters, generic
  destructor origin/capability keys and merges with handles/analyzer remain.
- analyzer-with single-origin lot ecda9e5 is READY: corrected complete LLVM
  workspace full/clippy/runtime298/fmt all pass. Root already ported this snapshot
  in experimental shared-values; integrate compiler independently. analyzer-rules separately
  owns Copy requirements (MEM003 Task, MEM004 implicit collection), callable
  provenance, Resource alias effects/facts, with, remaining rules and pass fusion.
- proofs interpreter-continuations2ec0e57 is READY: complete LLVM workspace full
  exit0, final fmt/clippy pass; integrate after lead source unfreezes. Log
  /tmp/fa-continuations-workspace-full.log. Heap continuations preserve depth10000,
  exact steps/heap accounting and run non-tail/callback chains depth9000 on256KiB.
- proofs arena-regions1a0d6bf is READY: complete LLVM workspace full/clippy/fmt
  pass, /tmp/fa-arena-workspace-corrected-full.log. A private Root used only in
  an imported function passes leak-check. Keep the worktree: agent adds an
  independent arena-capacity fact on a new branch, no change to delivered HEAD.
  shared-handles on0290400 owns Guard/Worker/Pool/Socket/Listener/Subscription,
  closures and Task interthread affinity, coordinating provenance with analyzer.
- /tmp/fa-native-next, worktree-native-perf-next: Frame strings frozen/full queued.
  /tmp/fa-native-frame, worktree-native-regions: child arrays/regions/sexp prototype.
  4K regions are only experimental (small instruction regressions elsewhere),
  no CPU victory claimed before isolated measurements across original16 programs.

Seventeen clean integrated worktrees removed. Old c80d6d9 worktree was removed only
with agent consent; Goose home copied to /tmp/fa-native-perf-next/goose-home and39
text reports archived under /tmp/fa-native-perf-next/archived-c80d6d9. Branches and
other live-session locked/dirty trees remain. Cleanup JSON is authoritative.

## Inter-worker resource rule (latest answer)

The user requires independent copies, a common Mutex, or an exclusive move
invalidating the source and aliases. Ordinary Resource values are now unique,
not automatically shared. A handle borrowed from Guard.value cannot escape the
protection by copying it. Task remains affine to its executor and never
transferable between workers. Codec transport is still being implemented.

## Validated combined delivery (2026-11-07)

The root worker/Error lot is committed as126f483 after complete LLVM full0,
style/clippy/runtime/site checks. The following three delivered lots are now
combined on lead, preserving its NEXT and current runtime/performance work:

- ecda9e5: written method origins, owner/forall ABI and method-specific native keys;
-1a0d6bf: closed ROOT_REGIONS proof/manifest, common whole-unit facts and cache reads;
-2ec0e57: heap continuations, exact evaluation budgets and stable call depth10000.

Combined workspace clippy, analyzer/interpreter/JS crates,47 LLVM IR checks and
COMPLETE workspace LLVM full all exit0. Logs /tmp/fa-three-deliveries-{clippy,
frontends,llvm-checks,workspace-full}.log. Cache version combines origins,
platform/effects/errors/regions as split7. Metaprogramming implementation note
finding42 is applied and removed from pending-documentation.

Frame64byte strings daa8ebc and capacity-unobserved ROOT_REGIONS fact a9b1375
are now combined here. Regenerated all3 runtime IR from merged C. Workspace
clippy,14 region tests,49 LLVM IR checks and COMPLETE LLVM workspace full all
exit0, /tmp/fa-frame-capacity-merged-{clippy,regions,checks,workspace-full}.log
(full session43720 consumed). Runtime text enters every object-cache options
key, so changed Frame IR invalidates older objects. Keep worktrees while agents
reuse them or until subsequent deltas are safe.

Physical region layouts100f4b9 and the small string/array follow-up dbcee1d are
now combined on lead, including mixed-sign interval index proofs. All three
runtime IR files were regenerated. Workspace clippy,55 LLVM IR checks,6 cache
tests and COMPLETE LLVM workspace full exit0:
/tmp/fa-physical-small-merged-{clippy,checks,workspace-full}.log. The full session
56850 is consumed. No source changes followed that passing full run.

Physical layouts include root-owned child buffers, compact String/Array payloads,
16-bit marked ids and32-bit child links with conservative ABI fallbacks. The
follow-up fuses pure split/nonempty filters and keeps late type ids disjoint from
compact markers. Cache suffix combines split7/platform/effects/errors/regions/
packed-capacity/frame. Runtime contents also enter every object-cache key.

Metrology correction: earlier original16 reports mixed11 original and5 secondary
programs. The authoritative set is binary-trees/blur/calc/graph/interp/lru/particles/
push/records/respond/scene/sexp/strlist/sum/tree/words. The new default-size matched
Clang18 baseline is under /tmp/fa-native-perf-next/default-clang18-original16.
Historical GCC13 CPU runs stay separately labeled. wait4 RSS inherited the Python
fork image floor; use the separate GNU time measurements instead. No overall
performance win is claimed. The frozen metrology lot is independently validating.

Root shared-values has experimental origins+Copy+callable checkpoints plus
TypeInfo tag4, Task/default guards and native entry ownership. Its new
owned-task-parameters program passes Bun and3LLVM/leak-check, including failed
earlier default releasing a later provided Task. Shared/default resources pass
Bun and3LLVM. JS runtime308 and entire JS crate pass after reviewed goldens.
Those changes are NOT a frozen/complete compiler integration candidate yet.

Latest semantic clarification RESOLVES Resource sharing: types declaring destroy
are noncopyable, and documentation must direct users to create new instances.
Root is replacing its unintegrated generalized RC prototype with unique owners.
Analyzer delta /tmp/fa-resource-unique-frontend.patch is validated independently;
TypeInfo.isUnique is separate from ownsTask (Task ABI stops at Resource/Sync).
Generic copies reject unique types. Set.key is removed by the separate standard
hash/equality decision. These rules supersede the older experimental notes above.

Root's latest AOT work: Codec clean577e1e0, facade binary primitives/fromFunctions
only; reproducible pinned runtime in shared-values. Copy/drop/is functions are
generated per checked type with worklists, including erased generics. External
24000node chain passes Bun/Node without stack recursion; loop builder MOVE fixed
by analyzer and CLI24k now passes too. No runtime eval/schema interpretation.
Generated codecs and closure/Sync ownership checkpoints are being coalesced.
Do not claim the root copy/transport lot fully validated or ready to integrate.

## Sync payload merge correction (validated on lead)

The parallel TypeStore merger now remaps the payload of every Sync kind. A
generic Mutex<Array<T>?> previously kept a local type id after publication,
misreading it as Integer in the merged store. Exhaustive leaf matching prevents
new composite kinds from silently taking that fallback. The regression covers
the actual pipeline at1/2 threads and multiple seeds, plus the direct merger.
Cache suffix sync-payload-1 invalidates objects whose earlier stable hash was
already correct despite the malformed merged payload. Workspace clippy and
COMPLETE LLVM workspace full pass, exit0:
/tmp/fa-sync-payload-lead-{clippy,workspace-full}.log (session80560 consumed).

Resource-unique frontend393 units and all cases pass independently. Root has
unique C/JS resource owners, updated docs and passing Bun/3LLVM basic lifetime
programs; mixed borrowed/fresh native control results still need conditional
cleanup (proofs owns that repair). Generic Bun pointers now pass5 programs using
TypeInfo plus generated copy/drop and C-layout metadata. NeedsCodec tag5/hash110
is coalesced in root; binary worker transport remains unconnected. Shared-value
root remains WIP, with no Task RC or automatic Resource sharing.

Three more clean integrated worktrees were removed with agent confirmation:
fa-native-region-validation, interpreter-continuations and arena-regions. Their
branches and logs remain; /tmp/fa-resume-cleaned-worktrees.json is updated.

## Reproducible Goose metrology (merged validation complete)

The audit runner defaults to the exact16 original programs, keeping9 secondary
ports separate. Every executable has a checked manifest (sources, size, flags,
modes, exact output, compiler paths/versions/hashes and binary hash); stale,
tampered or unmanifested reuse is refused. The causal detached-task handshake
is ec5c93b. The remaining benchmark-only delta448e7b8 is coalesced here without
replacing NEXT. Python16/16, workspace clippy and COMPLETE LLVM workspace full
all exit0: /tmp/fa-metrology-lead-{python,clippy,workspace-full}.log, session54936
consumed. The previous full's temporary30s suspension produced no failure.

Baseline dbcee1d and exact Clang18 executables are archived in
/tmp/fa-native-perf-next/compilers; reports are default-clang18-original16,
initial-arrays-cpu.json and initial-arrays-memory.json. The latter uses GNU time,
not inherited wait4 RSS. Baseline interp10496/Goose5504KiB, sexp57856/27136,
scene7552/6656. CPU gaps remain largest on interp2.08x, scene1.50x, sexp1.39x,
respond1.17x, records1.14x and lru1.12x. A general empty-array private-buffer
prototype was rejected for regressions elsewhere. No overall win is claimed.
Direct Root-to-Root compact fields remain a separate prototype; its interp
instruction count regresses5.2 percent, so CPU/memory still require measurement.

## Region String views (isolated soundness repair)

Frozen String payloads now retain the borrowed-view kind when reconstructed as
ordinary Strings. Previously their capacity word looked like an immortal literal,
so storing a copy could keep a pointer into an already freed root. The O0 regression
extracts a String from an inner scope, frees that owner, churns the same arena with
different bytes, then checks the escaped text and an independently extended copy.
Native, freestanding and wasm all pass. The external-payload path receives the same
view kind. No String ABI or C runtime change is needed; the object-cache suffix
region-string-view-1 prevents reusing units that constructed the old literal kind.

Validation is complete on the exact compiler3235494 base: workspace clippy,
targeted native/free/wasm, complete LLVM workspace suite and JS runtime300/300 all
exit0. Logs: /tmp/fa-region-string-fix-{base323-clippy,base323-targeted,
workspace-full,js-runtime}.log. This fix is independent of the direct/relative
root-edge prototypes. The explicit `copy parent.text` regression is staged for
Root's new Copy frontend in /tmp/fa-native-perf-next/region-string-explicit-copy.fa;
the ancestor-arena MOVE materialization must also be preserved at that merge.

## Direct root edges and relative links (validated delivery)

The candidate based on 874cf12 consumes the existing closed ROOT_REGIONS proof without
widening it. Direct same-root fields use 32-bit links; supported scalar fields retain
natural alignment. Pure-link payloads may locally shrink to signed 16-bit offsets in
eight-byte units, with a 32-bit fallback after the actual allocation address is known.
Both layouts keep one independent owner per root. Constructors, deep copies, patterns,
comparisons, formatting and JSON reconstruct the ordinary payload ABI. Split/foreign/
opaque/unsupported cases retain the prior layout. Header bounds, constant codes, large
member families and relative-distance boundaries are checked; cache adds region-edges-1.
Only a simple scalar first arm of a throwing direct recursion is expanded, preserving
the result/error struct and evaluating arguments once. General arithmetic leaf inlining
and switch-table experiments are excluded from the delivered candidate.

The all-payload relative16 experiment is preserved in
/tmp/fa-native-perf-next/compilers/relative-edges-stage8 with strict manifests and reports
relative16-{selected-instructions,memory,selected-cpu}.json. At original default sizes,
18 quiet rotations using the same Clang 18 toolchain gave interp 66.89→54.31 ms versus
Goose 39.45 ms; child RSS 7808→5888 KiB and faults 1635→1162. Instructions increased
151.39→164.03 M. Calc regressed 220.67→240.05 ms without a memory saving, so the final
candidate keeps all mixed/scalar payloads in the fixed 32-bit layout. These CPU numbers are
the experiment's measurements, not final candidate measurements or an overall victory.
The baseline, direct32 checkpoint and all reports/executables remain archived under
/tmp/fa-native-perf-next. The final original 16 source/output/manifests are verified in
direct-relative-final-clang18/preparation.json. Thirteen executable hashes are unchanged;
their checked instruction counts are explicitly reused in instructions.json. The changed
programs were profiled again: calc 1251.812 M (3.08 percent below baseline), interp 164.029 M
(11.76 percent above baseline) and sexp 789.235 M (611 instructions above baseline). Final
CPU samples await the next naturally quiet window after Root's combined full; no overall
Goose victory is claimed.

Exact-source validation is complete: fmt, workspace clippy, all 61 LLVM IR checks, all
targeted region programs, C ASan/UBSan, COMPLETE LLVM workspace full and JS runtime
300/300 pass. Logs: /tmp/fa-direct-final-{clippy,targeted,runtime-sanitizers,
workspace-full,js-runtime}.log; full session21647 returned0. The independent next
experiments (arithmetic leaf expansion and shared range differences) live only in the
older prototype tree and archived compilers, not in this delivery.
The exact candidate also passes the region corpus under QEMU AArch64: native12/12,
freestanding10/10 plus the2 documented JSON/libc skips, in
/tmp/fa-direct-final-aarch64{,-free}.log. A separate O0 probe with an external18KiB
Unicode payload fails on the old compiler and survives parent destruction/churn with
the String fix: /tmp/fa-region-external-string-check/main.fa.
