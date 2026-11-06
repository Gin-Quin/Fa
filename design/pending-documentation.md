# Pending documentation notes

The angle-bracket rewrite has arrived. Its pending notes were applied to the rewritten pages on 2026-11-06; their history remains in Git. Add new notes here only when their implementation or specification still needs resolution.

## Metaprogramming evaluation depth

Analyzer review finding 42 is resolved by explicit interpreter continuations. In the metaprogramming page, add the stable default limit of 10 000 nested Fa function calls (`META003`), distinct from the 256 nested type-function applications (`META004`). The same Fa limit applies to defaults and callbacks, independently of native stack size, thread count or address-space limits. The former 3 178-call limit was a proportionally reduced Rust stack allowance and is removed. Execution frames and continuations count toward the existing 256 MB heap budget; preparation/resumption do not add evaluation steps.
