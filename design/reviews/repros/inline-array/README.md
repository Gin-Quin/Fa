# InlineArray review probes

Reviewed branch: `links/inline-array` at `f4d2f7f5`. These programs preserve
observed failures and positive controls; comments describe the required result.
They are independent review artifacts, outside the compiler's passing corpus.
No implementation or existing expectation is changed.

Run from the repository root, with Bun/Node on PATH. Pin all tools to CPUs 0–4.
For example:

```sh
taskset -c 0-4 target/debug/fa run design/reviews/repros/inline-array/comptime-generic.fa
taskset -c 0-4 target/debug/examples/build design/reviews/repros/inline-array/comptime-generic.fa --target=native -O0 --leak-check --output=/tmp/review-inline-native --run
```

For each LLVM probe, repeat with `-O3` and with `--target=wasm` and
`--target=native-freestanding`. The build example needs LLVM enabled when built.
Allow the linker/V8 virtual address reservations; enforce the 512 MiB limit on
resident memory, rather than address space.

For Node and web:

```sh
taskset -c 0-4 target/debug/fa build design/reviews/repros/inline-array/comptime-generic.fa --target=node --output=/tmp/review-inline-js
taskset -c 0-4 "$HOME/.local/bin/node" /tmp/review-inline-js/node/main.js
taskset -c 0-4 target/debug/fa build design/reviews/repros/inline-array/comptime-generic.fa --target=web --output=/tmp/review-inline-js
taskset -c 0-4 "$HOME/.local/bin/node" --experimental-vm-modules --disable-warning=ExperimentalWarning packages/cli/tests/programs/web-runner.mjs /tmp/review-inline-js/web/main.js
```

| Program | Expected | Observed at reviewed commit |
| --- | --- | --- |
| `comptime-generic.fa` | Runtime and static both `[7]`, length 1 | Static `[7, 8] 2` on JS; LLVM reads corrupt second item (`2`, or `770` at native O3) |
| `comptime-none-test.fa` | `true` twice | `true`, `false` on every target |
| `comptime-none.fa` | `[none]` twice | JS formatting masks malformed constant; LLVM rejects it |
| `generic-capacity.fa` | Capacity 2 and `[1]` | JS compiler panic; LLVM unsupported capacity |
| `generic-bound.fa` | `[9]` | Incorrect E-TYPE-1 at concrete argument |
| `generic-cleanup.foreign.fa` | `drop one` | Empty output on Bun/Node/web; all LLVM targets correct |
| `map-key.fa` | `found` | Invalid LLVM IR; all JS targets correct |
| `schema.fa` | Round-trip type, capacity 2 | TF043: `Collection` is not a collection kind |
| `nested-capacity.fa` | `full`, `[1]` | Correct on Bun and every LLVM target |
| `zero-recursive.fa` | Name and empty children | Correct on Bun and every LLVM target |
| `async-return.fa` | Two strings survive suspension and return | Correct on every target |
| `get-alias.fa` | Borrowed point changes in original list | Correct on Bun; LLVM optional-get borrow unsupported (inherited Array limitation) |
| `nested-comptime.fa` | Capacity failure preserves prefix | Nested mutating receiver unsupported during compile-time evaluation (inherited Array limitation) |
| `bound-push.fa` | Checked bound method | Builtin method as value unsupported (inherited Array limitation) |

The external review report contains severities, source locations, controls,
validation results and suggested fixes.
