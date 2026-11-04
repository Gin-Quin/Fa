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

`fa build --output=<directory>` writes into `<directory>/<target>/`, where the runner looks for the output (`buildFa` in `run.ts`). The `native` and `wasm` targets need `fa-native` next to `fa` (`cargo build --release -p fa` builds both).

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

Deviations: `sexp` keeps the children of a list in an `Array` (Rust: `Vec<Node>`) instead of Goose's first-child and next-sibling links; `graph` stores vertex ids as `Integer` (8 bytes) where Rust and Goose use 4; `scene` writes the 3x3 matrix as nine `Float32` fields (Fa has no fixed-size arrays); `respond`, `strlist`, `records`, `words`, `sexp` and `calc` read the bytes of a string with `text.bytes()` (sixth native shift), as Goose reads its `u8` buffers: the parsers of `calc` and `sexp` index `self.text.bytes()` (read in place, no array) instead of an `Array(Character)`, and `sexp` slices its symbols out of the text.

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

~~Missing in the library: **a cheap way to read a character's code or a string's bytes**~~: `text.bytes()` (`design/decisions.md`, "Strings: byte access"), read in place when only indexed, measured or iterated.

Slow (the [comparison](#fa-rust-and-goose) has the numbers), and what the fifth native shift did about it (`design/decisions.md`, "Simple strings" and the entries after it):

- **Indexing a string natively was linear**: it is constant time on an ASCII string (the `SIMPLE` flag of the capacity word), and a character of an ASCII string is one of 128 static strings, so `text.characters()` no longer allocates per character either. The parsers still read `text.characters()` once, as written.
- **Appending one character at a time** (`text += letters[…]`): no allocation for the character nor the template any more, an inlined append into the block (`strlist` 3.05 s → 0.56 s of CPU).
- **`blur`**: the common path of a check is `0 <= i < length`, which LLVM's range check elimination takes out of the main loop, the destination's header is no longer reloaded (scalar TBAA), and the byte sums are 16-bit lanes: it vectorizes, as fast as Rust.
- **`tree`**: the box of a `Node?` field is allocated before its value is computed, so the tree is laid out in the order the walk visits it (the Rust version allocates parents first too; allocating after the children makes it 3x slower).
- **On Bun**, the `UInt64` generator is a `BigInt` (exact, as documented, but it dominates the Goose-suite programs there), and `particles` creates an object for every vector `add` (the likely cause of its time there, not profiled).

## Native performance audit

[`audit/`](audit/README.md) holds the tools that compare Fa's native executables with Goose's (CPU time against the same binaries, Goose given its size at run time, callgrind profiles, hot instruction runs). What the native backend changed for the Goose suite, under which conditions, what it gained and what remains is in `design/llvm-backend-plan.md` ("Native representations of the Goose work").

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

Measured on 2026-11-02 with `GOOSE_CC=~/.local/llvm18/usr/bin/clang-18 bun benchmarks/run.ts --runs 3` (one runner process, every benchmark at its default size, under `ulimit -v 4000000`; `binary-trees`, `blur` and `calc` run again alone after a transient build failure): rustc 1.99.1, Bun 1.4.4, `fa` built with LLVM 18 at the end of the fifth native shift (`b0f6659`), Goose `eadca40` (its C compiled by clang 18.1.3 at `-O2`), on the 6-core AMD EPYC virtual machine of the [Rust baseline](#rust-baseline). **The machine was busy** (load average 10 to 16 on 6 cores from other jobs): a time can be off by 50% (Rust's `blur` took 92 ms in the midpoint run and 198 ms here), so the CPU times of the [Goose suite](#against-goose-on-its-own-suite) below are the better guide. The fourth shift's run (`bcbefe4`) had Fa / Rust 2.16x and Fa / Goose 3.96x on the Goose suite.

All the implementations of a benchmark printed the same output: the runner compares every one with Rust's before timing it.

| Benchmark | Rust | Fa native | Goose | Fa / Rust | Goose / Rust | Fa / Goose | Rust ovf | Fa ovf | Fa wasm | Fa Bun | Rust size | Fa size | Goose size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fib` | 1.16 s | 1.36 s | 1.60 s | 1.17x | 1.37x | 0.85x | 2.11 s | 1.97 s | 2.81 s | 3.84 s | 331.0 KiB | 22.3 KiB | 18.4 KiB |
| `nbody` | 1.17 s | 1.42 s | 2.05 s | 1.21x | 1.75x | 0.69x | 1.17 s | 1.77 s | 2.64 s | 4.96 s | 335.9 KiB | 28.2 KiB | 18.5 KiB |
| `spectral-norm` | 1.19 s | 1.24 s | 1.02 s | 1.04x | 0.86x | 1.21x | 1.78 s | 1.22 s | 1.48 s | 2.27 s | 333.7 KiB | 25.9 KiB | 22.5 KiB |
| `mandelbrot` | 1.55 s | 1.34 s | 1.24 s | 0.86x | 0.80x | 1.08x | 1.29 s | 1.52 s | 1.41 s | 1.71 s | 331.4 KiB | 26.5 KiB | 18.4 KiB |
| `fannkuch-redux` | 0.28 s | 0.28 s | 0.49 s | 1.00x | 1.78x | 0.57x | 0.31 s | 0.35 s | 0.52 s | 0.57 s | 332.1 KiB | 27.0 KiB | 18.5 KiB |
| `binary-trees` | 1.98 s | 1.03 s | 0.24 s | 0.52x | 0.12x | 4.21x | 1.70 s | 1.04 s | 1.59 s | 1.72 s | 332.6 KiB | 31.9 KiB | 18.4 KiB |
| `string-building` | 1.29 s | 0.80 s | 0.16 s | 0.62x | 0.12x | 4.95x | 1.41 s | 0.68 s | 1.17 s | 1.56 s | 336.4 KiB | 46.8 KiB | 18.5 KiB |
| `word-count` | 1.92 s | 0.90 s | 0.46 s | 0.47x | 0.24x | 1.98x | 1.18 s | 1.15 s | 1.89 s | 4.86 s | 348.1 KiB | 34.8 KiB | 22.5 KiB |
| `sort-objects` | 1.14 s | 1.00 s | 0.40 s | 0.88x | 0.35x | 2.50x | 0.91 s | 1.20 s | 1.70 s | 4.32 s | 350.4 KiB | 37.8 KiB | 34.5 KiB |
| `sieve` | 0.95 s | 0.91 s | 1.02 s | 0.96x | 1.08x | 0.89x | 0.87 s | 0.94 s | 1.11 s | 1.32 s | 331.6 KiB | 24.7 KiB | 18.5 KiB |
| `sum` | 0.28 s | 0.38 s | 0.45 s | 1.36x | 1.61x | 0.84x | 0.49 s | 0.74 s | 0.63 s | 15.78 s | 332.7 KiB | 22.1 KiB | 18.4 KiB |
| `push` | 0.69 s | 0.59 s | 0.66 s | 0.84x | 0.96x | 0.88x | 0.57 s | 0.57 s | 2.33 s | failed¹ | 332.4 KiB | 22.3 KiB | 18.4 KiB |
| `strlist` | 0.42 s | 0.38 s | 0.23 s | 0.90x | 0.56x | 1.62x | 0.43 s | 0.40 s | 0.74 s | 15.93 s | 333.2 KiB | 29.2 KiB | 18.4 KiB |
| `records` | 0.67 s | 0.55 s | 0.21 s | 0.83x | 0.32x | 2.58x | 0.65 s | 0.44 s | 5.65 s | 10.18 s | 333.5 KiB | 26.5 KiB | 18.4 KiB |
| `tree` | 0.63 s | 0.73 s | 0.48 s | 1.15x | 0.76x | 1.52x | 0.85 s | 0.50 s | 8.00 s | 7.47 s | 331.7 KiB | 21.4 KiB | 18.4 KiB |
| `interp` | 0.14 s | 0.07 s | 0.04 s | 0.53x | 0.32x | 1.63x | 0.20 s | 0.09 s | 0.24 s | 2.25 s | 332.1 KiB | 21.9 KiB | 18.4 KiB |
| `graph` | 0.26 s | 0.26 s | 0.50 s | 0.99x | 1.90x | 0.52x | 0.30 s | 0.32 s | 0.89 s | 1.83 s | 333.8 KiB | 23.0 KiB | 18.4 KiB |
| `words` | 0.30 s | 0.20 s | 0.08 s | 0.65x | 0.26x | 2.52x | 0.38 s | 0.32 s | 0.57 s | 2.21 s | 346.9 KiB | 32.9 KiB | 18.5 KiB |
| `particles` | 0.21 s | 0.25 s | 0.28 s | 1.15x | 1.32x | 0.88x | 0.26 s | 0.20 s | 0.58 s | 20.88 s | 332.2 KiB | 23.0 KiB | 18.4 KiB |
| `sexp` | 0.41 s | 0.98 s | 0.22 s | 2.41x | 0.53x | 4.54x | 0.39 s | 0.62 s | 9.09 s | 11.18 s | 334.6 KiB | 35.4 KiB | 18.5 KiB |
| `lru` | 0.54 s | 0.81 s | 0.25 s | 1.50x | 0.45x | 3.30x | 0.57 s | 0.94 s | 1.83 s | 6.12 s | 346.8 KiB | 27.8 KiB | 18.5 KiB |
| `scene` | 0.17 s | 0.21 s | 0.04 s | 1.23x | 0.23x | 5.37x | 0.18 s | 0.14 s | 0.26 s | 0.84 s | 334.3 KiB | 27.2 KiB | 18.5 KiB |
| `calc` | 0.52 s | 0.85 s | 0.29 s | 1.63x | 0.54x | 2.99x | 0.54 s | 0.97 s | 1.97 s | 22.66 s | 335.1 KiB | 45.1 KiB | 22.5 KiB |
| `respond` | 0.62 s | 0.60 s | 0.32 s | 0.97x | 0.51x | 1.91x | 0.61 s | 0.87 s | 0.91 s | 43.35 s | 336.8 KiB | 43.1 KiB | 22.4 KiB |
| `blur` | 0.20 s | 0.23 s | 0.14 s | 1.19x | 0.69x | 1.73x | 1.40 s | 1.13 s | 1.49 s | 6.83 s | 334.5 KiB | 27.2 KiB | 18.4 KiB |
| geometric mean | | | | 0.97x | 0.59x | 1.65x | | | | | | | |

¹ Bun crashed on `push` (32 M objects under the 4 GB address-space cap: a `RangeError: Out of memory` in the earlier run). The other Bun times of the Goose-suite programs are dominated by the `UInt64` generator, a `BigInt` there.

Ratios are of times: below 1, the first one is faster. Sizes are stripped executables (Rust: `-C panic=abort -C strip=symbols`). Goose has no overflow checks in an optimized build, so it has no "ovf" column.

What it says:

- **Fa native is as fast as Rust** overall (geometric mean 0.97x): 0.88x on the original ten and 1.03x on the Goose suite (2.16x before the fifth shift). The shift closed the gaps of `blur` (31x → about Rust: it vectorizes), `strlist` (6.7x → 0.9x), `sexp` (5.7x → 1.7 to 2.4x), `tree` (3.8x → about 1x), `records` (3.5x → 0.8x) and `calc` (3.3x → 1.6x). Fa is ahead of Rust where Rust pays for its allocator and formatting (`binary-trees`, `string-building`, `word-count`, `words`).
- **Goose is the fastest of the three** on most programs: 0.59x Rust overall, 0.54x on its own suite against the Rust rows matching the Fa ports. Its wins are its design (no allocator, no teardown, inline strings, arenas, 32-bit references): `binary-trees` and `string-building` 6 to 8x faster than Rust, `scene` and `interp` 3 to 4x. It is slower than Rust on `graph`, `particles` and `sieve`.
- **Fa is 1.65x slower than Goose** overall (2.8x before the shift), 1.90x on the Goose suite (3.96x before) and 1.28x on the original ten. Fa is ahead of or level with Goose on `sum`, `push`, `tree`, `graph`, `particles`, `blur`, `fib`, `nbody`, `fannkuch-redux` and `sieve`; behind where Goose's design avoids allocations and indirections (see below).
- **Size**: Fa's native executables (15 to 45 KiB) and Goose's (18 to 35 KiB, runtime included with `--standalone`) are both an order of magnitude below Rust's 330 KiB.

### After the native costs of the minimalism audit

Measured on 2026-11-04 with the same command (`--runs 3`, `ulimit -v 8000000`), `fa` at `102d4ee` (before the `bytes()` ports): the native-goose work (inline strings and unions, arenas, borrowed parameters) and the follow-ups of [the minimalism audit](../design/minimalism-audit.md) (size-class string blocks, templates appended in place, in-place `+`, one-byte appends in registers, parameters never copied, module constants, `capacity`/`reserve`, presized maps in `lru` and `words`). The machine was busy again (load average 5 to 7): `respond` took 818 ms without overflow checks and 440 ms with them, so read the times as ±30%.

| Benchmark | Fa native | Goose | Fa / Goose | | Benchmark | Fa native | Goose | Fa / Goose |
| --- | ---: | ---: | ---: | --- | --- | ---: | ---: | ---: |
| `binary-trees` | 238 ms | 164 ms | 1.46x | | `records` | 343 ms | 248 ms | 1.38x |
| `blur` | 186 ms | 82 ms | 2.26x | | `respond` | 818 ms | 269 ms | 3.05x |
| `calc` | 520 ms | 255 ms | 2.04x | | `scene` | 22 ms | 22 ms | 0.97x |
| `fannkuch-redux` | 441 ms | 470 ms | 0.94x | | `sexp` | 321 ms | 196 ms | 1.64x |
| `fib` | 1.49 s | 1.59 s | 0.94x | | `sieve` | 914 ms | 1.17 s | 0.78x |
| `graph` | 228 ms | 670 ms | 0.34x | | `sort-objects` | 437 ms | 368 ms | 1.19x |
| `interp` | 70 ms | 49 ms | 1.43x | | `spectral-norm` | 1.01 s | 1.14 s | 0.89x |
| `lru` | 813 ms | 289 ms | 2.82x | | `string-building` | 557 ms | 149 ms | 3.74x |
| `mandelbrot` | 1.30 s | 1.31 s | 0.99x | | `strlist` | 344 ms | 181 ms | 1.91x |
| `nbody` | 1.14 s | 1.22 s | 0.94x | | `sum` | 272 ms | 255 ms | 1.07x |
| `particles` | 191 ms | 173 ms | 1.11x | | `tree` | 125 ms | 330 ms | 0.38x |
| `push` | 385 ms | 435 ms | 0.88x | | `word-count` | 570 ms | 361 ms | 1.58x |
| | | | | | `words` | 129 ms | 69 ms | 1.86x |

Fa / Goose is 1.26x in geometric mean (1.65x on 2026-11-02). The instructions executed (callgrind, the audit sizes of `benchmarks/audit/sizes.txt`) are steadier than these times; Fa / Goose:

| | `records` | `strlist` | `respond` | `interp` | `words` | `lru` | `calc` | `sexp` | `binary-trees` | `scene` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| audit (2026-11-02) | 2.22 | 1.83 | 2.73 | 1.15 | 2.14 | 2.08 | 4.19 | 3.09 | 2.39 | 1.63 |
| `compiler` with `bytes()` (`da95d6f`) | 1.80 | 1.88 | 1.84 | 0.98 | 2.03 | 2.38 | 2.23 | 1.48 | 1.22 | 1.17 |
| and the follow-ups above | 1.70 | 1.89 | 1.69 | 0.98 | 1.92 | 2.22 | 2.24 | 1.49 | 1.22 | 1.17 |

The times above predate the ports reading `bytes()` (`calc` and `sexp` still parsed an `Array(Character)` then); the instructions are on top of them. `lru` pays three map probes per miss and unproven index checks; `strlist` and `words` copy every word out of `split`.

### Against Goose on its own suite

The goal set for the native backend is to be faster than Goose on Goose's own benchmarks (release builds: `fa build`, overflow checks off). CPU time (user + system, median of 7, Fa and Goose executables run one after the other at their default sizes) on 2026-11-04, `compiler` with the merged `perf/native-goose` work and `bytes()` (the sixth native shift), on the same busy machine (load average 6 to 8; two passes differ by up to 40% on the shortest programs). The fifth shift's column is the previous version of this table.

| Benchmark | Fa | Goose | Fa / Goose | Fifth shift | What Goose still does that Fa doesn't |
| --- | ---: | ---: | ---: | ---: | --- |
| `sum` | 0.30 s | 0.27 s | 1.13x | 1.37x | nothing: the same loops (within noise) |
| `push` | 0.40 s | 0.36 s | 1.11x | 1.12x | nothing (within noise) |
| `strlist` | 0.37 s | 0.19 s | 1.97x | 2.13x | one character appended per step from registers; words as slices of the text |
| `records` | 0.38 s | 0.22 s | 1.75x | 1.64x | variable-size records inline in one buffer (Fa boxes the larger events) |
| `tree` | 0.18 s | 0.42 s | 0.42x | 0.86x | nothing (Fa faster) |
| `interp` | 0.03 s | 0.04 s | 0.68x | 2.66x | nothing (Fa faster) |
| `graph` | 0.16 s | 0.38 s | 0.43x | 0.59x | nothing (Fa faster) |
| `words` | 0.11 s | 0.06 s | 1.83x | 2.71x | slices as map keys (the split copies each word); the map itself is now a small share |
| `particles` | 0.12 s | 0.14 s | 0.89x | 1.00x | nothing (Fa level) |
| `sexp` | 0.24 s | 0.19 s | 1.26x | 3.61x | first-child/next-sibling links, inline symbols; the port now parses `text.bytes()` |
| `lru` | 0.65 s | 0.24 s | 2.70x | 2.91x | 16-byte nodes (`i32` fields): its 8 MB fit the 8 MB L3, Fa's 32-byte nodes plus the stable map entries (16 MB) don't; `unlink` inlined |
| `scene` | 0.02 s | 0.02 s | 1.07x | 5.47x | nothing (children inline in their arrays now) |
| `calc` | 0.45 s | 0.22 s | 1.99x | 3.33x | text edited in place, number templates appended without growth; the port now parses `text.bytes()` |
| `respond` | 0.62 s | 0.27 s | 2.29x | 2.44x | one byte buffer per response (Fa grows strings and builds templates) |
| `blur` | 0.09 s | 0.10 s | 0.96x | 1.12x | nothing (Fa vectorizes, Goose doesn't) |
| `binary-trees` | 0.15 s | 0.12 s | 1.24x | 3.83x | 32-bit relative links (Fa's arenas hold 16-byte nodes of two pointers) |
| geometric mean | | | 1.19x | 1.94x | |

The `lru` and `words` rows were measured again after the map work, on the ports presized since (`Map.from([], capacity = …)`; median of 21, Fa and Goose alternated, load average 6 to 7): each index slot holds a 32-bit hash tag and the entry's position (a mismatch never reads an entry), the entries are `{ key, value }` with the occupancy bits in the chunk directory (16 bytes per `Map(Integer, Integer)` entry instead of 32), deletion shifts the probe run back instead of leaving tombstones, integer keys are hashed with one multiplication and compared bitwise, and `delete` takes the key out in the walk that finds it (`documentation/collections/maps.md`, implementation section). Against the previous compiler on the same machine and program: `lru` 1.12x faster (38.7 M instructions instead of 42.1 M at the audit size; 4.2 M last-level misses instead of 5.9 M in cachegrind with an 8 MB L3); `words` unchanged (its map is about 15% of its instructions and its probe got about 8% cheaper, within the noise of the rest). `lru` now asks for `capacity` entries, the most it holds, instead of twice as many: a `Map` keeps twice as many index slots by itself, as Goose's table. What remains of `lru` is outside the map: Goose's working set fits the cache (0.13 M misses), `unlink` is a call with four bounds checks, and the values of a Fa `Map` stay in stable slots (a reference into a map survives insertions), one more cache line per hit than Goose's slot.

Fa is ahead of or level with Goose on `tree`, `interp`, `graph`, `particles`, `blur`, `push`, `sum` and `scene`, and behind on string building (`strlist`, `respond`, `calc`, `records`) and maps (`lru`, `words`). What changed since the fifth shift: inline strings, nullable recursive edges, root arenas, inline recursive collection items, borrowed parameters and module constants (`perf/native-goose`, `design/llvm-backend-plan.md`), and `text.bytes()` read in place, which `calc` and `sexp` parse instead of an `Array(Character)` and the other ports read where Goose reads bytes.

The full three-way table below (`run.ts`, wall times) was not run again; a run on 2026-11-04 (wall times, three runs, busier machine) gave Fa / Goose 1.21x on `binary-trees`, 1.48x on `calc`, 1.58x on `sexp`, 1.89x on `strlist`, 1.83x on `records`, 1.82x on `respond`, 3.95x on `lru`, 1.12x on `scene`, 0.43x on `words`, and Fa native at 0.16x (`binary-trees`) to 1.62x (`lru`) Rust on the Goose suite.

### Against Goose's published results

[Goose's results page](https://aardappel.github.io/goose/bench/results.html) (2026-10-16; Ryzen 9 9950X3D, Windows 11, clang-cl 22 `/O2`, rustc 1.98 `-O -C codegen-units=1`) compares Goose with the fastest safe Rust row of each benchmark at its large size (the "vs rust (clang)" column, inverted here into a ratio of times). This run uses the Rust row whose data structure matches the Fa port, at our default size, on Linux with clang 18:

| Benchmark | Goose / Rust, published | Goose / Rust, here | Rust row here | Fa / Rust, here | Fa / Goose, here |
| --- | ---: | ---: | --- | ---: | ---: |
| `sum` | 1.02x | 1.61x | `Vec`, collect | 1.36x | 0.84x |
| `push` | 0.93x | 0.96x | `Vec` + reserve, indices | 0.84x | 0.88x |
| `strlist` | 0.82x | 0.56x | `Vec<String>` | 0.90x | 1.62x |
| `records` | 0.50x | 0.32x | enum + `String` | 0.83x | 2.58x |
| `tree` | 0.97x | 0.76x | `Box` nodes | 1.15x | 1.52x |
| `interp` | 0.78x | 0.32x | enum + `Box` | 0.53x | 1.63x |
| `graph` | 0.76x | 1.90x | `Vec<Vec>` | 0.99x | 0.52x |
| `words` | 0.57x | 0.26x | `HashMap<&str>` | 0.65x | 2.52x |
| `particles` | 0.97x | 1.32x | AoS, elementwise | 1.15x | 0.88x |
| `sexp` | 0.86x | 0.53x | enum + `Box` + `String` | 2.41x | 4.54x |
| `lru` | 0.92x | 0.45x | `HashMap` + arena | 1.50x | 3.30x |
| `scene` | 0.98x | 0.23x | `Vec` children | 1.23x | 5.37x |
| `calc` | 1.12x | 0.54x | `Box` + `Result` | 1.63x | 2.99x |
| `bintrees` (`binary-trees`) | 0.86x | 0.12x | `Option<Box>` (ours) | 0.52x | 4.21x |
| `respond` | 0.74x | 0.51x | DTO + `String` | 0.97x | 1.91x |
| `blur` | 0.89x | 0.69x | flat indexing | 1.19x | 1.73x |
| geometric mean | 0.84x | 0.54x | | 1.03x | 1.90x |

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
| all | `fa build --target=native` | unsupported | | | | | the LLVM backend compiled one standalone file then (see "After step 6") |

What `--timings` reports at 1,000 files: 1,005 units (files), the largest 4,000 nodes; 193,438 types in the store; 3.9 M nodes, so **466.8 MB of trees** (64 B per node, 16 B per span, the sources) and 78 MB of per-node side tables, retained until the end.

Findings:

- **One core.** Every phase is sequential; CPU time grows linearly with the files (2.1 s, 23 s).
- **Memory grows with the project**, at about 0.7 MB per file of 500 lines for one analysis: `fa check --target-share 0` (no file reads `target`) on 1,000 files takes 10.7 s of CPU and 741 MB. The trees and side tables are 545 MB of it.
- **`fa check` analyzes the whole package again for each target** as soon as one file reads `target` (5% of the files here): it doubles the CPU time and, since both analyses stay alive, the memory (§4 #19 of the plan).
- **JavaScript generation costs more memory than the analysis**: 259 MB against 152 MB at 100 files, and above 4 GB at 1,000 files, where `fa check` needs 1.45 GB (the per-file copies of §4 #13–14).
- Two analyzer bugs found while writing the generator, worked around in it: a cycle between three files that export namespaces gives `E-DECL-1` (a member of a namespace whose analysis is in progress is not found), and two object types of the same structure declared in different files lose the methods of one of them (`E-OBJ-5`). The JavaScript of a package whose namespaces declare types or `static`s doesn't run (`"m0001$Item" is not declared in this file`, from `fa run`). (All three are fixed since step 2; the generator no longer works around them.)

### After steps 1 to 5

Measured on 2026-11-02 at the end of step 5, same machine and cap (load average 9 to 15 on 6 cores: the CPU column is what `fa` got, not what it could use). The packages come from the current generator (no workaround of the bugs: fewer distinct types than the baseline's).

| Package | Command | Wall | CPU | CPU time | Peak RSS | Baseline (CPU time, peak RSS) |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| 100 files | `fa check` | 0.73 s | 1.26 cores | 0.92 s | 96 MB | 2.25 s, 197 MB |
| | `fa build --target=bun` | 0.76 s | 1.61 cores | 1.22 s | 98 MB | 1.91 s, 304 MB |
| 1,000 files | `fa check` | 5.53 s | 2.31 cores | 12.8 s | 182 MB | 24.3 s, 1,498 MB |
| | `fa build --target=bun` | 5.82 s | 2.65 cores | 15.4 s | 184 MB | out of memory |
| 2,000 files | `fa check` | 14.8 s | 2.49 cores | 36.7 s | 185 MB | out of memory |
| | `fa build --target=bun` | 18.1 s | 2.59 cores | 47.0 s | 187 MB | out of memory |

The peak memory no longer grows with the package (182 MB at 1,000 files, 185 MB at 2,000). mimalloc (P8) was tried and dropped: its peaks were higher (194 to 370 MB at 2,000 files, depending on its purge delay) than glibc's. `cargo test --release -p fa --test memory -- --ignored` checks 2,000 generated files in process under 512 MB.

- **`--timings` of a package**: `parse` is reading, parsing and scanning every file; `analysis` the wall time of the units; `generation` the code generation fused into the units, summed over the threads (so it can exceed the wall time).
- **Every core**: files are read, parsed and scanned in parallel; the units (strongly connected components of the module graph) are analyzed on the worker pool as soon as the units they read are published, the longest chain of dependents first; each module's JavaScript is generated and written by the thread that analyzed it. `--timings` reports up to 6 units in flight.
- **Memory**: a unit's trees, side tables and local types are dropped once its diagnostics and code are produced; what stays is the interfaces (the global type store: 45 k types at 1,000 files, 90 k at 2,000) and the diagnostics. Trees parsed by the scan are kept within a 128 MB budget, the others parsed again by their unit (711 of 1,000).
- **Other targets** (`fa check`): only the units whose files read `target`, and the units reading an interface that changed, are analyzed again (the baseline analyzed the whole package again).
- **Determinism**: `cargo test -p fa-codegen-js --test pipeline` runs a package with 1, 2 and 8 threads under several `FA_SCHEDULE_SEED`s: the diagnostics and every JavaScript module are byte-identical to the sequential analysis. `FA_PIPELINE=0` runs `fa check` and `fa build` on the sequential analysis (one session, every file kept), to compare.

### After step 7 (the language server, `fa run`, `fa test`)

Measured on 2026-11-02 on a package of `generate.ts --files 1000 --lines 500 --tests 50` (19.3 MB, 50 test files of 3 tests), release build, same machine (busy).

| What | Wall | CPU time | Peak RSS | Sequential analysis (`FA_PIPELINE=0`) |
| --- | ---: | ---: | ---: | --- |
| `fa test` (150 tests, Bun included) | 9.8 s | 32.1 s | 137 MB | 19.4 s, 806 MB |
| Language server: load (first document opened) | 6.6 s | | | |
| Language server: 5 open documents | | | 113 MB RSS, 137 MB peak | (every tree kept) |
| Edit of a leaf module (its interface changes, read by none) | 167 ms | | | |
| Edit of a core module's body (its interface hash is the same) | 129 ms | | | |
| Edit of a core module's interface (its readers analyzed again) | 311 ms | | | |

The language server keeps the trees of the 5 open documents' units only: the other files are interfaces. An edit is an incremental run of the pipeline: the edited module is analyzed again, then the units reading a module whose interface hash changed, in parallel; a body edit costs the module's analysis plus about 100 ms of scheduling and program checks over the 1,000 interfaces. `FA_LS_PACKAGE=<package> cargo test --release -p fa-language-server --test package -- --ignored --nocapture` measures it (a 60-file version runs by default).

### After step 6 (native builds per unit)

Measured on 2026-11-04 with `/usr/bin/time -v` under `ulimit -v 8000000`, release build, same 6-core machine, busy (load average 7 to 14 from other jobs: read the walls as ±30%). "Before" is the `fa` of `395f54c`, which compiled a package as one LLVM module from the sequential analysis; it couldn't build the generated packages (a generic function of a module file failed, fixed since), so its rows are on the same packages with the calls of the generic functions removed.

| What | Wall | CPU | CPU time | Peak RSS | Before (wall, CPU time, peak RSS) |
| --- | ---: | ---: | ---: | ---: | --- |
| 100 files: `fa build --target=native` (ThinLTO) | 16.5 s | 3.2 cores | 52 s | 134 MB | 64 s, 41 s, 289 MB |
| 1,000 files: `fa build --target=native` (ThinLTO) | 2 min 54 s | 3.2 cores | 555 s | 314 MB | 9 min 14 s, 509 s, 2,293 MB |
| 1,000 files: `fa run --target=native`, empty object cache | 7 min 30 s | 3.2 cores | 1,440 s | 253 MB | |
| 1,000 files: `fa run --target=native`, cached objects | 9.3 s | 1.9 cores | 18 s | 223 MB | |
| A file of 50,000 lines of top-level code (`fa build`) | 48 s | 1 core | 47 s | 324 MB | 58.5 s, 54 s, 1,557 MB |

- **Memory** no longer grows with the program: `fa-native` peaks at 290 MB on 1,000 files (the pipeline's interfaces and units in flight, and at most 5 LLVM jobs at once: `FA_LLVM_MEMORY`, 512 MB by default, / 96 MB), and the peak RSS is the largest of a process (`ld.lld` included). The 50,000-line file is split into functions of about 16 KB of source (`lower/chunks.rs`): LLVM no longer compiles a function of hundreds of thousands of instructions.
- **CPU time**: a release build optimizes each unit (`thinlto-pre-link<O3>`), then the link optimizes again what `main` reaches, with the functions it imports: about the CPU time of one module, on every core. A development build (`fa run`) compiles every function of every unit at `-O3` (no unit knows what the others don't call): the generated package, where `main` reaches about 1% of the functions, makes it 2.6 times the release build's cold, and 9 s once the objects are cached (only the analysis runs again, and the units whose sources or read interfaces changed are compiled).
- **Size** of a 100-file program (the generated package, its entry reading `arguments` so that the program isn't folded at compile time): one module 1.13 MB, ThinLTO 592 KB, `FA_LTO=full` 447 KB. ThinLTO's default import limit (100 instructions) imported the runtime's allocator slow paths into every unit, where they were inlined: 1.32 MB; the link imports functions up to 40 instructions (`IMPORT_LIMIT` in `link.rs`).
- **The benchmarks** are single files, compiled as one module as before: their native and wasm executables are byte-identical to `395f54c`'s.
- **Determinism**: the objects of the 100-file package (`fa run`, 1 and 6 threads, two seeds) and its executable and `.wasm` module (`fa build`) are byte-identical.
