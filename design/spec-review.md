# Fa specification review (2026-11-01)

This is a design record, like [`compilation-with-llvm.md`](compilation-with-llvm.md): it is not a language page, and the pages of `documentation/` stay the source of truth. It records a full review of the documentation, made to answer one question: *is the specification clear and complete enough to start implementing the compiler?* (a checker shared by all targets, then the JavaScript backend, then native code through LLVM). It lists every problem found, how it was resolved, and what is still open.

## Method

- All 66 pages of `documentation/` were read in full, in six scopes (values and operations, control flow and functions, types, memory and relations, concurrency, architecture and the "Fa for…" pages), each checked against the language reference.
- Every ```fa block of the documentation was run through `fa-parser`.
- The author approved every recommendation of the review. The cross-cutting decisions were written down first ([decisions](#decisions)), then applied to every page, then the website was built to check the links and anchors.
- Where a reviewer had to decide a point that the approved recommendations did not cover, the item says ***decided by the reviewer***: these are the choices to review first ([points to review](#points-to-review)).

## Verdict

**Before the review:** not ready for a checker. The language was coherent and the reference was good user documentation, but the normative rules were missing in four areas: numeric and literal typing, narrowing, effects through functions, and above all reference checking. There were also about thirty contradictions between pages.

**After the review:** ready to start. Every contradiction found is resolved, every blocking ambiguity has a written rule, and reference checking has its own normative page, [`advanced/reference-checking`](../documentation/advanced/reference-checking.md). What remains is either a point to review (decided by a reviewer, not by the author), or a parser gap. The native-backend questions of `compilation-with-llvm.md` were decided afterwards (see [still open](#still-open)).

Checks after the fixes:

- `fa-parser` on the 910 ```fa blocks: 886 parse. The 24 failures are 4 intended error examples and 20 blocks using syntax the parser doesn't support yet ([parser follow-ups](#parser-follow-ups)).
- `bun run build` of the website: passes (no broken link or anchor).

## Decisions

The cross-cutting decisions, applied to every page. Each one names its canonical page.

### Contradictions resolved

| # | Decision | Canonical page |
| --- | --- | --- |
| C1 | A lambda takes its types and effects from the expected function type; "a block body declares its return type and effects" only applies to `function` declarations | `functions/defining-functions` |
| C2 | `==` and `!=` against a literal or `none` narrow, like `is` | `control-flow/conditionals#narrowing` |
| C3 | Union members are always qualified in `when` (`ApiError.NotFound`) | `control-flow/conditionals` |
| C4 | Integer types are ranges with a storage width (`Int8` is `Integer<min = -128, max = 127>`); a value fits wherever its range fits; narrowing is `Int8(x)`; conversions between kinds stay explicit | `values/numbers#integer-types-are-ranges` |
| C5 | On JavaScript, `Integer` overflow is checked at ±(2^53 − 1) and stops the program; `Int64`, `UInt64`, 128-bit types and `BigInt` are JavaScript `BigInt`s; `Integer.jsMaxSafeInteger` | `values/numbers` |
| C6 | `use` on a place holding a reference binds its target; `LinkedList` rewritten with exact effects | `advanced/reference-checking`, `advanced/relations` |
| C7 | `map[key] = value` declares `mutates self[key], self.size` | `collections/maps` |
| C8 | `start do` is an async context; `Task.all`, `Task.any`, `Task.settled` return a `Task` | `advanced/concurrency` |
| C9 | Thread-safe types (`Mutex`, atomics, `Channel`) don't declare `mutates self`, so several tasks can use them | `advanced/concurrency` |
| C10 | Reactivity is push-pull: `derived` is lazy and always fresh, only `watch` blocks are deferred, runtime tracking | `advanced/reactivity` |
| C11 | `Any` is removed | `types/operations` |
| C12 | `.=` listed with the assignments; `Void` is a unit type usable as a type argument; capitalized generic parameters are always types (`Pick<ObjectType, keys: …>`) | reference, `types/type-functions` |

### Types

| # | Decision | Canonical page |
| --- | --- | --- |
| T1 | An unannotated binding has the widened type (`let x = 42` is an `Integer`); the exact literal, range and narrowed member are flow facts kept until the place is mutated | `values/declaring-values#inferred-types`, `types/declaring-types` |
| T2 | Generic bodies are checked once against their constraints; `@for`, `@if` and type functions after substitution; methods on a type parameter resolve on the actual type | `types/generics` |
| T3 | A union constraint binds the parameter to exactly one member | `types/generics` |
| T4 | Rule telling generic application, call and conversion apart (`repeatText<3>("ab")`, `Int8(x)`, `Integer<min = 0>`) | `types/generics` |
| T5 | Compatibility compares public data fields; methods and defaults resolve statically; a type with `#private` fields is nominal | `advanced/structural-typing#what-is-compared` |
| T6 | Anonymous unions must be distinguishable at runtime; `is Type` narrows unions only, and is decided at compile time otherwise | `types/unions` |
| T7 | Recursion through named declarations only; coinductive equality; polymorphic recursion is an error | `types/unions` |
| T8 | Floats follow IEEE 754 (`/` never throws); SameValueZero for set elements and map keys; a full table of result types | `values/numbers#floats`, `#result-types` |

### Flow

| # | Decision | Canonical page |
| --- | --- | --- |
| F1 | Range analysis is specified (intervals, `index < collection.length` facts, loop fixpoint with widening); it decides where `try` is needed | `values/numbers#range-analysis` |
| F2 | Narrowing, TypeScript style: what narrows, what ends a narrowing | `control-flow/conditionals#narrowing` |
| F3 | `{` after `=>`, `if`, `else`, `do`, `loop`, `for`, `while`, `watch`, `test`, `namespace` opens a block, elsewhere an object; `{ id }` is `{ id = id }` | `getting-started/syntax-basics#braces` |
| F4 | Full precedence table, including `with`, `??`, `try`/`await`/`start`, `catch`, `return`/`throw`/`break`; `try scores[index] = 0` | `operations/arithmetic#order-of-operations` |

### Functions and effects

| # | Decision | Canonical page |
| --- | --- | --- |
| E1 | Whether a function stores a callback parameter is inferred and part of its interface; a non-stored callback's effects become effects of the call | `functions/defining-functions` |
| E2 | A non-escaping closure captures by reference; an escaping one copies what it uses | `functions/defining-functions` |
| E3 | Function types: parameters by position, contravariant parameters, covariant returns, `=> Void` accepts any return, `async` in the type | `functions/defining-functions#function-types` |
| E4 | Definition of a pure function | `functions/defining-functions#pure-functions` |
| E5 | `Never` is the bottom type of `throw`, `return`, `break`, `continue`, a `loop` without `break` | `types/operations` |
| E6 | A place passed to a mutated parameter can't overlap another argument, a requirement, a module-level value the callee uses, or a reference captured by a callback (LLVM Q3, adopted) | `advanced/reference-checking#arguments-that-alias` |

### References

All in the new page `advanced/reference-checking`: R1 places and paths (`players[..].score`, comma lists, overlap rule), R2 the fixed facts of "may designate", R3 branches, loops and declaring again, R4 the implied effects of relations, R5 reference types that name paths, R6 who may write a relation field, R7 `use` on non-places, R8 read-only `Set` elements and `Map` keys, R9 no local collections of references.

### Modules

M1 lookup order and collisions, M2 initialization in dependency order and declarations only in package files, M3 a file's own name designates its export, M4 the prelude and `std`, and the export forms. Canonical page: `architecture/modules`.

### Async and requirements

A1 `async` in function types, A2 colorless async on JavaScript (`___sync` rules, per-target primitives, part of the interface), A3 `Task<T> throws E`, A4 structured, move-only tasks, A5 tasks on one thread, A6 requirements keyed by exact shape, `provide` semantics, resolved at the call site. Canonical pages: `advanced/concurrency`, `advanced/requirements`. One correction during the work: A6 said a `derived` keeps what was provided, but a `derived` that uses requirements is not pure (E4), so a `derived` can't use requirements.

### Other cross-cutting decisions

Stops (a failed runtime check stops the program, exit code 101, uncatchable; `values/errors#stops`), deep equality (`operations/logical#equality`), optionals (`T??` is `T?`, `?.` short-circuits the chain), strings (grapheme `length` as a deliberate difference from JavaScript, slices typed `String in text`), destructuring (rest patterns, defaults, `try let`), tests (test files export nothing, fresh module-level values per test), build-time code (one interpreter, declared configuration types, network only through the lockfile).

## Points to review

The reviewers decided these points where the approved recommendations were silent. They are applied in the documentation; the most consequential ones are listed here, and every one is marked *decided by the reviewer* in the items below.

**Language core**

- Number literals: exponents allowed, `.5` and `5.` rejected, ASCII identifiers only (V28). Line continuation: a line starting with `.`, `?.` or a binary operator other than `-` continues the previous one (V29).
- `Range = { start: Integer?, end: Integer?, includesEnd: Boolean }`, no steps, empty when inverted (V20).
- String escapes and interpolation (V23); the Math and String method tables, no regular expressions yet (V30).
- `Error` only exposes `toString()` (V32).
- A non-escaping closure may modify a captured `mutable` local without a declared effect (CF9).
- `then`: only the first step may take several parameters or mutate them (CF22); `with` binds by name only (CF23).
- Parameters share the scope of the body's top-level declarations (CF28).

**Types**

- `@` is written on every compile-time call, including inside type functions; compile-time calls may throw; evaluation limits (100 million steps, 256 nested type functions) (TY11).
- A non-object union payload is built with parentheses, `UserId.User(12)`, the one exception to braces (TY10).
- `when Value is` on types is a non-distributive subtype test (TY12).
- The whole layout of `Schema` (TY27); `Keys` only on object types, new `KeyType<Map>` (TY29).
- Enumerations: pinned values all or none (TY30).

**Memory and relations**

- E6 still allows the overlaps a callee's signature declares (`people.delete(ana)`, `befriend(ana, bao)`) (MR16).
- Many-to-many pools hold unique links, `add`/`delete` return a `Boolean` (MR19); copies keep links only inside one copied owner (MR22); a collection with owner fields is only modified through its owner (MR23).
- A reference that is not inside a collection can't be returned (MR14); a call applies its effects even when it throws (MR29).
- `OrderedColumnar` has the positional interface of an `Array` (MR6).

**Concurrency and reactivity**

- Detached `start` (a task not kept copies what it uses, can't throw, the program waits for it) and deferred task start (AC4, AC14).
- Thread-safe values are reference-counted handles shared by copies (AC7, AC16), the only reference counting in Fa.
- The flush point of `watch` blocks (AC5), their rules (AC24), reactiveness as a property of places (AC19).
- The `for parallel` rules (AC22), worker semantics and `WorkerPool` (AC27), the `Channel` and atomics APIs (AC28), the mutex guard (AC29).
- A blocking call behaves the same on every target, and native blocking calls may wait on a channel or a mutex (AC11).

**Toolchain**

- Dependency resolution: one version per repository and major version, Git tags, lockfile format, cache location, no test-only dependencies (AR16); remote folders need a `manifest.fa` (AR17).
- Configuration types, defaults and merge order (AR13, AR24); what configuration files see (AR14).
- Test runner: order, parallelism by file, `test only` under `CI` (AR20); `fa run` on a file with an exported function (AR7); the CLI table and exit codes (AR25).
- Shadowing a prelude name is a warning (AR1); a worker file is named by the part before `.worker.fa` (AR34).

## Still open

- **Native backend questions**: all decided on 2026-11-01, for the best runtime performance and the fastest compilation (see [questions to decide](compilation-with-llvm.md#questions-to-decide)): canonical field order with an opt-in C layout once foreign code is specified (Q1); escaping closure environments inline up to 8 bytes, task frames from a per-thread size-class allocator, inline small `BigInt` (Q4); small-string optimization, cached grapheme count and breadcrumbs (Q5); stable map slots with a segmented entry arena and no index for small maps (Q6); collections in Fa on an `intrinsics` namespace, with precompiled common instances (Q7); a runtime of our own instead of Tokio and Rayon (Q9); design records in `design/` (Q10).
- **Stale package READMEs**: `packages/transpiler/README.md` and `packages/library/README.md` describe running through Bun and compiling through Rust or Zig.
- **Stale analysis code**: `packages/parser/src/analysis/`, `types.rs` and `scope.rs` predate the specification (`Integer` as 32 bits, `Bag`); the checker should be a new crate, `fa-checker`, and `parse()` should stop calling the `analyze` stub.

## Parser follow-ups

Syntax decided in the documentation that the parser doesn't support yet:

| Construct | Example | Blocks |
| --- | --- | --- |
| `export mutable` | `export mutable: Pool<Person> = []` | relations, modules (×3), declaring-values, reference |
| Other export forms | `export async function = …`, `export type(First, Second) = …`, `export function(Item) = …` | modules (×4), web-server |
| `mutable` member of a namespace | `namespace counters = { mutable count = 0 }` | modules |
| `[..]` in effect paths | `mutates players[..].score` | defining-functions, reference-checking, relations |
| Async function types | `type Handler = async (request: Request) => Response` | defining-functions, concurrency |
| Annotated function declaration | `function ascending: Comparator = (a, b) => a - b` | defining-functions |
| Rest patterns | `{ name, ...details }`, `[first, ...others]` | destructuring |
| Nested strings and objects in interpolation | `"{user.name ?? "anonymous"}"` | strings |
| Precedence | `not` must bind tighter than `and` and `or` (the parser makes `not a or b` mean `not (a or b)`); `??`, `with`, `try`/`await`/`start` and `catch` per the table | — |
| Tokens | `parallel` is a keyword, `yield` is not | — |

Also: the parser panics on errors (about 50 `panic!`) instead of reporting diagnostics, which the checker and the language server need; an overflowing literal (`10_000_000_000_000_000_000`) should be a diagnostic; chained comparisons are stored as nested binary operations; placeholder `...` bodies parse as dangling spreads.

## Next steps

1. Review the [points to review](#points-to-review), and correct any choice you disagree with.
2. Parser: the follow-ups above, starting with diagnostics instead of panics and the `not` precedence.
3. Create `fa-checker` as a new crate, and start milestone M0 on single files: name resolution, scopes, base types, objects, arrays, optionals, `throws`/`try`/`catch`, `when` on named unions; then flow facts, narrowing and range analysis; then effects and reference checking from `advanced/reference-checking`.
4. A JavaScript backend on the same subset, then modules and packages.

## Items

Every problem found, by scope. Severity: blocking (a checker can't be written without guessing), important, minor. Locations are those of the documentation before the fixes.

### Values, operations and getting started

Files edited: `getting-started/introduction.md`, `syntax-basics.md`; `operations/arithmetic.md`, `destructuring.md`, `logical.md`, `string.md`; `values/declaring-values.md`, `errors.md`, `numbers.md`, `objects.md`, `optionals.md`, `ranges.md`, `strings.md`, `tuples.md`. Canonical homes: `/values/numbers` (`#integer-types-are-ranges`, `#range-analysis`, `#floats`, `#result-types`, `#percentages`), `/operations/arithmetic#order-of-operations` (precedence), `/getting-started/syntax-basics#braces`, `/values/objects#field-shorthand`, `/values/errors#stops`, `/operations/logical#equality`, `/values/declaring-values#inferred-types`.

- **V1** (blocking) `values/numbers.md:136`, `operations/arithmetic.md:123` vs "no implicit numeric conversions": `let c: Int16 = a + b` with `Int8` operands. **Resolution:** C4. *Decided by the reviewer:* with operands of mixed integer types, an unannotated result takes the type whose range contains the other, otherwise an annotation is needed. **Status:** fixed (table "Integer types are ranges", "Ranges through arithmetic").
- **V2** (blocking) `numbers.md:153`: integers silently lost precision on JS. **Resolution:** C5. *Decided by the reviewer:* the 64-bit types are `BigInt`s only when written explicitly; passing an `Integer` where an `Int64` is expected converts with `BigInt(value)`. **Status:** fixed in numbers.md, arithmetic.md, reference.
- **V3** (important) `introduction.md:21`: "no `any`" while `Any` existed. **Resolution:** C11; also fixed "like Rust, no escape hatch" (Rust has `unsafe`). **Status:** fixed.
- **V4** (important) `values/errors.md:155-175`: bare member names in `when`. **Resolution:** C3. **Status:** fixed.
- **V5** (important) `.=` missing from compound assignments and precedence. **Resolution:** C12, F4 level 18. **Status:** fixed.
- **V6** (important) "stops the program" undefined; "no panic" elsewhere. **Resolution:** Stops (exit code 101, uncatchable, fails a test, `FaStop` on JS). **Status:** fixed (`/values/errors#stops`).
- **V7** (minor) braces only for objects vs blocks. **Resolution:** F3. **Status:** fixed.
- **V8** (minor) `self` listed as a keyword. **Resolution:** *decided by the reviewer:* `self` and `Self` are special names; `skip` and `only` are contextual keywords. **Status:** fixed.
- **V9** (minor) shadowing rule missing invalidated references and `mutable use`. **Resolution:** R3. **Status:** fixed.
- **V10** (minor) declaration shape lacked generics, `export`, `reuse`. **Status:** fixed in syntax-basics.md and the reference.
- **V11** (blocking) range analysis unspecified. **Resolution:** F1 as a numbered rule set. *Decided by the reviewer:* a collection's length is known from its literal; a guard can be a place plus or minus a constant (`index + 1 < items.length`); copies and `map` keep their source's length; an assignment replaces a place's facts, any other mutation drops them; comparisons with constants refine intervals. **Status:** fixed (`#range-analysis`).
- **V12** (blocking) object or block; field shorthand. **Resolution:** F3. **Status:** fixed (`#braces`, `#field-shorthand`).
- **V13** (blocking) precedence of `try`, `catch`, `??`, `with`. **Resolution:** F4. **Status:** fixed (canonical table in arithmetic.md).
- **V14** (blocking) literal typing and widening. **Resolution:** T1. **Status:** fixed (`#inferred-types`).
- **V15** (blocking) result type per operator and number kind. **Resolution:** T8 table; a percentage is stored as its fraction. *Decided by the reviewer:* `BigInt / BigInt` gives a `Float`; an integer `**` with an exponent not proven non-negative throws `ArithmeticError`; `Percentage * Percentage` is a `Percentage`, `Percentage / Percentage` a `Float`, other mixes are errors. **Status:** fixed (`#result-types`, `#percentages`).
- **V16** (blocking) float division by zero, NaN, −0. **Resolution:** T8; added `isNaN()` and `isFinite()`. **Status:** fixed (`#floats`).
- **V17** (blocking) narrowing rules. **Resolution:** F2, C2. **Status:** fixed (links to `/control-flow/conditionals#narrowing`).
- **V18** (blocking) deep `==`. **Resolution:** equality decision. *Decided by the reviewer:* two union values are equal when they hold the same member with equal contents. **Status:** fixed (`#equality`).
- **V19** (important) runtime `is Type`, values on the right of `is`, operands of `is in`. **Resolution:** T6. *Decided by the reviewer:* the right side of `is` is always a pattern (use `==` for a value, `is in` for a stored `Range`); `is in` accepts collections, `Map` keys and `Range`, not strings. **Status:** fixed; examples testing `is Type` on non-unions rewritten.
- **V20** (important) the `Range` type. **Resolution:** *decided by the reviewer:* `Range = { start: Integer?, end: Integer?, includesEnd: Boolean }`; `5..2` is empty; no steps or descending ranges; range patterns only match integers; negative slice bounds count from the end, an inverted slice is empty. **Status:** fixed in ranges.md.
- **V21** (important) graphemes on JS, search units, `<`, slice type. **Resolution:** Strings decision. **Status:** fixed (`#grapheme-clusters`).
- **V22** (important) `Character` vs `String`. **Resolution:** Strings decision. **Status:** fixed.
- **V23** (important) interpolation nesting, multi-line strings, escapes. **Resolution:** *decided by the reviewer:* any expression inside interpolation braces, including nested strings and object literals; line breaks kept, CRLF becomes `\n`; `\xNN` only `00`–`7F`; `\u{}` must be a scalar value; an unknown escape is an error. **Status:** fixed.
- **V24** (important) nested optionals, `?.`, `??`. **Resolution:** `T??` is `T?`; `?.` short-circuits the chain. *Decided by the reviewer:* `??` is optional only when its right side is; its right side is evaluated only when needed. **Status:** fixed (`#nested-optionals`).
- **V25** (important) destructuring. **Resolution:** Destructuring decision. *Decided by the reviewer:* a failing array destructuring is `try let [a, b] = values`; destructured names in a loop are references like the loop variable. **Status:** fixed (`#default-values`, `#rest`).
- **V26** (important) `readonly`/`#private` at creation, value spread, defaults, methods. **Resolution:** T5. *Decided by the reviewer:* a `readonly` field can be set in the creating literal; `#private` fields can't be set from outside; in a value spread, a later field replaces a copied one of the same type; defaults are evaluated at each creation and can't see other fields. **Status:** fixed.
- **V27** (important) `use` on non-places and the JS lowering of `use` and copies. **Resolution:** R7, C6. *Decided by the reviewer:* JS lowering (a primitive place becomes its container and key; a `mutable use` is a pair of locals; a copy calls a generated copy function unless no later write is observable). **Status:** fixed.
- **V28** (important) number literal rules. **Resolution:** *decided by the reviewer:* exponents allowed (`1e6` is a `Float`); `.5` and `5.` are errors; underscores only between digits; `n` and `%` on any fitting literal; a leading `-` is part of the constant; an out-of-range literal is an error; identifiers are ASCII. **Status:** fixed.
- **V29** (important) line continuation. **Resolution:** *decided by the reviewer:* a line starting with `.`, `?.` or a binary operator other than `-` continues the previous one; `-`, `(` and `[` start a new statement; `else` goes on the line of the `}` (matches the parser). **Status:** fixed.
- **V30** (important) standard function signatures, postfix `?`. **Resolution:** *decided by the reviewer:* Math and String method tables (float rounding returns floats; `Integer.round` and friends throw `ArithmeticError` on non-finite floats; halves round away from zero; `sqrt` of a negative is NaN; `compareLocale` returns an `Integer`; no regular expressions in this version); bitwise shifts drop bits, invalid shifts are compile errors or throw; postfix `?` removed from the precedence. **Status:** fixed.
- **V31** (minor) small fixes (precedence table, destructuring wording, `std::string::String`, `persons`, `jsMaxSafeInteger`, `assert ... is Float`, tuples, `derived` wording). **Status:** fixed.

- **V32** (minor, follow-up) `values/errors.md`: what the built-in `Error` type exposes. **Resolution:** *decided by the reviewer:* every member of an `Errors` type satisfies `Error`, which only exposes `toString(): String` (the qualified member name and its data); a `message` field was rejected because every member would have to declare it. **Status:** fixed.
- **V33** (minor, follow-up) cross-file requests applied: `use`/`mutable use`/`reuse` details, redeclaration in inner blocks, the module-level example wrapped in a function, `catch` handlers are not functions, lambdas anywhere except as a declaration value, method resolution from the written type. **Status:** fixed.

#### Parser gaps

- `values/numbers.md:62`: intended error example, but the parser panics on an overflowing integer literal instead of reporting it.
- `values/strings.md:73`: a string or object literal inside an interpolation (`"{user.name ?? "anonymous"}"`).
- `operations/destructuring.md:48`: rest patterns `{ name, ...details }`, `[first, ...others]`.

### Control flow and functions

Files edited: `control-flow/conditionals.md`, `control-flow/do.md`, `control-flow/loops.md`, `functions/defining-functions.md`, `functions/function-composition.md`.

- **CF1** (blocking) `functions/defining-functions.md:44,53-56`, `control-flow/loops.md:270-275,304-311`, `values/declaring-values.md:296-297`: lambdas with a block body return values without a return type, against "a block body declares its return type". **Resolution:** C1: the rule only applies to `function` declarations; a lambda takes its types and effects from the expected function type and infers the rest; a block without a return type drops its last value (warning only when computing it has no effect). **Status:** fixed in defining-functions.md.
- **CF2** (blocking) `operations/logical.md:107` vs `operations/arithmetic.md:97-101`, `defining-functions.md:288-289`: `==` doesn't narrow on one page, others rely on it. **Resolution:** C2/F2. **Status:** fixed in conditionals.md (## Narrowing) and logical.md.
- **CF3** (blocking) `values/errors.md:155-163,174` vs the reference: bare and qualified union members in `when`, subject narrowed without `as`. **Resolution:** C3: always qualified; only a plain local subject is narrowed. **Status:** fixed in conditionals.md and errors.md.
- **CF4** (important) `defining-functions.md:188-189`, `loops.md:138`: `scores[index] = 0` can fail without `try`; parallel index proofs unspecified. **Resolution:** F4 (`try scores[index] = 0`), index proofs per F1. **Status:** fixed in defining-functions.md, loops.md.
- **CF5** (important) braces: block or object literal. **Resolution:** F3. **Status:** fixed in defining-functions.md (lambdas) and syntax-basics.md, sets.md.
- **CF6** (blocking) effects through higher-order functions. **Resolution:** E1, a new "Functions as parameters" section. **Status:** fixed. *Decided by the reviewer:* inside the callee, calling a non-stored parameter needs no `try`, and the callee can't catch those errors.
- **CF7** (blocking) narrowing underspecified. **Resolution:** F2, canonical section `/control-flow/conditionals#narrowing`. **Status:** fixed. *Decided by the reviewer:* `or` also narrows its right operand; `is in` narrows only against a literal array.
- **CF8** (blocking) function type compatibility. **Resolution:** E3 (positional matching, variance, `=> Void`, sub-paths, extra effects warned, fewer lambda parameters, no named arguments through values, `async` in the type). **Status:** fixed in defining-functions.md (## Function types).
- **CF9** (blocking) closures: capture and escape. **Resolution:** E2. **Status:** fixed (### Closures). *Decided by the reviewer:* a non-escaping closure may modify a captured `mutable` local without a declared effect; module-level values and requirements are never captured.
- **CF10** (blocking) what `is Type` tests at runtime. **Resolution:** T6. **Status:** fixed in conditionals.md, full rule in types/unions.md.
- **CF11** (blocking) `async` in function types, `await` in `start do`. **Resolution:** A1, C8. **Status:** fixed in defining-functions.md and do.md (## `start do`). *Decided by the reviewer:* `async` is never inferred.
- **CF12** (important) "pure" undefined. **Resolution:** E4. **Status:** fixed, canonical section `/functions/defining-functions#pure-functions`.
- **CF13** (important) the single-expression loophole and recursive inference. **Resolution:** *decided by the reviewer:* a single expression is any body that doesn't start with `{`; a function in a recursive cycle declares its return type, its effects are still inferred. **Status:** fixed.
- **CF14** (important) declaring effects a function doesn't have. **Resolution:** E3: allowed with a warning. **Status:** fixed.
- **CF15** (important) no path for a field of every element. **Resolution:** R1 `mutates players[..].score`. **Status:** fixed in defining-functions.md, loops.md.
- **CF16** (important) mutability of the loop variable per collection. **Resolution:** R8 plus a table. **Status:** fixed in loops.md. *Decided by the reviewer:* `item, index` only for `Array`, `GrowingArray` and `String`; a range without a start can't be iterated; `0..` runs until `break`.
- **CF17** (important) changing a collection during iteration. **Resolution:** R4: every effect of the body counts (aliases, calls, implied relation effects); writing an element in place is allowed. **Status:** fixed in loops.md.
- **CF18** (important) typing of `loop`, `break`, `Never`. **Resolution:** E5; `break value` in `for`/`while` is an error. **Status:** fixed. *Decided by the reviewer:* `break`/`continue` don't cross a function boundary.
- **CF19** (important) result type of `if`/`when`. **Resolution:** T1. **Status:** fixed in conditionals.md.
- **CF20** (important) `when` pattern details. **Resolution:** F4. *Decided by the reviewer:* a pattern table, no tuple or nested patterns; `as` destructuring keeps the fields common to every alternative; unreachable branches and a useless `else` are warnings; `as` only exists in `when`; exhaustiveness table (Boolean, enums, literal unions, integers by range coverage; `Float` and `String` need `else`). **Status:** fixed.
- **CF21** (important) generic inference at call sites. **Resolution:** T1/T2/T4. **Status:** fixed (links to generics). See TY items for the literal `0` against `Value: Integer | Float`.
- **CF22** (important) typing of `then`. **Resolution:** *decided by the reviewer:* the first step may have any parameters (kept with their defaults); later steps have exactly one parameter; effects are the union, but only the first step may mutate its parameters; `async` if any step is; a generic first step keeps the composition generic; nested compositions are flat; runtime operands give a plain value. **Status:** fixed in function-composition.md.
- **CF23** (important) typing of `with`. **Resolution:** *decided by the reviewer:* binding by name only; bound values evaluated and copied once; remaining parameters keep order and defaults; mutated and variadic parameters can't be bound; same effects; binds like a call (F4). **Status:** fixed.
- **CF24** (important) factories "known at compile time", `function f: Type`, where lambdas may appear. **Resolution:** *decided by the reviewer:* a factory call is known at compile time when the factory is pure and its arguments are literals, `static` values or compile-time functions; `function name: FunctionType = lambda` is allowed; lambdas are allowed anywhere a value is expected except as the value of a declaration. **Status:** fixed.
- **CF25** (important) defaults, named arguments, spread. **Resolution:** *decided by the reviewer:* defaults evaluated per call and may use earlier parameters; passing a parameter twice is an error; a variadic is never named; any collection of the right element type spreads, only into the variadic. **Status:** fixed.
- **CF26** (important) method dispatch. **Resolution:** T5, T2. *Decided by the reviewer:* inside a type, associated functions are called as `Self.create(...)`. **Status:** fixed.
- **CF27** (important) initialization order, mutual recursion of local functions. **Resolution:** M2. *Decided by the reviewer:* local functions can call themselves but not each other. **Status:** fixed (Hoisting).
- **CF28** (minor) scoping of parameters, `as` and `catch` bindings, module-level values in closures. **Resolution:** *decided by the reviewer:* parameters share the scope of the body's top-level declarations (redeclaring one is an error); `as` names live in their branch and may shadow; closures use module-level values directly. **Status:** fixed.
- **CF29** (minor) `while let` details. **Resolution:** *decided by the reviewer:* evaluated before each iteration, binds a name only (a copy); the `T??` consequence is documented. **Status:** fixed in loops.md.
- **CF30** (minor) wording: "three loop statements", "generator", `@typeToString` of a function, binding `Void`, `return` in a module-level `do`, values of a statement `when`. **Resolution:** reworded; `return` outside a function is an error; a statement `if`/`when` drops its branch values. **Status:** fixed.

- **CF31** (follow-up) cross-file requests applied: links to `/advanced/reference-checking`, `[..]` and comma paths, `invalidates` implies `mutates ….size`, E6 stated, read-only `GrowingSet` and `GrowingMap` keys in loops, `Void` per C12, `await` contexts (async functions and lambdas, `start do`, tests, the top level of a standalone file), detached `start do`, link to the parallel rules. *Decided by the reviewer:* only ranges with literal bounds count for `when` exhaustiveness. **Status:** fixed.

#### Parser gaps

- `functions/defining-functions.md:203`: effect path `mutates players[..].score`.
- `functions/defining-functions.md:500`: `function ascending: Comparator = (a, b) => a - b`.
- `functions/defining-functions.md:542`: async function type `type Handler = async (request: Request) => Response`.

### Types, structural typing and metaprogramming

Files edited: the nine pages of `types/`, `advanced/structural-typing.md`, `advanced/metaprogramming.md`. New sections: "Inferred types and known values" (declaring-types), "Checking generic code" and "Generic application or call" (generics), "Members must be distinguishable" (unions), "What is compared" (structural-typing), a full `Schema` definition (types-at-runtime).

- **TY1** (blocking) `types/generics.md:118-122`, `fields.md:117-119`, `type-functions.md:6-10`: generic bodies checked once or per instantiation. **Resolution:** T2 (with method constraints). **Status:** fixed in generics.md, type-functions.md, fields.md, metaprogramming.md.
- **TY2** (blocking) `generics.md:86-100`: meaning of a union constraint. **Resolution:** T3. **Status:** fixed.
- **TY3** (blocking) `type-conversion.md:107-110` vs `numbers.md:133-136`: `Int8` into `Int64` rejected while `Int8 + Int8` into `Int16` accepted. **Resolution:** C4; `Int8(x)` is proven or `throws ArithmeticError`. **Status:** fixed (new "Integers compare by range").
- **TY4** (blocking) literal types: exact or widened, join of branches. **Resolution:** T1. **Status:** fixed in declaring-types.md, generics.md, type-functions.md.
- **TY5** (blocking) generic application, call and conversion look the same. **Resolution:** T4, with "every argument is a type" requiring at least one argument (`empty()` stays a call). **Status:** fixed (new "Generic application or call").
- **TY6** (blocking) structural identity vs methods, `readonly`, private fields, defaults. **Resolution:** T5. **Status:** fixed (new "What is compared").
- **TY7** (blocking) anonymous unions: normalization and runtime discrimination. **Resolution:** T6. *Decided by the reviewer:* normalization by flattening and absorption (literal into base, `Int8` into `Integer`, `Never`); runtime kinds include `BigInt` and function; each object member needs a field of its own or a discriminant; a value satisfying several object members can't be stored. **Status:** fixed in unions.md, operations.md, types-at-runtime.md, structural-typing.md.
- **TY8** (blocking) `is Type` on a non-union value. **Resolution:** T6. **Status:** fixed.
- **TY9** (blocking) recursive types. **Resolution:** T7. **Status:** fixed (a `Folder` example through `Array`, coinductive interning).
- **TY10** (blocking) building `Union { User = Integer }`, bare members. **Resolution:** *decided by the reviewer:* a bare member carries no data; a non-object payload is built with parentheses, `UserId.User(12)` (the one exception to braces); named unions and their members are nominal; a `Shape.Circle` is a `Shape`, a plain object isn't a `Shape.Circle`; members are only reachable qualified; two members can't accept the same literal. **Status:** fixed in unions.md, declaring-types.md.
- **TY11** (blocking) `@` inside type functions, purity, termination. **Resolution:** *decided by the reviewer:* `@` is written on every compile-time call, including inside type functions and `static` values (consistent with `static six = @add(2, 4)`); compile-time calls follow E4 except that they may throw (the error becomes a compile error); bounds of 100 million steps per `@` call and 256 nested type-function applications; results memoized. **Status:** fixed in type-functions.md, metaprogramming.md.
- **TY12** (blocking) `when Value is` on types. **Resolution:** *decided by the reviewer:* a subtype test on the whole type, not distributive (unlike TypeScript). **Status:** fixed.
- **TY13** (important) capitalized parameters receiving values. **Resolution:** C12. *Decided by the reviewer:* `Pick` and `Omit` take `DeepKeys<ObjectType>` (nested picks). **Status:** fixed.
- **TY14** (important) `{ foo }` as key set or object shorthand; `Fields` identity. **Resolution:** F3; `Fields` structural, `Enum` nominal. **Status:** fixed.
- **TY15** (important) a `Set<Hero>` seen as `Set<Human>`. **Resolution:** R8; `==` compares the fields of the declared type. **Status:** fixed.
- **TY16** (important) function-type assignability. **Resolution:** E3. **Status:** fixed by the control-flow reviewer.
- **TY17** (important) `Void` only in function types vs `Task<Void>`. **Resolution:** C12. **Status:** fixed.
- **TY18** (important) `Any`. **Resolution:** C11, removed. **Status:** fixed.
- **TY19** (important) `Never`. **Resolution:** E5. **Status:** fixed.
- **TY20** (important) optional fields, `Partial`, `Required`, `T??`. **Resolution:** *decided by the reviewer:* a literal may omit a field with a default or accepting `none`; an existing value missing a field is never assignable; `Required` removes `None`; `T??` is `T?`. **Status:** fixed.
- **TY21** (important) spread semantics. **Resolution:** *decided by the reviewer:* only object types spread; data fields with defaults and `readonly` are copied, methods and `#private` fields are not; a duplicate of the same type is kept once, a different type is an error (no override); order only affects printing and schemas. **Status:** fixed.
- **TY22** (important) `-` on types. **Resolution:** *decided by the reviewer:* removes the members that satisfy the right side; removing nothing is an error; ranges keep their remaining parts; `String - "x"` is an error; a named union keeps its identity; an empty result is `Never`. **Status:** fixed.
- **TY23** (important) what `Type<e>` is. **Resolution:** *decided by the reviewer:* the inferred type without facts, `e` not evaluated; parameters, `use` aliases and loop variables keep the type of what they designate; a single-expression body returning a parameter returns `Type<param>`. **Status:** fixed.
- **TY24** (important) "never guesses" vs `sum([1, 2, 3])`. **Resolution:** *decided by the reviewer:* inference order: explicit parameters, expected type, argument types (base types of literals), defaults. **Status:** fixed.
- **TY25** (important) compile-time value parameters. **Resolution:** *decided by the reviewer:* values with exact equality (booleans, integers, strings, characters, enum and `Fields` values, and arrays, tuples, objects of them; no floats or functions); equal values give equal types; an empty range is `Never`. **Status:** fixed.
- **TY26** (important) ranged integer details. **Resolution:** C4 and the table in values/numbers.md. **Status:** fixed.
- **TY27** (important) `Schema` undefined. **Resolution:** *decided by the reviewer:* a built-in named union with one member per kind of type, plus `SchemaProperty`, `SchemaMember`, `SchemaFlag` (whole layout to review; the field `isReadonly`, since `readonly` is a keyword). **Status:** fixed in types-at-runtime.md.
- **TY28** (important) `@for` over key sets. **Resolution:** *decided by the reviewer:* leaves in declaration order; `key` is a compile-time path printed with dots; body checked per key after unrolling; key sets never inferred. **Status:** fixed.
- **TY29** (important) `Keys<Map>` returning a type. **Resolution:** *decided by the reviewer:* `Keys`/`DeepKeys` only on object types, excluding methods and `#private` fields; new `KeyType<Map>`; `Values` only on collections. **Status:** fixed.
- **TY30** (minor) enumerations. **Resolution:** *decided by the reviewer:* nominal; pinned values are distinct non-negative integers, all pinned or none; `members` in declaration order. **Status:** fixed.
- **TY31** (minor) `@typeToString`. **Resolution:** *decided by the reviewer:* the name written at the use site. **Status:** fixed.
- **TY32** (minor) two members accepting the same literal. **Resolution:** an error. **Status:** fixed.
- **TY33** (minor) exact literals against a union or generic type. **Resolution:** *decided by the reviewer:* must match one member exactly; a generic parameter takes the literal's full type. **Status:** fixed.
- **TY34** (minor) mutating through a narrower view. **Resolution:** "exactly the type" applies to the fields actually written. **Status:** fixed.
- **TY35** (minor) `...` listed as a type operator. **Status:** fixed (removed).
- **TY36** (minor) `Error`. **Resolution:** every member of an `Errors` type satisfies `Error`. **Status:** fixed; what `Error` exposes is in values/errors.md.
- **TY37** (minor) generic type without arguments in an annotation. **Resolution:** an error, except for parameters with defaults. **Status:** fixed.
- **TY38** (minor) `Point { x = 12, y = 1.5 }`. **Resolution:** an error by T1. **Status:** fixed.
- **TY39** (minor) hidden `IndexError` in `capitalizeSecondLetter`. **Status:** fixed.

- **TY40** (follow-up) `structural-typing.md`: `Set`/`Map` key references are read-only (link to reference checking); `reactive` is part of a field like `readonly`; metaprogramming states that configuration files run on the same interpreter. **Status:** fixed.

#### Parser gaps

None (only the intended error example `types/declaring-types.md:15`).

### Memory, references, relations and collections

New normative page `advanced/reference-checking.md` (listed in the navigation after "Relations"): places and paths (R1), owning edges and flatness, mutations and invalidations, what a body's declared effects must cover, "may designate" (R2), control flow and declaring again (R3), reference types (R5), `use` and `mutable use` (R7, C6, R8, R9), iteration, implied effects of relations (R4), writing relation fields (R6), arguments that alias (E6), tasks, and the worked examples P1–P8 with their verdicts. Also edited: `memory-management.md`, `owned-references.md`, `relations.md`, every page of `collections/`.

- **MR1** (important) `relations.md:357, 372-386, 395, 411`: `LinkedList.add` declared `mutates self` while claiming references survive; `remove` wrote undeclared fields; the example redeclared a valid `first`. **Resolution:** `add` declares `mutates self.#nodes.size, self.#nodes[..].next, self.firstChild, self.lastChild` and returns a fresh node; `remove` declares `mutates self.#nodes[..].next, self.firstChild, self.lastChild invalidates node`; the example no longer redeclares `first`. **Status:** fixed.
- **MR2** (blocking) `use next = node.next` aliasing the field would corrupt the list. **Resolution:** C6. **Status:** fixed in reference-checking.md, relations.md, owned-references.md, declaring-values.md.
- **MR3** (important) mutable loop variables and `use` over a `Set` or `Map` keys break hashing. **Resolution:** R8. **Status:** fixed in sets.md, maps.md, growing-collections.md, loops.md, structural-typing.md.
- **MR4** (important) writing an absent map key hid the size change. **Resolution:** C7; writing a key while iterating the map is an error (write through the loop's value). **Status:** fixed.
- **MR5** (important) capture of reference names by closures. **Resolution:** E2. **Status:** fixed in memory-management.md, defining-functions.md.
- **MR6** (minor) `OrderedColumnar` "same interface" but deletes by index; missing from tables. **Resolution:** *decided by the reviewer:* it has the positional interface of an `Array` (`add` appends and returns a reference, `[index] = value`, `delete(index)`, `take(index)`, `pop`, `deleteWhere`, `clear`), an effects table, no growing counterpart, no linked objects. **Status:** fixed; reference updated.
- **MR7** (minor) "you never write it" vs reference types written in signatures. **Resolution:** inferred for `use`, written in parameters, return types and relation fields. **Status:** fixed.
- **MR8** (minor) three wordings of copy-to-move. **Resolution:** a copy becomes a move when the source isn't used afterwards (by name or alias); never observable. **Status:** fixed.
- **MR9** (minor) two meanings of "flat". **Resolution:** "flat" means invalidation-free (`Integer?` is not flat, a relation reference is); the LLVM document says "trivially copyable" for "no copy or drop function". **Status:** fixed.
- **MR10** (blocking) no grammar for effect paths, no overlap rule. **Resolution:** R1 (grammar, roots, index terms, owning edges, overlap rules). **Status:** fixed.
- **MR11** (blocking) precision of "may designate". **Resolution:** R2: exact values, integer ranges, the same immutable binding plus an offset, freshness; applied at the first step where two paths differ; no other reasoning. **Status:** fixed.
- **MR12** (blocking) branches, loops, scope of a redeclaration. **Resolution:** R3. **Status:** fixed.
- **MR13** (blocking) writes made by relation updates and deletions weren't effects. **Resolution:** R4, a normative table (direct-side writes, many-to-many pools, owner fields, `add` with direct sides, `delete`/`take`, `deleteWhere`/`clear`, replacing the collection). **Status:** fixed.
- **MR14** (blocking) path-dependent reference types. **Resolution:** R5 (nearest collection or string, substitution of parameter names, a parameter typed by another one, private paths in signatures, owners identified by path). *Decided by the reviewer:* a reference that isn't inside a collection (`use address = user.address`) has the plain type and can't be returned; a reference returned into a temporary can only be copied. **Status:** fixed.
- **MR15** (blocking) who may write a direct side. **Resolution:** R6. **Status:** fixed.
- **MR16** (blocking) aliasing between arguments (LLVM Q3). **Resolution:** E6. *Decided by the reviewer:* the overlaps the callee's signature declares stay allowed: an argument for a parameter typed `T in p` is inside the argument of `p` (`people.delete(ana)`), and an implied effect on `C[..].f` overlaps an argument typed `T in C` (`befriend(ana, bao)`); otherwise every `delete` and relation write would be an error. **Status:** fixed.
- **MR17** (important) `use` on a non-place. **Resolution:** R7. **Status:** fixed.
- **MR18** (important) local collections of references. **Resolution:** R9. **Status:** fixed.
- **MR19** (important) the many-to-many API. **Resolution:** *decided by the reviewer:* relation pools hold links; `add(target)` and `delete(target)` return a `Boolean`; a target is linked at most once; `clear`, `size`, `includes`, `find` and iteration give the targets; no `= []` initializer on either side. **Status:** fixed.
- **MR20** (minor) relation shapes beyond the table. **Resolution:** *decided by the reviewer:* only the shapes of the table, exactly one reverse side per direct side, ordered relations as a linked list, self-links allowed. **Status:** fixed.
- **MR21** (important) assigning a whole linked element; literals with direct sides. **Resolution:** *decided by the reviewer:* a whole-value assignment replaces the data fields and keeps the links; a literal given to `add` may set direct sides, never reverse sides. **Status:** fixed.
- **MR22** (important) copying relations. **Resolution:** *decided by the reviewer:* links are kept only between collections copied together inside one copied owner; any other copy has no links. **Status:** fixed.
- **MR23** (important) how a deletion finds owner fields. **Resolution:** *decided by the reviewer:* owner fields are optional, several may point at the same element; a collection with owner fields is only modified through a path from its owner. **Status:** fixed.
- **MR24** (important) storing a callback isn't visible in the signature. **Resolution:** E1. **Status:** fixed.
- **MR25** (important) when a task's effects reach the caller. **Resolution:** *decided by the reviewer:* at the `start`; the borrow lasts until the task ends (A4). **Status:** fixed in reference-checking.md, concurrency.md.
- **MR26** (important) flatness under structural views and generics. **Resolution:** *decided by the reviewer:* decided by the callee's declared type with the type arguments substituted; a value of a type parameter is never flat in a generic body. **Status:** fixed.
- **MR27** (important) `invalidates element` and the collection's size. **Resolution:** R4. **Status:** fixed.
- **MR28** (minor) a reference returned by a call with effects. **Resolution:** *decided by the reviewer:* valid after the call's effects; exact identities don't cross a user function, only freshness does; `getOrInsert(key)` keeps the identity of `key`. **Status:** fixed.
- **MR29** (minor) effects of a call that throws. **Resolution:** *decided by the reviewer:* applied on every exit. **Status:** fixed.
- **MR30** (minor) gaps in the effect tables. **Resolution:** `Set.take` (returns the value or `none`, `mutates self`), `Set.deleteWhere`, `GrowingPool.add` and `GrowingColumnar.add` return fresh references, tables for growing collections, a path designating no element invalidates nothing, `Map` values mutable and keys read-only in a loop. **Status:** fixed.
- **MR31** (minor) observable moves, partial moves. **Resolution:** moves are never observable, so partial moves are free. **Status:** fixed.
- **MR32** (minor) `mutable use` details. **Resolution:** *decided by the reviewer:* allowed on an immutable collection (read-only, movable); `reuse` takes the forms and type of `use` (`none` if optional); `use y = x` designates what `x` designates at that moment. **Status:** fixed.

- **MR33** (follow-up) `memory-management.md`: task frames, escaping closure environments and thread-safe handles are heap blocks made by the compiler; thread-safe handles are reference-counted, so copying one shares it (the only exception to "assignment copies"). Also `sets.md` brace wording and `DeepKeys`, `arrays.md` range-analysis link, negative slice bounds and `try let`. **Status:** fixed.

#### Parser gaps

- `advanced/reference-checking.md:32` and `advanced/relations.md:373`: the `[..]` path after `mutates`. Comma lists and `.#private` paths already parse.

### Concurrency, reactivity, requirements and guides

Files edited: `advanced/concurrency.md`, `advanced/reactivity.md`, `advanced/requirements.md`, `guides/hello-world.md`, `build-your-application.md`, `performance.md`, `web-server.md`, `deploy-a-library.md`. The three advanced pages now have `:::implementation` sections (async instances and blocking mode, task frames and cancellation, native executors, Web Workers and `MessagePort` on JavaScript, push-pull signals, requirements as hidden parameters).

- **AC1** (blocking) `concurrency.md:395`: `Task.all` without `await` would block. **Resolution:** C8. **Status:** fixed.
- **AC2** (blocking) `concurrency.md:111` vs the channel and mutex examples. **Resolution:** C9; `mutates` removed from those examples. **Status:** fixed.
- **AC3** (blocking) tasks on one thread vs Tokio work-stealing. **Resolution:** A5; Tokio/Rayon moved to the implementation section. **Status:** fixed.
- **AC4** (blocking) `await` in `start do` outside async functions. **Resolution:** C8. *Decided by the reviewer:* async contexts are async functions, async lambdas, `start do`, tests and the top level of a standalone file; `start` takes any call; a started task begins at the starter's next `await` or when the starter returns to the event loop (not synchronously like a JavaScript `async` call). **Status:** fixed.
- **AC5** (blocking) batched recomputation vs a fresh read. **Resolution:** C10. *Decided by the reviewer:* watches flush after the current statement of the outermost running code (entry function, task, test, watch, platform callback, standalone top-level statement) and before each `await`; a `do` block counts as one statement. **Status:** fixed.
- **AC6** (blocking) static or dynamic dependency tracking. **Resolution:** C10: runtime tracking, reads inside called functions count. **Status:** fixed.
- **AC7** (important) channels and mutexes can't reach a worker if arguments are copied. **Resolution:** *decided by the reviewer:* thread-safe values are reference-counted handles (a copy designates the same mutex, atomic or channel) and cross worker boundaries; on JavaScript and wasm, a mutex or atomic shared with a worker only holds flat values; a JavaScript channel is a `MessagePort`. **Status:** fixed.
- **AC8** (minor) `Task<Void>`. **Resolution:** C12. **Status:** fixed by the control-flow and types reviewers.
- **AC9** (minor) `mutable subscription` vs `let`; `cancel()` on a `let task`. **Resolution:** *decided by the reviewer:* `let subscription = watch`; `stop()` consumes the subscription (and a worker handle); `cancel()` doesn't mutate the handle. **Status:** fixed.
- **AC10** (minor) parallel iteration on wasm. **Resolution:** `for parallel` and `parallel()` are native only. **Status:** fixed.
- **AC11** (blocking) what `___sync` contains, how primitives declare their versions. **Resolution:** A2. *Decided by the reviewer:* blocking calls behave the same on every target (inside one, `await` blocks and `start` runs eagerly to a finished task; blocking mode is dynamic); awaiting, inside a blocking call, a task started outside it is an error everywhere; native blocking calls may wait on a channel, mutex or worker, JavaScript and wasm may not; a per-target primitive table; primitives declare their versions through a standard-library intrinsic. **Status:** fixed.
- **AC12** (blocking) function types without `async`. **Resolution:** A1. *Decided by the reviewer:* a non-async function can be stored in an async type; a lambda is async when its expected type is; `items.map(fetchData)` with an async callback is an error. **Status:** fixed.
- **AC13** (blocking) `Task<T>` without errors. **Resolution:** A3 and a table of the combinators' types. *Decided by the reviewer:* `Task.settled` gives `Array<T | E>`. **Status:** fixed.
- **AC14** (blocking) task lifetime and cancellation. **Resolution:** A4. *Decided by the reviewer:* a `start` statement whose task isn't kept is **detached**: it copies its arguments and captures, uses no locals or requirements, can't throw, and the program waits for it before exiting; `cancel()` is synchronous; `await` adds `Cancelled` only when `cancel()` may have been called. **Status:** fixed.
- **AC15** (blocking) what a task borrows; `await` and module-level references. **Resolution:** A5. **Status:** fixed.
- **AC16** (blocking) thread-safe types undefined. **Resolution:** *decided by the reviewer:* `Mutex<T>`, `AtomicInteger`, `AtomicBoolean`, `Channel<T>`, and objects or tuples whose fields are all thread-safe; handles whose methods don't declare `mutates self`. **Status:** fixed.
- **AC17** (blocking) requirement keys under structural typing. **Resolution:** A6. *Decided by the reviewer:* generic types are keyed with their arguments. **Status:** fixed.
- **AC18** (blocking) "pure" undefined. **Resolution:** E4, linked. **Status:** fixed.
- **AC19** (important) reactivity "part of the type system". **Resolution:** *decided by the reviewer:* reactiveness is a property of places (a copy is plain; references and parameters stay tracked; `mutates` writes notify); `reactive` is part of a field like `readonly`; functions are instantiated per reactiveness. **Status:** fixed.
- **AC20** (important) `provide` semantics. **Resolution:** A6; a `provide` inside a branch lasts until the end of the branch; a mutated requirement can't overlap a mutated argument (E6). **Status:** fixed.
- **AC21** (important) requirements and closures, watches, entry point. **Resolution:** A6, E1. *Decided by the reviewer:* the entry function, the top level of a standalone file and tests provide requirements rather than declare them; a `derived` can't use requirements (E4 makes it impure, which corrects A6). **Status:** fixed.
- **AC22** (important) `for parallel` rules. **Resolution:** *decided by the reviewer:* reads of values no iteration modifies; writes through the loop variable, to `target[index]` and below (with `try` when unproven), to thread-safe values and locals; calls checked through their effects; requirements readable, `console` allowed; no `break`, `return`, `await` or reactive values; the failed iteration with the smallest index wins; same rules for `parallel()`. **Status:** fixed.
- **AC23** (important) blocking-callability missing from the interface. **Resolution:** part of the interface; adding or removing `async`, or losing blocking calls on a target, is a major change. **Status:** fixed in concurrency.md, deploy-a-library.md.
- **AC24** (important) `watch` semantics. **Resolution:** *decided by the reviewer:* a watch can't throw and isn't an async context (tasks it starts are cancelled when it runs again or is destroyed); writing a value it reads itself is a compile error, more than 100 rounds stops the program; pending watches run in declaration order; the cleanup is a final lambda; non-reactive locals are copied when declared; a module-level watch lives as long as the program. **Status:** fixed.
- **AC25** (important) no lowering of reactivity. **Resolution:** implementation section (versioned sources, push then pull, flush gated by a depth counter, explicit `dispose` on JavaScript). **Status:** fixed.
- **AC26** (minor) reactivity and threads. **Resolution:** reactive values belong to their thread; parallel loops can't use them; workers get a plain copy. **Status:** fixed.
- **AC27** (important) worker semantics. **Resolution:** *decided by the reviewer:* move-only handles; calls queued and run one at a time; no field access from outside; `stop` is reserved (waits for the current call, cancels queued ones, consumes the handle); worker code can't use non-thread-safe module-level `mutable` values or `requires`; `WorkerPool` (`submit`, `maxQueueSize`, `timeout` → `WorkerTimeout`, `shutdown`). **Status:** fixed.
- **AC28** (minor) incomplete APIs. **Resolution:** *decided by the reviewer:* `Channel` (unbounded by default, `send` throws `ChannelClosed`, `trySend`, `tryReceive`, `close`), atomics (`load`, `store`, `increment`/`decrement`/`add` return the new value, `swap`, `compareAndSwap` returns a `Boolean`, named `ordering` argument, `Ordering` from `std.concurrency`), `Mutex.tryLock()`, `parallel()` in the target table. **Status:** fixed.
- **AC29** (important) the mutex guard bound with `use`. **Resolution:** *decided by the reviewer:* `mutable guard = await stats.lock()`; the guard owns the lock until the end of its scope, `guard.value` is a reference. **Status:** fixed.
- **AC30** (minor) async entry function. **Resolution:** M4. **Status:** fixed in web-server.md.
- **AC31** (minor) blocking `server.listen()` on `bun`. **Status:** fixed (async entry, `try await server.listen()`).
- **AC32** (minor) a stored handler mutating a module-level `pool`. **Status:** fixed (handler type stated, `Database` provided per request, `pool` thread-safe).
- **AC33** (minor) undefined `routes`. **Status:** fixed.
- **AC34** (minor) unknown `createUser` errors. **Status:** fixed.
- **AC35** (minor) `await ... catch` precedence. **Resolution:** F4. **Status:** fixed.
- **AC36** (minor) `for parallel` suggested for file I/O. **Status:** fixed (`start` with `Task.all` for I/O).
- **AC37** (minor) scaffold without `dependencies.fa`. **Status:** fixed.
- **AC38** (minor) undefined `sleep`, `DataProcessor`, `Ordering`. **Status:** `Ordering` fixed; `sleep` added to the prelude; `DataProcessor` stays illustrative.
- **AC39** (minor) "`/` cannot throw `DivisionByZero`". **Resolution:** T8. **Status:** fixed.
- **AC40** (minor) `Integer.JS_MAX_SAFE_INTEGER` in performance.md. **Resolution:** C5. **Status:** fixed.
- **AC41** (minor) no implementation sections in the advanced pages. **Status:** fixed.

#### Parser gaps

- `advanced/concurrency.md:94`: async function type `type Handler = async (request: Request) => Response`.
- `guides/web-server.md:11`: `export async function = ...`.

### Architecture, modules, toolchain and "Fa for…" pages

Files edited: `architecture/modules.md`, `overview.md`, `packages.md`, `workspaces.md`, `configuration.md`, `dependencies.md`, `testing.md`, `hot-reloading.md`, the seven `fa-for/*` pages, root `README.md`, `AGENTS.md` (paths). New anchors: `/architecture/modules#name-lookup`, `#the-files-own-name`, `#export-forms`, `#module-level-values-and-initialization`, `#prelude`, `/architecture/overview#command-line`, `/architecture/configuration#how-the-configuration-is-merged`, `/architecture/testing#how-tests-run`.

- **AR1** (blocking) `architecture/modules.md:56-61`: lookup order and name collisions unspecified. **Resolution:** M1 (local scopes including the file's own name, siblings with merged `(group)` contents, root entries and `library`, dependencies, prelude); same-level duplicates are errors. *Decided by the reviewer:* shadowing a prelude name is a warning. **Status:** fixed (## Name lookup).
- **AR2** (blocking) `modules.md:59`: "root files" while root folders were used as namespaces. **Resolution:** root entries (files and folders). **Status:** fixed.
- **AR3** (blocking) `modules.md:222-237`: `(group)` folders below the root. **Resolution:** a group can be at any level; its contents merge into the parent folder for every rule. **Status:** fixed.
- **AR4** (blocking) file names as identifiers. **Resolution:** M1: `.fa` files and folders containing `.fa` files must be valid identifiers; `*.test.fa` are never names; case-only differences are errors. *Decided by the reviewer:* other files, folders without `.fa` files and dot-names are ignored. **Status:** fixed.
- **AR5** (blocking) `advanced/relations.md:65-79`: a file couldn't name its own export. **Resolution:** M3. *Decided by the reviewer:* a module-level declaration with the file's name is an error. **Status:** fixed (## The file's own name).
- **AR6** (blocking) initialization, cycles, teardown of module-level values. **Resolution:** M2. *Decided by the reviewer:* ties broken by declaration order then file path; only reachable values are initialized; initializers can't use `try` or `await`; what a hot reload re-initializes. **Status:** fixed in modules.md, hot-reloading.md.
- **AR7** (blocking) top-level statements in package files. **Resolution:** M2: package files contain only declarations. *Decided by the reviewer:* `fa run file.fa` runs its statements in order (may use `try`/`await`), an escaping error exits with 1, an exported function is then called as the entry. **Status:** fixed in modules.md, overview.md, packages.md, configuration.md.
- **AR8** (blocking) `std`, `console`, `Task`, `Math`, built-in errors undefined. **Resolution:** M4 prelude table (built-in types and constructors, `console`, `Math`, `Task`, `IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`, `Cancelled`, `target`, `configuration`, `std`). **Status:** fixed (## Prelude).
- **AR9** (blocking) `packages/parser/src/priority.rs:10-13`: the parser makes `not` looser than `or`. **Resolution:** F4 keeps the reference order. **Status:** still open (parser).
- **AR10** (blocking) `parse_expression.rs:1144`: `export mutable` doesn't parse. **Status:** still open (parser).
- **AR11** (important) `configuration.md:9` vs `metaprogramming.md`: two evaluators for build-time code. **Resolution:** one compile-time interpreter. **Status:** fixed (the package READMEs are still stale, see open items).
- **AR12** (important) "no network at compile time" vs fetching in `dependencies.fa`. **Resolution:** only dependency sources and `@import` reach the network, through the lockfile. *Decided by the reviewer:* config files can't read environment variables but can use `@readFile`; order `dependencies.fa`, `workspace.fa`, `package.fa`; a config file never uses the dependencies it declares. **Status:** fixed.
- **AR13** (important) no configuration API. **Resolution:** declared types `Target`, `PackageConfiguration` (with `BuildOptions`, `DevelopOptions`, `TestOptions`, `TargetOverride(s)`), `WorkspaceConfiguration`, `DependencySource = String | { source, generate }`. *Decided by the reviewer:* the shapes and defaults. **Status:** fixed.
- **AR14** (important) names visible in config files. **Resolution:** *decided by the reviewer:* the `.fa` files next to them and, through `workspace`, those at the workspace root; not `library/`, `application/`, `scripts/`, `packages/`. **Status:** fixed.
- **AR15** (important) `workspaces.md:55` vs `:64`. **Resolution:** *decided by the reviewer:* relative paths stay and follow folders; a package's name is its unique `name`; the scan doesn't enter a package; CLI `<package>` is the name. **Status:** fixed.
- **AR16** (important) version resolution, lockfile, cache. **Resolution:** *decided by the reviewer:* one version per repository and major version (latest satisfying every range); transitive dependencies invisible; versions are semver Git tags matching `package.fa`'s `version`; pre-releases only when named; one `dependencies.lock` per workspace, package or standalone file, in Fa syntax, SHA-256; cache in the user cache directory or `FA_CACHE`; no test-only dependencies; `gitlab:` and `git+https:` sources. **Status:** fixed.
- **AR17** (important) remote folders can't be listed. **Resolution:** *decided by the reviewer:* a remote folder serves a `manifest.fa` and is its own root. **Status:** fixed.
- **AR18** (important) `mutable` members of an exported namespace. **Resolution:** *decided by the reviewer:* every file reads them, only the declaring file writes them. **Status:** fixed.
- **AR19** (important) incomplete export forms. **Resolution:** M4 table; no `reactive`, `derived` or `static` exports; async entry. *Decided by the reviewer:* generic exports `export type(First, Second) = …`, `export function(Item) = …`. **Status:** fixed (## Export forms).
- **AR20** (important) test runner semantics. **Resolution:** sheet decisions (test files export nothing, async bodies, fresh module-level values, `@test` on `fa check`/`fa test`, stops). *Decided by the reviewer:* tests only in test files; duplicate test names are errors; tests of a file in order, files in parallel; `test only` fails under `CI` or on the Git default branch; `fa test` exits 0 or 1. **Status:** fixed (## How tests run).
- **AR21** (important) what scripts see. **Resolution:** *decided by the reviewer:* like a standalone file rooted at `scripts/`, plus `library` and the package's dependencies. **Status:** fixed.
- **AR22** (important) `use User = User`. **Resolution:** M1. **Status:** fixed.
- **AR23** (important) bare members in `when` in six fa-for pages. **Resolution:** C3. **Status:** fixed.
- **AR24** (important) config merge rules, entry function on `web`. **Resolution:** *decided by the reviewer:* workspace, package, target overrides, then defaults; objects merge deeply, arrays replace; on `web`, `arguments` is empty and the exit code ignored; exit codes: returned value or 0, 1 for an escaped error, 101 for a stop. **Status:** fixed.
- **AR25** (important) incomplete CLI. **Resolution:** *decided by the reviewer:* a full table (`fa run … -- arguments`, `fa new workspace [<name>]`, `fa format --check`, `fa build <file.fa>`, `fa preview` default target, `fa update [<dependency>...]`, `fa help`, `fa version`, exit codes). **Status:** fixed (`/architecture/overview#command-line`), reference updated.
- **AR26** (minor) "package root" wording in five fa-for pages. **Status:** fixed.
- **AR27** (minor) Go page: two tasks declaring `mutates channel`, "`start` is `go`", slicing row. **Resolution:** C9, A5, precise slicing row. **Status:** fixed.
- **AR28** (minor) `.=` missing from the reference, keyword list vs tokenizer, stale AGENTS paths, stale package READMEs. **Status:** AGENTS paths and reference fixed; tokenizer is a parser gap; READMEs still open.
- **AR29** (important) "no panic" statements in the Rust, Go, Java pages. **Resolution:** Stops. **Status:** fixed.
- **AR30** (minor) "like `Task.Run`" and thread wording. **Resolution:** A5. **Status:** fixed.
- **AR31** (minor) TypeScript page lacked the JS differences. **Resolution:** watch-outs for grapheme `length` and `Integer` stops beyond 2^53 − 1. **Status:** fixed.
- **AR32** (minor) Rust page "same fields, same type", C page "the division throws". **Resolution:** T5, T8. **Status:** fixed.
- **AR33** (minor) `README.md:75` grammar. **Status:** fixed.

- **AR34** (follow-up) prelude and `std`: `std.concurrency` exports listed, `std.time` (`sleep`), top-level `watch` allowed in package files, `.worker.fa` code can't use non-thread-safe module-level `mutable` values, detached `start` in the Go page, reference types in the Rust page. *Decided by the reviewer:* a worker file `MathWorker.worker.fa` is named `MathWorker`. **Status:** fixed.

#### Parser gaps

- `architecture/modules.md:160, 178, 314`: `export mutable`.
- `architecture/modules.md:197, 541`: `export async function`.
- `architecture/modules.md:227, 232`: `export type(First, Second)`, `export function(Item)`.
- `architecture/modules.md:269`: a `mutable` member inside an exported namespace.
- Precedence: `not` (F4), placement of `??`, `with`, `try`/`await`/`start` up to `catch`; chained comparisons stored as nested binary operations.
- Placeholder `...` bodies parse as dangling spreads in 16 blocks.
- Tokenizer: `yield` is a keyword, `parallel` is not.
- About 50 `panic!` instead of diagnostics; `parse()` prints "Analyzing...".
