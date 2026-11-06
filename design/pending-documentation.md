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
