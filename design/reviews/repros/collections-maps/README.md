# Collection map review

Review source: `perf/collections` at `6436bed3`, base `cbe3c5a6`.
Full report: `/home/claude-dev/fa-lead/reports/review-collections-maps.md`.

Run from the repository root, with LLVM 18 and Bun installed as in AGENTS.
Every command belongs on CPUs 0–4; no benchmark timing is needed here.

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
