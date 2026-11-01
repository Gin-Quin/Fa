# Compiling Fa with LLVM

This is a **strategy document** for the `native` and `wasm` targets. It is not a language specification: the pages of `documentation/` stay the source of truth, and nothing here changes the language. Where the documentation is silent or ambiguous, this document says so and proposes an answer in [Questions to decide](#questions-to-decide), instead of choosing silently.

It merges two earlier proposals with the decisions taken since on structural typing.

It is organized around five goals:

1. **Compile fast**: cached modules and libraries, simple LLVM usage, a very short path from the frontend to the backend.
2. **Run fast**: output as close as possible to hand-written C, Rust or Zig.
3. **Stay simple**: as few mechanisms as possible, each one reused everywhere.
4. **Structural typing built for LLVM**: the feature is designed into the data layout, not emulated on top of it.
5. **Spend the language's guarantees**: Fa's safe memory model removes runtime checks that other languages must keep.

Goals 2 and 5 are the same goal seen from two sides, and goal 1 is mostly won by goal 3: the fewer distinct things the compiler does, the less it has to do twice.

## Summary of the strategy

| # | Decision | Recommendation |
| --- | --- | --- |
| 1 | Pipeline | One shared **checker**, then for native: **monomorphize**, **analyze**, **lower straight to LLVM**. No CFG-based middle IR; explicit *places* on the tree. |
| 2 | LLVM interface | The LLVM C API through a thin wrapper over `llvm-sys`, in-process, one context per thread. Text IR is only an output (`--emit=llvm-ir`, golden tests). |
| 3 | Caching | Content-addressed, **staged** (parse, interface, body, instance, object, link). Unit of reuse for objects = **codegen unit**, keyed by the hashes of the sources, of the *interfaces* it uses, and of the build configuration. Sound because compile-time evaluation is deterministic. |
| 4 | Structural typing | **Specialization by view**: a structural parameter is an implicit generic over the layout of its argument. Types are interned by *shape*, so `Human` and `Animal` are the same instance. Generated code is deduplicated by hash. |
| 5 | Object layout | Fields **sorted by alignment, then by name**: a layout depends only on the shape, never on the declaration order. |
| 6 | Memory | Objects inline in their enclosing storage, collections on the heap, **no runtime tracking of any kind**: no counters, no generations, no barriers, no unwinding. Every fact the checker proved, and only those, becomes an LLVM attribute. |
| 7 | Collections | Known designs: Vec (`Array`), Swiss table (`Set`), compact dict with stable slots (`Map`), slab/hive (`Pool`), chunked struct-of-arrays (`Columnar`), segmented list (`Growing*`), intrusive lists (relations). Written in Fa on top of a small internal intrinsic layer, so the monomorphizer and the cache serve them too. |
| 8 | Errors | Errors are values: `throws` is an extra return path. No exceptions, no landing pads, `nounwind` everywhere. |
| 9 | Profiles | `fa develop` stays on JavaScript. Native: `dev` (`-O0`, FastISel), `release` (`O2`), `max` (ThinLTO, later PGO), `size`. Linking with `lld`. |

## Contents

1. [Starting point](#starting-point)
2. [Architecture](#architecture)
3. [Compiling fast](#compiling-fast)
4. [Running fast: what the language lets LLVM assume](#running-fast-what-the-language-lets-llvm-assume)
5. [Data layout](#data-layout)
6. [Structural typing on LLVM](#structural-typing-on-llvm)
7. [Memory management](#memory-management)
8. [Collections](#collections)
9. [Errors and runtime checks](#errors-and-runtime-checks)
10. [Concurrency, targets and compile-time code](#concurrency-targets-and-compile-time-code)
11. [Testing and validation](#testing-and-validation)
12. [Milestones](#milestones)
13. [Questions to decide](#questions-to-decide)

---

## Starting point

The repository has a parser with an indexed syntax tree and source locations, and syntax for effects, references, relations, compile-time operations and concurrency. The entry points are `packages/parser/src/parsing/parse.rs` and `parse_single_statement.rs`.

There is no semantic checker yet: `packages/parser/src/analysis/analyze.rs` is a stub, and the existing inference falls back to `Any` in many cases. The name `TypedSyntaxTree` must not be read as "type checking is done". The checker is the first and largest piece of work, and the native backend must never infer types on its own.

---

## Architecture

```
 source (.fa) + locked dependencies + configuration
     │  discover namespaces, tokenize, parse     packages/parser (exists)
     ▼
 syntax tree (arena, indexed)
     │  resolve names, evaluate compile-time code,
     │  check types, ranges, effects, references
     ▼
 checked tree  ── JavaScript backend (structured emit, no layouts, no monomorphization)
     │
     │  native / wasm only
     ▼
 monomorphize  (demand-driven from the entry point, exports and tests)
     │  instances: functions with concrete types, layouts and views
     ▼
 analyze      (per instance, on the tree: ranges, moves, fusion)
     ▼
 lower        (tree → LLVM IR, scope-stack drops, attributes)
     ▼
 LLVM         (optimize per profile)  →  object files (cached)
     ▼
 lld          →  executable / .wasm
```

Checking is not one left-to-right pass: type functions and compile-time evaluation can reveal new types and dependencies. It is a set of **memoized queries with cycle detection**: signatures first, then bodies, recursive groups together.

### What is shared with the JavaScript target

Everything up to the **checked tree** is target-neutral: parsing, name resolution, structural typing, ranges, the `mutates` / `invalidates` / `throws` / `requires` effects, reference invalidation, relation validation. This is not an economy, it is the point of the design: memory safety in Fa is a *checking* problem, solved once, and both backends inherit it. The compile-time interpreter runs on the same checked tree.

Everything after it is native-only. In particular, **monomorphization and layouts do not exist for JavaScript**, where `Array(Item)` is erased and an object is a JavaScript object. A language feature is acceptable only if it lowers cheaply on both sides. The rule from the design notes applies here: *no feature may force per-value wrapping on a JavaScript hot path*. Each design choice below is checked against it, and where a native technique has a JavaScript cost, it is stated.

### Why no middle IR with a control-flow graph

Rust has MIR, Swift has SIL, Clang has none. Fa's structure favours Clang's route:

- Control flow is **structured**: no `goto`, no labeled loops, `break` and `continue` act on the innermost loop. Scope exits are lexical, so dropping values at scope exit is a stack of cleanups, not a dataflow problem.
- Borrow checking is **not** done on the native side. The checker did it on the language level, for every target.
- The analyses that remain after monomorphization (ranges, last use, iterator fusion) are walks over structured code. Loops need a **bounded fixed point with widening**: a fact that does not stabilize becomes unknown, never an exponential search or an unsound assumption.

So the **monomorphic tree** (the checked tree after type substitution, still arena-indexed) is the only IR. Two things make it fit for lowering:

- **Explicit places.** Every read and write names a *place*: a local, a module-level value, a field, an indexed element, a union payload, a row of a `Columnar`. Invalidation, moves and attributes are all stated on places.
- **Meaningful operations stay high-level** until their analysis is done: `Copy`, `Move`, `Drop`, `CheckIndex`, `RelationSet` and the collection calls are nodes of the tree, lowered at the last step.

A CFG IR can be added later if an analysis needs it; nothing in this plan prevents it.

### Crates

| Crate | Role |
| --- | --- |
| `fa-parser` | exists |
| `fa-checker` | names, types, ranges, effects, references, compile-time evaluation. Shared by all targets. Also feeds the language server. |
| `fa-native` | monomorphization, layouts, views, analyses, lowering. Knows nothing about LLVM's C API: it talks to a `Backend` trait. |
| `fa-llvm` | the `Backend` implementation over `llvm-sys`, the pass pipelines, object emission. |
| `fa-runtime` | a `staticlib` with a C ABI: allocator, panic, I/O, Unicode tables, threads, async. Prebuilt per target and profile, embedded in the `fa` binary. |
| `fa-link` | `lld` invocation and the cache directory. |

The `Backend` trait is small (types, functions, blocks, loads, stores, arithmetic, calls, attributes). It exists to keep `fa-native` testable without LLVM installed (a recording backend that prints text), not to support a second backend. `fa check` and the language server never initialize LLVM.

---

## Compiling fast

The techniques below are ordered by how much time they save, which is also the order to implement them in.

### 1. Do less: demand-driven compilation

Only what is reachable from the entry point, the exports being built and the tests being run is monomorphized and lowered. Libraries are distributed as source (see [modules](../documentation/architecture/modules.md#building-a-library)), so an application using one function of a large library compiles one function, and a program that starts no worker does not pull the concurrency runtime in.

### 2. A front end designed to be parallel

Fa's signature rules give the checker an unusual property: **a function body never changes any signature except its own, and only when the body is a single expression.**

- A block body must declare its return type, `throws`, `requires`, `mutates` and `invalidates`.
- A single-expression body infers them, but it is small, and checking it can be done on demand.

So the checker works in two phases: first collect every signature (cheap, parallel by file), then check bodies **in parallel**, each body only reading signatures. The only ordering constraint is among *inferred* single-expression functions that call each other. The documentation does not say what happens when they are mutually recursive; the simplest rule is to require a signature on one of them.

Other front-end rules:

- **Interned types.** Every type is interned once into an arena and referred to by a `TypeId`. Equality is an integer comparison, a type is its own cache key, and structural identity (below) is free. Recursive structural types are interned as cyclic graphs, compared with a cycle-aware equivalence, never expanded.
- **Arena everything.** The parser already stores nodes by index. Keep it: no `Box`, no `Rc`, no per-node allocation; one bump arena per compilation.
- **One pass per concern.** No pass re-walks the tree for a fact a previous pass could have recorded.

### 3. A thin, in-process LLVM

How to talk to LLVM, in order of simplicity:

| Option | Pros | Cons |
| --- | --- | --- |
| Emit `.ll` text, run `clang` / `llc` | No build dependency on LLVM, trivial to debug | Process spawn, text printing and parsing on every unit, an external toolchain to ship |
| **Raw C API via `llvm-sys`, thin wrapper of our own** | In-process, no text, fastest path, small surface, handles are plain indices in arenas | Needs a pinned LLVM; unsafe code confined to one crate |
| `inkwell` | Safer Rust API | `'ctx` lifetimes leak into every type, lags behind LLVM releases, hides little we would not wrap anyway |
| MLIR | Rich infrastructure | Far more machinery than a language this size needs |

**Recommendation: `llvm-sys`, pinned to one LLVM major version.** A small C++ shim is allowed only for what the C API lacks (some ThinLTO or debug-info entry points). Text IR is *derived* (`LLVMPrintModuleToString`) for `--emit=llvm-ir` and golden tests, never used as an interchange format.

Rules for using LLVM simply and quickly:

- **Opaque pointers only.** Every pointer is `ptr`. There are no pointer types to track, which removes a whole class of frontend code.
- **The new pass manager only**, through `LLVMRunPasses(module, "default<O2>", ...)`. No custom pass ordering. Fa gets its optimization from *facts handed to LLVM* ([next section](#running-fast-what-the-language-lets-llvm-assume)), not from custom passes.
- **No exceptions.** No `invoke`, no landing pads, no personality function, no `.eh_frame` for the language's own errors.
- **Allocas in the entry block, SSA for temporaries.** `mem2reg` and SROA do the rest.
- **Aggregates by `memcpy`**, never by first-class aggregate `load`/`store` beyond two scalars. Big aggregate loads and stores are a known source of slow LLVM compiles.
- **One `LLVMContext` and one `TargetMachine` per worker thread**, with one global job budget shared by the front end, codegen and ThinLTO.
- **Never emit what LLVM would delete.** No dead blocks, no unused declarations, no redundant casts: they cost time even when removed.

### 4. Profiles

| Profile | LLVM | Intended for |
| --- | --- | --- |
| `dev` | `-O0`, FastISel, no mid-end passes, lazy debug info, cached objects | `fa develop --target=native`, `fa test` |
| `release` | `default<O2>`, loop and SLP vectorization, a portable CPU baseline, cached objects | `fa build` |
| `max` | `release` plus ThinLTO with its own cache; O3 only where benchmarks show it wins; PGO once there are real applications to profile | deployments where the last percents matter |
| `size` | `default<Oz>`, dead stripping | small native and `wasm` artifacts |

These are internal profiles; the documented configuration only has `build.optimize`, and their user-facing names are to be decided. No profile uses the host CPU implicitly, and none enables fast-math: it changes observable results.

`fa develop` stays on JavaScript and Bun by default ([hot reloading](../documentation/architecture/hot-reloading.md)). A Cranelift backend for `dev` is **not recommended now**: it would compile faster than `-O0`, at the price of a second backend to keep identical in behavior.

### 5. The cache

The compiler is a build system with a deterministic function from inputs to outputs, so it caches like one.

**Determinism is already specified**: compile-time evaluation cannot read the clock, the network or random numbers, and the files it reads become build inputs ([metaprogramming](../documentation/advanced/metaprogramming.md#determinism)). The remaining sources of non-determinism are ours to avoid: iteration over hash maps (use insertion-ordered or sorted maps), absolute paths and timestamps in IR or debug info (remap them), and thread scheduling (partition deterministically).

#### Stages

| Cached artifact | Keyed by |
| --- | --- |
| Parsed file | source content, parser version |
| Checked interface | exported types, effects, requirements, compile-time results it exposes |
| Checked body | source and the exact interfaces it used, checker version |
| Instance | the instance key ([structural typing](#5-instance-keys-and-deduplication)), the body hash, async mode |
| Object (codegen unit) | the hashes of its instances, the interfaces and inlined bodies they use, the full backend configuration |
| ThinLTO result | managed by LLVM's own ThinLTO cache, never replaced by the object cache |
| Executable | the ordered list of inputs, the runtime, linker and link options |

Hashes of contents prove equality; timestamps only avoid reading unchanged files.

#### Units

| Level | What | Why |
| --- | --- | --- |
| Source unit | a `.fa` file (one exported entity) | Fa files are tiny and few of them change at a time |
| Codegen unit | a group of instances, compiled into one object | A file per object would mean thousands of tiny objects and a slow link |
| Instance | one monomorphic function | the granularity of deduplication |

Codegen units are formed **by namespace**: the functions of one folder, split when a unit exceeds a size budget, with the split decided by a stable hash of the instance key so that adding one function does not reshuffle the others. Measure before refining.

#### What a key must cover

```
unit key = hash(
	compiler version, runtime ABI version, LLVM version,
	target triple, data layout, CPU and features, profile, every codegen option
	(debug info, sanitizers, relocation model, code model),
	for each member instance:  hash of its monomorphic tree,
	for each dependency used:  hash of its *interface*, plus the hash of every body
	                           it exposes for inlining
)
```

- The **interface hash** of a function covers its complete signature: types, `throws`, `requires`, `mutates`, `invalidates`. Block bodies must declare all of these, so *editing a body without touching its signature recompiles that body's unit only*. An **inferred** effect that changes (in a single-expression function) is an interface change.
- A function small enough to be **inlined across modules** exposes its body: it is part of its interface hash, and callers re-emit it locally (`linkonce_odr` or `available_externally`). Editing it recompiles its callers, which is the price of cross-module inlining.
- A **layout** change invalidates every instance that uses the layout.
- Dependencies are recorded **by the query that used them**, and include more than calls: type functions, default values, generic definitions, every file and directory listing read by compile-time code, locked remote content, and **name-resolution results, including names that were missing**: adding a sibling file can change what an implicit namespace resolves to, without any existing file changing.
- Types are keyed by **shape** (below), so two packages that define the same structural type share instances.

#### Where the cache lives

- A **local, content-addressed** directory (`~/.cache/fa/`), shared by every project of the machine. Entries are published atomically; a corrupt or interrupted entry is a miss; eviction is bounded.
- Libraries are shared as source and pinned by the lockfile hash ([dependencies](../documentation/architecture/dependencies.md#lockfile)), so a library's non-generic units have a key independent of the application. They are compiled once per (library version, configuration). A cached archive is a local build product, not a package format.
- **Generic instances** are keyed by (generic function, canonical type arguments, layouts), not by the project using them, and get deterministic symbol names. `Array(Integer).push` is compiled once per machine, per compiler version.
- A **remote cache** is the same store behind a URL.
- The toolchain ships **pre-built instances** of the common standard collections (`Array(Byte)`, `Array(Integer)`, `Map(String, String)`...).
- A no-op build runs no LLVM work and no link.

### 6. Linking

Bundle **`lld`** (it also provides `wasm-ld`), so one fast linker serves every target, and pass `--icf=safe` as a safety net for [instance deduplication](#5-instance-keys-and-deduplication). `lld` alone does not know the platform's startup objects and system libraries: start by invoking the platform's compiler driver once for the final link, and replace it with direct `lld` calls per platform when those details are known. Native objects and `wasm` objects are separate cache entries.

### 7. A budget

Compile time deserves the same discipline as run time. Track, in CI, the time of the phases (parse, check, monomorphize, lower, LLVM, link), peak memory, cache hit rate and instance count on a fixed corpus, and fail a change that regresses one beyond a threshold. Measure separately: cold build, warm no-op build, private body edit, signature or effect edit, layout edit, generic definition edit, edit of a file read at compile time.

---

## Running fast: what the language lets LLVM assume

Most of Fa's performance does not come from clever code generation. It comes from **handing LLVM facts that C and C++ compilers have to guess**, because the checker already proved them, and from **avoiding entire costs**: no intermediate collections, no needless deep copies, no per-object allocation, no validation of proven-safe references.

### Checks Fa never emits

| Danger | Typical cost elsewhere | In Fa |
| --- | --- | --- |
| Dangling reference | garbage collector, reference counts, generation counters, borrow checker | tracked statically ([owned references](../documentation/advanced/owned-references.md)); relations are cleaned at deletion in O(k) |
| Iterator invalidation | version counters (Java, Python), undefined behavior (C++) | compile error |
| `null` | null checks | no null: optionals are explicit unions |
| Data race | locks everywhere, or undefined behavior | values are copied across threads; only thread-safe types are shared |
| Out-of-bounds index | check on every access | removed when the index range is proven; the rest is a typed error |
| Integer overflow | wrap, or check on every operation | removed when the result range is proven |
| Division by zero | check on every division | divisor proven non-zero, or a typed error |
| Non-exhaustive `when` | a default arm | compile error: the `switch` default is `unreachable` |
| Use of uninitialized data | zero-fill, or undefined behavior | definite initialization: every load is `noundef` |
| Exceptions | unwind tables, landing pads, cleanup code on every call | errors are values |
| Structural field access | dictionary lookup or vtable | a constant offset ([specialization](#3-passing-a-superset-where-a-subset-is-expected)) |

### How a proven fact reaches LLVM

An attribute is emitted only when its exact LLVM contract holds ([LangRef](https://llvm.org/docs/LangRef.html) is the authority): a wrong `noalias` or `range` is undefined behavior, far worse than a missing one. Start conservative, add each attribute behind focused tests and the [verify mode](#verify-mode).

| Fa knows | LLVM encoding |
| --- | --- |
| A parameter is an immutable reference | `ptr readonly nonnull noundef align A dereferenceable(N)`; `nocapture` unless a reference into it is returned; `noalias` only under [Q3](#q3-may-a-mutated-place-be-aliased-by-another-argument) |
| A parameter is `mutates` | `ptr nonnull noundef align A dereferenceable(N)`; `noalias` only under Q3 |
| `Integer(min = a, max = b)` | `range(i64 a, i64 b+1)` on parameters and returns, `!range` on loads (computed without overflow at the integer limits) |
| A sum cannot overflow | `add nsw` / `nuw` |
| A sum can overflow | `llvm.sadd.with.overflow`, then a branch with weights marking the overflow path cold |
| An index is proven in bounds | `getelementptr inbounds`, no comparison at all |
| An index is not proven | `icmp ult` against the length, branch to the typed-error path (cold); `inbounds` only after the guard |
| A function only reads its parameters | `memory(argmem: read)` (only when it also allocates nothing and calls nothing impure); `memory(read)` if it also reads module-level values |
| No error can unwind | `nounwind` on every function, including the runtime's |
| An exhaustive `when` | `switch` with an `unreachable` default |
| A `throws` path | branch weights, the error continuation block marked cold |
| Runtime allocation entry points | `allockind` and `"alloc-family"` on `fa_alloc`, `fa_realloc`, `fa_free`, so LLVM may remove unused allocations and pair them with their frees |
| A flat value (see below) | no drop code at all |

`dereferenceable(N)` covers the bytes the function actually touches, which keeps the IR of equivalent specializations identical (see [instance deduplication](#5-instance-keys-and-deduplication)).

### Fa's effects are LLVM's function attributes

`mutates` is a finer, checked version of what LLVM calls `memory(...)`. The function-level attributes are derived from the effects, never guessed:

- no `mutates`, no module-level write, no I/O, no allocation: `memory(argmem: read)` (or `memory(read)` if module-level values are read);
- `mutates self.score`: a write to one place of one parameter. Field-level `!alias.scope` metadata is possible later.

Memory safety alone does not imply Rust's exclusive mutable borrows: Fa allows aliases, and two parameters may designate overlapping memory. `noalias` therefore needs the rule of Q3; without it, it is never emitted.

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
| `BigInt` | `i64` | a value that fits in 63 bits is inline (tagged); a larger one points at uniquely owned limbs ([Q4](#q4-where-does-the-environment-of-an-escaping-closure-live)) |
| unions of literals, `Enum` | the smallest integer holding the member count | |
| `Fields` | a bitset | |
| `None` / `Void` | zero-sized, absent from signatures | |

### Strings

A `String` is `{ ptr, length in bytes, capacity | flags }`, 24 bytes, UTF-8.

- **Static strings** (literals) live in read-only data, with a "static" flag in the capacity word. They are never freed, and a first mutation allocates and copies. Since identity is not observable, this is invisible, and it makes the most common string, the literal, free to "copy". This is not copy-on-write: an owned string is never shared.
- **Small strings** (at most 23 bytes) are stored inline in the 24 bytes, with no allocation.
- **An "ASCII only" flag** is maintained on every write (a cheap vectorized check of the new bytes). It makes `length` and indexing O(1) in the common case. Otherwise, a heap string caches its grapheme count and lazily builds breadcrumbs (the offset of every 64th grapheme), so indexing scans at most 64 graphemes ([strings](../documentation/values/strings.md#indexing), [Q5](#q5-string-representation-and-the-cost-of-grapheme-length)).
- **A slice** (`use word = text[0..5]`) is `{ ptr, length }`: a view, never owning, tracked by the invalidation rules. Its bounds are grapheme boundaries.
- A sequential walk over a string runs one segmentation scan, not one per index. The Unicode tables are generated into the runtime at build time.
- A `Character` can hold several code points: it is a small string with inline storage, not one integer.

### Objects and tuples

An object is a struct stored inline: in an `alloca`, in a parent object, in a collection slot. A tuple is the same, with positional fields. A single-field object has exactly the layout of its field, so the "distinct id types" idiom of the [structural typing](../documentation/advanced/structural-typing.md#when-you-want-types-to-be-distinct) page costs nothing.

**Field order is canonical**: sorted by alignment (largest first), then by field name. See [structural typing on LLVM](#structural-typing-on-llvm) for why this, and not declaration order. Declaration order is kept separately for what the user can observe (printing, `@typeToSchema`). Methods are not stored (they are static functions); function-pointer fields are stored. Fields are never packed below their alignment.

### Unions and optionals

| Kind | Representation |
| --- | --- |
| `Union { ... }`, `Errors { ... }` (named) | `{ tag, payload }`, tag = declaration index, the smallest integer that fits, payload sized and aligned for the largest member. A single member needs no tag. |
| `A \| B \| C` (anonymous) | the same, with tag = index in the **canonical order of the member types** (sorted by shape hash), so `A \| B` and `B \| A` are one type. |
| `T?` | a union with `None`, with **niche optimization**: when `T` has an invalid bit pattern, `none` uses it and no tag is stored. |

Niches come free from many Fa types: a non-null pointer (arrays, strings and collections always hold a non-null pointer, an empty array points at a dangling aligned address as in Rust), an enum with spare values, a `Boolean`, a ranged integer (`Integer(min = 0)?` can use -1), a stored reference. `Optional(Integer)` is the only common optional that needs a tag. The first version can use explicit tags everywhere and add niches one by one.

### Functions and calls

- Internal functions use `fastcc` and `internal` linkage; exported ones and the runtime use the C convention. Parameters of aggregate type are passed **by pointer** ([parameters are references](../documentation/advanced/memory-management.md#parameters-are-references)); results of up to two scalars are returned in registers, anything larger is **constructed in the caller's destination** (`sret`).
- **`throws`** makes the result the union `T | E`, laid out as above. The caller's `try` is one compare on the tag, with the error path cold. A function without `throws` has no error in its ABI.
- **`requires`** is dictionary passing: each requirement is a hidden pointer parameter appended in a canonical order (by the requirement's `TypeId`). They are resolved at compile time by `provide`, and a `provide` is a stack slot or a reference to an existing value. There is no lookup at runtime. Function types carry their requirements, so hidden parameters are part of the ABI of a function value.
- **Function values** are `{ fn, env }`. A call whose callee is known at compile time is a direct call; so is every lambda passed to a generic function (`map`, `filter`...), because each lambda is its own type and the callee is specialized on it, like a C++ template or a Rust closure. Only a function stored in a field, in a collection or in a variable is an indirect call. A function used as a value also gets a **view entry point**, which takes an offset table with each structural parameter ([function values](#function-values)).
- **Closures capture by copy**, so the environment is a struct of the captured copies. A closure that does not escape keeps its environment in the caller's frame. An escaping closure stores an environment of at most 8 bytes in its `env` word, and a larger one in a heap block it owns ([Q4](#q4-where-does-the-environment-of-an-escaping-closure-live)).

### Module-level values

A module-level value is an LLVM global with a stable address, which is exactly what the language promises ([module-level values](../documentation/values/declaring-values.md#module-level-values)).

- A value computable at compile time (literals, `static`, constant objects) is emitted as constant data and needs no initialization code.
- Other values are initialized by a generated function, in the topological order of the module dependency graph, before `main`.
- `export mutable` is replaced in place: assignment destroys the old value and writes the new one to the same address. Unlike JavaScript, which needs a `{ value }` wrapper, native code needs nothing.

Initialization cycles, teardown and what a worker thread sees are unspecified: see [Q8](#q8-module-level-mutable-values-and-threads).

---

## Structural typing on LLVM

This is where Fa differs most from Rust, C++ and Zig, whose types are nominal and whose layouts therefore never need to *agree across names*. In Fa:

- two types with the same fields are the same type ([`Human` and `Animal`](../documentation/advanced/structural-typing.md));
- a type with *more* fields can be passed where fewer are expected, **without a copy and without truncation** (parameters are references; `use` aliases do not copy);
- a copy to a narrower type drops the extra fields, and an object literal may not have extra fields;
- a read-only collection of supersets can be passed where a collection of the narrower type is expected ([collections of supersets](../documentation/advanced/structural-typing.md#collections-of-supersets));
- a parameter keeps the type of its argument, `Type(parameter)`: an unannotated copy of it keeps every field ([keeping the type of the argument](../documentation/advanced/structural-typing.md#keeping-the-type-of-the-argument));
- generic constraints are shapes (`Entity: { id: Integer }`);
- anonymous unions are structural.

The compiler keeps three notions apart:

1. the **semantic type**: what the checker reasons about, including ranges, effects and owners;
2. the **stored representation**: the layout of a concrete shape;
3. the **view**: how to reach the fields of a narrower type inside an actual representation.

A naive native implementation makes structural typing expensive (copies, vtables, dictionaries). The decided strategy is **specialization**: every structural parameter is compiled as an implicit generic over the *layout* of its argument, so field accesses are constant offsets, as in hand-written C.

### 1. Identity is the shape

Types are interned by **shape**: the sorted set of `(field name, field type)` pairs, recursively. Consequences:

- `Human` and `Animal` are one `TypeId`: one layout, one set of instances, one cache entry.
- Two packages defining the same structural type share compiled code.
- Equality, hashing and copying are generated **per shape**: `eq<Human>` and `hash<Human>` exist once.

Exceptions (decided 2026-11-01): a type with `#private` fields, named unions, their members and enumerations are nominal, and their identity includes their declaration; `Fields` types are structural. The declared name is not part of the identity of the other types, but it is not lost: the checker records, at each use, the name the type was written with, for diagnostics, `@typeToString` and `@typeToSchema`. An instance whose code depends on a name (because it reflects on a type) includes that name in its key.

Named unions, `Errors` and `Enum` are the exception, as the language intends: their identity includes their declaration, because "`MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match".

### 2. Layout depends on the shape only

Layout is computed from the canonical field order (alignment descending, then name), with sizes and alignments from the target's data layout. Declaration order, spread order and type name play no role. So two structurally equal types always have the same layout, and **no conversion is ever needed between equal shapes**. The layout rule is part of Fa's ABI and is versioned: it is never borrowed from Rust's default struct layout, and never exposed as a stable C ABI ([Q1](#q1-canonical-field-order-and-the-c-abi)).

Alternatives considered:

| Layout | Pros | Cons |
| --- | --- | --- |
| **Sorted by alignment, then by name** | minimal padding, canonical, same code for equal shapes | a superset is not a byte-prefix of its subset in general |
| Declaration order, spreads first | a spread child is a byte-prefix of its parent, so views are free by construction | only for types built with a spread, and only the first one; padding depends on how the author wrote the type; equal shapes with different orders have different layouts and need adapters |
| Per-field separate arrays | no padding | `Columnar`, not an object layout |

**Decision: sorted.** Prefix compatibility would be an accident; the next section handles the general case at no runtime cost.

### 3. Passing a superset where a subset is expected

Given a parameter of declared type `D` and an argument of actual type `S ⊇ D`, the callee can only touch fields of `D` ("the narrowed view is a compile-time restriction, not a runtime transformation"). So its machine code depends on exactly one thing about `S`: **where each field of `D` sits inside `S`**. That is the *view* of `D` in `S`:

- for an object in memory: the offset of each field of `D`, recursively for nested objects stored inline (`human.address.city` is one constant offset);
- for a row of a `Columnar(S)`: the column of each field of `D`, plus the row index;
- for an anonymous union field whose members differ (`A | B` in `S`, `A | B | C` in `D`): the tag mapping.

| Situation | Lowering | Runtime cost |
| --- | --- | --- |
| `D` and `S` have equal shapes | pass the pointer | none |
| `S ⊇ D`, and `D`'s fields sit at the same offsets in `S` | pass the pointer, reuse the instance of `D` | none |
| `S ⊇ D`, offsets differ, **direct call** | **specialize the callee on the view**: an instance whose field accesses use `S`'s offsets | none (one more instance, cached) |
| `S ⊇ D`, **indirect call** (function value) | the callee's view entry point, with a static offset table for (`S`, `D`): see [function values](#function-values) | one argument, and one load per field |
| `let human: Human = hero` | `copy<Hero → Human>`: copies the fields of `Human` only, with owning copies of strings and collections | the copy the language specifies |
| `use human: Human = hero` | no code: the static type stays `Hero`, and `Human` only restricts which fields are visible | none |

A narrowing copy is **never** a `memcpy` of the start of `S`, and it never destroys the fields it leaves behind: they still belong to the source.

This is the generic machinery that explicit generics need anyway: `function readId(Item: { id: Integer })` is instantiated on the *full* type of the argument. A structural parameter is the same thing, with two differences: the type checker still sees `D` (the specialization is invisible to it), and the key is the view, not the type, so it shares more.

**Writing through a view.** A `mutates` parameter writes the fields of `D` into `S`. Decided ([mutating through a narrower type](../documentation/advanced/structural-typing.md#mutating-through-a-narrower-type)): reading is covariant, writing is invariant, so the checker only accepts an argument whose fields named by `mutates` have exactly the type of `D`'s. A view typed `score: Integer` never writes into a field `score: Integer(min = 0, max = 10)`.

### 4. `Type(parameter)`: when the full type matters

Decided: inside a function, a parameter keeps the type of its argument, `Type(human)`, which only exposes the fields of `Human`. So `let copy = human` copies every field of the argument, and only an annotation (`let copy: Human = human`) narrows.

A function that copies a parameter without annotation, returns `Type(human)` or reflects on it needs more than the view: the size and alignment of `S`, and its `copy` and `drop` functions. Such a parameter is specialized on the **full type** `S`, exactly like an explicit generic parameter; its result is constructed in the caller's destination, which knows `S`. A parameter that is only read or written field by field keeps a view key, which shares more instances. The checker records, per function and per parameter, which of the two keys it needs.

### 5. Instance keys and deduplication

Specialization could multiply code. Two cheap levels keep it in check:

1. **Instance key** (before lowering, from the types only): `hash(function, for each structural parameter its view (or its full type, for a parameter that is copied, returned as Type() or reflected on), generic arguments, names it reflects on)`. Two arguments with the same view share an instance, whatever their types. This key is the cache key. It must cover everything the generated code depends on; two instances are never merged on equal LLVM types alone.
2. **Code hash** (after lowering): hash the generated function. If it equals an existing one, emit an LLVM `alias` instead of a second body. `ageOf(hero)` and `ageOf(human)` below produce the same code and one machine function.

The linker's `--icf` is the last net. None of this needs a footprint analysis of the function body: the work is done by hashing. (Keying instances on the fields actually read, rather than on all of `D`, would share a little more, but the key would then change with the body; the code hash already catches those cases.)

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
define fastcc i64 @"fa.scoreOf.human"(ptr readonly nocapture nonnull noundef align 8 dereferenceable(40) %human) nounwind {
	%field = getelementptr inbounds i8, ptr %human, i64 32
	%score = load i64, ptr %field, align 8
	ret i64 %score
}

define fastcc i64 @"fa.scoreOf.hero"(ptr readonly nocapture nonnull noundef align 8 dereferenceable(64) %human) nounwind {
	%field = getelementptr inbounds i8, ptr %human, i64 56
	%score = load i64, ptr %field, align 8
	ret i64 %score
}
```

Neither function copies anything, truncates anything, or looks anything up. After inlining (these are single-expression functions, so they are exposed for cross-module inlining), both calls become a single `load`.

### Columnar rows

`for human in people`, with `people: Columnar(Hero)`, hands out a reference that is a row index, not a pointer. Calling `scoreOf(human)` specializes `scoreOf` on a *row view*: the base of the `score` column and the index. A destructured loop (`for { age } in people`) walks the `age` column only and never assembles a `Hero`. Reading a whole row is a gather, as the documentation says.

### Unions

An anonymous union `A | B` passed where `A | B | C` is expected has a different tag numbering (tags follow the canonical order). The same pattern applies:

- **by value**: re-tag with a constant mapping (`tag' = map[tag]`, one load or a `select`);
- **by reference, direct call**: specialize the callee on the actual union, which also gives better code (the `when` only has the actual branches);
- **by reference to a mutated union**: rejected. Writing a `C` into storage sized for `A | B` would be unsound.

Unions of literals (`"north" | "south"`) are plain integers, and widening them is the same mapping. The members of an anonymous union are distinguishable at runtime (decided 2026-11-01, see [unions](../documentation/types/unions.md)): a value never satisfies two object members, so the tag identifies the member.

### Collections of supersets

Decided (see [collections of supersets](../documentation/advanced/structural-typing.md#collections-of-supersets)): `Array(Hero)` is accepted where `Array(Human)` is expected **when the parameter is not `mutates`**. A mutated collection of supersets is a compile error, because the callee could add a `Human` to storage laid out for `Hero`; the author writes the generic form instead:

```fa
function keepNamed(Item: Human) = (items: Array(Item)) mutates items => {
	items.deleteWhere((item) => item.name == "")
}
```

Lowering, the same as for objects:

- **Direct call, read-only parameter:** specialize the callee on the element view: the stride of `Hero` plus the offsets of `Human`'s fields inside it. When the layouts are compatible (same stride, same offsets), the `Human` instance is reused.
- **Indirect call:** the offset table carries the stride ([function values](#function-values)).
- **Generic form:** `Item` is the full element type, so the instance is specialized on its stride and offsets: ordinary monomorphization.
- **`Columnar`:** the view ignores the extra columns.

Storing into a collection always narrows: `push` / `add` of a superset copies `D`'s fields into the slot. A value is moved (`memcpy`) only when its shape equals the element type. An object literal is never materialized as a temporary: its fields are stored straight into the slot (after evaluating the field expressions that may throw or call), and it may not contain extra fields.

### Alternatives considered

| Technique | Where it comes from | Why it is not the default |
| --- | --- | --- |
| **Specialization by view** (chosen) | C++ templates, Rust `impl Trait` parameters, the "opportunistic traits" explored for a Rust target | — |
| A view of field pointers: one pointer per field of `D` | an earlier proposal | one argument per field (stack-passed beyond six on x86-64), nested objects multiply them, and collections need a stride anyway |
| A pointer plus an offset table, for every call | Go interfaces, Swift existentials, `dyn Trait` | one more argument and one more load per field on every call, and loops over collections lose constant offsets, hence vectorization |
| A temporary copy shaped like `D` | — | `mutates` writes would be lost and returned references would dangle, unlike JavaScript |
| Prefix layouts | C struct embedding, Go embedding | see the layout table |

Offset tables are used in the one place specialization cannot reach: function values.

### Function values

Decided. A function value is compiled once, so it cannot be specialized on its arguments:

```fa
type HumanHandler = (human: Human) => Void

let handlers: Array(HumanHandler) = [logScore, logName]

for handler in handlers {
	handler(hero) -- `logScore` reads `score` at offset 32, `Hero` has it at 56
}
```

A function used as a value (stored in a variable, a field or a collection) gets a second entry point, the **view entry point**, generated only for such functions. With each structural parameter, it takes a pointer to a static table describing the actual argument:

```
HeroAsHuman = { offsets: { age = 0, name = 32, score = 56 }, stride = 64,
                size = 64, align = 8, copy = copy<Hero>, drop = drop<Hero> }
```

- The offsets are recursive for nested objects; the stride serves collections of supersets; size, alignment, copy and drop serve a parameter that is copied without annotation or returned as `Type(parameter)`, since the caller can't know whether the callee does it.
- A table is static data, generated once per pair (actual type, declared type). A caller passing the exact type passes the identity table.
- In the entry point, a field access is a load of the offset (hoisted out of loops by LLVM) and a load of the field. A result typed `Type(parameter)` is constructed in the caller's destination.
- `mutates` writes go to the original, and returned references stay valid, so a call through a value behaves exactly like a direct call, and like JavaScript.

Function types declare their effects like functions do (`type Reset = (player: Player) => Void mutates player`), and a function can be stored in a function type only if the type declares all its effects ([function types](../documentation/functions/defining-functions.md#function-types)).

### JavaScript

None of the machinery above exists for the JavaScript target: objects are dynamic, a view is just a property read, and there is nothing to specialize. A narrowing copy calls the copy function generated for the destination type (never a spread, which would keep the extra fields).

`Type(parameter)` is erased, except where the function needs the actual type: an unannotated copy of a parameter, and a reflection on it. Decided ([Q16](#q16-copying-a-parameter-in-javascript)): such a function is emitted once per argument type, as in native code, and a function value receives the copy function of the actual argument as a hidden argument, like the native table. Structural typing therefore costs the JavaScript backend nothing, which is the point of keeping layouts out of the shared checker.

---

## Memory management

The documentation's model maps almost one-to-one onto LLVM. The compiler's work is mostly *not* emitting things.

### Rules and their lowering

| Rule of the language | Lowering |
| --- | --- |
| Objects are inline values | `alloca` in the entry block, or in place in their parent or collection slot. `mem2reg` and SROA turn most allocas into registers. |
| Collections own the heap | one header inline, buffers from `fa_alloc` |
| Every assignment is a copy | scalars: a register copy. Flat aggregates: `memcpy`. Owning values: a generated `copy<Shape>` function. |
| A copy becomes a move when the source dies | a **last-use analysis** on the tree (a backward walk of structured code). The source must be dead by name *and* through every `use` alias. A moved value is `memcpy`'d and not dropped. |
| Results and inserted values | **constructed in their destination** (`sret`, collection slot): no temporary, no copy |
| `use` creates an alias | a pointer; no code |
| Parameters are references | a pointer parameter with the attributes above |
| Iteration yields references | a pointer to the current element, or a row index for `Columnar` |
| Values are destroyed at scope end, in reverse order | a **cleanup stack** while lowering |
| A field holds a copy, or is a relation | inline storage, or a [relation link](#relations) |

### Trivially copyable values need no code

A value is **trivially copyable** when it owns no memory: a primitive, an optional or a union of trivially copyable values, a stored reference, or an object whose fields are all trivially copyable. (The documentation's word **flat** means something else: *invalidation-free*. `Integer?` is trivially copyable but not flat, since mutating it invalidates references into its payload.) A trivially copyable value has **no copy function, no drop, no move bookkeeping**: copying is `memcpy`, and destruction is nothing. The compiler computes this once per shape.

Only values that own memory (collections, strings, recursive types, closures, and objects containing them) have `copy<Shape>` and `drop<Shape>` functions. They are generated per shape, deduplicated by interning, and called directly. Equality and hashing are generated per shape too, field by field: they never compare or hash padding bytes.

### Scope exit without unwinding

Control flow is structured, so the lowering keeps a stack of live owning values. At every exit from a scope (falling through, `break`, `continue`, `return`, a `throw` or a `try` propagating) it emits drops for the values declared inside, in reverse order, and branches on. Cleanup blocks are shared where several exits drop the same values. There are no cleanup landing pads and no personality routine.

A value that is *conditionally* moved (moved in one branch only), or partially initialized when an error is thrown, gets a one-byte drop flag, tested at scope exit. The analysis knows which values need one; most do not.

### References and invalidation cost nothing at runtime

The checker rejects any use of an invalidated reference. Native code therefore contains **no** liveness check, generation counter or tombstone for local references. They are plain pointers, valid by construction.

This is only sound if the checker is precise. Its rules are normative, in [reference checking](../documentation/advanced/reference-checking.md): a fixed set of facts, and a reference returned by `add` (or by a user function inferred to return fresh references) is distinct from every reference live before the call. Arguments may overlap a mutated place only where the callee's signature declares it (an argument for a parameter typed `T in p`, an implied `C[..].f` effect against an argument typed `T in C`). It tracks, for each reference, the owner *instance* (`townA.people` and `townB.people` are different owners even though they come from one field declaration), the root place and projection, and the known element identity (an exact index or key, or "returned by this `add`"). At each call, it applies the callee's effects with the actual arguments substituted, and invalidates every reference that may overlap. Effects are checked **inside** callees too: a function receiving two aliasing arguments must not delete through one and read through the other. Callbacks, closure captures and relation side effects take part in the same analysis. An uncertain case is a compile error naming the invalidating operation, never a runtime check.

A reference's representation depends on the collection it points into, which its type names (`Person in people`):

- row-oriented collections (`Array`, `Pool`, `Map`, `Growing*`): a pointer to the element or slot;
- `Columnar`: a slot index. The type already names the columnar, so the reference does not need to carry it.

### Recursive types

The recursive part of `T?` or of a union is a heap block owned by its parent: allocated by the compiler, copied by `copy<Shape>`, freed by `drop<Shape>`. Non-recursive members stay inline. Copying and dropping a deep recursive value (a long list built from recursive optionals) use an **iterative walk**, never native recursion, so that a deep value cannot overflow the stack. A per-thread size-class allocator inside the runtime is the optimization to reach for when trees dominate a profile.

### The allocator interface

The runtime exposes `fa_alloc(size, align)`, `fa_realloc(ptr, old, new, align)` and `fa_free(ptr, size, align)`: **sized** deallocation, so a better allocator can be dropped in later. The first implementation sits on the system allocator. Allocation failure aborts with a message; the multiplication `count * size` is checked for overflow before allocating. Large arenas (Pool chunks) use aligned `mmap`.

Local references are never stored in data, so **an ordinary Fa value is trivially relocatable**: moving it is a `memcpy`, and `Array` growth can use `realloc` and `mremap`. Relations are the one place where pointers are stored: they point into chunks that never move, never into a collection header, so moving the header of a `Pool` keeps them valid. Copying an owner whose collections are linked by relations is not a `memcpy`: the copy runs in two passes, copying the elements while recording old → new addresses, then reconnecting the internal links. An element copied out of its collection, taken with `take` or added to another collection comes out without links ([copying linked objects](../documentation/advanced/relations.md#copying-linked-objects)).

---

## Collections

The documentation already picks the semantics; this section picks the data structures. Each is a well-known design in C++, Rust or Zig, chosen because it makes the stated invalidation rules *true by construction*. Hot paths (indexing, iteration, append) are tiny bodies LLVM inlines; allocation, rehashing and diagnostics are out-of-line cold paths.

| Fa | Layout and technique | Borrowed from | Elements move on growth? |
| --- | --- | --- | --- |
| `Array(T)` | `{ ptr, length, capacity }`, doubling growth, inline fast path for `push` | Rust `Vec`, C++ `std::vector` | yes |
| `Set(T)` | open-addressing **Swiss table**: 1 control byte per slot (7-bit hash tag), group probing with SIMD | Abseil `flat_hash_set`, Rust `hashbrown`, Go 1.24 maps | yes (rehash) |
| `Map(K, V)` | **compact dict**: a Swiss-style index of `u32` slot ids, plus entries in a stable chunked arena | Python dict, Zig `ArrayHashMap` | **no** (stable slots, as documented) |
| `Pool(T)` | **slab / hive**: size-aligned chunks, occupancy bitmap, per-chunk free lists, lowest chunk filled first | C++26 `std::hive` / `plf::colony`, `slab` | **no** |
| `Columnar(T)` | one chunked column per field, same slot index across columns, shared occupancy | Zig `MultiArrayList`, struct-of-arrays | no |
| `OrderedColumnar(T)` | one `Array` per field | Zig `MultiArrayList` | yes |
| `GrowingArray(T)` | **segmented list**: segments of doubling size, `O(1)` index with `clz` | Zig `SegmentedList`, C++ `std::deque` | **no** |
| `GrowingPool`, `GrowingMap`, `GrowingSet`, `GrowingColumnar` | the same without deletion: pure bump arenas, no free list or bitmap | `bumpalo`, `typed-arena` | no |

The invalidation table in the documentation ("Effects of the collections") and this table are the same table: a collection *declares* `mutates self` when its representation can move elements, and `mutates self.length` or `mutates self.size` when it cannot.

### Array

`push` has an inline fast path (`len < cap`: store and increment) and a cold call to `fa_array_grow(header, element_size, align)`. An object literal is stored field by field into the new slot. `capacity = n` in `from` is `reserve`. `get`, `[]` with a proven index, `for` and slicing all compile to pointer arithmetic. Because values are trivially relocatable, growth is a `realloc`.

`insert` and `delete(index)` are a `memmove` of the tail (declared as `invalidates self[index..]`).

### Set

A Swiss table with the value stored inline: the fastest general design, and the documented contract (an `add` may move values) allows it. Probing uses 16-byte SIMD groups (SSE2, NEON; a portable 8-byte word otherwise). `==`, hashing and copy come from the per-shape functions, **inlined into the probe loop** by monomorphization, which a type-erased runtime could not do. A custom `key = (value) => …` gives both equality and hash, so lookup stays constant on average.

The value of an element defines its place in the table, so it must not change while the element is in the set (see the gaps listed in [other questions](#other-gaps-to-check)). `Float` elements need a decided equality and hash for `NaN` and `-0`.

### Map

The documentation requires stable slots: `getOrInsert` declares `mutates self.size` and references survive insertion. A compact dict gives this at the cost the documentation already admits (one indirection per lookup):

```
index   :  control bytes + u32 slot ids       (Swiss-style, may rehash freely)
entries :  chunked arena of { key, value }    (never moves; free list for deleted slots)
```

Rehashing rebuilds only the index, from the hashes stored in the entries; entries and references into them are untouched. Iteration walks the entries, not the table, which is faster and dense. Maps of at most 8 entries have no index. The entries live in a segmented arena with doubling segments ([Q6](#q6-the-price-of-maps-stable-slots)).

### Pool

The data structure behind relations, entity systems and `add` returning a reference.

- **Chunks** of one compile-time size per `Pool(T)`, aligned to that size (64 KiB or more for large `T`). A header at the start of each chunk holds an occupancy bitmap, a count and the link to the next chunk.
- `add`: pop the free list of the lowest chunk that has a free slot, else bump the current chunk, else allocate a chunk (cold). It returns the slot pointer. A literal argument is **constructed directly in the slot**. Filling the lowest chunks first keeps the pool dense; the details are in the implementation section of `documentation/collections/pools.md`.
- `delete(ref)`: relation cleanup, `drop<T>`, clear the occupancy bit (the chunk header is `ptr & ~(chunk_size - 1)`, so no search), push the slot on the free list (the next index is stored in the dead slot). Slots are reused without any generation check: static invalidation and eager relation cleanup make it safe.
- Iteration: for each chunk, for each 64-bit occupancy word, `tzcnt` to the next live slot. A Pool of flat elements is destroyed by freeing its chunks without visiting the elements.
- `deleteWhere` and `clear` visit the occupancy bits. A pool is never compacted: relation links are raw pointers into its slots.

### Columnar

One chunked column per field. A slot has the same index in every column, so a reference is that index. `for { age } in columnar` touches **one** column: a contiguous run per chunk, which LLVM vectorizes. Reading a whole element is a gather, as the documentation says. Deletion marks the slot free in the shared occupancy bitmap and recycles it. `OrderedColumnar` uses one `Array` per field: a single reallocating buffer would not keep the stable addresses `Columnar` promises.

An alternative for module-level singleton collections, not for the first version: reserve each column in virtual memory so that the address is `base + index * size`, with no chunk table.

### Growing collections

Nothing is deleted, so there is no free list, no bitmap and no per-slot state:

- `GrowingPool`, `GrowingColumnar`: a bump pointer in the current chunk; destroying them frees the chunks (and visits elements only if they own memory).
- `GrowingArray`: a segmented list, not a vector without deletion (vector growth would move its elements). With a first segment of `2^k` elements, element `i` lives in segment `s = floor(log2((i >> k) + 1))` at offset `i - ((2^s - 1) << k)`: two shifts, one `clz`, one load of the segment pointer. Elements never move, so references are valid for the collection's lifetime.
- `GrowingSet`, `GrowingMap`: an arena of entries plus an index; values are stable even though the index rehashes.

Destroying a growing collection whose elements own memory is still O(n).

### Iteration and fusion

`for` over a collection lowers to a counted or chunked loop with a pointer to the element, no iterator object. A chain such as

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

is rewritten **by the compiler, on the tree**, into one loop with the lambdas inlined and no intermediate array, when its callbacks are pure (below). This is deliberate: relying on LLVM to inline layers of iterator adaptors (as Rust does) makes IR big and compilation slow; the front end can do it directly because `filter`, `map` and `reduce` are known operations. A chain that is broken across statements is not fused, exactly as the [performance guide](../documentation/guides/performance.md#let-chained-methods-fuse) says. A `filter` whose result is collected reserves an upper bound and fills once.

Decided ([Q12](#q12-the-order-of-callbacks-in-a-fused-chain)): a chain runs **step by step**, as with JavaScript arrays and Gleam's `list` module, and the compiler fuses it only when this can't be seen: every callback is proven pure (no `mutates`, no `throws`, no call to a function that is not pure). The lambdas of a chain are written inline, so their purity is inferred from their bodies. Pure chains are the common case (arithmetic, comparisons, field reads); a chain with a `console.log` keeps its intermediate collections. The JavaScript backend applies the same rule, and emits plain loops rather than `Array.prototype.map` and `filter` with closures. A chain that starts with `values()` is element by element by definition, so it is always fused.

### Hashing

`hash<Shape>` is derived per shape, structurally, like `==`. The hash function is fast and non-cryptographic with a **per-collection random seed** (as Go does): resistant to hash flooding without paying for SipHash, which matters for the server use cases of the documentation. Iteration order is documented as "not guaranteed", which allows this; tests can fix the seed. Determinism of builds does not require a fixed seed at runtime.

### Relations

[Relations](../documentation/advanced/relations.md) become intrusive links in the elements themselves, with no allocation and no hidden index:

| Kind | Direct side stores | Reverse side stores |
| --- | --- | --- |
| one-to-many (`bestFriend` / `bestFriendOf`) | target pointer, `prev`, `next` | list head, count |
| one-to-one (`mentor` / `mentee`) | target pointer | target pointer |
| symmetric (`spouse`) | target pointer | itself |
| many-to-many (`friends` / `friendOf`) | head of an **edge list** | head of an edge list |

A many-to-many edge is a small node `{ a, b, links in a's list, links in b's list }` allocated from a per-relation slab; a target is linked at most once, so there are no duplicate edges. Writing `ana.bestFriend = cleo` is the three steps of the documentation (detach, update, attach), each a constant number of pointer writes. One-to-one and symmetric writes also detach the displaced partner; self-links and re-assigning the current target are handled explicitly. Any allocation happens before the first write, so a failure never leaves a half-updated relation.

Owner fields are cleared statically: a collection with owner fields is only modified through a path from its owner, so the owner is always known at the deletion. Deleting an object runs a generated `unlink<Shape>`: unlink the direct sides in O(1), and walk each reverse list in O(k) to set the referrers to `none`. When both sides of a relation die together (the owner of both collections is destroyed), the unlinking is skipped. Following a relation is the ordinary optional test, then a load: no liveness lookup.

Every write path goes through these generated helpers: assignment, `.=` updates, `take`, copy, deletion, `clear`. Copying the bytes of a reverse list would corrupt the graph. The implied writes (reverse side, displaced partner, cleared referrers) are part of the effects the checker sees.

This is the reason relations require stable-address collections: the links are raw pointers into slots that never move. In JavaScript the same links are ordinary object fields, so the design adds no wrapper there either.

### How the collections are written

**Decided: in Fa, in the standard library, on top of the internal `intrinsics` namespace** (raw allocation, pointer arithmetic, load, store, `memcpy`, `clz`...), visible only to the standard library ([Q7](#q7-the-internal-intrinsic-layer-for-the-standard-library)). Reasons:

- the same monomorphizer, cache and attribute logic serve user code and the standard library, which is the "one mechanism" goal;
- `hash`, `eq` and `copy` for the element type are *inlined* into the collection code, which a type-erased Rust runtime cannot do;
- a precompiled instance such as `Map(String, Integer)` is shared by every program through the cache, and the common instances ship with the compiler.

The rejected alternative was hand-written IR builders in the compiler, one per operation, with a type-erased Rust runtime for the cold paths: more code, in the less testable place.

For JavaScript, none of this applies: collections are JavaScript structures, as the documentation says.

---

## Errors and runtime checks

### Errors are values

`throws E` makes the result a union `T | E` and `try` a branch on its tag. Nothing unwinds, so:

- no landing pads, no personality function, no exception tables;
- `nounwind` on every function;
- the error path is an ordinary, cold, predicted-not-taken branch, which runs the scope's cleanups.

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
| Input decoding | always: untrusted bytes are never assumed valid | validated once, while building the typed value |

Everything else was removed by the checker. The range analysis ([numbers](../documentation/values/numbers.md#ranges-through-arithmetic)) runs in the checker for concrete code and again, cheaply, per monomorphic instance whose ranges depend on generic parameters (`Matrix(Float, 4, 4)` bounds the indices of `get` by `rows - 1`). Its proofs have two jobs: deciding whether an operation adds a typed error to a function's signature (which must be right before LLVM sees anything), and letting the lowering omit the check. Each emitted check consults a "proven" bit recorded on the tree node. A fact is killed by any write, call, callback, suspension or thread that can change what it is about.

### Arithmetic that LLVM does not do the Fa way

- **`//` and `%%` floor**, LLVM's `sdiv` and `srem` truncate toward zero: add the correction when the remainder is non-zero and the signs differ. `Int64.MIN // -1` overflows and is handled before the `sdiv`, which would be undefined.
- **`/` produces a float**: it is never an integer `sdiv`.
- **Negative indices** (`length + index`) are normalized without overflowing at `Int64.MIN`.
- **On 32-bit targets** (`wasm32`), an `Int64` index is proven in bounds before being narrowed to the pointer width.
- **Inclusive ranges** never compute `max + 1` at the integer limit.
- **Shifts** by a dynamic amount are range-checked, since LLVM gives poison beyond the width.

### Verify mode

The compiler's claims are only as good as the checker. A `--verify` build turns every *removed* check back into an assertion with a distinct message ("compiler bug: index proven in bounds was not"), and every attribute that asserts a fact (`noalias`, `range`) into a runtime test where one can be written. It is for CI and fuzzing, not for production, and it is the cheapest way to find an unsound proof. See [testing](#testing-and-validation).

---

## Concurrency, targets and compile-time code

### Async, workers and `for parallel`

The native runtime is our own ([Q9](#q9-the-native-runtime-and-the-documented-use-of-tokio-and-rayon)): one single-threaded executor per thread, a fork-join pool for `for parallel`, and OS threads for workers, all inside the prebuilt `fa-runtime` behind a C ABI. Fa programs are never translated to Rust and rebuilt with Cargo.

- **`for parallel`**: the loop body is outlined into a function, and the loop becomes `fa_parallel_for(count, body, env)`, where `body` takes a range `(start, end, env)`: the runtime's fork-join pool splits the range lazily between its threads. The compiler's proof that writes are disjoint is what makes the outlined body free of synchronization.
- **Workers**: a thread with its own state. Arguments and results are copied across the boundary (the language's ordinary copy), through a multi-producer queue in the runtime.
- **`async`**: an `async` function is lowered to a resumable state machine, generated only when an asynchronous instance is requested. Colorless async is a front-end matter: the checker requests a synchronous and/or an asynchronous instance of the function, exactly as the JavaScript backend emits `___sync` and `___async`, and both are cached separately. LLVM coroutines (`llvm.coro.*`, switched-resume lowering) are the simplest way to generate the state machine; a hand-lowered one gives more control over frame allocation. The runtime wraps it in a task with a small poll / wake / drop ABI, polled by the executor of its thread (tasks run on one thread). A blocking call sets a depth counter that makes `await` block and `start` run eagerly.
- **Frames and borrowed arguments**: a task frame outlives the call that started it, so it is a compiler-made heap block from a per-thread size-class allocator ([Q4](#q4-where-does-the-environment-of-an-escaping-closure-live)), and a task that uses a borrowed argument or requirement must finish (or be fully cancelled) before its owner dies ([tasks and references](../documentation/advanced/concurrency.md#tasks-and-references)). The checker treats a running task like a pending call: the values it reads can't be modified, the values it modifies can't be used, and an `await` kills the refinements on values another task can change.

This is the last milestone.

### Targets

| Target | LLVM | Notes |
| --- | --- | --- |
| `native` | host triple (x86-64 and AArch64 first), `lld`, system libc to start | the primary target |
| `wasm` | `wasm32-unknown-unknown`, `wasm-ld` | same lowering; the runtime provides its own allocator (no libc) and its own imports and exports; no `for parallel`, threads only with shared memory |
| `rust` | not LLVM | emits Rust source; independent of this document |
| JavaScript | not LLVM | the checked tree, structured emit |

Getting `wasm` is almost free once `native` works, which is a strong argument for LLVM over a hand-written backend. Features a target lacks are already compile errors, per the documentation.

### Foreign code

Decided on 2026-11-01, specified in [foreign code](../documentation/advanced/foreign-code.md): bindings live in `*.foreign.fa` files, the only code the checker trusts instead of proving (`@extern` declarations, `Pointer(T)`, `CString`, `destroy` on move-only handle types). `CStruct` gives the C layout ([Q1](#q1-canonical-field-order-and-the-c-abi)); every other type keeps the canonical layout, and the view ABI is never exposed. A call to C is a direct call with the target's C convention; `CStruct` arguments are classified like clang does (`fa-llvm` implements the System V, AAPCS64 and Windows x64 rules). `fa bind` reads headers with libclang once, and its output is committed, so builds never parse C. `build.cLibrary` exports a foreign file's namespace as C functions (status codes for `throws`, thread-local last error, initialization at load time). Cross-language ThinLTO with clang-built libraries goes through `lld`. A Rust shim must not build a `&mut` to Fa storage that may be aliased, and must not let a Rust panic unwind into Fa code. A Rust shim must not build a `&mut` to Fa storage that may be aliased, and must not let a Rust panic unwind into Fa code.

### Compile-time evaluation

`static`, `@if`, `@for` and `@function()` need an evaluator at compile time. The documentation describes **an interpreter shared with `fa run`**. Keep it that way: an interpreter over the checked tree has no startup cost, and compile-time code is usually small. It uses the target's arithmetic (widths, overflow) and deterministic library operations, and resolves `@import` through the locked dependency loader, never through arbitrary network access. A JIT through LLVM's ORC would pay for itself only on very heavy `static` computations, and can be added behind the same entry point if one appears. Compile-time code is a build input like any other: its results are cached by the hash of the code, of its arguments and of the files it read, and a hit is accepted only after checking that those files did not change.

### Reactivity and requirements

`reactive`, `derived` and `watch` are runtime-library features (a dependency graph, batching, scope cleanup) lowered from the checked tree, and are out of scope for this first plan. Values that are not reactive never carry reactive metadata. `requires` / `provide` are covered above.

---

## Testing and validation

The tests that matter most are the ones that compare Fa against itself. The LLVM verifier proves none of the language's safety properties.

- **Differential testing against JavaScript and the interpreter.** The JavaScript backend is the reference implementation of the semantics. Run every example of `documentation/` (the same blocks validated by the parser) on the backends and compare outputs. A difference is a bug in one of them.
- **Verify mode** ([above](#verify-mode)) in CI, on the test corpus and on fuzzed programs: it finds unsound proofs.
- **Sanitizers.** The runtime and generated code are run under AddressSanitizer and leak detection. Fa promises no leaks and no use-after-free; the sanitizer is the independent check of that promise.
- **Golden IR tests.** Lower small programs and compare the text IR with committed files. These pin the attributes and the layouts, which is where regressions hide. Most run on the recording `Backend`, whose output is LLVM-syntax text, so they need no LLVM; a smaller set goes through real LLVM and its verifier (`--emit=llvm-ir`).
- **Layout tests.** For a corpus of types: equal shapes have equal layouts, every view offset is in bounds, and `size`/`align` match what LLVM's data layout reports.
- **Structural-typing tests.** The cases of the [view table](#3-passing-a-superset-where-a-subset-is-expected): different offsets, nested views, columnar rows, narrowing copies that keep the source's extra fields alive, `Type(parameter)`, collections of supersets, and the instance and alias counts of the worked example.
- **Memory tests.** Last use with a live `use` alias, partial initialization, cleanup on every kind of exit, stale aliases through callbacks and module-level values, deep recursive values (no stack overflow), integer limits.
- **Relation tests.** Every cardinality, cycles, self-links, replacement, deletion, duplicates, immediate slot reuse, and copies and moves of owners.
- **Cache tests.** Change a body: one unit rebuilds. Change a signature: its callers rebuild. Add a sibling file: name resolution is re-checked. Rebuild twice: byte-identical objects.
- **Performance tests.** A small benchmark suite (array scans and appends, fragmented pool iteration, relation reads, writes and deletions, map rehash with live references, single-column loops, Unicode iteration, closure calls), compared against the equivalent C++, Rust or Zig with the same allocation, overflow and Unicode semantics, plus the compile-time budget from [above](#7-a-budget).

---

## Milestones

Each milestone ends with something that runs, and adds the timers and golden tests for what it introduced. The cache boundary (stable semantic ids, recorded dependencies, serializable interfaces) is designed in M0, even though the persistent cache arrives in M2.

| # | Content | Done when |
| --- | --- | --- |
| **M0** | `fa-checker` skeleton on the parser's tree (names, signatures, a first subset of types and effects); `Backend` trait with the recording backend; integers, booleans, functions, `if`, loops, `fa-runtime` with `print` and `panic`; `fa build --target=native` through `llvm-sys` and `lld`; every unsupported construct is a clear compile error | a Fibonacci program compiles, runs, and its IR is a golden test |
| **M1** | objects, tuples, optionals, unions, `when`, `throws` / `try` / `catch`; layouts; flat vs owning; copies, moves, construction in the destination and the cleanup stack; references and invalidation for a synchronous subset; `String`, `Array` | the examples of `values/` and `control-flow/` run, with ASan clean |
| **M2** | generics and monomorphization; specialization by view, `Type(parameter)` and instance deduplication; the staged local cache | the worked example above produces two instances and one alias; a body edit rebuilds one unit |
| **M3** | `Set`, `Map`, `Pool`, `Columnar`, `Growing*`; relations; fusion; the standard library on intrinsics | the examples of `collections/` and `advanced/relations` run; benchmarks vs C++/Rust/Zig exist |
| **M4** | the `release`, `max` and `size` profiles; cross-module inlining; parallel codegen; ThinLTO; remote cache; verify mode in CI | compile-time budget in CI; differential tests green |
| **M5** | async, workers, `for parallel`; `wasm` | the examples of `advanced/concurrency` run on `native` |

---

## Questions to decide

The documentation is the specification, so open questions are listed, not applied, each with one recommendation. Every question is decided: they stay here as a record, and point to the section or page that applies them. Q1 and Q4 to Q10 were decided on 2026-11-01 with one criterion, the best runtime performance and the fastest compilation. The [spec review](spec-review.md) of the same day settled many other points of the language; it lists them.

### Q1. Canonical field order and the C ABI

Decided (2026-11-01): **canonical order for every Fa type** (alignment, largest first, then name). Equal shapes share one layout, one set of instances and one cache entry, which compiles less code, and padding is minimal, which runs faster in less memory. The canonical layout is never an ABI.

A C layout is **opt-in per type**, with the `CStruct` type constructor (nominal, fields in declaration order, C alignment rules), so that buffers shared with C or the GPU (vertex buffers, OS structures) are passed without a copy. A type never gets a C layout implicitly. Foreign code was specified the same day: see [foreign code](../documentation/advanced/foreign-code.md) and [the section below](#foreign-code).

### Q2. Structural arguments through function values

Decided: direct calls are specialized; function values use an offset table ([function values](#function-values)); a read-only collection of supersets is accepted, a mutated one needs a generic function; function types declare their effects, and storing a function requires all of them.

### Q3. May a mutated place be aliased by another argument?

`noalias` on a `mutates` parameter, and `readonly noalias` on the others, is the largest single optimization available, and it needs a rule the documentation does not state:

> A place passed to a `mutates` parameter must not overlap any other argument of the same call, nor a `requires` value used by the callee.

`addBonus(player)` while another argument is `player.score` would be a compile error, as in Rust. The `use` aliases remain free to alias each other (they are plain pointers); only calls are constrained.

**Recommendation: adopt the rule.** If it is rejected, the compiler never emits `noalias`, and read-only parameters lose `readonly` whenever the function also mutates something else reachable from them.

Decided (2026-11-01): adopted, extended to the module-level values the callee uses and to the references captured by callback arguments. See [reference checking](../documentation/advanced/reference-checking.md).

### Q4. Where does the environment of an escaping closure live?

Decided (2026-11-01), the compiler-made heap blocks of [memory management](../documentation/advanced/memory-management.md#heap-blocks-made-by-the-compiler), each uniquely owned by its value:

- **Non-escaping closures** keep their environment in the caller's frame. Most have no environment object at all: a lambda passed to a generic function is specialized and inlined, and its captures become plain values of the caller.
- **Escaping closures**: a function value is `{ fn, env }`. An environment of at most 8 bytes that is trivially copyable is stored **in the `env` word itself**, with no allocation (one captured integer, float or small object). A larger environment is a heap block whose header points at a static descriptor `{ size, align, copy, drop }`; copying the closure copies the block, destroying it frees the block.
- **Task frames**: the frame size of each async instance is known at compile time, so frames come from a **per-thread size-class allocator** (free lists per size class, no lock, no atomic). A frame is freed when its task is awaited, cancelled or destroyed.
- **`BigInt`**: one 64-bit word. A value that fits in 63 bits is stored inline (tagged), so the common case is plain integer arithmetic with an overflow check; a larger value points at a uniquely owned block of limbs.
- **`Character`**: up to 15 bytes of UTF-8 inline (every grapheme in practice); longer grapheme clusters go to a heap block.

The documentation now says that objects are values stored inline, where they are declared (in a frame, in their parent, in a collection slot), rather than "on the stack".

### Q5. String representation and the cost of grapheme `length`

Decided (2026-11-01). `length` is not O(1) in general, but every common case is, and no loop over indices is quadratic:

- **Small-string optimization**: a `String` is 24 bytes, and a string of at most 23 bytes is stored inline in them (the last byte holds the remaining capacity and the flags). Most keys, names and identifiers never allocate.
- **Flags**: static (a literal in read-only data), inline, ASCII. The ASCII flag is maintained on every write by a vectorized check of the new bytes; with it, `length`, indexing and slicing are byte arithmetic.
- **Literals** are read-only blocks whose header holds their grapheme count and ASCII flag, computed at compile time.
- **Heap strings** have a block header with a cached grapheme count (computed on the first `length` after a write) and a pointer to lazily built **breadcrumbs**: the byte offset of every 64th grapheme, built by the second indexing of a non-ASCII string after a write. Indexing then scans at most 64 graphemes, so `for index in 0..text.length { text[index] }` is linear. A write clears both, which costs nothing when they were never built. Inline strings need neither: scanning 23 bytes is cheaper than any cache.
- Sequential iteration stays one segmentation scan, and remains the fastest way to read a whole string.

### Q6. The price of Map's stable slots

Decided (2026-11-01): **keep the stable slots**, a language guarantee (`getOrInsert` declares `mutates self.size`, and references survive insertion), with no inline variant. The indirection is made as cheap as possible:

- **Small maps** (at most 8 entries) have no index: a lookup is a linear scan of the entries, often within one or two cache lines.
- **The index** is a Swiss table of control bytes (7-bit tag, SIMD group match) and `u32` entry ids (`u64` above 2^32 entries). A tag mismatch filters 127 of 128 wrong candidates before any entry is touched, so the extra load happens about once per successful lookup.
- **The entries** live in a segmented arena with doubling segments, like `GrowingArray`: an entry id becomes an address with one `clz`, two shifts and one load of the segment pointer, and segments never move. Each entry stores its 32-bit hash, so a rehash rebuilds the index by reading the entries sequentially without hashing keys again. Deleted entries go on a free list (iteration order is not promised).

### Q7. The internal intrinsic layer for the standard library

Decided (2026-11-01): **the collections are written in Fa**, in the standard library, on top of an internal namespace `intrinsics` that only the standard library's files can see. It is not written with `@`, which means compile time in Fa. It provides a `Pointer(T)` type and operations that each lower to one LLVM instruction or intrinsic: sized `allocate`, `reallocate` and `free`, `offset`, `load`, `store`, `copyBytes`, `moveBytes`, `fillBytes`, `countLeadingZeros`, `countTrailingZeros`, `popCount`, `byteSwap`, a 16-byte group match for Swiss tables, atomic operations, and the hints `likely`, `assume` and `unreachable`.

- **Run fast**: the element's `hash`, `==` and copy are inlined into the collection code by monomorphization, which a type-erased runtime can't do.
- **Compile fast**: the collections go through the same monomorphizer and the same cache as user code, and the instances of the common types (`Array`, `Set` and `Map` of the primitives and of `String`) are **precompiled per target and profile and shipped with the compiler** as a seed of the cache: a small program instantiates none of them.
- On JavaScript, the collections are JavaScript structures with their own implementations, as before.

### Q8. Module-level mutable values and threads

Three gaps: (1) the **initialization order** of non-constant module-level values, especially across files with cyclic references; (2) their **destruction** at program exit; (3) what a **worker thread** sees of a module-level `export mutable` value, since workers share nothing. **Recommendation:** initialization in dependency order, a cycle being a compile error; destruction in reverse order; module-level mutable values are rejected inside workers unless their type is thread-safe.

Decided (2026-11-01): initialization in dependency order, a cycle between value initializers is an error, destruction in reverse order (see [modules](../documentation/architecture/modules.md)); the worker rule is in [concurrency](../documentation/advanced/concurrency.md).

### Q9. The native runtime and the documented use of Tokio and Rayon

Decided (2026-11-01): **a runtime of our own**, in `fa-runtime` (Rust, no Tokio, no Rayon), prebuilt per target and profile:

- **One single-threaded executor per thread**: a ready queue and a timer wheel. Tasks never move between threads (they run on one thread), so the hot path has no atomic and frames need no synchronization. Tokio's main value, its work-stealing multi-threaded scheduler, would be unused.
- **I/O**: `io_uring` on Linux (with an `epoll` fallback), `kqueue` on macOS and the BSDs, IOCP on Windows; on `wasm`, the host's event loop through imports.
- **`for parallel`**: one process-wide fork-join pool, one thread per core, Chase-Lev work-stealing deques and lazy binary splitting of the index range; the calling thread takes part. The outlined body receives a range `(start, end, env)`, not one index, so LLVM vectorizes its inner loop. Rayon's API is generic Rust closures, which LLVM-generated code could only reach through a type-erased call per chunk.
- **Workers**: OS threads, each with its own executor; a call is a message on a multi-producer queue. A `Mutex` wakes a waiter of another thread through that thread's executor (an `eventfd` or a `kqueue` user event).
- **Compile fast**: a small runtime links faster than one embedding Tokio and Rayon, and makes smaller binaries; the frame allocator, the wakers and the `wasm` port are ours to tune.

### Q10. Where this document should live

Decided (2026-11-01): design records live in `design/`, at the repository root: this document and the [spec review](spec-review.md). Not in `documentation/`, whose pages are the language specification rendered by the website, and not in `idea/`, which holds unsorted notes.

### Q11. Writing through a wider view

Decided: reading is covariant, writing is invariant. See [writing through a view](#3-passing-a-superset-where-a-subset-is-expected).

### Q12. The order of callbacks in a fused chain

Decided: step by step, fused only when every callback is pure. See [iteration and fusion](#iteration-and-fusion). Element-by-element evaluation is explicit, with `values()`: see [Q17](#q17-an-explicit-element-by-element-chain).

### Q13. Copying an owner whose collections are linked by relations

Decided: a whole owner copies its internal relations; a single element copied out of its collection keeps no links. See [the allocator interface](#the-allocator-interface).

### Q14. Tasks that borrow

Decided: tasks are scoped. See [async, workers and `for parallel`](#async-workers-and-for-parallel). Refined on 2026-11-01: tasks are move-only, leaving their scope cancels and waits for them, and a `start` whose task is not kept is detached (it copies what it uses).

### Q15. An unannotated copy of a parameter

Decided: `let copy = human` has the type of the argument, `Type(human)`, and keeps every field. See [`Type(parameter)`](#4-typeparameter-when-the-full-type-matters).

### Q16. Copying a parameter in JavaScript

Decided: a JavaScript function that copies a parameter without annotation, or reflects on it, is emitted once per argument type, as in native code; a function value receives the copy function as a hidden argument. See [JavaScript](#javascript).

### Q17. An explicit element-by-element chain

Decided: `values()` returns an iterator, as with JavaScript's iterator helpers (`array.values().filter(f).map(g).toArray()`). See [element by element with `values()`](../documentation/control-flow/loops.md#element-by-element-with-values). An iterator is a local reference into its collection. Lowering: a chain consumed where it is written (`toArray`, `reduce`, `find`, `for`) is one loop, in native code and in JavaScript, since its semantics already is element by element; an iterator kept in a variable is a small struct (a position in the collection, plus the captured callbacks). JavaScript can also emit the native iterator helpers directly.

The method that keeps the first elements is `firstValues(count = 1)`, not `take(n)` as in JavaScript: `take` already moves an element out of a collection in Fa, and one name must have one meaning. Its plural name echoes `values()` and makes clear that it returns an iterator, not an element.

### Other gaps to check

Raised during review, all decided on 2026-11-01:

- the value of a `Set` element and the key of a `Map` entry must not change while they are stored: references to them are read-only, and they don't accept structural views of supersets;
- `Float` follows IEEE 754 for `==` (`NaN != NaN`, `-0 == 0`) and SameValueZero as a `Set` element or `Map` key; values containing functions can't be compared; tasks are move-only; relation fields are not compared by `==`;
- the `LinkedList` example declares exact effects (relations page);
- `Any` is removed from the language.
