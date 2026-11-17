## Decision (lead, 2026-11-16)

# Pool links and inline arrays: the lead's decision (2026-11-16)

The design study is `/home/claude-dev/fa-lead/reports/design-pool-links.md` (read all of it: "Option A", "A1" to "A6", "Complete scene and tree designs", "The other six ports", "Implementation plan for A"). **Option A is adopted**, with every proposed answer of its section "Decisions for the lead", and these precisions:

1. **`link`** is a field modifier, meaningful only before a field name inside a type body (`link next: (Node in nodes)? = none`, also in union member records). It is not a reserved word elsewhere: `link` stays usable as an identifier, and a field may be named `link`. A stored reference without `link` keeps today's meaning (a relation, with its reverse side); the absence of a reverse side never turns a relation into a link.
2. A link designates a **whole element** of a `GrowingPool` or `GrowingArray` (collections that never remove or move an element), named by the existing `T in owner.path` types, inside a **complete declared owner** (the record that owns the target collections and every holder of links into them). The complete owner moves, returns and is copied (`copy owner`) with its internal links preserved; a link-bearing element is never copied or moved out on its own (borrow it, or project its data); a target pool is not reset or extracted while a holder outside it may survive. Deletable pools keep relations. Option B (proven deletion) and owning edges are deferred.
3. **`InlineArray<Item, capacity>`**: a general bounded list stored in its owner (any item type), ordered, appendable up to its capacity, never spilling to the heap; `push` on a possibly full list throws `CapacityError` (a compile-time error when certainly full, no check when the room is proven); the usual collection effects, with item places that never move on `push`. `CArray` stays the fixed-count foreign array.
4. **Index width**: a parameter of the target collection, `GrowingPool<Node, Index = UInt32>` (also `GrowingArray`), `UInt8`/`UInt16`/`UInt32`/`UInt64`; a link stores the element's ordinal plus one (zero is `none`: an optional link costs no extra bit). Reaching the limit stops the program with a clear message before the element is published (a resource limit like allocation exhaustion, not a new `throws` on every insertion); a statically impossible size is a compile-time error. No silent widening. The native encoding behind it (ordinals resolved through the collection's pages) is an implementation choice that measurements may change; it is not visible in the source.
5. `GrowingPool` iterates in insertion order (now guaranteed by the specification).
6. Printing, equality and hashing skip link fields as they skip relations; converting a link-bearing owner to JSON is a compile-time error whose fix is an explicit data type with identifiers; workers receive only a complete owner (copied or exclusively transferred).
7. Layout scope of the first version: stable dense pages, fixed-stride slots (a union element uses its largest member). Packed variable-size union nodes come later, as a measured decision.
8. `lru` is not concerned: it keeps `Map` and its arena of indices.

Landing: the work lands on the integration branch `feature/links` in slices. Now, independent of the ownership enforcement in progress on `feature/ownership`: the specification, the rejection of reference types stored outside a relation or a link (the hole found by the benchmark audit), reference parameters and results of user functions through owner paths, and `InlineArray` on every target. After the ownership enforcement lands: the `link` contract itself (containment, copies, effects), the native compact pools, the complete-owner copies, the boundaries (workers, constants, reflection), then the seven benchmark ports.

# Pool links and bounded inline lists — design study

2026-11-16. Branch `design/pool-links`, base/HEAD `4785c025`. Proposal for the lead; no language decision is adopted here. This report is the only deliverable: compiler, specification and benchmarks remain unchanged.

## Findings established from the specification

- `GrowingPool` already guarantees stable slots and prohibits individual deletion. It needs no new append-only collection or mode. Whole-value replacement (`pool = []`) is nevertheless allowed, invalidates local references, and retains storage; stored incoming links need a stronger rule at this boundary.
- A reference field is presently a bidirectional relation even in a growing pool. Its reverse side provides targeted cleanup on deletion, with implied write effects; ordered reference arrays are prohibited. Owner fields are the sole existing one-way exception and have special mutation restrictions.
- Relation links are non-owning metadata: whole-element assignment preserves them; detached copies erase them; copying the enclosing owner remaps links among jointly copied collections. Copying a collection field alone erases its relations. These rules cannot silently be reused for a bounded list of links without specifying its copy behavior.
- The audit reports `REL001` for direct one-way fields, `REF010` for local reference collections, and an erroneous acceptance of `Array<SceneNode in nodes>` that stores owned copies. Acceptance is not a language feature.
- Native pools are chunked, not one globally contiguous allocation. Growing-pool scans can be dense within each chunk; whole-pool destruction avoids visiting slots only when the element type owns no memory.

## Recommendation

Add explicitly declared **stored links into `GrowingPool`**, using the existing `T in owner.path` reference types, and a general `InlineArray<Item, capacity>` with an initialized prefix. Keep ordinary relations for deletable objects. Do not add owning tree edges or heap-reachability proofs to this first design.

Proposed declaration:

```fa
-- PROPOSED: `link` field modifier, pool Index parameter, InlineArray.
-- The enclosing-type collection binding and `T in nodes` already exist.
type Scene = {
    nodes: GrowingPool<Node> = [] -- proposed default Index = UInt32
    type Node = {
        id: Int32
        link children: InlineArray<Node in nodes, 4> = []
    }
}
```

A link is a non-owning reference whose validity follows from a declared collection and an enclosing ownership boundary. It does not maintain an inverse. The spelling `link` is new because a relation promises inverse maintenance, `use` binds a local alias, and an ordinary field owns its value; none currently denotes this storage contract. The modifier makes the distinction explicit even when a growing pool also participates in ordinary relations. Removing a reverse declaration never silently changes a relation into a link.

Two alternatives are developed below. The first is recommended. The second shows exactly what allowing deletion would add; it is not an implementation prerequisite.

## Option A — links contained by an owner of growing pools

### A1. Typing and the safety invariant

A `link` field may contain `T in c`, `(T in c)?`, `Array<T in c>` or `InlineArray<T in c, n>`. Each leaf denotes a **whole element** of the named `GrowingPool`, never a string slice, object subfield, union payload, or arbitrary address. `T` must be the pool's complete element type; narrowing a union element retains that element's identity. An optional defaults only when its declaration says `= none`; a list defaults only when it says `= []`. A required link must be supplied at construction. Repeated targets, self-links, cycles and shared children are permitted; list order and duplicates are preserved. Cyclic construction starts from initialized optional/list fields or a link-free union leaf, then patches links (or replaces that stable union element). An always-required same-pool link cannot seed an empty pool by itself: no uninitialized publication or new constructor-self syntax is introduced.

The existing nested-type form binds the target collection per owner instance. A `Node in a.nodes` cannot be stored in a field expecting `Node in b.nodes`. This binding is a static identity, with no stored owner tag. A target in another pool is allowed when both pools belong to the same enclosing owner. GrowingArray also satisfies the target guarantee: no removal or movement, stable references and an existing indexed API. Permit it as a target as well, with the same Index parameter; it is needed for graph heads indexed by vertex. GrowingSet keys are immutable and GrowingMap values are not pool elements; neither is included in this first extension. GrowingColumnar could satisfy the same lifetime proof, but needs a corresponding columnar ordinal decoder; it is outside the two initial target kinds, not fundamentally unsound. Source elements need not themselves have stable addresses: a source record can live in an owning Array, since links are measured from the target pool, not from the source field.

For every stored edge `h → e`, the checker must establish:

1. `e` is an initialized whole element of the exact target pool instance.
2. The target pool cannot remove or relocate `e` individually.
3. Every operation ending that pool incarnation also ends **all surviving holders** of links into it. No link can escape as a detached owning value.
4. A move invalidates old local aliases; a copy produces a separate closed owner and rebinds its internal collection paths.

Rules 2 and 3 are different. `GrowingPool` supplies rule 2; its existing reset/assignment API does not supply rule 3.

For a copy/ownership transfer, the boundary must include outgoing targets as well as incoming holders: moving a node pool away from a label pool it references would otherwise change or lose the named binding. Only a complete declared owner rebinds those paths. Reset is different: it destroys outgoing holders rather than transferring them, and therefore needs the incoming-holder check.

The enclosing owner is the existing ordinary record, not a new resource, runtime region token or allocator object. All target pools and stored holders must be owned below that record. Containment stops at a shared handle, task frame or escaping environment: a holder behind a Mutex cannot borrow a pool outside that payload. Move/copy a complete closed graph into such a boundary, or use the existing checked local borrows; never count a shared payload as an exclusively owned child. Direct self-links in a module-level GrowingPool may use that pool as the boundary. For cross-pool links, place the pools in one record, which may itself be module-level. Independent global variables and arbitrary borrowed external owners are not a second lifetime system in this option.

```fa
-- PROPOSED fields; existing nested types and owner paths.
type Model = {
    nodes: GrowingPool<Node> = []
    labels: GrowingPool<Label> = []
    link root: (Node in nodes)? = none
    type Label = { text: String }
    type Node = {
        value: Int32
        link next: (Node in nodes)? = none
        link label: (Label in labels)? = none
    }
}
mutable first: Model = {}
mutable second: Model = {}
use label = first.labels.add({ text = "one" })
use node = first.nodes.add({ value = 1, label = label })
first.root = node
-- second.root = node  -- reject: first.nodes and second.nodes differ
```

`nodes` and `labels` are recognized as collection fields wherever their enclosing type is instantiated. A per-call local `mutable model: Model = {}` therefore needs no runtime pool name in a generic list. Do not write a runtime pool as a lowercase compile-time value parameter: that would contradict Fa's generics.

**Reset, replacement and partial moves.** A pool whose only persistent incoming links are inside its own elements may be reset: those holders die with the pool. If `model.root` or another pool in `model` may hold incoming links, replacing or extracting `model.nodes` alone is rejected. Reset or move the smallest complete owner instead (`model = {}` here). This is a type/path containment rule, not a search for currently non-null fields. Setting `model.root = none` does not discharge another potential holder hidden in `model`; the first slice does not infer heap contents. These containment dependencies concern the new `link` fields. Existing relation/owner fields retain their automatic cleanup and original full-owner-path restrictions; they are not untracked incoming holders. A broad `mutates model.nodes` call is subject to the same rule even if today's body merely appends. `mutates model.nodes.size` and `.capacity` remain permitted.

Mutating node data or replacing a whole node preserves the *element place*, so incoming links remain valid and see the new value. Unlike a bidirectional relation, outgoing `link` fields are explicitly initialized/replaced with that node's written value; there is no inverse to preserve. Changing a union tag invalidates payload borrows under the existing rules. Union pool nodes use stable max-payload slots (A5); a link never targets a movable payload.

**Escape and destruction.** Moving/copying a holder to another destination **inside the same owner binding** is allowed, subject to its ordinary data/unique category; its links still name the same targets. This includes shifting a table Slot and replacing an existing node from an explicit internal copy. A link-bearing element/holder cannot be copied or moved out as a standalone owner, returned as ordinary data, thrown, or captured independently by an escaping closure. Borrow it with `use`, return `T in parameter.path`, or project its ordinary fields explicitly. The complete enclosing owner can move/return normally, and `copy owner` can copy it when its owned data is copyable. A borrowed enclosing parameter still cannot move. This is an additional closure condition on ownership, not permission to duplicate structures implicitly.

During destruction, links are inert and are not followed. A destructor of an element or its owned data must not access a pool being destroyed in that same boundary, directly or through calls/module values. Check that prohibition from destructor read/call summaries before admitting such a resource into the boundary. Otherwise a destructor could reach an already destroyed sibling. Unique items make the owner unique; they do not make links own their targets. Task-containing boundaries retain the executor restrictions. Nodes containing only flat data and the new links need no element destruction walk; Strings, external arrays, resources, handles and allocated ordinary-relation bookkeeping retain their cleanup costs.

### A2. Local references, functions, effects and generic code

Reading a scalar link with `use` binds its target, as for a relation. Writing the field later does not reseat that local alias. Iterating a link list yields target references, never owning element copies or aliases of the encoded offsets. `let x = node.next` requests the designated value under the usual ownership rules; it does not create an untracked handle. A link-bearing node cannot be detached by this route. Bare local `Array<T in pool>` remains forbidden: persistent root lists are explicit `link` fields in the enclosing owner.

```fa
-- PROPOSED: `link` field and Index parameter.
type Chain<Item, Index = UInt32> = {
    nodes: GrowingPool<Node, Index> = []
    type Node = {
        value: Item
        link next: (Node in nodes)? = none
    }
    function add = (self, value: Item): Node in self.nodes
        mutates self.nodes.size => {
        self.nodes.add({ value = copy value, next = none })
    }
    function head = (self): (Node in self.nodes)? =>
        self.nodes.find((node) => true)
}
```

The `add` declaration carries the existing generic semantic copy condition from `copy value`: a unique Item cannot instantiate this copying helper. Construct a unique payload directly in its final slot instead. `Index` is restricted to the unsigned storage types listed in A5. Generic functions are checked against the symbolic owner paths and conditions once, and specialized for concrete layouts normally. Imports and function values must retain owner substitution, effects and those conditions.

A function following links needs a named owner path in its signature (or a statically named module owner), so its pool header/base is available through that owner argument. A bare borrowed `Node` structural view can read ordinary fields, but cannot invent the missing collection binding to follow its links. There is no extra owner pointer in each link; nonescaping callbacks can borrow the enclosing owner. A function taking a link uses the existing dependent parameter form `(model: Model, node: Model.Node in model.nodes)`; a result is `Model.Node in model.nodes`. No new function-result handle type is needed. A lookup return has unknown identity; an add return is fresh only under the existing all-return-path freshness rule. An inferred ordinary return is still not a reference. Private pool paths can remain in public signatures as already specified.

Following a link restarts the effect path at its target pool, just as following a relation does. Thus recursive scene updates declare `mutates self.nodes[..].world`, not just `mutates node.world`. Writes to `.children` declare that field alone; adding nodes declares `.nodes.size`; neither implies a write to another node or a reverse side. Pool growth and link-field writes invalidate no target alias. Replacing an InlineArray or removing an item invalidates the appropriate *slot* aliases but does not invalidate previously bound target aliases.

A recursive traversal cannot pass `node.world` as an independent borrowed argument to a call allowed to mutate every `.world`: a child might be the parent, and the arguments overlap. Take an ordinary scalar-only Transform snapshot first. This accepts arbitrary graphs without weakening Fa's alias rule or assuming that a field named children forms a tree. Cyclic traversal termination is the program's responsibility.

### A3. Copy and move contract

| Operation | Proposed result |
| --- | --- |
| Move the complete owner | Transfer its pools/headers; no node-link rewrites; invalidate old owner aliases. |
| `copy owner`, all links internal | Copy owned payloads once; each edge designates the copied target. Preserve cycles and sharing, without recursively following links. |
| Copy a pool's enclosing wrapper containing only that pool | Same whole-pool copy, including self-links. There is no requirement to rewrite stored indices. |
| `copy model.nodes` as an ordinary detached collection | Reject if its element template needs the enclosing binding or other pools; copy the declared owner instead. No silently stripped edges. |
| Copy/move one linked element or Array of linked elements out | Reject; borrow it or construct a data-only projection. |
| Copy an owner containing a unique value | `MEM003`, as today. |
| `take` or delete from a linked GrowingPool | Method absent, as today. |

An owner containing both kinds also copies/remaps its ordinary relation sides by the existing rules; only the new indexed links avoid fixups.

This deliberately differs from **relations**, whose detached copies erase sides and whose whole-element assignments preserve sides. Explicit `link` keeps those existing rules unambiguous; changing relation-copy semantics is unnecessary. Structural views may read ordinary fields and borrow nodes; they cannot erase the ownership boundary to smuggle link-bearing data into an independent owner. Generic copies acquire the same closure condition.

A local return of a complete owner is safe on success; partial construction is destroyed on error. Returning a reference into that local owner is not safe. An exclusive move across workers transfers the complete boundary and invalidates its aliases. An independent worker copy copies the boundary. This is a deliberate extension of today's concurrency rule, which forbids references/relations in messages; it does not authorize sending an individual link. Mutex/Channel payloads can only receive a complete closed boundary under their existing protection/transfer rules, never a Guard borrow. Task remains non-transferable.

Reflection must retain link storage kind and symbolic owner paths, along with InlineArray capacity and collection Index; it must not turn a link into an owned recursive field. Schema.Reference remains a recursive *type* name. TypeFromSchema rejects unresolved/escaping owner paths, and generated codecs still use analyzed types rather than interpreting schemas at runtime.

Generic printing can omit link fields as it already omits relations; equality/hash should likewise ignore them rather than walk cycles. Document that data equality is not graph-isomorphism equality. For JSON, recommend an explicit compile-time rejection of graph-bearing owner types instead of silently losing edges or recursing forever. Use a data-only DTO with integer node IDs and adjacency lists; validate IDs while rebuilding links. Generated binary worker codecs can transport complete closed graphs, preserving IDs/sharing and checking imported data at the boundary, with no checks when later following links. Compile-time evaluation uses the same owner/edge rules and can publish a complete immutable graph, never an interpreter address.

### A4. General bounded inline lists

`InlineArray<Item, capacity: Integer<min = 0>>` is proposed. `[]` is empty, `[a, b]` requires capacity at least two, `.length` is in `0..=capacity`, and `.capacity` is the compile-time bound. The list is ordered, allows duplicates, and never spills to a heap buffer. It is a value usable in any field or local for **any item type**, not a special scene collection. `InlineArray<Integer, 4>`, `InlineArray<String, 20>` and a list of unique resources all use the same initialization and ownership rules. A list of references additionally needs the `link` field contract above.

API: indexing/get/iteration and assignment; `push(value)`, `pop`, `delete(index)`, `take(index)`, `clear`. No reserve or capacity-changing operation. `push` returns nothing, so a failed push cannot leak a partially published slot. Unknown-fullness push throws proposed `CapacityError.InlineFull`; no mutation occurs on that error. A statically full push is a compile error. A proven remaining slot removes that error and its check, like a proven valid index removes IndexError. No unchecked append API or implicit spill. A loop bounded by four does not automatically prove the evolving child-list length: retain `try` unless the specified fact analysis actually proves it.

`push` has `mutates self.length` and initializes a fresh last slot; it never moves old slots. `pop` invalidates the previous last element, `delete`/`take` invalidate the shifted suffix, and `clear` mutates the whole list. Replacing a list invalidates every old item/payload borrow. Writing an item preserves its place and invalidates only its owned interiors. Iteration prohibits changing length or order, while element writes are allowed. These are the existing collection effect rules applied to an inline representation. Link-list slots themselves are not exposed as reference places: `use child = node.children[i]` binds the target.

Native layout is `[capacity × stride(Item)]` plus the smallest unsigned length capable of representing **capacity** (capacity 255 fits UInt8; 256 needs UInt16), padded/aligned normally. Only `[0, length)` is initialized and may be read/copied/destroyed; no zero-fill or dummy initialization of unused resources. Capacity zero is valid and always empty. A failure constructing an item destroys that partial item and leaves length unchanged; normal/error exits drop exactly the initialized prefix. An inline list inherits its items' ownership category, as CArray does: a scalar-only bounded list needs no implicit heap-structure category; lists owning structures need explicit copy and lists owning unique data cannot be copied. Logical item-lifetime invalidation still applies despite physical inline storage.

`CArray<T, n>` already supplies **exactly n initialized inline elements**, indexing and iteration, including a 3×3 matrix. It has no evolving length or append operation. Adding a manual count to a CArray of integer links would expose a hand-written container, not provide this collection abstraction. On JavaScript InlineArray is one ordinary dense array; its existing `.length` is the initialized prefix, the capacity is a static constant at checked pushes, and no wrapper/count object is allocated. The engine may allocate its backing storage: native inline storage is not a JavaScript object-layout promise.

### A5. Widths and native addressing

Proposed parameter: `GrowingPool<Node, Index = UInt32>` (also GrowingArray), where Index is UInt8, UInt16, UInt32 or UInt64. Choose it **on the target collection**, never independently per field. Every stored incoming link to that collection uses its width; local aliases remain machine pointers. A type's layout is therefore stable across optimization and split compilation. Index is part of collection type identity and is invariant for mutation/storage; abstract it with a generic Index parameter rather than implicitly widening a borrowed collection. `reserve(n)` promises room, not a maximum, so it never selects a smaller width.

Recommend an **element ordinal biased by one**: slot i is encoded as i+1, zero is none. Required and optional links occupy the same bits. An UInt16 collection has at most 65,535 elements; UInt8 has 255, UInt32 has 4,294,967,295. This is an element bound, independent of node size, alignment and target pointer width. UInt64 is the explicit escape for larger counts, additionally bounded by the target's usable integer/address space. Ranged integers used as payloads still follow their existing storage rules; a link index is a typed reference encoding, never an integer that arbitrary source code can forge.

An element ordinal is preferable here to a signed displacement from each link field: source holders and InlineArray items may move, and table slots may shift, without re-encoding links. A copy of a complete pool keeps the same ordinals, including cycles and sharing. Its own storage bases change; its link bytes do not. This also admits cross-pool/root-array links that a field-relative same-pool encoding cannot represent. The cost is address translation through the target collection, rather than Goose's single field-relative addition.

At the limit, `add`/`push` **stops before publishing another element**, with “pool reference limit reached (Index is UInt16); use a wider Index or reject this input before adding”. `reserve` and `from(source)` enforce the same representability limit. A statically impossible append/literal is a compile error; a proven count bound removes the runtime guard. This guard remains active with arithmetic overflow checks disabled. There is no auto-widening or near/far tag. Index exhaustion is a representation/resource limit, like allocation exhaustion, not a new `throws` effect on every existing insertion. Default UInt32 still introduces a finite bound and needs lead approval. JavaScript/interpreter enforce the same count limit and need no virtual native byte budget.

An application wanting a recoverable “too many nodes” error can test the declared index limit before adding and throw its own ParseError; that guard can also discharge the builtin limit check. In contrast, InlineArray capacity is an explicit data limit and has the typed CapacityError of A4. Making every recursive pool builder propagate a new capacity-error ABI would be a substantial source/performance cost without helping link-follow safety.

A native GrowingPool keeps stable dense pages and a page directory. Give each page a power-of-two slot count and a first ordinal. For nonzero x, i = x−1, resolve as `pages[i >> slotShift] + (i & slotMask) * stride(Node)`. This is one directory load and arithmetic, without a bounds, generation or liveness check. A single-page pool can use its base directly. A GrowingArray uses its existing segment decoder and stable segments for the same ordinal. Returned add/push references can retain ordinals in SSA; arbitrary pointer-to-index stores can recover page/segment metadata. No stored owner word is needed because the reference type fixes its target path.

Pages may require per-page padding or external metadata to obtain power-of-two slot counts; do not assume today's aligned slab layout already supplies that decoder. Directory reallocation moves no element. Payloads are contiguous **within each page**, not necessarily one virtual range. Flat scans use a page/segment outer loop and a plain stride inner loop that LLVM can vectorize. No per-item boundary or occupancy branch on GrowingPool. For payloads containing only ordinary data and the new indexed links, whole-pool duplication is one memcpy per flat payload run plus a new directory, with the same ordinal ranges; no indexed-link fixups. Existing pointer-based relations or owning payloads still require their specialized copy work. Moving a complete owner can transfer headers. Worker codecs recreate the ordinal mapping at the receiver, independently of its physical chunk sizes.

Use stable fixed-stride slots for all element types initially, with an inline discriminant and maximum payload for a union: no per-node box. This preserves element identity under whole-node/tag replacement. Copy/drop of a slot operates on its active member. Packed variable-sized union nodes, inline symbol bytes and varints are **not** solved by compact links. Reusing the existing region payload layout would need a separate immutable-size proof and a node-ID-to-location mapping (or a different byte-offset ABI); do not silently select it and promise the same decoder. These are explicit remaining layout costs for interp, sexp and calc.

Expected fixed-record payloads with 32-bit links: tree 12 bytes, binary-trees 8 bytes, graph Edge 8 bytes, and scene **120 bytes** with ordinary 4-byte alignment (4-byte id, two 48-byte transforms, a 20-byte InlineArray of four links plus count/padding). The source benchmark's 117-byte packed scene record is not automatically Fa's aligned size. No speedup ratio is established by this study.

### A6. Diagnostics

New diagnostic IDs are for the implementation to allocate; the required messages/fixes are:

| Invalid operation | Diagnostic and actionable fix |
| --- | --- |
| `link` targets an ordinary Pool | “nodes can remove their elements; use GrowingPool/GrowingArray, or declare a relation with its reverse side.” |
| Store a reference from another owner | “Expected Node in second.nodes; got Node in first.nodes.” Name both collection paths. |
| Copy a linked element outside its boundary | “This value contains links into model.nodes; borrow it with use, project its data, or copy model.” |
| Reset/extract a pool with surviving holder fields | “model.root / model.otherNodes[..].next may outlive model.nodes; replace/move the complete model.” |
| Recursive effect names only the starting node | “This call may write self.nodes[..].world; declare that path.” No need to broaden to mutates self. |
| Push known full inline list | “children has 4 items, its capacity is 4; remove an item or choose a larger capacity.” Dynamic overflow is CapacityError.InlineFull. |

## Option B — the same links, with statically justified deletion

B uses the same explicit `link` declarations, owner containment, InlineArray, link widths, ordinary reference results, copy restrictions and JavaScript object references. B additionally specifies Pool iteration in slot-ordinal order: insertion order before any deletion, slot order after reuse; current Pool leaves this unspecified. Its extension is `Pool<Node, Index = UInt32>` as a target of one-way links. `invalidates element` is allowed only when **every potential persistent incoming link is proven not to designate that element**. There is no inverse bookkeeping and no dynamic validation. It also retains A's GrowingArray target for indexed graph heads.

This requires a second abstract state beside local-reference validity. For every link field/storage family below a complete owner, track Empty, a finite set of target identities from the existing exact/fresh rules, or Unknown for its target collection. Exact single-holder writes are strong updates; writes through an unknown holder are weak unions. Appending unions a target into a list's summary; clearing the exact whole list proves Empty. Removing one item does not remove an abstract target unless no other occurrence is proven possible. Default-initialized optional fields/lists start Empty. Joins union target sets; loops widen fresh sets to Unknown. Function parameters' persistent contents start Unknown; a potentially writing call makes the affected family Unknown. No optimistic function-body specialization or whole-heap reachability solver is assumed.

Before deletion, inspect all type-declared holder families, not merely local aliases or the references last read by this function. Delete is permitted only if each family is Empty or all its finite targets are provably distinct from the deleted element. Destruction of a complete owner needs no such incoming-link proof. A target collection whose possible holders cannot be enumerated/contained is rejected as in A. Aliases into the deleted element are then invalidated by the existing checker. Disjoint field effects still preserve all other facts.

```fa
-- PROPOSED B. Unlike A, `nodes` may delete elements.
type Pair = {
    nodes: Pool<Node, UInt32> = []
    type Node = {
        value: Integer
        link next: (Node in nodes)? = none
    }
}
mutable pair: Pair = {}
use a = pair.nodes.add({ value = 1 })
use b = pair.nodes.add({ value = 2 })
a.next = b
-- pair.nodes.delete(b) -- reject: a.next may designate b; name its store
-- With exact fresh holders, this clears the only incoming edge:
a.next = none
pair.nodes.delete(b) -- accepted by B; later use of b is still REF001
```

Deletion through a generic `remove(self, node: Node in self.nodes)` is usually rejected: the parameter's incoming graph is Unknown. A helper promising only `invalidates node` is not a certificate that it has no stored incoming links. To accept an unlinking helper for arbitrary graphs, B would need an additional checked postcondition or a precise universal clear proof exported in interfaces. That is a further language decision, not something silently implied by `mutates`.

Native reads/stores have A's compact-link cost, but Pool also has occupancy bits and a free list. Reused slots are safe only after the above proof and ordinary alias invalidation; no generation is needed. Index width bounds **slot identities**, including holes, not merely the current live size. Copying a B pool preserves occupied ordinals and its holes, or must explicitly rewrite links; it cannot compact silently. Flat scans skip holes. Whole-pool flat destruction still frees chunks without following graph edges. JavaScript uses ordinary object references and Pool's existing Set; proof metadata exists only in the checker. Generic errors, moves, workers, compile-time evaluation and JSON follow A, with the extra incoming-link summaries in interfaces/flow.

B accepts all the construction/traversal code below: use `Pool<Node, UInt32>` instead of `GrowingPool<Node>` in Scene and Tree. Those complete programs never delete, so no incoming-link proof is needed while they build, animate or traverse. The resulting Pool bitmap/free-list and scan overhead is real. B also accepts small exact-holder deletion examples such as Pair. It rejects most dynamic unlink/delete workloads, including lru, once loops or helper calls have made the possible incoming sets Unknown. A field/range write elsewhere does not gratuitously invalidate an unrelated link family.

Example diagnostic: “Cannot delete b: pair.nodes[..].next may still point at it (written at line ...). Clear every possible incoming holder, use a bidirectional relation, or destroy the complete owner.” Never suggest a generation handle as an equivalent fix under this task's requirements.

**Why not recommend B first:** it adds persistent alias-state analysis and API proof obligations, yet unlocks none of the seven grow-only ports that A does not already unlock. It does not justify the important dynamic deletions in lru. If deletion becomes necessary, keep ordinary relations until the lead chooses a specific, testable proof contract.

## Why owning links are not a third complete option

Owning tree edges are already expressible as recursive optional/union values. A newly proposed `owns` edge into a Pool would need to transfer ownership out of the pool (or change the pool to non-owning storage), prevent two parents, reject cycles, and invalidate descendants when an edge is replaced. It accepts trees; it rejects graph sharing and DAGs unless another non-owning link kind is introduced. Removing a subtree requires visiting its descendants to reclaim slots and to run nontrivial destructors; no reverse-free constant-time deletion of an arbitrary subtree follows from ownership. Layout can be coallocated, but that is an allocator optimization, not the missing graph semantics. Diagnostics would be moved-owner/second-parent and borrow-after-subtree-invalidation errors. This duplicates a substantial ownership mechanism while still needing A for graph/lru and shared AST nodes, so it is not retained.

A “reusable slot stays a valid value forever” model is another possible trade-off, illustrated by the lru source's slot_pool. Fa already allows explicit replacement of a GrowingPool element while references keep designating that place. This must not be advertised as `Pool.delete` retaining valid references: Fa specifies that deletion ends the element identity. Replacing slots via a preallocated GrowingArray and a manually managed free list is expressible after A, but is a distinct container/workload policy, not a faithful ordinary Pool port automatically enabled by this proposal.

## Comparison

| Property | Existing relations | A: growing owner | B: proven deletion |
| --- | --- | --- | --- |
| Safety argument | Reverse cleanup and local invalidation | Whole elements persist; holders contained by owner | A plus proof of no persistent incoming link at deletion |
| Individual delete | Yes; clears/relinks sides | Absent on target collection | Only with finite disjoint/Empty incoming summaries |
| Stored reverse state | Required (owner-field exception) | None | None |
| Write work | Direct + old/new reverse updates | One scalar/index store or bounded list append | Same as A; analyzer tracks writes |
| Follow cost | Pointer/read; no liveness test | Decode index; no liveness test | Same as A |
| Flat native scan | Bitmap/hole handling on Pool | Dense page/segment loops | Bitmap/hole handling on Pool |
| Ordered children inline | Not provided | InlineArray | InlineArray |
| General graphs/DAGs/cycles | Yes | Yes, until the owner is replaced | Yes; deletion usually needs relations instead |
| Arbitrary lru deletion | Supported relation semantics, extra reverse work | Not enabled | Still rejected for unknown incoming sets |
| Detached element copy | Clears relation sides | Reject; project data | Same as A |
| Native storage of two optional links | Usually two machine pointers plus reverse state | Two 32-bit indices by default | Same link bits plus Pool slot state |
| JavaScript hot link | Direct object reference | Direct object reference | Direct object reference |
| Checker cost | Existing pairing/effects | Owner paths, containment and copy conditions | A plus persistent points-to state/fixpoints |

## Complete scene and tree designs

These are complete **proposed programs**, not checker-tested new features. New syntax/API: `link`, InlineArray and its CapacityError, and the Index parameter/default. Nested types, `use`, dependent reference results, `mutates`, `try`, CArray, integer operations and object literals are existing syntax. For B, substitute `nodes: Pool<Node, UInt32> = []` for the GrowingPool field in each program; every other line is identical. No separate reverse fields are required in either option.

### Scene: parent first, flat animation, recursive update, flat checksum

```fa
function xorshift = (x: UInt64): UInt64 => {
    mutable v = x
    v = v.bitXor(v.shiftLeft(13))
    v = v.bitXor(v.shiftRight(7))
    v.bitXor(v.shiftLeft(17))
}
function randomBelow = (x: UInt64, bound: Integer<min = 1>): Integer =>
    Integer(x %% UInt64(bound))

type Vector = { x: Float32, y: Float32, z: Float32 }
type Transform = { matrix: CArray<Float32, 9>, translation: Vector }
let identity = Transform {
    matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    translation = { x = 0.0, y = 0.0, z = 0.0 }
}
function rotation = (seed: UInt64): CArray<Float32, 9> => {
    let axis = randomBelow(seed, 3)
    let quarter = randomBelow(seed.shiftRight(8), 4)
    mutable c: Float32 = 1.0
    mutable s: Float32 = 0.0
    if quarter == 1 {
        c = 0.0
        s = 1.0
    } else if quarter == 2 {
        c = -1.0
        s = 0.0
    } else if quarter == 3 {
        c = 0.0
        s = -1.0
    }
    if axis == 0 {
        return [1.0, 0.0, 0.0, 0.0, c, 0.0 - s, 0.0, s, c]
    }
    if axis == 1 {
        return [c, 0.0, s, 0.0, 1.0, 0.0, 0.0 - s, 0.0, c]
    }
    [c, 0.0 - s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]
}
function compose = (parent: Transform, local: Transform): Transform throws IndexError => {
    mutable matrix: CArray<Float32, 9> = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    for i in 0..3 {
        for j in 0..3 {
            mutable sum: Float32 = 0.0
            for k in 0..3 {
                sum += (try parent.matrix[i * 3 + k]) * (try local.matrix[k * 3 + j])
            }
            matrix[i * 3 + j] = sum
        }
    }
    let p = parent.matrix
    let t = local.translation
    Transform {
        matrix = matrix
        translation = {
            x = p[0] * t.x + p[1] * t.y + p[2] * t.z + parent.translation.x
            y = p[3] * t.x + p[4] * t.y + p[5] * t.z + parent.translation.y
            z = p[6] * t.x + p[7] * t.y + p[8] * t.z + parent.translation.z
        }
    }
}

type Scene = {
    nodes: GrowingPool<Node> = []
    type Node = {
        id: Int32
        local: Transform
        world: Transform
        link children: InlineArray<Node in nodes, 4> = []
    }
    function build = (self, depth: Integer<min = 0>, seed: UInt64): Node in self.nodes
        throws CapacityError | ArithmeticError
        mutates self.nodes.size, self.nodes[..].children => {
        let state = xorshift(seed)
        use node = self.nodes.add({
            id = try Int32(self.nodes.size)
            local = {
                matrix = rotation(state)
                translation = {
                    x = Float32(randomBelow(state.shiftRight(16), 9))
                    y = Float32(randomBelow(state.shiftRight(24), 9))
                    z = Float32(randomBelow(state.shiftRight(32), 9))
                }
            }
            world = identity
            children = []
        })
        if depth == 0 { return node }
        let count = 1 + randomBelow(state.shiftRight(40), 4)
        for index in 0..count {
            use child = try self.build(depth - 1, xorshift(state.wrappingAdd(UInt64(index))))
            try node.children.push(child)
        }
        node
    }
    function update = (self, node: Node in self.nodes, parent: Transform)
        throws IndexError mutates self.nodes[..].world => {
        node.world = try compose(parent, node.local)
        let world = node.world -- ordinary inline value snapshot; no heap copy
        for child in node.children {
            try self.update(child, world)
        }
    }
}

mutable scene: Scene = {}
use root = try scene.build(12, 12345)
mutable total = 0
for frame in 0..16 {
    let step = Float32(frame %% 5)
    for node in scene.nodes {
        if node.id %% 7 == frame %% 7 {
            node.local.translation.x = step
            node.local.translation.z = 8.0 - step
        }
    }
    try scene.update(root, identity)
    for node in scene.nodes {
        let t = node.world.translation
        total += try Integer.truncate(t.x + t.y + t.z)
    }
}
console.log(scene.nodes.size)
console.log(total)
```

Only the prefix of children is initialized. The parent reference survives recursive growth. The capacity checks/error paths are explicit; no successful link follow tests a generation. All three passes retain their source order. Dense pool slot iteration is specified here as insertion order for a GrowingPool (no deletion); make that guarantee explicit rather than relying on the current unordered Pool wording. This matters when effects/floating-point order are observable. B includes an additional slot-order Pool guarantee; current Pool's unspecified order alone would not justify an order-sensitive port.

### Tree: left, right, parent construction; eight pre-order sums

```fa
function xorshift = (x: UInt64): UInt64 => {
    mutable v = x
    v = v.bitXor(v.shiftLeft(13))
    v = v.bitXor(v.shiftRight(7))
    v.bitXor(v.shiftLeft(17))
}
type Tree = {
    nodes: GrowingPool<Node> = []
    type Node = {
        value: Int32
        link left: (Node in nodes)? = none
        link right: (Node in nodes)? = none
    }
    function build = (self, depth: Integer<min = 0>, seed: UInt64): Node in self.nodes
        mutates self.nodes.size => {
        if depth == 0 {
            return self.nodes.add({ value = Int32(seed %% 1000) })
        }
        use left = self.build(depth - 1, xorshift(seed))
        use right = self.build(depth - 1, xorshift(seed.wrappingAdd(1)))
        self.nodes.add({ value = Int32(seed %% 1000), left = left, right = right })
    }
    function sum = (self, node: Node in self.nodes, pass: Integer): Integer => {
        mutable total = Integer(node.value).bitXor(pass)
        use left = node.left
        if left is not none { total += self.sum(left, pass) }
        use right = node.right
        if right is not none { total += self.sum(right, pass) }
        total
    }
}
mutable tree: Tree = {}
use root = tree.build(21, 12345)
mutable total = 0
for pass in 0..8 { total += tree.sum(root, pass) }
console.log(tree.nodes.size)
console.log(total)
```

The task's parent-first requirement applies to scene; the checked-in tree source deliberately builds children before their parent. Both are expressible without changing allocation order. These payloads own no external storage, so destroying Tree visits its chunks rather than recursively following left/right.

## The other six ports

The same proposed A syntax is used below. For B, replace the GrowingPool with Pool where one occurs; its extra proof is relevant only to deletion. Helpers below are sketches with explicit dependencies, not complete executable ports or measured replacements.

| Port | A representation and order | B and remaining differences |
| --- | --- | --- |
| binary-trees | Per-call owner containing GrowingPool of Node with two optional links; post-order build, recursive count, then scope exit. Keep the long-lived tree in a separate owner. | Same program with Pool; occupancy overhead, no proof benefit. |
| interp | Pool of Expr union; Num has Int32, Add/Mul have two required links, Neg one. Post-order allocation; evaluate left before right, eight pass-dependent walks. | Same with Pool. Fixed max-payload union slots remain larger than Goose's packed variable records in both options. |
| graph | GrowingArray of `{ to: UInt32, link next: (Edge in edges)? }`; first V entries are heads, then append 8V edges and prepend by rewriting each head. Four BFS runs follow reverse insertion order, reusing queue/distances. No extra head array. | GrowingArray already supplies the no-deletion case; B adds no expressiveness. Using Pool alone lacks indexed heads and must not introduce an undisclosed head table. |
| sexp | Expr union pool with optional next on every variant and first on List; owner `link roots: Array<Expr in nodes>`. Prepend sibling chains during recursive parse; walk in reverse source order four times. Keep eof/unclosed errors. | Same with Pool. Strings remain owned String values; no varint/inline-symbol parity is claimed. |
| calc | Per-input owner with GrowingPool<Expr, UInt16>, separate parser position; Num UInt64, Neg/Paren one required link, Bin Byte op and two links. Keep Paren and the same damaged inputs and deep errors. | Same with Pool<Expr, UInt16>. Neither option implies variable-length number encoding. |
| lru | Ordinary Pool + existing relations remains the deletion-safe choice, with extra reverse work. A can express preallocated permanent slots and explicit payload reuse, but that is a separately approved container variant. | Dynamic unlink/map-shift/delete leaves Unknown incoming sets; B rejects free/delete without a new proof contract. Neither option makes this an exact safe ordinary-Pool port. |

Per-call local ownership for binary-trees retains its 8-byte node shape:

```fa
-- PROPOSED link fields; other syntax exists.
type BinaryTree = {
    nodes: GrowingPool<Node> = []
    type Node = {
        link left: (Node in nodes)? = none
        link right: (Node in nodes)? = none
    }
    function build = (self, depth: Integer<min = 0>): Node in self.nodes
        mutates self.nodes.size => {
        if depth == 0 { return self.nodes.add({}) }
        use left = self.build(depth - 1)
        use right = self.build(depth - 1)
        self.nodes.add({ left = left, right = right })
    }
    function count = (self, node: Node in self.nodes): Integer => {
        mutable total = 1
        use left = node.left
        if left is not none { total += self.count(left) }
        use right = node.right
        if right is not none { total += self.count(right) }
        total
    }
}
function treeCheck = (depth: Integer<min = 0>): Integer => {
    mutable tree: BinaryTree = {}
    use root = tree.build(depth)
    tree.count(root)
}
```

For interp/calc the essential union declaration is:

```fa
-- PROPOSED: link fields can occur in union member records.
type Calculation = {
    nodes: GrowingPool<Expr, UInt16> = []
    type Expr = Union {
        Num = { value: UInt64 }
        Neg = { link operand: Expr in nodes }
        Paren = { link operand: Expr in nodes }
        Bin = { op: Byte, link left: Expr in nodes, link right: Expr in nodes }
    }
}
```

Parsing methods return `Expr in self.nodes`, declare `throws ParseError`, and mutate only `self.position` and `self.nodes.size`. Put the parser state and pool in the same owner if self owns position; keep text as a borrowed caller argument rather than copying the reused input buffer into every Calculation. `runOne(text)` owns the Calculation local, calls parse with `try`, checks trailing input, then eval with `try`. `ParseError` has eof/paren/token variants, and evaluation declares division-by-zero separately. Every return/throw destroys the partial pool; Num/Byte/link nodes need no node cleanup. A successful `runOne` returns the numeric result, never its local root. No external shared node counter is lost on failure: if retaining the benchmark's global total, increment it after each successful add under a declared effect.

For graph, an insertion is conceptually: borrow `head = graph.edges[u]`; borrow its old next; append a new Edge holding old next; then write head.next to the fresh Edge. Evaluate construction operands before publishing the new element. The add/push effect changes only length, so head survives segment growth. Vertex UInt32 validates when used as a distance index; a general 32-bit vertex is not automatically a Fa bounds proof. Retain `try` on dynamic array indexing unless the existing range analysis proves the index. Never substitute CSR, reverse the BFS order, or append neighbours in forward order.

For sexp, keep roots in the enclosing owner, since a bare local Array of references remains forbidden. A recursive `parseNode(previous: (Expr in self.nodes)?)` returns a reference into the same pool, initialized after its children. Generic read of a root list must bind the target, including through erased JavaScript callbacks. The scope of a root array is no longer a hidden independent lifetime.

## Implementation plan for A

These are implementation slices, **not changes made by this study**. They depend on enforcing the current memory-management specification (category/unique checks, alias/branch/partial moves and protected borrows), whose integration status still names pending slices. New links must use that shared ownership engine; current acceptance of an ownership violation supplies no proof. Slices 1–2 are independent foundations after slice 0; 3 uses 1–2; 4–6 implement the same analyzed contract per engine; 7 depends on complete copy/transfer rules. Keep each slice reviewable and keep unsupported storage rejected until its backend implements it.

| Slice | Work and boundaries | Required evidence |
| --- | --- | --- |
| 0. Close the current hole | In `analyzer/check/type_decls.rs` field checking and `check/relations.rs`, reject any nested reference type outside a recognized relation/link storage contract, **before** it erases to an owned element type. The existing `contains_reference_type` walker is recursive, but local declarations call its rejection while unmatched ordinary fields do not. Follow type aliases/imports too; a syntax-only scan is insufficient. | scene-children must become REF010, plus optional/tuple/nested-collection/aliased/generic variants. Existing legal relation fields still work. No generated owning copy is a fallback for an unsupported reference field. |
| 1. Owner paths and reference results | Preserve target paths in analyzed reference/storage types and function interfaces; substitute receiver/parameter/private/nested owner paths through calls, returned references, structural views and imports. Reuse the existing path/freshness checker, not a second alias engine. Fix the documented user-method reference-return gap seen in the local-owner probe. | Return add results into a caller-owned local wrapper; reject a local-pool escape and cross-owner stores, including function values/generic/import wrappers. |
| 2. InlineArray | Add a builtin generic type with compile-time capacity using existing angle/value-parameter parsing. Add length/range facts, conditional capacity error, method effects, ownership/category/initialized-prefix operations. Implement LLVM inline aggregate ABI and JS dense-array operations; interpreter mirrors prefix lifetime. | N=0, 1, 255, 256; exactly-full/overflow error; no mutation on failure; order/duplicates; partial item errors; strings/resources; moves/copy and suffix invalidation. |
| 3. Explicit link contract | Add contextual `link` before a type field, including union-member fields; do not change expression `use` or ordinary relation parsing. Keep marker, exact target collection and list shape in type/interface metadata. Classify links separately from relation sides. Compute containment dependencies and reject partial reset/move/copy, detached captures/returns, subfield targets, and destructors accessing a dying boundary. Add accurate target-root effects and generic copy/closure conditions. | Self/cross-pool cycles and DAGs accepted; wrong pool, ordinary Pool target, broad invalidating helpers, target-pool-only reset/extraction rejected. Narrow field/size effects stay accepted. Destructor/cancellation/error-path regressions. |
| 4. Native compact pools | Extend `lower/pools.rs`, `lower/references.rs` and value/layout operations with Index storage, a stable ordinal/page mapping, inline union slots and link loads/stores. Distinguish reference-to-target from address-of-encoded-field. Extend runtime pool/segmented helpers where needed; regenerate checked-in IR after C changes. Publish one ABI in split compilation and bump cache versions. | Native x86-64/AArch64, freestanding and wasm32 layout tests; 8/16-bit boundary errors without huge allocations; unchecked-overflow safety; zero/null and required links; directory/page growth; IR with no link liveness/generation test; vectorizable inner scans. |
| 5. Native/interpreter graph ownership | Generate complete-owner copy/move/drop operations; preserve ordinal sharing/cycles and handle owned payloads once. Reuse lexical/error cleanup, return-output slots, in-place construction and heap-page allocators. Interpreter uses internal owner IDs + ordinals for stored links; exports graph constants symbolically, never host addresses. | Independent owner copies, unchanged native index payloads, alias invalidation after moves, once-only unique destruction, partial-construction failure, recursive compile-time build/read/copy with existing step/heap limits. |
| 6. JavaScript | A link into an object pool is the **object itself**, optional none is undefined, and a link list is an ordinary array of those objects. Primitive targets use slot indices into the statically named collection, as existing pools do; union nodes are tagged objects. A setter stores the target reference directly, not the normal owning copier. Whole-element assignment mutates the existing target object in place (including a union tag change), preserving the identity all links designate; it must not just replace the pool array entry with a new object. Complete-owner copies allocate every destination node first, then fill data/edges using one temporary old→new map. Erased generic operations carry the link-storage/copy contract. No map or wrapper on reads. | The valid new scene variant observes child mutation as 2, not 0; cycles/sharing survive copies; no wrappers/copiers on follow/append; run on Bun, Node and web. JS GC may visit/reclaim graphs later; do not promise native bulk-free timing. |
| 7. Complete-owner boundaries | Extend generated worker codecs/transfer conditions, compile-time constant emission, schemas/reflection, JSON rejection, printing/equality omissions and reactive link-field reads/writes. Ordinary relation helpers keep their reverse maintenance; new links never call them. A watch of the link list reacts to link writes, a watch of target data to its data writes. | No Task/Guard/individual-link transfer; independent graph copies and exclusive moves; untrusted graph DTO ID validation before publication; no cyclic stringify; imported/generic reactive writes. |
| 8. Ports and measurements | Replace each eligible main only after semantic and backend tests agree. Retain recursive variants and pair the appropriate Rust arena rows. Keep lru's exact-table/slot-reuse decision separate. | Original construction and traversal order, scalar widths, error counts, checksums and destructor behavior. Record actual layout/padding, scan vectorization and decode instructions; then CPU/RSS under the benchmark protocol. |

The current implementation also needs a real growing-pool fast path: `lower/pools.rs` shares `fa_pool_new`/`fa_pool_add_slot` with ordinary Pool, and the runtime helper still writes occupancy/free-slot metadata. The documentation's bump-only growing layout is the target contract, not evidence that today's compiler already generates it. Likewise, the current reverse-set layout uses pointer-sized Repr::Link/Repr::Links. On a 64-bit target, a tree record with Int32 value, two optional direct sides and two Pool reverse sides has four pointer fields (40 bytes including ordinary alignment), before lazily allocated FaLinks state, versus A's 12-byte record. This is a layout deduction from the source, not a measured ABI report. A one-to-many reassignment reads the old target, removes/adds reverse membership and can allocate reverse-set storage; A stores one index. Deletion of degree k clears k incoming/outgoing memberships, with implied writes that can conflict with iteration. No existing relation follow performs a liveness test.

The existing **root-region optimization is not the language feature**. `analyzer/regions.rs` and LLVM `lower/trees.rs` currently infer closed recursive owning trees, conservatively reject mutations/unknown generic uses, and may use a process address band, compressed/near-far links and cleanup records. They cannot justify a public persistent-link type or scene's mutation by merely adding these nodes to the admitted shapes.

Reuse allocator pages, bump allocation, in-place aggregate construction and cleanup routines where their invariants agree. Explicit GrowingPool storage gets its own declared owner and ordinal ABI; it must not depend on TLS “current recursive builder”, a Linux low-address reservation, root-builder names or a near/far test. Keep root-region inference for existing recursive-value programs, where it still implements a different ownership contract. Retire its special paths only after equivalent coverage and measurements; this study recommends no speculative deletion of that machinery. For fixed/max-payload linked nodes, do not reuse an owning recursive-child copier or destructor: edges neither allocate nor recurse.

## Relevant comparisons and source basis

Goose's spec distinguishes field-relative signed byte offsets (same exact root) from global-pool-relative biased byte offsets. Its limited arrays retain initialized slot values after pop; Fa should instead keep its specified element invalidation and destruction rules. Goose's root analysis specializes calls by root and restricts copying self-relative-containing values; A uses Fa's existing explicit owner paths and complete-owner copies. The ordinal choice sacrifices its direct addition and byte-packed union storage, but avoids field-relative fixups and global-base restrictions. The installed `~/.local/goose` contains the binary/stdlib, **not the promised spec checkout**; I read the pinned audit revision [Goose spec at eadca40, §§3.9, 5.1, 5.3, 9.2 and A.2](https://github.com/aardappel/goose/blob/eadca40/docs/goose_spec.md), plus all eight checked-in Goose programs and the installed slot_pool source. No statement about Goose's checker is inferred solely from benchmark comments.

Rust's Vec+indices arena is close to A's storage: an index can survive buffer movement, but ordinary safe lookup still checks bounds and does not statically brand it to one Vec. [NonZeroU32](https://doc.rust-lang.org/std/num/type.NonZeroU32.html) supplies the same-size optional niche. [typed-arena](https://docs.rs/typed-arena/latest/typed_arena/) shows why no individual deletion permits same-lifetime cyclic references; its lifetime/borrowing API is not Fa's freely aliasable field-effect API. [slotmap](https://docs.rs/slotmap/latest/slotmap/) supports slot reuse by versioned keys and a version comparison, precisely the runtime mechanism excluded here; B cannot acquire that flexibility for free.

Zig's [BoundedArray at 0.14.1](https://github.com/ziglang/zig/blob/0.14.1/lib/std/bounded_array.zig) is a useful explicit buffer+length/capacity-error precedent, distinct from a spilling small vector. Its [ArenaAllocator](https://github.com/ziglang/zig/blob/master/lib/std/heap/arena_allocator.zig) supplies allocator bulk lifetime, not A's checked owner-link containment. These are scoped comparisons, not a claim that the versioned BoundedArray API exists unchanged in every Zig release.

Swift's [InlineArray](https://developer.apple.com/documentation/swift/inlinearray) is fixed-count inline storage requiring all elements initialized, so corresponds more closely to CArray than to the proposed initialized-prefix list. Swift's ordinary class links use ARC; [weak/unowned](https://docs.swift.org/latest/documentation/the-swift-programming-language/declarations/) either maintain weak state, diagnose a dead unowned access at runtime, or opt out of safety. They do not satisfy this task's check-free compile-time guarantee.

Fa sources read: the requested collection/reference/relation/memory/functions/types/foreign-code pages, including their implementation sections; the relevant native/regions/relations/compressed-payload/ownership/benchmark records in `design/decisions.md`; `perf-ports.md`'s audit/probes; all eight current Fa/Goose sources; and the analyzer/backend entry points named above. Existing relation implementations also use runtime link sets for some reverse shapes; the implementation is not universally an intrusive O(1) doubly-linked reverse list just because a prose layout note describes one. The reliable comparison here is the extra reverse state, updates and O(k) deletion cleanup, not an invented exact byte cost for every relation cardinality.

## Decisions for the lead (proposed answers)

1. **Safety model:** adopt A; targets are GrowingPool and GrowingArray, with declared owner containment. Keep ordinary relations for deletion. Defer B and owning edges.
2. **Spelling:** adopt `link field: T in pool`, optional and Array/InlineArray forms. Plain reference fields keep existing relation semantics; no reverse-absence inference. A runtime pool is never a compile-time generic value.
3. **Owner/copy semantics:** a complete declared owner moves/returns/copies with internal links preserved. Reject detached link-bearing element copies and incomplete pool copies; projection is explicit. Outgoing links are replaced by whole-node assignment, unlike relations. Partial target reset/extraction requires the complete containment boundary.
4. **Inline list:** `InlineArray<Item, capacity>`, no spill, smallest length counter, initialized-prefix lifetime, typed capacity failure, ordinary collection invalidation. CArray stays fixed-count.
5. **Widths/limits:** target collection chooses Index, default UInt32; UInt8/16/64 explicit. Limit is representable element ordinals, not byte distances or reservation hints. Approve the finite bound explicitly; enforce index exhaustion as a mandatory resource stop, reserving typed CapacityError for InlineArray. Never widen silently. Purely inferred narrowing is deferred until there is a portable declared maximum.
6. **Layout scope:** dense stable pages/segments and fixed-stride union slots first. Accept one directory/segment translation per uncached link, 120-byte aligned scene nodes and larger union slots as disclosed costs. Packed variable-node/varint parity is separate work requiring a measured ABI decision.
7. **Boundaries and observation:** allow complete closed-owner worker transfer/copy when its data permits it; reject individual links/Guard/Task escapes. Reject graph JSON without an explicit DTO. Omit link fields from generic print/equality/hash as relations do; a custom graph comparison/serializer must say how it treats topology. Check destructive callbacks against the dying boundary.
8. **Iteration:** explicitly guarantee insertion-order iteration for GrowingPool. B's no-deletion Pool history would need slot-order semantics for strict order-sensitive ports; do not extend unspecified order by implementation accident.
9. **lru:** do not declare it solved. Use existing relations for the ordinary deletable model, or approve a distinct permanent-slot container and exact open-table comparison. Neither A nor conservative B supplies arbitrary safe reverse-free deletion.
10. **Landing order:** fix the reference-array rejection independently, then owner-path results + InlineArray, explicit links, engines/copy, boundaries, and finally the seven eligible ports. No port replacement based only on checksum agreement.

## Validation and handoff

Only this report was written as a deliverable. No compiler/specification/benchmark/decision/todo files changed, and no commits were needed for a document outside the worktree. Branch `design/pool-links`, unchanged HEAD `4785c025`; final worktree status is clean. Temporary probes and generated code are under `/tmp`, builds under ignored `target/`.

Current-checker probes, built with `CARGO_BUILD_JOBS=2` and LLVM disabled:

| Probe | Observed result |
| --- | --- |
| `benchmarks/audit/ports/pool-links/main.fa` | Rejected: REL001 on left and right. |
| `benchmarks/audit/ports/local-marks/main.fa` | Rejected: REF010. |
| `benchmarks/audit/ports/scene-children/main.fa` | Incorrectly accepted. Fresh Bun build prints **0** after child.id becomes 2; this confirms the owning-copy hole, not reference semantics. Bun peak RSS 16,512 KiB. The audit's native/wasm observations were read, not rerun here. |
| Local owner method returning `Node in self.nodes` from add | Rejected at `use node = arena.add(3)`: REF004, consistent with the documented owner-path/result implementation gap. A prerequisite, not a reason to change the proposed syntax. |
| Local GrowingPool reset then old reference read | Rejected: REF001, invalidated by assignment. This existing check does not settle persistent incoming links. |

The eight report code blocks pass the existing parser/formatter **after removing only the proposed `link` modifier**. This verifies the existing syntax skeleton, not the new feature's type/ownership semantics. The scene arithmetic preamble type-checks separately; dynamic reads through CArray parameters currently require explicit IndexError propagation, included in the complete scene program. No proposed program is claimed to compile unchanged today.

Repository baseline checks: `cargo fmt --all` passed with no changes; `cargo clippy --workspace --all-targets` passed with no Clippy diagnostics; `cargo test --workspace` exited 0, reporting **1,272 passed, 0 failed, 8 pre-existing ignored**. LLVM was deliberately disabled for this design-only task; the expected disabled-backend build notice remains, and these are not native/wasm feature-validation results. All builds/checks/tests were pinned to CPUs 0–4, one Cargo command at a time with two build jobs; checker probes used a 512 MiB address-space cap. No timed benchmark was run.

Next action belongs to the lead: arbitrate the ten decisions above before an implementation task. The first concrete independent fix is the invalid reference-array acceptance. Seven pool-based ports can then regain the intended ownership/construction/traversal shapes; variable-record packing, exact lru slot reuse/open-addressing policy and all performance claims remain separate, explicit follow-ups.
