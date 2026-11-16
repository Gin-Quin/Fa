# Reproducible Fa / Goose / Rust measurements

`compare.py` is the single native audit driver. It replaces the first audit's
build/profile/report/rss shell pipeline and variants timer. `run.ts` remains a
quick cross-target wall-time smoke check; use this driver for performance claims.
`hot.py` and `show.py` inspect profiles; `ir_counts.py` counts static IR separately.

```sh
export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
export PATH="$HOME/.bun/bin:$HOME/.local/bin:$PATH"
export GOOSE="$HOME/.local/goose/goose"
export GOOSE_CC="$HOME/.local/llvm18/usr/bin/clang-18"
export CC="$GOOSE_CC"
CARGO_BUILD_JOBS=2 cargo build --release -p fa
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
# Finish all builds before starting CPU measurements. Default: original Goose 16.
python3 benchmarks/audit/compare.py --suite all --directory benchmarks/audit/build/default --default-sizes --mode prepare
python3 benchmarks/audit/compare.py --suite all --directory benchmarks/audit/build/audit --mode prepare
python3 benchmarks/audit/compare.py --suite all --directory benchmarks/audit/build/default --default-sizes --reuse --mode cpu --runs 21 --cpu 1 --report benchmarks/results/cpu.json
python3 benchmarks/audit/compare.py --suite all --directory benchmarks/audit/build/audit --reuse --mode profile --report benchmarks/results/profiles.json
# Secondary targets: same default inputs, three rounds, a 512 MiB RSS ceiling.
python3 benchmarks/audit/compare.py --suite all --directory benchmarks/audit/build/secondary --default-sizes --mode secondary --runs 3 --report benchmarks/results/secondary.json
# Optionally repeat at audit sizes in another directory for successful small-input proxies.
python3 benchmarks/audit/compare.py --render benchmarks/results/cpu.json benchmarks/results/profiles.json benchmarks/results/secondary.json --report benchmarks/results/baseline.json --markdown benchmarks/results/baseline.md --update-readme
```

Positional names select benchmarks/variants; `--suite secondary` selects own ports
and variants. The original suite is exactly `binary-trees`, `blur`, `calc`, `graph`,
`interp`, `lru`, `particles`, `push`, `records`, `respond`, `scene`, `sexp`, `strlist`,
`sum`, `tree`, `words`. `binary-trees` is Goose's `bintrees`, also one of the original
ten: there are 25 unique original ports, plus growing-index and four variants.
`push-array` preserves the original reserved Array port; `push` grows an unreserved
GrowingArray, with integer marks as prescribed by Fa's reference documentation.
Both use the suite's sole safe Rust push row (Vec+reserve, indices).
Use `growing-index growing-index-array --size 16` (then 1000 and 1000000) with the
same prepare/cpu/profile workflow. Render keeps size overrides as separate rows.

Native release: synchronous `fa build -O3 --overflow-checks=false`, Goose
`-O2 --standalone` with explicit Clang 18, safe Rust `-C opt-level=3` without overflow
checks. Source algorithms/widths are preserved. Constant BENCH_N sizes are rewritten
on both Fa and Goose. `--goose-runtime-size` is an explicitly different experiment;
never remove Goose's `checkfree` annotations to make it compile.

Every executable has a checked manifest. It includes effective source hashes,
compiler paths/versions/hashes (Fa companion and linker, Goose C compiler and
stdlib, Rust toolchain), flags, environment, size/modes, executable SHA-256 and
exact Rust reference output. `--reuse` rejects any stale source, mode, compiler,
executable or output. `--reuse --rebuild-fa` checks Goose and Rust before rebuilding
only Fa. Use the real compiler for `FA`; wrappers hide dependencies and are rejected.
Wasm manifests also validate the binary separately from its loader. Every timed,
RSS and profiled run checks stdout, not only the warmup. No missing target is
silently successful: error records remain in JSON and the driver exits nonzero.

CPU rounds interleave Fa, Goose, Rust and **the exact Fa binary again** under the
`fa_control` label. The order reverses each round and rotates every two rounds;
each sample retains user/system CPU (`wait4`), wall time and RSS. The comparison
retains both ratio of medians and median of per-round ratios, plus the latter's
Q1/Q3 (linear interpolation at `(n-1)*p`, type 7). No outliers are discarded.
CPU acceptance needs at least 21 rounds; shorter smoke series remain unresolved.
Near-threshold results (±0.05 around 1.10 for Goose, 1.00 for Rust) count only when
the control differs from one by **strictly less than 3% on both measures**. A
straddling pair of estimates is also unresolved. Profile unresolved rows; never
pick the favorable statistic. CPU is pinned to one logical core; wait until builds,
tests and profilers finish. Load snapshots are recorded at phase and row boundaries.

Callgrind measures the entire process at `sizes.txt` audit sizes. Cache/branch
simulation uses an explicit 32 KiB/8-way I1 and D1 and 8 MiB/16-way last-level
cache, 64-byte lines; these are modeled misses, not hardware counters. Retained
profiles have instruction addresses, exclusive/inclusive function annotations and
hot assembly runs. Fa currently emits no source DWARF: attribution by “line” means
machine instruction/disassembly, checked against the retained optimized IR and
benchmark sources; it does not pretend to be source-line DWARF attribution. If Valgrind loses the
main ELF's object name, the postprocessor recovers anonymous PCs only when at
least 90% of their instruction-weighted addresses match the saved executable's
disassembly at one known load base. Empty executable instruction maps fail the
profile step; they are not published as a successful attribution.
Rust HashMap keeps its randomized default hasher for fairness: lru, words and
word-count retain three Rust profiles and use the median-Ir execution, with every
event set retained so hash-probe variation is visible. No deterministic hasher is
substituted. Callgrind counts executed instructions exactly; entropy can still
change which instructions a randomized program executes.
GNU time measures peak RSS independently at the CPU size; executable sizes are
both original and `strip` on a copy of the actual timed binary.

## JSON schema (version 2)

A phase object has `schema: 2`, `mode` (`prepare`, `cpu`, `profile`, `secondary`),
`revision`, dirty flag, runner-source SHA-256, command line, compiler identities,
machine/kernel/CPU, affinity, start/end UTC/load averages, `runs`, `size_set`,
selection and `benchmarks`. Benchmark records have exact `size`, output SHA-256,
checked artifact manifests and start/end load snapshots, or an explicit `error`.

- CPU: per-language `samples` with seconds `user`, `system`, `cpu`, `wall`, and
  `peak_rss_kib`; medians, separate `gnu_peak_rss_kib`, original/stripped byte sizes;
  `orders`; `control` and `comparisons.{goose,rust}` with `ratio_of_medians`, paired
  `ratios`, `q1`, `median`, `q3`, threshold and verdict. Control includes `passes`.
- Profile: per-language `events` (`Ir`, `Dr`, `Dw`, `D1mr`, `DLmr`, etc.), path and
  SHA-256 of callgrind, function/assembly annotations. Counts are whole-process.
- Secondary: runtime identity and `targets.{wasm,bun}` with CPU/wall samples and
  medians and artifact manifest, or the explicit build/run/output/timeout/memory-limit error. Sizes
  remain explicit: audit-size secondary times must not be divided by default-size
  native times.

`--render` combines phase objects into `{schema: 2, series: [...]}` and derives
Markdown from those records. Keep the JSON with a published table (force-add a
named dated result under the normally ignored `benchmarks/results/`). Nothing is
entered by hand into the generated tables. Raw executables/profiles remain ignored.

For a direct GrowingArray/Array (or String/bytes) CPU gate, interleave the prepared
variant in the **same rounds**, rather than pairing samples from separate rows:

```sh
python3 benchmarks/audit/compare.py growing-index --size 16 --directory benchmarks/audit/build/index-16 --reuse --mode cpu --runs 21 --additional growing-index-array=benchmarks/audit/build/index-16/growing-index-array --report benchmarks/results/index-16-paired.json
```

`--additional NAME=DIRECTORY` validates all three existing manifests against the
current sources, flags, modes, exact size and Rust reference, and revalidates the
saved compiler's live identity, then adds the Fa
binary as another label. No bare unvalidated executable is accepted. The result
retains its artifact manifests, samples, orders and `additional_pairs` (Fa divided
by the variant, ratio of medians and paired median/quartiles). Apply the documented
1.10 GrowingArray/Array gate to both estimates with the same control; this field
does not invent a threshold for arbitrary variants. The same option replaces the
old variants timer's multi-binary series.

`FA_SOURCE_REVISION` records the compiler source commit separately from the
runner/input commit (use it for parent/custom compiler builds). `--supplemental`
retains a diagnostic series in combined JSON while excluding it from headline
Markdown rows: parent profiles and extra variant series cannot silently replace
trunk measurements. Native variant CPU pairs can be supplemental while the
primary per-program series stays fixed.

The additional manifest may name a different compiler (for a parent/before/after
control). Its actual Fa executable, backend and linker are identified again;
a changed compiler or source still invalidates reuse. Example after preparing
`word-count` with another `FA` at default size: add
`--additional word-count=<that-directory>/word-count` to the trunk CPU series.
Use `--supplemental` for that diagnostic series.

`--update-readme` replaces only the marked generated result block, preserves the
rest of the README byte-for-byte, links the canonical JSON and is idempotent.
