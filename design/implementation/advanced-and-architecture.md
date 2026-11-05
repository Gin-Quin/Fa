# Implementation brief: advanced features and architecture

> **Note (2026-11-02)**: there is no `rust` target anymore (see `design/decisions.md`, foreign code): Rust code is used through the C ABI. The mentions of `rust` in the target tables and the tiering below are obsolete.

Working brief for the engineers of `fa-analyzer` (the type and effect checker), `fa-codebase-parser` (module graph), `fa` (CLI), `fa-codegen-js` and `fa-codegen-llvm`. It covers:

- `documentation/advanced/*`: memory management, owned references, reference checking, relations, requirements, structural typing, reactivity, metaprogramming, foreign code, concurrency (**without** `async` / `await` / `start` / tasks, which are out of scope for this iteration);
- `documentation/architecture/*`: overview and CLI, modules, packages, workspaces, configuration, dependencies, testing, hot reloading;
- `documentation/guides/*`: what they require from the toolchain;
- `design/compilation-with-llvm.md`, `design/spec-review.md` and `design/compiler-architecture.md` (the last one fixes the crate names and the "one analysis walk" rule, and wins where the older LLVM document says otherwise; see [D1](#d1-one-walk-vs-memoized-queries)).

The documentation is the source of truth. Where this brief adds a rule the documentation does not state, the rule is marked **(decision Dn)** and listed in [Ambiguities and decisions](#ambiguities-and-decisions) for the project lead to accept or override.

Contents:

1. [Conventions](#1-conventions)
2. [Where this fits: the single analysis walk](#2-where-this-fits-the-single-analysis-walk)
3. [Places, paths and effects](#3-places-paths-and-effects)
4. [Reference types, `use`, `mutable use`, `reuse`](#4-reference-types-use-mutable-use-reuse)
5. [The reference checker](#5-the-reference-checker)
6. [Memory management: copies, moves, destruction, closures](#6-memory-management-copies-moves-destruction-closures)
7. [Collection effects (owned references)](#7-collection-effects-owned-references)
8. [Relations](#8-relations)
9. [Requirements](#9-requirements)
10. [Structural typing](#10-structural-typing)
11. [Reactivity](#11-reactivity)
12. [Metaprogramming and the interpreter](#12-metaprogramming-and-the-interpreter)
13. [Foreign code](#13-foreign-code)
14. [Concurrency without async](#14-concurrency-without-async)
15. [Modules](#15-modules)
16. [Packages, workspaces, configuration](#16-packages-workspaces-configuration)
17. [Dependencies](#17-dependencies)
18. [Testing](#18-testing)
19. [Hot reloading](#19-hot-reloading)
20. [The `fa` command line](#20-the-fa-command-line)
21. [Compilation units, caching, incrementality](#21-compilation-units-caching-incrementality)
22. [Standard library surface](#22-standard-library-surface)
23. [The guides as acceptance tests](#23-the-guides-as-acceptance-tests)
24. [Ambiguities and decisions](#ambiguities-and-decisions)
25. [Tiering](#tiering)

---

## 1. Conventions

### Diagnostics

Every diagnostic has a stable code, a primary span, a message written to be acted upon, and related spans. Codes used in this brief:

| Prefix | Area |
| --- | --- |
| `EFF` | effects (`mutates`, `invalidates`) and mutability |
| `REF` | reference checking |
| `REL` | relations |
| `REQ` | requirements |
| `STR` | structural typing |
| `REA` | reactivity |
| `META` | compile-time evaluation |
| `FFI` | foreign code |
| `CONC` | concurrency |
| `MOD` | modules and names |
| `CFG` | configuration files |
| `DEP` | dependencies and the lockfile |
| `TEST` | tests and assertions |
| `CLI` | command line usage |

The message texts quoted from the documentation are normative in substance (what they name, what fix they propose), not in wording. Every reference-checking error names: the reference, the operation that invalidated it with its line, and the fix (`declare it again with use x = ...`, `reuse x = ...` for a `mutable use`, a copy with `let` for an overlapping argument).

### Test notation

```fa
-- prints: Ana            the program prints this line (one `prints` per line, in order)
-- error REF001           a compile error with this code, on this line
-- warning MOD013         a warning
-- exit: 101              the exit code of the program
```

A test without an `error` line must compile and print exactly the listed lines. Unless a test says otherwise, it is a standalone file run with `fa run`, and it uses these declarations when it names them:

```fa
type Pet = { name: String }
type Person = { name: String, age: Integer = 0, pets: Array(Pet) = [] }
type Player = { name: String, score: Integer }
```

---

## 2. Where this fits: the single analysis walk

`design/compiler-architecture.md` fixes three passes over a file: tokenize, parse, analyze. **Analyze is one walk** that resolves names, infers types, infers and checks effects, tracks narrowing, ranges and reference validity, and emits every diagnostic. Everything in this brief that is checked statically is done inside that walk. This section says how.

### 2.1 What the walk carries

| Component | Kind | Owner |
| --- | --- | --- |
| `TypeStore`, `SymbolTable`, side tables by node | global, append-only | analyzer core |
| **Function summary** (interface) per function | memoized on demand | this brief, [2.3](#23-function-summaries) |
| **Flow state** | forked and joined along control flow | narrowing, ranges (other briefs) + reference validity ([5](#5-the-reference-checker)) |
| **Scope stack** | lexical | names, `provide` frames ([9](#9-requirements)), iteration stack ([5.8](#58-iteration)), parallel-loop frame ([14.2](#142-for-parallel)) |
| **Effect accumulator** of the current function | per function | [3.6](#36-what-a-body-may-do) |
| **Pending constraints** | per function | escaping closures ([5.11](#511-closures)), move candidates ([6.2](#62-copies-and-moves)) |

The reference checker is not a separate pass: it is the part of the **flow state** that tracks local references, and it is updated by the same transfer functions that update narrowing and integer ranges. A call node is visited once for types; at that moment the arguments' places and the callee's summary are known, so the effects are applied to the flow state right there.

### 2.2 The order of the walk

Statements are walked in evaluation order. Each expression node is visited by `check(node, expected) -> TypeId`, and the flow transfer of the node happens after its operands are checked (operands are evaluated first). Two constructs revisit nodes:

1. **Loops** re-run the **flow transfer only** of their body until the flow state at the loop head stops changing ([5.6](#56-control-flow)). Types and resolutions are memoized per node on the first visit and never recomputed. Flow diagnostics (invalidated references, narrowing errors) are collected per iteration and only those of the **last, stable** iteration are emitted.
2. **Compile-time parameters**: an `@for` or `@if` that depends on a compile-time parameter, and the body of a function with compile-time value parameters, are analyzed once per instantiation ([12](#12-metaprogramming-and-the-interpreter)).

Every other piece of information this brief needs is either local to the walk, or a summary of another function obtained on demand.

### 2.3 Function summaries

A function's summary is everything a caller needs, and is computed when the function is first needed (hoisting by need, as `compiler-architecture.md` describes). It is the **interface** of the function, hashed for caching ([21](#21-compilation-units-caching-incrementality)):

| Field | Declared or inferred | Used by |
| --- | --- | --- |
| parameter and return types, generic parameters | declared (return type inferred for a single expression) | everything |
| `throws` | declared / inferred | errors brief |
| `requires` keys with their names | declared / inferred | [9](#9-requirements) |
| `mutates` / `invalidates` paths (normalized) | declared / inferred | [3](#3-places-paths-and-effects), [5](#5-the-reference-checker) |
| implied effects of its declared effects | computed from the relations | [8.6](#86-implied-effects) |
| **returns fresh** (bit) | inferred | [5.4](#54-identities-and-may-designate) |
| **stored** bit per function-typed parameter | inferred | closures, effects of callbacks |
| **module-level reads** (set of module-level values, transitive) | inferred | alias rule, purity, initialization order, tasks |
| **module-level writes** (part of `mutates`) | declared / inferred | |
| **pure** (bit) | inferred | `@` calls, `derived`, fusion |
| **reads reactive** (set of reactive module-level places, transitive) and **may write reactive** (bit) | inferred | [11](#11-reactivity) |
| **needs full argument type** per structural parameter (`Type(parameter)` copy, return or reflection) | inferred | [10.7](#107-lowering) |
| **depends on target** (bit: reads `target` or `configuration`) | inferred | per-target caching |
| **reaches a C call / an I/O primitive** | inferred | purity, `fa run` sandbox |

A block body declares its types and effects, so its summary is available **before its body is analyzed**: only the inferred bits (fresh, stored, module-level reads, purity, reactive sets) need the body. They are computed lazily: asking for an inferred bit of a block-body function analyzes its body on demand. A single-expression body is analyzed on demand to produce the whole summary.

### 2.4 Recursion

The documentation: "A function that calls itself, directly or through other functions, must declare its return type; its effects are still inferred when its body is a single expression."

On-demand analysis detects the cycle: when the analysis of `f` needs the summary of `g`, which is in progress, it is a strongly connected component (SCC). **(decision D2)** The analyzer handles an SCC like a loop:

1. A call to an in-progress function uses its declared return type (missing: error `EFF020` "`f` is recursive: declare its return type") and its **current provisional effects**, initially its declared ones (empty for a single expression).
2. When the root of the SCC finishes, the inferred summaries of its members are recomputed from their recorded calls (with the substitutions recorded at each call site). If any summary grew, the **flow transfer** of the members' bodies is re-run (types are memoized), exactly like a loop body, until no summary changes. Summaries only grow, and paths are bounded by the parameters' types, so this terminates; in practice it stops after one extra round.
3. Only the diagnostics of the last round are emitted.

### 2.5 Things that look like they need a second pass, and why they don't

| Need | How it fits in one walk |
| --- | --- |
| Is a local `function` (closure) escaping? Its capture mode depends on later uses. | Analyze the body when declared, record its captures as **pending constraints**; the first escaping use checks them ([5.11](#511-closures)). |
| Is a read the last use of a local (copy or move)? | Record the last use event per local; decide at the end of its scope ([6.2](#62-copies-and-moves)). |
| Order of module-level initialization | Summaries record module-level reads; the graph is sorted when the program is assembled, not in the walk ([15.7](#157-module-level-values-and-initialization)). |
| A `derived` value that reads itself through a cycle | On-demand analysis of the declarations: reading a derived declaration that is in progress is the cycle ([11.3](#113-static-semantics)). |
| Effects inferred in recursive functions | SCC rounds over the flow transfer only ([2.4](#24-recursion)). |
| `@if target` | The analysis is parameterized by the target; the untaken branch is not analyzed ([12.4](#124-if-and-for)). |

### 2.6 What the backends get

The analyzer never rewrites nodes. The backends read side tables:

- `places[node]`: for each place expression, its canonical path ([3.1](#31-paths)), used by both backends to lower `use` aliases and by the LLVM backend for attributes;
- `refs[symbol]`: kind of local reference (`use`, `mutable use`, parameter, loop variable) and, for JavaScript, whether it designates a primitive place (container plus key lowering, [6.6](#66-javascript-lowering-of-references));
- `moves[node]`: reads that are moves ([6.2](#62-copies-and-moves));
- `escapes[closure]`, `captures[closure]`: capture mode and captured symbols;
- `relationWrites[node]`: the relation operation to generate at an assignment or a collection call ([8](#8-relations));
- `requirementArgs[call]`: for each hidden requirement argument, the resolved provider ([9](#9-requirements));
- `reactiveReads[node]`, `reactiveWrites[node]`, `flushAfter[statement]`, `reactiveInstance[call]` ([11](#11-reactivity));
- `instantiations[call]`: generic arguments, and for the LLVM backend the structural views ([10.7](#107-lowering));
- `staticValues[node]`: results of compile-time evaluation ([12](#12-metaprogramming-and-the-interpreter)).

---

## 3. Places, paths and effects

### 3.1 Paths

Grammar (from reference checking):

```
path   = root { "." field | ".#" field | "[" index "]" | "[" range "]" | "[..]" }
root   = parameter | "self" | module-level value | requirement name
index  = term [ ("+" | "-") integer literal ]
term   = literal | parameter | root ".length" | root ".size"
range  = [ index ] ( ".." | "..=" ) [ index ]
```

Internally a path is interned (`PathId`), a root plus a list of steps:

```rust
enum Root {
	Parameter(SymbolId),        // includes `self`
	ModuleValue(SymbolId),      // module-level value, or `mutable` member of a namespace
	Requirement(TypeKey),       // inside the function that declares it
	Local(SymbolId),            // inside a body only: a local value of the current function
	Temporary(NodeId),          // a value that is not a place (never overlaps anything)
}

enum Step {
	Field(Name),                // `.name` and `.#name`
	Index(Identity),            // `[i]`, `[key]`
	Range { start: Option<Identity>, end: Option<Identity>, inclusive: bool },
	All,                        // `[..]`
	Length,                     // `.length` of Array, GrowingArray, String, OrderedColumnar
	Size,                       // `.size` of Set, Map, Pool, Columnar, growing collections
	Characters(Range),          // a string slice `text[1..3]` (owning edge)
	Payload(MemberId),          // the payload of a union / optional, after narrowing
	Child,                      // the heap child of a recursive value (owning edge)
}
```

Canonicalization rules:

- A parameter typed `T in p` is **not a root**: it is canonicalized to `p[Index(Identity::Symbol(param, 0))]` (or below, if `p` is a path). So `node: Node in self.#nodes` is the path `self.#nodes[@node]`, and `invalidates node` is `invalidates self.#nodes[@node]`.
- A local reference stands for the place it designates: `use person = people[0]; person.age` is the path `people[0].age`.
- An element whose index or key isn't known is `[..]`.
- **Relation crossing**: a place reached through a relation field restarts at the collection of the target. In `node.next?.data` with `next: (Node in self.#nodes)?`, the place is `self.#nodes[..].data`. The identity of the step is `Unknown`, and the read of `node.next` itself is a read of `self.#nodes[@node].next`.
- A string slice `text[a..b]` is `text.Characters(a..b)`.
- After narrowing `x is Circle` (or `x is not none`), reads of `x`'s fields go through `Payload(Circle)`.

### 3.2 Owning edges and flatness

| Step | Owns memory |
| --- | --- |
| `Index`, `Range`, `All` on a collection | yes |
| `Characters` | yes |
| `Payload`, `Child` | yes |
| `Field` | no |
| `Length`, `Size` | no |

`isFlat(type)`: a primitive, a union of literals, an enumeration, a relation reference (direct side, one-to-one reverse side, owner field), or an object whose fields are all flat. Any other union (including `Integer?`) is **not** flat; a collection and a string are not flat; inside a generic body, a value of a type parameter is never flat. Flatness is computed once per `TypeId` and cached.

### 3.3 Effects in signatures

Syntax, after the return type, `throws` and `requires`:

```fa
function f = (a: A, b: B): R throws E requires r: Q mutates a.x, b[..].y invalidates b[i..] => ...
```

Static rules (each violation is an error at the effect path):

- `EFF008` the root is not a parameter, `self`, a module-level value or a requirement name of this function; or a field does not exist in the **declared** type of the path; or an index term is not a literal, a parameter, `root.length` or `root.size` (± an integer literal).
- `EFF005` `mutates p` where `p` is a parameter of a primitive type (numbers, booleans, characters, strings, unions of literals): "a primitive parameter can't be mutated". A field of a primitive type below a parameter is fine (`mutates player.score`). **(decision D13)** The same holds for a requirement of a primitive type.
- `EFF009` **(decision D40)** `invalidates p` where `p` is a whole parameter, module-level value or requirement that is not typed `T in c`: "`invalidates` names precise places (an element or a range); a whole collection is `mutates`". `invalidates self[key]`, `invalidates self[index..]`, `invalidates element` (with `element: T in self`) are the valid forms.
- `EFF010` a path ending in `.length` or `.size` written by a function outside the implementation of the collection (the standard library's collection files) — `mutates people.size` is a valid **effect** (it describes `add`), but `people.size = 3` is not a valid **statement**.

Normalization of a set of effects (applied to declared and inferred sets): `invalidates P` implies `mutates P`; `invalidates P` with `P` typed `T in c` implies `mutates c.size`; a path covered by another path of the same kind is removed.

### 3.4 Effects of operations in a body

The walk computes, for every operation, the effects it has on places. An effect on a place whose root is `Local` or `Temporary` is an effect on a local (it updates the flow state but is not an effect of the function). An effect whose root is a parameter, a module-level value or a requirement is accumulated in the **effect accumulator** of the function.

| Operation | Effects |
| --- | --- |
| `place = value` | `mutates place` (+ implied relation effects when `place` is a relation field, [8.6](#86-implied-effects)) |
| `place += value` and every compound assignment, `place .= method(...)` | `mutates place` |
| `try place[index] = value` | `mutates place[index]` (and `place.size` for a map: `mutates self[key], self.size`) |
| a call `f(args)` | the callee's effects with parameters substituted by the argument paths, requirements by the provided places, plus implied effects ([5.7](#57-calls)) |
| a method call `x.m(args)` | same, with `self` substituted by the path of `x` |
| writing through a loop variable or a `use` alias | `mutates` of the designated path |
| a call of a non-stored callback parameter `cb(...)` inside a callee | none in the callee: the callback's effects are effects of the caller's call ([5.11](#511-closures)) |
| a non-escaping closure that writes a captured local | `mutates` of that local, applied where the closure is called |

### 3.5 Substitution at a call

`subst(effectPath, call)`:

1. Replace the root parameter by the path of the corresponding argument (`self` by the receiver's path). If the argument is a temporary, the effect is dropped (nothing else can observe it).
2. Replace index terms that name parameters by the **identity** of the argument (its exact value, its symbol plus offset, or its interval), and `root.length` by the identity `LengthOf(argumentPath)`.
3. Replace a requirement root by the path of the place that provides it at this call ([9.3](#93-resolution-at-a-call)); a temporary provider drops the effect.
4. The **declared type** of the callee's parameter, with the call's type arguments substituted, is attached to the substituted path: it decides the owning edges ([5.5](#55-what-an-effect-invalidates)).

Reference types of the callee's parameters and return type are substituted the same way: `firstPerson(persons: Array(Person)): (Person in persons)?` called as `firstPerson(town.people)` returns `(Person in town.people)?`.

### 3.6 What a body may do

- **Single-expression body** (any body that does not start with `{`): the inferred effects are the normalized union of the accumulator.
- **Block body**: every accumulated effect must be **covered** by a declared one, else:
  - `EFF001` "`reset` mutates `player`, but its signature doesn't say so: add `mutates player`" (for an assignment through a parameter the documentation's wording is "`player` is not declared as mutated. Add `mutates player` to the signature.");
  - `EFF004` "only `player.score` is declared as mutated" when a declared path exists on the same root but doesn't cover;
  - `EFF002` for an uncovered invalidation.
- A declared effect that the body doesn't have: warning `EFF011` (callers handle it anyway).
- Effects on the function's own locals are never effects of the function.

`covers(D, A)`, with `D` declared and `A` actual:

- same root, and every step of `D` matches the step of `A` at the same position, and `D` is not longer than `A`;
- a step matches when equal, when `D`'s step is `[..]` (matches any `Index`, `Range`, `All`), or when both are `Index` of the same symbol with the same offset;
- `invalidates D` covers `invalidates A` and `mutates A`; `mutates D` covers `mutates A`, and **(decision D41)** covers `invalidates A` when `A` is strictly below `D` through at least one owning edge (an operation that deletes elements of a collection may be declared `mutates self`, like `deleteWhere`);
- a declared effect covers its implied effects ([8.6](#86-implied-effects)): `mutates person.bestFriend` covers the implied `mutates people[..].bestFriendOf`.

### 3.7 Mutability errors

| Code | Condition | Message (substance) |
| --- | --- | --- |
| `EFF003` | writing through a parameter not covered by a declared `mutates` in a block body | "`player` is not declared as mutated. Add `mutates player` to the signature." |
| `EFF006` | assigning a parameter as a whole (`input = {...}`) | "a parameter can't be replaced; modify its fields, or return a new value" |
| `EFF007` | calling a function that mutates a parameter (or a requirement) with an immutable argument (a `let`, an immutable parameter, an exported immutable value, a read-only reference) | "`addBonus` mutates its argument, but `alice` is immutable" |
| `EFF012` | writing a `let` value, an `export =` value, or through it | "`x` is immutable (declared with `let`)" |
| `EFF013` | writing a `mutable` member of a namespace from another file | "`counter.count` can only be written from \"counter.fa\"" |
| `EFF014` | writing a reverse relation side, `length` or `size` outside the collection implementation | see [8.9](#89-errors) |

### 3.8 Tests

```fa
function addBonus = (player: Player) => {
	player.score += 10 -- error EFF003
}
```

```fa
function addBonus = (player: Player) mutates player => {
	player.score += 10
}
let alice = Player { name = "Alice", score = 0 }
addBonus(alice) -- error EFF007
mutable bob = Player { name = "Bob", score = 0 }
addBonus(bob)
console.log(bob.score) -- prints: 10
```

```fa
function resetScore = (player: Player) mutates player.score => {
	player.score = 0
	player.name = "" -- error EFF004
}
```

```fa
function increment = (value: Integer) mutates value => { -- error EFF005
	value += 1
}
```

```fa
function addGuest = (names: Array(String)) => names.push("guest") -- inferred: mutates names
let fixed = ["a"]
addGuest(fixed) -- error EFF007
mutable guests = ["a"]
addGuest(guests)
console.log(guests.length) -- prints: 2
```

```fa
function reset = (player: Player) => {
	player.score = 0 -- error EFF003 (block body: effects must be declared)
}
```

```fa
function resetAll = (players: Array(Player)) mutates players[..].score => {
	for player in players {
		player.score = 0
	}
}
function rename = (players: Array(Player)) mutates players[..].score => {
	for player in players {
		player.name = "x" -- error EFF004
	}
}
```

```fa
function remove = (items: Pool(String)) invalidates items => { -- error EFF009
	items.clear()
}
```

```fa
function compute = (x: Integer): Integer mutates nothing => x -- error EFF008 (unknown root `nothing`)
```

```fa
function noop = (player: Player) mutates player => { -- warning EFF011
	console.log(player.name)
}
```

```fa
mutable people: Pool(Person) = []
function register = (name: String) mutates people.size => {
	people.add({ name = name })
	people = [] -- error EFF004 (only `people.size` is declared)
}
```

```fa
function wipe = (items: Array(Integer)) mutates items => {
	items.deleteWhere((item) => item < 0) -- valid: `mutates items` covers the deletions
}
```

---

## 4. Reference types, `use`, `mutable use`, `reuse`

### 4.1 Reference types

`T in path` is a type: `T` is the type of the designated value, `path` names its **owner**, the nearest collection or string that contains the place. It is a variant of the type table: `Reference { target: TypeId, owner: PathId }`. `(T in path)?` is an optional reference.

| Declaration | Type |
| --- | --- |
| `use someone = persons[1]` | `Person in persons` |
| `use age = people[0].age` | `Integer in people` |
| `use pet = people[0].pets[0]` | `Pet in people[0].pets` |
| `use middle = text[1..3]` | `String in text` |
| `use score = scores.get("ana")` | `(Integer in scores)?` |
| `use address = user.address` (no collection on the path) | `Address`, a **plain alias**: tracked like any reference, never returnable |
| `use x = somePool.add(...)` | `T in somePool`, fresh |

Rules:

- Written in full in parameters, return types and relation fields; inferred for `use` (may be annotated: `use someone: Person in persons = persons[1]`, error `REF006`-like type mismatch if the owner differs).
- Owner paths follow the path grammar. A private path (`self.#nodes`) may appear in a public signature.
- A parameter may be typed by another parameter (`(self, node: Node in self.#nodes)`): at a call, the argument for `node` must be a reference whose owner is the argument of `self` (with its identity), else `REF020` "`node` must be a reference into `list.#nodes`".
- Inside a type, a field type names a collection of the enclosing value: a `Person` reached as `Person in town.people` has `bestFriend: (Person in town.people)?`.
- A function returns a reference **only** when its declared return type says `in`, and the owner path starts at a parameter or a module-level value (`REF008` otherwise). With an inferred return type, a reference-typed expression is copied: `function pick = (persons, index) => persons[index]` returns a `Person`.
- When the argument substituted into a returned reference type is a temporary (`firstPerson(makePeople())`), the result is a reference into a temporary: it can only be copied with `let` (`REF017` with `use`).
- `REF010` a local collection of references: any type that contains a reference type as a type argument outside a relation field (`Array(Person in people)`, `Map(String, Person in people)`, an object type with a reference-typed field that is not a relation side), in a local, a parameter, a return type or a module-level value: "keep indices, or declare a relation `Pool`".
- Assignability: `T in p` is accepted where `T in q` is expected only when `p` and `q` are the same owner path (after substitution). `T in p` is accepted where a plain `T` is expected by a **read** (parameter, comparison): it is a reference. Assigned to `let`/`mutable`, it is copied. `none` is accepted where `(T in p)?` is expected.

### 4.2 `use`

`use x = E` accepts exactly:

1. a **place**: a local, a parameter, a module-level value, a field of a place, an element `a[i]`, a key `m[k]` (not a `get`), a string slice, a requirement name, a loop variable, another `use`;
2. a **call that returns a reference**: `pool.add(...)`, `scores.get(key)`, `people.find(...)`, `getOrInsert(key)`, a user function whose return type says `in`. `try` before the call or the place is allowed (`use chosen = try pick(people, 2)`).

Anything else is `REF004` "`use` needs a place or a call that returns a reference: `5` has no place to designate. Use `let` to keep a value, or `mutable use` and `reuse` to choose between places." This includes `if`/`when` expressions, calls returning values, literals, operators.

Semantics:

- When the place holds a relation reference (direct side, one-to-one reverse side, owner field), `x` designates the **target**: `use next = node.next` is a reference typed `(Node in self.#nodes)?` designating the node, and a later write to `node.next` doesn't move `x`.
- `use { name, age } = person` declares one alias per field (each a `use`).
- Writable iff the place is writable (a `mutable` root, a `mutates`-covered parameter path, a writable reference) and the type is not a read-only element (a `Set`/`GrowingSet` element, a `Map`/`GrowingMap` key: `REF014` when written).
- `use` of a module-level namespace member or of a dependency path (`use { formatDate } = utils.text`) is a **name alias**, not a tracked reference: values exported by files are immutable or `export mutable` (stable address), and their names never dangle. **(decision D14)** A `use` alias of an `export mutable` value itself is a name alias too; a `use` into its elements (`use ana = people[0]`) is a tracked reference.

### 4.3 `mutable use` and `reuse`

- `mutable use x = E` declares a reference that can be moved. Its type is fixed by `E` (or an annotation).
- `reuse x = E`: `x` must be a `mutable use` in scope (`REF005` "only a `mutable use` can be moved with `reuse`" for a `use`, a `let` or a parameter). `E` takes the forms of `use`, or `none` when the type is optional. The type of `E` must be the declared type with the **same owner** (`REF006` "`best` is a `Player in players`: it can't designate an element of `others`").
- `reuse` is a statement, not a declaration: it moves the outer reference even from an inner block, and makes it valid.
- A `mutable use` can be declared on an immutable collection: it designates read-only places.
- `use y = x` where `x` is a `mutable use` designates what `x` designates now; a later `reuse x` doesn't move `y`.
- A `mutable use` is never declared again with `use` in the same scope (`REF007` "`current` is a `mutable use`: move it with `reuse current = ...`").
- A closure can't capture a `mutable use` (`REF011`).

### 4.4 Declaring again

- `use x = ...` in the same scope as an existing `use x` is allowed iff the existing `x` is invalidated or maybe invalidated **at this point of the flow**; otherwise `REF003` "`x` is still valid: declaring it again is an error; choose another name". This is the only exception to "a name is declared once per scope".
- The new declaration is a new symbol (it may have another type: `Person in people` can become `(Person in people)?`). The old symbol is unreachable after it.
- A declaration in an inner block (branch, loop body) is a new name that ends with the block; the outer `x` keeps its state after the block.

### 4.5 Tests

```fa
mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]
let copied = people[1]
use referenced = people[1]
referenced.name = "Cleo"
console.log(people[1].name) -- prints: Cleo
console.log(copied.name) -- prints: Bao
```

```fa
use x = 5 -- error REF004
function makePerson = () => Person { name = "Ana" }
use p = makePerson() -- error REF004
use chosen = if true { 1 } else { 2 } -- error REF004
```

```fa
let persons: Array(Person) = [Person {}, Person {}]
use someone = persons[1]
console.log(@typeToString(Type(someone))) -- prints: Person in persons
someone.name = "x" -- error REF015 (`persons` is immutable: the alias is read-only)
```

```fa
mutable players: Array(Player) = [Player { name = "Ana", score = 3 }, Player { name = "Bao", score = 7 }]
mutable use best = players[0]
for player in players {
	if player.score > best.score {
		reuse best = player
	}
}
best.score += 10
console.log(players[1].score) -- prints: 17
```

```fa
mutable players: Array(Player) = [Player { name = "Ana", score = 3 }]
mutable others: Array(Player) = [Player { name = "Bao", score = 7 }]
mutable use best = players[0]
reuse best = others[0] -- error REF006
use fixed = players[0]
reuse fixed = players[0] -- error REF005
```

```fa
mutable scores: Map(String, Integer) = [("ana", 10)]
mutable use current = scores.get("ana")
reuse current = none
console.log(current) -- prints: none
```

```fa
mutable people: Array(Person) = [Person { name = "Ana" }]
use a = people[0]
use a = people[0] -- error REF003
```

```fa
mutable people: Pool(Person) = []
mutable use current = people.add({ name = "Ana" })
use current = people.add({ name = "Bao" }) -- error REF007
```

```fa
mutable tags: Set(String) = Set.from(["a"])
for tag in tags {
	tag = "b" -- error REF014 (set elements are read-only)
}
```

```fa
function firstPerson = (persons: Array(Person)): (Person in persons)? => {
	if persons.length == 0 {
		return none
	}
	use person = persons[0]
	person
}
mutable persons = [Person { name = "Ana" }]
use first = firstPerson(persons)
if first is not none {
	console.log(first.name) -- prints: Ana
}
```

```fa
function addressOf = (user: { address: { city: String } }): { city: String } in user => user.address -- error REF008 (no collection on the path)
function makePeople = () => [Person { name = "Ana" }]
function firstOf = (persons: Array(Person)): (Person in persons)? => persons.get(0)
use p = firstOf(makePeople()) -- error REF017
let q = firstOf(makePeople()) -- valid: a copy
```

```fa
function keep = (people: Pool(Person), chosen: Array(Person in people)) => {} -- error REF010
```

---

## 5. The reference checker

Normative source: `documentation/advanced/reference-checking.md`. Every compiler accepts **exactly** these rules: no smarter analysis.

### 5.1 State

Part of the flow state:

```rust
struct RefState {
	kind: RefKind,              // Use | MutableUse | Parameter | LoopVariable | Iterator | Provided
	places: SmallVec<[Place; 1]>, // one place; several for a `mutable use` after joins
	writable: bool,
	status: Status,             // Valid | Invalidated { by: NodeId, maybe: bool }
	birth: Time,                // logical time of creation, for freshness
}

struct Place {
	path: PathId,               // full path from a root, identities on index steps
	owner: Option<PathId>,      // nearest collection or string prefix; None for a plain alias
	declaredType: TypeId,       // type of the designated value
}

struct FlowRefs {
	refs: PersistentMap<SymbolId, RefState>, // persistent (cheap fork/join)
	lengthFacts: ...,           // owned by range analysis; read here
	time: Time,                 // incremented by every call that may create fresh references
}
```

Parameters are entered at function entry, with `birth = 0`, place = their canonical path ([3.1](#31-paths)), `Valid`.

### 5.2 Uses of a reference

Reading through it, writing through it, passing it as an argument, capturing it, creating another reference from it (`use y = x`, `use y = x.field`, `reuse y = x`), returning it: each is a **use**. A use of `x` with `status = Invalidated { by, maybe }` is:

- `REF001` (`maybe = false`): "`someone` was invalidated by `persons.push` (line 12): declare it again with `use someone = persons[...]`";
- `REF002` (`maybe = true`): "`ana` may have been invalidated by `people.delete` (line 8)" with the same fix. For a `mutable use`, the fix is "move it with `reuse ana = ...`".

The fix text uses the original right-hand side with the index or key replaced by `...` when it was not a literal.

### 5.3 Creating a reference

`use x = E` / `mutable use` / `reuse` / a loop variable / a returned reference bound by `use`:

- **Place** `E`: `place = places[E]`; identities of index steps are computed from the current facts ([5.4](#54-identities-and-may-designate)); `birth = now`. If `E` is itself a reference `r` (or below one), the new place is `r`'s place extended, and **birth and identities are inherited** from `r`.
- **Call** `E`: after applying the call's effects ([5.7](#57-calls)), the returned reference's place is the substituted return type's owner with:
  - `Fresh(t)` identity if the callee returns fresh references (built-in `add` of `Pool`, `GrowingPool`, `Columnar`, `GrowingColumnar`, or an inferred-fresh user function), where `t = ++time`;
  - the key's identity for built-ins that keep it: `getOrInsert(key)`, `get(key)`, `m[key]`;
  - `Unknown` for every other call (identities don't cross a user function).
- **Relation field** `E = a.f`: the target's place is `C[..]` with `Unknown` identity, `birth = now`.

### 5.4 Identities and may-designate

An identity is attached to every `Index`/`Range` step:

```rust
enum Identity {
	Exact(ConstValue),             // literal, or exact literal fact of the index value
	Symbol(SymbolId, i64),         // immutable binding (let, parameter) + offset
	LengthOf(PathId, i64),         // `c.length - 1` while the length of `c` hasn't changed
	Interval(i64, i64),            // snapshot of a range fact, when the index is not an immutable binding
	Fresh(Time),                   // returned by a fresh-returning call at this time
	Unknown,
}
```

- The identity is taken when the reference is created: a later change of a `mutable` index doesn't change it (a `mutable` index gives an `Interval` snapshot or `Exact` snapshot, never `Symbol`).
- For `Symbol(s, k)`, the interval used by rule 2 is **the current interval fact of `s` plus `k`** (an immutable binding never changes, but its facts are refined by guards: this is why `c = people[j]` sees `j`'s narrowed range inside `if j < i`).
- `LengthOf(c, k)` degrades to `Unknown` when the length of `c` changes (`mutates c`, `mutates c.length`, `mutates c.size`).

`distinct(a, b)` (the four rules, applied at the **first step where two paths differ**):

1. Different `Field` names; different roots (two locals, two parameters, a parameter and a module-level value) — parameters are distinct roots because of the alias rule, except a parameter typed `T in p`, which is canonicalized inside `p` ([3.1](#31-paths)).
2. Two `Exact` that differ; two intervals that don't intersect (computed from `Exact`, `Symbol` + current facts, `Interval`); the same `Symbol` (or the same `LengthOf`) with different offsets. Against a `Range` step `[s..e]`: an index is distinct when it is proven below `s` or above `e` (exclusive or inclusive end) by the same comparisons.
3. One is `Fresh(t)` and the other reference's `birth < t`; or both are `Fresh` with different times. Every identity on a path carries the birth of the reference it came from, so the rule applies the same way when one of the two paths is an effect path: in `pool.delete(found)`, the step of the effect carries the birth of `found`, which is before the `add` that made `added` fresh.
4. Their owners are distinct by rules 1 to 3 (`towns[0].people` vs `towns[1].people`): rule 4 is just rules 1–3 applied at an earlier step.

Nothing else: no relation between two different symbols is kept (`if j < i` only narrows `i` and `j`), no other arithmetic.

```rust
/// May `r` designate `p`, or a place below `p`? (`r` at or below `p`)
fn at_or_below(r: &Path, p: &Path, birth_r: Time) -> bool {
	if !roots_may_alias(r.root, p.root) { return false }
	for k in 0..p.steps.len() {
		if k == r.steps.len() { return false }          // r is strictly above p
		if steps_distinct(&r.steps[k], &p.steps[k], birth_r) { return false }
	}
	true
}

/// May two places overlap (one designates the other or a place below it)?
fn overlap(a: &Path, b: &Path) -> bool {
	if !roots_may_alias(a.root, b.root) { return false }
	for k in 0..min(a.len(), b.len()) {
		if steps_distinct(&a.steps[k], &b.steps[k], ..) { return false }
	}
	true
}
```

`steps_distinct` returns `true` for `Field(x)` vs `Field(y)` with `x != y`, for `Length`/`Size` vs any element step, for two index identities proven distinct, and `false` for `All` against anything.

### 5.5 What an effect invalidates

`applyEffect(effect, flow)` for each live reference `R` (each of its places for a `mutable use`):

- **`invalidates P`**: `R` is invalidated iff `at_or_below(R.place, P)` (it may designate `P` or a place below `P`). A reference strictly above `P` stays valid.
- **`mutates P`**: `R` is invalidated iff `R` is **strictly below** `P` (`at_or_below` and longer) **and** the route from `P` to `R` crosses an owning edge **according to the declared type attached to `P`** ([3.5](#35-substitution-at-a-call)):

```rust
fn mutation_invalidates(r: &Path, p: &Path, mut ty: TypeId) -> bool {
	// precondition: at_or_below(r, p) and r.len() > p.len()
	for step in &r.steps[p.len()..] {
		match step {
			Field(name) => match field_type(ty, name) {
				None => return false,           // the callee's type has no such field: it can't write there
				Some(t) => ty = t,
			},
			Length | Size => return false,
			_ /* Index, Range, All, Characters, Payload, Child */ => {
				if !is_type_parameter(ty) && is_flat(ty) { return false }
				return true                     // an owning edge
			}
		}
	}
	false
}
```

  A type parameter in a generic callee is never flat (the check returns `true` at its first owning edge, and also: a `Field` step below a type parameter is treated as possibly owned).
- An invalidation applies on **every exit** of the call, including when it throws: apply before forking for a `catch` handler.
- `mutates` also drops facts (narrowing, ranges, exact literals) of `P` and below, and length facts of collections at or below `P` (owned by the range-analysis brief; listed here because `LengthOf` identities depend on them).
- `mutates c.size` / `mutates c.length` invalidates nothing (flat), but drops the length facts of `c`.
- Setting `R.status = Invalidated { by: callNode, maybe: false }`. Joins produce `maybe: true`.

### 5.6 Control flow

The checker walks structured code once (plus loop iterations):

- **Sequence**: state threads through.
- **Branches** (`if`/`else`, `when` arms, `catch` handlers, the right side of `and`, `or`, `??`, `?.` chains): fork the state; walk each branch; **join** the states of the branches that reach the end. A branch ending in `return`, `throw`, `break`, `continue` (type `Never`) does not reach the join (its state goes to the function exit, the loop exit, or the loop head).
- **Join**: pointwise over the references live in both states (references declared inside a branch are dropped at its end):
  - status: `Valid ⊔ Valid = Valid`; anything else is `Invalidated { maybe: true }` (keep the `by` of an invalidating branch for the message), except `Invalidated{maybe:false} ⊔ Invalidated{maybe:false}` = definite;
  - `mutable use` places: union of the sets;
  - identities: the convex hull (`Exact(3) ⊔ Exact(5) = Interval(3, 5)`; `Symbol(i,0) ⊔ Symbol(i,0)` stays; different symbols → hull of their intervals; anything with `Unknown` → `Unknown`; `Fresh(t) ⊔ Fresh(t)` stays, otherwise `Unknown`).
- **Loops** (`for`, `while`, `while let`, `loop`), to a fixpoint:

```
head₀ = before
repeat:
	walk the body from head_i (flow transfer only after the first visit)
	head_{i+1} = join(before, end of body, every `continue` state)
until head_{i+1} == head_i
after = join(head (for `for`/`while`: the condition may be false), every `break` state)
```

  Intervals are widened as in range analysis (a bound that grows twice becomes unbounded), so the fixpoint is reached in a few rounds. A `loop` without `break` has no "after" (type `Never`).
- **Declaring again** inside a branch or loop body creates a new name that ends with the block ([4.4](#44-declaring-again)). Only `reuse` repairs an outer reference from inside.

### 5.7 Calls

Order at a call node `f(a1, ..., an)`:

1. Check each argument expression (uses of references inside them are checked against the current state).
2. Compute the **used places**: each argument's place (or temporary), the places captured by each lambda argument (for non-stored parameters: captured by reference, read or written during the call), the requirement providers the callee uses, the module-level values the callee reads or mutates (summary).
3. Compute the callee's effects **substituted** ([3.5](#35-substitution-at-a-call)), plus their implied relation effects ([8.6](#86-implied-effects)), plus the effects of non-stored lambda arguments (their writes to captured places and their own calls' effects, already computed when the lambda was checked against the expected parameter type).
4. Check **mutability** (`EFF007`) of every argument whose parameter is mutated, and of every provider of a mutated requirement (`REQ004`).
5. Check the **alias rule** ([5.9](#59-arguments-that-alias)).
6. Check the **iteration rule** against the iteration stack ([5.8](#58-iteration)).
7. **Apply** the effects to the flow state ([5.5](#55-what-an-effect-invalidates)). Increment `time` if the callee returns fresh references.
8. Accumulate the effects whose roots are not locals into the current function's accumulator.
9. Bind the result (a reference if the return type says `in`, [5.3](#53-creating-a-reference)).

### 5.8 Iteration

Entering `for x in L` (where `L` is a place; a temporary collection has no constraint) pushes `L` on the **iteration stack** of the flow state; an iterator from `L.values()` pushes `L` for as long as the iterator is live. Every effect applied inside the body (direct, through aliases, through callees, implied) is checked against every entry `L` of the stack:

- `REF012` `mutates P` where `at_or_below(L, P)` (it mutates `L` or a place above `L`): "can't add elements to `scores` while iterating over it" / "`register` mutates `people.size` while the loop iterates over `people`";
- `REF012` `mutates L.length`, `mutates L.size`;
- `REF012` `invalidates P` where `P` may designate an element of `L` (`overlap(P, L[..])`, P at or below `L[..]`);
- writing elements and places inside them (`player.score += 1`, `scores[index] = 0`) is allowed.

For relations, the implied effects are what triggers the error: `for friend in ana.friends { town.people.delete(friend) }`: `L = town.people[@ana].friends`; the implied effect `mutates town.people[..].friends` has `L` at or below it → `REF012` "deleting `friend` removes it from `ana.friends` (implied `mutates town.people[..].friends`), which is being iterated".

The loop variable: a reference with place `L[Unknown]` (or `L[Symbol(index, 0)]` when the index binding exists), `kind = LoopVariable`, writable iff `L` is writable and the element is not read-only (set elements, map keys). For a range or a string, the loop variable is a copy (`Integer`, `Character`).

### 5.9 Arguments that alias

For a call, let `M` be the set of places it mutates or invalidates (declared, substituted, with implied effects). For each `m ∈ M` and each **other used place** `u` (other arguments, including the same value passed twice; requirement providers the callee uses; module-level values the callee reads or mutates; references captured by callback arguments): if `overlap(m, u)` → `REF013` "`push` mutates `numbers`, and its argument `numbers[0]` is inside `numbers`: copy it first with `let`".

The argument whose parameter `m` derives from is not "other" for `m` itself. Two allowed overlaps (the callee checks its body knowing them):

- the argument of a parameter typed `T in p` is inside the argument of `p` (`people.delete(ana)`, `list.remove(node)`);
- an **implied** effect `C[..].f` overlaps an argument typed `T in C` (`befriend(ana, bao)` writes the reverse sides of `people[..]`).

Declared effects are still checked: `befriend(ana, ana)` → `REF013` (`mutates people[@ana].bestFriend` overlaps the argument `friend = people[@ana]`); `befriend(ana, someone)` where `someone` may be `ana` → `REF013`.

`use` aliases may alias each other freely; inside a callee, two parameters are distinct roots (except the declared overlaps).

### 5.10 Returning references

- A `return e` (or final expression) where the declared return type is `T in p` (or optional): `e` must be a reference (valid) whose owner equals `p` after substituting the function's own parameters — `REF009` "`person` is a reference into `others`, but `firstPerson` returns a reference into `persons`".
- The returned reference is valid after the call's effects: the callee's body must not invalidate it after creating it (checked naturally: it's a use of the reference at the return).
- **Freshness inference**: the function returns fresh references iff every returned reference value is `Fresh` (created by a fresh-returning call in this body, or returned by a fresh-returning callee) — `none` returns are allowed in an optional. Recorded in the summary. A call through a function value never returns fresh references.
- With an inferred or plain return type, returning a reference copies the value.

### 5.11 Closures

Capture mode depends on **escape**. A closure escapes when it is stored (field, collection, module-level value), returned, passed to a parameter whose `stored` bit is set, captured by another escaping closure, or used by a started task. For a lambda written as an argument, escape is known at once (the callee's summary). For a local `function` declaration, it is known only at its later uses **(decision D3)**:

1. When the closure is declared, check its body with the current flow state; record its **captures**: each captured symbol with read/write, and the captured references.
2. Every captured reference must be **valid at the declaration** (an escaping closure copies the designated value there).
3. Record a **pending constraint**: "if this closure escapes: it must not write any captured local (`REF021` "`count` is captured by copy, since the closure escapes (stored at line 14)"), and must not capture a `mutable use`" (a `mutable use` capture is `REF011` in every case).
4. At each **call** of a non-escaping closure (direct call, or the call it is passed to), the captured references must be valid **there** (`REF001`/`REF002`), its writes to captured locals are applied to the flow state as `mutates local`, and its effects on outer places are effects of that call.
5. At the first escaping use, check the pending constraint, mark `escapes[closure] = true`. Once escaping, every use copies (the copy is taken at the declaration: the backends emit the environment copy at the declaration point, which is why the decision must be available before code generation, and is, since the backends run after the walk).

The **stored** bit of a callee's function-typed parameter is inferred in the callee's own walk: the parameter is stored if it flows into any escaping position listed above. A stored parameter's function type must declare every effect of the argument (`EFF015` "`onClick` is `() => Void`, which doesn't declare the errors of `save`"); a non-stored one adopts the argument's effects as effects of the call. Inside the callee, calling a non-stored parameter needs no `try` and can't be caught.

Module-level values and requirements are never captured.

### 5.12 Error catalog

| Code | Condition |
| --- | --- |
| `REF001` | use of a reference that is definitely invalidated |
| `REF002` | use of a reference that is maybe invalidated (after a branch or a loop) |
| `REF003` | `use x` again while the existing `x` is valid |
| `REF004` | `use` / `reuse` on something that is neither a place nor a reference-returning call |
| `REF005` | `reuse` on a name that is not a `mutable use` |
| `REF006` | `reuse` (or an annotated `use`) with a different type or owner |
| `REF007` | `use x` again while `x` is a `mutable use` |
| `REF008` | a return type `T in p` whose path doesn't start at a parameter or a module-level value, or a plain alias returned as a reference |
| `REF009` | a returned reference whose owner differs from the declared one |
| `REF010` | a local collection of references, or a reference-typed field that is not a relation side |
| `REF011` | a closure captures a `mutable use` |
| `REF012` | the iteration rule |
| `REF013` | the alias rule |
| `REF014` | writing through a read-only reference (immutable collection, set element, map key) |
| `REF015` | writing through a `use` alias of an immutable place |
| `REF017` | `use` of a reference into a temporary |
| `REF020` | argument of a `T in p` parameter that is not a reference into the argument of `p` |
| `REF021` | an escaping closure writes a captured value |

### 5.13 Tests

Valid programs print; invalid ones are rejected at the marked line. Most come from the documentation.

```fa
mutable persons: Array(Person) = [Person {}, Person {}, Person {}]
use someone = persons[1]
someone.age = 30
persons.pop() -- invalidates persons[persons.length - 1] = persons[2]: `someone` (index 1) stays valid
console.log(someone.age) -- prints: 30
persons.push(Person {})
console.log(someone.age) -- error REF001
```

```fa
mutable people: Array(Person) = [Person { name = "Ana", age = 31, pets = [Pet { name = "Rex" }] }]
use person = people[0]
use age = people[0].age
use initials = people[0].name[0..1]
use pet = people[0].pets[0]
people[0].age = 32
people[0] = Person { name = "Bao", age = 12, pets = [] }
console.log(person.name) -- prints: Bao
console.log(age) -- prints: 12
console.log(pet.name) -- error REF001
console.log(initials) -- error REF001
```

```fa
mutable persons: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }]
use someone = persons[1]
persons.delete(0)
console.log(someone.name) -- error REF001
use someone = persons[0]
console.log(someone.name) -- prints: Bao
```

```fa
mutable scores: Map(String, Integer) = [("ana", 10), ("bao", 7)]
use score = scores.get("ana")
scores.delete("bao")
if score is not none {
	console.log(score) -- prints: 10
}
```

```fa
function readName = (): String => "ana"
mutable scores: Map(String, Integer) = [("ana", 10), ("bao", 7)]
use score = scores.get("ana")
let name = readName()
scores.delete(name)
console.log(score) -- error REF001
```

```fa
mutable people: Pool(Person) = []
use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })
people.delete(bao)
console.log(ana.name) -- prints: Ana
use someone = people.find((person) => person.name == "Cleo")
if someone is not none {
	people.delete(someone)
	console.log(ana.name) -- error REF001
}
```

```fa
mutable people: Pool(Person) = []
use ana = people.add({ name = "Ana", age = 31 })
people.add({ name = "Bao", age = 12 })
people.deleteWhere((person) => person.age < 18)
console.log(ana.name) -- error REF001
```

```fa
mutable people: Pool(Person) = []
use ana = people.add({ name = "Ana", age = 31 })
people.add({ name = "Bao", age = 12 })
people.deleteWhere((person) => person.age < 18)
use ana = people.find((person) => person.name == "Ana")
if ana is not none {
	console.log(ana.name) -- prints: Ana
}
```

```fa
mutable people: Pool(Person) = []
mutable use current = people.add({ name = "Ana" })
people.delete(current)
reuse current = people.add({ name = "Bao" })
console.log(current.name) -- prints: Bao
```

```fa
let leaving = true
mutable people: Pool(Person) = []
use ana = people.add(Person { name = "Ana" })
if leaving {
	people.delete(ana)
}
console.log(ana.name) -- error REF002
```

```fa
let leaving = true
mutable people: Pool(Person) = []
use ana = people.add(Person { name = "Ana" })
if leaving {
	people.delete(ana)
}
use ana = people.find((person) => person.name == "Ana") -- valid: maybe invalidated, so declared again
console.log(ana?.name ?? "gone") -- prints: gone
```

```fa
function example = (index: Integer(min = 0, max = 2)) => {
	mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }, Person { name = "Dan" }]
	use current = people[index]
	use last = people[3]
	people.delete(index + 1)
	console.log(current.name) -- valid
	console.log(last.name) -- error REF001
}
```

```fa
function example = (i: Integer(min = 1, max = 2), j: Integer(min = 0, max = 3)) => {
	mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }, Person { name = "Dan" }]
	use a = people[0]
	use b = people[i]
	use c = people[j]
	if j < i {
		people.delete(i)
		console.log(a.name) -- valid
		console.log(b.name) -- error REF001
		console.log(c.name) -- error REF001
	}
}
```

```fa
mutable pool: Pool(Person) = []
pool.add(Person { name = "Ana" })
use found = pool.find((person) => person.name == "Ana")
use added = pool.add(Person { name = "Bao" })
if found is not none {
	pool.delete(found)
	console.log(added.name) -- prints: Bao
}
```

```fa
mutable pool: Pool(Person) = []
use added = pool.add(Person { name = "Bao" })
use found = pool.find((person) => person.name == "Bao") -- found after the add: may be `added`
if found is not none {
	pool.delete(found)
	console.log(added.name) -- error REF001
}
```

```fa
function pick = (persons: Array(Person), index: Integer): Person in persons => persons[index] -- throws IndexError, inferred
mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }]
use chosen = try pick(people, 2)
people.delete(1)
console.log(chosen.name) -- error REF001 (identities don't cross a user function)
```

```fa
mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]
use first = people[0]
for index in 0..10 {
	console.log(first.name) -- error REF002
	if index == 3 {
		people.push(Person { name = "Cleo" })
		use first = people[0]
	}
}
```

```fa
mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]
mutable use first = people[0]
for index in 0..3 {
	console.log(first.name)
	if index == 1 {
		people.push(Person { name = "Cleo" })
		reuse first = people[0]
	}
}
-- prints: Ana
-- prints: Ana
-- prints: Ana
```

```fa
mutable numbers = [1, 2, 3]
numbers.push(numbers[0]) -- error REF013
let first = numbers[0]
numbers.push(first)
console.log(numbers) -- prints: [1, 2, 3, 1]
```

```fa
mutable people: Pool(Person) = []
use ana = people.add(Person { name = "Ana", age = 31 })
people.deleteWhere((person) => person.age < ana.age) -- error REF013
let limit = ana.age
people.deleteWhere((person) => person.age < limit) -- valid
```

```fa
function swap = (a: Player, b: Player) mutates a, b => {
	let score = a.score
	a.score = b.score
	b.score = score
}
mutable team = Player { name = "Ana", score = 1 }
swap(team, team) -- error REF013
```

```fa
mutable pool: Pool(Person) = []
for human in pool {
	if human.name == "John" {
		pool.delete(human) -- error REF012
	}
}
```

```fa
mutable scores = [1, 2, 3]
for score in scores {
	scores.push(0) -- error REF012
}
for score, index in scores {
	scores[index] = 0 -- valid: writes an element
}
for score in scores {
	score += 1
}
console.log(scores) -- prints: [1, 1, 1]
```

```fa
mutable people: Pool(Person) = []
function register = (name: String) mutates people.size => {
	people.add({ name = name })
}
for person in people {
	register("{person.name}'s friend") -- error REF012
}
```

```fa
mutable numbers = [1, 2, 3]
let iterator = numbers.values()
numbers.push(4) -- error REF012 (an iterator into `numbers` is still used below)
console.log(iterator.toArray())
```

```fa
type Guest = { name: String, welcomed: Boolean = false }
function addGuest = (guests: Array(Guest), name: String) mutates guests => {
	guests.push(Guest { name = name })
}
function welcome = (guest: Guest) mutates guest.welcomed => {
	guest.welcomed = true
}
mutable guests: Array(Guest) = [Guest { name = "Ana" }, Guest { name = "Bao" }]
use second = guests[1]
welcome(guests[1])
console.log(second.welcomed) -- prints: true
addGuest(guests, "Chloé")
console.log(second.name) -- error REF001
```

```fa
function age = (person: { age: Integer }) mutates person => {
	person.age += 1
}
mutable people: Array(Person) = [Person { name = "Ana", pets = [Pet { name = "Rex" }] }]
use pet = people[0].pets[0]
age(people[0]) -- the callee's type has only `age`: nothing owned is reachable
console.log(pet.name) -- prints: Rex
```

```fa
function main = () => {
	mutable count = 0
	mutable callbacks: Array(() => Void) = []
	callbacks.push(() => {
		count += 1 -- error REF021
	})
}
```

```fa
function main = () => {
	mutable greeting = "Hello"
	function greet = (name: String) => console.log("{greeting}, {name}!")
	greeting = "Bye"
	greet("John") -- prints: Bye, John!
}
```

```fa
function main = () => {
	mutable numbers = [1, 2]
	use first = numbers[0]
	function grow = () => numbers.push(3) -- non-escaping: writes the captured local
	grow()
	console.log(first) -- error REF001 (the call of `grow` mutates `numbers`)
}
```

```fa
function main = () => {
	mutable people = [Person { name = "Ana" }]
	mutable use current = people[0]
	function show = () => console.log(current.name) -- error REF011
}
```

```fa
type Inventory = {
	items: Pool(String)
	function remove = (self, item: String in self.items) invalidates item => {
		self.items.delete(item)
	}
}
mutable inventory = Inventory { items = [] }
use sword = inventory.items.add("sword")
use shield = inventory.items.add("shield")
inventory.remove(sword)
console.log(shield) -- prints: shield
console.log(sword) -- error REF001
```

```fa
mutable people: Pool(Person) = []
use ana = people.add(Person { name = "Ana" })
mutable others: Pool(Person) = []
others.delete(ana) -- error REF020 (`ana` is a reference into `people`, not `others`)
```

---

## 6. Memory management: copies, moves, destruction, closures

The rules (memory-management page): objects inline; collections own the heap; every assignment is a copy except thread-safe handles; copies become moves when the source is dead; `use` aliases; parameters are immutable references; iteration yields references; every value is destroyed at the end of its scope, in reverse declaration order; a field holds a copy unless it is a relation side.

### 6.1 Static rules

- `MEM001` a recursive type without an indirection through a union or an optional: "`Node` has infinite size" (`next: Node`). Recursion through `T?`, a named union, a collection (`Array(Folder)`) is fine.
- Recursion is allowed through named declarations only; polymorphic recursion is an error (types brief).
- A move-only value (task — out of scope —, a `destroy` type, a worker handle, a `Subscription`) used after it was moved: `MEM002` "`database` was moved at line 7".
- A `Void` value can't be bound with `let`.

### 6.2 Copies and moves

Copy-to-move is an optimization that is never observable, except for move-only types where it is the semantics. The analyzer records, in the same walk:

- for each local `v`: its **last use event** (a node that reads `v`, or reads through an alias whose root is `v`), and whether that event is inside a loop or a closure that doesn't contain `v`'s declaration;
- every `use` alias rooted at `v` extends `v`'s life to the alias's last use (the alias's reads are attributed to `v`).

At the end of `v`'s scope: if the last use event is a copy position (`let`/`mutable` initializer, assignment right side, argument to a collection insertion like `push`/`add`, a return, a field of an object literal, a captured value of an escaping closure) and is not inside a deeper loop or closure, mark `moves[event] = true`. A narrowing copy (destination type narrower than the source) is never a plain move: it copies the destination's fields; the source's remaining fields are still destroyed with the source (native) — documented as "Narrowing is always a copy".

Move-only values: every copy position is a move; a later use of the source is `MEM002`. A conditional move (moved in one branch) is allowed; the source is then unusable after the join (maybe moved), and native code needs a drop flag.

### 6.3 Destruction

- Native: a cleanup stack while lowering; at every scope exit (fallthrough, `break`, `continue`, `return`, `throw`, a propagating `try`) drop the owning values declared in the scope, in reverse order; drop flags for conditionally moved or partially initialized values. No unwinding.
- JavaScript: no destructors, except the values that need an action at scope exit: `watch` subscriptions and `derived` nodes (`dispose`), `destroy` types of foreign code on `bun`, worker handles. For these, the compiler emits the action at every exit of the scope (end of block, `return`, `break`, `continue`) and in a `finally` for error propagation, since Fa errors are JavaScript exceptions on this target (errors brief).
- Module-level values are destroyed in reverse initialization order at normal exit ([15.7](#157-module-level-values-and-initialization)).

### 6.4 Heap blocks made by the compiler

- **Recursive types**: the recursive member of a union is a heap block owned by the parent: copied by `copy<Shape>`, freed by `drop<Shape>`, both iterative (no native recursion) to survive deep values. JavaScript: plain nested objects.
- **Escaping closures**: environment of captured copies; ≤ 8 bytes and trivially copyable inline in the `env` word, else a heap block with a `{ size, align, copy, drop }` descriptor (LLVM Q4). JavaScript: a closure over `const` locals holding the copies (`const display$copy = copy_Display(display)` before the arrow function); no copy when the captured value is immutable (`let`, immutable parameter of a value nobody can mutate during the closure's life — see [6.5](#65-javascript-copies)).
- **Thread-safe handles**: reference counted ([14](#14-concurrency-without-async)).
- **Task frames**: out of scope.

### 6.5 JavaScript copies

- Primitives and strings are immutable JavaScript values: never copied.
- Objects and collections: `let b = a` emits `const b = copy_T(a)` (a generated function per destination type, which reads only the destination's fields — never `structuredClone`, never a spread), **unless** no later write to either side can be observed. The cheap sufficient rule, decided in one walk **(decision D15)**: no copy when (1) the read is a move (`moves[node]`), or (2) the source and the destination are both immutable for the rest of their lives (`let` destination and a source that is a `let`, an `export =` value, an immutable parameter whose caller can't mutate it during the call — always true, since parameters are references to values the callee doesn't mutate and the alias rule forbids mutation through other arguments), **and** the destination type has the same fields as the source (no narrowing).
- A narrowing copy always calls `copy_Destination(value)`.

### 6.6 JavaScript lowering of references

From `values/declaring-values` (implementation section):

- `use x = place` of an object or collection: `const x = <place expression>` (the JavaScript reference).
- `use x = place` of a **primitive place** (`player.score`, `scores[i]`, a map value): the container and the key are evaluated once into locals (`const x$o = player; const x$k = "score"`), and every use of `x` is rewritten to the place (`x$o[x$k]`, or `x$o.score` when the key is a constant field). For a `Map`, the container is the map and accesses are `x$o.get(x$k)`/`x$o.set(x$k, v)`.
- `mutable use` of a primitive place: a pair of `let` locals, which `reuse` reassigns. Of an object place: one `let`.
- A loop variable over a collection of primitives that is written in the body: an index loop, with writes rewritten to `array[i]`.
- Parameters: always the JavaScript value. A primitive parameter can't be mutated, so passing its value is exact; the alias rule guarantees that nothing mutates the argument during the call through another path.
- A string slice reference `use middle = text[1..3]`: the slice is computed once (strings are immutable in JavaScript); the slice is invalidated whenever `text` is mutated, so it never needs to observe changes.

### 6.7 Native lowering

As in `compilation-with-llvm.md` (memory management and data layout): `use` is a pointer, a slice `{ptr, len}`, `mutable use` a pointer variable; trivially copyable values have no copy or drop code; owning values get per-shape `copy<S>`/`drop<S>`; construction in the destination (`sret`, collection slots); last-use moves are `memcpy` without drop. References cost nothing at runtime: no counters, no generations.

### 6.8 Tests

```fa
mutable object = { x = 12, hello = "world", nested = { y = 21 } }
let object2 = object
object.hello = "you"
console.log(object2.hello) -- prints: world
```

```fa
type Node = {
	value: String
	next: Node -- error MEM001
}
```

```fa
type Node = { value: String, next: Node? }
let list = Node { value = "a", next = Node { value = "b", next = none } }
mutable copy = list
copy.next = none
console.log(list.next?.value ?? "none") -- prints: b
```

```fa
type Tree = Union {
	Leaf = { value: Integer }
	Branch = { left: Tree, right: Tree }
}
function sum = (tree: Tree): Integer => when tree is {
	Tree.Leaf as leaf => leaf.value
	Tree.Branch as branch => sum(branch.left) + sum(branch.right)
}
console.log(sum(Tree.Branch { left = Tree.Leaf { value = 1 }, right = Tree.Leaf { value = 2 } })) -- prints: 3
```

```fa
-- deep recursive value: copying and dropping must not overflow the native stack
type Link = { value: Integer, next: Link? }
mutable head: Link? = none
for index in 0..1_000_000 {
	head = Link { value = index, next = head }
}
let copy = head
console.log(copy?.value ?? -1) -- prints: 999999
```

```fa
mutable pool: Pool(Person) = []
use john = pool.add({ name = "John", age = 20 })
john.age += 1
console.log(pool.find((person) => person.name == "John")?.age ?? 0) -- prints: 21
```

```fa
type Human = { name: String, age: Integer }
mutable people: Array(Human) = [{ name = "John", age = 20 }, { name = "Jane", age = 21 }]
mutable copiedHuman = people[1]
copiedHuman.age = 23
console.log(people[1].age) -- prints: 21
use favoriteHuman = people[1]
favoriteHuman.age = 23
console.log(people[1].age) -- prints: 23
```

---

## 7. Collection effects (owned references)

The collection pages and the standard library own the APIs; the reference checker only needs their **signatures**, which are ordinary Fa declarations in the standard library (so no special case in the checker). The effects to declare:

| Collection | Operation | Signature effect | Returns |
| --- | --- | --- | --- |
| `Array` | `array[index] = value` | `mutates self[index]` | |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` | `take`: `T` |
| | `pop()` | `invalidates self[self.length - 1]` | `T?` |
| | `push`, `insert`, `sortInPlace`, `reverseInPlace`, `deleteWhere`, `clear` | `mutates self` | |
| | `get(index)` | — | `(T in self)?` (bound by `use`) or a copy |
| `Map` | `map[key] = value` | `mutates self[key], self.size` | |
| | `getOrInsert(key, ...)` | `mutates self.size` | `V in self`, identity of `key` |
| | `get(key)` | — | `(V in self)?`, identity of `key` |
| | `delete(key)`, `take(key)` | `invalidates self[key]` | |
| | `deleteWhere`, `clear` | `mutates self` | |
| `Set` | `add`, `delete`, `take`, `deleteWhere`, `clear` | `mutates self` | `take`: `T?` |
| `Pool`, `Columnar` | `add(value)` | `mutates self.size` | `T in self`, **fresh** |
| | `delete(element)`, `take(element)` with `element: T in self` | `invalidates element` | |
| | `find(predicate)` | — | `(T in self)?` |
| | `deleteWhere`, `clear` | `mutates self` | |
| `OrderedColumnar` | `columnar[index] = value` | `mutates self[index]` | |
| | `add` | `mutates self` | `T in self` (not fresh: positional) |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` | |
| | `pop` | `invalidates self[self.length - 1]` | |
| | `deleteWhere`, `clear` | `mutates self` | |
| `GrowingArray` | `push` | `mutates self.length` | |
| | `array[index] = value` | `mutates self[index]` | |
| `GrowingMap` | `map[key] = value` | `mutates self[key], self.size` | |
| | `getOrInsert` | `mutates self.size` | |
| `GrowingSet`, `GrowingPool`, `GrowingColumnar` | `add` | `mutates self.size` | `GrowingPool`/`GrowingColumnar`: fresh `T in self` |
| relation pools ([8.4](#84-writing-relations)) | `add(target)`, `delete(target)`, `clear()` | `mutates <the field>` | `Boolean` |

`length`/`size` are read-only outside the collection implementation (`EFF010`/`EFF014`). Fresh-returning functions in the standard library are marked by inference like user code (the body of `Pool.add` returns the result of an intrinsic slot allocation, which is the base case of freshness: a std-internal intrinsic `intrinsics.freshSlot` is fresh by definition).

---

## 8. Relations

### 8.1 Syntax

```fa
type Town = {
	people: Pool(Person) = []
	type Person = {
		name: String
		bestFriend: (Person in people)?            -- direct side, one-to-many
		bestFriendOf: Pool(people->bestFriend)     -- reverse side
		mentor: (Person in people)?                -- direct side, one-to-one
		mentee: (people->mentor)?                  -- reverse side
		friends: Pool(Person in people)            -- direct side, many-to-many
		friendOf: Pool(in people->friends)         -- reverse side
		spouse: (people->spouse)?                  -- symmetric
	}
}
```

Parser nodes: `In { left, right }` for `T in c`, `Relation { left, right }` for `c->f`, `InRelation { right }` for `in c->f` (its doc comment mentions `Bag`, stale).

### 8.2 Classification (at type declaration, once per relation)

For each field of an object type `E` (the element type) whose type mentions `in` or `->`:

1. Resolve the collection name `c` in the field's scope: a field of an **enclosing** type's value (`people` of `Town` for a `Person` declared inside `Town`), a field of `E`'s own type (an **owner field** candidate: `firstChild: (Node in #nodes)?` is a field of `LinkedList`, which owns `#nodes`), or a module-level value (`people` exported by `people.fa`). Else `REL005` "`people` is not a collection visible from `Person`: declare it as a field of the enclosing type, or as a module-level value".
2. The collection type must be **stable-address**: `Pool`, `Columnar`, `GrowingPool` (and `GrowingColumnar`, **decision D16**). Else `REL003` "`people` is an `Array`, whose elements move: a relation needs a `Pool`, a `Columnar` or a `GrowingPool`".
3. The target type `T` in `T in c` must be the element type of `c` (by identity) — `REL004`.
4. Shape → kind:

| Field type | Kind |
| --- | --- |
| `(T in c)?`, field of the element type of some collection `H`, `c` not owned by the field's own type | direct side (one-to-many or one-to-one: decided by its reverse side) |
| `Pool(T in c)` | direct side, many-to-many |
| `(T in c)?` on the type that owns `c` | owner field |
| `Pool(c->f)` | reverse side, one-to-many of `f` |
| `(c->f)?` where `f` is another field | reverse side, one-to-one of `f` |
| `(c->f)?` where `f` is this field | symmetric |
| `Pool(in c->f)` | reverse side, many-to-many of `f` |
| `T in c` (not optional), `Array(T in c)`, any other wrapping | `REL002` "a direct side is optional or a `Pool`: it becomes `none` (or loses its target) when the target is deleted" |

5. Pairing: every direct side has **exactly one** reverse side, on the element type of its target collection `c`, naming the holder collection `H` (the collection whose elements carry the direct side) and the field. Errors:
   - `REL001` direct side without reverse side: "`bestFriend` has no reverse side: declare one, for example `bestFriendOf: Pool(people->bestFriend)`";
   - `REL006` two reverse sides for the same direct side;
   - `REL007` reverse shape doesn't match the direct side (`Pool(people->friends)` for a many-to-many: "use `Pool(in people->friends)`"; `Pool(in people->bestFriend)` for an optional direct side);
   - `REL008` the reverse side names a field that is not a direct side, or a collection whose elements don't have that field;
   - `REL009` the reverse side is declared on a type that is not the element type of the direct side's target collection;
   - `REL015` an owner field that is not optional; `REL018` an owner field into a collection of another type;
   - `REL013` a default value or initializer on any relation side (`= []`, `= none`).
6. Module-level collections: when a relation's holder or target collection is module-level, both must be `mutable` (`REL019` "`posts` must be declared `mutable`: deleting a user writes `posts[..].author`").
7. **(decision D17)** A module-level value or a local whose type is a reference type (`mutable first: (Person in people)?` at module level) is `REF010`: only fields can store references, and only as relation sides or owner fields.

Each relation is recorded as:

```rust
struct Relation {
	kind: OneToMany | OneToOne | ManyToMany | Symmetric | OwnerField,
	holder: CollectionRef,   // H, relative to the owner (or module-level)
	direct: FieldId,         // f, on the element type of H
	target: CollectionRef,   // C
	reverse: Option<FieldId>,// r, on the element type of C (None for owner fields; == direct for symmetric)
	owner: Option<TypeId>,   // the type owning H and C, None if module-level
}
```

Element types record their relations (as holder, as target, and owner fields of their own type), so that `delete` can generate the cleanup.

### 8.3 Active and inactive relation fields

A value of an element type `E` has **active** relation fields only when it is reached through a reference typed `E in H` where `H` is the relation's holder (or target) collection **instantiated** with an owner path (`town.people`, or the module-level `people`). Everywhere else (a copy, a local `let`, an element of another collection, an element taken with `take`) relation fields read as `none` / empty and are **read-only**: writing them is `REL011` "relation fields of a copy are `none` and read-only: write them through a reference into `town.people`".

Statically, a relation field access `x.f` gets the type `(T in o.C)?` where `o` is the owner path of `x`'s reference type (substitution of the enclosing-value field names, reference-checking page "Inside a type"); for an inactive value, the type is still `(T in ?)?` but every write is an error and reads produce `none` at runtime **(decision D18:** the runtime representation of an inactive value simply has empty links; no special case in reads).

### 8.4 Writing relations

- Only the direct side is written: `x.f = t` (one-to-many, one-to-one, symmetric), `x.f.add(t)` / `x.f.delete(t)` / `x.f.clear()` (many-to-many pool). The reverse side is read-only: `REL010` "`bestFriendOf` is the reverse side of `bestFriend` and is read-only: write `ana.bestFriend = cleo`".
- `x` must be a reference typed `E in H` (active); `t` must be a reference typed `T in C` with the **same owner** as `H` (`REL012` "`bao` is a person of `town2.people`, `ana` of `town1.people`: a relation links elements of the same town"), or `none`. Module-level collections have one owner.
- `c.add({ ..., bestFriend = ana })`: a literal given to `add` may set direct sides (and, **decision D19**, a many-to-many direct side with an array literal of references `friends = [ana, bao]`); they are linked after the element is stored. Setting a reverse side in a literal is `REL010`.
- Whole-value assignment to a linked element (`ana = Person { name = "Ana" }`) replaces the data fields and **keeps the links**; **(decision D20)** if the right side is a literal that sets a relation field, `REL021` "a whole-value assignment keeps the links: write `ana.bestFriend = ...` separately".
- Relation pools (direct many-to-many sides and reverse `Pool` sides) hold links: `add(target): Boolean` (`false` when already linked), `delete(target): Boolean`, `clear()`, `size`, `includes(target)`, `find(predicate)`, iteration (yields `T in o.C`, no guaranteed order). A reverse side has only the reading methods.
- `use friend = ana.bestFriend` designates the target ([4.2](#42-use)).

### 8.5 Runtime semantics of writes

Let `x.f = t` with old target `o = x.f`. All steps are generated by the compiler; allocation (many-to-many edge) happens before the first write.

```
oneToMany(x, f, r, t):
	o = x.f
	if o == t: return
	if o != none: o.r.unlink(x)
	x.f = t
	if t != none: t.r.link(x)

oneToOne(x, f, r, t):
	o = x.f
	if o == t: return
	if o != none: o.r = none
	if t != none:
		p = t.r                     -- previous holder of t
		if p != none: p.f = none    -- p != x, since o != t
		t.r = x
	x.f = t

symmetric(x, s, t):
	o = x.s
	if o == t: return
	if o != none: o.s = none
	if t != none:
		if t.s != none: t.s.s = none     -- detach t's previous partner (it may be o: already none)
		t.s = x
	x.s = t                              -- self-link t == x: x.s = x

manyToManyAdd(x, f, r, t) -> Boolean:
	if t in x.f: return false
	x.f.link(t); t.r.link(x); return true

manyToManyDelete(x, f, r, t) -> Boolean:
	if t not in x.f: return false
	x.f.unlink(t); t.r.unlink(x); return true

manyToManyClear(x, f, r):
	for t in x.f: t.r.unlink(x)
	x.f.clear()

ownerFieldWrite(o, g, t): o.g = t        -- no reverse side
```

Self-links are allowed (`ana.bestFriend = ana`: Ana is in her own `bestFriendOf`).

**Deletion** `c.delete(e)` (and `take`, and every element of `deleteWhere`, `clear`, and replacing `c`):

```
unlink(e, c):
	for each relation R with target collection C == c (instantiated):
		OneToMany:  for h in e.R.reverse: h.R.direct = none
		OneToOne:   if e.R.reverse != none: e.R.reverse.R.direct = none
		ManyToMany: for h in e.R.reverse: h.R.direct.unlinkWithoutReverse(e)
		Symmetric:  if e.s != none and e.s != e: e.s.s = none
	for each relation R with holder collection H == c (e holds the direct side):
		OneToMany:  if e.f != none: e.f.r.unlink(e)
		OneToOne:   if e.f != none: e.f.r = none
		ManyToMany: for t in e.f: t.r.unlinkWithoutDirect(e)
	for each owner field g of the owner o of c pointing into c:
		if o.g == e: o.g = none
	(then drop e's data and free its slot)
```

Cost O(k) for k links. `deleteWhere` and `clear` run `unlink` per deleted element; when every relation of the collection is internal (holder and target are the same collection, no owner fields) and the whole collection is cleared, the unlinking is skipped. Destroying the owner of both collections skips all unlinking.

**Copies**:

- Copying an owner (`let copiedTown = town`) copies its collections and **relinks** the internal relations to the copies: two passes (copy elements recording old → new, then rewrite every link, owner fields included).
- Copying a single element out (`let anaCopy = ana`), `take`, or adding a value to another collection gives an element **without links**.
- Copying a collection on its own (`let snapshot = town.people`) gives elements without links.
- Equality (`==`) ignores relation fields. **(decision D21)** Printing (`console.log(ana)`, interpolation) omits relation fields (no cycles, no unbounded output).

### 8.6 Implied effects

Computed once per relation; at a use, `H` and `C` are instantiated with the owner path of the operated collection (`town.people`), or are the module-level collections.

| Operation | Declared effect | Implied effects |
| --- | --- | --- |
| `x.f = t`, one-to-many | `mutates x.f` | `mutates C[..].r` |
| `x.f = t`, one-to-one | `mutates x.f` | `mutates C[..].r`, `mutates H[..].f` |
| `x.f = t`, symmetric | `mutates x.f` | `mutates C[..].f` |
| `x.f.add/delete/clear`, many-to-many | `mutates x.f` | `mutates C[..].r` |
| `o.g = t`, owner field | `mutates o.g` | none |
| `c.add(value)` | `mutates c.size` | the implied effects of writing the direct sides the value sets |
| `c.delete(e)`, `c.take(e)` | `invalidates e` | `mutates c.size`; for each relation targeting `c`: `mutates H[..].f`; for each relation held by elements of `c`: `mutates C[..].r`; `mutates o.g` for each owner field into `c` |
| `c.deleteWhere`, `c.clear`, `c = ...`, any `mutates c` | `mutates c` | the implied effects of `delete`, for every element |

Relation sides (reference fields, and the relation pools of many-to-many and reverse sides) are **flat**: a local reference never points into them (`use` and iteration give targets), so none of these implied `mutates` invalidates local references (they only matter for the iteration rule, the alias rule, mutability of the owner, and reactivity).

Implied effects count: for "calling a mutating function requires a `mutable` value" (deleting a user of `blog.users` writes `blog.posts[..].author`, so `blog` must be mutable — it is, since `blog.users` is), for the iteration rule, for the alias rule (with the two allowed overlaps), and for reactivity.

**(decision D47)** When a function receives the collection alone (`users: Pool(Blog.User)`), the implied effect is `owner(users).posts[..].author`, an effect on a place of the **caller**: at the call site, `owner(blog.users)` = `blog`. Inside the callee, it is covered by the declared `mutates users` / `invalidates user` (declared effects cover their implied effects).

### 8.7 Owner fields

- A field of the type that owns a collection, pointing into it (`firstChild: (Node in #nodes)?`). Optional; several may point at the same element; no reverse side; cleared by `delete`.
- **(decision D48)** A collection that owner fields point into is modified **only through a path from its owner** (`self.#nodes.delete(node)`, `list.#nodes`): `REL016` when such a collection is aliased with `use`/`mutable use`, or passed to a parameter of a user function that mutates or invalidates it. Built-in collection operations on the full path are allowed (the compiler knows the owner there and generates the owner-field cleanup at the call site).

### 8.8 Lowering

**JavaScript** (pool of objects is a `Set`; references are the objects):

| Side | Representation |
| --- | --- |
| direct `(T in c)?` | the field holds the target object, or the `none` value |
| reverse one-to-many `Pool(c->f)` | a field holding a JavaScript `Set` of holder objects |
| reverse one-to-one, symmetric | a field holding the object, or `none` |
| direct many-to-many `Pool(T in c)` | a `Set` of targets |
| reverse many-to-many `Pool(in c->f)` | a `Set` of holders |
| owner field | a field holding the object or `none` |

Writes call small runtime helpers (`fa.rel.setOneToMany(x, "bestFriend", "bestFriendOf", t)`, …) or are inlined; the read of a relation pool exposes the `Set` through the Fa relation-pool API (`size`, `includes`, iteration). An element constructed by `add` gets its relation fields initialized (`none`, empty `Set`s) by the generated element constructor; an inactive copy gets the same empty fields, created by `copy_E`, so reads need no check. `delete` calls a generated `unlink_E(e, owner)` then `set.delete(e)`. On a `Columnar` represented by columns, references are slot indices and links store indices (the columnar brief decides the representation; the algorithms are unchanged).

**Native** (`compilation-with-llvm.md`, relations): intrusive links in the elements — one-to-many direct side `{target, prev, next}`, reverse `{head, count}`; one-to-one and symmetric a target pointer on each side; many-to-many edges `{a, b, links in a's list, links in b's list}` from a per-relation slab. Generated `unlink<Shape>`. Relations require stable addresses because links are raw pointers into slots that never move.

### 8.9 Errors

| Code | Condition |
| --- | --- |
| `REL001` | direct side without reverse side |
| `REL002` | direct side neither optional nor `Pool` |
| `REL003` | relation into a collection without stable addresses |
| `REL004` | `T` is not the element type of the collection |
| `REL005` | the collection is not visible (not a field of an enclosing type, not module-level) |
| `REL006` | a second reverse side for the same direct side |
| `REL007` | reverse side shape doesn't match the direct side |
| `REL008` | reverse side names something that isn't a direct side of that collection |
| `REL009` | reverse side on the wrong type |
| `REL010` | writing a reverse side (or setting it in a literal) |
| `REL011` | writing a relation field of an inactive value (copy, other collection) |
| `REL012` | linking elements of two different owners |
| `REL013` | initializer on a relation side |
| `REL015` | owner field not optional |
| `REL016` | collection with owner fields modified other than through its owner |
| `REL018` | owner field into a collection of another owner |
| `REL019` | module-level relation collection not `mutable` |
| `REL021` | whole-value assignment with a literal setting relation fields |

### 8.10 Tests

```fa
type Town = {
	people: Pool(Person) = []
	type Person = {
		name: String
		bestFriend: (Person in people)?
		bestFriendOf: Pool(people->bestFriend)
		mentor: (Person in people)?
		mentee: (people->mentor)?
		friends: Pool(Person in people)
		friendOf: Pool(in people->friends)
		spouse: (people->spouse)?
	}
}
mutable town: Town = {}
use ana = town.people.add({ name = "Ana" })
use bao = town.people.add({ name = "Bao" })
use cleo = town.people.add({ name = "Cleo" })

ana.bestFriend = bao
ana.bestFriend = cleo
console.log(bao.bestFriendOf.size) -- prints: 0
console.log(cleo.bestFriendOf.size) -- prints: 1

ana.friends.add(bao)
console.log(bao.friendOf.size) -- prints: 1
console.log(ana.friends.add(bao)) -- prints: false
ana.friends.delete(bao)
console.log(bao.friendOf.size) -- prints: 0

ana.mentor = bao
cleo.mentor = bao
console.log(bao.mentee?.name ?? "none") -- prints: Cleo
console.log(ana.mentor?.name ?? "none") -- prints: none

ana.spouse = bao
console.log(bao.spouse?.name ?? "none") -- prints: Ana
cleo.spouse = bao
console.log(bao.spouse?.name ?? "none") -- prints: Cleo
console.log(ana.spouse?.name ?? "none") -- prints: none

ana.bestFriend = bao
cleo.friends.add(bao)
town.people.delete(bao)
console.log(ana.bestFriend?.name ?? "none") -- prints: none
console.log(cleo.friends.size) -- prints: 0
console.log(cleo.spouse?.name ?? "none") -- prints: none
```

```fa
-- continuing the previous town
console.log(bao.name) -- error REF001
```

```fa
type Person = {
	name: String
	bestFriend: (Person in people)? -- error REL001
}
mutable people: Pool(Person) = []
```

```fa
mutable people: Array(Person) = []
type Person = {
	name: String
	bestFriend: (Person in people)? -- error REL003
	bestFriendOf: Pool(people->bestFriend)
}
```

```fa
mutable people: Pool(Person) = []
type Person = {
	name: String
	bestFriend: Person in people -- error REL002
	bestFriendOf: Pool(people->bestFriend)
}
```

```fa
mutable people: Pool(Person) = []
type Person = {
	friends: Pool(Person in people)
	friendOf: Pool(people->friends) -- error REL007
}
```

```fa
mutable people: Pool(Person) = []
type Person = {
	bestFriend: (Person in people)?
	bestFriendOf: Pool(people->bestFriend)
	alsoBestFriendOf: Pool(people->bestFriend) -- error REL006
}
```

```fa
mutable people: Pool(Person) = []
type Person = {
	bestFriend: (Person in people)?
	bestFriendOf: Pool(people->bestFriend) = [] -- error REL013
}
```

```fa
-- `town` as above
mutable town: Town = {}
use ana = town.people.add({ name = "Ana" })
use cleo = town.people.add({ name = "Cleo" })
cleo.bestFriendOf.add(ana) -- error REL010
let anaCopy = ana
anaCopy.bestFriend = cleo -- error REL011
console.log(anaCopy.bestFriend?.name ?? "none") -- prints: none (once the errors are removed)
```

```fa
mutable town1: Town = {}
mutable town2: Town = {}
use ana = town1.people.add({ name = "Ana" })
use bao = town2.people.add({ name = "Bao" })
ana.bestFriend = bao -- error REL012
```

```fa
mutable town: Town = {}
use ana = town.people.add({ name = "Ana" })
for friend in ana.friends {
	town.people.delete(friend) -- error REF012
}
for friend in ana.friends {
	friend.name = "{friend.name}!" -- valid: writes an element of `town.people`
}
```

```fa
mutable town: Town = {}
use ana = town.people.add({ name = "Ana" })
use bao = town.people.add({ name = "Bao", bestFriend = ana })
console.log(ana.bestFriendOf.size) -- prints: 1
mutable copiedTown = town
use copyOfBao = copiedTown.people.find((person) => person.name == "Bao")
console.log(copyOfBao?.bestFriend?.name ?? "none") -- prints: Ana
copiedTown.people.clear()
console.log(ana.bestFriendOf.size) -- prints: 1 (the original is untouched)
```

```fa
type Blog = {
	users: Pool(User) = []
	posts: Pool(Post) = []
	type User = { name: String, articles: Pool(posts->author) }
	type Post = { title: String, author: (User in users)? }
}
mutable blog: Blog = {}
use ana = blog.users.add({ name = "Ana" })
use post = blog.posts.add({ title = "Hello" })
post.author = ana
console.log(ana.articles.size) -- prints: 1
blog.users.delete(ana)
console.log(post.author?.name ?? "none") -- prints: none
```

```fa
mutable people: Pool(Person) = []
type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Pool(people->bestFriend)
}
function befriend = (person: Person in people, friend: Person in people) mutates person.bestFriend => {
	person.bestFriend = friend
}
use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })
befriend(ana, bao)
console.log(bao.bestFriendOf.size) -- prints: 1
befriend(ana, ana) -- error REF013
```

```fa
-- the LinkedList of the relations page, verbatim, then:
mutable list: LinkedList(String) = {}
use first = list.add("first")
use second = list.add("second")
list.add("third")
console.log(first.next?.data ?? "") -- prints: second
list.remove(second)
console.log(first.next?.data ?? "") -- prints: third
console.log(first.next?.previous?.data ?? "") -- prints: first
console.log(second.data) -- error REF001
```

```fa
type Bag = {
	#items: Pool(Item) = []
	first: (Item in #items)?
	type Item = { label: String }
	function clearAll = (self) mutates self.#items => {
		use items = self.#items -- error REL016
		items.clear()
	}
}
```

```fa
mutable people: Pool(Person) = []
type Person = { name: String, bestFriend: (Person in people)?, bestFriendOf: Pool(people->bestFriend) }
use ana = people.add({ name = "Ana" })
ana.bestFriend = ana
console.log(ana.bestFriendOf.size) -- prints: 1
people.delete(ana)
console.log(people.size) -- prints: 0
```

---
## 9. Requirements

### 9.1 Syntax

```fa
function greet = () requires session: Session, logger: Logger => { ... }    -- after the return type and `throws`
type Loader = (id: String) => User throws LoadError requires database: Database
provide Session = { user = { name = "Ada", email = "ada@example.com" } }     -- a statement
function addBreadcrumb = (label: String) requires trace: Trace mutates trace => { ... }
```

Parser: `Requires { signature, requirements }` and `Provide { type_expression, expression }`.

### 9.2 Keys

The key of a requirement is the **exact shape** of its type:

- aliases are transparent (`type AppLogger = Logger` is the key of `Logger`);
- two named object types with the same public data fields are the same key; a type with `#private` fields, a named union, an `Errors` type, an `Enum` and a `CStruct` are keyed by declaration;
- generic types are keyed with their arguments (`Cache(User)` ≠ `Cache(Post)`);
- no subtyping: `requires logger: Logger` is satisfied only by a provided `Logger`-shaped key.

`keyOf(type) = internedShape(resolveAliases(type))`. **(decision D11)** For the ABI order of hidden parameters, keys are sorted by a **stable content hash of the shape** (field names and field type hashes, recursively; declaration identity for nominal types), not by `TypeId`, whose numbering depends on the order of analysis.

### 9.3 Resolution at a call

`provide` frames live in the scope stack. `provide K = e` adds `K → provider` to the frame of the **current block**, visible from the statement to the end of the block (a second `provide K` in the same block replaces it for the rest of the block; a `provide` in a branch of `if`/`when` ends with the branch). For each requirement key `K` of a callee, at the call:

1. the innermost visible `provide K` → provider = its place (an alias) or its temporary;
2. else, if the current function declares `requires k: K` → provider = its own hidden parameter (root `Requirement(K)`);
3. else, if the current function has a single-expression body → `K` is added to its **inferred** requirements, provider = own hidden parameter;
4. else, inside a lambda checked against a **non-stored** parameter type: resolve in the enclosing context where the lambda is written (its requirements are requirements of the call);
5. else → `REQ001` "`greet` requires `Session` and `Logger`, which are not provided. Use `provide Session = ...` or declare `requires session: Session` in `main`." (all missing keys of the call in one diagnostic).

Contexts that can't declare requirements (they provide them): the entry function, the top-level statements of a standalone file and of a script, tests, module-level initializers (which can't provide either: a requiring call in an initializer is `REQ001`). Declaring `requires` on the entry function, a worker method, an `@extern` function or a function of a `cLibrary` API is `REQ002`/`REQ006`.

### 9.4 `provide`

- `provide K = place` provides an **alias** of the place (like `use`): no copy; the provided value is mutable iff the place is.
- `provide K = expression` provides a **temporary** owned by the block, destroyed at its end. **(decision D12)** A temporary provider is immutable: a callee that `mutates` the requirement needs a `mutable` place (`REQ004` "`addBreadcrumb` mutates `Trace`, but the provided value is a temporary: declare it with `mutable trace = ...`, then `provide Trace = trace`").
- The value only needs to satisfy `K` structurally (a value with more fields is seen through `K`, without copy). An object literal must have exactly the fields of `K` (`STR002`).
- A provided place is a **use** of the reference it designates (valid at the `provide`); while provided, it is used at each call that receives it.

### 9.5 Requirements as places and effects

- Inside the function, a requirement name is a root of paths: `mutates trace`, `use first = trace.breadcrumbs[0]`.
- At a call, the callee's effects on a requirement root are substituted with the provider's place ([3.5](#35-substitution-at-a-call)): `addBreadcrumb("start")` with `provide Trace = trace` is `mutates trace` in the caller, which invalidates `use first = trace.breadcrumbs[0]`.
- A caller that doesn't provide a mutated requirement must declare both `requires trace: Trace` and `mutates trace` (block body; inferred for a single expression).
- The alias rule: a place passed to a mutated parameter can't overlap a requirement provider the callee uses (and a mutated requirement can't overlap another argument).
- **(decision D13)** A requirement of a primitive type can't be mutated (`EFF005`), like a parameter. So no requirement ever needs a wrapper in JavaScript.

### 9.6 Function values, closures, watch blocks

- Requirements are never captured by a closure. A stored function's type must declare its requirements (they are resolved where it is eventually called); a function can be stored in a function type only if the type declares every requirement it needs (`REQ007`, a case of `EFF015`).
- A lambda passed to a non-stored parameter has its requirements resolved where it is written.
- A `watch` block keeps the providers visible where it is declared, and can't outlive the block that provides them: moving its `Subscription` out of that block is `REQ010`.
- A `derived` can't use requirements (`REQ005`: it would not be pure).
- `for parallel` bodies may read requirements, not mutate them unless thread-safe (`CONC012`). Workers: none.

### 9.7 Lowering

- **Native**: one hidden pointer parameter per requirement key, appended after the regular parameters, in the canonical key order (D11). A provided place passes its address; a temporary is a stack slot destroyed at the end of the block. Function types with requirements carry the hidden parameters in their ABI.
- **JavaScript**: extra trailing arguments in the same order. A provided place is passed as its JavaScript value (an object is shared by reference; a primitive is passed by value, which is exact because a primitive requirement can't be mutated, D13). A provided temporary is a `const` local.

```fa
function greet = () requires session: Session, logger: Logger => {
	logger.info("Hello, {session.user.name}!")
}
function main = () => {
	provide Session = { user = { name = "Ada", email = "ada@example.com" } }
	provide Logger = { info = (message) => console.log(message) }
	greet()
}
```

```js
// keys sorted by stable hash, say Logger < Session
function greet(logger, session) {
	logger.info(`Hello, ${session.user.name}!`)
}
function main() {
	const session$p = { user: { name: "Ada", email: "ada@example.com" } }
	const logger$p = { info: (message) => console.log(message) }
	greet(logger$p, session$p)
}
```

### 9.8 Errors

| Code | Condition |
| --- | --- |
| `REQ001` | a call's requirement is neither provided nor declared (block body) |
| `REQ002` | `requires` on an entry function, a test, a standalone file's top level |
| `REQ003` | the provided value doesn't satisfy the key |
| `REQ004` | a mutated requirement whose provider is immutable (immutable place or temporary) |
| `REQ005` | a `derived` uses a requirement |
| `REQ006` | `requires` on a worker method, an `@extern` function, a `cLibrary` API function |
| `REQ007` | a function stored in a function type that doesn't declare its requirements |
| `REQ010` | a watch outlives the block that provides its requirements |

### 9.9 Tests

```fa
type Session = { user: { name: String } }
type Logger = { info: (message: String) => Void }
function greet = () requires session: Session, logger: Logger => {
	logger.info("Hello, {session.user.name}!")
}
provide Logger = { info = (message) => console.log(message) }
provide Session = { user = { name = "John Doe" } }
do {
	provide Session = { user = { name = "Jane Smith" } }
	greet() -- prints: Hello, Jane Smith!
}
greet() -- prints: Hello, John Doe!
```

```fa
function greet = () requires logger: Logger => logger.info("hi")
function main = () => {
	greet() -- error REQ001
}
```

```fa
function welcome = () => greet() -- valid: single expression, `requires logger: Logger` is inferred
function caller = () => {
	welcome() -- error REQ001
}
```

```fa
type UserSession = { user: { name: String } }
type RequestInfo = { requestId: String }
function logRequest = () requires request: RequestInfo => console.log("Request {request.requestId}")
function greetUser = () requires session: UserSession => console.log("Hello, {session.user.name}!")
provide UserSession = { user = { name = "Ana" } }
provide RequestInfo = { requestId = "abc-123" }
logRequest() -- prints: Request abc-123
greetUser() -- prints: Hello, Ana!
```

```fa
type Source = { path: String }
type Destination = { path: String }
function show = () requires source: Source => console.log(source.path)
provide Source = { path = "a" }
provide Destination = { path = "b" } -- same shape: replaces `Source`
show() -- prints: b
```

```fa
type Logger = { info: (message: String) => Void }
type VerboseLogger = { info: (message: String) => Void, debug: (message: String) => Void }
function log = () requires logger: Logger => logger.info("x")
let verbose: VerboseLogger = { info = (message) => console.log(message), debug = (message) => console.log(message) }
provide VerboseLogger = verbose
log() -- error REQ001 (no lookup by subtype)
provide Logger = verbose
log() -- prints: x
```

```fa
type Trace = { breadcrumbs: Array(String) }
function addBreadcrumb = (label: String) requires trace: Trace mutates trace => {
	trace.breadcrumbs.push(label)
}
function main = () => {
	mutable trace: Trace = { breadcrumbs = [] }
	provide Trace = trace
	use first = trace.breadcrumbs.get(0)
	addBreadcrumb("start")
	addBreadcrumb("loaded")
	console.log(trace.breadcrumbs) -- prints: ["start", "loaded"]
	console.log(first) -- error REF001
}
```

```fa
function main = () => {
	provide Trace = { breadcrumbs = [] }
	addBreadcrumb("start") -- error REQ004
}
```

```fa
function middle = () requires trace: Trace => {
	addBreadcrumb("x") -- error EFF001 (`addBreadcrumb` mutates `trace`: declare `mutates trace`)
}
```

```fa
export function = () requires logger: Logger => {} -- error REQ002 (in application/main.fa)
```

```fa
type Loader = (id: String) => String requires database: Database
type Database = { find: (id: String) => String }
function loadAll = (ids: Array(String), load: Loader): Array(String) requires database: Database => ids.map((id) => load(id))
provide Database = { find = (id) => "user {id}" }
function fromDatabase = (id: String): String requires database: Database => database.find(id)
console.log(loadAll(["1"], fromDatabase)) -- prints: ["user 1"]
```

```fa
type Settings = { factor: Integer }
function factor = (): Integer requires settings: Settings => settings.factor
provide Settings = { factor = 2 }
reactive count = 0
derived scaled = count * factor() -- error REQ005
```

```fa
function inc = (counter: { value: Integer }) requires total: { value: Integer } mutates counter, total => {
	counter.value += 1
	total.value += 1
}
mutable total = { value = 0 }
provide { value: Integer } = total
inc(total) -- error REF013 (the argument overlaps the requirement)
```

---

## 10. Structural typing

The types brief owns type syntax and inference; this section owns the rules of the structural-typing page and their lowering.

### 10.1 Identity

- Object types are compared by their **public data fields** (names, types, `readonly`, `reactive`). Methods and default values are not compared; they belong to the type **written at the use site** (method calls resolve statically on the written type; in generic code, on the actual type).
- A type with `#private` fields is nominal (distinct from every other type). Named unions and their members, `Errors`, `Enum`, `CStruct` are nominal. `Fields` types are structural.
- `TypeStore` interns object types by shape: `Human` and `Animal` are one `TypeId`; the declared name is recorded at each use for diagnostics, `@typeToString` and `@typeToSchema`. Recursive shapes are interned as cycles and compared coinductively.
- A field and a method with the same name in one type: `STR008`.

### 10.2 Satisfaction (`S` satisfies `D`)

`S` satisfies object type `D` iff for every field `f: Tf` of `D`, `S` has `f` with a type satisfying `Tf` for reading (covariant), the same `readonly`-ness or a writable field where `D`'s is `readonly` (a `readonly` field of `S` doesn't satisfy a writable field of `D` that is **written**), and the same `reactive`-ness or a reactive field where `D` has a plain one (a value with reactive fields is accepted through a plain view; the reads and writes through it are tracked). Missing field → `STR001` "`Comment` has no field `id: Integer`". An existing value missing a field is never accepted, even an optional one (`STR003` "a `Human` has no `email` field").

### 10.3 Object literals

A literal written where a type is expected must have exactly the fields of that type: an extra field is `STR002` "`z` is not a field of `Point`". It may omit a field with a default value or whose type accepts `none`. Where the expected type is a union, the literal must match exactly one member (`STR010`). Where it is a generic parameter, the literal gives the parameter its full type. The same applies to literals passed to functions, added to collections (`points.add({ x = 1, y = 2, z = 3 })` is `STR002`), provided, or returned.

### 10.4 Copies narrow, references don't

- `let human: Human = superHuman` copies `Human`'s fields only (a narrowing copy). Moving is possible only into the same shape.
- `use human: Human = superHuman` aliases; `human.powers` is a type error (`powers` is not part of `Human`); nothing is dropped.
- Parameters are references: passing a `SuperHuman` to a `Human` parameter copies nothing.
- A parameter keeps the type of its argument, `Type(parameter)`: `mutable copy = human` (no annotation) copies every field of the argument; `function same = (human: Human) => human` returns `Type(human)`. `use` aliases and loop variables keep the type of what they designate. `Type(human)` is a hidden generic parameter.
- A collection stores the layout of its element type: `humans.push(superHuman)` narrows; `let copied: Array(Human) = superHumans` copies every element narrowed.

### 10.5 Mutation through a narrower type

- A function that **mutates** a parameter receives only arguments whose **written** fields (the fields named by its `mutates` paths, down to the fields actually written) have **exactly** the type it writes: `STR004` "`penalize` writes `score: Integer` into `player.score`, an `Integer(min = 0, max = 10)`". Reading is covariant, writing is invariant. Fields not written are free.
- A `readonly` field is never accepted where a function writes that field (`STR007`).
- A collection of supersets is accepted by a parameter that is only read; a **mutated** collection parameter needs exactly its element type (`STR005` "`addJohn` mutates `humans`, so it needs an `Array(Human)`: make it generic").
- `Set` elements and `Map` keys are invariant even when read: a `Set(Hero)` is never a `Set(Human)` (`STR006`). Map values follow the general rule.
- `==` compares the fields of the **declared** type of its operands (two `Human` parameters are equal when their `name` is).

### 10.6 Errors

| Code | Condition |
| --- | --- |
| `STR001` | a value lacks a field of the expected type |
| `STR002` | an object literal has a field the expected type doesn't have |
| `STR003` | an existing value lacks an optional field |
| `STR004` | a mutated parameter's written field has a different type in the argument |
| `STR005` | a mutated collection parameter receives a collection of supersets |
| `STR006` | `Set`/`Map` key types of different shapes |
| `STR007` | a `readonly` field passed where it is written |
| `STR008` | a field and a method with the same name |
| `STR009` | a nominal type (private fields, named union) given where another is expected |
| `STR010` | a literal matching zero or several union members |

### 10.7 Lowering

**Native** (`compilation-with-llvm.md`, structural typing): canonical layout (alignment descending, then name); **specialization by view**: a direct call whose argument has a different view gets an instance whose key is `(function, view of each structural parameter, generic arguments, names reflected on)`; a parameter that needs the full type (unannotated copy, `Type(parameter)` return, reflection) is keyed by its full type (summary bit, [2.3](#23-function-summaries)); deduplication by code hash; function values get a view entry point with a static table `{offsets, stride, size, align, copy, drop}` per (actual, declared) pair; a narrowing copy calls `copy<Source → Destination>`, never a `memcpy` of a prefix; read-only collections of supersets specialize on stride and offsets.

**JavaScript**: objects are dynamic; a view is a property read; nothing is specialized, except:

- a narrowing copy calls `copy_Destination(value)` (reads only the destination's fields; never a spread);
- a function whose summary says it needs the full argument type (an unannotated copy of a parameter, or a reflection on it) is emitted **once per argument type** (`renamed$SuperHuman`, `renamed$Human`) with the right copy function; a function value of such a function receives the copy function of the actual argument as a hidden argument (LLVM Q16);
- a function whose argument may have reactive fields gets per-reactiveness instances ([11.6](#116-lowering)).

### 10.8 Tests

```fa
type Human = { name: String }
type Animal = { name: String }
let animal = Animal { name = "Zeus" }
let human: Human = animal
function logAnimalName = (animal: Animal) => console.log(animal.name)
logAnimalName(human) -- prints: Zeus
```

```fa
type Human = {
	name: String
	function describe = (self): String => "a human named {self.name}"
}
type Animal = {
	name: String
	function describe = (self): String => "an animal named {self.name}"
}
let animal = Animal { name = "Zeus" }
let human: Human = animal
console.log(human.describe()) -- prints: a human named Zeus
console.log(animal.describe()) -- prints: an animal named Zeus
```

```fa
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
let superHuman = SuperHuman { name = "John", powers = "unlimited" }
let human: Human = superHuman
console.log(human) -- prints: { name = "John" }
use view: Human = superHuman
console.log(view.name) -- prints: John
console.log(view.powers) -- error (unknown field `powers` on `Human`, types brief)
```

```fa
type Point = { x: Integer, y: Integer }
let point: Point = { x = 12, y = 13, z = 14 } -- error STR002
```

```fa
type Contact = { name: String, email: String? }
type Human = { name: String }
let contact: Contact = { name = "Ana" }
console.log(contact.email ?? "none") -- prints: none
let human = Human { name = "Bao" }
let other: Contact = human -- error STR003
```

```fa
type Scored = { score: Integer }
type Player = { name: String, score: Integer(min = 0, max = 10) }
function penalize = (scored: Scored) mutates scored => {
	scored.score = -5
}
mutable player = Player { name = "Ana", score = 3 }
penalize(player) -- error STR004
function rename = (named: { name: String }) mutates named.name => {
	named.name = "Bo"
}
rename(player) -- valid: `score` is not written
console.log(player.name) -- prints: Bo
```

```fa
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
function logNames = (humans: Array(Human)) => {
	for human in humans {
		console.log(human.name)
	}
}
let superHumans = [SuperHuman { name = "A", powers = "x" }]
logNames(superHumans) -- prints: A
function addJohn = (humans: Array(Human)) mutates humans => {
	humans.push({ name = "John" })
}
mutable heroes = [SuperHuman { name = "A", powers = "x" }]
addJohn(heroes) -- error STR005
function keepNamed(Item: Human) = (items: Array(Item)) mutates items => {
	items.deleteWhere((item) => item.name == "")
}
keepNamed(heroes) -- valid
```

```fa
function countTags = (tags: Set(Human)) => tags.size
let heroes: Set(SuperHuman) = Set.from([SuperHuman { name = "A", powers = "x" }])
countTags(heroes) -- error STR006
```

```fa
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
function renamed = (human: Human, name: String): Type(human) => {
	mutable copy = human
	copy.name = name
	copy
}
let clark = renamed(SuperHuman { name = "Kal", powers = "flight" }, "Clark")
console.log(clark.powers) -- prints: flight
let john = renamed(Human { name = "Ana" }, "John")
console.log(john) -- prints: { name = "John" }
```

```fa
function readId(Item: { id: Integer }) = (item: Item): Integer => item.id
type Comment = { text: String }
console.log(readId({ id = 1, name = "Ana" })) -- prints: 1
readId(Comment { text = "Hello" }) -- error STR001
```

```fa
type UserId = { user: Integer }
type ProductId = { product: Integer }
let userId = UserId { user = 12 }
let productId: ProductId = userId -- error STR001
type A = Union { User = Integer }
type B = Union { Product = Integer }
let a: A = A.User(1)
let b: B = a -- error STR009
```

```fa
type Shape = { width: Float, function area = (self): Float => self.width }
type Box = { width: Float, area: Float } -- valid (different types)
type Bad = { area: Float, function area = (self): Float => 1.0 } -- error STR008
```

---

## 11. Reactivity

### 11.1 Syntax

```fa
reactive number = 3                 -- Reactive { name, type_expression, expression }
reactive count: Integer = 0
derived multiplied = number * 2     -- Derived { ... }
watch { console.log(multiplied) }   -- Watch { body }
let subscription = watch { ... }    -- a watch used as an expression
type Counter = { label: String, reactive count: Integer }  -- a reactive field
```

### 11.2 Values and places

- `reactive x = e` declares a **mutable** place that can be observed (assignable without `mutable`). Allowed in a function body and at the top of a module (and as namespace members); not with `export` (`REA005`).
- `derived x = e` declares a read-only value computed from reactive values. Writing it is `REA010`.
- Reactiveness is a property of **places**, not of types: `reactive count = 0` is an `Integer`. A copy (`let snapshot = count`) is plain. A `use` alias of a reactive place and a parameter that receives one are tracked (reads) and notify (writes). A function declared with `mutates numbers` that pushes into a reactive array runs the watches that read it.
- A reactive field is part of the field, like `readonly`: `{ reactive count: Integer }` ≠ `{ count: Integer }`; a value with reactive fields satisfies the plain shape (tracked through the view). Writing a reactive field needs a `mutable` object (`EFF012`). A copy of an object keeps its reactive fields, with no dependents.
- Fine-grained dependencies: a read depends on the **path** it reads (`numbers.length`, `numbers[2]`, `settings.database.connections`); reading a whole value (iteration, interpolation, copy) depends on all of it. A write changes the path it writes and everything below; adding/removing elements also changes the length/size. Assigning a flat value equal to the current one changes nothing.

### 11.3 Static semantics

- **`derived` purity** (`REA002`): the expression must be pure (no throw, no `mutates`/`invalidates` except its own locals, no requirements, no I/O including `console`, no read of a module-level `mutable` value that isn't reactive). "a derived value must be pure".
- **Captures**: a `derived` and a `watch` refer to the reactive values they read and **copy** every other local value they use, when declared (escaping-closure semantics, [5.11](#511-closures)): writing a captured non-reactive local inside a watch is `REF021`.
- **Derived cycles** (`REA001` "cyclic dependency between `a` and `b`"): computed from the static may-read sets over **every branch** and every called function. In the walk: a `derived` declaration is analyzed on demand; reading a derived whose declaration is in progress (directly, or through a function whose summary reads it) is the cycle.
- **A watch writes what it reads** (`REA003` "this `watch` writes `count`, which it reads: it would run forever"): the static may-write set (assignments, callee `mutates` substituted, implied effects) overlaps (path overlap, [5.4](#54-identities-and-may-designate)) the static may-read set (reads, arguments of calls, callees' reactive read sets) of the same watch. Fine-grained: reading `numbers.length` and writing `numbers[0]` don't overlap.
- **A watch can't throw** (`REA004`): an error that may escape the body must be handled with `catch`. A watch is not an async context.
- **Cleanup**: when the last expression of the body is a lambda (`() => Void`), it is the cleanup.
- **Subscriptions**: `watch { }` used as an expression has type `Subscription` (decision D25), move-only; `stop()` consumes it (`MEM002` on a later use). Without a binding, the watch lives to the end of its scope; at module level, for the program.
- **Threads**: reactive and derived values can't be read or written in a `for parallel` body or a `parallel()` callback (`REA007`); a worker receives a plain copy.
- A `watch` captures the requirement providers visible where it is declared ([9.6](#96-function-values-closures-watch-blocks)).

### 11.4 Runtime semantics

Push-pull signals, the same runtime library on every target (reactivity page, implementation section):

- **Sources**: each reactive place has a version and observers; a reactive object or collection has one source for the whole value and, lazily, sources for its length/size, for each element or key read individually, for each field. A write bumps the source it writes, the sources below, the length on add/remove, and the whole value.
- **Derived** nodes: cached value, `dirty`, recorded sources with versions. Reading a dirty derived checks the versions (pulling derived ones recursively) and recomputes only if one changed: always fresh, computed at most once per change.
- **Watch** nodes: dependencies, cleanup, sequence number (declaration order).
- **Push**: `notify` walks observers; derived → `dirty`; watch → pending queue (once).
- **Flush**: run pending watches in declaration order; each run calls the previous cleanup, then the body under tracking. Repeat while new watches become pending; after **100 rounds**, the program **stops** (exit code 101).
- **When**: after each statement of the **outermost running code** (the entry function, a test, a watch run, a platform callback, a top-level statement of a standalone file or script). A `do` block is one statement; a call is part of the caller's statement. (Before an `await` too, out of scope here.)

### 11.5 Flush placement

The compiler emits `flush()` after each statement that **may write a reactive place** (summary bit "may write reactive", transitive through the instances it calls). A thread-local depth counter is incremented around calls of instances that may write reactive places, so that `flush()` does nothing except at depth 0; entry points start at depth 0. Code that never touches a reactive place is compiled as if reactivity didn't exist.

### 11.6 Lowering

Values stay plain; sources are side objects, so nothing boxes the value itself on hot paths.

**JavaScript** (runtime module `fa/reactive`, tree-shaken):

```fa
let multiplier = 2
reactive number = 3
derived multiplied = number * multiplier
watch {
	console.log("Multiplied is {multiplied}")
}
number = 5
```

```js
import { source, track, notify, derived, read, watch, flush } from "./fa/reactive.js"
const multiplier = 2
let number = 3
const number$s = source()
const multiplied = derived(() => (track(number$s), number) * multiplier)
const watch$1 = watch(1, () => { console.log(`Multiplied is ${read(multiplied)}`) })
if (number !== 5) { number = 5; notify(number$s) }
flush()
```

- A reactive collection or object: the value is the plain JavaScript value; its source tree `x$s` creates child sources on demand: `track(x$s.index(2))`, `track(x$s.length)`, `notify(x$s.field("count"))`. Collection mutators are followed by the notifications their effects imply (`push`: the length and the whole value).
- A reactive field of an object type: a hidden property `count$s` created by the constructor of that type; reads and writes through a field typed `reactive` are instrumented.
- **Instances by reactiveness**: a function receiving a reactive place (or a value with reactive fields through a plain view) gets an instance compiled for it, with a hidden source argument per reactive parameter (`push$r(array, array$s, value)`). **(decision D24)** This is the one place, with `Type(parameter)`, where the JavaScript backend emits several instances of a function.
- `watch` returns a node; at every exit of its scope the compiler emits `dispose(watch$1)` (and in `finally` for errors). `subscription.stop()` is `dispose`.
- `derived` nodes are disposed the same way at the end of their scope.

**Native**: the same runtime, written in Fa over the standard library (nodes in pools owned by the thread's reactive context; thread-local current observer and depth counter); `dispose` is the ordinary destruction at scope exit.

### 11.7 Errors

| Code | Condition |
| --- | --- |
| `REA001` | cycle between derived values (static, all branches) |
| `REA002` | impure `derived` expression |
| `REA003` | a watch may write a place it may read |
| `REA004` | an error may escape a watch |
| `REA005` | `export reactive`, `export derived` |
| `REA007` | reactive value in a parallel body or callback |
| `REA010` | writing a `derived` value |

### 11.8 Tests

```fa
let multiplier = 2
reactive number = 3
derived multiplied = number * multiplier
watch {
	console.log("Multiplied is {multiplied}")
}
number = 5
-- prints: Multiplied is 6
-- prints: Multiplied is 10
```

```fa
reactive numbers = [1, 2, 3]
watch {
	console.log("Numbers are {numbers}")
}
numbers.push(4)
-- prints: Numbers are [1, 2, 3]
-- prints: Numbers are [1, 2, 3, 4]
```

```fa
reactive numbers = [1, 2, 3]
watch {
	console.log("Length {numbers.length}")
}
numbers[0] = 10 -- the length doesn't change: the watch doesn't run
numbers.push(4)
-- prints: Length 3
-- prints: Length 4
```

```fa
reactive width = 10
reactive height = 20
derived area = width * height
watch {
	console.log("Area: {area}")
}
do {
	width = 30
	height = 40
	assert area == 1200
}
-- prints: Area: 200
-- prints: Area: 1200
```

```fa
reactive count = 1
watch {
	console.log(count)
}
count = 1 -- equal flat value: nothing changes
count = 2
-- prints: 1
-- prints: 2
```

```fa
reactive firstName = "Ada"
reactive lastName = "Lovelace"
reactive showFull = false
derived label = if showFull { "{firstName} {lastName}" } else { firstName }
watch {
	console.log(label)
}
lastName = "Byron" -- `label` didn't read `lastName`: nothing runs
showFull = true
-- prints: Ada
-- prints: Ada Byron
```

```fa
type Counter = { label: String, reactive count: Integer }
mutable counter = Counter { label = "clicks", count = 1 }
derived next = counter.count + 1
counter.count = 5
assert next == 6
console.log(next) -- prints: 6
let frozen = Counter { label = "x", count = 0 }
frozen.count = 1 -- error EFF012
```

```fa
reactive channelName = "general"
function joinChannel = (name: String) => console.log("join {name}")
function leaveChannel = (name: String) => console.log("leave {name}")
watch {
	joinChannel(channelName)
	() => leaveChannel(channelName)
}
channelName = "random"
-- prints: join general
-- prints: leave general
-- prints: join random
```

```fa
reactive count = 0
let subscription = watch {
	console.log(count)
}
subscription.stop()
count = 1
subscription.stop() -- error MEM002
-- prints: 0
```

```fa
function bump = (numbers: Array(Integer)) mutates numbers => numbers.push(0)
reactive values = [1]
watch {
	console.log(values.length)
}
bump(values) -- a parameter receiving a reactive place notifies
-- prints: 1
-- prints: 2
```

```fa
derived a = b + 1 -- error REA001
derived b = a + 1
```

```fa
reactive items = [1, 2]
derived loggedTotal = do { -- error REA002
	console.log("computing the total")
	items.length
}
```

```fa
reactive count = 0
watch {
	count = count + 1 -- error REA003
}
```

```fa
reactive source = "1"
watch {
	let value = try Integer.parse(source) -- error REA004
}
watch {
	let value = Integer.parse(source) catch error => 0 -- valid
}
```

```fa
export reactive count = 0 -- error REA005
```

```fa
reactive a = 0
reactive b = 0
watch { b = a + 1 } -- valid: writes b, reads a; runs at its declaration: b = 1
watch { console.log(b) } -- runs at its declaration
a = 5 -- the first watch runs (b = 6), which makes the second pending in the same flush
-- prints: 1
-- prints: 6
```

```fa
reactive count = 0
watch { count = count + 1 } -- error REA003
reactive ping = 0
reactive pong = 0
watch { pong = ping + 1 }
watch { ping = pong + 1 } -- valid statically (each watch writes what the other reads)
-- the watches schedule each other forever: the program stops after 100 rounds
-- exit: 101
```

---

## 12. Metaprogramming and the interpreter

### 12.1 Syntax summary

| Syntax | Node |
| --- | --- |
| `static name = expression`, `static name: Type = expression` | `Static` |
| `@f(arguments)` | a compile-time call (a `FunctionCall` whose callee is marked compile-time) |
| `@if condition { } @else { }` | `If { is_compile_time: true }` |
| `@for item in items { }` | `For { is_compile_time: true }` |
| `@readFile(path)`, `@readDirectory(path)`, `@parseJson(text)`, `@typeToString(T)`, `@typeToSchema(T)`, `@import(url)`, `@extern(symbol)` | built-in compile-time operations |
| `@test "name" { }` | `Test { is_compile_time: true }` |
| `Type(value)` | type function (no `@`) |

`@` is never used on a value: `@version` is `META012` "a static value is used plainly: write `version`".

### 12.2 Static values

- Computed by the interpreter, then every use is replaced by the value (constant folding into the output: `static foo = 12; console.log(foo)` emits `console.log(12)`). Not present in the output as a binding.
- Allowed at module level (not exported: `META011`/`MOD011`; namespace members allowed) and in function bodies.
- Provided by the compiler: `target: Target` (the target being compiled) and `configuration: PackageConfiguration` (merged for the target). Reading them marks the summary "depends on target" (cache key, [21](#21-compilation-units-caching-incrementality)).
- The type of a static value is its **exact** compile-time type (a literal type, an object type of literal fields), so that `@if settings.debug` and `@for fruit in fruits` work.

### 12.3 Compile-time calls

- Arguments must be compile-time known: literals, static values, compile-time (lowercase generic) parameters, other compile-time operations, local values of a type function → else `META001` "`count` is not known at compile time".
- The callee (and everything it calls) must be **pure**, except that it may throw: `META002` "`now` is not deterministic" naming the impure primitive reached (I/O, `console`, clock, randomness, module-level `mutable`/reactive reads, requirements, `mutates` outside its locals, C calls).
- An error thrown at compile time is `META005` at the `@` call, showing the error's `toString()`.
- Bounds: more than 100 million evaluation steps in one `@` call → `META003`; more than 256 nested type-function applications → `META004`. **(decision D26)** An evaluation step is one evaluated expression node of the checked tree.
- **(decision D49)** Arithmetic is the **target's** arithmetic: an `Integer` overflow at compile time while compiling for a JavaScript target (beyond 2^53 − 1) is `META013`, a compile error at the `@` call (a stop at compile time).
- Results are memoized per (function, arguments) during a build, and cached across builds by the hash of the code, the arguments and the contents of the files read.

### 12.4 `@if` and `@for`

- `@if`: the condition is a compile-time `Boolean` (`META006`). **(decision D22)** Only the taken branch is **analyzed** (and compiled); the other is only parsed. So the analysis of a file depends on the target when it reads `target`/`configuration`; `fa check` analyzes each configured target ([20](#20-the-fa-command-line)), sharing every declaration whose summary doesn't depend on the target.
- `@for x in e`: `e` is compile-time (`META007`): a static array, a range with compile-time bounds, a key set (leaves in declaration order, `key` printed with dots). The body is **unrolled**: analyzed once per item, with `x` bound as a static value.
- Bodies that depend on a compile-time parameter are analyzed per instantiation; an error is reported at the call that gave the parameter.
- At module level, `@if`/`@for` may surround declarations. **(decision D27)** A module-level `@for` may only contain declarations that bind no name (`watch`, and `test` / `@test` in test files, whose names may interpolate static values): a named declaration would be declared once per iteration (`MOD018`).

### 12.5 The file system and other built-ins

| Operation | Type | Rules |
| --- | --- | --- |
| `@readFile(path: String)` | `String` (exact) | path relative to the folder of the current file; must stay inside the root (package folder, workspace folder, or the standalone file's folder): `META008`; missing: `META009`. The file becomes a build input. |
| `@readDirectory(path)` | `Array(DirectoryEntry)`, `DirectoryEntry = { name: String, path: String, isDirectory: Boolean }` (decision D28), sorted by name | `path` of an entry is relative to the same folder as the argument, so `@readFile(file.path)` works; the listing is a build input |
| `@parseJson(text)` | the exact type of the JSON value (objects → object types with those keys, arrays → arrays of the join of their elements, integral numbers → `Integer`, others → `Float`) | malformed: `META005` |
| `@typeToString(T)` | `String` | the name written at the use site; `Person in persons` for a reference |
| `@typeToSchema(T)` | `Schema` | per types-at-runtime |
| `@import("...")` | a namespace | string literal, non-local source (Git, remote file or folder): `META010`; resolved and locked like a dependency ([17](#17-dependencies)) |
| `@extern("symbol")` | the declared function type | foreign files only ([13](#13-foreign-code)) |

### 12.6 The interpreter

- A tree-walking interpreter over the **checked tree**, in `fa-analyzer` (it needs types and resolutions) or a crate it depends on. Values: scalars with the target's semantics, strings, objects, collections implemented natively in Rust (not through the standard library's intrinsics), closures.
- It runs: `static` initializers, `@` calls, `@if` conditions, `@for` iterables, type functions, `@test` blocks, configuration files ([16.3](#163-configuration-files)) and `DependencySource.generate` ([17](#17-dependencies)).
- **(decision D6)** It does **not** run programs: `fa run` compiles to the `bun` target and runs the output (see the decision for the reasons). The interpreter therefore needs no I/O beyond `@readFile`/`@readDirectory` and the lockfile-backed `@import`.
- Determinism: no clock, randomness, environment variables, network; the files read and the remote content (through the lockfile) are the only inputs.

### 12.7 Errors

| Code | Condition |
| --- | --- |
| `META001` | argument of a compile-time call not known at compile time |
| `META002` | impure function called at compile time |
| `META003` | step budget exceeded |
| `META004` | type-function nesting exceeded |
| `META005` | an error thrown at compile time (including malformed JSON) |
| `META006` | `@if` condition not a compile-time `Boolean` |
| `META007` | `@for` iterable not compile-time |
| `META008` | file read outside the root |
| `META009` | file or folder not found |
| `META010` | `@import` with a non-literal or local source |
| `META011` | `export static` |
| `META012` | `@` on a value |
| `META013` | overflow during compile-time evaluation |

### 12.8 Tests

```fa
static foo = 12
console.log(foo) -- prints: 12 (and the emitted JavaScript contains `console.log(12)`)
```

```fa
function add = (x: Integer, y: Integer): Integer => x + y
let result = @add(2, 4)
console.log(result) -- prints: 6
function readInput = (): Integer => 3
let count = readInput()
let total = @add(count, 4) -- error META001
```

```fa
function stamp = (): String => "{std.time.now()}"
static timestamp = @stamp() -- error META002
```

```fa
static fruits = ["apple", "banana", "cherry"]
@for fruit in fruits {
	console.log("Fruit: {fruit}")
}
-- prints: Fruit: apple
-- prints: Fruit: banana
-- prints: Fruit: cherry
```

```fa
function repeatText(count: Integer(min = 0)) = (value: String): String => {
	mutable result = ""
	@for 0..count {
		result += value
	}
	result
}
console.log(repeatText(3)("ab")) -- prints: ababab
```

```fa
@if target is "native" {
	console.log("native")
} @else {
	console.log("other")
}
-- prints: other   (under `fa run`, which runs on bun)
```

```fa
@if target is "native" {
	for parallel item in [1, 2] { } -- not analyzed when compiling for `bun`: no error
}
```

```fa
@if 1 + 1 { } -- error META006
```

```fa
-- with version.txt containing "1.2.3" next to the file
static version = @readFile("version.txt")
console.log(version) -- prints: 1.2.3
static missing = @readFile("nope.txt") -- error META009
static outside = @readFile("../../etc/passwd") -- error META008
```

```fa
-- with settings.json = {"port": 8080, "debug": false}
static settings = @parseJson(@readFile("settings.json"))
console.log(settings.port) -- prints: 8080
@if settings.debug {
	console.log("debug") -- removed
}
```

```fa
function fail = (): Integer throws ParseError => try Integer.parse("x")
static broken = @fail() -- error META005
```

```fa
function loop = (n: Integer): Integer => if n == 0 { 0 } else { loop(n + 1) }
static forever = @loop(1) -- error META003 (or a range/overflow stop first: META013)
```

```fa
static version = "1"
console.log(@version) -- error META012
```

```fa
let x = 12
console.log(@typeToString(Type(x))) -- prints: Integer
type User = { name: String }
console.log(@typeToString(User)) -- prints: User
```

---

## 13. Foreign code

### 13.1 Files and visibility

- A foreign file ends with `.foreign.fa` and is named by the part before (`Database.foreign.fa` is `Database`); single export like any file.
- Only foreign files can: declare and call `@extern` functions (`FFI001`), use `Pointer(T)` and `CString` (`FFI002`), declare `destroy` in a type (`FFI003`).
- Other files can't use a value whose **public** type contains `Pointer` or `CString` (`FFI002`). **(decision D29)** "Contains" means: in a public field, a parameter, a result, a type argument. A **private** field (`#handle: Pointer(Sqlite3)`) of a type declared in a foreign file is allowed: that is how `Database` is used from `main.fa`.
- `CStruct`, `CArray` and the `std.c` types are usable in every file. `Pointer` and `CString` are in the prelude of foreign files only.

### 13.2 C types and declarations

| C | Fa |
| --- | --- |
| `int8_t`…`uint64_t` | `Int8`…`UInt64` |
| `float`, `double` | `Float32`, `Float64` |
| `bool` | `Boolean` |
| `char`, `int`, `long`, `size_t`, … | `c.Char`, `c.Int`, `c.Long`, `c.SizeT` from `std.c`, sized for the target |
| `T *` | `Pointer(T)?` (`none` is `NULL`); `_Nonnull`: `Pointer(T)` |
| `void *` | `Pointer(Void)?` |
| `const char *` | `CString` |
| `struct` | `CStruct { ... }` (nominal, C layout, declaration order); opaque: `CStruct { }` |
| `T field[N]` | `CArray(T, N)` |
| `enum`, integer `#define` | `static` integers |
| function pointer | a function type |

Declaration: `function close: (database: Pointer(Sqlite3)) => c.Int = @extern("sqlite3_close")`. Rules:

- A parameter C writes through is declared `mutates` (trusted).
- `FFI005`: an `@extern` function declares `throws` or `requires`. It is never pure.
- `FFI009`: a `CStruct` field whose type isn't a C type (numbers, `std.c`, `CStruct`, `CArray`, `Pointer` — the last one only in foreign files).
- `FFI007`: C unions, bit fields, variadic functions (only reachable through `fa bind`, which comments them out).
- Callbacks (`FFI006`): a function-typed parameter of an `@extern` function receives a function **known at compile time**, capturing nothing, with C types, that can't throw.

### 13.3 Passing values to C

| Parameter | Argument | C receives |
| --- | --- | --- |
| `Pointer(T)` | `Array(T)` | pointer to the first element |
| `Pointer(T)` with `mutates` | a `mutable` value of type `T` | its address (`FFI010` if not mutable) |
| `Pointer(T)` without `mutates` | any `T` | its address, read-only |
| `CString` | `String` | a zero-terminated copy (literals are stored with the zero byte) |
| `Pointer(Void)` | anything a `Pointer(T)` parameter accepts **(decision D50)** | as above (C's implicit conversion to `void *`) |

Pointers made from Fa values are valid for the duration of the call.

### 13.4 Pointers

`pointer.value` (a reference, `use x = pointer.value`), `read()`, `write(value)`, `offset(count)`, `toArray(count)`, `cString.toString()`, `Pointer.allocate(value)`, `pointer.free()`. Unchecked.

### 13.5 Destroy and move-only types

- `destroy = (self) => { ... }` in a type of a foreign file; runs when a value is destroyed: scope end, replacement, removal from a collection. Can't throw or be `async` (`FFI011`).
- A type with `destroy`, and every type containing one, is **move-only**: assignment moves; using the source afterwards is `MEM002`. It can't be captured by copy (an escaping closure capturing it is `MEM003` "a `Database` can't be copied") nor sent to a worker.

### 13.6 Exposing a C library (`build.cLibrary`)

- `{ header, prefix, api }`: the foreign file `api` exports a namespace; each function becomes `prefix_snake_case`.
- Parameters and results have C types; Fa values cross behind `Pointer` (opaque `typedef struct prefix_Farm prefix_Farm;`).
- A `throws` function returns `int` (0 success, else the 1-based position of the error member, declared in an `enum`), its result through a last out-parameter; `prefix_last_error_message()` is thread-local.
- `REQ006` `requires` on an API function; `FFI012` an API function that uses a module-level `mutable` value not thread-safe; module-level values initialized at load time.
- `FFI013` `cLibrary` with a JavaScript target.
- `FFI014` a `cLibrary` API that isn't a namespace of functions with C types, throwing one `Errors` type.

### 13.7 Targets

| Target | Calling C | C library |
| --- | --- | --- |
| `native` | direct C ABI calls; `CStruct` classified like clang (System V, AAPCS64, Win64) | yes |
| `wasm` | imports (module `env`), resolved by `wasm-ld` or the host | yes (exports) |
| `bun` | `bun:ffi` `dlopen`, one symbol table from the `@extern` declarations; `Pointer` is a number; `read`/`write` through a `DataView` over `toArrayBuffer`; callbacks `JSCallback`; no `CStruct` by value (`FFI008`) | no |
| `web`, `node` | `FFI008` "C functions can't be called on target `web`: branch with `@if target`" | no |

`destroy` on `bun` is called explicitly at every scope exit ([6.3](#63-destruction)). Linking uses `build.link` and `build.libraryPaths`.

### 13.8 `fa bind`

`fa bind <header.h> [--output <file>] [-- clang flags...]` uses libclang once: functions → `@extern` declarations with camelCase names (a common prefix becomes a namespace: `sqlite3_open` → `sqlite3.open`; else `sqlite3Open`); structs → `CStruct`, opaque → `CStruct { }`; enums and integer `#define`s → `static`; `const` pointer params have no effect, other pointer params get `mutates`; pointers optional unless `_Nonnull`; untranslatable declarations written as comments. Output is committed; builds never read headers.

### 13.9 Tests

```fa
-- main.fa (not a foreign file)
function close: (handle: c.Int) => c.Int = @extern("close") -- error FFI001
```

```fa
-- Database.foreign.fa
export type = {
	#handle: Pointer(Sqlite3)
	function destroy = (self) => sqlite3.close(self.#handle)
}
-- main.fa
let database = try Database.open("farm.db") -- valid: `Pointer` is private
let other = database -- a move
console.log(database) -- error MEM002
```

```fa
-- main.fa
function leak = (pointer: Pointer(Integer)) => {} -- error FFI002
```

```fa
-- x.foreign.fa
export namespace = {
	function bad: (x: c.Int) => c.Int throws ParseError = @extern("bad") -- error FFI005
}
```

```fa
type Vertex = CStruct {
	position: CArray(Float32, 3)
	color: CArray(UInt8, 4)
}
type Wrong = CStruct { name: String } -- error FFI009
```

```fa
-- compiled for target `web`
use { open } = sqlite3
open(...) -- error FFI008
```

```fa
-- native, linking libc: qsort with a compile-time callback
-- libc.foreign.fa
export namespace = {
	function sort: (base: Pointer(Void), count: c.SizeT, size: c.SizeT, compare: (a: Pointer(Void), b: Pointer(Void)) => c.Int) => Void mutates base = @extern("qsort")
	function compareInts = (a: Pointer(Void), b: Pointer(Void)): c.Int => ...
	function sorted = (values: Array(Int32)): Array(Int32) => {
		mutable copy = values
		sort(copy, copy.length, 4, compareInts)
		copy
	}
}
-- main.fa
console.log(libc.sorted([3, 1, 2])) -- prints: [1, 2, 3]
```

---

## 14. Concurrency without async

### 14.1 Scope of this iteration

| Feature | This iteration |
| --- | --- |
| `async`, `await`, `start`, `start do`, `Task`, `Task.all/any/settled`, `Cancelled` | **no**: `CONC011` "`async` is not supported yet" on the keywords; the prelude names exist but can't be used |
| `for parallel`, `parallel()` | **yes**, native |
| thread-safe types, atomics | yes, every target (single-threaded semantics on JavaScript and wasm) |
| `Mutex` | `tryLock()` everywhere; `lock()` called blocking on native only (it is an `async` function of the standard library; its synchronous instance exists on native) |
| `Channel` | `trySend`/`tryReceive`/`close` everywhere; `send`/`receive` blocking on native only |
| workers | native only, methods called **blocking** (each call waits for the result); JavaScript workers need async and come with it |
| `WorkerPool` | no (its `submit` takes an async function) |

### 14.2 `for parallel`

```fa
for parallel number, index in numbers { results[index] = expensiveComputation(number) }
for parallel(threads = 2) number, index in numbers { ... }
```

Static rules, checked with a **parallel frame** on the scope stack (collection `L`, loop variable, index symbol `i`):

- **Writes** (every effect applied in the body, direct, through callees, implied): allowed only on
  1. places below the loop variable, when `L` is mutable (each iteration owns its element; set elements and map keys stay read-only);
  2. `T[i]` or a path below it, where `i` is the loop's index binding (identity `Symbol(i, 0)` exactly) and `T` is an `Array` or a `Columnar`; when `i < T.length` isn't proven (range analysis: equal lengths of a copy, `map`), the write must be `try T[i] = ...`;
  3. thread-safe values (their methods declare no `mutates`, so they produce no effect);
  4. locals declared in the body.
  Anything else is `CONC002` "`results.push` mutates `results`, shared by every iteration: write to `results[index]` instead". Module-level values and requirements mutated by a callee: `CONC012`, unless thread-safe.
- **Reads**: any value no iteration modifies. The walk records the shared places read in the body and checks them at the end of the body against the written places generalized over `i` (`T[i]` → `T[..]`): reading `results[0]` while iterations write `results[index]` is `CONC013`. Reads of the iteration's own places are fine.
- `break`, `return` (`CONC003`); `await` (out of scope anyway); `continue` ends the iteration.
- Reactive values: `REA007`.
- `console` output allowed (lines may interleave).
- The index binding exists only for `Array`, `Columnar`, `GrowingArray` (`CONC005`).
- Errors: the body may throw; the call site of the loop is the `try` point; the error of the smallest failing index wins; iterations not started are skipped.
- Targets: `native` only; elsewhere `CONC001` "`for parallel` is only available on `native`: branch with `@if target`".
- `parallel()`: `collection.parallel()` returns a view whose `map`, `filter`, `reduce`, `sum`, … run in parallel, with callbacks following the same rules; `map`/`filter` keep the order; `reduce` groups unspecified.

Lowering (native): outline the body into `body(start, end, env)`; `env` holds pointers to the captured places; `fa_parallel_for(count, body, env, threads)` on the runtime's fork-join pool; per-chunk error slots, smallest index wins.

### 14.3 Thread-safe types

- `Mutex(T)`, `AtomicInteger`, `AtomicBoolean`, `Channel(T)` from `std.concurrency`, and objects/tuples whose fields are all thread-safe.
- **Handles**: copying shares (reference counted, atomic counts on native); the only exception to "assignment copies".
- Their methods don't declare `mutates self`, so several iterations/threads can use one at once, and a function receiving one doesn't declare `mutates`.
- **(decision D23)** Declared `mutable` where created; `let counter = AtomicInteger { value = 0 }` is `CONC007` "a thread-safe value is declared with `mutable`".
- `T` of `Mutex(T)`/`Channel(T)` can't contain references or relations (`CONC006`).
- `Mutex.lock()` returns a guard (`mutable guard = stats.lock()`), released at the end of the guard's scope; `guard.value` is a reference valid while the guard lives, written through a `mutable` guard; `tryLock(): Guard?`.
- Atomics: `load`, `store`, `increment`/`decrement`/`add` (return the new value), `swap`, `compareAndSwap(expected, value): Boolean`, optional `ordering = Ordering.relaxed | acquire | release | acquireRelease | sequential` (default).
- Channel: `Channel(T) { capacity = n }` (unbounded without), `send(value)` (throws `ChannelClosed`), `receive(): T?` (`none` when closed and empty), `trySend(value): Boolean`, `tryReceive(): T?`, `close()`.
- Blocking waits on JavaScript and wasm (`lock`, `send`, `receive`, worker calls): `CONC010` "`lock` can't be called blocking on target `bun`".

Lowering: native — refcounted heap blocks; `Mutex` a lock whose blocking path parks the thread; atomics are LLVM atomic instructions with the ordering. JavaScript — an `AtomicInteger` is an object `{ value }` updated with plain operations (single thread); a `Mutex` an object with a `locked` flag and the value.

### 14.4 Workers (native, blocking calls)

- A `.worker.fa` file exports a type; creating a value starts a thread; the caller holds a move-only **handle** (`MEM002` after a move; `CONC009` when copied).
- Calls are queued, run one at a time; `mutates self` methods need a `mutable` handle; the caller can't read or write fields (`CONC008`), only call methods; fields are given at creation.
- Arguments and results are **copied** across threads (moved when dead); references and relations can't cross (`CONC008`); thread-safe handles are shared.
- Worker code can't use a module-level `mutable` value unless thread-safe (`CONC008`), and worker methods can't declare `requires` (`REQ006`).
- `stop()` waits for the current call, cancels the queued ones, stops the thread, consumes the handle; a type can't declare a method named `stop` (`CONC008`); the handle's destruction stops the worker.
- In this iteration, a method call is blocking (`let total = worker1.sum()`); `await`/`start` come with async.

Lowering: a thread with its own copy of the immutable module-level values; a call is a message (copied arguments + completion slot) on the worker's queue; the caller parks until the completion.

### 14.5 Errors

| Code | Condition |
| --- | --- |
| `CONC001` | `for parallel` / `parallel()` on a target other than `native` |
| `CONC002` | parallel body writes a shared place |
| `CONC003` | `break`/`return` in a parallel body |
| `CONC005` | index binding on a collection without positions |
| `CONC006` | `Mutex`/`Channel` element type with references or relations |
| `CONC007` | thread-safe value declared with `let` |
| `CONC008` | worker rule (fields, references, `stop`, module-level `mutable`) |
| `CONC009` | worker handle copied |
| `CONC010` | blocking wait on JavaScript or wasm |
| `CONC011` | `async`/`await`/`start` not generated by the backend of the target yet (code generation; the analyzer types them, `design/async.md`) |
| `CONC014` | `await` outside an async context (alias `FN016`) |
| `CONC015` | `await` of a call of a non-async function, or of a value that is neither a call nor a `Task` |
| `CONC016` | a detached task that may throw |
| `CONC017` | a detached task uses a requirement or a local it can't copy |
| `CONC018` | a task outlives a value it uses |
| `CONC019` | a worker handle stopped while a started call is pending, or `start` of a `mutates self` method on a handle that isn't `mutable` |
| `WCONC001` | asynchronous mode forced off while the program starts tasks |
| `CONC012` | parallel body mutates a module-level value or requirement that isn't thread-safe |
| `CONC013` | parallel body reads a place other iterations write |

### 14.6 Tests

```fa
-- native
function expensiveComputation = (n: Integer): Integer => n * n
let numbers = [1, 2, 3, 4]
mutable results = numbers
for parallel number, index in numbers {
	results[index] = expensiveComputation(number)
}
console.log(results) -- prints: [1, 4, 9, 16]
```

```fa
-- native
let numbers = [1, 2, 3]
mutable results: Array(Integer) = []
for parallel number in numbers {
	results.push(number) -- error CONC002
}
```

```fa
-- native
mutable total = 0
for parallel number in [1, 2, 3] {
	total += number -- error CONC002
}
use { AtomicInteger } = std.concurrency
mutable sum = AtomicInteger { value = 0 }
for parallel number in [1, 2, 3] {
	sum.add(number)
}
console.log(sum.load()) -- prints: 6
```

```fa
-- native
mutable values = [1, 2, 3]
for parallel value, index in values {
	value += 1 -- valid: the loop variable of a mutable collection
	if index == 1 {
		break -- error CONC003
	}
}
```

```fa
-- native
mutable values = [1, 2, 3]
for parallel value, index in values {
	values[index] = values[0] -- error CONC013 (iteration 0 writes values[0])
}
```

```fa
-- target bun
for parallel x in [1, 2] { } -- error CONC001
```

```fa
use { AtomicInteger } = std.concurrency
let counter = AtomicInteger { value = 0 } -- error CONC007
```

```fa
use { Mutex } = std.concurrency
mutable people: Pool(Person) = []
mutable shared = Mutex(Person in people) { value = ... } -- error CONC006
```

```fa
-- native
use { Mutex } = std.concurrency
type Stats = { visits: Integer }
mutable stats = Mutex(Stats) { value = { visits = 0 } }
for parallel index in 0..100 {
	mutable guard = stats.lock()
	guard.value.visits += 1
}
console.log(stats.lock().value.visits) -- prints: 100
```

```fa
-- target bun
use { Mutex } = std.concurrency
mutable stats = Mutex(Integer) { value = 0 }
let guard = stats.lock() -- error CONC010
let maybe = stats.tryLock() -- valid
```

```fa
-- MathWorker.worker.fa
export type = {
	numbers: Array(Integer)
	function sum = (self): Integer => self.numbers.reduce((total, number) => total + number, 0)
}
-- main.fa (native)
mutable worker = MathWorker { numbers = [1, 2, 3] }
console.log(worker.sum()) -- prints: 6
console.log(worker.numbers) -- error CONC008
let other = worker -- a move
worker.sum() -- error MEM002
```

```fa
async function fetchData = (url: String): String => url -- error CONC011
```

---

## 15. Modules

### 15.1 Roots and contexts

| Context | Root of implicit names | Also sees |
| --- | --- | --- |
| a file of `library/` | `library/` | dependencies, prelude |
| a file of `application/` | `application/` | `library` namespace (if `library/` exists), dependencies, prelude |
| a script (`scripts/**.fa`) | `scripts/` | `library`, dependencies, prelude |
| a standalone file and its siblings | the folder of the file | prelude (remote code only with `@import`) |
| a test file | the root of its folder | as the files of that root |
| a configuration file (`package.fa`, `workspace.fa`, `dependencies.fa`) | the `.fa` files next to it, except configuration files; not `library/`, `application/`, `scripts/`, `packages/` | `workspace` namespace (the `.fa` files at the workspace root), configuration types, prelude, `@import` |
| a dependency's `library/` | that `library/` | its own dependencies, prelude |
| a remote folder | the folder (from its `manifest.fa`) | prelude, `@import` |
| a remote file | nothing | prelude, `@import` |

### 15.2 The scan (`fa-codebase-parser`)

Before parsing, the file system of each root is scanned into **name tables**:

```
scanFolder(path) -> NameTable:
	table = {}
	for entry in list(path), skipping names that start with ".":
		if entry is a file ending in ".fa":
			if it ends in ".test.fa": record as a test file of this folder (never a name); continue
			name, kind = strip ".worker.fa" | ".foreign.fa" | ".fa"
			if name is not an ASCII identifier: MOD001 on the file ("`my-file.fa` is not a valid name")
			add(table, name, File(kind, path))
		elif entry is a folder:
			if entry's name is "(x)": merge(table, scanFolder(entry))       -- a group: its contents merge here, recursively
			elif the folder contains a .fa file at any depth:
				if not an identifier: MOD001
				add(table, name, Folder(scanFolder(entry)))
			else: ignore (readable with @readFile)
		else: ignore (non-.fa files)
	add/merge report: same name twice (file and folder, file and grouped file, ...) → MOD003 on both entries
	names equal ignoring case → MOD002
```

The root table is the table of the root folder. For `application/`, a root entry named `library` is `MOD003`. A dependency key equal to a root entry is `MOD003`/`DEP005`. Collisions are reported on the entries during the scan, wherever the name is used.

### 15.3 Name lookup

A name is looked up in order, first match wins:

1. local scopes (blocks, function), then the module-level declarations of the file and the **file's own name** (it designates the file's export);
2. the **siblings**: the name table of the file's folder (groups merged);
3. the **root entries**, and the `library` namespace in `application/` and `scripts/`;
4. the **dependencies** of the package (keys of the merged `dependencies.fa`);
5. the **prelude**.

Shadowing a later level is allowed; shadowing a prelude name is `MOD013` (warning). The right-hand side of a declaration never sees the name being declared (`use User = User` designates the sibling `User.fa`). A qualified name `a.b.c` resolves `a` by lookup, then members: a folder's entries, a namespace's public members (`#private` ones only from inside: `MOD009` "`#parseInternal` is private"), a type's associated declarations. An unknown name is `MOD012` with suggestions (closest names at every level, and "test files can't be used from other files" when the name is a test file).

Every lookup result **including misses** is recorded as a dependency of the declaration being analyzed ([21](#21-compilation-units-caching-incrementality)).

### 15.4 Exports

| Form | Exports |
| --- | --- |
| `export = value`, `export: Type = value` | a deeply immutable value |
| `export mutable = value`, `export mutable: Type = value` | a value every file may modify (stable address) |
| `export function = ...`, `export function(Item) = ...` | a function, generic or not |
| `export async function = ...` | `CONC011` in this iteration |
| `export type = ...`, `export type(First, Second) = ...` | a type |
| `export namespace = { ... }` | a namespace |

- Each non-test file exports exactly one entity. **(decision D30)** A file of `library/` or `application/` without an `export` is `MOD005` ("each file exports one entity: add `export` to its main declaration"), except the standalone file given to `fa run`/`fa build` and scripts; two exports are `MOD005`. Test files export nothing (`MOD005` if they do).
- A module-level declaration with the file's name is `MOD004`.
- `export reactive`, `export derived`, `export static` → `MOD011` (they may be members of an exported namespace).
- A `mutable` member of an exported namespace is read everywhere and written only from its file (`EFF013`); other files change it through functions of the namespace that declare it in their effects.
- An `export mutable` value is written by any function that declares the effect (`mutates people.size`, `mutates people`); it can be replaced as a whole (`people = []`), unlike a parameter.

### 15.5 What a file may contain

- Files of `library/` and `application/` (and `.worker.fa`, `.foreign.fa`): only declarations — `let`, `mutable`, `use`, `static`, `function`, `type`, `namespace`, `reactive`, `derived`, `watch`, `export`, and `@if`/`@for` around them. Any other statement (call, assignment, `provide`, `assert`) is `MOD006` "a statement at the top of a package file: move it into a function; top-level statements are only allowed in standalone files and scripts".
- Standalone files and scripts: any statement at the top level, run in order (they may use `try`; an escaping error ends the program with exit code 1).
- Test files: declarations and tests (in namespaces); no other statements (`MOD014`).

### 15.6 The entry point

- The entry file (`entry`, default `application/main.fa`) exports a function. Accepted signatures: parameters `()` or `(arguments: Array(String))`; return type none or `Integer`; `throws` anything; no `requires` (`REQ002`); `async` later. Anything else is `MOD015` "the entry function takes `(arguments: Array(String))` or nothing, and returns an `Integer` exit code or nothing".
- **(decision D51)** The effects of the entry function are inferred, even with a block body, like those of tests and top-level statements: nothing calls it, so there is no caller to inform (it may still declare them).
- Exit code: the returned `Integer`, or `0`; `1` if an error escapes (reported on stderr with its `toString()`); `101` on a stop. On `web`, `arguments` is empty and the exit code ignored.
- `arguments` holds the arguments after the program name, without it. **(decision D4)** A standalone file and a script see an implicit immutable value `arguments: Array(String)` at their top level (the guides use it); its index 0 is the **first argument** (the guides' `arguments.get(1)` must become `arguments.get(0)`). If the standalone file also exports a function, it is called after the top-level statements, with the same arguments, and its result is the exit code.

### 15.7 Module-level values and initialization

```
initializationOrder(program):
	roots = entry function | standalone top-level statements | tests being run
	reachable = module-level values read from the roots, transitively through function summaries
	for v in reachable:
		deps(v) = module-level values read by v's initializer, directly or through the functions it calls
		if v's initializer is compile-time evaluable (literal, static, constant object): emit as constant data; no runtime step
	order = topological sort of the other values by deps, ties broken by declaration order in a file, then file path (alphabetical)
	a cycle → MOD007 "the initializers of `a` and `b` depend on each other" (on each value of the cycle)
	module-level `watch` blocks are created at their position in this order (deps = what they read)
	destruction at normal exit: reverse order
```

- Initializers can't use `try` (`MOD008` "an initializer can't propagate an error: handle it with `catch`") or `await`.
- Cycles between types and between functions are allowed.
- `.worker.fa` code can't use a module-level `mutable` value unless thread-safe ([14.4](#144-workers-native-blocking-calls)).

Lowering:

- **JavaScript**: one ES module per Fa file. A module-level value is a module-level `let`. The program has one generated `init()` (in the entry module) that calls the initializers in the computed order (`a$init()` exported by each module), never relying on `import` evaluation order. **(decision D10)** `export mutable` uses ES live bindings: `export let people = ...; export function people$set(value) { people = value }`; other modules read `people` directly (live binding: no getter) and replace it with `people$set(...)`; field and element writes go through the binding as usual. No wrapper object (this corrects `guides/performance.md`, which still says "wrapped in an object with a `value` field").
- **Native**: globals with stable addresses; constant data for compile-time values; one init function before the entry; one teardown function at normal exit; `export mutable` replaced in place.

### 15.8 Errors

| Code | Condition |
| --- | --- |
| `MOD001` | file or folder name is not a valid identifier |
| `MOD002` | names that differ only by case in one folder |
| `MOD003` | two entries with the same name at the same level (file/folder, grouped file, dependency/root entry, `library` in `application/`) |
| `MOD004` | module-level declaration named like the file |
| `MOD005` | a module file without export, with two exports, or a test file with an export |
| `MOD006` | a statement at the top of a package file |
| `MOD007` | cycle between initializers |
| `MOD008` | `try`/`await` in an initializer |
| `MOD009` | access to a private member from outside |
| `MOD011` | export of `reactive`, `derived`, `static` |
| `MOD012` | unknown name |
| `MOD013` | shadowing a prelude name (warning) |
| `MOD014` | a test outside a test file; a statement in a test file |
| `MOD015` | invalid entry function signature, or no exported function in the entry file |
| `MOD018` | a named declaration inside a module-level `@for` |

### 15.9 Tests

Multi-file tests list each file with a `-- file:` header; the run uses `fa run application/main.fa` semantics through the package entry.

```fa
-- file: library/myFolder/myFile.fa
export function = () => console.log(myOtherFile, mySubFolder.mySubFile, myRootFile)
-- file: library/myFolder/myOtherFile.fa
export = 1
-- file: library/myFolder/mySubFolder/mySubFile.fa
export = 2
-- file: library/myRootFile.fa
export = 3
-- file: application/main.fa
export function = () => library.myFolder.myFile()
-- prints: 1 2 3
```

```fa
-- file: library/format.fa
export function = (text: String) => "root {text}"
-- file: library/reports/format.fa
export function = (text: String) => "sibling {text}"
-- file: library/reports/summary.fa
export function = () => format("x")
-- file: application/main.fa
export function = () => console.log(library.reports.summary())
-- prints: sibling x
```

```fa
-- file: library/(utils)/format.fa
export function = (text: String) => "[{text}]"
-- file: library/reports/(charts)/bar.fa
export = "bar"
-- file: application/main.fa
export function = () => console.log(library.format("a"), library.reports.bar)
-- prints: [a] bar
```

```fa
-- file: library/report.fa
export = 1
-- file: library/report/x.fa
export = 2
-- error MOD003 (on both entries)
```

```fa
-- file: library/User.fa
export type = { name: String }
-- file: library/user.fa
export = 1
-- error MOD002
```

```fa
-- file: library/my-file.fa
export = 1
-- error MOD001
```

```fa
-- file: library/people.fa
export mutable: Pool(Person) = []
function clearPeople = () mutates people => {
	people.clear()
}
-- file: library/Person.fa
export type = { name: String }
-- file: library/register.fa
export function = (name: String) mutates people.size => {
	people.add({ name = name })
}
-- file: application/main.fa
export function = () => {
	library.register("Ana")
	console.log(library.people.size)
}
-- prints: 1
```

```fa
-- file: library/counter.fa
export namespace = {
	mutable count = 0
	function increment = () mutates count => {
		count += 1
	}
}
-- file: application/main.fa
export function = () => {
	library.counter.increment()
	console.log(library.counter.count) -- prints: 1
	library.counter.count = 10 -- error EFF013
}
```

```fa
-- file: library/parser.fa
export namespace = {
	function parse = (input: String): Integer => #parseInternal(input, 0)
	function #parseInternal = (input: String, depth: Integer): Integer => depth
}
-- file: application/main.fa
export function = () => {
	console.log(library.parser.parse("1 + 2")) -- prints: 0
	library.parser.#parseInternal("1", 0) -- error MOD009
}
```

```fa
-- file: library/a.fa
export = b + 1
-- file: library/b.fa
export = a + 1
-- error MOD007
```

```fa
-- file: library/greet.fa
console.log("hi") -- error MOD006
export function = () => {}
```

```fa
-- file: library/User.fa
export type = { name: String }
type User = { id: Integer } -- error MOD004
```

```fa
-- file: library/two.fa
export = 1
export = 2 -- error MOD005
```

```fa
-- file: library/config.fa
function load = (): Integer throws ParseError => try Integer.parse("x")
export = try load() -- error MOD008
```

```fa
-- file: library/order.fa
export namespace = {
	let first = second + 1
	let second = 10
}
-- file: application/main.fa
export function = () => console.log(library.order.first)
-- prints: 11
```

```fa
-- file: application/main.fa
export function = (arguments: Array(String)): Integer => {
	console.log(arguments.length)
	3
}
-- the built program run with the arguments `a b`
-- prints: 2
-- exit: 3
```

```fa
-- file: application/main.fa
export function = (count: Integer) => {} -- error MOD015
```

```fa
-- standalone: hello.fa, run as `fa run hello.fa Ada`
let name = arguments.get(0) ?? "World"
console.log("Hello, {name}!") -- prints: Hello, Ada!
```

```fa
-- standalone, with mySiblingFile.fa (`export = "sibling"`) and utils/format.fa next to it
console.log(mySiblingFile) -- prints: sibling
let console = 1 -- warning MOD013
```

---

## 16. Packages, workspaces, configuration

### 16.1 Discovery

From the current directory upward: the closest `package.fa` gives the package; the closest `workspace.fa` (at or above it) gives the workspace, if the package is inside its `packages/`. With a `workspace.fa` and no `package.fa` on the way, the command applies to the workspace. With neither: only `fa run <file.fa>`, `fa build <file.fa>`, `fa format`, `fa bind`, `fa new`, `fa help`, `fa version` work; the others fail with `CLI001` "no `package.fa` or `workspace.fa` found in this folder or its parents".

### 16.2 Layouts

- Package: `package.fa` (required), `dependencies.fa`, `library/`, `application/`, `scripts/`. `library/` is published; `application/` is private; scripts run with `fa run scripts/x.fa`.
- Workspace: `workspace.fa`, `dependencies.fa`, `packages/` scanned recursively for folders with a `package.fa`; the scan doesn't enter a package; a `package.fa` inside a package's folder is `CFG006`. Package names come from `name` and are unique (`CFG004`); intermediate folders don't matter.
- Commands run for every package that "implements" them **(decision D44)**: `build`/`develop`/`preview` for packages with an `application/` (or an entry), `test` for packages with test files, `check`/`format` for all.

### 16.3 Configuration files

Evaluation by the interpreter ([12.6](#126-the-interpreter)), deterministic (no clock, randomness, environment variables, network; `@readFile` of package/workspace files, which become build inputs; remote code only through `@import` and the lockfile). Order:

1. `dependencies.fa` of the workspace, then of the package (they can use `@import`, sibling `.fa` files, the `workspace` namespace; never the dependencies they declare: `CFG003`);
2. `workspace.fa`, then `package.fa` (they can use `@import`, not the package's dependencies).

Types of the exports: `package.fa` → `PackageConfiguration`, `workspace.fa` → `WorkspaceConfiguration` (same fields, all optional), `dependencies.fa` → an object whose fields are `DependencySource`. These types, `Target`, `BuildOptions`, `CLibraryOptions`, `DevelopOptions`, `TestOptions`, `TargetOverride`, `TargetOverrides`, `DependencySource` are in the prelude of configuration files (definitions in `architecture/configuration`). Errors are ordinary type errors on the export (`STR002` for an unknown field, with autocompletion in the editor); a missing `name` in `package.fa` is `STR001`.

### 16.4 Merging

The configuration of a package for a target is built from the fields **written** in each file:

```
merged(target) =
	defaults(PackageConfiguration)
	<- written(workspace.fa)
	<- written(package.fa)
	<- written(workspace.fa).overrides[target] (entry, build, develop, test)
	<- written(package.fa).overrides[target]
where `a <- b`: objects merge field by field at every depth; any other value of b (arrays included) replaces a's
```

**(decision D32)** "Written" is tracked by the interpreter: an object literal typed by a configuration type keeps the fields it omits **absent** (no default applied) until the merge; a value built by a function keeps the presence of the literal it came from. Defaults are applied last. The result is the static `configuration` ([12.2](#122-static-values)).

### 16.5 Tests

```fa
-- workspace.fa
export = { name = "myWorkspace", version = "1.0.0", license = "MIT", targets = ["web", "native"], build = { minify = false } }
-- packages/tool/package.fa
export = { name = "tool", targets = ["native"], build = { output = "out" } }
-- expected merged configuration of `tool` for `native`:
--   version = "1.0.0", license = "MIT", targets = ["native"],
--   build = { optimize = true, sourceMaps = false, output = "out", minify = false, link = [], libraryPaths = [] }
```

```fa
-- package.fa
let version = "2.1.0"
function withServerTargets = (targets: Array(Target)): Array(Target) => [...targets, "bun", "node"]
export = {
	name = "farm"
	version = version
	description = "Farm utilities, version {version}"
	targets = withServerTargets(["web"])
}
-- expected: targets = ["web", "bun", "node"]
```

```fa
-- package.fa
export = { name = "farm", target = ["web"] } -- error STR002 (`target` is not a field: did you mean `targets`?)
```

```fa
-- package.fa
export = { name = "farm", version = "{std.time.now()}" } -- error META002
```

```fa
-- dependencies.fa
export = { farm = "github:farmers/farm@4" }
-- package.fa
export = { name = farm.name } -- error CFG003 (a configuration file never uses the dependencies it declares)
```

```fa
-- myWorkspace/shared.fa
export = { defaultTargets = ["web", "native"], license = "MIT" }
-- myWorkspace/packages/farm/package.fa
use { defaultTargets, license } = workspace.shared
export = { name = "farm", license = license, targets = [...defaultTargets, "wasm"] }
-- expected: targets = ["web", "native", "wasm"]
```

```fa
-- in library/setup.fa of a package with targets ["web", "native"]
export function = () => {
	@if target is "native" {
		console.log("file storage")
	} @else {
		console.log("browser storage")
	}
}
-- `fa build --target=native` output prints: file storage; `--target=web`: browser storage
```

---

## 17. Dependencies

### 17.1 Sources

| Source | Form | Content |
| --- | --- | --- |
| Git | `github:owner/repo@range`, `gitlab:owner/repo@range`, `git+https://host/path.git@range` | a Fa package: its `library/` is the namespace, its `dependencies.fa` its dependencies; `application/`, `scripts/`, tests ignored |
| sibling package | `"../shared"` (relative to the package folder) | a package of the same workspace, built from source, never locked (`DEP008` if not a package of the workspace) |
| remote file | `https://.../types.fa` | one module; no siblings; `@import` only |
| remote folder | `https://.../ui@2.1.0/` | a root, listed by its `manifest.fa` (`export = ["button.fa", "layout/grid.fa"]`); `DEP007` without manifest |
| generated | `{ source = url, generate = (content: String) => String }` | the fetched content is locked; `generate` runs on the interpreter (pure); its result is parsed as a remote file |

A key is an identifier and not a root entry name (`DEP005`). Only declared dependencies are visible (not transitive ones). A workspace `dependencies.fa` applies to every package; a package key replaces the same workspace key. An unversioned URL is `DEP004` (warning). `@import("...")` takes the same sources (non-local only) and is locked the same way.

### 17.2 Versions and resolution

- Versions of a Git repository: tags that are semver (`4.2.1` or `v4.2.1`); the tagged commit's `package.fa` must have the same `version` (`DEP003` rejects that version).
- Ranges: `@4` = latest `4.x.y`; `@4.2` = latest `4.2.x`; `@4.2.1` = exactly; `@0` = latest `0.x.y` (no special rule for 0). Pre-releases only when named exactly.
- A library is identified by its normalized repository URL, never by its key.

```
resolve(requests):                       -- request = (repository, range, requester)
	worklist = direct requests of the workspace, its packages, and @import sources
	loop:
		group requests by (repository, major)
		for each group: chosen = max version satisfying every range; none → DEP002 naming the requesters of each range
		add the requests declared by each chosen version's dependencies.fa (transitively)
		until no chosen version changes
	different majors coexist as distinct libraries
	locked versions are kept unless `fa update` names them
```

### 17.3 Lockfile and cache

- One `dependencies.lock` per workspace (root), or per package outside a workspace, or per standalone file (next to it). Fa syntax, generated, sorted by source then version:

```fa
-- dependencies.lock: generated by `fa`, do not edit
export = [
	{
		source = "github:farmers/farm"
		version = "4.2.1"
		commit = "9f2c1e7a4b0d3c8e6f5a2b1c0d9e8f7a6b5c4d3e"
		hash = "sha256:3a7bd3e2..."
	}
	{
		source = "https://api.weather.com/fa/types.fa"
		hash = "sha256:ef2d127d..."
	}
]
```

- Hash: SHA-256 of the content: a remote file's bytes; for a remote folder the manifest and the listed files; for Git, the files the dependency uses (`package.fa`, `dependencies.fa`, `library/**`). **(decision D45)** Canonical serialization for multi-file content: files sorted by path, each as `path NUL length NUL bytes`.
- Every build checks the contents against the hashes: a change is `DEP001` "the content of `weather` changed since it was locked … help: run `fa update weather` to accept the new version".
- A new source is resolved and locked by the next command that needs it; `fa update [names]` re-resolves.
- Cache: `$FA_CACHE`, else `~/.cache/fa` (Linux), `~/Library/Caches/fa` (macOS), `%LOCALAPPDATA%\fa` (Windows); entries addressed by hash, published atomically, deletable at any time. A build whose dependencies are all locked and cached needs no network (`DEP006` when the network is needed and fails).
- A consumer building for a target the library doesn't list in `targets` gets `DEP009` "`farm` doesn't support target `wasm`".

### 17.4 Tests

```fa
-- dependencies.fa
export = { farm = "github:farmers/farm@4", legacyFarm = "github:farmers/farm@3" }
-- both majors are resolved; `farm.cows` and `legacyFarm.cows` coexist
```

```fa
-- packages/a/dependencies.fa: export = { farm = "github:farmers/farm@4.2.1" }
-- packages/b/dependencies.fa: export = { farm = "github:farmers/farm@4.3" }
-- error DEP002 (no 4.x satisfies 4.2.1 and 4.3; requested by `a` and `b`)
```

```fa
-- dependencies.fa
export = { library = "github:x/y@1" } -- error MOD003 (when `library` is a root entry name) / DEP005
export = { weather = "https://api.weather.com/fa/types.fa" } -- warning DEP004
```

```fa
use weather = @import("https://api.weather.com/fa/types.fa")
use local = @import("./types.fa") -- error META010
```

---

## 18. Testing

### 18.1 Syntax and static rules

```fa
test "adds two numbers" { assert math.add(2, 3) == 5 }
test skip "not ready yet" { ... }
test only "the one I am debugging" { ... }
@test "the version parses" { ... }
namespace division = { test "divides evenly" { ... } }
```

- Only in `*.test.fa` files (`MOD014`), at the top level or in namespaces. Two tests with the same name in the same namespace: `TEST002`.
- `assert` takes a `Boolean` (`TEST001`), without parentheses.
- A test body is a block that may use `try` (an escaping error fails the test), `catch`, `provide` (tests can't declare requirements), and `await` later (it is an async context).
- `@test` bodies are pure, run by the interpreter during `fa check` and `fa test`; a failure is a compile error `TEST003`.
- Test files see the names of their folder; `math.test.fa` sees `math`.

### 18.2 Run semantics

- Discovery: `test.include` globs (default `**/*.test.fa`) relative to the package; workspace: every package with tests, or the named one.
- Each test runs with **freshly initialized module-level values** (only the values reachable from the test). **(decision D33)** In-process: before each test, run the initializers of the values the test reaches (JavaScript: re-run the `x$init()` functions, which reassign the module bindings; native: run the test's init function, then its teardown after the test). Never a process per test.
- The tests of a file run in order, one at a time; different files may run in parallel (one worker or process per file).
- A test passes when its body ends; fails on a false `assert`, an escaping error, or exceeding `test.timeout` ms.
- A stop (overflow, failed `assert` outside a test) inside a test fails it; the remaining tests of the file are reported as not run; the runner continues with the next file (so each file runs in its own isolated worker/process).
- `test only`: when any exists, only focused tests run. `fa test` fails (`TEST004`) if a `test only` exists and the `CI` environment variable is set, or the current Git branch is the default branch. **(decision D34)** The default branch is `git symbolic-ref refs/remotes/origin/HEAD`, else `init.defaultBranch`, else `main`; no Git: the check is skipped.
- Exit code `0` when every test that ran passed, `1` otherwise.
- Default target `bun` (even if not in `targets`); `--target=native|web|...` runs elsewhere (`web`: in a headless browser, tier 4).
- `assert` failure report: the source text of the condition, the values of the operands of its top-level comparison, `is`, `is in` or chained comparison, and the location:

```
✗ adds two numbers (library/math.test.fa:2)
  assert math.add(2, 3) == 6
         ↳ left: 5, right: 6
```

- Outside tests, a false `assert` stops the program with the same report on stderr and exit code `101`.
- `--coverage`: line and branch coverage, LCOV in `build/coverage` (tier 4). `--watch`: re-run on change (tier 3).

### 18.3 Lowering

- JavaScript: each test file becomes a module exporting `tests = [{ name, path, line, modifier, run }]`; the runner (part of the `fa` JavaScript runtime, executed by Bun) imports it, applies the filters, and for each test calls the init functions, then `run()`, with a timeout. An `assert` compiles to a check that throws a `FaAssertionFailure` carrying the report (in a test) or a `FaStop` (elsewhere).
- Native: one test executable per package, with a runner in `fa-runtime`; a stop ends the file's process.

### 18.4 Tests

```fa
-- math.fa: export namespace = { function add = (a: Integer, b: Integer) => a + b }
-- math.test.fa
test "adds two numbers" {
	assert math.add(2, 3) == 5
}
test "fails" {
	assert math.add(2, 3) == 6
}
-- `fa test` reports 1 passed, 1 failed (left: 5, right: 6); exit: 1
```

```fa
-- counter.fa: export mutable: Integer = 0
-- counter.test.fa
test "first" {
	counter += 1
	assert counter == 1
}
test "second" {
	counter += 1
	assert counter == 1 -- fresh module-level values: passes
}
```

```fa
-- x.test.fa
test "a" { }
test "a" { } -- error TEST002
console.log("x") -- error MOD014
assert 1 -- error TEST001
```

```fa
-- greeting.test.fa
test "greets the current user" {
	provide Session = { user = { id = "42", name = "Ada" } }
	assert greeting() == "Hello, Ada!"
}
```

```fa
-- version.test.fa
@test "the version parses" {
	let version = try parseVersion("1.2.3")
	assert version.major == 2 -- error TEST003 (at compile time, in `fa check`)
}
```

```fa
-- overflow.test.fa
test "stops" {
	let big: UInt8 = 200
	let sum: UInt8 = big + big -- not provable: a runtime check that stops
}
test "never runs" { }
-- `fa test`: "stops" failed (stop), "never runs" not run; exit: 1
```

---

## 19. Hot reloading

`fa develop [<package>] [--target=<target>]`, `bun` by default:

1. Build to JavaScript with source maps, unoptimized, into the cache (not `build/`).
2. Run in a Bun process with a small development runtime **(decision D43)**: a module registry keyed by Fa file path; calls between modules go through the registry's current function table (one indirection, development builds only).
3. Watch the files of the package (and sibling packages it depends on, files read at compile time, configuration files).
4. On change: re-analyze the changed files and their dependents ([21](#21-compilation-units-caching-incrementality)); regenerate their modules; replace them in the registry. Module-level values of unchanged files keep their values; those of changed files, and the values whose initializers read them, are initialized again; reactive sources survive (kept by declaration path). Re-run the tests whose reachable set includes a changed declaration.
5. **Restart** instead, with the reason, when: a type whose values are kept changed shape, an export was renamed or removed, a file was added/removed/renamed (module structure), a configuration file changed, or `develop.hotReload = false`.
6. `develop.port`: the port of the development server for `web` (serving the output and the reload channel); `develop.open` opens a browser on start.
7. `--target=native`: rebuild and restart on each change (no hot replacement).

---

## 20. The `fa` command line

| Command | Behaviour | Exit code |
| --- | --- | --- |
| `fa run <file.fa> [arguments...]` | Run a standalone file or a script. **(decision D6)** Compiles it for `bun` and runs the output with Bun. `--target=<t>` compiles for `t` and runs the result (`native`: in the cache, then executed; `node`: with Node.js; `web`: `CLI006`). **(decision D5)** Every argument after the file is passed to the program; a `--` separator is accepted and dropped. | the program's (0, its returned value, 1 for an escaped error, 101 for a stop) |
| `fa new package <name>` | Scaffold `<name>/` (`CLI007` if it exists and isn't empty, `CLI008` if its name isn't an identifier) | 0/1 |
| `fa new workspace [<name>]` | Scaffold a workspace in `<name>/` or the current folder | 0/1 |
| `fa develop [<package>] [--target=<t>]` | [19](#19-hot-reloading) | runs until stopped |
| `fa test [<package>] [--watch] [--coverage] [--target=<t>]` | [18](#18-testing) | 0 if all ran passed, else 1 |
| `fa check [<package>]` | Analyze every file (library, application, scripts, tests) for every configured target (sharing target-independent results, D22), run `@test` blocks; no output | 0, or 1 on any error |
| `fa format [<paths>...] [--check]` | Format; `--check` lists unformatted files | 0, or 1 with `--check` and unformatted files |
| `fa build [<package> \| <file.fa>] [--target=<t>]` | Analyze all files of the compiled roots (library and application; tests and scripts excluded), generate code for the reachable declarations, for each target of the configuration (or `t`; a target not in `targets` is allowed with warning `CLI003`) | 0/1 |
| `fa preview [<package>] [--target=<t>]` | Run the output of `fa build`: without `--target`, the first configured target that runs here (`native` built for this host, then `bun`, `node`; `web`: a static server on `develop.port`) | the program's |
| `fa update [<dependency>...]` | Re-resolve and rewrite the lockfile | 0/1 |
| `fa bind <header.h> [--output <file>] [-- clang flags...]` | [13.8](#138-fa-bind) | 0/1 |
| `fa help [<command>]`, `fa version` | | 0 |

- `<package>` is the `name` of a package of the workspace (`CLI004` unknown); without it, the command runs for every package that implements it (D44).
- `--target` must be a `Target` (`CLI002`).
- **(decision D35)** Bun is required by `fa run`, `fa test` (default) and `fa develop`: `fa` looks for `bun` on `PATH`, then in `~/.bun/bin`; missing: `CLI005` "Bun is needed to run Fa code on the `bun` target: install it from https://bun.sh, or pass `--target=native`".
- **(decision D36)** Output layout: `<package>/<build.output>/<target>/`: `native/<name>` (`.exe` on Windows); `wasm/<name>.wasm` plus `<name>.js` loader; `bun|node/` one ES module per Fa file plus `main.js` and the runtime modules; `web/main.js` (bundled, minified with `build.minify`) and `main.js.map` with `build.sourceMaps`. `fa build file.fa` writes to `./build/<target>/` next to the file.
- Diagnostics are printed as `error[REF001]: message` with the file, line, column, the source line, and the related spans; `--format=json` prints one JSON object per diagnostic (for tools and agents).
- Scaffolds **(decision D46)**: `fa new package hello` writes `package.fa` (`export = { name = "hello", version = "0.1.0" }`), `dependencies.fa` (`export = {}`), `library/greet.fa`, `library/greet.test.fa`, `application/main.fa` (the hello-world guide's files). `fa new workspace` writes `workspace.fa` (`export = { name = "<name>" }`), `dependencies.fa`, `packages/`.

Error codes: `CLI001` no package or workspace; `CLI002` unknown target; `CLI003` target not configured (warning); `CLI004` unknown package; `CLI005` Bun missing; `CLI006` target can't be run; `CLI007` scaffold folder not empty; `CLI008` package name not an identifier.

---

## 21. Compilation units, caching, incrementality

### 21.1 Units

| Unit | What | Produced by |
| --- | --- | --- |
| file | tokens and `TypedSyntaxTree` (the whole file, always) | `fa-parser`, in parallel, independent of other files |
| name table | one per folder, one per root | the scan |
| declaration | a module-level declaration: its summary (interface) and its body's side tables | `fa-analyzer`, on demand, memoized |
| file analysis | the side tables of all the file's nodes for one target (shared across targets when no declaration depends on the target) | `fa-analyzer` |
| JavaScript module | one per Fa file | `fa-codegen-js` |
| native instance / codegen unit / object | as in `compilation-with-llvm.md` | `fa-codegen-llvm` |

What is analyzed **(decision D42)**: `fa check` and `fa build` analyze **every** file of the roots they cover (a published library is source: its errors must surface in its own `fa check`); code generation is demand-driven from the entry, the tests or the exports.

### 21.2 Dependencies recorded per declaration

While analyzing a declaration, record: the summaries (interface hashes) it used; every name lookup with its result, **including misses** (a new sibling can change it); the files and directory listings read at compile time with their content hashes; the remote sources it used (lockfile hashes); whether it read `target` or `configuration` (and which fields).

### 21.3 Interface hash

The hash of a declaration's summary ([2.3](#23-function-summaries)): types, declared and inferred effects, requirements, freshness, stored bits, purity, module-level reads, reactive sets, needs-full-type bits, target dependence, and for a type its shape, its relations and its methods' summaries. A block body's edit that doesn't change these leaves dependents untouched. A single-expression body's edit that changes an inferred effect changes the interface.

### 21.4 Incremental analysis (language server, `fa develop`, `fa test --watch`)

```
onFileChanged(F):
	reparse F (whole file)
	reanalyze every declaration of F; collect the declarations whose interface hash changed
	worklist = declarations (in other files) that recorded a use of a changed interface
	while worklist: reanalyze; add the users of any interface that changed in turn
	publish diagnostics of every reanalyzed file

onFileCreatedDeletedRenamed(path):
	rescan its folder's name table (and the root table if it's a root entry); report collisions
	reanalyze every declaration whose recorded lookups touched a changed table entry (hits and misses)

onConfigurationOrLockfileChanged: reload the package: reevaluate configuration, re-resolve dependencies, reanalyze all
```

Everything is in memory for the server and the watchers. Persisted on disk (tier 3+): compile-time evaluation results, dependency sources, native objects ([`compilation-with-llvm.md` caching](../compilation-with-llvm.md#5-the-cache)). Per target, an analysis is reused across targets when it recorded no read of `target`/`configuration`.

### 21.5 Module-level graph

Initialization order and reachability ([15.7](#157-module-level-values-and-initialization)) are computed when the program is assembled (build, test run), from the summaries; never in the walk.

---

## 22. Standard library surface

### 22.1 The prelude (from `architecture/modules`)

| Kind | Names |
| --- | --- |
| Types | `Boolean`, `String`, `Character`, `Integer`, `Float`, `BigInt`, `Percentage`, `Byte`, `Int8`…`Int128`, `UInt8`…`UInt128`, `Float32`, `Float64`, `None`, `Never`, `Void`, `Array`, `Set`, `Map`, `Pool`, `Columnar`, `OrderedColumnar`, `GrowingArray`, `GrowingSet`, `GrowingMap`, `GrowingPool`, `GrowingColumnar`, `Tuple`, `Range`, `Task`, `Optional`, `Error`, `Schema`, `Subscription` (D25) |
| Type constructors and functions | `Enum`, `Union`, `Errors`, `Fields`, `CStruct`, `CArray`, `Pick`, `Omit`, `Type`, the other type functions |
| Values | `console`, `Math`, `Task` |
| Errors | `IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`, `Cancelled`, `ParseError` (used by `Integer.parse`) |
| Static values | `target`, `configuration` |
| Namespace | `std` |
| Foreign files only | `Pointer`, `CString` |
| Configuration files only | `PackageConfiguration`, `WorkspaceConfiguration`, `Target`, `BuildOptions`, `CLibraryOptions`, `DevelopOptions`, `TestOptions`, `TargetOverride`, `TargetOverrides`, `DependencySource`, the `workspace` namespace |
| Standalone files and scripts | `arguments` (D4) |

### 22.2 What the documentation uses

| Name | Signature (this brief) | Targets | Tier |
| --- | --- | --- | --- |
| `console.log(values...)` | variadic, prints the values separated by a space and a newline, in Fa syntax (top-level strings raw; strings inside values quoted; `none`; objects `{ name = "John" }`; arrays `[1, 2, 3]`) | all | 1 |
| `console.error(values...)` **(decision D39)** | same, on stderr | all | 1 |
| `Math.min`, `max`, `abs`, `floor`, `round`, `sign`, `pow`, `fround` | per `values/numbers` | all | 1 |
| `Integer.parse(text): Integer throws ParseError` **(decision D7)** | | all | 1 |
| `Set.from`, `Map.from`, `Array(T).from(values, capacity = n)` | collections brief | all | 1–2 |
| `std.fs.File.read(path): String throws FileError` **(decision D9)** | blocking | native, bun, node (`web`: compile error) | 2 |
| `std.json.Json.parse(text): T throws JsonError` (type-directed by the expected type), `Json.stringify(value): String` (D9) | | all | 3 |
| `std.time.now(): Integer` (milliseconds; impure) (D9) | | all | 2 |
| `std.time.sleep(milliseconds)` | async | — | async |
| `std.concurrency`: `Mutex`, `AtomicInteger`, `AtomicBoolean`, `Ordering`, `Channel`, `ChannelClosed`, `WorkerPool`, `WorkerTimeout` | [14](#14-concurrency-without-async) | | 3 |
| `std.c`: `Char`, `Int`, `Long`, `SizeT`, … | [13](#13-foreign-code) | native, wasm, bun | 3 |
| `std.http` (`Server`, `Request`, `Response`, `ServerError`, `get`) | provisional, async | — | async |
| `Task.all`, `Task.any`, `Task.settled` | async | — | async |

### 22.3 How the standard library is built

`packages/library/` holds only empty `.fa`/`.ts`/`.rs` files and a stale README (Zig, Rust modules, Tokio). **(decision D38)** Replace it with `packages/std/`:

- Fa sources for everything expressible in Fa (collections on native through the `intrinsics` namespace, LLVM Q7; `Math`; formatting; the reactive runtime; the test runner's Fa side).
- **Primitives** (console output, files, clock, atomics, threads) declared in Fa with a standard-library-only intrinsic, `intrinsics.primitive("fa_console_write")` as the value of a function declaration with a full type (like `@extern`, but not `@`, which means compile time). Each backend maps the name: `fa-codegen-js` to an export of its JavaScript runtime modules (`packages/codegen-js/runtime/`), `fa-codegen-llvm` to a C ABI symbol of `fa-runtime`; a primitive missing on a target makes its use a compile error for that target (this is also the mechanism of `async`-only primitives later).
- On JavaScript, collections are JavaScript structures implemented in the JavaScript runtime (collections brief); the Fa declarations carry their signatures and effects, which is all the checker needs.

---

## 23. The guides as acceptance tests

Each guide is a test of the toolchain once its features exist. With the decisions of this brief applied (D4, D7, D8, D9):

- **Hello world** (tier 1): `fa run hello.fa` prints `Hello, World!`; the `greet` version prints `Hello, World!`; `fa run hello.fa Ada` with `arguments.get(0)` prints `Hello, Ada!`; `fa new package hello` + `fa develop` prints `Hello, World!`; `fa test` passes `greets by name`; `fa build` produces the configured targets.
- **Build your application** (tiers 1–2): with `scores.csv` = `Ana, 90\nBao, 70\n`, `fa run` / `fa preview` prints `Scores:    2`, `Average:   80` (the float formatting of `80.0.round()` is owned by the numbers brief), `Highest:   Ana (90)`, `Lowest:    Bao (70)`, exit 0; a missing file: the error `ScoreError.FileNotFound` is reported, exit 1; the four tests pass; `build/native/scores` exists after `fa build --target=native`. Needs `std.fs.File.read`, `Integer.parse`, `arguments.get(0)`.
- **Performance** (tier 2–3): fusion of pure chains (observable only by allocation counts and by the order of effects of impure callbacks), `Array(Integer).from([], capacity = n)`, `static` + `@if`; `fa preview --profile` is not in the CLI table: tier 4, add it to the table when implemented.
- **Deploy a library** (tier 3): sources, tags, lockfile, `fa check`, `fa test --target=native`. The breaking-change table is informative; a future `fa` command could diff interface hashes between two tags.
- **Web server**: needs `async` and `std.http`: after this iteration. The fixes it needs now (D8: `error.message` → `error.toString()`; `now()` → `std.time.now()`) are documentation fixes.

---

## Ambiguities and decisions

Each item: the problem, then the recommended resolution (applied in this brief). The project lead accepts or overrides. Documentation fixes listed here must be made in `documentation/` first (it is the source of truth).

### D1. One walk vs memoized queries

`compilation-with-llvm.md` describes `fa-checker` as "a set of memoized queries", followed (native only) by separate monomorphize / analyze / lower passes; `compiler-architecture.md` (newer) fixes `fa-analyzer` and **one analysis walk**. **Resolution:** they are compatible: the single walk *is* demand-driven (declarations analyzed on first need, memoized, cycles detected), which is what "memoized queries" meant. Follow `compiler-architecture.md` for crate names (`fa-analyzer`, `fa-codegen-js`, `fa-codegen-llvm`, `fa`). Everything checked statically in this brief happens inside the walk ([2](#2-where-this-fits-the-single-analysis-walk)); the only re-walks are loop bodies and recursive SCCs (flow transfer only), and per-instantiation analysis of compile-time-parameterized code. Native monomorphization, layouts and last-use moves stay in the LLVM backend's own single walk, fed by side tables. Update `compilation-with-llvm.md`'s crate names.

### D2. Effects inferred in recursive functions

The spec infers the effects of a single-expression function even in a recursion cycle, which seems to need a fixpoint. **Resolution:** an SCC is handled like a loop: provisional summaries, then re-run the flow transfer (not type inference) of the SCC's bodies until the summaries stop growing; emit the last round's diagnostics ([2.4](#24-recursion)). Alternative rejected: require declared effects in cycles (diverges from the spec, more to write).

### D3. When is a local `function` escaping?

Capture mode depends on later uses, which a single forward walk hasn't seen. **Resolution:** record the closure's captures as a pending constraint, check captured references at the declaration (escaping copies) and at each call (non-escaping), and check the "no writes to captures" constraint at the first escaping use ([5.11](#511-closures)). No look-ahead pass.

### D4. `arguments` in standalone files, and its first index

`hello-world` and `build-your-application` use `arguments.get(1)` for the first user argument, while `modules` says `arguments` excludes the program name (so the first argument is index 0); and only `hello-world` says a standalone file has an implicit `arguments`. **Resolution:** keep `modules`: index 0 is the first argument (like `process.argv.slice(2)`, no surprise for JS developers who already slice); standalone files and scripts get an implicit immutable `arguments: Array(String)` at their top level. Fix both guides to `arguments.get(0)` and mention the implicit `arguments` in `architecture/overview`.

### D5. `fa run file.fa a b` vs `fa run file.fa -- a b`

The overview writes `-- first second`, the hello-world guide `fa run hello.fa Ada`. **Resolution:** everything after the file is the program's arguments (like `bun file.ts a b` and `node file.js a b`); a `--` right after the file is accepted and dropped. Update the CLI table to `fa run <file.fa> [arguments...]`.

### D6. `fa run` on the compile-time interpreter

The spec says `fa run` uses "the same interpreter" as compile-time code. That interpreter must then implement every I/O primitive and every collection a third time (after JavaScript and native), with non-deterministic I/O that compile-time evaluation forbids. **Resolution:** `fa run` compiles to `bun` and runs it (the semantics are the same on every target; start-up stays fast, the JavaScript backend exists first); the interpreter is compile-time only (static values, `@` calls, type functions, `@test`, configuration files, `generate`). Change `architecture/overview` and `advanced/metaprogramming` ("This is the same interpreter used by `fa run`").

### D7. `Integer.parse` vs `Integer.fromString`

`types/type-conversion` and `defining-functions` use `Integer.parse` (`throws ParseError`); the guides use `Integer.fromString`. **Resolution:** `Integer.parse`, one name; fix the two guides.

### D8. `error.message`

`web-server` and `concurrency` read `error.message`, while `values/errors` (V32) says `Error` only exposes `toString()`. **Resolution:** keep V32; fix the guides and examples to `error.toString()` (or a member's own field after narrowing).

### D9. `File.read`, `Json.parse`, `now()`

Used by the guides, but not in the prelude nor in `std`. **Resolution:** `std.fs` (`File.read(path): String throws FileError`, blocking, not on `web`), `std.json` (`Json.parse` directed by the expected type, `Json.stringify`), `std.time.now(): Integer` (milliseconds, impure). The guides write `use { File } = std.fs` and `use { Json } = std.json`. `std.http` stays provisional.

### D10. `export mutable` on JavaScript

`guides/performance` says an `export mutable` value is "wrapped in an object with a `value` field"; `architecture/modules` says "a getter and a generated setter function, so no value is wrapped". **Resolution:** ES module live bindings plus a generated setter (`export let x`, `export function x$set(v)`): reads are plain, no wrapper, no getter call. Fix `performance.md`.

### D11. Order of the hidden requirement parameters

"Canonical order by key" with `TypeId` keys depends on the analysis order, so the ABI of a function value would differ between builds. **Resolution:** sort by a stable content hash of the key's shape.

### D12. Mutability of a provided temporary

`provide Trace = { breadcrumbs = [] }` with a callee that `mutates trace`: unspecified. **Resolution:** a temporary provider is immutable; write `mutable trace = ...; provide Trace = trace` (one way to write a mutable provided value, and the caller can then read the result).

### D13. Mutating a primitive requirement

The requirements page's implementation note mentions "the same wrapper as a mutated `export mutable` value" for a provided primitive place. **Resolution:** a requirement of a primitive type can't be mutated (like a parameter, `EFF005`), so requirements never need a wrapper on JavaScript (return the new value instead). Remove the wrapper sentence.

### D14. `use` of names vs tracked references

`use { formatDate } = utils.text` and `use Component = react.Component` use the same keyword as tracked aliases. **Resolution:** a `use` whose right side is a module path (a file, a namespace member, a dependency path, including an `export mutable` value itself) is a **name alias**, never invalidated; a `use` of a place inside a value (`people[0]`, `town.people`) is a tracked reference.

### D15. When the JavaScript backend may skip a copy

"Unless the compiler proves that no later write to either side can be observed" needs a cheap, deterministic rule. **Resolution:** no copy when the read is a move, or when both sides are immutable for their whole lives and the shapes are equal ([6.5](#65-javascript-copies)). Nothing smarter, so output is predictable.

### D16. `GrowingColumnar` in relations

The relations page lists `Pool`, `Columnar` and `GrowingPool` as the stable-address collections; `GrowingColumnar` is just as stable. **Resolution:** allow it (consistency: each stable collection's growing variant is allowed). Add it to the relations page.

### D17. A module-level value holding a reference

`mutable first: (Person in people)?` at module level isn't a field (so not a relation side) nor a local reference. **Resolution:** `REF010` in this version; use an owner type, or keep an index.

### D18. Reading a relation field of a copy

"Relation fields are `none` (or empty) and read-only" in a copy. **Resolution:** a copy's relation fields are physically empty (the copy function doesn't copy links), so reads need no check; writes are compile errors (`REL011`), decided statically from the reference type ([8.3](#83-active-and-inactive-relation-fields)).

### D19. A many-to-many side in the literal given to `add`

"An object literal given to `add` may set the direct sides". **Resolution:** a many-to-many direct side may be set with an array literal of references (`friends = [ana, bao]`), linked in order after the element is stored.

### D20. A whole-value assignment with a literal that sets links

`ana = Person { name = "Ana", bestFriend = bao }` keeps the links and replaces data fields: what about `bestFriend` in the literal? **Resolution:** `REL021`: write the relation separately (one way to change a link).

### D21. Printing linked objects

Printing a `Person` with `bestFriendOf` could recurse forever or dump a whole graph. **Resolution:** printing and interpolation omit relation fields (like equality).

### D22. Is the untaken branch of `@if` checked?

"The other branch is not compiled at all, so it may use features that don't exist on the current target." **Resolution:** it is parsed but not analyzed (like Rust's `cfg`); the analysis is parameterized by the target; `fa check` analyzes each configured target, sharing every declaration that doesn't read `target`/`configuration`.

### D23. A thread-safe value declared with `let`

"They are declared `mutable` where they are created", but their methods don't declare `mutates`, so `let` would technically work. **Resolution:** `CONC007`: declare them `mutable` (they change; one way to write it).

### D24. Per-instance code on JavaScript

"Functions are monomorphized" for reactivity, and `Type(parameter)` needs the actual copy function, but the JavaScript backend erases generics. **Resolution:** the JavaScript backend emits extra instances only in these two cases: a function receiving a reactive place (or a value with reactive fields), keyed by which parameters are reactive; and a function that copies or reflects on a parameter's full type, keyed by the argument type. Everything else is one JavaScript function.

### D25. The type of `let subscription = watch { }`

Unnamed in the spec. **Resolution:** `Subscription`, in the prelude, move-only, with `stop()` consuming it.

### D26. What is an "evaluation step"

The 100 million step bound needs a unit. **Resolution:** one evaluated node of the checked tree.

### D27. Module-level `@for` around declarations

Allowed by `modules`, but a named declaration would be declared once per iteration with the same name. **Resolution:** a module-level `@for` may only contain unnamed declarations: `watch`, and `test`/`@test` in test files (whose names may interpolate the loop value); `MOD018` otherwise.

### D28. `@readDirectory`'s result

The web-server guide uses `file.name` and `file.path`. **Resolution:** `Array(DirectoryEntry)` with `{ name: String, path: String, isDirectory: Boolean }`, sorted by name, `path` relative to the folder of the reading file.

### D29. "A value whose type contains `Pointer`"

`Database` (used from `main.fa`) has a `#handle: Pointer(...)` field. **Resolution:** "contains" means its public shape; private fields of a type declared in a foreign file may hold pointers.

### D30. A module file without export

"Each file exports exactly one entity" doesn't say what a file with none is. **Resolution:** `MOD005` for files of `library/` and `application/`; allowed for the standalone entry file and scripts.

### D31. Concurrency without async

Mutex `lock`, channel `send`/`receive` and worker methods are `async` functions. **Resolution:** this iteration ships their **synchronous instances** only, on `native` (blocking, as the spec's blocking-call rules allow), `tryLock`/`trySend`/`tryReceive`/atomics everywhere, `for parallel` on native; `async`/`await`/`start` are `CONC011`; workers on JavaScript and `WorkerPool` wait for async.

### D32. "Fields written" in configuration merging

Merging uses the fields written in each file, but a typed literal normally gets its defaults at construction. **Resolution:** the interpreter keeps omitted fields absent in values typed by a configuration type, and applies defaults after the merge.

### D33. Fresh module-level values per test

**Resolution:** re-run the initializers reachable from the test, in-process, before each test (JavaScript: reassign the module bindings; native: init/teardown functions); each test file runs in its own worker/process so that a stop only ends that file.

### D34. "The default branch of the Git repository"

**Resolution:** `origin/HEAD`, else `init.defaultBranch`, else `main`; no Git repository: no check.

### D35. Bun as a dependency of the toolchain

`fa test`, `fa develop` and (with D6) `fa run` default to `bun`. **Resolution:** require Bun, found on `PATH` or `~/.bun/bin`, with a clear `CLI005` otherwise.

### D36. Output layout of `fa build`

Only `build/native/scores` is documented. **Resolution:** `<output>/<target>/`, detailed in [20](#20-the-fa-command-line).

### D37. Configuration files before the interpreter exists

Every package needs `package.fa` evaluated, but the interpreter is tier 2. **Resolution:** tier 1 evaluates configuration files whose export is a literal (strings, numbers, booleans, arrays, objects, references to `let` literals of the same file); anything else is `CFG007` "computed configuration needs the compile-time interpreter (not available yet)".

### D38. The standard library's implementation layout

`packages/library/` is empty stubs with a stale README (Zig/Rust modules, Tokio, Rayon). **Resolution:** `packages/std/` in Fa, with `intrinsics.primitive("name")` for per-target primitives mapped by each backend ([22.3](#223-how-the-standard-library-is-built)); delete `packages/library/` and the stale `packages/transpiler/` README.

### D39. `console` beyond `log`

Only `console.log` is used. **Resolution:** `console.log` and `console.error` (stderr), both variadic, Fa-syntax formatting. Nothing else until needed.

### D40. `invalidates` on a whole value

"This effect is reserved for precise places" — reserved how? **Resolution:** a compile error (`EFF009`), naming `mutates` as the fix.

### D41. Does `mutates self` cover deletions in the body?

`deleteWhere` is declared `mutates self` and deletes elements. **Resolution:** `mutates P` covers `invalidates Q` when `Q` is strictly below `P` through an owning edge (the caller sees a superset of invalidations).

### D42. What `fa build` checks

Demand-driven generation could skip errors in unreachable library files. **Resolution:** `fa build` analyzes every file of the roots it compiles (excluding tests and scripts) and generates code only for what is reachable.

### D43. How hot reloading swaps code

**Resolution:** development builds call across modules through a registry (one indirection); restart on shape changes, renamed/removed exports, added/removed files, configuration changes ([19](#19-hot-reloading)).

### D44. "Every package that implements" a command

**Resolution:** `build`, `develop`, `preview`: packages with an application; `test`: packages with test files; `check`, `format`: all.

### D45. Hash of multi-file dependencies

**Resolution:** SHA-256 over files sorted by path, each `path NUL length NUL bytes`.

### D46. Scaffolds

**Resolution:** `fa new package` writes the hello-world guide's package (including a test); `fa new workspace` writes `workspace.fa`, `dependencies.fa`, `packages/`.

### D47. Implied effects through a collection received alone

A function receiving `users: Pool(Blog.User)` deletes a user, which writes `blog.posts[..].author`, a place it can't name. **Resolution:** the implied effect is expressed on the owner of the argument (`owner(users).posts[..].author`) and substituted at the call site; inside the callee it is covered by the declared effect on `users`.

### D48. Enforcing "only modified through its owner"

**Resolution:** a collection that owner fields point into can't be aliased with `use`/`mutable use` nor passed to a user function's mutated or invalidated parameter (`REL016`); built-in operations on the full path are allowed (the call site knows the owner).

### D49. Compile-time arithmetic and the target

Compile-time code uses "the target's arithmetic": a value valid on `native` may overflow on JavaScript. **Resolution:** evaluate with the target's semantics; an overflow is the compile error `META013`.

### D50. `Pointer(Void)` parameters

The `qsort` binding takes `base: Pointer(Void)`, but the passing table only maps `Array(T)` to `Pointer(T)`. **Resolution:** a `Pointer(Void)` parameter accepts every argument a `Pointer(T)` parameter would (like C's conversion to `void *`).

### D51. Effects of the entry function

A block-body entry function that calls `library.register(...)` would have to declare `mutates library.people.size`, though nothing calls it. **Resolution:** its effects are inferred (as for tests and top-level statements); declaring them is allowed. Everything else keeps the rule.

### Documentation fixes collected

- `guides/hello-world`, `guides/build-your-application`: `arguments.get(0)` (D4), `Integer.parse` (D7), `use { File } = std.fs` (D9).
- `guides/web-server`, `advanced/concurrency`: `error.toString()` (D8), `std.time.now()` (D9).
- `guides/performance`: `export mutable` lowering (D10).
- `advanced/requirements` (implementation): drop the wrapper sentence (D13).
- `advanced/relations`, `reference`: add `GrowingColumnar` (D16).
- `architecture/overview`, `advanced/metaprogramming`: `fa run` on `bun` (D6), arguments without `--` (D5).
- `packages/parser/src/nodes.rs`: the `InRelation` comment still says `Bag(in Self->bestFriends)`.
- Parser follow-ups needed by this brief (spec review): `export mutable` and the other export forms, `[..]` effect paths, `mutable` namespace members, diagnostics instead of panics.

---

## Tiering

`async` (and therefore tasks, worker calls on JavaScript, `WorkerPool`, async channels and mutexes, `std.http`, `sleep`, the web-server guide) is out of scope for every tier below.

### Tier 1: a first working compiler

Goal: the hello-world and build-your-application guides run with `fa run`, `fa build` and `fa test`, on `bun` first, then `native`, and every program they accept is memory safe.

| Area | Content |
| --- | --- |
| CLI | `fa run` (via `bun`), `fa build` (`bun`, `node`, `web`; `native` as soon as the LLVM backend reaches M1), `fa check`, `fa test` (`bun`), `fa new`, `fa help`, `fa version`; diagnostics output with codes; exit codes |
| Modules | the scan and name tables, lookup order, single export and export forms (minus async), file's own name, `library` namespace, scripts, standalone files with `arguments`, `#private`, namespace `mutable` members, entry function, module-level initialization order, `export mutable` (JS live bindings, native globals) |
| Configuration | literal-only `package.fa`, `workspace.fa`, `dependencies.fa` (D37), merge, defaults, `targets`, `entry`, `build.output` |
| Effects | paths, `mutates`/`invalidates`, substitution, coverage, inference, mutability errors |
| Reference checker | the whole of [5](#5-the-reference-checker) for `use`, parameters, loop variables, `mutable use`/`reuse`, declaring again, identities (exact, symbol + offset, intervals, fresh), branches and loop fixpoints, iteration rule, alias rule, returned references, closures. It is in tier 1 because native memory safety depends on it and because it lives in the same walk as type inference: adding it later would mean reopening every transfer function. |
| Requirements | all of [9](#9-requirements) (hidden parameters: cheap) |
| Structural typing | satisfaction, exact literals, narrowing copies, mutation invariance, collections of supersets, `Set`/`Map` invariance; JS copy functions; native canonical layouts |
| Memory | copies and moves (last use), JS copy elision (D15), native cleanup stack and drop flags, recursive types |
| Tests | test files, `test`, `skip`, `only`, `assert` with report, `provide` in tests, fresh module-level values, timeout, exit codes |
| Standard library | `console.log`/`error`, `Math`, `Integer.parse`, `std.fs.File.read`, and the signatures and effects of `Array`, `Map`, `Set`, `Pool` |

### Tier 2: the language's distinctive features

| Area | Content |
| --- | --- |
| Interpreter | `static`, `@` calls, `@if`/`@for`, `@readFile`, `@readDirectory`, `@parseJson`, `@typeToString`, `@typeToSchema`, `@test`, limits, purity checks, per-target analysis (D22), computed configuration files, `target` and `configuration` |
| Relations | classification and errors, writes, implied effects, `unlink`, owner fields, copies with relinking; JavaScript, then native (with `Pool`, `Columnar`, `GrowingPool`, `GrowingColumnar`) |
| Structural typing | `Type(parameter)` instances on JavaScript; views and specialization on native (LLVM M2) |
| Packages | workspaces, sibling packages, Git and remote dependencies, lockfile, cache, `fa update`, `@import`, `DependencySource.generate` |
| Tooling | language server with incremental analysis ([21.4](#214-incremental-analysis-language-server-fa-develop-fa-test---watch)); `fa format` |
| Standard library | `std.time.now`, `Set.from`/`Map.from`, `Array.from(capacity)` |

### Tier 3: runtime systems

| Area | Content |
| --- | --- |
| Reactivity | the runtime library (JavaScript, then native), instrumentation, per-reactiveness instances, flush placement, dispose, the static checks |
| Foreign code | foreign files, `@extern`, `Pointer`, `CString`, `CStruct`, `CArray`, `destroy` and move-only types, `bun:ffi`, native C ABI, `fa bind` |
| Concurrency (D31) | `for parallel` and `parallel()` (native), atomics, `Mutex`/`Channel` (blocking on native, `try*` everywhere), workers with blocking calls on native |
| Development | `fa develop` with hot reloading, `fa test --watch`, persistent caches |
| Standard library | `std.json`, `std.concurrency`, `std.c` |

### Tier 4: later

`build.cLibrary`, `wasm` and `rust` targets, `fa test --coverage`, `fa test --target=web`, `fa preview --profile`, remote cache. Then async (outside this iteration): tasks, colorless instances, async primitives, workers on JavaScript, `WorkerPool`, `std.http`.

### Dependency order

```
parser follow-ups (diagnostics, export forms, [..] paths)
  └─ analyzer core (names, scopes, types, literals, ranges, narrowing)      [other briefs]
       └─ module scan + name lookup + module-level init order               [15]
       └─ effects and paths                                                  [3]
            └─ reference checker                                             [4, 5]
                 ├─ requirements                                             [9]
                 ├─ structural typing rules                                  [10]
                 └─ copies, moves, destruction                               [6]
                      └─ JavaScript backend core → CLI run/build/check/test, literal config, test runner   [16, 18, 20]
                           └─ native backend M0–M1 (LLVM document)
                           └─ interpreter → static/@ → computed config → per-target analysis               [12]
                                └─ Pool/Columnar/Growing collections → relations                           [8]
                                └─ workspaces, dependencies, lockfile, @import                             [17]
                                └─ language server incremental analysis                                    [21]
                                     └─ reactivity                                                         [11]
                                     └─ foreign code (needs native M1, interpreter for @extern)            [13]
                                     └─ for parallel, thread-safe types, workers (needs native runtime)    [14]
                                     └─ hot reloading (needs incremental analysis + JS backend)            [19]
                                          └─ cLibrary, wasm, rust, coverage, profiling                    [tier 4]
```
