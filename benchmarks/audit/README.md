# Native Fa / Goose audit tools

Tools to compare Fa's native executables with Goose's and to read the code they run. The findings
live in the design records: `design/minimalism-audit.md` (the first audit) and
`design/llvm-backend-plan.md`, "Native representations of the Goose work" (what the backend changed,
its conditions, measured gains and remaining gaps). Outputs go to `benchmarks/audit/build/`
(ignored).

| Tool | What it does |
| --- | --- |
| `compare.py` | Builds Fa and Goose for the given benchmarks (all by default), checks both outputs against Rust's, then alternates timed runs and reports CPU-time medians (`--runs`, `--cpu` pins the processes, `--default-sizes` or the smaller audit sizes). `--profile` adds callgrind instruction counts; `--reuse` times saved builds, `--rebuild-fa` rebuilds only Fa; `--goose-runtime-size` gives Goose its `N` at run time, as Fa reads it. Keeps the IR, Goose's C, the assembly and a JSON report. |
| `build.sh`, `profile.sh`, `report.sh`, `rss.sh` | The first audit's shell pipeline at the sizes of `sizes.txt`: builds with IR, callgrind profiles, per-function reports (`hot.py`), peak RSS. |
| `hot.py`, `show.py` | The hottest instruction runs of a callgrind profile, interleaved with the disassembly. |
| `ir_counts.py` | Static LLVM operation counts per function (not executed instructions). |

```sh
export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
export GOOSE="$HOME/.local/goose/goose" GOOSE_CC="$LLVM_SYS_181_PREFIX/bin/clang"
cargo build --release -p fa
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --default-sizes --cpu 1 --runs 7
# change the compiler, rebuild it, then only Fa against the same Goose binaries:
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --reuse --rebuild-fa --default-sizes --cpu 1 --runs 7 --report after.json
```

Run timings after builds, tests and profilers finish: on the shared machine, the same executable
varies by up to 10 % between two series. Instruction counts, CPU time and peak RSS are separate
measurements, and a change is judged on all three. With `--goose-runtime-size`, Goose's `blur`
fails its `checkfree` proof: report it, never remove the annotation.
