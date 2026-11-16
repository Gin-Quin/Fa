# Goose port audit — 2026-11-16

Source audit at `e7e05356`, before replacing any port. `G`, `F` and `R` below mean
`benchmarks/<row>/main.goose`, `main.fa` and `main.rs`; numbers are source lines.
All sixteen sources in all three languages were read. A matching checksum does
not establish matching storage, construction order or traversal order.

“Same collection” means the corresponding documented Fa collection, not identical
allocator internals. Differences caused by its implementation remain collection
costs. Explicit reservation, changing node ownership, changing link widths or
changing passes are called out separately. `Integer` is 64-bit natively; a native
reference is pointer-sized unless a collection specifies another representation.
Goose `varint` payloads have value-dependent widths: Fa has no corresponding
source-level integer encoding option.

| Port | Goose: ownership, widths, passes | Existing Fa and paired Rust | Finding / disposition |
| --- | --- | --- | --- |
| sum | One grow-only flat `i32` buffer, append N values, eight forward scans, `i64` sum (G12, G14, G19). | F17 explicitly reserves N `Int32` values; F25 has the same eight scans. R34 collects into `Vec<i32>` with exact-size reservation, then scans (R41). | Payload and scans match; explicit preallocation hides growth. Keep the existing reserved form when adding an unreserved main. Rust `sum.rs` is the suite's only Rust row; its `collect` reservation must remain explicit in comparisons. |
| push | One growing `Item { i32, i32 }` array and one array of pointer-sized marks; retain every 64th returned reference across growth, then scan items and marks in insertion order (G11, G14, G15, G19, G24, G26). | F20 reserves a moving `Array<Item>`; F21 stores 64-bit integer indices. R38 reserves `Vec<Item>`, R39 stores `usize` marks, same two scans (R52, R56). | Replace main with unreserved `GrowingArray<Item>`; indices are the documented local-reference-collection idiom. Keep the original as `push-array`. There is no closer safe Rust row than `push.rs` (Vec+reserve, indices); its different allocation policy is disclosed, not called segmented storage. |
| strlist | ASCII byte buffer; split into an owning variable-length string list; four flat scans of length and first/last byte (G16, G29, G30, G33). | F18 builds a String; F30 uses `split(...).filter(...)`; F33 scans `Array<String>` four times. R46 uses `split_whitespace().map(String::from).collect()`, R48 scans `Vec<String>`. | Ownership and scan order match. The high-level split/filter must not be described as Goose's single `each_split` pass: Fa's implementation counts separators, writes parts, then filters. Packed strings versus 24-byte Fa strings/borrowed split views are collection costs, not grounds for replacing strings with byte ranges. Rust `strlist_owned.rs` remains the owning-list comparator. |
| records | Variable-sized tagged log: tag + `i32`, `i16`, `i16`; tag + `i32` + varint-length text; tag + `i32`, `u8`; four forward dispatch scans (G16, G37, G56). | Same declared scalar widths and owning String payload, `Array<Event>` built by push and scanned four times (F7, F37, F59). R10 enum+String, R54 push and R78 scans. | Faithful collection and passes. Fa packed union arrays already correspond to the variable log; payload padding/string storage and varint text lengths are compiler costs. Keep `records.rs`. |
| tree | One pool of 12-byte nodes: `i32` and two nullable `u32` relative links; left, right, parent construction; eight recursive pre-order sums; whole-pool release (G17, G20, G23, G36, G47). | F15 recursively owns optional children, F22 constructs parent fields before children; recursive sums match. R10 uses `Option<Box<Node>>`; R39 allocates parent before children; recursive drop. | Unfaithful ownership and construction. Needs GrowingPool and compact one-way links. Fa requires reverse relations even in GrowingPool (`pool-links/`); no `u32` reference option. Intended Rust row `tree_arena.rs`; retain `tree_box.rs` for recursive variant. |
| interp | Variable-size union nodes in one pool, `i32` literal and `u32` relative operand links; post-order construction; eight recursive left-before-right evaluations (G14, G21, G23, G42). | F7 owns recursive union operands; F31 builds the same expression order and F47 evaluates left-before-right. R15 owns `Box<Expr>` children with the same evaluation order. | Unfaithful ownership despite matching recursion. Needs GrowingPool links in union payloads, without reverse bookkeeping or widened links. Intended Rust `interp_arena.rs`; current `interp_box.rs` belongs to recursive variant. |
| graph | First V entries of one pool are heads; subsequent 8V edges are `{ u32 vertex, nullable u32 next }`; prepend each edge, then four BFS runs follow chains in reverse insertion order. Reuse queue and `i32` distances (G25, G27, G56, G64, G72). | F24 uses `Array<Array<Int32>>`, appends neighbours in forward insertion order, initializes distances after edges (F37), and reserves queue. R42 nested `Vec<Vec<i32>>`, same forward order and setup (R49, R50). | Unfaithful pool, link shape, signedness, traversal and initialization order. BFS checksum conceals order difference. Need GrowingPool edges/heads and 32-bit one-way links, `UInt32` vertex ids; intended Rust `graph_arena.rs`, current `graph_nested.rs` retained as variant. No CSR substitution. |
| words | Vocabulary of bounded inline 20-byte words; owning variable-length split list; fixed open-addressed table of borrowed byte slices + `i32` count, hash/linear probe, then full slot scan (G14, G21, G42, G45, G50, G65). | F22 reserves `Array<String>` vocabulary; F43 split/filter; F46 `Map<String,Integer>` gives 64-bit counts and ordinary string key stores; F53 iterates live entries. R39 `Vec<String>` vocabulary, R58 borrowed `Vec<&str>`, R60 `HashMap<&str,i64>`, live-entry scan. | Unfaithful count width and table algorithm/scan; bounded word storage and borrowed slice-key ownership differ too. Fa Map is the idiomatic map comparator, but cannot be represented as this exact table without replacing the benchmark with a hand-written container. Stored borrowed string slices need a lifetime/ownership decision (maps reject reference keys). Candidate Rust `words_open.rs` for exact slot/probe shape; current `words_hashmap.rs` is a separate standard-map comparison. |
| particles | One AoS array of two `float3` (six `f32`); append, 200 forward updates (velocity, position, bounce), final flat checksum (G19, G24, G36, G47). | Same six `Float32` fields, arithmetic order and scans, but F35 explicitly reserves N. R72 reserves a `Vec<P>`; R88 same elementwise updates, R99 checksum. | Kernel/layout faithful; source-level reservation differs from Goose. Preserve reserved variant when adding unreserved main. Same `particles.rs` elementwise Rust row; per-component/SoA rows would change work. |
| sexp | One variable-node pool with varint number/symbol length, `u32` first-child/previous-sibling links; top-level pointer array. Recursive parsing has eof/unclosed error returns. Four root scans, recursive sibling-chain walks in reverse source order (G27, G35, G36, G72, G129, G126, G142). | F68 owns recursive unions, String symbols, `Int32` numbers and `Array<Sexp>` children; F90 parser has no error path; F137 walks children in forward source order; F153 owns roots. R90 `enum`/String/Vec, R102 parser, R142 forward walk. | Unfaithful pool, links, integer encoding, sibling order, error path and teardown. Needs compact one-way pool links and an error-bearing parser, not merely reversing child arrays. Candidate `sexp_arena.rs` has matching sibling chains but borrowed `&str` symbols (not Goose ownership); current adapted `sexp_box.rs` stays with the array-child variant. |
| lru | Reusable slot_pool; `{ i32 key, i32 value, u32 prev, u32 next }`; fixed open-addressed map `{ i32 key, u32 nullable pool-ref }`, backward-shift deletion, then MRU→LRU chain checksum (G37, G38, G40, G47, G60, G155). | F18 same scalar widths with integer links, F62 `Array<Node>` plus free-index stack, F69 `Map<Int32,UInt32>`; same operation stream and final list order. R37 same arena, R100 `HashMap<i32,u32>`. | Arena/free-list ownership and link widths are equivalent by the documented index idiom, but Map hides different probe/hash/rehash work and the failed-lookup slot is recomputed on insert. Exact container counterpart is Pool with relations, which adds reverse work; need lead decision before changing it. `lru_open.rs` matches Goose's slot algorithm; `lru_hashmap.rs` matches current Fa Map. |
| scene | One pool, `i32` id, 24 `f32` transform fields, bounded inline four-child list of `u32` relative references. Parent allocated before children. Sixteen frames: flat animation, recursive pre-order update, flat checksum (G27, G28, G30, G51, G83, G94, G99). | F33 owns `Array<SceneNode>` children; matrix has nine scalar fields, same numeric widths; F137/F147/F155 make three recursive passes. R32 owns `Vec<SNode>` children, same three recursive passes (R134, R142, R153). | Unfaithful ownership and two traversal shapes. Fixed `CArray<Float32,9>` exists (foreign-code.md:321), so matrix storage is expressible; bounded appendable inline reference list and 32-bit links are not. Intended Rust `scene_arena.rs` has `[u32;4]`, `u8` count and two flat scans; retain current adapted `scene_box.rs` with recursive variant. |
| calc | Per-input variable-node pool, varint numeric payload and `u16` operand links, `u8` op; reused byte text; recursive parse then recursive evaluation with error return and whole-pool release on every path (G33, G40, G92, G120, G166, G183). | F100 owns recursive union, `UInt64` number and `Byte` op; F109 parser, F209 evaluation, F246 reused String. R42 owns Box nodes (`i64` number/`u8` op), byte text, Result/recursive cleanup. | Unfaithful ownership, link/number encodings and partial-tree cleanup. Needs local GrowingPool, compact links and parser/evaluator error paths. Intended Rust `calc_arena.rs`; current `calc_box.rs` stays with recursive variant. Do not replace Paren nodes or adjust corruption. |
| respond | DTO owns varint scalars, inline user text and inline item list with sku text. Construct per request, emit escaped strings directly into one output buffer, then FNV scan and discard (G22, G23, G71, G80, G114). | F20/F26 owns String/Array DTO with 64-bit Integer fields; F76 builds a separate quoted string and F88/F94 append interpolations. R37 DTO of String/Vec + i64, R96 emits escaping directly into final output. | DTO ownership/order match; Fa materializes a quoted string per field where Goose/Rust emit directly. A faithful mutating output helper is expressible: preserve current interpolation form as a variant when replacing it. Varint field storage/bounded temporary digit list are separate compiler/language costs. Keep `respond_dto.rs`, not streaming. |
| blur | Two flat byte arrays, interleaved unreserved initialization, 16 alternating interior stencil scans, final flat scan; temporary sum is `u16` (G18, G22, G31, G34, G39, G44). | F29/F30 explicitly reserve both arrays; F20 declares Integer sum, then /16; same interleaved writes and scans. R31 uses `u16` sum, R56 unreserved Vec buffers. | Kernel/order faithful, reservation and temporary width differ. Faithful unreserved source with explicit `UInt16` sum is expressible; preserve original variant. Existing `blur_index.rs` remains the flat-index Rust comparator. |
| binary-trees | Local pool per tree, two nullable `u32` child links (8 bytes); post-order recursive build, recursive count, pool release; separate long-lived pool, same depth/iteration schedule (G12, G14, G21, G30, G44). | F6 owns recursive optional Tree children and same recursion/schedule (F28, F30). R8 owns `Option<Box<Tree>>`; recursive construction/count/drop. | Unfaithful ownership and destruction. Same compact one-way link gap as tree, plus collection-owner-scoped references for each local pool. Intended Rust `bintrees_arena.rs`; preserve current Box version as recursive variant. |

## Representation probes

These small programs are findings, not timed alternative benchmarks. None is a
proposed syntax extension. They use existing syntax to isolate the restrictions:

- `pool-links/main.fa`: requires `REL001` for each of `left` and `right`; current
  checker does reject them. Adding reverse sides would add fields/writes absent
  from Goose and would not be a faithful workaround.
- `local-marks/main.fa`: `REF010`, “only the fields of a relation store references:
  declare it with `use`, or keep an index”. The new push port follows that fix.
- `scene-children/main.fa`: the specification also prohibits this ordered Array
  of stored references. The checker currently accepts the field, and native,
  wasm and Bun print `0` after the child changes to `2`: it silently stores an
  owning value. This is a missing rejection, not valid reference-list semantics.
  Neither that acceptance nor using a CArray of integer indices authorizes a
  substitute for the missing bounded reference-list abstraction.

No complete pool port is published until these representation decisions are
settled. Existing recursive mains and their Rust references stay runnable; their
results must not be called same-structure collection comparisons with Goose.

## Rust provenance and string API

Rust row names were checked against Goose's
[eadca40 manifest](https://github.com/aardappel/goose/blob/eadca40/bench/run_bench.py).
The manifest supplies arena alternatives for tree/interp/graph/sexp/scene/calc/
bintrees, and open-addressing alternatives for words/lru. It supplies only the
reserved Vec+indices row for push. `scene_arena.rs` was read directly: it retains
the parent-before-children build, `[u32;4]` plus `u8` count, two flat scans and one
recursive update. No alternative Rust source is installed before its Fa port can
be completed, so the current rows continue to match the retained implementations.

Trunk's compiler still has `Character`/`String.bytes()`, although the specification
already describes the string redesign. `git show feature/strings:benchmarks/calc/main.fa`
and the corresponding sexp file were inspected. They change lexer constants to
UInt16, use `codeUnitAt` and String.length, and widen calc's operator to UInt16.
They do not change either AST representation, traversal order or parser error
shape. New trunk code must use its existing API; once migrated, keep a byte
operator payload for a faithful calc port instead of silently widening it.

## Next ports

The lead continuation decisions are recorded in design/decisions.md under
Benchmark ports. The seven pool ports wait for the language study, retaining
their current mains. Standard Map is the intended main comparison for words/lru,
with the same 1.10-times-Goose goal. Lru-open is the expressible explicit-table
variant; the words counterpart is blocked by REL001/REL002 in
words-slice-key/main.fa. Main words now uses 32-bit counters and words-wide keeps
the original 64-bit-counter sources. The table above remains the source audit at
e7e05356, before these continuation changes.

After the push checkpoint: unreserved sum/particles/blur variants, direct-output
respond, then any pool ports enabled by the lead's relation/compact-link/inline-list
decisions. Resolve words/lru exact-table-versus-standard-Map policy before
rewriting them. Retain original source files byte-for-byte as variants and re-pair
Rust by structure whenever a main changes.
