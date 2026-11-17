# Memory management

Fa manages memory without a garbage collector on native and WebAssembly targets. Objects live inline, collections own their storage, and references name the places they designate. JavaScript has the same observable rules.

## Copies, moves, unique values and shared handles

Storing a value means initializing or replacing a local, a module value, a field or a collection element, returning an owned result, or putting a capture into an escaping closure. The same rules apply at every such site:

| Category | Storing a value read from a place | `copy value` |
| --- | --- | --- |
| Ordinary value | independent value semantics; the source remains usable | an independent value |
| Structure | move at a proven last use; otherwise `MEM004` | an independent deep copy, leaving the source usable |
| Unique value, or a value owning one | move; the source and its aliases become invalid | `MEM003`: cannot be copied |
| Synchronization handle | another handle to the same state | another handle to the same state |

Unique ownership takes precedence: an array of resources is unique, not a copyable collection. Shared handles are ownership boundaries: a `Mutex<Array<Integer>>` shares its state; copying its handle does not duplicate its array.

### Ordinary values

Numbers, booleans, `none`, ranges, enumerations, flags, strings, and nonrecursive objects, tuples, optionals, unions, `CArray` and `InlineArray` values containing only ordinary data have value semantics. Assignment leaves both values usable. Strings are ordinary values: a string is an immutable sequence; `+=` replaces a variable's value, and never changes another string.

```fa
mutable person = { name = "Ana", age = 20 }
let previous = person
person.name = "Bao"
console.log(previous.name, person.name) -- Ana Bao
```

Function values copy their callable handle and may retain an immutable environment; this does not duplicate the structures already captured at creation. Captures themselves follow the [closure storage rules](#closures).

An object whose only non-scalar fields are strings is ordinary. `String` never requires `copy`. The native cost of storing a long string is specified in the implementation notes of this page and of [strings](/values/strings).

### Structures

A **structure** is an unbounded collection, a value of a recursive type, or any value owning either transitively through fields, tuples, optionals or unions. Its owned data can grow without a bound given by the type, so it is never copied silently: its last use moves, and otherwise keeping two owners needs `copy` (`MEM004`). This category excludes scalars, strings, ranges, enumerations, flags, and nonrecursive objects, tuples, optionals and unions made only of those. Unique ownership still takes precedence; references borrow and shared handles keep identity rather than duplicating their payload.

The unbounded collection types are `Bytes`, `Array`, `GrowingArray`, `Set`, `GrowingSet`, `Map`, `GrowingMap`, `Pool`, `GrowingPool`, `Columnar`, `GrowingColumnar` and `OrderedColumnar`. This includes tables represented by maps or columnars, and a relation collection when materialized as an owned value. A relation field itself is a link, not an owned copy of its targets.

An object owns a structure when one of its data fields is a collection, a recursive value, or owns one transitively. Tuples, optionals and unions own the structures in their items or alternatives; named-union payloads follow the same rule. A union is copyable only if every remaining alternative is copyable. Narrowing to a particular alternative uses that alternative's category. A fixed inline `CArray` or bounded `InlineArray` owns its initialized items; fixed inline storage alone does not make it a structure. An `InlineArray` of ordinary items is ordinary; structure or unique items propagate their category. Copying synchronization handles inside one preserves their shared state. See [inline arrays](/collections/inline-arrays#ownership-and-values). References and shared handles stop this traversal: a reference borrows its target, a synchronization handle shares its state.

```fa
mutable original = { items = [1, 2] }
mutable duplicate = copy original
duplicate.items.push(3)
console.log(original.items, duplicate.items) -- [1, 2] [1, 2, 3]
```

```fa
let pair = ([1, 2], 3)
let duplicatePair = copy pair
let optional: Array<Integer>? = [4]
let duplicateOptional = copy optional
let choice: Array<Integer> | String = [5]
let duplicateChoice = copy choice
console.log(pair, duplicatePair, optional, duplicateOptional, choice, duplicateChoice)
```

`Bytes` is a collection because its items can be changed in place:

```fa
mutable bytes = "hello".toUtf8()
mutable duplicate = copy bytes
duplicate[0] = 72
console.log(bytes[0], duplicate[0]) -- 104 72
```

An error payload is data and cannot own a unique value (`MEM005` at its declaration): keep an identifier or a description of the resource instead. Shared synchronization handles are allowed. The open `Error` type is conservatively a structure, never unique: a last use moves it, and keeping both values needs `copy` (`MEM004`). A concrete error member uses its payload's category. See [errors](/values/errors#declaring-error-types).

A copy preserves contents, not spare capacity. Reserve the destination separately if it needs more room.

### Explicit `copy`

`copy expression` evaluates its operand once and produces a fresh, independent value in the expected type, or the operand's type when none is written. It is a value, never an assignable place or an alias. For a plain data record, an expected-type projection copies only its retained fields; omitted fields stay with the source. A retained unique field makes the copy illegal. A resource or built-in lifetime owner always needs a written data projection first: `copy resource` is `MEM003`, even with a plain expected type. It recursively copies owned data, including collections and recursive children; shared handles remain shared. It accepts ordinary values too. It cannot duplicate a Task, a resource or any owned composite containing one (`MEM003`).

Access, indexing and calls bind more tightly than `copy`; arithmetic binds less tightly: `copy source.items + other` means `(copy source.items) + other`. Parenthesize a larger operand: `copy (left ?? right)`. `copy` is reserved, like `await`: `copy(x)` copies the parenthesized expression, `copy [1, 2]` and `copy[0]` copy array literals, and `copy -x` copies the negated value. Whitespace does not change its meaning; a newline may follow it. A variable, parameter, function or type cannot be named `copy` (`E-LEX-3`: “`copy` is reserved: it copies a value (`copy items`); choose another name”). After a dot it is accepted as a member name, just like `await`: `object.copy(value)` calls that member.

`copy` authorizes copying or converting its operand's completed value; fields, items and arguments inside the expression that computes it are checked at their own stores:

```fa
let small: Array<Int8> = [1, 2]
let invalid: { items: Array<Integer> } = copy { items = small } -- error: MEM004: the enclosing copy does not reach this field store
let valid: { items: Array<Integer> } = { items = copy small }
console.log(valid)
```

```fa
function duplicate = <Item>(value: Item): Item => copy value
let source = [1, 2]
let independent = duplicate(source)
console.log(source, independent)
```

Generic bodies infer copy conditions on their type parameters. They are checked at each concrete instantiation, including imports, wrappers and permitted function-valued callbacks. No `Copyable` bound is written. `duplicate<Integer>` and `duplicate<Array<Integer>>` work; `duplicate<Task<Integer>>` and a resource instantiation fail with `MEM003`. Returning a borrowed generic parameter without `copy` requires an ordinary instantiation; a structure instantiation fails with `MEM004`. Merely reading or borrowing a generic parameter imposes no copy requirement.

### Last uses and moves

A move transfers ownership without duplicating the owned data. The last use of an owner is the last use of its parts: reading a structure field from an owner with no later use transfers that field and destroys unused siblings. Patterns (`when` and destructuring) over structures use exactly the same rules as field reads in that storage or borrowing position. The old place and every reference into it become invalid, including `use` aliases, aliases of fields or elements, and captures that still read it. Reading, writing, passing, indexing, calling a method, testing or capturing that old value counts as using it. Reading any field of a moved whole object is a use of that object.

```fa
function build = (): Array<Integer> => {
	let values = [1, 2]
	let result = values -- last use: moves the array
	result
}
console.log(build())
```

Last use is a control-flow property, not the last textual occurrence. Mutually exclusive branches can each transfer the same owner if no later path uses it. Loop back edges count: moving an outer value on one iteration cannot leave the next iteration using it. A replacement can consume the previous value and install the new one (`values = copy values` is an explicit copy; `head = Link { next = head }` can transfer the old root). A failed operation follows the failure edge with the ownership state at that evaluation point; a lexical `catch` does not restore a moved value.

```fa
function choose = (flag: Boolean): Array<Integer> => {
	let values = [1]
	if flag { values } else { values }
}
console.log(choose(true), choose(false))
```

The compiler must prove exclusive ownership. A borrowed parameter, a `use` alias, an iteration variable, an indexed element of a collection that stays live, or an imported module value cannot silently surrender its owner's storage. Use `copy` to obtain independent copyable data; use `take` to remove an element with ownership. The fields of a last-use data record, fresh or already bound, may transfer while its unused fields are destroyed. A resource's fields cannot move out from under `destroy`: transfer the whole resource, or explicitly copy a copyable data field.

#### Partial field moves

A move invalidates exactly the moved path. A field path can contain fields of fields and known tuple positions, in a local or temporary that the code owns. After a field moves, its siblings remain usable; the moved field and references into it are invalid. Passing, copying, storing or returning the owner as a whole is `MEM002` until the missing field is assigned again. The diagnostic names the moved path and the line of its move, and suggests `copy` at that move or replacing the field before the whole use.

```fa
mutable holder = { nested = { items = [1], count = 2 }, pair = ([3], 4) }
let items = holder.nested.items
let tupleItems = holder.pair[0] -- a tuple position, not a collection lookup
console.log(holder.nested.count, holder.pair[1], items, tupleItems)
holder.nested.items = [5]
holder.pair[0] = [6]
console.log(holder) -- every field is initialized again
```

A temporary can give up a field too. An indexed collection element transfers only when removed by an owning operation, or when the whole collection is at its last use:

```fa
type Holder = { items: Array<Integer>, count: Integer }
function make = (): Holder => Holder { items = [1], count = 2 }
let items = make().items -- transfers from the temporary, destroys unused siblings
let matrix = [[1], [2]]
let first = matrix[0] -- consumes the collection's last use; remaining items are destroyed
console.log(items, first)
```

A whole use before restoring a moved field is invalid:

```fa
mutable holder = { items = [1], count = 2 }
let items = holder.items
console.log(holder.count, items) -- siblings remain valid
console.log(holder) -- error: MEM002: holder.items moved on line 2
-- Fix: write `copy holder.items` at the move, or assign holder.items before this whole use.
```

A conditional partial move invalidates the same field on paths where it happened. After joining the branches, a whole use is invalid until the field is definitely initialized on every path; sibling reads still work:

```fa
function invalid = (take: Boolean) => {
	mutable holder = { items = [1], count = 2 }
	if take {
		let items = holder.items
		console.log(items)
	}
	console.log(holder.count)
	console.log(holder) -- error: MEM002: holder.items may have moved in the branch
	-- Fix: copy at the branch's move, or assign holder.items after the branch.
}
```

A partial move never leaves a hole through a collection index or key while that collection stays live. Use `take` to remove the item, or `copy` to keep both:

```fa
mutable matrix = [[1], [2]]
let row = matrix[0] -- error: MEM004: use `copy matrix[0]` or `matrix.take(0)`
console.log(matrix, row)
mutable rows: Map<String, Array<Integer>> = [("one", [1])]
let entry = rows.get("one") -- error: MEM004: use `copy rows.get("one")` or `rows.take("one")`
console.log(rows, entry)
```

A `use` reference cannot give away the value it borrows, even when its target is a local:

```fa
mutable holder = { items = [1], count = 2 }
use borrowed = holder
let items = borrowed.items -- error: MEM004: write `copy borrowed.items`
console.log(holder.count, items)
```

A borrowed parameter cannot transfer a field either:

```fa
function extract = (holder: { items: Array<Integer> }): Array<Integer> => holder.items -- error: MEM004
-- Fix: return `copy holder.items`, or return a reference naming the caller's owner.
```

A resource stays whole for its destructor, including at its last explicit use:

```fa
-- Held.foreign.fa
type Held = { items: Array<Integer>, function destroy = (self) => {} }
let held = Held { items = [1] }
let items = held.items -- error: MEM004: write `copy held.items`; destroy still owns this field
console.log(items)
```

A unique resource field cannot be copied as a fix (`MEM003`); transfer its complete resource owner instead. These limits apply equally to pattern bindings. A conditional whole move and a conditional partial move use the same flow rule.

### Unique values

A `Task`, a type declaring `destroy` (a resource), and every composite owning one are unique. Built-in lifetime owners are unique too; the only shared identity types are the synchronization handles listed below. Except for the Task-consuming contract below, its parameter borrows; it has no counted shared-owner exception. Assignment transfers ownership; using the old owner is `MEM002`. `copy` is `MEM003`. A second resource requires a new instance, such as `open`, `create` or a constructor.

| Built-in value | Category and lifetime |
| --- | --- |
| `Task<Result>` (with its declared errors) | unique; await/Task-consuming calls consume its owner; cancellation ends its computation; executor-local |
| Worker handle of a `*.worker.fa` type | unique; `stop()` or scope exit ends the worker |
| `WorkerPool<Worker>` | unique; `shutdown()` or scope exit ends its pool |
| `std.net.TcpListener` | unique; `close()` or destruction ends the listener |
| `std.net.TcpStream` | unique; `close()` or destruction ends the socket |
| `Guard<Value>` returned by Mutex locking | unique; destruction ends the lock acquisition |
| `Subscription` | unique; `stop()` or scope exit ends the subscription |
| `Mutex<Value>`, `Channel<Value>`, `AtomicInteger`, `AtomicBoolean` | shared synchronization handles; copies retain the same identity |

Any further built-in lifetime owner, including a library server resource with close/stop/dispose, follows the unique rule; the explicit synchronization handles retain their sharing contract. A method name alone does not make an ordinary user type unique: its `destroy` does. Foreign `Pointer` and `CString` are addresses, not automatic allocation owners; foreign code wraps an owned C lifetime in a resource with `destroy`.

To make a unique handle available to several users, put its one owner behind a common Mutex or communicate through a Channel to the task that owns it. Copying the handle itself cannot create another owner.

```fa
let task = start do { 42 }
let owner = task -- transfers the task
console.log(await owner)
```

Resource parameters borrow. The callee never finalizes the caller's resource, even if it contains a Task. It may read it repeatedly or mutate it under a declared effect; it cannot return it as an owned result, store it in another owner or move one of its unique fields. Task parameters are the exception to the ordinary borrowed-parameter rule: a Task, or a plain composite owning one, is transferred to its callee and consumed there. That transfer stops at a resource or shared-handle boundary.

```fa
-- Held.foreign.fa: only foreign files may declare destroy.
type Held = { value: Integer, function destroy = (self) => {} }
function read = (held: Held): Integer => held.value
let first = Held { value = 1 }
let second = Held { value = 2 } -- a new resource, not a copy
console.log(read(first), read(first), read(second)) -- parameters only borrow
```

An implicit function adapter cannot copy a resource result into a plain data result. Write the projection and any required `copy` visibly in the source function. A borrowed structural view can read fields for the duration of a call; it does not remove the resource's destructor.

### Structural projections

A store is checked on what is actually stored. A conversion to a narrower data type builds a value from its retained fields only. Keeping only ordinary fields needs no `copy`, even when a discarded field owns a structure:

```fa
function label = (box: { items: Array<Integer>, name: String }): { name: String } => box
```

A retained structure field follows the normal store rule. A borrowed parameter cannot surrender it; a still-used source needs `copy` (`MEM004` on the retained field). A last-use owner may transfer its retained fields under the field move rules:

```fa
function select = (box: { items: Array<Integer>, name: String }): { items: Array<Integer> } => box -- error: MEM004: box.items needs copy
function selectCopy = (box: { items: Array<Integer>, name: String }): { items: Array<Integer> } => ({ items = copy box.items })
```

A resource declaring `destroy`, or a built-in lifetime owner, cannot be narrowed implicitly when stored (`MEM002`). Write the data projection visibly; `copy resource` remains `MEM003`. No conversion may silently drop its destructor:

```fa
-- Held.foreign.fa: destroy declarations belong to foreign files.
type Held = { name: String, function destroy = (self) => {} }
let resource = Held { name = "file" }
let invalid: { name: String } = resource -- error: MEM002: write { name = resource.name }
let plain = { name = resource.name }
console.log(plain)
```

Borrowing a narrower view does not truncate its owner. See [structural typing](/advanced/structural-typing#copying-narrows-referencing-does-not).

### Converting structures

The ownership and conversion restrictions apply to stores. Equality and ordering, `is`, membership, printing and hashing for a lookup only inspect their operands: they retain their comparability rules and never copy or convert either collection.

An implicit conversion never converts the inside of a structure. Its item type is invariant, except literal and range widening within one primitive type: `Array<1 | 2>` or `Array<Integer<min = 0, max = 10>>` can move or borrow as `Array<Integer>`, `Array<"a" | "b">` as `Array<String>`, and boolean literals as `Boolean`. Those types have their primitive's storage, so this exception never copies items. The exception holds at any depth: tuples and objects with the same fields may differ only by such widenings. For example, `[("é", 0), ("日", 1)]` can be stored as `Array<(String, Integer)>` without `copy`.

Every other change builds a new structure by converting its parts and requires `copy` (`MEM004`): another numeric type, adding union alternatives or optionality, projecting items' fields, or a different recursive type. This applies at last use and to borrowed arguments too:

```fa
let small: Array<Int8> = [1, 2]
let invalid: Array<Integer> = small -- error: MEM004: write copy small
let wide: Array<Integer> = copy small
function read = (items: Array<Integer>) => console.log(items)
read(copy small)
```

```fa
type Small = { value: Int8, next: Small? }
type Wide = { value: Integer, next: Wide? }
let small = Small { value = 1, next = Small { value = 2 } }
let invalid: Wide = small -- error: MEM004: the conversion walks the whole value
let wide: Wide = copy small
console.log(wide.value)
```

`Array<Int8>` to `Array<Int8 | String>`, `Array<Integer>` to `Array<Integer?>`, and `Array<Person>` to `Array<{ name: String }>` require `copy` as well. For a parameter, `<Item: { name: String }>(items: Array<Item>)` keeps the actual item type and borrows without conversion. Projecting only the value's own fields, while keeping its items and recursive parts at their types, follows the preceding projection rule.

This is a language rule shared by every target. A backend cannot give implicitly compatible item types different layouts. Its range-packing proofs must preserve every allowed view through the primitive base type. The compiler may fuse an explicit copy with destruction of a last-use source. See [type conversion](/types/type-conversion#converting-the-inside-of-a-structure).

### Shared handles

`Mutex`, `AtomicInteger`, `AtomicBoolean` and `Channel` handles share identity: assignment or `copy` gives access to the same state. An object, tuple, optional or union containing only ordinary values and these handles has ordinary value semantics: its data copies as values and its handles keep their identity. An explicit copy of a composite containing both a handle and a structure duplicates its owned data independently and retains the handle; keeping both composites requires `copy`. Sharing a handle does not authorize an unprotected borrow of its payload.

```fa
use { Mutex } = std.concurrency
mutable mutex = Mutex { value = 1 }
mutable same = mutex
-- Both names designate the same protected value.
```

## Objects are stored inline

An object lives in its function frame, in a parent field or in a collection slot. It has no separate allocation. Heap storage belongs to collections, recursive values, task frames, escaping environments and shared handles. The physical choice of frame, static or heap storage does not change ownership.

### Recursive types

An object cannot contain itself directly: its size would be infinite. Recursion through a union or optional is allowed; the compiler supplies the indirection. Each value owns its children; use a pool and [relations](/advanced/relations) for a graph with shared nodes.

```fa
type Node = { value: Integer, next: Node? }
type Tree = Union {
	Leaf = { value: Integer }
	Branch = { left: Tree, right: Tree }
}
```

A recursive value is a structure even when its leaves contain only ordinary data. Building and consuming a list use moves, including when assignment replaces the variable being read:

```fa
type List = { head: Integer, tail: List? }
mutable list: List? = none
for head in 0..3 {
	list = List { head = head, tail = list } -- moves the old root into tail
}
mutable cursor = list -- last use transfers the list
while cursor is not none {
	console.log(cursor.head)
	cursor = cursor.tail -- moves the child; drops the rest of the old root
}
```

To walk without consuming, use a reseatable reference:

```fa
type List = { head: Integer, tail: List? }
let list = List { head = 1, tail = List { head = 2, tail = none } }
mutable use cursor: List? = list
while cursor is not none {
	console.log(cursor.head)
	reuse cursor = cursor.tail
}
console.log(list.head) -- the owner is still usable
```

Keeping a child as a second owner while reading that same child again requires `copy`; a reference needs `use`. Reading only sibling fields permits a [partial move](#partial-field-moves): reading `list.head` alone does not keep ownership of `list.tail`.

```fa
type List = { head: Integer, tail: List? }
let list = List { head = 1, tail = none }
let rest = list.tail -- error: MEM004: use `use rest = list.tail` or `copy list.tail`
console.log(list.tail, rest)
```


## Collections own the heap

A collection owns its elements and releases them with its storage. Construction builds fresh values directly in their final slots where possible. Passing an existing value into an insertion is a storage site and follows the category rules above.

### Heap blocks made by the compiler

The compiler also allocates recursive nodes, task frames and escaping closure environments when their lifetime requires it. Synchronization handles use reference counting. An implementation may count immutable function environments. Acquisition/subscription bookkeeping does not give their handles shared ownership; resources and Tasks are never made copyable by internal counting. Ordinary objects and strings have no reference count.

### Iterating yields references

Iteration borrows elements. It does not copy them. A loop over a mutable array can modify its elements:

```fa
mutable values = [1, 2, 3]
for item in values { item += 1 }
console.log(values) -- [2, 3, 4]
```

Storing an ordinary element copies its value. Storing a structure element requires `copy` unless it can transfer from a last-use owner; storing a unique element requires removal by an owning operation such as `take`.

### Aliases

`use` designates a place, without copying or taking ownership. It cannot outlive that place. `mutable use` can later be reseated with `reuse`; it is still a reference.

```fa
mutable values = [1, 2]
use first = values[0]
first += 1
console.log(values) -- [2, 2]
```

`use` needs a place or a call returning a reference, such as `pool.add`; a call returning an ordinary value has no place to alias. A relation field is a stored link managed by its collection, not an independently owned target. See [owned references](/advanced/owned-references) and [relations](/advanced/relations).

### Removing while iterating

An operation that invalidates an iteration's references is rejected. Use `deleteWhere` to remove matching elements, rather than deleting through a live iteration. See [iteration](/advanced/reference-checking#iteration).

## Parameters are references

Ordinary and structure parameters are borrowed, never copied at a call. A resource parameter is borrowed too. Only the explicit Task-consuming contract above transfers ownership. `mutates` permits mutation of a borrowed place and requires a mutable argument. [Defaults](/functions/defining-functions#parameters) follow written arguments: an existing value is borrowed, and a fresh value is a temporary owned by the call; consuming defaults must build their value.

```fa
function append = (values: Array<Integer>) mutates values => values.push(3)
mutable values = [1, 2]
append(values)
console.log(values) -- [1, 2, 3]
```

Storing a borrowed structure is different from passing it: write `copy`. Destructuring a parameter also borrows its fields. Returning a reference is allowed only when the caller already owns its target; returning owned data follows the storage rules.

## Lifetimes and destruction

Owned values are destroyed in reverse order of declaration at scope exit, including error and cancellation paths. A moved source is no longer an owner and is not destroyed a second time. Replacing a mutable place destroys its previous value. A resource's exact written `destroy`, bound with its owner and generic capabilities at construction, runs once before its fields are destroyed. It is never selected by searching for an equal structural type.

```fa
do {
	let first = [1]
	let second = [2]
	console.log(first, second)
} -- second, then first
```

## Closures

A closure never consumes its captures. A nonescaping closure borrows them; an escaping closure may own immutable snapshots, but its body still cannot move them out because it can be called again. Awaiting or otherwise consuming a captured Task is `MEM002`: pass it from its owning scope as a consuming parameter, such as `finish(task)`. Storing captured structure data needs `copy` (`MEM004`); move or create a unique resource in its owning scope instead. Structure and resource parameters still borrow: adding a parameter alone does not permit moving them. Only Task-owning parameters currently transfer ownership.

Passing a borrowed closure, including through a saved function handle, uses its captured references for the duration of the receiving call. Its owners must remain valid until that call completes, as specified by [reference control flow](/advanced/reference-checking#control-flow).

A nonescaping closure borrows its captures. An escaping closure owns immutable captures, using the same storage rules: ordinary values copy, a structure can move at its last use, and otherwise needs an explicit independent value before capture. An escaping closure cannot own any unique value, even at its last use; there is no one-shot closure exception (`MEM003`). A `mutable use` cannot be captured. Module-level values and requirements retain their separate rules in [closures](/functions/defining-functions#closures).

An escaping closure captures only the field paths its body uses, including fields of fields and tuple positions of a value owned by the enclosing code. For `() => holder.count`, only `holder.count` is stored: an unused sibling array or Task is neither captured nor moved. Each selected path copies, moves or is refused according to its own category. A whole-value use, passing the value, indexing into a collection inside it, or using it through a reference captures the whole binding. See the [field capture example](/functions/defining-functions#closures).

```fa
type Reader = () => Integer
function reader = (input: Array<Integer>): Reader => {
	let snapshot = copy input
	() => snapshot.length -- last use moves snapshot into the environment
}
console.log(reader([1, 2])())
```

An escaping local function creates its environment once when execution reaches its declaration. Its captures are stores at that point; subsequent function handles share the immutable environment. For `let items = [1]; function read = () => items.length; let readers = (read, read)`, `items` moves at the declaration and both tuple items share it. Before the declaration, a function with captures can only be called directly, borrowing initialized captures. Every function-value use there is an error, whether or not it escapes: move the declaration above the use, or call directly. Each execution of the declaration, including another loop iteration, creates its own environment. An escaping function also captures the handles of local functions it calls: a capturing callee must be declared first. Two mutually calling capturing functions cannot both escape; pass their data as parameters or make them methods of one object. See the [closure example](/functions/defining-functions#closures).

Across workers, use an independent copy, a common `Mutex`, or a proven exclusive transfer invalidating the source and all its aliases. An unprotected alias or a value borrowed from `Guard.value` cannot escape its protection. A Task never crosses a worker boundary, including inside another value. See [thread safety](/advanced/concurrency#thread-safety-and-synchronization).

## Diagnostics and fixes

| Code | Message and rule | Fix |
| --- | --- | --- |
| `MEM002` | Value was moved or invalidated; it cannot be used here. A partial move names the exact path and move line; the whole owner is invalid until the field is assigned again. Also: a borrowed resource or a field needed by its destructor cannot transfer ownership. | Use the new owner, copy copyable data before moving, acquire a new resource, or transfer the complete resource. |
| `MEM003` | `T` is unique or owns a unique value and can't be copied; move it instead. An escaping capture cannot own it either. | Move the owner, create a new resource, or keep the closure nonescaping. `copy` cannot fix this. |
| `MEM004` | Copying `T` duplicates a structure; write `copy value` explicitly or stop using the source so it can move. | Write `copy` at the storage site; for a capture, first create a local snapshot and capture its last use. |
| `MEM005` | An error payload owns a unique value; error payloads must be copyable data. | Keep an identifier or a description of the resource instead; keep the unique owner outside the error. Shared synchronization handles are allowed. |

The following deliberately invalid example documents `MEM004`:

```fa
let source = [1, 2]
let duplicate = source -- error: MEM004: write `copy source`
console.log(source, duplicate)
```

A move invalidates aliases too (`MEM002`):

```fa
let task = start do { 1 }
use borrowed = task
let owner = task
console.log(await borrowed catch _error => -1) -- error: MEM002: use `owner`
```

An explicit copy cannot consume a unique value (`MEM003`):

```fa
function invalid = (task: Task<Integer>): Task<Integer> => copy task -- error: MEM003
```

## Complete owners of links

A [link](/advanced/relations#links-without-a-reverse-side) borrows its target; it never owns, recursively copies or destroys that target. The **complete declared owner** contains every target collection and holder below one ordinary record. Its ordinary category still governs moves/copies: unique payloads make it unique (`MEM003`), a borrowed parameter cannot give it away, and keeping two structure owners requires explicit `copy`.

Moving/returning that complete owner transfers its storage and invalidates old aliases. `copy owner` copies each owned payload once and makes every internal link designate the copied target, preserving cycles and shared children. A record containing just one pool and its internal links can also be copied as a whole. Relation fields inside the owner retain their existing remapping rules.

A link-bearing element or Array of holders cannot move/copy out independently, be returned as ordinary data, thrown or captured in an escaping closure. Borrow with `use`, return `T in parameter.path`, or explicitly construct a projection of ordinary data. `copy model.nodes` is refused when its element template needs the enclosing binding or another pool; copy model instead. You can move or copy holders between places within the same owner, including when an Array shifts its elements. The same restrictions apply through a generic function or a structural view: neither can turn a borrowed node into an independent owner. Replacing a linked node writes its outgoing links from the replacement and keeps its element place for incoming links, unlike relation-side preservation.

Target-only reset is permitted only when all its incoming holders die with it. Otherwise replace the complete owner; clearing a visible optional field is not a proof about all possible holders. The [containment rules and diagnostics](/advanced/reference-checking#link-containment) also constrain broad mutation helpers and destructors. A destructor cannot access a pool dying in its same boundary. Error/cancellation cleanup destroys only initialized owned payloads, never follows edges, and performs unique cleanup once.

## References

Local references name their owner and are checked statically. Stored references are [relations](/advanced/relations), with maintained reverse sides, or explicit links contained by a complete growing-collection owner. Moving an owner invalidates its local references, even when the physical address stays the same. No raw pointer or lifetime annotation is needed outside foreign code. See [reference checking](/advanced/reference-checking).

## Summary

`let b = a` preserves ordinary value semantics, moves a structure at its proven last use, transfers a unique owner, or shares a synchronization handle. `copy a` visibly duplicates copyable owned data. `use b = a`, borrowed calls and iteration create no copy.

:::implementation
## Implementation details

### Native and WebAssembly

A move transfers headers/pointers or inline fields and clears the source's ownership/drop state; it never walks or duplicates a collection's payload. Returned fresh aggregates build into their final destination under the existing return ABI. A mutable or escaping collection starts in suitable storage: never construct in a frame and add a hidden deep copy to rescue its escape. Scope, replacement, error and cancellation cleanup agree on which path owns each value.

Union stores select the semantic alternative before erasing its type to a tag and payload. A collection can retain its storage through primitive literal/range widening only when its native item layout remains readable through the widened type. Matching physical layouts alone never authorizes an item conversion or selects a different written alternative. Primitive widening through tuple/object components keeps those same item bytes. The shared type query compares type-pair dependencies with a memoized work stack, including optional alternatives, rather than recursively expanding combinations. Array equality reads each operand with its own item stride and uses a cached pair comparator; recursive comparisons likewise cache a helper before descending into child types. Neither operation converts or copies a collection to compare it.

An owned immutable function environment has a common header containing its release operation, retain operation and reference count. A function-handle copy retains this environment; the last release destroys its payload once. Closures, error adapters, compositions and generic lambda environments use this contract. Native counts are atomic; current single-threaded WebAssembly uses ordinary counts. Borrowed frame environments keep their no-op lifetime operations and cannot escape. This private counting does not make resources or other unique values copyable.

Constructing a nested closure also observes the data inputs it needs from the enclosing environment. Those inputs are recorded on the active enclosing function before its capture list, creation types and source paths are published, even when its body never names them directly. Named callee dependencies remain separate handles.

Recursive calls reuse the running environment. Taking the function's own handle retains that environment instead of copying its captures. The compile-time VM resolves a self handle through the callable already held by its invocation; a partial application's bound arguments are not part of the full function's self handle.

The analysis records callable-slot sources when each read executes. Assignments replace those sources and control-flow joins combine them; looking up a mutable slot's initializer later cannot recover an earlier handle's captures. A nonescaping closure borrowing a callable slot observes its value at the call, after argument evaluation and before call effects, rather than the value present at its declaration. Capture-storage checks must use these same facts as indirect-call observations, on every target. Creation-time type snapshots are published in `Analysis::capture_types`, preserving narrowing for every consumer and analysis clone. They describe the logical captured value; native materialization separately records its physical source layout. Publishing a type snapshot does not authorize a transfer. Escaping function handles propagate escape validation through their captured callable origins. The current unique-capture check conservatively classifies the whole binding; when an unused unique sibling prevents a field-only capture, its diagnostic suggests binding that field to a local first. Field-path capture plans remain the next refinement; the language rule above is unchanged.

`copy` invokes the existing type-specialized copy operation: flat data copies inline; collection headers receive independent storage and every owned item is copied recursively. Shared handles retain their state; unique values have no copy operation. Copies preserve contents rather than spare capacity. Recursive copy and destruction must handle arbitrary finite depth without growing the native call stack; this is a backend obligation of the recursive ownership slices. Linked collections copied as one owner remap links to their copied targets; detached element copies lose links under the relation rules. An explicit copy of an arena-backed view must materialize independent storage before the ancestor arena dies.

Reference aliases also carry ownership-only storage origins in flow: correlated source paths and logical capture inputs. Reseating replaces them; control-flow joins preserve each possibility, and an explicit empty designation stays empty. Read snapshots freeze the current pass and are cleared between key specializations. Move checking observes every possible source of a joined alias. Reference validity and the reference ABI remain separate. A source query crosses capture boundaries individually: a borrowed entry follows its frozen input paths, and an owned entry reads its stored environment. Nested origins remain attached until their own boundary is crossed; independent direct and global observations remain visible. Frozen whole-owner source facts use stable owner and captured-binding identities in analysis clones, without retaining the walker's temporary place arena. Exact paths remain separate and these roots never authorize partial moves. An unfinished source body stays pending; a missing observation is not a proof that its owner can move. Known read-only borrowed calls, callbacks and omitted defaults also provide positive source-use witnesses for capture `MEM004`; replacements and unresolved conditional dependencies require the complete lifetime query. These witnesses never authorize a transfer.

Named callable dependencies are published separately from data captures, including calls in defaults and transport through enclosing environments. Capture state is `Empty`, `Capturing` or `Pending`. A per-key snapshot retains edges to an enclosing function whose body is unfinished; its consumer must settle that state before selecting an environment layout. Complete captureless call chains need no environment. These facts do not authorize capture transfers. Native and VM named declarations bind one environment for each execution; direct calls borrow it and handle reads retain it. Self, sibling, nested, started and parallel consumers transport that binding rather than rebuilding its data. The current copy-first prerequisite preserves source owners and conservative source observations until checked capture transfers land. Named reads also record lexical declaration readiness separately for borrowed and owned entries. The ownership CFG retains lexical block entries, including fresh loop-body activations; known wrapper calls, borrowed callbacks and omitted defaults propagate early, ready or mixed states. Unfinished contexts remain pending. Invocation entries distinguish a lambda body from evaluation of a default even when they share a syntax node. Conditional source dependencies retain one declaration guard per edge; nested conditions stay graph paths rather than expanded caller combinations.

Default argument binding distinguishes an existing borrowed place from a fresh temporary on each selected result path. Native binding preserves the place's address, so mutation reaches its owner, and guards cleanup of the temporary before evaluating any later default. JavaScript similarly records whether the selected branch constructed its value and finalizes only that branch's temporary. Blocks, conditionals and coalescing do not themselves authorize a copy; locals created by the argument must leave their inner scope under the ordinary move rules.

### Strings

Strings have no reference count. At most 23 bytes are inline and copy like a scalar. Literals use static storage and are never copied merely to store their value. For a longer string, an assignment or store is a move at last use; otherwise it is a view if the destination is immutable, does not outlive the source, and no mutation of the source occurs while the view lives. Escape and lifetime facts must prove those conditions. Otherwise it makes one allocation and one block copy of the bytes. `use` rules that copy out. A mutable string first obtains writable storage before changing static or borrowed bytes. See [strings](/values/strings#implementation-details).

### JavaScript

A move is a JavaScript assignment; source invalidation is checked at compile time. Explicit `copy` uses the existing generated typed copier (or the runtime structural copy helper where required): primitive arrays use `slice`, nested owners copy recursively, synchronization handles retain identity. An ordinary object copy preserves value semantics; a Fa string is a JavaScript string. The engine manages ordinary object storage; generated cleanup still handles resources, tasks and counted handles deterministically.

Copy/read/write/drop operations and binary worker codecs are generated at build time from analyzed types; generic consumers receive the required concrete operations. Execution does not compile functions, interpret Fa schemas, or serialize protected data as JSON. Recursive copies use a worklist rather than the JavaScript call stack.

### Compile-time evaluation

Demand-time checking and finished-file evaluation share revisioned function code. Changes to retention/capture facts, final move decisions and source invalidation advance a session-wide revision, including for callers in other files. Both hosts reuse only current entries. Purity traversal keeps its active recursion guards separately from cached answers. Reachable environment and invocation plans must be settled before evaluation; invalidating code cannot repair a value already produced under an incorrect plan.

### Integration status

The `copy` syntax, category queries, concrete unique-copy and `MEM005` payload checks, and whole-value `MEM004` store checks are available. Detached argument stores include fields and declared reference results; their fixes preserve argument evaluation and detached-call behavior. Existing supported unique-field moves remain available. Ownership enforcement is still being integrated: checked capture transfers and early calls, generic instantiation conditions, additional structure-field/recursive moves, construction stores and binary transport remain subsequent work. The rules above specify the complete language; current acceptance of a prohibited copy is not permission to rely on it.
### Complete-owner link operations

Native copies of flat linked payloads copy runs once per page/segment, rebuild headers/directories and preserve ordinal bytes without edge fixups; owning payloads and ordinary relations still use specialized operations. Moves transfer headers. Stable fixed-stride union slots copy/drop only their active payload. Never use recursive owning-child copy/drop for links or assume TLS root builders/near-far addressing.

JavaScript complete-owner copies allocate all destination nodes first, then fill owned data and edges through one temporary old-to-new map; reads and stores need no map. Whole-element replacement mutates the target object rather than swapping its collection entry. Erased generic operations receive the analyzed storage/copy contract. Interpreter owner IDs/ordinals become symbolic graph constants, not host pointers. These link operations land after the shared ownership enforcement; unsupported detached copies remain rejected throughout.

:::
