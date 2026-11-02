# Async spike: LLVM coroutines for native Fa

A spike to de-risk the native async of Fa (one event loop per thread, colorless functions with sync and async variants, fair scheduling) with LLVM 18's switched-resume coroutines, built by hand with the IR builder, against hand-made state machines, Tokio and Go. It is not a design: it measures, and recommends.

- Code: `packages/codegen-llvm/examples/coroutines/` (`spike.rs` builds the IR with inkwell, `driver.c` is the allocator and run queue, `bench.mjs` runs wasm in node), tested by `packages/codegen-llvm/tests/coroutines.rs` (native at `default<O0>` and `default<O3>`, wasm at `default<O3>`; skipped without `LLVM_SYS_181_PREFIX`).
- Run: `cargo run -p fa-codegen-llvm --release --example coroutines [-- switches]`; Tokio: `cargo run --release -- [n] [current|multi]` in `benchmarks/async`; Go: `GOTOOLCHAIN=local ~/.local/go/bin/go -C benchmarks/async/go run . [n]`.
- Versions: LLVM 18.1.3 (inkwell 0.11, llvm-sys 181), rustc 1.99.1, gcc 13.3 (the C driver), Node 24.21.0 (V8, wasm), Tokio 1.54.1, Go 1.27.3 (linux-amd64, `~/.local/go`).
- Machine: a shared 6-core x86-64 VPS with a load average of 12 to 15 during the runs: the times are the best of 3 runs and vary by ±30 %; compare orders of magnitude and ratios, not digits.

## What was built

Every coroutine is `ptr f(i64)`, `presplitcoroutine`, with the canonical sequence: `alloca` promise → `llvm.coro.id(align, promise, null, null)` → `llvm.coro.alloc` ? `fa_alloc(llvm.coro.size.i64())` : null → `llvm.coro.begin`; each suspension is `llvm.coro.suspend` + `switch i8` (0 resume, 1 cleanup, default return); the cleanup is `llvm.coro.free` → `fa_free`; every return goes through `llvm.coro.end(handle, false, token none)` then `ret handle`. Callers use `llvm.coro.resume`, `llvm.coro.done`, `llvm.coro.promise`, `llvm.coro.destroy`. The frame allocator is the runtime's (`fa_alloc`/`fa_free`, size-class free lists), not `malloc`.

| Benchmark | What it does |
| --- | --- |
| `gen_opaque` | generator yielding `1..=n`; created and drained by two non-inlined functions (one heap frame, `n` resumes) |
| `gen_inline` | the same generator, created and drained in one function (candidate for `coro-elide`) |
| `sm_direct` / `sm_indirect` / `sm_inline` | the same generator as a state machine (`switch` on a state field of an explicit frame struct), called directly, through a function pointer in its frame (as a scheduler would), or inlined |
| `task_yield` | a task awaiting `n` children one after the other; each child pushes itself on the run queue, suspends once, then pushes its awaiter (the `waiter` of its promise) and reaches its final suspension |
| `task_sync` | the same, with children that return without suspending |
| `task_spawn` | a task creating 100 000 yielding children, then awaiting them in order (the Tokio/Go smoke test) |

## Measurements

**Passes.** `default<O3>` contains `coro-early`, `coro-split` (in the CGSCC pipeline, with `coro-elide` in the function simplification pipeline) and `coro-cleanup` (`opt -passes='default<O3>' -print-pipeline-passes`). `default<O0>` lowers coroutines too (`CoroConditionalWrapper`): 0 `llvm.coro.*` calls left at both levels, `.resume`/`.destroy` functions created (plus `.cleanup` at O0, used when the frame is elided). Without the coro passes, code generation aborts the process: `LLVM ERROR: Do not know how to promote this operator!` (`llc` on `spike.native.input.ll`).

**Cost per switch** (one `resume` that runs to the next `suspend` and returns), nanoseconds, `n` = 10^8 (tasks: 10^7; spawn: 10^5):

| Benchmark | Native | wasm32 (node) | Allocations |
| --- | ---: | ---: | ---: |
| `gen_opaque` (LLVM coroutine, heap frame) | 2.9 | 7.6–10 | 1 |
| `sm_direct` (state machine, direct call) | 2.6 | 6–7 | 1 |
| `sm_indirect` (state machine, function pointer) | 3.1 | 9–9.5 | 1 |
| `gen_inline` (coroutine, elided) | 0.5 | 1.6–2.8 | 0 |
| `sm_inline` (state machine, inlined) | 0.6 | 2.4 | 0 |
| `task_yield` (per await: alloc, 2 queue round trips, 2 resumes, free) | 17–33 | 38–41 | `n` + 1 |
| `task_sync` (per await) | ≈ 0 (folded) | ≈ 0 (folded) | 1 |
| `task_spawn` (per spawned task, 100 000 alive) | 92–100 | 84–276 | `n` + 2 |

**Spawn 100 000 tasks, yield once, await all** (checksum 9 999 900 000 everywhere), per task: Fa spike ≈ 0.1 µs; Tokio `current_thread` 0.65–0.86 µs, Tokio multi-thread 0.86–1.7 µs; Go goroutines (`WaitGroup`, `runtime.Gosched`) 1.2–3.3 µs (`GOMAXPROCS=6`), 2.4–4 µs (`GOMAXPROCS=1`). The spike has no waker, no reference count, no join handle, no cancellation and no I/O: it is a lower bound, not a fair fight; it says the LLVM primitive is not the bottleneck.

**Frame sizes** (bytes, native / wasm32): generator 48 / 40 (`{ resume, destroy, promise i64, n, i, index i1 }`: the header is two function pointers, the suspend index is a small integer at the end); yielding child 48 / 40; awaiting parent 72 / 56; spawning parent 80 / 64. State machine: 32 / 32 (one pointer, state, three `i64`). LLVM keeps `i` and the promise as two fields: frames are not minimal, but close (spilled values only, no stack).

**Code size** (bytes, native / wasm32): generator as coroutine 100 / 131 (ramp 61/65, `.resume` 34/55, `.destroy` 5/11) vs state machine 113 / 131 (`sm_new` 37/40, `sm_next` 76/91): equal. The ramp duplicates the code before the first suspension (`parent_yield`: ramp 209 + resume 178 bytes native), and every coroutine costs a `.destroy` (and a `.cleanup` at O0).

**Elision.** `coro-elide` works when the ramp is inlined into a caller that destroys the handle on every path: `gen_inline` has no allocation and becomes the plain loop of `sm_inline` (same speed). In `task_sync`, the children awaited right after their call are inlined into the (non-inlined) parent, their frames elided, `coro.done` folds to true, the parent's only suspension disappears (no `.resume` left) and the loop is folded to a closed form: an async function whose callees complete synchronously costs what its sync variant costs. A child that suspends (`task_yield`, `task_spawn`) escapes through the run queue and keeps its heap frame, as expected. A `noinline` coroutine is never elided.

**wasm32.** Coroutines lower fine (the resume and destroy pointers become `call_indirect` through the table; no stack switching, no `tail-call` feature, no Asyncify). An indirect call costs more in V8 than natively (signature check), so `sm_direct` beats both indirect forms there. V8 does not replace a running wasm loop with optimized code: without a warm-up, the long `gen_drain` loop stayed in the baseline compiler (Liftoff) and measured 2× slower; `bench.mjs` warms every benchmark up first.

## Recommendation for the architect

1. **Use LLVM's switched-resume coroutines** (`llvm.coro.id`, not `coro.id.retcon`/`async`) for Fa's async functions, natively and on wasm. They are the primitive "closest to LLVM": the frontend emits the straight-line function with suspension points, and LLVM computes the frame, the spills and the state machine, at the cost of a hand-written state machine (±10 %), with equal code size, at O0 as at O3. A Fa-side state machine transform would reinvent `coro-split` (liveness, spills, frame layout) for no measured gain.
2. **Frames from the runtime allocator**: `llvm.coro.alloc` + `fa_alloc(llvm.coro.size)` and `llvm.coro.free` + `fa_free`. With size classes, a frame round trip is a free-list pop/push; frames are small (40–80 bytes here).
3. **The sync variant of a colorless function may need no separate codegen**: an awaited call of a coroutine that does not suspend is inlined, elided and folded to the sync code. Emit the sync variant as a plain function only where the async one cannot be inlined (recursion, function values, cross-module without LTO); measure before duplicating.
4. **The task model of the spike is enough for an event loop**: the promise holds `{ waiter, result }`; `await` = `coro.done` ? read result : (store self as waiter, suspend); completion pushes the waiter on the run queue before the final suspension; the loop pops and `coro.resume`s. Fairness = FIFO queue (a yield pushes self at the back). ~20–30 ns per await that really suspends, natively.
5. **Link the scheduler into the module** (the Fa runtime is already IR): here `fa_queue_push`/`pop` were C calls that LLVM could not inline.

## Pitfalls found

- **inkwell has no `token` type**: `llvm.coro.id`, `coro.suspend`, `coro.end` and `coro.free` take or return tokens, and inkwell panics on them (`Unsupported type: Token`). Build those calls with `llvm-sys` (`LLVMBuildCall2`, `LLVMGlobalGetValueType`) and get `token none` from `LLVMConstNull(LLVMTokenTypeInContext(..))`; wrap the results with `IntValue::new`/`PointerValue::new`.
- **The passes are mandatory**: any pipeline without them (e.g. a custom pipeline string) aborts the process at code generation instead of returning an error. Always use `default<Ox>` or include `coro-early`, `cgscc(coro-split)`, `coro-cleanup`.
- **Mark the function `presplitcoroutine`**: `coro-split` only splits the functions that have it. Its other attributes are copied to the clones: a `noinline` coroutine gives `noinline` `.resume`/`.destroy` functions and is never elided.
- **Intrinsic signatures are version-specific**: in LLVM 18, `llvm.coro.end(ptr, i1, token)` has a third operand (`token none`); older versions take two operands, LLVM 19 adds `llvm.coro.await.suspend.*` (for a multi-threaded `await_suspend`), and later versions annotation-based elision. Pin the IR to LLVM 18 and re-test on upgrades.
- **Set the target data layout before the passes**: the frame layout (`coro.size`, field offsets, the `.Frame` type) is computed from it (48 bytes native, 40 on wasm32 for the generator).
- **Promise alignment**: the alignment given to `llvm.coro.id` and to every `llvm.coro.promise(handle, align, false)` must be the same, or the promise is read at the wrong offset (silently). Frames are aligned to their largest field; the allocator must return at least that (16 here). An over-aligned local (e.g. SIMD, align 32) needs `llvm.coro.align` and an aligned allocation, which `coro.alloc` does not request by itself.
- **Final suspension**: `coro.done` is "the resume pointer is null", true only at the final suspension; resuming there is undefined behavior (`unreachable` on the resume edge), and the frame must still be destroyed (`coro.destroy`) to be freed.
- **Waking before suspending is single-thread only**: the child pushes its awaiter on the queue and then suspends; that is correct with one event loop per thread (nothing runs in between), but a scheduler with other threads could resume and destroy the child first (the reason for LLVM 19's `coro.await.suspend`).
- **Liveness is the frontend's job**: a value live across a suspension is spilled to the frame, including copies (`i` and the promise here). Keep live state small; do not keep pointers to `alloca`s across suspensions unless they are in the frame (LLVM moves escaped allocas into it).
- **The example must link the crate**: an example or test that uses only inkwell and llvm-sys does not link `libLLVM` (the crate's build script does): `extern crate fa_codegen_llvm;`.
