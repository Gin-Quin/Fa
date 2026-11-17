# Audit of `fa check`: are its errors correct, complete and clear?

Auditor: Opus 5.5, 2026-11-17, on the trunk `main` at `5e3b602d` (read-only worktree
`audit-main`). Repro programs: `/home/claude-dev/fa-lead/reports/repros/diagnostics/`.
Status: **complete** (2026-11-17, 19:00 CET). Nothing in the repository was modified.

## Summary

- **Registry** (434 codes): 400 emitted, **34 without an emitter**, **9 emitted and tested
  nowhere**, 366 expected by an active case, **33 mentioned in the documentation** (no page
  lists the codes). The case harness checks codes and lines only: **no message, column or note
  is tested**, so the wording below has never been under test.
- **Corpus**: 249 one-mistake programs over every area of the language (thin: workers,
  foreign code, type functions, workspaces and dependencies; ownership has 4 programs). **64 cascade** (2–5
  diagnostics for one mistake), **6 wrong programs accepted**, **more than 40 programs get wrong
  advice**, 0 crashes. Harvest: 1,666 real messages of 343 codes from the analyzer's cases;
  67 codes have a message without a fix.
- **Per-code verdicts**: good 276, reword 103, wrong 12, missing 29, dead 5, unjudged 9 (§6).
- **The ten worst** (§4): F1 declarations from other languages (`const`, `fn`, `class`…) →
  3–5 errors and "declare `const` with `let`"; F2 `{ name: "Ada" }` → "not supported yet by
  the analyzer"; F3 `function add(a: Integer) {` → generic advice and `fa migrate-generics`;
  F4 fixes that write removed `Map(String, Integer)`; F5 `n % 2` → "write `(n)%`"; F6 unknown
  names: no "did you mean", "declare `null` with `let`"; F7 accepted: method value without a
  call, `Set.from(…, key =)`, `static x = f()` without `@`, `MEM004`, bad file name; F8 "remove
  the `catch`" after an undeclared `throws`; F9 the documented `catch error => 0` warns
  (language question); F10/F11 borrowed operators and syntax (`?:`, `as`, `&&`, `===`, guards,
  `try {}`) give cascades in the wrong order.
- **Rules** for the last pass: §5 (shape, the user's words, vocabulary, notes instead of line
  numbers, one error per mistake, other languages, one code scheme).
- **Question for the lead**: F9 (unused `catch` binding).

## Rubric

Every message is judged against seven criteria. A message passes a criterion or it doesn't;
the verdict of a code is the worst of its criteria (good / reword / wrong / missing).

1. **User terms.** Says what is wrong in the words of the documentation: no internal type
   names (`Integer<0..10>` is fine if the docs show it, `TypeId`, `Ty::Named`, `Slot`,
   `NodeKind`, `sink`, `lowering`, `M2` are not), no compiler jargon ("place", "lvalue",
   "effect row", "monomorphize") unless the documentation teaches the term.
2. **Names the construct and the types as written.** Quotes the name, the operator, the
   field, the type as the user wrote it (`Array<Int32>`, not an expanded or canonical form).
3. **Points at the right place.** The primary span covers the mistake (not the whole
   statement, not the enclosing function); a secondary span (`note:`) when another place
   explains the error: the earlier declaration, the move, the invalidating call, the
   conflicting use, the declared type.
4. **Says how to fix it.** A concrete rewrite in Fa syntax ("write `let mutable x`",
   "add `try`"), exact enough to apply mechanically where only one fix exists.
5. **One error per mistake.** A single mistake yields one diagnostic; no cascade of follow-on
   errors (unknown name → unknown type → wrong argument count…).
6. **Consistency.** The same term for the same thing across codes and with the
   documentation; the same message for the same mistake reached by different paths; one
   style (backticks around code, sentence case, no trailing period or always one).
7. **Correctness.** The diagnostic is emitted when, and only when, the program is wrong;
   the severity is right; a wrong program is never accepted silently and a right one never
   rejected.

## 1. The registry: emitters, tests, documentation

Method: `/tmp/diag/scripts/coverage.py` (copied to `repros/diagnostics/tools/`). For each of the
**434 codes** of `packages/analyzer/src/diagnostics/codes/` (409 errors, 25 warnings, 0 hints):
an *emitter* is a use of its static (or of its id as a string, for the CLI) in non-test Rust
source outside the registry; a *test* is an `-- error:`/`-- warning:` annotation naming the
code or an alias in a non-pending analyzer case, or (column "other") a mention in a CLI
integration test or a `#[cfg(test)]` module; *documented* is a mention in `documentation/`.

| | count |
| --- | --- |
| codes | 434 |
| with an emitter | 400 |
| **without an emitter** | **34** (inventory of 11-15 said 38: `CONC011`, `MEM003`, `TY071`, `GN030` gained one) |
| expected by an active analyzer case | 366 |
| tested only by CLI / unit tests | 30 (CLI, CFG, DEP, FFI012-014, TEST004…) |
| **emitted and tested nowhere** | **9**: `E-LOOP-13`, `E-WHEN-5`, `TY071` (only pending cases), `CLI002`, `CLI003`, `CLI005`, `CONC011`, `REF014`, `AN003` |
| pending analyzer cases | 110 |
| **mentioned anywhere in `documentation/`** | **33** codes (+ `REF017`, an alias) |

**Codes without an emitter (34).** Values: `E-CMP-10`, `E-ERR-5`, `E-OBJ-14`, `E-OPT-5`,
`E-STR-6`, `E-WHEN-4`, `E-WHEN-7`, `W-ERR-4`, `W-LEX-1`. Types: `CL023`, `CV003`, `EF020`,
`FN072`, `FN086`, `FN094`, `FN100`–`FN104` (partial application `with`: no checking at all),
`FN110`, `GN017`, `TF002`, `TY001`, `TY012`, `TY043`, `TY044`, `TY072`, `TY073`, `WFN002`.
Advanced: `CFG003`, `FFI007`, `MOD011`, `STR010`. `code-map.txt` explains seven of them
(reported under another code: `MOD011`→`REA005`/`META011`, `STR010`→`E-TYPE-1`, `MEM001`→`TY060`…);
the other 27 are either dead (a condition reported under another code: delete or alias them) or
a missing check. Each one is a row of the table in §6.

**Codes documented but absent:** none (every code named by the documentation is registered).
**Codes absent from the documentation:** 401 of 434. The documentation names codes only in
passing (33 of them, in 22 pages); there is no page that lists them, no `fa explain <code>`, and
the CLI does not print a link. An error code a user can't look up is only a search key.

**Structural findings about the registry and its tests** (they explain most of §2):

- **The test harness never checks a message, nor a column.** `tests/cases.rs` compares the
  code and the *line* of each diagnostic (`compare`, cases.rs:340-405); message text, span start
  and end, and `related` notes are never asserted outside a handful of unit tests. So the 366
  "tested" codes are tested for *being emitted on the right line*, not for *saying the right
  thing at the right place*. A message can regress to nonsense, or a span move to the wrong
  token on the same line, with the whole suite green.
- **Two naming schemes for codes**: `E-NUM-10` (values brief) and `TY040`/`REF001` (types and
  advanced briefs), plus 60+ aliases. Users see both side by side (`E-DECL-1` next to `MEM004`).
  The aliases only exist for the briefs' history.
- **Severity has no `Hint` code** although the renderer supports hints.
- **`fa check --format=json` drops the `related` notes** (report.rs:171-208): an agent reading
  JSON never sees "declared here", "moved here", "invalidated here". It also has no `fixes`
  field: the `Diagnostic` struct (diagnostics/mod.rs) has no way to carry a machine-applicable
  rewrite, so criterion 4 can only ever be met in prose.
- **A note in another file loses its position** in text mode (`render`, mod.rs:243-249: it
  prints `note: …` without the file and line), and notes never show the source line.
- **"Did you mean" exists only for configuration fields** (`configuration.rs:753 closest`):
  not for names, fields, methods, types, named arguments or members.

## 2. The messages (static read of every emitter)

Method: `tools/templates.py` extracts the string literals around every emitter of every code
(398 codes with an emitter site; `/tmp/diag/templates.txt`, copied to `tools/`), and I read
them all; §3 then checks them on real programs, and the harvest of the analyzer cases (§3)
checks the rendered text.

**What is good, and should be kept as the house style.** Most analyzer messages already follow
"what is wrong: how to fix it", name the construct in backticks, and give a Fa rewrite:
`` `{text}` is declared with `let`: declare it with `mutable` to assign it ``,
`` this `if` is used as a value, so it needs an `else`: add `else { … }` ``,
`` `try` is written before `await`: write `try await {inner}` ``,
`` there is no `String(value)`: write `value.toString()`, or a template like `"{value}"` ``,
`` `!` is not an operator: use `not` to negate a boolean, or `!=` for inequality ``,
the relation codes (REL001–REL021) and the reference codes (REF003–REF016). The habit-aware
lexer messages (`;`, `'…'`, `!`, `&&`, labeled `break`) are exactly what a newcomer needs.

**Systemic problems (each affects many codes):**

1. **Secondary spans are almost never used.** Only 4 emitters attach a `related` note
   (`E-DECL-2` "declared here", `E-ERR-9`, `E-OPT-3`/`E-OPT-6` "narrowed here") plus the
   parser's notes. The codes whose whole point is *another place* write it into the text as
   "(line N)" instead (15 sites): `REF001`/`REF002` ("was invalidated by `push` (line 3)"),
   `MEM002` ("was moved on line 4"), `REA002`, `REQ005`, `REF021`. A line number in the text
   is not clickable in an editor, is lost by `--format=json` consumers that want a range, has
   no column, and names no file when the place is in another file. Rule: **every "earlier
   place" is a `related` location with a short note, never a line number in the message.**
2. **No machine-applicable fixes.** The fix is always prose after the colon. Many are exact
   (`write `try await f()``, `write `{x}%``, `write `x.toString()``, `declare it with
   `mutable``, `write `Color.red``, `use `!=``) and could be a `fix: { span, replacement }`
   in the diagnostic and in the JSON, which an agent or the editor applies directly.
3. **Two casing styles.** Parser messages start with a capital letter ("Statements are
   separated by new lines, not `;`", "Expected `)` to close the `(`, found …", "A `static`
   value can't be exported"), analyzer messages with a lowercase letter. In one listing the
   user sees both. Rule: lowercase first word (the code and file prefix the message), no final
   period.
4. **Three ways to introduce an example**: "ex: `…`" (≈60 sites, an abbreviation English
   readers don't use), "like `…`" (≈25), "for example `…`" (a few). Rule: "for example `…`",
   or better, a direct imperative "write `…`" when the rewrite is exact.
5. **"cannot" and "can't"** both appear (`cannot compare a …` vs `can't be compared`). Rule:
   contractions everywhere (the majority).
6. **Compiler vocabulary leaks** where the documentation has a user term: "place"
   (`REF004` "`use` needs a place: `5` has no place to designate"), "temporary value"
   (`EF012`, `FN092`, `REF014`), "sink" (in registry summaries only), "facts"/"proven by the
   fact" (`W-NUM-1`, `W-NUM-2`), "module-level value" (the docs use it: fine),
   "a slice view" (`REF016`), "whose elements move" (`REL003`), "consumed" (`MEM002`). The
   documentation does teach "place" on the reference pages, but a user who wrote `use x = 5`
   hasn't read them; say "`use` names an existing value (a variable, a field, an element):
   `5` is not one. Write `let five = 5`".
7. **"is not supported yet by the analyzer"** (`AN001`, 36 sites): "the analyzer" is a
   component name; the user ran `fa check`. Write "`…` is not implemented yet in Fa
   <version>" and say what to write instead when there is a workaround (most sites have one).
   Two AN001 messages contradict the documentation: "recursive types are not supported yet by
   the analyzer" (the language reference says they are allowed, `MOD007` site) and "`Array`"
   as a value (`expr.rs:105`: `Array<Integer>.from([], capacity = 1000)` is documented).
8. **Types are shown through the analyzer's display**, which is the user's form for written
   types, but inferred integer types show as ranges the user never wrote
   (`Integer<min = 0>`, `Integer<min = 3, max = 3>`): fine for a range error, noise
   elsewhere (see §3).
9. **Same mistake, different words.** "is not declared: declare it with `let` or `mutable`
   first" (`E-DECL-1`, values) vs "`{text}` is not a type" (`TY007`) vs "has no member"
   (`E-OBJ-5`) vs "`{owner}` has no field `{text}`" (`EFF008`) vs "is not a field of"
   (`E-DST-3`, `E-OBJ-11`); the "for example" forms above; "a `{}` is never a `{}`" (`E-TYPE-1`
   for nominal types) vs "this value is not accepted where a `{expected}` is expected"
   (`E-TYPE-1`, generic) for the same mismatch.
10. **The `fa` command's own errors use three formats**: `error[CLI001]: …` (no position),
   `fa build: …` / `fa: can't read …` (no code), and `error: `{url}` is not a file` (`DEP010`,
   no code at all); none has the `file:line:col` and caret of the analyzer's diagnostics, even
   when the error is in `package.fa` or `dependencies.fa`.

## 3. Running it: the corpus

**226 programs** in `repros/diagnostics/` (one mistake each; the first line says which;
`out/<name>.txt` is what `fa check` printed, `out/<name>.json` the JSON; rerun with
`tools/run-corpus.sh`). Areas covered: values and declarations (`a*`, 32), operators (`b*`,
26), control flow (`c*`, 20), functions (`d*`, 30), types and generics (`e*`, 30),
collections (`f*`, 15), references and effects (`g*`, 10), ownership (`h*`, 3), errors (`i*`,
14), async (`j*`, 5), modules and packages (`k*`, 10 folders), compile-time evaluation,
tests and reactivity (`l*`, 11), habits of agents from Rust, Go, Python, TypeScript, C (`m*`,
20), and a second batch `n*` (23: concurrency, relations, foreign code, reactivity, generics,
packages and `package.fa`). **Thin or not covered:** workers and worker pools, channels,
owner fields, `fa bind` and C libraries, type functions and schemas, workspaces and
dependencies (`dependencies.fa`, lockfile), hot reloading, ownership (4 programs: the
enforcement is still landing on trunk). Those areas are exercised only through the harvest of
the analyzer's own cases below.

**Harvest of the analyzer's own cases.** `tools/harvest.py` ran `fa check --format=json` on the
805 single-file analyzer cases that expect a diagnostic: 1,666 real messages for **343 codes**
(`/tmp/diag/harvest-by-code.json`, samples in `tools/harvest-samples.txt`). Pattern counts over
those real messages: 20 codes start with a capital letter (the parser's), 36 use "ex:", 9 use
"cannot", 7 put a line number in the text, 6 show a literal as if it were a type ("a `"1"`"),
6 show an inferred range type the user never wrote (`Integer<min = 0>`), 2 use a placeholder
name instead of the user's, and **67 codes have at least one message with no fix at all**
(no colon: "`z` is not a field of `Point`", "this function takes 3 arguments",
"cannot compare a `String` with an `Integer`", "`Logger` is not a type").

**Counts on the first batch** (226 programs, 223 with a mistake; the whole corpus below):

| outcome | programs |
| --- | --- |
| exactly one diagnostic | 158 |
| **a cascade (2–5 diagnostics for one mistake)** | **59** (26 %) |
| **wrong program accepted** (no diagnostic) | **6**: `d18`/`d30` (method value without a call), `f05` (`Set.from(…, key = …)`), `h01`/`h02` (structure kept twice without `copy`, `MEM004`), `l07` (`static x = f(3)` without `@`), `k10` (file `my-utils.fa`) |
| crash | 0 (good: no panic, no timeout) |
| a message that gives **wrong advice** (applying it doesn't fix the program or makes it worse) | **more than 40**: the "declare it with `let`" of F1/F6 (≈28 programs), F3–F5, F8, F9, F12, `b18`, `c01`, `c08`, `m03`, `m04`, `m08`, `a32`, `g07` |


**Second batch** (23 programs, `n*`: concurrency, relations, foreign code, reactivity,
generics, configuration files and packages): the advanced areas are the best part of the
checker. `REL001` (`` `bestFriend` has no reverse side: declare one on the elements of `people`,
for example `bestFriendOf: Pool<people->bestFriend>` ``), `REL003`, `REA003`, `REA004`,
`REA010`, `REF011`, `CONC007`, `TY052`, `E-OBJ-13`, `E-DECL-1` for a `std` name (`` `Channel`
is a name of `std.concurrency`: bring it with `use { Channel } = std.concurrency` ``) and the
configuration's `` `target` is not a field of `PackageConfiguration`: did you mean `targets`? ``
are the model for the rest. Problems: `n01` reports `CONC001` **once per target** (`bun`, then
`web`, same span) plus `CONC002`; `n08` `first<String>([1, 2])` gives one `E-TYPE-1` per
element; `n17` a variadic parameter not last gives `FN028` + `W-DECL-2` + `FN024`; `n23` a
script-style `application/main.fa` gives `MOD005` ("add `export` to the main declaration of
`main`": there is none), `MOD015` with its caret on the first comment line, and `MOD006`;
`n13` `Color.red + 1` → `` `+` doesn't apply to a `Color.red` `` (the narrowed member shown as
the type; say `` enumerations have no arithmetic: compare `Integer(Color.red)` ``); `n14`
`text[0] = "H"` → "assign the owner instead"; `n07` `GN015` suggests `` `f<ItemType>(…)` `` (the
function is `empty`); `n22` `version = 1` → no fix. Counts for the whole corpus (249 programs,
246 with a mistake): **64 cascade**, 6 wrong programs accepted, 0 crashes.
## 4. Findings, ranked by how much they hurt a user

Each finding: the programs (in `repros/diagnostics/`), what is printed today (shortened), and
the proposed message with its span. "One error" means: emit this diagnostic and suppress
every follow-on diagnostic in the same statement and every "not declared" for the names the
broken statement was meant to declare (declare them with an unknown type instead).

### F1. A declaration written in another language gives 3–5 errors and the wrong fix (critical)

`a02` `const limit = 10`, `a03` `var`, `d06`/`j03` `const f = (…) => …`, `d02` `fn add(…) -> i32 {`,
`d03` `func`, `d04` `def`, `e01` `interface User {`, `e02` `class`, `e13` `enum Color {`,
`m01` `struct`/`impl`, `m20` `pub function`, `e06` `new User {…}`, `c06` `match n {`.
Today (a02):

```text
2:7  error[AN002]: Unexpected `limit` after the end of the statement: a statement ends at the end of its line. Add an operator, or start a new line
2:1  error[E-DECL-1]: `const` is not declared: declare it with `let` or `mutable` first
3:13 error[E-DECL-1]: `limit` is not declared: declare it with `let` or `mutable` first
```

Three errors, the first one points at the wrong token, and the advice "declare `const` with
`let`" is absurd. This is the first thing a TypeScript developer or an agent writes.
Proposed: one error, span on the foreign keyword, a new code (say `E-LEX-7`, "a keyword of
another language"), the rest of the statement parsed as the Fa form so that the declared name
exists:

| written | message |
| --- | --- |
| `const x = …`, `var x = …` | `` Fa has no `const`: write `let limit = 10` (immutable) or `mutable limit = 10` `` |
| `let mut x` | `` Fa writes `mutable count = 0`, not `let mut` `` |
| `fn`, `func`, `def`, `function f(…) {` | `` a function is declared `function add = (a: Integer, b: Integer): Integer => …` `` |
| `const f = (…) => …` | `` a function is declared with `function`: write `function add = (a: Integer, b: Integer) => a + b` `` |
| `class`, `interface`, `struct` | `` Fa declares types with `type`: write `type User = { name: String }` `` |
| `enum` | `` write `type Color = Enum { red, green, blue }` `` |
| `impl` | `` methods are declared inside the type: `type Point = { …, function sum = (self): Integer => … }` `` |
| `pub`, `public`, `export default`, `export const` | `` a file exports one value named after the file: write `export function = …` `` (see F12) |
| `new User { … }` | `` Fa has no `new`: write `User { name = "Ada" }` `` |
| `match`, `switch` | `` write `when n is { 1 => … else => … }` `` |
| `import … from`, `require(…)` | `` Fa has no imports: the sibling file `double.fa` is already visible as `double`; `use { a } = b` makes an alias `` |

### F2. `{ name: "Ada" }` is "not supported yet by the analyzer" (critical)

`e03` `let ada: User = { name: "Ada", age: 36 }` → `error[AN001]: this object literal is not
supported yet by the analyzer`. The most common JavaScript habit gets an "unsupported" error
that suggests waiting for a new version. Same family: `d11` `greet("Ada", greeting: "Hi")` and
`c04` `if n > 2:` → `AN001: a field declaration outside an object is not supported yet by the
analyzer`; `f11` `{ "Ana": 30 }` for a map → `AN002: Expected a member…`.
Proposed (span: the first `:`): `` the fields of an object are given with `=` (`:` gives a type):
write `{ name = "Ada", age = 36 }` ``; for a named argument
`` a named argument is written with `=`: `greeting = "Hi"` ``; for a block opened by `:`
`` a block is written between braces: `if n > 2 { … }` ``; for a map literal
`` a `Map` is written as a list of pairs: `[("Ana", 30)]` ``.

### F3. `function add(a: Integer, b: Integer) { … }` is read as old generics (critical)

`d01` → `error[GN031]: The generic parameters of a function belong to its value: write
`function add = <a: Integer, b: Integer>(…) => …`. Run `fa migrate-generics --write <path>`…`
plus `E-LEX-3 Expected a member…` plus `AN002`. The fix turns the parameters into
compile-time generic parameters and sends the user to a migration tool. Proposed (one error,
span `add(`): `` a function is declared `function add = (a: Integer, b: Integer): Integer => …`:
add `=` after the name, and `=>` before the body ``. GN031 should only fire when the list
holds capitalized names or the program used the old syntax elsewhere.

### F4. Fixes that suggest the removed parenthesized generics (high: the fix is an error)

`e16` `Map<String>` → `TY003: `Map` takes the type of its keys and of its values, ex:
`Map(String, Integer)``; harvest: `` `GrowingPool` takes the type of its items, ex:
`GrowingPool(Integer)` ``, `E-NUM-13` `` ex: `{storage}(min = 0, max = 10)` ``, `TY003`
`` `Type` takes one value, ex: `Type(user)` ``. Applying them gives `GN031`. Proposed:
`` `Map` takes two type arguments, the keys and the values: write `Map<String, Integer>` ``.
A grep for `(` inside backticked type examples in `check/` finds every site.

### F5. `%` for the remainder is told to become a percentage (high: wrong advice)

`b04` `n % 2` → `E-NUM-5: `%` only applies to a number literal or to an expression in
parentheses: write `(n)%``. The user wanted `%%`. Proposed (span `%`): when `%` is followed by
an operand on the same line, `` `%` is the percentage suffix; the remainder is `%%`: write
`n %% 2` ``; keep today's message for `fifty%`.

### F6. Unknown names: wrong advice, no "did you mean" (high)

`E-DECL-1` always says `` `X` is not declared: declare it with `let` or `mutable` first ``:
`a13` typo `cuont`, `d21` `print(…)`, `d24` `len(items)`, `c03` `range(3)`, `a17` `null`,
`a18` `undefined`, `a20` `None`, `m07`/`i11` `nil`, `m05` `True`, `m02` `Some(…)`,
`a25` `Number(x)`, `j02` `Promise.all`, `d20` `this`, `d19` a field without `self.`,
`k08` `require`. Telling the user to declare `print` or `null` is wrong. The same gap for
types (`TY007 `number` is not a type`, `string`, `i32`, `any`, `Option`, `Usr`), fields
(`E-OBJ-5 a `User` has no field or method `nmae``), members (`k07`), named arguments
(`d10`), and compile-time operations (`l03` `@readFil`). Proposed:
1. edit distance ≤ 2 (the `closest` of `configuration.rs`, moved to a shared module) over the
   names in scope, the fields, the members, the parameters: `` `cuont` is not declared: did
   you mean `count`? `` with a related note `count is declared here`;
2. a table of the other languages' names, one line each:
   `null`/`undefined`/`nil`/`None` → `` Fa has one absent value: write `none` ``;
   `True`/`False` → `` write `true` ``; `print`/`println!`/`fmt.Println` →
   `` write `console.log(…)` ``; `len(x)` → `` write `items.length` (`size` for sets and maps) ``;
   `range(n)` → `` write `0..n` ``; `Number(x)`/`parseInt` → `` write `Float(count)` or
   `Integer.parse(text)` ``; `Promise` → `` write `Task.all([…])` ``; `this` → `` write `self` ``;
   `Some(x)` → `` an optional holds the value itself: write `"Ada"` ``;
   `number` → `` write `Integer` or `Float` ``; `string`/`boolean` → `` write `String`/`Boolean` ``;
   `i32`/`u8`/`int` → `` write `Int32` (or `Integer`) ``; `any` → `` Fa has no `any`: use a union
   or a generic parameter ``; `Option<T>` → `` write `T?` ``;
3. a bare name that is a field of the enclosing type, in a method: `` `value` is a field of
   `Counter`: write `self.value` (and add `self` as the first parameter) ``;
4. for an assignment to an undeclared name (`a04`): `` `total` is not declared: write
   `mutable total = 3` to declare it ``, and no second error on the next read.

### F7. Accepted wrong programs (high: silent)

- `d18`/`d30` `let read = counter.get` (a method without a call): accepted, then
  `E-STR-12` at the print. The reference says it is an error; `FN086` is registered without an
  emitter. Proposed (span `counter.get`): `` a method is called: write `counter.get()`, or a
  lambda `() => counter.get()` ``.
- `f05` `Set.from([…], key = (user) => user.id)`: accepted; the reference says `key =` is
  `FN023`. Proposed: `` `Set.from` has no option `key`: a `Set` compares whole items; to key by a
  field, use a `Map<Integer, User>` ``.
- `h01`/`h02` `let b = a` then a use of `a` (an `Array`): accepted; memory-management.md says
  `MEM004`. Probably the ownership enforcement still landing (`reports/ownership-2-*`);
  listed so that the last pass checks it.
- `l07` `static value = double(3)` without `@`: accepted; the reference says `@` is written on
  every compile-time call. Proposed: `` a function called at compile time is written with `@`:
  `@double(3)` ``.
- `k10` a sibling file `my-utils.fa` next to a standalone program: no `MOD001`. Proposed:
  `` `my-utils` is not a valid name: a file is named like an identifier, rename it
  `myUtils.fa` `` (span: the file, line 1).

### F8. Follow-on warnings that tell the user to make it worse (high)

`g04`, `i05`, `i12`: after `E-ERR-2`/`E-ERR-4`/`E-ERR-3` in a callee whose block body doesn't
declare its errors, every caller's `catch` gets `W-ERR-2 this expression can't fail: remove the
`catch``. Removing it is wrong once the callee is fixed. `i09` `throws String` gives
`E-ERR-10 declares `throws String` but never propagates an error: remove it` (it does throw) plus
`E-ERR-1`. Proposed: a callee with an error-declaration diagnostic is treated as declaring what
its body throws (no `W-ERR-2` at its callers); `throws String` gets `EF020` (registered, no
emitter): `` `String` is not an error type: declare one, `type CheckError = Errors { Negative }`,
and write `throws CheckError` ``, and no `E-ERR-10`.

### F9. The documented idiom `catch error => 0` warns, and the fix is an error (high; language question)

`i14` `let score = loadScore("3") catch error => 0` (copied from
fa-for-typescript-developers.md:87; the same form is all over the documentation) →
`W-DECL-2: the error `error` is never used: remove it, or start its name with `_``. "Remove it"
is `E-ERR-9` (a `catch` names its error), and the span is the whole expression, not the name.
**Question for the lead**: (a) don't warn on an unused `catch` binding (recommended: a catch
that gives a fallback is the common case, and the documentation uses `error` everywhere);
(b) allow `catch => 0` without a name; (c) keep the warning and rewrite the documentation with
`_error`. With (c), the message must say `` start it with `_`: `catch _error => 0` `` and point at
`error`.

### F10. Borrowed operators and syntax: cascades, nonsense, or "not supported yet" (high)

| program | today | proposed (one error) |
| --- | --- | --- |
| `b06` `c ? "even" : "odd"` | `AN002 Unexpected a string…` + `AN001 type expressions in value position is not supported yet by the analyzer` | `` Fa has no `? :`: write `if n %% 2 == 0 { "even" } else { "odd" }` `` |
| `b25` `2 in items` | `AN001 type expressions in value position is not supported yet…` | `` membership is written `2 is in items` `` |
| `i06` `check(1)?` | same AN001 | `` Fa has no postfix `?`: write `try check(1)` `` |
| `b19` `count as Float` | `AN002 Unexpected `as` after the end of the statement…` | `` `as` never converts: write `Float(count)` `` |
| `b18` `name!.length` | `E-LEX-4 … use `not` …, or `!=`` | `` Fa has no `!` assertion: narrow first (`if name is not none`), or write `name?.length` `` |
| `b02` `a && b` | 3 errors (`&` twice, then `Expected , or )`) | `` write `a and b` `` |
| `b03` `name \|\| "nobody"` | 2 errors, no fix | `` write `name ?? "nobody"` for a default, `a or b` for booleans `` |
| `b05` `===`, `b11` `!==` | 2 errors each, no fix | `` write `==` `` / `` write `!=` `` |
| `b20` `n ^ 3` | "use the methods of integers…" | `` write `n ** 3` for a power, `n.bitXor(3)` for a bitwise xor `` |
| `a19` `count++` | 2 errors, the second on the next line | `` write `count += 1` `` |
| `a07` `count := 0`, `m14` `:= range` | 2–3 errors | `` write `mutable count = 0` `` / `` write `for x, i in items` `` |
| `m08` `if n = 3` | `E-DECL-3 declare `n` with `mutable`` + `E-BOOL-1 … found a `Void`` | `` `=` assigns: compare with `==`, `if n == 3` `` |
| `c01` `for (let i = 0; i < 3; i++)` | `E-LEX-1 Statements are separated by new lines, not `;`` | `` write `for i in 0..3 { … }` `` |
| `c02` `for (const item of items)` | `Expected `)` to close the `(`, found `item`` | `` write `for item in items { … }` `` |
| `c04` `if n > 2:` | `AN001 a field declaration…` + error at end of file | `` a block is written between braces: `if n > 2 { … }` `` |
| `c14` `elif` | `Unexpected `elif`…` | `` write `else if` `` |
| `c08` `_ => …` in `when` | `TY007 `_` is not a type` | `` the fallback branch is `else => …` `` |
| `c12` `x if x > 2 => …` | **5 errors**, one quoting two lines of code inside the message | `` `when` has no guards: match `else`, then test inside the branch with `if` `` (`E-WHEN-4`, registered, no emitter) |
| `a11` `// comment` | `E-DECL-1 `the` is not declared` | `` comments start with `--` (`//` is floor division) `` |
| `m19` `/* … */` | 3 errors | `` block comments are written `--- … ---` `` |
| `m03` `vec![…]`, `m04` `println!(…)` | `!` is not an operator | `` Fa has no macros: write `[1, 2, 3]` `` / `` write `console.log("Hello {name}")` `` |
| `m06` `f"…"` | `Expected , or )` | `` every string interpolates: remove the `f` `` |
| `e09` `Integer[]`, `m17` `readonly Integer[]` | `Expected an expression after `[`` | `` write `Array<Integer>` `` |
| `e12` `first::<Integer>(…)` | `Expected an expression after `:`` | `` write `first<Integer>(…)` `` |
| `e27` `name?: String`, `m12` `(name?: String)` | `AN002`; `AN001 this kind of parameter…` + `E-DECL-1` | `` an optional field is written `name: String?` `` / `` an optional parameter has a default: `name: String? = none` `` |
| `m09` `;` between the fields of a type | `Statements are separated…` + `E-OBJ-1` on a later line | `` fields are separated by commas or new lines, not `;` `` (and keep the field) |
| `m16` `(): Point => { x = 0, y = 0 }` | 3 errors, one says "a tuple can't hold it" | `` after `=>`, braces open a block: write `=> Point { x = 0, y = 0 }` `` (`E-OBJ-14`, registered, no emitter) |
| `i01` `throw new Error("…")` | `Unexpected `Error`…` | `` throw a member of an `Errors` type: `throw CheckError.Negative` `` |
| `i02` `try { … } catch (e) { … }` | 4 errors, the first one on line 5 and E-ERR-6 "on the same expression" | `` `try` is not a block: mark each call, `try check(1)`, or handle it with `check(1) catch error => …` `` |

### F11. Diagnostics are not printed in source order, and parser cascades are not cut (high)

`b08` (`if a<b {`) prints first `5:9 Expected `=`, `:`, `,` or a new line after the member
`console`, found `.`` and only second the right message `4:5 `<b` opens a generic list that is
never closed: … write `a < b``; `i02`, `c04` likewise (an error at the end of the file before the
one on line 3). `check/mod.rs:730` sorts the checker's diagnostics and appends them after the
parser's, which stay in emission order (`render_all` prints them as given): the parser's
recovery errors come first, out of order. Proposed: sort every file's diagnostics by
position before rendering and in the JSON; after a parser error, drop the other parser
errors of the same statement and the analyzer errors inside its span; never quote source text
that contains a line break in a message (`c12`).

### F12. Module mistakes give absurd fixes (high)

`k02` `export const double = …` → `AN002 An export has no name: … Write `export = const``,
`MOD005 each file exports one entity: add `export` to the main declaration of `double`` (there is
an `export`), and `E-DECL-1 `double.fa` exports nothing` in `main.fa`: 3 errors in 2 files.
`k03` `export default function double(…)` → `Write `export = default``. `k04` two `export
function name =` → two `Expected `=` after `export`, found `double``. `k06` writing a sibling's
`export = 0` → `` `counter` is declared with `let` `` (it is not). Proposed: `` an export is
named after its file: write `export function = (n: Integer): Integer => n * 2` `` (span
`const double`); for two exports `` a file has one export, named after the file: export a
namespace, `export namespace = { function double = …, function triple = … }` ``; for `k06`
`` `counter` is exported by `counter.fa` without `mutable`: write `export mutable = 0` there ``
with a related note at the export.

### F13. The other place is a line number in the text, and JSON loses notes (medium-high)

`g01` `` `first` was invalidated by `items.push` (line 4): declare it again with `use first =
items[0]` `` — the message is good, but "(line 4)" should be a related span (`note: `items.push`
invalidates `first` here`), and so for `REF002`, `MEM002` ("was moved on line 4"), `REA002`,
`REQ005`, `REF021`, `REF013`. `--format=json` prints no `related` field at all
(report.rs:171). A note in another file prints no position (mod.rs:243). Proposed: a
`related: [{file, range, message}]` array in the JSON; `note:` lines with `file:line:col` and
the source line with a caret, as the primary span has.

### F14. "Not supported yet" for things that will never exist (medium)

`c18` `items.forEach(…)` → `AN001 `forEach` of arrays is not supported yet by the analyzer`;
`f13` `items.slice(1, 2)` → same; `k09` `std.fs.readFileSync` → `` `std.fs.readFileSync` is not
supported yet by the analyzer ``; `l03` `@readFil` → `` can't be evaluated at compile time yet ``.
The user waits for a release instead of rewriting. Proposed: the Fa form: `` arrays have no
`forEach`: write `for item in items { … }` ``; `` write `items[1..2]` for a slice ``;
`` `std.fs` is not a module of the standard library: read a file with `File.readText(path)` ``;
`` `@readFil` is not a compile-time operation: did you mean `@readFile`? ``. And `AN001`'s
wording everywhere: "not implemented yet" without "by the analyzer".

### F15. Types shown in the compiler's form, not the user's (medium)

- A literal shown as a type with an article: `` a `"1"` is not accepted where an `Integer` is
  expected `` (`d09`, `d13`, `c15`, `f07`, `e28`, `a31`), `` found a `3`: use `"…{3}"` ``
  (`b23`), `` not a `3` `` (`j04`). Proposed: `` `"1"` is a `String`, but `add` takes an
  `Integer` for `a` `` with a related note at the parameter `a: Integer`.
- An inferred range: `d25` `items.length()` → `` an `Integer<min = 0>` is not a function: it
  can't be called `` → proposed `` `length` is a property: write `items.length` ``;
  `E-NUM-14` `` `counter` may be -9223372036854775808 to 9223372036854775807, which doesn't fit in a
  `UInt8` `` → `` `counter` is an `Integer`, which doesn't fit in a `UInt8` (0 to 255) ``;
  `W-NUM-2` `(20..=9223372036854775807 < 20)`.
- Built-in errors doubled: `` can fail with `ParseError.ParseError | HttpError.HttpError` ``
  (`E-ERR-3`), `` members are always qualified: write `ParseError.ParseError` `` (`TY057`).
  Proposed: a built-in error is shown and written `ParseError`.
- `e15` `` a `"pending"` is not accepted where a `"done" | "ready"` is expected `` (the union
  reordered, its name `Status` lost) → `` `"pending"` is not a `Status` (`"ready" | "done"`) ``.
- `(Unknown) => Void` in `E-STR-12`; the anonymous configuration type
  `{ bits: Integer, max: BigInt?, … }` in `E-OBJ-1`; `ParseError { message = "…" }` in
  `META005` (fine, but say "throws `ParseError`: malformed JSON at line 1").
- The same range written two ways: `(-128 to 127)` (`E-NUM-6`) and `(-128..=127)` (`E-NUM-13`).

### F16. Fixes with placeholder names instead of the user's (medium)

`b13` `if name {` → `` test it, ex: `value is not none` `` (proposed: `` write `if name is not
none` ``); `b14`/`c11`/`l09` `` ex: `value > 0` `` (→ `` `count > 0` ``); `a32` `a + n` (a
`BigInt` and an `Integer`) → `` convert with `BigInt(i)`, or `Integer(b)` `` (the user's names
are `n` and `a`: → `` write `a + BigInt(n)`, or `Integer(a) + n` ``); `FN024` `missing
argument: #2` (→ `` `add` needs its second argument `b: Integer` ``); `FN025` `this function takes
2 arguments` (→ `` `add` takes 2 arguments, `(a: Integer, b: Integer)`: remove `3` ``, span the
extra arguments); `g07` at the top level `declare `requires …` in the enclosing function` (the
top level can't declare requirements: → `` provide it: `provide Logger = …` ``).

### F17. A bogus diagnostic inside a string (medium)

`l06` `static configuration = @parseJson("{ port: 80 }")` → `META005 … throws ParseError …`
(right) **and** `AN001 a field declaration outside an object is not supported yet` with its span
*inside the string literal* (2:38). Something parses the JSON text as Fa. Proposed: only the
`META005`, worded `` this JSON is malformed (line 1: unexpected character): JSON keys are
quoted, `{ "port": 80 }` ``.

### F18. Same mistake, two codes, or two errors (medium)

- `f04` adding to a collection inside its own `for`: `REF012` and `REF013` on the same span;
  keep `REF012`.
- `c17` index binding on a `Set`: `CL032`, while `E-LOOP-3` is registered for the same
  condition (and emitted for ranges): one code.
- `d12` a block body without return type whose value is used: `WFN001` + `E-TYPE-1 a `Void`
  is not accepted…`: keep one error at the call, with the fix `declare its return type,
  `(n: Integer): Integer => {``.
- `l05` a compile-time call that divides by zero: `E-NUM-12` in the function and `META005` at the
  call; acceptable (two places), but `META005` should be a related note of the first.
- `l03` `@readFil`: `E-DECL-1` + `AN001` (F14).
- Lexer messages without a code: `` `&` is not an operator `` (also `^`, `~`) is reported as
  `AN002` (`tokenize.rs:395`, code `None`), while its neighbours `;` (`E-LEX-1`, whose
  registry summary is just "in source"), quotes (`E-STR-13`) and `!` (`E-LEX-4`) have codes.
  `d01` gets `E-LEX-3` ("a keyword used as a name") for a `return` inside what the parser
  took for an object: right by its rule, but a symptom of F3.

### F19. Messages that are right but miss the fix (medium; mechanical)

67 codes have a message without a fix (see §3). The ones a newcomer meets first:
`E-CMP-1 cannot compare a `String` with an `Integer`` (→ `` convert one: `name == "7"` or
`Integer.parse(name)` ``); `E-OBJ-1 `age` is not a field of `User`` (→ add the closest field,
or `` remove it, or add `age: Integer` to `User` `` with a note at `User`); `E-OBJ-2 missing field
of `User`: `age`` (→ `` add `age = …` ``, note at the field declaration); `E-WHEN-1 this `when`
doesn't handle `Color.blue`` (→ `` add `Color.blue => …`, or `else => …` ``); `TY007` (F6);
`E-OBJ-5` (F6); `FN025`/`FN024` (F16); `E-TYPE-1` (F15); `a23` `let half: Integer = 10 / 4` → `a
`Float` is not accepted where an `Integer` is expected` (→ `` `/` always gives a `Float`: write
`10 // 4` for an integer division ``); `e21` `Circle { … }` for a union member → `` `Circle` is not a
type `` (→ `TY055`: `` write `Shape.Circle { radius = 1.0 }` ``); `f09` `.take(2)` on an iterator
(→ `` iterators have no `take`: write `firstValues(2)` ``); `f12` `ages.set(k, v)`
(→ `` write `ages["Ana"] = 30` ``); `m10` `name.len()` → today a generic tail about `+=` and
`reserve` (→ `` write `name.length` ``); `f10` `REF010` mentions "links are not implemented yet"
(→ `` a local collection can't hold references: store indices, or declare a relation ``).

### F20. The checker's own tests can't catch any of this (structural)

The case harness checks codes and lines only (§1). Proposed: an `-- message:` annotation (a
substring of the message) on new cases, and a snapshot test that runs `fa check` (text and
JSON) on a corpus like this one and compares with checked-in `.expected` files
(`FA_UPDATE_DIAGNOSTICS=1` to rewrite), so that wording, spans, notes, order and cascades are
reviewed in diffs. This corpus can be its seed.

### F21. Smaller ones

- Casing, "ex:", "cannot" (§2, items 3–5); `WCL002` starts with "bad practice:" (tone).
- `W-DECL-2` on a `catch` points at the whole expression instead of the name (`i14`).
- `E-NUM-5` "`(fifty)%`" is fine; `E-NUM-11` appends `in `count * ratio`` after the fix, which
  repeats the underlined text.
- `E-STR-7` stacks two colons: `` `+` joins strings: expected a `String`, found an `Integer`: use … ``.
- `TY003`/`GN007` disagree on the form of "takes N type arguments".
- `AN001` "recursive types are not supported yet by the analyzer" (`MOD007` site) contradicts
  the language reference, which allows recursion through an optional or a collection.
- Per-target duplicates (`n01`): a diagnostic found for several targets is printed once per
  target with `(for the target `web`)`; merge them: `` … only available on `native`, not on
  `bun` and `web` (the targets of this program) ``.
- `n23`: a package entry file without an export gets three errors; one is enough: `` the entry
  file exports the function the program starts with: wrap these statements in
  `export function = () => { … }` `` (span: the first statement, not the comment on line 1).
- Documentation: fa-for-typescript-developers.md's heading "Generics use parentheses" sits over a
  paragraph about angle brackets.

## 5. Terminology and style rules (proposed for the last pass)

1. **Shape.** `<what is wrong>: <how to fix it>`. Lowercase first word, no final period,
   contractions (`can't`, never `cannot`). Every error has a fix part; a fix that is exact is
   an imperative with the code (`` write `items.length` ``) and, in the `Diagnostic`, a
   machine-applicable `fix { span, replacement }` exported in the JSON; an illustrative one
   says "for example `…`" (never "ex:", never "like" for code).
2. **The user's words.** Names, operators, fields and types as the user wrote them; the
   user's own names in the fix (`name is not none`, never `value is not none`); `…` for what the
   message can't know. A type alias by its name first (`Status`, then `("ready" | "done")` if it
   helps); a literal as a value ("`"1"` is a `String`"), never as a type ("a `"1"`"); a built-in
   error by its name (`ParseError`); an inferred integer range only in a range error, in one
   format (`0 to 255`); never `Unknown`, `Ty…`, `Integer<min = 0>` the user didn't write.
3. **Vocabulary** (the documentation's): *value* (declared with `let`/`mutable`), *reference*
   (what `use` declares; *alias* only when explaining `use`), *element* (of a collection; not
   *item*), *field* (of an object), *member* (of a union, an enumeration, a namespace),
   *method* (a function declared in a type; not "the functions of `User`"), *error* (thrown;
   "can fail with `E`"), *optional* and `none` (never null), *effect* (`throws`, `requires`,
   `mutates`, `invalidates`). Never *place* (say "a variable, a field or an element"),
   *temporary* (say "a value computed here, not stored"), *sink*, *facts*, *consumed*,
   *slice view* (say "a reference to a slice"), "the analyzer" (say nothing, or "Fa").
4. **Other places are notes.** Every earlier or other place (declaration, move, invalidating
   call, conflicting use, the declared type or parameter, the export in another file) is a
   `related` location with a short note, rendered `note: … (file:line:col)` with its source
   line, and present in the JSON. Never "(line N)" in the message.
5. **One error per mistake.** After a syntax error, nothing else in the same statement; a
   name the broken statement meant to declare is declared (unknown type); no warning that
   depends on a failed check (`W-ERR-2` after an undeclared `throws`); diagnostics sorted by
   position.
6. **Other languages.** A construct of another language that Fa recognizes is named once:
   "Fa has no `X`: write `Y`" (the F1/F6/F10 tables), never "not declared" or "not supported
   yet". "Not implemented yet" is reserved for documented Fa that the compiler doesn't handle,
   and always comes with the workaround.
7. **Codes.** One scheme for users (recommendation: keep the area prefixes `E-…`/`W-…` and
   turn the `TY`/`FN`/`REF`… codes into aliases, or the reverse, but print one); a code per
   parser error class instead of `AN002`; every code on one documentation page with an example
   and the fix, reachable as `fa help <code>`.

## 6. Per-code table

Verdicts: **good** (no problem found against the rubric; the systemic rules of §5 still apply),
**reword** (right condition, message to change), **wrong** (wrong condition, wrong advice, or a
wrong program accepted), **missing** (registered, no emitter, or a condition that should have
it), **dead** (registered, reported under another code: alias or delete), **unjudged** (message
built outside the analyzer and not read). Columns: *tested* = active analyzer cases + `p`
pending cases + `o` mentions in CLI/unit tests; *message today* = the first message harvested
from the analyzer's cases, or `(source)` = the template read in the source when no case emits
it. Also as TSV: `repros/diagnostics/tools/per-code.tsv`.

Totals: 434 codes — **good 276, reword 103, wrong 12, missing 29, dead 5, unjudged 9**.

| code | sev | emitted | tested | documented | verdict | message today (sample) | what to do |
| --- | --- | --- | --- | --- | --- | --- | --- |
| E-BOOL-1 | E | yes | 6+1p+6o | — | **reword** | expected a `Boolean`, found an `Integer`: compare it, ex: `value > 0` | F16: placeholder `value` in the fix instead of the user's name; cascade after `if n = 3` |
| E-CMP-1 | E | yes | 4 | — | **reword** | cannot compare a `Float32` with a `Float` | F19: no fix; "cannot"; style: "cannot" |
| E-CMP-2 | E | yes | 9+2o | — | good | an `Item` has no order: `<`, `<=`, `>` and `>=` compare numbers and strings | style: "cannot" |
| E-CMP-3 | E | yes | 2 | — | good | a comparison chain goes in one direction: use `<`/`<=` or `>`/`>=`, or split it with `and` |  |
| E-CMP-4 | E | yes | 3+2o | — | **reword** | functions can't be compared with `==` | F19: a message has no fix: add one |
| E-CMP-5 | E | yes | 3+1p | — | good | `range` is a value, not a pattern: test the membership with `is in range` |  |
| E-CMP-6 | E | yes | 3 | — | good | `is in` takes an array, a set, a map or a range, not an `Integer`: compare with `==`, or test a pattern with ` |  |
| E-CMP-7 | E | yes | 2+1p | — | good | `n is String` is never true: an `Integer` is never a `String` |  |
| E-CMP-10 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| E-DECL-1 | E | yes | 17+4o | — | **wrong** | `Semaphore` is not a member of `std.concurrency`: it has `AtomicInteger`, `AtomicBoolean`, `Mutex`, `Channel`, | F1/F6: advises `let`/`mutable` for `null`, `print`, `const`, typos; add did-you-mean and the foreign-name table; no second error on the next read; style: "ex:" |
| E-DECL-2 | E | yes | 5+4o | — | good | `a` is already declared in this scope: give it another name |  |
| E-DECL-3 | E | yes | 10+1p+1o | — | **reword** | a string has no item assignment: build a new string and assign the owner instead | F10/F12: `if n = 3` gets "declare it with `mutable`"; a sibling file's `export =` is called "declared with `let`" |
| E-DECL-4 | E | yes | 8+2o | — | good | `frozen` is immutable: declare it with `mutable` to change its fields or elements |  |
| E-DECL-6 | E | yes | 2+5o | — | good | a function is declared with `function`: write `function addOne = …` |  |
| E-DECL-8 | E | yes | 1+1o | — | good | Assignments can't be chained, like `a = b = c`: write one assignment per line | style: capitalized |
| E-DECL-9 | E | yes | 2+5o | — | **reword** | this expression has no value (`Void`): `result` can't hold it | F10: "a tuple can't hold it" for `=> { x = 0, y = 0 }` (m16); style: "ex:" |
| E-DECL-10 | E | yes | 1 | — | good | `x` has no value: give it one, ex: `mutable x = …` | style: "ex:" |
| E-DECL-11 | E | yes | 3+2o | — | good | `Json.parse` reads the type you expect: write it, ex: `let user: User = Json.parse(text)` | style: "ex:" |
| E-DECL-15 | E | yes | 1 | — | good | assigning `x` can't fail: put `try` on the value instead, `x = try …` |  |
| E-DO-1 | E | yes | 2 | — | good | this block has no value: it ends with a statement, so end it with the value it gives |  |
| E-DO-2 | E | yes | 1+4o | — | good | `return` is only allowed in a function: at the top level, the program ends with its last statement |  |
| E-DST-1 | E | yes | 1 | — | good | `mutable use` can't destructure: write one `mutable use` per field, or `use { a, b } = place` |  |
| E-DST-2 | E | yes | 3 | — | **reword** | an object pattern `{ a, b }` can't destructure an `Integer` | F19: a message has no fix: add one |
| E-DST-3 | E | yes | 2 | — | good | `w` is not a field of `Point`: give it a default, ex: `w = …` | style: "ex:" |
| E-DST-4 | E | yes | 1 | — | good | a `use` pattern designates fields of a place: a default value is not a place, use `let` |  |
| E-DST-5 | E | yes | 3 | — | good | this value has 2 elements (`Tuple<Integer, Integer>`), the pattern 3: bind every element |  |
| E-DST-6 | E | yes | 2+2o | — | good | The items of an array or tuple pattern have no default value: check the length first, or use an object pattern | style: capitalized |
| E-DST-7 | E | yes | 2 | — | good | `r` is bound twice by this pattern: rename one with `as` |  |
| E-DST-8 | E | yes | 1+1o | — | good | `:` is always followed by a type: to rename a field, write `a as other` |  |
| E-DST-9 | E | yes | 1+1o | — | **reword** | `...rest` must be the last item of a pattern | F19: a message has no fix: add one |
| E-ERR-1 | E | yes | 1+2o | — | good | `throw` needs an error value, not an `Integer`: declare an `Errors` type and throw one of its members |  |
| E-ERR-2 | E | yes | 1+1p+2o | — | good | this throws `ApiError.Timeout`, which `saveScore` doesn't declare: add `throws ApiError.Timeout`, or handle it |  |
| E-ERR-3 | E | yes | 34+2p+10o | — | **reword** | `persons[index]` can fail with `IndexError`: use `try` to propagate the error, or `catch` to handle it | F15: built-in errors shown `ParseError.ParseError`; style: "ex:" |
| E-ERR-4 | E | yes | 3+1o | — | good | `try` propagates `LoadError`, which `undeclared` doesn't declare: add `throws LoadError`, or handle it with `c |  |
| E-ERR-5 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| E-ERR-6 | E | yes | 5+2o | — | **reword** | `try` and `catch` on the same expression: keep `catch` to handle the error, or `try` to propagate it | F10: also fires for a `try { } catch` block (i02), where "the same expression" is misleading; style: "ex:" |
| E-ERR-7 | E | yes | 3 | — | good | errors are never part of the return type: remove `E.Bad` from `Integer \| E.Bad`, and declare `throws E.Bad` |  |
| E-ERR-8 | E | yes | 1 | — | good | `try` is written before `await`: write `try await fetchNumber()` |  |
| E-ERR-9 | E | yes | 1 | — | good | `error` is the error of this `catch`: give this value another name | style: "ex:" |
| E-ERR-10 | E | yes | 13+4o | — | **wrong** | this function declares `throws HttpError` but never propagates an error: remove it | F8: `throws String` gets "never propagates an error" although it throws; EF020 instead |
| E-IF-1 | E | yes | 1 | — | good | this `if` is used as a value, so it needs an `else`: add `else { … }` |  |
| E-IF-2 | E | yes | 3+2o | — | good | the branches give an `Integer` and a `Float`, which don't make a valid union: annotate the value, or convert o |  |
| E-IF-3 | E | yes | 1 | — | good | this branch has no value: it ends with a statement, so end it with the value it gives |  |
| E-LEX-1 | E | yes | 1+1o | — | **reword** | Statements are separated by new lines, not `;` | F10/F18: the `;` message is misleading in a C `for` and between type fields; registry summary is just "in source"; `&`/`^`/`~` have no code at all (AN002); style: capitalized |
| E-LEX-2 | E | yes | 1+3o | — | good | Names can only use ASCII letters, digits and `_`: other characters are only allowed in strings and comments | style: capitalized |
| E-LEX-3 | E | yes | 1+2o | advanced/memory-management, reference/language-reference | **reword** | Expected a name after `let`, found `loop` | F18: appears in the F3 cascade (`return` read as an object member); fine once F3 is fixed; style: capitalized |
| E-LEX-4 | E | yes | 2+2o | — | **reword** | `!` is not an operator: use `not` to negate a boolean, or `!=` for inequality | F10: `name!.` (assertion), `vec![`, `println!(` get "use `not` or `!=`" |
| E-LEX-5 | E | yes | 1+1o | — | good | `else` must be on the line of the `}` that closes the `if` block: write `} else {` |  |
| E-LEX-6 | E | yes | 1+2o | — | good | This string is never closed: end it with `"` | style: capitalized |
| E-LOOP-1 | E | yes | 1 | — | good | cannot iterate over an integer: use a range, ex: `0..n` | style: "ex:", "cannot" |
| E-LOOP-2 | E | yes | 2 | control-flow/loops | good | a `for` can't destructure an array: its length isn't known. Destructure a tuple, `for (a, b) in pairs`, or rea |  |
| E-LOOP-3 | E | yes | 1+1p+2o | — | **reword** | a range has no index: its values are already the positions | F18: same condition as CL032: merge; no active case; style: "ex:" |
| E-LOOP-6 | E | yes | 1+1p | — | good | a `for` loop has no value: compute the value in a variable, or use `loop` with `break value` |  |
| E-LOOP-7 | E | yes | 1 | — | good | `for` and `while` loops have no value: use `loop` to break with a value |  |
| E-LOOP-9 | E | yes | 1 | — | good | `while let` binds a name, not a pattern: write `while let item = stack.pop() { ... }` and destructure inside t |  |
| E-LOOP-10 | E | yes | 1 | — | good | `while let` stops at the first `none`, and an `Integer` is never `none`: use `while` with a condition |  |
| E-LOOP-11 | E | yes | 2 | — | good | this `loop` is used as a value: give the `break` one, ex: `break result` | style: "ex:" |
| E-LOOP-12 | E | yes | 2 | — | good | `break` is outside of a loop: it can only be used in `for`, `while` or `loop` |  |
| E-LOOP-13 | E | yes | 0+1p | — | good | `break` can't leave a function to act on the loop around it: return a value and test it in the loop | no test: add a case |
| E-LOOP-14 | E | yes | 2 | control-flow/loops | good | Fa has no labeled loops: `break outer` can't name a loop. `break` leaves the innermost loop; to leave several, | style: capitalized |
| E-NAR-1 | E | yes | 1 | — | good | `v` may be any of `Integer \| String`: test it with `is` first |  |
| E-NUM-1 | E | yes | 2+1o | — | good | `.5` is not a number: write `0.5` |  |
| E-NUM-2 | E | yes | 2+1o | — | good | `1_`: an underscore must be between two digits, like `1_000` |  |
| E-NUM-3 | E | yes | 3+1o | — | good | `0XFF` is not a valid number: a number literal can't be followed by letters or digits of another base |  |
| E-NUM-4 | E | yes | 1+1o | — | good | `12.5n` is not a valid number: a number literal can't be followed by letters or digits of another base |  |
| E-NUM-5 | E | yes | 1 | — | **wrong** | `%` only applies to a number literal or to an expression in parentheses: write `(fifty)%` | F5: `n % 2` is told to write `(n)%`; say `%%` |
| E-NUM-6 | E | yes | 14+5o | — | **reword** | this literal is -1, which doesn't fit in an `Integer<min = 0>` (0 to 9223372036854775807) | F15: shows `Integer<min = 0>` and a second range format |
| E-NUM-7 | E | yes | 1 | — | good | 10000000000000000000 doesn't fit in an `Integer`: write `10000000000000000000n` for a `BigInt` |  |
| E-NUM-8 | E | yes | 3 | — | good | 16777217 can't be represented exactly by a `Float32`: write a float literal, ex: `16777217.0` | style: "ex:" |
| E-NUM-9 | E | yes | 2 | — | good | a `Percentage` is written with `%`: write `50%` |  |
| E-NUM-10 | E | yes | 13+1p+8o | — | **reword** | this conversion always fails: the value is outside of `Int64` (-9223372036854775808 to 9223372036854775807) | F19: a message has no fix: add one |
| E-NUM-11 | E | yes | 7+4o | — | **reword** | `Math.max` takes two numbers of the same kind, not a `Float` and an `Integer`: convert one, ex: `Float(…)` | F16/F21: placeholder names (`BigInt(i)`, `Integer(b)`), "cannot", repeats the expression after the fix; style: "ex:", "cannot" |
| E-NUM-12 | E | yes | 3 | — | good | this divides by zero: the divisor is always 0 |  |
| E-NUM-13 | E | yes | 3 | — | **wrong** | `min` (5) is greater than `max` (1): this type has no value | F4: fix `{storage}(min = 0, max = 10)` uses parentheses; range shown `-128..=127` vs `-128 to 127` elsewhere; style: "ex:" |
| E-NUM-14 | E | yes | 5+1p+2o | — | **reword** | `counter` may be -9223372036854775808 to 9223372036854775807, which doesn't fit in a `UInt8` (0 to 255): narro | F15: "-9223372036854775808 to 9223372036854775807" for a plain `Integer` |
| E-NUM-15 | E | yes | 1+3o | — | good | an `Int8` and a `UInt8` have no common type: annotate the result, ex: `let value: Int16 = …` | style: "ex:" |
| E-NUM-16 | E | yes | 4 | — | **reword** | there is no conversion from `Float` to `BigInt`: round it first, ex: `BigInt(Integer.round(x))` | F19: a message has no fix: add one |
| E-NUM-17 | E | yes | 26+6o | — | good | this slice is out of bounds: use bounds from `-length` to `length` |  |
| E-NUM-18 | E | yes | 3 | — | **reword** | `*` doesn't apply to a `String`, a member of the constraint of `Value`: every member must allow it | n13: `Color.red + 1` shows the member as the type; say enumerations have no arithmetic and how to compare; style: "ex:" |
| E-NUM-19 | E | yes | 1+5o | — | good | (source) {value} doesn't fit in an `Integer` on JavaScript, which holds ±(2^53 − 1): use an `Int64` or a `BigI |  |
| E-NUM-20 | E | yes | 3 | — | good | the shift amount is outside of 0..8: a `UInt8` has 8 bits |  |
| E-NUM-21 | E | yes | 1 | — | good | `/=` gives a `Float`, which an `Integer` can't hold: write `x = x / …` into a `Float` value |  |
| E-OBJ-1 | E | yes | 9+8o | — | **reword** | `z` is not a field of `Point` | F19: no fix; closest field; note at the type; anonymous configuration type shown in full |
| E-OBJ-2 | E | yes | 10+3o | — | **reword** | missing field of `Person`: `name` | F19: add `` add `age = …` `` and a note at the field declaration; style: "ex:" |
| E-OBJ-3 | E | yes | 2+2o | — | **reword** | the field `x` is given twice | F19: a message has no fix: add one |
| E-OBJ-4 | E | yes | 3+3p+2o | — | good | `#secret` is private: only the functions of `Point` can give it |  |
| E-OBJ-5 | E | yes | 23+6o | — | **reword** | a `Human` has no field or method `powers` | F6/F19: did-you-mean; habit table (`take`→`firstValues`, `set`→`[k] = v`, `len`→`length`); drop the generic `+=`/`reserve` tail on strings; style: "ex:" |
| E-OBJ-6 | E | yes | 4+2o | — | good | values are built with braces: write `Point { field = value, … }` |  |
| E-OBJ-7 | E | yes | 2+2o | — | good | `port` replaces a field copied by the spread: it keeps its type `Integer`, not `String` |  |
| E-OBJ-8 | E | yes | 1 | — | good | `port` is written before this spread, which replaces it: write it after the spread |  |
| E-OBJ-9 | E | yes | 1 | — | **reword** | only an object can be spread, not an `Integer` | F19: a message has no fix: add one |
| E-OBJ-10 | E | yes | 3+2o | — | good | `id` is `readonly`: only the functions of `Point` can change it |  |
| E-OBJ-11 | E | yes | 6+1p+4o | — | **reword** | `length` can't be assigned | "`length` can't be assigned": say why and what to call |
| E-OBJ-12 | E | yes | 2+4o | — | good | `area` is both a field and a function of this type: rename one |  |
| E-OBJ-13 | E | yes | 1 | — | **reword** | `.=` expects an object literal, like `settings .= { port = 8080 }` | F19: a message has no fix: add one; style: "ex:" |
| E-OBJ-14 | E | **no** | 0+1p | — | **missing** | — | F10: `=> { x = 0, y = 0 }` gives 3 errors incl. "a tuple can't hold it"; emit this instead |
| E-OPT-1 | E | yes | 21+2o | — | good | `current` may be `none`, and an `Integer` is expected: test it with `if current is not none` first, or use `?? | style: "ex:" |
| E-OPT-2 | E | yes | 2 | — | good | a `Boolean` can't be `none`: make the type optional, ex: `Boolean?` | style: "ex:" |
| E-OPT-3 | E | yes | 5 | — | good | a `U` is never `none`: use `.` instead of `?.` |  |
| E-OPT-5 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| E-OPT-6 | E | yes | 8 | — | good | `??` only applies to optionals, and an `Integer` is never `none`: remove `?? …` | style: "ex:" |
| E-OPT-9 | E | yes | 2 | — | good | `Void` is not a value type: it can't be optional |  |
| E-RNG-2 | E | yes | 2 | — | **reword** | the bounds of a range are integers, not a `Float` | F19: a message has no fix: add one |
| E-RNG-3 | E | yes | 1 | — | good | a range without a start can't be iterated: give it one, ex: `0..` | style: "ex:" |
| E-RNG-4 | E | yes | 3 | — | **reword** | a range pattern matches integers only | F19: a message has no fix: add one |
| E-STR-1 | E | yes | 1+3o | — | good | Unknown escape sequence `\q`: the escapes are `\\`, `\"`, `\n`, `\r`, `\t`, `\0`, `\xNN` and `\u{...}` | style: capitalized |
| E-STR-2 | E | yes | 2+2o | — | good | `\x` is followed by two hexadecimal digits, from `\x00` to `\x7F`: write other characters with `\u{...}` |  |
| E-STR-3 | E | yes | 6+3o | — | **reword** | `\u` is followed by a Unicode scalar value of 1 to 6 hexadecimal digits between braces, like `\u{1F600}` | F19: a message has no fix: add one |
| E-STR-4 | E | yes | 1+1o | — | good | A `}` in a string must be doubled: write `}}` for a literal brace | style: capitalized |
| E-STR-5 | E | yes | 1+1o | — | good | Empty `{}` in a string: write an expression between the braces, or `{{` and `}}` for literal braces | style: capitalized |
| E-STR-6 | E | **no** | 0 | — | **missing** | — | unterminated interpolation: check what the lexer reports today (E-LEX-6?) and alias |
| E-STR-7 | E | yes | 2+1o | — | **reword** | `+` joins strings: expected a `String`, found an `Integer`: use `"…{score}"` or `score.toString()` | F15: "found a `3`"; two stacked colons |
| E-STR-11 | E | yes | 2+1o | — | good | this expression has no value (`Void`): there is nothing to print |  |
| E-STR-12 | E | yes | 2+1o | — | **reword** | a function (`(Unknown) => Void`) has no `toString`: call it, or print something else | F15: shows `(Unknown) => Void` |
| E-STR-13 | E | yes | 1+2o | — | **reword** | Strings are written between double quotes, like "text" | F19: a message has no fix: add one; style: capitalized |
| E-TUP-1 | E | yes | 3 | — | good | a tuple has at least two elements: there is no empty or one-element tuple (`(value)` is a parenthesized expres |  |
| E-TUP-2 | E | yes | 1 | — | good | a tuple index must be a literal, ex: `pair[0]`: the type of the element depends on it | style: "ex:" |
| E-TUP-3 | E | yes | 3 | — | good | this tuple has 2 elements: its indices go from 0 to 1 |  |
| E-TUP-4 | E | yes | 1+1o | — | good | Tuples are indexed with brackets: write `pair[0]` | style: capitalized |
| E-TYPE-1 | E | yes | 73+3p+37o | — | **reword** | an `A` is never a `B`: each declaration of a named union, an enumeration or a type with private fields is its  | F15: shows a literal as a type ("a `"1"` is not accepted"), loses alias names (`Status`); add a note at the declared type/parameter; style: "ex:", "cannot" |
| E-GEN-1 | E | yes | 3 | types/generics | **reword** | the constraint `Integer` of `T` is narrower than the type's: a `<T: Integer \| Float>(T) => T` accepts an `Inte | F19: a message has no fix: add one |
| E-GEN-2 | E | yes | 2 | types/generics | good | a `(Integer) => Integer` isn't generic: a `<Value>(Value) => Value` must accept any type for `Value`, write a  | style: "ex:" |
| E-WHEN-1 | E | yes | 9 | — | **reword** | this `when` doesn't handle `Color.Blue` | F19: add `` add `Color.blue => …`, or `else => …` `` |
| E-WHEN-3 | E | yes | 1 | — | **reword** | an `Int8` never matches `300` | F19: a message has no fix: add one |
| E-WHEN-4 | E | **no** | 0+1p | — | **missing** | — | F10: a guard gives 5 errors (c12); emit this instead |
| E-WHEN-5 | E | yes | 0+1p | — | good | (source) `when` matches a type, a member or a literal: bind the value with `as` and test its parts inside the  | no test: add a case |
| E-WHEN-6 | E | yes | 1+1o | — | good | The `else` branch can't be bound with `as`: bind the subject before the `when` | style: capitalized |
| E-WHEN-7 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| E-WHEN-8 | E | yes | 1 | — | **reword** | `else` is the last arm of a `when` | F19: a message has no fix: add one |
| W-BOOL-1 | W | yes | 3+1p | — | **reword** | this condition is always true | F19: a message has no fix: add one |
| W-CMP-1 | W | yes | 2+3p | — | good | `x is Integer` is always true: an `Integer` is always an `Integer` |  |
| W-DECL-1 | W | yes | 2+3o | — | good | `console` hides the built-in `console`: give it another name |  |
| W-DECL-2 | W | yes | 4+8o | — | **reword** | the reference `x` is never used: remove it, or start its name with `_` to say that it is unused on purpose | F9: warns on the documented `catch error => 0` and says "remove it" (E-ERR-9); span the name, not the expression (language question) |
| W-DO-1 | W | yes | 6 | — | good | `x + 1` computes a value and drops it: this statement has no effect. Remove it, or use its value |  |
| W-DO-2 | W | yes | 3+1o | — | good | this code is never run: the code before it always leaves (`return`, `throw`, `break`, `continue`) |  |
| W-ERR-1 | W | yes | 4+4o | — | good | this expression can't fail: remove the `try` |  |
| W-ERR-2 | W | yes | 2 | — | **wrong** | this expression can't fail: remove the `catch` | F8: follows an undeclared-throws error in the callee and says "remove the `catch`" |
| W-ERR-4 | W | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| W-LEX-1 | W | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| W-LOOP-1 | W | yes | 1 | — | good | the value of this `break` is dropped: the `loop` is not used as a value |  |
| W-NUM-1 | W | yes | 2 | — | good | this conversion does nothing: an `Int8` is already accepted where an `Integer` is expected |  |
| W-NUM-2 | W | yes | 8+3o | — | **reword** | the condition `value < 0` is always false: the types of its operands decide it (0..=255 < 0) | F15: raw ranges `(20..=9223372036854775807 < 20)` |
| W-OBJ-1 | W | yes | 3+3o | — | good | the type is written twice: drop the annotation, or the type before the braces |  |
| W-RNG-1 | W | yes | 2 | — | good | `is in` takes a range value: with a range literal, write `is 0..3` |  |
| W-WHEN-1 | W | yes | 2+1p | — | good | this arm never runs: the arms above already match every value of `5`. Remove it, or move it up |  |
| W-WHEN-2 | W | yes | 2 | — | good | this `else` never runs: the other arms handle every value. Remove it |  |
| CL005 | E | yes | 1 | — | good | the items of a map are `(key, value)` pairs, ex: `[("ana", 12)]` | style: "ex:" |
| CL006 | E | yes | 1 | — | **reword** | the key "a" is given twice | F19: a message has no fix: add one |
| CL007 | E | yes | 2 | — | good | a `Pool` has a `size`, not a `length`: its elements have no positions |  |
| CL010 | E | yes | 2+2o | — | good | a `Set<SuperHuman>` is never a `Set<Human>`: the elements of a `Set` are exact; copy it with `Set.from(…)` int |  |
| CL020 | E | yes | 1 | — | **reword** | a `Set` element needs `==`, but a `Handler` can't be compared (it contains a function, an iterator or a task) | F19: a message has no fix: add one |
| CL021 | E | yes | 1 | — | good | a `Columnar` stores one column per field: its elements are objects, not `Integer`; use a `Pool` or an `Array` |  |
| CL023 | E | **no** | 0 | — | **dead** | — | impure key function: `Set` lost `key`; delete |
| CL024 | E | yes | 1 | — | good | `sum` adds numbers, and the items are `String`: use `reduce` to combine them |  |
| CL025 | E | yes | 1 | — | good | the natural order needs items that can be compared with `<`, and a `Player` can't: give `sort` a comparator, l |  |
| CL030 | E | yes | 2 | — | good | an iterator reads its collection while it runs: it can't be returned. Consume it where it is written, ex: `.to | style: "ex:" |
| CL031 | E | yes | 11+2o | — | good | `tag` is a key or an element of a set: it can't be modified; delete it and add the new value |  |
| CL032 | E | yes | 3 | — | **reword** | an iterator has no positions: its elements have no index (only an `Array`, a `GrowingArray`, an `OrderedColumn | F18: same condition as E-LOOP-3: merge |
| CL040 | E | yes | 4+1o | — | good | a `GrowingArray` never removes elements: it has no `clear` (an `Array` has) |  |
| CV002 | E | yes | 2 | — | good | there is no `String(value)`: write `value.toString()`, or a template like `"{value}"` |  |
| CV003 | E | **no** | 0 | — | **missing** | — | F10: `count as Float` gives "Unexpected `as` after the end of the statement" |
| CV005 | E | yes | 1 | — | good | an `Array<Int64>` is not an `Array<Integer>`: the same values are stored differently inside them (a 64-bit int | style: "ex:" |
| EF004 | E | yes | 5+1p | advanced/structural-typing | good | `renamed` writes `name: String` into a copy of `Tight { name = "a", age = 1 }` that it returns as its type, wh |  |
| EF012 | E | yes | 2 | — | good | `addBonus` mutates this argument, but it is a temporary value: the mutation would be lost (store it in a `muta |  |
| EF018 | E | yes | 2 | — | good | the errors of a callback this function only calls belong to the caller: this `catch` can't catch them (handle  |  |
| EF020 | E | **no** | 0+1p | — | **missing** | — | F8: `throws String` |
| FN011 | E | yes | 2 | — | good | missing value at the end of `missing`: it returns an `Integer` |  |
| FN012 | E | yes | 1 | — | good | `stop` returns `Never`: it must always throw, stop or loop forever |  |
| FN013 | E | yes | 2 | — | good | `noType` has no return type: `return` can't give a value. Declare one, ex: `(…): Integer =>` | style: "ex:" |
| FN015 | E | yes | 2 | — | good | `second` is declared later in this block: two functions of a block can't call each other, declare them at the  |  |
| FN020 | E | yes | 2 | — | good | the default value uses `end`, a later parameter: it can only use the parameters before it |  |
| FN021 | E | yes | 2 | — | **reword** | a positional argument can't follow a named argument | F19: a message has no fix: add one |
| FN022 | E | yes | 2 | — | **reword** | `x` is already given | F19: a message has no fix: add one |
| FN023 | E | yes | 7 | collections/arrays, collections/sets, reference/language-reference | **wrong** | `Array.from` has no option `size`: its only option is `capacity` | F7: `Set.from(…, key = …)` accepted although documented as FN023; add did-you-mean for named arguments; style: "ex:" |
| FN024 | E | yes | 6 | — | **reword** | missing argument: `y` | F16: "missing argument: #2"; name the function and the parameter with its type; style: "ex:" |
| FN025 | E | yes | 13 | — | **reword** | this function takes 3 arguments | F16: name the function and its parameters; span the extra arguments; style: "ex:" |
| FN026 | E | yes | 2 | — | good | the variadic parameter `values` can't be named: pass its values by position |  |
| FN027 | E | yes | 2 | — | good | named arguments only exist for direct calls of a `function`: pass the arguments by position |  |
| FN028 | E | yes | 2 | — | **reword** | a variadic parameter is the last one, and a function has at most one | n17: followed by W-DECL-2 and FN024 for the same mistake |
| FN029 | E | yes | 1 | — | **reword** | a collection can only be spread into a variadic parameter | F19: a message has no fix: add one |
| FN030 | E | yes | 1 | functions/defining-functions | good | a function type cannot declare a default: put the default on a named function, or pass this argument explicitl | style: "cannot" |
| FN040 | E | yes | 2 | — | **reword** | `self` is only a parameter of the functions declared in a type | F19: a message has no fix: add one |
| FN041 | E | yes | 1 | — | good | `self` has no annotation: its type is the type of the method |  |
| FN042 | E | yes | 1 | — | good | functions are declared in a `type`: declare one, or use a field holding a lambda |  |
| FN043 | E | yes | 1 | — | good | `make` is a function of the type: write `Self.make(…)`, or `self.make(…)` for a method |  |
| FN061 | E | yes | 2 | — | good | the type of `c` is not known: annotate it, ex: `c: Integer` | style: "ex:" |
| FN062 | E | yes | 1+1p | — | **reword** | this function takes at most 2 parameters here | F19: a message has no fix: add one |
| FN063 | E | yes | 1 | — | good | this function is `async`, but the function type expected here isn't: a call through it would have to block; re |  |
| FN072 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| FN084 | E | yes | 3 | — | good | an `async` function doesn't fit `(Request) => Response`, which isn't `async`: a call through it would have to  |  |
| FN086 | E | **no** | 0+1p | — | **missing** | — | F7: `let read = counter.get` accepted |
| FN090 | E | yes | 1 | — | good | `add` takes 2 parameters: a following step of a composition takes exactly one, the result of the step before i |  |
| FN091 | E | yes | 1 | — | **reword** | `stringLength` expects a `String`, but receives an `Integer` | F19: a message has no fix: add one |
| FN092 | E | yes | 1 | — | good | `bump` changes its parameter, but in a composition it receives a temporary value: call it on a place instead |  |
| FN093 | E | yes | 1 | — | **reword** | this slice is out of the 4 steps of `oneToFour` | F19: a message has no fix: add one |
| FN094 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| FN095 | E | yes | 1 | — | good | `pipeline` is a plain `(Integer) => Integer`: it has no composition chain to slice, only a function declared w |  |
| FN100 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| FN101 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| FN102 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| FN103 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| FN104 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| FN110 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| GN001 | E | yes | 2+1o | — | good | The compile-time value parameter `size` needs a type, like `size: Integer`. Type parameters are capitalized | style: capitalized |
| GN003 | E | yes | 1 | — | **reword** | the default `String` doesn't satisfy the constraint `Integer` | F19: a message has no fix: add one |
| GN004 | E | yes | 1 | — | **reword** | `T` is already a parameter of `twice` | F19: a message has no fix: add one |
| GN005 | E | yes | 1 | — | good | `U` comes after a parameter with a default: give it one too |  |
| GN006 | E | yes | 2 | — | good | a compile-time value is compared exactly: a `Float` can't be one (use an integer, a string, a boolean, an enum |  |
| GN007 | E | yes | 3 | types/generics | **reword** | `Box` takes 1 type argument, not 2 | F19: a message has no fix: add one |
| GN008 | E | yes | 1+1o | — | **reword** | a compile-time value is expected here: a literal or a constant expression | F18: follow-on of GN001 (e22) |
| GN010 | E | yes | 4+1p+1o | — | **reword** | `String` doesn't satisfy the constraint `Integer` of `T` | F19: a message has no fix: add one |
| GN011 | E | yes | 2 | — | good | `Point2` satisfies several members of the constraint `Point2 \| Point1` of `Shape`: give it explicitly |  |
| GN013 | E | yes | 5 | — | **reword** | the method `describe` of `Dog` is a `() => Integer`, but `Item` requires a `() => String` | F19: a message has no fix: add one |
| GN014 | E | yes | 3 | — | good | `Item` is `Integer` here and `String` there: give it explicitly, or make the values agree |  |
| GN015 | E | yes | 9 | — | **reword** | `Output` can't be inferred here: give it explicitly after the name of the function, ex: `f<OutputType>(…)`, or | F16: suggests `f<ItemType>(…)` instead of the function's name (n07); style: "ex:" |
| GN016 | E | yes | 3 | — | good | `Value` has no constraint allowing `*`: constrain it, ex: `<Value: Integer \| Float>` | style: "ex:" |
| GN017 | E | **no** | 0+1o | — | **missing** | — | registered, no emitter: implement or delete |
| GN020 | E | yes | 1+1o | — | good | `List<Item>` refers to itself with other type arguments (polymorphic recursion): use its own parameters, `List |  |
| GN030 | E | yes | 6+3o | functions/defining-functions, reference/language-reference | good | a value of a generic function type must be known at compile time: only a parameter can have this type, and it  |  |
| GN031 | E | yes | 15+1o | — | **wrong** | Generic parameters use angle brackets: write `<Item>`. Run `fa migrate-generics --write <path>` to update exis | F3: `function add(a: Integer) {` gets generic advice and `fa migrate-generics`; style: capitalized |
| TF001 | E | yes | 1+1o | — | good | `Loop<N>` applies itself at new arguments each time: its applications nest without end (more than 256) |  |
| TF002 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| TF004 | E | yes | 2 | types/types-at-runtime | **reword** | only string literals | "only string literals" / "Only string literals are supported": say what is not a literal and how to fix |
| TF007 | E | yes | 1 | — | good | no branch of this `when` matches `Boolean`: add a branch for it, or an `else` |  |
| TF010 | E | yes | 5 | — | **reword** | `age` is not a key of `User2` | F19: a message has no fix: add one; style: "ex:" |
| TF011 | E | yes | 5+1o | types/fields | good | for the key `port`: a `{ size: Integer }` has no field `port` |  |
| TF020 | E | yes | 1 | — | good | `Integer` is already a type: write it without `Type<…>` |  |
| TF021 | E | yes | 1 | — | **reword** | `Keys` takes an object type (the keys of a map are `KeyType<…>`), not a `Map<String, Integer>` | F19: a message has no fix: add one |
| TF022 | E | yes | 1 | — | **reword** | `KeyType` takes a map type, not an `Array<Integer>` | F19: a message has no fix: add one |
| TF024 | E | yes | 1 | — | good | `nothing` is always `none`: it can't be required |  |
| TF026 | E | yes | 1 | — | good | the parameter 1 of `(Integer, Integer) => Integer` has no name: `Parameters` needs named parameters, ex: `(id: | style: "ex:" |
| TF028 | E | yes | 1 | — | **reword** | `Uppercase` takes a string type or string literals, not an `Integer` | F19: a message has no fix: add one |
| TF040 | E | yes | 1 | — | good | invalid schema: `Schema.Reference { name = "Nowhere" }` refers to no enclosing `Schema.Named` |  |
| TF041 | E | yes | 2 | — | good | invalid schema: two properties are named `a` |  |
| TF042 | E | yes | 18+5o | — | good | invalid schema: an inline-array capacity must be from 0 through 4294967295: choose a capacity in that range |  |
| TF043 | E | yes | 1 | — | good | invalid schema: `Bag` is not a collection kind |  |
| TY001 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| TY003 | E | yes | 1 | — | **wrong** | `GrowingPool` takes the type of its items, ex: `GrowingPool(Integer)` | F4: fixes use removed parentheses (`Map(String, Integer)`, `GrowingPool(Integer)`, `Type(user)`); style: "ex:" |
| TY007 | E | yes | 4+1p+4o | — | **reword** | `Logger` is not a type | F6: suggest close names and the Fa spelling (`number`→`Integer`/`Float`, `string`→`String`, `i32`→`Int32`, `any`, `Option<T>`→`T?`); `Circle`→`Shape.Circle` (TY055) |
| TY008 | E | yes | 6+1p+2o | — | good | `notAType` is a value, not a type: write its type, or `Type<notAType>` | style: "ex:" |
| TY010 | E | yes | 1 | — | **reword** | the field `a` is declared twice | F19: a message has no fix: add one |
| TY012 | E | **no** | 0 | — | **dead** | — | same summary as TY010 (duplicate field/method), no emitter: delete or alias |
| TY013 | E | yes | 1 | — | good | a default value can't use `a`: it is computed before the other fields exist |  |
| TY021 | E | yes | 1 | — | good | a `Shape` may hold any of its members: narrow it to a member first, with `when` or `is` |  |
| TY040 | E | yes | 5+6o | — | good | `Integer` and `Float` can't be told apart at runtime: declare a named union, ex: `Union { A = …, B = … }` | style: "ex:" |
| TY041 | E | yes | 1 | — | good | an `AB` is both an `A` and a `B`: copy it into one of them first |  |
| TY043 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| TY044 | E | **no** | 0+1p+1o | — | **dead** | — | reported as E-OPT-3/E-OPT-6 (code-map overlap): make it an alias |
| TY050 | E | yes | 1 | — | **reword** | the member `Circle` is declared twice | F19: a message has no fix: add one |
| TY051 | E | yes | 1 | — | good | two members accept `"x"`: a literal belongs to one member |  |
| TY052 | E | yes | 2 | — | good | `Shape.Circle` holds fields: write `Shape.Circle { field = value, … }` |  |
| TY053 | E | yes | 1 | — | good | `Shape.Labelled` holds a value, not fields: write `Shape.Labelled(value)` |  |
| TY054 | E | yes | 1 | — | good | `Shape.Point` holds no data: write it without parentheses |  |
| TY055 | E | yes | 2 | — | good | a `Shape` is built by naming its member: `Shape.Circle { … }` |  |
| TY056 | E | yes | 1+1p | — | good | a plain object is never a `Shape.Circle`: write `Shape.Circle { … }` |  |
| TY057 | E | yes | 2+1p+2o | — | **reword** | members are always qualified: write `ParseError.ParseError` | F15: "write `ParseError.ParseError`" |
| TY060 | E | yes | 1+1p+2o | — | good | `Node` contains itself: a value would never end; make the field optional (`Node?`), or hold it in a collection |  |
| TY071 | E | yes | 0+2p | — | **unjudged** | — | message built elsewhere (CLI or a helper): not read by this audit; no test: add a case |
| TY072 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| TY073 | E | **no** | 0 | — | **missing** | — | registered, no emitter: implement or delete |
| TY080 | E | yes | 1 | — | **reword** | the member `Red` is declared twice | F19: a message has no fix: add one |
| TY081 | E | yes | 1 | — | **reword** | either every member of an enumeration has a value, or none | F19: a message has no fix: add one |
| TY082 | E | yes | 1 | — | **reword** | 1 is already the value of another member | F19: a message has no fix: add one |
| TY083 | E | yes | 1 | — | **reword** | the value of a member is a non-negative integer literal | F19: a message has no fix: add one |
| TY084 | E | yes | 1 | — | **reword** | write `Type.red`: the type of `.red` is not known here | placeholder `Type.red` |
| TY090 | E | yes | 2 | — | **reword** | the flag `read` is listed twice at this level | F19: a message has no fix: add one; style: "ex:" |
| TY091 | E | yes | 1 | — | good | a `Fields` type has at most 128 flags, counting the members of its groups; this one has 129: split it |  |
| TY092 | E | yes | 4 | — | **reword** | `delete` is not a flag of `Permissions` | F19: a message has no fix: add one |
| TY100 | E | yes | 1 | — | **reword** | only object types can be spread, not an `Integer` | F19: a message has no fix: add one |
| TY101 | E | yes | 1 | — | good | the type of `foo` is ambiguous: `String` or `Integer`; a spread never overrides a field (remove one, ex: with  | style: "ex:" |
| TY110 | E | yes | 1+1o | — | good | `String` can't be split: no type means every value of `String` but some; list the values you want |  |
| TY111 | E | yes | 1 | — | good | `"c"` is not a member of `"a" \| "b"`: nothing is removed |  |
| WCL001 | W | yes | 2 | — | good | the type is already known here: write the list literal without `Set.from` |  |
| WCL002 | W | yes | 1 | — | **reword** | bad practice: this is a copy of the stored element; keep the reference with `use … = points.add(…)` | F21: "bad practice:" prefix |
| WFN001 | W | yes | 2 | — | **reword** | this value is dropped: `dropped` has no return type. Declare one, ex: `(…): Integer =>` | F18: paired with E-TYPE-1 at the call for one mistake (d12); style: "ex:" |
| WFN002 | W | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| CFG003 | E | **no** | 0+1p | — | **missing** | — | registered, no emitter: implement or delete |
| CFG004 | E | yes | 0+2o | — | good | (source) error[CFG004]: two packages of the workspace are named `{name}`: `{}` and `{}`; rename one of them in |  |
| CFG006 | E | yes | 0+1o | — | good | (source) fa: can't read `{}`: {error} |  |
| CFG007 | E | yes | 0+1o | — | good | (source) computed configuration needs the compile-time interpreter (not available yet): write a literal |  |
| CLI001 | E | yes | 0+3o | — | good | (source) fa: can't read `{}`: {error} |  |
| CLI002 | E | yes | 0 | — | good | (source) fa build: `deno` is not a target: Fa doesn't support Deno; use `bun` or `node` to run on a server Jav | no test: add a case |
| CLI003 | W | yes | 0 | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit; no test: add a case |
| CLI004 | E | yes | 0+1o | — | good | (source) error[CLI004]: the workspace has no package named `{name}`: its packages are {} |  |
| CLI005 | E | yes | 0 | — | good | (source) error[CLI005]: Bun is needed to run Fa code on the bun target, and it is neither on the PATH nor in ~ | no test: add a case |
| CLI006 | E | yes | 0+1o | — | good | (source) fa preview: can't start the program: {error} |  |
| CLI007 | E | yes | 0+4o | — | good | (source) error[CLI007]: `{name}` already exists and isn't empty: choose another name, or empty it |  |
| CONC001 | E | yes | 6+3o | — | **reword** | `for parallel` is only available on `native`, not on `bun`: branch with `@if target is "native"`, or write a p | F21: printed once per target for the same span (n01): merge the targets into one message; style: "ex:" |
| CONC002 | E | yes | 4 | — | good | `results.push` changes `results`, shared by every iteration of the parallel loop: write to `results[index]` in |  |
| CONC003 | E | yes | 2 | — | good | `break` can't leave a parallel loop: its iterations run at the same time, in any order; `continue` ends the cu |  |
| CONC005 | E | yes | 1 | — | good | a parallel loop over a `Set<String>` has no index: only an `Array`, a `Columnar` and a `GrowingArray` have pos | style: "ex:" |
| CONC006 | E | yes | 1 | — | good | a `Mutex` holds owned data, shared between threads: `Person in people` contains a reference or a relation; sto |  |
| CONC007 | E | yes | 1 | — | good | an `AtomicInteger` changes: declare it with `mutable`, ex: `mutable … = AtomicInteger { value = 0 }` | style: "ex:" |
| CONC008 | E | yes | 11 | advanced/concurrency | good | (source) {what} may be called from any thread: it can't use the module-level `mutable` value `{}`{through}, wh |  |
| CONC010 | E | yes | 13+2o | — | good | `recordTwice` can't be called blocking on target `bun`: recordTwice → record → Mutex.lock (blocking only on `n |  |
| CONC011 | E | yes | 0 | — | **unjudged** | — | message built elsewhere (CLI or a helper): not read by this audit; no test: add a case |
| CONC012 | E | yes | 1 | — | good | `record` changes `hits`, which every iteration shares: a parallel loop can only change a module-level value or |  |
| CONC013 | E | yes | 1 | — | good | `values[0]` reads `values`, whose elements other iterations write at the same time: read only `values[index]`, |  |
| CONC014 | E | yes | 10 | — | good | `await` is only allowed in an async context: declare `async function blocking = …`, or call the function block |  |
| CONC015 | E | yes | 3 | — | **reword** | `plain` isn't `async`: it never waits, so call it directly, without `await` | F15: "not a `3`" |
| CONC016 | E | yes | 3 | — | good | this task is detached (it isn't kept), so nobody awaits it, and it can fail with `ParseError`: handle its erro |  |
| CONC017 | E | yes | 3 | — | good | this task is detached, so it copies what it uses, and it uses a requirement: keep the task and await it, or gi |  |
| CONC018 | E | yes | 6 | — | good | this task outlives a value it uses: a provided requirement, which ends with its block; give the task values it |  |
| CONC019 | E | yes | 2 | — | good | (source) the worker is used by {task}, a call that may still be pending: await it (or cancel it) before stoppi |  |
| DEP001 | E | yes | 0+4o | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| DEP002 | E | yes | 0+2o | — | good | (source) error[DEP002]: `{}` has no version: tag a commit with a semantic version, ex: `1.0.0` | style: "ex:" |
| DEP003 | E | yes | 0+2o | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| DEP004 | W | yes | 0+1p+3o | architecture/dependencies | good | (source) warning[DEP004]: `{}` has no version range: {} gets its latest version, which can change; write `{}@< | only pending cases |
| DEP005 | E | yes | 0+1p+1o | — | good | (source) {shown}: error[DEP005]: `{key}` can't name a dependency: a key is an identifier | only pending cases; style: "ex:" |
| DEP006 | E | yes | 0+2o | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| DEP007 | E | yes | 0+2o | architecture/dependencies | good | (source) error[DEP007]: the `manifest.fa` of `{url}` doesn't export a list of paths, ex: `export = [\"button.f | style: "ex:" |
| DEP008 | E | yes | 0+3o | architecture/dependencies | good | (source) error[DEP008]: `{repository}@{}` depends on `{key}` by a path (`{path}`): a library of a repository c |  |
| DEP009 | E | yes | 0+3o | architecture/packages | good | (source) error[DEP009]: `{name}` doesn't support the target `{target}`: its `package.fa` lists {} |  |
| DEP010 | E | yes | 0+1o | — | **reword** | (source) error: `{url}` is not a file | printed as `error: …` without its code (dependencies.rs) |
| EFF001 | E | yes | 6+1o | — | good | this call mutates `items`, but the signature of `remove` doesn't say so: add `mutates items` |  |
| EFF002 | E | yes | 1+2o | — | good | `trim` invalidates `numbers[..]`, but its signature doesn't say so: add `invalidates numbers[..]` (or `mutates |  |
| EFF003 | E | yes | 7+1p+2o | — | good | `player` is not declared as mutated: add `mutates player` to the signature of `addBonus` |  |
| EFF004 | E | yes | 13+1o | — | good | only `player.score` is declared as mutated, but `resetScore` also changes `player.name`: declare `mutates play |  |
| EFF005 | E | yes | 4+4o | — | good | `value` is an `Integer`: a primitive value can't be mutated, return the new value instead |  |
| EFF006 | E | yes | 1+2p+4o | — | good | `input` is a parameter: it can't be replaced, return the new value instead |  |
| EFF007 | E | yes | 14+6o | — | good | `addBonus` mutates its argument, but `alice` is declared with `let`: declare it with `mutable` |  |
| EFF008 | E | yes | 6+2o | — | **reword** | `nothing` is not a parameter, `self` or a module-level value: an effect path starts at one of them | F19: a message has no fix: add one; style: "ex:" |
| EFF009 | E | yes | 2+1p+2o | — | good | `invalidates` names elements (`items[index]`, `items[..]`): a change of `items` itself is `mutates items` |  |
| EFF011 | E | yes | 9+8o | — | good | `mutates player` is declared, but `noop` never changes it: remove it |  |
| EFF013 | E | yes | 2 | — | good | (source) `{text}` can only be written from `{path}`: call a function of its namespace that changes it |  |
| EFF014 | E | yes | 1 | — | good | `bestFriendOf` is a reverse relation side: it changes only through its direct side, whose write implies it. De |  |
| EFF020 | E | yes | 3+4o | — | good | `forever` is recursive: declare its return type, ex: `function forever = (…): Integer => …` | style: "ex:" |
| FFI001 | E | yes | 1 | — | good | C functions can only be declared in a foreign file (`*.foreign.fa`): move this declaration into one | style: "ex:", capitalized |
| FFI002 | E | yes | 2 | — | good | `Pointer` can only be used in a foreign file (`*.foreign.fa`): bind the C library there, and export a safe int |  |
| FFI003 | E | yes | 1 | — | good | (source) only a type of a foreign file (`*.foreign.fa`) can declare `destroy` |  |
| FFI005 | E | yes | 1 | — | good | (source) a C function can't declare `throws` or `requires`: C can't propagate a Fa error |  |
| FFI006 | E | yes | 1 | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| FFI007 | E | **no** | 0+1o | advanced/foreign-code | **missing** | — | only reachable through `fa bind` (code-map); give `fa bind` the emitter or delete |
| FFI008 | E | yes | 1+5o | — | good | (source) a `CStruct` can't be given to C by value on target `{target}`: pass a `Pointer` to it |  |
| FFI009 | E | yes | 1 | — | **reword** | a `CStruct` field has a C type (numbers, `std.c` types, `CStruct`, `CArray`, and `Pointer` in foreign files),  | F19: a message has no fix: add one |
| FFI010 | E | yes | 1 | — | good | (source) C writes through this parameter (`mutates`): pass a `mutable` value, not `{}` |  |
| FFI011 | E | yes | 1 | — | good | (source) `destroy` takes `self` only, and can't throw or be `async`: `function destroy = (self) => { … }` |  |
| FFI012 | E | yes | 0+2o | — | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| FFI013 | E | yes | 0+2o | architecture/configuration | **unjudged** | (source)  | message built elsewhere (CLI or a helper): not read by this audit |
| FFI014 | E | yes | 0+3o | advanced/foreign-code | good | (source) the API of a C library (`build.cLibrary.api`) exports a namespace of functions: `export namespace = { |  |
| MEM002 | E | yes | 30+6o | advanced/foreign-code, advanced/memory-management, reference/language-reference | **reword** | `subscription` was stopped on line 5, which consumed it: it can't be used afterwards | F13: "moved on line N" in the text; "consumed"; "cannot"; style: "cannot" |
| MEM003 | E | yes | 0+1o | advanced/concurrency, advanced/foreign-code, advanced/memory-management, advanced/reactivity, collections/arrays, functions/defining-functions, reference/language-reference, types/generics, values/declaring-values, values/objects | good | (source) this value owns a Task or a resource and cannot be copied: borrow it, move the owner, or copy a data  | style: "cannot" |
| MEM004 | E | yes | 14+7o | advanced/concurrency, advanced/memory-management, collections/arrays, functions/defining-functions, reference/language-reference, types/generics, values/declaring-values, values/objects | **wrong** | this escaping closure captures `n`, a reference into `local`, whose elements may not outlive the closure: capt | F7: `let b = a` then a use of `a` (an Array) accepted (h01/h02); ownership enforcement may still be landing |
| META001 | E | yes | 1 | — | good | `count` is not known at compile time: compile-time code takes literals, static values, compile-time parameters |  |
| META002 | E | yes | 12+2p+1o | — | good | `now` is not deterministic: a function called at compile time must be pure |  |
| META003 | E | yes | 2+2o | advanced/metaprogramming | good | this compile-time evaluation nests more than 10000 calls: it may never end |  |
| META004 | E | yes | 0+1o | advanced/metaprogramming | good | (source) more than {MAX_NESTING} nested applications of type functions: it may never end |  |
| META005 | E | yes | 4 | — | **reword** | this compile-time evaluation throws `ParseError { message = "`x` is not an integer" }`: an error at compile ti | F17: followed by a bogus AN001 inside the JSON string (l06); say what is malformed |
| META006 | E | yes | 1 | — | **reword** | the condition of `@if` is a `Boolean` known at compile time, not an `Integer` | F19: a message has no fix: add one |
| META007 | E | yes | 1 | — | good | the items of `@for` are known at compile time: `items` is not |  |
| META008 | E | yes | 1 | — | good | (source) `{relative}` is outside the root of the program: compile-time code reads the files of its package (or |  |
| META009 | E | yes | 1 | — | good | (source) `{relative}` is outside the root of the program |  |
| META010 | E | yes | 0+1p+2o | — | good | the source `https://api.weather.com/fa/types.fa` of `@import` isn't resolved: `fa` resolves it when it builds  | only pending cases |
| META011 | E | yes | 2+2o | — | good | A `static` value can't be exported: declare it as a member of an exported namespace | style: capitalized |
| META012 | E | yes | 1 | — | **reword** | `@` is written before a compile-time call, never on a value: a static value is used plainly, write `version` | F14: `@readFil` gets E-DECL-1 + AN001; say "not a compile-time operation: did you mean `@readFile`?" |
| META013 | E | yes | 2 | — | good | `9007199254740000 * 2` overflows at compile time: compile-time arithmetic follows the target (`bun`) |  |
| MOD001 | E | yes | 1+2o | — | **wrong** | (source)  | F7: sibling `my-utils.fa` of a standalone file accepted (k10) |
| MOD002 | E | yes | 2+2o | — | good | (source) `{this}` and `{that}` differ only by case: some file systems can't tell them apart, rename one of the |  |
| MOD003 | E | yes | 2+1p+1o | — | good | (source) `{name}` names both `{on}` and `{other}`: rename one of them |  |
| MOD004 | E | yes | 1 | — | good | (source) `{text}` is the name of this file, which designates its export: give this declaration another name |  |
| MOD005 | E | yes | 5 | — | **wrong** | each file exports one entity: export a namespace to export several values | F12: `export const x` gets "add `export`" (there is one); say "export a namespace" / "named after the file"; style: "ex:" |
| MOD006 | E | yes | 3 | — | good | (source) a test file holds declarations and tests: move this statement into a test |  |
| MOD007 | E | yes | 10+2o | — | good | (source) the initializer of `{name}` depends on itself |  |
| MOD008 | E | yes | 2 | — | good | (source) an initializer can't propagate an error: handle it with `catch` |  |
| MOD009 | E | yes | 3 | — | good | (source) `{text}` is private: it is only visible inside its namespace |  |
| MOD011 | E | **no** | 0+1o | — | **dead** | — | reported as REA005/META011 (code-map): alias or delete |
| MOD014 | E | yes | 3 | — | good | (source) tests are declared in test files: move it to `{module}.test.fa`, which sees the names of `{module}.fa |  |
| MOD015 | E | yes | 4+1o | — | **reword** | (source) the entry file exports the function the program starts with, ex: `export function = () => { … }` | n23: caret on the comment of line 1; three errors for a script-style entry file (with MOD005, MOD006); style: "ex:" |
| MOD018 | E | yes | 1 | advanced/metaprogramming | good | a module-level `@for` can't declare a name: it would be declared once per item. Declare it before the loop, or |  |
| REA001 | E | yes | 1 | — | good | cyclic dependency between `a` and `b`: a derived value can't depend on itself, even in a branch that doesn't r |  |
| REA002 | E | yes | 2 | — | **reword** | a derived value must be pure, but it uses `console` (line 3): put side effects in a `watch` block | F13: "(line N)" in the text |
| REA003 | E | yes | 3 | — | good | this `watch` block writes `people[…].bestFriendOf`, which it reads: it would run forever. Compute the value wi |  |
| REA004 | E | yes | 1 | — | good | an error can't escape a `watch` block, and this can fail with `ParseError`: handle it with `catch` |  |
| REA005 | E | yes | 1+1o | — | good | A `reactive` value can't be exported: export a function that reads it, or a plain value | style: capitalized |
| REA007 | E | yes | 1 | — | good | `count` is a reactive value: a `for parallel` body can't use it. Copy it with `let` before the loop |  |
| REA010 | E | yes | 1 | — | good | `double` is a derived value: it is computed from the reactive values it reads, change them instead |  |
| REF001 | E | yes | 64+2p+13o | — | **reword** | `someone` was invalidated by `persons.push` (line 10): declare it again with `use someone = persons[1]` | F13: the invalidating call is "(line N)" in the text: make it a related span |
| REF002 | E | yes | 25+1o | — | **reword** | `ana` may have been invalidated by `people.delete` (line 8): find the element again (`use ana = ….find(…)`), o | F13: same as REF001 |
| REF003 | E | yes | 2 | — | good | `a` is still valid: declaring it again is an error, choose another name |  |
| REF004 | E | yes | 8+1p+2o | advanced/owned-references | **reword** | `use` needs a place: `5` has no place to designate. Use `let` to keep a value, or `mutable use` and `reuse` to | §2.6: "needs a place … has no place to designate": say "names an existing value (a variable, a field, an element)" |
| REF005 | E | yes | 3+2o | — | good | only a `mutable use` can be moved with `reuse`: `fixed` is declared with `use` |  |
| REF006 | E | yes | 4+2o | — | good | `best` designates an element of `players`: it can't designate an element of `others` |  |
| REF007 | E | yes | 2+1p+2o | — | good | `current` is a `mutable use`: move it with `reuse current = …` |  |
| REF008 | E | yes | 8 | advanced/owned-references | **reword** | `user` must name a collection or string reached from a parameter of `addressOf` or a module-level value: pass  | §2.6: "this result has no borrowed place" |
| REF009 | E | yes | 13+5o | advanced/owned-references | good | `b.get(0)` is not inside `a`, the declared owner: return an element of `a`, or change the return type |  |
| REF010 | E | yes | 31 | advanced/owned-references, advanced/reference-checking, advanced/relations | **reword** | this type contains stored references outside a relation: borrow with `use`, declare a relation with its revers | F19: mentions `link` fields "not implemented yet"; say "store indices, or declare a relation" |
| REF011 | E | yes | 3+1p+4o | — | good | `current` is a `mutable use`: a closure can't capture it. Copy what you need with `let` first |  |
| REF012 | E | yes | 42+3p+3o | — | good | this call changes `pool[..]` while the loop iterates over `pool`: collect the changes, and apply them after th |  |
| REF013 | E | yes | 27+2p+4o | — | **reword** | `push` changes `numbers`, and its argument `numbers[0]` is inside it: copy it first with `let` | F13/F18: also emitted with REF012 for the same mistake (f04); task line in text |
| REF014 | E | yes | 0 | — | good | (source) `{name}` designates `{text}`, and `{root}` is immutable: the alias is read-only | no test: add a case |
| REF015 | E | yes | 1+1p | — | good | `someone` designates `persons[1]`, and `persons` is immutable: the alias is read-only |  |
| REF016 | E | yes | 21+2o | advanced/reference-checking | **reword** | `word` is a slice view of a range of items: it cannot be assigned as a whole; assign the owning string instead | §2.6: "slice view", "cannot", "owning string"; style: "cannot" |
| REF020 | E | yes | 10+3o | advanced/reference-checking, functions/defining-functions | good | `ana` is not an element of `others`: `delete` takes a reference into it, from `use ana = others.add(…)`, `find |  |
| REF021 | E | yes | 3 | — | **reword** | `count` is captured by copy, since the closure escapes (line 4): the write would be lost. Return the new value | F13: "(line N)" in the text |
| REL001 | E | yes | 4 | — | good | `bestFriend` has no reverse side: declare one on the elements of `people`, for example `bestFriendOf: Pool<peo |  |
| REL002 | E | yes | 1 | — | good | a direct side is optional or a `Pool`: it becomes `none` (or loses its target) when the target is deleted, ex: | style: "ex:" |
| REL003 | E | yes | 2 | — | good | `people` is an `Array`, whose elements move: a relation needs a `Pool`, a `Columnar`, a `GrowingPool` or a `Gr |  |
| REL004 | E | yes | 1 | — | good | `posts` holds `Post` elements: write `Post in posts` |  |
| REL005 | E | yes | 1 | — | good | `towns` is not a collection visible from this type: declare it as a field of the enclosing type, or as a modul |  |
| REL006 | E | yes | 1 | — | good | `bestFriend` already has a reverse side: a direct side has exactly one |  |
| REL007 | E | yes | 1 | — | good | this reverse side doesn't match the direct side `friends`: use `Pool<in people->friends>` |  |
| REL008 | E | yes | 2 | — | good | the elements of `users` have no direct side `author`: a reverse side names a field `(T in c)?` or `Pool<T in c | style: "ex:" |
| REL009 | E | yes | 1 | — | good | `author` points into `users`: its reverse side is declared on the type of the elements of `users` |  |
| REL010 | E | yes | 6 | — | good | this pool is the reverse side of a relation and is read-only: `add` the targets through the direct side of the |  |
| REL011 | E | yes | 1 | — | good | `anaCopy` is a copy: its relation fields are `none` and read-only. Write `bestFriend` through a reference into |  |
| REL012 | E | yes | 4 | — | good | `bao` is an element of `town2`, `ana` of `town1`: a relation links elements of the same owner |  |
| REL013 | E | yes | 1 | — | good | `bestFriendOf` is a side of a relation: it has no initial value (an optional side starts as `none`, a pool sid |  |
| REL015 | E | yes | 1 | — | good | an owner field is optional: it becomes `none` when its target is deleted, ex: `(Node in #nodes)?` | style: "ex:" |
| REL016 | E | yes | 2 | — | good | owner fields point into `self.#items`: change it only through its full path (`self.#items.delete(…)`), so that |  |
| REL018 | E | yes | 1 | — | good | `#items` holds `Item` elements: write `Item in #items` |  |
| REL019 | E | yes | 2 | — | good | `posts` must be declared `mutable`: deleting a target of its relation writes the fields of its elements |  |
| REL021 | E | yes | 2 | — | good | a whole-value assignment keeps the links: write `ana.bestFriend = …` separately |  |
| REQ001 | E | yes | 10+1p+2o | — | **reword** | `greet` requires `Logger`, which is not provided: use `provide Logger = …`, or declare `requires …` in `main` | F16: at the top level suggests `requires` (not allowed there): suggest `provide` |
| REQ002 | E | yes | 1+1p | — | good | the entry function can't declare `requires`: nothing calls it to provide them. Provide them in its body with ` |  |
| REQ004 | E | yes | 3 | — | good | `addBreadcrumb` mutates `Trace`, but the provided value is a temporary: declare it with `mutable value = …`, t |  |
| REQ005 | E | yes | 1 | — | **reword** | a derived value can't use requirements (line 2): give the value as a reactive value instead | F13: "(line N)" in the text |
| REQ006 | E | yes | 1+2o | — | good | (source) `{name}` is a function of the C library: it can't declare `requires`, since C has nothing to provide; |  |
| REQ007 | E | yes | 2+1o | — | good | this function is stored, but `greet` requires `Logger`: a stored function's type must declare the requirements |  |
| REQ010 | E | yes | 1 | — | good | this watch uses `Settings`, provided in this function: its `Subscription` can't leave the block that provides  |  |
| STR001 | E | yes | 3+2o | — | good | a `Comment` has no field `id: Integer`: `Item` needs `{ id: Integer }` |  |
| STR003 | E | yes | 2+2o | — | good | a `Human` has no `email` field: a `Contact` is expected (only a literal may leave an optional field out) |  |
| STR004 | E | yes | 1 | — | good | `penalize` writes `score: Integer` into `player`, whose `score` is an `Integer<min = 0, max = 10>`: a written  |  |
| STR005 | E | yes | 1+1p | — | good | `addJohn` mutates `heroes`, so it needs an `Array<Human>`, not an `Array<SuperHuman>`: make it generic |  |
| STR007 | E | yes | 1 | — | good | `reset` may write `balance`, but `balance` is readonly in `Account`: only the methods of its type change it |  |
| STR010 | E | **no** | 0+1o | — | **dead** | — | reported as E-TYPE-1 (code-map): alias or delete |
| TEST002 | E | yes | 2 | — | good | (source) there is already a test named {quoted} here: give this one another name |  |
| TEST003 | E | yes | 1+1p | — | good | (source) this compile-time test fails: {reason} |  |
| TEST004 | E | yes | 0+5o | — | good | (source) fa test: no test file (`*.test.fa`) in this package |  |
| WCONC001 | W | yes | 0+2o | — | good | (source) asynchronous mode is off, and {name} starts tasks: tasks run to their end when they are started, so t |  |
| AN001 | E | yes | 4+4o | — | **wrong** | these effects in function types is not supported yet by the analyzer | F2/F10/F14: fires for plain mistakes (`{ a: 1 }`, `?:`, `in`, `x?`, `if x:`, `name?:` param, `forEach`, `slice`, `std.fs`); say "not implemented yet" without "by the analyzer", with the workaround |
| AN002 | E | yes | 3+3o | — | **reword** | Unexpected `)` after the end of the statement: a statement ends at the end of its line. Add an operator, or st | F1/F10/F11: catch-all code for ~160 parser messages; give each class its code; cut cascades; sort by position; style: capitalized |
| AN003 | W | yes | 0 | — | **reword** | (source)  | no test; catch-all code for parser warnings; no test: add a case |
| AN004 | E | yes | 3+7o | operations/json | good | `TcpStream#read` doesn't exist on target `web`: branch with `@if target != "web"`, or build for another target |  |
| AN005 | E | yes | 2 | — | **reword** | `Json.parse` reads and writes JSON values, and a `(Integer) => Integer` is not one | F19: a message has no fix: add one |
