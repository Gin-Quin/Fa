# Native performance: resumed state (2026-11-06)

Worktree: `agent-a375af73b4e738f36`, branch of the same name. Main checkout is shared and dirty;
do not copy its runtime or documentation edits. Lead integrates validated commits into `compiler`.

## Validated work

- The original compressed arena links and packed literal choices passed the complete workspace
  suite with LLVM, workspace clippy, formatting and all 296 JavaScript runtime tests.
- `04bb84b`: repaired stale C runtime test signatures/platform stubs and stopped treating an actual
  test compilation failure as “sanitizers unavailable”. The real ASan+UBSan runtime test passes
  outside `ulimit -v`; ASan's support probe fails normally under the full suite's 8 GB cap.
- `2d86c56`: scoped noalias metadata now rejects `i + k` when `k > Int64.maxValue - LENGTH_MAX`,
  with an IR test for both `i+1` and the wrapping offset. The new `LENGTH_MAX` constant is identical
  to the analyzer branch's; keep one definition and its length interval helper when merging.
- The follow-up integer return-path duplication and empty arena leaf inlining are complete.
  `src/optimize.rs` exposes a short pure integer return suffix before LLVM tail-recursion
  elimination; optional recursive sums become accumulator loops. `lower/inlining.rs` can bypass
  an empty arena leaf's recursive call, in either arm, transferring the leaf temporary correctly.
  Computed defaults and asymmetric children retain their semantics. New IR tests and
  `recursive-return-paths.fa` cover these cases.
- This exact source snapshot passed `cargo fmt --all`, `cargo clippy --workspace --all-targets`
  and the full `cargo test --workspace --no-fail-fast -- --test-threads=2`, LLVM included (exit 0,
  `/tmp/fa-perf-binary-workspace.log`). The two independent changes are committed separately.

Full suites use `flock --close /tmp/fa-compiler-full-suite.lock ...`; never pass the lock FD to test
children. Env: LLVM 18 prefix `/home/claude-dev/.local/llvm18/usr/lib/llvm-18`, Bun/LLVM on PATH,
Cargo jobs 1, full suite `ulimit -v 8000000`. Do not cap virtual memory for a dedicated ASan run.

## Actual release measurements

At audit size 14, binary-trees drops from 139,390,371 to 112,713,221 instructions; Goose uses
128,147,370 (Fa/Goose 0.88). Both outputs equal Rust. At size 17, 15 alternating runs on CPU 1
had CPU medians 121 ms/113 ms (Fa/Goose 1.07), with other compilation active: the CPU win remains
open. Do not substitute the earlier manually modified IR prototype's better timing.

The other nine programs were rebuilt and profiled too: no instruction regression. Current counts:
records 83.71 M, strlist 94.71 M, respond 207.41 M, interp 17.22 M, words 30.22 M, lru 23.16 M,
calc 129.99 M, sexp 93.18 M, scene 9.36 M. Saved reports and binaries are ignored files under
`benchmarks/audit/build/before/`: `binary-loops.json`, `binary-loops-suite.json`. Default-size CPU
samples: `/tmp/fa-native-perf-next/binary-trees-actual-cpu.json`.

## Prepared next changes, not in the repository

`/tmp/fa-native-perf-next/pending/` contains a ready first draft, syntax-checked Rust and runtime
sources tested together under ASan+UBSan (`/tmp/fa-native-perf-next/inline-map-tests.log`, exit 0):

1. `tables.rs`: 32-bit integer inline map keys select `TABLE_INLINE_KEY32`. Slots hold key/value
   only (8 bytes), with zero represented in one extra slot. `fa_runtime-map-inline.c` updates all
   find/append/remove/shift/grow/reserve/copy/clear/free paths; the test's randomized model checks
   both legacy and compact layouts, zero/extrema, wraparound collisions, independent copies.
   Hash32 multiplies unsigned raw bits, rotates right 24 with LLVM's intrinsic; only zero hashes
   to zero. The runtime uses the matching Clang builtin. Scratch generated code measured
   20,083,670 instructions for lru against 23,165,678 before and Goose 17,889,255: still behind.
2. `appends.rs`: dynamic counted append loops call `fa_string_reserve_inline`; the previous
   shortcut only accepted literal bounds and didn't turn an empty static string into an inline
   value. Every eligible loop can keep a short result inline, and the slow multi-byte/digit paths
   re-reserve in the same mode. The C tests assert no allocation for fitting prefixes/loops and
   safe growth for multi-byte suffixes. `compact-int-maps.fa` and `inline-append-loops.fa` parse
   and type check with the current compiler; they still need the backend run after applying.
3. Apply to this worktree only after the validated commits; regenerate the runtime IR, then run
   focused LLVM checks/programs, a real ASan run, fmt/clippy/full suite and actual compiler
   benchmark measurements. Lead is adding AArch64 runtime IR; it will regenerate all three IR
   files after merging the C, so do not overwrite its generator changes.

`/tmp/fa-native-perf-next/inline-loop-plan.txt` has the earlier investigation; use the pending code
and the latest actual counts above. Earlier map scratch replacement missed an LLVM constant
expression flag: `replace_tables_fixed.py` corrects it. Do not report that experiment as compact.

## Remaining work

- Stack strings 24–64 bytes. Lead approved a 64-byte frame buffer from initialization for a
  non-escaping mutable whose length may exceed inline room, including one initialized by `""`.
  A `let` and proven small maxima retain inline storage; immutable 24–64 byte constructions may
  use the frame. No extra mutator ABI. Every move/store between locals must copy into the
  destination's own frame buffer, including loop reuse; fallback heap beyond 64. The design
  research (not implementation) is `/tmp/fa-native-perf-next/stack-string-plan.txt`; its latest
  refinement considers a low capacity bit and even heap capacities to avoid reducing wasm's
  available capacity. Check borrowed parameters, captures, returned views, async and all stores.
- sexp uses **no arena today**: `List.items: Array<Sexp>` and `roots: Array<Sexp>` both fail the
  old single-file proof. Lead approved conservative regions spanning root/child collections,
  shared across relevant units (reject when unavailable), then compact member payloads. Preserve
  Errors and C ABI. Cover copies/subtrees, mutation, collection refs, returns, captures, branches,
  separate units and conservative fallback. Proofs agent doesn't touch arena classification.
- lru still pays for index×16 link decoding and a separate free array; Goose keeps byte offsets
  relative to `pool.items` and its freelist in dead slots. Keep programs unchanged and optimize
  only proven generic shapes; no benchmark names or expected values in the compiler.
- Remaining binary-trees CPU gap, words, scene, other ratios and a full 25-program Goose run
  still need work. The final goal is every sync release benchmark; fewer instructions alone
  do not establish a CPU win.

## Audited older todo items

Already implemented/tested: capacity/reserve/from; borrowed aggregate parameters including
function values/defaults; module constants; nullable recursive pointers; root arenas; characters
as UTF-8 views and compact character arrays; nbody's scoped noalias (now with a wrapping guard).
The stack buffer part is genuinely missing; root collections/inter-unit arena proof and automatic
relational narrowing of lru's integer links are also still open. Lead is cleaning the shared todo.
