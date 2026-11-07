# Pending documentation notes

The angle-bracket rewrite has arrived. Its pending notes were applied to the rewritten pages on 2026-11-06; their history remains in Git. Add new notes here only when their implementation or specification still needs resolution.

## Metaprogramming evaluation depth

Analyzer review finding 42: audit the undocumented “nests more than 3178 calls” limit before documenting a fixed evaluation limit. Assigned to the analyzer follow-up with the remaining documented rules.

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
