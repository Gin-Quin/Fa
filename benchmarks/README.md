# Benchmarks

The same programs written in Fa (`<name>/main.fa`), in Rust (`<name>/main.rs`) and in [Goose](https://github.com/aardappel/goose) (`<name>/main.goose`), and a runner that builds them, checks that they print the same output, and reports their run time and binary size: Fa native against Rust and Goose, Fa compiled to WebAssembly, and Fa compiled to JavaScript and run on Bun.

Sixteen of the benchmarks come from [Goose's benchmark suite](https://aardappel.github.io/goose/bench/results.html) (see [the Goose benchmarks](#the-goose-benchmarks)); the [trunk baseline](#trunk-baseline--2026-11-16) is at the end.

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

A program taken from the Goose suite has its size on a `let N = …; // BENCH_N` line, as in Goose's own harness (Goose programs have no preprocessor): the runner rewrites the number for `--size` and `--quick`, and doesn't pass arguments. Their Fa ports do the same (decided: ports match their sources): their size is a module constant on a `let n = … -- BENCH_N` line, which the runner (and `audit/build.sh`) rewrites the same way, so that a division by a size is by a constant on both sides. The Goose versions of the other benchmarks read their size from their arguments (`args()`, `parse_int`), like the Fa and Rust ones.

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

The defaults run in about 0.25 to 2 seconds in Rust (see the [baseline](#trunk-baseline--2026-11-16)).

### The Goose benchmarks

[Goose](https://github.com/aardappel/goose) is a systems language that compiles to C, with inline data structures, data stacks instead of an allocator, and relative references. Its suite (`bench/goose/`, `bench/rust/`, `bench/cpp/` and the manifest `bench/run_bench.py` of its repository, explained in `bench/design.md`) has sixteen benchmarks. Each was ported here:

- **`main.goose` is the headline Goose implementation, verbatim** (the first `goose=` row of the manifest: `records_var.goose` for `records`, `graph.goose`, `particles.goose`, `respond.goose`, `blur.goose`, …), with only the number of its `// BENCH_N` line changed to our default size. It imports `rng.goose`, the suite's xorshift64, kept in `benchmarks/goose/`.
- **`main.rs`** is adapted from the Goose repository's Rust row whose data structure matches the Fa port (written in each file's header): `Vec` (`sum`, `push`, `graph` nested, `blur` flat indexing), `Vec<String>` (`strlist`), `enum` + `String` (`records`), `Box` nodes (`tree`, `interp`, `sexp`, `calc`, `scene` with `Vec` children), `HashMap` (`words`, `lru`), the DTO (`respond`), elementwise `particles`. The Goose page compares Goose with the fastest safe Rust row, often an index arena, which Fa has no reason to imitate; the rows here are the ones that do the same work as the Fa program.
- **`main.fa`** is an idiomatic Fa port: recursive values through optionals and unions where Goose links nodes with relative references, arrays of objects with integer indices where Goose keeps typed references into a grow-only array (`push`, `lru`), `Map` where Goose uses its open-addressing table or `dictionary`. The generator is `UInt64` with `bitXor`, `shiftLeft` and `shiftRight`, so it is a JavaScript `BigInt` on Bun and wasm's loader: exact, but slow there.
- **`bintrees` is `binary-trees`**: the same Benchmarks Game program. `binary-trees/main.goose` is Goose's `bintrees.goose` with the output of `main.rs`. `words` stays separate from `word-count`: same idea, but a different generator, text and output.
- The other ten benchmarks got a Goose version written for them (`main.goose`, a direct port of `main.rs` reading its size from its arguments): inline `u8[..8]` strings where the words fit (`word-count`, `sort-objects`), `dictionary` for the maps, `sort` with a block for the comparator.

The ports use the integer widths of their sources (`design/decisions.md`, "Benchmark ports keep the source's integer widths"): `Int32` keys, values and `UInt32` links in `lru`, `Int32` vertex ids and distances in `graph`, `Int32` numbers in `interp` and `sexp`, `Int32` ids in `scene`; `calc` reuses one generator text and damages it in place with string slices (`text = text[..index] + "#" + text[index + 1..]`, edited in place natively), as its sources do. Deviations: `sexp` keeps the children of a list in an `Array` (Rust: `Vec<Node>`) instead of Goose's first-child and next-sibling links; `scene` writes the 3x3 matrix as nine `Float32` fields (Fa has no fixed-size arrays); `respond`, `strlist`, `records`, `words`, `sexp` and `calc` read the bytes of a string with `text.bytes()` (sixth native shift), as Goose reads its `u8` buffers: the parsers of `calc` and `sexp` index `self.text.bytes()` (read in place, no array) instead of an `Array(Character)`, and `sexp` slices its symbols out of the text.

#### What the ports found missing or wrong in Fa

Found with `target/release/fa` at the commit of the [trunk baseline](#trunk-baseline--2026-11-16). The programs use the workaround named, and stay idiomatic otherwise.

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

Slow (the [comparison](#trunk-baseline--2026-11-16) has the numbers), and what the fifth native shift did about it (`design/decisions.md`, "Simple strings" and the entries after it):

- **Indexing a string natively was linear**: it is constant time on an ASCII string (the `SIMPLE` flag of the capacity word), and a character of an ASCII string is one of 128 static strings, so `text.characters()` no longer allocates per character either. The parsers still read `text.characters()` once, as written.
- **Appending one character at a time** (`text += letters[…]`): no allocation for the character nor the template any more, an inlined append into the block (`strlist` 3.05 s → 0.56 s of CPU).
- **`blur`**: the common path of a check is `0 <= i < length`, which LLVM's range check elimination takes out of the main loop, the destination's header is no longer reloaded (scalar TBAA), and the byte sums are 16-bit lanes: it vectorizes, as fast as Rust.
- **`tree`**: the box of a `Node?` field is allocated before its value is computed, so the tree is laid out in the order the walk visits it (the Rust version allocates parents first too; allocating after the children makes it 3x slower).
- **On Bun**, the `UInt64` generator is a `BigInt` (exact, as documented, but it dominates the Goose-suite programs there), and `particles` creates an object for every vector `add` (the likely cause of its time there, not profiled).

## Native performance audit

[`audit/`](audit/README.md) documents the single checked-manifest driver for native
CPU, paired ratios/quartiles, a same-binary control, Rust, Callgrind/cache profiles,
RSS, executable sizes and secondary-target timings. Use it for acceptance claims;
the `run.ts` runner above is a quick wall-time smoke check.

`string-building/bytes.fa` uses `Array<Byte>` buffers, split ranges and hand-written
integer formatting, like the Goose port. `string-building/growing.fa` uses
`GrowingArray<Byte>` buffers with stable item addresses. Both print the reference
output and are measured as `string-building-bytes` and `string-building-growing`
against the same Rust and Goose sources. `growing-index-array` is the `Array`
control for `growing-index` (see the indexing methodology below).

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

<a id="rust-baseline"></a>
<a id="fa-rust-and-goose"></a>

<!-- BEGIN GENERATED TRUNK BASELINE -->
Baseline measurement in progress; this block is generated from schema-2 JSON by
`audit/compare.py --render`. Compiler baseline: `ff63f472`.
<!-- END GENERATED TRUNK BASELINE -->

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

### After the follow-ups of step 6 (what the entry reaches, asynchronous packages)

Measured on 2026-11-04 like the section above (the machine busier: load average 9 to 23), against `eff9d33` (step 6 merged). Units are compiled once the analysis is over, only for what the entry reaches (the call graph of the interfaces), each analyzed again.

| What | Before (`eff9d33`) | After |
| --- | --- | --- |
| 1,000 files, `fa build --target=native` | 4 min 7 s, 534 s CPU, 319 MB | 1 min 27 s, 197 s CPU, 208 MB |
| 1,000 files, `fa run --target=native`, empty cache | 14 min 15 s, 1,453 s CPU, 229 MB | 1 min 36 s, 185 s CPU, 231 MB |
| 1,000 files, `fa run --target=native`, cached objects | | 8.1 s, 13 s CPU, 203 MB |
| 100 files, `fa build` | | 10.1 s, 19 s CPU, 111 MB |
| 100 files (entry reading `arguments`): ThinLTO / `FA_LTO=full` | 592 KB / 447 KB | 579 KB / 437 KB |

- A cold `fa run` now costs about what `fa build` costs: neither compiles what the entry doesn't reach (99% of the generated package).
- The size gap with full LTO stays at 32%: it isn't dead code (ThinLTO already dropped it) but what full LTO does across modules, inlining the functions called once and folding through them; importing the units' functions into their callers (`FA_IMPORT_LIMIT` up to 3,000) doesn't close it. The runtime's functions above 40 instructions are no longer imported (`noinline`), so the units import its small ones at the default limit's price.
- Asynchronous packages, generic functions of another unit called from inside an instance and module values owning a resource are compiled per unit; no package of the end-to-end corpus falls back to one module.
- The benchmarks' native and wasm executables are byte-identical to `eff9d33`'s; the objects (`fa run`) and executables (`fa build`) of the 100-file package are byte-identical with 1 and 6 threads.

### Full LTO when it fits the memory budget

Measured on 2026-11-04 (load average 9 to 13), `fa build --target=native` choosing by itself (full LTO for all three under the 512 MB budget) against `FA_LTO=thin`. "50,000 lines" is a package of 10 files of 620 functions (49,640 lines; a standalone file is one module either way).

| Program | Full LTO (chosen): wall, CPU, peak RSS, size | ThinLTO: wall, CPU, peak RSS, size | Estimate |
| --- | --- | --- | ---: |
| 100 files (entry reading `arguments`) | 19.3 s, 17 s, 115 MB, 444 KB | 13.1 s, 24 s, 121 MB, 625 KB | 114 MB |
| 50,000 lines of functions | 10.2 s, 7 s, 175 MB, 312 KB | 14.7 s, 20 s, 250 MB, 937 KB | 111 MB |
| 1,000 files | 3 min 31 s, 185 s, 374 MB, 3.78 MB | 1 min 28 s, 239 s, 204 MB, 6.13 MB | 362 MB |

Full LTO gives the smallest executables (29 to 67% smaller) and less CPU, and its link is one thread: the 1,000-file build takes 2.4 times the wall time of ThinLTO's, within the memory budget. The executables print the same; they are byte-identical with 1 and 6 threads.

## GrowingArray indexing audit

`growing-index/main.fa` uses an unreserved `GrowingArray<Integer>`;
`array.fa` uses an unreserved `Array<Integer>`, Goose uses `i64[>..]`,
and `main.rs` is the safe `Vec` reference. Each fills 16, 1,000 or 1,000,000 items,
then performs four million random reads and four million random writes using
Park-Miller (seed 7), and prints the read checksum and final item checksum
modulo 1,000,000,007. Initialization and the final sequential checksum are
included. No version announces capacity. Every intermediate stays below 2^53
at these sizes.

Use the same driver for each size, preserving distinct build directories and
reports. For example (repeat with 1000 and 1000000):

```bash
python3 benchmarks/audit/compare.py growing-index growing-index-array --size 16 --directory benchmarks/audit/build/index-16 --mode prepare
python3 benchmarks/audit/compare.py growing-index growing-index-array --size 16 --directory benchmarks/audit/build/index-16 --reuse --mode cpu --runs 21 --cpu 1 --report benchmarks/results/index-16-cpu.json
python3 benchmarks/audit/compare.py growing-index growing-index-array --size 16 --directory benchmarks/audit/build/index-16 --reuse --mode profile --report benchmarks/results/index-16-profile.json
```

Both implementations share the safe Rust `Vec` oracle and Goose reference. The
native CPU series includes its own interleaved same-binary control; no separate
variant timing script is needed.
