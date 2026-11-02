# Minimalism audit: Fa against Goose on Goose's suite

Why compiled Fa programs execute more instructions and use more memory than Goose on the ten
benchmarks where Fa is behind, measured against the "Minimal output" principle of
`design/compiler-architecture.md`. Audit of 2026-11-02 on branch `compiler` (3125187), read-only on
the compiler.

## Method

- Fa: `fa build main.fa --target=native --emit-ir` (release: `-O3`, overflow checks off). Goose:
  `goose build -O2 --standalone`, clang 18 (`GOOSE_CC`). Same size for both: the Goose program's
  `// BENCH_N` line is rewritten, Fa takes it as its argument (`benchmarks/audit/sizes.txt`, about
  a tenth of the default sizes; `binary-trees` 14, `interp` 17, `scene` 9).
- Instructions: callgrind `Ir` with `--dump-instr=yes`, attributed to the disassembly
  (`benchmarks/audit/hot.py`: cost per function, hottest instruction runs, call edges). Almost
  everything of Fa is inlined into `main` or the user functions, so causes were classified by
  reading the hot runs against the source and the optimized IR (`main.ll`).
- Memory: peak RSS (`/usr/bin/time %M`) at the audit sizes and at the default sizes.
- Scripts: `benchmarks/audit/build.sh` (both builds, IR, Goose's C, `objdump`), `profile.sh`
  (callgrind and RSS), `report.sh` (`hot.txt` per side), `show.py`, `rss.sh` (default sizes).
  Outputs land in `benchmarks/audit/build/` (ignored).
- Classes of extra instructions: (a) checks the analyzer could prove away, (b) copies, (c) drops
  and frees, (d) allocations and allocator overhead, (e) indirections, boxing, value sizes,
  (f) generic runtime paths instead of specialized code, (g) missed LLVM optimizations
  (attributes, values kept in memory, inlining), (h) port or harness differences.
- One harness difference applies everywhere: Goose's sizes are compile-time constants, Fa's are
  read at run time. It changes few instructions (`cmp $imm` against `cmp reg`), but a division by
  the size becomes a `div` (25–40 cycles) in Fa where Goose multiplies by a reciprocal (`lru`'s
  `%% keyCount`, `words`' `%% vocabularySize`).

## Summary

| Benchmark | Ir Fa (M) | Ir Goose (M) | Fa/Goose | Extra (M) | RSS audit Fa/Goose (MB) | RSS default Fa/Goose (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `records` | 163.3 | 73.4 | 2.22 | 89.9 | 13.8 / 5.5 | 126.6 / 38.0 (3.3x) |
| `strlist` | 148.1 | 81.1 | 1.83 | 67.0 | 21.9 / 7.3 | 189.3 / 56.6 (3.3x) |
| `respond` | 517.5 | 189.7 | 2.73 | 327.8 | 1.4 / 1.9 | 1.4 / 1.9 (0.7x) |
| `interp` | 20.2 | 17.6 | 1.15 | 2.6 | 2.3 / 2.3 | 11.3 / 5.5 (2.0x) |
| `words` | 47.9 | 22.4 | 2.14 | 25.5 | 11.5 / 5.6 | 49.0 / 18.7 (2.6x) |
| `lru` | 37.1 | 17.9 | 2.08 | 19.3 | 6.8 / 2.9 | 60.9 / 10.0 (6.1x) |
| `calc` | 459.4 | 109.6 | 4.19 | 349.8 | 1.5 / 1.9 | 1.5 / 1.8 (0.9x) |
| `sexp` | 239.7 | 77.5 | 3.09 | 162.2 | 32.3 / 4.4 | 307.3 / 27.0 (11.4x) |
| `binary-trees` | 306.8 | 128.1 | 2.39 | 178.7 | 3.5 / 2.7 | 17.9 / 8.1 (2.2x) |
| `scene` | 12.9 | 7.9 | 1.63 | 5.0 | 1.7 / 2.2 | 7.6 / 6.5 (1.2x) |
| total / geomean | | | 2.22 | 1,227.8 | | |

Both sides print the same output on every benchmark. Fa's own scanning loops are often *cheaper*
than Goose's (`records`' scan −9 M: Goose walks variable-size records; `sexp`'s walk −10 M;
`interp`'s and `scene`'s recursive updates are level): the gap is in building values, in strings,
and in freeing.

## Value sizes

| Benchmark | Value | Fa layout (bytes) | Goose (bytes) |
| --- | --- | --- | --- |
| `records` | `Event` | 8 (array slot) + box: `Move`/`Quit` 16, `Say` 40 + text block ≥ 16, `Tick` shared: **32 per event** on average | 1 tag + payload, inline, variable: **9.3** on average |
| `strlist` | word | 24 (header) + 16–32 (block), twice in the split and the filtered array | length + bytes in one block (≈ 16) |
| `interp` | `Expression` | box 24 (`{i64 id, ptr, ptr}`), 16 for a leaf | 5 (leaf), 9 (binary), one pool, `u32` links |
| `words` | map entry | 48 (`{hash, live, String, Integer}`) + 8-byte index slot | 24-byte slot (slice + `i32`), presized |
| `lru` | `Node` | 32 (four `Integer`) | 16 (`i32`, `i32`, two `u32` pool links) |
| `lru` | map entry | 32 (`{hash, live, key, value}`) + 8-byte index slot, dead entries kept until compaction | 8 (`i32` key + `u32` link) |
| `calc` | `Expression` | box 16 (`Number`, `Negation`, `Parenthesized`), 48 (`Binary`: a 24-byte `Character` operator) | 1 tag + varint / `u16` links |
| `calc`, `sexp` | parser text | `Array(Character)`: 24 per character | the text's bytes: 1 per character |
| `sexp` | `Sexp` | box 32 + name block 16 (`Symbol`), 16 (`Number`), 16 + array header 24 + 8 per child (`List`) | 1 tag + 4-byte link + inline name / varint |
| `binary-trees` | `Tree` | 32 (`{i1, ptr}` × 2), one block per node | 8 (two `u32` relative links), one pool per tree |
| `scene` | `SceneNode` | 112 boxed + array header + 8 per child | 117 inline in one pool, `u32` child links |

`sexp`'s 11x RSS is almost entirely `generator.text.characters()`: 9.6 M characters × 24 bytes,
plus the doubling of the array while it grows. `lru`'s 6x is the 32-byte nodes, the 40+ bytes per
map entry, and the table and array copies while they grow (Goose presizes its table).

## Per benchmark

Shares are of the extra instructions (Fa − Goose); they are estimates from the hot runs and can
overlap by a few percent. Paths are under `packages/codegen-llvm/`.

### `records` (+89.9 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| `word += letters[i]`: each character is a 1-byte `String` (`ascii_table`, `'\r'` test, flag), appended by the generic path (length dispatch of `copy_bytes`, header in a stack slot); 77 instructions per character against 16 | b f g | 65% | `fa_string_get` (`src/lower/calls.rs:745`), `fa_string_append` (`src/lower/arith.rs:826`, runtime) |
| `letters` is a runtime global (`fa.global.0/1/2`) reloaded and re-tested for every character | g | (7%, inside the above) | `slot_of` (`src/lower/mod.rs:1448`) |
| Every word starts empty and grows once to a 16-byte block (`string_append_slow` → `string_grow` → `fa_alloc`) | d | 21% | runtime `string_grow` |
| One box per event, allocated by the out-of-line bump path (`heap_alloc_slow`, 47 instructions) | d e | 18% | `box_value` (`src/lower/objects.rs:94`), runtime `heap_alloc_slow` |
| The scan is cheaper than Goose's walk | | −10% | |

### `strlist` (+67.0 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| `text += letters[i]`: the string header lives in a stack slot (its address escapes to the slow path) and is reloaded and stored per character; a capacity check per byte (Goose reserves its buffer); the `'\r'` test of a 1-byte string | g a f | 50% | `fa_string_append` call (`src/lower/arith.rs:826`) |
| `split` copies every word into its own block (200 k allocations through `heap_alloc_slow`) | d b | 24% | runtime `split_one` / `owned_copy_with` |
| The scan: `codes[0]`, `codes[codes.length - 1]` bounds-checked, the SIMPLE flag tested per word, 24-byte headers | a e | 20% | `src/lower/arrays.rs:568–860` (virtual `codePoints`) |
| `.filter` copies the 24-byte headers into a second array (and grows it) | b | 9% | `src/lower/arrays.rs` (`filter`) |

### `respond` (+327.8 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| String growth: every template, every `quoted()` result and every built name starts without capacity and grows ~14 times per request (`string_grow` + `fa_alloc` + `copy_bytes` + `heap_alloc_slow`) | d b | 28% | `src/lower/text.rs:245–300` (templates), runtime `string_grow` |
| `quoted()`: a 1-byte `String` appended per character (≈ 50 instructions per character against 6), and a new string returned then copied into the template | b f g | 23% | `src/lower/patterns.rs` (`for character in`), `src/lower/text.rs` |
| `fnv1a` over `codePoints()`: per byte a UTF-8 decode test and a bounds check, not unrolled: 10 instructions per byte against 3.75 | f a | 17% | `for_code_points` (`src/lower/patterns.rs:643`), `fa_next_code_point` |
| `simple_flag` rescans of every appended piece (numbers, names) although they are known ASCII | f | ~10% | `fa_string_append` callers, runtime `simple_flag` |
| `sku`: a template per digit (`"{below(…)}"`), `uppercase[i] + uppercase[j]` concatenation; global reloads of `lowercase`/`uppercase` | b f g | ~10% | `src/lower/text.rs`, `src/lower/mod.rs:1448` |

### `interp` (+2.6 M; RSS 2x at depth 21)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| 60 k boxes through the out-of-line bump path (47 instructions each) | d | 100%+ | runtime `heap_alloc_slow` |
| `%%` with sign fix-ups (`test`, `cmovns`, `lea`) although every operand is proven in `0..modulus` | a | ~40% | `src/lower/arith.rs` |
| 24-byte boxes against 5/9-byte pool records: no instruction cost at this size, 2x memory at the default | e | | `src/lower/objects.rs` |

### `words` (+25.5 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| Map lookups: an index slot then a 48-byte entry, stored-hash compare, then a byte-at-a-time key comparison; the table grows from empty (Goose presizes) | e f h | ~40% | `src/lower/tables.rs` (equality, hash at `:123`), runtime `table_index` |
| `split` allocates and copies each of the 100 k words | d | ~30% | runtime `split_one` |
| `.filter` copies the array of headers | b | ~15% | `src/lower/arrays.rs` |
| Vocabulary and text building: character appends, string growth | d f | ~12% | as in `records` |

### `lru` (+19.3 M; RSS 6x)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| Map probing: `fmix64` (8 instructions) against a multiply-shift, index slot then a 32-byte entry, a stored-hash compare; `get`, then `positions[key] =`, then `delete` probe again where Goose reuses its slot | e f | ~40% | `src/lower/tables.rs:123`, runtime `FaTable` |
| Table rehashes (`table_index`, `memset`) and node-array growth: Goose presizes both | h d | ~22% | runtime `table_index`, `array_grow` |
| `unlink` is a call (not inlined) returning an `IndexError` pointer, checked at each call; three bounds checks in it and four in `linkFront` | a g | ~25% | `src/lower/calls.rs`, `src/lower/arrays.rs` (indexing) |
| 32-byte `Node` (four `Integer`): memory only at this size | e | | `src/lower/objects.rs` (layout) |

### `calc` (+349.8 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| `current()` = `characters.get(position) ?? "\0"`: not inlined, handles negative indices, and *copies* the `Character` string it reads (`fa.copy`) — 2.1 M calls, 31 instructions each | b a g | 19% | `array_get` (`src/lower/arrays.rs:365`), `emit_copy` (`src/lower/ownership.rs:217`) |
| `digitValue` is `digits.indexOf(character)`: a linear search of "0123456789" (6 compares on average) after reloading the global `digits` | h f g | 15% | the port; runtime `fa_string_index_of` |
| Generation: number templates, operator literals appended to `self.text` through `self`, growth | b d f | 20% | `src/lower/text.rs` |
| `text.characters()` per input (24 bytes per character) and its drop, which visits every element to find none owned | b c | 10% | runtime `fa_string_characters`; drop in `src/lower/ownership.rs` |
| One box per node and a recursive drop (`fa.drop.11`) on every input, error paths included; Goose resets a pool | c d | 10% | `src/lower/ownership.rs:929` |
| `edited` rebuilds the text character by character where Goose edits in place | h | 7% | the port |

### `sexp` (+162.2 M; RSS 11x)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| Parser: `current()` copies a `Character` from `Array(Character)` (1.43 M calls, 31 instructions each), `digitValue` = `digits.indexOf` | b a g h | 45% | as in `calc` |
| Generation: letters appended as 1-byte strings, numbers through a template, `self.text` through `self` | b f g | 25% | `src/lower/text.rs`, `src/lower/arith.rs:826` |
| Symbols built character by character (`name += character`) into a fresh block, a box per node, an array (header + items) per list | d e | 20% | runtime `string_grow`, `heap_alloc_slow` |
| `characters()` of the 9.6 M-byte text: 230 MB of 24-byte strings | e b | 6% (and the RSS) | runtime `fa_string_characters` |
| Walk is cheaper than Goose's | | −6% | |

### `binary-trees` (+178.7 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| Per-node drop (`fa.drop.9`, 27 instructions per node: free-list push) where Goose resets a pool | c | 49% | `src/lower/ownership.rs` |
| `bottomUp` returns a 32-byte `{ {i1, ptr}, {i1, ptr} }` through memory, then copies it field by field into the parent's block | b e | 37% | `Repr::Optional` (`src/lower/values.rs:433`) |
| `itemCount` takes the 32-byte `Tree` by value (four registers) and tests two flags | e | 11% | as above |
| Allocation of the first blocks (bump path out of line) | d | 2% | runtime `heap_alloc_slow` |

### `scene` (+5.0 M)

| Cause | Class | Share | Where |
| --- | --- | ---: | --- |
| `sumWorld(node: SceneNode)` takes the 112-byte node *by value*: 24 floats loaded and spilled per child; checked `Integer.truncate` and the `try` test per call | b a | 66% | function signatures (`src/lower/calls.rs`, `src/lower/mod.rs`) |
| `animate` and `sumWorld` recurse over the tree where Goose scans its pool linearly | h | 28% | the port (inherent to an owned tree) |
| `build`: an array header and an items block per node, boxed `SceneNode` items | d e | 16% | `src/lower/arrays.rs` |
| `update` is cheaper than Goose's | | −8% | |

## Ranked fixes

Expected gains are instructions at the audit sizes, summed over the ten benchmarks (1,228 M extra
in total); they overlap, so the column doesn't add up. "Sixth" says what the LLVM sixth shift
(inline short strings, `Character` scalar, `Node?` as a nullable pointer, arenas, compact fields,
map entries without stored hash) already covers.

| # | Fix | Gain (M) | Benchmarks | Sixth |
| --- | --- | ---: | --- | --- |
| 1 | `Character` as a scalar, and a specialized `string += character` (and `+=` of a known short ASCII piece): capacity check plus byte store, the string header kept in SSA values (a slow path that returns the new header instead of writing through a pointer), no `'\r'`/flag work for ASCII | ~330 | `records`, `strlist`, `respond`, `calc`, `sexp` | the scalar (~200); the append lowering and the header in registers (~130) are not |
| 2 | Strings written into their destination: a function whose result is appended (`out += quoted(x)`, templates) builds into the destination (result location); capacity reserved from the template's known lengths; static ASCII facts instead of `simple_flag` rescans | ~210 | `respond`, `calc`, `sexp` | inline strings remove the allocation of short temporaries (~60) |
| 3 | Arenas for the boxes of a recursive value, freed at once | ~135 | `binary-trees`, `calc`, `sexp`, `interp` | yes |
| 4 | Nullable-pointer optionals of references; small aggregates returned in registers, not `sret` + copy | ~90 | `binary-trees`, `tree` | yes |
| 5 | `codePoints()` and `for character in` over a simple string: test the flag once, then a plain byte loop (no decode, no per-byte bounds check, unrollable) | ~65 | `respond`, `strlist`, `records`, `words` | no |
| 6 | Inline the allocator's bump path (`class_next`/`class_end`) in `fa_heap_alloc`; only page refills stay out of line | ~50 | all that allocate | no (arenas take part of it) |
| 7 | Read-only aggregate parameters by pointer (alias rule), borrowed reads: no copy for `get`/`??`/indexing whose result doesn't escape | ~45 | `calc`, `sexp`, `scene`, `binary-trees` | no |
| 8 | Port fidelity: `digitValue` as a range test (or constant-folded `indexOf` on a constant string, see 9), `edited` in place, presized `Map`s (`words`, `lru`), Goose sizes as Fa constants when comparing | ~85 | `calc`, `sexp`, `words`, `lru` | no |
| 9 | Module-level `let` with a compile-time value emitted as an LLVM constant, not a zeroed global stored at startup and reloaded after every call | ~25 | `records`, `respond`, `calc`, `sexp`, `interp` | no |
| 10 | Unboxed non-recursive unions in arrays (tag + largest payload inline; with inline strings `Event` is 32 bytes, no allocation) | ~17 (+ RSS 3.3x → ~1.5x) | `records` | no |
| 11 | `split` returning views of the text (or inline strings), `filter` of a temporary array in place | ~25 | `strlist`, `words` | inline strings remove the allocations (~15) |
| 12 | Maps: no stored hash and narrow index slots; multiply-shift hash for integer keys; word-wise key equality; probe reuse between `get` and the following store/delete of the same key | ~16 | `lru`, `words` | stored hash and slots (~5) |
| 13 | Range facts: `%%`/`//` without sign fix-ups, `get` without negative-index handling, bounds checks on proven indices; small helpers (`unlink`, `current`) inlined | ~20 | `interp`, `scene`, `calc`, `sexp`, `lru`, `strlist` | compact fields use the same facts |

Memory, by expected effect: `sexp` (11x) needs fix 1 (4-byte characters: ~60 MB) or, better,
reading the text's bytes directly (`bytes()`, or O(1) indexing of a simple string) — 1x; `lru`
(6x) needs compact fields (16-byte nodes), 8-byte map slots and presizing; `records` and `strlist`
(3.3x) need 10, inline strings and 11; `interp` and `binary-trees` (2x) need arenas with 32-bit
links, which the sixth shift's arenas and nullable pointers address only partly (8-byte pointers
against Goose's 4-byte relative links).

By these estimates, the sixth shift alone removes about 45% of the extra instructions (≈ 550 M of 1,228 M):
`binary-trees` and `interp` reach Goose, `records`, `strlist`, `words` and `sexp` stay 1.3–1.8x
behind, `respond` and `calc` stay 2x+ behind until fixes 1 (append lowering), 2, 5 and 7 land.

## What the analyzer must expose

- **Constant module-level values** (`Analysis::constant` exists): the backend must use it in
  `slot_of` for every `let` it can fold, strings included (fix 9).
- **Range facts per expression and per field over every store**: non-negative operands of `%%`
  and `//`, non-negative indices (`get`, `[]`), the widths of fields (compact fields), facts from
  a `filter` predicate (`not word.isEmpty()` makes `codes[0]` safe) (fixes 12, 13).
- **ASCII-ness of strings as a static fact**: literals, number formatting, characters of an
  ASCII string, concatenations of ASCII pieces; the backend then skips `simple_flag` and picks
  byte loops statically (fixes 1, 2, 5).
- **Escape and lifetime facts**: a recursive value whose nodes never outlive its root (arena,
  fix 3); a returned string only appended to a destination (result location, fix 2); a read whose
  result doesn't escape (borrow instead of copy, fix 7); an array whose elements own nothing
  (`characters()` of an ASCII string: no element drops).
- **Read-only, non-aliased parameters** (the alias rule) so that large objects pass by pointer
  with `noalias readonly` (fix 7).
- **Same key, no intervening mutation** between a map `get` and a store/delete (fix 12).

## Port fidelity (class h)

- `calc`, `sexp`: `digitValue` uses `digits.indexOf(character)` (a search) where Goose compares a
  byte range; `calc`'s `edited` copies the text where Goose edits its buffer in place; both parse
  an `Array(Character)` built from the text where Goose indexes the bytes (the comment "indexing a
  string counts grapheme clusters" is outdated for simple strings).
- `words`, `lru`: Goose presizes its table (4x the vocabulary, 2x the capacity); Fa's `Map` grows
  from empty. `lru` probes three times per miss (`get`, store, `delete` by key) where Goose keeps
  the slot it found.
- `scene`: Goose animates and sums by scanning its node pool; Fa recurses (the natural shape of
  an owned tree; acceptable, but 28% of the gap).
- Harness: Goose's sizes are constants of the program, Fa's are run-time arguments (see Method).
