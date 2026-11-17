# String landing instruction gate (2026-11-17)

Whole-process Callgrind from the unchanged audit driver, audit sizes, CPU 5.
Main compiler: `caeec90c282a3862edb7bddbf49088f817f5338a`; merged compiler: `80ea837602c2194e60e1863d8d091604424074ed`.
Both compilers were built in release mode with LLVM 18. Target compilation uses O3, synchronous execution, and overflow checks off.

Every execution matches the exact safe Rust reference output. Goose/Rust sources and reference hashes match across both series. The changed Fa ports differ only by the previously reviewed `bytes()` to `toUtf8()` API migration; algorithms, widths and sizes are unchanged.

| Benchmark | Audit n | Main Fa Ir | Merged Fa Ir | Change |
| --- | ---: | ---: | ---: | ---: |
| lru | 200,000 | 19,155,098 | 19,155,186 | +0.0005% |
| words | 100,000 | 28,383,959 | 28,590,868 | +0.7290% |
| word-count | 500,000 | 236,447,198 | 236,120,049 | -0.1384% |
| calc | 40,000 | 125,780,350 | 123,705,411 | -1.6497% |
| sexp | 2,000 | 81,323,929 | 81,335,765 | +0.0146% |
| strlist | 200,000 | 86,937,058 | 85,937,907 | -1.1493% |
| sort-objects | 100,000 | 176,812,766 | 171,552,925 | -2.9748% |
| records | 400,000 | 80,088,966 | 77,838,697 | -2.8097% |
| respond | 40,000 | 234,855,082 | 235,271,092 | +0.1771% |
| respond-interpolation | 40,000 | 204,718,662 | 205,915,674 | +0.5847% |
| string-building | 300,000 | 236,971,286 | 228,873,854 | -3.4171% |
| string-building-bytes | 300,000 | 175,145,040 | 174,544,971 | -0.3426% |
| string-building-growing | 300,000 | 186,741,924 | 186,742,087 | +0.0001% |
| words-wide | 100,000 | 28,383,981 | 28,590,859 | +0.7289% |

Gate: 14/14 rows at or below +1%.
No CPU-time conclusion is drawn from instruction counts.

Raw profiles: [2026-11-17-strings-landing-tip-main-profiles.json](2026-11-17-strings-landing-tip-main-profiles.json), [2026-11-17-strings-landing-tip-merged-profiles.json](2026-11-17-strings-landing-tip-merged-profiles.json). Artifacts, compiler identities, hashes and profiles are retained in those records; binaries and disassembly remain under `/tmp/strings-land-tip-{main,merged}-audit/`.
This series supersedes the earlier comparison against `71af4fd0`; main is fixed at `caeec90c` for landing. Earlier profiles and their correction history remain committed separately.
