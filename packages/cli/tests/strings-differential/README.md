# Generated string differential tests

`cargo test -p fa --test programs generated_string_differential -- --nocapture`
runs the generator and the shared bounded CLI runner for Bun, Node and the real
web sandbox. With LLVM enabled it also runs native, wasm and native-freestanding
at O0 and O3 with allocation leak checks. `FA_PROGRAMS=semantic-00` selects a
generated shard. Failed sources and output remain under `target/tmp/strings-differential-PID/`.

The standard-library Python generator runs direct JavaScript expressions under
Bun, independently of Fa analysis, interpretation and code generation. It emits
exact UTF-16 unit sequences, not lossy console strings or hashes. The fixed seed
is `0xFA1616`. Every subject meets an empty needle, both surrogate halves, and
one seeded ordinary needle; this samples the subject/needle product. Ten fixed
pairs add the decided edges a sampled needle may miss (non-overlapping matches,
empty texts and separators, a filler cut inside a pair, over-long searches). Bounds
cover negative, open, inclusive, exclusive, reversed, empty and invalid ranges.
Fa's declared deviations (negative indexing, optional searches, literal dollar
replacements, checked bounds) are modeled explicitly. Decoder error positions
come independently from Python's strict UTF-8 decoder; lossy results come from
Bun's WHATWG TextDecoder with BOM retention.

The 70 subjects cover empty/ASCII, two/three/four-byte scalars, lone high/low
surrogates in several positions, cut and rejoined pairs, combining marks, ZWJ
emoji, final sigma, sharp s, Turkish i, trimming, BOM, NUL, inline/frame and
64-unit breadcrumb boundaries, and ASCII/Unicode payloads beyond 4 KiB. The
semantic probes cover every specified pure string operation and byte reads,
slices, encodings, decoding and interpolation, at compile time and runtime. Runtime probes rotate ten storage forms,
sampling subject/needle/form triples; separate storage probes use every subject
in every form.
Additional byte probes cover all 256 byte values, constructors, copies, mutable
views and reseating, equality and inequality, `isEmpty` and `toArray`; 156 decoder cases cover malformed classes at vector edges.

Every subject also passes through owner/view/split-part/object/array/pool,
recursive union/packed array, frame and reserved append paths, map keys, set
items, sorting, and an await boundary. These storage probes run at runtime:
Pool.add is an existing unsupported compile-time operation; the semantic probes
still exercise each subject as a compile-time static. Physical storage choices
are compiler optimizations, so this corpus alone does not prove which layout
was selected; the existing LLVM IR layout checks remain necessary.

JSON has independent runtime, compile-time parse, and compile-time round-trip
shards. Freestanding JSON retains its documented AN004 restriction. The separate
compile-time round-trip shard deliberately exposes the known missing serializer
without preventing other shards from running. No disagreement is ratcheted,
ignored, or treated as success. A review branch can therefore have a failing
regression until the implementation is repaired.

A case with a literal beyond 2000 characters gets a shard of its own: its
compile-time probe takes several seconds in a debug build, and every program
must finish within the runner's 20-second budget.
