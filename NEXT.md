## Current analyzer-partial checkpoint (2026-11-07, supersedes old notes below)

- Parenthesized local/imported forall function values keep their full generic
  type on every Group and the leaf. The declaration lookup already strips
  Groups in this complete frontend. The new CLI fixture passes Bun and all
 3LLVM targets/leak checks; cumulative analyzer457 complete and workspace
  clippy LLVM are green. Sequential delta after Mutex stores:
  /tmp/fa-partial-grouped-forall.patch and matching checkpoint.
- Mutex payloads use the normal owned-store checks, including conditional
  Task movement for erased parameters, and rejecting borrowed Resource/Array
  stores. This restores proofs' validated Mutex hook in this older source.
  WorkerPool also preserves its production contract: state must be copyable
  for every worker/restart, and the state expression is an owned sink. A
  Resource-containing state is rejected (Mutex<Resource> remains a handle).
  Native same_layout accepts equal global Sync payloads without ending the
  coinductive check early. Root's K::Sync remap + regression is ported too;
  without it a forall returning Mutex<T> leaked a local TypeId into global
  class_key and panicked. Targeted FE3, remap test, pool negative case and
  factory/forall on3LLVM/leaks are green. The cumulative analyzer457 complete
  suite/cases/corpus/pipeline/docs and workspace clippy LLVM are green; JS
  TypeInfo execution is validated in proofs' production checkpoint, not this
  old minimal JS backend.
- Qualified worker constructors and aliases keep the `.worker` export's
  written identity, including `library.Echo { … }`, `type Alias = Echo` and
  `use { Echo } = library`. They construct a worker handle and take the same
  RPC/affinity path as a local export proxy; nested types of a worker file do
  not become workers merely because of their file. Field access stays CONC008.
  Cache advances qualified-worker-1. Analyzer453 full/cases/corpus/pipeline/docs,
  workspace clippy0, Bun package and native/freestanding/leak are green; wasm
  keeps the documented worker platform skip. Sequential delta after union
  defaults: /tmp/fa-qualified-worker-constructors.patch and matching snapshot.
- Inline object payloads of named unions keep their own defaults by
  (DeclKey, member index), including equal-shaped variants. The interface
  publishes/remaps/hashes those defaults (tag112) separately from object
  defaults, so imports never pick another variant's defaults. Recursive Tree
  payloads and empty generic collections construct fresh values each time.
  Native and JS local defaults receive the union's type/value binders; the
  native cache advances union-defaults-1. Substitution keeps a declared
  single-member union as a type namespace (Label<text>.Text remains valid).
  Analyzer453 full/cases/corpus/pipeline/docs, JS crate full,2 fixtures on all
 3LLVM targets/leaks, Bun stdout and workspace clippy0 are green. Final small
  cleanup (type alias + conservative module-cycle deferral) is also clippy0
  and imported-case green. Sequential delta after qualified function values:
  /tmp/fa-union-payload-defaults.patch, snapshot of the same name.
- Still to audit: defaults depending on a generic value across files need a
  portable recipe (the existing constant-default interface rejects them as
  runtime defaults). Anonymous nested object annotations also lack a written
  default owner; do not infer their defaults from structural TypeId equality.
  Implicit default/destructor effects must contribute at their evaluation sites.
- Final ownership policy: Resource/destroy and Task are unique/noncopyable;
  Resource parameters borrow, even when containing Task. Task-consuming ABI
  stops at Resource/Sync. Collections need explicit copy unless a proved move.
  Escaping captures cannot own any unique value. Root owns JS/native unique
  Resource/AOT integration; do not restore the abandoned Resource-RC prototype.
- VM factories now retain semantic instances (exact type/value bindings and
  selected written methods), including forwarding forall functions, executing
  constraint methods, and returning a closed forall. API/snapshot:
  /tmp/fa-vm-function-instances.patch and /tmp/fa-vm-function-instances-checkpoint.
  Analyzer439+cases/corpus/pipeline/docs, clippy,2fixtures x3LLVM were green.
- Retained defaults of with are implemented: portable SignatureNames/defaults,
  Function origin/constant intrinsic defaults, named omissions, generic explicit
  applications, a separate native default entry in a counted adapter, default
  flags for source lambdas, and split generic producer role/cache separation.
  Ten with fixtures pass3LLVM with leak-check, including imported factory
  defaults, outer generic value binders and Resource default cancellation.
  Source Function.captures is the one capture order; preserve bindings/methods.
- The fresh child of an async Partial is created before its own first suspend;
  cancellation guards drop that child before its borrowed parameters/env. This
  preserves the EXISTING native source-function default timing.
- OPEN language-order question sent to Root: Native source start evaluates
  parameter defaults immediately; JS source start waits for its first turn.
  The docs specify arguments now/body later, but not default binding explicitly.
  Do not change JS globally until answered. Repro saved outside corpus at
  /tmp/fa-with-start-default-timing.fa. Corpus tests independent of this choice
  cover actual default values and cancellation after the child has started.
- Native indirect Function ABI now borrows aggregates independently of program
  async mode; forwarding thunks adapt raw source entries. Partial/await/start
  preserve changed argument places. Lambda parameter effects are kept (captured
  effects were already separated), and Function effects use covering paths.
  Analyzer441+cases/corpus/pipeline/docs,11 with fixtures x3LLVM, and clippy are
  green. Checkpoint: /tmp/fa-function-value-abi-checkpoint; sequential delta
  /tmp/fa-function-value-abi.patch after the defaults checkpoint.
- Concrete native Function coercions are compiled adapters with counted envs,
  argument/result conversion, structural mutation writeback on return/error/
  cancellation and requirements forwarded by key. Async wrappers keep input/env
  owners until real invocation. On wasm only an async source signature may use
  the low-bit DATA-table tag; synchronous function-table indices can be odd.
- Native results transfer retained fields and destroy excluded fields using
  Root's owned_projections module (copied here, with the old Object-Resource
  representation gate only). Function children are marked fresh before convert.
  Local/global field names compare by text in same_layout and projections;
  otherwise a representation-only conversion could copy and double-free a Task.
  Non-generic destroy lookup also matches the globalized declared type.
  Root's final Task-parameter predicate is used (Resource params borrow), and
  proofs' pending_box-in-Frame fix is coalesced. Preserve Root ResourceDrop ABI.
- Validation: analyzer442+cases/corpus/pipeline/docs;47 LLVM checks;8 function-value
  native programs (7+one historical platform skip on wasm/freestanding),11 with
  and2 factories programs on all3 LLVM targets;clippy clean. New regressions cover
  optional 128-bit inputs, captured callable owners, changed views, errors, async
  lifts, Task/Resource carriers, dropped extras and Function-valued return fields.
- Concrete JS numeric adapter is supplied only as a patch against proofs'
  production: /tmp/fa-js-function-numeric-adapters.patch. It uses his compiled
  templates, copies/adopts the target once, handles numeric optionals/async and
  preserves non-numeric omission markers. Proofs will validate after his freeze;
  it is NOT applied to this old minimal JS tree. Forall/structural deep adaptation
  still needs the shared AOT operations; no runtime Fa schema walk is allowed.
- Native generic-to-generic adapters now forward target-class requests to the
  source class, preserving exact type/value arguments and method selections.
  A counted adapter environment retains the original unboxed function pair;
  each requested entry reuses the compiled concrete signature conversion.
  Producer roles and source/target signatures survive per-unit instance waves.
  Class instances compare exact arguments as well as representations/methods:
  `label<"first">()` and `label<"second">()` must not share one String instance.
  Generic function types now accept their documented compile-time value binders,
  with type/value kind and constraint checks. Unannotated declaration aliases
  still report GN015, including phantom parameters absent from runtime types.
- Validation of that delta: analyzer444+cases/corpus/pipeline/docs, clippy
  workspace clean, and both forall conversion fixtures pass all3 LLVM targets
  with leak checking (one also requires compilation by units). The cases cover
  numeric arguments, fewer errors, async conversions/lifts, captured callables,
  same-layout Person/Greeting methods, phantom types and distinct String values.
  Sequential patch after the concrete checkpoint:
  /tmp/fa-generic-function-adapters.patch; snapshot of the same name.
- Native object conversions now check child representations coinductively before
  reusing an LLVM layout. Equal-size Function pairs can require different call
  entries; skipping their adapters trapped in wasm or ran a lifted body eagerly.
  Deep object fields now retain numeric/error/async conversions and owned envs.
- The documented return-to-Void rule is implemented in Function assignability:
  any source result is accepted and dropped. Native counted adapters already
  drop returned Resource/Task/string values on sync and async paths; regressions
  also cover forall values and a converted Function field. Analyzer445 complete,
 47 LLVM checks, both new fixtures on3 targets with leak checks, clippy clean.
  Delta: /tmp/fa-function-deep-void.patch after the generic checkpoint.
  JS consumer-only follow-up /tmp/fa-js-function-void-adapters.patch is supplied
  sequentially after the numeric adapter; proofs validates it in production.
- Function aliases now materialize their named values as owned computed locals,
  consume JS expression temporaries, and register their escaping capture copies.
  Their complete generic contract is retained, including phantom value binders;
  statically known calls still keep Source FnInfo's defaults/callback instances.
  Targeted analyzer regression and named alias/captures/defaults/Mutex/value-binder
  fixture on3LLVM/leak are green. Full analyzer/clippy of this new alias delta
  is still required before final integration (prior445/clippy preceded it).
- Function types accept complete `mutates`/`invalidates` paths and `requires`.
  They declare parameters/requirements in their own scope and reuse the body's
  path validation/normalization. Requirement assignment compares keys by type,
  independently of their local names. Type-only parameters have no unused-value
  warning. The adversarial req-06 case is active again and checks missing provide.
  Analyzer447 full, JS crate full, Bun fixture,3LLVM fixture/leaks and clippy are
  green; this also validates the preceding alias delta. The golden check-bits.fa
  has the same explicit `copy items` migration already present on Root.
  Delta /tmp/fa-function-type-clauses.patch follows the alias checkpoint.
- Plain module-level aliases now forward to their source function instead of
  reading an uninitialized computed slot when used before their declaration.
  Aliases of runtime `let` values or callback parameters report FN110; named
  functions with runtime DATA captures and checked compositions/with values keep
  their lexical owners. Constant factory aliases retain the cached VM function.
  Native imported functions materialize ordinary or forall values, including
  declaration defaults and per-unit generic producer/default tables.
  Analyzer448 full, JS crate full, early alias Bun probe, two alias fixtures on
  all3LLVM/leak, and workspace clippy are green. Sequential delta:
  /tmp/fa-function-alias-hoisting.patch after function-type-clauses.
- Compile-time value defaults now report GN003 outside their constraint, GN008
  for runtime defaults, and GN005 after either a type or value default. Forall
  applications reuse the declaration's default/constraint/binder filtering;
  named calls also infer phantom/value parameters absent from their runtime
  signature. Rigid recursion and explicit applications keep their bindings.
  Analyzer450 full and clippy workspace are green; function-generic-defaults.fa
  passes Bun and all3LLVM targets with leak checks. Sequential delta:
  /tmp/fa-generic-defaults.patch after function-alias-hoisting.
- Qualified function aliases (`namespace.function`, `Type.method`) are accepted
  and retain names/defaults/generics. Native namespace reads materialize the
  function pair; historical ft-9 now passes all3targets and its exclusions are
  removed. JS computed module functions use real exported init functions and
  assignments to predeclared bindings, matching Program.init_order. Root's
  ConstantFunction branch must use the same binding, and worker_program must
  join the initializer predicate when coalescing production.
  Nineteen obsolete pending markers are removed after analyzer checks; executable
  co/ft, Error.toString and union tests pass Bun and3LLVM. co-11's factory marker
  remains until the production ConstantFunction consumer is coalesced (its
  analyzer passes, but this old JS tree still rejects function factory values).
  Analyzer450 full, JS crate full, workspace clippy, local/foreign alias fixtures
  and a real Bun package with imported qualified aliases are green. Delta:
  /tmp/fa-qualified-function-values.patch after generic-defaults.
- OPEN before final full/integration: JS erased F:(UInt32)=>UInt128 needs a
  static call capability when concrete F
  actually accepts UInt128/returns UInt32; concrete numeric adapters alone do not
  cover that generic constraint. Also audit deep structural Function fields.
  Field/index effects/requires and native returns-as-Void are validated above;
  JS adapters still need the production-template consumer and its full run.
- OPEN language questions sent to Root (still errored at this checkpoint):
  (1) start/default timing above; (2) adapting Resource-with-Array returns to
  Plain-with-Array would need a structural copy before destroy can observe its
  fields: restriction vs implicit copy contract; (3) direct throw in do/catch:
  analyzer ignores catch, both backends use it, errors.md says throw exits the
  function like return. Repro /tmp/fa-split-do-catch-repro/main.fa. No guessed
  semantic edit was made. Local methods capturing DATA also need constructor
  provenance/environment transport (Projection::Method like Finalizer), no new
  language prohibition; coordinated with proofs.
- Pending after these: constant-composition factories/finite-depth reifier,
  union written-origin joins, deferred generic keyloop effect signatures,
  documented rules/Schema/native nominal cases, destruction effect summaries,
  and measured analyzer pass fusion. No final fullLLVM/commit yet; coalesce
  Root AOT/TypeInfo/Resource and proofs wire/Function templates before full.

## with implementation active (analyzer-partial, 2026-11-07)

- This separate tree starts at compiler aa3e050 (ecda origins + root proofs +
  interpreter heap continuations). It does NOT include unintegrated root
  Copy/TypeInfo/AOT/Resource/Function-env WIP. Keep its target separate.
- Uncommitted frontend: PartialApplication side table; FN100..104, FN072;
  captures checked and bound parameters inferred, other generics retained;
  residual effects/positions remapped. Direct function declarations accept with.
- SignatureNames is portable metadata, not part of FnSig identity. References
  by DeclKey keep recursive named types finite. The module interface publishes,
  hashes (tag109) and merges these names. with uses visible annotations, including
  function parameters/fields/imported aliases. 5 targeted tests pass, including
  partially fixed generics and structural equality with differently named params.
- Expr::Partial is flattened to Op::Partial and uses Gather/Finish in the VM;
  no Rust evaluation recursion. Closure.bound already existed. Three interpreter
  tests cover evaluation once/in source order, defaults seeing bound parameters,
  and binding residual positions in another with. Analyzer lower/read_symbol
  handles with and local declared aliases. comptime::visit handles its children.
- Full analyzer+interpreter green after migrating obsolete walk_tests AN001
  expectation to the three actual unresolved names. Log:
  /tmp/fa-partial-frontend-interpreter-recovery.log (session70616).
- Do not remove co-9..12 pending until all backends work. Named/default calls of
  declared partials, constant factory FN110, original generic order/capabilities,
  imported/native split declarations, async forwarding, and copy-core/provenance
  integration remain open. Index-in-effect bindings currently conservatively All;
  precise constant indices can be retained later.
- Root specifically requires NoTask on bound values even generic/fresh, no FnOnce.
  Copy/move collection policy must be applied after copy-core rebase; note_copy_site
  alone would wrongly use Task parameter consumption. Add portable Partial origin
  (callee + bound values + residual argument mapping) for copy/CONC008 summaries.
- JS MUST use the existing counted Function protocol. Proofs says:
  faOwnShared(fn, finalizer) returns a count1 function; copy Function retains;
  register_function_temporary(site, value) once; values via copy_stored; static
  finalizer calls storage_drop_function(ty), function target faReleaseShared.
  aa3e050 lacks these hooks/flags/scope drops; do not invent a parallel protocol.
  Proofs/root will extract a coalesced FnEnv delta after validation. VM/native can
  proceed independently until then.
- Native design: closure env {drop ptr, copy ptr, refs:i64, target Function pair,
  bound values}. First two fields preserve existing Function dispatcher ABI.
  Refcount header is the same as proofs normal closures. Body forwards residual
  ABI arguments and borrows env fields for bound composites; mutated bound params
  are rejected. Generic partial values should reuse generic_values tables with a
  ProducerKind::Partial (lambda=true in split serialization; detect Partial AST
  at define_generic_entry). Original target can itself be a generic function
  table; wrapper entry selects its full original type arguments, combining fixed
  and residual ones. Avoid losing original forall binders to residual binding
  maps (temporarily exclude original binder ids when concretizing its class).
  Producer keys and helper names must include exact types/method selections,
  not only Repr, including phantom outer bindings.
- Native monomorphic and residual-forall with are now implemented. Common
  review/with-runtime.fa and with-generic.fa pass on native/freestanding/wasm,
  including captured prefix, fresh array, runtime snapshot and two types of one
  residual forall. Source FnInfo.captures replaces the old scanner and generic
  declarations with captures now materialize a real environment. Constructing
  a forall residual must not automatically instantiate that new value using
  the target's full instantiation list (currently its list is removed from a).
- Proofs' /tmp/fa-function-rc-native.patch and /tmp/fa-function-rc-js.patch are
  applied (all3 runtime IR regenerated), plus task lifetime runtime delta
  /tmp/fa-function-task-lifetime-analyzer.patch. Keep root/proofs coalescence;
  the JS minimal bridge uses local contains_function_value/faDropFunctions and
  must coalesce to root AOT/contains_shared_handle instead of publishing a second
  runtime graph walker. JS with uses faOwnShared, staged capture guards,
  register_function_temporary, copy_stored and storage_drop_function.
  Bun CLI printing tests 5/5 green with the two common fixtures;3 LLVM targets
  repassed after the Function RC change (/tmp/fa-partial-native-function-rc.log).
- Names/bound-parameter metadata just extended for named residual-declaration
  calls and FN102 on binding again through chained with. cargo check analyzer
  green; new tests and named/chained runtime additions await the end of root's
  CPU window. SignatureNames.Function.bound=Some(..) distinguishes partials;
  merge/hash remap names. Defaults not yet implemented in this metadata.
- OPEN with work: retained defaults (FnSig deliberately has no default flags;
  need a secondary default-aware entry/portable visible contract, preserving
  ordinary first-class Function ABI), true async native wrapper retaining env
  before initial suspension (proofs supplies runtime/RC helpers; current partial
  calls target sync), compiletime factory binding FN110 and emitted constant
  closures, imported/split partial declarations, requires forwarding, callback
  error specializations, explicit Copy/capture/provenance/TypeInfo integration.
  Do not remove co-9..12 pending or claim the whole with feature complete yet.

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
- Resource handles and other shared handles remain copyable with reference counts,
  sharing the same state. Resource handles are exempt from explicit collection
  copy requirements even when their shared state contains a collection.
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
invalidating the source and aliases. Resource handles still share cells locally;
at a worker boundary they therefore require a common Mutex or exclusive transfer.
No automatic synchronization of Resource fields. Analyzer must track the cell
aliases and Guard loans: neither an old alias outside a Mutex nor a Resource
handle copied from Guard.value may bypass the protection. Task remains affine
to its executor and never transferable between workers.

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

Further delivered, not yet merged: daa8ebc Frame64byte strings (native_perf),
a9b1375 capacity-unobserved ROOT_REGIONS fact (proofs). Both passed full/clippy.
Keep their worktrees while agents reuse them or until subsequent deltas are safe.
Physical region layouts are separately frozen748d3a4 in
/tmp/fa-native-region-validation, full /tmp/fa-region-frozen-workspace-full.log.

Root shared-values has experimental origins+Copy+callable checkpoints plus
TypeInfo tag4, Task/default guards and native entry ownership. Its new
owned-task-parameters program passes Bun and3LLVM/leak-check, including failed
earlier default releasing a later provided Task. Shared/default resources pass
Bun and3LLVM. JS runtime308 and entire JS crate pass after reviewed goldens.
Those changes are NOT a frozen/complete compiler integration candidate yet.

Latest open semantic clarification: user asks what “shared resource” means in the
generic exact-mutation question. Root explained a type with destroy and copies
sharing one counted instance/state, as previously selected. No choice of generic
instantiation restrictions vs conditional effects has been made; do not assume
an elapsed-time answer. Agents continue concrete cell/loan proofs independently.
