# Independent review results

Implementation: `6e41789b`. Each row uses the committed source SHA-256; only the final version of a program is included. `OK` means exit 0 **and** stdout matches every `-- prints:` expectation. `WRONG` means exit 0 with different output; `EXIT n` means a nonzero exit. Negative checker cases are described in README/report.

Columns: Bun; native O0/O3; wasm O0/O3; freestanding O0/O3. LLVM runs enable leak checks. Commands and full output are reproducible with `matrix.py`.

| Program | Bun | N0 | N3 | W0 | W3 | F0 | F3 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| async-bound-allocated.fa | OK | EXIT 102 | EXIT 102 | EXIT 1 | EXIT 102 | EXIT 102 | EXIT 102 |
| async-bound-array-control.fa | OK | OK | OK | OK | OK | OK | OK |
| async-bound-monomorphic.fa | OK | EXIT 1 | EXIT 1 | EXIT 1 | WRONG | EXIT 1 | EXIT 1 |
| async-bound.fa | OK | WRONG | WRONG | EXIT 1 | WRONG | WRONG | WRONG |
| async-direct.fa | OK | OK | OK | OK | OK | OK | OK |
| async-operations-direct.fa | OK | OK | OK | OK | OK | OK | OK |
| c-array.foreign.fa | OK | OK | OK | OK | OK | OK | OK |
| callback-cleanup-array-control.foreign.fa | WRONG | OK | OK | OK | OK | OK | OK |
| callback-cleanup.foreign.fa | OK | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG |
| capacity-function-values.fa | OK | OK | OK | OK | OK | OK | OK |
| captured-capacity-control-runtime.fa | OK | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG |
| captured-capacity-runtime.fa | OK | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 |
| containers.fa | OK | OK | OK | OK | OK | OK | OK |
| direct-cleanup.foreign.fa | OK | OK | OK | OK | OK | OK | OK |
| generic-item.fa | OK | OK | OK | OK | OK | OK | OK |
| generic-method.fa | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 | EXIT 1 |
| json.fa | OK | OK | OK | OK | OK | — | — |
| negative-get-array-control.fa | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG |
| negative-get.fa | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG |
| operations.fa | OK | OK | OK | OK | OK | OK | OK |
| schema-hash.fa | OK | OK | OK | OK | OK | OK | OK |
| sync-bound-in-async.fa | OK | WRONG | WRONG | WRONG | WRONG | WRONG | WRONG |
| union-items.fa | OK | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 |
| union-keys-array-control.fa | OK | OK | OK | OK | OK | OK | OK |
| union-keys-runtime.fa | OK | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 |
| union-keys.fa | OK | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 |
| union-read-array-control.fa | OK | OK | OK | OK | OK | OK | OK |
| union-read.fa | OK | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 | EXIT 101 |
| wide-key.fa | OK | OK | OK | EXIT 1 | EXIT 1 | OK | OK |

Checker-only cases: `negative-indices{,-array-control}.fa` reject with META005; `captured-capacity{,-control}.fa` reject with AN001; `generic-method-array-control.fa` rejects with GN007/AN001. These are inherited controls, not new passing executions.


The ordinary resource-Array callback has an inherited missing JS drop; the InlineArray callback is correct on Bun. Captured values, negative indices and independent method generics have ordinary controls and are recorded separately from merge blockers. The minimized wide-key failure is a wasm module validation error, not a timeout.

Actual per-unit `split/`: Bun plus all six LLVM configurations pass; the driver requires `PackageOutcome::Built` and enables leak checks.

Original reproductions: 70/70 intended outcomes across the same seven configurations; ordering is an expected checker rejection.
