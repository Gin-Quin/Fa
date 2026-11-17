# Dev loop speed measurements

Measured on 2026-11-17, starting from `5e3b602d`. LLVM 18 enabled; Rust 1.99.1;
6 CPUs and 12 GiB RAM shared with other engineers. Builds/tests use CPUs 0–4
and two Cargo jobs. Compiler/linker microbenchmarks use CPU 5. Logs are under
`/tmp/fa-dev-loop-speed/`; the lead's full incremental report is
`/home/claude-dev/fa-lead/reports/dev-loop-speed.md`.

These are baseline observations, not completed after-change results.

| Measurement | Baseline |
|---|---:|
| Empty-target workspace build, LLVM enabled | 407.32 s |
| Unchanged workspace build | 4.28 s |
| Test-artifact compilation after ordinary build | 10m57s |
| CLI package binary, 31 tests passed | 1546.91 s |
| CLI program binary, 12 passed / 3 timed out | 575.16 s |
| Quick string differential, 38 shards / 0 disagreements | 527.51 s |
| Target after build / after test compilation | 3.1 / 17 GiB |
| Inactive incremental data removed after compilation | 3,436,796 KiB |

The workspace invocation stopped at the failed CLI binary. Remaining prebuilt
binaries are queued as a continuation; no complete successful baseline workspace
elapsed time is claimed. The full string tier is still running. Neither the
compiler, profiles, fixture expectations nor timeouts has been changed yet.

## Compilation and linking

Cargo timings: analyzer 114.93 s, LLVM backend 86.72 s, CLI library 49.03 s,
lsp-types 38.94 s, language-server library 38.07 s, JS backend 37.47 s,
Tokio 35.15 s, interpreter 21.88 s, LLVM C++ shim build 20.68 s. These overlap;
they are not additive wall-clock time. LLVM bindings are a small part:
llvm-sys 0.68 s, its build 0.61 s, inkwell 7.10 s, inkwell macros 3.12 s.

Rust already uses bundled lld 23; Fa already uses lld 18. Direct single-CPU
full-debug linking took 23.462 s cold. Interleaved warm linker probes give lld
6.285/5.598/6.039 s and mold 7.030/6.714 s. Cold mold was 32.848 s. The installed
mold 2.30.0 did not demonstrate a gain, so it is an optional comparison tool.

## Native runtime reuse

The package harness runs 54 native multi-file fixtures serially and gives each
one a fresh XDG cache. An unchanged baseline compiler, pinned to CPU 5, running
the `modules` package with cache roots a/a/b/a gives:

| Cache | Wall s | Peak RSS KiB | Exit |
|---|---:|---:|---:|
| fresh a | 33.678 | 142596 | 3 |
| reused a | 1.564 | 96684 | 3 |
| fresh b | 35.031 | 143592 | 3 |
| reused a | 1.368 | 105472 | 3 |

Exit 3 belongs to this fixture. This proves existing reuse is valuable; it does
not yet measure the prepared separate runtime-cache option. Runtime keys also
currently include program field/shape facts although runtime lowering reads none;
unit-object keys must retain those facts.

## Slow fixtures

`fa check --timings` on CPU 5 takes 25.232 s for
`division-boundaries-bigint.fa` (25.053 s in analysis, 27260 KiB RSS), and
30.815 s for `division-boundaries-int128.fa` (20812 KiB). Both finish correctly
but exceed the unchanged 20 s harness deadline even on the reserved CPU. Both
time out on Bun, Node and web in the baseline; `js-million-copy.fa` also times
out on web. Profile experiments will retain assertions and overflow checks.

## Shared registry compilation proof

`scripts/check-shared-cache.py` creates two scratch projects with the workspace's
registry dependency graph, separate targets and Cargo locks. Both builds use one
Cargo job, and their intervals must overlap. It rejects a build-directory lock
wait. Two uncached builds completed in 878.765 and 835.922 s under shared load;
cache seeding and the cached repeat are still running. This measures registry
compilation sharing, not shared Cargo targets or cached workspace crates.
