# Review probes at 6a3ac8fd

These `review-*.fa` files are review evidence, not fixed acceptance tests. No compiler/runtime source was changed.

For any probe, compile each target:

```sh
taskset -c 0-4 target/debug/fa build design/reviews/repros/js-followups/review-optional-start.fa --target=bun --output=/tmp/fa-review-start
# Run /tmp/fa-review-start/bun/main.js with ~/.bun/bin/bun.
# Repeat with --target=node and ~/.local/bin/node.
# For --target=web, run:
taskset -c 0-4 ~/.local/bin/node --experimental-vm-modules --no-warnings packages/cli/tests/programs/web-runner.mjs /tmp/fa-review-start/web/main.js
```

Keep each Fa process below 512 MiB; LLVM stays disabled for this review.

| Probe | Expected | Observed on Bun, Node and web |
| --- | --- | --- |
| `review-optional-start.fa` | `none`, then `Ada`; the declared `Task<String?>` always holds a Task | Builds; `await` of the absent path crashes reading `undefined.state` |
| `review-optional-nullable-result.fa` | Reject `bad` with a diagnostic asking for `?.` on the nullable result | Builds; the correct `good` control prints `none none`, then `bad` crashes on `undefined.trim()` |
| `review-json-comptime.fa` | Each static/runtime pair has the same canonical JSON error | Three differing pairs, including invalid UTF-8 |
| `review-static-empty-tables.fa` | `0 false 0 none`, then `1 0` | Pass; empty structural constants materialize independent mutable tables |
| `review-optional-evaluation.fa` | `none R`, `13 RAB`, `none R`, `11 RFA` | Pass |
| `review-dense-control.fa` | `927`, then `-1 12 99` | Pass; switch `continue` and throwing/returning arms |
| `review-dense-patterns.fa` | `1169 1169`, `10 10 20 999`, `42 43 14 -1` | Pass; alternatives, range fallback, nested matches and compile-time control |
| `review-async-function-value.fa` | An instantiated constraint-method function can be used as a concrete function value (with web blocking restrictions checked) | AN001 generic-instance refusal on every target; retained incomplete metadata family, not an internal-error conversion |
| `review-optional-index.fa` | Records the frontend boundary for a nonnullable collection reached through an optional chain | E-OPT-1 on the indexed continuation; no backend code reached |

The original census repros in this directory belong to the implementation branch and are unchanged.
