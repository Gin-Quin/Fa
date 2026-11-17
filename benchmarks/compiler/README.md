# Checker speed audit

From the repository root, with Bun and Valgrind installed as in AGENTS.md:

```sh
export PATH="$HOME/.bun/bin:$HOME/.local/bin:$PATH"
bun install --frozen-lockfile --cwd packages/benchmark-typescript
# One command builds release Fa, regenerates both inputs, independently checks
# the TS with pinned TypeScript 7, measures, profiles and checks the own corpus:
taskset -c 0-4 env -u LLVM_SYS_181_PREFIX python3 benchmarks/compiler/check_speed.py --report benchmarks/results/check-speed.json
```

The command writes JSON and a generated Markdown table beside it. Default sizes
are one standalone file, 100, 1,000 and 2,000 library files, approximately 500 Fa
lines each. Packages also have three cyclic namespace files, an entry and a Fa
configuration; JSON records exact file/line/byte counts and SHA-256s of the inputs.
`--files 1,100 --lines 100 --runs 3` is a smoke run, never an acceptance result.
`--mode prepare|measure|profile|corpus|render` resumes one stage using the report.
Keep the first primary report: use a different `--report` for a supplemental run.
There is no outlier removal. Resumed runs reject changed executables and inputs.

`paired.ts` renders the existing Fa generator's restricted fixture templates as
TypeScript. It preserves seeded constants, functions, loops, union/enum cases,
call edges and cycles, rather than generating unrelated code of equal line count.
The main comparison uses targetShare=0: both check one target-independent program.
The original `generate.ts` and `run.ts` retain their multi-target benchmark modes.
TS uses a class for Fa's object with a method, discriminated unions, an enum,
generic arrays, throw/catch and ordinary constants for Fa's compile-time statics.
It performs less proof work: TS does not check thrown-error sets, ownership or
integer overflow/bounds. These are equivalent algorithmic workloads, not identical
language guarantees. Neither fixture tests InlineArray or explicit owner-path
references; those are covered by the compiler's correctness suites.

`bun check` is Bun's built-in checker. The driver requires its
`process.versions.typescript` to match the repository's pinned native TS7 version
(7.0.2 in this audit). The TypeScript 6 API package exists for API clients and is
not the checker selected by `bun check`. Each generated TS project is also checked
with the installed TS7 binary, without incremental state. See
[Bun's checker documentation](https://bun.com/docs/runtime/check).

Each mode warms both tools once, then runs at least eleven rounds, interleaving
Fa, Bun and the **identical Fa binary** under the control label. Order reverses
every round and rotates every two rounds. Headline: default worker selection on
CPUs 0–4, wall-time Fa/Bun <=1.20, on a shared machine. Reference: one worker on
CPU 5 (`RAYON_NUM_THREADS=1`, `bun check --threads=1`), recording CPU cost as well as
wall time. All preparation, builds, tests and profilers use CPUs 0–4. The driver
itself runs on CPUs 0–4. Each child retains microsecond user/system CPU via wait4,
wall time and kernel peak RSS; Fa checks are monitored against 512 MiB RSS. Bun's
RSS is reported without imposing Fa's bound. Child output must be empty and exit
zero in every measured run. Bun's variable success summary on stderr is excluded.

Distributions use Q1/median/Q3 with type-7 linear interpolation. Both ratio of
medians and median of per-round ratios are retained. A result within ±0.05 of
1.20 requires the identical-binary control to differ by strictly less than 3% on
both estimators for that metric; straddling estimators are unresolved. Controls
and load snapshots remain visible for large-margin results too. Fewer than eleven
rounds are explicitly labeled smoke only. Every invocation starts a fresh process:
only OS filesystem pages survive. Neither a daemon nor an incremental TS Program
nor `.tsbuildinfo` is reused. Fa's interners/interfaces/parse tree retention and
compile-time caches exist only within that invocation; Bun also loads its built-in
standard declarations each time.

The largest fixture gets separate `fa check --timings` runs for broad wall phases
and Callgrind (one worker on CPUs 0–4) for function instruction attribution.
Exclusive categories are disjoint: parsing/reading, scanning/resolution, type
checking, flow/effects, references/moves/relations, compile time, rendering,
pipeline/session and runtime/other. They are **instruction attribution**, not
inferred wall milliseconds; optimized inlining charges code to its surviving
caller. `--timings` phases overlap (pipeline contains parse and analysis), so they
must not be added together. Raw Callgrind and annotation files stay in
`benchmarks/build/check-speed/profile/`; hashes, function counts and phase text
are retained in JSON. Profiling memory includes Valgrind and is not checker RSS.

The corpus sanity pass checks every tracked `.fa` file individually in its normal
repository context, sequential fresh processes, CPU 5/one worker. Intentional
negative fixtures keep exit 1; abnormal exit, timeout and RSS failure are retained.
Its aggregate includes repeated startup/package discovery and expected diagnostics,
so it is not a throughput ratio against the generated positive TS fixtures.

Validation:

```sh
taskset -c 0-4 python3 -m unittest discover -s benchmarks/compiler -p 'test_*.py'
taskset -c 0-4 bun check -p benchmarks/compiler
taskset -c 0-4 bun test benchmarks/compiler/paired.test.ts
```
