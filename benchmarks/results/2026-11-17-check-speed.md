# Checker comparison

Generated from the accompanying JSON; seconds and MiB. Default parallelism on CPUs 0–4 is the wall-time goal; single worker on CPU 5 measures CPU cost. Shared machine, warm filesystem cache, fresh checker processes.

| Files | Mode | Observed load L1 | Fa wall Q1 / med / Q3 | Bun wall Q1 / med / Q3 | Fa/Bun wall medians / paired [Q1,Q3] | Wall goal | CPU ratio medians / paired | Control wall / CPU | Fa / Bun peak MiB (max) |
|---:|---|---:|---:|---:|---:|---|---:|---|---:|
| 1 | default | 30.86–33.54 | 0.1635 / 0.3277 / 0.4806 | 0.2729 / 0.3946 / 0.6056 | 0.830 / 0.778 [0.302,1.545] | provisional: met | 0.571 / 0.512 | 0.517,0.463 fail / 0.846,0.805 fail | 28.1 / 50.6 |
| 1 | single | 31.07–31.48 | 0.1373 / 0.4362 / 0.9161 | 0.1926 / 0.2596 / 0.2685 | 1.680 / 2.187 [0.789,4.001] | provisional: reference only | 0.845 / 0.927 | 1.653,1.080 fail / 1.253,2.210 fail | 26.7 / 43.2 |
| 100 | default | 33.54–33.72 | 1.7865 / 3.0324 / 3.7192 | 0.9596 / 1.4567 / 1.6288 | 2.082 / 2.082 [2.002,2.874] | provisional: behind | 1.887 / 1.905 | 0.703,0.932 fail / 0.965,0.957 fail | 167.1 / 90.8 |
| 100 | single | 31.48–33.62 | 3.9227 / 4.2802 / 4.8946 | 1.3708 / 1.6374 / 2.2733 | 2.614 / 2.946 [1.743,3.664] | provisional: reference only | 2.006 / 2.245 | 0.906,0.886 fail / 0.973,0.919 fail | 79.6 / 78.8 |
| 1000 | default | 15.66–33.72 | 7.9471 / 9.5071 / 11.1373 | 4.0731 / 5.8525 / 6.2266 | 1.624 / 1.890 [1.618,1.978] | provisional: behind | 2.119 / 2.211 | 1.056,1.090 fail / 1.018,1.009 pass | 283.9 / 428.5 |
| 1000 | single | 33.62–37.93 | 29.2513 / 42.3405 / 75.4858 | 11.6361 / 13.7800 / 27.0447 | 3.073 / 2.393 [2.237,3.347] | provisional: reference only | 2.600 / 2.534 | 0.885,1.096 fail / 1.003,1.054 fail | 213.3 / 404.6 |
| 2000 | default | 18.14–34.34 | 23.4984 / 24.9962 / 28.4075 | 11.2126 / 13.5306 / 14.4715 | 1.847 / 2.061 [1.801,2.134] | provisional: behind | 2.327 / 2.354 | 0.943,0.948 fail / 0.992,0.942 fail | 405.4 / 812.4 |
| 2000 | single | 17.68–37.93 | 107.3995 / 114.7886 / 151.8033 | 43.0849 / 50.6422 / 58.8158 | 2.267 / 2.457 [2.095,2.930] | provisional: reference only | 2.635 / 2.604 | 1.199,1.060 fail / 0.990,1.014 pass | 263.8 / 770.1 |

| Files | Mode | Tool | Observed load L1 | User Q1 / med / Q3 | System Q1 / med / Q3 | CPU Q1 / med / Q3 | CPU/wall (median ratio) | RSS MiB Q1 / med / Q3 |
|---:|---|---|---:|---:|---:|---:|---:|---:|
| 1 | default | fa | 30.86–33.54 | 0.0118 / 0.0141 / 0.0163 | 0.0284 / 0.0431 / 0.0748 | 0.0413 / 0.0551 / 0.0912 | 0.17 | 11.3125 / 12.4180 / 16.6680 |
| 1 | default | bun | 30.86–33.54 | 0.0433 / 0.0478 / 0.0508 | 0.0462 / 0.0488 / 0.0590 | 0.0908 / 0.0964 / 0.1079 | 0.24 | 47.7207 / 48.2461 / 49.1250 |
| 1 | single | fa | 31.07–31.48 | 0.0119 / 0.0143 / 0.0158 | 0.0449 / 0.0799 / 0.0984 | 0.0586 / 0.0912 / 0.1246 | 0.21 | 26.4023 / 26.4062 / 26.5312 |
| 1 | single | bun | 31.07–31.48 | 0.0432 / 0.0613 / 0.0637 | 0.0408 / 0.0452 / 0.0587 | 0.0828 / 0.1079 / 0.1218 | 0.42 | 41.6191 / 42.2617 / 42.7656 |
| 100 | default | fa | 33.54–33.72 | 1.3777 / 1.4560 / 1.5337 | 0.0870 / 0.1430 / 0.6118 | 1.4399 / 1.6326 / 2.1961 | 0.54 | 104.4375 / 150.9883 / 153.0664 |
| 100 | default | bun | 33.54–33.72 | 0.6906 / 0.7183 / 0.7723 | 0.1179 / 0.1379 / 0.2005 | 0.8186 / 0.8651 / 0.9552 | 0.59 | 88.6875 / 89.0664 / 90.0977 |
| 100 | single | fa | 31.48–33.62 | 1.4255 / 1.5629 / 1.5714 | 0.0917 / 0.3120 / 0.8573 | 1.6376 / 1.7179 / 2.4264 | 0.40 | 56.3438 / 61.4570 / 68.5684 |
| 100 | single | bun | 31.48–33.62 | 0.6334 / 0.7457 / 0.7742 | 0.1101 / 0.1218 / 0.1392 | 0.7550 / 0.8564 / 0.9304 | 0.52 | 75.7227 / 77.6172 / 78.6504 |
| 1000 | default | fa | 15.66–33.72 | 14.6864 / 14.9588 / 15.3058 | 0.7625 / 1.0796 / 1.9005 | 15.6435 / 15.8877 / 16.5552 | 1.67 | 245.1113 / 273.2344 / 276.5801 |
| 1000 | default | bun | 15.66–33.72 | 5.9937 / 6.3073 / 6.8838 | 0.8561 / 1.1952 / 1.4396 | 6.9367 / 7.4989 / 8.2106 | 1.28 | 423.0391 / 423.9492 / 424.5625 |
| 1000 | single | fa | 33.62–37.93 | 15.9759 / 16.6670 / 18.2694 | 1.1012 / 1.8601 / 2.3680 | 17.0215 / 18.6141 / 20.4141 | 0.44 | 134.6641 / 150.9922 / 191.1094 |
| 1000 | single | bun | 33.62–37.93 | 6.1008 / 6.3487 / 6.8390 | 0.7812 / 0.8591 / 0.9054 | 6.9389 / 7.1588 / 7.6821 | 0.52 | 401.0508 / 402.0352 / 402.5000 |
| 2000 | default | fa | 18.14–34.34 | 31.2770 / 32.4226 / 33.2212 | 0.8762 / 1.7694 / 2.7550 | 32.3553 / 33.6433 / 35.9761 | 1.35 | 339.6016 / 359.1328 / 377.3984 |
| 2000 | default | bun | 18.14–34.34 | 12.4026 / 12.7873 / 13.0838 | 1.5650 / 1.7151 / 2.1281 | 13.9782 / 14.4549 / 15.2118 | 1.07 | 807.8652 / 808.5664 / 810.2363 |
| 2000 | single | fa | 17.68–37.93 | 37.0004 / 38.4103 / 39.1069 | 2.5315 / 2.9753 / 3.9535 | 39.1196 / 41.3856 / 43.2273 | 0.36 | 238.3203 / 240.2539 / 247.0938 |
| 2000 | single | bun | 17.68–37.93 | 13.9761 / 14.1565 / 14.3103 | 1.4426 / 1.5715 / 1.6356 | 15.4986 / 15.7066 / 15.9559 | 0.31 | 767.5293 / 768.0391 / 768.3223 |

All timing figures with observed one-minute load >=6 are provisional. Load columns show min/max of captured observations; old rows have endpoint readings, newer samples also retain start/end load. No historical per-sample load is invented.

Small initial CPU-5 rows use separately labeled eleven-run GNU-time RSS supplements; their primary CPU/wall samples remain unchanged.

| Library files | Fa source files / lines / bytes | TS source files / lines / bytes |
|---:|---:|---:|
| 1 | 1 / 492 / 18658 | 1 / 486 / 19463 |
| 100 | 105 / 50586 / 1937170 | 104 / 50488 / 2019797 |
| 1000 | 1005 / 504979 / 19345000 | 1004 / 504189 / 20175774 |
| 2000 | 2005 / 1010349 / 38708359 | 2004 / 1008781 / 40369169 |

## Largest-size profile

Exclusive user-space instruction attribution; inlining charges helpers to their surviving caller. These are not CPU/wall milliseconds; kernel time is outside Callgrind.

| Phase/helper category | Instructions | % of whole process |
|---|---:|---:|
| types_checking | 15,996,733,645 | 23.82 |
| runtime_other | 11,787,913,591 | 17.55 |
| syntax_decode | 9,900,426,848 | 14.74 |
| reading_parsing | 7,788,076,238 | 11.60 |
| pipeline_session | 6,646,997,131 | 9.90 |
| scanning_resolving | 5,995,627,897 | 8.93 |
| flow_effects | 4,870,125,799 | 7.25 |
| references_moves_relations | 4,111,951,858 | 6.12 |
| compile_time | 56,079,470 | 0.08 |
| rendering | 5,304,789 | 0.01 |

| Function (exclusive) | Instructions | % of process |
|---|---:|---:|
| `???:fa_analyzer::syntax::view [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 6,566,154,503 | 9.78 |
| `???:fa_parser::tokenize::tokenize_range [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 3,497,722,547 | 5.21 |
| `???:<fa_analyzer::check::Walker>::publish_interface [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,864,220,483 | 2.78 |
| `???:<hashbrown::map::HashMap<&str, (), rustc_hash::FxBuildHasher>>::insert [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,681,197,817 | 2.50 |
| `./string/../sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S:__memcpy_avx_unaligned_erms [/usr/lib/x86_64-linux-gnu/libc.so.6]` | 1,678,847,728 | 2.50 |
| `???:mi_free [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,145,115,273 | 1.71 |
| `???:<fa_analyzer::check::Walker>::record_value_origins [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,078,225,237 | 1.61 |
| `???:mi_theap_malloc_aligned [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,065,323,008 | 1.59 |
| `???:fa_analyzer::scan::scan [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 1,031,943,217 | 1.54 |
| `./string/../sysdeps/x86_64/multiarch/memcmp-avx2-movbe.S:__memcmp_avx2_movbe [/usr/lib/x86_64-linux-gnu/libc.so.6]` | 952,400,008 | 1.42 |
| `???:fa_parser::parsing::parse_expression::parse_expression_inner'2 [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 848,256,261 | 1.26 |
| `???:<fa_analyzer::types::layer::Syms>::intern [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 808,928,916 | 1.20 |
| `???:<fa_parser::nodes::Node>::for_each_child::<fa_analyzer::syntax::children::{closure#0}> [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 681,790,693 | 1.02 |
| `???:fa_analyzer::syntax::id [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 667,540,390 | 0.99 |
| `???:_mi_page_malloc_zero [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 621,245,949 | 0.93 |
| `???:fa_parser::parsing::parse_expression_right::parse_expression_right'2 [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 595,817,004 | 0.89 |
| `???:<fa_parser::typed_syntax_tree::TypedSyntaxTree>::name [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 589,757,671 | 0.88 |
| `???:<fa_analyzer::types::store::TypeStore>::as_int [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 537,919,042 | 0.80 |
| `???:<hashbrown::raw::RawTable<(fa_analyzer::ids::Sym, ())>>::reserve_rehash::<hashbrown::map::make_hasher<fa_analyzer::ids::Sym, (), rustc_hash::FxBuildHasher>::{closure#0}> [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 537,547,546 | 0.80 |
| `???:<fa_parser::source::SourceMap>::new [/home/claude-dev/.paseo/worktrees/0rhyrubi/check-speed/target/release/fa]` | 514,476,257 | 0.77 |

### Diagnostic wall phases: single (load not captured; provisional when busy or not captured)

```text
timings: fa check
  phase scan                 9.0 ms
  phase configuration        0.2 ms
  phase pipeline        114145.2 ms
  phase parse             2384.7 ms
  phase analysis        110192.7 ms
  phase generation           5.5 ms
  phase render               0.0 ms
  phase total           114645.8 ms
  units 2002 (strongly connected components of 2004 files; programs analyzed: 1)
  largest unit library/ringA (3 files)
  types 92347 (global store: what the interfaces reach)
  trees streamed: dropped with their unit, 1710 parsed again (tree budget); peak units in flight 1
  workers 1: busy 100% (setup 23%, analysis 73%, merge 3%, sink 0%, teardown 1%), idle 0%
    in units: on a core 33%, waiting for a core 50%, blocked 17% (locks, evaluation threads, page faults)
  peak rss 262.5 MB
```

### Diagnostic wall phases: default (load not captured; provisional when busy or not captured)

```text
timings: fa check
  phase scan                37.8 ms
  phase configuration        0.2 ms
  phase pipeline         46925.9 ms
  phase parse             6513.5 ms
  phase analysis         39052.8 ms
  phase generation          14.4 ms
  phase render               0.0 ms
  phase total            54220.3 ms
  units 2002 (strongly connected components of 2004 files; programs analyzed: 1)
  largest unit library/ringA (3 files)
  types 92347 (global store: what the interfaces reach)
  trees streamed: dropped with their unit, 1713 parsed again (tree budget); peak units in flight 5
  workers 5: busy 99% (setup 15%, analysis 79%, merge 5%, sink 0%, teardown 1%), idle 1%
    in units: on a core 20%, waiting for a core 72%, blocked 8% (locks, evaluation threads, page faults)
  peak rss 330.9 MB
```

## Original target-dependent generator

One correctness/memory probe, 88 target readers, 2005 files, 1010789 lines: 279.21 MiB peak. Per-probe load was not captured; archived phase times are provisional. This is not an acceptance timing series.

```text
timings: 88 modules read the target (first: library/m0002)
timings: fa check
  phase scan               260.5 ms (×2)
  phase configuration        0.4 ms (×2)
  phase pipeline         62681.0 ms (×2)
  phase parse             5945.7 ms (×2)
  phase analysis         54683.8 ms (×2)
  phase generation           8.9 ms (×2)
  phase render               0.0 ms
  phase total            64211.4 ms
  units 2002 (strongly connected components of 2004 files; programs analyzed: 2)
  largest unit library/ringA (3 files)
  types 92347 (global store: what the interfaces reach)
  trees streamed: dropped with their unit, 1712 parsed again (tree budget); peak units in flight 5
  workers 5: busy 99% (setup 8%, analysis 84%, merge 7%, sink 0%, teardown 1%), idle 1%
    in units: on a core 16%, waiting for a core 79%, blocked 5% (locks, evaluation threads, page faults)
  workers 5: busy 93% (setup 13%, analysis 73%, merge 4%, sink 0%, teardown 1%), idle 7%
    in units: on a core 22%, waiting for a core 74%, blocked 4% (locks, evaluation threads, page faults)
  peak rss 292.8 MB
```

## Repository corpus

Observed load L1 25.62–33.46; timings provisional when >=6. 2553 tracked .fa files, fresh process per file in its repository context: 484.819 s aggregate wall, 9.606 s user + 74.680 s system = 84.286 s CPU; peak 74.87 MiB. Exit counts: {'0': 1422, '1': 1131}; abnormal/timeout/memory failures: 0. Expected diagnostic fixtures keep exit 1.
