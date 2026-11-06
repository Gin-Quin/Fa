# Analyzer engineer 2: state at pause (2026-11-05)

## Resumption (2026-11-06)

- Initial integration is green: strict `cargo test --workspace --no-fail-fast
  -- --test-threads=2` exited 0, LLVM enabled, in
  `/tmp/fa-resume-analyzer-workspace-fixed.log`. Final workspace clippy is
  clean (`/tmp/fa-resume-analyzer-clippy-final.log`), as are Rust formatting,
  Bun formatting checks and all 296 runtime JS tests. The implementation
  below is committed as the initial integration; new callback work is in
  the separate `codex/analyzer-callbacks` branch and worktree, with its own
  `target/` directory.
- The analyzer crate passes after fixing the registry coverage test to count
  canonical codes and aliases by prefix. Workspace clippy and formatting pass;
  runtime JavaScript: 296 tests pass. Dependent unit tests: CLI 49, JS codegen 27,
  language server 142 pass without filtering the new warning.
- First full workspace run: `/tmp/fa-resume-analyzer-workspace.log` (session
  30442, LLVM enabled, `FA_UPDATE_SKIPS=1`, under
  `flock /tmp/fa-compiler-full-suite.lock`). Found three failures: CLI native
  callback case, UInt32 wrapping on JavaScript, and unused `arguments` in the
  generated JS-pipeline and language-server package fixtures. It ended with
  exactly these four failed targets; every LLVM program suite passed.
- Fixes on disk: the callback multi-file case declares its unsupported native
  targets until native callback specialization lands; the pipeline fixture uses
  `_arguments`; integer sinks use `ArithInfo.interval` before wrapping when
  deciding `SINK_CHECK`, rather than the conservative post-wrap flow interval.
- Also found a soundness hole in `counts_elements`: it bounded every built-in
  integer result of a collection by `0..=2^48`, including `sum`, `reduce`, and
  `Map.getOrInsert`. `read([-100])` could reach an unchecked local array access
  and print `undefined` on Bun. Fixed by whitelisting actual search positions;
  `review/length-bound-user-data.fa` checks the three operations with negative
  and positive indices. The analyzer unit tests and cases pass with the fix.
- Before the initial integration, the targeted CLI/JS/LLVM reruns, the reviewed
  callback-only expected-skips additions, final clippy and the complete second
  workspace run all passed.
- Targeted reruns pass (including the new bounds repro on Bun, native,
  freestanding and wasm). The second full ran after performance/proofs:
  session 75639, `/tmp/fa-resume-analyzer-workspace-fixed.log`; it is strict
  (no `FA_UPDATE_SKIPS`) and uses `flock --close` so descendants inherit no lock.
- The first suite left the known deep-recursion Bun case running after timeout:
  the harness killed only `fa`. `cli/tests/programs.rs` now runs each program in
  its own Unix process group and terminates the group before waiting. A shell
  regression checks that the grandchild stops too; the actual `adv-6.8-05` case
  stopped at 20.05 s with no remaining Bun process.
- A used local module alias also got a false W-DECL-2 after its resolutions
  became External. Uses are now bits filled in `set_resolution` during the
  walk, so the warning needs no full tree scan and remembers aliases. The
  `unused-alias-files.fa` case and all analyzer unit/case tests pass.
- Baseline (release, 1000 files, 504979 lines, 19.2 MB, `--target-share 0`):
  diagnostic output identical at 1/2/6 threads and seeds 1/7/19; 13020 intentional
  unused bindings produce warnings. Wall times 10.62/5.97/4.24 s; peak RSS
  174/192/222 MiB. A copy changing only `error`/`arguments` to `_error`/`_arguments`
  has no diagnostic: 3.43 s wall, 9.63 s user, 229 MiB at 6 threads. Generator
  cleanup is kept for after the initial integration. Logs and fixtures:
  `/tmp/fa-resume-analyzer-1000*`.
- Next callback work: have the analyzer mark direct calls of `only_called`
  parameters. On JS, only such calls inside a `catch` need protection: evaluate
  the arguments first, then capture the identity of a callback exception in a
  function-local temporary; that function's catches rethrow that identity.
  This preserves the original error without runtime wrappers and works with
  awaited calls. Native specialization needs the widened expected callback
  types already calculated at each call in an analysis side table, and must
  bypass the callee's catch when returning a callback error. `start callback()`
  must not count as `only_called`. The lead asked the user whether a callback's
  explicitly declared `throws` also bypasses catches; that extension is pending
  their answer (the existing decision covers parameters without `throws`).
- `GN030` is obsolete: generic lambdas are valid under the new decision. The
  lead will update old briefs; retire this code instead of adding an emitter.

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
