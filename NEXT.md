# Native performance: state for a successor (2026-11-05)

Branch: this worktree's branch (based on `compiler` at `b06e4f5`). Nothing pushed.

## Done and committed

- `bea3bcc` `benchmarks/audit/compare.py` rewrites the `let n = … -- BENCH_N` constant of the Fa ports
  (before, Fa ran at its default size while Goose and Rust ran at the audit size: "output differs").
- `2374918` Compressed links (task 1) and packed literal choice (task 4):
  - `src/lower/trees.rs`: `compress_tree_links` (choose the arena object types with `T?` self-edges,
    clear `object_types`, re-declare every function), `compressed_edge`, `edge_address` (link → ptr:
    `zext × 8`), `address_edge` (ptr → link, cached for new nodes), `through_links` (new nodes
    addressed through one opaque link: inline asm identity, else LLVM keeps address and link both),
    `load_edge`. Edge values *and* roots are `i32` links (one global range, base address 0) — this
    departs from the decision text ("one range per arena"); written up in `design/decisions.md`
    under the decision, for the lead to confirm.
  - Runtime (`runtime/fa_runtime.c`, regenerated IR): `take_arena_page` reserves 2–32 GiB with
    `fa_os_reserve` (`MAP_FIXED_NOREPLACE`, halving under `ulimit -v`), commits 1 MiB at a time
    (`fa_os_commit`), named stop when full; `fa_compressed_links` set by `lower/mod.rs`.
    Arena bump/end moved to thread-locals (`tree_bump`, `tree_end`); runtime TLS is local-exec in
    executables (`Runtime::internalize`; C libraries still switch to general dynamic).
  - Sites decoding/encoding edges: `split_optional`/`pack_optional`/`llvm_type`/`same_layout`
    (values.rs), `unfold_rec`, `box_rec`, Rec copy loop (ownership.rs), `place_members`,
    `paired_boxes` (stride rounded to 8) (objects.rs), `payload_place` (patterns.rs), references.rs,
    Rec equality (arith.rs), Rec hash (tables.rs), `tree_root_pointer`.
  - `patterns.rs` `packed_literal_choice`: `when i is { 0 => "a", …, else => "z" }` with equal-length
    literals → one constant at `base + length × min(i, n)`.
  - Tests: `tests/programs/compressed-links.fa`, `tests/programs/packed-literal-choice.fa`.
- Uncommitted → committed with this file: `design/decisions.md` (implementation note under
  "Compressed links…"), `design/pending-documentation.md` (two notes).

## Tested

- `cargo test --release -p fa-codegen-llvm --test programs` (native, freestanding, wasm, leak
  checks): all pass with compressed links (run before the packed-choice patch); after it, the
  `packed-literal` programs pass on every target. `cargo clippy -p fa-codegen-llvm --all-targets`
  clean, `cargo fmt` done.
- The full `cargo test --workspace` (ulimit -v 8000000) was stopped by the pause after 33 suites,
  all ok, while running `fa-codegen-llvm` `programs` in debug: **rerun it**. Determinism and
  object-cache tests not yet confirmed.

## Measurements (instructions at audit sizes, callgrind, M; CPU noisy, load ~10)

| | records | strlist | respond | interp | words | lru | calc | sexp | binary-trees | scene |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before (M) | 83.7 | 94.7 | 207.7 | 17.3 | 30.5 | 23.2 | 130.9 | 93.6 | 136.8 | 9.4 |
| after (M) | 83.7* | 94.7* | 207.7* | 17.3* | 30.5* | 23.2* | 130.0 | 93.6* | 139.4 | 9.4* |
| Goose (M) | 73.4 | 81.1 | 189.7 | 17.6 | 22.4 | 17.9 | 109.6 | 77.5 | 128.1 | 7.9 |
| CPU Fa/Goose before | 0.95 | 0.99 | 1.20 | 1.20 | 1.04 | 1.53 | 0.94 | 1.17 | 1.08 | 0.95 |

\* not re-measured (unaffected code paths, except the arena TLS change for arena programs).
binary-trees: without compressed links (same build, `FA_COMPRESSED_LINKS=0`) 133.9 M; with 139.4 M;
D1 misses at 17: 3.5 M → 1.7 M (Goose 1.7 M); RSS 11.1 → 9.2 MB; CPU at 17: 0.144 s vs 0.160 s
uncompressed vs Goose 0.112 s (one 9-run series). calc CPU 0.98, binary-trees 1.17 (audit size).

## Next actions, in order

1. Rerun the full workspace test; fix anything red.
2. binary-trees: remaining gap is `itemCount` (62.8 M vs Goose `check` 51.6 M: no accumulator
   tail-recursion because `?? 0` lowers to phis; the `shl` decode per edge) and leaf calls paying the
   full prologue (one merged return block prevents shrink-wrapping; SimplifyCFG re-merges returns).
   Idea: peel a recursive function's call-free base case into its self-call sites.
3. sexp (task 2): pack recursive unions in arenas (member byte + payload, like `lower/packed.rs`);
   unions are not compressed yet (`compress_tree_links` only takes object types).
4. lru (task 3): hit path and 12-byte map slots (`lower/tables.rs`, inline map slots).
5. Full Goose-suite run: `python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --reuse --rebuild-fa --profile --runs 7 --report after.json` (env `GOOSE`, `GOOSE_CC`), and add a section to `benchmarks/README.md`.

Scripts used (scratchpad, not committed): A/B CPU with `FA_COMPRESSED_LINKS=0` builds; callgrind +
`benchmarks/audit/hot.py` for per-instruction profiles.
