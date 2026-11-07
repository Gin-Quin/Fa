# Native Fa / Goose audit tools

Tools to compare Fa's native executables with Goose's and to read the code they run. The findings
live in the design records: `design/minimalism-audit.md` (the first audit) and
`design/llvm-backend-plan.md`, "Native representations of the Goose work" (what the backend changed,
its conditions, measured gains and remaining gaps). Outputs go to `benchmarks/audit/build/`
(ignored).

| Tool | What it does |
| --- | --- |
| `compare.py` | Builds Fa and Goose, checks both outputs against Rust's, then alternates timed runs and reports CPU-time medians (`--runs`, `--cpu` pins the processes, `--default-sizes` or the smaller audit sizes). The default selection is the 16 original Goose benchmarks; `--suite secondary` selects the 9 other ports and `--suite all` selects 25. `--profile` adds callgrind instruction counts; `--prepare-only` only builds and validates. `--reuse` validates saved manifests before timing; `--rebuild-fa` rebuilds only Fa after validating Goose and Rust. `--goose-runtime-size` rewrites Goose's constant `N` to an argument. Keeps the manifests, inputs, IR, Goose's C, assembly and JSON report. |
| `build.sh`, `profile.sh`, `report.sh`, `rss.sh` | The first audit's shell pipeline at the sizes of `sizes.txt`: builds with IR, callgrind profiles, per-function reports (`hot.py`), peak RSS. |
| `hot.py`, `show.py` | The hottest instruction runs of a callgrind profile, interleaved with the disassembly. |
| `ir_counts.py` | Static LLVM operation counts per function (not executed instructions). |

```sh
export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
export GOOSE="$HOME/.local/goose/goose" GOOSE_CC="$LLVM_SYS_181_PREFIX/bin/clang"
export CC="$GOOSE_CC"
cargo build --release -p fa
export FA="$PWD/target/release/fa"
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --default-sizes --prepare-only
# wait until builds, tests and other measurements have finished:
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --default-sizes --reuse --cpu 1 --runs 7
# change the compiler, rebuild it, then only Fa against the same Goose binaries:
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --reuse --rebuild-fa --default-sizes --cpu 1 --runs 7 --report after.json
```

The original suite is exactly `binary-trees`, `blur`, `calc`, `graph`, `interp`, `lru`,
`particles`, `push`, `records`, `respond`, `scene`, `sexp`, `strlist`, `sum`, `tree`, and
`words`. Keep secondary ports in a separate report. The main comparison uses the default
sizes from `benchmarks/README.md`, native synchronous release programs, Fa `-O3` with
overflow checks disabled, and Goose `-O2 --standalone` with the **same LLVM/Clang 18**
that Fa uses. The runner passes Goose's C compiler explicitly and records its identity;
an unset `GOOSE_CC` chooses `cc`, which may be GCC and must be reported as such.

Each executable has a manifest: `fa/manifest.json`, `goose/manifest.json`, and
`rust.manifest.json`. It records the effective source hashes after size rewriting,
compiler paths, versions and executable hashes, flags, size and modes, executable hash,
arguments, exact stdout and Rust's reference-output hash. Fa includes its companion
`fa-native` and link-driver identities; Goose includes its C compiler and standard-library hashes. Use a
real `fa` or `fa-native` executable as `FA`; script adapters hide compiler dependencies
and are rejected.

`--reuse` rejects a missing manifest or a change to any of these inputs, the saved source,
the executable, or stdout. It never relabels an old constant-sized binary with a new
size, even when both saved binaries still print the same old output. Rebuild legacy
directories without `--reuse`; do not manufacture manifests for historical binaries.
`--rebuild-fa` permits new Fa source/compiler inputs but still requires the requested
Goose and Rust inputs, size and modes to match their saved builds.

Run timings after builds, tests and profilers finish: on the shared machine, the same executable
varies by up to 10 % between two series. Instruction counts, CPU time and peak RSS are separate
measurements, and a change is judged on all three. With `--goose-runtime-size`, Goose's `blur`
fails its `checkfree` proof: report it, never remove the annotation.
