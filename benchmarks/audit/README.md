# Native Fa / Goose performance audit

This directory contains the tools used to compare Fa's native backend with
Goose and to investigate the generated code. The implementation changes belong
to the LLVM backend and its runtime; the benchmark programs are unchanged.

The [full report](../../design/native-goose-performance.md) records the
iterations, safety conditions, measurements and remaining work. The latest
measurements were taken on 4 November 2026 on a shared Linux x86-64 VM.

## Changes implemented

| Area | Change | Safety condition |
| --- | --- | --- |
| Recursive collection storage | Store recursive object and tuple items directly in collection slots; reuse the existing representation of recursive aliases to containers. This includes objects mixing collection edges and direct recursive fields. | Prove finite inline layouts across the type graph. Keep indirection for direct cycles, and use the actual stored layout for copies, extraction, reinsertion and destruction. |
| Collection callbacks | Borrow the original Array, Map, Set and Pool slots when a recursive parameter can use their address. | The source slot remains alive for the call; borrowed views never acquire ownership of the stored item. Columnar rows can still require adaptation. |
| Stable tables | Store Map values in stable chunks, use a 32-bit index where it fits with a 64-bit fallback, and retain insertion probes through tombstones. | Growth moves the index rather than live values. Reuse probes only while the table and key remain valid. Hash-only equality requires an injective integer hash. |
| Recursive arenas | Support finite passive payloads owning strings and collections, with cleanup callbacks and tighter allocation granularity. | Use LLVM's alignment requirements, including 16-byte payloads. Retain general ownership when mutation or resource lifetimes prevent the arena proof. |
| Array loops | Load the storage and length before iteration; reserve a counted loop's additions to an initially empty object field. | Iteration already forbids structural mutation. Field pushes retain capacity checks because hidden aliases may add more items; existing nonempty items are not relocated early. |
| Ownership and conversions | Move directly returned passive locals when proven safe; move detailed Float-to-Integer error classification to the cold path. | Exclude escaping aliases, captures, asynchronous functions and custom destruction from the return optimization. Preserve ordered bounds checks, NaN/infinity rejection and error messages. |
| Strings and hashing | Inline short ASCII comparisons and compact character reads; specialize integer-key comparison after full-hash equality. | Keep general Unicode/string handling and structural equality for non-injective hashes. No pointer or integer truncation is introduced. |
| Composite helpers | Cache recursive formatters and equality helpers; clean up temporary views and argument conversions in their defining branch. | Compare optional payloads only when present. Keep IEEE ordinary equality separate from recursive SameValueZero equality for Map/Set keys. |
| Native allocator | Reuse 2 MiB spans on 64-bit targets and request transparent huge pages for eligible aligned native Linux allocations. Wasm32 keeps 1 MiB refills. | Account for every unused page in free spans. Retry ordinary alignment on allocation failure and tolerate a refused `madvise` hint. |

The transformations depend on types, proven uses and the platform. They do not
recognize benchmark names, input sizes or expected answers. Array indexing and
stable Map value references retain their existing guarantees.

## Results and limits

For `scene`, the same executable reads all three depths at runtime. These are
median process CPU times from 45 alternating runs pinned to CPU 1, in seconds:

| Depth | Fa before | Fa after | Goose |
| --- | ---: | ---: | ---: |
| 9 | 0.002835 | 0.002491 | 0.002738 |
| 12 | 0.100562 | 0.018024 | 0.018121 |
| 13 | 0.257315 | 0.047689 | 0.052418 |

The standard case is about 5.58 times faster than the old Fa binary and reaches
measured parity with Goose. The small difference between the two languages is
within the variability of this VM. Peak RSS rises from 7.38 to 9.25 MiB for Fa;
Goose uses 6.50 MiB. Huge pages trade some resident memory for faster address
translation on this host, whose transparent huge page policy is `madvise`.

Parity across all 25 programs remains open. Redundant boxes are not eliminated
in every representation: `Array(Node?)`, some nested tuples/aggregates, and
some unions containing collections retain their current layout. A columnar row
may need materialization for a callback expecting a contiguous object. Direct
recursion uses the arena only when its existing proof succeeds.

Small timing differences must be read with the controls: identical `sum`
binaries varied noticeably, and an apparent `fannkuch-redux` regression did not
reproduce in a longer alternating run. Instruction counts, CPU time and RSS are
separate measurements.

The checked-in data retains samples, commands, sizes, hashes and validation:

- [Stable tables and arena payloads](../../design/native-goose-optimization-results.json).
- [Alignment, strings and probes](../../design/native-goose-round3-results.json).
- [Recursive storage and final scene measurements](../../design/native-goose-round4-results.json).

After the final composite-key equality fix, all 25 Fa executables were rebuilt
and verified byte-for-byte identical to the timed binaries. Their outputs match
the Rust reference.

## Reproducing the comparison

Install LLVM 18 using `scripts/setup-llvm.sh`, then make the Rust toolchain,
Python 3, `objdump`, Goose and clang 18 available. From the repository root:

```sh
export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
export GOOSE="$HOME/.local/goose/goose"
export GOOSE_HOME="$(dirname "$GOOSE")"
export GOOSE_CC="$LLVM_SYS_181_PREFIX/bin/clang"
cargo build --release -p fa
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
python3 benchmarks/audit/compare.py scene \
  --directory benchmarks/audit/build/reproduce-scene \
  --goose-runtime-size --default-sizes --cpu 1 --runs 45
```

Choose a CPU available to the process. Run timings after builds, tests and
profilers finish. The script checks outputs against Rust before timing and
retains the generated Goose copy, C/LLVM/assembly where available, build logs
and JSON under the ignored output directory.

The recorded native comparison uses generic x86-64, Fa `-O3` with
`--overflow-checks=false`, and Goose `-O2` with clang 18. Bounds, key, divisor
and conversion checks remain enabled. The checked builds described below enable
overflow checks as well. The ordinary `fa build` native release defaults match
the comparison's overflow setting.

`--goose-runtime-size` changes only generated copies: it reads `N` from argv
and initializes dependent globals in order. It preserves `checkfree` annotations.
A full-suite run currently records a Goose compilation failure for dynamic-size
`blur`, whose bounds proof no longer succeeds; it must not be reported as a
successful comparison or bypassed by removing the annotation.

Use a new `--directory` to preserve a baseline. `--reuse` times existing builds;
`--rebuild-fa` with `--reuse` rebuilds only Fa. A runtime-size run refuses a saved
Goose binary that still embeds a constant `N`.

## Validation

With LLVM 18 enabled, the following passed after the implementation changes:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
```

The workspace includes 33 IR checks, programs on native, native without libc
and wasm, the C runtime under ASan/UBSan, and allocation loops under a 128 MiB
limit. The focused validation adds:

- 44 successful executions at `-O0`/`-O3` with overflow and leak checks; four
  exclusions require floating-point formatting not yet implemented without libc.
- The eight native fixtures with forced failure of huge-alignment allocations
  and huge-page advice, checking the ordinary fallback.
- `scene` at depths 9, 12 and 13 on native and wasm with overflow and leak checks.
- Independent copies after destroying their sources; references surviving Map
  growth; extraction/reinsertion; alias-driven extra pushes; early returns;
  recursive formatting; optional equality; and NaN keys in recursive composites.

The fixtures live in [the LLVM program tests](../../packages/codegen-llvm/tests/programs/).
After changing `runtime/fa_runtime.c`, regenerate both checked-in IR files with
`packages/codegen-llvm/runtime/generate.sh` before validating or measuring.
