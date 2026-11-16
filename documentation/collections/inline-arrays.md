# Inline arrays

An `InlineArray<Item, capacity>` is an ordered list with room for at most `capacity` items. It allows duplicates and stores its items inside its owner. It never grows beyond its declared capacity or spills into a heap buffer.

Use an inline array for a small, bounded list, such as four neighbours or a short history. Use an [Array](/collections/arrays) when the number of items can grow without a fixed bound. A [CArray](/advanced/foreign-code) has exactly its declared count of initialized items; an inline array starts empty and changes length.

## Construction

The capacity is a nonnegative integer known at compile time. Zero is allowed. Write an array literal whose items fit:

```fa
mutable scores: InlineArray<Integer, 4> = []
mutable names: InlineArray<String, 3> = ["Ada", "Ada"]
let empty: InlineArray<Integer, 0> = []
```

Without an expected inline-array type, a list literal constructs an ordinary `Array`. A literal that exceeds the expected capacity is a compile-time error: remove items or increase the capacity.

Only the first `length` slots contain values. Unused slots are never initialized and their items' constructors and destructors do not run.

The capacity can also be a constant expression or a compile-time parameter:

```fa
function empty = <Item, capacity: UInt32>(): InlineArray<Item, capacity> => []
let values = empty<Integer, 4>()
```

The implementation accepts capacities from 0 through 4,294,967,295. A capacity parameter must be constrained to that range, for example with `UInt32`.

## Reading and writing

`length` is the current number of items, between zero and `capacity`. `capacity` is the declared bound and never changes. Indexing, negative indices, `get`, iteration and item assignment follow the rules of an [Array](/collections/arrays):

```fa
mutable values: InlineArray<Integer, 4> = [10, 20]
console.log(values.length, values.capacity) -- 2 4
console.log(values[0], values[-1]) -- 10 20
console.log(values.get(3)) -- none
values[1] = 30
for value in values { console.log(value) }
```

A proven index needs no check; an index certainly outside the initialized prefix is rejected at compile time. Otherwise indexing throws `IndexError`: use `try`, `catch`, or `get`.

## Adding and removing items

- `push(value)` appends one item and returns nothing.
- `pop()` removes and returns the last item, or `none` when empty.
- `delete(index)` destroys the indexed item and shifts the following items left.
- `take(index)` removes and returns the indexed item, shifting the following items left.
- `clear()` destroys all initialized items and sets the length to zero.

`delete` and `take` use the same index checks and negative indices as indexing. There is no `reserve` or operation that changes the capacity.

### When the list is full

When room is proven by the compiler's [range analysis](/values/numbers#range-analysis), `push` needs no check and throws nothing:

```fa
mutable values: InlineArray<Integer, 2> = []
values.push(7)
values.push(7)
```

A push to a list that is certainly full is a compile-time error. Remove an item or choose a larger capacity. When the compiler cannot prove there is room, `push` throws `CapacityError.InlineFull`; propagate it with `try` or handle it with `catch`. A full-list failure leaves the list unchanged.

```fa
function append = (values: InlineArray<Integer, 4>, value: Integer)
	throws CapacityError mutates values.length => {
	try values.push(value)
}
```

An item's construction may itself throw. The item is published and the length increases only after its construction succeeds. An error destroys the partial item and leaves the existing prefix intact.

The compiler uses its existing length and index facts. A bounded loop alone does not establish the evolving length of an arbitrary list; keep `try` unless those facts prove room for every push.

## References and effects

`push` never moves an existing item, so references to items survive it. Removing items invalidates the places that cease to exist or shift:

| Operation | Effect |
| --- | --- |
| `push` | `mutates self.length` |
| `pop` | `invalidates self[self.length - 1]` |
| `delete(index)`, `take(index)` | `invalidates self[index..]` |
| `clear` | `mutates self` |
| `self[index] = value` | `mutates self[index]` |
| `length`, `capacity`, `get`, iteration | none |

Effects use the values before the call. Popping an empty list invalidates nothing. Writing an item preserves its place and invalidates references into its previous owned contents. Replacing the entire list invalidates all its old item references. Iteration permits item writes but prohibits changing the length or order.

## Ownership and values

An inline array has its items' [ownership category](/advanced/memory-management): ordinary items make an ordinary value, structure items make a structure, and unique items make a unique owner. Inline storage alone does not require `copy`. Copy a structure explicitly when keeping both owners; move unique values instead. Copies visit only the initialized prefix and retain the declared capacity. Removed items are destroyed exactly once, unless `pop` or `take` transfers them to a new owner.

[Type reflection](/types/types-at-runtime) describes an inline array as `Schema.InlineArray { item, capacity }`. `TypeFromSchema` preserves both parts of its type and validates the capacity against the same 0…4,294,967,295 range as a written type.

Reference item types are not supported in this implementation slice. Store owned data instead; bounded lists of links require the separately declared link-field contract.

Equality compares the length and corresponding initialized items in order. Unused slots do not participate. Printing uses the usual array form, such as `[10, 30]`. JSON writes an ordinary JSON array; reading JSON into an inline-array type checks both its items and the capacity and reports `JsonError` if they do not fit.

:::implementation
## Implementation details

### Native and WebAssembly

An inline array is an aggregate containing `capacity` slots of the item's stride and the smallest unsigned length that can represent `capacity`, with normal alignment and padding. Capacities 0 through 255 use UInt8; 256 through 65,535 use UInt16, then UInt32. Capacity zero has no item storage. Fields, locals, parameters, results and collection elements use this same layout; the list has no separately allocated header or item buffer.

A recursive item keeps the owning indirection needed to break a cycle through inline lists; the fixed slot array never expands an infinite inline type.

Aggregate parameters borrow their caller's place through direct calls, function values, generic thunks and awaited calls. A detached task snapshots its arguments into its frame before suspension. Resource items do not change this borrowing convention. Aggregate helpers take addresses and copies receive a destination slot, so a wide capacity does not expand a WebAssembly function's parameters or results. Copy, destruction, hashing and nested equality traverse initialized places without loading the entire capacity into SSA values. Promoting an item type into a shared interface preserves its nominal identity and representation. After a resource's destructor mutates its fields, field cleanup reads the resulting initialized prefix from that same place.

Only `[0, length)` is initialized. Construction, copying, moves, replacement, destruction and error cleanup must preserve that invariant. Unused slots are neither zero-filled nor interpreted as items. Copies construct a new initialized prefix; moves transfer its ownership; destruction drops only that prefix. Hashing visits that same prefix in order and includes its length, consistently with equality. An append constructs its item before increasing length. Removing an item moves the suffix without copying or destroying its transferred owners a second time.

A wide evaluated value can retain a private shallow stack snapshot until its consumers finish. The snapshot is made at the original read, before later expressions can mutate the source. Helpers borrow its address and stores remain memory copies before LLVM scalar replacement. Transferring opaque representation bytes, including padding, does not create another owner or interpret spare slots; an owning `copy` still constructs only the initialized prefix.

An unproven push compares the length to the constant capacity and returns `CapacityError.InlineFull` before publishing an item. A proven push omits that branch. References point directly at item slots, which do not move during append.

### JavaScript

An inline array is one ordinary dense JavaScript array, without a wrapper or separate count. Its `length` is the initialized prefix. Checked pushes compare that length against the capacity constant; proven pushes use ordinary `push`. Copy and cleanup use the item type's existing operations. Erased generic calls carry an item destructor capability (a shared no-op for ordinary data), including through imported interfaces and function values. Cleanup functions that use this capability stay inside its scope, including recursive generic owners. The engine manages the array's own backing storage.

### Compile-time evaluation

The interpreter uses the same initialized-prefix operations, capacity errors and ownership rules. Capacity is part of the analyzed type rather than a second runtime container. Generic calls and function values carry their instantiated value parameters, so capacity reads and pushes use the actual bound. An explicitly supplied `none` remains an argument. Native constant emission independently rejects a prefix longer than the declared capacity before writing any storage.
:::
