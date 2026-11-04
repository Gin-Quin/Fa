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

Measured on 2026-11-04 with the scheduler that serves woken tasks before yielded ones (`design/decisions.md`, "Async scheduler: wakes before yields"), on a 6-core VPS (Linux 6.8) shared with other jobs (load average 6–7 during the runs). Each configuration ran once; expect noise of ±20% on latencies (an earlier `echo` run of the same build gave Fa 33k and Tokio 38k requests/s).

### B1 (best of 3, pinned on core 0)

| benchmark | Fa | Tokio current_thread | Go GOMAXPROCS=1 |
| --- | ---: | ---: | ---: |
| `spawn` 1M tasks | 0.08 s, 94.9 MB | 0.61 s, 384.0 MB | 1.80 s, 209.4 MB |
| `ping-pong` 10M messages | 0.46 s, 3.3 MB | 1.28 s, 2.4 MB | 1.90 s, 1.9 MB |

### timers (n = 1,000,000)

| config | total ms | lateness p99 µs | lateness max µs | peak MB |
| --- | ---: | ---: | ---: | ---: |
| fa | 2033 | 313780 | 313780 | 223.2 |
| tokio current | 2522 | 379514 | 379514 | 384.1 |
| go GOMAXPROCS=1 | 7883 | 98938 | 434781 | 379.5 |
| tokio multi | 2656 | 60143 | 317439 | 376.6 |
| go default | 6109 | 46018 | 252893 | 487.5 |

### echo (10,000 connections, 64 B, 10 s)

| config | requests/s | p50 µs | p99 µs | p99.9 µs | errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| fa | 61043 | 151437 | 369945 | 380417 | 0 |
| tokio current | 61145 | 150171 | 357943 | 398156 | 0 |
| go GOMAXPROCS=1 | 52542 | 184421 | 285712 | 299389 | 0 |

### fairness (64 short clients, 4 long, 10 s)

| config | long | short p50 µs | short p99 µs | short p99.9 µs | short requests | long requests | errors |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| fa | yield | 7605 | 42883 | 109453 | 67541 | 1241 | 0 |
| tokio current | yield | 4802 | 17573 | 47682 | 115330 | 1428 | 0 |
| go GOMAXPROCS=1 | yield | 10567 | 31139 | 87229 | 57055 | 1221 | 0 |
| tokio multi | yield | 2899 | 10330 | 28587 | 208448 | 1622 | 0 |
| go default | yield | 5012 | 21137 | 52002 | 105409 | 1465 | 0 |
| fa | spin | 81324 | 107028 | 129035 | 8104 | 484 | 0 |
| tokio current | spin | 81366 | 118783 | 154602 | 8125 | 481 | 0 |
| go GOMAXPROCS=1 | spin | 40880 | 81261 | 84264 | 15608 | 501 | 0 |
| tokio multi | spin | 5827 | 24118 | 54358 | 89202 | 1535 | 0 |
| go default | spin | 4444 | 28117 | 65838 | 98664 | 1519 | 0 |
| fa | auto | 12228 | 69779 | 110950 | 46417 | 1110 | 0 |

### starve (3,000 ticks, a flood of 5,000 connections)

| config | lateness p50 µs | lateness p99 µs | lateness max µs | connections | ttfb p50 µs | ttfb p99 µs | ttfb max µs | old p99 µs | unanswered | errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fa | 4763 | 18673 | 115007 | 274385 | 245841 | 1363194 | 4429731 | 1473125 | 3335 | 1105 |
| tokio current | 3984 | 14005 | 73872 | 193403 | 335835 | 1477389 | 8511572 | 1439375 | 374 | 4097 |
| go GOMAXPROCS=1 | 19209 | 39432 | 73130 | 1299 | 41132410 | 61747500 | 61748199 | 20522592 | 1579 | 3437 |

With the previous single FIFO queue, the same machine gave Fa a `starve` lateness p99 of 25 ms (17–19 ms now). The `starve` errors are mostly the connections still pending when the server exits, which then get reset. The second `echo` and `starve` runs only measured the three single-thread configurations.

## Acceptance targets (design/async.md §6)

| Target | Result |
| --- | --- |
| `spawn` and `ping-pong` match or beat Tokio current_thread | **Met**: 7.6× and 2.8× faster. |
| `echo` within 10% of Tokio current_thread in throughput and p99 | **Met**: −0.2% throughput, p99 +3%. |
| Timer lateness p99 under 2 ms in `starve` | **Not met**: 17–19 ms (Tokio 14 ms, Go 39 ms). Woken tasks now run first in the next tick, but timers only fire *between* ticks, and a tick of 61 turns, each of which spends a full coop budget on channel operations, lasts about 5 ms (the p50 lateness). Reaching 2 ms needs the timers checked inside a tick (a clock read every few turns, about 25 ns), or shorter ticks while timers are pending. |
| Bytes per suspended task at most 1.5× Tokio's | **Met**: 0.58× (`timers`: 223 MB against 384 MB for 1M sleeping tasks). |

Other observations:
- **`fairness` with `yield`.** The short requests wait about 2× longer than with Tokio. Their wakes now come first, but the long requests' turns (1 ms each) are what they wait behind.
- **`fairness` with `spin`.** Fa and Tokio pay the full 20 ms per long request. Go, which preempts, does better, as §6 expected.
- **`fairness` with `auto`.** The A2 back-edge yields bring Fa between the two variants with no change to the code.
- **JavaScript.** The wakes-before-yields order already holds on Bun and Node: timers and I/O run in their own phases of the host's loop before `setImmediate`, which carries the coop yields.
