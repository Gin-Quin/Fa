# Async in Fa: design

Status: accepted by the project lead (2026-11-02), with the amendment of A2 below. Steps A1, A3 and A2 are implemented in the analyzer (`packages/analyzer/src/check/tasks.rs`; the backend contract is documented in `packages/analyzer/src/analysis.rs`, section "Async"). Implementation choices: a loop back-edge is an interleaving point in the body of an `async` function, an async lambda or a `start do`, and, since the follow-ups, at the top level and in tests too, where only what other tasks may change is forgotten; the borrow of a task that is not bound to a local (`tasks.push(start f(x))`) lasts to the end of the function, and the borrows of a task bound in a block end with the block (`design/decisions.md`, after the second analyzer review); an awaited value of an `async` function type counts as blocking everywhere (only `suspends` is conservative). Steps N1 and N2 are implemented natively (`packages/codegen-llvm/src/lower/tasks.rs`, the "tasks and the event loop" section of `runtime/fa_runtime.c`, tested by `tests/programs/async-*.fa`, `tests/async_ir.rs` and the sanitized `runtime/test/executor_test.c`). Implementation choices: the top level of an asynchronous program is a coroutine that `main` runs with `fa_run_main`; a started call that is not a direct call of a function (a function value, a built-in) runs at once and gives a finished block, and an `await` of an async function value is a blocking call (until function values carry their async instance); `fa_resume`/`fa_destroy` are the `fastcc` calls that `llvm.coro.resume`/`destroy` lower to, so finished blocks need no coroutine pass in synchronous mode; the coop point of A2 is at the start of each iteration (the same count as the back-edge), and "the turn lasted more than 2 ms" is measured from the first time the budget was spent in the turn (no clock read per turn); a blocking wait on a channel or a mutex outside a parallel loop stops with `deadlock` (no other thread can end it); the blocking pool (`fa_blocking_submit`) and the descriptors of epoll (`fa_io_register`/`fa_io_wait`) wait for their Fa APIs (S1); workers keep D31 (N3); in asynchronous mode, wasm reported `CONC011` until W1. Step J1 is implemented in the JavaScript backend (`packages/codegen-js/src/generate/tasks.rs`, runtime `packages/codegen-js/runtime/tasks.js`, tested by the native `async-*.fa` programs run on Bun, `cargo test -p fa --test programs async`). Implementation choices: an async instance takes its task first (`f___async($task, …)`), so no global "current task" is needed; the waits of the runtime (`faJoin`, `faSleep`, `faLock`, `faSend`, `faReceive`, `faYield`, `faCoop`) check the cancellation when they resume and throw a signal that no `catch` keeps, so a cancelled task runs its `finally` blocks (guards are released) a microtask later, while `cancel()` itself (ending the wait, cancelling the owned tasks) is synchronous; a task owns the tasks started in it that aren't awaited yet, cancelled when it ends (besides the drop at the end of the scope of a local); an `async` function that loops gets an async instance even when it can't suspend (its loops spend the coop budget, like the native started calls); a loop of an async instance checks the clock every 128 iterations and gives the thread back with a macrotask once the current macrotask lasted 4 ms, and 128 awaits resolved without waiting also give it back; an async lambda was a blocking function (both instances since the follow-ups below); a deadlock is detected when the event loop empties (`beforeExit`) before `main` ends; `fa test` ran the tests blocking (the tests that await run as tasks since the follow-ups). An `await` of a mutex or a channel that is ready doesn't `await` at all (`faLockNow`, `faSendNow`, `faReceiveNow`, until the coop budget is spent), as natively, so the native programs print in the same order on Bun; an awaited call of an async instance still lets the other tasks run (a JavaScript `async` call), which A1 allows. Step W1 is implemented (the wasm section of the event loop in `runtime/fa_runtime.c`, `wasm_event_loop` in `src/lower/tasks.rs`, `host/loader.mjs`; the native `async-*.fa` programs run on wasm). Implementation choices: the timers stay in the executor's heap, read with the host's clock (the import `fa.clock`, `performance.now()`), and `fa.timer(token, ms)` only asks the host to run a batch at the next deadline (the latest request replaces the earlier ones), so that cancelling a sleep needs nothing from the host; `fa_run` (exported) runs a batch (the timers, at most 61 turns) and says whether tasks are still ready (the loader calls it again from a `setImmediate`, or a `MessageChannel` without it), idle (a timer was asked for) or done; `fa_wake(token)` (exported) queues the task of a wait the host completed (for `fa.fetch` and the sockets, later); the end of `main` (flush, leak check, exit code) is `fa.finish`, called by the runtime once the root and the detached tasks are done; a deadlock is detected by the batch itself (nothing ready, no timer). Sources: `documentation/advanced/concurrency.md` (with its `:::implementation` section), the reference's "Concurrency" section, the advanced brief §14 and D31, `design/decisions.md` ("Concurrency without async"), `design/compilation-with-llvm.md` (Q4, Q9, "Async, workers and `for parallel`"), `design/llvm-backend-plan.md`, `design/compiler-architecture.md`. Code read: `packages/analyzer/src/check/concurrency.rs` (CONC011), `packages/analyzer/src/types/relate.rs` (the `is_async` rule already exists), `packages/codegen-llvm/src/lower/parallel.rs`, `packages/codegen-llvm/runtime/fa_runtime.c` (pool, `fa_heap_alloc` with thread-local size classes), `packages/codegen-js/runtime/concurrency.js`.


Steps S1 and N3 are implemented natively, with the async function values of §2.2 (the I/O surface for the backends is documented in `packages/analyzer/src/analysis.rs`, "The I/O of the standard library"; the native part is `src/lower/io.rs`, the end of `src/lower/tasks.rs`, and the "sockets and files" and "workers" sections of `runtime/fa_runtime.c`; tested by `tests/programs/async-net.fa`, `async-files.fa`, `async-workers.fa`, `async-worker-stress.fa`, `async-function-values.fa` and `runtime/test/executor_test.c`). Implementation choices:
- **S1.** `std.time.monotonic()` (nanoseconds); `File.read`, `File.readBytes`, `File.write(path, text)` run in place when blocking and on the blocking pool when awaited; `std.net`: `TcpListener { host = "127.0.0.1", port }` listens when evaluated (a failure is thrown by its first `accept()`; `port = 0` takes a free port, read back with `listener.port`), `accept()`, `TcpStream.connect(host, port)`, `read(buffer)` (`mutates buffer`, an effect on the argument's place), `write(bytes, count = bytes.length)` (the `count` lets an echo write back what it read without a slice), `close()`; hosts are IPv4 addresses or `localhost` (no name resolution yet). Sockets are `SyncKind::TcpListener`/`TcpStream` (move-only resources) and natively `FaShared` blocks whose drop closes the descriptor. Sockets block only on `native`; files everywhere but `web`; `CONC010` is not reported beside `AN004` for a primitive missing on the target. A kept task whose argument is a temporary owning memory is `CONC018` (the use after free found by N1/N2); a start awaited at once and a worker call (which copies) are exempt.
- **Event loop.** The timer heap keeps the deadlines inline (a sift reads one cache line per level: 1M timers went from 4.5 s to 2.4 s); a readiness seen at `EAGAIN` is stale (epoll is only read between turns), so it is cleared without a system call (echo throughput doubled); the descriptors are also polled after a tick in which a task yielded (`Task.yield`, the coop budget), as Tokio defers `yield_now`, so that I/O comes in between the turns of yielding tasks; epoll returns up to 256 events.
- **N3.** In asynchronous mode a worker is `Repr::Sync`: a block holding its state and an OS thread that runs its calls one at a time (A9) with the **synchronous instance** of the method (a blocking call on the worker's thread: an `await` there waits, a `start` runs at once; the worker's thread has nothing else to run, by A9). A call is a message whose frame has a task's layout (the generated resume and destroy functions call `fa_call_resume`/`fa_call_destroy`): awaited or started, it is the task, ended through the caller's remote queue; a blocking caller waits on the worker's condition variable. Arguments are copied into the message, the result moved back. Each call holds a handle on its worker, so the block outlives the tasks of its calls whatever the order of the drops; `stop()` and the last handle cancel the queued detached calls, let the others run and join the thread (A10). Without threads (freestanding) a call runs at once. Synchronous programs keep D31.
- **Async function values.** A value keeps two words: the entry of an `async` function's value is a constant table `{ sync, async }` tagged with its low bit (a value of a function that isn't `async` has none, and awaiting it calls it); awaited in an async instance the async entry runs, a kept `start` queues it, a blocking call reads the synchronous entry. This keeps copies and drops of function values unchanged (no third word).
- **`WorkerPool`** (decided in `design/decisions.md`): a pool block holds a copy of `state` and its workers, each made from a copy (`spawn`, generated); `submit` is a generated coroutine that takes a free worker (or waits in the pool's FIFO line), runs the work (its async entry) and gives the worker back; with a `timeout`, a timer armed beside the work cancels it when it fires first, replaces the worker by one made from a fresh copy of `state`, detaches the old one (its thread ends the abandoned call, then frees it) and fails the task with `WorkerTimeout`. The pool and its tasks are used by one thread. Leaving `state` out works when every field of the worker type has a default, but the analyzer doesn't know the defaults of a type of another file yet, and a worker type is always one.
- **Lost wakeup.** The executor parked in epoll could miss the end of the last remote source (it read `sources` before setting `parked`): it now checks `sources` again after setting `parked`, and `fa_run_main` frees the executor only once no other thread is inside `fa_remote_source_end` (`ending`). The sanitized test passed 300 runs in a row (one hang in 150 before).
- **Wakes before yields** (`design/decisions.md`): the executor keeps a woken queue (timers, I/O readiness, remote wakes, a finished task's waiter, new tasks) and a yielded queue (a spent coop budget, `Task.yield`, a back-edge yield, a sleep of 0); a tick serves the woken tasks first, at most 30 of its 61 turns when both queues hold tasks (wasm shares it). `starve` went from 25 ms to 17–19 ms of p99 lateness: timers still fire only between ticks, and a tick of budget-spending turns lasts about 5 ms. `WorkerPool.submit` is `async`: an awaited `submit` waits while `maxQueueSize` calls are queued (a call leaves the queue when it takes a worker) and gives the task. Called blocking (allowed natively), it waits on a condition variable: the workers' threads pull the jobs from the pool's queue themselves (a job runs the submitted function with the worker, on the worker's thread, where the worker's own calls run in place), so room appears whatever the submitter waits on. The executor also fires the expired timers every 8 turns of a tick (`design/decisions.md`), so a timer waits a few turns at most.

The JavaScript side of S1 and step J2 are implemented (`packages/codegen-js/src/generate/system.rs`, `workers.rs`, `tasks.rs`; runtime `system.js`, `net.js`, `workers.js`; the native `async-*.fa` programs, multi-file ones included, run on Bun; decisions in `design/decisions.md`, "Async follow-ups on JavaScript"). Implementation choices:
- **S1 on JavaScript.** `monotonic()` is `performance.now()` in nanoseconds; files go through `node:fs` (`*Sync` blocking, `fs.promises` awaited, a cancelled wait drops the result); errors carry the text of `strerror`, as natively. Sockets go through `node:net`, awaited only (`CONC010` blocking): a stream buffers what it receives (paused beyond 64 KiB), sockets are `unref`'d unless a task waits on them, so they neither hold the process nor hide a deadlock. A `start` of a primitive runs its awaited form as a task (`faStart($task, faReadFileAsync, path)`). The blocking socket calls of `async-net.fa` moved to `async-net-blocking.fa` (native only).
- **Async function values on JavaScript.** The value of an `async` function that may suspend is its synchronous instance carrying the async one as `$async` (`faAsyncValue`); an async lambda that may suspend gets both instances. Awaited or started calls through a value pick the instance at run time (`faCallValue`, `faStartValue`); a blocking call is a plain call. A detached task copies the arguments and captures its `StartInfo::copies` lists (A12).
- **J2.** A worker is a thread (`worker_threads`, Web Workers) serving the calls one at a time with the synchronous instance of the method; a call names its method `"<URL of the module>#<export>"`, so the thread imports the worker's module on its first call. A started call sends a prepared message at once (its arguments are copied at the `start`); the caller commits execution from a microtask, one call at a time after the previous answer. A cancelled queued call is discarded before its commit; cancellation during an import is checked again before entering the method. `WorkerPool` on JavaScript: awaited `submit`, `maxQueueSize`, a `timeout` terminating and replacing the worker; the submitted function runs on the main thread.
- **Tests and the top level.** A test that awaits runs as a task on Bun (`faTestTask`); the loops of the top level and of tests spend the coop budget on every backend, and the analysis forgets there only what other tasks may change.

Checked on the installed LLVM 18 with IR piped through `opt` and `llc`, no files written:
- A switched-resume coroutine that allocates through `fa_heap_alloc(coro.size)` splits into `f.resume`, `f.destroy` and `f.cleanup`, with a 64-byte frame.
- It compiles for `wasm32`.
- `default<O0>` already runs `coro-early`, `coro-split` and `coro-cleanup`.
- `default<O2>` adds `coro-elide`.
- LLVM 18 has no `llvm.coro.await.suspend.*` intrinsics and no `coro_elide_safe`. Both arrived in LLVM 19 and 20.

---

## 1. Semantics recap and decisions

### 1.1 Recap (what the docs already say)

- **Colorless.** An `async function` may suspend, and the caller chooses how to call it:
  - **blocking**: the synchronous instance runs;
  - **awaited**: `await f()` suspends the caller;
  - **started**: `start f()` gives a `Task(T) throws E`.
- A non-`async` function is always synchronous. When it calls an `async` function, it blocks. `async` is never inferred, and `async` is part of the function type. A non-async function type cannot hold an async function. An async function type can hold a non-async one.
- **Async contexts**: async function bodies, async lambdas, `start do`, tests, and the top level of a standalone file. `await` takes an async call or a `Task`. `start` takes any call or a `do` block.
- **A started task does not run right away.** It begins when the starter suspends or returns to the event loop. Tasks of one thread run one at a time and take turns at `await`. Threads only come from workers and `for parallel`.
- **Blocking mode.** Inside a blocking call:
  - `await` blocks;
  - `start` runs eagerly and gives a finished task;
  - awaiting a task started outside the blocking call is an error.

  A thread-local blocking-depth counter drives this for non-async functions that contain `start`.
- **Tasks are structured and move-only.**
  - `await task` consumes the task.
  - Leaving the task's scope cancels it and waits for it.
  - `cancel()` is cooperative: the task stops at its current suspension point, and its scope cleanups run.
  - Awaiting a task that may have been cancelled adds `Cancelled` to the errors.
- **Detached tasks** (a `start` whose task is not kept):
  - they copy their arguments and captures;
  - they cannot have `throws` and cannot use requirements;
  - the program waits for them before it exits.
- **References.** A task borrows what it uses until it ends: its arguments, the locals a `start do` uses, the module-level values in its effects, and its requirements. A task cannot outlive those values. At every `await`, what is known about module-level `mutable` values is forgotten.
- **Combinators.** `Task.all`, `Task.any` and `Task.settled` move the tasks in and return a `Task`. `all` and `any` cancel the losers.
- **Requirements cross `start`** (same thread) but not workers. Worker methods behave like async functions from the caller's side, and arguments and results are copied. `Mutex.lock`, `Channel.send` and `Channel.receive` are async.
- **Blocking per target is part of a function's interface.** It is inferred from the call chain down to the primitives. When a primitive has no synchronous implementation on a target, a blocking call to it is a compile error there, with the call chain in the message.

### 1.2 Ambiguities and decisions

| # | Question | Decision |
| --- | --- | --- |
| A1 | Does every `await` let other tasks run, including one that completes without waiting? | **No.** Other tasks *may* run at any `await`. They *do* run when the awaiting code actually waits, or when the coop budget forces a yield (§3.6). An awaited call that finishes without waiting is a plain call. A started task therefore begins at the starter's next *waiting* `await`. The checker already treats every `await` as a possible interleaving point, so this is sound. Native and wasm are deterministic. JavaScript may interleave earlier (microtasks), which is allowed by "may". Update the docs sentence "it starts when the current code suspends (at its next `await`)". |
| A2 | Yield points at loop back-edges? | **Yes, in async instances only (lead's amendment, for the user's "no stuck request" requirement).** The back-edge of a loop in an async instance decrements the thread's coop budget (one load, decrement, branch); when the budget is spent *and* the turn has lasted more than 2 ms, the task re-queues itself at the tail, exactly like `await Task.yield()`. The checker treats such a back-edge as a possible interleaving point, like an `await`: it forgets what it knew about module-level `mutable` values there (sound; the loop body of an async function can't rely on them across iterations). Synchronous instances (including the synchronous helpers an async function calls) never yield, so hot CPU code keeps full optimization; dev builds warn about turns longer than 50 ms spent in synchronous code, pointing to `Task.yield()`, a worker or `for parallel`. On JavaScript, the same back-edge check awaits a macrotask (`faCoop`). |
| A3 | `for parallel` inside async code | **It stays a blocking construct.** The calling thread joins the pool, and its event loop is paused for the loop's duration. Releasing the loop would let other tasks write module-level values that the parallel body reads (a data race). Pool threads have no event loop. Calls in a parallel body are blocking calls (synchronous instances). |
| A4 | What does "may suspend" mean (which functions need an async instance)? | `may_suspend(f)` holds when f is declared `async` and its body has an `await` of one of the following: a call to a function that may suspend; a `Task`; a suspending primitive (sleep, I/O, `lock`, `send`, `receive`, a worker call, `Task.yield`); a call through an `async` function *value* (counted conservatively). It is a fixpoint over recursive cycles. An `async` function that cannot suspend gets **one** instance, and `await f()` of it is a plain call. |
| A5 | Program-level switch | `build.asynchronous: Boolean?` in `BuildOptions` (`async` is a reserved keyword, so it can't be the field name). CLI: `--asynchronous=true\|false` on `fa build`, `fa run` and `fa test`, next to `--overflow-checks`. Per-target `overrides` work. **Unset means detection** (A6). |
| A6 | Detection rule | Asynchronous mode is on exactly when the entry's **transitive summary** has `starts` (a reachable `start` or `start do`) or `suspends` (a reachable `await` that may suspend). This also counts the top level of a standalone file and, for `fa test`, the test bodies. `start` alone turns it on, because deferred start changes the output order. `async function main` that never waits leaves it **off**. |
| A7 | Off mode | The whole program is one blocking call of `main`. Only synchronous instances exist, with no coroutine, executor, epoll or frames. `start` is eager and gives finished tasks, and combinators work on finished tasks. A primitive with no synchronous version on the target is `CONC010` at the entry, with the chain. Forcing off while the program uses `start` gives the warning `WCONC001` ("tasks run to their end when started: output order may change"). A wait that no thread can end stops with `stop: deadlock: … (built with asynchronous = false)`. |
| A8 | Cancelling an operation already in flight | Timers and socket waits are deregistered at once. An offloaded file operation or a worker call **already running** finishes in the background, and its result is dropped (its side effects may still happen). Worker calls still queued are removed. This is documented. |
| A9 | Worker running an async method | A worker runs **one call at a time, even across the `await`s inside a method**. The next queued call starts when the previous one finishes. A method may `start` its own tasks. |
| A10 | Detached tasks and program exit | The main thread's executor runs until `main`'s task and the detached tasks of the main thread are done. `stop()` on a worker waits for the current call and for that worker's detached tasks, and cancels the queued calls. |
| A11 | When `Cancelled` is added to an `await` | For a local task binding: when `cancel()` may have been called on it (a flow fact). For a task that comes from a parameter, a field or a collection element: always (the cancel may have happened elsewhere). For the result of a combinator call: never. |
| A12 | Detached `start` of move-only values (`TcpStream`) | "Copied" means **moved when it is the last use**, as everywhere else. Otherwise it is `MEM002`. `start handle(connection)` inside an accept loop works. |
| A13 | Task in a synchronous instance | A *finished block* with the same header layout as a frame (§3.2). `await task` reads its result. Awaiting a task received from outside makes the synchronous instance impossible on every target (`CONC010`, chain "awaits a task it receives"). |
| A14 | Watches and suspension | Pending `watch` blocks run before a **real** suspension (A1). |
| A15 | `await start f()` and `await Task.all(...)` | Peephole: awaiting a task-returning expression right away is a direct call of the async instance. No queue round-trip. |
| A16 | Fairness between tasks | **FIFO.** No LIFO slot. Mutex and channel waiters are FIFO with direct hand-off, so no barging. |
| A17 | New primitive | `Task.yield()` (async, a prelude member of `Task`). It re-queues the task at the tail. It does nothing in a blocking call. |

---

## 2. Analyzer

### 2.1 Summaries (part of the interface and of its hash)

These are computed during the single walk, like effects. They are transitive through callees, and recursive cycles are settled the same way effects are.

```rust
pub struct AsyncSummary {
    pub suspends: bool,        // may suspend when awaited (A4); false for non-async functions
    pub starts: bool,          // a `start` is reachable from the body
    pub blocking: TargetSet,   // targets where the synchronous instance can exist
}
```

- `blocking` is the intersection, over awaited and blocking callees, of their `blocking` sets, and of each primitive's synchronous targets. Awaiting a received task clears it. On JavaScript and wasm, `lock`, `send`, `receive` and worker calls clear it, which generalizes the current CONC010 rule.
- **The program decision is O(1)**: `configured.unwrap_or(entry.suspends || entry.starts)`. There is no whole-program pass, so streaming is preserved.
- In detection mode, only the **async instances** depend on the decision. The backend defers emitting them until the entry module is analyzed (the entry is analyzed last). Everything else streams.
- A non-async function containing `start` always gets the dynamic depth check. In off mode the runtime starts with depth 1, so that `start` is eager. Its code is therefore the same in both modes.

### 2.2 Contract for the backends (`Analysis` side tables)

| Table | Content |
| --- | --- |
| `async_summaries: FxHashMap<NodeId, AsyncSummary>` | Per function declaration and lambda. |
| `awaits: FxHashMap<NodeId, AwaitInfo>` | `kind`: `Call` (callee instance chosen by the backend), `Task`, `FunctionValue`, `Primitive(name)`. `suspends: bool`. `cancelled: bool` (A11). `immediate_task: bool` (A15). |
| `starts: FxHashMap<NodeId, StartInfo>` | `detached`; for a detached start, the moved/copied values (captures and arguments); the scope owning the task (its drop point); requirement arguments (as in `requirement_args`). |
| `Program::asynchronous: bool` | The resolved mode for the target being compiled. |
| Function types | `Signature.is_async` already exists. An async function value is `{sync, async, env}` natively and two properties on JavaScript. In off mode it is only `{sync, env}`. |

**Instance selection (mechanical, in the backend).** A call node uses the **async instance** when all of the following hold:
- it is awaited or started;
- the enclosing instance is an async instance;
- the program is asynchronous;
- the callee `suspends`.

Otherwise it uses the synchronous instance. Instances are generated on demand from the entry, and each one is cached separately (`fa_codegen` instance key gets an `async` bit).

### 2.3 Checks

CONC011 is retired. It stays only as "not supported on target X yet" during the rollout, for targets whose backend lands later.

| Code | Condition |
| --- | --- |
| `CONC010` (generalized) | A blocking call whose chain reaches a primitive with no synchronous version on the target. Also: off mode impossible on the target. Message: `fetchData → std.http.get → fetch (async-only on web)`. |
| `CONC014` | `await` outside an async context, including parallel bodies, `watch`, `derived`, and non-async functions and lambdas. |
| `CONC015` | `await` of a call of a non-async function ("call it directly"). |
| `CONC016` | A detached task whose function or block may throw ("handle its errors with `catch` inside"). |
| `CONC017` | A detached task uses a requirement or a borrowed local it cannot copy. |
| `CONC018` | A task outlives a value it uses (returned or stored while borrowing a local or a requirement). |
| `CONC019` | `start` of a worker method on a non-`mutable` handle when the method `mutates self`, or a worker handle used while a started call is pending. |
| `WCONC001` | Asynchronous mode forced off while the program contains `start`. |
| Existing codes reused | Task copy or use after move: `MEM002` (Task is a resource kind). Modifying a value a task reads, or using one it modifies: the REF codes with a "used by `task`" note. Async in a non-async function type: the existing `relate.rs` mismatch, with the message "a call through it would have to block". |

Flow: at each `await` (A1), facts on module-level `mutable` places are killed and references into them are invalidated. Tasks are pending borrows until their binding is awaited, cancelled or dropped. Moving a task into `Task.all` transfers the borrow.

---

## 3. Native

### 3.1 Coroutine lowering: LLVM switched-resume, decided

| Option | Verdict |
| --- | --- |
| **LLVM coroutines, switched-resume** (`coro.id`, `coro.begin`, `coro.suspend`, `coro.end`, `coro.free`) | **Chosen.** LLVM computes the frame: it spills only what lives across suspends, and overlaps allocas by liveness. The tree-walking backend keeps its entry-block allocas unchanged. Each `coro.suspend` has a **destroy edge**, which is exactly Fa's "cancel at a suspension point runs the scope's cleanups": the backend emits its existing cleanup stack there. This is mature (C++20 and Swift use it), works at `-O0`, and works on wasm32 unchanged. |
| Hand-written state machine | Rejected. It duplicates CoroFrame (liveness, spilling, frame layout) in a backend with no SSA of its own, for no gain. |
| Stackful (goroutine-like) | Rejected. It needs at least one 4 KiB page of resident memory per task plus a guard (1M tasks is more than 8 GB, against roughly 100 to 200 B per Tokio task). It needs assembly context switches per architecture, and is impossible on wasm without stack switching. Cancellation would need unwinding, which the backend has no tables for. |
| `coro.id.async` (Swift) or retcon | Rejected. It requires tail-call conventions everywhere (wasm tail calls) and is more complex than switch lowering. |

**Instance shape.** `ptr name.async(params…, i1 lazy)` is a `presplitcoroutine`, `fastcc` ramp.
- Its body is the same lowering as the synchronous instance.
- Each suspending `await` becomes a suspend point with a resume edge (continue) and a destroy edge (cleanups, then `coro.free`).
- `lazy` adds an initial suspend for `start`. Awaited calls pass `false`, which folds away after inlining, so they run eagerly until their first real wait.
- **Promise**: `{ FaTaskHeader (32 B), result: T | E }` at the fixed offset `coro.promise` gives (16 B on 64-bit).
- **Allocation**: `coro.alloc` → `fa_heap_alloc(coro.size.i64)`, and `coro.free` → `fa_heap_free`. That allocator already has thread-local size-class free lists, which are exactly Q4's per-thread frame allocator. Tasks never migrate.
- **Done-ness**: `coro.done` (LLVM nulls the resume pointer at the final suspend).
- **Calling convention**: resume and destroy are `internal fastcc`. The C runtime must never call them directly. Tier-0 IR adds `fa_resume(ptr)` and `fa_destroy(ptr)` (C convention, wrapping `llvm.coro.resume` and `llvm.coro.destroy`), inlined by LTO into the bitcode runtime.
- **Dev profile**: its pipeline is currently "none", so it must run `coro-early,cgscc(coro-split),coro-cleanup` (or `default<O0>`).

**`await g(x)` in an async instance:**
```
h = g.async(x, lazy=false)               ; runs g eagerly
if coro.done(h) goto ready               ; fast path: no executor involved
fa_task_await(h, self)                   ; h.header.parent = self
switch coro.suspend(): 0 → ready, 1 → cancel: fa_destroy(h); <caller cleanups>; free
ready: move result out of coro.promise(h)+32; fa_destroy(h)   ; frees the child frame
```

**Continuations use a trampoline, not musttail.**
- At its final suspend, a coroutine calls `fa_task_finish(self)`.
  - If `parent` is a frame, it sets the thread-local `fa_next = parent`.
  - If `parent` is a join node, it notifies the node.
  - If the task is detached, it sets the thread-local `fa_to_free`.
- The executor's turn loop is `while (h) { fa_resume(h); h = take(fa_next); }`. This gives O(1) resumption of the waiting chain with no stack growth, and needs no tail calls (LLVM 18 has no `coro.await.suspend.handle`, and wasm tail calls are not universal).
- `musttail` symmetric transfer is a later optimization on targets that support it.

**Frame elision.** LLVM 18's `coro-elide` does not fire for awaits that cross a caller suspension. Baseline: one size-class pop and push per *suspending* awaited call. Calls that cannot suspend never create frames (A4), which covers most helper functions. Step N5 bumps the pinned LLVM to 20 or 21 (as `llvm-backend-plan.md` §6 already recommends) and marks awaited call sites `coro_elide_safe`, so the child frame lives inside the parent's frame (Rust-like zero allocation).

### 3.2 Task representation

- A `Task` value is one word: a pointer to a frame, or to a *finished block* (synchronous instances, A13) with the same header. The sentinel `1` means cancelled.
- `FaTaskHeader { void *parent; FaTaskHeader *next, *prev; uint32_t state; uint32_t flags; }`. The states are `CREATED/QUEUED/WAITING/DONE`; the flags are `DETACHED` and `JOIN`. The queue links are intrusive, so queueing never allocates.
- **Drop of a task** (`fa_task_drop`):
  - `DONE`: destroy it, which frees it.
  - `QUEUED`: unlink it, then destroy it at the initial suspend.
  - `WAITING`: `fa_destroy`, which runs the destroy edge: cleanups, deregistering its wait node, recursively destroying an awaited child frame, and dropping its own tasks.

  All of this is synchronous: one thread, and the cancelled task is never running at that moment.
- `cancel()` performs the drop and stores the sentinel. A later `await` throws `Cancelled`.
- **Combinators**: a `FaJoin { waiter, remaining, done ring, mode }`. A task's `parent` may be a join node (tag bit). `Task.all`, `any` and `settled` are small coroutines generated per `(T, E)` in `lower/tasks.rs`, looping on `await join.next()`. This is O(n). The losers are dropped (cancelled).
- **Primitive waits** use a `FaWait` node that is an *alloca of the waiting frame*, so a wait allocates nothing. The node is linked into a timer heap, an fd registration, or a mutex or channel waiter list. Its destroy edge calls `fa_wait_cancel(node)`. A waker is a raw frame pointer, with no reference count and no vtable. This is safe because a registration never outlives its frame.

### 3.3 Runtime: one executor per thread (C, `fa_runtime.c`, linked as bitcode)

- **Created lazily** on the main thread (asynchronous programs: `fa_run_main` runs the entry inside it) and on each worker thread. Pool threads never have one.
- **Ready queue**: an intrusive, doubly linked FIFO. No atomics.
- **Remote queue**: a Vyukov intrusive MPSC queue plus an `eventfd` registered in epoll. It carries wakes from other threads (mutex and channel hand-offs, worker completions, blocking-pool completions). The `eventfd` is written only when the target executor is parked (an atomic `parked` flag).
- **I/O: epoll, edge-triggered, decided.** Readiness only, which makes cancellation trivial (forget the interest) and is what Tokio and Go use.
  - Sockets are registered once (`EPOLLIN|EPOLLOUT|EPOLLRDHUP|EPOLLET`). The registration keeps readiness bits plus one read waiter and one write waiter.
  - An operation tries the syscall. On `EAGAIN` it clears the bit, links the waiter and suspends.
  - **io_uring is not used first.** Cancelling a completion-based read whose buffer lives in a frame is unsafe, and io_uring is often blocked by seccomp in containers. It can come later behind the same API, with owned buffers.
  - macOS uses kqueue with the same structure. The freestanding target makes epoll raw syscalls.
- **Files, stdin and DNS** go to a **blocking pool**: up to 64 threads, lazy, idle for 10 s before exiting.
  - A job is a heap block holding owned buffers, an atomic state, the executor and the frame. Its completion goes through the remote queue.
  - Cancellation sets `ABANDONED`, and the pool thread frees the job (A8).
  - With no threads (freestanding), these run inline.
- **Timers**: a **4-ary min-heap** per executor (Go's choice). The `FaWait` node stores its heap index, so cancellation is O(log n). The clock (`CLOCK_MONOTONIC`) is read once per tick. A hierarchical wheel is only worth it if the timer-storm benchmark shows the heap losing.
- **Synchronous instances** (blocking): `nanosleep`; `poll(fd)` then retry for sockets (works on non-blocking fds); a direct `read` or `write` for files; futex or condvar parking for `lock`, `send` and `receive` (the waiter lists hold either a task waiter `(executor, frame)` or a thread waiter `(futex)`).
- **Deadlock stop.** If nothing is ready, no timer is armed, no I/O is registered, no remote source is alive, and the root is not done, the program stops with `stop: deadlock: every task waits (main waits on Channel.receive at file:line)`.

### 3.4 Workers and `for parallel` (asynchronous programs)

- **A worker** gets an OS thread with its own executor. A call is a message (copied arguments, plus a completion block shared with an atomic state) on the worker's MPSC queue, and calls run one at a time (A9).
  - `await` or `start` of a call: the caller's frame waits on the completion.
  - A blocking call: the caller's thread parks on a futex.
  - In **off** mode, workers keep today's D31 lowering (calls run on the caller's thread). This is observationally equal and has no threads.
- **Thread-safe handles** (`Mutex`, `Channel`): waiter lists under the existing system lock. Wakes are FIFO hand-offs, made local (ready queue) or remote (MPSC queue plus eventfd).
- **`for parallel`**: unchanged (A3). It runs on the existing pool, joined by the calling thread, whether that thread is the main thread, a worker, or a pool thread.

### 3.5 Memory

- Frames come from `fa_heap_alloc` and are freed when the task is awaited, completes detached, is cancelled or is dropped. Leak checks (`Options::leak_check`) count them like any other block.
- Wait nodes live inside frames. Pool jobs and completion blocks are freed by whichever side releases them last (an atomic state).
- The tests run every async program with leak checks on native, freestanding and wasm.

### 3.6 Anti-starvation scheduler (concrete)

```
tick:
  n = min(ready.len, 61)                      ; EVENT_INTERVAL, as in Tokio
  repeat n: t = ready.pop_front(); turn(t)    ; tasks woken now go to the tail → next tick
  now = monotonic()
  fire expired timers (deadline order)        → ready.push_back
  drain remote queue                          → ready.push_back
  epoll_wait(timeout = ready.empty ? until next timer (or ∞) : 0, max 256 events)
                                              → readers then writers push_back
turn(t): budget = 128; trampoline-resume t
```

- **FIFO everywhere.** A newly accepted connection or a new request queues behind those already ready, so new arrivals cannot keep pushing older work back.
- **The coop budget** works at `await`s only. Each `await` of a primitive that is *already ready* (a readable socket, a channel with an item, a free mutex, an `accept` with a pending connection) takes one unit. At 0, the `await` re-queues the task at the tail and suspends, even though it is ready. This stops `while let x = await ch.receive()` and accept floods from monopolizing the thread.
- **Timers and I/O cannot be starved by ready tasks.** They are checked every 61 turns, and every turn is bounded by the budget unless code runs without awaiting.
- **Long CPU turns.**
  - Dev profiles (`fa run`, `fa test`) time each turn and print `warning: a task kept the thread for 230 ms (handleReport, file:line): await Task.yield() or move the work to a worker` once per call site.
  - Release builds pay nothing.
  - The tools are `Task.yield()`, workers and `WorkerPool`. `for parallel` shortens the pause but does not remove it (A3).

---

## 4. WebAssembly and JavaScript

**wasm: the same coroutine lowering as native, decided.**
- **JSPI** is rejected: it is unavailable on some hosts the wasm target runs on (Bun and JavaScriptCore), and every suspension would cross into JavaScript.
- **Asyncify** is rejected: it needs Binaryen, adds about 50% to code size, and instruments every function.

The executor is the same C code, with TLS as globals and no epoll. The host's event loop drives it:
- Imports: `fa.timer(token, ms)`, `fa.fetch(token, …)`, and later `fa.socket_*`.
- Exports: `fa_wake(token)` (push to the ready queue) and `fa_run()` (one tick batch).
- The `.mjs` loader calls `fa_run` from a `MessageChannel` (web) or `setImmediate` (Node, Bun) macrotask whenever the ready queue is not empty. That macrotask is the coop yield.
- `_start` returns once `main` waits. The loader calls `fa.exit` when the root task and the detached tasks are done.
- No threads and no workers on wasm in this plan.

**JavaScript** (as in the docs' implementation section, with these decisions):
- **Naming.** A function that `suspends` gets `name___async` (a JavaScript `async function`) and/or `name___sync`, each emitted only when requested. A function with one instance keeps its plain name, which keeps the output readable.
  - Exports of a library also get `name`, an alias of the async instance (Promise API), for JavaScript consumers.
  - `.d.ts` files declare both instances.
- **`start`**: `faStart(() => f___async(args))`, which begins in a microtask. The `Task` object holds `{promise, cancelled, cleanups}`. Promises always resolve to a value or an error record, never a rejection.
- **Cancellation**: set the flag, run the cleanups synchronously. The generated code checks the flag after each `await` resumes.
- **Coop budget**: microtasks can starve I/O macrotasks. `faCoop()` in the channel, mutex, `Task.yield` and primitive awaits counts the resolutions that happen without waiting. Every 128 of them, or every 4 ms, it awaits a macrotask (`MessageChannel` or `setImmediate`).
- **Off mode**: only `name___sync`/`name` is emitted. Async-only primitives (timers and network on `web`) make the forced-off program `CONC010`.
- **Workers** (step J2): Web Workers or `worker_threads`, a `MessagePort` per channel, as the docs say.

---

## 5. Standard library async surface (built-ins first, `analyzer/src/builtins`)

| API | Native | wasm | JavaScript |
| --- | --- | --- | --- |
| `Task.yield()`, `Task.all/any/settled`, `task.cancel()` | yes | yes | yes |
| `std.time.sleep(milliseconds)` | sync and async | async | sync (`Atomics.wait`, server runtimes) and async |
| `std.time.monotonic(): Integer` (nanoseconds, for benchmarks) | yes | yes | yes |
| `std.fs.File.read(path)`, `readBytes`, `write(path, text)` | sync and async (pool) | — | sync and async (server runtimes) |
| `std.net.TcpListener { host, port }`: `accept(): TcpStream throws NetError` | sync and async | — | async (server runtimes) |
| `TcpStream.connect(host, port)`, `read(buffer: Array(Byte)): Integer` (`mutates buffer`, 0 means closed), `write(bytes)`, `close()` | sync and async | — | async |
| `Channel.send/receive`, `Mutex.lock` (async variants), worker calls, `WorkerPool` | yes | single thread | yes |
| `std.http` minimal (HTTP/1.1 `Server.listen`, `get`), written in Fa on `std.net` | after B2 | — | async (`fetch`, `Bun.serve`) |

`TcpStream` and `TcpListener` are resources (move-only; `destroy` closes them). Network sockets are created non-blocking and registered with epoll at creation.

---

## 6. Benchmarks vs Tokio and Go

**Layout**:
- `benchmarks/async/<name>/main.fa` and `main.go`.
- `benchmarks/async/rust/` is one Cargo project with `src/bin/<name>.rs`. It has its own empty `[workspace]` table, so it stays outside the root workspace, and uses `tokio = { version = "1", features = ["full"] }` with a release profile of `lto = "fat"`, `codegen-units = 1` and `panic = "abort"`.
- The load generator is `benchmarks/async/load/main.go`.
- The runner is `bun benchmarks/async/run.ts [names] [--runs n]` (`just bench-async`). It reports the median wall time, peak RSS (`resourceUsage().maxRSS`), and the latency percentiles the programs print, and writes `results/async-latest.json`.

**Configurations**:
- Primary, single-thread fairness: Fa vs Tokio `current_thread` vs Go `GOMAXPROCS=1`.
- Reference: Tokio `multi_thread` and Go with default `GOMAXPROCS`.
- Servers are pinned with `taskset -c 0`, and the load generator with `-c 1-5`.

| Benchmark | Program | Metrics |
| --- | --- | --- |
| `spawn` | Start N = 1,000,000 tasks that each `await Task.yield()` once, then `Task.all` | time, RSS, bytes per task |
| `ping-pong` | Two tasks exchanging 10M messages over capacity-1 channels; a cross-thread variant with a worker | ns per round trip |
| `timers` | 1M tasks sleeping a random 0–1000 ms, plus one task measuring its 1 ms tick lateness | total time, RSS, p99 and max lateness |
| `echo` | TCP echo server; the load generator opens 10,000 connections, each sends 64 B and waits for the echo, for 20 s | requests/s, p50, p99, p99.9 |
| `fairness` | Server with `short` (echo) and `long` (20 ms of CPU) requests, in two variants: `long` yields every 1 ms (`Task.yield`, `yield_now`, Go needs nothing), and `long` never yields. 4 long clients, 64 short clients | p99 and p99.9 of short requests; Go wins the no-yield variant (preemption), which is reported as is |
| `starve` | 1000 tasks spinning on always-ready channels, one 1 ms timer task, one accept loop under a connection flood | timer lateness p99; time-to-first-byte of the oldest connections |

**Targets (acceptance)**:
- `spawn` and `ping-pong` match or beat Tokio `current_thread` (no atomics, no reference-counted wakers).
- `echo` is within 10% of Tokio `current_thread` throughput and p99.
- Timer lateness p99 stays under 2 ms in `starve`.
- Bytes per suspended task are at most 1.5x Tokio's.

**Go, installed user-locally** (`scripts/setup-go.sh`, `just setup-go`; go1.27.3 is already in `~/.local/go`):
```bash
version=$(curl -fsSL 'https://go.dev/VERSION?m=text' | head -1)          # e.g. go1.27.3
sum=$(curl -fsSL 'https://go.dev/dl/?mode=json' | jq -r --arg f "$version.linux-amd64.tar.gz" \
      '.[].files[] | select(.filename==$f) | .sha256')
dir=$(mktemp -d); curl -fsSLo "$dir/go.tgz" "https://go.dev/dl/$version.linux-amd64.tar.gz"
echo "$sum  $dir/go.tgz" | sha256sum -c - && rm -rf ~/.local/go && tar -C ~/.local -xzf "$dir/go.tgz"
export PATH="$HOME/.local/go/bin:$PATH" GOTOOLCHAIN=local
```
Tokio: `cargo build --release --manifest-path benchmarks/async/rust/Cargo.toml`. Cargo fetches Tokio from the registry, and its lockfile is committed.

---

## 7. Implementation plan (mergeable steps)

| Step | Area | Content | Acceptance |
| --- | --- | --- | --- |
| **A1** | analyzer | Remove CONC011. Type `await`, `start` and `start do`, `Task(T) throws E` (resource kind), `cancel`, the combinators, `Cancelled` (A11), async lambdas, async contexts. Add `CONC014`, `CONC015` and `CONC016`. Compute the `AsyncSummary` bits and generalize `CONC010` with chains. Add the side tables of §2.2. | Every `fa` block of `concurrency.md` checks. Diagnostic corpus for each code. The interface hash changes when `suspends`/`blocking` change. |
| **A2** | analyzer | Tasks as borrows (REF notes), detached copies and moves (A12), `CONC017`, `CONC018` and `CONC019`, forgetting module-level facts at `await`, requirements across `start`. | The reference-checking and requirements examples. Negative tests from the docs ("`scores` is read by `task`"). |
| **A3** | analyzer, CLI, configuration | `build.asynchronous`, `--asynchronous`, detection (A6), `WCONC001`, `Program::asynchronous`. `fa build -v` prints the reason ("on: main awaits TcpListener.accept"). | Detection table tests (none, `start` only, an awaited non-suspending function, a suspending await, a forced off). |
| **N1** | codegen-llvm (spike) | Async instances with `llvm.coro.*`. Destroy edges reuse the cleanup stack. `fa_resume`/`fa_destroy` IR shims. Coroutine passes in the dev profile. A minimal executor (ready queue, timer heap, `sleep`, `Task.yield`), Task drop and cancel, the trampoline, finished blocks for synchronous instances. | `start`, `await`, `cancel` and scope-drop programs run on native, freestanding and wasm with leak checks. Golden IR of one await. `spawn` and `ping-pong` (single thread) measured against Tokio. |
| **N2** | runtime | epoll registration and readiness, remote MPSC queue plus eventfd, the tick of §3.6, the coop budget, the deadlock stop, the dev-profile watchdog, the blocking pool. Async `lock`, `send` and `receive` with FIFO hand-off. Join nodes and the `Task.all/any/settled` coroutines. | Tests on starvation (budget and timers), cancellation during I/O, and deadlock messages. ASan run of the runtime. |
| **N3** | runtime, codegen-llvm | Workers on their own threads with executors (asynchronous programs), cross-thread completions, `stop()` semantics (A9, A10), `WorkerPool`. | The worker and pool examples of `concurrency.md` on native. A stress test with 1000 calls and random cancels, no leaks. |
| **S1** | std (builtins) | `sleep`, `monotonic`, `std.fs` async, `std.net` TCP, sync and async per target; JavaScript runtime mappings. | Echo server example on native and bun. `CONC010` for `sleep` called blocking on `web`. |
| **J1** | codegen-js | Instances and naming (§4), `faStart`, Task runtime, cancellation, combinators, `faCoop`, off mode. | Differential tests: the same output on bun and native for every async example where A1 doesn't make the order differ (those are flagged). |
| **J2** | codegen-js | Workers as Web Workers or `worker_threads`, channels as `MessagePort`. | The worker examples on bun and node. |
| **W1** | codegen-llvm, loader | wasm executor glue, timer import, `fa_wake`/`fa_run`, exit on completion. | The `sleep`, `start` and channel examples on wasm (Node and Bun). |
| **B1** | benchmarks | `setup-go`, the Tokio project, the runner, then `spawn`, `ping-pong` and `timers`. | Tables in `benchmarks/README.md`. |
| **B2** | benchmarks | Load generator, `echo`, `fairness`, `starve`. | The acceptance targets of §6 met, or gaps recorded with causes. |
| **N5** | codegen-llvm | Bump the pinned LLVM to 20 or 21. Mark awaited calls `coro_elide_safe`. `musttail` symmetric transfer on native. | `spawn` and `echo` allocate one frame per *started* task only (counted by leak stats). |
| **D1** | documentation | Update `concurrency.md` (A1 wording, `Task.yield`, implementation section: LLVM coroutines decided, epoll first, blocking pool, 4-ary heap, coop budget). Update `configuration.md` (`build.asynchronous`), the reference, and `decisions.md`. Update `compilation-with-llvm.md` Q9: io_uring is replaced by epoll first, and the runtime is C bitcode rather than Rust, as plan §2 already decided. | Docs check passes. |

**Order**: A1 → A3 → N1 (the spike checks frame sizes and costs early) → A2 ∥ N2 → S1 → B1 → N3 ∥ J1 → B2 → W1 → J2 → N5. D1 goes in with each step.

**Risks**:
- Frame bloat from large inline locals kept live across awaits. The golden tests track frame sizes.
- LLVM 18's lack of elision. N5 addresses it.
- The A1 ordering difference between JavaScript and native. It is documented, and the differential tests flag it.
- Workers moving to real threads in asynchronous mode only. Covered by the D31 equivalence tests.

### Critical files for implementation
- /home/claude-dev/projects/Fa/packages/analyzer/src/check/concurrency.rs
- /home/claude-dev/projects/Fa/packages/analyzer/src/analysis.rs
- /home/claude-dev/projects/Fa/packages/codegen-llvm/src/lower/parallel.rs (and a new `src/lower/tasks.rs`)
- /home/claude-dev/projects/Fa/packages/codegen-llvm/runtime/fa_runtime.c
- /home/claude-dev/projects/Fa/packages/codegen-js/runtime/concurrency.js
- /home/claude-dev/projects/Fa/documentation/advanced/concurrency.md
