# Cross-file generic metadata, validated 2026-11-06

Worktree `cross-file-metadata`, branch of the same name. The lead integrates this
branch into `compiler`; keep the lead's central NEXT.md when reconciling notes.

## Implemented

- Interfaces retain complete generic order, runtime requirements, derived Pick/Omit
  results, implicit copy-write paths and foreign key-set applications.
- EF004 checks imported functions, import aliases and wrappers. Compiler-owned
  implicit/derived binders cannot be overridden by extra explicit arguments.
- Foreign `@for` bodies are checked once the callers are known. The pipeline hands
  that checked analysis to the JavaScript sink, rather than checking it again to
  generate it. Hidden key-set arguments carry stable tags and paths.
- Imported compile-time applications work as values on JavaScript and native code;
  native instance requests use the full binder order. Object cache version split-5.
- Native generic fields resolve their concrete slot by name, including physical
  places, nested ownership paths, frozen reads, reactive sources and hoisted pushes.
- Erased JavaScript parameters are potentially composite at ordinary copy sites:
  mutable copies, borrowed returns and stored elements preserve value semantics.
- The carried proof fixtures use angle generics and the platform engineer's
  deterministic two-phase async handshake.

## Validation

- Full LLVM workspace suite exits 0: /tmp/fa-metadata-workspace-full.log.
- Workspace clippy is clean: /tmp/fa-metadata-final-clippy.log.
- JavaScript runtime: 296 passing, /tmp/fa-metadata-js-runtime-verified.log.
- Whole JS crate, golden and pipeline tests: /tmp/fa-metadata-final-js-crate.log.
- The new copy regression passes native, wasm and freestanding:
  /tmp/fa-metadata-final-native-copies.log.
- Release 1000-file JS build: 17.05 s user CPU, 218732 KiB RSS with four threads;
  16.83 s CPU, 188180 KiB with one thread and another schedule seed. All 1006
  outputs are identical. The silent variant checks in 10.98 s CPU / 5.14 s wall,
  174208 KiB RSS with six threads. These runs share the machine with other suites.

## Integration

- Keep the compiler lead's generic syntax/documentation migration and exact effects.
- The lead's separate generic-pointer work extends GenericRequirement with Layout.
- The analyzer-with engineer owns the two explicit pending regressions:
  generic-hidden-arguments.fa (written method identities) and
  generic-key-loop-inferred-errors.fa (refresh inferred signatures and callers).
- The concrete-field patch was supplied to that engineer in
  /tmp/fa-generic-concrete-fields.patch; identical hunks should coalesce at merge.
- A separate `proofs/arena-regions` branch/worktree now owns closed whole-program
  region proofs. It does not alter this validated snapshot.
