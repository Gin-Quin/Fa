> Historical inventory: line numbers refer to `compiler` at `9cbb7558`.

# Static audit: what is still missing for the base of Fa

## Scope and method

- **Branches read:** `compiler` (lead worktree, HEAD `9cbb7558`, 515 commits not in `perf/native-goose`) and `perf/native-goose` (HEAD `d256f11a`, 41 commits not in `compiler`).
- **What I did:** read and grepped files only; nothing was built or run. Three read-only sub-agents swept the backends, the analyzer and interpreter, and the CLI. I spot-checked their key claims.
- **Paths:** relative to the lead worktree `/home/claude-dev/projects/Fa/.claude/worktrees/lead` unless prefixed `perf:`.
- **What "ok" means:** the harnesses run every runnable analyzer case on every target:
  - On Bun: `packages/cli/tests/programs.rs:651`. Its only backend exception is `BACKEND_GAPS` (`programs.rs:640-643`, adv-6.8-05).
  - On native, wasm and native-freestanding: `packages/codegen-llvm/tests/programs.rs`, where any failure not listed in `tests/expected-skips/<target>.txt` fails the suite.
  - So a feature whose cases are neither pending nor skip-listed is "ok" on Bun, native and wasm. NEXT.md records complete green suites on lead; I did not re-run them.
- **Not covered by "ok":** `node` and `web` have no general run harness. Node runs 4 fixtures (`packages/codegen-js/tests/method_origins.rs:49`); web runs only through `fa develop --target=web` (`cli/tests/workspaces.rs:1384`).

## 1. Support matrix

**Columns:** Analyzer, Interp (compile-time interpreter), Bun, Node/Web (same code generator as Bun; only target-specific runtime branches differ), Native, wasm.
**Cells:** ok, partial, missing, by design (a documented target limit), unknown.

| Feature (doc page) | Analyzer | Interp | Bun | Node/Web | Native | wasm |
|---|---|---|---|---|---|---|
| Booleans | ok | ok | ok | unknown (not run) | ok | ok |
| Integers, ranges, overflow (values/numbers, operations/arithmetic) | ok | ok (META013) | ok (64-bit as BigInt; E-NUM-19 for values beyond ±2^53, by design) | unknown | partial: `Integer#checkedPow` (cli/tests/programs/builtin-methods.fa:34); `Float.nan`/`Float.infinity`/`Float.maxValue`/`Float.epsilon` used as values (number-api.fa:47, "this name") | same as native |
| Floats, Float32, Percentage, BigInt | ok | ok | ok | unknown | ok; native-freestanding: no float printing (56 entries in `expected-skips/native-freestanding.txt`), no `BigInt` (conversions.fa:6) | ok (`pow` comes from the host; at `-O0`, `floats_and_strings` lacks the `pow` import: perf:design/todo.md:114) |
| Strings, current grapheme semantics | ok | ok | ok | unknown | partial: `use word = text[0..5]` (T-STR-29, "indexing this value") | same |
| Strings as UTF-16 over UTF-8, `Bytes`, no `Character` (user decision 2026-11-15) | missing | missing | missing | missing | missing | missing (perf:design/todo.md:67-97; no `codeUnitAt`/`fromUtf8`/`utf8Length` in either branch) |
| Tuples | ok | ok | ok | unknown | partial: rest pattern (T-DST-5/6) | same |
| Ranges | ok | ok | ok | unknown | partial: assigning `r.start`/`r.end` (T-RNG-2, tuples-ranges-patterns.fa:27, "this place"); `r == 1..3` (T-CMP-17) | same |
| Optionals | partial: E-OPT-5 has no emitter (T-OPT-25 pending); W-CMP-1 (T-OPT-28-b) | ok | ok | unknown | partial: `Array<Integer?>.get` (T-OPT-7); `while let` only over optional scalars (lower/mod.rs:2910, :2928) | same |
| Objects | partial: E-OBJ-14 has no emitter; T-OBJ-23-b, T-OBJ-36 pending; "this object literal" AN001 (check/objects.rs:81, :262) | ok | ok | unknown | partial: a field of an anonymous union of object types (un-5, un-15, "this member") | same |
| Errors | partial: E-ERR-5, W-ERR-4 have no emitter; T-ERR-2/6/7/8/23/25/27 and advanced/14-async/doc-11-errors pending | ok | ok | unknown | ok | ok |
| Declarations (`let`/`mutable`/`static`/`reactive`/`derived`) | partial: destructuring of `reactive`/`derived`/`static` is AN001 (check/destructuring.rs:71); E-DECL-7/12/13 pending | `let`/`mutable`/`static` only (lower.rs:752) | ok | unknown | ok | ok |
| Comparison, `is`, `is in` (operations/logical) | partial: E-CMP-8/9/10 pending (E-CMP-10 has no emitter); `is in` on a pool or columnar is AN001 (ops.rs:1115) | ok | ok | unknown | partial: range equality | same |
| Destructuring (operations/destructuring) | partial: `use` destructuring AN001 (destructuring.rs:115); nested patterns and defaults in `as` AN001 (patterns.rs:535); T-DST-11/13 pending | missing: object/array/tuple patterns, `...rest` (comptime/lower.rs:829-839, :1767) | ok | unknown | partial: rest patterns; defaults over optional fields (T-DST-3/4, "Optional→Int") | same |
| `if`, `when` (control-flow/conditionals) | partial: E-WHEN-4/7 have no emitter; T-WHEN-4/23/24/25/27/30 pending | partial: "this `when`" (lower.rs:1588) | ok | unknown | ok | ok |
| Loops | partial: E-LOOP-3/4/6/13 and T-LOOP-44 pending; `for` over some values AN001 (control.rs:686); destructuring a range value (control.rs:997) | partial: no `for parallel` (lower.rs:1153) | ok | unknown | partial: `while let` | same |
| `do` blocks | ok (T-DO-4 pending) | ok | ok | unknown | ok | ok |
| Defining functions, defaults, named arguments, variadics | partial: pa-11/12/14/16/18/19, fd-4, fd-10 pending; review #6 (effects of default values) has a case | ok | ok | unknown | ok | ok |
| Lambdas, closures | partial: FN072 has no emitter; la-5, cl-6 pending | ok | ok | unknown | ok | ok |
| Function types | partial: only `throws`/`mutates` clauses (type_expr.rs:607); TY072, FN086 have no emitter; ft-2/3/4/6/10 and adversarial/req-06 pending | n/a | ok | unknown | partial: `Human.describe` as a value (ft-9, "this member") | same |
| Composition `then` (functions/function-composition) | ok | missing (lower.rs:777) | ok | unknown | partial: falls back to a runtime value when a step can't be lowered (lower/compose.rs:50) | same |
| Partial application `with` | **missing**: `check_partial_application` only reports AN001 (check/functions.rs:1475-1482); FN100-104 have no emitter; co-9/10/11 pending | missing (lower.rs:778) | missing (no lowering) | missing | missing | missing |
| Factories, purity | partial: FN110 has no emitter; co-12, pu-2 pending | — | ok | unknown | ok | ok |
| Declaring types | partial: a type that depends on itself through an alias is AN001 "recursive types" (check/decl.rs:366); TY001, TY012 have no emitter; ty-decl-7/12/14/15 pending | — | ok | unknown | ok | ok |
| Unions and enumerations | partial: TY043, TY044 have no emitter; un-2/10/11/14, nu-7/13/14/15, en-6 pending; written types with different methods joining into a union not done (design/todo.md:40) | ok | ok | unknown | partial: `Integer(Color.red)` (en-1, en-2) | same |
| Generics (angle brackets) | partial: the narrower GN030 and the single E-DECL-6 not done (perf:design/todo.md:126-127); GN017, GN030 have no emitter | — | ok | unknown | ok | ok |
| Fields, key sets | partial: a key-set parameter outside `@for` is AN001 (key_sets.rs:337) | ok | ok | unknown | ok | ok |
| Type functions, `TypeFromSchema`, `@typeToSchema` | partial: `TypeFromSchema` inside a generic type function (schemas.rs:76, design/todo.md:26); TF002 has no emitter | ok | ok | unknown | **missing**: compile-time named union members (comptime/schemas.fa, schema-types.fa, schema-field-defaults.fa are skipped on native, wasm and freestanding) | same |
| Type conversion | partial: CV003 has no emitter; cv-4, cv-5 pending | partial: "this conversion" (lower.rs:1503) | ok | unknown | partial: Enum→Int, Optional→Int | same |
| Structural typing | partial: adv-10.8-03 pending | — | ok | unknown | ok | ok |
| Arrays, GrowingArray | partial: ar-5/22/25 pending | ok | ok | unknown | ok (T-OPT-7 aside) | ok |
| Maps | ok | ok | ok | unknown | ok | ok |
| Sets | ok, but the docs and the analyzer still have `Set.from(…, key = …)`, which the user removed (NEXT.md:119) | ok | ok | unknown | missing `key` (se-3); moot once `key` is removed | same |
| Pools | ok | n/a | partial: reference to a primitive element (`pool.find`) (generate/references.rs:501; collections.rs:480; perf todo:139) | unknown | ok | ok |
| Columnars (incl. Ordered) | ok | n/a | ok | unknown | ok (by columns; `OrderedColumnar` and element types with relation sides stay in rows: decisions.md:38) | ok |
| Relations | partial: adv-8.10-15 and adversarial/rel-12 pending (references returned by a function, `invalidates` of a reference parameter, owner-path substitution, `use` of a relation field: decisions.md:225); `parallel()` over a relation side is AN001 | n/a | ok | unknown | partial: "assigning a many-to-many side" (lower/relations.rs:381) | same |
| References `use`/`mutable use`/`reuse`, owned references | partial: functions returning references (adv-4.5-09..12, ref-24/25/34/35 pending, "M2") | n/a | ok (pool primitive aside) | unknown | partial: `reuse current = none` over `map.get` (adv-4.5-06); `use big = values.find(…)` (references.fa:42) | same |
| Reference checking | partial: adv-5.13-01/16/20/25/32 pending | n/a | n/a | n/a | n/a | n/a |
| Memory management: `copy value`, noncopyable resources (decisions 2026-11-07 and NEXT.md:110-126) | **missing**: no `Copy` node in the parser; MEM003 has no emitter; MEM004 not found; the lot is unintegrated (design/todo.md:38) | — | partial: `destroy` of resources inside values, collections or replaced locals (decisions.md, Foreign code "Move-only" gap); a 1M-deep copy overflows Bun's stack (adv-6.8-05) | unknown | ok | ok |
| Requirements | partial: adv-9.9-10/11 pending | — | ok | unknown | ok | ok |
| Metaprogramming (`static`, `@if`, `@for`, `@readFile`, `@readDirectory`, `@parseJson`, `@typeToString`, `@import`, `@test`) | partial: `@extern` outside a C declaration is AN001 (comptime.rs:684); a compile-time parameter (comptime.rs:156); a static holding a function (:389) | partial (refusal list in §2, Interpreter) | ok | unknown | **missing**: `@readDirectory` (comptime/directories.fa skipped on native, expected skip on wasm); per-item `@for` (comptime/for-per-item.fa) | same |
| Reactivity | ok | n/a | ok | unknown | ok (fixtures reactivity.fa, reactivity-rounds.fa) | ok |
| Async, tasks | ok | n/a | partial: guard of an awaited `lock()` used as a temporary is never released (decisions.md, Concurrency "Guards") | unknown | ok | partial: `async-function-values.fa` is skipped on wasm with no reason given; the host drives the event loop |
| Workers, `WorkerPool` | ok | n/a | ok (J2) | web needs cross-origin isolation for shared memory (concurrency.js:17-37) | ok | by design (CONC010) |
| `for parallel`, `parallel()`, atomics, mutex, channel | ok | n/a | by design: CONC001; one-thread semantics | by design | ok | by design: CONC001; freestanding runs in order |
| Foreign code (`@extern`, `CStruct`, `CArray`, `Pointer`, `build.cLibrary`, `fa bind`) | partial: FFI007 has no emitter | n/a | partial: a `CStruct` by value is FFI008 (generate/foreign.rs:372); destroy gaps above | **by design**: C calls only on Bun (runtime/foreign.js:22: run-time error); FFI008 at build | ok (x86-64 SysV and AArch64 AAPCS64: lower/foreign.rs:52-60) | partial: env imports and a C library factory ok; libc-based programs skipped; freestanding cannot build a C library (link.rs:279) |
| `std.time`, `std.fs`, `std.json` | ok | — | ok | web: no `std.fs` (AN004, by design) | ok (async-files, json*) | ok (files through the host loader `fileRun`); freestanding: no fs/json (AN004) |
| `std.net` | ok | — | ok | Node ok; web by design | ok | by design |
| `std.http` (guides/web-server.md, concurrency.md:37, modules.md:435) | **missing**: no `std.http` module in the analyzer (std_library.rs:187-203) | — | missing | missing | missing | missing |
| Modules, packages, workspaces, configuration, dependencies | see §6 | configuration files ok | ok | builds only | per unit + fallback | ok |
| Testing (`*.test.fa`, `@test`, skip/only) | partial: adv-18.4-04/05/06 pending | `@test` ok | ok | rejected (testing.rs:33) | ok (linked executable; JIT decided 2026-11-14, on branch `tooling/native-jit`) | ok |
| Hot reloading, `fa develop` | — | — | ok (module swap) | web ok | ok (rebuild and restart) | rejected |

## 2. Gap inventory (deduplicated)

Format: `[area] description — evidence — size — depends on`.

### Native and wasm (LLVM backend: the `expected-skips` ratchet plus skip-target annotations)

1. [native] `use`/`reuse` over a reference returned by `map.get` or `array.find` ("a `use` of this value") — expected-skips/native.txt: adv-4.5-06, references.fa:42; lower/pools.rs:505 — M — none.
2. [native] A field read on an anonymous union of object types, and a type method used as a value (`Human.describe`) ("this member") — un-5, un-15, ft-9; lower/mod.rs:2422 — M — none.
3. [native] Enum→Integer conversion (`Integer(Color.red)`), and Optional→Int for destructuring defaults — en-1, en-2, T-DST-3, T-DST-4; lower/values.rs:1015 — S — none.
4. [native] Rest patterns in destructuring — T-DST-5/6; lower/tuples.rs:856 — M — none.
5. [native] Range values: field assignment and `==` — T-RNG-2, tuples-ranges-patterns.fa:27, T-CMP-17 — S — none.
6. [native] `Array<T?>.get` — T-OPT-7; lower/arrays.rs:506 — S — none.
7. [native] `use` of a string slice — T-STR-29 — S — wait for the UTF-16 lot (#38), which rewrites slicing.
8. [native] `Integer#checkedPow`; float constants used as values (`Float.nan`, `Float.infinity`…) — builtin-methods.fa:34, number-api.fa:47; lower/numbers.rs:472 — S — none.
9. [native] `Set.from(…, key = …)` — se-3 — S — moot once `key` is removed (#44).
10. [native] `while let` over non-scalar optionals — lower/mod.rs:2910-2928; perf todo:62 — S — none.
11. [native] Compile-time named union members (`Schema` values) — 3 comptime/schema*.fa cases skipped on native, wasm and freestanding; design/todo.md:28 — M — none.
12. [native] A compile-time directory listing (`@readDirectory`) and per-item `@for` — comptime/directories.fa, comptime/for-per-item.fa (skip-target native; expected skips on wasm and freestanding: "this member", "a value of type Void") — M — none.
13. [native] Assigning a many-to-many relation side — lower/relations.rs:381 — S/M — check the docs first.
14. [native] "Unsupported" messages print internal types (`Enum(NominalId(2147483659))`) — backends review #44; expected-skips lines — S — none.
15. [native] Generic instance fallbacks in the per-unit build: many "(split build)" refusals fall back to one module (units.rs:1615-2000, generic_values.rs:414-577, split.rs:269) — M — perf only.
16. [native] A returned `Tuple<T, T>` fails outside its generic instance — perf:design/todo.md:32-34 — S/M — none.
17. [native] `fa test` and `fa run` through the ORC JIT, forked child per test, `dev` profile — decisions.md:749-756; design/todo.md:34 (branch `tooling/native-jit`) — L — none.
18. [wasm] `async-function-values.fa` skipped on wasm with no reason — codegen-llvm/tests/programs/async-function-values.fa:1 — S/M (to diagnose) — none.
19. [wasm] At `-O0`, `floats_and_strings` needs a `pow` import, and `tree-arena-large-payload` declares 8,196 parameters (Node allows 1,000) — perf:design/todo.md:114 — S — matters once the JIT/`dev` profile reaches wasm.
20. [wasm] Verify `wasm-ld --stack-first` and that a deep shadow-stack overflow traps — perf:design/todo.md:43-48 — S — none.
21. [native-freestanding] No float printing (Ryu-like algorithm planned), no BigInt, no JSON, no stack-overflow handler, no C library — decisions.md:27, :35; expected-skips/native-freestanding.txt (81 entries, 56 for floats); compile.rs:298 — M/L — none. Optional target; low priority for "the base".

### JavaScript

22. [JS] `with` partial application: no lowering — analyzer gap #32 — M — #32.
23. [JS] A reference to a primitive element of a pool, and removing such an element — generate/references.rs:501, collections.rs:480; perf todo:139 — S/M — none.
24. [JS] The guard of an awaited `lock()` used as a temporary is never released — decisions.md (Concurrency, "Guards"); perf todo:139 — S — none.
25. [JS] `destroy` of resources held inside values or collections, moved on some paths only, or replaced — decisions.md (Foreign code, "Move-only", "(gap)") — M — #43 (unique-resource rules).
26. [JS] A `CStruct` passed by value is FFI008 — generate/foreign.rs:368-376 — M — none (needs `bun:ffi` support).
27. [JS] Generic `Pointer` capabilities on Bun: synthesized descriptor names and owner-argument order fail — design/todo.md:25; NEXT.md:46-53 — M — analyzer origin metadata.
28. [JS] Worker transport through codecs generated at build time (copy/read/write/drop functions in TypeInfo) — NEXT.md:128-145, 263-269; decisions.md:647-651 — L — #43.
29. [JS] A 1M-deep recursive value overflows Bun's stack when copied — cli/tests/programs.rs:640-643 (adv-6.8-05) — M — none.
30. [JS] Node and web are not run by any general harness: Node runs 4 fixtures (method_origins.rs:49); web is never executed except through `fa develop --target=web` — S/M (a harness) — none.
31. [JS] Possibly open, unverified: JSON keeps a lone surrogate (backends review #21; no `toWellFormed` in runtime/*.js). The UTF-16 decision now allows lone surrogates in strings, so only the JSON output rule (U+FFFD) remains to define — S — #38.

### Analyzer

32. [analyzer] Partial application `with`: no checking at all, FN100-104 have no emitter — check/functions.rs:1475-1482; cases co-9, co-10, co-11 pending; design/todo.md:26, :41 — M — none.
33. [analyzer] The 38 registered codes with no emitter:
    - Values brief: E-CMP-10, E-ERR-5, E-OBJ-14, E-OPT-5, E-STR-6, E-WHEN-4, E-WHEN-7, W-ERR-4, W-LEX-1.
    - Types brief: CL023, CV003, EF020, FN072, FN086, FN094, FN100-104, FN110, GN017, GN030, TF002, TY001, TY012, TY043, TY044, TY071, TY072, TY073, WFN002.
    - Advanced brief: CFG003, CONC011, FFI007, MEM003, MOD011, STR010.
    - Registries: codes/values.rs, codes/types.rs, codes/advanced.rs (sub-agent sweep).
    - Size M overall (each S) — none.
34. [analyzer] 116 `-- pending` cases. The reasons, counted from the headers:
    - "M1 imported from the types brief": 26.
    - "M1 imported from the values brief": 30.
    - "M2 imported from the advanced, types or values brief": 46.
    - "M2 key-loop refresh": 1.
    - "M2 user functions returning references": 4.
    - "M3": 7 (adv-16.5-01/04/05/06 configuration merge, adv-17.4-03/04 dependencies, adv-8.10-15).
    - AN001 effects in function types: 1 (adversarial/req-06).
    - An error-type gap: 1 (advanced/14-async/doc-11-errors).
    - The file list is in `grep -rn -- "^-- pending" packages/analyzer/tests/cases`.
    - L in total — none.
35. [analyzer] References returned by user functions (`(): Person in people`) and `invalidates` of a reference parameter — ref-24/25/34/35, adv-4.5-09..12, adv-8.10-15, adversarial/rel-12 — M — none.
36. [analyzer] Function types carry only the `throws` and `mutates` clauses (`requires`, `invalidates` and paths are AN001) — type_expr.rs:607, :619; decisions.md (Foreign code, "Function types") — M — none.
37. [analyzer] Destructuring: `use` patterns, `reactive`/`derived`/`static` destructuring, nested patterns and defaults in `as` — destructuring.rs:71, :115; patterns.rs:535 — M — none.
38. [analyzer + every backend + interpreter] UTF-16 strings, `Bytes`, `Utf8Error`, removal of `Character`, `codeUnitAt`, `utf8Length`, `toUtf8`/`fromUtf8`/`fromUtf8Lossy`, removal of `bytes()`, `File.readText`/`writeText`, `Json.parse(Bytes)` — perf:design/todo.md:67-97 (32 lead analyzer files name `Character`) — L — the perf merge.
39. [analyzer] Generic follow-ups: the narrower GN030; a single E-DECL-6 on `let f = <T>(…) => …` — perf:design/todo.md:126-127 — S — none.
40. [analyzer] `TypeFromSchema` inside a generic type function other than as its value — schemas.rs:76; design/todo.md:26 — M — none.
41. [analyzer] Key-loop effects: inferred expression bodies must refresh their thrown effects and their callers — modules/generic-key-loop-inferred-errors.fa (pending); design/todo.md:27 — M — none.
42. [analyzer] Written types with identical fields but different methods or destructors must keep both alternatives in an inferred union — design/todo.md:40 — M — origin metadata.
43. [analyzer] Noncopyable `Task` and `destroy` types; `copy value` for collections and their owners — design/todo.md:38; NEXT.md:110-126; no parser `Copy` node on lead (the WIP is in `/tmp/fa-shared-values`) — L — documentation (#58).
44. [analyzer] `Set` without the `key` callback: remove the `key` option (collections.rs:386; builtins/array.rs:325) — NEXT.md:119; design/todo.md:39 — S — documentation (#59).
45. [analyzer] Smaller AN001 refusals of documented code:
    - `is in` on a pool or columnar (ops.rs:1115); members and indexing of other collections (objects.rs:1337, :1664; collections.rs:141); `parallel()` of a relation side (concurrency.rs:878).
    - `@extern` outside a C declaration (comptime.rs:684); a type depending on itself through an alias (decl.rs:366); `for` over some values (control.rs:686); spreads (expr.rs:220); calling a type (calls.rs:169).
    - The full list (~75 sites) is in the analyzer sweep, file:line each.
    - M — none.
46. [analyzer, `fa check`] `fa check` never runs the backends, so native and JS "not supported yet" errors appear only at `fa build`/`fa run`. AN001 is hidden in the editor by default (language-server/src/analyzer_backend.rs:1108, `show_unsupported: false`) — design choice — M — fixed for good only by closing the backend gaps.
47. [analyzer] Single-walk pass fusion — a performance item, not required for "the base" (design/todo.md:8).

### Interpreter

48. [interpreter] Refuses: `with`, `then`, object/array/tuple patterns and `...rest`, readonly/union/optional/ref/throws/requires/effect types as values, big-integer edge cases, async code (`await`, `start`), `for parallel`, some assignments (`.=`), some `@if`, conversions, `when`, receivers, members and objects, statics of other files not computed yet, `static` holding a function, compile-time parameters — comptime/lower.rs:470-1767; check/comptime.rs:156, :389; interpreter/src/builtins.rs:726-879 (any builtin outside its lists) — M per group — none.
49. [interpreter] Same UTF-16/`Bytes` rules (#38) — perf:design/todo.md:93 — M — #38.
50. [interpreter] Test `the_budget_and_the_depth_are_bounded` fails under `ulimit -v 4000000` — perf:design/todo.md:60 (dropped from the lead todo; continuations work `2ec0e57` may have fixed it) — S — none.

### Parser and formatter

51. [parser] `<Item> async (…)` gives no message naming the right order; an unclosed generic list is not re-read as a comparison ("this generic list is never closed: add `>`") — perf:design/todo.md:128-130; no hit in parser/src — S/M — none.
52. [parser] `copy value` syntax (contextual prefix) — NEXT.md:112 — S — #43.
53. [formatter] Breaking long generic lists — perf:design/todo.md:129 — S/M — none.
54. [parser/analyzer] A block-bodied lambda ending in a tuple is typed `Void` — perf:design/todo.md:131 — S — none.

### CLI and project shapes

55. [CLI] Testing options:
    - `fa test --coverage` is documented (architecture/testing.md:220-226; getting-started/installation.md:65) but rejected (testing.rs:39-41).
    - The per-test `test.timeout` and `test.include` are parsed (analyzer/src/configuration.rs:149-150, :979-983) and never consumed: only the file-level `FA_TEST_TIMEOUT` exists (testing.rs:222).
    - M — none.
56. [CLI] Standalone file and package-name issues:
    - `fa test file.fa` prints "no test file" and exits 0 on every target (analyzer program.rs:38, testing.rs:155, 546).
    - `fa run --target=native|wasm <package>` doesn't resolve a workspace package name (run.rs:221 vs 63-73).
    - S — none.
57. [CLI] Native and wasm tests ignore `build.link` and `build.libraryPaths` (`Libraries::default()`, testing.rs:596) and always compile one module — S — #17.

### Documentation mismatches (which side changes is the lead's call)

58. Docs say "every assignment is a copy" (advanced/memory-management.md:9, both branches); the decisions require `copy value` for collections and their owners, and make `destroy` types noncopyable with no reference counting. Not documented.
59. Docs: `Set.from(…, key = …)` (collections/sets.md:48, :236); decision: no `key` callback.
60. Docs: `std.http` (guides/web-server.md, marked provisional; concurrency.md:37; modules.md:435); no implementation.
61. Docs: `fa test --coverage` and the per-test `timeout`/`include` (#55).
62. After the perf merge: docs describe UTF-16 strings and `Bytes`; code implements graphemes and `Character` (#38). The design records still describing graphemes are listed in perf:design/todo.md:97.
63. Docs: native case mapping uses full Unicode (decisions "after the backends review" (6)). `cli/tests/programs/case-conversion.fa` exists; whether it covers non-ASCII is unverified.
64. Pending documentation notes (design/pending-documentation.md: root regions, frame strings, recursive payloads, split fusion): implementation sections only.
65. `design/spec-review.md` "Still open" and "Parser follow-ups" are stale (packages/transpiler and packages/library no longer apply; the parser no longer panics). Historical.

## 3. Known bugs still open

**From the lead to-do list, `design/todo.md`:**
- `W-ERR-1` on a string slice in the Goose ports — not reproduced; needs the snippet (design/todo.md:12).
- Generic Bun pointers: synthesized descriptor parameter names and owner-capability argument order (design/todo.md:25; NEXT.md:49-53).

**From `perf:design/todo.md:58-65`; most are absent from the lead to-do list, so verify before scheduling:**
- Async lambdas and awaits through function values are blocking calls (native and JS).
- Detached tasks don't copy their arguments on JS.
- `.d.ts` files don't declare the two instances of an async function.
- A long top-level loop never yields. The lead decision "Loop back-edges of the top level and of tests yield" says this is done.
- The interpreter ulimit test (#50).
- Native pa-10 (a `Set` spread into a variadic parameter sums garbage) and ft-1 (calling a `mutates` function value overflows the stack). Neither is in expected-skips nor pending on lead, so probably fixed; confirm with a native run.
- A 50,000-line program takes 1.4 GB to build natively (one huge top-level function); split top-level code into several functions.

**Review findings of 2026-11-05 with no regression fixture found** (repros in design/reviews/repros/):
- Backends #19 `list-prepend-quadratic.fa`: deep copy on `list = List { head, tail = list }`. Performance, 24 s for 20,000 nodes.
- Backends #20 `repros/worker-cancel/`: a worker call cancelled right after `start` still runs on JS. Unverified: workers.js:16 now says a pending call is removed.
- Backends #21 `js-json-lone-surrogate.fa` (see #31).
- Backends #32 `js-enum-equality-size.fa`: enum `==` goes through `faEquals`. Size only; no `Repr::Enum` found.
- Analyzer #13 `move-mutable-use-branch.fa` and #10 `callback-capture-overlap.fa`.
- Analyzer #34 `ref-duplicate-report.fa` and `ref-error-cascade.fa` (diagnostic quality).
- Analyzer #24 `slow-literal-union-250.fa` (performance).
- Analyzer #23 `stack-overflow-chain-1500.fa`: the session now uses a 64 MB thread (analyzer/src/session.rs:913, parallel.rs:10); whether standalone `fa check` goes through it is unverified.
- Analyzer #42: the undocumented "3178 nested calls" compile-time limit.

Every other finding has a case named after its repro (analyzer/tests/cases/review/*.fa; cli/tests/programs/proof-*.fa, dense-map-indexed-insert.fa, destination-module-read.fa, never-negative-*.fa, task-settled-heap-results.fa, tasks-awaited-starter.fa, special-names.fa, release-int64-wrap.fa, segmentation-fault.foreign.fa, case-conversion.fa).

**Tooling review:** most findings have fixes and tests:
- forged lockfile, hostile sources, local-file reads, concurrent runs, `x.fa.fa`, non-UTF-8, error totals, test timeout (tests in workspaces.rs and robustness.rs);
- symlinks (format.rs:165), `--` before Git URLs (dependencies.rs:718), the hot-reload token (hot.rs:246-286), `didChangeWatchedFiles` (server.rs:143), `origin/` stripping (project.rs:704).

Unverified: #8 (each workspace command resolves the whole workspace once per package, O(packages²)), #9 (a standalone file scans up to 5,000 files: codebase.rs:239), #11 (the language server does network I/O on its analysis thread).

## 4. Proposed work packages (native first, then JS)

**WP1. Native expected-skips to zero.**
- Scope: gaps #1-#8 and #10, plus #14 (Fa type names in messages).
- Files: `packages/codegen-llvm/src/lower/{pools,references,objects,values,tuples,arrays,numbers,mod}.rs`.
- Prerequisites: none. Leave T-STR-29 for WP9 and se-3 for WP6.
- Done when: `tests/expected-skips/native.txt` and `wasm.txt` hold only se-3 (until WP6), T-STR-29 (until WP9), and on wasm the two CONC001 cases; the LLVM programs suite passes on native, wasm and freestanding.

**WP2. Native compile-time values.**
- Scope: #11 (Schema named union members) and #12 (`@readDirectory`, per-item `@for`).
- Files: `lower/constants.rs`, `lower/calls.rs`, the `unrolled_items` consumers.
- Done when: the `skip-target` lines of comptime/schemas.fa, schema-types.fa, schema-field-defaults.fa, directories.fa and for-per-item.fa are removed and these cases pass on native and wasm.

**WP3. Native `while let`, relations and wasm leftovers.**
- Scope: #10 if not done in WP1, #13, #18, #19, #20.
- Done when: async-function-values.fa runs on wasm (or a documented reason is recorded); the `-O0` wasm runs pass; there is a stack-overflow trap test on wasm.

**WP4. Native test and run path.**
- Scope: #17 (ORC JIT, fork per test, `dev` profile) and #57.
- Files: `packages/cli/src/{testing,run,native}.rs`, the codegen-llvm JIT module.
- Prerequisite: the `tooling/native-jit` branch.
- Done when: packages.rs:823, testing.rs:182/:232 and workspaces.rs:1814 pass through the JIT, a package with `build.link` passes `fa test --target=native`, and the wall-time comparison is recorded.

**WP5. Analyzer: partial application `with` end to end.**
- Scope: #32 and #22, plus the interpreter case of #48 and native lowering.
- Files: `analyzer/src/check/functions.rs`, `comptime/lower.rs`, `codegen-js/src/generate/functions.rs`, `codegen-llvm/src/lower/closures.rs`.
- Done when: co-9, co-10 and co-11 are un-pended and pass on Bun, native and wasm, and FN100-104 have emitters.

**WP6. Analyzer: ownership decisions.**
- Scope: #43, #44, #52, #25 and #58-#59 (the documentation first: the lead decides the wording).
- Files: parser `Copy` node, analyzer moves and MEM codes, both backends' ownership, `sets.md`, `memory-management.md`.
- Prerequisites: the documentation is written; merge the `/tmp/fa-shared-values` WIP.
- Done when: explicit-copy and shared-resources fixtures pass on Bun, native, freestanding and wasm with leak checks, MEM003 and MEM004 are emitted, and se-3 is rewritten to `Map`.

**WP7. Analyzer: pending cases, batch A (values brief, M1).**
- Scope: the 30 values-brief cases and the missing emitters E-CMP-10, E-ERR-5, E-OBJ-14, E-OPT-5, E-STR-6, E-WHEN-4, E-WHEN-7, W-ERR-4, W-LEX-1.
- Files: `check/{ops,control,patterns,objects,narrowing}.rs`.
- Done when: those cases pass with `FA_STRICT_PENDING=1`, and runnable ones also run on Bun and native.

**WP8. Analyzer: pending cases, batch B (types and advanced, M2/M3).**
- Scope: #35, #36, #37, #40, #41, #42, the types-brief cases, and codes FN072, FN086, FN110, GN017, TY001, TY012, TY043, TY044, TY071-073, CV003, EF020, WFN002, CL023, CFG003, MOD011, STR010, FFI007, MEM003.
- Split by sub-area if needed: references/relations; function types; types and unions; configuration/dependencies (adv-16.5, adv-17.4).
- Done when: the pending count goes to 0, there are no registered codes without an emitter (or each is marked reserved), and a registry test enforces it.

**WP9. Strings as UTF-16 and `Bytes`.**
- Scope: #38, #49, #7, #31, #62.
- Follow the order in perf:design/todo.md:67-97: analyzer, then both backends and the interpreter together, then fixtures.
- Prerequisite: the perf merge.
- Done when: the T-STR cases (12-14, 20, 21, 26, 27, 31, 32, 46, 47) and new non-ASCII cases agree on Bun, native, wasm and the interpreter; the 11 fixtures that use `Character` are updated; the benchmarks still pass.

**WP10. Generic follow-ups.**
- Scope: #39, #51, #53, #54.
- Files: `parser/src` (tokenize, recovery), `analyzer/src/check/{decl,generics}.rs`, `formatter`.
- Done when: the tests listed in perf:design/todo.md:126-131 exist (recovery.rs, gn-angle-brackets.fa, formatter corpus).

**WP11. JS gaps.**
- Scope: #23, #24, #26, #27, #29, plus a JS ratchet listing (the JS equivalent of expected-skips).
- Files: `codegen-js/src/generate/{references,collections,foreign,tasks}.rs`, `runtime/*.js`.
- Done when: BACKEND_GAPS is empty, a pool-primitive fixture and an awaited-lock-temporary fixture pass on Bun, and `bun test packages/codegen-js/runtime` passes.

**WP12. Node and web execution harness.**
- Scope: #30.
- Run the analyzer `prints:` cases on Node, and on web through a headless runner (or Bun with a web-target shim); record target-specific expected skips.
- Done when: Node and web columns can be marked from fixtures.

**WP13. CLI and project shapes.**
- Scope: #55, #56, and the coverage gaps in §6 (native and wasm runs of workspaces and dependencies; CLI error codes without tests).
- Files: `packages/cli/src/{testing,run}.rs`, `packages/cli/tests/{workspaces,packages}.rs`.
- Done when: the tests listed in §6 exist; `--coverage` is either implemented (LCOV) or removed from the docs (lead's call); the per-test timeout is enforced on Bun, native and wasm.

**WP14. Interpreter coverage.**
- Scope: #48: patterns, `when`, `then`, conversions, members.
- Files: `analyzer/src/comptime/lower.rs`, `interpreter/src/*`.
- Done when: an `@test` per construct passes, and META/AN001 "can't be evaluated at compile time yet" is reached only by impure or async code.

**WP15 (optional target). native-freestanding completeness.**
- Scope: #21: float printing without libc, BigInt, JSON, stack overflow.
- Done when: `expected-skips/native-freestanding.txt` shrinks to the AN004 entries.

**Suggested order:** WP1, WP2, WP4, WP3 (native); WP5, WP6 (decisions with cross-target impact); WP9 after the merge; WP11, WP12 (JS); WP7, WP8, WP10, WP13, WP14 in parallel; WP15 last.

## 5. Test annotations and skip lists (item 1)

**`-- pending:` in analyzer cases (116):**

| Reason | Count |
|---|---|
| M1 (values brief) | 30 |
| M1 (types brief) | 26 |
| M2 (advanced brief) | 21 |
| M2 (types brief) | 18 |
| M2 (values brief) | 7 |
| M2 (user functions returning references: ref-24/25/34/35) | 4 |
| M2 key-loop refresh (modules/generic-key-loop-inferred-errors) | 1 |
| M3 (advanced brief: adv-16.5-01/04/05/06, adv-17.4-03/04) | 6 |
| M3 rel-12 (`invalidates` of a reference parameter) | 1 |
| M3 adv-8.10-15 (references returned by a function, `invalidates`, private pool paths) | 1 |
| AN001 effects in function types (adversarial/req-06) | 1 |
| Error-type gap (advanced/14-async/doc-11-errors) | 1 |

No `-- pending` exists in cli/tests/programs or codegen-llvm/tests/programs.

**`-- skip-target:` (48):**
- No threads on wasm (workers, parallel), `wasm` only, 9: arena-parallel, async-worker-stress, async-workers, parallel-collections, parallel-loops, parallel-views, workers (codegen-llvm); proofs-worker (cli). Plus async-function-values (codegen-llvm), which gives no reason.
- No threads, sockets, libc or FFI, on `wasm` and `native-freestanding`, 15:
  - analyzer adv-13.9-07, 14-async/worker-pool-errors;
  - cli foreign-destroy, foreign-destroy-zero, foreign-destroy-holders, foreign-libc (all `.foreign`), pointer-allocate-c.foreign, worker-error-identity;
  - codegen-llvm async-net, async-worker-channel, async-worker-pool, async-worker-shared, parallel-channel, segmentation-fault.foreign;
  - async-net-blocking (also skips bun).
- `for parallel` needs threads, `wasm` and `bun`, 1: proofs-parallel (cli).
- libc floats, JSON, fs or stack guard, `native-freestanding` only, 11:
  - cli error-typed-display, json, json-defaults, json-unions, std-library;
  - codegen-llvm async-files, json-recursive-union, json-values, region-direct-edges-json, region-forests-json, stack_overflow.
- **Real backend gaps, 5:** comptime/directories (native), comptime/for-per-item (native), comptime/schemas, schema-types, schema-field-defaults (native, wasm and freestanding).
- Semantics differ by design, 1: overflow-unchecked (native, wasm, freestanding).
- Wasm-only limit tests, skipped on `native` and `native-freestanding`, 6: adv-12.8-06, array-from-wasm-limit, map-from-wasm-limit, string-from-wasm-limit, string-reserve-wasm-limit, string-reserve-explicit-wasm-limit.

**`-- asynchronous: false` (29):** they force synchronous mode for optimizations gated per program:
- regions, frame strings, return slots, reservation overflow, compact maps: region-roots (analyzer modules); region-*, frame-strings, recursive-return-paths, compact-int-maps, *-reserve-overflow, *-from-overflow, decimal-appends, inline-append-loops, split-returned-views, throwing-union-leaves, the *-wasm-limit files (codegen-llvm);
- async-blocking (codegen-llvm): tests the synchronous mode itself.

They are not feature gaps. They do show these optimizations are off in async programs; perf:design/todo.md:45 asks for a per-function gate.

**`-- opt-level: 0` (4):** async-long-turn, region-string-escape, string-reserve-wasm-limit, segmentation-fault.foreign.

**`#[ignore]`:** language-server/tests/memory.rs:136 and :149; language-server/tests/package.rs:215; analyzer/tests/benchmark.rs:105; parser/tests/doc_snippets.rs:310; cli/tests/memory.rs:161 and :168; cli/tests/package_memory.rs:91. All are long-running memory, benchmark or fuzz tests, not feature gaps.

**Hard-coded lists in the harnesses:**
- codegen-llvm/tests/programs.rs:210-211 drops every path containing "javascript" (JS-only programs: ±2^53, names).
- codegen-llvm/tests/programs.rs:212-216 excludes adv-15.9-16 and adv-15.9-18 (they need arguments).
- cli/tests/programs.rs:
  - `BACKEND_GAPS` = [adv-6.8-05] (:640);
  - `ANALYZER_GAPS` and `JAVASCRIPT_ORDER` are empty (:634, :525);
  - `NATIVE_STDERR` = [async-long-turn] (:529);
  - `MINIFIED`: 7 programs (:589).
- The LLVM harness drops `kind: test` cases (programs.rs:182), so `*.test.fa` never runs there.
- expected-skips: native 19, wasm 23 (native plus adv-14.6-01, adv-14.6-09 (CONC001), comptime/directories, comptime/for-per-item), freestanding 81 (56 of them `snprintf` floats).
- codegen-js tests have no skip lists.

**Backend refusal counts:**
- LLVM: about 422 `unsupported(…)` / `Error::Unsupported` sites. The largest groups: functions, calls and generics 53; objects and unions 40; foreign 39; arrays and iterators 38; modules and split build 29; tasks 27; parallel 25; numbers 25.
- JS: about 91 (88 `unsupported(…)`, which reports AN001 "… is not supported by the JavaScript backend yet" (generate/mod.rs:1042), plus E-NUM-19 ×2 and FFI008).
- No `todo!`, `unimplemented!`, TODO or FIXME anywhere in packages/*/src.
- The sub-agent's list is grouped by feature with file:line; the feature-relevant ones are cited above. Ask for it if you want the whole dump.

## 6. Project shapes (item 6)

**Covered by tests:**
- Standalone file:
  - check (robustness.rs:56; packages.rs:639 JSON);
  - build with default targets web+bun+native (packages.rs:515), and bun+native when the program has a foreign file (:568);
  - run on Bun (programs.rs: every program and analyzer case);
  - multi-file standalone run on native (packages.rs:1005);
  - single-file run on native, wasm and freestanding only through the LLVM library harness, not through the CLI.
- Package:
  - check and build on Bun, web and node (packages.rs:285-740, :483);
  - run on Bun, native and wasm (packages.rs:132; relations, reactiveRelations, maps, cursors on bun and native with split on and off);
  - native build output, LTO and determinism (packages.rs:360-452);
  - C library build on native and wasm (c_library.rs);
  - test on Bun (:775), native (:823), wasm (testing.rs:182, :232);
  - develop on bun, web and native (workspaces.rs:1384, :1541, :1593-1745);
  - new, format.
- Workspace:
  - check, test, run `api`, build at the root, preview on bun, preview node gives CLI006 (workspaces.rs:196-265);
  - dependencies: Git, versions, generated sources, imports, path, remote files, lockfile, update (workspaces.rs:271-1132; all on check or Bun run);
  - develop bun by name (:1308); `test --watch` on bun (:1785) and native/wasm (:1814, a package).

**Supported but untested:**
- Workspace build, test, run and develop on native or wasm; whole-workspace develop and preview.
- Any dependency (sibling, Git, remote) built or run on native, wasm or web.
- `fa preview` of native, wasm or web outputs.
- Standalone `fa build --target=wasm` and `fa run --target=native|wasm` through the CLI.
- native-freestanding through the CLI.
- Multi-file cases on wasm through the CLI.

**Rejected:**
- `fa test` on node, web and native-freestanding (testing.rs:33, no code).
- `fa develop` on wasm and node (develop.rs:773).
- `fa preview` of a standalone file (preview.rs:107).
- `fa run` on node and web (CLI002).
- `--coverage` (testing.rs:39).

**Silently unsupported:** `fa test <file.fa>` (program.rs:38 passes no test files).

**Codes with no test:** CLI002, CLI003, CLI005, CLI009, the deno message, "built without LLVM", CLI001 from `fa update`.

**Gaps against the "three shapes on every target" goal:**
- `fa run --target=native|wasm <workspace package>` (run.rs:221).
- Native and wasm tests ignore `build.link` and never build per unit (testing.rs:596).
- Native dependencies are untested.
- No `fa test` for a standalone file.

## 7. What I could not determine statically, and the cheapest way to find out

1. **Whether the lead suites are green at HEAD.** NEXT.md records green complete runs at earlier merges.
   - Check: `cargo test -p fa-codegen-llvm --test programs` (with `LLVM_SYS_181_PREFIX`), `cargo test -p fa --test programs --test packages --test workspaces`, and `FA_STRICT_PENDING=1 cargo test -p fa-analyzer`. The last one also shows pending cases that now pass (the analyzer review found 5).
2. **Node and web behavior.**
   - Check: run the `prints:` analyzer cases with `fa build --target=node` plus `node main.js`; for web, the same build loaded in headless Chromium.
3. **The bugs listed only in the perf to-do** (async lambdas blocking; detached JS task argument copies; `.d.ts` async instances; pa-10; ft-1; the interpreter ulimit test).
   - Check: run pa-10 and ft-1 with `--target=native --leak-check`; run the interpreter test under `ulimit -v 4000000`; write one fixture each for an async lambda awaited through a function value and a detached task given a mutable argument, then run on Bun and native.
4. **Open review repros** (worker-cancel, json-lone-surrogate, move-mutable-use-branch, callback-capture-overlap, ref-duplicate-report, stack-overflow-chain-1500, list-prepend-quadratic).
   - Check: `fa check` / `fa run` each file in design/reviews/repros/ and compare with the review's expected output.
5. **Why async-function-values.fa is skipped on wasm.**
   - Check: remove the annotation and run `FA_PROGRAMS=async-function-values cargo test -p fa-codegen-llvm --test programs`.
6. **Native dependencies and native workspaces.**
   - Check: `fa build --target=native` in `packages/cli/tests/workspaces/demo/applications/api`, and in a scratch package depending on a sibling package.
7. **Whether case-conversion.fa covers non-ASCII text** (backends review #18).
   - Check: read it, or run it on native.
8. **How much of the perf branch's "Features still missing" (perf:design/todo.md:133-140) is stale.** Most items have lead evidence of completion: relations, columnar layout, reactivity and compile-time collections natively; `CArray`; the C library; AArch64 struct ABI; `parallel()`; J2; hot reload; per-file test workers; `DependencySource.generate`; native std.
   - Check: drop those items at merge after the suite is green, rather than carrying them into the merged to-do list.

## Key paths

- Lead to-do: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/design/todo.md`
- Perf to-do: `/home/claude-dev/projects/Fa/design/todo.md`
- Reviews and repros: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/design/reviews/`
- LLVM ratchet: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/packages/codegen-llvm/tests/expected-skips/{native,wasm,native-freestanding}.txt`
- Bun harness lists: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/packages/cli/tests/programs.rs:520-660`
- Analyzer `with` stub: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/packages/analyzer/src/check/functions.rs:1475`
- Test options: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/packages/cli/src/testing.rs:27-41`, `:596`
- Std availability per target: `/home/claude-dev/projects/Fa/.claude/worktrees/lead/packages/analyzer/src/builtins/std_library.rs:150-160`