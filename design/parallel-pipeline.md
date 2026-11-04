# Parallel and streaming pipeline: implementation plan

Status: accepted by the project lead (2026-11-02). It builds on `design/compiler-architecture.md` (§ Performance principles) and `design/analyzer-design.md`. It **replaces decision A7** (single-threaded `Session`) and the "on demand across files" paragraph of the architecture doc. Analysis on demand stays the rule inside a module and inside a cycle of modules.

## 0. Decisions

| # | Decision | Why |
|---|---|---|
| P1 | **Types live in one `GlobalStore`.** It is append-only and `Sync`. Each analysis unit (a thread) reaches it through its own **view**, `TypeStore { global: Arc<GlobalStore>, local, caches }`. The view has two modes. **Direct**: everything is interned straight into the global store (language server, tests, library API). **Overlay**: new structures go to a local overlay, and at unit completion only what the interface reaches is merged into the global store, with id remapping (`fa check` and `fa build`). | The walker keeps its `&mut TypeStore` API, so 44k lines don't move. Hot-path writes stay local with no contention. Local and literal types die with their module, so memory stays bounded. |
| P2 | **Id encoding: bit 31 of every store id = local.** Equality stays an integer compare inside a view. Global ids are canonical: one structure, one global id. | One well-predicted branch in `kind()`. No id carries a shard. |
| P3 | **Ids are not deterministic. Outputs are.** Nothing is ever ordered by a store id: unions are ordered by `stable_hash`, names come from `DeclKey`s and source positions, and object display names use a deterministic winner. A stress test (P10) enforces it. | Merge order depends on thread timing. Requiring deterministic ids would force layer barriers. |
| P4 | **Scheduling unit = one SCC of the module graph.** The graph comes from a scan fused into the parse. Units run on Rayon as soon as their dependencies have published their interfaces. The ready queue is a priority heap on critical-path length. There is no signature pre-pass and no blocking wait on another thread. | One walk per file. A module never reads a dependency that isn't finished, so cross-module on-demand analysis only happens inside an SCC, sequentially, as today. |
| P5 | **A module is read only through its `ModuleInterface`**, never through another file's `Analysis` or tree. `Resolution::External` names a `DeclKey` (decision A10), not a `NodeId`. | This is what lets trees be dropped. It also fixes stale `NodeId`s after a re-parse in the language server. |
| P6 | **Lifetimes.** A tree, its side tables and the unit's overlay are dropped as soon as the unit's diagnostics are rendered and its code is generated. Outputs are written by the worker that produces them, into `<out>.partial/`, which is renamed at the end. | Memory doesn't depend on project size. Outputs are never accumulated. |
| P7 | **LLVM: one LLVM context and module per unit, compiled in parallel to objects, then linked.** Release builds use ThinLTO in `ld.lld`/`wasm-ld`; development builds link plain objects, cached per module. Generic instances are requested by users and emitted by the **defining** module in later "instance waves". | Memory stays bounded and all cores are used. Size is kept by ThinLTO and `--gc-sections`. |
| P8 | **Worker threads have 64 MiB stacks** (`ThreadPoolBuilder::stack_size`). The binaries use **mimalloc**. | The walk recurses deeply, and Rayon's default is 2 MiB. glibc arenas keep the memory freed by dropped trees. |
| P9 | **Per-thread caches**: `assign_cache`, `subst_cache`, scratch buffers and coinduction assumptions live in the view, never in the global store. A result that involves an *incomplete* type (payloads not set yet, a `Rec` without a target, a parameter without bounds) is not cached, so no cache is ever cleared. | Removes today's whole-cache clears (§4 #7) and shared mutable caches. |
| P10 | **Determinism is tested.** The same inputs are run with 1, 2 and N threads under several `FA_SCHEDULE_SEED`s, which shuffle the order of ready units with equal priority. Diagnostics and every output file must be byte-identical. | This is the guard for P3. |

## 1. Concurrency model of the analyzer

### 1.1 Options considered

- **Sharded global interner only (ids encode the shard).**
  - Every new type is a write to shared state.
  - Literal types (one per literal) accumulate forever, so memory grows with the project.
  - The late-bound tables (`payloads`, `recs`, `params`, `names`) become concurrent maps that every read goes through.
  - Kept only as the Direct mode of P1, where it is simple and memory doesn't matter (the language server, tests).
- **Frozen global store plus per-thread overlays, swapped per layer.**
  - Reads need no lock, but a new snapshot either copies the index or waits at a layer barrier.
  - One big module idles the whole layer.
  - Rejected.
- **Chosen: an append-only global store with lock-free reads, and per-unit overlays merged at completion (P1).**

### 1.2 `GlobalStore`

Every table of today's `TypeStore` (`packages/analyzer/src/types/store.rs:76-114`) moves into it, with the same layout:

- **Arenas**: `kinds`, `flags`, `stable`, `syms` bytes, `lists`, `field_lists`, `shapes`, `sigs`, `paths`, `effects`, `masks`, `consts`, `nominals`, `modules`, `decl_keys`, `params`, `spellings`, `recs`. Each is a **segmented vector**: 32 buckets of size 2^k, an `AtomicPtr` per bucket, never reallocated.
  - `get(i)` takes no lock and returns `&T` that lives as long as the store.
  - `syms.resolve` returns `&str` from a chunked byte arena under the same rule.
- **Index** (structure → id): 16 shards of `RwLock<hashbrown::HashTable<u32>>`, keyed by the FxHash already computed for the value. This is one hash per intern, not two as with `HashIndex` today (`intern.rs:22-37`).
- **Intern**:
  1. Read-lock the shard and look the value up.
  2. On a miss, write-lock the shard, look again, push to the arena, insert.
  - The arena push is the only cross-shard operation: an atomic `len` reservation.
- **Late-bound data**: the payloads of a nominal, the target of a `Rec`, the bounds of a parameter. Each becomes a **write-once slot** (`OnceLock` in the arena entry).
  - Only the declaring unit ever writes it: these ids are keyed by the declaration's `DeclKey`, so no other unit can create them.
  - Other units read it only through a published interface. The scheduler's hand-off establishes happens-before.
- **`names`** (the display name of a structural object type, `store.rs:647-653`): "first wins" becomes "smallest `(module path, offset)` wins", compared under the shard lock. Deterministic.
- **The prelude** and the predefined ids (`TypeId::NEVER…RANGE`) are interned into the global store before any unit starts, so their constants keep working.

### 1.3 The view, `TypeStore`

```rust
pub struct TypeStore {                 // one per unit; the API the walker uses, unchanged (&mut self)
	global: Arc<GlobalStore>,
	mode: StoreMode,                   // Direct | Overlay
	local: LocalTables,                // same tables as today's TypeStore (Vec + HashIndex); empty in Direct
	assign_cache: FxHashMap<(TypeId, TypeId, Mode), Result<(), Reason>>,   // P9
	subst_cache: FxHashMap<(TypeId, ListId), TypeId>,
	scratch: Vec<TypeId>, rec_assumptions: Vec<(TypeId, TypeId)>,
}
```

**Interning in Overlay mode:**

1. If any child id is local, the structure can't exist globally: intern it locally and stop.
2. Otherwise look in the local map, then in the global index. On a global hit, **record the global id in the local map** so the global index is asked at most once per distinct structure and unit. Otherwise intern locally.

**Why one structure gets one id inside a unit:**

- The local map is consulted first, and whatever it records is final.
- Every global id a unit can hold either comes from its dependencies' interfaces, which were merged before the unit started (so a global lookup would have found it), or comes from its own global lookups.

**Merging a unit (Overlay mode):**

- The roots are its interfaces (§2.2) plus, for LLVM, the type arguments of its instance requests.
- A remap visitor deep-copies every local id they reach into the global store, children first, deduplicated by the global index:
  - `TypeKind`, `ListId`, `ShapeId`, `SigId`, `PathId`, `EffectSetId`, `MaskId`, `ConstId`, `NominalId` (with payloads), `RecId` (with target), `ParamId` (with bounds), `DeclKey`, `ModuleId`, `Sym`, `SpellingId`.
- The interface records are rewritten with the remap. The unit's own side tables keep their local ids: they are only read by this unit's code generation, while the view is alive.
- A debug assertion on every global insert checks that no local bit appears in a global entry.

### 1.4 Symbols, declaration keys, facts, compile-time evaluation

- **`SymbolId`, `ScopeId`, `NodeId`**: per file already (`symbols.rs:191`). They stay in `Analysis` and are never shared.
- **`DeclKey`, `ModuleId`, `Sym`**: interned through the view like types (local or global by the same rule).
  - Module ids are interned in the global store at the scan: one per module path, deterministic order.
- **`PlaceId`, `FactId`, `IntervalId`**: already per walker (`check/mod.rs:456-457`, `Analysis::interval_values`). No change.
- **`comptime::State`** (`comptime/mod.rs:49-62`):
  - `functions` and `purity` become a shared cache: a sharded `Mutex<FxHashMap<FnKey, Arc<Function>>>`.
  - The static values of a module go into its interface.
  - `globals` stays per unit. Cross-thread cycles are impossible, because a cycle between modules is one unit (one thread).
  - Evaluation runs on large-stack threads (decisions.md, "Bounds"): a pool of up to one per core, reused, so the evaluations of parallel units run in parallel (see the implementation status).

### 1.5 Session split

`Session` (`session.rs:47-65`) becomes two parts.

- **`Shared`**, which is `Sync`:
  - the global store,
  - the resolver (`ModuleResolver: Send + Sync`; today it is only `Send`, `modules.rs:63`),
  - the module table (path → `ModuleId`/`FileId`, fixed after the scan),
  - `interfaces: Vec<OnceLock<Arc<ModuleInterface>>>`,
  - the compile-time cache,
  - target and configuration.
- **`UnitSession`**, owned by one thread: `{ shared: &Shared, types: TypeStore, files: SmallVec<[FileState; 1]> }`.
  - `Walker.s` becomes `&mut UnitSession`.
  - `self.s.types.*` calls don't change. Reads of other files (`self.s.source_file(f).interface`) become `self.s.interface(module)`.

`Session` stays as the facade for the language server, tests and `analyze_source`: `Shared` plus retained units, in Direct mode, with the same query API.

## 2. Scheduling

### 2.1 Module graph

- The parse job of each file also produces a **`FileScan`**, collected while nodes are pushed:
  - the names used (and the syntactic access chains `a.b.c` that start at a name),
  - namespace blocks,
  - whether there is reactivity,
  - parse error spans.
- Its import part **is** the analyzer's import collection: `import_names` (`check/modules.rs:451-482`) then reads it instead of scanning the tree. So the edges are a superset of what the walk reads, by construction.
  - A name shadowed locally gives a false edge. That only costs parallelism.
  - A debug assertion fires if the walk reads a module outside the unit's dependency closure.
- **Edges**: `resolver.lookup(module, name)`.
  - `File(m)` gives an edge to `m`.
  - `Folder(p)` follows the recorded access chain through `resolver.member`. A folder used as a value gives edges to every file below it.
- **Tarjan SCCs**, visited in module-path order (deterministic), then the condensation DAG. Each unit gets:
  - its critical-path length (the longest chain to a sink, in total source bytes),
  - a pending count (its dependencies),
  - its reverse edges. The language server's `update` uses these instead of scanning every file (`program.rs:538-550`).

### 2.2 What a unit reads from its dependencies: `ModuleInterface`

```rust
pub struct ModuleInterface {
	module: ModuleId, role: FileRole, hash: u128,            // hash from stable hashes (§4 #6)
	export: Option<DeclIdx>, members: FxHashMap<(DeclKey /*namespace*/, Sym), DeclIdx>,
	decls: Vec<ExportedDecl>,       // by DeclIdx; DeclKey → DeclIdx map
	type_decls: FxHashMap<NominalId /*global*/, SmallVec<[(Sym, DeclIdx); 4]>>, // functions/methods of exported types
	init: Vec<(DeclKey, SmallVec<[DeclKey; 4]>)>,            // module-level values and the module values they read (walk-recorded)
	reads_target: bool, build_inputs: Vec<String>, generics: Vec<DeclKey>,
}
pub struct ExportedDecl {
	key: DeclKey, kind: SymbolKind, ty: TypeId /*global*/, name_span: SourceSpan, node: NodeId, // node: same-version backends and the LSP
	params: SmallVec<[(Sym, bool /*has default*/); 4]>, is_method: bool,
	summary: Option<FnSummary>, static_value: Option<fa_interpreter::Value>,
	js_export: Option<Box<str>>, mutable: bool, is_namespace: bool,
}
```

The interface replaces every place that reads another file's analysis or tree today:

| Today | Replaced by |
|---|---|
| `external_type_function` (`check/modules.rs:237-288`) | `type_decls` |
| `external_parameters` (`:317-349`) | `params` |
| `proxy_decl` (`:291-313`) | `export` / `members` |
| `Session::external` / `type_origins` (`session.rs:165-227`) | `decls` |
| Compile-time lowering of external declarations (`comptime/lower.rs:486`, `:1113`, `:1334`) | `decls` |
| The JavaScript `export_names` / `program_contexts` (`codegen-js/src/generate/modules.rs:91`, `:182-233`) | `js_export`, `is_namespace`, `mutable` |
| Initialization order (`program.rs:263-407`) | `init` |

### 2.3 Running units

```
rayon::in_place_scope(|scope| {
	for unit in initially_ready { ready.push(unit); scope.spawn(token) }
	token: unit = ready.lock().pop()              // max (critical path, then path order ⊕ seed)
		parse missing trees (§3.1) → analyze the unit's files → merge (§1.3) → publish interfaces (OnceLock)
		→ render diagnostics into the reorder buffer → generate code / object → drop (§3.1)
		for d in rdeps[unit] { if pending[d].fetch_sub(1) == 1 { ready.push(d); scope.spawn(token) } }
})
```

- **Inside a unit**: today's mechanism, unchanged.
  - Pending declarations are analyzed on demand.
  - Another file of the unit is analyzed on demand (`ensure_analyzed`, `check/modules.rs:713`).
  - `deferred` rounds, but only over the unit's files: `settle` currently re-walks every file of the program (`program.rs:93-112`).
  - A unit with one file never defers.
- **Why not wait for another thread, or run a signature pre-pass?**
  - Waiting blocks Rayon workers and risks deadlock inside the pool.
  - A pre-pass is a second walk.
  - The DAG makes both unnecessary. The cost is that the critical path bounds the speedup. Generated and real projects are wide, and the critical-path priority keeps long chains moving.
- **Program-level checks** run after the scope, from interfaces only:
  - the initialization order and `MOD007` (from `init`),
  - `MOD015` (from `export`),
  - the scan diagnostics.
- **Determinism**:
  - Each file's diagnostics are sorted by span (as today).
  - Files are printed in module-path order through a **reorder buffer** that holds rendered strings only and flushes the longest finished prefix.
  - Generated code depends only on the unit and the interfaces' contents (P3).
  - Instance symbols and union order use stable hashes and `DeclKey` paths.
  - Cancellation (language server) is checked between units and at the existing declaration and loop boundaries.

## 3. Streaming and memory

### 3.1 Lifetimes (`fa check`, `fa build`)

| Structure | Created | Dropped |
|---|---|---|
| `FileScan`, module graph | scan phase (parallel parse) | end of the run (a few hundred bytes per file) |
| Source text `Arc<str>` | read in the parse job | after the unit's diagnostics are rendered |
| Tree | scan phase, kept within a **tree budget** (128 MB, first come); beyond it, dropped and parsed again when the unit becomes ready | after code generation (`fa check`: after the diagnostics) |
| `Analysis` side tables, walker maps, `FactStore`/`Places` | start of the unit | with the tree |
| Overlay `LocalTables` and view caches | start of the unit | after merge and code generation |
| `ModuleInterface` + its global types | merge | end of the run |
| Rendered diagnostics | unit end | flushed by the reorder buffer |
| Generated JavaScript / object file | the worker | written immediately, never held |

**Budget for 1,000 files of 500 lines.** About 5 k nodes per file, at about 80 B per node for the tree plus about 30 B for the side tables, is about 0.5 MB per file. Retaining everything (today) is about 550 MB. Streaming holds the tree budget, plus the units in flight (threads × about 2 MB), plus the interfaces (about 20–50 KB per file, so 20–50 MB). The target is **< 200 MB**, far under 512 MB, and growing only with the size of the interfaces. Step 0 measures the per-node figures.

**Other rules:**

- **Compile-time evaluation into a dropped module** (rare): rehydrate it. Parse it again and analyze it again against the global interfaces (deterministic), lower the needed functions into the shared cache, drop it. An LRU keeps up to 32 MB of rehydrated units.
- **`fa check` for several targets** (today `packages/cli/src/lib.rs:344-375` loads and analyzes the whole project again per target): for each extra target, analyze again only the units with `reads_target`, plus the dependents whose interface hash changes. Same scheduler, one target at a time.

### 3.2 JavaScript

- `generate_module` already takes `&Session` (`codegen-js/src/generate/mod.rs:229`). It runs in the unit's worker right after analysis.
- The module is written straight into `<out>.partial/<path>.js`, parts written directly with no `format!` concatenation (`codegen-js/src/lib.rs:301-313`).
- Folders are created once from the scan.
- The helpers used are folded into a `Mutex<BTreeSet<&'static str>>`.
- `$runtime.js` and `main.js` are written after the scope, using the initialization order from the interfaces.
- Any error in any unit: generation stops for the remaining units, analysis continues to report every error, and `.partial` is deleted. Without errors, it is renamed. `write_run_script` already does this (`cli/src/lib.rs:900-925`).

### 3.3 LLVM

**Today** one standalone file is compiled into one module (`codegen-llvm/src/compile.rs`); several files are refused (`cli/src/lib.rs:712-721`).

**Per unit:**

- `Context::create()` and one module inside the job (`Context` is `!Send`, so it is created and dropped in the worker).
- Lower, verify, run the module pipeline (`-Oz` friendly), write `obj/<module>.o`, or `.bc` for ThinLTO.
- LLVM targets are initialized once, before the pool.
- At most `min(cores, budget / 96 MB)` LLVM jobs run at once.

**Symbols and program structure:**

- Symbols are mangled from `DeclKey`: `fa.<module path>.<names>[.<disambiguator>]`, hidden visibility. Module values become globals with an `$init` function.
- The entry object holds `main`: it runs the `$init`s in initialization order, calls the entry function, and drops the module values at the normal end.

**Runtime:** the C runtime IR is compiled once per (target, options) into `runtime.o`/`.bc`, cached, instead of being linked into each module as `internal` (decisions.md, "Tier 1 runtime").

**Linking:**

- **Release**: ThinLTO (`ld.lld --thinlto-jobs=N`, `wasm-ld --thinlto-jobs`) restores cross-module inlining and removes dead code, keeping the size goals. `--lto=full` is the option for the smallest binaries; memory then grows with the program, but only for a final build.
- **Development** (`fa run --target=native`): plain objects with function sections and `--gc-sections`, cached per module under the key (source hash, dependencies' interface hashes, options).

**Generic instances** (decisions.md: monomorphized per representation):

- Lowering a call to another module's generic declaration emits an external call to `mangle(DeclKey, representation key)` and records a request `(DeclKey, representation key, global type arguments)`. The arguments are merge roots (§1.3).
- After the first wave, requests are grouped by defining module and deduplicated.
- Each module with new requests is parsed and analyzed again (deterministic, against the interfaces), or reused if still resident. Its instances are emitted into `obj/<module>.inst<k>.o`.
- New requests made by instances start the next wave, until none are left.
- Instances of a generic declaration in its own module are emitted in the first wave, as today.

### 3.4 Language server

- Direct mode: ids are global, so queries need no per-file view, and `Session`'s query API is unchanged.
- **Initial load of a package**: the same scheduler, in parallel.
  - Open documents keep their tree and analysis.
  - Closed files keep only their interface. Their trees are dropped, unlike `analyze_package` in `language-server/src/analyzer_backend.rs:62-73` today.
  - Go-to-definition into a closed file uses `ExportedDecl.name_span`.
  - Opening a file analyzes it again.
- **Edit**:
  - Analyze the changed file again.
  - Compare its stable interface hash.
  - Dependents come from the reverse edges and are analyzed again in parallel by DAG order, with cancellation between units.
- **Store growth**: the epoch reset of analyzer-design §4 stays.

## 4. Hot loops: passes to fuse, copies to remove

| # | Where | Today | Change |
|---|---|---|---|
| 1 | `check/modules.rs:142-147` | `start_modules` scans every node for namespace blocks | `FileScan.namespace_blocks` (parse time) |
| 2 | `check/modules.rs:461-476` | `import_names` scans every node and allocates `String`s per lookup | `FileScan.names`, resolved once at the scan (edges) |
| 3 | `check/reactivity.rs:301-308` | scans every node for reactivity | `FileScan.has_reactivity` |
| 4 | `check/modules.rs:155-163` | `finish_modules` rewrites every resolution in a second pass | write `Resolution::External(DeclKey)` when the proxy is resolved (dependencies are done, so the declaration is known) |
| 5 | `program.rs:190-232` | `file_dependencies` scans all resolutions per file; `dependency_order` calls `files.contains` (`:220`), which is O(n²) | the module graph (§2.1) |
| 6 | `modules.rs:410-433` | `Interface::hash`: `format!` and `types.display` strings per member | fold `stable_hash` (u128) and the kinds, once at publish |
| 7 | `types/store.rs:640-643`, `:727-735`, `:761-767` | `set_payloads`, `set_rec_target`, `set_param_bounds` clear the whole `assign_cache` (once per type declaration) | P9: don't cache results that involve incomplete types |
| 8 | `program.rs:263-407` | the initialization order walks the trees of every file again after analysis, and clones `used` vectors (`:303-306`, `:397-400`) | module reads recorded by the walk per module-level declaration (`FnSummary.module_reads`, analyzer-design §2.5) in `interface.init` |
| 9 | `program.rs:93-112` | `settle` resets and re-walks every analyzed file when one is deferred | only the unit's files |
| 10 | `program.rs:538-550` | `update` scans every file per changed module | reverse edges |
| 11 | `check/modules.rs:262-266` | linear scan of `proxies` per method lookup | `FxHashMap<(FileId, NodeId), SymbolId>` |
| 12 | `check/modules.rs:324`, `:333` | clones another file's `FnInfo` and `Symbol` per call | `ExportedDecl.params` (borrowed) |
| 13 | `codegen-js/src/generate/modules.rs:225`, `:229` | `externals.clone()` and `namespace_decls.clone()` **per file**: O(files × exports) | one shared `&` lookup into the interfaces |
| 14 | `codegen-js/src/lib.rs:256-264`, `:335`, `:313` | clones every error up front; `files.contains` in the initialization loop; collects all modules | error flag per unit; `FxHashSet`; streamed writes |
| 15 | `cli/src/project.rs:329-339`, `:277-286` | `write_modules` after collecting everything; `rendered()` builds one big `String` | the worker writes; reorder buffer to `BufWriter<Stderr>` |
| 16 | `session.rs:523-525` | `convert_all` builds a `LineIndex` (a scan of the text) per file even without diagnostics, duplicating the tree's `SourceMap::new` (`parser/src/typed_syntax_tree.rs:25`) | reuse `tree.source_map`; skip when there are no diagnostics |
| 17 | `session.rs:156-162` | `file_of_module` parses lazily, on the analysis thread | parsing in the scan phase or the unit job |
| 18 | `intern.rs:22-37` | `HashIndex` = `FxHashMap<u64, SmallVec<[u32; 1]>>`: two hashes per lookup, a `SmallVec` per bucket | `hashbrown::HashTable<u32>` with the precomputed hash |
| 19 | `cli/src/lib.rs:344-375` | the whole project analyzed again per extra target | §3.1, units that depend on the target only |
| 20 | `codegen-js/src/generate/references.rs:648-665` | a full node scan for reference roots per file | the analyzer records `use`/`reuse`/`provide` nodes in a side list during the walk |

Rule for review from now on: a pass over all nodes or all resolutions outside the walk or the generation walk needs a written reason. Side tables are written once per node, and nothing is cloned per file in a loop over files.

## 5. Milestones

Each step can be merged on its own, keeps every existing test green, and is measured with the step 0 harness. Order: 0 → 1 → 2 → 3 → 4 → 5. Step 6 needs 2 (it is parallel after 4 and bounded after 5). Step 7 needs 4.

### Step 0: harness and baseline (small)

- **Generator** `benchmarks/compiler/generate.ts` (Bun, seeded): N files × L lines.
  - Mix: types, unions, generics, loops with facts, `when`, errors, a few `static`s.
  - Each file imports 3–8 earlier files: DAG depth about 20, plus one 3-file cycle.
  - Options: share of target-dependent files, error injection.
- **Runner** `benchmarks/compiler/run.ts`: runs `/usr/bin/time -v` on `fa check`, `fa build --target=bun` and later `--target=native`, at 100, 1,000 and 2,000 files of 500 lines.
  - Records wall time, **CPU utilization** ((user + sys) / wall) and **peak RSS** into `benchmarks/results/compiler-<date>.json`.
- **`--timings`** on `fa check`/`fa build`: phase times (scan, analysis, generation, write, link), unit count, largest unit, peak units in flight, global store sizes, retained tree bytes.
- Record the baseline. Expected: CPU ≈ 1 core, RSS above about 500 MB at 1,000 files (to confirm).

### Step 1: parallel parsing, parallel JavaScript generation, streamed writes

- Add `rayon` to the analyzer, the CLI and `fa-codegen-js`. `ModuleResolver: Send + Sync`.
- `Session::load_modules(&[String])`: reads, parses and converts parse diagnostics with `par_iter`, then inserts in sorted order (deterministic `FileId`s). `analyze_package`/`analyze_program` call it with every module of the codebase.
- `generate_program` (`codegen-js/src/lib.rs:239`) takes `&Session`:
  - `files.par_iter()` → `generate_module` → write into `.partial/`.
  - One shared `ProgramCx` table (§4 #13, #14), helpers folded.
  - `main.js` and `$runtime.js` written last, then the rename.
- Measure: parse and generation phases at ≥ 0.7 × cores. RSS unchanged.
- Test: the build output with `RAYON_NUM_THREADS=1` is byte-identical to the default.

### Step 2: interfaces and facts recorded by the walk (still sequential)

- `ModuleInterface` (§2.2). `Resolution::External(DeclKey)`.
- Every read of another file's `Analysis` or tree goes through interfaces: the analyzer, compile-time lowering, the JavaScript `program_contexts`.
- `FileScan` built at parse time (§4 #1–3). Direct `External` resolutions (#4). Walk-recorded module reads and initialization order from interfaces (#8). Stable interface hash (#6). Module graph and SCCs used for `dependency_order`, `settle` and `update` (#5, #9, #10). P9 cache rule (#7). Fixes #11, #12, #16, #18.
- **Invariant test**: analyze a package, drop the trees and analyses of every file except one (`Session::forget_tree`), then generate that file and compute the program's initialization order. Both must succeed. This proves that modules are read only through interfaces.
- Measure: single-thread analysis time, expected −10 to −20%.

### Step 3: concurrent `GlobalStore` (Direct mode) and the `Session` split

- `GlobalStore` (§1.2): segmented arenas, sharded index, write-once slots, deterministic `names`. `TypeStore` becomes a view (Direct only). `Shared`/`UnitSession` split with the `Session` facade. Shared compile-time cache.
- **Canonical union order by `stable_hash`** (`types/union.rs:39`, `:63`): changes some message and display orders once, and expected outputs are updated in the same change.
- Analysis is still one unit at a time.
- Measure: single-thread time within 5% of step 2.
- Tests:
  - 16 threads intern overlapping random structures and must get the same id for each structure.
  - Shuffling the order of modules leaves outputs unchanged.

### Step 4: parallel analysis by the module DAG

- Scheduler (§2.3) with units = SCCs, the priority ready queue, the reorder buffer, 64 MiB worker stacks (P8), and JavaScript generation fused into the unit job.
- **P10 test**: the end-to-end corpus and the generated project with threads 1, 2 and 8 × 3 seeds. Diagnostics and outputs must be byte-identical.
- Measure: analysis phase at ≥ 0.7 × cores on 1,000 files. `fa check` and `fa build` wall time.

### Step 5: memory-bounded streaming (`fa check` < 512 MB)

- Overlay mode with merge and remap (§1.3) and the "no local id in the global store" assertion.
- Tree budget with parse again on ready. Drops (§3.1). Compile-time rehydration. Multi-target check by dirty units (#19). mimalloc.
- Measure: peak RSS at 1,000, 2,000 and 4,000 files must stay < 512 MB, and its slope must match the size of the interfaces (about 50 KB per file at most). Wall time within 5% of step 4.
- Tests:
  - An ignored test in the CLI generates 2,000 files, runs `check` in-process and asserts `getrusage().ru_maxrss` < 512 MB.
  - P10 again, plus Overlay vs Direct: identical outputs.

### Step 6: LLVM per module, in parallel, then linked

- §3.3: mangling, the entry object, initialization and drops, the runtime object, per-unit contexts, instance waves, ThinLTO for release, cached objects for development. This removes the single-file restriction (`cli/src/lib.rs:712-721`).
- Measure:
  - Native build wall time, CPU utilization and RSS on the generated project.
  - The benchmarks (single file): size and run time unchanged within ±2%.
  - A multi-file program: ThinLTO size within 3% of `--lto=full`.
- Tests: the multi-file end-to-end programs run native and wasm. Objects are byte-identical across thread counts.

### Step 7: language server

- §3.4: parallel initial load, closed files kept as interfaces only, parallel re-analysis of dependents, reverse edges.
- Measure: memory on a 1,000-file package with 5 open files; latency after editing a leaf module and a core module.

### Implementation status (steps 1 to 5, 2026-11-02)

Done, with these choices made while implementing (results: `benchmarks/README.md`, "After steps 1 to 5"):

- **One mode of the store** (accepted by the lead): every view interns new structures locally (bit 31) and reads the global store; the global store is written only by the merge of a finished unit (`types/merge.rs`, one merge at a time) and by the predefined types. A single session (the language server, tests, `fa run`) is a view whose types are all local. So no sharded concurrent interning: the global index is asked once per structure and view.
- **Determinism without ordered ids**: unions are ordered by `TypeStore::canonical_order` (predefined types, then string and integer literals by value, then stable hash); the stable hash no longer reads path or effect ids; the names of global object types are shown from the modules a unit may read (smallest path); method candidates of a structure (`type_origins`) are restricted to the modules a file may read.
- **Steps 4 and 5 are one commit**: a unit's session is created, analyzed, published, generated and dropped by its worker (`pipeline.rs`), so streaming is the shape of the parallel run. `fa check` and `fa build` of a package use it; a standalone file, a script, `fa run`, `fa test` and the language server keep the sequential analysis (`FA_PIPELINE=0` forces it, to compare).
- **Diagnostics**: each module's sorted diagnostics are kept (not the reorder buffer of rendered strings) and rendered in path order at the end, with the program checks (`MOD007`, `MOD015`, scan problems): the output is byte-identical to the sequential analysis.
- **The module reads of the initialization order** are collected by one pass over the tree at the end of its walk (`check/interface.rs`), not by the walk itself (§4 #8).
- **Compile-time code** reading another unit's function loads that module again into its unit's session (`Session::rehydrate`, analyzed against the published interfaces), without an LRU. One evaluation thread (1 GiB stack) ran at a time in the process (until the pass below).
- **Contention removed (pipeline performance pass)**: merges run concurrently. The global index is sharded open addressing whose lookups take no lock (`types/global.rs`: atomics, tables replaced by larger ones and kept until the store drops); an insert locks one shard of one table, so two merges of one structure give one id. A recursive reference is reserved under the lock of its shard of the late-bound data before its target is merged; a concurrent merge of the same reference shares it and waits for its target before its unit publishes (`types/merge.rs`). The late-bound data and `Shared.origins` are sharded; the origins of a type are kept sorted by module path, so `type_origins` reads them lazily and stops at the first that fits. Evaluations run on a pool of reusable large-stack threads (up to one per core, sharing a quarter of a limited address space), and the memory budget of an evaluation counts what its thread allocates (`fa_interpreter::memory`), so parallel evaluations don't charge each other. A unit releases its dependents once its interfaces are published, before its sink and teardown. `--timings` prints the use of the workers (busy, idle, and on Linux on a core, waiting for one, blocked).
- **mimalloc (P8) was measured and dropped**: higher peaks than glibc's allocator.

Not done: the walk-recorded caching rule P9 (§4 #7: the whole-cache clears remain, per view); `Resolution::External(DeclKey)` (external resolutions still name `(module, node)`, which is stable within a run; it matters for step 7); the reverse edges in `Session::update` (§4 #10, step 7). The three leftovers are done in step 7 (below).

### Implementation status (step 7, 2026-11-02)

- **`Resolution::External(DeclKey)`** (P5): a declaration other files name gets its key from its symbol (`Walker::symbol_key`, `check/modules.rs`): the module, the names of the enclosing namespaces, types and functions, its own name, and a disambiguator counted in the order the walk asks for keys. Every interface maps its keys to the current node and spans (`Interface::keys`, `KeyedDecl`); `Session::external_node` and `Session::keyed` follow a key to the current version of the file (the session's file, else the published interface). `Declared::uses`, `Interface::uses`, the initialization order and the origins of global types (`Session::type_origins`) are keys too. The interface hash no longer reads nodes: a declaration that only moves keeps its key, so its readers are not analyzed again and their resolutions stay valid.
- **P9** (§4 #7): a relation that read an incomplete type (payloads not set, a `Rec` without its target, a parameter declared with `add_param_pending` whose bounds are not set) is not cached; the three whole-cache clears are gone.
- **Reverse edges** (§4 #10): the session keeps the readers of each module from the scans, updated as versions are added; `Session::update` walks them unit by unit in dependency order, analyzes a unit again only when a module it reads changed its interface, and stops between units on the cancel flag. A new module still looks at the files whose lookups of its name failed.
- **Measured** (`benchmarks/README.md`, "After step 7"): on 1,000 files of 500 lines with 5 open documents, the language server peaks at 137 MB; an edit of a leaf module takes 167 ms, of a core module 129 ms (body) and 311 ms (interface); `fa test` takes 9.8 s and 137 MB (sequential: 19.4 s, 806 MB).
- **Language server** (§3.4, `language-server/src/analyzer_backend.rs`): a package is loaded with the parallel pipeline; the sink keeps the sessions of the units holding an open document (`UnitSink::keep`), every other unit is dropped, so closed files are interfaces only. The open documents' texts replace the disk's through `OverlayResolver`. An edit or an open runs the pipeline incrementally (`Options::dirty`: the dirty modules are scanned and analyzed again, a unit whose dependencies' interface hashes didn't change keeps the previous run's interfaces and diagnostics), in parallel by DAG order, stopping between units on the cancel flag (`Report::cancelled`: the run is dropped and its modules wait for the next one). After a run, every kept session reads the newest interfaces (`Session::set_shared`). Go-to-definition into a file whose tree is dropped uses the spans of its key. The package's dependencies are resolved like the command line (`fa::editor_session`, the lockfile read, never written). The epoch reset reloads the package when its global store has grown four times.
- **`fa run` and `fa test` of a package** run on the pipeline (`streaming::Purpose`), each module written by the thread that analyzed it into the cache, then run with Bun. A standalone file, a script, `fa develop`'s watched tests and a package that configures its asynchronous mode (`--asynchronous`, `build.asynchronous`: their checks read the whole program) stay on the sequential analysis.

### Implementation status (step 6, 2026-11-04)

`fa build` and `fa run` of a package for `native`, `native-freestanding` and `wasm` compile it per unit (`fa_codegen_llvm::compile_package`, the sink of the pipeline; `codegen-llvm/src/units.rs` and `lower/units.rs`). A standalone file stays one module, so the benchmarks' executables are byte-identical to before.

- **Per unit**: once the run is over (the sink only records the units), the units holding what the entry reaches are analyzed again, alone, against the published interfaces (`pipeline::Reanalyzer`, from any thread) and lowered in parallel, each in an LLVM context of its own whose module starts as the declarations of the tier 1 runtime (parsed once per process from bitcode). At most `min(cores, FA_LLVM_MEMORY / 96 MB)` LLVM jobs run at once (512 MB by default: 5).
- **What the entry reaches** (follow-up, decisions.md "Native builds per unit, follow-ups of step 6"): walked before any LLVM work from the call graph of the interfaces (`Declared::uses`: what a declaration names or calls directly), from the entry's uses and its entry function; a type reached reaches its functions (methods called through a constraint or a function value), a namespace's members are reached one by one, and in an asynchronous program a function that may suspend reaches its async instance. A unit lowers its reached declarations, its module values reached, and what they call (instances, private functions). Each unit reports the symbols it exports and the symbols of other units it uses: one another unit doesn't export (the walk missed it) makes that unit compile again, reaching it (a safety net; none in the corpus or the generated packages).
- **Reading other units**: through their published interfaces only. A function of another unit is declared from its type in the interface (parameters by pointer from its effects, borrowed aggregates, defaults from `Interface::parameters`, requirements, `throws`) with its symbol, a module value as an external global.
- **Symbols** (as decided): `fa.<module path>.<names>[#<disambiguator>]` from the `DeclKey`, hidden; everything else is internal. Module values: `fa.init.<module>.<node>` per declaration (the cleanups it leaves are the values, dropped by `fa.drop.<module>.<node>`); the entry file's top level and entry function: `fa.main.<module>`, which returns the exit code.
- **The entry object** (`main`, `_start` on wasm): the stack guard, the arguments, the `fa.init`s in `Report::init_order`, `fa.main`, the `fa.drop`s in the reverse order when a drop does more than free memory or the leaks are counted (`fa.drops_at_exit`), the leak check, the flush and the exit code.
- **Unions**: in a program compiled per unit, the id of a member is a stable hash of its union's `DeclKey` and type arguments and of its index (of a member type of an anonymous union: its stable hash), the same in every unit; each unit renames its `fa.drop_box`, `fa.copy_box` and `fa_format_union` switches after itself, and the entry object's switches call, for each id, the first unit that registered it. The built-in errors are registered by the entry's unit only.
- **The runtime object**: the tier 1 runtime and the platform layer, compiled once per target and options (`~/.cache/fa/native/runtime/`, keyed by the runtime's IR, the target and the options) instead of being linked into each unit.
- **Instance waves** (§3.3): a call of another unit's generic function declares the instance `<symbol><stable hashes of the type arguments>` and requests it; its type arguments are merged into the global store from the sink (`TypeStore::merger`). After the pipeline, each declaring module is analyzed again, alone, against the published interfaces (`pipeline::Report::reanalyze`) and lowers the requested instances, and internal copies of the private functions they call, into `<module>.inst<k>`; the requests of these instances make the next wave. A call from inside an instance whose type arguments name the instance's type parameters requests the types the instance was made for (noted by representation as instances are made); a composite of them (`Array(Item)`) is substituted in the global store (a scratch view interns, the merge publishes), in one module too.
- **Linking**: `fa build` (release) writes ThinLTO bitcode (the module summary is written by `shim/thin_bitcode.cpp`, the C API has no writer for it; the module hash names the promoted locals), the entry object too, and links with `ld.lld --thinlto-jobs` / `wasm-ld --thinlto-jobs`; the functions carry the target's CPU and features for the link's code generator (wasm: `memory.copy`, not `memcpy`). `FA_LTO=full` writes plain bitcode instead: whole-program LTO. `fa run` (development) writes plain objects with one section per function, linked with `--gc-sections`, and keeps each unit's object in `~/.cache/fa/native/objects/`, keyed by the unit's sources, the hashes of the interfaces it reads, the entry and the options: a unit that didn't change isn't compiled again (an object with instance requests holds types of its run, and isn't cached).
- **Full LTO when it fits** (decisions.md, "Full LTO when it fits the memory budget"): once the units are compiled, a full LTO link is estimated at 80 MB + 18 × their bitcode (measured: 100 generated files 1.96 MB → 116 MB, 1,000 files 16.5 MB → 373 MB, 50,000 lines of functions 1.69 MB → 112 MB). Within `FA_LLVM_MEMORY` (512 MB) the units are written again without their summaries and linked as one module; above it with ThinLTO. `FA_LTO=full|thin` forces one, `--verbose` prints the choice and the estimate. The runtime stays ThinLTO bitcode under full LTO: in its own partition its functions aren't inlined into the program's (444 KB against 590 KB merged, 100 files).
- **Asynchronous programs** (follow-up): every unit lowers in the program's mode (configured, else detected from the entry, A6). An async instance of another unit's function is declared as `<symbol>.async`, defined and exported by its unit; the entry's top level is the coroutine `fa.main.<module>` (`fa.exit_status` exported), which the entry object runs on the event loop (`fa_run_main`, or the host's on wasm: `fa_run` and `fa_wake`, kept by `--export-if-defined`); the entry object defines `fa_resume` and `fa_destroy`.
- **Module values** whose cleanup is a resource's drop flag (a `destroy`): the flag moves from the init function's frame to a global, read by `fa.drop.*`.
- **Falling back to one module**: what the units don't compile yet (`Error::Unsupported` in a unit: a module value whose cleanup is neither a global nor a resource, none in the corpus; `--verbose` names it), and a package whose asynchronous mode is off by choice (`--asynchronous=false`, `build.asynchronous = false`: CONC010 and WCONC001 read the whole program). `FA_SPLIT=0` forces it.
- **Huge top levels** (`lower/chunks.rs`): above 64 KB of source, the top level of the entry file is lowered into functions of about 16 KB each (`noinline`), which `main` calls in order; a top-level local read by a later part is a global, what a part's own locals own is dropped at its end under the rule of the drops at exit. Not with reactivity, `provide`, `mutable use` or the asynchronous mode.
- **Determinism**: the objects of a package (development) and its executables and `.wasm` modules (release) are byte-identical with 1 and 6 threads and another `FA_SCHEDULE_SEED`.
- **Tests**: `codegen-llvm/tests/programs.rs` compiles every package of the corpus per unit too (with leak checks, on the three targets), and `tests/programs/units.fa` must compile per unit (functions, methods, defaults, module values, unions and errors across units, instance waves). `tests/programs/units-async.fa` is an asynchronous package; a package of `tests/programs` that falls back is a failure (`FA_SPLIT_TRACE=1` prints the rounds and the fallbacks).

### Whole-program facts in builds per unit (2026-11-04)

Some layouts depend on what the whole program does with a type (dense maps: nothing keeps a reference into the values of maps of that type, `design/decisions.md`). A program lowered as one module computes them from all its files; per unit, no session sees every body, so the facts go through a small typed registry, `fa_codegen_llvm::facts` (`packages/codegen-llvm/src/facts.rs`):

- **A fact kind** is a `FactKind { name, collect, merge }`: `collect(&Session, &[FileId]) -> BTreeSet<String>` is what one unit's analyzed files contribute, `merge(&mut BTreeSet<String>, BTreeSet<String>)` folds a contribution into what is merged so far (`facts::union` for a "may" fact). Kinds are listed in `facts::KINDS`; a new user adds a constant name and an entry there.
- **Keys are stable names**: the same string in the session of every unit. `facts::stable_type_name(types, ty)` spells a type canonically: a declaration by its identity (`DeclKey`: `fa.<module path>.<names>[#n]`, as the symbols of units, then its type arguments), whatever a unit imported or aliased; an anonymous structural type from its parts (an object's fields sorted by name, a tuple's items, a union's members sorted, a named union's or enumeration's members held by index); built-in scalars as Fa writes them; a recursive reference as `^n` (n levels up); a function type by its parameters (variadic marked), result, sorted `throws` members, requirements, effects (paths rooted at parameters by position, sorted), stored callbacks and async-ness; a type parameter by its position `$i` with its constraint, so that types equal up to the renaming of their parameters share a name; a task by its result and errors. `None` only for references, iterators and the like: a consumer treats them conservatively.
- **The driver** (`units.rs`): `UnitRecorder::unit`, the analysis' unit sink, calls `ProgramFacts::of_unit(session, files)` for every unit (every unit is analyzed in a build) and merges it; `compile_package` then puts the merged `Arc<ProgramFacts>` in `SplitOptions::facts` for every lowering it starts (units, instance waves, the entry), and `options_key` hashes it into every object's cache key (`ProgramFacts::hash_into`): a change of a fact anywhere recompiles the objects, never reuses one built with another layout. Deterministic: sets of strings, merged in any order to the same result.
- **The lowering** reads them with `Lowering::with_program_facts` (`self.program_facts`, `ProgramFacts::contains(kind, key)`); a program lowered as one module (`with_whole_program`) keeps its own, more precise, facts by `TypeId` (`program::unsafe_maps`).
- **Users**: dense maps (`facts::REFERENCED_MAPS`, `facts::DELETED_WHILE_ITERATED_MAPS`); next, the index proofs of fields. The tables of generic function values are not facts (some of their instances are only known once generic code is instantiated): the driver defines them after the instance waves (`design/llvm-backend-plan.md`, "Generic function values").

## 6. Risks

| Risk | Mitigation |
|---|---|
| A hidden dependency on id order: `FxHashMap` iteration over ids that reaches output, sorts by id | P3 rule, the P10 test from step 3 on, a review of `iter()` over maps keyed by ids in both backends |
| The scan's edges miss something the walk reads (a dependency not finished yet) | The scan is the analyzer's own import collection (§2.1). Debug assertion. In release: `UNKNOWN` plus an internal `AN` diagnostic rather than a race |
| A large SCC (everything imports everything) runs serially and holds its whole memory | Correct, just slower. `--timings` reports the largest unit. Declaration-level analysis across the files of a unit is a later improvement |
| The remap misses a table, leaving a dangling local id in the global store | One remap visitor per id type, with the bit-31 assertion on every global insert. Step 5 also runs P10 Overlay vs Direct |
| Stack overflow on worker threads | 64 MiB stacks (P8); the deep-nesting tests run on the pool |
| Parsing again beyond the tree budget costs time | Measured. The budget can be configured. Parsing is a small share of analysis cost |
| Instance waves analyze generic-heavy libraries again | Reuse units that are still resident; group all requests per module per wave; measured in step 6 |
| LLVM memory per context × threads in native builds | A separate limit on LLVM jobs (§3.3). Plain objects in development |
| ThinLTO binaries larger than one module | `--lto=full` option; size gate in step 6 |
| freed memory not returned to the system (allocator fragmentation) | mimalloc (P8); RSS measured, not heap size |
| Churn in the `Session` API for the language server and tests | The `Session` facade keeps the query API. Step 3 changes types, not behavior |

### Critical Files for Implementation
- `/home/claude-dev/projects/Fa/packages/analyzer/src/types/store.rs`
- `/home/claude-dev/projects/Fa/packages/analyzer/src/session.rs`
- `/home/claude-dev/projects/Fa/packages/analyzer/src/check/modules.rs`
- `/home/claude-dev/projects/Fa/packages/analyzer/src/program.rs`
- `/home/claude-dev/projects/Fa/packages/codegen-js/src/lib.rs`
