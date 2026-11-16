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

A program taken from the Goose suite has its size on a `let N = …; // BENCH_N` line, as in Goose's own harness (Goose programs have no preprocessor): the runner rewrites the number for `--size` and `--quick`, and doesn't pass arguments. Their Fa ports do the same (decided: ports match their sources): their size is a module constant on a `let n = … -- BENCH_N` line, which the runner (and `audit/compare.py`) rewrites the same way, so that a division by a size is by a constant on both sides. The Goose versions of the other benchmarks read their size from their arguments (`args()`, `parse_int`), like the Fa and Rust ones.

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
| `push` | an unreserved `GrowingArray` of small objects, keeping every 64th index across growth (`push-array`: the original reserved `Array`) | 32,000,000 |
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
- **`main.rs`** is adapted from the Goose repository's Rust row whose data structure matches the current Fa port (written in each file's header): `Vec` (`sum`, `push`, `graph` nested, `blur` flat indexing), `Vec<String>` (`strlist`), `enum` + `String` (`records`), `Box` nodes (`tree`, `interp`, `sexp`, `calc`, `scene` with `Vec` children), `HashMap` (`words`, `lru`), the DTO (`respond`), elementwise `particles`. When a Fa port becomes a pool implementation, pair it with Goose's corresponding Rust arena row; keep the Box row with the recursive variant. For `push`, the suite supplies only safe Rust `Vec+reserve` with indices: it matches the payload and marks, but has a different allocation policy from Fa's segmented GrowingArray.
- **`main.fa`** must use the documented Fa collection corresponding to Goose's, with the same construction and traversal order. The [port audit](audit/ports/README.md) records all sixteen representations, passes, source-line evidence, intended Rust pairings and remaining gaps. Several existing mains still use owning recursive values where Goose uses pool links; their checksum agreement is not a same-structure comparison. Keep these implementations as named variants when faithful replacements become expressible. `push` now uses an unreserved GrowingArray and integer marks, the alternative explicitly prescribed by Fa's local-reference documentation. The generator is `UInt64` with `bitXor`, `shiftLeft` and `shiftRight`, so it is a JavaScript `BigInt` on Bun and wasm's loader: exact, but slow there.
- **`bintrees` is `binary-trees`**: the same Benchmarks Game program. `binary-trees/main.goose` is Goose's `bintrees.goose` with the output of `main.rs`. `words` stays separate from `word-count`: same idea, but a different generator, text and output.
- The other ten benchmarks got a Goose version written for them (`main.goose`, a direct port of `main.rs` reading its size from its arguments): inline `u8[..8]` strings where the words fit (`word-count`, `sort-objects`), `dictionary` for the maps, `sort` with a block for the comparator.

The ports preserve many scalar widths (`design/decisions.md`, "Benchmark ports keep the source's integer widths"): `Int32` keys/values and `UInt32` links in `lru`, `Int32` literals in `interp`, `Int32` ids in `scene`. The audit identifies remaining differences: Goose's graph ids are unsigned, its sexp/calc numbers use varints, and its words counts are `i32` rather than Fa's Integer. `calc` reuses one generator text and damages it in place with string slices (`text = text[..index] + "#" + text[index + 1..]`, edited in place natively), as its sources do. `sexp` still owns child Arrays and walks them in forward source order, whereas Goose follows reverse-order sibling links. `scene` still owns child Arrays and makes three recursive passes; Goose animates and sums with flat pool scans. Its matrix has nine Float32 fields; fixed inline `CArray<Float32, 9>` is available, but an appendable bounded list of compact child references is not specified. The current trunk compiler exposes `text.bytes()` (read in place when indexed), which these ports use; the string-redesign migration is described in the audit.

The lead has deferred the pool ports to a language study (2026-11-16).
`tree`, `binary-trees`, `interp`, `sexp`, `scene` and `calc` are **a different data
structure from Goose's (recursive owning values), kept until Fa has pool links**.
`graph` keeps its nested adjacency arrays until that same decision. Do not read
their current ratios as same-pool-layout comparisons. `words` and `lru` keep
standard Fa Map as the main rows, with the same 1.10× Goose goal; explicit-table
variants, where expressible, are separate comparisons with the matching Rust
open-addressing rows.

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

`push-array` keeps the original `push/array.fa` (reserved Array plus integer marks).
The main `push` row uses an unreserved GrowingArray. Both use the suite's only
safe Rust push row (reserved Vec plus indices), and the same Goose source.
Select either name with `audit/compare.py`; `run.ts` runs the main only.
The [port audit](audit/ports/README.md) explains the remaining differences and
which recursive mains await language representation decisions.

The continuation keeps `sum`, `particles` and `blur` as unreserved Array mains;
`sum-reserved`, `particles-reserved` and `blur-reserved` retain the original
sources, including the old Integer stencil sum. Main `blur` uses UInt16, as
Goose and Rust do. `respond` emits escaping directly into one output buffer;
`respond-interpolation` retains the original temporary quoted strings. These
variants keep the same Rust rows: sum.rs (collect), particles.rs (elementwise),
blur_index.rs (flat indexing), and respond_dto.rs (the DTO, not streaming).

`lru-open` is the explicit 8-byte Int32-key/UInt32-index slot table with Goose's
Fibonacci hash, linear probing and backward-shift deletion; it uses the separate
Rust `lru/open.rs`, adapted from lru_open.rs with zero as its empty marker.
The main `lru` keeps standard Map and lru_hashmap.rs. Main `words` keeps standard
Map with Int32 counts to match Goose; `words-wide` preserves the original Fa and
Rust rows with 64-bit counts. Both are paired with words_hashmap.rs in the
corresponding width. The requested borrowed-key words table is **blocked**:
`audit/ports/words-slice-key/main.fa` gets REL001/REL002 for storing a String slice
reference in a slot. No owning-key or encoded-range substitute is measured.

The [2026-11-16 push checkpoint](results/2026-11-16-ports.md)
([raw samples and provenance](results/2026-11-16-ports.json)) measures both forms
with the current compiler. Main push is 1.119 / 1.143 [1.024, 1.196] times Goose
CPU at 32 million items, with a passing control; audit instructions at 3.2 million
items are 155.50 M versus Goose 121.47 M. Its GrowingArray scan stays scalar
(25.60 M instructions versus 12.00 M for Goose's vectorized scan), and append
reloads/writes cursor and length metadata. The separate push-array CPU series
fails its control and is **not resolved** under this task's stricter rule; its
generated table uses `--require-control`, preserving raw phase verdicts in JSON.
The direct paired GrowingArray/Array estimates straddle equality (1.078 / 0.995),
so they establish no CPU direction. The historical trunk baseline below is
unchanged. No compiler optimization is included in this port checkpoint.

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
## Trunk baseline — 2026-11-16

[Measurements and provenance](results/2026-11-16-trunk.json).

Generated by `audit/compare.py --render` from schema-2 JSON; times are CPU seconds.
Native CPU/RSS sizes and round counts are recorded per series in JSON. Instructions/cache: audit sizes.
Each ratio cell is ratio of medians / paired median [Q1, Q3]. Near-threshold CPU gates
require the same-binary control to differ by <3% on both measures. `within noise` is unresolved.

| Suite | Benchmark | CPU size | Fa / Goose CPU | Fa / Rust CPU | Fa / Goose Ir | Fa / Rust Ir | Control | Goose / Rust goal |
| --- | --- | ---: | --- | --- | ---: | ---: | --- | --- |
| goose | `binary-trees` | 17 | 0.972 / 0.988 [0.893, 1.068] | 0.117 / 0.118 [0.113, 0.128] | 0.894 | 0.137 | 1.005/1.019 pass | goal met / goal met |
| goose | `blur` | 3000 | 1.007 / 1.020 [0.873, 1.122] | 0.984 / 0.968 [0.793, 1.142] | 1.141 | 0.918 | 1.023/0.985 pass | goal met / goal met |
| goose | `calc` | 400000 | 1.035 / 1.061 [0.999, 1.180] | 0.590 / 0.611 [0.542, 0.676] | 1.147 | 0.494 | 0.992/1.010 pass | goal met / goal met |
| secondary | `fannkuch-redux` | 10 | 1.085 / 1.082 [1.011, 1.169] | 1.009 / 1.007 [0.976, 1.038] | 1.262 | 1.291 | 0.997/1.017 pass | goal met / behind |
| secondary | `fib` | 42 | 1.002 / 0.964 [0.875, 1.057] | 0.930 / 0.930 [0.870, 0.996] | 1.000 | 0.808 | 0.954/0.951 fail | goal met / goal met |
| goose | `graph` | 100000 | 0.553 / 0.579 [0.526, 0.614] | 0.984 / 1.040 [0.791, 1.166] | 1.492 | 0.584 | 0.980/0.995 pass | goal met / within noise |
| secondary | `growing-index` | 1000000 | 1.127 / 1.105 [1.036, 1.279] | 1.170 / 1.204 [1.121, 1.268] | 1.320 | 1.336 | 0.966/0.982 fail | within noise / behind |
| secondary | `growing-index-array` | 1000000 | 1.072 / 1.051 [0.991, 1.123] | 1.106 / 1.066 [0.901, 1.195] | 1.063 | 1.076 | 1.009/0.969 fail | within noise / behind |
| secondary | `growing-index-array@1000` | 1000 | 1.059 / 1.015 [0.952, 1.496] | 1.099 / 1.062 [0.929, 1.196] | 1.066 | 1.095 | 1.090/1.096 fail | within noise / behind |
| secondary | `growing-index-array@16` | 16 | 1.038 / 1.058 [0.930, 1.234] | 0.938 / 0.982 [0.875, 1.289] | 1.066 | 1.095 | 0.901/0.958 fail | within noise / within noise |
| secondary | `growing-index@1000` | 1000 | 1.087 / 1.087 [1.010, 1.170] | 1.054 / 1.079 [0.843, 1.164] | 1.293 | 1.328 | 1.013/1.058 fail | within noise / behind |
| secondary | `growing-index@16` | 16 | 1.046 / 1.049 [0.963, 1.128] | 1.003 / 0.992 [0.942, 1.060] | 1.293 | 1.328 | 1.008/1.016 pass | goal met / within noise |
| goose | `interp` | 21 | 1.428 / 1.432 [1.137, 1.604] | 0.450 / 0.449 [0.411, 0.535] | 1.097 | 0.519 | 1.038/1.024 fail | behind / goal met |
| goose | `lru` | 2000000 | 1.106 / 1.117 [1.006, 1.319] | 0.544 / 0.551 [0.444, 0.618] | 1.131 | 0.376 | 0.989/0.983 pass | behind / goal met |
| secondary | `mandelbrot` | 4000 | 1.032 / 1.021 [0.901, 1.101] | 1.021 / 1.005 [0.934, 1.112] | 0.999 | 1.001 | 1.015/0.993 pass | goal met / behind |
| secondary | `nbody` | 15000000 | 0.899 / 0.888 [0.869, 0.951] | 1.208 / 1.176 [1.110, 1.279] | 0.995 | 1.204 | 0.962/0.984 fail | goal met / behind |
| goose | `particles` | 300000 | 0.969 / 0.980 [0.881, 1.030] | 1.028 / 1.037 [0.894, 1.096] | 0.996 | 0.997 | 1.025/1.018 pass | goal met / behind |
| goose | `push` | 32000000 | 1.034 / 1.024 [0.978, 1.110] | 0.984 / 1.021 [0.923, 1.088] | 1.112 | 0.935 | 0.925/0.968 fail | goal met / within noise |
| goose | `records` | 4000000 | 1.193 / 1.158 [1.082, 1.266] | 0.586 / 0.584 [0.549, 0.650] | 1.091 | 0.615 | 1.066/1.073 fail | behind / goal met |
| goose | `respond` | 400000 | 1.292 / 1.222 [1.060, 1.376] | 0.602 / 0.632 [0.529, 0.708] | 1.079 | 0.490 | 1.128/1.023 fail | behind / goal met |
| goose | `scene` | 12 | 2.762 / 2.780 [2.368, 3.007] | 0.470 / 0.435 [0.396, 0.535] | 1.208 | 1.033 | 0.926/0.942 fail | behind / goal met |
| goose | `sexp` | 20000 | 1.456 / 1.438 [1.349, 1.679] | 0.837 / 0.841 [0.797, 0.916] | 1.049 | 0.686 | 1.030/1.032 fail | behind / goal met |
| secondary | `sieve` | 100000000 | 1.136 / 1.141 [1.009, 1.195] | 1.113 / 1.109 [0.959, 1.202] | 0.816 | 0.999 | 1.094/1.078 fail | within noise / behind |
| secondary | `sort-objects` | 1000000 | 1.187 / 1.161 [1.066, 1.273] | 0.485 / 0.495 [0.418, 0.511] | 1.481 | 0.856 | 0.969/0.997 fail | behind / goal met |
| secondary | `spectral-norm` | 4000 | 0.951 / 0.934 [0.873, 1.019] | 0.884 / 0.904 [0.830, 0.994] | 0.881 | 0.775 | 0.987/1.014 pass | goal met / goal met |
| secondary | `string-building` | 3000000 | 1.626 / 1.697 [1.598, 1.779] | 0.274 / 0.274 [0.262, 0.300] | 1.596 | 0.272 | 0.913/0.968 fail | behind / goal met |
| secondary | `string-building-bytes` | 3000000 | 1.155 / 1.122 [1.040, 1.209] | 0.180 / 0.175 [0.167, 0.190] | 1.178 | 0.201 | 1.025/0.995 pass | behind / goal met |
| secondary | `string-building-growing` | 3000000 | 1.162 / 1.203 [0.978, 1.312] | 0.179 / 0.182 [0.168, 0.193] | 1.256 | 0.214 | 0.962/0.965 fail | behind / goal met |
| goose | `strlist` | 2000000 | 1.060 / 1.027 [0.891, 1.150] | 0.454 / 0.463 [0.399, 0.504] | 1.072 | 0.446 | 1.066/1.145 fail | within noise / goal met |
| goose | `sum` | 32000000 | 1.037 / 1.020 [0.954, 1.119] | 0.998 / 0.965 [0.908, 1.084] | 1.134 | 1.082 | 1.019/0.965 fail | goal met / within noise |
| goose | `tree` | 21 | 0.474 / 0.525 [0.430, 0.546] | 0.458 / 0.456 [0.416, 0.508] | 0.998 | 0.404 | 0.996/1.028 pass | goal met / goal met |
| secondary | `word-count` | 5000000 | 1.925 / 1.795 [1.648, 1.985] | 0.714 / 0.684 [0.639, 0.799] | 1.605 | 0.616 | 1.081/1.026 fail | behind / goal met |
| goose | `words` | 500000 | 1.174 / 1.139 [1.022, 1.378] | 0.446 / 0.461 [0.357, 0.536] | 1.266 | 0.456 | 1.112/1.037 fail | within noise / goal met |

### Absolute measurements

RSS is a separate GNU-time run, in KiB; stripped executable size is in bytes.
Wasm and Bun timings use the size shown, including process startup; errors are retained.

| Benchmark | CPU Fa / Goose / Rust (s) | RSS Fa / Goose / Rust (KiB) | Size Fa / Goose / Rust (bytes) | Ir size | Ir Fa / Goose / Rust (M) | Secondary size | Wasm (s) | Bun (s) |
| --- | --- | --- | --- | ---: | --- | ---: | --- | --- |
| `binary-trees` | 0.122 / 0.125 / 1.042 | 5888 / 8064 / 18304 | 34808 / 18888 / 342920 | 14 | 114.535 / 128.146 / 836.867 | 17 | 0.310 | 0.864 |
| `blur` | 0.097 / 0.097 / 0.099 | 19200 / 19584 / 19584 | 20976 / 18864 / 344120 | 1000 | 92.416 / 80.963 / 100.698 | 3000 | 0.681 | 5.897 |
| `calc` | 0.237 / 0.229 / 0.401 | 1792 / 1792 / 1920 | 45336 / 23000 / 346000 | 40000 | 125.641 / 109.553 / 254.348 | 400000 | 0.455 | 14.436 |
| `fannkuch-redux` | 0.262 / 0.241 / 0.259 | 1664 / 2048 / 1920 | 30256 / 18904 / 341752 | 9 | 136.695 / 108.296 / 105.878 | 10 | 0.338 | 0.481 |
| `fib` | 1.038 / 1.036 / 1.116 | 1664 / 1792 / 1920 | 25072 / 18888 / 340480 | 35 | 236.885 / 236.991 / 293.224 | 42 | 1.641 | 2.777 |
| `graph` | 0.224 / 0.406 / 0.228 | 12032 / 9728 / 12032 | 21760 / 18888 / 343688 | 10000 | 10.448 / 7.002 / 17.901 | 100000 | 0.330 | 1.815 |
| `growing-index` | 0.249 / 0.221 / 0.212 | 9472 / 9856 / 9856 | 29488 / 18888 / 341984 | 1000000 | 418.170 / 316.768 / 312.957 | 1000000 | 0.377 | 2.082 |
| `growing-index-array` | 0.238 / 0.222 / 0.215 | 9472 / 9856 / 9856 | 30592 / 18888 / 341984 | 1000000 | 336.665 / 316.768 / 312.957 | 1000000 | 0.334 | 1.879 |
| `growing-index-array@1000` | 0.069 / 0.065 / 0.063 | 1664 / 2048 / 1920 | 30592 / 18888 / 341984 | 1000 | 320.195 / 300.301 / 292.343 | 1000 | 0.146 | 0.631 |
| `growing-index-array@16` | 0.065 / 0.062 / 0.069 | 1664 / 1920 / 1920 | 30592 / 18888 / 341984 | 16 | 320.179 / 300.285 / 292.322 | 16 | 0.151 | 0.772 |
| `growing-index@1000` | 0.067 / 0.061 / 0.063 | 1664 / 2048 / 2048 | 29488 / 18888 / 341984 | 1000 | 388.210 / 300.301 / 292.343 | 1000 | 0.137 | 0.688 |
| `growing-index@16` | 0.070 / 0.067 / 0.070 | 1664 / 1920 / 1920 | 29488 / 18888 / 341984 | 16 | 388.179 / 300.285 / 292.321 | 16 | 0.146 | 0.766 |
| `interp` | 0.063 / 0.044 / 0.141 | 5888 / 5376 / 18432 | 13704 / 18864 / 342480 | 17 | 19.286 / 17.588 / 37.149 | 21 | 0.134 | 1.708 |
| `lru` | 0.290 / 0.262 / 0.533 | 10752 / 9984 / 12416 | 18944 / 18896 / 357136 | 200000 | 20.238 / 17.886 / 53.799 | 2000000 | 0.459 | 5.137 |
| `mandelbrot` | 1.264 / 1.225 / 1.239 | 1664 / 1920 / 1920 | 27632 / 18888 / 340912 | 1000 | 441.162 / 441.516 / 440.804 | 4000 | 1.452 | 1.342 |
| `nbody` | 1.120 / 1.246 / 0.927 | 1664 / 2048 / 2048 | 32872 / 18912 / 346712 | 1500000 | 867.181 / 871.786 / 720.319 | 15000000 | 1.587 | 3.027 |
| `particles` | 0.132 / 0.136 / 0.128 | 8576 / 8960 / 8832 | 23872 / 18880 / 341712 | 30000 | 105.167 / 105.577 / 105.519 | 300000 | 0.198 | 17.081 |
| `push` | 0.363 / 0.351 / 0.369 | 256640 / 255872 / 255872 | 20072 / 18864 / 341992 | 3200000 | 135.092 / 121.470 / 144.457 | 32000000 | 0.661 | > 512 MiB |
| `records` | 0.262 / 0.220 / 0.448 | 38784 / 38144 / 158080 | 23048 / 18888 / 343208 | 400000 | 80.085 / 73.386 / 130.258 | 4000000 | 0.367 | > 512 MiB |
| `respond` | 0.390 / 0.302 / 0.648 | 1664 / 1920 / 2048 | 47760 / 22976 / 347112 | 40000 | 204.714 / 189.703 / 417.976 | 400000 | 0.535 | 27.448 |
| `scene` | 0.060 / 0.022 / 0.127 | 9984 / 6656 / 10240 | 31176 / 18896 / 344112 | 9 | 9.538 / 7.896 / 9.232 | 12 | 0.140 | 0.872 |
| `sexp` | 0.293 / 0.201 / 0.350 | 57856 / 27136 / 102784 | 31904 / 18904 / 345032 | 2000 | 81.320 / 77.534 / 118.519 | 20000 | 0.442 | > 512 MiB |
| `sieve` | 1.242 / 1.093 / 1.116 | 99328 / 99584 / 99456 | 26848 / 18904 / 341096 | 10000000 | 203.734 / 249.542 / 203.878 | 100000000 | 1.372 | 1.716 |
| `sort-objects` | 0.461 / 0.389 / 0.950 | 80896 / 34176 / 91648 | 48840 / 35288 / 360784 | 100000 | 176.809 / 119.372 / 206.574 | 1000000 | 0.651 | 5.027 |
| `spectral-norm` | 1.003 / 1.055 / 1.135 | 1664 / 1920 / 2048 | 32208 / 23016 / 343424 | 1000 | 535.728 / 608.312 / 691.207 | 4000 | 1.130 | 2.067 |
| `string-building` | 0.262 / 0.161 / 0.954 | 1664 / 2048 / 2176 | 52024 / 18896 / 346296 | 300000 | 237.268 / 148.695 / 872.492 | 3000000 | 0.573 | 1.292 |
| `string-building-bytes` | 0.178 / 0.154 / 0.990 | 1664 / 1920 / 2176 | 44904 / 18896 / 346296 | 300000 | 175.141 / 148.695 / 872.492 | 3000000 | 0.569 | refused E-NUM-19 |
| `string-building-growing` | 0.187 / 0.161 / 1.043 | 1664 / 2048 / 2176 | 49528 / 18896 / 346296 | 300000 | 186.738 / 148.695 / 872.492 | 3000000 | 0.495 | refused E-NUM-19 |
| `strlist` | 0.196 / 0.185 / 0.433 | 73984 / 56704 / 135680 | 31896 / 18888 / 343208 | 200000 | 86.933 / 81.082 / 195.018 | 2000000 | 0.307 | > 512 MiB |
| `sum` | 0.286 / 0.276 / 0.287 | 126592 / 126976 / 126848 | 14752 / 18864 / 342304 | 3200000 | 161.779 / 142.684 / 149.520 | 32000000 | 0.357 | > 512 MiB |
| `tree` | 0.168 / 0.355 / 0.368 | 71552 / 51072 / 132992 | 12600 / 18864 / 341664 | 17 | 56.684 / 56.773 / 140.311 | 21 | 0.222 | 6.747 |
| `word-count` | 0.862 / 0.448 / 1.208 | 6528 / 7296 / 5732 | 44632 / 23000 / 359136 | 500000 | 235.015 / 146.465 / 381.265 | 5000000 | 0.940 | 4.765 |
| `words` | 0.094 / 0.080 / 0.211 | 22528 / 18816 / 19572 | 41568 / 18904 / 357328 | 100000 | 28.380 / 22.411 / 62.219 | 500000 | 0.237 | 1.788 |

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
