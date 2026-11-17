### Controlled CPU series against Goose

Default sizes, 21 rotating/reversing rounds, CPU 5, after all own builds, tests and profilers finished. Fa and its control are the identical executable. Every sample matches the exact Rust reference output. The table keeps ratio of medians and paired median with type-7 Q1/Q3; quartiles are sample dispersion, not confidence bounds. Publication uses --require-control: both control estimates must differ from 1 by strictly less than 3%.

| Benchmark | Default n | CPU Fa / Goose (s) | Ratio of medians | Paired median [Q1, Q3] | Control ratios | Result |
| --- | ---: | --- | ---: | --- | --- | --- |
| respond | 400,000 | 0.399407 / 0.288518 | 1.384 | 1.365 [1.153, 1.447] | 1.036 / 0.980 fail | not resolved |
| words | 500,000 | 0.099272 / 0.084447 | 1.176 | 1.132 [1.005, 1.609] | 1.030 / 1.086 fail | not resolved |
| strlist | 2,000,000 | 0.208135 / 0.189765 | 1.097 | 1.109 [0.963, 1.312] | 0.981 / 1.011 pass | not resolved (threshold) |
| records | 4,000,000 | 0.247784 / 0.231824 | 1.069 | 1.060 [0.951, 1.183] | 0.925 / 0.979 fail | not resolved |
| string-building | 3,000,000 | 0.267841 / 0.173230 | 1.546 | 1.563 [1.355, 1.863] | 1.006 / 1.005 pass | behind |
| calc | 400,000 | 0.274883 / 0.251340 | 1.094 | 1.042 [0.922, 1.351] | 1.125 / 1.054 fail | not resolved |
| sexp | 20,000 | 0.294976 / 0.206128 | 1.431 | 1.476 [1.299, 1.625] | 1.043 / 1.034 fail | not resolved |
| word-count | 5,000,000 | 0.764693 / 0.438573 | 1.744 | 1.903 [1.630, 1.979] | 0.898 / 0.950 fail | not resolved |

Two controls pass. String-building is behind Goose under the recorded rule; strlist straddles 1.10 and stays unresolved. The other six rows fail their control and stay unresolved. All raw samples, including wide quartiles, are retained. This first primary series is preserved; a quiet recheck must be a separately labeled supplemental series. Instruction acceptance is the complete 36-row comparison with trunk above.
