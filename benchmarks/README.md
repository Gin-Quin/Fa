# Benchmarks

The same programs written in Fa (`<name>/main.fa`), in Rust (`<name>/main.rs`) and in [Goose](https://github.com/aardappel/goose) (`<name>/main.goose`), and a runner that builds them, checks that they print the same output, and reports their run time and binary size: Fa native against Rust and Goose, Fa compiled to WebAssembly, and Fa compiled to JavaScript and run on Bun.

Fifteen of the benchmarks come from [Goose's benchmark suite](https://aardappel.github.io/goose/bench/results.html) (see [the Goose benchmarks](#the-goose-benchmarks)); the [three-way comparison](#fa-rust-and-goose) is at the end.

## Running

```bash
just bench                    # every benchmark
just bench fib nbody          # some of them
just bench --quick            # tiny sizes: checks that everything builds and agrees, in seconds

bun benchmarks/run.ts [<name>...] [--runs <n>] [--size <n> | --quick] [--compare <file.json>] [--output <file.json>]
```

| Option | |
| --- | --- |
| `--runs <n>` | timed runs per program (default 3); the table shows the median |
| `--size <n>` | the size argument given to every selected program (default: each program's own) |
| `--quick` | a small size per benchmark, set in `run.ts` |
| `--compare <file.json>` | shows the change of every time and size against an earlier run |
| `--output <file.json>` | where to write the results (default `benchmarks/results/latest.json`) |

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

#### What the ports found missing or wrong in Fa

Found with `target/release/fa` at the commit of the [three-way comparison](#fa-rust-and-goose). The programs use the workaround named, and stay idiomatic otherwise.

Wrong results:

- **`c is ")"` on a `Character` (or `Character?`) is true for any character**, natively and on Bun: it compiles to a type test (`typeof x === "string"` in JavaScript) instead of comparing the value. `calc` and `sexp` use `==`.

Not implemented:

- ~~**String slicing on the LLVM targets**~~: `text[a..b]` builds natively since the fifth native shift; `sexp` and `calc` keep their loops (they were written without it, and stay comparable with the earlier runs).
- **`while let`**: `while let vertex = queue.get(head) { … }` gives `AN001: while let is not supported yet by the analyzer` (`graph` uses `while head < queue.length`).
- **A tuple type written `(UInt64, Integer)`** as a return type gives `AN001`; `Tuple(UInt64, Integer)` works.

Rejected although the documentation allows it:

- **A `while` guard is forgotten when the array grows in the loop**: `while head < queue.length { let v = queue[head]; head += 1; queue.push(x) }` gives ``E-ERR-3: `queue[head]` can fail with `IndexError` `` (without the `push`, it is accepted). `graph` writes `try queue[head]`.
- **`throws IndexError | DivisionByZero`**: a `try` of either error gives `E-ERR-4` ("doesn't declare … `IndexError.IndexError | DivisionByZero.DivisionByZero`"). `calc` catches the division error where it happens.
- **Narrowing**: `c == " "` with `c: Character?` gives `E-CMP-1` even after `if c is none { return }`; a field is not narrowed by `if node.key < 0 { break }` (then `walk.wrappingAdd(node.key)` gives `E-NUM-14`; copied to a local first); `is not "+" | "-"` doesn't narrow a `Character` to the remaining literals.
- **`if p { aString } else { aCharacter }`** gives `E-IF-2` (a `String` and a `Character` "don't make a valid union").

Misleading diagnostics:

- `W-NUM-1` on `Integer(value).bitXor(pass)` (`value: Int32`) says the conversion "does nothing", but without it `value.bitXor(pass)` fails with `E-NUM-14` (`pass` doesn't fit in an `Int32`).
- `W-ERR-1` says a string slice "can't fail", while the documentation says it throws `IndexError`.

Missing in the library: **a cheap way to read a character's code or a string's bytes**. `codePoints()` allocates an `Array(UInt32)` per call (FNV-1a over a response in `respond`, `strlist`'s `codes` local); natively, `text.codePoints()[i]` and `text.codePoints().get(i)` read the code point without building the array (`sexp`, `records`).

Slow (the [comparison](#fa-rust-and-goose) has the numbers), and what the fifth native shift did about it (`design/decisions.md`, "Simple strings" and the entries after it):

- **Indexing a string natively was linear**: it is constant time on an ASCII string (the `SIMPLE` flag of the capacity word), and a character of an ASCII string is one of 128 static strings, so `text.characters()` no longer allocates per character either. The parsers still read `text.characters()` once, as written.
- **Appending one character at a time** (`text += letters[…]`): no allocation for the character nor the template any more, an inlined append into the block (`strlist` 3.05 s → 0.56 s of CPU).
- **`blur`**: the common path of a check is `0 <= i < length`, which LLVM's range check elimination takes out of the main loop, the destination's header is no longer reloaded (scalar TBAA), and the byte sums are 16-bit lanes: it vectorizes, as fast as Rust.
- **`tree`**: the box of a `Node?` field is allocated before its value is computed, so the tree is laid out in the order the walk visits it (the Rust version allocates parents first too; allocating after the children makes it 3x slower).
- **On Bun**, the `UInt64` generator is a `BigInt` (exact, as documented, but it dominates the Goose-suite programs there), and `particles` creates an object for every vector `add` (the likely cause of its time there, not profiled).

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

## Fa, Rust and Goose

Measured on 2026-11-02 with `GOOSE_CC=~/.local/llvm18/usr/bin/clang-18 bun benchmarks/run.ts --runs 3` (one runner process, every benchmark at its default size, under `ulimit -v 4000000`): rustc 1.99.1, Bun 1.4.4, `fa` built with LLVM 18 at the end of the fifth native shift (`4c77c21`), Goose `eadca40` (its C compiled by clang 18.1.3 at `-O2`), on the 6-core AMD EPYC virtual machine of the [Rust baseline](#rust-baseline). **The machine was busy** (load average 10 to 16 on 6 cores from other jobs), so a time can be off by 30% or more: the CPU times measured alone for the shift's changes are lower for Fa's `calc` (0.86 s) and `sexp` (0.69 s). Read the ratios as orders of magnitude, and re-run a benchmark alone before drawing a conclusion from one cell. The previous run (fourth shift, `bcbefe4`) had Fa / Rust 2.16x and Fa / Goose 3.96x on the Goose suite.

All the implementations of a benchmark printed the same output: the runner compares every one with Rust's before timing it.

| Benchmark | Rust | Fa native | Goose | Fa / Rust | Goose / Rust | Fa / Goose | Rust ovf | Fa ovf | Fa wasm | Fa Bun | Rust size | Fa size | Goose size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fib` | 1.17 s | 1.26 s | 1.32 s | 1.07x | 1.13x | 0.95x | 1.66 s | 1.70 s | 2.81 s | 3.00 s | 331.0 KiB | 18.3 KiB | 18.4 KiB |
| `nbody` | 1.04 s | 1.22 s | 1.29 s | 1.18x | 1.24x | 0.95x | 0.89 s | 1.19 s | 2.05 s | 2.78 s | 335.9 KiB | 23.8 KiB | 18.5 KiB |
| `spectral-norm` | 1.85 s | 1.79 s | 1.53 s | 0.97x | 0.83x | 1.17x | 2.72 s | 2.00 s | 2.64 s | 3.46 s | 333.7 KiB | 22.5 KiB | 22.5 KiB |
| `mandelbrot` | 1.35 s | 1.61 s | 1.49 s | 1.19x | 1.10x | 1.08x | 1.83 s | 1.42 s | 1.96 s | 1.75 s | 331.4 KiB | 20.8 KiB | 18.4 KiB |
| `fannkuch-redux` | 0.35 s | 0.33 s | 0.36 s | 0.95x | 1.04x | 0.91x | 0.33 s | 0.40 s | 0.67 s | 0.67 s | 332.1 KiB | 22.3 KiB | 18.5 KiB |
| `binary-trees` | 1.18 s | 0.67 s | 0.14 s | 0.57x | 0.12x | 4.73x | 1.07 s | 0.60 s | 0.87 s | 1.18 s | 332.6 KiB | 25.0 KiB | 18.4 KiB |
| `string-building` | 1.35 s | 0.95 s | 0.22 s | 0.70x | 0.16x | 4.31x | 1.34 s | 1.02 s | 1.19 s | 1.92 s | 336.4 KiB | 36.9 KiB | 18.5 KiB |
| `word-count` | 1.12 s | 0.84 s | 0.38 s | 0.75x | 0.34x | 2.20x | 1.12 s | 0.82 s | 1.31 s | 4.47 s | 348.1 KiB | 29.2 KiB | 22.5 KiB |
| `sort-objects` | 1.91 s | 2.69 s | 0.53 s | 1.41x | 0.28x | 5.06x | 1.95 s | 2.03 s | 4.68 s | 8.54 s | 350.4 KiB | 31.2 KiB | 34.5 KiB |
| `sieve` | 1.59 s | 1.66 s | 1.98 s | 1.04x | 1.24x | 0.84x | 1.47 s | 1.96 s | 2.00 s | 2.79 s | 331.6 KiB | 19.7 KiB | 18.5 KiB |
| `sum` | 0.28 s | 0.25 s | 0.30 s | 0.90x | 1.07x | 0.83x | 0.47 s | 0.53 s | 0.44 s | 16.81 s | 332.7 KiB | 18.9 KiB | 18.4 KiB |
| `push` | 0.43 s | 0.63 s | 0.41 s | 1.46x | 0.93x | 1.56x | 0.43 s | 0.59 s | 1.90 s | failed¹ | 332.4 KiB | 19.2 KiB | 18.4 KiB |
| `strlist` | 0.70 s | 0.80 s | 0.19 s | 1.14x | 0.27x | 4.27x | 0.59 s | 0.65 s | 2.06 s | 17.98 s | 333.2 KiB | 26.2 KiB | 18.4 KiB |
| `records` | 0.44 s | 0.49 s | 0.22 s | 1.10x | 0.50x | 2.18x | 0.55 s | 0.42 s | 5.49 s | 14.37 s | 333.5 KiB | 23.2 KiB | 18.4 KiB |
| `tree` | 0.45 s | 0.38 s | 0.49 s | 0.83x | 1.08x | 0.77x | 0.41 s | 0.38 s | 10.64 s | 7.54 s | 331.7 KiB | 18.7 KiB | 18.4 KiB |
| `interp` | 0.23 s | 0.19 s | 0.06 s | 0.82x | 0.24x | 3.40x | 0.30 s | 0.18 s | 0.71 s | 2.47 s | 332.1 KiB | 22.0 KiB | 18.4 KiB |
| `graph` | 0.32 s | 0.38 s | 0.53 s | 1.19x | 1.66x | 0.72x | 0.31 s | 0.43 s | 0.75 s | 2.37 s | 333.8 KiB | 21.3 KiB | 18.4 KiB |
| `words` | 0.19 s | 0.22 s | 0.07 s | 1.16x | 0.36x | 3.24x | 0.17 s | 0.19 s | 0.46 s | 1.91 s | 346.9 KiB | 29.6 KiB | 18.5 KiB |
| `particles` | 0.15 s | 0.13 s | 0.24 s | 0.88x | 1.60x | 0.55x | 0.18 s | 0.19 s | 0.47 s | 18.56 s | 332.2 KiB | 20.2 KiB | 18.4 KiB |
| `sexp` | 0.50 s | 0.85 s | 0.28 s | 1.69x | 0.55x | 3.06x | 0.47 s | 0.84 s | 10.28 s | 13.49 s | 334.6 KiB | 35.0 KiB | 18.5 KiB |
| `lru` | 0.84 s | 1.05 s | 0.36 s | 1.26x | 0.42x | 2.97x | 1.12 s | 1.25 s | 2.07 s | 7.34 s | 346.8 KiB | 24.5 KiB | 18.5 KiB |
| `scene` | 0.16 s | 0.18 s | 0.04 s | 1.12x | 0.25x | 4.44x | 0.13 s | 0.19 s | 0.50 s | 1.53 s | 334.3 KiB | 24.6 KiB | 18.5 KiB |
| `calc` | 0.78 s | 1.51 s | 0.29 s | 1.94x | 0.37x | 5.28x | 0.76 s | 0.94 s | 2.28 s | 28.90 s | 335.1 KiB | 42.9 KiB | 22.5 KiB |
| `respond` | 0.58 s | 0.58 s | 0.41 s | 1.00x | 0.71x | 1.42x | 0.68 s | 0.82 s | 1.16 s | 38.02 s | 336.8 KiB | 38.8 KiB | 22.4 KiB |
| `blur` | 0.09 s | 0.12 s | 0.21 s | 1.34x | 2.32x | 0.58x | 1.04 s | 1.83 s | 1.00 s | 10.34 s | 334.5 KiB | 23.9 KiB | 18.4 KiB |
| geometric mean | | | | 1.07x | 0.61x | 1.76x | | | | | | | |

¹ Bun crashed on `push` (32 M objects under the 4 GB address-space cap: a `RangeError: Out of memory` in the earlier run). The other Bun times of the Goose-suite programs are dominated by the `UInt64` generator, a `BigInt` there.

Ratios are of times: below 1, the first one is faster. Sizes are stripped executables (Rust: `-C panic=abort -C strip=symbols`). Goose has no overflow checks in an optimized build, so it has no "ovf" column.

What it says:

- **Fa native is as fast as Rust** overall (geometric mean 1.07x): 1.01x on the original ten and 1.10x on the Goose suite (2.16x before the fifth shift). The shift closed the gaps of `blur` (31x → 1.3x: it vectorizes), `strlist` (6.7x → 1.1x), `sexp` (5.7x → 1.7x), `tree` (3.8x → 0.8x), `records` (3.5x → 1.1x) and `calc` (3.3x → 1.9x; 1.1x in CPU time alone). Fa is ahead of Rust where Rust pays for its allocator and formatting (`binary-trees`, `string-building`, `word-count`).
- **Goose is the fastest of the three** on most programs: 0.61x Rust overall, 0.57x on its own suite against the Rust rows matching the Fa ports. Its wins are its design (no allocator, no teardown, inline strings, arenas): `binary-trees` and `string-building` 6 to 8x faster than Rust, `scene` and `interp` 4x. It is slower than Rust on `blur` (2.3x), `graph`, `particles`, `sieve` and `nbody`.
- **Fa is 1.76x slower than Goose** overall (geometric mean; 2.8x before), 1.92x on the Goose suite (3.96x before) and 1.51x on the original ten. Fa is ahead of Goose on `sum`, `tree`, `graph`, `particles`, `blur`, `fib`, `nbody`, `fannkuch-redux` and `sieve`; behind it where Goose's arenas and inline strings avoid allocations (`calc`, `sort-objects`, `binary-trees`, `scene`, `strlist`, `string-building`, `interp`, `words`, `sexp`, `lru`).
- **Size**: Fa's native executables (15 to 45 KiB) and Goose's (18 to 35 KiB, runtime included with `--standalone`) are both an order of magnitude below Rust's 330 KiB.

### Against Goose's published results

[Goose's results page](https://aardappel.github.io/goose/bench/results.html) (2026-10-16; Ryzen 9 9950X3D, Windows 11, clang-cl 22 `/O2`, rustc 1.98 `-O -C codegen-units=1`) compares Goose with the fastest safe Rust row of each benchmark at its large size (the "vs rust (clang)" column, inverted here into a ratio of times). This run uses the Rust row whose data structure matches the Fa port, at our default size, on Linux with clang 18:

| Benchmark | Goose / Rust, published | Goose / Rust, here | Rust row here | Fa / Rust, here | Fa / Goose, here |
| --- | ---: | ---: | --- | ---: | ---: |
| `sum` | 1.02x | 1.07x | `Vec`, collect | 0.90x | 0.83x |
| `push` | 0.93x | 0.93x | `Vec` + reserve, indices | 1.46x | 1.56x |
| `strlist` | 0.82x | 0.27x | `Vec<String>` | 1.14x | 4.27x |
| `records` | 0.50x | 0.50x | enum + `String` | 1.10x | 2.18x |
| `tree` | 0.97x | 1.08x | `Box` nodes | 0.83x | 0.77x |
| `interp` | 0.78x | 0.24x | enum + `Box` | 0.82x | 3.40x |
| `graph` | 0.76x | 1.66x | `Vec<Vec>` | 1.19x | 0.72x |
| `words` | 0.57x | 0.36x | `HashMap<&str>` | 1.16x | 3.24x |
| `particles` | 0.97x | 1.60x | AoS, elementwise | 0.88x | 0.55x |
| `sexp` | 0.86x | 0.55x | enum + `Box` + `String` | 1.69x | 3.06x |
| `lru` | 0.92x | 0.42x | `HashMap` + arena | 1.26x | 2.97x |
| `scene` | 0.98x | 0.25x | `Vec` children | 1.12x | 4.44x |
| `calc` | 1.12x | 0.37x | `Box` + `Result` | 1.94x | 5.28x |
| `bintrees` (`binary-trees`) | 0.86x | 0.12x | `Option<Box>` (ours) | 0.57x | 4.73x |
| `respond` | 0.74x | 0.71x | DTO + `String` | 1.00x | 1.42x |
| `blur` | 0.89x | 2.32x | flat indexing | 1.34x | 0.58x |
| geometric mean | 0.84x | 0.57x | | 1.10x | 1.92x |

Goose's page finds Goose faster than the best safe Rust by 1.2x (geometric mean, Goose / Rust 0.84x); here it is 1.8x faster (0.57x), mostly because several Rust rows here are the owning-pointer ones (`Box`, `HashMap` with its default hasher, `Vec<String>`) rather than index arenas and open addressing, and because the default sizes are the page's small or medium ones. The exceptions point the other way: `graph` (Goose 3x slower than nested `Vec`s here, 1.3x faster than the best Rust row there at 2 M vertices) and `blur` (2x slower here: Rust and Fa vectorize the kernel, Goose's C doesn't).

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
