# Proofs engineer: handover (2026-11-05)

## Done and committed (branch `worktree-agent-aca08cbb0b8394a82`, on `compiler` b06e4f5)

- `c5b9c8a` JS package builds use the program's proofs (`packages/cli/src/streaming.rs`):
  `BuildSink::unit` collects `proofs::unit_claims_of_goal` (one goal: the build's overflow checks)
  and writes at once every module for which `fa_codegen_js::reads_program_claims` is false; the
  others are deferred, then `BuildSink::generate_deferred` resolves the claims, re-analyzes their
  units (`Report::reanalyzer`, worker pool) and generates them with `ProgramProofs::Claims`.
  `FA_DEFER_UNITS=1` defers everything (test `a_re_analyzed_unit_generates_the_same_javascript`
  in `cli/tests/packages.rs`). Measured on the 1,000-file package (`bun benchmarks/compiler/generate.ts
  --files 1000`): user 22 s -> 25 s, peak RSS 140-165 MB before and after (every unit deferred: 37 s,
  same peak).
- `a1b2996` Async natively: removed the `self.tasks.asynchronous` early return in
  `codegen-llvm/src/lower/index_proofs.rs` (`compute_index_facts`); it came with 734d5d5 with no reason.
  Soundness: claims are flow-insensitive, a proven array never shrinks anywhere. Tests (both harnesses):
  `cli/tests/programs/proofs-async.fa`, `proofs-parallel.fa` (native only), `proofs-worker.fa`.
- `e9060d2` Implementation note under "JavaScript package builds use the program's proofs…" in
  `design/decisions.md`.

## Test state

`cargo clippy --workspace --all-targets`: clean. Full `cargo test --workspace` (LLVM on) was
interrupted by the pause: everything that had finished passed (cli packages 29/29, LLVM native and
freestanding programs ok, analyzer 350 ok); not seen: LLVM wasm programs and the crates after it.
Run it again first.

## Next actions

1. Rerun the full test command (see the brief); fix anything new (`FA_UPDATE_SKIPS=1` only if a skip changed legitimately).
2. Index proofs for packages (not started). Finding: on JavaScript the never-negative places only turn
   a `FromEnd` index direct (`codegen-js/src/generate/numbers.rs::index_access`), and the analysis
   clears an index check only when it knows the sign, so per-unit claims almost never change JS
   (0 deferred modules in the benchmark). The index proofs (`proofs.rs::proven_indices`, Checked -> Direct)
   are what matter, and they are single-file. Plan for a sound per-unit version:
   - in a unit, run `proven_indices` for an array `X` that is not exported, whose `array_family`
     params belong to functions other files can't call (module-level non-exported, or nested);
   - a new line kind in the unit claims naming the module that stores to each typed place
     (`field:T.f`, `items:ty`, `values:ty`, the `*` forms, `fields` shapes), resolved into
     "only module M stores to key K"; a Field/Items/MapValues claim of the Index goal holds only when
     F's own stores prove it and no other module stores to that key, no `field:*.f`/`items:*`/`values:*`
     from another module, no wider shape from another module, no `opaque`;
   - `ProofScope::Unit` then also fills `proven`; `fa_codegen_js::reads_program_claims` must also
     defer a file with a `Checked` index proven optimistically; natively `compute_index_facts` per unit;
     also give whole-program multi-file native builds (`ProofScope::Nothing` today when
     `other_files` is non-empty) the claims of all files, as JS `program_claims` does.
   - tests: a package fixture where another file pushes an out-of-bounds literal of the type (check
     stays) and one where nothing else touches it (JS output has no `faIndex` for that access).

## Good to know

- Commands with `$VAR` paths or loops are refused by the worktree guard: use literal paths, or a
  script file in the scratchpad run with `bash`.
- `-- skip-target:` reads every word of the line, a parenthetical included: don't write a target
  name in the explanation (`(… native only)` skipped native).
