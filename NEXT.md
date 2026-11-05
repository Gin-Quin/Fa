# Analyzer engineer 2: state at pause (2026-11-05)

## Done and committed (on top of `compiler` 35433d0)

- 2df2793 Lengths bounded by 2^48 (`types/int.rs` `LENGTH_MAX`/`lengths()`; `check/numbers.rs` `counts_elements`, wrap rule for every width, overflowing nodes record their type range; `check/control.rs` for-positions; `check/ops.rs` `square_root_bound` for `i <= n // i`; `lower/index.rs`). Cases `review/length-bound.fa`, `review/release-wrap-64.fa`. JS golden updated (fewer checks). Bun + LLVM (native, freestanding, wasm) pass for `review/`.
- 470717a Callbacks that throw in user functions (`check/callbacks.rs`: `only_called`, `calls_passed_callback`; `calls.rs` `BindParam::passed`; interface `Parameter::only_called`; EF018 in `errors.rs`; `NodeFlags::PASSES_ERRORS`). Bun passes. Natively: `codegen-llvm/src/lower/calls.rs` returns *unsupported* for PASSES_ERRORS calls (it miscompiled before) -> the LLVM expected skips must be regenerated (`FA_UPDATE_SKIPS=1`) for `review/callback-throws*.fa` and `fp-4.fa`.
- 5b1ff73 Lambda arguments see the callee's own effects (`effects.rs` `callee_effect_places`, `calls.rs`); REF013 for module reads of callees of other files (`effects.rs` `check_external_read_aliasing`). Cases `review/lambda-after-callee-effects.fa`, `review/module-value-alias-files.fa`.
- 98f0aa0 W-DECL-2 unused values (`check/unused.rs`). The case harness checks it only in files naming W-DECL-2; unit-test helpers filter it. **Not yet run: other crates' tests (language server, cli) may assert "no diagnostics" and now see W-DECL-2.**
- 466d71d CV002, CL025, EF012 (object literals only), E-ERR-8 (`tasks.rs` `check_await`). Case `registry/documented-rules.fa`; cv-7, pa-15 un-pended.
- WIP commit (last): the codes another code reports become aliases in the registry files (`diagnostics/codes/*.rs`, hand-edited; `scripts/extract-codes.py` is stale vs the committed files: don't regenerate). It compiles; **no test run yet** (`codes/mod.rs` tests, cases).

## Next actions, in order

1. `cargo test -p fa-analyzer` (aliases); fix the tests in `diagnostics/codes/mod.rs` if they list removed codes.
2. Full run: `export CARGO_BUILD_JOBS=2 LLVM_SYS_181_PREFIX=$HOME/.local/llvm18/usr/lib/llvm-18; (ulimit -v 8000000; cargo test --workspace --no-fail-fast -- --test-threads=2)`, with `FA_UPDATE_SKIPS=1` once for the LLVM harness (review the diff: only the callback cases should be added). Fix W-DECL-2 fallout in other crates (filter it or rename to `_x`).
3. `cargo clippy --workspace --all-targets` (only `-p fa-analyzer` was run).
4. Measure `fa check` on benchmarks/compiler (1,000 files): wall, CPU, peak RSS, determinism across thread counts.
5. Remaining codes without emitter (classification in the scratchpad was: DOC but not implemented): FN100-FN104, FN110, FN072 (need `with` partial application, a feature), FN086 (`h.describe` as a value), WFN002, MEM003, E-OPT-5 (`s?[0]`, now AN001), GN017, GN030, TF002 (type functions checked at declaration), CFG003, FN094 (can't trigger), parse-error codes (CV003, E-CMP-10, E-WHEN-4, E-WHEN-7, TY001, E-OBJ-14: the parser reports them as AN002: remove or alias), undocumented: CL023, EF020, W-LEX-1.
6. Native instance per error set for PASSES_ERRORS calls (decision "Callbacks that throw"); JS gap: a `catch` in the callee around both its own failing calls and a passed callback call catches the callback's exception too (doc says it shouldn't).

## Notes

- `proofs::reliable_interval` (src/proofs.rs, not mine) still says "the analysis assumes an overflow stops the program"; with this branch the analyzer's flow intervals are wrap-sound for every width, so its narrow-type exception is only conservative.
- Agent finding to check: ft-6 false E-TYPE-1 on a `Void`-returning function-type fit; TF021 on `type Wrap<T> = Keys<T>` at declaration.
