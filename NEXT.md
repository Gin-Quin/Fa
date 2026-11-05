# Next actions (lead, paused 2026-11-05)

The compiler work lives on branch `compiler` (lead worktree `.claude/worktrees/lead`; never work in the main checkout, which is shared). Every arbitration is in `design/decisions.md`, open items in `design/todo.md`, documentation notes waiting for the external rewrite in `design/pending-documentation.md`, review reports in `design/reviews/`.

## State at the pause

- `compiler` passes the full suite with LLVM (`cargo test --workspace --no-fail-fast`, under `ulimit -v 8000000`) and the JavaScript runtime tests, at the last merge before this file.
- Three engineers were stopped mid-shift. Each wrote a `NEXT.md` at the root of its worktree, committed on its branch:
  - `worktree-agent-aca08cbb0b8394a82` (head `a3d64ac`) — proofs: JavaScript package builds collect claims and rebuild dependent modules from a re-analysis (`c5b9c8a`); proofs enabled in async natively, with async/parallel/worker soundness tests (`a1b2996`). Its full suite was cut short (wasm programs and later crates not run): rerun before merging. Not done: index proofs for packages (units must publish which file stores to each field or collection; plan in its `NEXT.md`). Package build on 1,000 files: user time 22 s → 25 s, 140–165 MB.
  - `worktree-agent-a375af73b4e738f36` (head `feb1e12`) — native performance: compressed 32-bit arena links done (one shared range for all arenas, nodes in the 2–32 GiB band, link = address / 8, roots hold 32-bit links too so methods need no second instance — accepted by the lead, it amends "one range per arena"); binary-trees L1 misses halved, peak memory 11.1 → 9.2 MB, but instructions 136.8 M → 139.4 M (Goose 128.1 M): close its walk and leaf-call gap first. `calc` packed literal choice done (130.9 M → 130.0 M, Goose 109.6 M). `benchmarks/audit/compare.py` now sets the ports' size constant (Fa ran at its default size before). Full suite stopped after 33 passing suites: rerun before merging. Not started: `sexp` packing, `lru` slots.
  - `worktree-agent-a2f066f35c6147839` (head `4a14b42`) — analyzer: 2^48 length bound done (sieve still proven); callbacks that throw in user functions done on JavaScript (refused natively as unsupported: the native instance per error set is still to do; on JavaScript a `catch` inside the callee can still catch the callback's errors, which the docs forbid); W-DECL-2, CV002, CL025, EF012, E-ERR-8 implemented. The last commit (42 codes made aliases) is untested; the full suite, workspace clippy, the LLVM expected skips and the 1,000-file measurement were not run: W-DECL-2 may break other crates' tests. Do all of that before merging. Left: documented codes not implemented, FN100–104 (needs `with`).
- To resume: read each branch's `NEXT.md`, merge the branches whose work is green (`git merge <branch>` in the lead worktree, then the full suite), and start fresh engineers from those notes.

## Next, in order

1. Merge the three paused branches (each after its suite passes; conflicts in the runtime IR: regenerate with `packages/codegen-llvm/runtime/generate.sh`; expected LLVM skips: `FA_UPDATE_SKIPS=1`, review the diff).
2. Generics flag day, step 2, once the rewritten documentation lands (`design/decisions.md`, "Flag day of the generics migration"): the old parenthesized spelling becomes an error, the transition code goes; then apply `design/pending-documentation.md` to the rewritten pages.
3. Goal of the user: Fa faster than Goose on every Goose benchmark (sync, release). Latest Fa/Goose instructions: records 1.14, strlist 1.17, respond 1.09, interp 0.99, words 1.36, lru 1.29, calc 1.19, sexp 1.21, binary-trees 1.07, scene 1.19 (CPU noisy: sexp and lru furthest).
4. Fuse the analyzer's passes (about nine per file, against the single-walk design), measured on the 1,000-file package.
5. Remaining items of `design/todo.md`: Pick/Omit and `EF004` from another file, `is` with sets, `@for` over key sets of another file, WebAssembly C libraries, `Pointer.allocate`/`free` on Bun, native AArch64 runtime, `Color.members.length` natively.

## Rules that held

- Engineers in their own worktrees, briefed with the files to read, told not to edit `documentation/` (external rewrite) and to commit often.
- Before assigning an item, check that no other engineer holds it (two duplicates happened).
- Delete the `target/` of finished worktrees after merging: the disk filled up once.
