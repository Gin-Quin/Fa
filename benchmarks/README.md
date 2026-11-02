# Benchmarks

The same programs written in Fa (`<name>/main.fa`), in Rust (`<name>/main.rs`) and in [Goose](https://github.com/aardappel/goose) (`<name>/main.goose`), and a runner that builds them, checks that they print the same output, and reports their run time and binary size: Fa native against Rust and Goose, Fa compiled to WebAssembly, and Fa compiled to JavaScript and run on Bun.

Fifteen of the benchmarks come from [Goose's benchmark suite](https://aardappel.github.io/goose/bench/results.html) (see [the Goose benchmarks](#the-goose-benchmarks)); the [three-way comparison](#fa-rust-and-goose) is at the end.

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

The runner needs `rustc`, `strip` and Bun, and the Goose compiler for the Goose column. Everything it builds goes to `benchmarks/build/<name>/` (git-ignored), including `expected.txt`, the output of the Rust version.

### The Goose side

The runner uses the compiler named by `GOOSE_COMPILER`, else `~/.local/goose/goose`, else `goose` on the `PATH`; without one, the Goose columns show `n/a`, as for a benchmark without `main.goose`. To install it from source (CMake 3.27+, a C++20 compiler; the unity build of `src/main.cpp` takes about 2 GB of memory and several minutes of CPU):

```bash
git clone https://github.com/aardappel/goose ~/src/goose
cmake -S ~/src/goose -B ~/src/goose/build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build ~/src/goose/build -j2
mkdir -p ~/.local/goose && cp ~/src/goose/build/goose ~/.local/goose/ && cp -r ~/src/goose/stdlib ~/.local/goose/
```

The compiler finds its standard library in a `stdlib` folder beside it. For each benchmark, the runner copies `main.goose` and the shared modules of `benchmarks/goose/` (`rng.goose`, the xorshift64 generator of the Goose suite) into `build/<name>/goose/`, and runs:

```bash
goose build -O2 --standalone -o build/<name>/goose/main build/<name>/goose/main.goose
```

which compiles the program to C, then the C (with the Goose runtime, `--standalone`) with a C compiler at `-O2`: `GOOSE_CC` if set (the runs below use `GOOSE_CC=~/.local/llvm18/usr/bin/clang-18`, the LLVM of Fa's native backend), else the one `goose toolchain` finds (`cc`, gcc here). The size is the stripped executable. Goose has no overflow checks outside `--debug` (which also turns off the C optimizations), and keeps the bounds checks it can't prove, like Rust.

A program taken from the Goose suite has its size on a `let N = …; // BENCH_N` line, as in Goose's own harness (Goose programs have no preprocessor): the runner rewrites the number for `--size` and `--quick`, and doesn't pass arguments. The Goose versions of the other benchmarks read their size from their arguments (`args()`, `parse_int`), like the Fa and Rust ones.

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

From the Goose suite (the Fa and Rust versions print what the Goose program prints: one integer per line):

| Benchmark | What it stresses | Default size |
| --- | --- | --- |
| `sum` | the control: an `Array(Int32)` filled from a generator, scanned eight times | 32,000,000 |
| `push` | pushing small objects, keeping the index of every 64th one across the growth | 32,000,000 |
| `strlist` | a text built character by character, split into a list of owned strings, scanned four times | 2,000,000 words |
| `records` | a log of variant records (a named union, one member with a `String`), aggregated four times with `when` | 4,000,000 |
| `tree` | a complete binary tree of `Node?` children (one heap allocation per node), summed eight times | depth 21 |
| `interp` | an expression tree (a recursive named union), evaluated eight times | depth 21 |
| `graph` | adjacency lists (`Array(Array(Integer))`) grown edge by edge, four breadth-first searches | 100,000 vertices, 800,000 edges |
| `words` | a skewed text of random words, split, counted in a `Map(String, Integer)` | 500,000 words |
| `particles` | 200 steps of `Float32` vector math over an array of objects (an `add` method per vector) | 300,000 particles |
| `sexp` | generating an s-expression text, recursive-descent parsing into a tree of unions, four walks | 20,000 forms |
| `lru` | an LRU cache: `Map(Integer, Integer)`, a doubly-linked list in an array of objects, a free list | 2,000,000 operations |
| `scene` | a scene graph of objects owning their children, 16 frames of 3x3 `Float32` transforms | depth 12 |
| `calc` | parsing and evaluating small expressions, a quarter of them malformed: `throws`, `try`, `catch` deep in a recursion | 400,000 expressions |
| `respond` | building and rendering JSON responses (interpolation, escaping), checksummed with 64-bit FNV-1a | 400,000 responses |
| `blur` | 16 passes of a 3x3 blur over a W×W `Array(UInt8)`, indices computed from `y * width + x` | W = 3,000 |

The defaults run in about 0.25 to 2 seconds in Rust (see the [baseline](#rust-baseline)).

### The Goose benchmarks

[Goose](https://github.com/aardappel/goose) is a systems language that compiles to C, with inline data structures, data stacks instead of an allocator, and relative references. Its suite (`bench/goose/`, `bench/rust/`, `bench/cpp/` and the manifest `bench/run_bench.py` of its repository, explained in `bench/design.md`) has sixteen benchmarks. Each was ported here:

- **`main.goose` is the headline Goose implementation, verbatim** (the first `goose=` row of the manifest: `records_var.goose` for `records`, `graph.goose`, `particles.goose`, `respond.goose`, `blur.goose`, …), with only the number of its `// BENCH_N` line changed to our default size. It imports `rng.goose`, the suite's xorshift64, kept in `benchmarks/goose/`.
- **`main.rs`** is adapted from the Goose repository's Rust row whose data structure matches the Fa port (written in each file's header): `Vec` (`sum`, `push`, `graph` nested, `blur` flat indexing), `Vec<String>` (`strlist`), `enum` + `String` (`records`), `Box` nodes (`tree`, `interp`, `sexp`, `calc`, `scene` with `Vec` children), `HashMap` (`words`, `lru`), the DTO (`respond`), elementwise `particles`. The Goose page compares Goose with the fastest safe Rust row, often an index arena, which Fa has no reason to imitate; the rows here are the ones that do the same work as the Fa program.
- **`main.fa`** is an idiomatic Fa port: recursive values through optionals and unions where Goose links nodes with relative references, arrays of objects with integer indices where Goose keeps typed references into a grow-only array (`push`, `lru`), `Map` where Goose uses its open-addressing table or `dictionary`. The generator is `UInt64` with `bitXor`, `shiftLeft` and `shiftRight`, so it is a JavaScript `BigInt` on Bun and wasm's loader: exact, but slow there.
- **`bintrees` is `binary-trees`**: the same Benchmarks Game program. `binary-trees/main.goose` is Goose's `bintrees.goose` with the output of `main.rs`. `words` stays separate from `word-count`: same idea, but a different generator, text and output.
- The other ten benchmarks got a Goose version written for them (`main.goose`, a direct port of `main.rs` reading its size from its arguments): inline `u8[..8]` strings where the words fit (`word-count`, `sort-objects`), `dictionary` for the maps, `sort` with a block for the comparator.

Deviations: `sexp` keeps the children of a list in an `Array` (Rust: `Vec<Node>`) instead of Goose's first-child and next-sibling links; `graph` stores vertex ids as `Integer` (8 bytes) where Rust and Goose use 4; `scene` writes the 3x3 matrix as nine `Float32` fields (Fa has no fixed-size arrays); `respond`, `strlist`, `records`, `words` and `sexp` read the bytes of an (ASCII) string through `codePoints()`, since Fa has no byte access to a `String`.

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

## Compiler

How fast the compiler itself is, and how much memory it takes, on big projects (design/parallel-pipeline.md, step 0). `compiler/generate.ts` writes a Fa package of N files of about L lines, and `compiler/run.ts` measures `fa check`, `fa build --target=bun` and `fa build --target=native` on packages of 100, 1,000 and 2,000 files of 500 lines.

```bash
cargo build --release --bin fa
just bench-compiler                                   # 100, 1,000 and 2,000 files
just bench-compiler --files 100 --commands check      # one size, one command

bun benchmarks/compiler/run.ts [--files <n,n,…>] [--lines <n>] [--seed <n>]
                               [--commands <check,bun,native>] [--memory-cap <MB>] [--target-share <0..1>]
                               [--compare <file.json>]
bun benchmarks/compiler/generate.ts [--files <n>] [--lines <n>] [--seed <n>] [--target-share <0..1>]
                                    [--errors <n>] [--no-cycle] [--output <directory>]
```

| Option | |
| --- | --- |
| `--files <n,n,…>` | the package sizes, in files (default `100,1000,2000`) |
| `--lines <n>` | lines per file (default 500) |
| `--seed <n>` | the seed of the generator: a seed always gives the same package (default 1) |
| `--commands` | which of `check`, `bun` and `native` to measure (default all) |
| `--memory-cap <MB>` | the virtual memory limit of each command (`ulimit -v`, default 4,000 MB): a command that needs more fails with `out of memory` instead of swapping or bringing the machine down |
| `--target-share <0..1>` | the share of generated files that read `target` (default 0.05) |
| `--compare <file.json>` | shows the change of wall time and peak memory against an earlier run |

Each command runs once under `/usr/bin/time -v`, with `--timings`. The runner records the wall time, the **CPU utilization** ((user + system) / wall: the cores used on average), the **peak RSS** and the phases of `--timings` into `benchmarks/results/compiler-<date>.json` (git-ignored), and prints a table. The packages go to `benchmarks/build/compiler/` (git-ignored); `generate.ts` writes there by default too.

**The generated package.** `package.fa` (targets `bun` and `native`), `application/main.fa`, and `library/m0001.fa`, `library/m0002.fa`, …: each file exports a namespace with an object type with a method, a named union, an enumeration and an `Errors` type, then blocks of functions up to the line count: `when` over the union and the enumeration, a loop whose indexing is proven by its range, `throw` and `catch`, a generic function, and calls to the files it uses (functions, and their types through `make(x).score()`). The files form a DAG of 20 levels: a file uses 3 to 8 files of earlier levels, one of them from the level just before. `ringA`, `ringB` and `ringC` call each other in a cycle, and three files of the middle level call them. 5% of the files read `target` (`@if target is "native"`), so `fa check` analyzes the package once per target; every 25th file declares `static` values. `--errors <n>` injects a type error, an unknown name or a `when` without every case into n files. Without errors, `fa check` reports nothing.

**`--timings`** (`fa check` and `fa build`) prints, to stderr after the command:

- the time of each phase: `scan` (listing the files), `configuration` (`package.fa`), `parse` (reading and parsing every file), `analysis`, `render` (the diagnostics), `generation`, `write` and `minify` for JavaScript, `llvm` (generation, optimization and link) for native targets; a phase run several times (`fa check` once per target) adds up, with its count. For a standalone file, the files are parsed during the analysis.
- the units (files, until the analysis is scheduled by strongly connected components) and the largest one,
- the number of types in the type store,
- the nodes of the trees and the bytes they hold (nodes, spans and sources), and the bytes of the per-node side tables of the analyses (types, resolutions, flags; the maps keyed by node are not counted),
- the peak RSS of the process.

### Compiler baseline

Measured on 2026-11-02 at `fc9f174` (before any step of the parallel pipeline) with `just bench-compiler` (seed 1, 500 lines per file, a 4,000 MB cap), on the 6-core virtual machine of the [Rust baseline](#rust-baseline). The machine was busy (load average 9 to 12 on 6 cores), so the wall times are about twice what an idle machine gives, and the CPU column shows the time `fa` waited for a core rather than any parallelism: `fa` runs on one thread. The **CPU time** (user + system) and the **peak RSS** are the stable figures: an earlier run at a load of 9 gave walls of 3.16 s and 29.7 s for `fa check` at 100 and 1,000 files, for CPU times within 10% of these and the same RSS.

| Package | Command | Status | Wall | CPU | CPU time | Peak RSS | Phases (CPU time of the phase, roughly) |
| --- | --- | --- | ---: | ---: | ---: | ---: | --- |
| 100 files (1.9 MB) | `fa check` | ok | 7.08 s | 0.29 cores | 2.07 s | 152 MB | parse 0.41 s, analysis 6.43 s (×2 targets) |
| | `fa build --target=bun` | ok | 4.75 s | 0.36 cores | 1.70 s | 259 MB | parse 0.33 s, analysis 2.46 s, generation 1.73 s, write 0.04 s |
| 1,000 files (19.3 MB) | `fa check` | ok | 60.67 s | 0.38 cores | 22.97 s | 1,454 MB | parse 4.71 s, analysis 54.69 s (×2 targets) |
| | `fa build --target=bun` | **out of memory** | 42.61 s | 0.53 cores | 22.48 s | > 4,045 MB | failed during generation |
| 2,000 files (38.5 MB) | `fa check` | **out of memory** | 68.44 s | 0.45 cores | 30.72 s | > 2,551 MB | the second target's analysis can't reserve the 1 GiB stack of the compile-time thread |
| | `fa build --target=bun` | **out of memory** | 82.10 s | 0.43 cores | 35.04 s | > 3,927 MB | |
| all | `fa build --target=native` | unsupported | | | | | the LLVM backend compiles one standalone file for now (`cli/src/lib.rs`) |

What `--timings` reports at 1,000 files: 1,005 units (files), the largest 4,000 nodes; 193,438 types in the store; 3.9 M nodes, so **466.8 MB of trees** (64 B per node, 16 B per span, the sources) and 78 MB of per-node side tables, retained until the end.

Findings:

- **One core.** Every phase is sequential; CPU time grows linearly with the files (2.1 s, 23 s).
- **Memory grows with the project**, at about 0.7 MB per file of 500 lines for one analysis: `fa check --target-share 0` (no file reads `target`) on 1,000 files takes 10.7 s of CPU and 741 MB. The trees and side tables are 545 MB of it.
- **`fa check` analyzes the whole package again for each target** as soon as one file reads `target` (5% of the files here): it doubles the CPU time and, since both analyses stay alive, the memory (§4 #19 of the plan).
- **JavaScript generation costs more memory than the analysis**: 259 MB against 152 MB at 100 files, and above 4 GB at 1,000 files, where `fa check` needs 1.45 GB (the per-file copies of §4 #13–14).
- Two analyzer bugs found while writing the generator, worked around in it: a cycle between three files that export namespaces gives `E-DECL-1` (a member of a namespace whose analysis is in progress is not found), and two object types of the same structure declared in different files lose the methods of one of them (`E-OBJ-5`). The JavaScript of a package whose namespaces declare types or `static`s doesn't run (`"m0001$Item" is not declared in this file`, from `fa run`).
