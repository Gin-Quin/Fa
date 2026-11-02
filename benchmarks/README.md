# Benchmarks

The same programs written in Fa (`<name>/main.fa`) and in Rust (`<name>/main.rs`), and a runner that builds them, checks that they print the same output, and reports their run time and binary size: Fa native against Rust, Fa compiled to WebAssembly, and Fa compiled to JavaScript and run on Bun.

## Running

```bash
just bench                    # every benchmark
just bench fib nbody          # some of them
just bench --quick            # tiny sizes: checks that everything builds and agrees, in seconds

bun benchmarks/run.ts [<name>...] [--runs <n>] [--size <n> | --quick] [--compare <file.json>]
```

| Option | |
| --- | --- |
| `--runs <n>` | timed runs per program (default 3); the table shows the median |
| `--size <n>` | the size argument given to every selected program (default: each program's own) |
| `--quick` | a small size per benchmark, set in `run.ts` |
| `--compare <file.json>` | shows the change of every time and size against an earlier run |

Every run writes `benchmarks/results/latest.json` (git-ignored). To compare a change, keep a copy first:

```bash
cp benchmarks/results/latest.json benchmarks/results/before.json
# ... change the compiler, rebuild it ...
just bench --compare benchmarks/results/before.json
```

The runner needs `rustc`, `strip` and Bun. Everything it builds goes to `benchmarks/build/<name>/` (git-ignored), including `expected.txt`, the output of the Rust version.

### The Fa side

The runner uses `target/release/fa` (`cargo build --release -p fa`), or the compiler named by the `FA_COMPILER` environment variable. While it is missing, the Fa columns show `n/a`. For each target, it runs:

```bash
fa build benchmarks/<name>/main.fa --target=<native|wasm|bun> --output=<directory>
```

and looks in the directory for:

| Target | Output | Run with | Size reported |
| --- | --- | --- | --- |
| `native` | an executable | the executable | stripped (`strip` on a copy) |
| `wasm` | a `.wasm` file and a `.js` or `.mjs` loader | `bun <loader>` | the `.wasm` file |
| `bun` | a `.js` or `.mjs` file | `bun <file>` | the `.js` file |

The `--output` option is the runner's assumption: the documentation gives `fa build [<package> | <file.fa>] [--target=...]` and an `output` directory in the build configuration, but no command-line option yet. If the CLI ends up different, adapt `buildFa` in `run.ts`.

A failure doesn't stop the run: the cell shows `build fail`, `no output`, `run fail` or `mismatch` (the output differs from Rust's), and the reason is printed under the table.

## The benchmarks

| Benchmark | What it stresses | Default size |
| --- | --- | --- |
| `fib` | recursive calls, integer additions with overflow checks | `fib(42)` |
| `nbody` | float arithmetic and `sqrt` on an array of objects, mutation through loop references and `use` aliases | 15,000,000 steps |
| `spectral-norm` | nested loops over float arrays, integer-to-float conversions, mutated array parameters | 4,000 |
| `mandelbrot` | float loops with an early exit, integer packing; prints a checksum instead of the image | 4,000 × 4,000 |
| `fannkuch-redux` | small integer arrays, data-dependent indices (`try` on unproven indexing), array copies | 10 |
| `binary-trees` | allocating and destroying many small recursive values (`Tree?` children, stored on the heap) | depth 17 |
| `string-building` | interpolation, appending to a mutable string, `padStart`, `split`, `join`, `length` | 3,000,000 entries |
| `word-count` | `Map(String, Integer)` and `Map(Integer, Integer)`: `getOrInsert`, iteration | 5,000,000 words and numbers |
| `sort-objects` | `sortInPlace` on an array of objects with a three-key comparator, then on integers | 1,000,000 |
| `sieve` | a large `Array(Boolean)`, strided writes, a linear scan | primes up to 100,000,000 |

The defaults run in about 0.25 to 2 seconds in Rust (see the [baseline](#rust-baseline)).

## Fairness rules

- **Same algorithm, same output.** Both versions do the same work in the same order, including the floating-point operations (the order of the operands matters for the last bits), and print exactly the same text. The runner compares every output with the Rust one before timing.
- **Idiomatic, safe Rust.** No `unsafe`, no external crates, no `get_unchecked`: indexing is bounds-checked as usual, `HashMap` keeps its default hasher, sorting uses `sort_by`. Rust may use its standard library where Fa has an equivalent operation (`rotate_left`, `reverse` on a slice, `vec![value; n]`).
- **Idiomatic Fa**, per the documentation: what the type checker should accept, without tricks to help the compiler.
- **Release builds.** Rust is built with `rustc --edition 2024 -C opt-level=3`. The `Rust ovf` column adds `-C overflow-checks=on`, which gives Rust the overflow semantics of Fa (an overflow stops the program); the ratio is computed against plain `-C opt-level=3`, the usual Rust release build. `fa build` doesn't check integer overflows by default (they wrap, as with `rustc -O`), so the `Fa native` column compares with plain Rust; the `Fa ovf` column is Fa native built with `--overflow-checks=true`, to compare with `Rust ovf`.
- **Sizes are stripped.** `Rust size` is a build with `-C opt-level=3 -C panic=abort -C strip=symbols`, the smallest without changing the code; `Rust default` is the `-C opt-level=3` build, stripped; the Fa executable is stripped too. The JSON also keeps the unstripped size of the default Rust build.
- **Wall-clock time** of the whole process, measured by the runner: the median of `--runs` runs, after one untimed run that checks the output and warms the caches.
- **Deterministic and portable output.** The programs print integers and strings only, never floats, whose formatting differs between languages: a float result is printed rounded to 9 decimals as an integer (`Integer.round(value * 1e9)`, Rust's `(value * 1e9).round() as i64`, both rounding halfway cases away from zero). Every intermediate integer stays below 2^53, so that an `Integer` never stops the program on JavaScript targets: checksums are reduced modulo 1,000,000,007, the random numbers come from the Park-Miller generator (`state * 48271 %% 2147483647`), and the sieve checks `i <= length // i` before computing `i * i`. Map iteration order is not specified, so the map results are order-independent (sums, and a maximum with a tie-break).

## Writing a Fa benchmark

Each `main.fa` is a standalone file: its top-level statements run in order. It reads its size from `arguments`, as the [hello world guide](../documentation/guides/hello-world.md) shows:

```fa
let n = try Integer.parse(arguments.get(0) ?? "42")
```

- `arguments.get(0)` is the first argument after the program, following the guide (`fa run hello.fa Ada` reads `Ada` with `arguments.get(0)`); index 0 is the program itself, as in C and Rust.
- `Integer.parse` comes from [type conversion](../documentation/types/type-conversion.md) and `throws ParseError`: an invalid size escapes the top level and ends the program with the exit code 1.
- The same `n` is the first argument of the Rust version, read with `std::env::args().nth(1)`.

Indexing uses the forms the [range analysis](../documentation/values/numbers.md#range-analysis) proves (`for item, index in array`, `for index in 0..array.length`, a literal array of known length indexed by `i %% array.length`), and `try` where an index comes from data (`fannkuch-redux`, the random syllables). Other choices made where the documentation is silent or ambiguous:

- **Starting a range elsewhere than 0.** `for j in i + 1..bodies.length` (`nbody`) and `for i in 2..isPrime.length` (`sieve`) are assumed to prove `j < bodies.length` like `0..bodies.length` does; the range analysis only names `for index in 0..collection.length`.
- **`while` as a guard.** `while multiple < isPrime.length { isPrime[multiple] = false ... }` (`sieve`) is assumed to prove the index like `if multiple < isPrime.length`; the range analysis names `if` guards, and lists `while` among the conditions that refine intervals.
- **Filling an array.** There is no documented way to create an array of `n` copies of a value (Rust's `vec![value; n]`): the programs push in a loop, with `Array(Boolean).from([], capacity = n)` when the size is known.
- **Copying into an existing array.** `current = permutation` (`fannkuch-redux`) copies the array each iteration; a good compiler reuses the storage of `current`, as Rust's `copy_from_slice` does.
- **No float formatting.** The documentation doesn't say how a `Float` is printed (`toString`, decimals), hence the rounding to integers above.
- **`Integer.parse` or `Integer.fromString`.** Both names appear in the documentation (type conversion, and the build-your-application and web-server guides); the programs use `Integer.parse`.

Every `main.fa` parses with the current parser (`cargo build -p fa-parser`, then `target/debug/fa-parser benchmarks/<name>/main.fa`).

## Rust baseline

Measured on 2026-11-01 with `bun benchmarks/run.ts` (3 runs, median), rustc 1.99.1, Linux 6.8 on a 6-core AMD EPYC virtual machine. The machine was shared with other jobs (load average around 7), so the times vary by up to 40% between runs: `nbody`, `sieve` and `word-count` even ran faster with overflow checks than without. Use them as orders of magnitude, and compare runs made on the same machine at the same time.

| Benchmark | Rust | Rust ovf | Rust size | Rust default (stripped) | Rust default (unstripped) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `binary-trees` | 0.93 s | 0.85 s | 332.6 KiB | 334.9 KiB | 4.33 MiB |
| `fannkuch-redux` | 0.24 s | 0.24 s | 332.1 KiB | 333.7 KiB | 4.33 MiB |
| `fib` | 0.94 s | 1.38 s | 331.0 KiB | 332.5 KiB | 4.33 MiB |
| `mandelbrot` | 1.15 s | 1.16 s | 331.4 KiB | 332.9 KiB | 4.33 MiB |
| `nbody` | 1.78 s | 1.32 s | 335.9 KiB | 338.6 KiB | 4.34 MiB |
| `sieve` | 1.44 s | 0.95 s | 331.6 KiB | 333.1 KiB | 4.33 MiB |
| `sort-objects` | 0.91 s | 0.92 s | 350.4 KiB | 352.3 KiB | 4.36 MiB |
| `spectral-norm` | 1.07 s | 1.27 s | 333.7 KiB | 335.4 KiB | 4.34 MiB |
| `string-building` | 0.88 s | 0.96 s | 336.4 KiB | 338.1 KiB | 4.34 MiB |
| `word-count` | 2.25 s | 1.39 s | 348.1 KiB | 350.7 KiB | 4.36 MiB |

The outputs, which every Fa target must reproduce, are in `benchmarks/build/<name>/expected.txt` after a run. Known values check the Rust versions: `fannkuch-redux` prints `73196` and `Pfannkuchen(10) = 38`, `nbody` ends at an energy of `-169083134 (x 1e-9)` after 5,000,000 steps, and the sieve finds 5,761,455 primes below 100,000,000.

Most of a Rust binary is its standard library (formatting, panics, the allocator): about 330 KiB stripped whatever the program, which is the bar for Fa's "tiny runtime".
