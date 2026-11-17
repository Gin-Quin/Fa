# Generated string differential tests

`cargo test -p fa --test programs generated_string_differential -- --nocapture`
runs the regular quick tier. `FA_STRINGS_DIFFERENTIAL_FULL=1` runs the complete
corpus; **the full tier must pass before any string implementation change lands**.
Enable LLVM 18 for validation of every engine. `FA_PROGRAMS=semantic-00` selects
a generated shard. Failed sources and output remain under
`target/tmp/strings-differential-PID/`.

Both tiers use three parallel workers, sharing the program harness's bounded
execution permits. They compare compile-time statics, Bun, Node, the real web
sandbox, and native, wasm and native-freestanding with leak checks (O0 in the
quick tier, O0/O3 in the full tier).
Freestanding JSON retains its documented AN004 restriction. No disagreement is
ratcheted, ignored or treated as success. Output and error logs are unique to
shard, engine and optimization level.
The RSS check starts after exec: a just-spawned child can briefly expose the
runner's inherited LLVM heap, which is not memory allocated by that program.

The independent standard-library Python generator runs direct JavaScript
expressions under Bun, without Fa analysis, interpretation or code generation.
It compares exact UTF-16 units. Seed `0xFA1616` fixes both tiers. The quick tier
selects representatives of every semantic subject class, rotates all ten
runtime storage forms, runs every operation in `probe`, and keeps the fixed
search edges. Dedicated copy probes include large ASCII/Unicode strings and
both inline-capacity thresholds without multiplying expensive semantic probes.
The complete tier retains every subject/needle pair from the review and adds
class-based regressions.

Subjects include empty/ASCII, two/three/four-byte scalars, lone high/low
surrogates in several positions, reversed pairs, combining marks, ZWJ emoji,
final sigma, sharp s, Turkish i, trimming, BOM, NUL, inline/frame and breadcrumb
boundaries, and payloads beyond 4 KiB. Bounds include negative, open, inclusive,
exclusive, reversed, empty and invalid ranges. All pure string operations,
Bytes reads/slices/conversion/decoding, direct and grouped method facts and
generic receivers run at compile time and runtime. Fa's negative indexing,
optional searches, literal dollar replacements and checked bounds are modeled
explicitly. Strict decoder error positions come from Python's UTF-8 decoder;
lossy results come from WHATWG TextDecoder with BOM retention.

Split-storage probes keep the source live while reading ordinary and filtered
parts; native IR pins all three split-view constructors, including arrays in
caller storage. Independent region-slice shards put sliced strings into copied
recursive trees without split helpers that could mask a missing view capability;
the IR contract also pins the region-slice constructor.

Storage probes exercise owners, views, split parts, object fields, arrays,
pools, recursive unions, packed arrays, frame and reserved append paths, map
keys, sets, sorting and await boundaries. Copy probes cross every subject and
reserved-capacity class with local copies, generic calls, fields, repeated
array entries, pools, recursive union fields, map keys and set items, then
mutate the source. These probes run at runtime because Pool.add is an existing
unsupported compile-time operation; semantic probes still use compile-time
statics. The LLVM IR and C allocation tests separately verify physical layout
and absence of allocations.

The shared `json-syntax.fa` fixture also runs as a required generated shard:
all syntax diagnostic classes, UTF-8 failure classes, error precedence and the
1000-container boundary, each at compile time and runtime for String/Bytes.
JSON shards cover runtime, compile-time parsing, all-subject round trips,
String/Bytes union input, and serialization through objects, arrays, maps,
optionals, primitive/object/empty named members, pinned/unpinned enumerations,
flags and recursive records. Required JSON cases include same-name recursive types in separate scopes inside
a declared envelope, and object-union shape/literal discrimination. Required
key-read probes cross the subjects with nested keyed String borrows and sort
their output, avoiding an assumption about field enumeration. Allocation shards compare the exit code and stop
message for impossible repetition, both padding directions and byte buffers.

A literal beyond 2000 characters gets its own shard: debug compile-time
analysis can take several seconds, and every program must finish within the
shared runner's 20-second budget.

The lead explicitly defers static-type field enumeration to
[O34c](../../../../design/todo.md#o34c-static-type-field-enumeration).
`DEFERRED_CASE_CLASSES` in `../programs/string_differential.rs` is the named list:
`field-order-` and `field-enumeration-`. They are generated and each exclusion is
reported with its to-do link, but neither belongs to the required quick/full set.
`FA_STRINGS_DIFFERENTIAL_DEFERRED=1 FA_PROGRAMS=field-` opts into those acceptance
cases; they are expected to fail until the written-origins work lands. No other
class is excluded.
