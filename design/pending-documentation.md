# Pending documentation notes

The angle-bracket rewrite has arrived. Its pending notes were applied to the rewritten pages on 2026-11-06; their history remains in Git. Add new notes here only when their implementation or specification still needs resolution.

## Native root-region proof

The 2026-11-16 aggregate work also admits recursive object child arrays whose
structural writes are confined to independent builder locals, while finite
non-owning payload fields may change. Unknown mutating calls, aliases and unproven
defaults/spreads retain ordinary ownership. Array/memory implementation notes now
describe the exact region header/buffer and independent root-copy path; retain
that distinction when reconciling the broader union/array notes below.

When updating the memory-management implementation notes, describe the closed proof in `fa_analyzer::regions` and `design/decisions.md`: whole-module and per-unit compilation require identical facts from every scanned module. Missing or opaque contributions reject the optimization. Child arrays in a recursive union belong to its root region, ordinary arrays own independent roots, and copying a subtree out never shares its owner's lifetime. Generic/unknown payloads, resources/tasks, escaping references, mutations and foreign entry points are conservatively excluded. The backend retains target layout guards; the proof alone does not enable every array-bearing shape. The single shared address band for compressed links is unchanged.

## `documentation/values/strings.md`, `:::implementation` (frame buffers)

Clarify the 64-byte frame budget: an immutable runtime value stays inline up to 23 bytes on
64-bit targets (11 on wasm32), then uses its own frame buffer up to 64 bytes. Literals keep
permanent storage. A non-escaping mutable that may exceed inline room starts with a 64-byte
buffer even when initialized by an empty string; a mutable whose writes are all proven small
keeps inline storage. This avoids adding a hidden buffer argument to mutations.

Every declaration owns a distinct buffer. A copied or moved frame-backed value copies into
the destination's buffer, including loop declarations; an ordinary ownership boundary
(containers, a returned concatenation or foreign allocation) first materializes independent
bytes. Larger strings use the heap. An explicit heap reservation above 64 bytes survives
moving its value, and clearing a mutable string retains its reserved capacity. A frame buffer
that survives `await` belongs to the coroutine frame.

The normal String ABI remains three words. Its writable non-owning frame kind uses the low
capacity bit with room 64; heap capacities are even. Appends normalize this bit only for room
checks and preserve it in the header. Copies, destruction and growth distinguish the tag
from inline values, views and owned blocks, so a frame pointer is never freed or moved into
another variable.

Counted inline appends keep their length and tag flags in separate scalar slots, refreshed on the general suffix path. LLVM promotes those slots to loop values and avoids reloading the payload word after a character store. Locals with references or shared mutation keep the general path.

## Native recursive-union payloads

The arrays/strings/union implementation sections should describe the closed root-region
proof published by every module, independent root owners and child arrays whose buffers
share that owner. One-field String and Array payloads can be frozen beside the member
header, with ordinary borrowed ABIs reconstructed on access and independent copies when
kept. Long payloads retain an external owner and one idempotent cleanup. A capacity reader
in any file disables exact-length freezing through a separately cached proof answer.
Whole-module Linux64 code may use marked16-bit member ids and 32-bit child links after
complete type-store bounds; split units, errors, C interfaces and unsupported targets keep
the ordinary ABI. Missing proof contributions, escaping views/references, captures and
node mutations always keep the general representation.

Bounded decimal append fast paths handle up to five digits through an unaligned two-byte
lookup-table store; general values retain normal conversion.

## Split filter fusion and array representation checks

Add to the String/Array implementation notes: a split immediately followed by the pure
nonempty-part lambda can filter during production while keeping raw split capacity and
evaluation order. Unicode/empty separators and other callbacks retain their semantics.
Array copy/assignment only consider compact characters at the String element width.

## Direct recursive fields in native root payloads

The recursive-union implementation notes should also cover direct fields of the same
root. Closed immutable members may store those fields as 32-bit links beside a compact
header; supported scalar fields retain their natural ABI alignment. Ordinary payload
objects are reconstructed on access, including patterns, comparisons, formatting and
JSON. Copying a root or an escaping subtree recreates its children under the destination
owner. No region lifetime is transferred by moving a projection alone.

For payloads containing only root links, a smaller 16-bit layout is selected locally when
signed offsets from the actual node address fit after scaling by eight. Distant links and
constant-member code collisions use the 32-bit layout; both may coexist under one owner.
A reserved header bit records the layout. The reservation requires the complete prepared
tag bound to fit, and typed byte-tag tests require distinct low bytes for every member of
that root. Payloads with scalar fields retain the 32-bit layout, even if 16-bit links would
save bytes, because measured scalar traversal slows down. Split units, unsupported targets,
foreign boundaries, errors and missing root proofs retain their previous representations.

The near allocator reserves the wide size to learn the real address after any refill,
then returns the unused tail to that same active bump before any other allocation or user
code. It preserves root owners and the shared address domain. Relative arithmetic uses
integer addresses, not an inbounds GEP between separate nodes. The String reconstructed
from a frozen payload is a borrowed view, never an immortal literal; retaining it copies
the bytes before the parent region dies.
