# Collection map review

Review source: `perf/collections` at `6436bed3`, base `cbe3c5a6`.
Full report: `/home/claude-dev/fa-lead/reports/review-collections-maps.md`.

Run from the repository root, with LLVM 18 and Bun installed as in AGENTS.
Every command belongs on CPUs 0–4; no benchmark timing is needed here.
The standalone runner uses `target/debug/fa` and `target/debug/examples/build`;
build them with LLVM enabled first if the warm artifacts are absent.

```sh
taskset -c 0-4 python3 design/reviews/repros/collections-maps/run.py
taskset -c 0-4 bash design/reviews/repros/collections-maps/run_package.sh
taskset -c 0-4 env FA_REVIEW_PACKAGE=per-unit-delete bash design/reviews/repros/collections-maps/run_package.sh
taskset -c 0-4 env FA_REVIEW_PACKAGE=per-unit-generic bash design/reviews/repros/collections-maps/run_package.sh
taskset -c 0-4 python3 design/reviews/repros/collections-maps/ir_controls.py
taskset -c 0-4 python3 design/reviews/repros/collections-maps/audit.py
taskset -c 0-4 env CC=gcc packages/codegen-llvm/runtime/test/run.sh /tmp/collections-asan
```

`run.py` compares every standalone program with its `prints` annotations and
Bun, then native, wasm and native-freestanding, each at O0/O3 with leak checking.
It retains emitted IR, outputs, return codes and peak compiler RSS in `/tmp`.
`FA_REVIEW_OUT` chooses another output directory; `FA_REVIEW_FILTER` selects a
filename. It deliberately fails on `generic-delete.fa`, the new scalar64 layout
regression. Native artifacts segfault; wasm leaves all three entries present.
The example runner converts child signals to exit 1; invoke the emitted native
artifact directly to observe SIGSEGV. No failure is skipped or treated as passing.

`illegal-iteration.fa` is a negative analyzer test; run `target/debug/fa check`
on it separately. It must reject deletion while iterating with REF012.

The package runner uses the actual per-unit pipeline API and leak instrumentation,
without a fallback to whole-program compilation. `per-unit` is the non-generic
control. `per-unit-delete` reproduces the new generic deletion bug across files.
`per-unit-generic` retains the separate, pre-existing generic mutation/reference
layout bug, also reproduced on the base compiler. Freestanding O0 package linking
fails on the base and branch due to unresolved runtime libc symbols; O3 links.
Serialize this runner with other Cargo commands. The review additionally compiled
the same Rust driver against the existing rlibs directly, with matching dependency
versions, while the workspace test was running.

`ir_controls.py` uses the unchanged sources and firing assertions of the new IR
checks against archived before/after compilers; it also verifies all six budget
combinations. Its docstring names the source revisions needed to rebuild archives.
`audit.py` checks all 36 source/output hashes at every phase and prints the report's
map instruction rows directly from the checked-in driver JSON.

The other programs cover cached hit/miss invalidations, cyclic collision shifts,
error cleanup, references, owner methods, closures, await, moves/copies/returns,
frame budget overflow and recursion, every signed/unsigned integer width with
64-bit and narrow values, stable references, deleteWhere, deterministic collision
stress and compile-time evaluation.

`verification.json` retains the final 105 standalone executions, 18 per-unit
outcomes, 28 IR controls, instruction rows and fixture hashes. The six failing
standalone LLVM runs are C1. The package failures distinguish C1 from the two
pre-existing issues; none is suppressed as an expected skip.

## Fixed-layout regressions

The fixes on `perf/collections` (`632d4eed`, `75613ae9`) promote C1/C2 and the sibling boundaries into `packages/cli/tests/programs/map-generic-*.fa` and `packages/codegen-llvm/tests/programs/async-map-generic-layout.fa`. The shared corpus runs O3 on Bun/Node/web and LLVM targets, including whole-program and split packages. `cargo test -p fa-codegen-llvm --test map_layouts` supplies 24 O0/leak executions across all three LLVM targets. The `checks` suite checks caller/instance layout agreement at every inline integer width and separate generic readonly chunked/inline headers on native and wasm.

`fixes.json` retains final adversarial outcomes, source hashes, split constructor/instance IR calls, and the optimization firing/budget controls. All 105 standalone outcomes pass. Each of the three package matrices has five passing executable checks; its freestanding O0 link is still blocked by the pre-existing M1 (`_exit`, `sigaction`, `raise`, plus `strtod` with leak checking). Keep reporting that link failure, not treating it as a passing execution.

To inspect the split IR, run the package helper above with `FA_KEEP_OBJECTS=1` and `FA_REVIEW_PACKAGE=per-unit-delete` or `per-unit-generic`, then disassemble its O3 `objects/*.bc` files with LLVM 18 `llvm-dis`. For deletion, `application~main` and `library~prune.inst1` must both use `1099511627792` (DENSE plus 16-byte entry size). For reference stability, `library~create` and `library~replace.inst1` must both use 16 (chunked). This holds on native, wasm and freestanding.

Final LLVM-enabled workspace validation: **1,387 passed, zero failed, eight pre-existing ignored**, 69 result groups; fmt and warning-free workspace Clippy pass. Native/freestanding/wasm program counts are 822/733/801, with unchanged skips. The independent JS generic wide-literal follow-up is retained under `js-generic-wide-literal/` and in `design/todo.md`; the native ABI fixture keeps all value assertions and varies Number-backed key widths instead.
