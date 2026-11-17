# Set standard equality: native instruction gate

Before: `5e3b602d`. After: `9b0000e4706cc98a92aa78be89d1c974e96a99a2`.

Callgrind executed instructions at audit sizes, pinned to CPU 5. Every output is checked against Rust by the audit driver. The gate permits at most +1% per benchmark.

Result: **36/36 pass**. [Recorded events and manifests](2026-11-17-set-standard.json).

| Benchmark | Size | Before Ir | After Ir | Change | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| binary-trees | 14 | 114537451 | 114537398 | -0.000046% | pass |
| blur | 1000 | 92419207 | 92419180 | -0.000029% | pass |
| blur-reserved | 1000 | 92419165 | 92419126 | -0.000042% | pass |
| calc | 40000 | 123704280 | 123704244 | -0.000029% | pass |
| fannkuch-redux | 9 | 137427633 | 137427594 | -0.000028% | pass |
| fib | 35 | 236888537 | 236888501 | -0.000015% | pass |
| graph | 10000 | 10450956 | 10450906 | -0.000478% | pass |
| growing-index | 1000000 | 365174770 | 365174745 | -0.000007% | pass |
| growing-index-array | 1000000 | 314168595 | 314168527 | -0.000022% | pass |
| interp | 17 | 18856528 | 18856506 | -0.000117% | pass |
| lru | 200000 | 19154055 | 19154014 | -0.000214% | pass |
| lru-open | 200000 | 20952712 | 20952698 | -0.000067% | pass |
| mandelbrot | 1000 | 440664818 | 440664804 | -0.000003% | pass |
| nbody | 1500000 | 753184335 | 753184313 | -0.000003% | pass |
| particles | 30000 | 105170163 | 105170149 | -0.000013% | pass |
| particles-reserved | 30000 | 105170158 | 105170155 | -0.000003% | pass |
| push | 3200000 | 135550813 | 135550777 | -0.000027% | pass |
| push-array | 3200000 | 135095218 | 135095179 | -0.000029% | pass |
| records | 400000 | 77837530 | 77837494 | -0.000046% | pass |
| respond | 40000 | 235269925 | 235269903 | -0.000009% | pass |
| respond-interpolation | 40000 | 205914507 | 205914518 | +0.000005% | pass |
| scene | 9 | 9336323 | 9336268 | -0.000589% | pass |
| sexp | 2000 | 81334634 | 81334607 | -0.000033% | pass |
| sieve | 10000000 | 203737571 | 203737535 | -0.000018% | pass |
| sort-objects | 100000 | 171201763 | 171201710 | -0.000031% | pass |
| spectral-norm | 1000 | 535731270 | 535731231 | -0.000007% | pass |
| string-building | 300000 | 228872664 | 228872620 | -0.000019% | pass |
| string-building-bytes | 300000 | 175744414 | 175744439 | +0.000014% | pass |
| string-building-growing | 300000 | 187641551 | 187641548 | -0.000002% | pass |
| strlist | 200000 | 85936740 | 85936704 | -0.000042% | pass |
| sum | 3200000 | 161782638 | 161782588 | -0.000031% | pass |
| sum-reserved | 3200000 | 161782593 | 161782554 | -0.000024% | pass |
| tree | 17 | 56687465 | 56687410 | -0.000097% | pass |
| word-count | 500000 | 236118841 | 236118816 | -0.000011% | pass |
| words | 100000 | 28589701 | 28589665 | -0.000126% | pass |
| words-wide | 100000 | 28589651 | 28589626 | -0.000087% | pass |
