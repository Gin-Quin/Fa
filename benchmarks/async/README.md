# Async benchmarks

Fa's native event loop (`design/async.md` §3) against Tokio and Go, on one thread and with their multithreaded runtimes as a reference (`design/async.md` §6, steps B1 and B2).

| Benchmark | What it does | Measures |
| --- | --- | --- |
| `spawn` | starts 1,000,000 tasks that each yield once, then joins them | time, peak memory |
| `ping-pong` | two tasks exchange 10,000,000 messages over channels of capacity 1 | time |
| `timers` | 1,000,000 tasks sleep 0–1000 ms (task `i` sleeps `(i * 7919) %% 1001` ms) while one task ticks every 1 ms | total time, lateness of the ticks, peak memory |
| `echo` | a TCP echo server; the load generator opens 10,000 connections, each sends 64 bytes and waits for the echo, for 10 s | requests/s, latency percentiles |
| `fairness` | an echo server where 4 clients send long requests (20 ms of CPU) and 64 clients send short ones | latency of the short requests |
| `starve` | 1,000 tasks spin on always-ready channels, one task sleeps 1 ms 3,000 times, an accept loop answers a flood of connections | lateness of the ticks, time to first byte |

`fairness` has three variants for its long requests:
- `yield`: the request yields every 1 ms (`await Task.yield()`, Tokio's `yield_now`, Go's `runtime.Gosched`).
- `spin`: it never yields. In Fa the loop is in a function that isn't `async`, whose synchronous instance never yields.
- `auto` (Fa only): the loop is in the async function. Its back-edges spend the coop budget and yield once the turn has lasted 2 ms (A2).

## Layout

- `fa/<name>.fa`: the Fa programs.
- `go/` (`spawn` at the root, `go/<name>/` for the others) and `load/` (the load generator): the Go programs.
- `src/main.rs` (`spawn`) and `src/bin/<name>.rs`: the Tokio programs. Their Cargo project has its own empty `[workspace]`.

## Running

```bash
benchmarks/async/run.sh [spawn] [ping-pong]                         # B1: Fa, Tokio current_thread, Go GOMAXPROCS=1
FA_BIN_DIR=benchmarks/async/build benchmarks/async/run-b2.sh [timers] [echo] [fairness] [starve]
```

- **Building.** `run-b2.sh` builds the Go and Tokio sides. The Fa servers are built first with `cargo run -p fa-codegen-llvm --example build -- benchmarks/async/fa/<name>.fa --target=native --output=benchmarks/async/build`.
- **Pinning.** Fa, Tokio `current_thread` and Go `GOMAXPROCS=1` run pinned on core 0. Tokio `multi_thread` and Go with the default `GOMAXPROCS` run on cores 0–3. The load generator runs on cores 4–5.
- **Memory cap.** Every process except Go runs under `ulimit -v 8000000`.
- **Environment.** `DURATION` (default `10s`), `CONNECTIONS` (default 10000), `FLOOD_CONNECTIONS`, `TICKS` and `TIMERS_N` change the sizes.

## Results

Measured on 2026-11-04 on a 6-core VPS (Linux 6.8) shared with other jobs (load average 5–7 during the runs), each configuration run once. Expect noise of ±20% on latencies.

### B1 (best of 3, pinned on core 0)

| benchmark | Fa | Tokio current_thread | Go GOMAXPROCS=1 |
| --- | ---: | ---: | ---: |
| `spawn` 1M tasks | 0.14 s, 93.5 MB | 0.69 s, 384.0 MB | 2.11 s, 187.9 MB |
| `ping-pong` 10M messages | 0.57 s, 1.8 MB | 1.80 s, 2.4 MB | 2.43 s, 1.9 MB |

### timers (n = 1,000,000)

| config | total ms | lateness p99 µs | lateness max µs | peak MB |
| --- | ---: | ---: | ---: | ---: |
| fa | 1494 | 100837 | 209604 | 223.1 |
| tokio current | 1715 | 91942 | 174777 | 384.1 |
| go GOMAXPROCS=1 | 6668 | 80536 | 396019 | 450.9 |
| tokio multi | 2672 | 186003 | 368660 | 376.5 |
| go default | 5383 | 37097 | 370656 | 580.7 |

### echo (10,000 connections, 64 B, 10 s)

| config | requests/s | p50 µs | p99 µs | p99.9 µs | errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| fa | 41334 | 229536 | 360863 | 376236 | 0 |
| tokio current | 42142 | 232643 | 447239 | 453908 | 0 |
| go GOMAXPROCS=1 | 44086 | 217869 | 344310 | 352641 | 0 |
| tokio multi | 43431 | 203008 | 670840 | 850990 | 0 |
| go default | 45486 | 206918 | 401525 | 423610 | 0 |

### fairness (64 short clients, 4 long, 10 s)

| config | long | short p50 µs | short p99 µs | short p99.9 µs | short requests | long requests | errors |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| fa | yield | 7135 | 33930 | 92444 | 76618 | 1335 | 0 |
| tokio current | yield | 4696 | 21063 | 51262 | 114432 | 1423 | 0 |
| go GOMAXPROCS=1 | yield | 10446 | 28124 | 70045 | 57651 | 1242 | 0 |
| tokio multi | yield | 2997 | 19593 | 69306 | 174135 | 1452 | 0 |
| go default | yield | 4586 | 30221 | 118716 | 104396 | 1426 | 0 |
| fa | spin | 81495 | 126117 | 147669 | 8058 | 473 | 0 |
| tokio current | spin | 81314 | 99599 | 103140 | 8275 | 486 | 0 |
| go GOMAXPROCS=1 | spin | 40887 | 83386 | 130223 | 15217 | 492 | 0 |
| tokio multi | spin | 6455 | 28533 | 87616 | 81998 | 1412 | 0 |
| go default | spin | 5408 | 33948 | 62065 | 88900 | 1398 | 0 |
| fa | auto | 13479 | 56707 | 135967 | 41477 | 1037 | 0 |

### starve (3,000 ticks, a flood of 5,000 connections)

| config | lateness p50 µs | lateness p99 µs | lateness max µs | connections | ttfb p50 µs | ttfb p99 µs | ttfb max µs | old p99 µs | unanswered | errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fa | 5077 | 25411 | 136098 | 197317 | 111604 | 742512 | 1577541 | 417071 | 407 | 4404 |
| tokio current | 4841 | 18010 | 90957 | 191546 | 400314 | 1576462 | 7639352 | 451881 | 1698 | 2338 |
| go GOMAXPROCS=1 | 19205 | 39588 | 89982 | 1326 | 41221042 | 61769362 | 61779149 | 20650233 | 903 | 4174 |
| tokio multi | 1050 | 15356 | 214711 | 80683 | 317852 | 1382946 | 1839607 | 253953 | 2078 | 2219 |
| go default | 19251 | 50596 | 209129 | 4294 | 33320810 | 64876849 | 64903717 | 11369150 | 2499 | 2480 |

The `starve` errors are mostly the connections still pending when the server exits, which then get reset.

## Acceptance targets (design/async.md §6)

| Target | Result |
| --- | --- |
| `spawn` and `ping-pong` match or beat Tokio current_thread | **Met**: 4.9× and 3.2× faster, with less memory. |
| `echo` within 10% of Tokio current_thread in throughput and p99 | **Met**: −2% throughput, and a p99 19% lower. With 10,000 connections on two load-generator cores, every configuration is within 10% of the others: the load generator is the limit. |
| Timer lateness p99 under 2 ms in `starve` | **Not met**: 25 ms (Tokio 18 ms, Go 40 ms). A woken timer task joins the tail of the FIFO run queue (A16), behind the 1,000 always-ready spinners, each of which keeps the thread for its whole coop budget (128 channel operations). Reaching 2 ms needs timer wakes ahead of the queue (a priority lane) or a smaller budget: a design decision, not a bug. Noise from the shared machine adds to it. |
| Bytes per suspended task at most 1.5× Tokio's | **Met**: 0.58× (`timers`: 223 MB against 384 MB for 1M sleeping tasks; `spawn`: 93.5 MB against 384 MB). |

Other observations:
- **`fairness` with `yield`.** The short requests wait about 1.6× longer than with Tokio: the descriptors are polled after a tick in which a task yielded, as Tokio defers `yield_now`, but a tick still runs every ready task first.
- **`fairness` with `spin`.** Fa and Tokio pay the full 20 ms per long request. Go, which preempts, does better, as §6 expected.
- **`fairness` with `auto`.** The A2 back-edge yields bring Fa halfway between the two variants with no change to the code.
- **`starve` connections.** Fa answers the oldest connections of the flood as fast as Tokio and much faster than Go: the accept loop runs every 61 turns and isn't starved.
