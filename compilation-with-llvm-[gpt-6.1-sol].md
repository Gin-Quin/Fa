# Compiling Fa with LLVM: strategy

Author: GPT-6.1-sol. Written against the working-tree documentation on 2026-10-27.

This is an implementation proposal, not a change to the language specification. It uses
the current documentation, including its uncommitted revisions, as the intended language.
No other `compilation-with-llvm*.md` document was read. Unsettled language rules are called
out explicitly; their proposed resolutions require a separate specification decision.

## 1. Recommended direction

Build a native compiler in Rust around a small, typed Fa intermediate representation,
then emit LLVM IR directly through an in-process LLVM API. Keep memory safety, structural
typing, copy semantics, collection operations and effect checking in the Fa frontend.
Give LLVM conventional control flow, concrete layouts and ordinary memory operations.

Use cached native object files for fast development and ordinary builds. Offer a stronger
release pipeline with cached ThinLTO, selective specialization and eventually profile-guided
optimization. Compile the runtime and standard libraries once per compatible toolchain,
target and build configuration, rather than compiling Rust runtime dependencies inside
every Fa application build.

There is a real trade-off between compilation time and the last few percent of execution
performance. Expose that trade-off through a few profiles instead of making every build
pay for whole-program optimization.

| Goal | Main mechanism | Cost that remains |
| --- | --- | --- |
| Fast first build | Small frontend IR, direct LLVM emission, prebuilt runtime | Semantic analysis and code generation for new code |
| Fast repeated build | Dependency-aware semantic and native object caches | Discovering and validating changed inputs |
| Fast edited build | File-sized compilation units, interface summaries | Recompiling affected units and linking |
| Fast execution | Concrete data, static safety proofs, inlining, vectorization | Irreducible allocation, lookup, errors and relation updates |
| Highest release performance | ThinLTO, measured specialization, optional PGO | Longer optimization and link steps |
| Simple implementation | One semantic model, a few representations, stock LLVM passes | The safety analysis itself cannot be skipped |

The initial backend should be ahead-of-time compilation for one native platform. Add
other native platforms and WebAssembly through LLVM target support after the semantics
are reliable. Preserve the documented JavaScript development path. Native hot patching,
an LLVM JIT, MLIR and a second optimizing backend are unnecessary for the first compiler.

## 2. Starting point and architecture

The repository already has a useful indexed syntax tree, source locations and syntax for
effects, references, relations, compile-time operations and concurrency. However,
[`analysis/analyze.rs`](packages/parser/src/analysis/analyze.rs) is a stub, and the existing
inference code falls back to `Any` for many cases. For example, it currently infers a
32-bit integer where the documented default is `Int64`. The existing `TypedSyntaxTree`
name must not be taken as evidence that full semantic checking is implemented.

Actual parser entry points are now in
[`parsing/parse.rs`](packages/parser/src/parsing/parse.rs) and
[`parsing/parse_single_statement.rs`](packages/parser/src/parsing/parse_single_statement.rs).
The filesystem package already provides a place to start module discovery. There is no
need to duplicate the parser or let an LLVM backend infer types independently.

The proposed pipeline is:

```text
source + locked dependencies + effective configuration
    -> filesystem namespaces, parsed files and declarations
    -> elaboration: resolve names/types, evaluate compile-time code, check effects
    -> typed Fa control-flow IR with places, ownership and reference provenance
    -> safety/refinement analysis and explicit copy/move/drop lowering
    -> concrete layouts and LLVM IR
    -> profile-specific LLVM optimization and object/bitcode emission
    -> platform link
```

Elaboration is not necessarily one left-to-right pass: type functions and compile-time
evaluation can reveal additional types and dependencies. Make these memoized queries with
cycle detection. Resolve signatures first; check recursive groups together where needed.

Start with one executable Fa IR, plus side tables on the existing syntax tree. It needs
basic blocks, typed values, calls, branches, returns, and explicit **places**: a local,
global, field, indexed element, union payload or columnar row. Keep a few meaningful
operations such as `Copy`, `Move`, `Drop`, `CheckIndex`, `RelationSet` and collection calls
until their semantic analysis is finished. Lower them before handing the program to LLVM.
Do not create a succession of largely redundant IRs.

Use interned symbols and types, integer IDs and arena-backed storage. Separate immutable
type information from per-program-point facts such as a known index, length or valid
reference. Memoize structural compatibility and type-function evaluation. Analyze loops
with a bounded fixed-point algorithm and widening; an uncertain fact must become unknown,
not trigger exponential reasoning or an unsound optimization.

The documented compile-time interpreter should execute this same semantic model, with
deterministic library operations and target-correct arithmetic. It must not need LLVM or
JIT compilation for every `@call`. Reuse its evaluator for configuration and compile-time
tests. Source spans should survive every stage for useful diagnostics.

Suggested eventual package responsibilities are a shared semantic/compiler core, an LLVM
backend and a runtime. Add packages when the boundary becomes useful; keep LLVM out of
the parser and language server. `fa check` and cached editor analysis should not initialize
LLVM at all.

## 3. Compilation units, libraries and caches

### Begin with files, refine only after measurement

Use one Fa source file as the initial LLVM compilation unit. Folder namespaces do not
require runtime objects or force a whole folder into one LLVM module. A namespace export
can contain several functions in that file. Cross-file recursion uses ordinary symbol
declarations; cyclic semantic dependencies need strongly connected groups during checking,
but do not inherently require one giant LLVM module.

File-sized units keep the first implementation understandable. Very large generated files
can later be split into deterministic groups of functions. One LLVM module per tiny
function adds setup, optimization and link overhead, so do not assume it is fastest.

### Cache distinct stages

| Cached artifact | Essential inputs |
| --- | --- |
| Parsed file | Source content, parser/schema version |
| Checked interface | Exported semantic types, effects, requirements, relevant compile-time results |
| Checked body / Fa IR | Source and exact semantic dependencies, compiler semantics version |
| Generic or structural specialization | Definition/body hash, semantic arguments, representation, async mode |
| Native object | Lowered IR, target and full backend configuration |
| ThinLTO result | Bitcode, imports/exports and whole-program decisions; managed by LLVM's cache |
| Linked executable | Ordered input manifest, runtime/system libraries, linker and link options |

Use content hashes, not timestamps as proof of equality. Timestamps and a watcher can
avoid unnecessary reads, but a clean build must validate the underlying inputs. Persist
versioned records without serializing pointers, arena addresses or process-local IDs.

An object cache key must include compiler and runtime ABI versions, LLVM build/version,
target triple and data layout, CPU/features, optimization and code-generation settings,
debug/sanitizer settings, relocation/PIC and code model, and all options that affect the
output. Configuration and compile-time `target` inputs belong in semantic keys whenever
they affect elaboration. Do not use host CPU capabilities implicitly in a portable build.

Dependencies include more than explicitly called functions:

- The exported type, return ownership/provenance, `throws`, `requires`, `mutates` and
  `invalidates` summaries consumed by a caller.
- Type functions, default values, inline bodies and generic definitions used during
  elaboration or specialization.
- Every file read by compile-time code, and directory entries read by `@readDirectory`.
- Locked remote content and resolved package versions.
- Filesystem name-resolution results, including previously missing names: adding a sibling
  file can change implicit namespace resolution even when no old source file changes.

Record dependencies at the query that used them. A private implementation edit can reuse
callers if its public semantic summary is unchanged and they did not consume its body.
An inferred effect change is an interface change even when the programmer did not edit a
signature. A layout change invalidates every user of that layout. Link again when object
contents change even if caller objects remain reusable.

Cache compile-time evaluations using their function bodies, arguments and recorded input
dependencies. Validate their previous input manifest before accepting a hit; discover and
store a new manifest when evaluating again. Resolve `@import` through the locked dependency
loader rather than allowing arbitrary network I/O in the interpreter.

Publish cache entries atomically, tolerate interruption/corruption as misses, and use a
bounded eviction policy. Store shared entries by content and use per-project manifests to
find them. A background compiler can retain interned data and LLVM initialization between
edits, but disk caches must also work in a fresh process and offline.

### Reuse libraries without changing source distribution

Fa libraries are [distributed as source](documentation/guides/deploy-a-library.md). Keep
that contract. On first use, cache checked interfaces, reusable bodies, native objects
and required generic instantiations. On later uses, reuse compatible artifacts across
projects. A cached native archive is a local build product, not a new package format.

Prebuild the allocator, Unicode support, platform I/O and optional concurrency runtime
for supported targets/profiles. Keep ordinary generic collection algorithms as small,
specializable Fa/compiler IR bodies; keep target/runtime services behind a narrow ABI.
Demand code generation from entry-point reachability so an unused worker or networking
module does not pull Tokio or the entire standard library into a simple program.

Give generic instances deterministic symbols based on definition identity and semantic
arguments. Assign a deterministic home or use valid ODR/COMDAT deduplication supported by
the target. Cache instances across consumers instead of recompiling every template in
every caller. Linker deduplication is a fallback; it does not refund duplicate compilation.

For the fast path, link cached objects normally. For ThinLTO, retain appropriate bitcode
and summaries, use the pre-link ThinLTO pipeline, and enable the linker's ThinLTO cache.
An unchanged module can still need new machine code when its imported body or global
analysis changes. Do not substitute the ordinary object cache for LLVM's ThinLTO cache.
The [ThinLTO documentation](https://clang.llvm.org/docs/ThinLTO.html) describes this separate
summary/import/backend process and its cache controls.

## 4. Structural typing designed for native code

### Separate semantic compatibility from physical layout

An exact object shape gets a concrete LLVM struct layout. Equal structural types should
reuse their semantic representation where the language permits it; a larger shape is
compatible with a smaller constraint, but generally has a different layout.

Keep at least these concepts distinct:

1. A **semantic type/constraint**, including effects, ranges and owner dependencies.
2. An **exact stored representation**, including fields, alignments and union encoding.
3. A **reference view**, describing how to access the fields of an existing representation.

Normalize field sets and anonymous unions for stable identity. Recursive structural types
need a cycle-aware graph representation and equivalence check, rather than infinitely
expanding their definitions. Named union/member identity must survive normalization.
Layout equivalence alone is insufficient for sharing generic code: compile-time reflection,
nominal union distinctions, effects or ranges can make two otherwise similar instances
behave differently.

Use deterministic layout rules, initially a canonical field order, and obtain padding and
alignment from the target data layout. Preserve source evaluation order and any observable
schema/display ordering separately. Canonical field order does **not** make narrower
structs prefixes of wider ones: extra fields can change offsets. A layout policy is a
versioned Fa ABI decision, not something to infer from Rust's default struct layout.

### A view contains references to the requested fields

For example:

```fa
type Named = { name: String }
type User = { id: Integer, name: String, email: String }

function readName = (value: Named): String => value.name
```

Compile the ordinary separately compiled `readName` entry point with a view of `name`,
conceptually a pointer to the caller's string field. The caller computes that pointer
using the exact `User` layout. No string or `User` is copied to pass the argument.
Returning `value.name` as an owning `String` is a separate copy operation; only argument
passing is free of copies in this example.

For several required fields, the view is a small aggregate of field references. Flatten
small views into pointer arguments; pass larger descriptors by reference when the ABI
requires it. For nested shapes, compose views recursively. Reuse the same mechanism for
references to fields held in separate columnar arrays. View adaptation constructs addresses;
it does not allocate, hash field names or validate shapes at runtime.

Equal exact layouts can use a plain object pointer when no adaptation is needed. Known
calls and inlining can eliminate the view wrapper. For hot paths, optionally specialize
the callee for an exact layout or a columnar access pattern, yielding constant-offset
loads. Keep the common view entry point so an ordinary structural function does not need
one compiled body for every larger shape that ever calls it.

This is a deliberate compromise: a non-inlined view can use more arguments than a single
pointer. It still avoids heap boxing, string-based property lookup and mandatory runtime
type descriptors. Measure view size and call cost before adding a second descriptor
scheme. If a compatible field has a different representation, such as a widened union,
it needs a correctly typed adapter or specialization; reinterpreting its pointer is wrong.

### Copying and aliasing must remain different

The [structural typing specification](documentation/advanced/structural-typing.md) says:

- `let narrow: Named = user` constructs a new owning value containing only `name`.
- `use narrow: Named = user` constructs a borrowed view and hides the other fields.
- Passing `user` to a `Named` parameter borrows that same storage.
- A generic parameter `Item` preserves the full argument type when the function returns
  `Item`; a plain `Named` parameter does not become an implicit full-type generic.

Generate projection-copy helpers, not `memcpy(sizeof(Named))` from the start of `User`.
Nested collections and strings need proper owning copies. Dropped extra fields still
belong to the original; narrowing a copy must not destroy the original's fields.

For mutations through a narrow view, write only permitted fields into the original place.
Parameters cannot be replaced as a whole under the documented rules. Do not overwrite
an entire wider object with a narrower struct or accidentally destroy its extra fields.
Writable fields need compatibility in both read and write directions: a view accepting
any `Integer` cannot write into an actual `Integer(min = 0, max = 10)` field. The same
issue applies to wider unions, nested mutable objects and mutable collections.

### Generics and dynamic function values

Monomorphize explicit generics by semantic arguments and compile-time values, then cache
them. Type-check a generic body against its constraints before specializing. Ordinary
shape-constrained functions use the view ABI; specialization is an optimization, not a
requirement for every structural call.

Use deterministic limits on optional specialization and inlining to control code size.
Mandatory generic expansion still needs cycle/depth diagnostics; silently erasing a
generic that depends on `Type(Item)` would change the language. Deduplicate instances
only after establishing semantic equivalence, not merely equal LLVM types.

A stored function value has a stable signature-specific calling convention. Calls to
known methods/functions remain direct. Function-pointer fields and escaping closures use
an indirect callable representation; any adapter must preserve parameter views, return
ownership, requirements and effects. Required services can use these callable fields
without turning every structural object into a virtual object.

## 5. Use the memory model to prove safety before LLVM

### Represent local references as ordinary addresses

For a valid local `use` reference, a machine pointer is usually sufficient. A string slice
needs an address and length; a columnar row needs field references or a row locator.
Collection identity and lifetime normally remain compile-time facts. There is no need to
attach a generation number, reference count or validity check to every dereference.

This is sound only after implementing the actual safety analysis. Track each reference's
owner **instance**, root place, projection, lifetime and known element/key identity.
`townA.people` and `townB.people` must not be confused because both originated from the
same field declaration. Returned references substitute callee owner paths with the actual
caller arguments. A reference with unresolved owner identity cannot simply be assumed
to belong to the required collection.

At each mutation/call, instantiate the checked effects using pre-call values and invalidate
every possibly overlapping reference or refinement:

- `Array.push`: all references into array storage, regardless of spare capacity.
- `Array.delete(i)`: references to indices at or after `i`.
- `Bag.delete(x)`: references that may designate `x`, including aliases of aliases.
- `Map.delete(k)`: references whose key may equal `k`.
- Replacing a string/collection/union payload: references into its previous owned memory.
- Replacing a flat field: references to that still-existing place stay usable.

Keep exact keys, ranges and fresh insertion identities when cheaply provable. At joins,
validity must hold on every incoming path; loops need a conservative fixed point. Distinct
insertions can prove distinct elements, but a static call-site ID alone is not a unique
runtime element identity across arbitrary loop iterations.

Effect checking must also work **inside** callees when arguments alias each other or a
global/requirement/capture. Checking only references in the caller after a function returns
does not prevent a callee from deleting storage and then reading it through another
parameter. Higher-order function effects, closure captures and relation side effects
must participate in the same analysis. Reject an uncertain unsafe use and explain the
invalidating operation; do not silently install a runtime borrow checker.

### Copy, move and destruction

Assignments are owning copies unless a transformation is proven unobservable. Emit
type-specific clone/drop logic once per semantic representation. Flat data can use plain
loads/stores or bulk copies; strings, collections, recursive children and closures need
ownership-aware handling.

Use destination construction for returns and collection insertion. Replace a copy with a
move only when no later value use, live alias, required destruction behavior or relation
invariant can distinguish it. A source being unused by name is insufficient if a `use`
alias still observes it. A still-usable insertion argument must retain its value: the
documentation's insertion examples must not become unconditional destructive moves.

At scope exits, destroy values in the specified reverse declaration order. Share cleanup
blocks where useful, and cover `return`, `throw`, `break`, `continue`, partial initialization
and eventual async cancellation. Generate no drop for trivial types. Recursive tree clone
and destruction should use iterative traversal or another bounded-stack strategy for
deep inputs. Collection teardown must still run nested element destructors; releasing
an arena's blocks is not enough when elements own their own allocations.

Avoid pervasive copy-on-write or reference counting. They add branches, complicate mutation
and conflict with the documented unshared ownership model. Immutable static literals can
use read-only storage with a representation that distinguishes static from owned bytes;
mutable owning copies must preserve their independent value semantics.

### Do not overpromise LLVM alias information

Memory safety does not imply Rust-style exclusive mutable borrows. Fa allows aliases;
two parameters can designate overlapping memory unless the compiler proves otherwise.
Neither `use` nor `mutates` justifies blanket `noalias`, and an immutable parameter does
not imply that another parameter or relation cannot mutate the same storage.

Emit `noalias`, alias scopes, `readonly`, `nocapture`, `nonnull`, `dereferenceable`, alignment
and lifetime information only when their precise LLVM contracts hold. In particular,
`readonly` describes writes through a parameter, not global immutability; `nocapture`
must account for returned references. Lifetime-end markers require the absence of later
accesses through aliases. Start conservatively and add annotations behind focused tests.
The [LLVM language reference](https://llvm.org/docs/LangRef.html) is the authority for these
contracts, poison values, overflow flags and pointer arithmetic.

## 6. Collections: familiar techniques, Fa-specific guarantees

Keep element storage concrete and specialized. Hot indexing, iteration and append paths
should be tiny IR bodies LLVM can inline. Use out-of-line slow paths for allocation,
rehashing and diagnostics, rather than enormous always-inlined collection implementations.

| Collection | Proposed representation | Important constraint |
| --- | --- | --- |
| `Array(T)` | Contiguous buffer with pointer, length, capacity | Growth/shift invalidation follows documented effects |
| `Bag(T)` | Nonmoving chunks of slots, occupancy metadata and free list | Never swap-remove or compact live slots |
| `Columnar(T)` | Aligned chunks of separate field arrays with shared row occupancy | Growth/deletion cannot move surviving fields |
| `OrderedColumnar(T)` | Resizable contiguous field arrays, common length/capacity | Growth and ordered deletion invalidate like arrays |
| `Set(T)` | Flat open-addressed hash table with compact control bytes | Rehash may move elements; hash must agree with equality |
| `Map(K,V)` | Hash index pointing to stable value slots | Rehash moves index entries, not live values |
| `GrowingArray(T)` | Append-only indexed chunks | Existing elements keep stable addresses |
| `GrowingBag(T)` | Append-only chunk arena | No individual deletion metadata needed |
| `GrowingSet/Map` | Rehashable index plus nonmoving element/value chunks | Append must preserve existing references |
| `GrowingColumnar(T)` | Append-only field chunks with synchronized row allocation | No compaction of existing rows |

### Arrays

Follow the vector approach familiar from C++, Rust and Zig: contiguous elements,
amortized growth, explicit initial capacity, and a small append fast path. Rust's
[`Vec` documentation](https://doc.rust-lang.org/std/vec/struct.Vec.html) explains the relevant
capacity and contiguous-storage guarantees; Fa should choose and benchmark its own
growth policy rather than promise another language's unspecified growth factor.

Check allocation-size multiplication and length arithmetic for overflow before allocation.
Use bulk relocation for types proven trivially relocatable. For owning elements, relocation
transfers ownership without duplicating or dropping the old bits as a second value.
Relation-containing values need stricter treatment described below.

An array copy allocates independent storage and clones owning elements. Slicing with
`let` is a copy under the current specification. Iteration borrows the buffer and lowers
to a canonical counted loop. Preserve references to elements before a deleted suffix,
as the documented effects require. Reserve capacity optimizes allocations but does not
change `push`'s public invalidation effect.

### Bags

Allocate many slots per chunk and keep chunk allocations fixed. An expandable table of
chunk pointers may move because element pointers designate the chunks themselves.
Insertion uses a reusable free slot or bump allocation in the current chunk. Deletion
runs relation cleanup and element destruction, then returns the slot to the free list.

Use occupancy bitsets to skip holes and optionally track nonempty chunks. Dense iteration
is excellent inside occupied chunks; heavily fragmented bags need measurement. Do not
claim that every live object is one globally contiguous array. Reclaim a wholly empty
chunk only when no legal reference or relation can still designate its storage.

There are no generation checks in ordinary Fa references. Static invalidation and eager
relation cleanup must make slot reuse safe. Metadata for allocation and iteration is
still necessary; absence of reference-validation overhead does not mean absence of metadata.

### Columnars

Use synchronized row slots across field chunks. A row can be represented internally as
a chunk/slot locator; field access becomes a known column base plus row offset. A local
view can instead hold direct field pointers. Destructured loops over `{ age }` must walk
the age column directly without assembling temporary `Human` objects.

Zig's [`MultiArrayList` source](https://raw.githubusercontent.com/ziglang/zig/master/lib/std/multi_array_list.zig)
is a useful reference for field-separated storage. Fa's stable `Columnar` additionally
needs chunking and shared occupancy. A single reallocating `MultiArrayList`-style buffer
does not satisfy Fa's stable-address guarantee; it is a closer fit for `OrderedColumnar`.
Whole-row copies gather fields; whole-row borrowed access can remain a field view.

### Sets and maps

Use a Swiss-table-style index: compact fingerprints/control bytes and grouped probing,
with a portable scalar implementation and SIMD paths when profitable for a target.
[Abseil's design notes](https://abseil.io/about/design/swisstables) describe this approach.
For a `Set`, elements may live inline. For a `Map`, store references/slot locators to stable
values, accepting the indirection promised in the specification.

Generate equality and hashing from the same semantic type. Do not hash padding bytes or
compare an owning struct with `memcmp`. Arrays are ordered; sets and bags are unordered,
and bag multiplicity matters. For floating values, signed zero and NaN need an explicit
hash/equality policy. Use collision-resistant defaults for externally controlled keys;
do not require one predictable hash seed merely to make builds deterministic.

An arbitrary `compare = (a, b) => ...` supplies equality, not a compatible hash. Initially
use a correct linear-search fallback for custom comparison. A later explicit key/hash
facility can restore average constant-time lookup if its contract is specified. Inferring
a hash from arbitrary callback code is not a sound or simple general solution.

### Growing variants

`GrowingArray` cannot be an ordinary vector with deletion methods removed: vector growth
still moves its elements. Use fixed-size indexed chunks or a similarly stable scheme.
Indexing has an extra chunk calculation/load, while chunk-wise iteration remains dense.
Likewise, `GrowingSet` must keep its values stable when its hash index rehashes.

Append-only storage avoids per-element reclamation, but destroying nested strings, arrays
and recursive children can still take O(n). Several chunks usually mean several frees.
The documentation's expressions such as "one deallocation" should be understood as
bulk lifetime management, not an achievable universal constant-time teardown guarantee.

## 7. Relations: spend work on updates to make reads cheap

Relations are a central part of the native representation, not ordinary pointers with a
validity check added later. Resolve relation declarations and their owner collections
statically. Generate specialized update and destruction helpers from each relation schema.

An optional direct relation can hold a nullable stable target pointer, or a stable row
locator for a columnar target. The owner identity is statically known. A reverse relation
stores the bookkeeping needed to find incoming edges without scanning all objects.

For one-to-many links, an intrusive edge/list entry with a back-link to its reverse slot
allows constant-time detachment. For many-to-many, allocate a stable edge record linking
both adjacency lists, with enough links to remove it from either side in constant time.
Duplicate edges need their own records if relation bags retain ordinary bag multiplicity.
This metadata is compiler-maintained relation storage, not a generation-validation system.

Generate direct-side writes as detach/update/attach operations. One-to-one and symmetric
writes also detach displaced partners. Handle self-links, assigning the existing target,
duplicate many-to-many edges and cleanup of the last edge explicitly. Potential allocation
must occur before exposing a partially updated relation, or the operation needs rollback.

Before recycling a deleted object slot, disconnect all incoming and outgoing edges and
clear relevant owner fields. Deletion takes O(k) relation work for k incident edges, plus
destruction of owned values. Owner fields can be cleared using the statically known set
of such fields in the containing owner. No collection-wide scan is necessary.

Relation traversal then needs only the normal optional test when absence is possible,
followed by a load. It needs no generation comparison, liveness lookup or reference count.
Do not describe that optional test as an unnecessary runtime validation: `none` is a real
language-level state.

Every write path must use generated helpers: direct assignments, `.=` updates, replacement,
`take`, copy, deletion and bulk clearing. Byte-copying reverse-list pointers would corrupt
the graph. Invalidation/effect analysis must include implied reverse writes, displaced
partners and owner-field cleanup. A local reference obtained by following a relation must
remain tied to the target's owner collection, even if that relation later becomes `none`.

Copying or moving an owner containing internal relations needs a graph-aware protocol;
see the decisions below. This is where a blanket "all owning values are trivially movable"
assumption would fail.

## 8. Remove checks by proof, preserve required failures

Use a lightweight abstract interpretation over the Fa IR: integer intervals, literal
keys/values, union alternatives, collection length/presence facts and selected relational
facts such as `0 <= index < collection.length`.

These proofs have two jobs. They determine whether indexing/arithmetic can contribute a
typed error to a function, and they allow code generation to omit the corresponding check.
LLVM optimization cannot replace frontend reasoning here: a public `throws` signature
must be correct before LLVM sees the program.

| Operation | When the check can disappear | When work remains |
| --- | --- | --- |
| Local reference dereference | Lifetime and invalidation analysis accepts it | Unsafe source use is rejected |
| Array access | Index normalized and proven within current length | Checked access branches to `IndexError` |
| Map access | Key presence is known and not invalidated | Hash lookup remains; unknown absence raises `KeyNotFound` |
| Optional/union access | Dominating refinement remains valid | Tag/null branch selects the actual value |
| Integer overflow | Result range fits the selected width | Overflow test and terminating diagnostic, or explicit checked variant |
| Division by zero | Divisor is proven nonzero | Typed `DivisionByZero` path |
| Structural field access | Static view construction established the field | No dynamic shape validation |
| Input decoding | Never assumed from untrusted bytes | Validate once while constructing a typed value |

Facts must be killed when an aliasing write, effect, relation update, callback, suspension
or thread can change the relevant state. A proof about `array.length` before an arbitrary
mutating call is not a proof about its length afterwards. Public parameter range contracts
are checked in callers and at foreign/input boundaries; they are not silently trusted
when decoding JSON or receiving external data.

Normalize negative indices without overflowing on `Int64.MIN`. Widen intermediate
arithmetic or use safe comparisons. On 32-bit/Wasm targets, bounds on an `Int64` index must
be established before narrowing it to the target pointer width. Preserve inclusive range
semantics without blindly computing `maximum + 1` at the integer limit.

Lower possible integer overflow with appropriate checked arithmetic, for example LLVM's
overflow intrinsics, and branch to a cold failure block. Only use `nsw`/`nuw` where proof
justifies them; these flags do not perform overflow checking. Ranged `Integer` remains
64-bit unless a separate representation optimization is proven compatible with its ABI.

Fa's `//` and `%%` use floor semantics, while LLVM signed division/remainder truncate
toward zero. Emit the correction when a nonzero remainder and differing signs require
it. Guard the signed-minimum divided by `-1` case before LLVM signed division, and specify
the corresponding remainder behavior. `/` produces floating output; it is not integer
`sdiv`. Dynamic shifts also need the documented range check.

Emit direct branches around fallible operations, then let LLVM simplify them. Prefer
canonical counted loops and ordinary scalar operations. Add targeted loop/check
optimizations only when measured, rather than implementing a second general optimizer.

## 9. Other language features and representations

### Values, unions and errors

Use LLVM integers/floats at the documented widths, `i1` for computed booleans with an
explicit storage ABI, inline structs for objects/tuples, compact unsigned enums and bitsets
for `Fields`. Avoid packing ordinary fields so tightly that alignment or disjoint writes
become expensive. One-field wrappers need no extra payload storage, while semantic type
distinctions still survive in the frontend.

Represent general unions as a tag plus properly aligned payload storage. Named members
retain their nominal identity; anonymous unions normalize structurally. Start with an
explicit tag, then add proven niche encodings, such as a nullable non-null reference for
an optional. Union widening can require tag remapping, and narrowing can require a view
of the active payload; it is not always a bitcast.

Recursive-by-union values need compiler-owned indirection at recursive edges. Nonrecursive
fields stay inline. Clone/drop follows unique ownership rather than tracing or reference
counting. This is distinct from collections of objects connected by relations.

Lower `throws` internally to a result discriminant and success/error payload, with output
slots for larger values where appropriate. This is a backend ABI representation, not a
change to the source return type. `try` branches to cleanup and propagation; `catch`
branches to its handler. No C++ exceptions, stack unwinder or heap allocation is needed
just to carry an ordinary small typed error. Functions with no errors have no error ABI.

### Requirements and globals

Lower `requires` to hidden borrowed arguments in a deterministic order, passed explicitly
through the call chain. `provide` resolves lexically during elaboration. No runtime
dependency-injection map, thread-local lookup or dynamic type search is necessary.
Service function fields can remain indirect unless their implementation is known.

Place mutable module values at stable global addresses, with generated initialization
and destruction routines. Replacing one drops its old owned contents and invalidates
dependent local references; the global place itself remains stable. Specify initialization
dependencies and reject unsafe cycles before relying on globals in native programs.

### Strings and big integers

Use UTF-8 byte storage owned with vector-like capacity; slices borrow byte ranges whose
endpoints have been established as grapheme boundaries. Literal bytes can be static.
`String.length` and indexing count grapheme clusters, so they are not byte-length loads
or `i32` character accesses. A `Character` may contain arbitrarily many code points and
cannot generally fit in one scalar integer.

Use a precompiled Unicode segmentation implementation. Fuse sequential iteration with
one segmentation scan, and add an ASCII fast path. A grapheme-count cache is optional
and must be invalidated by mutation. Byte storage, Unicode version and string ownership
belong to the runtime ABI. An owning `Character` can use a small-string representation
with overflow storage; its allocation policy is one of the pending specification details.

Implement `BigInt` as uniquely owned limb storage, preferably using a proven arithmetic
implementation behind a narrow ABI. Copy/move/drop semantics remain Fa semantics. Neither
arbitrary-precision numbers nor Unicode text become allocation-free merely because their
surface types look primitive.

### Closures, async and reactivity

Known nonescaping closures can become direct calls with an inline/stack environment.
Escaping closures must own their copied captures; use a uniform invoke/environment/clone/
drop contract. A practical erased closure can use a uniquely owned environment allocation,
but this is a proposed extra compiler allocation case, not an already specified rule.
Never store a borrowed stack environment in a returned function value.

For colorless async, generate synchronous and resumable forms only when demanded. Key
both forms separately in caches. Represent the resumable form as an explicit state
machine with storage for live locals and cleanup state; use a small poll/wake/drop ABI.
This avoids forcing every ordinary call through an executor or coroutine allocation.

The docs select Tokio and Rayon for native concurrency. Reuse them through precompiled
runtime shims, not by translating every Fa program into Rust and rebuilding it with
Cargo. Same-thread `start` needs a local executor policy. Tokio's
[`LocalSet`](https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html) provides same-thread
execution, but its spawned futures still require owned/static storage; it does not solve
Fa's borrowed task lifetime problem automatically.

Generate cancellation cleanup and keep borrowed arguments/requirements alive until every
task that uses them is finished or fully cancelled. A blocking call from within an executor
must not recursively invoke an unsupported `block_on` or deadlock its own I/O; define a
runtime-safe blocking path and diagnose unsupported call contexts where necessary.
Workers copy messages and reject references/relations as documented. Parallel loops use
chunked jobs and proven disjoint writes; fall back to sequential execution for small
workloads only where the target's documented semantics permit it.

Reactive values need observer/dependency storage, batching and scope cleanup. Compile
known dependency edges and derived computations directly where possible; dynamic reads
may still require tracking. Ordinary values should not carry reactive metadata. Do not
claim all runtime bookkeeping disappears when the program requests reactive behavior.

## 10. LLVM integration and optimization profiles

Use one pinned LLVM release and matching bindings/tools in the Fa toolchain. A small Rust
wrapper over the C API is a reasonable starting point; keep raw LLVM handles private to
the backend. Use a narrow C++ shim only for required capabilities unavailable through the
chosen release's API, such as any missing ThinLTO/debug integration. Do not spread LLVM
version conditionals throughout the semantic compiler.

Emit IR in memory. Textual `.ll` is a debug/export option, not an intermediate file that
must be printed and parsed on every build. Set target triple/data layout and conservative
linkage/attributes; use ordinary GEPs, loads, stores and calls. Use entry-block allocas for
addressable locals and simple SSA values for expression temporaries; let standard
promotion and scalar replacement remove unnecessary storage.

LLVM's [frontend performance guidance](https://llvm.org/docs/Frontend/PerformanceTips.html)
supports conventional IR shapes, explicit target information and avoiding unnecessary
optimizer complexity. Do not add `inbounds` GEPs before establishing the required pointer
arithmetic conditions. Form invalid speculative addresses only if the LLVM rules permit
them; guards must dominate unsafe memory operations.

Use stock new-pass-manager pipelines first. The
[`LLVMRunPasses` API](https://llvm.org/doxygen/llvm-c_2Transforms_2PassBuilder_8h_source.html)
accepts standard pipeline descriptions. Select a pipeline and matching machine-code
optimization level for each profile; debug behavior is not determined solely by the IR
optimization setting.

| Proposed profile | LLVM/link strategy | Intended use |
| --- | --- | --- |
| Development | Minimal IR optimization, fast codegen, debug info, cached objects, no LTO | Small edit-to-run time; aligns with documented unoptimized development |
| Release | Standard O2 pipeline, optimized codegen, cached objects | Good execution speed without mandatory whole-program work |
| Maximum performance | Benchmark O3 versus O2, ThinLTO cache, optional PGO | Final deployment/hot computation |
| Size | Os/Oz-style pipeline and dead stripping; target-specific link settings | Small native/Wasm artifacts |

These are proposed internal profiles; the currently documented configuration has
`build.optimize`, not all of these named switches. Add user-facing options separately.
O3 is not universally faster than O2. PGO and target-specific CPU tuning need representative
measurements and an explicit deployment compatibility choice. Do not enable unsafe
floating-point reassociation/fast-math globally; it changes observable numeric behavior.

Use bounded parallel workers with independent LLVM contexts/modules and a global job
budget covering frontend work, code generation and ThinLTO. Initialize the target backend
once per process. Avoid multiplying frontend worker count by all available ThinLTO threads.

Initially invoke a matching compiler/link driver once for the final link so it supplies
the correct startup objects, system libraries and platform conventions; use LLD where
appropriate. Do not assume `ld.lld` alone knows the entire platform runtime contract.
WebAssembly has its own imports, exports, memory and runtime subset; the
[Wasm linker documentation](https://lld.llvm.org/WebAssembly.html) describes those separate
constraints. Native objects cannot be reused as Wasm objects.

For foreign interfaces, use explicit C-compatible wrappers and ownership/validation
boundaries. Never expose Rust-default layouts or Fa's evolving structural view ABI as an
accidental stable public ABI. FFI shims must not manufacture Rust `&mut` references to
possibly aliased Fa storage or let Rust panics unwind through an unsupported boundary.

## 11. Optimizations to prioritize, and their limits

Prioritize avoiding entire costs: cache hits, no intermediate collections, no unnecessary
deep copies, no per-object allocation, and no validation on proven-safe references. These
matter more than tuning a long LLVM pass list.

For chained collection methods, recognize library operations in Fa IR, inline small
callbacks and build one canonical loop. A terminal reduction can avoid intermediate
allocation; a filtered result can reserve an upper bound and fill once. Do not automatically
fuse unrelated separately materialized statements, as the documentation makes their
intermediate collections real values.

Fusion requires a semantic decision about observable callback ordering. A conventional
eager `filter` followed by `map` runs all predicates before any mapping; a fused loop
interleaves them. I/O, mutation and the first thrown error can differ. Until the rule is
specified, fuse only callbacks whose purity/totality makes the transformation valid.
Do not assume the surface chaining syntax by itself proves equivalence.

For columnar/numeric loops, provide contiguous accesses, small bodies and proven alias
information so LLVM can vectorize. Checked reductions and strict floating arithmetic may
limit reassociation; respect those semantics. Parallelization also needs disjoint writes
and the documented associative reduction contract.

Selectively specialize small generic/structural hot paths, then inspect optimization
remarks. Do not force inline recursive helpers, allocators or every collection operation.
Reserve whole-program work for release profiles and keep runtime slow paths out of hot
instruction caches.

## 12. Specification decisions before relying on unchecked native code

The following are proposals or unresolved conflicts, not silent amendments to the docs.

| Issue in current documentation | Proposed resolution / question to settle |
| --- | --- |
| Objects described as stack-only, but collection elements are inline in heap storage | Use the more precise rule from the objects page: objects are inline values, living in their enclosing storage. Permit LLVM registers and destination construction. |
| Heap allocation exceptions omit escaping closure environments, async frames, Unicode characters and `BigInt` | Permit compiler-owned unique backing storage for these representations; specify clone/drop behavior. Keep user objects free from independent implicit shared identity. |
| Deep copying an owner containing relation-linked collections is unspecified | Clone owned elements in two passes, build old-to-new correspondence, then reconnect internal edges and owner fields. Rebuild reverse metadata. Reject unsupported partial graph copies until their semantics are decided. |
| Copying a single linked node, `take`, or copying links into an externally owned collection is unspecified | Decide whether links detach, reconnect to original targets, or make the operation ill-typed. Prefer no unregistered external links; a standalone copy must not retain stale reverse metadata. |
| Moving an owner can move owner fields or descriptors referenced by its elements | Prefer locators relative to stable owned chunks plus a transferable control block where needed. Otherwise patch the affected owner links during move. Do not treat such moves as unconditional byte relocation. |
| A wider writable view can permit values invalid for the actual narrower field type | Require write-compatible/invariant field representations and constraints for mutated paths; specify safe readonly covariance separately. |
| Set elements and map keys appear covered by generic mutable iteration rules | Make hash/equality-defining storage immutable while resident, or specify a checked remove/update/reinsert operation. Arbitrary in-place key mutation breaks lookup and uniqueness. |
| Custom set comparison has no hash/key API | Use linear comparison initially; later design an explicit compatible hash/key contract. |
| `start` can retain borrowed arguments and requirements, and `Task` is storable | Prefer scoped tasks: the compiler proves completion/cancellation before borrowed owners die and restricts mutations while tasks may use them. Specify task copying, escape and cancellation completion before implementation. |
| Mutation of ordinary state may interleave across `await` on one thread | Define suspension as a point that loses unprotected shared refinements; apply the documented thread-safe-type restrictions and effect analysis to tasks, not just OS threads. |
| Linked-list `add` declares `mutates self` but claims references into its bag survive | General rules say mutation of the owner can replace its collections and invalidate their elements. Prefer a precise `mutates self.#nodes.length` plus owner-field effects, or an explicitly specified stronger preservation summary. Do not special-case this example unsoundly. |
| `LinkedList` also redeclares a reference that its example says remained valid | Correct the example when settling the effect rule; a still-valid `use` name cannot be redeclared in the same scope. |
| Relation cleanup changes more places than the surface direct effect names | Make implicit relation writes, owner-field cleanup and invalidations part of semantic summaries, including references into edge bags and loaded targets. |
| Unconditional chain fusion promises identical meaning for effectful callbacks | Specify streaming chain semantics, or constrain fusion to transformations that preserve effects/error ordering. |
| Overlapping anonymous structural union members and mutable refinements | Define representation/narrowing for values satisfying more than one member; preserve first-match `when` behavior and kill refinements on aliasing writes. A simple tag is not proof that only one structural constraint matches. |
| Equality on cyclic relation graphs, closures and synchronization/task values is not defined | Specify which types are comparable and whether relation equality follows graph content. Do not implement recursive graph equality as naive pointer recursion or silently expose identity. |
| `Any` is listed as a special type, while introductory docs reject an unsafe `any` escape | Treat it as a safe top constraint. Ordinary erased values require owned storage/type operations only where values are actually stored dynamically; generic constraints alone need no runtime boxing. Settle supported operations and representation before allowing them. |
| Floating NaN equality and floor-division/remainder edge cases are incomplete | Specify hashing/equality and numeric failures explicitly; never delegate these choices to LLVM undefined behavior. |
| Module initialization cycles and program-wide teardown are unspecified | Define an initialization dependency order and reject unsafe cycles. Choose destruction behavior for globals and external relation owners. |

These decisions can be staged. Native arithmetic and arrays do not need all graph-copy or
task semantics immediately, but the compiler must reject unsupported constructs with a
clear diagnostic until their safe lowering exists. Falling back to `Any` or unchecked
pointers is not an acceptable temporary implementation.

## 13. Implementation order and acceptance criteria

1. **Semantic foundation.** Intern semantic types, resolve filesystem namespaces and
   signatures, check structural compatibility/effects, and build typed control flow.
   Implement ownership, provenance, invalidation and refinement analysis for a small
   synchronous subset. Preserve shared semantics with the compile-time interpreter.
2. **First native program.** Add the pinned LLVM wrapper, primitive arithmetic/control
   flow, static strings, typed errors, object layouts and simple link/runtime startup.
   Reject unsupported constructs explicitly. Verify emitted IR and execute programs.
3. **Value ownership and arrays.** Add copy/move/drop, destination construction, array/string
   allocation and checked indexing. Validate alias invalidation and cleanup on all exits.
4. **Structural views and generics.** Add nested field views, writable compatibility,
   function-value adapters, cached generic instances and columnar-ready place access.
5. **Incremental build.** Add interface/body/object manifests and cross-project library
   reuse. Make a no-op native build execute no LLVM work and perform no link when its
   valid output already exists. Confirm private body changes leave unrelated callers cached.
6. **Stable collections and relations.** Implement chunks/free lists, stable map values,
   columnar field storage and graph maintenance. Settle copy/take semantics before allowing
   arbitrary copies of relation owners. Test slot reuse and all relation cardinalities.
7. **Release quality.** Add measured fusion, optional layout specialization, O2/O3
   comparisons, ThinLTO with proper summaries/cache, debug information and optimization
   remarks. Add Wasm using a separate target/runtime configuration.
8. **Advanced runtime.** Add safe escaping closures, scoped async/cancellation, Tokio/Rayon
   bridges, workers and reactive scheduling after their lifetime rules are fixed. Add PGO
   after there are representative applications to profile.

Do not postpone the architectural cache boundary until the compiler is large: stable
semantic IDs, dependency recording and serializable interfaces should be designed during
the first step even if persistent caching arrives later.

Test the compiler's guarantees, not just LLVM text snapshots. Required cases include
structural views with different field offsets, nested writable constraints, columnar
mutation without a row copy, last-use copies with live aliases, partial initialization,
cleanup on errors, stale aliases through callbacks/globals, and integer minimum/maximum
edges. Relation tests must cover cycles, self-links, replacement, deletion, duplicates,
owner moves/copies and immediate slot reuse. Run runtime memory tests with sanitizers
and differential tests against the interpreter; the LLVM verifier alone proves none of
these language safety properties.

Measure separately:

- Cold build with and without cached dependencies, warm no-op build, private body edit,
  public effect/layout edit, generic-definition edit, and compile-time file/directory edit.
- Frontend time, LLVM time, link time, peak memory, cache hit rate, object/code size and
  generated specialization count.
- Array scans/appends/copies, fragmented bag iteration, relation reads/updates/deletion,
  map rehash with retained references, columnar single-field loops, Unicode iteration,
  closure invocation and eventual async scheduling.
- Equivalent C++/Rust/Zig workloads with matching allocation, overflow, Unicode and
  optimization semantics, reporting both throughput and allocation/memory costs.

Use measurements to select defaults; no universal compile-time or throughput guarantee
is credible before the compiler exists. The core design should make ordinary safe Fa
code lower to ordinary native data and instructions, while concentrating unavoidable
work at ownership changes, collection updates and explicit dynamic boundaries.
