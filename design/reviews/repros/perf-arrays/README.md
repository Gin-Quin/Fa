# Array optimization review

Reviewed `perf/arrays @ 627042e4`, base `71af4fd0`. Production code is unchanged.
Full report: `/home/claude-dev/fa-lead/reports/review-perf-arrays.md`.

Run under `taskset -c 0-4`, with LLVM 18 enabled, Bun and Node on PATH.
Build `cargo build -p fa-codegen-llvm --example build` first (two Cargo jobs).
The matrix runner uses this example at O0 and O3 with leak checking, compares
against Bun, and retains commands, stdout/stderr and RSS under `/tmp`.

```sh
taskset -c 0-4 python3 design/reviews/repros/perf-arrays/run.py \
  design/reviews/repros/perf-arrays/nested-fill.fa \
  design/reviews/repros/perf-arrays/nested-copy.fa
```

Both programs intentionally expose C1: all six LLVM combinations give wrong
lengths and contents; Bun is correct. `nested-fill.fa` passes on the archived
pre-cache compiler (`68e00504`). The fix must synchronize both bulk fill and
bulk copy paths with an enclosing push cache, or decline caching there.

Passing probes: `growing-boundaries.fa`, `fill-fallbacks.fa`,
`extents-alias-fp.fa`, `growing-fallbacks.fa`,
`extent-fallbacks.fa`. Run them with the same runner. Use
`REVIEW_ASYNC=true` for `async-fallbacks.fa`; `REVIEW_OVERFLOW=false` exercises
the unchecked remainder path. Set `REVIEW_OUTPUT` to preserve distinct runs.
`remainder-widths.fa` preserves a separate pre-existing failure at UInt32.maxValue:
literal remainder by 7 prints/computes 1 instead of 3. The minimal
`remainder-uint32-max.fa` and full `remainder-endpoints-observe.fa` preserve it
without hiding later widths. The base compiler `71af4fd0` fails identically.
`alias-negative.fa` is a checker rejection control (REF013), not a runnable test.

`extents-alias-fp.fa` prints sign, binary exponent and all 53 significand bits
of finite doubles, including signed zero and a subnormal result. This permits
exact comparisons with Bun and O0 without float formatting or C-library calls.
Its dynamic indices can designate the same element as the foreach reference.

`ir-controls.py` checks firing against archived before compilers for all four
optimizations; `verify-data.py` checks all 240 instruction gate rows against the
raw driver JSON and regenerates all seven Markdown tables byte for byte.
`per-unit/` reproduces the same nested-fill defect in a separately compiled unit;
its generic exercise is reached only after the failing assertion is repaired.
It is compatible with the existing collections review's `run_package.rs`
runner: pass this directory and a scratch output directory; expected stdout
is `per-unit ok`. The runner executes every target at O0/O3 with leak checking;
the current compiler fails the nested-fill assertion. `run-package.py` compiles
that existing Rust runner against the current warm LLVM-enabled libraries.

Closing validation: LLVM-enabled workspace **1,400 passed / zero failed /
eight existing ignored**, 74 groups, including 104 IR checks and all three
runtime corpora. Clippy is warning-free; fmt and all three formatter corpus
tests pass. The intentionally failing review programs are outside the regular
runtime corpus. C1 is the new merge blocker; the UInt32 issue reproduces on
the base. Per-unit freestanding O0 also retains the already documented hosted
runtime link failure (M1); the other five combinations expose C1.
