# Implementation brief: values, operations and control flow

Working brief for the engineers of the type checker (`fa-checker`), the JavaScript backend and the LLVM backend. It is a design record, not a language page: the pages of `documentation/` stay the source of truth. Where this brief **decides** something the pages leave open, the decision is marked **[D-n]** and listed in [Ambiguities and decisions](#ambiguities-and-decisions), for the project lead to accept or override.

Scope: `documentation/getting-started/*`, `documentation/values/*`, `documentation/operations/*`, `documentation/control-flow/*`, cross-checked against `documentation/reference/language-reference.md`. Other pages (unions, functions, reference checking, collections, concurrency) are only summarized where a feature of this scope depends on them. Related records: `design/spec-review.md` (decisions of 2026-11-01), `design/compilation-with-llvm.md` (native layouts and milestones).

State of the parser: checked on 2026-11-01 against `packages/parser` (commit `ac98879`). Every behaviour quoted as "parser:" was observed by running it.

## Contents

1. [Conventions of this brief](#1-conventions-of-this-brief)
2. [Global model](#2-global-model): types and facts, expected types, sinks, places, `Never`/`Void`/`None`, stops, printing
3. [Lexical rules and statements](#3-lexical-rules-and-statements)
4. [Precedence and the parser tree](#4-precedence-and-the-parser-tree)
5. [Declarations and assignment](#5-declarations-and-assignment): `let`, `mutable`, `use`, `mutable use`, `reuse`, `static`, shadowing
6. [Numbers](#6-numbers): literals, integer types as ranges, range analysis, arithmetic, overflow, `/`, `//`, `%%`, `**`, floats, `BigInt`, percentages, conversions, wrapping/checked/saturating, bitwise, `Math`, compound assignment
7. [Strings and characters](#7-strings-and-characters)
8. [Booleans](#8-booleans)
9. [`none` and optionals](#9-none-and-optionals): `?.`, `??`
10. [Tuples](#10-tuples)
11. [Objects](#11-objects): literals, shorthand, spread, `.=`, `readonly`, `#private`, equality
12. [Ranges](#12-ranges)
13. [Comparison, equality, `is`, membership, logic](#13-comparison-equality-is-membership-logic)
14. [Narrowing](#14-narrowing)
15. [Errors](#15-errors): `throw`, `try`, `catch`, built-in errors, stops, `assert`, no `!`
16. [Destructuring](#16-destructuring)
17. [Conditionals](#17-conditionals): `if`, `if` as an expression, `when`
18. [Loops](#18-loops): `for`, `while`, `while let`, `loop`, `break`, `continue`
19. [`do` blocks and block values](#19-do-blocks-and-block-values)
20. [Ambiguities and decisions](#ambiguities-and-decisions)
21. [Dependency order](#dependency-order)

---

## 1. Conventions of this brief

- **Diagnostics** have stable identifiers so that tests can assert them: `E-<AREA>-<n>` for errors, `W-<AREA>-<n>` for warnings. Areas: `LEX`, `DECL`, `NUM`, `STR`, `BOOL`, `OPT`, `TUP`, `OBJ`, `RNG`, `CMP`, `NAR`, `ERR`, `DST`, `IF`, `WHEN`, `LOOP`, `DO`. The message sketches are indicative; the identifier and the condition are normative for the test suite. Every message names the construct, the types involved and the fix, as the documentation requires ("clear error messages, written to be acted upon").
- **Tests** are written `T-<AREA>-<n>`. A test is a small standalone file; in this brief, ` · ` separates lines of the file (a newline in the real test). `→` gives the expected standard output (lines separated by ` / `), `⇒` the expected diagnostic, `⇒ stop` a [stop](#27-stops) (exit code 101, message on stderr), `⇒ exit 1` an escaped error. Unless written otherwise, every valid test must give the same output on every target (`native`, JavaScript).
- `console.log(a, b, …)` prints the `toString` of each argument, separated by one space, followed by a newline (see [printing](#28-printing-tostring)). The tests rely on it.
- "Interval" means a closed interval of mathematical integers `[lo, hi]`, with `lo`/`hi` possibly `−∞`/`+∞` in the checker. The checker computes intervals with arbitrary precision (or saturating at ±2^200), never with machine integers.
- "Base type" of a ranged integer type: the type it refines (`Integer` for `Integer(min = 0)`, `Int8` for `Int8(min = 0)`).
- "Kind" of a number: integer (all integer types, ranged or not), `Float32`, `Float64`, `BigInt`, `Percentage`. Integer types of different widths are the **same kind**.

---

## 2. Global model

### 2.1 Types and facts

The checker keeps two separate things about every value (`values/declaring-values#inferred-types`, spec review T1):

- its **type**: what `Type(value)` returns and what generic inference uses. An unannotated binding gets the **widened** type of its initializer: a literal gives its base type (`42` → `Integer`, `"ready"` → `String`, `true` → `Boolean`, `1.5` → `Float`), never a literal type. An annotated binding has the annotated type. A `static` value keeps its literal type.
- **flow facts** about each place: exact literal value, integer interval plus a set of excluded values, union member it was narrowed to (including `None`/not-`None`), length of a collection (and relational facts `index < collection.length`), validity of references. Facts make more programs valid without changing the type. They are used for assignability into ranged types, exhaustiveness, index and divisor proofs, overflow checks, always-true/false warnings and reference invalidation.

Facts last while the place is not mutated: an assignment **replaces** the facts of the place by those of the new value; any other mutation (a `mutates` call, a write below the place) **drops** them; a `mutates p` call also drops the facts of every path below `p`. At an `await`, the facts of module-level values are dropped.

The fact lattice per integer place: `(interval, excluded: small set of integers)`; join = interval hull + intersection of excluded sets; per string/boolean/enum place: an exact literal or "unknown" (join of two different literals is "unknown" **[D-30]**); per optional/union place: a set of possible members.

### 2.2 Expected types and literal adaptation

Checking is bidirectional. An **expected type** flows down from: a declaration annotation, an assignment target, a parameter type at a call, a return type, a field type in an object literal, the element type of a collection literal with an expected type, the other operand of an arithmetic or comparison operator (for literals only), the other branches of an `if`/`when` (for literals only, [D-29]).

A **literal adapts** to its expected type (`values/numbers#literals`):

| Literal | Adapts to | Default without expected type |
| --- | --- | --- |
| integer (`42`, `0xFF`, `-128`) | any integer type whose range contains the value; `Float32`/`Float64` when exactly representable **[D-20]**; `BigInt` | `Integer` |
| float (`1.5`, `1e6`) | `Float32` (rounded to nearest, error if it overflows to infinity), `Float64` | `Float` |
| `12n` | `BigInt` only | `BigInt` |
| `50%`, `12.5%` | `Percentage` only | `Percentage` |
| string (`"a"`) | `String`; `Character` when it holds exactly one grapheme cluster | `String` |
| `true`/`false`, `none` | `Boolean`; any optional | `Boolean`; error without expected type [D-21] |

Never: an integer literal never becomes a `Percentage`; a float literal never becomes an integer; `none` never becomes a non-optional type.

**Constant expressions.** An expression made only of number literals, `static` values and the operators `+ - * / // %% **`, unary `-` and the `%` suffix is evaluated exactly by the checker (arbitrary precision for integers, IEEE for floats). Its exact value is a fact, so `let x: Int8 = 100 + 27` is valid and `let e: Int8 = 100 + 100` is `E-NUM-10`.

### 2.3 Sinks

A **sink** is a position where a value lands in a typed slot: the initializer of a declaration (its annotation, or its widened type), the right side of an assignment or compound assignment (the target type), an argument (the parameter type), a returned value (the return type), a field value in an object literal, an element of a collection literal, the receiver of a method call, an interpolated value. The checks of [integer ranges](#64-integer-typing-ranges-and-conversions) happen at sinks.

### 2.4 Places and mutability

A **place** is a local value, a parameter, a module-level value, or a path below one (`a.b`, `a[i]`, `a[..].b`). A place is **mutable** when its root is declared `mutable` (or is a `mutable use`/`use` of a mutable place, or a parameter declared with a covering `mutates` effect, or a mutable loop variable), and no step of the path is `readonly` from the current context, `#private` from outside, a `Set` element or a `Map` key. `let` is deeply immutable: no assignment to it or anything below it, no mutating method call on it.

### 2.4 bis General type mismatch

`E-TYPE-1`: a value whose type is not accepted by the expected type, when no more specific diagnostic applies (`let s: String = 5`, `let r: Float = 50%`, passing a `Point` where a `String` is expected). The message names both types and, when one exists, the explicit conversion (`Float(…)`, `.toString()`).

### 2.5 `Never`, `Void`, `None`

- `Never` (bottom): the type of `return`, `throw`, `break`, `continue`, a `loop` without a reachable `break`, a call to a function returning `Never`. It satisfies every type and disappears from unions. Code after a `Never` expression in the same block is unreachable: `W-DO-2` (unreachable code).
- `Void`: the result of a function without return type, of an assignment, of a declaration, of a `for`/`while`, of a statement `if`/`when`. A `Void` expression can't be bound (`let x = log("a")` → `E-DECL-9`), interpolated, compared, passed where a value is expected, or be the value of a block that is bound.
- `None`: the type of `none`, member of every optional. `T?` = `T | None` = `Optional(T)`, `T??` = `T?`, `Never?` = `None`, `Void?` → `E-OPT-9`.

### 2.6 Effects used by this scope

Only what values and control flow need: `throws` (section 15), `mutates`/`invalidates` (they end narrowings and drop facts, section 14), loops over collections (section 18). The full rules are in `advanced/reference-checking` and `functions/defining-functions`.

### 2.7 Stops

A **stop** (`values/errors#stops`) is the failure of a runtime check that the compiler could not prove away: integer overflow (including a failed range check at a sink), a false `assert` outside a test, and [D-26] stack overflow and out-of-memory. A stop:

- prints `stop: <reason>` and `  at <file>:<line>:<column>` to stderr (reason examples: `integer overflow: 300 doesn't fit in UInt8`, `assertion failed: total == 10`),
- exits with code `101`, without unwinding and without running destructors,
- can't be caught, thrown or declared; fails the current test in a test file.

Native: a call to the runtime routine `fa_stop(reason_id, location)` (`noreturn`, `cold`). JavaScript: `throw new FaStop(reason, location)`; every compiled `catch` rethrows it; the entry point catches it, prints, and calls `process.exit(101)` on server runtimes (on the web: reports in the console, stops the program). A JavaScript `RangeError` for call-stack overflow is converted to a stop at the entry point [D-26].

### 2.8 Printing (`toString`)

Every interpolated value and every `console.log` argument goes through `toString`. The documentation only fixes a few outputs (`{ name = "John" }`, `[2, 3, 4]`, `ApiError.NetworkError { message = "Connection lost" }`). This brief fixes the rest **[D-5]**, identically on every target:

| Value | Output |
| --- | --- |
| integer types | decimal, `-` for negatives: `-128` |
| `BigInt` | decimal, no `n`: `12` |
| `Float64` | ECMAScript `Number::toString` (shortest round-trip digits; exponent form when the exponent is ≥ 21 or ≤ −7): `2`, `0.5`, `3.142857142857143`, `0.30000000000000004`, `1e+21`, `1e-7`, `NaN`, `Infinity`, `-Infinity`; `-0.0` prints `0` |
| `Float32` | the shortest decimal that round-trips **as a `Float32`**, then the same formatting: `Float32` `0.1` prints `0.1` [D-5] |
| `Percentage` | `fraction × 100` formatted like a `Float64` after rounding to 15 significant digits, then `%`: `50%`, `12.5%`, `10%` (not `10.000000000000002%`) [D-6] |
| `Boolean` | `true`, `false` |
| `none` | `none` |
| `String`, `Character` at top level | raw text |
| `String`, `Character` nested in a composite | quoted, with Fa escapes: `"a\"b"` |
| tuple | `(1, "one")` |
| array (and growing array) | `[1, 2, 3]`, `[]` |
| object | `{ x = 1, y = 2 }` in declaration order, `{}` when empty; no type name (typing is structural) |
| named union member, error | qualified name, then the payload: `Shape.Circle { radius = 1 }`, `Shape.Point`, `UserId.User(12)`, `ApiError.Timeout` |
| `Range` | `1..3`, `0..=10`, `5..`, `..10`, `..` |
| function value | no `toString`: interpolating it is `E-STR-12` [D-39] |

Native: generated `toString<Shape>` per shape writing into a string builder; float formatting with a shortest-round-trip algorithm (Ryu) plus the ECMAScript layout rules. JavaScript: numbers and integers use `String(x)` (or a template literal), with helpers only for `Float32`, `Percentage`, optionals (`none` must not print `undefined`) and composites.

---

## 3. Lexical rules and statements

Source: `getting-started/syntax-basics`.

### Syntax (as parsed)

- UTF-8 source, `.fa` extension. Indentation is not significant [D-63]: tabs are the canonical style, enforced by `fa format`, not by the checker.
- Comments: `-- …` to end of line; `--- … ---` block comment (multi-line, or on a single line `--- like this ---`). Parser: `let x = 1 --- c --- + 2` is `1 + 2`.
- **Documentation comments**: a comment immediately above a declaration (no blank line in between) is attached to it. The parser must keep the comment text on the declaration node (needed by the language server and `@typeToString` of docs); a blank line detaches it.
- Statements are separated by newlines; no semicolons (`;` → `E-LEX-1`).
- **Continuation**: an expression continues on the next line after an opening bracket, after a binary operator, after a comma; a line starting with `.`, `?.` or a binary operator other than `-` continues the previous line. A line starting with `-`, `(` or `[` starts a new statement. `else` must be on the line of the closing `}`.
- Inside brackets, newlines and commas both separate items.
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*`, ASCII only; keywords can't be identifiers. Prefixes `#` (private) and `@` (compile-time). `self`/`Self` are special names; `skip`/`only` are keywords only after `test`.
- Reserved keywords: `and as assert async await break catch continue derived do else export false for function if in invalidates is let loop mutable mutates namespace none not or parallel provide reactive readonly requires return reuse start static test then throw throws true try type use watch when while with`.
- **Braces**: after `=>`, `if`, `else`, `do`, `loop`, `for`, `while`, `watch`, `test`, `namespace`, a `{` opens a block; after a type name, an object literal of that type; elsewhere in an expression, an object literal; where a key set is expected (`Pick(User, { name })`), a key set.
- There is no `!` token in the language: parser reports "Unexpected character '!'. Use `not`…" (keep, as `E-LEX-4`), except inside `!=`.

### Static semantics: errors

| Id | Condition | Message sketch |
| --- | --- | --- |
| E-LEX-1 | `;` in source | Fa has no semicolons: put each statement on its own line |
| E-LEX-2 | a non-ASCII character outside strings and comments, or an identifier starting with a digit | identifiers are ASCII |
| E-LEX-3 | a keyword used as a name (`let if = 1`) | `if` is a keyword |
| E-LEX-4 | `!` not followed by `=` (`!x`, `x!`) | Fa has no `!`: use `not` for booleans, `is not none` or `??` for optionals |
| E-LEX-5 | `else` at the start of a line | write `} else {` on one line (parser today silently produces garbage: an `if` then a dangling block) |
| E-LEX-6 | unterminated string or block comment | — |
| W-LEX-1 | a comment above a declaration separated by a blank line *and* starting with `---` (probably meant as documentation) | optional lint, low priority |

### Runtime semantics

None.

### Lowering

- JavaScript: comments dropped; doc comments may be emitted as JSDoc on exported declarations (optional).
- Native: nothing.

### Tests

- T-LEX-1 `let a = 1 · let b = 2 · console.log(a + b)` → `3`
- T-LEX-2 `let total = · ⇥1 + · ⇥2 · console.log(total)` → `3` (continuation after `=` and after `+`)
- T-LEX-3 `let isAllowed = false · ⇥or true · console.log(isAllowed)` → `true` (line starting with `or`)
- T-LEX-4 `let x = [1, 2, 3] · ⇥.length · console.log(x)` → `3`
- T-LEX-5 `let a = 5 · -1 · console.log(a)` → `5` and `W-DO-1` (the `-1` line is its own statement, a pure expression)
- T-LEX-6 `let a = 4 · let b = a --b · console.log(b)` → `4` (`--b` is a comment)
- T-LEX-7 `--- one-line block --- console.log("x")` → `x`
- T-LEX-8 `let a = 1; let b = 2` ⇒ E-LEX-1
- T-LEX-9 `let été = 1` ⇒ E-LEX-2
- T-LEX-10 `let loop = 1` ⇒ E-LEX-3
- T-LEX-11 `if not true { }` valid; `if !true { }` ⇒ E-LEX-4
- T-LEX-12 `let v: Integer? = 1 · console.log(v!)` ⇒ E-LEX-4
- T-LEX-13 `if true { · } · else { · }` ⇒ E-LEX-5
- T-LEX-14 `let s = "abc` ⇒ E-LEX-6
- T-LEX-15 `let a = [ · 1 · 2 · 3 · ] · console.log(a)` → `[1, 2, 3]`
- T-LEX-16 `let skip = 1 · console.log(skip)` → `1` (`skip` is contextual)
- T-LEX-17 Documentation: `-- doc · let a = 1` attaches `doc` to `a`; `-- note · · let a = 1` doesn't (checked through the language server API).
- T-LEX-18 `let _1 = 3 · console.log(_1)` → `3` (`_1` is an identifier, [D-10])

---

## 4. Precedence and the parser tree

Canonical table (`operations/arithmetic#order-of-operations`), tightest first:

| # | Operators | Assoc. |
| --- | --- | --- |
| 1 | `a.b` `a?.b` `a(b)` `a[b]` `a with (b = 1)` | left |
| 2 | postfix `%` | — |
| 3 | prefix `-` | — |
| 4 | `**` | **right** |
| 5 | `*` `/` `//` `%%` | left |
| 6 | `+` `-` | left |
| 7 | `..` `..=` | none |
| 8 | `<` `<=` `>` `>=` (chainable) | chain |
| 9 | `==` `!=` `is` `is not` `is in` `is not in` | none (chaining `==` is E-CMP-3) |
| 10 | `not` | prefix |
| 11 | `and` | left |
| 12 | `or` | left |
| 13 | `??` | left |
| 14 | `then` | left |
| 15 | `try` `await` `start` (cover everything on their right up to a `catch`) | prefix |
| 16 | `catch` (handler extends as far right as possible) | — |
| 17 | `return` `throw` `break` (cover everything on their right) | prefix |
| 18 | `=` `+=` `-=` `*=` `**=` `/=` `//=` `%%=` `.=` | statement |

In `when` patterns and after `is`, `|` is the loosest and `as` applies to the whole alternative.

A minus sign written immediately before a number literal is part of the constant (tokenized as a negative literal): `-2 ** 2` is `(-2) ** 2` = `4`, and `-2.4.abs()` is `(-2.4).abs()` = `2.4` [D-53]. With a name, `-x ** 2` is `(-x) ** 2` by precedence; `-(x ** 2)` is the opposite of the square.

### Parser discrepancies the checker must know (fix in the parser first)

Observed with the current parser; each is a parser follow-up, listed again in the ambiguities:

| Input | Parser today | Required |
| --- | --- | --- |
| `not a and b` | `not (a and b)` | `(not a) and b` (doc table: `not` binds tighter than `and`/`or`) [D-1] |
| `x is "y" \| "yes"`, `x is Integer \| String`, `x is 1 \| 2..5` | `(x is "y") \| "yes"` | `x is ("y" \| "yes")`: the right side of `is` is a pattern parsed with `\|` alternatives [D-2] |
| `a < b <= c` | `((a < b) <= c)` (indistinguishable from a chain except for `Group` nodes) | a `ComparisonChain { operands, operators }` node [D-3] |
| `a == b == c` | `Equal { operands: [a, b, c] }` | E-CMP-3 |
| `2 ** 3 ** 2` | `Power { operands: [2, 3, 2] }` | interpret right-to-left (`2 ** 9`) — fine if every consumer knows; better a nested right-associative tree |
| `try a // b catch e => 0` | `try ((a // b) catch …)` | E-ERR-6 (`try` and `catch` on the same expression) |
| `a-1`, `a -1` | panic (negative-literal token in infix position) | subtraction `a - 1` [D-14] |
| `18446744073709551615`, `9223372036854775808` | panic (`Node::Integer(i64)`) | store integer literals with arbitrary precision (text or `i128`/`u128` + `BigInt`) [D-13] |
| `1_`, `1__0` | accepted as `1`, `10` | E-NUM-2 [D-11] |
| `.5`, `5.` | dangling tokens | E-NUM-1 |
| `0XFF`, `1E6` | `0` then identifier `XFF` | E-NUM-3 [D-12] |
| `"a } b"` | accepted (`}` literal) | E-STR-4 [D-15] |
| `"a\r\nb"` (CRLF in the source) | keeps `\r\n` | `\n` [D-16] |
| `"{user.name ?? "anonymous"}"` | panic | nested strings and objects in interpolation |
| `let { name, ...details } = person`, `let [a, ...rest] = xs` | panic | rest patterns |
| `x += y += z` | accepted | E-DECL-8 |
| `let x: Integer` (no value) | accepted | E-DECL-10 [D-35] |
| `while let { a } = s.pop()` | accepted | E-LOOP-9 [D-34] |
| any syntax error | `panic!` | diagnostics with spans (the checker and the language server need a tree with error nodes) |

---

## 5. Declarations and assignment

Sources: `values/declaring-values`, `getting-started/syntax-basics#declarations`, reference "Declarations", `advanced/reference-checking#use-and-mutable-use`.

### Syntax (as parsed)

```
let <name>[: <Type>] = <expr>                  -- Node::Let
mutable <name>[: <Type>] = <expr>              -- Node::Mutable
use <name>[: <Type>] = <place-or-ref-call>     -- Node::Use { is_mutable: false }
mutable use <name>[: <Type>] = <place-or-ref-call>   -- Node::Use { is_mutable: true }
reuse <name> = <place-or-ref-call | none>      -- Node::Reuse (a statement)
static <name>[: <Type>] = <compile-time expr>  -- Node::Static
<place> = <expr>                               -- Node::Assignment
<place> <op>= <expr>                           -- Node::CompoundAssignment (+= -= *= **= /= //= %%= .=)
let|mutable|use <pattern> = <expr>             -- Node::Destructuring (section 16)
```

### Static semantics

**Scopes and names.**

- A name is introduced only by a declaration keyword. A bare `name = value` is an assignment; if `name` is not declared: `E-DECL-1`.
- The right-hand side of a declaration never sees the name being declared: `let x = x + 1` refers to an `x` of an enclosing scope (valid shadowing in an inner scope), and is `E-DECL-1` if there is none.
- Shadowing a name of an **enclosing** scope is allowed (a prelude name: `W-DECL-1`, spec review AR1). Declaring a name twice in the **same** scope: `E-DECL-2`, except a `use` reference that is invalidated or maybe invalidated, which may be declared again with `use` (redeclaring a valid reference is `E-DECL-2`). A `mutable use` is never declared again (`E-DECL-12`: use `reuse`). A redeclaration inside an inner block is a new name that ends with the block.
- Function parameters share the scope of the body's top-level declarations: `let a = 1` in a body with a parameter `a` is `E-DECL-2`.
- An initializer is required for `let`, `mutable`, `use` [D-35]: `E-DECL-10`.

**`let` and `mutable`.**

- Type: the annotation if present (the initializer is checked against it); otherwise the widened type of the initializer. The initializer's facts become the place's facts.
- `let x = none` and `mutable x = none` without annotation: `E-DECL-11` [D-21]. `let x = []` without annotation and without element type: `E-DECL-11` (no element type to infer).
- Binding a `Void` expression: `E-DECL-9`. Binding a lambda: `E-DECL-6` ("use `function`"). Annotating with `Void`: `E-DECL-9`.
- `let` is deeply immutable: assigning it (`E-DECL-3`), assigning a field or element below it (`E-DECL-4`), calling a method that `mutates self` on it or passing it to a `mutates` parameter (`E-DECL-5`), compound-assigning it (`E-DECL-3`).
- `mutable` copies its initializer; assignment `x = v` replaces the whole value (target type must accept `v`; facts replaced).
- Annotating with the same type as an explicitly typed object literal (`let p: Point = Point { … }`) is `W-OBJ-1` (section 11).

**Assignment.** `target = value`: `target` must be a mutable place (`E-DECL-3`/`E-DECL-4` otherwise); `value` is checked against the target's declared type (not its facts); the place's facts become the value's facts; narrowings of the place and of every path below it end (section 14); references into the old value are invalidated per `advanced/reference-checking`. Assignments are statements of type `Void`; chains `a = b = c` (parser: panic) and `x += y += z` are `E-DECL-8`. A parameter can't be assigned (`E-DECL-13`), even with `mutates` (only its inside can be changed). A primitive parameter can't be declared in `mutates` (`E-DECL-14`).

`try` before an assignment is only allowed when the **target** can fail (an index or key not proven): `try scores[index] = 0`. When only the value can fail, write `x = try f()`; `try x = f()` is `E-DECL-15` [D-87]. Likewise `try let x = …` is only for array destructuring (section 16): `try let x = items[i]` is `E-DECL-15` ("write `let x = try items[i]`").

**`use`** (alias, no copy):

- The initializer must be a **place** (`a`, `a.b`, `a[i]`, `text[1..3]`) or a **call that returns a reference** (a return type with `in`, `pool.add`, `scores.get(key)`). Anything else (`use x = 5`, `use p = makePerson()`, `use c = if r { a } else { b }`): `E-DECL-7`. A `try` before the call or place is allowed (`use x = try items[i]`).
- Type: the designated type, with its owner when the place is inside a collection or a string (`Person in people`, `String in text`, `(Integer in scores)?`); otherwise a plain alias. An annotation views the value through a narrower structural type (`use human: Human = hero`): no copy, extra fields invisible.
- Writing through a `use` requires the designated place to be mutable (`E-DECL-4`).
- `use` on a place holding a relation reference binds the target.
- Validity: invalidated by a mutation of what it points into or an invalidation of its place; using it afterwards is `E-DECL-16` (message: "`first` was invalidated by `numbers.push`: declare it again with `use first = numbers[...]`"). Full rules: `advanced/reference-checking`.

**`mutable use` and `reuse`:**

- `mutable use r = place` declares a movable reference. `reuse r = other` moves it: `other` must be of the same form as a `use` initializer (or `none` when the type is optional), with the same designated type **and the same owner** (`E-DECL-17`). `reuse` on a `use` or on a non-reference: `E-DECL-18`. `reuse` acts on the declaration even from an inner block (it is not a declaration). After `reuse`, the reference is valid.
- `=` through a `mutable use` writes into the designated value.
- A `mutable use` may designate places of an immutable collection (read-only places).
- `use other = r` designates what `r` designates at that moment.
- A closure can't capture a `mutable use` (`E-DECL-19`).

**`static`**: compile-time value, keeps its literal type, used plainly; its initializer must be compile-time evaluable (`E-DECL-20`). Details in `advanced/metaprogramming` (out of scope).

**Module-level values**: declared at the top of a module or as namespace members, stable address, can be replaced (even primitives) from functions that declare them in `mutates`.

### Diagnostics

| Id | Condition |
| --- | --- |
| E-DECL-1 | assignment to, or use of, an undeclared name |
| E-DECL-2 | same name declared twice in one scope (not an invalidated `use`) |
| E-DECL-3 | assignment or compound assignment to a `let`, a `use` of an immutable place, a loop variable copy, a parameter |
| E-DECL-4 | write below an immutable place (`myObject.foo += 1` with `let myObject`) |
| E-DECL-5 | mutating call on an immutable value (`increaseFoo(myValue)`: "`increaseFoo` mutates `input`, but `myValue` is immutable") |
| E-DECL-6 | `let f = (x) => …` |
| E-DECL-7 | `use` on a value that is neither a place nor a reference-returning call |
| E-DECL-8 | chained assignment |
| E-DECL-9 | binding or annotating `Void` |
| E-DECL-10 | declaration without initializer |
| E-DECL-11 | type can't be inferred (`none`, `[]` without context) |
| E-DECL-12 | redeclaring a `mutable use` |
| E-DECL-13 | assigning a parameter as a whole |
| E-DECL-14 | `mutates` on a primitive parameter |
| E-DECL-15 | `try` before an assignment or declaration whose target can't fail |
| E-DECL-16 | use of an invalidated reference |
| E-DECL-17 | `reuse` with a different type or owner |
| E-DECL-18 | `reuse` on something that is not a `mutable use` |
| E-DECL-19 | closure capturing a `mutable use` |
| E-DECL-20 | `static` initializer not evaluable at compile time |
| W-DECL-1 | shadowing a prelude name |

### Runtime semantics

- `let`/`mutable`: evaluate the initializer, copy it into the new place (a move when the source is dead afterwards; never observable).
- Assignment: evaluate the right side, then the target's container/index (left-to-right order: **target path first, then value** — [D-88]: the target's sub-expressions (`a[f()]`) are evaluated before the value, as in JavaScript), then destroy the old value and store.
- Values are destroyed at the end of their scope, in reverse declaration order.

### Lowering

Native (from `values/declaring-values:::implementation`): `let`/`mutable` copy (`memcpy` for flat values, `copy<T>` for owning ones, a move when the source is dead); `use` is a pointer (a slice is pointer + length); `mutable use` is a pointer variable that `reuse` overwrites. Scope exit: cleanup stack, drops in reverse order (see `compilation-with-llvm.md#scope-exit-without-unwinding`).

JavaScript:

- `let` → `const`, `mutable` → `let`. Objects and collections are copied by a generated copy function unless the compiler proves no later write to either side is observable (then a plain assignment). Primitives and strings are never copied.
- `use` of an object/collection: the JS reference. `use` of a primitive place: the container and key are evaluated once into locals, and each use of the alias is rewritten to the place (`use score = player.score` then `score += 1` → `const $c = player; $c.score += 1`). `use` of a primitive **local** (`use a = count`): every use is rewritten to `count`.
- `mutable use` of a primitive place: a pair of locals (container, key), reassigned by `reuse`.
- Facts and inferred types produce no code; they only remove checks.

### Tests

Valid:

- T-DECL-1 `let answer = 42 · console.log(answer)` → `42`
- T-DECL-2 `mutable count = 0 · count = 1 · count += 2 · console.log(count)` → `3`
- T-DECL-3 `let value = 1 · do { · let value = 2 · console.log(value) · } · console.log(value)` → `2 / 1`
- T-DECL-4 `let x = 1 · do { · let x = x + 1 · console.log(x) · }` → `2` (right side sees the outer `x`)
- T-DECL-5 `let original = { count = 1 } · mutable copy = original · copy.count = 2 · console.log(original.count, copy.count)` → `1 2`
- T-DECL-6 `mutable people = [{ name = "Ana" }, { name = "Bao" }] · use r = people[1] · r.name = "Cleo" · console.log(people[1].name)` → `Cleo`
- T-DECL-7 `mutable people = [{ name = "Ana" }, { name = "Bao" }] · let c = people[1] · mutable d = c · d.name = "X" · console.log(people[1].name)` → `Bao`
- T-DECL-8 `mutable players = [{ name = "Ana", score = 3 }, { name = "Bao", score = 7 }] · mutable use best = players[0] · for p in players { · if p.score > best.score { · reuse best = p · } · } · best.score += 10 · console.log(players[1].score)` → `17`
- T-DECL-9 `mutable s = { a = 1 } · use x = s.a · x += 1 · console.log(s.a)` → `2` (primitive alias, JS rewrite)
- T-DECL-10 `let five = 5 · let score: Integer(min = 0, max = 10) = five · console.log(score)` → `5` (fact)
- T-DECL-11 `let n: Integer? = none · console.log(n)` → `none`
- T-DECL-12 `let items = [10, 20, 30] · let last = items[2] · console.log(last)` → `30` (length fact, no `try`)
- T-DECL-13 `mutable xs = [1, 2] · use first = xs[0] · xs.push(3) · use first = xs[0] · console.log(first)` → `1` (declared again after invalidation)

Invalid:

- T-DECL-20 `total = 12` ⇒ E-DECL-1
- T-DECL-21 `let value = 1 · let value = 2` ⇒ E-DECL-2
- T-DECL-22 `let myValue = 12 · myValue += 1` ⇒ E-DECL-3
- T-DECL-23 `let o = { foo = 12 } · o.foo += 1` ⇒ E-DECL-4
- T-DECL-24 `function inc = (input: { foo: Integer }) mutates input => { · input.foo += 1 · } · let v = { foo = 1 } · inc(v)` ⇒ E-DECL-5
- T-DECL-25 `let add = (a: Integer, b: Integer) => a + b` ⇒ E-DECL-6
- T-DECL-26 `use count = 5` ⇒ E-DECL-7; `use c = if true { a } else { b }` ⇒ E-DECL-7
- T-DECL-27 `mutable a = 1 · mutable b = 2 · a = b = 3` ⇒ E-DECL-8
- T-DECL-28 `function log = (m: String) => { · console.log(m) · } · let r = log("a")` ⇒ E-DECL-9
- T-DECL-29 `mutable x: Integer` ⇒ E-DECL-10
- T-DECL-30 `mutable x = none` ⇒ E-DECL-11
- T-DECL-31 `mutable xs = [1] · mutable use r = xs[0] · mutable use r = xs[0]` ⇒ E-DECL-12
- T-DECL-32 `function f = (input: { a: Integer }) mutates input => { · input = { a = 1 } · }` ⇒ E-DECL-13
- T-DECL-33 `function inc = (value: Integer) mutates value => { · value += 1 · }` ⇒ E-DECL-14
- T-DECL-34 `mutable x = 0 · try x = 1` ⇒ E-DECL-15
- T-DECL-35 `mutable v = [1, 2, 3] · use first = v[0] · v.push(4) · console.log(first)` ⇒ E-DECL-16
- T-DECL-36 `mutable a = [{ s = 1 }] · let b = [{ s = 2 }] · mutable use r = a[0] · reuse r = b[0]` ⇒ E-DECL-17 (other owner)
- T-DECL-37 `mutable a = [1] · use r = a[0] · reuse r = a[0]` ⇒ E-DECL-18
- T-DECL-38 `function f = (a: Integer) => { · let a = 2 · }` ⇒ E-DECL-2
- T-DECL-39 `let console = 1` ⇒ W-DECL-1

---

## 6. Numbers

Sources: `values/numbers` (incl. `:::implementation`), `operations/arithmetic` (incl. `:::implementation`), `types/type-conversion#numbers`, reference "Numbers".

### 6.1 Literals

**Syntax (as parsed).**

| Form | Examples | Node |
| --- | --- | --- |
| decimal integer | `42`, `8_100_000_000` | `Integer(i64)` (must become arbitrary precision, [D-13]) |
| binary / octal / hex | `0b101010`, `0o52`, `0x2A`, `0xFF_88_00` | `Integer` |
| negative integer (minus glued in prefix position) | `-128` | `Integer(-128)` |
| float | `0.5`, `1.5e9`, `2.5e-3`, `1.5e+9`, `1e6` | `Number(f64)` |
| big integer | `12n`, `0xFFn` | `BigInteger(text)` |
| percentage | `50%`, `12.5%`, `1e6%`, `(expr)%` | `Percentage { value }` |

Lexical rules: a float has digits on both sides of `.`, or an exponent, or both; an exponent makes a float. Prefixes `0b`, `0o`, `0x` and the exponent `e` are lowercase; hex digits may be either case [D-12]. `_` only between two digits. `n` only on integer literals (any base). `%` on any integer or float literal, or on a parenthesized expression.

**Diagnostics.**

| Id | Condition | Message sketch |
| --- | --- | --- |
| E-NUM-1 | `.5`, `5.` | write `0.5` / `5.0` |
| E-NUM-2 | `1_`, `1__0`, `0x_FF`, `1_.5`, `1._5` | an underscore must sit between two digits |
| E-NUM-3 | `0XFF`, `0B1`, `1E6`, `0b2`, `0o8`, `0xG` | invalid digit / prefixes and exponents are lowercase |
| E-NUM-4 | `12.5n`, `1e3n` | `n` only applies to integer literals |
| E-NUM-5 | `x%` where `x` is not a number literal nor a parenthesized expression (`fifty%`, `f()%`, `a.b%`) | `%` only applies to a literal or a parenthesized expression: write `(fifty)%` |
| E-NUM-6 | integer literal that doesn't fit its expected type (`let small: UInt8 = 300`) | 300 doesn't fit in a `UInt8` (0 to 255) |
| E-NUM-7 | integer literal that doesn't fit `Integer` with no expected type (`10_000_000_000_000_000_000`) | doesn't fit in an `Integer`: use `10_000_000_000_000_000_000n` |
| E-NUM-8 | integer literal into a float type that isn't exactly representable (`let f: Float32 = 16_777_217`); float literal that rounds to infinity in its type (`let f: Float32 = 1e39`, `1e400`) [D-20] | — |
| E-NUM-9 | a literal where a `Percentage` is expected without `%` (`let p: Percentage = 50`) | write `50%` |

**Static semantics.** Literal adaptation per [2.2](#22-expected-types-and-literal-adaptation). `-128` is one constant: `let lowest: Int8 = -128` is valid. The minus glued to a literal binds before postfix access [D-53]: `-2.5.round()` is `(-2.5).round()`.

### 6.2 Number types

| Type | Kind | Range / format | Native | JavaScript |
| --- | --- | --- | --- | --- |
| `Int8`, `Int16`, `Int32` | integer | two's complement width | `i8`, `i16`, `i32` | number |
| `Integer` | integer | −2^63 … 2^63−1 (JS: checked at ±(2^53−1)) | `i64` | number |
| `Int64` (written explicitly) | integer | −2^63 … 2^63−1 | `i64` | `BigInt` |
| `Int128` | integer | −2^127 … 2^127−1 | `i128` | `BigInt` |
| `UInt8`/`Byte`, `UInt16`, `UInt32` | integer | 0 … 2^w−1 | `i8`…`i32` | number |
| `UInt64`, `UInt128` | integer | 0 … 2^w−1 | `i64`, `i128` | `BigInt` |
| `Integer(min = a, max = b)` | integer | `a..=b` | `i64` + `range` metadata | number |
| `Int8(min = a, max = b)`, … | integer | `a..=b` within the type | width of the type | as the base type |
| `Float32` | `Float32` | IEEE binary32 | `float` | number rounded with `Math.fround` |
| `Float`/`Float64` | `Float64` | IEEE binary64 | `double` | number |
| `BigInt` | `BigInt` | arbitrary | inline tagged 63-bit or heap limbs [D-54] | `BigInt` |
| `Percentage` | `Percentage` | fraction as binary64 (`50%` = `0.5`) | `double` | number |

`Integer` and `Int64` are distinct type names with the same range [D-8]: they are mutually assignable as scalars (the range fits), the conversion is `BigInt(x)`/`Number(x)` on JavaScript, and `Int64 + Integer` has the base type `Int64` (the explicit width wins). Containers are not converted implicitly: `Array(Int8)`, `Array(Integer)` and `Array(Int64)` are three different types; an `Array(Integer(min = 0))` value is accepted by an immutable parameter `Array(Integer)` (same storage), not by a `mutates` one.

Ranged types: `min`/`max` are compile-time value parameters; `min > max` is `E-NUM-13`; bounds outside the base type's range are `E-NUM-13`. `Integer(min = 0)` is the recommended non-negative type.

### 6.3 Literal and constant typing

- Without expected type: integer literal → `Integer`, float literal → `Float`. With expected type: [2.2](#22-expected-types-and-literal-adaptation).
- In an arithmetic operation, a literal operand takes the **kind** of the other operand (`ratio * 2` is `Float * Float`); for integers, it contributes its exact value without having to fit the other operand's type [D-94]: `a + 1000` with `a: Int8` has the base type `Int8` and the interval `[872, 1127]`, which is then checked at its sink (unannotated: certain overflow, `E-NUM-10`; `let s: Integer = a + 1000`: valid, no check).
- `Percentage` never absorbs a literal without `%`: `50% * 2` is `E-NUM-11` ("`Percentage * Integer`: write `2.0`, or `100%`") [D-46].
- Constant expressions (section 2.2) are folded exactly; their values are facts.

### 6.4 Integer typing: ranges and conversions

**Assignability** (C4): an integer value is accepted where an integer type is expected when its **interval** (from its type and its facts) is contained in the expected type's range. Otherwise:

- if the value is an **arithmetic expression** (a tree of `+ - * // %% **` and unary `-` over integers, or a compound assignment), the result is **checked at runtime** at the sink (a stop on failure), and a **certain** overflow (interval disjoint from the range) is `E-NUM-10`;
- any **other** expression (a name, a field, a call, an `if`, a literal) is a compile error, `E-NUM-14` ("`bar` may exceed the maximum value of 100: narrow it, or convert it with `Integer(max = 100)(bar)`"…; message suggests `T(value)` or an `if`), [D-89].

Examples: `let wide: Int16 = a + b` (Int8 operands): no check. `let d: Int8 = a + b`: runtime check. `let e: Int8 = 100 + 100`: E-NUM-10. `let byte: UInt8 = a` with `a: Int8`: E-NUM-14. `if a >= 0 { let p: UInt8 = a }`: valid (narrowed to `[0, 127]`).

**Base type of an unannotated arithmetic result**: the base type of its operands; with two different base types, the one whose range contains the other (`Int8 + Int32` → `Int32`; `Int64` wins over `Integer`); when neither contains the other (`Int8 + UInt8`) **and the result reaches a sink without expected type** (an unannotated binding, a method receiver, an interpolation), `E-NUM-15` ("annotate the result"). A comparison or a `/` consuming the result needs no base type [D-93].

**Conversions** (explicit calls, `values/numbers#conversions`):

| Call | Result | Fails |
| --- | --- | --- |
| `Float(i)`, `Float32(i)` | nearest float | never |
| `Float(f32)`, `Float32(f64)` | exact / nearest | never |
| `Float(p)` with `p: Percentage` | the fraction (`Float(50%)` = `0.5`) | never |
| `BigInt(i)` | exact | never |
| `Int8(i)`, `UInt16(i)`, `Integer(max = 100)(i)`, `Integer(b)` with `b: BigInt` | exact | `throws ArithmeticError` unless the value is proven to fit (facts, narrowing, literal); a certain failure (`Int8(300)`) is `E-NUM-10` |
| `Percentage(x)` | `x / 100` (formatter rewrites to `(x)%`) | never |
| `Integer.round(f)`, `.floor`, `.ceil`, `.truncate` | rounded integer (round: halves away from zero) | `throws ArithmeticError` when NaN, infinite or out of range, unless `f` is a literal (then checked at compile time) |
| `Integer.parse(s)` | — | `throws ParseError` (out of scope) |

`as` never converts. `BigInt(f)` from a float does not exist (`E-NUM-16`: round first with `Integer.round`) [D-45 note]. A conversion to the same type, or to a wider range, is `W-NUM-1` (useless conversion).

**Diagnostics** (cont.).

| Id | Condition |
| --- | --- |
| E-NUM-10 | certain overflow: an arithmetic result or a constant whose interval is disjoint from its sink type, or from the compute type at an intermediate node; certain failure of a conversion |
| E-NUM-11 | mixing kinds (`Integer * Float`, `Float32 + Float64`, `Integer + BigInt`, `Integer * Percentage`, `Percentage + Float`): "cannot multiply `Integer` by `Float`. Use `Float(count) * ratio`" |
| E-NUM-12 | integer or `BigInt` `//`/`%%` by a divisor proven to be zero (`5 // 0`) [D-91] |
| E-NUM-13 | invalid ranged type (`Integer(min = 5, max = 1)`, `Int8(max = 300)`) |
| E-NUM-14 | a non-arithmetic integer value not proven to fit its sink |
| E-NUM-15 | unannotated mix of integer base types where neither contains the other |
| E-NUM-16 | conversion that doesn't exist (`BigInt(1.5)`, `Percentage("a")`, `Integer(50%)`) |
| W-NUM-1 | useless conversion |

### 6.5 Range analysis (normative, F1)

Every compiler on every target must accept exactly the programs these rules accept (the exception for JavaScript is a backend error, see [D-9]). Facts per integer place: an interval and a list of excluded intervals [D-95] (at most 8; beyond, the oldest are dropped).

1. **Sources.** Type of the place (`Int8` → `[-128, 127]`), initializer or assigned value (a literal is exact), and interval arithmetic:

| Operation | Interval (`a = [a1, a2]`, `b = [b1, b2]`) |
| --- | --- |
| `a + b` | `[a1 + b1, a2 + b2]` |
| `a - b` | `[a1 - b2, a2 - b1]` |
| `a * b` | min/max of `a1b1, a1b2, a2b1, a2b2` |
| `-a` | `[-a2, -a1]` |
| `a // b` (`b` split into its negative and positive parts, 0 excluded) | for each part, min/max of `⌊ai / bj⌋` over the four corners; hull of the parts |
| `a %% b` | for the positive part of `b`: `[0, b2 - 1]`, narrowed to `a` itself when `0 ≤ a1` and `a2 < b1`; for the negative part: `[b1 + 1, 0]`, symmetrically; hull |
| `a ** b` (`b1 ≥ 0`) | both exact: the exact value. `a1 ≥ 0`: `[a1^b1, a2^b2]` (with `0^0 = 1`). Otherwise `[-M, M]` with `M = max(|a1|, |a2|)^b2` [D-90] |
| `x.abs()` | `[0, max(|a1|, |a2|)]`, or `[min(|a1|, |a2|), …]` when 0 ∉ a |
| `x.clamp(m, M)` | `[min(max(a1, m1), M1), min(max(a2, m2), M2)]` |
| `Math.max(a, b)` / `Math.min(a, b)` | `[max(a1, b1), max(a2, b2)]` / `[min(a1, b1), min(a2, b2)]` |
| `wrapping*`, `saturating*`, bitwise methods, any other call | the full range of the declared return type (`Integer(min = 0)` for `length`) |

2. **Conditions refine** in the branches of `if`, `while`, `when`, `and`, `or`, `not`, and after early exits: a comparison `x < e`, `x <= e`, `x > e`, `x >= e` where `e` is a constant or a place with a known interval refines `x` (and `e` symmetrically) in both branches; `x is a..b` intersects in the true branch and excludes in the false branch; `x == literal` makes `x` exact (false branch: excludes the value); `x != literal` the reverse. An excluded value at an interval end shrinks the interval (`[0, 10]` minus `{0}` is `[1, 10]`).
3. **Lengths.** The length of a collection (or the grapheme count of a string [D-97]) is known from its literal until it is mutated. Relational facts: `index < c.length` inside `for index in 0..c.length` and `for item, index in c`; after a guard `if index < c.length` (or in the `else` of its negation), where the guarded expression is a place or a place plus or minus a constant; equal lengths for a copy and for `map` of a collection. An index `i` into a collection of length `L` is proven when `0 ≤ i < L` or `-L ≤ i < 0` is implied by the facts.
4. **Loops** are analysed to a fixpoint. Widening [D-96]: from the second iteration on, a lower bound that decreased becomes `−∞` and an upper bound that increased becomes `+∞` (then clipped by the type). After `mutable count = 0` and a loop body `count += 1`, `count` is `[0, max]`.
5. **Mutations drop facts.** Assignment replaces; any other mutation of the place drops its facts; a mutation that may change a collection's length drops its length facts.

Nothing else is used: no reasoning on called function bodies (only signatures, including ranged return types), no other relation between places.

**Uses of the facts**: (a) assignability into ranged types, (b) overflow checks, (c) index proofs (`items[i]` needs `try` unless proven; certainly out of bounds `array[3]` on a 3-item array is `E-NUM-17` [D-91]), (d) divisor proofs, (e) shift amounts, (f) `when` exhaustiveness on integers, (g) always-true/false warnings `W-NUM-2` (`if value < 20` with `value: Integer(min = 20)`: "the condition is always false").

### 6.6 Arithmetic operators and result types

| Operands (same kind) | `+` `-` `*` | `/` | `//` | `%%` | `**` |
| --- | --- | --- | --- | --- | --- |
| integers | integer (base type, interval) | `Float` (Float64, for every width [D-48]) | integer | integer | integer |
| `Float32` | `Float32` | `Float32` | `Float32` | `Float32` | `Float32` |
| `Float64` | `Float64` | `Float64` | `Float64` | `Float64` | `Float64` |
| `BigInt` | `BigInt` | `Float` | `BigInt` | `BigInt` | `BigInt` |
| `Percentage` | `+`,`-`,`*` → `Percentage` | `Float` | E-NUM-11 | E-NUM-11 | E-NUM-11 |
| `Float32`/`Float64` × `Percentage` (either order, `*` only) | the float type | — | — | — | — |

Prefix `-` keeps the type of its operand (integers: interval rule; `Percentage`: `Percentage`). Unary `+` doesn't exist. Every other mix is E-NUM-11. Operands of other types (`String`, `Boolean`, objects): `E-NUM-18` ("`+` on `Boolean`"), except `String + String` (section 7).

### 6.7 Overflow and the evaluation of integer expressions [D-4]

An **arithmetic tree** is a maximal subtree of integer `+ - * // %% **` and unary `-` (parentheses included). Its value is the exact mathematical value, as long as every node fits the **compute type** `C`:

- `C` = `Int64` when every operand's storage type is within `Int64`; `Int128` when an operand is `UInt64` or `Int128`; with `UInt128`, every node is checked against `UInt128`;
- on JavaScript, for number-represented integers, `C` is `[-(2^53−1), 2^53−1]`; for `BigInt`-represented types it is the native one.

Checks:

1. A node whose interval fits `C` needs no check. A node whose interval may exceed `C` is checked against `C` (stop). A node whose interval is disjoint from `C`: E-NUM-10.
2. The **root** is checked at its sink against the sink type (section 6.4): no check when its interval fits, a runtime check (stop) when it may not, E-NUM-10 when it can't.
3. A comparison, a `/`, or a conversion consuming the root has no sink: only rule 1 applies (`a + b > 100` never stops for `Int8` operands).

Consequences: `let avg: Int8 = (a + b) // 2` never stops for `Int8` operands; `let x: Integer = a * b` with unknown `Integer` operands is checked once (native: `llvm.smul.with.overflow` at `i64`); on JavaScript every `Integer` node whose interval exceeds ±(2^53−1) is checked (as the documentation's JS section says).

Explicit variants (methods, not part of arithmetic trees): `wrappingAdd/Sub/Mul/Pow` (wrap modulo 2^width of the **storage** type; a ranged type wraps in its storage type, then the result has the storage type's full range), `saturatingAdd/Sub/Mul/Pow` (clamp to the receiver type's bounds), `checkedAdd/Sub/Mul/Pow` (`throws ArithmeticError`). Operand of the same type as the receiver (a literal adapts). On JavaScript, the variants on `Integer` compute at 64 bits (through `BigInt.asIntN(64, …)`) then go back to a number with the safe-integer check, which may stop [D-51]: hash code on JS should use `UInt32`/`Int32` or explicit `Int64`.

**JavaScript-only compile error** [D-9]: a node or a root whose interval is certainly outside ±(2^53−1) for a number-represented `Integer` (`let big: Integer = 2 ** 60`) is `E-NUM-19`, emitted by the JavaScript backend (not by the checker), only when building for a JavaScript target. The message suggests `Int64` or `BigInt`.

### 6.8 Division, floor division, remainder

- `/`: true division; integers → `Float` (both operands converted to `Float64`, then divided; for 64-bit and wider operands the conversion rounds); never throws; IEEE: `1 / 0` is `Infinity`, `0.0 / 0.0` is `NaN`.
- `//`: floor of the quotient (towards −∞). `%%`: `a - (a // b) * b`, the sign of the divisor. Identity `a == (a // b) * b + a %% b`. Floats: `7.5 // 2.0` = `3.0`; `%%` the matching remainder (`a - floor(a / b) * b`, IEEE; `x %% 0.0` is `NaN`, `x // 0.0` is ±`Infinity`/`NaN`).
- **Division by zero** (integers and `BigInt`): the divisor must be proven non-zero (a non-zero literal, a type range excluding 0, a narrowing `if divisor != 0`, an excluded value). Otherwise the operation `throws DivisionByZero`: the expression must be covered by `try` or `catch` (`E-ERR-3` otherwise). Proven zero: E-NUM-12.
- Overflow: `Integer.minValue // -1` overflows (interval rule catches it; checked like any node). `%%` by `-1` is always `0` but the native lowering must not execute `srem INT_MIN, -1` (undefined in LLVM) [D-49].

### 6.9 Exponentiation

- Right-associative; prefix `-` binds tighter (`-x ** 2` = `(-x) ** 2`).
- Integers and `BigInt`: the exponent must be proven non-negative (literal, `Integer(min = 0)`, narrowing), otherwise `throws ArithmeticError`; proven negative: E-NUM-10. `0 ** 0` = `1`. Result interval per 6.5; overflow per 6.7.
- Floats: IEEE `pow` (`Math.pow` on JavaScript). `Float ** Integer` is E-NUM-11 (a literal exponent adapts: `x ** 2` is valid).

### 6.10 Floats

IEEE 754 on every target: no fast-math. `/` never throws; overflow gives ±∞; `NaN == NaN` is `false`, every ordering with NaN is `false`, `-0.0 == 0.0` is `true`. As set elements and map keys: SameValueZero. Tests: `value.isNaN()`, `value.isFinite()`. `Float32` and `Float64` don't mix implicitly, in arithmetic, comparison or equality [D-19] (convert with `Float(x)` / `Float32(x)`). `Float32` results are rounded to binary32 after every operation (JS: `Math.fround` on each result).

### 6.11 Big integers

`12n`, `BigInt(12)`, or an integer literal where a `BigInt` is expected. Never overflow. `//`, `%%` throw `DivisionByZero` unless the divisor is proven non-zero (only literals and exact facts prove it: there are no ranged `BigInt` types); `**` needs a non-negative exponent (`throws ArithmeticError` otherwise). `/` gives a `Float`. Ordered among themselves; never mixed with other integers (E-NUM-11) — convert with `BigInt(i)` or `Integer(b)` (throws unless proven).

### 6.12 Percentages

- Literal `50%` (= fraction `0.5`), `12.5%`; `(expr)%` divides an integer-kind or float-kind expression by 100 [D-45] (on `BigInt`, `Percentage`, or anything else: E-NUM-16). `Percentage(x)` is accepted (formatter rewrites it).
- Precedence: postfix `%` binds tighter than prefix `-` (`-50%` = `-(50%)`).
- Operations: `P + P`, `P - P`, `P * P` → `P`; `P / P` → `Float`; `F * P`, `P * F` → `F` (`F` = `Float32` or `Float64`); `-P` → `P`. Everything else E-NUM-11. Ordering `< <= > >=` between percentages; equality only between percentages [D-47]. No implicit conversion to `Float` (`let r: Float = 50%` is E-TYPE-1: use `Float(50%)`).
- Printing: [D-6].

### 6.13 Bitwise methods

`bitAnd`, `bitOr`, `bitXor` (operand of the receiver's type; a literal adapts), `bitNot()`, `shiftLeft(n)`, `shiftRight(n)` on integer types. Result: the receiver's **storage** type with its full range (ranges are not propagated through bitwise operations) [D-50]. Shifts: logical for unsigned, arithmetic for signed; `shiftLeft` drops bits with no overflow check. Shift amount: proven in `[0, width)` → no error; proven outside → `E-NUM-20`; otherwise `throws ArithmeticError` [D-50].

### 6.14 Math

| Function | On | Result |
| --- | --- | --- |
| `round()`, `floor()`, `ceil()`, `truncate()` | floats | same float type; round: halves away from zero (`2.5.round()` = `3.0`, `(-2.5).round()` = `-3.0`) |
| `sqrt()` | floats | same type; NaN for negatives |
| `abs()` | integers, floats, `BigInt`, `Percentage` | same type; integers follow 6.5/6.7 (`Int8` `-128` → stop at the sink) |
| `Math.max(a, b)`, `Math.min(a, b)` | two numbers of the same kind | that kind; integers: interval rule, base type as for `+` |
| `clamp(min, max)` | integers, floats | receiver type; integers: interval rule |
| `isNaN()`, `isFinite()` | floats | `Boolean` |

Constants are not documented; proposal [D-52]: `Int8.minValue`, `Int8.maxValue` (every integer type), `Float.infinity`, `Float.nan`, `Float.epsilon`, `Integer.jsMaxSafeInteger` (documented, 2^53−1).

### 6.15 Compound assignment

`x op= e` for `op` in `+ - * ** / // %%` is `x = x op e` with `x`'s path evaluated **once**. The target must be a mutable place (E-DECL-3/4). The result type of `x op e` must be accepted by `x`'s declared type: `count /= 4` on an `Integer` is `E-NUM-21` ("the result is a `Float`, not an `Integer`"). Range: the compound assignment is an arithmetic tree whose sink is `x`'s declared type (runtime check when needed, E-NUM-10 when certain). Division checks apply: `x //= d` with an unproven divisor can fail, and is written `try x //= d` (the statement is covered, as for an index target, [D-87]); a `catch` can't follow an assignment (it would bind to `d` only), so handling it means `x = x // d catch error => x`. When the target can fail (unproven index), `try counts[key] += 1`. `.=`: section 11.

### Runtime semantics (summary)

- Integer arithmetic: exact within the compute type; checks at nodes and sinks per 6.7, a failure is a stop (`integer overflow: <value> doesn't fit in <Type>`; the value is printed when it is computable in 128 bits, otherwise "overflows `Int64`").
- `//`/`%%`: `DivisionByZero` (an error, not a stop).
- Conversions: `ArithmeticError` (an error).
- Floats: never stop, never throw.

### Lowering

**Native** (from the implementation sections):

- Each integer type in its width; a ranged `Integer(min, max)` in `i64` with `range` metadata; ranged sized types in their width.
- Arithmetic computed in the compute type; a possible overflow is `llvm.s{add,sub,mul}.with.overflow` (at the node's width) or a compare against the sink bounds, then a cold branch to `fa_stop`; proven-safe nodes get `nsw`/`nuw`.
- `//`, `%%` on signed: `sdiv`/`srem` then adjust by one when the remainder is non-zero and its sign differs from the divisor's; guard `INT_MIN %% -1` (return 0) and `INT_MIN // -1` (stop) before `sdiv`/`srem` [D-49]; unsigned: `udiv`/`urem`. An unproven divisor: `icmp eq 0`, cold branch to the `DivisionByZero` error path.
- `**` on integers: square-and-multiply with overflow intrinsics; exponent check first.
- Shifts: compare the amount with the width first; `shl`, `lshr`, `ashr`.
- `BigInt`: runtime library (inline tagged small values, heap limbs).
- Floats: `float`/`double`, no fast-math. `round` per the half-away-from-zero rule (`llvm.round`).
- `Percentage`: `double`; `(x)%` is a `fdiv` by 100 (after `sitofp`).

**JavaScript:**

- Number-represented integers: `+ - *` then, if the node may exceed `C`, `if (r > 9007199254740991 || r < -9007199254740991) $stop(...)`; sized types check their own bounds at sinks (`if (r < -128 || r > 127) $stop(...)`).
- `//` and `%%` on numbers [D-74]: the documentation's formulas (`Math.floor(a / b)`, `((a % b) + b) % b`) are not exact near 2^53. Use: `let r = a % b; let q = (a - r) / b; if (r !== 0 && (r < 0) !== (b < 0)) { q -= 1; r += b; }` (every step exact for safe integers: `a - r` is a multiple of `b` with `|a - r| ≤ |a|`). `q` is `a // b`, `r` is `a %% b`.
- Integers must never be `-0` in JavaScript [D-76]: `0 * -5`, `-0` from `-x`, `-4 % 2` produce `-0`; it is only observable after a conversion to a float, so `Float(i)` compiles to `(i + 0)` (or the producing operations normalize). `String(-0)` is `"0"`, so printing is unaffected.
- `/` on integers: plain `/` (numbers) or `Number(a) / Number(b)` (`BigInt` types).
- `**` on number integers: `**` then the check; on floats `Math.pow` (identical to `**` for numbers).
- `BigInt`-represented types (`Int64`, `UInt64`, `Int128`, `UInt128`): `BigInt` operators; sized ones check `BigInt.asIntN(64, r) !== r` (resp. `asUintN`) and stop; `wrapping*` use `asIntN`/`asUintN`. `//` on `BigInt`: `a / b` (truncates) adjusted by one when the signs differ and the remainder is non-zero. Mixing with number-represented operands converts (`BigInt(x)`) at the operation; **equality** between a `BigInt`-represented and a number-represented integer compiles to `BigInt(n) === b` (`===` between `1n` and `1` is `false`) [D-69]; relational operators work across both.
- `Float32`: `Math.fround` after every operation and at literal adaptation.
- Bitwise: on types up to 32 bits, `& | ^ ~ << >> >>>` then wrap to the type (`& 0xFF`, `<< 24 >> 24`, `>>> 0`); on `Integer`: through `BigInt` and `asIntN(64, …)`, back to a number with the safe-integer check.
- `round`: `Math.sign(x) * Math.round(Math.abs(x))`; `Math.max/min/sqrt/floor/ceil`, `Math.trunc` for `truncate`.
- `Percentage`: a number holding the fraction; `(x)%` is `x / 100`.

### Tests

Valid (output identical on all targets):

- T-NUM-1 `console.log(42, 0b101010, 0o52, 0x2A, 8_100_000_000)` → `42 42 42 42 8100000000`
- T-NUM-2 `console.log(0.5, 1.5e9, 2.5e-3, 1e6)` → `0.5 1500000000 0.0025 1000000`
- T-NUM-3 `let lowest: Int8 = -128 · console.log(lowest)` → `-128`
- T-NUM-4 `let explicitRatio: Float = 3 · console.log(explicitRatio / 2)` → `1.5`
- T-NUM-5 `let ratio = 0.5 · console.log(ratio * 2)` → `1`
- T-NUM-6 `console.log(22 / 7)` → `3.142857142857143`; `console.log(4 / 2)` → `2`
- T-NUM-7 `console.log(22 // 7, -7 // 2, 7 %% 3, -7 %% 3, 7 %% -3)` → `3 -4 1 2 -2`
- T-NUM-8 `console.log(7.5 // 2.0, -7.5 %% 2.0)` → `3 0.5`
- T-NUM-9 `console.log(2 ** 3 ** 2, -2 ** 2)` → `512 4`
- T-NUM-10 `let x = 3 · console.log(-x ** 2, -(x ** 2))` → `9 -9`
- T-NUM-11 `let a: Int8 = 100 · let b: Int8 = 100 · let wide: Int16 = a + b · console.log(wide)` → `200` (no check emitted: assert in the IR/JS golden)
- T-NUM-12 `let a: Int8 = 100 · let b: Int8 = 100 · let avg: Int8 = (a + b) // 2 · console.log(avg)` → `100` [D-4]
- T-NUM-13 `function f = (a: Int8, b: Int8): Int8 => a + b · console.log(f(100, 27))` → `127`; `console.log(f(100, 28))` ⇒ stop (`integer overflow: 128 doesn't fit in Int8`)
- T-NUM-14 `function g = (a: Int8, b: Int8) => a + b · let s = g(-100, -29)` ⇒ stop (unannotated base type `Int8`)
- T-NUM-15 `let x: UInt8 = 250 · let y: UInt8 = 10 · console.log(x.wrappingAdd(y), x.saturatingAdd(y))` → `4 255`
- T-NUM-16 `let x: UInt8 = 250 · let y: UInt8 = 10 · console.log(x.checkedAdd(y) catch error => 0)` → `0`
- T-NUM-17 `let small: Integer(min = 0, max = 100) = 87 · let bounded: Integer(min = 0, max = 200) = small * 2 · console.log(bounded)` → `174`
- T-NUM-18 `let a: Int8 = -5 · if a >= 0 { let p: UInt8 = a · console.log(p) } else { console.log("neg") }` → `neg`
- T-NUM-19 `function read = (): Integer => 0 · let d = read() · console.log(10 // d catch error => -1)` → `-1`
- T-NUM-20 `function read = (): Integer => 4 · let d = read() · if d != 0 { console.log(10 // d) }` → `2` (no `try`: narrowed)
- T-NUM-21 `let divisor: Integer(min = 1) = 5 · console.log(10 // divisor)` → `2`
- T-NUM-22 `console.log(1 / 0, -1 / 0, (0.0 / 0.0).isNaN())` → `Infinity -Infinity true`
- T-NUM-23 `let n = 0.0 / 0.0 · console.log(n == n, -0.0 == 0.0)` → `false true`
- T-NUM-24 `console.log(0.1 + 0.2)` → `0.30000000000000004`
- T-NUM-25 `console.log(1e21, 1e-7, 123456789012345680000.0)` → `1e+21 1e-7 123456789012345680000`
- T-NUM-26 `let s: Float32 = 0.1 · console.log(s)` → `0.1` [D-5]; `let t: Float32 = 0.1 · console.log(Float(t))` → `0.10000000149011612`
- T-NUM-27 `let big = 12n · let t: BigInt = 12 · console.log(big + t, BigInt(3) ** 40n)` → `24 12157665459056928801`
- T-NUM-28 `console.log(-7n // 2n, -7n %% 2n)` → `-4 1`
- T-NUM-29 `console.log(50%, 12.5%, 20% + 30%, 50% * 50%, 50% / 25%)` → `50% 12.5% 50% 25% 2`
- T-NUM-30 `console.log(40.0 * 50%, Float(50%))` → `20 0.5`
- T-NUM-31 `let width = 30 · let margin = 10 · console.log((width + margin)%)` → `40%`
- T-NUM-32 `console.log(10%, (1 + 2)%)` → `10% 3%` [D-6]
- T-NUM-33 `console.log(Integer.round(2.7), Integer.floor(2.7), Integer.round(-2.5))` → `3 2 -3` (literals: no `try`)
- T-NUM-34 `console.log(2.5.round(), (-2.5).round(), -2.5.round())` → `3 -3 -3`
- T-NUM-35 `function f = (): Float => 2.7 · console.log(Integer.round(f()) catch error => 0)` → `3`
- T-NUM-36 `function n = (): Integer => 300 · let i = n() · console.log(Int8(i) catch error => 0)` → `0` (with `let i = 300` the fact makes `Int8(i)` E-NUM-10); `let j: Integer = 3 · console.log(Int8(j))` → `3` (proven by the fact: no `try`)
- T-NUM-37 `let a: UInt8 = 0b1100 · let b: UInt8 = 0b1010 · console.log(a.bitAnd(b), a.bitOr(b), a.bitXor(b), a.bitNot(), a.shiftLeft(2), a.shiftRight(2))` → `8 14 6 243 48 3`
- T-NUM-38 `let a: Int8 = -8 · console.log(a.shiftRight(1))` → `-4` (arithmetic)
- T-NUM-39 `let m: UInt8 = 0b1000_0000 · console.log(m.shiftLeft(1))` → `0` (bits dropped, no check)
- T-NUM-40 `console.log(Math.max(3, 7), Math.min(3, 7), (-5).abs(), 15.clamp(0, 10), 2.0.sqrt())` → `7 3 5 10 1.4142135623730951`
- T-NUM-41 `mutable count = 10 · count += 1 · count -= 2 · count *= 3 · count //= 4 · count %%= 5 · count **= 2 · console.log(count)` → `1`
- T-NUM-42 `let a: Int8 = 10 · let b: Int32 = 1000 · let s = a + b · console.log(s)` → `1010` (`Int32`)
- T-NUM-43 `function f = (): Integer(min = 20) => 25 · mutable value: Integer(min = 20) = f() · value -= 6` ⇒ stop (runtime check on a compound assignment; with `= 25` directly, the fact makes it E-NUM-10)
- T-NUM-44 `let x: Int64 = 9_007_199_254_740_993 · console.log(x + 1)` → `9007199254740994` (exact on JS: `BigInt`)
- T-NUM-45 `let x: Integer = 9_007_199_254_740_991 · let y = x + 0 · console.log(y)` → `9007199254740991`
- T-NUM-46 (JS target) `function inc = (x: Integer) => x + 1 · console.log(inc(9_007_199_254_740_991))` ⇒ stop on JavaScript, `9007199254740992` on native (documented divergence)
- T-NUM-47 `let a: Int64 = 5 · let b: Integer = 5 · console.log(a == b)` → `true` (JS: `BigInt(b) === a`)
- T-NUM-48 `function zero = (): Integer => 0 · let w = -zero() · console.log(1.0 / Float(w))` → `Infinity` (no `-0` leak on JavaScript)
- T-NUM-49 (native only: the constant is beyond 2^53) `function m = (): Integer => -9_223_372_036_854_775_807 - 1 · let x = m() · console.log(x %% -1)` → `0` (no `srem` trap)
- T-NUM-50 `function m = (): Int32 => -2_147_483_648 · let x = m() · let y: Int32 = x // -1` ⇒ stop
- T-NUM-51 `function big = (): Integer => 3_000_000_000 · let a = big() · let b = big() · let c: Integer = a * b // 3_000_000_000 · console.log(c)` → native `3000000000`; JS ⇒ stop (intermediate beyond 2^53)
- T-NUM-52 `let value: Integer(min = 20) = 25 · if value < 20 { console.log("x") }` ⇒ W-NUM-2
- T-NUM-53 `mutable count = 0 · for 0..10 { count += 1 } · let c: Integer(min = 0) = count · console.log(c)` → `10` (widening keeps the lower bound; `count += 1` keeps a runtime check)
- T-NUM-54 `let i: Integer(min = 0, max = 2) = 1 · let array = [1, 2, 3] · console.log(array[i])` → `2` (no `try`)
- T-NUM-55 `console.log(0.5 ** 2.0, 2.0 ** 0.5)` → `0.25 1.4142135623730951`
- T-NUM-56 `console.log(-50%, 50% > 25%)` → `-50% true`

Invalid:

- T-NUM-60 `let a = .5` ⇒ E-NUM-1; `let b = 5.` ⇒ E-NUM-1
- T-NUM-61 `let a = 1_` ⇒ E-NUM-2; `let b = 1__0` ⇒ E-NUM-2
- T-NUM-62 `let a = 0XFF` ⇒ E-NUM-3; `let b = 1E6` ⇒ E-NUM-3; `let c = 0b2` ⇒ E-NUM-3
- T-NUM-63 `let a = 12.5n` ⇒ E-NUM-4
- T-NUM-64 `let fifty = 50 · let a = fifty%` ⇒ E-NUM-5
- T-NUM-65 `let small: UInt8 = 300` ⇒ E-NUM-6
- T-NUM-66 `let huge = 10_000_000_000_000_000_000` ⇒ E-NUM-7
- T-NUM-67 `let huge: BigInt = 10_000_000_000_000_000_000 · console.log(huge)` → `10000000000000000000000` (valid)
- T-NUM-68 `let f: Float32 = 16_777_217` ⇒ E-NUM-8; `let g = 1e400` ⇒ E-NUM-8
- T-NUM-69 `let p: Percentage = 50` ⇒ E-NUM-9
- T-NUM-70 `let e: Int8 = 100 + 100` ⇒ E-NUM-10; `let a: UInt8 = 200 + 100` ⇒ E-NUM-10
- T-NUM-71 `let x: UInt8 = 255 · let y = x + 1` ⇒ E-NUM-10 (fact: exactly 256 for a `UInt8` sink)
- T-NUM-72 `let count = 3 · let ratio = 0.5 · let r = count * ratio` ⇒ E-NUM-11
- T-NUM-73 `let a: Float32 = 1.0 · let b: Float = 2.0 · let c = a + b` ⇒ E-NUM-11; `let d = a == b` ⇒ E-CMP-1
- T-NUM-74 `let n = 3 · let p = n * 50%` ⇒ E-NUM-11; `let q = 50% * 2` ⇒ E-NUM-11; `let r = 50% + 0.5` ⇒ E-NUM-11
- T-NUM-75 `let x = 5 // 0` ⇒ E-NUM-12; `let y = 5 %% 0` ⇒ E-NUM-12
- T-NUM-76 `function read = (): Integer => 2 · let d = read() · let q = 10 // d` ⇒ E-ERR-3 (`DivisionByZero` not handled)
- T-NUM-77 `let r: Integer(min = 5, max = 1) = 3` ⇒ E-NUM-13
- T-NUM-78 `function read = (): Int8 => 1 · let a = read() · let byte: UInt8 = a` ⇒ E-NUM-14
- T-NUM-79 `mutable foo: Integer(max = 100) = 1 · function get = (): Integer(max = 200) => 150 · let bar = get() · foo = bar` ⇒ E-NUM-14
- T-NUM-80 `function i8 = (): Int8 => 1 · function u8 = (): UInt8 => 1 · let s = i8() + u8()` ⇒ E-NUM-15; `let t: Int16 = i8() + u8()` valid
- T-NUM-81 `let b = BigInt(1.5)` ⇒ E-NUM-16; `let p = (5n)%` ⇒ E-NUM-16
- T-NUM-82 `let array = [1, 2, 3] · let item = array[3]` ⇒ E-NUM-17; `let value: Integer(min = 4) = 4 · let x = array[value]` ⇒ E-NUM-17
- T-NUM-83 `let t = true + 1` ⇒ E-NUM-18
- T-NUM-84 `let a: UInt8 = 1 · let b = a.shiftLeft(8)` ⇒ E-NUM-20; `a.shiftLeft(-1)` ⇒ E-NUM-20
- T-NUM-85 `function n = (): Integer => 1 · let s = n() · let a: UInt8 = 1 · let b = a.shiftLeft(s)` ⇒ E-ERR-3 (`ArithmeticError` not handled)
- T-NUM-86 `mutable count = 10 · count /= 4` ⇒ E-NUM-21
- T-NUM-87 `function e = (): Integer => 2 · let x = 2 ** e()` ⇒ E-ERR-3 (exponent not proven non-negative)
- T-NUM-88 (JS target) `let big: Integer = 2 ** 60` ⇒ E-NUM-19 on JavaScript, valid on native
- T-NUM-89 `let r = Int8(300)` ⇒ E-NUM-10
- T-NUM-90 `let x: Int8 = 1 · let y = Int16(x)` ⇒ W-NUM-1

---

## 7. Strings and characters

Sources: `values/strings` (incl. `:::implementation`), `operations/string`, `getting-started/syntax-basics#strings`.

### Syntax (as parsed)

- `"…"` only (no single quotes, no backticks). A string may span lines; line breaks are kept, indentation of the following lines is kept. Node: `StringLiteral(String)` or `StringTemplate { parts: [Literal | Expression] }`.
- Escapes: `\\`, `\"`, `\n`, `\r`, `\t`, `\0`, `\xNN` (`00`–`7F`), `\u{X…}` (1–6 hex digits, a Unicode scalar value).
- Interpolation: `{ expr }`, any expression including nested strings and object literals (the string resumes after the `}` matching the `{`). `{{` and `}}` are literal braces.

### Static semantics

- A string literal is a `String`, or a `Character` when a `Character` is expected and the literal is exactly one grapheme cluster (UAX #29 tables of the compiler). An interpolated string is always a `String`.
- `Character` is accepted where `String` is expected (no conversion visible); the reverse is E-STR-8.
- `+`: both operands `String` or `Character`; the result is a `String`. Any other operand: E-STR-7. `+=` on a mutable `String` target with a `String` or `Character` value. There is no `*` (use `repeat`).
- Interpolation: each expression must have a `toString` (every type except function types and `Void`): E-STR-11 for `Void`, E-STR-12 for a function value [D-39]. An interpolation that can fail must be covered (`"{try items[i]}"` inside a function that declares `IndexError`, or the whole string followed by `catch`).
- `length`: property, `Integer(min = 0)`, grapheme count; known as a fact for literals [D-97].
- Indexing `s[i]` with an integer: a `Character`; negative indices count from the end; unproven index: `throws IndexError`; proven out of bounds: E-NUM-17. Non-integer index: E-STR-10. `s.get(i)`: `Character?`.
- Slicing `s[range]`: a new `String` (copy) with `let`/`mutable`; `use w = s[range]` declares a slice of type `String in s`, invalidated by any mutation of `s` (appending, assigning, assigning the field that holds it). Bounds from `−length` to `length`; an inverted range gives `""` [D-42]; unproven bounds: `throws IndexError`.
- Strings are immutable through methods: no method mutates. A `mutable` string changes only by assignment and `+=`.
- Ordering `< <= > >=`: by Unicode code point (on `String` and `Character`); `==`: same sequence of scalar values, no Unicode normalization [D-43]; a literal operand adapts to `Character` when the other side is a `Character`.
- `is in` on a string: E-CMP-6 ("use `includes`").
- Method table (`operations/string#methods`): `isEmpty()`, `get(index)`, `includes/startsWith/endsWith(search)`, `indexOf/lastIndexOf(search): Integer?`, `toUpperCase()/toLowerCase()` (Unicode default case mapping, locale-independent), `trim/trimStart/trimEnd()`, `padStart/padEnd(length: Integer, filler: String = " ")`, `repeat(count: Integer(min = 0))`, `replace(search, replacement)` (first match), `replaceAll(...)`, `split(separator): Array(String)` (empty separator: characters), `characters(): Array(Character)`, `codePoints(): Array(UInt32)`, `toString()`, `String.compareLocale(a, b, locale: String? = none): Integer`; `Array(String).join(separator)`. Every position and search is in grapheme clusters, and a search never matches part of a grapheme cluster (`"e\u{301}".includes("e")` is `false`). No regular expressions.

### Diagnostics

| Id | Condition |
| --- | --- |
| E-STR-1 | unknown escape (`\q`, `\a`, `\'`, `\{`) |
| E-STR-2 | `\x80`–`\xFF`, or `\x` without two hex digits |
| E-STR-3 | `\u{…}` that is empty, longer than 6 digits, a surrogate (`D800`–`DFFF`) or above `10FFFF`; `\u` without braces |
| E-STR-4 | a lone `}` in a string (write `}}`) [D-15] |
| E-STR-5 | empty interpolation `{}` or `{ }` |
| E-STR-6 | unterminated interpolation |
| E-STR-7 | `+` with a non-string operand: "expected a `String`, found an `Integer`: use `"Score: {score}"` or `score.toString()`" |
| E-STR-8 | `String` where a `Character` is expected (non-literal) |
| E-STR-9 | literal with zero or several grapheme clusters where a `Character` is expected (`let c: Character = "ab"`, `= ""`) |
| E-STR-10 | string indexed by a non-integer, non-range value |
| E-STR-11 | interpolating a `Void` expression |
| E-STR-12 | interpolating a function value [D-39] |
| E-STR-13 | single-quoted or backtick string (`'a'`): "strings use double quotes" |

### Runtime semantics

- Source line breaks are `\n`, including `\r\n` and lone `\r` in the source file [D-16].
- `length`, indexing, slicing, iteration, searches: grapheme clusters (UAX #29 of the compiler's Unicode version on native; of the engine's `Intl.Segmenter` on JavaScript — the one documented divergence).
- `IndexError` from unproven indices and slices.
- Iteration yields `Character` copies (with the index for `for c, i in s`).

### Lowering

**Native** (`values/strings:::implementation`, LLVM Q5): `String` = 24 bytes `{ ptr, byte length, capacity | flags (static, inline, ASCII) }`; ≤ 23 bytes inline; literals in read-only data with their grapheme count computed at compile time, copied only when a mutable string is created from one; heap strings cache their grapheme count and build breadcrumbs (byte offset of every 64th grapheme) at the second indexing after a write; a write clears both; ASCII flag maintained on every write (O(1) `length` and indexing). Slice = `{ ptr, byte length }` on grapheme boundaries. `<` = `memcmp` of UTF-8 (code point order). `Character` = small inline string. Interpolation: compute the pieces, sum the byte lengths, allocate once, write.

**JavaScript**: a JS string. `+` → `+`. Interpolation → a template literal; a `String`, `Character` or integer goes in directly, other values through `$fa.str(value)` (floats `Float32`, percentages, optionals, composites). Grapheme operations through a shared `Intl.Segmenter("und", { granularity: "grapheme" })` with an ASCII fast path (`/^[\x00-\x7F]*$/`). Caveat: JavaScript strings can't cache a grapheme count, so `length` on non-ASCII text is O(n) per call: the backend hoists `s.length` out of loops when `s` isn't mutated. `<` → helper comparing by code point only when either string contains a surrogate (`/[\uD800-\uDFFF]/`), else JS `<`. `==` → `===`. Slices → `substring` (a copy; invalidation is compile-time only). `indexOf` → JS `indexOf` converted to a grapheme index, `-1` → `undefined` (`none`); `includes`/`startsWith`/`endsWith`/`indexOf`/`replace`/`split` must reject matches whose ends are not grapheme boundaries (helper; ASCII fast path is the plain JS call). `toUpperCase`/`toLowerCase`/`trim*`/`padStart`/`padEnd` (pad length in graphemes: helper unless ASCII) /`repeat` → JS. `codePoints()` → `Array.from(s, (c) => c.codePointAt(0))`.

### Tests

Valid:

- T-STR-1 `let greeting = "Hello" · console.log(greeting)` → `Hello`
- T-STR-2 `let message = "First line · Second line" · console.log(message)` (a real line break in the literal) → `First line / Second line`
- T-STR-3 same file saved with CRLF line endings → identical output, and `message.length` is `22`
- T-STR-4 `console.log("Quote: \" Backslash: \\ Tab:\tEnd")` → `Quote: " Backslash: \ Tab:<TAB>End`
- T-STR-5 `console.log("\x41\u{1F600}")` → `A😀`
- T-STR-6 `let name = "Fa" · console.log("Hello {name}")` → `Hello Fa`
- T-STR-7 `let a = 2 · let b = 3 · console.log("{a} + {b} = {a + b}")` → `2 + 3 = 5`
- T-STR-8 `console.log("Braces: {{ and }}")` → `Braces: { and }`
- T-STR-9 `let user: { name: String? } = { name = none } · console.log("Name: {user.name ?? "anonymous"}")` → `Name: anonymous`
- T-STR-10 `type Point = { x: Integer, y: Integer } · console.log("Origin: {Point { x = 0, y = 0 }}")` → `Origin: { x = 0, y = 0 }`
- T-STR-11 `console.log("{0.5} {50%} {none} {true} {[1, 2]} {(1, "a")}")` → `0.5 50% none true [1, 2] (1, "a")`
- T-STR-12 `let snow: Character = "☃" · let flag: Character = "🇫🇷" · console.log(snow, flag)` → `☃ 🇫🇷`
- T-STR-13 `let first: Character = "F" · let text: String = first · console.log("Letter: " + first, text)` → `Letter: F F`
- T-STR-14 `console.log("héllo".length, "🇫🇷".length, "e\u{301}".length)` → `5 1 1`
- T-STR-15 `let name = "Fa" · console.log(name[0], name[-1], name[0] == "F")` → `F a true` (no `try`: literal length fact)
- T-STR-16 `let s = "hello" · console.log(s[1..3], s[1..=3], s[1..], s[..2], s[-3..])` → `el ell ello he llo`
- T-STR-17 `let s = "hello" · console.log(s[3..1] == "")` → `true` [D-42]
- T-STR-18 `function idx = (): Integer => 10 · let s = "abc" · console.log(s.get(idx()), s[idx()] catch error => "?")` → `none ?`
- T-STR-19 `mutable output = "" · for item in [1, 2, 3] { · output += item.toString() · output += "," · } · console.log(output)` → `1,2,3,`
- T-STR-20 `for c, i in "aé🇫🇷" { console.log("{i}:{c}") }` → `0:a / 1:é / 2:🇫🇷`
- T-STR-21 `console.log("abc" < "abd", "Z" < "a", "\u{FFFF}" < "\u{10000}")` → `true true true` (code point order; JS code-unit order would say `false` for the last)
- T-STR-22 `console.log("é" == "e\u{301}")` → `false` [D-43]
- T-STR-23 `let t = "hello world" · console.log(t.includes("world"), t.startsWith("hello"), t.endsWith("world"), t.indexOf("world"), t.indexOf("planet"))` → `true true true 6 none`
- T-STR-24 `console.log("hello-world".replace("-", " "), "a-b-c".replaceAll("-", " "), "a,b,c".split(","), ["a", "b"].join(", "))` → `hello world a b c ["a", "b", "c"] a, b`
- T-STR-25 `console.log("ß".toUpperCase(), "  padded  ".trim(), "ab".repeat(3), "7".padStart(3, "0"))` → `SS padded ababab 007`
- T-STR-26 `console.log("e\u{301}x".includes("e"), "🇫🇷🇩🇪".indexOf("🇩🇪"))` → `false 1`
- T-STR-27 `console.log("abc".codePoints(), "ab".characters(), "a😀".split(""))` → `[97, 98, 99] ["a", "b"] ["a", "😀"]`
- T-STR-28 `console.log(String.compareLocale("a", "b") < 0)` → `true`
- T-STR-29 `mutable text = "hello world" · use word = text[0..5] · console.log(word)` → `hello`
- T-STR-30 `console.log("a" + "b" + "c", "x".isEmpty(), "".isEmpty())` → `abc false true`

Invalid:

- T-STR-40 `let s = "\q"` ⇒ E-STR-1
- T-STR-41 `let s = "\x80"` ⇒ E-STR-2
- T-STR-42 `let s = "\u{D800}"` ⇒ E-STR-3; `"\u{110000}"` ⇒ E-STR-3
- T-STR-43 `let s = "a } b"` ⇒ E-STR-4
- T-STR-44 `let s = "a {} b"` ⇒ E-STR-5
- T-STR-45 `let score = 42 · let label = "Score: " + score` ⇒ E-STR-7
- T-STR-46 `let text = "F" · let c: Character = text` ⇒ E-STR-8
- T-STR-47 `let letters: Character = "ab"` ⇒ E-STR-9; `let e: Character = ""` ⇒ E-STR-9
- T-STR-48 `let s = "abc" · let c = s[true]` ⇒ E-STR-10
- T-STR-49 `function log = (m: String) => { · console.log(m) · } · console.log("{log("a")}")` ⇒ E-STR-11
- T-STR-50 `function f = (x: Integer) => x · console.log("{f}")` ⇒ E-STR-12
- T-STR-51 `let s = 'a'` ⇒ E-STR-13
- T-STR-52 `mutable text = "hello" · use word = text[0..2] · text += "!" · console.log(word)` ⇒ E-DECL-16
- T-STR-53 `function i = (): Integer => 1 · let s = "abc" · let c = s[i()]` ⇒ E-ERR-3 (`IndexError`)
- T-STR-54 `let s = "abc" · let c = s[5]` ⇒ E-NUM-17
- T-STR-55 `let s = "abc" · s.push("d")` ⇒ E-OBJ-5 (no such method)
- T-STR-56 `let ok = "a" is in "abc"` ⇒ E-CMP-6
- T-STR-57 `let x = "a" * 3` ⇒ E-NUM-18

---

## 8. Booleans

Source: `values/booleans`, `operations/logical`.

### Syntax and static semantics

- `true`, `false` (`Node::Boolean`); type `Boolean`. Literal types `true`/`false` exist only as facts and in exhaustiveness.
- `not e`, `a and b`, `a or b`: operands must be `Boolean` (`E-BOOL-1`, message: "expected a `Boolean`, found `User?`. Use `user is not none`"). The same for the condition of `if`, `while`, `assert`, and both sides of `and`/`or`.
- `and` binds tighter than `or`; `not` tighter than `and` (parser fix [D-1]); `not` looser than `==`/`is` (`not a == b` is `not (a == b)`).
- Equality `==`/`!=` between booleans; no ordering (`true < false`: E-CMP-2); no arithmetic (E-NUM-18).
- Constant conditions decided by facts or literals (`if true`, `while false`, `if x and not x`): `W-BOOL-1` (always true/false), with `while true { }` suggesting `loop` [D-38].

### Runtime semantics

Short-circuit: the right operand of `and` is evaluated only when the left is `true`, of `or` only when the left is `false`. Narrowing through `and`/`or`/`not`: section 14.

### Lowering

Native: `i1` in registers, `i8` in memory; `and`/`or` as branches (or `select` when the right side is pure and cheap). JavaScript: `true`/`false`, `&&`, `||`, `!` (operands are always booleans, so JS truthiness never applies).

### Tests

- T-BOOL-1 `let isVisible: Boolean = true · console.log(isVisible, not isVisible)` → `true false`
- T-BOOL-2 `function t = (): Boolean => { · console.log("t") · true · } · function f = (): Boolean => { · console.log("f") · false · } · let r = f() and t() · console.log(r)` → `f / false`
- T-BOOL-3 same with `t() or f()` → `t / true`
- T-BOOL-4 `let a = true · let b = false · let c = true · console.log(a or b and c, not a and b)` → `true false` (`not a and b` is `(not a) and b`)
- T-BOOL-5 `let a = true · let b = false · console.log(not a == b)` → `true` (`not (a == b)`)
- T-BOOL-6 `console.log(true == true, true != false)` → `true true`
- T-BOOL-7 `let count = 0 · if count { }` ⇒ E-BOOL-1
- T-BOOL-8 `let name = "" · if not name { }` ⇒ E-BOOL-1
- T-BOOL-9 `let u: Integer? = 1 · if u and true { }` ⇒ E-BOOL-1
- T-BOOL-10 `let x = true < false` ⇒ E-CMP-2
- T-BOOL-11 `if true { console.log("a") }` → `a` and W-BOOL-1
- T-BOOL-12 `while true { break }` ⇒ W-BOOL-1 (suggests `loop`)

---

## 9. `none` and optionals

Sources: `values/optionals`, `operations/logical#fallback-for-optionals`, `types/operations#optional`.

### Syntax (as parsed)

`none` (`Node::None`); type suffix `T?` (`Node::Optional`), `Optional(T)`, `T | None`; safe access `a?.b`, `a?.m(args)` (`OptionalAccess`); fallback `a ?? b` (`Fallback { operands }`, left-associative).

### Static semantics

- `T?` = `Optional(T)` = `T | None`; `T??` = `T?`. A single `none`.
- `none` is accepted only where the expected type contains `None`: `let dog: Dog = none` is `E-OPT-2`. Without an expected type: E-DECL-11 [D-21].
- A `T` is accepted where `T?` is expected (no conversion). A `T?` is not accepted where `T` is expected: `E-OPT-1` ("`user` may be `none`: narrow it with `if user is not none`, or use `?.` or `??`") — this covers field access, method calls, indexing, arithmetic, passing as an argument, using it in a condition (E-BOOL-1 there).
- Optional in a condition: E-BOOL-1 (no truthiness).
- **`?.`** (`user?.name`, `user?.greet()`): the left side must be optional (`E-OPT-3` when it isn't: "use `.`" [D-22]). It short-circuits the whole chain on its right (`.x`, `[i]`, `(args)`, further `?.`): when the left is `none`, the result is `none` and nothing on the right is evaluated. Result type: the type of the chain made optional once (`user?.age` with `age: Integer?` is `Integer?`). A `?.` call whose result is `Void` can only be used as a statement [D-99] (`E-OPT-4` if its value is used). Index or call through an optional base without `?.`: E-OPT-1; there is no `?[` and no `?(` (`E-OPT-5`, parser error today).
- **`??`**: the left side must be optional (`E-OPT-6`, "`??` only applies to optionals"; on a failing call: "use `catch`"). Result type: (left without `None`) ∪ right; optional only if the right is. Left-associative: in `a ?? b ?? c`, if `b` is not optional, the second `??` is E-OPT-6 [D-22]. The right side may be `Never` (`x ?? return 0`, `x ?? throw …`, `?? break`): the result is the left without `None` [D-100]; the left place itself is not narrowed by it.
- `??` precedence: looser than `or` (`a ?? b or c` is `a ?? (b or c)`), tighter than `then`, `try`, `catch`.
- `Void?` is `E-OPT-9`; `None?` is `None`.
- Tests on optionals: `is none`, `is not none`, `== none`, `!= none` narrow (section 14). `x is none` where `x` is not optional: `E-CMP-7` (never true) [D-28]; `x is not none` / `x != none` on a non-optional: `W-CMP-1` (always true).
- An optional compares with its base type: `maybeScore == 42` (false when `none`).
- `get` on collections of optionals returns a single optional (flattening).

### Runtime semantics

`?.` evaluates the left once; `??` evaluates the right only when the left is `none`.

### Lowering

- Native (LLVM "Unions and optionals"): `{ tag, payload }`, or a niche when `T` has one (non-null pointers of strings/collections, `Boolean`, enums with spare values, ranged integers excluding a value, references); the first version may use explicit tags everywhere. `?.` = test + branch; `??` = test + `select`/branch.
- JavaScript: `none` is `undefined` [D-7] (no Fa value is ever `null` or `undefined` otherwise; `Void` values never reach a variable). So `?.` → JS `?.`, `??` → JS `??`, `x is none` → `x === undefined`, optional fields absent from a literal are set explicitly to `undefined` (keeps object shapes stable). Printing goes through a helper (`none`, not `undefined`).

### Diagnostics

| Id | Condition |
| --- | --- |
| E-OPT-1 | optional used where a non-optional is required (member, index, call, operand, argument) |
| E-OPT-2 | `none` where the type has no `None` |
| E-OPT-3 | `?.` on a non-optional value |
| E-OPT-4 | value of a `?.` call returning `Void` used |
| E-OPT-5 | `?[` or `?(` |
| E-OPT-6 | `??` on a non-optional left side (or on a failing call: "use `catch`") |
| E-OPT-9 | `Void?` |

### Tests

- T-OPT-1 `mutable sum: Integer? = none · if sum is none { · sum = 0 · } · console.log(sum)` → `0`
- T-OPT-2 `let sum: Integer? = 0 · console.log(sum is 0, sum is not none, sum ?? 10)` → `true true 0`
- T-OPT-3 `type User = { name: String, address: { city: String }? } · let u: User? = { name = "Ana", address = none } · console.log(u?.name, u?.address?.city, u?.address?.city ?? "unknown")` → `Ana none unknown`
- T-OPT-4 `type U = { name: String } · let u: U? = none · function n = (): String => { · console.log("called") · "x" · } · console.log(u?.name ?? n())` → `called / x`
- T-OPT-5 `type U = { name: String } · let u: U? = { name = "Bo" } · function n = (): String => { · console.log("called") · "x" · } · console.log(u?.name ?? n())` → `Bo` (right side not evaluated)
- T-OPT-6 `type A = { b: { c: Integer } } · let a: A? = none · console.log(a?.b.c)` → `none` (whole chain short-circuited)
- T-OPT-7 `let values: Array(Integer?) = [1, none] · console.log(values.get(1), values.get(5))` → `none none`
- T-OPT-8 `let m: Integer? = none · console.log(m == 42, m == none)` → `false true`
- T-OPT-9 `function first = (xs: Array(Integer)): Integer => xs.get(0) ?? return -1 · console.log(first([]), first([7]))` → `-1 7`
- T-OPT-10 `type Dog = { name: String, function bark = (self) => console.log("{self.name}!") } · let d: Dog? = { name = "Rex" } · d?.bark()` → `Rex!`
- T-OPT-11 `let a: Integer? = none · let b: Integer? = 2 · console.log(a ?? b ?? 3)` → `2`
- T-OPT-12 `type MaybeInteger = Integer? · let x: Optional(Integer) = 3 · let y: Integer | None = x · let z: MaybeInteger = y · console.log(z)` → `3`
- T-OPT-20 `type Dog = { name: String } · let dog: Dog = none` ⇒ E-OPT-2
- T-OPT-21 `type Dog = { name: String } · let dog: Dog? = none · console.log(dog.name)` ⇒ E-OPT-1
- T-OPT-22 `let x: Integer? = 1 · let y = x + 1` ⇒ E-OPT-1
- T-OPT-23 `type Dog = { name: String } · let dog: Dog? = none · if dog { }` ⇒ E-BOOL-1
- T-OPT-24 `type U = { name: String } · let u = U { name = "a" } · let n = u?.name` ⇒ E-OPT-3
- T-OPT-25 `let s: Array(Integer)? = none · let x = s?[0]` ⇒ E-OPT-5
- T-OPT-26 `let n = 5 · let m = n ?? 0` ⇒ E-OPT-6
- T-OPT-27 `let a: Integer? = none · let r = a ?? 1 ?? 2` ⇒ E-OPT-6
- T-OPT-28 `let n = 5 · if n is none { }` ⇒ E-CMP-7; `if n is not none { }` ⇒ W-CMP-1
- T-OPT-29 `type T = Void?` ⇒ E-OPT-9
- T-OPT-30 `function load = (): Integer throws ParseError => 1 · let v = load() ?? 0` ⇒ E-OPT-6 ("use `catch`")

---

## 10. Tuples

Source: `values/tuples`.

### Syntax (as parsed)

Value `(a, b, …)` with at least two items (`Node::Tuple` inside a `Group`); type `Tuple(A, B, …)`; access `t[0]` (`Index` with a literal); pattern `(a, b)`. `()` is `EmptyGroup`; `(x)` is a parenthesized expression; `(1,)` is a parse error.

### Static semantics

- Type `Tuple(T1, …, Tn)`, n ≥ 2, element types widened when unannotated (`(1, "one")` is `Tuple(Integer, String)`). With an expected tuple type, elements are checked against it (literals adapt).
- `()` as a value: `E-TUP-1`; `Tuple(A)` or `Tuple()`: `E-TUP-1`.
- Indexing: the index must be a non-negative integer literal (or a constant expression) smaller than n [D-31]: a non-literal index is `E-TUP-2` ("a tuple index must be a literal"), out of range or negative is `E-TUP-3`. `t.0` is `E-TUP-4` ("write `t[0]`").
- Element assignment `t[0] = v` on a mutable tuple: allowed, `v` checked against the element type [D-31].
- Equality element-wise; the two tuple types must be compatible (same arity, compatible element types: E-CMP-1); no ordering (E-CMP-2).
- No conversion between tuples and objects (`let p: Point = tuple` is E-TYPE-1).
- Destructuring with an exact-arity tuple pattern (section 16). `when … as (a, b)` binding (section 17).
- Stored inline, copied on assignment.

### Lowering

Native: a struct with positional fields (same layout rules as objects). JavaScript [D-64]: a JS array `[a, b]` (no freezing); `t[0]` → `t[0]`; copies follow the object rule (copy only when a later write could be observed; shallow `slice()` for flat elements, generated copy otherwise); equality: generated element-wise function.

### Tests

- T-TUP-1 `let pair = (1, "one") · console.log(pair, pair[0], pair[1])` → `(1, "one") 1 one`
- T-TUP-2 `let typed: Tuple(Integer, String) = (1, "one") · let (n, t) = typed · console.log(n, t)` → `1 one`
- T-TUP-3 `function divide = (a: Integer, b: Integer(min = 1)): Tuple(Integer, Integer) => (a // b, a %% b) · let (q, r) = divide(17, 5) · console.log(q, r)` → `3 2`
- T-TUP-4 `console.log((1, "a") == (1, "a"), (1, "a") != (2, "a"))` → `true true`
- T-TUP-5 `mutable t = (1, 2) · let u = t · t[0] = 5 · console.log(t, u)` → `(5, 2) (1, 2)`
- T-TUP-6 `let f: Tuple(Float, Integer) = (1, 2) · console.log(f[0] / 2.0)` → `0.5` (literal adapts)
- T-TUP-7 `let x = (1) · console.log(x + 1)` → `2` (grouping)
- T-TUP-20 `let u = ()` ⇒ E-TUP-1
- T-TUP-21 `function readIndex = (): Integer => 0 · let pair = (1, "one") · let v = pair[readIndex()]` ⇒ E-TUP-2
- T-TUP-22 `let pair = (1, "one") · let v = pair[2]` ⇒ E-TUP-3; `pair[-1]` ⇒ E-TUP-3
- T-TUP-23 `let pair = (1, "one") · let v = pair.0` ⇒ E-TUP-4
- T-TUP-24 `type Point = { x: Integer, y: Integer } · let t: Tuple(Integer, Integer) = (3, 4) · let p: Point = t` ⇒ E-TYPE-1
- T-TUP-25 `let c = (1, 2) < (1, 3)` ⇒ E-CMP-2
- T-TUP-26 `let c = (1, 2) == (1, 2, 3)` ⇒ E-CMP-1
- T-TUP-27 `let pair = (1, 2) · pair[0] = 3` ⇒ E-DECL-4
- T-TUP-28 `let (a, b, c) = (1, 2)` ⇒ E-DST-5

---

## 11. Objects

Sources: `values/objects`, `getting-started/syntax-basics#braces`, `types/type-conversion`, `advanced/structural-typing` (for context).

### Syntax (as parsed)

- Object literal: `{ a = 1, b: T = 2, c, ...d }` (`Node::Members`; items separated by commas or newlines). Typed literal: `Point { x = 1 }`, `Point(Float) { … }`. Field access `a.b`; field assignment `a.b = v`; multi-field update `a .= { b = 1, c = 2 }` (`CompoundAssignment { operator: Update }`, the parser requires an object literal on the right).
- In a type: fields `name: T`, `name: T = default`, `name = default`, `readonly`, `#private`, methods `function m = (self, …) => …`, spreads `...Parent` (types are out of scope here).

### Static semantics

**Creating.**

- With an expected object type (annotation, parameter, field type, return type) the literal is checked against it; without one, `Type { … }` names it; without either, the literal has an anonymous object type whose field types are the widened types of the values (or the in-literal annotations `foo: Integer = 12`).
- Every field without default and not optional must be given: `E-OBJ-2` ("missing field `y`"). A field the type lacks: `E-OBJ-1` ("`z` is not a field of `Point`"). A field given twice: `E-OBJ-3`. A `#private` field given outside its type: `E-OBJ-4`. A method given a value: `E-OBJ-11`.
- Missing optional field without default: `none`. Missing field with a default: the default, evaluated at each creation (it can't refer to other fields).
- `readonly` fields can be given in the creating literal.
- Shorthand `{ id }` = `{ id = id }` (the name must exist: E-DECL-1).
- Spread `...value`: copies the public data fields of `value` (an object type: `E-OBJ-9` otherwise, incl. an optional object). A field written **after** a spread replaces the copied one and must have the same type (`E-OBJ-7`). A field written **before** a spread that provides it again is `E-OBJ-8` [D-79]. A later spread replaces fields of an earlier spread (same type required, E-OBJ-7). The spread's fields must all exist in the expected type (`E-OBJ-1`) [D-78].
- `let origin: Point = Point { … }`: `W-OBJ-1` (type written twice).
- `Point(3, 4)` / `Point(x = 3, y = 4)`: `E-OBJ-6` ("write `Point { x = 3, y = 4 }`").
- After `=>` (lambda body, `catch` handler, `when` branch) a `{` is a block: returning an object requires the type name (`=> Point { x = 0, y = 0 }`); a block whose content looks like fields (`=> { x = 1 }`) gives E-DECL-1 for the undeclared `x` with a hint "to return an object, write its type before the braces" (`E-OBJ-14`, specialised message).

**Reading and writing.**

- `a.b`: unknown member `E-OBJ-5`; `#private` from outside `E-OBJ-4`; on an optional `E-OPT-1`.
- `a.b = v`: `a` must be a mutable place (E-DECL-3/4); `readonly` from outside the type's mutating methods `E-OBJ-10`; replacing a method `E-OBJ-11`; function pointer fields can be replaced on mutable instances.
- Methods are resolved from the type written at the use site. A method declaring `mutates self` needs a mutable receiver (E-DECL-5).
- Assigning an object copies it (deep for owned data; relation fields per `advanced/relations`).

**`.=`** (`settings .= { port = 8080, secure = true }`):

- Target: a mutable place of an object type (not optional: E-OPT-1).
- Right side: an object literal written in place (E-OBJ-13 otherwise: no variable, no spread); each field must exist (E-OBJ-5), be writable (E-OBJ-10, E-OBJ-4, E-OBJ-11), have a value of its type (E-TYPE-1), at most once (E-OBJ-3); shorthand allowed. Shallow: nested objects are updated with their own `.=`.
- Semantics [D-17]: the field values are evaluated left to right **first**, then the fields are assigned in written order, in place (the object is not recreated; references to the object stay valid; references into the replaced fields are invalidated). So `p .= { x = p.y, y = p.x }` swaps.
- Effect (fine-grained [D-18]): `mutates settings.port, settings.secure`, not `mutates settings`; it ends the narrowings and facts of those field paths only.
- A statement of type `Void`.

**Equality**: structural and deep over public data fields; relation and `#private` fields ignored; a type containing a function pointer can't be compared (`E-CMP-4`); operand types must be compatible (E-CMP-1). Section 13.

### Diagnostics

| Id | Condition |
| --- | --- |
| E-OBJ-1 | literal or spread field absent from the expected type |
| E-OBJ-2 | required field missing |
| E-OBJ-3 | field given twice |
| E-OBJ-4 | `#private` member used or given outside its type |
| E-OBJ-5 | no such field or method |
| E-OBJ-6 | positional/named constructor call on an object type |
| E-OBJ-7 | field replacing a spread field with another type |
| E-OBJ-8 | field written before a spread that provides it |
| E-OBJ-9 | spread of a non-object (or optional) value |
| E-OBJ-10 | write to a `readonly` field outside the type's methods |
| E-OBJ-11 | assigning a method |
| E-OBJ-12 | a field and a method with the same name (type declaration) |
| E-OBJ-13 | `.=` with a right side that is not a plain object literal |
| E-OBJ-14 | block after `=>` that looks like an object literal (hint) |
| W-OBJ-1 | type written twice |

### Runtime semantics

Field values are evaluated in written order; spreads at their position; then defaults of missing fields, in declaration order. Objects are stored inline and copied on assignment.

### Lowering

- Native: struct in canonical field order (alignment, then name; `compilation-with-llvm.md` Q1), built in its destination (no temporary); methods are static functions; function pointer fields are `{ fn, env }`. `.=` = the field stores. Copy/equality/drop generated per shape.
- JavaScript [D-81]: a JS object literal with the fields in the **type's declaration order** (one hidden class per type); when the written order differs and values have effects, values go to temps first. Methods are free functions named after the type (`Point$describe(p)`), not prototype methods (method resolution is by the static type). Copies: a generated per-type copy function (never a spread for a narrowing copy). `.=`: temps for all values, then property stores. Equality: generated per type.

### Tests

- T-OBJ-1 `let original = { count = 1 } · mutable copy = original · copy.count = 2 · console.log(original.count, copy)` → `1 { count = 2 }`
- T-OBJ-2 `type Point = { x: Integer, y: Integer } · let origin: Point = { x = 0, y = 0 } · let other = Point { x = 1, y = 2 } · console.log(origin, other)` → `{ x = 0, y = 0 } { x = 1, y = 2 }`
- T-OBJ-3 `type M = { s: String, d = "Hello", o: String?, od: String? = "Hi" } · let m = M { s = "a" } · console.log(m)` → `{ s = "a", d = "Hello", o = none, od = "Hi" }`
- T-OBJ-4 `let name = "Ana" · let age = 30 · type Person = { name: String, age: Integer } · let p = Person { name, age = 31 } · console.log(p)` → `{ name = "Ana", age = 31 }`
- T-OBJ-5 `type S = { host: String, port: Integer, secure: Boolean } · let defaults = S { host = "localhost", port = 80, secure = false } · let custom: S = { ...defaults, port = 8080 } · console.log(custom)` → `{ host = "localhost", port = 8080, secure = false }`
- T-OBJ-6 `type S = { host: String, port: Integer, secure: Boolean } · mutable s = S { host = "a", port = 80, secure = false } · s .= { port = 8080, secure = true } · console.log(s)` → `{ host = "a", port = 8080, secure = true }`
- T-OBJ-7 `type P = { x: Integer, y: Integer } · mutable p = P { x = 1, y = 2 } · p .= { x = p.y, y = p.x } · console.log(p)` → `{ x = 2, y = 1 }` [D-17]
- T-OBJ-8 `type C = { a: Integer, b: Integer } · function n = (v: Integer): Integer => { · console.log(v) · v · } · let c = C { b = n(2), a = n(1) } · console.log(c)` → `2 / 1 / { a = 1, b = 2 }` (written order of evaluation, declaration order of printing)
- T-OBJ-9 `type A = { readonly balance: Integer = 0, function deposit = (self, amount: Integer(min = 0)) mutates self.balance => { · self.balance += amount · } } · mutable a = A {} · a.deposit(100) · console.log(a.balance)` → `100`
- T-OBJ-10 `type Counter = { #count: Integer = 0, function increment = (self) mutates self => { · self.#count += 1 · }, function current = (self): Integer => self.#count } · mutable c = Counter {} · c.increment() · console.log(c.current())` → `1`
- T-OBJ-11 `type Point = { x: Integer, y: Integer } · console.log(Point { x = 1, y = 2 } == Point { x = 1, y = 2 }, Point { x = 1, y = 2 } != Point { x = 2, y = 1 })` → `true true`
- T-OBJ-12 `type Human = { name: String } · type Super = { ...Human, powers: String } · let s = Super { name = "John", powers = "all" } · let h: Human = s · console.log(h)` → `{ name = "John" }`
- T-OBJ-13 `type T = { onClick: () => Void } · mutable t = T { onClick = () => console.log("a") } · t.onClick() · t.onClick = () => console.log("b") · t.onClick()` → `a / b`
- T-OBJ-14 `let o = { nested = { n = 3 } } · console.log(o.nested.n)` → `3`
- T-OBJ-15 `type Point = { x: Integer, y: Integer } · let points = [1, 2].map((v) => Point { x = v, y = 0 }) · console.log(points)` → `[{ x = 1, y = 0 }, { x = 2, y = 0 }]`
- T-OBJ-20 `type Point = { x: Integer, y: Integer } · let p: Point = { x = 12, y = 13, z = 14 }` ⇒ E-OBJ-1
- T-OBJ-21 `type Point = { x: Integer, y: Integer } · let p: Point = { x = 1 }` ⇒ E-OBJ-2
- T-OBJ-22 `let o = { a = 1, a = 2 }` ⇒ E-OBJ-3
- T-OBJ-23 `type C = { #count: Integer = 0 } · let c = C { #count = 3 }` ⇒ E-OBJ-4; `let d = C {} · console.log(d.#count)` ⇒ E-OBJ-4
- T-OBJ-24 `let o = { a = 1 } · console.log(o.b)` ⇒ E-OBJ-5
- T-OBJ-25 `type Point = { x: Integer, y: Integer } · let p = Point(3, 4)` ⇒ E-OBJ-6
- T-OBJ-26 `type S = { port: Integer } · let d = S { port = 1 } · let s: S = { ...d, port = "x" }` ⇒ E-OBJ-7
- T-OBJ-27 `type S = { port: Integer } · let d = S { port = 1 } · let s: S = { port = 2, ...d }` ⇒ E-OBJ-8
- T-OBJ-28 `let n = 3 · let o = { ...n }` ⇒ E-OBJ-9
- T-OBJ-29 `type A = { readonly balance: Integer = 0 } · mutable a = A {} · a.balance = 1000` ⇒ E-OBJ-10
- T-OBJ-30 `type T = { function d = (self): String => "x" } · mutable t = T {} · t.d = () => "y"` ⇒ E-OBJ-11
- T-OBJ-31 `type S = { port: Integer } · mutable s = S { port = 1 } · let patch = { port = 2 } · s .= patch` ⇒ E-OBJ-13 (parser error today)
- T-OBJ-32 `type S = { port: Integer } · mutable s = S { port = 1 } · s .= { port = 2, host = "a" }` ⇒ E-OBJ-5
- T-OBJ-33 `let myObject = { foo = 1 } · myObject.foo += 1` ⇒ E-DECL-4
- T-OBJ-34 `type Point = { x: Integer, y: Integer } · let origin: Point = Point { x = 0, y = 0 }` ⇒ W-OBJ-1
- T-OBJ-35 `type T = { f: () => Void } · let a = T { f = () => console.log("") } · let eq = a == a` ⇒ E-CMP-4
- T-OBJ-36 `type Point = { x: Integer, y: Integer } · let ps = [1].map((v) => { x = v, y = 0 })` ⇒ E-OBJ-14

---

## 12. Ranges

Sources: `values/ranges`, `control-flow/loops`, `operations/logical#membership`.

### Syntax (as parsed)

`a..b`, `a..=b`, `a..`, `..b`, `..=b`, `..` (`Node::Range { start?, end?, inclusive }`); `a..=` without end is a parse error. Precedence below arithmetic (`0..count + 1` is `0..(count + 1)`), above comparisons.

### Static semantics

- Type `Range` = `{ start: Integer?, end: Integer?, includesEnd: Boolean }` (a plain object type: fields readable, writable on a `mutable` range). `1..3` is `Range { start = 1, end = 3, includesEnd = false }`.
- Bounds: integer expressions whose interval fits `Integer` (an `Int8` bound is fine; a `UInt64` bound needs proof or conversion, E-NUM-14) [D-41]; float, `BigInt`, string or character bounds: `E-RNG-2`.
- A range with literal (or fact-known) bounds keeps them as facts.
- In `for`: needs a start (`E-RNG-3`, "this range has no start"); the loop variable is an immutable `Integer` copy with interval `[start.lo, end.hi − 1]` (`end.hi` for `..=`; the type maximum without end).
- Slicing `xs[r]`/`s[r]`: bounds from `−length` to `length`; missing bounds taken from the collection; inverted ranges give an empty result; unproven: `throws IndexError`.
- Pattern (`x is 0..60`, `when` branches): the subject must be of an integer type (`E-RNG-4`: "compare floats with `<` and `>=`"); bounds are any integer expressions; only literal bounds count for exhaustiveness. A range stored in a name is a value: `x is passing` is E-CMP-5 ("use `is in`").
- `is in` with a `Range` value: membership (`start ≤ x` and `x < end` / `x ≤ end`, missing bounds unbounded); narrows only when the range's bounds are known facts.
- `x is in 0..10` (a range *literal* after `is in`): `W-RNG-1`, "write `x is 0..10`" [D-66].
- Equality of ranges: structural (`(1..3) == (1..3)`); no ordering.

### Runtime semantics

- A range whose end is not after its start is empty (`5..2`, `3..3`); iteration counts up by one; the range is evaluated once before the loop.
- `for i in a..` runs until `break`/`return`; if `i` would exceed the `Integer` maximum, the program stops [D-40]. `for i in a..=Integer.maxValue` must terminate after the maximum (no wrap) [D-40].

### Lowering

- Native: `{ i64 start, i64 end, i1 hasStart, i1 hasEnd, i1 includesEnd }` (or optionals with niches); `for` over a range literal is a counted loop with `nsw` increments; an inclusive end is lowered as `if (i == end) break` before the increment.
- JavaScript: a `Range` value is an instance of a runtime class `FaRange { start, end, includesEnd }` (`undefined` for a missing bound); `for i in a..b` → `for (let i = a; i < b; i++)` (`<=` for `..=`), with `a`/`b` evaluated once into temps; `is a..b` → `a <= x && x < b`.

### Tests

- T-RNG-1 `let r = 1..3 · console.log(r, r.start, r.end, r.includesEnd)` → `1..3 1 3 false`
- T-RNG-2 `mutable r = 1..3 · r.start = 2 · r.end = 4 · console.log(r)` → `2..4`
- T-RNG-3 `for i in 0..3 { console.log(i) }` → `0 / 1 / 2`; `for i in 0..=3 { console.log(i) }` → `0 / 1 / 2 / 3`
- T-RNG-4 `for 1..=3 { console.log("hello") }` → `hello / hello / hello`
- T-RNG-5 `for i in 5..2 { console.log(i) } · for i in 3..3 { console.log(i) } · console.log("done")` → `done`
- T-RNG-6 `for i in 1.. { · if i > 3 { break } · console.log(i) · }` → `1 / 2 / 3`
- T-RNG-7 `let data = [10, 20, 30, 40, 50] · console.log(data[1..3], data[1..=3], data[2..], data[..2], data[..])` → `[20, 30] [20, 30, 40] [30, 40, 50] [10, 20] [10, 20, 30, 40, 50]`
- T-RNG-8 `function count = (): Integer => 9 · let data = [1, 2, 3] · console.log(data[0..count()] catch error => [])` → `[]`
- T-RNG-9 `let age = 20 · if age is 18.. { console.log("Adult") }` → `Adult`
- T-RNG-10 `let score = 150 · if score is not 0..=100 { console.log("Invalid") }` → `Invalid`
- T-RNG-11 `let passing = 60..=100 · let score = 70 · console.log(score is in passing, 59 is in passing)` → `true false`
- T-RNG-12 `function middleThird = (length: Integer): Range => (length // 3)..(2 * length // 3) · let data = [1, 2, 3, 4, 5, 6, 7, 8, 9] · console.log(data[middleThird(data.length)] catch error => [])` → `[4, 5, 6]`
- T-RNG-13 `let n = 5 · for i in 0..n - 1 { console.log(i) }` → `0 / 1 / 2 / 3` (`0..(n - 1)`)
- T-RNG-14 `mutable n = 3 · for i in 0..n { n += 1 } · console.log(n)` → `6` (evaluated once)
- T-RNG-15 `console.log(.., ..10, ..=10, 5..)` → `.. ..10 ..=10 5..`
- T-RNG-16 (native) `function m = (): Integer => 9_223_372_036_854_775_806 · mutable c = 0 · for i in m()..=9_223_372_036_854_775_807 { c += 1 } · console.log(c)` → `2` (no wrap)
- T-RNG-20 `for i in ..10 { }` ⇒ E-RNG-3
- T-RNG-21 `let r = 0.5..2.5` ⇒ E-RNG-2; `let s = "a".."z"` ⇒ E-RNG-2
- T-RNG-22 `let f = 1.5 · if f is 0..2 { }` ⇒ E-RNG-4
- T-RNG-23 `let passing = 60..=100 · let s = 70 · if s is passing { }` ⇒ E-CMP-5
- T-RNG-24 `let x = 1..=` ⇒ parse error
- T-RNG-25 `let s = 5 · if s is in 0..10 { }` ⇒ W-RNG-1
- T-RNG-26 `for i in 0..3 { i = 5 }` ⇒ E-DECL-3

---

## 13. Comparison, equality, `is`, membership, logic

Sources: `operations/logical`, `control-flow/conditionals#narrowing`, `types/unions` (runtime tests), `values/strings#characters`.

### Syntax (as parsed)

`a == b`, `a != b` (`Equal`/`NotEqual`, list nodes), `<`, `<=`, `>`, `>=` (binary nodes; chains are nested, to be replaced by a chain node [D-3]), `a is P`, `a is not P` (`Is { negated }`), `a is in c`, `a is not in c` (`IsIn { negated }`), `not`, `and`, `or`, `??`.

### Static semantics

**Equality** (`==`, `!=`, result `Boolean`):

- The operand types must be **compatible**: one is accepted where the other is expected. Specifically: an optional with its base type (or `none`); integers of any sizes and ranges with each other; `String` with `Character`; a literal adapts to the type of the other operand (`name[0] == "F"` compares characters; `point == { x = 1, y = 2 }` types the literal as the other side's type); structural object types where one is accepted by the other. Otherwise `E-CMP-1` ("cannot compare `String` with `Integer`"). Integers with floats, floats of different widths, `BigInt` with integers, `Percentage` with numbers: E-CMP-1.
- A type containing a function (function pointer field, function value) can't be compared: `E-CMP-4`.
- The right side is a type name (`living == Animal`): `E-CMP-8` ("`Animal` is a type: use `is`").
- Chaining `a == b == c`, `a != b == c`: `E-CMP-3`.
- Semantics: deep and structural. Objects: public data fields (relation fields and `#private` fields ignored). Arrays: items in order. `Set`, `Pool`, `Map`: without order (maps: same keys, equal values). Tuples: element-wise. Floats: IEEE (`NaN != NaN`, `-0.0 == 0.0`), also inside composites (an object holding NaN is not equal to itself). Unions: same member and equal contents. Optionals: `none == none`; `none != v`. Identity is not observable.
- Narrowing: only when one side is a literal or `none` (section 14).

**Ordering** (`<`, `<=`, `>`, `>=`):

- Ordered types: integers (any sizes, mixed), floats of the same type, `BigInt`, `Percentage`, `String`/`Character` (code point order). Anything else (`Boolean`, tuples, objects, collections, optionals, unions): `E-CMP-2`. Mixed kinds: E-CMP-1.
- Chains: `a < b <= c` means `a < b and b <= c` with `b` evaluated once, short-circuiting. All operators of a chain go in one direction (`<`/`<=` or `>`/`>=`): mixing is `E-CMP-3`. A parenthesized comparison is a `Boolean` operand: `(a < b) < c` is E-CMP-2.
- NaN: every ordering is `false`.

**`is` / `is not`**: the right side is a **pattern**: a literal (number, string, `true`/`false`, `none`), a type, a qualified member of a named union, an enumeration or an error type, a whole named union or error type, a range with integer bounds, or alternatives `P1 | P2` (parser fix [D-2]). Never a value:

- a name that denotes a value (`score is passing`): `E-CMP-5` ("`passing` is a value: use `==`, or `is in` for a `Range`");
- an unqualified union member (`error is NotFound`): `E-CMP-9` ("write `ApiError.NotFound`");
- `as` after `is`: `E-CMP-10` ("bindings with `as` only exist in `when`").

Decided at compile time [D-28]: `is Type` on a value whose type is not a union, and any pattern whose type can't match the value's type: always true → `W-CMP-1`, never true → `E-CMP-7` (`5 is none`, `count is "a"`, `x is String` with `x: Integer`). Facts that make a runtime test constant (a known literal) give `W-BOOL-1`, not an error. At runtime, the members of an anonymous union must be distinguishable (`types/unions#members-must-be-distinguishable`; the checker rejects indistinguishable unions where the union type is formed, `E-TYPE-2`).

**`is in` / `is not in`**: the right side is an `Array`, `Set`, `Pool`, `Columnar` or a growing version (an element equal to the value), a `Map` (a key equal to the value), or a `Range` (an integer between its bounds). A `String` or anything else: `E-CMP-6` ("to search a string, use `includes`"). The value must be compatible with the element/key type (E-CMP-1). A range literal on the right: W-RNG-1. Narrowing: only with a literal array (to the union of its literals).

**`not`, `and`, `or`**: section 8. **`??`**: section 9.

### Diagnostics

| Id | Condition |
| --- | --- |
| E-CMP-1 | incompatible operand types (`==`, `!=`, ordering, `is in` element) |
| E-CMP-2 | ordering on a non-ordered type |
| E-CMP-3 | chained `==`/`!=`, or a comparison chain mixing directions |
| E-CMP-4 | equality on a type containing a function |
| E-CMP-5 | value name used as a pattern after `is` |
| E-CMP-6 | `is in` with an operand that is not a collection, map or range |
| E-CMP-7 | `is` test that can never be true (type level) |
| E-CMP-8 | `==` with a type |
| E-CMP-9 | unqualified union member in a pattern |
| E-CMP-10 | `as` after `is` |
| W-CMP-1 | `is` test that is always true (type level) |

### Runtime semantics

Operands are evaluated left to right, each once; chains and `and`/`or` short-circuit.

### Lowering

- Native: scalar compares (`icmp`, `fcmp oeq/olt…` — ordered predicates, so NaN gives `false`; `fcmp une` for `!=`); composites call the generated `eq<Shape>` (never `memcmp`: padding, floats, owned data); strings compare lengths then bytes; `is Type` on a union = a tag compare; `is` range = two compares (one when a bound is absent); `is in` on arrays = a loop with `eq<T>`, sets/maps = hash lookup.
- JavaScript: primitives `===`/`!==` (mixed `BigInt`/number integers: convert, [D-69]); composites: generated `$eq$Type(a, b)`; strings `===`; ordering on strings: code-point helper when needed (section 7); chains: temps for the middle operands; `is Type` on an anonymous union: `typeof` tests / field presence / discriminant comparison, on a named union: tag compare; `is in` on an array of primitives: `indexOf(x) !== -1` (not `includes`, which finds `NaN` — Fa's `==` doesn't) [D-65]; of composites: `some((e) => $eq(e, x))`; `Set`/`Map`: `.has` (SameValueZero, as documented for sets and map keys); `Range`: bound compares.

### Tests

- T-CMP-1 `console.log(42 == 42, 42 != 0, "a" == "a")` → `true true true`
- T-CMP-2 `console.log([1, 2, 3] == [1, 2, 3], [1, 2, 3] != [3, 2, 1])` → `true true`
- T-CMP-3 `let a: Set(Integer) = [1, 2, 3] · let b: Set(Integer) = [3, 2, 1] · console.log(a == b)` → `true`
- T-CMP-4 `let a: Int8 = 5 · let b: UInt16 = 5 · console.log(a == b, a < b)` → `true false`
- T-CMP-5 `let x = 50 · console.log(0 < x <= 100, 0 < x < 10)` → `true false`
- T-CMP-6 `function f = (): Integer => { · console.log("f") · 5 · } · console.log(0 < f() <= 10)` → `f / true` (evaluated once)
- T-CMP-7 `function f = (): Integer => { · console.log("f") · 5 · } · console.log(10 < 0 < f())` → `false` (short-circuit: `f` not called)
- T-CMP-8 `let status = "ready" · console.log(status is "ready", status is not "ready", status is "a" | "ready")` → `true false true`
- T-CMP-9 `let answer = "yes" · console.log(answer is "y" | "yes")` → `true`
- T-CMP-10 `let living: Integer | String = "x" · console.log(living is String, living is Integer)` → `true false`
- T-CMP-11 `let role = "admin" · console.log(role is in ["admin", "moderator"], role is not in ["guest"])` → `true true`
- T-CMP-12 `let xs: Array(Float) = [0.0 / 0.0] · let n = 0.0 / 0.0 · console.log(n is in xs, xs == xs)` → `false false` [D-65]
- T-CMP-13 `let n = 0.0 / 0.0 · console.log(n < 1.0, n >= 1.0, n == n)` → `false false false`
- T-CMP-14 `console.log("abc" < "abd", "b" > "abc")` → `true true`
- T-CMP-15 `let m: Integer? = 3 · console.log(m == 3, m != none)` → `true true`
- T-CMP-16 `type P = { x: Float } · let n = 0.0 / 0.0 · console.log(P { x = n } == P { x = n })` → `false`
- T-CMP-17 `let r = 1..3 · console.log(r == 1..3)` → `true`
- T-CMP-20 `let a = "1" == 1` ⇒ E-CMP-1
- T-CMP-21 `let n = 1 · let a = n == 1.0` ⇒ E-CMP-1 (an `Integer` value and a `Float` literal; `1 == 1.0` between two literals is valid: the integer literal adapts)
- T-CMP-22 `let a = true < false` ⇒ E-CMP-2; `let b = (1, 2) < (1, 3)` ⇒ E-CMP-2; `let c = [1] < [2]` ⇒ E-CMP-2
- T-CMP-23 `let a = 1 · let b = 2 · let c = 3 · let x = a < b > c` ⇒ E-CMP-3; `let y = a == b == c` ⇒ E-CMP-3
- T-CMP-24 `type T = { f: () => Void } · let t = T { f = () => console.log("") } · let e = t == t` ⇒ E-CMP-4
- T-CMP-25 `let passing = 60..=100 · let s = 70 · let x = s is passing` ⇒ E-CMP-5
- T-CMP-26 `let found = "a" is in "abc"` ⇒ E-CMP-6
- T-CMP-27 `let n = 5 · let x = n is String` ⇒ E-CMP-7; `let y = n is Integer` ⇒ W-CMP-1
- T-CMP-28 `type Animal = { name: String } · let a = Animal { name = "x" } · let x = a == Animal` ⇒ E-CMP-8
- T-CMP-29 `type E = Errors { NotFound, Timeout } · function f = (e: E) => e is NotFound` ⇒ E-CMP-9
- T-CMP-30 `let v: Integer | String = 1 · if v is Integer as n { }` ⇒ E-CMP-10
- T-CMP-31 `let x = (1 < 2) < 3` ⇒ E-CMP-2

---

## 14. Narrowing

Sources: `control-flow/conditionals#narrowing` (canonical), `values/optionals#narrowing`, `values/numbers#range-analysis`, `advanced/reference-checking`.

Narrowing is a **fact** about a place, never a new declaration: the place keeps its declared type (`mutable x: Integer? = 5` can still be assigned `none`), and its narrowed type is used for reads.

### What can be narrowed

- a local value (`let`, `mutable`, `use`, `mutable use`), a parameter (including `self`), a loop variable, a `when`/`catch` binding;
- a path of fields of one of them (`user.address`, `self.owner`) as long as nothing in between can modify the path;
- a module-level value, like a local, but dropped at each `await`.

Not narrowed: calls (`findUser()` twice are two values), indexing (`items[0]`), `?.` chains.

### Tests that narrow

| Test | True branch | False branch |
| --- | --- | --- |
| `v is Type` / `is not` | the matching union members | the others |
| `v is none`, `v == none` | `None` | the base type |
| `v is not none`, `v != none` | the base type | `None` |
| `v is "ready"`, `v == "ready"` (any literal) | the literal (exact fact) | the type without that literal (a union of literals loses it; an open type records an excluded value) |
| `v is 0..60` | the range (`Integer(min = 0, max = 59)` as a fact) | the complement: one interval, or the interval with an excluded interval |
| `v != 0`, `v != literal` | excludes the literal | exact literal |
| `v < e`, `<=`, `>`, `>=` with a constant or a place with known interval | refined interval | refined interval |
| `v is in ["a", "b"]` (literal array only) | `"a" \| "b"` | unchanged |
| `v is Shape.Circle` | the member | the other members |
| `v is P1 \| P2` | union of the narrowings | intersection of the negations |

`==`/`!=` narrow only against a literal or `none`.

### Combining and flow

- `a and b`: `b` and the true branch see `a` true; the false branch sees the join of "`a` false" and "`a` true, `b` false".
- `a or b`: `b` and the false branch see `a` false; the true branch sees the join.
- `not a` swaps.
- Early exit: when a branch ends with `Never` (`return`, `throw`, `break`, `continue`, a `Never` call), the code after the `if` sees the other branch's facts. Same after `assert cond` [D-27]: the rest of the block sees `cond` true.
- `while cond { body }`: the body sees `cond` true; after a loop without `break`, the code sees `cond` false.
- `while let v = e`: `v` is the non-optional type in the body.
- `when` with a plain local subject: narrowed in each branch to the branch's pattern, and to the remaining members in `else`.
- `catch` handler: `error` is narrowed by `when error is` (it is a plain local).
- Closures [D-102]: a closure body sees the narrowings of the places it captures at its creation (an escaping closure captures a copy; a non-escaping one runs during the call); a write to the place inside the closure, or by another callback of the same call, drops the narrowing inside the closure.

### What ends a narrowing (and drops facts)

- An assignment to the place or to a prefix of its path (`user = other` ends `user.address`); the assigned value's facts replace them (assigning a `User` literal keeps `user` narrowed to `User`).
- A call whose effects may modify the place (`mutates user`, `mutates user.address`, an overlapping path, a module-level value in the callee's `mutates`), including writes by a non-escaping closure passed to the call to a captured `mutable` local (CF9).
- A write through any alias that may designate the place (`use` aliases, loop variables), per the "may designate" rules of reference checking.
- `reuse r = …` ends the narrowings that were about what `r` designated.
- An `await`, for module-level values.
- The end of the branch or loop body.
- After branches, the join: a place is narrowed after an `if`/`when` only if every reaching branch narrows it the same way (join of facts).

### Diagnostics

Reading a member of a place that is not (or no longer) narrowed: E-OPT-1 (optional) or `E-NAR-1` (union: "`value` may be a `String` here: test it with `is Integer` first"). Constant tests: W-BOOL-1, W-NUM-2, E-CMP-7/W-CMP-1.

### Lowering

None: narrowing only removes checks and enables direct field access (native: payload access without re-testing the tag; JS: nothing).

### Tests

- T-NAR-1 `function describe = (value: String | Integer): String => { · if value is String { · return "text of length {value.length}" · } · return "number {value}" · } · console.log(describe("ab"), describe(3))` → `text of length 2 number 3`
- T-NAR-2 `type User = { name: String, age: Integer } · function check = (user: User?) => { · if user is not none and user.age >= 18 { console.log("{user.name} is an adult") } · } · check({ name = "Ana", age = 20 })` → `Ana is an adult`
- T-NAR-3 `type User = { name: String } · function greet = (user: User?) => { · if user is none or user.name == "" { · return · } · console.log(user.name) · } · greet({ name = "Bo" }) · greet(none)` → `Bo`
- T-NAR-4 `mutable current: Integer? = 3 · mutable n = 0 · while current is not none { · console.log(current + 1) · n += 1 · current = if n < 2 { 10 } else { none } · }` → `4 / 11`
- T-NAR-5 `function f = (divisor: Integer) => { · if divisor == 0 { · return · } · console.log(100 // divisor) · } · f(0) · f(7)` → `14` (no `try`: excluded value)
- T-NAR-6 `function g = (score: Integer(min = 0, max = 100)) => { · if score is 0..60 { return } · let s: Integer(min = 60, max = 100) = score · console.log(s) · } · g(75)` → `75`
- T-NAR-7 `function h = (role: String) => { · if role is in ["admin", "owner"] { · let r: "admin" | "owner" = role · console.log(r) · } · } · h("owner")` → `owner`
- T-NAR-8 `type User = { name: String } · mutable user: User? = { name = "A" } · if user is not none { · user = { name = "B" } · console.log(user.name) · }` → `B` (the assigned value's facts keep the narrowing)
- T-NAR-9 `function find = (): Integer? => none · mutable x: Integer? = 1 · if x is not none { · x = find() · console.log(x + 1) · }` ⇒ E-OPT-1
- T-NAR-10 `type B = { v: Integer? } · function reset = (b: B) mutates b => { · b.v = none · } · mutable b = B { v = 1 } · if b.v is not none { · reset(b) · console.log(b.v + 1) · }` ⇒ E-OPT-1
- T-NAR-11 `function f = (x: Integer?) => { · assert x is not none · console.log(x + 1) · } · f(1)` → `2` [D-27]
- T-NAR-12 `let v: Integer | String = 1 · let n = v + 1` ⇒ E-NAR-1
- T-NAR-13 `function f = (x: Integer?): Integer => { · let items = [1, 2] · items.map((i) => if x is none { 0 } else { x + i }).length · }` valid (narrowing inside a closure)
- T-NAR-14 `function f = (x: Integer?) => { · if x is not none { · let g = () => x + 1 · console.log(g()) · } · } · f(4)` → `5` [D-102]
- T-NAR-15 `function k = (u: Integer?) => { · when u is { · none => console.log("nobody") · Integer => console.log(u + 1) · } · } · k(none) · k(1)` → `nobody / 2`

---

## 15. Errors

Sources: `values/errors` (incl. `:::implementation`), `operations/arithmetic#order-of-operations`, `functions/defining-functions` (effects inference, callbacks), reference "Errors and optionals".

### Syntax (as parsed)

- Error types: `type ApiError = Errors { AuthenticationError = { userId: String? }, TimeoutError, … }` (union syntax).
- `throw <expr>` (`Throw`, precedence 17: covers everything on its right).
- `try <expr>` (`Try`, precedence 15: up to a `catch`); `try await f()`.
- `<expr> catch <name> => <expr-or-block>` (`Catch { expression, error_name, handler }`, precedence 16; handler extends as far right as possible).
- Signatures: `(…): T throws E1 | E2 => …`; function types `(…) => T throws E`.
- `assert <expr>` (`Assert`).

### Static semantics

**Error values.** Every member of an `Errors` type satisfies the built-in `Error` type, whose only member is `toString(): String` (qualified member name + data, e.g. `ApiError.NetworkError { message = "Connection lost" }`). Members are always qualified (`ApiError.TimeoutError`). Built-in error types (each matched by its name in `when`): `IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`; their data is not specified: proposal [D-25] `IndexError { index: Integer, length: Integer }`, `KeyNotFound { key: String }` (the key's `toString`), `DivisionByZero` and `ArithmeticError` without data.

**Failing expressions.** An expression **can fail** with a set of error types when it is: a call to a function or function value whose type declares `throws` (or whose inferred effects include errors, or a call whose non-stored callback arguments throw), an index/key/slice not proven, an integer/`BigInt` `//`/`%%` with an unproven divisor, a conversion not proven to fit, an integer `**` with an unproven exponent, a shift by an unproven amount, a `checked*` method, an array destructuring not proven, an interpolation or argument containing one of these.

**Coverage rule.** Every failing expression must be covered, in the same function body (lambdas are their own bodies), by either:

- a `try` whose operand contains it: its errors are propagated; the enclosing function must declare them (`throws`), or infer them (a `function` with a single-expression body; a lambda whose expected function type declares them); otherwise `E-ERR-4` ("`try` propagates `ApiError`, which `printScore` doesn't declare: add `throws ApiError`, or handle it with `catch`"). At the top level of a standalone file and in a test body, `try` is allowed: an escaped error ends the program with exit code 1 (`error: <toString>` on stderr) or fails the test [D-59];
- a `catch` whose left operand contains it.

Uncovered: `E-ERR-3` ("`loadScore` can fail with `ApiError`. Use `try` to propagate the error, or `catch` to handle it."). For a failing assignment target: `try scores[index] = 0`, `try counts[key] += 1`; for a failing array destructuring: `try let [a, b] = values`.

**`throw e`**: `e` must be an error value (`E-ERR-1` otherwise: `throw 5`, `throw "x"`); its type must be declared by the enclosing function (`E-ERR-2`: "`saveScore` does not declare `ApiError`"), inferred for single-expression bodies, taken from the expected type for lambdas. `throw` has type `Never`. In a `catch` handler, `throw error` rethrows: after a `when error is { … else => throw error }`, `error` is narrowed to the remaining members, and only those must be declared.

**`catch`**:

- The name is required and exists only in the handler; its type is the union of the error types of the failing expressions of the left operand (left operand = everything the precedence gives it: `a + f() catch e => 0` covers `a + f()`).
- The handler is an expression or a block (`{` after `=>` is a block); `return`, `throw`, `break`, `continue` in it act on the enclosing function or loop (it is not a function).
- Type [D-98]: like the branches of an `if`: with an expected type, both the left operand and the handler are checked against it; without one, the handler is checked with the left operand's type as expected type (so `items[i] catch error => 0` stays an `Integer`, and `s[i] catch error => "?"` a `Character`), and the result is the union of both widened types; a `Never` handler adds nothing.
- `when error is { … }` in the handler must be exhaustive over the error union.

**Misuses:**

| Id | Condition |
| --- | --- |
| E-ERR-1 | `throw` of a value that is not an error |
| E-ERR-2 | `throw` of an error type the function doesn't declare |
| E-ERR-3 | failing expression not covered by `try` or `catch` |
| E-ERR-4 | `try` propagating errors the function doesn't declare |
| E-ERR-5 | `try`/`throw` in a context that can't propagate (a `watch` block, a detached `start do`, a function type without `throws` for a lambda) |
| E-ERR-6 | `try` and `catch` on the same expression (`try a catch e => b`, `try (a catch …)`), or a `try` anywhere inside the left operand of a `catch` in the same body [D-23] |
| E-ERR-7 | an error type in a return type or a value union (`: Integer \| ApiError`): "errors are declared with `throws`" |
| E-ERR-8 | `await try f()`: "write `try await f()`" |
| E-ERR-9 | `catch` without a name, or a name reused in its handler scope |
| W-ERR-1 | `try` on an expression that can't fail [D-23] |
| W-ERR-2 | `catch` on an expression that can't fail |
| W-ERR-3 | `throws E` declared but never thrown (CF14) |
| W-ERR-4 | `is Error` decided at compile time (always true; never true is E-CMP-7) |

Errors are not falsy (`if result` is E-BOOL-1); `?.`/`??` don't apply to failing calls (E-OPT-6); there is no `!` (E-LEX-4); there is no postfix `?` on calls (parse error: "use `try`").

**`assert cond`**: `cond` is a `Boolean` (E-BOOL-1); always evaluated (no release-mode removal) [D-27]; false: a stop outside tests (`assertion failed: <source text of cond>`), a test failure in a test; narrows the rest of the block.

### Runtime semantics

- `throw` exits the current function with the error (after running the scope cleanups); the caller's `try` propagates it further; `catch` evaluates the handler instead of the failed value. A call applies its effects even when it throws (MR29).
- Evaluation order inside a `catch` left operand: normal; at the first failure, the remaining evaluation is skipped and the handler runs.
- Stops are not errors (section 2.7).

### Lowering

- Native (`values/errors:::implementation`, LLVM "Functions and calls"): a `throws` function returns `T | E` (tag + payload, `sret` when large); `try` is a tag compare with a cold error path that runs the cleanups and returns the error; `catch` branches to the handler block; `throw` builds the error in the return slot. No unwinding, `nounwind` everywhere.
- JavaScript [D-24, D-92]: error members compile to classes extending a runtime base `FaError` (not `Error`: no stack capture on the hot failure path; a debug build may capture one); `throw` → `throw new ApiError$TimeoutError(30)`; `try` → nothing (JS propagation); `catch` → `try { $t = <left> } catch ($e) { if (!($e instanceof FaError)) throw $e; const error = $e; $t = <handler> }`, which rethrows `FaStop` and any JavaScript exception that is not a Fa error. The `try` block covers exactly the left operand, hoisted into a statement by the expression lowering (section 19). Built-in errors are runtime classes (`IndexError`…). The entry point catches an escaped `FaError`, prints `error: ${e.toString()}`, exits with 1.

### Tests

- T-ERR-1 `type ApiError = Errors { NotFound = { id: String }, Timeout } · function load = (id: String): Integer throws ApiError => { · if id == "" { · throw ApiError.NotFound { id } · } · 42 · } · console.log(load("") catch error => 0, load("a") catch error => 0)` → `0 42`
- T-ERR-2 same types · `function twice = (id: String): Integer throws ApiError => { · let v = try load(id) · v * 2 · } · console.log(twice("a") catch error => -1)` → `84`
- T-ERR-3 `type E = Errors { A, B = { n: Integer } } · function f = (k: Integer): Integer throws E => when k is { · 0 => throw E.A · 1 => throw E.B { n = 7 } · else => k · } · function show = (k: Integer) => { · let v = f(k) catch error => when error is { · E.A => -1 · E.B as { n } => n · } · console.log(v) · } · show(0) · show(1) · show(5)` → `-1 / 7 / 5`
- T-ERR-4 `type E = Errors { A, B } · function f = (): Integer throws E => throw E.B · function g = (): Integer throws E => { · let v = f() catch error => when error is { · E.A => 0 · else => throw error · } · v · } · console.log(g() catch e => 9)` → `9`
- T-ERR-5 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · function g = () => { · let v = f() catch error => { · console.log("failed: {error}") · return · } · console.log(v) · } · g()` → `failed: E.A`
- T-ERR-6 `function third = (items: Array(Integer)): Integer throws IndexError => try items[2] · console.log(third([1, 2, 3]), third([1]) catch e => -1)` → `3 -1`
- T-ERR-7 `function set = (s: Array(Integer), i: Integer) mutates s throws IndexError => { · try s[i] = 0 · } · mutable xs = [1, 2] · set(xs, 1) catch e => console.log("out") · set(xs, 5) catch e => console.log("out") · console.log(xs)` → `out / [1, 0]`
- T-ERR-8 `type E = Errors { NetworkError = { message: String } } · let e = E.NetworkError { message = "Connection lost" } · console.log(e.toString())` → `E.NetworkError { message = "Connection lost" }`
- T-ERR-9 `function parse = (t: String): Integer throws ParseError => try Integer.parse(t) · let total = (parse("1") + parse("x")) catch error => -1 · console.log(total)` → `-1` (one `catch` covers both calls)
- T-ERR-10 `function f = (x: Integer): Integer => { · let r = do { · if x < 0 { return -1 } · x · } · r · } · console.log(f(-5), f(5))` → `-1 5`
- T-ERR-11 `assert 1 + 1 == 2 · console.log("ok")` → `ok`; `assert 1 + 1 == 3` ⇒ stop (`assertion failed: 1 + 1 == 3`, exit 101) — also W-BOOL-1 (constant)
- T-ERR-12 (standalone file) `type E = Errors { Boom } · function f = (): Integer throws E => throw E.Boom · let v = try f()` ⇒ exit 1, stderr `error: E.Boom`
- T-ERR-13 `type ApiError = Errors { Timeout } · function loadScore = (): Integer throws ApiError => 1 · function printScore = () => { · let score = loadScore() · }` ⇒ E-ERR-3
- T-ERR-14 `function f = () => { · throw 5 · }` ⇒ E-ERR-1
- T-ERR-15 `type ApiError = Errors { TimeoutError = { duration: Integer } } · type StorageError = Errors { Full } · function saveScore = (score: Integer) throws StorageError => { · throw ApiError.TimeoutError { duration = 30 } · }` ⇒ E-ERR-2
- T-ERR-16 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · function g = (): Integer => { · try f() · }` ⇒ E-ERR-4
- T-ERR-17 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · function g = (): Integer throws E => try f() catch e => 0` ⇒ E-ERR-6
- T-ERR-18 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · function g = (): Integer throws E => (try f()) + 1 catch e => 0` ⇒ E-ERR-6
- T-ERR-19 `type E = Errors { A } · function f = (): Integer | E => 1` ⇒ E-ERR-7
- T-ERR-20 `function g = (x: Integer): Integer => try x + 1` ⇒ W-ERR-1; `let y = 5 catch e => 0` ⇒ W-ERR-2
- T-ERR-21 `type E = Errors { A } · function f = (): Integer throws E => { · 1 · }` ⇒ W-ERR-3
- T-ERR-22 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · let v = f() ?? 0` ⇒ E-OPT-6
- T-ERR-23 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · if f() { }` ⇒ E-BOOL-1 (and E-ERR-3)
- T-ERR-24 `type E = Errors { A, B } · function f = (): Integer throws E => throw E.A · let v = f() catch error => when error is { · E.A => 0 · }` ⇒ E-WHEN-1 (`E.B` not handled)
- T-ERR-25 `type E = Errors { A } · function f = (e: E): Boolean => e is Error` ⇒ W-ERR-4
- T-ERR-26 `assert 5` ⇒ E-BOOL-1
- T-ERR-27 (JS) a `catch` around a call into a foreign JS function that throws a `TypeError` must not catch it: the program crashes with the JS error (exit code of the runtime, not a Fa error) — backend test.
- T-ERR-28 `type E = Errors { A } · function f = (): Integer throws E => throw E.A · function g = (): Integer => f() catch error => { · return 3 · } · console.log(g())` → `3` (`return` in a handler returns from `g`)

---

## 16. Destructuring

Sources: `operations/destructuring`, `values/tuples#destructuring-a-tuple`, `control-flow/loops#patterns`, reference "Destructuring".

### Syntax (as parsed)

```
let|mutable|use <pattern> = <expr>          -- Node::Destructuring { kind, pattern, expression }
try let|mutable|use <array-pattern> = <expr>
object pattern:  { f, f as name, f = default, f as name = default, f { nested }, ...rest }   -- ObjectPattern
array pattern:   [p, p, ...rest]                                                         -- ArrayPattern
tuple pattern:   (p, p)                                                                  -- TuplePattern
```

Items are names or nested patterns. Also used in `for` (section 18), parameters (`({ x, y }: Point)`), `when … as` (section 17) and imports (`use { A as B } = ns`, out of scope). Rest patterns are not parsed yet.

### Static semantics

- Keywords: `let` (immutable copies), `mutable` (mutable copies), `use` (one alias per field/item). `mutable use` with a pattern: `E-DST-1` [D-32] (the parser accepts it).
- **Object pattern**: the value must be an object type (an optional: E-OPT-1; another type: `E-DST-2`). Each named field must exist in the type, unless it has a default (`E-DST-3`, "`timeout` is not a field of `Options`: give it a default with `timeout = 30`"). A **default** applies when the field is absent from the type or holds `none`; the binding is then non-optional (type: the field type without `None`, or the default's widened type when the field is absent); the default must be accepted by the field type without `None` (E-TYPE-1); it is evaluated only when used [D-33]. `as` renames (never converts). `:` in a pattern: `E-DST-8` ("`:` is followed by a type: rename with `as`").
- **`use` patterns**: aliases of the fields (`use { a, b } = object` designates `object.a`, `object.b`); a default is `E-DST-4` (a default is not a place) [D-32]; a rest is a copy (documented).
- **Array pattern**: the value must be an `Array` or `GrowingArray` (`E-DST-2` otherwise). Items are read by index: when the length is not proven ≥ the number of items (length facts, `if values.length >= 2`), the declaration can fail with `IndexError` and must be written `try let [...] = …` (E-ERR-3 otherwise); to handle the error instead, check the length first. Extra items are ignored. Defaults in array patterns: `E-DST-6` [D-32].
- **Tuple pattern**: the value must be a tuple of exactly the same arity (`E-DST-5`); a rest in a tuple pattern: `E-DST-6`.
- **Rest** `...name`: at most one, last (`E-DST-9`); object rest: a new object with every other field (its type: the object type minus the named fields); array rest: a new array of the remaining items. Always a copy, even with `use`.
- A name bound twice in one pattern: `E-DST-7`. Bound names follow the declaration rules (E-DECL-2 in the same scope).
- Nested patterns are allowed in all three forms (`let { nested { c } } = o`, `let [a, [b, c]] = m`, `let (a, (b, c)) = t`).
- Facts of the fields carry to the bindings.

### Diagnostics

| Id | Condition |
| --- | --- |
| E-DST-1 | `mutable use` with a pattern |
| E-DST-2 | pattern kind doesn't match the value (object pattern on a number, array pattern on a tuple or a `Set`, …) |
| E-DST-3 | unknown field without default |
| E-DST-4 | default in a `use` pattern |
| E-DST-5 | tuple pattern arity mismatch |
| E-DST-6 | default in an array or tuple pattern; rest in a tuple pattern |
| E-DST-7 | same name bound twice |
| E-DST-8 | `:` inside a declaration pattern |
| E-DST-9 | rest not last, or several rests |

### Runtime semantics

The right side is evaluated once. Object fields are read (copied for `let`/`mutable`) in pattern order; defaults evaluated when the field is `none` or absent from the type; array items read by index after one length check (`IndexError { index = n - 1, length }` when too short).

### Lowering

- Native: field loads/copies from the evaluated value (a temp, or the place itself); `use` patterns are pointers into the place.
- JavaScript: object patterns → JS destructuring (`const { a, b: c = 4 } = $v` — a JS default applies to `undefined`, which is exactly Fa's `none` and absent-field rule, [D-7]); copies of owning fields follow the copy rule (section 5); array patterns → a length check then `const a = $v[0], b = $v[1]` (not the JS iterator protocol); object rest → JS rest (`...details`) followed by the copy rule; array rest → `$v.slice(n)`; `use` patterns of primitive fields → rewritten to the places, as for `use`.

### Tests

- T-DST-1 `let point = { x = 1, y = 2, z = 3 } · let { x, y } = point · console.log(x, y)` → `1 2`
- T-DST-2 `let object = { a = 1, b = 2, nested = { c = 3 } } · let { a as first, b as second, nested { c } } = object · console.log(first, second, c)` → `1 2 3`
- T-DST-3 `type Options = { retries: Integer?, verbose: Boolean } · let options: Options = { retries = none, verbose = true } · let { retries = 3, verbose, timeout = 30 } = options · console.log(retries, verbose, timeout)` → `3 true 30`
- T-DST-4 `type O = { r: Integer? } · let o: O = { r = 5 } · function d = (): Integer => { · console.log("default") · 1 · } · let { r = d() } = o · console.log(r)` → `5` (default not evaluated)
- T-DST-5 `let person = { name = "Ana", age = 3, city = "X" } · let { name, ...details } = person · console.log(name, details)` → `Ana { age = 3, city = "X" }`
- T-DST-6 `let items = [1, 2, 3] · let [first, ...others] = items · console.log(first, others)` → `1 [2, 3]`
- T-DST-7 `let array = [1, 2, 3] · let [first, second] = array · console.log(first, second)` → `1 2` (proven by the literal length)
- T-DST-8 `function firstTwo = (values: Array(Integer)): Tuple(Integer, Integer) throws IndexError => { · try let [first, second] = values · (first, second) · } · console.log(firstTwo([4, 5, 6]), firstTwo([4]) catch e => (0, 0))` → `(4, 5) (0, 0)`
- T-DST-9 `function f = (values: Array(Integer)) => { · if values.length >= 2 { · let [a, b] = values · console.log(a + b) · } · } · f([1, 2])` → `3`
- T-DST-10 `let tuple = (1, "one") · let (number, name) = tuple · console.log(number, name)` → `1 one`
- T-DST-11 `mutable o = { a = 1, b = 2 } · use { a, b } = o · a = 10 · console.log(o)` → `{ a = 10, b = 2 }`
- T-DST-12 `let o = { a = 1 } · mutable { a } = o · a += 1 · console.log(a, o.a)` → `2 1`
- T-DST-13 `let m = [[1, 2], [3, 4]] · let [[a, b], [c, d]] = m · console.log(a + d)` → `5`
- T-DST-20 `let o = { a = 1 } · mutable use { a } = o` ⇒ E-DST-1
- T-DST-21 `let n = 5 · let { a } = n` ⇒ E-DST-2; `let t = (1, 2) · let [a, b] = t` ⇒ E-DST-2
- T-DST-22 `let o = { a = 1 } · let { b } = o` ⇒ E-DST-3
- T-DST-23 `let o = { a = 1 } · use { b = 2 } = o` ⇒ E-DST-4
- T-DST-24 `let (a, b, c) = (1, 2)` ⇒ E-DST-5
- T-DST-25 `let [a = 0] = [1]` ⇒ E-DST-6; `let (a, ...r) = (1, 2)` ⇒ E-DST-6
- T-DST-26 `let o = { a = 1, b = 2 } · let { a, b as a } = o` ⇒ E-DST-7
- T-DST-27 `let o = { a = 1 } · let { a: first } = o` ⇒ E-DST-8
- T-DST-28 `let xs = [1, 2] · let [...r, last] = xs` ⇒ E-DST-9
- T-DST-29 `function f = (values: Array(Integer)) => { · let [a, b] = values · }` ⇒ E-ERR-3 ("prefix the declaration with `try`")
- T-DST-30 `type U = { name: String } · let u: U? = none · let { name } = u` ⇒ E-OPT-1

---

## 17. Conditionals

Source: `control-flow/conditionals` (canonical for `if`, narrowing, `when`), `types/unions`, `values/errors` (`when` on errors).

### 17.1 `if`

**Syntax (as parsed).** `if <cond> { … } [else if <cond> { … }]* [else { … }]` (`Node::If`, `else_body: If | Block`). No parentheses needed; braces always required; `else` on the line of the `}` (E-LEX-5). `@if` is compile-time (out of scope).

**Static semantics.**

- The condition is a `Boolean` (E-BOOL-1). Facts and narrowings per section 14; constant conditions W-BOOL-1.
- **Statement** (its value unused): no `else` needed; branch values are dropped.
- **Expression** (its value is used: bound, passed, returned, an operand, or the last expression of a block whose value is used): `else` required (`E-IF-1`); each branch's value is the last expression of its block; a branch block ending with a statement has no value (`E-IF-3`).
- Type [D-29]: with an expected type, each branch is checked against it (literals adapt: `let ratio: Float = if c { 1 } else { 0.5 }` is valid). Without one: the union of the branch types, each widened to its base type; a literal branch first adapts to the type of the non-literal branches when it fits (`if c { a } else { 1 }` with `a: Int8` is an `Int8`); `Never` branches add nothing. The union must be a valid union (section 13 and `types/unions`): `if c { 1 } else { 0.5 }` and `if c { count } else { ratio }` (`Integer | Float`, two number kinds) are `E-IF-2` ("an `Integer` and a `Float` with no expected type: annotate, or write `1.0`"); `Int8 | Int16` flattens to `Int16`; `Int8 | UInt8` (overlapping, neither contains the other) is E-IF-2.
- Facts after the `if`: join of the branches that reach the end (interval hull; literal facts kept only when equal [D-30]).

**Runtime.** The condition, then exactly one branch.

**Lowering.** Native: `br` on the condition, `phi` for the value. JavaScript: statement `if`; an expression `if` is a ternary when both branches are single pure expressions, otherwise hoisted (`let $t; if (c) { …; $t = v1 } else { …; $t = v2 }`, section 19).

### 17.2 `when`

**Syntax (as parsed).** `when <subject> is { <pattern> [as <binding>] => <expr | block> … [else => <expr | block>] }` (`Node::When`, `WhenBranch { pattern: Expression | Else, binding, value }`); branches separated by newlines. Patterns use `|` for alternatives; `as` binds a name or a destructuring pattern (object `{ … }`, tuple `( … )`).

**Static semantics.**

- The subject is evaluated once; any type. `when` always has a subject (`E-WHEN-7` for `when { … }`).
- Patterns: literal (integer, negative integer, float [D-56], string/character, `true`/`false`, `none`), type, qualified member of a named union / enumeration / error type, whole named union or error type, range (integer subjects only, E-RNG-4; bounds any integer expressions), alternatives `|`. A value name: E-CMP-5; an unqualified member: E-CMP-9; a tuple or nested pattern (`(1, 2) =>`): `E-WHEN-5`; a guard (`Integer as n if n > 0 =>`): `E-WHEN-4` ("no guards: test with an `if` inside the branch").
- A pattern that can never match the subject's type (`300 =>` on an `Int8`, `"a" =>` on an `Integer`, `String =>` on `Integer | Boolean`): `E-WHEN-3` [D-77]. A type pattern on a non-union subject that always matches: W-CMP-1 [D-57].
- `else =>`: at most one, last (`E-WHEN-8`); `else as x`: `E-WHEN-6` (bind the subject before the `when`).
- `as`: binds the matched value with the narrowed type of the pattern (union of the alternatives); a destructuring `as { a, b }` only accepts fields common to every alternative (E-DST-3); the name exists only in its branch and may shadow.
- The subject is narrowed in each branch only when it is a plain local name (local value or parameter); in `else`, to the remaining members.
- Branch bodies: after `=>`, `{` opens a block.
- Type: as `if` (expected type, or union of widened branch types, `Never` branches ignored, E-IF-2 rules). Statement form drops values.
- **Exhaustiveness** (expression and statement): without `else`, the patterns must cover every value of the subject (`E-WHEN-1`, naming the missing members/values), using the subject's type **and its facts** (narrowed members, interval, excluded values) [D-55]:

| Subject | Covered by |
| --- | --- |
| named union, error type | every member, or the whole type |
| enumeration | every member |
| anonymous union, optional | every member type (`none` included); a member is covered by its type pattern, or (integers, `Boolean`, literal unions) by literal/range patterns covering it |
| `Boolean` | `true` and `false` |
| integer type | literals and ranges **with literal bounds** covering the subject's interval (minus excluded values) |
| union of literals | every literal |
| `Float`, `String`, `Character`, objects, collections… | `else` required |

- A branch whose patterns are all matched by earlier branches (including a duplicate literal): `W-WHEN-1`; an `else` when everything is covered: `W-WHEN-2`.

**Runtime.** Evaluate the subject once; test the branches in order; run the first match.

**Lowering.** Native: named unions and enumerations → `switch` on the tag with an `unreachable` default when exhaustive; integer literals → `switch`; ranges → compare chains; strings → length switch + `memcmp` (or a hash when many); anonymous unions → tag tests. JavaScript: `switch` on literals or on the tag of named unions (`switch ($v.$tag)`), `if` chains for types (`typeof`, field tests), ranges and alternatives; an expression `when` is hoisted (section 19) with the subject in a temp; an exhaustive `when` needs no default (optionally a debug `default: $unreachable()`).

### Diagnostics

| Id | Condition |
| --- | --- |
| E-IF-1 | `if` used as a value without `else` |
| E-IF-2 | branch types form an invalid union without expected type (number kinds mixed, overlapping integer types, indistinguishable members) |
| E-IF-3 | a branch without a value where a value is needed (block ending with a statement, `Void`) |
| E-WHEN-1 | non-exhaustive `when` (lists what is missing) |
| E-WHEN-3 | pattern that can never match the subject |
| E-WHEN-4 | guard after a pattern |
| E-WHEN-5 | tuple or nested pattern |
| E-WHEN-6 | `as` on `else` |
| E-WHEN-7 | `when` without subject |
| E-WHEN-8 | `else` not last, or several |
| W-WHEN-1 | unreachable branch |
| W-WHEN-2 | useless `else` |

### Tests

- T-IF-1 `let t = 35 · if t > 30 { console.log("hot") } else if t > 15 { console.log("mild") } else { console.log("cold") }` → `hot`
- T-IF-2 `let score = 40 · let label = if score >= 50 { "pass" } else { "fail" } · console.log(label)` → `fail`
- T-IF-3 `function v = (c: Boolean) => if c { 1 } else { "one" } · console.log(v(true), v(false))` → `1 one` (`Integer | String`)
- T-IF-4 `let c = true · let ratio: Float = if c { 1 } else { 0.5 } · console.log(ratio / 2.0)` → `0.5`
- T-IF-5 `function size = (text: String?): Integer => { · let s = if text is not none { text.length } else { return 0 } · s · } · console.log(size("abc"), size(none))` → `3 0`
- T-IF-6 `let value = 12 · let maximum = 10 · let clamped = if value > maximum { · console.log("clamping") · maximum · } else { · value · } · console.log(clamped)` → `clamping / 10`
- T-IF-7 `function f = (a: Int8, c: Boolean): Int8 => if c { a } else { 1 } · console.log(f(5, false))` → `1` (literal adapts to `Int8`)
- T-IF-8 `let c = true · let x = if c { 1 } else { 0.5 }` ⇒ E-IF-2
- T-IF-9 `let c = true · let x = if c { 1 }` ⇒ E-IF-1
- T-IF-10 `let c = true · mutable n = 0 · let x = if c { n = 1 } else { 2 }` ⇒ E-IF-3
- T-IF-11 `function i8 = (): Int8 => 1 · function u8 = (): UInt8 => 1 · let c = true · let x = if c { i8() } else { u8() }` ⇒ E-IF-2
- T-WHEN-1 `let foo = 3 · let result = when foo is { · 1 => "one" · 2 => "two" · 3 => "three" · else => "other" · } · console.log(result)` → `three`
- T-WHEN-2 `let foo: Integer | String = 3 · console.log(when foo is { · Integer => "integer" · String => "string" · })` → `integer`
- T-WHEN-3 `type MyUnion = Union { SomeString = String, SomeNumber = Integer, SomeObject = { baz: String, qux: Integer } } · let foo = MyUnion.SomeObject { baz = "hello", qux = 42 } · console.log(when foo is { · MyUnion.SomeNumber as number => "number: {number}" · MyUnion.SomeString as string => "string: {string}" · MyUnion.SomeObject as { baz, qux } => "object: {{ baz: {baz}, qux: {qux} }}" · })` → `object: { baz: hello, qux: 42 }`
- T-WHEN-4 `type Position = { x: Float, y: Float } · type Shape = Union { Point, Circle = { radius: Float }, Rectangle = { width: Float, height: Float }, Segment = Tuple(Position, Position) } · function area = (shape: Shape): Float => when shape is { · Shape.Point => 0.0 · Shape.Circle as circle => circle.radius * circle.radius * 3.0 · Shape.Rectangle as { width, height } => width * height · Shape.Segment as (start, end) => 0.0 · } · console.log(area(Shape.Circle { radius = 2.0 }), area(Shape.Rectangle { width = 2.0, height = 3.0 }))` → `12 6`
- T-WHEN-5 `function label = (score: Integer): String => when score is { · 0..60 => "fail" · 60..80 => "pass" · 80..=100 => "excellent" · else => "out of range" · } · console.log(label(59), label(60), label(100), label(101), label(-1))` → `fail pass excellent out of range out of range`
- T-WHEN-6 `function answer = (key: String): String => when key is { · "y" | "yes" => "accepted" · "n" | "no" => "refused" · else => "unknown" · } · console.log(answer("yes"), answer("n"), answer("?"))` → `accepted refused unknown`
- T-WHEN-7 `let b = false · console.log(when b is { · true => 1 · false => 0 · })` → `0` (exhaustive without `else`)
- T-WHEN-8 `function sign = (n: Integer): String => when n is { · ..0 => "negative" · 0 => "zero" · 1.. => "positive" · } · console.log(sign(-3), sign(0), sign(9))` → `negative zero positive` (exhaustive over `Integer`)
- T-WHEN-9 `function pct = (p: Integer(min = 0, max = 100)): String => when p is { · 0..=49 => "low" · 50..=100 => "high" · } · console.log(pct(50))` → `high`
- T-WHEN-10 `function f = (x: Integer) => { · if x < 0 { return } · console.log(when x is { 0..10 => "small" · 10.. => "big" }) · } · f(3)` → `small` (exhaustive thanks to the fact `x ≥ 0`, [D-55])
- T-WHEN-11 `function d = (v: Integer | String): String => when v is { · Integer as number => if number < 0 { "negative" } else { "positive or zero" } · String => "text" · } · console.log(d(-1), d("a"))` → `negative text`
- T-WHEN-12 `type E = Errors { Timeout, NotFound = { id: String } } · function m = (error: E): String => when error is { · E.Timeout => "timeout" · E.NotFound => "missing {error.id}" · } · console.log(m(E.NotFound { id = "7" }))` → `missing 7` (plain local subject narrowed)
- T-WHEN-13 `type Shape = Union { Circle = { radius: Float }, Square = { side: Float } } · function mk = (): Shape => Shape.Circle { radius = 1.0 } · console.log(when mk() is { · Shape.Circle as c => "circle {c.radius}" · else => "other" · })` → `circle 1`
- T-WHEN-14 `function n = (): Integer => { · console.log("once") · 2 · } · console.log(when n() is { 1 => "a" · 2 => "b" · else => "c" })` → `once / b`
- T-WHEN-15 `let x: Integer? = none · when x is { · none => console.log("nobody") · Integer => console.log(x + 1) · }` → `nobody`
- T-WHEN-16 `function f = (x: Float): String => when x is { · 0.5 => "half" · else => "other" · } · console.log(f(0.5))` → `half` [D-56]
- T-WHEN-20 `let foo: Integer | String = 3 · let r = when foo is { · Integer => "integer" · }` ⇒ E-WHEN-1 (`String` not handled)
- T-WHEN-21 `let s = "a" · when s is { "a" => 1 }` ⇒ E-WHEN-1 (`String` needs `else`)
- T-WHEN-22 `function f = (p: Int8) => when p is { 300 => 1 · else => 2 }` ⇒ E-WHEN-3
- T-WHEN-23 `function f = (v: Integer) => when v is { Integer as n if n > 0 => 1 · else => 2 }` ⇒ E-WHEN-4
- T-WHEN-24 `function f = (t: Tuple(Integer, Integer)) => when t is { (1, 2) => 1 · else => 2 }` ⇒ E-WHEN-5
- T-WHEN-25 `let s = 3 · when { s > 2 => 1 }` ⇒ E-WHEN-7
- T-WHEN-26 `let s = 3 · when s is { else => 1 · 3 => 2 }` ⇒ E-WHEN-8
- T-WHEN-27 `let s = 3 · when s is { 1 => 1 · 1 => 2 · else => 3 }` ⇒ W-WHEN-1
- T-WHEN-28 `let b = true · when b is { true => 1 · false => 2 · else => 3 }` ⇒ W-WHEN-2
- T-WHEN-29 `type E = Errors { NotFound } · function f = (e: E) => when e is { NotFound => 1 }` ⇒ E-CMP-9
- T-WHEN-30 `let passing = 60..=100 · let s = 70 · when s is { passing => 1 · else => 2 }` ⇒ E-CMP-5

---

## 18. Loops

Source: `control-flow/loops`, `values/ranges#iterating-over-a-range`, `advanced/reference-checking#iteration`.

### Syntax (as parsed)

- `for [<pattern> [, <index>] in] <iterable> { … }` (`Node::For { pattern, index, iterable, parallel, body, is_compile_time }`); `for parallel[(threads = n)] …`; `@for` (compile-time).
- `while <cond> { … }` (`While`), `while let <name> = <expr> { … }` (`WhileLet`).
- `loop { … }` (`Loop`), `break [<expr>]`, `continue`.

### Static semantics

**`for`:**

- Iterable, evaluated once: a `Range` with a start (E-RNG-3), a collection (`Array`, `Set`, `Map`, `Pool`, `Columnar`, `OrderedColumnar`, growing versions), a `String`, an iterator (`values()` chains). Anything else: `E-LOOP-1` ("cannot iterate over an `Integer`: use a range `0..count`"); an optional collection: E-OPT-1.
- Pattern: a name, `name, index`, or a destructuring pattern (object, tuple); omitted when unused (`for 0..10 { }`, `for items { }`). An array pattern in a `for`: `E-LOOP-2` (an element-wise `IndexError` would be hidden; use a tuple) [D-32].
- `index` only over `Array`, `GrowingArray`, `String` (and `OrderedColumnar`, which has the positional interface): `E-LOOP-3` otherwise [D-73]. It is an immutable `Integer` with the facts `0 ≤ index < collection.length`.
- Loop variables: over a range or a string, immutable copies (`Integer` with the range's interval; `Character`); assigning them is E-DECL-3 [D-72]. Over a collection, **references** to the elements: mutable when the collection is a mutable place (a `mutable` value, a `mutates` parameter, a `mutable use`), read-only otherwise; `Set`/`GrowingSet` elements and `Map` keys always read-only; destructured names are references to the fields. Writing through them mutates the collection (a parameter needs the corresponding `mutates` effect, e.g. `mutates players[..].score`).
- Iteration rule (`advanced/reference-checking#iteration`): the body can't mutate the iterated place `L` or a place above it, mutate `L.length`/`L.size`, or invalidate a place that may designate an element of `L`, directly, through an alias, or through the effects of a called function (implied relation effects included): `E-LOOP-4` (naming the operation: "cannot add elements to `scores` while iterating over it"; "`register` mutates `people.size` while the loop iterates over `people`"). Writing elements in place is allowed.
- `for` is a statement: using it as a value is `E-LOOP-6`; `break <value>` in a `for` or `while` is `E-LOOP-7`.
- `@for`: the iterable must be a `static` value (`E-LOOP-8`); unrolled (out of scope). `for parallel`: native only (concurrency, out of scope).

**`while`:** the condition is a `Boolean` (E-BOOL-1); it narrows the body, and its negation holds after a loop without `break`. A statement (E-LOOP-6 as a value). `while true` → W-BOOL-1 (use `loop`).

**`while let <name> = <expr>`:** `<expr>` must be optional (`E-LOOP-10`, "`while let` needs an optional: use `while`") [D-34]; only a name (`E-LOOP-9` for a pattern); the value is copied into `name` (non-optional type) for the body; evaluated before each iteration; stops at the first `none` (on `Array(Integer?)`, a stored `none` also stops it, documented).

**`loop`:** repeats until `break`, `return` or `throw`. Type: `Never` when no reachable `break` targets it; otherwise, as an expression, the union of the `break` values' types (widened, literal adaptation, E-IF-2 rules). In a `loop` whose value is used, every `break` carries a value (`E-LOOP-11`). In a `loop` used as a statement, `break <value>` drops the value: `W-LOOP-1` [D-62].

**`break`/`continue`:** act on the innermost loop of the current function body, through `do` blocks, `if`/`when` branches and `catch` handlers. Outside any loop: `E-LOOP-12`. Inside a lambda or a local function nested in the loop (crossing a function boundary): `E-LOOP-13`. Labels (`break outer`): `E-LOOP-14` (no labeled loops: use a function and `return`). Both have type `Never`.

### Runtime semantics

- Range: counts up by one from `start` while `< end` (`<= end` for `..=`); unbounded end runs until `break` (stop at overflow, [D-40]).
- Collections: in order for positional collections; iteration order of sets/maps/pools per the collection pages. Elements are visited by reference; the structure doesn't change during the loop (guaranteed statically).
- `while let`: evaluate, test `none`, bind, run body; repeat.
- `loop` expression: the value of the `break` that leaves it.

### Lowering

- Native: counted loops for ranges (`nsw` increment, inclusive end tested before the increment); a pointer walk for arrays (loop variable = pointer to the element; the length is loaded once since the structure can't change); strings: one segmentation pass (not one per index); `loop` = an unconditional back edge, a `phi` at the exit block for its value.
- JavaScript: ranges → `for (let i = a; i < b; i++)`; arrays → `for (let $i = 0; $i < $a.length; $i++)` with the element read as `$a[$i]` (object elements: `const player = $a[$i]`; a primitive element written through the variable is rewritten to `$a[$i]`, `score += 1` → `$a[$i] += 1`); maps → `for (const [k, v] of map)` with writes to `v` rewritten to `map.set(k, …)`; strings → an ASCII fast path (`for` over code units) or an `Intl.Segmenter` iteration; `while let v = e` → `for (let v; (v = e) !== undefined; ) { … }`; `loop` → `while (true)`; a `loop` expression → `let $t; while (true) { … $t = v; break; }`; `break`/`continue` → the JS keywords (a `break` inside a JS `switch` generated for a `when` needs a label on the loop: the backend labels loops whenever it emits a `switch` inside them).

### Diagnostics

| Id | Condition |
| --- | --- |
| E-LOOP-1 | iterable is not a range, collection, string or iterator |
| E-LOOP-2 | array pattern in a `for` |
| E-LOOP-3 | index binding on a collection without positions |
| E-LOOP-4 | structural modification of the iterated collection in the body |
| E-LOOP-6 | `for`/`while` used as a value |
| E-LOOP-7 | `break <value>` in `for`/`while` |
| E-LOOP-8 | `@for` over a non-static iterable |
| E-LOOP-9 | `while let` with a destructuring pattern |
| E-LOOP-10 | `while let` on a non-optional expression |
| E-LOOP-11 | `break` without value in a `loop` whose value is used |
| E-LOOP-12 | `break`/`continue` outside a loop |
| E-LOOP-13 | `break`/`continue` crossing a function boundary |
| E-LOOP-14 | labeled `break`/`continue` |
| W-LOOP-1 | `break <value>` in a statement `loop` |

### Tests

- T-LOOP-1 `for count in 0..3 { console.log("Count: {count}") }` → `Count: 0 / Count: 1 / Count: 2`
- T-LOOP-2 `let fruits = ["apple", "banana"] · for fruit, index in fruits { console.log("{index}: {fruit}") }` → `0: apple / 1: banana`
- T-LOOP-3 `let fruits = ["a", "b"] · for fruit, index in fruits { console.log(fruits[index]) }` → `a / b` (no `try`: index fact)
- T-LOOP-4 `mutable players = [{ name = "John", score = 10 }, { name = "Jane", score = 12 }] · for player in players { player.score += 1 } · console.log(players[0].score, players[1].score)` → `11 13`
- T-LOOP-5 `mutable scores = [1, 2, 3] · for score in scores { score += 1 } · console.log(scores)` → `[2, 3, 4]`
- T-LOOP-6 `let usersById: Map(Integer, String) = [(1, "Ana"), (2, "Bao")] · for (id, name) in usersById { console.log("{id} -> {name}") }` → `1 -> Ana / 2 -> Bao`
- T-LOOP-7 `let people = [{ name = "Ana", age = 3 }] · for { name, age } in people { console.log("{name} is {age}") }` → `Ana is 3`
- T-LOOP-8 `let users = [{ name = "Ana" }] · for { name as userName } in users { console.log(userName) }` → `Ana`
- T-LOOP-9 `mutable remaining = 3 · while remaining > 0 { · console.log(remaining) · remaining -= 1 · }` → `3 / 2 / 1`
- T-LOOP-10 `mutable stack = [1, 2, 3] · while let value = stack.pop() { console.log("Popped: {value}") }` → `Popped: 3 / Popped: 2 / Popped: 1`
- T-LOOP-11 `mutable stack: Array(Integer?) = [1, none, 3] · mutable n = 0 · while let v = stack.pop() { n += 1 } · console.log(n)` → `1` (stops at the stored `none`)
- T-LOOP-12 `let target = 50 · mutable candidate = 1 · let root = loop { · if candidate * candidate >= target { break candidate } · candidate += 1 · } · console.log(root)` → `8`
- T-LOOP-13 `let numbers = [5, -1, 200, 7] · for number in numbers { · if number < 0 { continue } · if number > 100 { break } · console.log(number) · }` → `5`
- T-LOOP-14 `function findFirstEven = (numbers: Array(Integer)): Integer? => { · for number in numbers { if number %% 2 == 0 { return number } } · none · } · console.log(findFirstEven([1, 3, 4, 6]), findFirstEven([1]))` → `4 none`
- T-LOOP-15 `function findEmpty = (rows: Array(Array(Integer?))): Tuple(Integer, Integer)? => { · for row, r in rows { · for cell, c in row { if cell is none { return (r, c) } } · } · none · } · console.log(findEmpty([[1, 2], [3, none]]))` → `(1, 1)`
- T-LOOP-16 `mutable i = 0 · loop { · i += 1 · if i == 3 { break } · } · console.log(i)` → `3`
- T-LOOP-17 `for c in "hé" { console.log(c) }` → `h / é`
- T-LOOP-18 `let evens = [1, 2, 3, 4].filter((n) => n %% 2 == 0) · console.log(evens)` → `[2, 4]`
- T-LOOP-19 `let numbers = [1, 2, 3] · let d = numbers.filter((n) => { · console.log("filter {n}") · n > 1 · }).map((n) => { · console.log("map {n}") · n * 2 · }) · console.log(d)` → `filter 1 / filter 2 / filter 3 / map 2 / map 3 / [4, 6]` (not fused: effects)
- T-LOOP-20 `let numbers = [1, 2, 3] · let d = numbers.values().filter((n) => { · console.log("filter {n}") · n > 1 · }).map((n) => { · console.log("map {n}") · n * 2 · }).toArray() · console.log(d)` → `filter 1 / filter 2 / map 2 / filter 3 / map 3 / [4, 6]`
- T-LOOP-21 `let x = 5 · for x in 0..2 { console.log(x) } · console.log(x)` → `0 / 1 / 5` (shadowing)
- T-LOOP-30 `let n = 5 · for i in n { }` ⇒ E-LOOP-1
- T-LOOP-31 `let pairs = [[1, 2]] · for [a, b] in pairs { }` ⇒ E-LOOP-2
- T-LOOP-32 `let s: Set(Integer) = [1] · for v, i in s { }` ⇒ E-LOOP-3
- T-LOOP-33 `mutable scores = [1, 2] · for score in scores { scores.push(0) }` ⇒ E-LOOP-4
- T-LOOP-34 `mutable players = [{ score = 0 }] · for player, index in players { if player.score == 0 { players.delete(index) } }` ⇒ E-LOOP-4
- T-LOOP-35 `let x = for i in 0..3 { }` ⇒ E-LOOP-6; `let y = while false { }` ⇒ E-LOOP-6
- T-LOOP-36 `for i in 0..3 { break i }` ⇒ E-LOOP-7
- T-LOOP-37 `let xs = [1, 2] · while let { a } = xs.pop() { }` ⇒ E-LOOP-9
- T-LOOP-38 `let n = 3 · while let v = n { }` ⇒ E-LOOP-10
- T-LOOP-39 `mutable i = 0 · let r = loop { · i += 1 · if i > 3 { break } · }` ⇒ E-LOOP-11
- T-LOOP-40 `break` at top level ⇒ E-LOOP-12; `function f = () => { continue }` ⇒ E-LOOP-12
- T-LOOP-41 `for i in 0..3 { · [1].map((x) => { · break · x · }) · }` ⇒ E-LOOP-13
- T-LOOP-42 `for i in 0..3 { break outer }` ⇒ E-LOOP-14
- T-LOOP-43 `let scores = [1, 2] · for score in scores { score += 1 }` ⇒ E-DECL-3 (immutable collection: read-only references)
- T-LOOP-44 `let s: Set(Integer) = [1] · mutable t = s · for v in t { v += 1 }` ⇒ E-DECL-3 (set elements read-only)
- T-LOOP-45 `mutable i = 0 · loop { · i += 1 · if i > 2 { break i } · }` ⇒ W-LOOP-1
- T-LOOP-46 `function server = (): Never => loop { console.log("x") }` valid (type `Never`); `function bad = (): Never => loop { break }` ⇒ E-TYPE-1

---

## 19. `do` blocks and block values

Sources: `control-flow/do`, `getting-started/syntax-basics#blocks-and-expressions`, `functions/defining-functions#return-type-and-effects`.

### Syntax (as parsed)

`do { … }` (`Node::Do`); `start do { … }` (concurrency, out of scope); `provide T = v` inside a block (requirements, out of scope).

### Static semantics

- **Block value** (function bodies, `if`/`when` branches, `catch` handler blocks, `do`, `loop` via `break`): the value of a block is its last expression; a block ending with a statement (declaration, assignment, `for`, `while`, statement `if`/`when`, an expression of type `Void`) has no value.
- `do` as an expression: the value and type of its last expression; a `do` without value (empty, or ending with a statement) where a value is needed: `E-DO-1` ("this block ends with a statement and has no value").
- `do` as a statement: a scope; values declared inside are destroyed at its end (reverse order).
- `return`, `break`, `continue` inside a `do` act on the enclosing function/loop. `return` outside a function (including at the top level of a standalone file and inside a `test` body? — a test body is a function: allowed) is `E-DO-2` [D-60].
- Statements whose value is dropped: a pure expression statement (no effect: `x + 1`, `a == b`, a literal) is `W-DO-1` [D-37]; dropping the result of a call is silent (as for a block function without return type). Code after a `Never` expression in the same block: `W-DO-2` (unreachable).
- A `function` with a block body and no return type returns nothing: its last value is dropped (W-DO-1 when pure, per `defining-functions`).

### Runtime semantics

Statements run in order; values declared in the block are destroyed at its end, also when leaving through `return`/`break`/`continue`/`throw`/`try`.

### Lowering

- Native: basic blocks; a cleanup stack emits the drops at every exit; the value is the last SSA value (a `phi` when control merges).
- JavaScript: a statement `do` → a JS block `{ … }` (`let`/`const` scoping matches). **Expression-position control flow** (`do`, `if`, `when`, `loop`, `catch` handlers, and any expression containing `return`/`break`/`continue`/`throw`) is lowered by statement lifting: the construct becomes statements writing a temp declared just before the enclosing statement; to keep the left-to-right evaluation order, the operands evaluated **before** the lifted construct in the enclosing expression are first stored in temps when they have effects or read places the construct may write (`f() + do { … }` → `const $a = f(); let $b; { …; $b = v } … $a + $b`). An IIFE is never used (it would break `return`/`break` and cost a closure).

### Diagnostics

| Id | Condition |
| --- | --- |
| E-DO-1 | block without value used as a value |
| E-DO-2 | `return` outside a function |
| W-DO-1 | pure expression statement / dropped pure value |
| W-DO-2 | unreachable code |

### Tests

- T-DO-1 `let helloWorld = do { · mutable hello = "Hello" · hello += " World" · hello += "!" · hello · } · console.log(helloWorld)` → `Hello World!`
- T-DO-2 `let array = [1, 2, 3, 4, 5] · let sum = do { · mutable total = 0 · for value in array { total += value } · total · } · console.log(sum)` → `15`
- T-DO-3 `let value = do { · let a = 2 · let b = 3 · a * b · } · console.log(value)` → `6`
- T-DO-4 `function f = (n: Array(Integer)): Integer? => { · let result = do { · for x in n { if x %% 2 == 0 { return x } } · none · } · result · } · console.log(f([1, 4]), f([1]))` → `4 none`
- T-DO-5 `function f = (): Integer => { · console.log("a") · 1 · } · let x = f() + do { · console.log("b") · 2 · } · console.log(x)` → `a / b / 3` (evaluation order kept by the JS lowering)
- T-DO-6 `for i in 0..3 { · do { if i == 1 { continue } } · console.log(i) · }` → `0 / 2`
- T-DO-7 `let x = do { · let a = 1 · }` ⇒ E-DO-1; `let y = do { }` ⇒ E-DO-1
- T-DO-8 `return 1` at the top level of a standalone file ⇒ E-DO-2
- T-DO-9 `let a = 1 · a + 1` ⇒ W-DO-1
- T-DO-10 `function f = (): Integer => { · return 1 · console.log("x") · }` ⇒ W-DO-2
- T-DO-11 `do { · let a = 1 · } · console.log(a)` ⇒ E-DECL-1 (scoped)
