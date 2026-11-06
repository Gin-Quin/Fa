# Root-region proof handoff (proofs/arena-regions, 2026-11-06)

Base: `compiler` at `ef777e4`. The worktree is isolated; root keeps its central NEXT.md when integrating.

The semantic scanner is `fa_analyzer::regions`. Unit claims require an exact manifest of every scanned module; a missing/opaque contribution rejects all regions. Stable type/declaration identities and builder/child-array facts feed ROOT_REGIONS in the native object cache. Whole-module and per-unit lowering use those same facts. The backend's old local semantic scanner is removed; target layout guards remain. Per-unit lowering now prepares arenas after indexing/captures and wraps only reached builders. Cache version: split-regions-1.

Twelve semantic tests cover complete graph/order/thread invariance, real sexp, root/child arrays, recursive objects, unrelated payload records, temporary pattern subjects, foreign entries, unresolved payloads, aliases and outgoing array writes. Only a fresh local array (or one at module level when the call is also there) accepts push/reserve; parameters, captured/global arrays from functions and mutating array helpers reject the proof. Native split cache regression runs both before/after a remote escape with leak checking and confirms the builder is rebuilt while an unrelated unit is reused. The multi-file region-roots.fa exercises independent forests and detached subtrees on all LLVM targets.

The two initial API/backend patches were given to native_perf. The final scanner snapshot is /tmp/fa-root-regions-final.rs; /tmp/fa-root-regions-unit-init.patch adds lower_unit setup, together with the is_reached guard in prepare_tree_arenas. native_perf owns array-bearing physical layouts, child-buffer cleanup and compact union payloads; the scanner's positive semantic fact alone does not turn those layouts on. No new ABI or compressed-link address-band change is introduced here.

Validation: the final full workspace suite with LLVM finished with exit0 (session88538, /tmp/fa-arena-workspace-corrected-full.log), serialized by flock --close /tmp/fa-compiler-full-suite.lock, two cargo jobs/test threads, ulimit -v 8000000. The first run failed only on the obsolete disjoint-array small-offset throws fixture; the already-integrated0290400 version is copied here and all45 LLVM checks passed serially before the final full. Additional logs: /tmp/fa-arena-semantic-final.log (12/12), /tmp/fa-arena-object-cache-leaks.log, /tmp/fa-arena-foreign-roots-fixed.log (three targets). Final clippy is /tmp/fa-arena-clippy-delivery.log.

No documentation/ edits. The technical decision and a note for the memory page are in design/decisions.md and design/pending-documentation.md.


Capacity proof follow-up (proofs/arena-capacity, based on delivered 1a0d6b): adds the positive `regions::capacity_unobserved_key(types, array)` fact inside ROOT_REGIONS using the existing semantic walk and exact manifest. Any capacity read for the root family removes it; incomplete/opaque graphs keep no permission. Native_perf owns physical buffer compaction. Targeted semantic and remote-cache tests precede fmt/clippy/full. No documentation/ edits.

Capacity delta frozen for full validation: regions14/14 and FactLog remote-capacity invalidation pass; workspace clippy has no warnings. Full log /tmp/fa-arena-capacity-workspace-full.log; jobs2, memory8GB and shared lock with --close. No JS runtime change in this delta.

Capacity follow-up validated: full workspace LLVM suite exit0 (session66527, /tmp/fa-arena-capacity-workspace-full.log); no errors or warnings. Semantic14/14, remote FactLog positive/negative reuse and clippy green. Only native_perf should connect the exact-length physical representation, consulting the positive key. Preserve lead NEXT on integration.
