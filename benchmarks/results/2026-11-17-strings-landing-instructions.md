# String landing instruction gate (2026-11-17)

Whole-process Callgrind from the unchanged audit driver, audit sizes, CPU 5.
Main compiler: `71af4fd032aa6dc4a4a15809a7440b4be6c46093`; merged compiler: `af99e15968511e48df55f78e13da39ec3fc09ad0`.
Both compilers were built in release mode with LLVM 18. Target compilation uses O3, synchronous execution, and overflow checks off.

Every execution matches the exact safe Rust reference output. Goose/Rust sources and reference hashes match across both series. The eight changed Fa ports differ only by the previously reviewed `bytes()` to `toUtf8()` API migration; algorithms, widths and sizes are unchanged.

| Benchmark | Audit n | Main Fa Ir | Merged Fa Ir | Change |
| --- | ---: | ---: | ---: | ---: |
| lru | 200,000 | 19,152,649 | 19,152,737 | +0.0005% |
| words | 100,000 | 28,381,507 | 28,505,822 | +0.4380% |
| word-count | 500,000 | 235,016,552 | 236,618,109 | +0.6815% |
| calc | 40,000 | 125,642,420 | 123,681,261 | -1.5609% |
| sexp | 2,000 | 81,321,480 | 81,334,912 | +0.0165% |
| strlist | 200,000 | 86,934,537 | 85,935,391 | -1.1493% |
| records | 400,000 | 80,086,445 | 77,836,181 | -2.8098% |
| respond | 40,000 | 234,433,621 | 234,668,576 | +0.1002% |
| respond-interpolation | 40,000 | 204,716,109 | 205,913,149 | +0.5847% |
| string-building | 300,000 | 237,269,368 | 230,071,900 | -3.0335% |
| string-building-bytes | 300,000 | 175,142,522 | 174,542,455 | -0.3426% |
| string-building-growing | 300,000 | 186,739,406 | 186,739,600 | +0.0001% |
| words-wide | 100,000 | 28,381,462 | 28,505,813 | +0.4381% |

Gate: 13/13 rows at or below +1%.
No CPU-time conclusion is drawn from instruction counts.

Raw profiles: [2026-11-17-strings-landing-main-profiles.json](2026-11-17-strings-landing-main-profiles.json), [2026-11-17-strings-landing-merged-profiles.json](2026-11-17-strings-landing-merged-profiles.json). Artifacts, compiler identities, hashes and profiles are retained in those records; binaries and disassembly remain under `/tmp/strings-land-{main,accept}-audit/`.
The rejected initial profile is retained separately in `2026-11-17-strings-landing-initial-profiles.json`, with its binaries and disassembly under `/tmp/strings-land-merged-audit/`.
The canonical Unicode correctness correction at `27ca5a6d` is separately retained in `2026-11-17-strings-landing-canonical-initial-profiles.json`; its words result exceeded the ceiling. The instruction-only snapshot at `3cc4b925` (retained in commit `ebb59b33`) was superseded after the Unicode assertion failure.
