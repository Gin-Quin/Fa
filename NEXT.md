# Compiler work in progress (2026-11-06)

Work only in `.claude/worktrees/lead`, branch `compiler`; the main checkout is shared and contains uncommitted work. The user requested completion of the whole remaining compiler list and explicitly authorized parallel agents.

## Integrated and validated branches

- `f2f43d6`: initial proofs branch `a3d64ac` (async native proofs, deferred/reanalyzed JavaScript units). Its complete workspace suite, LLVM included, clippy and 296 JS runtime tests passed; the merge also passed clippy and reanalysis/JS regressions.
- `ca9f85a`: initial native-performance branch `04bb84b` (compressed arena links, packed literal choices), including a repaired C sanitizer test. Its complete workspace suite, clippy, 296 JS runtime tests and a real ASan/UBSan run passed; the merge passed clippy and compressed-link regressions on all native targets.
- Nine clean, unlocked, already-integrated worktrees were removed. Locked trees still belong to a live process; a dirty integrated tree and unmerged branches were preserved.

## Current work and ownership

- Lead: the user confirmed that the uncommitted documentation in the main checkout is the angle-bracket rewrite. Its documentation/README changes were copied with a three-way merge, preserving newer compiler notes. Most pending documentation notes are applied. Stage 2 emits GN031 for old declarations/applications, preserves recovery for `fa migrate-generics`, and ignores only GN031 in migration verification. Parser/analyzer tests, cases, documentation parsing, pipeline determinism, website check/snippets/build passed before subsequent changes.
- Lead: Color.members.length and nested enumeration members fixed and tested on native/wasm; Fields is/is not implemented for exact sets, alternatives, optional narrowing and compile-time evaluation, tested on Bun/native/freestanding/wasm.
- Lead: Bun Pointer.allocate/read/write/free/value implemented for known types, C-compatible blocks and opaque Fa values, copy/drop semantics, C struct/fixed-array views; C-backed scalar places and references are in their final targeted run. Unresolved generic pointer layouts are still unsupported and need a follow-up. Native C-compatible pointer.value places were enabled only when C and Fa layouts agree. Common pointer fixture moved from LLVM into CLI for both harnesses.
- Agent `/root/analyzer`: initial branch worktree `agent-a2f066f35c6147839`. The first complete run found real UInt32 JS-wrap and index-proof bugs plus W-DECL-2 fallout. Fixes and targeted tests pass. Its stable corrected source is in a second strict full run, `/tmp/fa-resume-analyzer-workspace-fixed.log`, not integrated yet. It prepares callbacks, with, remaining documented rules and pass fusion separately, possibly in a second worktree. It also owns benchmark generator cleanup (`_arguments`, `_error`, Array<String>).
- Agent `/root/proofs`: initial worktree `agent-aca08cbb0b8394a82`. Index proofs across units and per-file native caches are implemented with soundness tests. A further key-loop snapshot hole was found and corrected conservatively; final full validation is queued in `/tmp/fa-unit-index-final-workspace.log`. After that it owns Pick/Omit derived parameters, EF004 and @for key sets across files, in a separate worktree if needed.
- Agent `/root/native_perf`: initial worktree `agent-a375af73b4e738f36`. The initial HEAD is integrated. It now ports a binary-trees optimization (prototype 111M instructions vs Goose128M) and prepares eight-byte map slots, then all remaining Goose gaps. No prototype is considered complete before tests and measurement.

## Still required

1. Integrate corrected analyzer initial HEAD after its complete suite passes, then the new proofs and performance HEADs after their validations.
2. Complete and commit the lead's current generics/docs/Fields/pointer changes after full workspace, clippy, JS runtime and website validation. Pending notes file is not cleared yet: reconcile newly merged notes, including compressed arenas and packed literal choices.
3. Bun pointer generic-layout support; C libraries in wasm; native AArch64 runtime. Cross-file type/key work is owned by proofs, callbacks/with/pass fusion by analyzer.
4. Reach the user's goal of beating Goose everywhere, using the compiler rather than changing benchmark semantics. Stack string buffers of24–64bytes remain open;23-byte inline strings, capacity/reserve/from, nbody noalias, module constants, borrowed aggregates and nullable pointers are already implemented.
5. Audit and clean the stale entries in design/todo.md without hiding unfinished features.

## Accepted effect rule (user, 2026-11-06)

An explicit `() => T throws E` callback parameter follows ordinary try/catch rules: the receiving function can intercept E, and declares it only if it escapes (including a manual rethrow). Extra errors passed through a non-stored callback parameter without throws remain transparent to the receiving function. Overdeclared throws and mutates are errors everywhere, including expression bodies and annotated declarations. Existing mutation-path coverage and fewer-effects assignability for function values remain valid. Analyzer implements this in its callbacks worktree; lead updates the specification.

## Validation and workspace notes

LLVM_SYS_181_PREFIX=/home/claude-dev/.local/llvm18/usr/lib/llvm-18; add its bin and /home/claude-dev/.bun/bin to PATH. Cargo jobs1–2, tests2 threads, ulimit-v8000000 for full suites. ASan must also run outside the virtual-memory limit.

Use `flock --close /tmp/fa-compiler-full-suite.lock <command>`: an orphan Bun inherited an FD9 lock and blocked the queue. Analyzer repaired its timeout runner to kill the whole child process group and added a grandchild regression. Never share target/ between validation and preparation worktrees.

Lead WIP backups are stashes `0bfc84d85797eeeca8e069c87bd9509677bbaf00` and `03cbc3f567fe3d1f1361f7569b33c5c6a3f3476c`, already applied; do not reapply them. The main documentation snapshot patch is /tmp/fa-resume-rewritten-documentation.patch. Git automatic gc cannot update the shared perf/native-goose ref; use `git -c gc.auto=0 commit` and preserve that ref's permissions.
