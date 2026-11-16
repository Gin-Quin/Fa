# InlineArray second review

Reviewed `links/inline-array` at `6e41789b`; artifacts on `review/inline-array-2`.
Only review programs/drivers/notes are changed. The external report is
`/home/claude-dev/fa-lead/reports/review-inline-array-2.md`.

Run from the repository root with the LLVM-enabled warm `target/debug/fa`,
`target/debug/fa-native` and `target/debug/examples/build`. Every command uses
CPUs 0–4. Add `~/.bun/bin` and `~/.local/bin` to PATH.

```sh
taskset -c 0-4 python3 design/reviews/repros/inline-array-2/matrix.py design/reviews/repros/inline-array-2/operations.fa
```

The serial runner executes Bun and native/wasm/native-freestanding O0/O3, enabling
LLVM leak checks. It records commands, outputs, exit status, source SHA-256 and
maximum sampled process RSS (including children), kills any process over 512 MiB,
and times out at 90 seconds. Artifacts/logs are under `/tmp/inline-array-2-matrix`;
filenames include the source digest. Exit zero alone does **not** mean correct:
several findings return success with wrong output; compare the `-- prints:` lines
and the table below. No finding is marked pending or integrated into an existing
passing test corpus.

## Findings

| Program | Required behavior | Observed on LLVM |
| --- | --- | --- |
| `async-bound.fa` | `["kept"]` | `[]`; wasm O0 signature trap |
| `async-bound-monomorphic.fa` | `["kept"]` | Native/freestanding crash; wasm O0 signature trap, O3 `[]` |
| `async-bound-allocated.fa` | One long formatted item | `[]`, `leak: 2 blocks`, exit 102; wasm O0 signature trap |
| `sync-bound-in-async.fa` | `["kept"]` | `[]` on all targets/O0/O3 |
| `callback-cleanup.foreign.fa` | `drop callback`, `0`, once only | `drop callback`, `1`, `drop callback` on all targets/O0/O3 |
| `async-operations.fa` | `["result"]` | Assertion failure; wasm O0 signature trap |
| `union-read.fa` | `true` | Compiler panic on all targets/O0/O3; ordinary Array control passes |
| `wide-key.fa` | `9` | wasm O0/O3 module rejected: 1025 parameters exceeds 1000 |

All these programs behave correctly on Bun. The named-union failure is minimized to three lines; `union-items.fa`, `union-keys-runtime.fa` and `union-keys.fa` preserve its mutation/key/compile-time siblings. `async-direct.fa`,
`async-bound-array-control.fa` and `direct-cleanup.foreign.fa` pass all seven
configurations. The ordinary resource-array callback control preserves the
caller's length and destroys exactly once on LLVM; Bun misses its generic
Array resource drop (an inherited generic Array lifetime boundary).

Inspect the async ABI:

```sh
taskset -c 0-4 target/debug/examples/build design/reviews/repros/inline-array-2/async-bound-monomorphic.fa --target=native -O0 --leak-check --emit-ir --output=/tmp/inline-array-2-ir
```

Its indirect async call passes `{ [1 x { ptr, i64, i64 }], i8 }` by value;
`fa.append.async.thunk` expects `ptr` in that position. The synchronous thunk
instead explicitly copies the mutable argument to its own stack slot.

`schema-hash-wide.fa` is the larger nested key that led to `wide-key.fa`:
wasm O0 exceeds 1000 parameters (1539), and native O3 compilation exceeded the
90-second runner limit. The smaller `schema-hash.fa` passes and also exercises
zero/one/255/256 generic capacities and compile-time builds.

## Positive coverage

- `operations.fa`: every specified operation through a capacity generic, a bound
  callback, a generic owner method and compile-time execution; full failure,
  prefix preservation, assignment, iteration, equality and printing.
- `generic-item.fa`: both Item and capacity generic, structure items, explicit
  copies, iteration writes, runtime/static agreement and returned ownership.
- `capacity-function-values.fa`: bound generic constructors forwarded as callbacks
  through another capacity generic; runtime/static agreement.
- `containers.fa`: full operation sequence in a generic row field of Array,
  GrowingArray, Pool, GrowingPool, Columnar, GrowingColumnar, OrderedColumnar,
  InlineArray, Map and GrowingMap; InlineArray keys/items of Set/GrowingSet.
- `c-array.foreign.fa`: the full operation sequence in an item of CArray.
- `async-operations-direct.fa`: full operation sequence, generic owner methods and
  repeated suspension, using direct calls (positive control for the callback gap).
- `json.fa`: generic owner with a capacity parameter, two different instances,
  capacity zero, oversized input, invalid item after initialized heap strings;
  Bun/native/wasm, with native/wasm O0/O3 leak checks. JSON's existing libc
  requirement excludes freestanding.
- `split/`: imported generic methods/defaults/callbacks, compile-time calls and an
  Array of imported generic owners; actual per-unit compilation required below.

## Shared inherited boundaries

`negative-indices.fa` and `negative-indices-array-control.fa` both fail compile-time
negative indexing with META005, despite the runtime rule accepting negative
indices. `captured-capacity.fa` and its scalar `captured-capacity-control.fa` both
fail compile-time capture of an outer generic value. The runtime InlineArray
variant works on Bun and is unsupported on LLVM; the scalar runtime control
prints `0` on LLVM instead of `1`. These are reported separately from blockers. `negative-get{,-array-control}.fa` silently print runtime `2` then static `none` on all seven configurations. Generic methods with their own value list (`generic-method{,-array-control}.fa`) are also rejected with GN007 in both variants.

## Per-unit runner with leak checks

The public CLI has no leak-check switch. `split.rs` calls `compile_package` with
leak checking enabled and treats any fallback/unsupported outcome as a failure.
It uses the same CodebaseResolver/pipeline entry as the LLVM corpus. Compile it
against the LLVM-enabled workspace rlibs, after building the workspace:

```sh
taskset -c 0-4 python3 - <<'PY'
from pathlib import Path
import os, subprocess
cmd = ['taskset', '-c', '0-4', 'rustc', '--edition=2024',
       'design/reviews/repros/inline-array-2/split.rs', '-o', '/tmp/inline-array-2-split',
       '-L', 'dependency=target/debug/deps',
       '-L', 'native=' + os.path.expanduser('~/.local/llvm18/usr/lib/llvm-18/lib')]
for name in ['fa_analyzer', 'fa_codegen_llvm']:
    lib = max(Path('target/debug/deps').glob('lib' + name + '-*.rlib'),
              key=lambda path: path.stat().st_mtime)
    cmd += ['--extern', name + '=' + str(lib)]
subprocess.run(cmd, check=True)
PY
taskset -c 0-4 /tmp/inline-array-2-split design/reviews/repros/inline-array-2/split native 0 /tmp/inline-array-2-split-native/main
```

Repeat for O3 and wasm/native-freestanding. All six runs print `["result"]`
three times and exit zero with leak checks. Bun:

```sh
taskset -c 0-4 target/debug/fa run design/reviews/repros/inline-array-2/split
```

## Original findings

All ten original reproductions for the seven findings were run unchanged from
`../inline-array/` on all seven configurations (70 runs). All now give their
specified results, including rejection of `ordering.fa` by the checker. The
new findings above are combinations beyond those repaired reproductions.
