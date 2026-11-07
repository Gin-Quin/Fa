# Pending documentation notes

The angle-bracket rewrite has arrived. Its pending notes were applied to the rewritten pages on 2026-11-06; their history remains in Git. Add new notes here only when their implementation or specification still needs resolution.

## Native root-region proof

When updating the memory-management implementation notes, describe the closed proof in `fa_analyzer::regions` and `design/decisions.md`: whole-module and per-unit compilation require identical facts from every scanned module. Missing or opaque contributions reject the optimization. Child arrays in a recursive union belong to its root region, ordinary arrays own independent roots, and copying a subtree out never shares its owner's lifetime. Generic/unknown payloads, resources/tasks, escaping references, mutations and foreign entry points are conservatively excluded. The backend retains target layout guards; the proof alone does not enable every array-bearing shape. The single shared address band for compressed links is unchanged.
