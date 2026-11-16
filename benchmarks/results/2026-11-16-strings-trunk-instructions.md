# String redesign: instructions after merging trunk

Generated from the two schema-2 profile JSON files; whole-process Callgrind at the published audit sizes. Trunk compiler: cbe3c5a6; redesign compiler: c8ad4a5c. CPU 5. Source ports differ only by bytes() → toUtf8(). Every execution matches the exact safe-Rust reference output. Ratios are redesign divided by Goose, separately from the trunk regression gate.

| Benchmark | Audit n | Trunk Fa Ir | Redesign Fa Ir | Change | Fa / Goose Ir |
| --- | ---: | ---: | ---: | ---: | ---: |
| binary-trees | 14 | 114,508,021 | 114,508,547 | +0.000% | 0.894 |
| blur | 1,000 | 92,389,836 | 92,390,399 | +0.001% | 1.141 |
| blur-reserved | 1,000 | 92,389,785 | 92,390,401 | +0.001% | 1.141 |
| calc | 40,000 | 125,614,692 | 123,860,738 | -1.396% | 1.131 |
| fannkuch-redux | 9 | 136,668,879 | 137,399,753 | +0.535% | 1.269 |
| fib | 35 | 236,859,176 | 236,860,571 | +0.001% | 0.999 |
| graph | 10,000 | 10,422,105 | 10,422,754 | +0.006% | 1.488 |
| growing-index | 1,000,000 | 418,142,024 | 414,143,436 | -0.956% | 1.307 |
| growing-index-array | 1,000,000 | 336,637,738 | 336,639,232 | +0.000% | 1.063 |
| interp | 17 | 18,826,275 | 18,827,125 | +0.005% | 1.070 |
| lru | 200,000 | 20,212,268 | 20,212,886 | +0.003% | 1.130 |
| lru-open | 200,000 | 20,923,883 | 20,924,528 | +0.003% | 1.170 |
| mandelbrot | 1,000 | 441,135,558 | 441,136,935 | +0.000% | 0.999 |
| nbody | 1,500,000 | 867,186,999 | 867,188,976 | +0.000% | 0.995 |
| particles | 30,000 | 105,141,919 | 105,143,660 | +0.002% | 0.996 |
| particles-reserved | 30,000 | 105,141,895 | 105,143,619 | +0.002% | 0.996 |
| push | 3,200,000 | 155,469,944 | 155,470,844 | +0.001% | 1.280 |
| push-array | 3,200,000 | 135,064,922 | 135,065,825 | +0.001% | 1.112 |
| records | 400,000 | 80,056,786 | 77,807,377 | -2.810% | 1.060 |
| respond | 40,000 | 234,406,913 | 234,802,459 | +0.169% | 1.238 |
| respond-interpolation | 40,000 | 204,689,463 | 206,007,063 | +0.644% | 1.086 |
| scene | 9 | 9,453,835 | 9,455,508 | +0.018% | 1.197 |
| sexp | 2,000 | 81,292,527 | 80,412,188 | -1.083% | 1.037 |
| sieve | 10,000,000 | 203,705,430 | 203,706,965 | +0.001% | 0.816 |
| sort-objects | 100,000 | 176,780,593 | 171,522,232 | -2.975% | 1.437 |
| spectral-norm | 1,000 | 535,733,536 | 535,735,073 | +0.000% | 0.881 |
| string-building | 300,000 | 237,241,957 | 230,046,483 | -3.033% | 1.547 |
| string-building-bytes | 300,000 | 175,115,844 | 174,516,352 | -0.342% | 1.174 |
| string-building-growing | 300,000 | 186,712,752 | 186,713,455 | +0.000% | 1.256 |
| strlist | 200,000 | 86,904,878 | 85,507,127 | -1.608% | 1.055 |
| sum | 3,200,000 | 161,751,876 | 161,752,439 | +0.000% | 1.134 |
| sum-reserved | 3,200,000 | 161,751,822 | 161,752,466 | +0.000% | 1.134 |
| tree | 17 | 56,657,199 | 56,658,008 | +0.001% | 0.998 |
| word-count | 500,000 | 234,988,225 | 235,823,128 | +0.355% | 1.610 |
| words | 100,000 | 28,352,632 | 28,352,528 | -0.000% | 1.265 |
| words-wide | 100,000 | 28,352,651 | 28,352,564 | -0.000% | 1.265 |

All 36 rows meet the +1% gate. Largest increase: respond-interpolation (+0.644%).

Sources: [2026-11-16-strings-main-profiles.json](2026-11-16-strings-main-profiles.json), [2026-11-16-strings-trunk-profiles.json](2026-11-16-strings-trunk-profiles.json). Initial regressing run retained separately in 2026-11-16-strings-trunk-initial-profiles.json.
