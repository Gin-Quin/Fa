# Compiling Fa with LLVM

This is a **strategy document** for the `native` and `wasm` targets. It is not a language specification: the pages of `documentation/` stay the source of truth, and nothing here changes the language. Where the documentation is silent or ambiguous, this document says so and proposes an answer in [Questions to decide](#questions-to-decide), instead of choosing silently.

It is organized around the five goals:

1. **Compile fast**: cached modules and libraries, simple LLVM usage, a very short path from the frontend to the backend.
2. **Run fast**: output as close as possible to hand-written C, Rust or Zig.
3. **Stay simple**: as few mechanisms as possible, each one reused everywhere.
4. **Structural typing built for LLVM**: the feature is designed into the data layout, not emulated on top of it.
5. **Spend the language's guarantees**: Fa's safe memory model removes runtime checks that other languages must keep.

Goals 2 and 5 are the same goal seen from two sides, and goal 1 is mostly won by goal 3: the fewer distinct things the compiler does, the less it has to do twice.

## Summary of the strategy

| # | Decision | Recommendation |
| --- | --- | --- |
| 1 | Pipeline | One shared **checker**, then for native: **monomorphize**, **analyze**, **lower straight to LLVM**. No CFG-based middle IR. |
| 2 | LLVM interface | The LLVM C API through a thin wrapper over `llvm-sys`, in-process, one context per thread. Text IR is only an output (`--emit=llvm-ir`, golden tests). |
| 3 | Caching | Content-addressed. **Unit of reuse = codegen unit** (a group of monomorphic functions), keyed by the hashes of the sources, of the *interfaces* it uses, and of the build configuration. Sound because compile-time evaluation is deterministic. |
| 4 | Generics and structural views | **Monomorphization keyed by layout**, with a second, coarser dedupe on the generated IR. Types are hash-consed by *shape*, so `Human` and `Animal` are the same instance. |
| 5 | Object layout | Fields **sorted by alignment, then by name**: a layout depends only on the shape, never on the declaration order. |
| 6 | Memory | Objects in `alloca`, collections on the heap, **no runtime tracking of any kind**: no counters, no generations, no barriers, no unwinding. Everything the checker proved becomes an LLVM attribute. |
| 7 | Collections | Known designs: Vec (`Array`), Swiss table (`Set`), compact dict with stable slots (`Map`), slab/hive (`Bag`), struct-of-arrays (`Columnar`), segmented list (`Growing*`), intrusive lists (relations). Written in Fa on top of a small internal intrinsic layer, so the monomorphizer and the cache serve them too. |
| 8 | Errors | Errors are values: `throws` is an extra return path. No exceptions, no landing pads, `nounwind` everywhere. |
| 9 | Dev loop | `fa develop` stays on JavaScript. Native dev builds use LLVM `-O0` (FastISel) and `lld`. |

## Contents

1. [Architecture](#architecture)
2. [Compiling fast](#compiling-fast)
3. [Running fast: what the language lets LLVM assume](#running-fast-what-the-language-lets-llvm-assume)
4. [Data layout](#data-layout)
5. [Structural typing on LLVM](#structural-typing-on-llvm)
6. [Memory management](#memory-management)
7. [Collections](#collections)
8. [Errors and runtime checks](#errors-and-runtime-checks)
9. [Concurrency, targets and compile-time code](#concurrency-targets-and-compile-time-code)
10. [Testing and validation](#testing-and-validation)
11. [Milestones](#milestones)
12. [Questions to decide](#questions-to-decide)

---

## Architecture

```
 source (.fa)
     │  tokenize, parse                      packages/parser (exists)
     ▼
 syntax tree (arena, indexed)
     │  resolve names, check types, ranges, effects, references
     ▼
 checked tree  ── JavaScript backend (structured emit, no layouts, no monomorphization)
     │
     │  native / wasm only
     ▼
 monomorphize  (demand-driven from the entry point, exports and tests)
     │  instances: functions with concrete types and concrete layouts
     ▼
 analyze      (per instance, on the tree: ranges, moves, fusion)
     ▼
 lower        (tree → LLVM IR, scope-stack drops, attributes)
     ▼
 LLVM         (optimize at -O2, or only emit at -O0)  →  object files (cached)
     ▼
 lld          →  executable / .wasm
```

### What is shared with the JavaScript target

Everything up to the **checked tree** is target-neutral: parsing, name resolution, structural typing, ranges, the `mutates` / `invalidates` / `throws` / `requires` effects, reference invalidation, relation validation. This is not an economy, it is the point of the design: memory safety in Fa is a *checking* problem, solved once, and both backends inherit it.

Everything after it is native-only. In particular, **monomorphization and layouts do not exist for JavaScript**, where `Array(Item)` is erased and an object is a JavaScript object. A language feature is acceptable only if it lowers cheaply on both sides. The rule from the design notes applies here: *no feature may force per-value wrapping on a JavaScript hot path*. Each design choice below is checked against it, and where a native technique has a JavaScript cost, it is stated.

### Why no middle IR with a control-flow graph

Rust has MIR, Swift has SIL, Clang has none. Fa's structure favours Clang's route:

- Control flow is **structured**: no `goto`, no labeled loops, `break` and `continue` act on the innermost loop. Scope exits are lexical, so dropping values at scope exit is a stack of cleanups, not a dataflow problem.
- Borrow checking is **not** done on the native side. The checker did it on the language level, for every target, without a CFG.
- The analyses that remain after monomorphization (ranges, last use, iterator fusion) are simple forward or backward walks over structured code.

So the **monomorphic tree** (the checked tree after type substitution, still arena-indexed like `TypedSyntaxTree`) is the only IR. One fewer representation to design, build, hash, serialize and debug. A CFG IR can be added later if an analysis needs it; nothing in this plan prevents it.

### Crates

| Crate | Role |
| --- | --- |
| `fa-parser` | exists |
| `fa-checker` | names, types, ranges, effects, references. Shared by all targets. Also feeds the language server. |
| `fa-native` | monomorphization, layouts, analyses, lowering. Knows nothing about LLVM's C API: it talks to a `Backend` trait. |
| `fa-llvm` | the `Backend` implementation over `llvm-sys`, the pass pipeline, object emission. |
| `fa-runtime` | a `staticlib` with a C ABI: allocator, panic, I/O, threads, async. Built once, embedded in the `fa` binary. |
| `fa-link` | `lld` invocation and the cache directory. |

The `Backend` trait is small (types, functions, blocks, loads, stores, arithmetic, calls, attributes). It exists to keep `fa-native` testable without LLVM installed (a recording backend that prints text), not to support a second backend.

---

## Compiling fast

The techniques below are ordered by how much time they save, which is also the order to implement them in.

### 1. Do less: demand-driven compilation

Only what is reachable from the entry point, the exports being built and the tests being run is monomorphized and lowered. Libraries are distributed as source (see [modules](documentation/architecture/modules.md#building-a-library)), so an application using one function of a large library compiles one function.

### 2. A front end designed to be parallel

Fa's signature rules give the checker an unusual property: **a function body never changes any signature except its own, and only when the body is a single expression.**

- A block body must declare its return type, `throws`, `requires`, `mutates` and `invalidates`.
- A single-expression body infers them, but it is small, and checking it can be done on demand.

So the checker works in two phases: first collect every signature (cheap, embarrassingly parallel by file), then check bodies **in parallel**, each body only reading signatures. The only ordering constraint is among *inferred* single-expression functions that call each other. The documentation does not say what happens when they are mutually recursive; the simplest rule is to require a signature on one of them.

Other front-end rules:

- **Hash-consed types.** Every type is interned once into an arena and referred to by a `TypeId`. Equality is an integer comparison, a type is its own cache key, and structural identity (below) is free.
- **Arena everything.** The parser already stores nodes by index. Keep it: no `Box`, no `Rc`, no per-node allocation; one bump arena per compilation, freed at exit.
- **One pass per concern.** No pass re-walks the tree for a fact a previous pass could have recorded.

### 3. A thin, in-process LLVM

How to talk to LLVM, in order of simplicity:

| Option | Pros | Cons |
| --- | --- | --- |
| Emit `.ll` text, run `clang` / `llc` | No build dependency on LLVM, trivial to debug | Process spawn, text printing and parsing on every unit, an external toolchain to ship |
| **Raw C API via `llvm-sys`, thin wrapper of our own** | In-process, no text, fastest path, small surface, handles are plain indices in arenas | Needs a pinned LLVM; unsafe code confined to one crate |
| `inkwell` | Safer Rust API | `'ctx` lifetimes leak into every type, lags behind LLVM releases, hides little we would not wrap anyway |
| MLIR | Rich infrastructure | Far more machinery than a language this size needs |

**Recommendation: `llvm-sys`, pinned to one LLVM major version.** Text IR is *derived* (`LLVMPrintModuleToString`) for `--emit=llvm-ir` and golden tests, never used as an interchange format.

Rules for using LLVM simply and quickly:

- **Opaque pointers only.** Every pointer is `ptr`. There are no pointer types to track, which removes a whole class of frontend code.
- **The new pass manager only**, through `LLVMRunPasses(module, "default<O2>", ...)`. No custom pass ordering. Fa gets its optimization from *facts handed to LLVM* ([next section](#running-fast-what-the-language-lets-llvm-assume)), not from custom passes.
- **No exceptions.** No `invoke`, no landing pads, no personality function, no `.eh_frame` for the language's own errors. This removes a large amount of IR and a whole LLVM pipeline stage.
- **Aggregates by `memcpy`**, never by first-class aggregate `load`/`store` beyond two scalars. Big aggregate loads and stores are a known source of slow LLVM compiles.
- **One `LLVMContext` and one `TargetMachine` per worker thread.** Contexts are independent, so codegen units compile in parallel with no locking.
- **Never emit what LLVM would delete.** No dead blocks, no unused declarations, no redundant casts: they cost time even when removed.

### 4. Two pipelines

| Profile | LLVM | Intended for |
| --- | --- | --- |
| `dev` | `-O0`, FastISel, no mid-end passes, lazy debug info | `fa develop --target=native`, `fa test` |
| `release` | `default<O2>` (O3 selectable), loop and SLP vectorization, target CPU baseline | `fa build` |

`fa develop` stays on JavaScript and Bun by default ([hot reloading](documentation/architecture/hot-reloading.md)); the `dev` profile only matters when the native target is explicitly requested, and there the cost is dominated by instruction selection, hence FastISel.

A possible later refinement, **not recommended now**: a Cranelift backend for `dev`, behind the same `Backend` trait. It would typically compile faster than LLVM `-O0`, at the price of a second backend to maintain and keep bit-identical in behavior. Revisit only if `dev` compile times become the bottleneck.

### 5. The cache

The compiler is a build system with a deterministic function from inputs to objects, so it caches like one.

**Determinism is already specified**: compile-time evaluation cannot read the clock, the network or random numbers, and the files it reads become build inputs ([metaprogramming](documentation/advanced/metaprogramming.md#determinism)). The remaining sources of non-determinism are ours to avoid: iteration over hash maps (use insertion-ordered or sorted maps), absolute paths and timestamps in IR or debug info (remap them), and thread scheduling (partition deterministically).

#### Units

| Level | What | Why |
| --- | --- | --- |
| Source unit | a `.fa` file (one exported entity) | Fa files are tiny and few of them change at a time |
| Codegen unit | a group of monomorphic functions, compiled into one object | A file per object would mean thousands of tiny objects and a slow link |
| Instance | one monomorphic function (function + concrete types + concrete layouts) | the granularity of deduplication |

Codegen units are formed **by namespace**: the functions of one folder, split when a unit exceeds a size budget, with the split decided by a stable hash of the instance key so that adding one function does not reshuffle the others. A unit is recompiled when any of its members changes.

#### Keys

```
unit key = hash(
	compiler version, LLVM version, target triple, CPU, profile, optimization flags,
	for each member instance:  hash of its monomorphic tree,
	for each dependency used:  hash of its *interface*, plus the hash of every body
	                           it exposes for inlining
)
```

- The **interface hash** of a function covers its complete signature: types, `throws`, `requires`, `mutates`, `invalidates`. Because block bodies must declare all of these, the interface is mostly written by the programmer, and *editing a body without touching its signature recompiles that body's unit only*. Callers are untouched.
- A function small enough to be **inlined across modules** (a single expression, or under a size threshold) exposes its body: it is part of its interface hash, and callers re-emit it locally (`linkonce_odr`, or `available_externally`) so LLVM can inline it. Editing such a body recompiles its callers, which is the price of cross-module inlining. Everything else is opaque to callers.
- Types are keyed by **shape** (below), so two packages that define the same structural type share instances.

#### Where the cache lives

- A **local, content-addressed** directory (`~/.cache/fa/`), shared by every project of the machine: `key → object`.
- Libraries are shared as source and pinned by the lockfile hash ([dependencies](documentation/architecture/dependencies.md#lockfile)), so a library's non-generic units have a key independent of the application. They are compiled once per (library version, configuration) and reused by every project.
- **Generic instances** are keyed by (generic function, canonical type arguments, layouts), not by the project using them. `Array(Integer).push` is compiled once per machine, per compiler version.
- A **remote cache** is the same store behind a URL. It is sound for the same reason the local one is.
- The toolchain can ship **pre-built instances** of the common standard collections (`Array(Byte)`, `Array(Integer)`, `Map(String, String)`...).

### 6. Linking

Linking is the next bottleneck after compilation. Bundle **`lld`** (it also provides `wasm-ld`), so the toolchain has no dependency on a system linker and one fast linker serves every target. Pass `--icf=safe`, which merges byte-identical functions: a cheap safety net for the [instance deduplication](#4-instance-deduplication) below. The linker is called in-process as a library, not through a system `cc`.

### 7. A budget

Compile time deserves the same discipline as run time. Track, in CI, the time of the phases (parse, check, monomorphize, lower, LLVM, link) on a fixed corpus, and fail a change that regresses one beyond a threshold. Do not guess where the time goes: the first milestone adds the timers.

---

## Running fast: what the language lets LLVM assume

Most of Fa's performance does not come from clever code generation. It comes from **handing LLVM facts that C and C++ compilers have to guess**, because the checker already proved them.

### Checks Fa never emits

| Danger | Typical cost elsewhere | In Fa |
| --- | --- | --- |
| Dangling reference | garbage collector, reference counts, generation counters, borrow checker | tracked statically ([owned references](documentation/advanced/owned-references.md)); relations are cleaned at deletion in O(k) |
| Iterator invalidation | version counters (Java, Python), undefined behavior (C++) | compile error |
| `null` | null checks | no null: optionals are explicit unions |
| Data race | locks everywhere, or undefined behavior | values are copied across threads; only thread-safe types are shared |
| Out-of-bounds index | check on every access | removed when the index range is proven; the rest is a typed error |
| Integer overflow | wrap, or check on every operation | removed when the result range is proven |
| Division by zero | check on every division | divisor proven non-zero, or a typed error |
| Non-exhaustive `when` | a default arm | compile error: the `switch` default is `unreachable` |
| Use of uninitialized data | zero-fill, or undefined behavior | definite initialization: every load is `noundef` |
| Exceptions | unwind tables, landing pads, cleanup code on every call | errors are values |

### How a proven fact reaches LLVM

| Fa knows | LLVM encoding |
| --- | --- |
| A parameter is an immutable reference | `ptr noalias readonly nocapture nonnull noundef align A dereferenceable(N)` |
| A parameter is `mutates` | `ptr noalias nocapture nonnull noundef align A dereferenceable(N)` (needs [Q3](#q3-may-a-mutated-place-be-aliased-by-another-argument)) |
| `Integer(min = a, max = b)` | `range(i64 a, i64 b+1)` on parameters and returns, `!range` on loads |
| A sum cannot overflow | `add nsw` / `nuw` |
| A sum can overflow | `llvm.sadd.with.overflow`, then a branch with weights marking the overflow path cold |
| An index is proven in bounds | `getelementptr inbounds`, no comparison at all |
| An index is not proven | `icmp ult` against the length, branch to the typed-error path (cold) |
| A function only reads its parameters | `memory(argmem: read)` (only when it also allocates nothing and calls nothing impure); `memory(read)` if it also reads module-level values |
| No error can unwind | `nounwind` on every function, including the runtime's |
| An exhaustive `when` | `switch` with an `unreachable` default |
| A `throws` path | branch weights, the error continuation block marked cold |
| Runtime allocation entry points | `allockind` and `"alloc-family"` on `fa_alloc`, `fa_realloc`, `fa_free`, so LLVM may remove unused allocations and pair them with their frees |
| A flat value (see below) | no drop code at all |

`dereferenceable(N)` covers the bytes the function actually touches, which keeps the IR of equivalent specializations identical (see [instance deduplication](#4-instance-deduplication)).

### Fa's effects are LLVM's function attributes

`mutates` is a finer, checked version of what LLVM calls `memory(...)`. The function-level attributes are derived from the effects, never guessed:

- no `mutates`, no module-level write, no I/O, no allocation: `memory(argmem: read)` (or `memory(read)` if module-level values are read);
- `mutates self.score`: a write to one place of one parameter. Parameter-level `noalias` is the sound first step; field-level `!alias.scope` metadata is possible later.

The finer the effect, the more LLVM can reorder around the call. This is the payoff of the principle that effects are as fine-grained as possible.

---

## Data layout

### Scalars

| Fa | LLVM | Notes |
| --- | --- | --- |
| `Int8`..`Int128`, `UInt8`..`UInt128`, `Integer` (`Int64`), `Byte` | `i8`..`i128` | `i128` needs the `compiler-rt` builtins (`__divti3`...) in the runtime |
| `Integer(min, max)` | `i64` with `range` | Storage is the base type; the range is a fact, not a representation. Sized types exist for compact storage. |
| `Float`, `Float32` | `double`, `float` | |
| `Boolean` | `i1` in registers, `i8` in memory | |
| `Percentage` | `double` | its own type, a plain float at runtime |
| `BigInt` | pointer to a runtime object | arbitrary precision lives in the runtime |
| unions of literals, `Enum` | the smallest integer holding the member count | |
| `None` / `Void` | zero-sized, absent from signatures | |

### Strings

A `String` is `{ ptr, length in bytes, capacity | flags }`, 24 bytes, UTF-8.

- **Static strings** (literals) live in read-only data, with a "static" flag in the capacity word. They are never freed, and a first mutation allocates and copies. Since identity is not observable, this is invisible, and it makes the most common string, the literal, free to "copy".
- **An "ASCII only" flag** is maintained on append (a cheap vectorized check of the new bytes). It makes `length` and indexing O(1) in the common case, and falls back to grapheme segmentation otherwise. `length` counts grapheme clusters and indexing is O(n) in general ([strings](documentation/values/strings.md#indexing)); the flag removes that cost wherever it can be removed. See [Q5](#q5-string-representation-and-the-cost-of-grapheme-length).
- **A slice** (`use word = text[0..5]`) is `{ ptr, length }`: a view, never owning, tracked by the invalidation rules.
- The Unicode tables for grapheme segmentation are generated into the runtime at build time.

### Objects and tuples

An object is a struct stored inline: in an `alloca`, in a parent object, in a collection slot. A tuple is the same, with positional fields. A single-field object has exactly the layout of its field, so the "distinct id types" idiom of the [structural typing](documentation/advanced/structural-typing.md#when-you-want-types-to-be-distinct) page costs nothing.

**Field order is canonical**: sorted by alignment (largest first), then by field name. See [structural typing on LLVM](#structural-typing-on-llvm) for why this, and not declaration order. Methods are not stored (they are static functions); function-pointer fields are stored.

### Unions and optionals

| Kind | Representation |
| --- | --- |
| `Union { ... }`, `Errors { ... }` (named) | `{ tag, payload }`, tag = declaration index, the smallest integer that fits, payload sized and aligned for the largest member. A single member needs no tag. |
| `A \| B \| C` (anonymous) | the same, with tag = index in the **canonical order of the member types** (sorted by `TypeId` shape hash), so `A \| B` and `B \| A` are one type. |
| `T?` | a union with `None`, with **niche optimization**: when `T` has an invalid bit pattern, `none` uses it and no tag is stored. |

Niches come free from many Fa types: a non-null pointer (arrays, strings and collections always hold a non-null pointer, an empty array points at a dangling aligned address as in Rust), an enum with spare values, a `Boolean`, a ranged integer (`Integer(min = 0)?` can use -1), a stored reference. `Optional(Integer)` is the only common optional that needs a tag.

### Functions and calls

- Internal functions use `fastcc` and `internal` linkage; exported ones and the runtime use the C convention. Parameters of aggregate type are passed **by pointer** ([parameters are references](documentation/advanced/memory-management.md#parameters-are-references)); results of up to two scalars are returned in registers, anything larger through `sret`.
- **`throws`** makes the result the union `T | E`, laid out as above. The caller's `try` is one compare on the tag, with the error path cold. See [errors](#errors-and-runtime-checks).
- **`requires`** is dictionary passing: each requirement is a hidden pointer parameter appended in a canonical order (by the requirement's `TypeId`). They are resolved at compile time by `provide`, and a `provide` is a stack slot or a reference to an existing value. There is no lookup at runtime. Function types carry their requirements, so hidden parameters are part of the ABI of a function value.
- **Function values** are `{ fn, env }`. A call whose callee is known at compile time is a direct call; so is every lambda passed to a generic function (`map`, `filter`...), because each lambda is its own type and the callee is specialized on it, like a C++ template or a Rust closure. Only a function stored in a field, or in a collection, is an indirect call.
- **Closures capture by copy**, so the environment is a struct of the captured copies. A closure that does not escape keeps its environment in the caller's frame. An escaping closure needs a heap block owned by the closure value: see [Q4](#q4-where-does-the-environment-of-an-escaping-closure-live).

### Module-level values

A module-level value is an LLVM global with a stable address, which is exactly what the language promises ([module-level values](documentation/values/declaring-values.md#module-level-values)).

- A value computable at compile time (literals, `static`, constant objects) is emitted as constant data and needs no initialization code.
- Other values are initialized by a generated function, in the topological order of the module dependency graph, before `main`.
- `export mutable` is replaced in place: assignment destroys the old value and writes the new one to the same address. Unlike JavaScript, which needs a `{ value }` wrapper, native code needs nothing.

Two points are unspecified in the documentation: initialization cycles, and what a worker thread sees. See [Q8](#q8-module-level-mutable-values-and-threads).

---

## Structural typing on LLVM

This is where Fa differs most from Rust, C++ and Zig, whose types are nominal and whose layouts therefore never need to *agree across names*. In Fa:

- two types with the same fields are the same type ([`Human` and `Animal`](documentation/advanced/structural-typing.md));
- a type with *more* fields can be passed where fewer are expected, **without a copy and without truncation** (parameters are references; `use` aliases do not copy);
- generic constraints are shapes (`Entity: { id: Integer }`);
- anonymous unions are structural.

A naive native implementation makes all of this expensive (copies, vtables, offset tables). The strategy below makes each case either free or a small, local, explicit cost.

### 1. Identity is the shape

Types are hash-consed by **shape**: the sorted set of `(field name, field type)` pairs, recursively. The name a type was declared with is a label in a side table, used for diagnostics and `@typeToString`, and is not part of the identity. Consequences:

- `Human` and `Animal` are one `TypeId`: one layout, one set of instances, one cache entry.
- Two packages defining the same structural type share compiled code.
- Equality, hashing and copying are generated **per shape**: `eq<Human>` and `hash<Human>` exist once.

Named unions, `Errors` and `Enum` are the exception, as the language intends: their identity includes their declaration, because "`MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match".

### 2. Layout depends on the shape only

Layout is computed from the canonical field order (alignment descending, then name). Declaration order, spread order and type name play no role. So two structurally equal types always have the same layout, and **no conversion is ever needed between equal shapes**.

Alternatives considered:

| Layout | Pros | Cons |
| --- | --- | --- |
| **Sorted by alignment, then by name** | minimal padding, canonical, same code for equal shapes | a superset is not a byte-prefix of its subset in general |
| Declaration order, spreads first | a spread child is a byte-prefix of its parent, so views are free by construction | padding depends on how the author wrote the type; equal shapes with different orders have different layouts and need adapters |
| Per-field separate arrays | no padding | `Columnar`, not an object layout |

**Recommendation: sorted.** Prefix compatibility is a convenient accident of the second option, not a requirement: the next section handles the general case at no runtime cost. See [Q1](#q1-canonical-field-order-and-the-c-abi).

### 3. Passing a superset where a subset is expected

Given a parameter of declared type `D` and an argument of actual type `S ⊇ D`, the callee can only touch fields of `D` ("the narrowed view is a compile-time restriction, not a runtime transformation"). So its machine code depends on exactly one thing about `S`: **the offset and type of each field of `D` inside `S`**. That is the *view*.

| Situation | Lowering | Runtime cost |
| --- | --- | --- |
| `D` and `S` have equal shapes | pass the pointer | none |
| `S ⊇ D`, and `D`'s fields sit at the same offsets in `S` | pass the pointer, reuse the instance | none |
| `S ⊇ D`, offsets differ, **direct call** | **specialize the callee on the view**: a new instance whose field accesses use `S`'s offsets | none (one more instance, cached) |
| `S ⊇ D`, offsets differ, **indirect call** (function value, function-pointer field) | the callee was compiled against `D`'s layout: repack `D`'s fields into a temporary shaped like `D` (a shallow, bitwise copy, no drop) | O(\|D\|) words, only here |
| `let human: Human = hero` | field-wise copy into `D`'s layout (a real copy, as for any assignment) | the copy the language specifies |
| `use human: Human = hero` | no code: the static type stays `Hero`, and `Human` only restricts which fields are visible | none |

An indirect call is only repacked when the signature has no `mutates` and returns no reference into its parameters, because only then is a copy unobservable (identity is not observable in Fa). Otherwise the call is a compile error telling the author to make the function generic ([Q2](#q2-subset-views-through-function-values-and-collections)).

The same rule is already needed for **generic constraints**: `function readId(Item: { id: Integer })` is instantiated on the *full* type of the argument, so there is nothing new to build. A subset parameter is just an implicit generic parameter over the layout of the argument.

### 4. Instance deduplication

Specialization on views could multiply code. Two cheap levels keep it in check:

1. **Instance key** (before lowering, from the types only): `hash(function, per parameter the view of D inside S, generic arguments)`. Two arguments with the same view share an instance, whatever their types. This key is the cache key.
2. **Code hash** (after lowering): hash the generated function. If it equals an existing one, emit an LLVM `alias` instead of a second body. `ageOf(hero)` and `ageOf(human)` below produce the same code and one machine function.

The linker's `--icf` is the last net. None of this needs a footprint analysis of the function body: the work is done by hashing.

### A worked example

```fa
type Human = {
	name: String
	age: Integer
	score: Integer
}

type Hero = {
	...Human
	alias: String
}

function ageOf = (human: Human) => human.age

function scoreOf = (human: Human) => human.score

let hero = Hero { name = "Ana", age = 31, score = 7, alias = "Zap" }

ageOf(hero)
scoreOf(hero)
```

Canonical layouts (`String` is 24 bytes, 8-aligned; every field is 8-aligned, so they sort by name):

| `Human` (size 40) | `Hero` (size 64) |
| --- | --- |
| `0` age | `0` age |
| `8` name | `8` alias |
| `32` score | `32` name |
| | `56` score |

- `ageOf(hero)`: `age` is at offset 0 in both. Same view, **same instance, same code**.
- `scoreOf(hero)`: `score` is at 32 in `Human` and at 56 in `Hero`. Different view, **a second instance**, identical except for one constant:

```llvm
define fastcc i64 @"fa.scoreOf.human"(ptr noalias nocapture readonly nonnull noundef align 8 dereferenceable(40) %human) nounwind {
	%field = getelementptr inbounds i8, ptr %human, i64 32
	%score = load i64, ptr %field, align 8
	ret i64 %score
}

define fastcc i64 @"fa.scoreOf.hero"(ptr noalias nocapture readonly nonnull noundef align 8 dereferenceable(64) %human) nounwind {
	%field = getelementptr inbounds i8, ptr %human, i64 56
	%score = load i64, ptr %field, align 8
	ret i64 %score
}
```

Neither function copies anything, truncates anything, or looks anything up. After inlining (these are single-expression functions, so they are exposed for cross-module inlining), both calls become a single `load`.

### Unions and structural typing

An anonymous union `A | B` passed where `A | B | C` is expected has a different tag numbering (tags follow the canonical order). The same pattern applies:

- **by value**: re-tag with a constant mapping (`tag' = map[tag]`, one load or a `select`);
- **by reference, direct call**: specialize the callee on the actual union, which also gives better code (the `when` only has the actual branches);
- **by reference to a mutated union**: rejected. Writing a `C` into storage sized for `A | B` would be unsound.

Unions of literals (`"north" | "south"`) are plain integers, and widening them is the same mapping.

### JavaScript

None of the machinery above exists for the JavaScript target: objects are dynamic, a view is just a property read, and there is nothing to specialize. Structural typing therefore costs the JavaScript backend nothing, which is the point of keeping layouts out of the shared checker.

### Collections are invariant

`Array(Hero)` is **not** accepted where `Array(Human)` is expected. Strides differ, and a `mutates` array would be unsound. The generic form covers the need and is exactly the example in the documentation:

```fa
function findById(Entity: { id: Integer }) = (entities: Array(Entity), id: Integer): Entity? => {
	for entity in entities {
		if entity.id == id {
			return entity
		}
	}
	none
}
```

`Entity` is the full element type, so the instance is specialized on its stride and offsets: ordinary monomorphization. See [Q2](#q2-subset-views-through-function-values-and-collections) for the one place the documentation does not say.

---

## Memory management

The documentation's model maps almost one-to-one onto LLVM. The compiler's work is mostly *not* emitting things.

### Rules and their lowering

| Rule of the language | Lowering |
| --- | --- |
| Objects live on the stack | `alloca` in the entry block. `mem2reg` and SROA at `-O2` turn most of them into registers. |
| Collections own the heap | one header on the stack, buffers from `fa_alloc` |
| Every assignment is a copy | scalars: a register copy. Flat aggregates: `memcpy`. Owning values: a generated `copy<Shape>` function. |
| A copy becomes a move when the source dies | a **last-use analysis** on the tree (a backward walk of structured code); a moved value is `memcpy`'d and not dropped |
| `use` creates an alias | a pointer; no code |
| Parameters are references | a pointer parameter with the attributes above |
| Iteration yields references | a pointer to the current element |
| Values are destroyed at scope end, in reverse order | a **cleanup stack** while lowering |
| A field holds a copy, or is a relation | inline storage, or a [relation link](#relations) |

### Flat values need no code

The documentation already classifies values as **flat**: a primitive, a union of literals, a stored reference, or an object whose fields are all flat. A flat value owns nothing, so it has **no copy function, no drop, no move bookkeeping**: copying is `memcpy`, and destruction is nothing. Every `Integer`, `Boolean`, every point or color, every flat object. The compiler computes flatness once per shape.

Only values that own memory (collections, strings, recursive types, and objects containing them) have `copy<Shape>` and `drop<Shape>` functions. They are generated per shape, deduplicated by the hash-consing, and called directly.

### Scope exit without unwinding

Control flow is structured, so the lowering keeps a stack of live owning values. At every exit from a scope (falling through, `break`, `continue`, `return`, a `throw` or a `try` propagating) it emits drops for the values declared inside, in reverse order, and branches on. There are no cleanup landing pads and no personality routine.

A value that is *conditionally* moved (moved in one branch only) gets a one-byte drop flag, tested at scope exit. The analysis knows which values need one; most do not.

### References and invalidation cost nothing at runtime

The checker rejects any use of an invalidated reference. Native code therefore contains **no** liveness check, generation counter or tombstone for local references. They are plain pointers, valid by construction.

A reference's representation depends on the collection it points into, which its type names (`Person in people`):

- row-oriented collections (`Array`, `Bag`, `Map`, `Growing*`): a pointer to the element or slot;
- `Columnar`: a slot index. The type already names the columnar, so the reference does not need to carry it.

### Recursive types

The recursive part of `T?` or of a union is a heap block owned by its parent: allocated by the compiler, copied by `copy<Shape>`, freed by `drop<Shape>`. Non-recursive members stay inline. The allocation goes through `fa_alloc`; a per-thread size-class allocator inside the runtime is the optimization to reach for when trees dominate a profile.

### The allocator interface

The runtime exposes `fa_alloc(size, align)`, `fa_realloc(ptr, old, new, align)` and `fa_free(ptr, size, align)`: **sized** deallocation, so a better allocator can be dropped in later. The first implementation sits on the system allocator. Allocation failure aborts with a message. Large arenas (Bag chunks) use aligned `mmap`.

Because the language's references are never interior pointers stored in data, **every Fa value is trivially relocatable**: moving it is a `memcpy`. `Array` growth can use `realloc` and `mremap`.

---

## Collections

The documentation already picks the semantics; this section picks the data structures. Each is a well-known design in C++, Rust or Zig, chosen because it makes the stated invalidation rules *true by construction*.

| Fa | Layout and technique | Borrowed from | Elements move on growth? |
| --- | --- | --- | --- |
| `Array(T)` | `{ ptr, length, capacity }`, doubling growth, inline fast path for `push` | Rust `Vec`, C++ `std::vector` | yes |
| `Set(T)` | open-addressing **Swiss table**: 1 control byte per slot (7-bit hash tag), group probing with SIMD | Abseil `flat_hash_set`, Rust `hashbrown`, Go 1.24 maps | yes (rehash) |
| `Map(K, V)` | **compact dict**: a Swiss-style index of `u32` slot ids, plus entries in a stable chunked arena | Python dict, Zig `ArrayHashMap` | **no** (stable slots, as documented) |
| `Bag(T)` | **slab / hive**: size-aligned chunks, occupancy bitmap, intrusive free list | C++26 `std::hive` / `plf::colony`, `slab` | **no** |
| `Columnar(T)` | one chunked column per field, same slot index across columns | Zig `MultiArrayList`, struct-of-arrays | no |
| `OrderedColumnar(T)` | one `Array` per field | Zig `MultiArrayList` | yes |
| `GrowingArray(T)` | **segmented list**: segments of doubling size, `O(1)` index with `clz` | Zig `SegmentedList`, C++ `std::deque` | **no** |
| `GrowingBag`, `GrowingMap`, `GrowingSet`, `GrowingColumnar` | the same without deletion: pure bump arenas, no free list or bitmap | `bumpalo`, `typed-arena` | no |

The invalidation table in the documentation ("Effects of the collections") and this table are the same table: a collection *declares* `mutates self` when its representation can move elements, and `mutates self.length` when it cannot.

### Array

`push` has an inline fast path (`len < cap`: store and increment) and a cold call to `fa_array_grow(header, element_size, align)`. `capacity = n` in `from` is `reserve`. `get`, `[]` with a proven index, `for` and slicing all compile to pointer arithmetic. Because values are trivially relocatable, growth is a `realloc`.

`insert` and `delete(index)` are a `memmove` of the tail (declared as `invalidates self[index..]`).

### Set

A Swiss table with the value stored inline: the fastest general design, and the documented contract (an `add` may move values) allows it. Probing uses 16-byte SIMD groups (SSE2, NEON; a portable 8-byte word otherwise). `==`, hashing and copy come from the per-shape functions, **inlined into the probe loop** by monomorphization, which a type-erased runtime could not do.

### Map

The documentation requires stable slots: `getOrInsert` declares `mutates self.length` and references survive insertion. A compact dict gives this at the cost the documentation already admits (one indirection per lookup):

```
index   :  control bytes + u32 slot ids       (Swiss-style, may rehash freely)
entries :  chunked arena of { key, value }    (never moves; free list for deleted slots)
```

Rehashing rebuilds only the index; entries and references into them are untouched. Iteration walks the entries, not the table, which is faster and dense. If profiling shows the indirection matters for flat values, a variant storing flat values in the index is possible but would change `getOrInsert`'s declared effect ([Q6](#q6-the-price-of-maps-stable-slots)).

### Bag

The data structure behind relations, entity systems and `add` returning a reference.

- **Chunks** of one compile-time size per `Bag(T)`, aligned to that size (64 KiB or more for large `T`). A header at the start of each chunk holds an occupancy bitmap, a count and the link to the next chunk.
- `add`: pop the free list, else bump the current chunk, else allocate a chunk (cold). It returns the slot pointer.
- `delete(ref)`: `drop<T>`, clear the occupancy bit (the chunk header is `ptr & ~(chunk_size - 1)`, so no search), push the slot on the free list (the next pointer is stored in the dead slot, so a slot is at least 8 bytes).
- Iteration: for each chunk, for each 64-bit occupancy word, `tzcnt` to the next live slot. A Bag of flat elements is destroyed by freeing its chunks without visiting the elements.
- `deleteWhere` and `clear` visit the occupancy bits.

### Columnar

One chunked column per field. A slot has the same index in every column, so a reference is that index. `for { age } in columnar` touches **one** column: a contiguous run per chunk, which LLVM vectorizes. Reading a whole element is a gather, as the documentation says. Deletion marks the slot free in a shared occupancy bitmap and recycles it. `OrderedColumnar` uses one `Array` per field.

An alternative for module-level singleton collections, not for the first version: reserve each column in virtual memory so that the address is `base + index * size`, with no chunk table.

### Growing collections

Nothing is deleted, so there is no free list, no bitmap and no per-slot state:

- `GrowingBag`, `GrowingColumnar`: a bump pointer in the current chunk; destroying them frees the chunks (and visits elements only if they own memory).
- `GrowingArray`: a segmented list. With a first segment of `2^k` elements, element `i` lives in segment `s = floor(log2((i >> k) + 1))` at offset `i - ((2^s - 1) << k)`: two shifts, one `clz`, one load of the segment pointer. Elements never move, so references are valid for the collection's lifetime.
- `GrowingSet`, `GrowingMap`: an arena of entries plus an index; values are stable even though the index rehashes.

### Iteration and fusion

`for` over a collection lowers to a counted or chunked loop with a pointer to the element, no iterator object. A chain such as

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

is rewritten **by the compiler, on the tree**, into one loop with the lambdas inlined and no intermediate array. This is deliberate: relying on LLVM to inline layers of iterator adaptors (as Rust does) makes IR big and compilation slow; the front end can do it directly because `filter`, `map` and `reduce` are known operations. A chain that is broken across statements is not fused, exactly as the [performance guide](documentation/guides/performance.md#let-chained-methods-fuse) says.

### Hashing

`hash<Shape>` is derived per shape, structurally, like `==`. The hash function is fast and non-cryptographic with a **per-collection random seed** (as Go does): resistant to hash flooding without paying for SipHash, which matters for the server use cases of the documentation. Iteration order is documented as "not guaranteed", which allows this; tests can fix the seed.

### Relations

[Relations](documentation/advanced/relations.md) become intrusive links in the elements themselves, with no allocation and no hidden index:

| Kind | Direct side stores | Reverse side stores |
| --- | --- | --- |
| one-to-many (`bestFriend` / `bestFriendOf`) | target pointer, `prev`, `next` | list head, count |
| one-to-one (`mentor` / `mentee`) | target pointer | target pointer |
| symmetric (`spouse`) | target pointer | itself |
| many-to-many (`friends` / `friendOf`) | head of an **edge list** | head of an edge list |

A many-to-many edge is a small node `{ a, b, links in a's list, links in b's list }` allocated from a per-relation slab. Writing `ana.bestFriend = cleo` is the three steps of the documentation, each a constant number of pointer writes. Deleting an object runs a generated `unlink<Shape>`: unlink the direct sides in O(1), and walk each reverse list in O(k) to set the referrers to `none`. When both sides of a relation die together (the owner of both collections is destroyed), the unlinking is skipped.

This is the reason relations require stable-address collections: the links are raw pointers into slots that never move. In JavaScript the same links are ordinary object fields, so the design adds no wrapper there either.

### How the collections are written

**Recommendation: in Fa, in the standard library, on top of a small set of internal intrinsics** (raw allocation, pointer arithmetic, load, store, `memcpy`, `clz`), available only to the standard library. Reasons:

- the same monomorphizer, cache and attribute logic serve user code and the standard library, which is the "one mechanism" goal;
- `hash`, `eq` and `copy` for the element type are *inlined* into the collection code, which a type-erased Rust runtime cannot do;
- a precompiled instance such as `Map(String, Integer)` is shared by every program through the cache.

The alternative is hand-written IR builders in the compiler, one per operation, with a type-erased Rust runtime for the cold paths. It needs no internal `unsafe` dialect, but it is more code, in the less testable place. The cost of the recommendation is a design for that internal layer ([Q7](#q7-the-internal-intrinsic-layer-for-the-standard-library)).

For JavaScript, none of this applies: collections are JavaScript structures, as the documentation says.

---

## Errors and runtime checks

### Errors are values

`throws E` makes the result a union `T | E` and `try` a branch on its tag. Nothing unwinds, so:

- no landing pads, no personality function, no exception tables;
- `nounwind` on every function;
- the error path is an ordinary, cold, predicted-not-taken branch.

Swift's `swifterror` (the error in a dedicated register) would make the happy path marginally cheaper, but it only carries a pointer-sized error. Fa's errors carry typed payloads, so the plain union is simpler and almost as fast. Do not optimize this before measuring.

### The checks that remain

| Check | When | Result |
| --- | --- | --- |
| Index out of bounds | the index range is not proven | typed error `IndexError` |
| Key missing | presence is not proven | typed error `KeyNotFound` |
| Division by zero | the divisor is not proven non-zero | typed error `DivisionByZero` |
| Possible overflow | the result range is not proven | **stops the program** (cold call, `noreturn`) |
| Allocation failure | always | stops the program |
| Stack overflow | always | guard page |

Everything else was removed by the checker. The range analysis ([numbers](documentation/values/numbers.md#ranges-through-arithmetic)) runs in the checker for concrete code and again, cheaply, per monomorphic instance whose ranges depend on generic parameters (`Matrix(Float, 4, 4)` bounds the indices of `get` by `rows - 1`). Each emitted check consults a "proven" bit recorded on the tree node.

### Verify mode

The compiler's claims are only as good as the checker. A `--verify` build turns every *removed* check back into an assertion with a distinct message ("compiler bug: index proven in bounds was not"), and every attribute that asserts a fact (`noalias`, `range`) into a runtime test where one can be written. It is for CI and fuzzing, not for production, and it is the cheapest way to find an unsound proof. See [testing](#testing-and-validation).

---

## Concurrency, targets and compile-time code

### Async, workers and `for parallel`

The documentation names Tokio and Rayon for the native target. Both are Rust libraries and cannot be called from LLVM-generated code directly, so they live inside `fa-runtime` behind a C ABI.

- **`for parallel`**: the loop body is outlined into a function, and the loop becomes `fa_parallel_for(count, body, env)`, a thin shim over Rayon's chunked parallel iteration. The compiler's proof that writes are disjoint is what makes the outlined body free of synchronization.
- **Workers**: a thread with its own state. Arguments and results are copied across the boundary (the language's ordinary copy), through a channel in the runtime.
- **`async`**: an `async` function is lowered to an **LLVM coroutine** (`llvm.coro.*`, switched-resume lowering), and the runtime wraps the coroutine handle in a task polled by Tokio's executor. Colorless async is a front-end matter: the checker requests a synchronous and/or an asynchronous instance of the function, exactly as the JavaScript backend emits `___sync` and `___async`.
- **Frames**: a coroutine frame is allocated by the runtime when the task starts. This is a hidden heap allocation per task, which the memory-management page does not mention: see [Q4](#q4-where-does-the-environment-of-an-escaping-closure-live).

This is the last milestone: it is also the one with the most design left open ([Q9](#q9-the-native-runtime-and-the-documented-use-of-tokio-and-rayon)).

### Targets

| Target | LLVM | Notes |
| --- | --- | --- |
| `native` | host triple (x86-64 and AArch64 first), `lld`, system libc to start | the primary target |
| `wasm` | `wasm32-unknown-unknown`, `wasm-ld` | same lowering; the runtime provides its own allocator (no libc); no `for parallel`, threads only with shared memory |
| `rust` | not LLVM | emits Rust source; independent of this document |
| JavaScript | not LLVM | the checked tree, structured emit |

Getting `wasm` is almost free once `native` works, which is a strong argument for LLVM over a hand-written backend. Features a target lacks are already compile errors, per the documentation.

### Compile-time evaluation

`static`, `@if`, `@for` and `@function()` need an evaluator at compile time. The documentation describes **an interpreter shared with `fa run`**. Keep it that way: an interpreter over the checked tree has no startup cost, and compile-time code is usually small. A JIT through LLVM's ORC would pay for itself only on very heavy `static` computations, and can be added behind the same entry point if one appears. Compile-time code is a build input like any other: its results are cached by the hash of the code and of the files it read.

### Reactivity and requirements

`reactive`, `derived` and `watch` are runtime-library features (a dependency graph) lowered from the checked tree, and are out of scope for this first plan. `requires` / `provide` are covered above.

---

## Testing and validation

The tests that matter most are the ones that compare Fa against itself.

- **Differential testing against JavaScript.** The JavaScript backend is the reference implementation of the semantics. Run every example of `documentation/` (the same blocks validated by the parser) on both backends and compare outputs. A difference is a bug in one of them.
- **Verify mode** ([above](#verify-mode)) in CI, on the test corpus and on fuzzed programs: it finds unsound proofs.
- **Sanitizers.** The runtime and generated code are run under AddressSanitizer and leak detection. Fa promises no leaks and no use-after-free; the sanitizer is the independent check of that promise.
- **Golden IR tests.** Lower small programs and compare the text IR with committed files. These pin the attributes and the layouts, which is where regressions hide. Most run on the recording `Backend`, whose output is LLVM-syntax text, so they need no LLVM; a smaller set goes through real LLVM and its verifier (`--emit=llvm-ir`).
- **Layout tests.** For a corpus of types: equal shapes have equal layouts, every view offset is in bounds, and `size`/`align` match what LLVM's data layout reports.
- **Structural-typing tests.** The cases of the [view table](#3-passing-a-superset-where-a-subset-is-expected), including the instance count and alias count for the worked example.
- **Cache tests.** Change a body: one unit rebuilds. Change a signature: its callers rebuild. Rebuild twice: byte-identical objects.
- **Performance tests.** A small benchmark suite per collection, compared against the equivalent C++, Rust or Zig, with the compile-time budget from [above](#7-a-budget).

---

## Milestones

Each milestone ends with something that runs, and adds the timers and golden tests for what it introduced.

| # | Content | Done when |
| --- | --- | --- |
| **M0** | `fa-checker` skeleton on the parser's tree; `Backend` trait with the recording backend; integers, booleans, functions, `if`, loops, `fa-runtime` with `print` and `panic`; `fa build --target=native` through `llvm-sys` and `lld` | a Fibonacci program compiles, runs, and its IR is a golden test |
| **M1** | objects, tuples, optionals, unions, `when`, `throws` / `try` / `catch`; layouts; flat vs owning; copies, moves and the cleanup stack; `String`, `Array` | the examples of `values/` and `control-flow/` run, with ASan clean |
| **M2** | generics and monomorphization; the structural view machinery and instance dedupe; the unit cache (local) | the worked example above produces two instances and one alias; a body edit rebuilds one unit |
| **M3** | `Set`, `Map`, `Bag`, `Columnar`, `Growing*`; relations; fusion; the standard library on intrinsics | the examples of `collections/` and `advanced/relations` run; benchmarks vs C++/Rust/Zig exist |
| **M4** | the `release` pipeline; cross-module inlining; parallel codegen; remote cache; verify mode in CI | compile-time budget in CI; differential tests green |
| **M5** | async, workers, `for parallel`; `wasm` | the examples of `advanced/concurrency` run on `native` |

---

## Questions to decide

The documentation is the specification, so these are listed, not applied. Each has one recommendation. The first five change what the compiler must do; the last ones can wait until their milestone.

### Q1. Canonical field order and the C ABI

The documentation says an object is "a container that associates named fields with values" and says nothing about memory order. The plan reorders fields (alignment, then name).

- **Yes (recommended).** Equal shapes share layout and code; padding is minimal. The cost: objects cannot be handed to C as they are.
- No: keep declaration order. Free spread-prefix views, but equal shapes can differ in layout.

The C interop story (if any) is not documented. If it is wanted, an explicit `repr`-like marker on a type would opt out of reordering for that type only. Is there a C interop requirement for `native`?

### Q2. Subset views through function values and collections

Two cases the documentation does not state:

1. **Function values.** A value of type `(human: Human) => Void` is called with a `Hero` whose layout is not `Human`-compatible. Proposed: allowed with a shallow repack when the signature has no `mutates` and returns no reference into its parameters, otherwise a compile error that suggests a generic function.
2. **Collections.** Is `Array(Hero)` accepted where `Array(Human)` is expected? Proposed: **no**, collections are invariant in their element type, because strides differ and a `mutates` array would be unsound. Generic constraints (`Entity: { id: Integer }`) are the supported way.

Do you want both rules? Option 2 means a function taking `Array(Human)` cannot be handed an `Array(Hero)`, which TypeScript allows.

### Q3. May a mutated place be aliased by another argument?

`noalias` on a `mutates` parameter, and `readonly noalias` on the others, is the largest single optimization available, and it needs a rule the documentation does not state:

> A place passed to a `mutates` parameter must not overlap any other argument of the same call, nor a `requires` value used by the callee.

`addBonus(player)` while another argument is `player.score` would be a compile error, as in Rust. The `use` aliases remain free to alias each other (they are plain pointers); only calls are constrained.

**Recommendation: adopt the rule.** If it is rejected, the compiler cannot mark mutated parameters `noalias`, and read-only parameters lose `readonly` whenever the function also mutates something else reachable from them.

### Q4. Where does the environment of an escaping closure live?

"Objects are allocated on the stack. The only other heap allocation is made by the compiler for recursive types." A closure that is returned or stored (`makeGreeter`) needs its captured copies to outlive the frame, and an async task needs a frame.

**Recommendation:** document a second compiler-made allocation: the environment of an *escaping* closure and the frame of a task are heap blocks owned by the closure or task value, copied and freed with it. Non-escaping closures keep their environment on the stack, which covers `map`, `filter` and every callback passed to a function that does not store it.

### Q5. String representation and the cost of grapheme `length`

`length` counts grapheme clusters and indexing is O(n), which would make `for index in 0..text.length` quadratic. Proposed: UTF-8, plus an "ASCII only" flag maintained on append, making both O(1) for ASCII text.

Options for non-ASCII: **compute on demand** (recommended: simplest, no header growth), or cache the grapheme count in the header (+8 bytes per string). Is `length` expected to be O(1)?

### Q6. The price of Map's stable slots

A `Map` with stable slots costs one indirection per lookup compared to values stored inline in the table, and the stability is a documented guarantee (`getOrInsert` declares `mutates self.length`). **Recommendation:** keep the guarantee and the compact-dict design, measure it, and consider an opt-in inline variant only if benchmarks show a real gap.

### Q7. The internal intrinsic layer for the standard library

Writing collections in Fa requires raw allocation and pointer arithmetic, which the language deliberately lacks. **Recommendation:** a small set of `@`-intrinsics (allocation, pointer offset, load, store, `memcpy`, `clz`), accepted only in files of the standard library. Alternative: hand-written IR builders in the compiler. The choice decides how much of the collection code is Fa and how much is Rust.

### Q8. Module-level mutable values and threads

Two gaps: (1) the **initialization order** of non-constant module-level values, especially across files with cyclic references; (2) what a **worker thread** sees of a module-level `export mutable` value, since workers share nothing. **Recommendation:** initialization in dependency order, a cycle being a compile error; module-level mutable values are rejected inside workers unless their type is thread-safe.

### Q9. The native runtime and the documented use of Tokio and Rayon

The concurrency page says the native target uses Tokio and Rayon. They can be embedded behind a C ABI (section above), with LLVM coroutines for `async`. **Recommendation:** keep them as documented, wrapped in `fa-runtime`, and decide the details in M5. The alternative is a smaller runtime of our own (a work-stealing pool and an `epoll`/`kqueue` loop), which gives control over frame allocation and binary size, especially for `wasm`.

### Q10. Where this document should live

It is at the repository root. Files under `documentation/` are language pages rendered by the website, and each one must be listed in `packages/website/src/lib/docs/navigation.ts` or the build fails; this is an internal design document, so it was kept out. Tell me if it should move to `documentation/` (with a navigation entry) or to `idea/`.
