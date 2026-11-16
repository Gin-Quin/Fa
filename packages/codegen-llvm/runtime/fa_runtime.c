// The tier 1 runtime of Fa's LLVM backend: printing, strings, stops and
// errors, compiled by clang 18 to LLVM IR (`generate.sh`) and linked into
// every program module, where the optimizer inlines what is used and drops
// the rest (`src/runtime.rs` makes every definition `internal`).
//
// It only depends on the platform layer that `src/runtime.rs` emits as IR
// per target (`fa_write`, `fa_exit`, `fa_alloc`), on the host imports on
// wasm, and on `snprintf`/`strtod` of the C library for the shortest float
// digits on `native`. Only fixed-width scalars and pointers cross the
// boundary with the generated code, so that the IR signatures don't depend
// on the C ABI of the target (a string result is written to an out pointer).
//
// Printing follows the JavaScript runtime (`packages/codegen-js/runtime`):
// values brief section 2.8, D-5, D-6.

#include <stddef.h>
#include <stdint.h>

#define UINT64_MAX_VALUE 0xFFFFFFFFFFFFFFFFull

#define FA_EXPORT
#define FA_COLD __attribute__((cold, noinline))
#define FA_NORETURN __attribute__((noreturn))

// The state of each thread (`for parallel`, workers: the concurrency
// section): the text builder and the free lists of the allocator. The
// freestanding target has no threads: `src/runtime.rs` makes them plain
// globals there. WebAssembly has no threads either.
#if defined(__wasm__)
#define FA_THREAD_LOCAL
#else
#define FA_THREAD_LOCAL _Thread_local __attribute__((tls_model("initial-exec")))
#endif

/// A lock of a few instructions, for the runtime's shared state (the
/// output buffer, the free spans of the allocator): held briefly, never
/// while waiting for anything else.
static void spin_lock(int32_t *lock) {
#if defined(__wasm__)
	// one thread, and a flush after a trap must not need the stack
	(void)lock;
	return;
#endif
	while (__atomic_exchange_n(lock, 1, __ATOMIC_ACQUIRE) != 0) {
		while (__atomic_load_n(lock, __ATOMIC_RELAXED) != 0) {
#if defined(__x86_64__)
			__builtin_ia32_pause();
#endif
		}
	}
}

static void spin_unlock(int32_t *lock) {
#if defined(__wasm__)
	(void)lock;
	return;
#endif
	__atomic_store_n(lock, 0, __ATOMIC_RELEASE);
}

// ── platform layer (src/runtime.rs) ─────────────────────────────────────

void fa_write(int32_t fd, const uint8_t *bytes, size_t length);
FA_NORETURN void fa_exit(int32_t code);
void *fa_alloc(size_t size, size_t align);
void fa_free(void *bytes, size_t size, size_t align);
/// A block of `fa_alloc` of `old` bytes replaced by one of at least `size`
/// bytes holding its first `used` bytes (`fa_heap_grow`); null when the
/// system has no more memory.
void *fa_grow(void *bytes, size_t old, size_t used, size_t size);

#if defined(__wasm__)
#define FA_IMPORT(name) __attribute__((import_module("fa"), import_name(name)))
// Writes the ECMAScript `Number::toString` of the value (of `Math.fround`
// for a Float32 printed as one, `kind` 1; of a Percentage, `kind` 2) into
// `out` (at least 32 bytes) and returns its length.
FA_IMPORT("formatFloat") int32_t fa_host_format_float(double value, int32_t kind, uint8_t *out);
// The UTF-8 bytes of the user arguments, each followed by a NUL byte.
FA_IMPORT("argumentsSize") int32_t fa_host_arguments_size(void);
FA_IMPORT("argumentsCopy") void fa_host_arguments_copy(uint8_t *out);
// A file operation of the host (`node:fs`, the files section): 0 when it
// worked (what was read is pending), 1 when it failed (the text of the
// error is pending); `size` receives the pending bytes' count, which
// `fileTake` copies to `out`.
FA_IMPORT("fileRun")
int32_t fa_host_file_run(int32_t kind, const uint8_t *path, size_t path_length, const uint8_t *bytes,
                         size_t length, int32_t *size);
FA_IMPORT("fileTake") void fa_host_file_take(uint8_t *out);
#else
int snprintf(char *buffer, size_t size, const char *format, ...);
double strtod(const char *text, char **end);
#endif

// ── values ──────────────────────────────────────────────────────────────

/// A string is three words: pointer, byte length, and capacity/flags.
/// SIMPLE means ASCII; STRING_SURROGATES permits WTF-8 lone surrogates.
/// Inline payloads occupy all but the final flag/length byte (23 bytes on
/// 64-bit targets, 11 on wasm32). External literals carry their UTF-16
/// count in the capacity word, tagged STRING_LITERAL. KIND_MASK denotes
/// a borrowed view; low bit STRING_FRAME tags a 64-byte function buffer.
/// Owned blocks have a StringHeader prefix with eager counts and lazy breadcrumbs.
typedef struct {
	const uint8_t *bytes;
	size_t length;
	size_t capacity;
} FaString;

#define SIMPLE ((size_t)1 << (sizeof(size_t) * 8 - 1))
#define STRING_INLINE ((size_t)1 << (sizeof(size_t) * 8 - 3))
#define STRING_SURROGATES ((size_t)1 << (sizeof(size_t) * 8 - 2))
#define STRING_LITERAL ((size_t)1 << (sizeof(size_t) * 8 - 4))
#define STRING_COUNTED_VIEW (STRING_LITERAL | STRING_SURROGATES)
#define STRING_FLAGS (SIMPLE | STRING_SURROGATES)
#define KIND_MASK (~(SIMPLE | STRING_INLINE | STRING_SURROGATES | STRING_LITERAL))
#define STRING_FRAME ((size_t)1)
#define FRAME_STRING_BYTES 64
#define INLINE_STRING_BYTES (sizeof(FaString) - 1)
#define INLINE_LENGTH_SHIFT ((sizeof(size_t) - 1) * 8)

// A closed program specializes this after all constructors/callbacks have
// been lowered. Separate units and standalone runtime users keep support.
FA_EXPORT int32_t fa_virtual_string_slices = 1;

static int span_low_head(const uint8_t *bytes, size_t length) {
	return fa_virtual_string_slices && length >= 2 && bytes[0] >= 0x80 && bytes[0] < 0xC0;
}

static int span_high_tail(const uint8_t *bytes, size_t length) {
	return fa_virtual_string_slices && length >= 3 &&
	       bytes[length - 3] >= 0xF0 && bytes[length - 3] <= 0xF4;
}

static int span_has_edges(const uint8_t *bytes, size_t length) {
	return span_low_head(bytes, length) || span_high_tail(bytes, length);
}

static uint16_t span_low_unit(const uint8_t *bytes) {
	return (uint16_t)(0xDC00 | ((bytes[0] & 15) << 6) | (bytes[1] & 63));
}

static uint16_t span_high_unit(const uint8_t *bytes) {
	uint32_t prefix = ((uint32_t)(bytes[0] & 7) << 12) |
	                  ((uint32_t)(bytes[1] & 63) << 6) | (bytes[2] & 63);
	return (uint16_t)(0xD800 + ((prefix - 0x400) >> 4));
}

static const uint8_t *string_bytes(const FaString *string) {
	return string->capacity & STRING_INLINE ? (const uint8_t *)string : string->bytes;
}

static size_t string_length(const FaString *string) {
	return string->capacity & STRING_INLINE ? (string->capacity >> INLINE_LENGTH_SHIFT) & 31
	                                        : string->length;
}

static void inline_string_tag(FaString *string, size_t length, size_t simple) {
	// Constructors initialize the payload first. Store a complete word so
	// LLVM can forward its encoding bits into subsequent representation tests;
	// a byte store followed by a capacity-word load loses that information.
	size_t payload = string->capacity & (((size_t)1 << INLINE_LENGTH_SHIFT) - 1);
	string->capacity = payload | STRING_INLINE | (simple & STRING_FLAGS) | (length << INLINE_LENGTH_SHIFT);
}

/// Whether a string with this capacity word owns its heap block.
static int string_owned(size_t capacity) {
	size_t kind = capacity & ~(STRING_FLAGS);
	return kind != 0 && kind < KIND_MASK && !(kind & STRING_FRAME);
}

static int string_static(size_t capacity) {
	return !(capacity & STRING_INLINE) &&
	       ((capacity & ~(SIMPLE)) == 0 ||
	        (capacity & (STRING_LITERAL | STRING_SURROGATES)) == STRING_LITERAL);
}

static int string_counted_view(size_t capacity) {
	return !(capacity & STRING_INLINE) &&
	       (capacity & STRING_COUNTED_VIEW) == STRING_COUNTED_VIEW;
}

static int string_view(size_t capacity) {
	return !(capacity & STRING_INLINE) &&
	       (string_counted_view(capacity) || (capacity & ~STRING_FLAGS) == KIND_MASK);
}

/// A content view is a descriptor value, whereas a whole String alias
/// still designates its owner's place. Snapshot only the former when a
/// designation is copied or reseated; no payload is copied or owned here.
FA_EXPORT __attribute__((always_inline)) const FaString *fa_string_reference(
    const FaString *place, FaString *snapshot) {
	if (!place) { return 0; }
	size_t capacity = place->capacity;
	if (string_view(capacity)) {
		*snapshot = *place;
		return snapshot;
	}
	return place;
}

/// A counted view never owns bytes. ASCII keeps the existing compact tag;
/// literals cannot collide with the counted-view kind (they are well formed).
static size_t string_view_capacity(size_t flags, size_t units) {
	return flags & SIMPLE ? SIMPLE | KIND_MASK : STRING_COUNTED_VIEW | (units & KIND_MASK);
}

/// Unicode-only metadata in compact stored strings. ASCII retains exactly
/// its old header and payload; no count field is emitted or read.
FA_EXPORT __attribute__((always_inline)) size_t fa_string_view_count_size(size_t units, size_t flags) {
	if (flags & SIMPLE) { return 0; }
	size_t size = 1;
	while (units >= 128) { units >>= 7; size += 1; }
	return size;
}

FA_EXPORT __attribute__((always_inline)) uint8_t *fa_string_write_view_count(uint8_t *cursor,
                                                                          size_t units, size_t flags) {
	if (flags & SIMPLE) { return cursor; }
	do {
		uint8_t byte = (uint8_t)(units & 127);
		units >>= 7;
		*cursor++ = (uint8_t)(byte | (units ? 128 : 0));
	} while (units);
	return cursor;
}

FA_EXPORT __attribute__((always_inline)) const uint8_t *fa_string_read_packed_view(
    const uint8_t *cursor, size_t length, size_t flags, FaString *out) {
	size_t units = length;
	if (!(flags & SIMPLE)) {
		units = 0;
		for (unsigned shift = 0; shift < sizeof(size_t) * 8; shift += 7) {
			uint8_t byte = *cursor++;
			units |= (size_t)(byte & 127) << shift;
			if (!(byte & 128)) { break; }
		}
	}
	*out = (FaString){cursor, length, string_view_capacity(flags, units)};
	return cursor + length;
}

static __attribute__((always_inline)) inline int string_frame(size_t capacity) {
	return !(capacity & (STRING_INLINE | STRING_LITERAL)) && (capacity & KIND_MASK) == (FRAME_STRING_BYTES | STRING_FRAME);
}

static __attribute__((always_inline)) inline int string_writable(size_t capacity, size_t needed) {
	size_t room = capacity & ~(STRING_FLAGS | STRING_FRAME);
	return room < (KIND_MASK & ~STRING_FRAME) && needed <= room;
}

/// The ASCII flag is computed in machine words, including carriage returns.
static size_t simple_flag(const uint8_t *bytes, size_t length) {
	uint64_t seen = 0;
	size_t index = 0;
	for (; index + 8 <= length; index += 8) {
		uint64_t word;
		__builtin_memcpy(&word, bytes + index, 8);
		seen |= word;
	}
	for (; index < length; index += 1) { seen |= bytes[index]; }
	return (seen & UINT64_C(0x8080808080808080)) == 0 ? SIMPLE : 0;
}

static size_t text_flags(const uint8_t *bytes, size_t length) {
	size_t ascii = simple_flag(bytes, length);
	if (ascii) { return ascii; }
	if (span_has_edges(bytes, length)) { return STRING_SURROGATES; }
	for (size_t i = 0; i + 2 < length; i += 1) {
		if (bytes[i] == 0xED && bytes[i + 1] >= 0xA0) { return STRING_SURROGATES; }
	}
	return 0;
}

/// The strings of one ASCII byte, which a character of a simple string
/// points into instead of allocating.
static const uint8_t ascii_table[128] = {
#define FA_ROW(b) b, b + 1, b + 2, b + 3, b + 4, b + 5, b + 6, b + 7
    FA_ROW(0),  FA_ROW(8),  FA_ROW(16), FA_ROW(24), FA_ROW(32),  FA_ROW(40),  FA_ROW(48),  FA_ROW(56),
    FA_ROW(64), FA_ROW(72), FA_ROW(80), FA_ROW(88), FA_ROW(96), FA_ROW(104), FA_ROW(112), FA_ROW(120),
#undef FA_ROW
};

/// The one-byte string of an ASCII byte (static).
static FaString ascii_string(uint8_t byte) {
	FaString string;
	string.bytes = ascii_table + byte;
	string.length = 1;
	string.capacity = SIMPLE;
	return string;
}

/// Stops the program: no memory left (as `fa_alloc` does).
static FA_NORETURN void out_of_memory(void) {
	static const char message[] = "stop: out of memory\n";
	fa_write(2, (const uint8_t *)message, sizeof(message) - 1);
	fa_exit(101);
}

static void copy_bytes(uint8_t *to, const uint8_t *from, size_t length);
static size_t encode_point(uint8_t *out, uint32_t code);

/// The physical span omits one byte of a low half's WTF-8 encoding. A
/// high half retains three bytes, which must be rewritten when copied.
FA_EXPORT __attribute__((always_inline)) size_t fa_string_encoded_length(
    const uint8_t *bytes, size_t length, size_t capacity) {
	return length + (!(capacity & SIMPLE) && span_low_head(bytes, length));
}

/// Materialize into an owning/output destination, never into borrowed
/// storage. The middle is copied in one block; only cut ends are encoded.
FA_EXPORT __attribute__((always_inline)) void fa_string_copy_bytes(
    uint8_t *out, const uint8_t *bytes, size_t length, size_t capacity) {
	if ((capacity & SIMPLE) || !span_has_edges(bytes, length)) {
		copy_bytes(out, bytes, length);
		return;
	}
	int low = span_low_head(bytes, length);
	int high = span_high_tail(bytes, length);
	size_t start = low ? 2 : 0;
	size_t suffix = high ? 3 : 0;
	if (length < start + suffix) { __builtin_trap(); }
	size_t end = length - suffix;
	uint16_t low_unit = low ? span_low_unit(bytes) : 0;
	uint16_t high_unit = high ? span_high_unit(bytes + end) : 0;
	size_t written = low ? 3 : 0;
	if (end > start) { __builtin_memmove(out + written, bytes + start, end - start); }
	if (low) { encode_point(out, low_unit); }
	if (high) { encode_point(out + written + end - start, high_unit); }
}

/// Compiler block copies retain their memcpy form on canonical/ASCII paths.
FA_EXPORT __attribute__((always_inline)) void fa_string_copy_block(
    uint8_t *out, const uint8_t *bytes, size_t length, size_t capacity) {
	if (!(capacity & SIMPLE) && span_has_edges(bytes, length)) {
		fa_string_copy_bytes(out, bytes, length, capacity);
	} else { __builtin_memcpy(out, bytes, length); }
}

/// A borrowed reader can visit at most three canonical pieces. The tiny
/// synthetic halves live only for the callback; the middle is never copied.
static __attribute__((noinline)) void string_span_parts(const uint8_t *bytes, size_t length,
    void (*read)(const uint8_t *, size_t)) {
	int low = span_low_head(bytes, length);
	int high = span_high_tail(bytes, length);
	size_t start = low ? 2 : 0;
	size_t end = length - (high ? 3 : 0);
	uint8_t edge[3];
	if (low) { encode_point(edge, span_low_unit(bytes)); read(edge, 3); }
	read(bytes + start, end - start);
	if (high) { encode_point(edge, span_high_unit(bytes + end)); read(edge, 3); }
}

/// A block of `old` bytes replaced by one of `size` bytes holding its first
/// `used` bytes: a large block by `fa_grow` (which moves the pages of a
/// mapped block), a small one by a new block and a copy, as before.
static __attribute__((always_inline)) inline void *grow_block_at(void *bytes, size_t old, size_t offset,
                                                               size_t used, size_t size, size_t align) {
	if (old < ((size_t)1 << 20)) {
		uint8_t *grown = fa_alloc(size, align);
		copy_bytes(grown + offset, (const uint8_t *)bytes + offset, used);
		fa_free(bytes, old, align);
		return grown;
	}
	void *grown = fa_grow(bytes, old, offset + used, size);
	if (grown == 0) { out_of_memory(); }
	return grown;
}

static __attribute__((always_inline)) inline void *grow_block(void *bytes, size_t old, size_t used, size_t size, size_t align) {
	return grow_block_at(bytes, old, 0, used, size, align);
}

static void copy_bytes(uint8_t *to, const uint8_t *from, size_t length) {
	// Up to 32 bytes: two bounded overlapping loads and stores, without
	// a library call. Load both ends before writing either of them.
	if (length <= 32) {
		if (length >= 16) {
			uint8_t first[16];
			uint8_t last[16];
			__builtin_memcpy(first, from, 16);
			__builtin_memcpy(last, from + length - 16, 16);
			__builtin_memcpy(to, first, 16);
			__builtin_memcpy(to + length - 16, last, 16);
		} else if (length >= 8) {
			uint64_t first;
			uint64_t last;
			__builtin_memcpy(&first, from, 8);
			__builtin_memcpy(&last, from + length - 8, 8);
			__builtin_memcpy(to, &first, 8);
			__builtin_memcpy(to + length - 8, &last, 8);
		} else if (length >= 4) {
			uint32_t first;
			uint32_t last;
			__builtin_memcpy(&first, from, 4);
			__builtin_memcpy(&last, from + length - 4, 4);
			__builtin_memcpy(to, &first, 4);
			__builtin_memcpy(to + length - 4, &last, 4);
		} else if (length > 0) {
			uint8_t first = from[0];
			uint8_t middle = from[length / 2];
			uint8_t last = from[length - 1];
			to[0] = first;
			to[length / 2] = middle;
			to[length - 1] = last;
		}
		return;
	}
	// `llvm.memcpy`: the C library's on native, `memory.copy` on wasm,
	// `rep movsb` when freestanding.
	__builtin_memcpy(to, from, length);
}

/// Short appends whose byte count is known only at run time should keep
/// the small-copy path, instead of calling libc's memcpy for two bytes.
FA_EXPORT __attribute__((always_inline)) void fa_copy_bytes(uint8_t *to, const uint8_t *from,
                                                          size_t length) {
	copy_bytes(to, from, length);
}

/// Small padding runs need only bounded stores, rather than a libc call.
static void fill_bytes(uint8_t *to, uint8_t byte, size_t length) {
	if (length <= 16) {
		uint64_t word = UINT64_C(0x0101010101010101) * byte;
		if (length >= 8) {
			__builtin_memcpy(to, &word, 8);
			__builtin_memcpy(to + length - 8, &word, 8);
		} else if (length >= 4) {
			uint32_t half = (uint32_t)word;
			__builtin_memcpy(to, &half, 4);
			__builtin_memcpy(to + length - 4, &half, 4);
		} else if (length) {
			to[0] = byte;
			to[length / 2] = byte;
			to[length - 1] = byte;
		}
	} else { __builtin_memset(to, byte, length); }
}

static size_t c_length(const char *text) {
	size_t length = 0;
	while (text[length] != 0) {
		length += 1;
	}
	return length;
}

// Unicode owners publish a fixed byte/unit base. Only ASCII appends leave
// it behind, so current units are base units plus the byte-length delta.
// Readers lock only the independently mutable breadcrumbs, never this base.
typedef struct {
	size_t bytes;
	size_t units;
	size_t *breadcrumbs;
	uint32_t conversions;
	int32_t lock;
} StringHeader;

static StringHeader *string_header(const uint8_t *bytes) {
	return (StringHeader *)(void *)bytes - 1;
}

static __attribute__((noinline)) size_t unit_count(const uint8_t *text, size_t length);
static __attribute__((noinline)) size_t string_length_slow(const uint8_t *text, size_t length);
FA_EXPORT __attribute__((always_inline)) size_t fa_string_length(const uint8_t *text, size_t length, size_t capacity);
FA_EXPORT __attribute__((always_inline)) void fa_string_set_unit_count(const uint8_t *bytes, size_t length, size_t capacity, size_t units);
FA_EXPORT void fa_string_from(const uint8_t *bytes, size_t length, size_t capacity, size_t room, FaString *out);

static void string_init_header(StringHeader *header) {
	*header = (StringHeader){SIZE_MAX, 0, 0, 0, 0};
}

/// The prefix is reserved with the block so appending non-ASCII text
/// never breaks an existing reservation. ASCII operations never touch it.
static __attribute__((always_inline)) uint8_t *string_alloc_block(size_t capacity, size_t flags) {
	if (capacity > KIND_MASK - sizeof(StringHeader)) { out_of_memory(); }
	StringHeader *header = fa_alloc(sizeof(StringHeader) + capacity, sizeof(size_t));
	if (!(flags & SIMPLE)) { string_init_header(header); }
	return (uint8_t *)(header + 1);
}

static __attribute__((noinline)) uint8_t *string_alloc_ascii(size_t capacity) {
	return string_alloc_block(capacity, SIMPLE);
}

static __attribute__((noinline)) uint8_t *string_alloc_unicode(size_t capacity) {
	return string_alloc_block(capacity, 0);
}

/// Keep allocation out of growing caller loops, while specializing away
/// cache initialization and its live flag register for known ASCII text.
static __attribute__((always_inline)) uint8_t *string_alloc(size_t *capacity, size_t flags) {
	return flags & SIMPLE ? string_alloc_ascii(*capacity) : string_alloc_unicode(*capacity);
}

static void string_clear_crumbs(StringHeader *header) {
	if (header->breadcrumbs) {
		fa_free(header->breadcrumbs, header->breadcrumbs[0] * sizeof(size_t), sizeof(size_t));
		header->breadcrumbs = 0;
	}
	header->conversions = 0;
}

static void string_clear(StringHeader *header) {
	string_clear_crumbs(header);
	header->bytes = SIZE_MAX;
	header->units = 0;
}

static __attribute__((noinline, cold)) void string_free_metadata(StringHeader *header) {
	string_clear(header);
}

FA_EXPORT __attribute__((always_inline)) void fa_string_free(const uint8_t *bytes, size_t capacity) {
	StringHeader *header = string_header(bytes);
	if (!(capacity & SIMPLE)) { string_free_metadata(header); }
	fa_free(header, sizeof(StringHeader) + (capacity & KIND_MASK), sizeof(size_t));
}

static __attribute__((always_inline)) uint8_t *string_grow_block(const uint8_t *bytes, size_t old_capacity, size_t used,
                                 size_t *capacity, size_t flags) {
	size_t units = 0;
	if (!(flags & SIMPLE)) { units = old_capacity & SIMPLE ? used : string_length_slow(bytes, used); }
	StringHeader *header = string_header(bytes);
	if (!(old_capacity & SIMPLE)) { string_free_metadata(header); }
	header = grow_block_at(header, sizeof(StringHeader) + (old_capacity & KIND_MASK), sizeof(StringHeader), used,
	                    sizeof(StringHeader) + *capacity, sizeof(size_t));
	if (!(flags & SIMPLE)) {
		string_init_header(header);
		header->bytes = used;
		header->units = units;
	}
	return (uint8_t *)(header + 1);
}

static void string_changed(FaString *string) {
	if (!string_owned(string->capacity)) { return; }
	StringHeader *header = string_header(string->bytes);
	if (string->capacity & SIMPLE) { string_init_header(header); }
	else { string_clear(header); }
}

// ── output ──────────────────────────────────────────────────────────────

// The standard output is buffered, flushed when full, before anything is
// written to the standard error, and at exit.
#define OUTPUT_CAPACITY 8192
static uint8_t output[OUTPUT_CAPACITY];
static size_t output_length;
/// Several threads may print (`console.log` in a parallel loop).
static int32_t output_lock;

static void flush_locked(void) {
	if (output_length > 0) {
		fa_write(1, output, output_length);
		output_length = 0;
	}
}

#if defined(__wasm__)
// exported, so that the loader flushes the output after a trap
__attribute__((export_name("flush")))
#endif
FA_EXPORT void fa_flush(void) {
	spin_lock(&output_lock);
	flush_locked();
	spin_unlock(&output_lock);
}

FA_EXPORT void fa_out(int32_t fd, const uint8_t *bytes, size_t length) {
	spin_lock(&output_lock);
	if (fd != 1) {
		flush_locked();
		fa_write(fd, bytes, length);
		spin_unlock(&output_lock);
		return;
	}
	if (output_length + length > OUTPUT_CAPACITY) {
		flush_locked();
		if (length > OUTPUT_CAPACITY) {
			fa_write(1, bytes, length);
			spin_unlock(&output_lock);
			return;
		}
	}
	copy_bytes(output + output_length, bytes, length);
	output_length += length;
	spin_unlock(&output_lock);
}

/// Flushes the output and exits.
FA_EXPORT FA_NORETURN void fa_terminate(int32_t code) {
	fa_flush();
	fa_exit(code);
}

static void string_finish_slow(FaString *string);
static size_t string_join_pairs(uint8_t *bytes, size_t length, size_t *flags);
FA_EXPORT __attribute__((always_inline)) void fa_string_finish_known(FaString *string, size_t flags) {
	if (!(flags & SIMPLE)) { string_finish_slow(string); }
}
FA_EXPORT __attribute__((always_inline)) void fa_string_finish(FaString *string) {
	fa_string_finish_known(string, string->capacity);
}
FA_EXPORT void fa_string_write_utf8(uint8_t *out, const uint8_t *bytes, size_t length, size_t capacity);

// ── the text builder ────────────────────────────────────────────────────

// One builder serves `console.log` lines, interpolations and `toString`:
// the generated code evaluates every part before it starts building, so
// building is never nested.
static FA_THREAD_LOCAL uint8_t *builder;
static FA_THREAD_LOCAL size_t builder_length;
static FA_THREAD_LOCAL size_t builder_capacity;
/// The length of a prefix of the built text known to be simple (`SIMPLE`):
/// pieces known to be simple extend it while it covers the whole text, so
/// that taking the text scans only the rest.
static FA_THREAD_LOCAL size_t builder_simple;

static void reserve(size_t extra) {
	if (builder_length + extra <= builder_capacity) {
		return;
	}
	size_t capacity = builder_capacity < 256 ? 256 : builder_capacity * 2;
	while (capacity < builder_length + extra) {
		capacity *= 2;
	}
	uint8_t *bytes = fa_alloc(capacity, 1);
	// A plain block copy: the builder holds at least as many bytes as it
	// needs to grow, and the short-copy branches would make every inlined
	// `reserve` larger.
	if (builder_length) { __builtin_memcpy(bytes, builder, builder_length); }
	if (builder != 0) {
		fa_free(builder, builder_capacity, 1);
	}
	builder = bytes;
	builder_capacity = capacity;
}

FA_EXPORT void fa_b_reset(void) {
	builder_length = 0;
	builder_simple = 0;
}

FA_EXPORT void fa_b_bytes(const uint8_t *bytes, size_t length) {
	reserve(length);
	copy_bytes(builder + builder_length, bytes, length);
	builder_length += length;
}

/// Appends bytes known to be simple (ASCII).
static void append_simple(const uint8_t *bytes, size_t length) {
	int covered = builder_simple == builder_length;
	fa_b_bytes(bytes, length);
	if (covered) {
		builder_simple = builder_length;
	}
}

/// Appends a string, simple when its capacity word says so.
FA_EXPORT void fa_b_string(const uint8_t *bytes, size_t length, size_t capacity) {
	if (capacity & SIMPLE) {
		append_simple(bytes, length);
	} else {
		size_t encoded = fa_string_encoded_length(bytes, length, capacity);
		reserve(encoded);
		fa_string_copy_bytes(builder + builder_length, bytes, length, capacity);
		builder_length += encoded;
	}
}

static size_t simple_flag(const uint8_t *bytes, size_t length);

/// The `SIMPLE` flag of the built text: only its unknown part is scanned.
static size_t builder_flag(void) {
	return builder_simple == builder_length
	           ? SIMPLE
	           : simple_flag(builder + builder_simple, builder_length - builder_simple);
}

static void append_c(const char *text) {
	fa_b_bytes((const uint8_t *)text, c_length(text));
}

static void append_byte(uint8_t byte) {
	reserve(1);
	builder[builder_length] = byte;
	builder_length += 1;
}

/// Appends a new line and writes the line to `fd` (1: stdout, 2: stderr).
FA_EXPORT void fa_b_line(int32_t fd) {
	int ascii = builder_simple == builder_length;
	append_byte('\n');
	if (!ascii) {
		// Interpolations can put two individually lone halves next to each
		// other. Normalize their join before encoding the line for output.
		size_t flags;
		builder_length = string_join_pairs(builder, builder_length, &flags);
		fa_string_write_utf8(builder, builder, builder_length, flags);
	}
	fa_out(fd, builder, builder_length);
	builder_length = 0;
	builder_simple = 0;
}

static FaString owned_copy_with(const uint8_t *bytes, size_t length, size_t simple);

static uint32_t size_class(size_t size, size_t *block);

/// The size of the block that `fa_heap_alloc` gives for `size` bytes: a
/// heap string's capacity, the room it has anyway, so that it grows less
/// often (larger sizes are not rounded).
static __attribute__((always_inline)) inline size_t string_block(size_t size) {
	if (size <= 128) {
		return size <= 8 ? 8 : (size + 7) & ~(size_t)7;
	}
	if (size > 8192) {
		return (size + 1) & ~(size_t)1;
	}
	size_t block;
	size_class(size, &block);
	return block;
}

/// A new string owning a copy of `length` bytes (the empty string and the
/// ASCII characters are static).
static FaString owned_copy(const uint8_t *bytes, size_t length) {
	return owned_copy_with(bytes, length, 0);
}

/// `owned_copy` of bytes known to be simple (`SIMPLE`), or to be scanned (0).
static FaString owned_copy_with(const uint8_t *bytes, size_t length, size_t simple) {
	size_t encoded = fa_string_encoded_length(bytes, length, simple);
	if (encoded >= KIND_MASK) { out_of_memory(); }
	FaString string;
	if (length == 0) {
		string.bytes = (const uint8_t *)"";
		string.length = 0;
		string.capacity = SIMPLE;
		return string;
	}
	if (length == 1 && bytes[0] < 0x80) {
		return ascii_string(bytes[0]);
	}
	if (encoded <= INLINE_STRING_BYTES) {
		FaString small = {0};
		fa_string_copy_bytes((uint8_t *)&small, bytes, length, simple);
		inline_string_tag(&small, encoded, (simple & SIMPLE) ? SIMPLE : text_flags((uint8_t *)&small, encoded));
		return small;
	}
	size_t block = string_block(encoded);
	uint8_t *copy = string_alloc(&block, simple);
	fa_string_copy_bytes(copy, bytes, length, simple);
	string.bytes = copy;
	string.length = encoded;
	string.capacity = block | ((simple & SIMPLE) ? SIMPLE : text_flags(copy, encoded));
	if (!(string.capacity & SIMPLE)) {
		fa_string_set_unit_count(copy, encoded, string.capacity, unit_count(copy, encoded));
	}
	return string;
}

/// Materialize a borrowed string, including short character/slice views.
FA_EXPORT void fa_string_own(const uint8_t *bytes, size_t length, size_t capacity, FaString *out) {
	*out = owned_copy_with(bytes, length, capacity & STRING_FLAGS);
}

/// The built text as a new string.
FA_EXPORT void fa_b_take(FaString *out) {
	size_t flags = builder_flag();
	*out = owned_copy_with(builder, builder_length, flags);
	fa_string_finish_known(out, flags);
	builder_length = 0;
	builder_simple = 0;
}

// ── integers ────────────────────────────────────────────────────────────

static __attribute__((always_inline)) size_t decimal_u64(uint64_t value, int negative, uint8_t digits[24]) {
	// All decimal pairs, indexed by a remainder modulo 100. This conversion
	// applies to every 64-bit integer; the table contains no program data.
	static const uint8_t pairs[] =
	    "0001020304050607080910111213141516171819"
	    "2021222324252627282930313233343536373839"
	    "4041424344454647484950515253545556575859"
	    "6061626364656667686970717273747576777879"
	    "8081828384858687888990919293949596979899";
	size_t position = 24;
	while (value > UINT32_MAX) {
		uint64_t pair = value % 100;
		value /= 100;
		position -= 2;
		__builtin_memcpy(digits + position, pairs + pair * 2, 2);
	}
	// Once the remainder fits, narrower division avoids a 128-bit multiply
	// on 64-bit targets (and an i64 division on wasm).
	uint32_t remaining = (uint32_t)value;
	while (remaining >= 10000) {
		uint32_t group = remaining % 10000;
		remaining /= 10000;
		position -= 4;
		__builtin_memcpy(digits + position, pairs + (group / 100) * 2, 2);
		__builtin_memcpy(digits + position + 2, pairs + (group % 100) * 2, 2);
	}
	if (remaining >= 100) {
		uint32_t pair = remaining % 100;
		remaining /= 100;
		position -= 2;
		__builtin_memcpy(digits + position, pairs + pair * 2, 2);
	}
	if (remaining >= 10) {
		position -= 2;
		__builtin_memcpy(digits + position, pairs + remaining * 2, 2);
	} else { digits[--position] = (uint8_t)('0' + remaining); }
	if (negative) {
		position -= 1;
		digits[position] = '-';
	}
	return position;
}

/// Decimal parts of an interpolation live in the caller's 24-byte scratch
/// buffer. Return the offset of the first digit, without packing a String.
FA_EXPORT size_t fa_decimal_i64(int64_t value, uint8_t *digits) {
	return decimal_u64(value < 0 ? 0 - (uint64_t)value : (uint64_t)value, value < 0, digits);
}

FA_EXPORT size_t fa_decimal_u64(uint64_t value, uint8_t *digits) {
	return decimal_u64(value, 0, digits);
}

static void append_u64(uint64_t value, int negative) {
	uint8_t digits[24];
	size_t position = decimal_u64(value, negative, digits);
	append_simple(digits + position, sizeof digits - position);
}

static void string_u64(uint64_t value, int negative, FaString *out) {
	uint8_t digits[24];
	size_t position = decimal_u64(value, negative, digits);
	size_t length = sizeof digits - position;
#if UINTPTR_MAX > UINT32_MAX
	// At most 20 decimal digits and one sign, all within the native SSO.
	FaString small = {0};
	copy_bytes((uint8_t *)&small, digits + position, length);
	inline_string_tag(&small, length, SIMPLE);
	*out = small;
#else
	*out = owned_copy_with(digits + position, length, SIMPLE);
#endif
}

FA_EXPORT void fa_string_i64(int64_t value, FaString *out) {
	string_u64(value < 0 ? 0 - (uint64_t)value : (uint64_t)value, value < 0, out);
}

FA_EXPORT void fa_string_u64(uint64_t value, FaString *out) {
	string_u64(value, 0, out);
}

/// Divides the 128-bit number in `limbs` (most significant first) by `divisor`
/// in place, with 64-bit operations only (no `__udivti3`), and returns the remainder.
static uint32_t divide_limbs(uint32_t limbs[4], uint32_t divisor) {
	uint64_t remainder = 0;
	for (int index = 0; index < 4; index += 1) {
		uint64_t current = (remainder << 32) | limbs[index];
		limbs[index] = (uint32_t)(current / divisor);
		remainder = current % divisor;
	}
	return (uint32_t)remainder;
}

static void append_u128(unsigned __int128 value, int negative) {
	uint64_t high = (uint64_t)(value >> 64);
	uint64_t low = (uint64_t)value;
	if (high == 0) {
		append_u64(low, negative);
		return;
	}
	uint32_t limbs[4] = {(uint32_t)(high >> 32), (uint32_t)high, (uint32_t)(low >> 32),
	                     (uint32_t)low};
	char digits[48];
	size_t position = sizeof digits;
	for (;;) {
		uint32_t chunk = divide_limbs(limbs, 1000000000);
		int rest = (limbs[0] | limbs[1] | limbs[2] | limbs[3]) != 0;
		for (int digit = 0; digit < 9 && (rest || chunk != 0); digit += 1) {
			position -= 1;
			digits[position] = (char)('0' + (int)(chunk % 10));
			chunk /= 10;
		}
		if (!rest) {
			break;
		}
	}
	if (negative) {
		position -= 1;
		digits[position] = '-';
	}
	append_simple((const uint8_t *)digits + position, sizeof digits - position);
}

FA_EXPORT void fa_b_i64(int64_t value) {
	// `0 - Int64.MIN` wraps to itself, which is the right magnitude unsigned.
	append_u64(value < 0 ? 0 - (uint64_t)value : (uint64_t)value, value < 0);
}

FA_EXPORT void fa_b_u64(uint64_t value) {
	append_u64(value, 0);
}

static unsigned __int128 join(uint64_t low, uint64_t high) {
	return ((unsigned __int128)high << 64) | low;
}

FA_EXPORT void fa_b_i128(uint64_t low, uint64_t high) {
	unsigned __int128 value = join(low, high);
	int negative = (int64_t)high < 0;
	append_u128(negative ? 0 - value : value, negative);
}

FA_EXPORT void fa_b_u128(uint64_t low, uint64_t high) {
	append_u128(join(low, high), 0);
}

FA_EXPORT void fa_b_bool(int32_t value) {
	append_simple((const uint8_t *)(value ? "true" : "false"), value ? 4 : 5);
}

// ── big integers (`BigInt`, numbers page, `src/lower/big_integers.rs`) ──

/// A `BigInt` is one 64-bit word (`i64` in the generated code). An odd word
/// is a small value, inline: the value is the word shifted right by one
/// (arithmetic), from -2^62 to 2^62 - 1, and needs no allocation. An even
/// word is the address of a block holding the magnitude in 32-bit limbs,
/// least significant first, and the sign. The form is canonical: a value
/// that fits inline is never in a block, so that two equal values have the
/// same form. The word 0 is no value (a moved-from local): dropping it does
/// nothing. A block of capacity 0 is a constant of the program (a literal),
/// never freed. Every operation borrows its operands and returns a value
/// the caller owns. Limbs of 32 bits keep every product and division in 64
/// bits (wasm32 and the freestanding target have no 128-bit routines).
typedef struct {
	uint32_t length;
	uint32_t capacity;
	uint32_t negative;
	uint32_t padding;
	uint32_t limbs[];
} FaBig;

#define BIG_HEADER 16
#define BIG_SMALL_MAX (((int64_t)1 << 62) - 1)
#define BIG_SMALL_MIN (-((int64_t)1 << 62))

/// A read-only view of a value: its magnitude and sign (an inline value's
/// limbs are copied into `small`, which the view must not outlive).
typedef struct {
	const uint32_t *limbs;
	uint32_t length;
	uint32_t negative;
	uint32_t small[2];
} BigView;

static int64_t big_inline(int64_t value) {
	return (int64_t)(((uint64_t)value << 1) | 1);
}

static FaBig *big_block(int64_t word) {
	return (FaBig *)(uintptr_t)(uint64_t)word;
}

static void big_view(int64_t word, BigView *view) {
	if (word & 1) {
		int64_t value = word >> 1;
		uint64_t magnitude = value < 0 ? 0 - (uint64_t)value : (uint64_t)value;
		view->small[0] = (uint32_t)magnitude;
		view->small[1] = (uint32_t)(magnitude >> 32);
		view->length = magnitude == 0 ? 0 : (magnitude >> 32) != 0 ? 2 : 1;
		view->negative = value < 0;
		view->limbs = view->small;
	} else {
		FaBig *block = big_block(word);
		view->limbs = block->limbs;
		view->length = block->length;
		view->negative = block->negative;
	}
}

static FaBig *big_allocate(uint32_t capacity) {
	if (capacity == 0) {
		capacity = 1;
	}
	FaBig *block = fa_alloc(BIG_HEADER + (size_t)capacity * 4, 8);
	block->length = 0;
	block->capacity = capacity;
	block->negative = 0;
	block->padding = 0;
	for (uint32_t index = 0; index < capacity; index += 1) {
		block->limbs[index] = 0;
	}
	return block;
}

static void big_free(FaBig *block) {
	if (block->capacity != 0) {
		fa_free(block, BIG_HEADER + (size_t)block->capacity * 4, 8);
	}
}

/// The canonical value of a block whose limbs are computed: trimmed, inline
/// (and the block freed) when it fits.
static int64_t big_finish(FaBig *block, uint32_t length, uint32_t negative) {
	while (length > 0 && block->limbs[length - 1] == 0) {
		length -= 1;
	}
	if (length <= 2) {
		uint64_t magnitude = length == 0 ? 0
		                     : length == 1
		                         ? block->limbs[0]
		                         : ((uint64_t)block->limbs[1] << 32) | block->limbs[0];
		if (magnitude <= (uint64_t)BIG_SMALL_MAX || (negative && magnitude == (uint64_t)1 << 62)) {
			big_free(block);
			int64_t value = negative ? (int64_t)(0 - magnitude) : (int64_t)magnitude;
			return big_inline(value);
		}
	}
	block->length = length;
	block->negative = negative;
	return (int64_t)(uintptr_t)block;
}

/// A value of a magnitude given as 64 bits and a sign.
static int64_t big_from_magnitude(uint64_t magnitude, uint32_t negative) {
	if (magnitude <= (uint64_t)BIG_SMALL_MAX) {
		return big_inline(negative ? -(int64_t)magnitude : (int64_t)magnitude);
	}
	FaBig *block = big_allocate(2);
	block->limbs[0] = (uint32_t)magnitude;
	block->limbs[1] = (uint32_t)(magnitude >> 32);
	return big_finish(block, 2, negative);
}

FA_EXPORT int64_t fa_bigint_from_i64(int64_t value) {
	if (value >= BIG_SMALL_MIN && value <= BIG_SMALL_MAX) {
		return big_inline(value);
	}
	return big_from_magnitude(value < 0 ? 0 - (uint64_t)value : (uint64_t)value, value < 0);
}

FA_EXPORT int64_t fa_bigint_from_u64(uint64_t value) {
	return big_from_magnitude(value, 0);
}

/// A 128-bit integer (two's complement when `is_signed`), as two halves.
FA_EXPORT int64_t fa_bigint_from_128(uint64_t low, uint64_t high, int32_t is_signed) {
	uint32_t negative = is_signed && (int64_t)high < 0;
	if (negative) {
		// the magnitude: 0 - value, on two words
		low = ~low + 1;
		high = ~high + (low == 0);
	}
	if (high == 0) {
		return big_from_magnitude(low, negative);
	}
	FaBig *block = big_allocate(4);
	block->limbs[0] = (uint32_t)low;
	block->limbs[1] = (uint32_t)(low >> 32);
	block->limbs[2] = (uint32_t)high;
	block->limbs[3] = (uint32_t)(high >> 32);
	return big_finish(block, 4, negative);
}

FA_EXPORT void fa_bigint_drop(int64_t word) {
	if (word != 0 && !(word & 1)) {
		big_free(big_block(word));
	}
}

FA_EXPORT int64_t fa_bigint_copy(int64_t word) {
	if (word == 0 || (word & 1) || big_block(word)->capacity == 0) {
		return word;
	}
	FaBig *from = big_block(word);
	FaBig *to = big_allocate(from->length);
	for (uint32_t index = 0; index < from->length; index += 1) {
		to->limbs[index] = from->limbs[index];
	}
	to->length = from->length;
	to->negative = from->negative;
	return (int64_t)(uintptr_t)to;
}

static int big_compare_magnitudes(const uint32_t *a, uint32_t a_length, const uint32_t *b,
                                  uint32_t b_length) {
	if (a_length != b_length) {
		return a_length < b_length ? -1 : 1;
	}
	for (uint32_t index = a_length; index > 0; index -= 1) {
		if (a[index - 1] != b[index - 1]) {
			return a[index - 1] < b[index - 1] ? -1 : 1;
		}
	}
	return 0;
}

/// -1, 0 or 1 as `a` is less than, equal to or greater than `b`.
FA_EXPORT int32_t fa_bigint_compare(int64_t a, int64_t b) {
	if ((a & 1) && (b & 1)) {
		return a < b ? -1 : a > b ? 1 : 0;
	}
	BigView x, y;
	big_view(a, &x);
	big_view(b, &y);
	int x_sign = x.length == 0 ? 0 : x.negative ? -1 : 1;
	int y_sign = y.length == 0 ? 0 : y.negative ? -1 : 1;
	if (x_sign != y_sign) {
		return x_sign < y_sign ? -1 : 1;
	}
	int order = big_compare_magnitudes(x.limbs, x.length, y.limbs, y.length);
	return x_sign < 0 ? -order : order;
}

FA_EXPORT int32_t fa_bigint_equal(int64_t a, int64_t b) {
	if ((a & 1) || (b & 1)) {
		return a == b;
	}
	return fa_bigint_compare(a, b) == 0;
}

/// -1, 0 or 1: the sign of a value.
FA_EXPORT int32_t fa_bigint_sign(int64_t word) {
	if (word & 1) {
		return word < 1 ? -1 : word > 1 ? 1 : 0;
	}
	return big_block(word)->negative ? -1 : 1;
}

/// `a + b` (`b` negated when `subtract`), from views.
static int64_t big_add_views(const BigView *x, const BigView *y, int subtract) {
	uint32_t y_negative = y->negative ^ (subtract && y->length != 0);
	uint32_t length = (x->length > y->length ? x->length : y->length) + 1;
	FaBig *block = big_allocate(length);
	if (x->negative == y_negative || x->length == 0 || y->length == 0) {
		uint64_t carry = 0;
		for (uint32_t index = 0; index < length; index += 1) {
			uint64_t sum = carry;
			if (index < x->length) {
				sum += x->limbs[index];
			}
			if (index < y->length) {
				sum += y->limbs[index];
			}
			block->limbs[index] = (uint32_t)sum;
			carry = sum >> 32;
		}
		return big_finish(block, length, x->length != 0 ? x->negative : y_negative);
	}
	// different signs: the smaller magnitude from the larger
	int order = big_compare_magnitudes(x->limbs, x->length, y->limbs, y->length);
	const BigView *large = order >= 0 ? x : y;
	const BigView *small = order >= 0 ? y : x;
	uint32_t negative = order >= 0 ? x->negative : y_negative;
	int64_t borrow = 0;
	for (uint32_t index = 0; index < large->length; index += 1) {
		int64_t difference = (int64_t)large->limbs[index] - borrow;
		if (index < small->length) {
			difference -= small->limbs[index];
		}
		borrow = difference < 0;
		block->limbs[index] = (uint32_t)(difference + (borrow << 32));
	}
	return big_finish(block, large->length, order == 0 ? 0 : negative);
}

FA_EXPORT int64_t fa_bigint_add(int64_t a, int64_t b) {
	if ((a & 1) && (b & 1)) {
		return fa_bigint_from_i64((a >> 1) + (b >> 1));
	}
	BigView x, y;
	big_view(a, &x);
	big_view(b, &y);
	return big_add_views(&x, &y, 0);
}

FA_EXPORT int64_t fa_bigint_subtract(int64_t a, int64_t b) {
	if ((a & 1) && (b & 1)) {
		return fa_bigint_from_i64((a >> 1) - (b >> 1));
	}
	BigView x, y;
	big_view(a, &x);
	big_view(b, &y);
	return big_add_views(&x, &y, 1);
}

FA_EXPORT int64_t fa_bigint_multiply(int64_t a, int64_t b) {
	if ((a & 1) && (b & 1)) {
		int64_t x = a >> 1, y = b >> 1;
		// both within 31 bits: the product fits in 62 bits
		if (x >= -0x7FFFFFFF && x <= 0x7FFFFFFF && y >= -0x7FFFFFFF && y <= 0x7FFFFFFF) {
			return fa_bigint_from_i64(x * y);
		}
	}
	BigView x, y;
	big_view(a, &x);
	big_view(b, &y);
	if (x.length == 0 || y.length == 0) {
		return big_inline(0);
	}
	uint32_t length = x.length + y.length;
	FaBig *block = big_allocate(length);
	for (uint32_t i = 0; i < x.length; i += 1) {
		uint64_t carry = 0;
		uint64_t limb = x.limbs[i];
		for (uint32_t j = 0; j < y.length; j += 1) {
			uint64_t product = limb * y.limbs[j] + block->limbs[i + j] + carry;
			block->limbs[i + j] = (uint32_t)product;
			carry = product >> 32;
		}
		block->limbs[i + y.length] = (uint32_t)carry;
	}
	return big_finish(block, length, x.negative ^ y.negative);
}

/// The number of leading zero bits of a nonzero limb.
static uint32_t big_leading_zeros(uint32_t limb) {
	return (uint32_t)__builtin_clz(limb);
}

/// The quotient and remainder of magnitudes (Knuth's algorithm D, with
/// 32-bit limbs): `quotient` has room for `a_length - b_length + 1` limbs,
/// `remainder` for `b_length`; `b_length` > 0 and `a` >= `b`.
static void big_divide_magnitudes(const uint32_t *a, uint32_t a_length, const uint32_t *b,
                                  uint32_t b_length, uint32_t *quotient, uint32_t *remainder) {
	if (b_length == 1) {
		uint64_t rest = 0;
		for (uint32_t index = a_length; index > 0; index -= 1) {
			uint64_t current = (rest << 32) | a[index - 1];
			quotient[index - 1] = (uint32_t)(current / b[0]);
			rest = current % b[0];
		}
		remainder[0] = (uint32_t)rest;
		return;
	}
	// normalized copies: the divisor's top bit set
	uint32_t shift = big_leading_zeros(b[b_length - 1]);
	size_t bytes = ((size_t)a_length + 1 + b_length) * 4;
	uint32_t *u = fa_alloc(bytes, 4);
	uint32_t *v = u + a_length + 1;
	for (uint32_t index = b_length - 1; index > 0; index -= 1) {
		v[index] = shift == 0 ? b[index] : (b[index] << shift) | (b[index - 1] >> (32 - shift));
	}
	v[0] = b[0] << shift;
	u[a_length] = shift == 0 ? 0 : a[a_length - 1] >> (32 - shift);
	for (uint32_t index = a_length - 1; index > 0; index -= 1) {
		u[index] = shift == 0 ? a[index] : (a[index] << shift) | (a[index - 1] >> (32 - shift));
	}
	u[0] = a[0] << shift;
	const uint64_t base = (uint64_t)1 << 32;
	for (uint32_t j = a_length - b_length + 1; j > 0; j -= 1) {
		uint32_t at = j - 1;
		uint64_t numerator = ((uint64_t)u[at + b_length] << 32) | u[at + b_length - 1];
		uint64_t estimate = numerator / v[b_length - 1];
		uint64_t rest = numerator % v[b_length - 1];
		while (estimate >= base ||
		       estimate * v[b_length - 2] > ((rest << 32) | u[at + b_length - 2])) {
			estimate -= 1;
			rest += v[b_length - 1];
			if (rest >= base) {
				break;
			}
		}
		// multiply and subtract
		int64_t borrow = 0;
		uint64_t carry = 0;
		for (uint32_t index = 0; index < b_length; index += 1) {
			uint64_t product = estimate * v[index] + carry;
			carry = product >> 32;
			int64_t difference = (int64_t)u[index + at] - borrow - (int64_t)(uint32_t)product;
			u[index + at] = (uint32_t)difference;
			borrow = difference < 0;
		}
		int64_t top = (int64_t)u[at + b_length] - borrow - (int64_t)carry;
		u[at + b_length] = (uint32_t)top;
		if (top < 0) {
			// the estimate was one too large: add back
			estimate -= 1;
			uint64_t sum = 0;
			for (uint32_t index = 0; index < b_length; index += 1) {
				sum = (uint64_t)u[index + at] + v[index] + (sum >> 32);
				u[index + at] = (uint32_t)sum;
			}
			u[at + b_length] += (uint32_t)(sum >> 32);
		}
		quotient[at] = (uint32_t)estimate;
	}
	for (uint32_t index = 0; index < b_length; index += 1) {
		remainder[index] = shift == 0 ? u[index]
		                              : (u[index] >> shift) | (u[index + 1] << (32 - shift));
	}
	fa_free(u, bytes, 4);
}

/// `a // b` (`modulo` 0) or `a %% b` (`modulo` 1), rounded towards −∞; the
/// caller checked that `b` is not 0.
static int64_t big_floor_divide(int64_t a, int64_t b, int modulo) {
	if ((a & 1) && (b & 1)) {
		int64_t x = a >> 1, y = b >> 1;
		int64_t quotient = x / y, remainder = x % y;
		if (remainder != 0 && ((remainder < 0) != (y < 0))) {
			quotient -= 1;
			remainder += y;
		}
		return fa_bigint_from_i64(modulo ? remainder : quotient);
	}
	BigView x, y;
	big_view(a, &x);
	big_view(b, &y);
	int order = big_compare_magnitudes(x.limbs, x.length, y.limbs, y.length);
	FaBig *quotient;
	FaBig *remainder = big_allocate(y.length);
	uint32_t quotient_length;
	if (order < 0) {
		// |a| < |b|: the quotient 0, the remainder `a`
		quotient = big_allocate(2);
		quotient_length = 1;
		for (uint32_t index = 0; index < x.length; index += 1) {
			remainder->limbs[index] = x.limbs[index];
		}
	} else {
		quotient_length = x.length - y.length + 1;
		quotient = big_allocate(quotient_length + 1);
		big_divide_magnitudes(x.limbs, x.length, y.limbs, y.length, quotient->limbs,
		                      remainder->limbs);
	}
	uint32_t remainder_length = y.length;
	while (remainder_length > 0 && remainder->limbs[remainder_length - 1] == 0) {
		remainder_length -= 1;
	}
	uint32_t differ = x.negative != y.negative;
	if (remainder_length != 0 && differ) {
		if (modulo) {
			// r + b, signs differing: |b| - |r|, with the sign of b
			for (uint32_t index = 0, borrow = 0; index < y.length; index += 1) {
				int64_t difference = (int64_t)y.limbs[index] - remainder->limbs[index] - borrow;
				borrow = difference < 0;
				remainder->limbs[index] = (uint32_t)difference;
			}
			big_free(quotient);
			return big_finish(remainder, y.length, y.negative);
		}
		// q - 1 on a negative quotient: |q| + 1
		uint64_t carry = 1;
		for (uint32_t index = 0; index < quotient_length + 1 && carry != 0; index += 1) {
			uint64_t sum = (uint64_t)quotient->limbs[index] + carry;
			quotient->limbs[index] = (uint32_t)sum;
			carry = sum >> 32;
		}
		quotient_length += 1;
	}
	if (modulo) {
		big_free(quotient);
		return big_finish(remainder, remainder_length, x.negative);
	}
	big_free(remainder);
	return big_finish(quotient, quotient_length, differ);
}

FA_EXPORT int64_t fa_bigint_floor_divide(int64_t a, int64_t b) {
	return big_floor_divide(a, b, 0);
}

FA_EXPORT int64_t fa_bigint_floor_modulo(int64_t a, int64_t b) {
	return big_floor_divide(a, b, 1);
}

/// `-a`.
FA_EXPORT int64_t fa_bigint_negate(int64_t word) {
	if (word & 1) {
		return fa_bigint_from_i64(-(word >> 1));
	}
	int64_t copy = fa_bigint_copy(word);
	FaBig *block = big_block(copy);
	if (block->capacity == 0) {
		// a constant: a block of its own
		block = big_allocate(block->length);
		FaBig *from = big_block(word);
		for (uint32_t index = 0; index < from->length; index += 1) {
			block->limbs[index] = from->limbs[index];
		}
		block->length = from->length;
		block->negative = from->negative;
	}
	return big_finish(block, block->length, !block->negative);
}

/// A word that equal values share (the inline word, or one mixed from the
/// limbs and the sign of a block), for hash tables.
FA_EXPORT int64_t fa_bigint_hash(int64_t word) {
	if (word & 1) {
		return word;
	}
	FaBig *block = big_block(word);
	uint64_t hash = block->negative ? 0x9E3779B97F4A7C15u : 0;
	for (uint32_t index = 0; index < block->length; index += 1) {
		hash = (hash ^ block->limbs[index]) * 0x100000001B3u;
	}
	return (int64_t)(hash & ~(uint64_t)1);
}

/// `|a|`.
FA_EXPORT int64_t fa_bigint_abs(int64_t word) {
	return fa_bigint_sign(word) < 0 ? fa_bigint_negate(word) : fa_bigint_copy(word);
}

/// `a ** b`; the caller checked that `b` is not negative. An exponent of
/// more than 32 bits only fits for a base of 0, 1 or -1.
FA_EXPORT int64_t fa_bigint_power(int64_t a, int64_t b) {
	BigView e;
	big_view(b, &e);
	if (e.length > 1) {
		if (a == big_inline(0) || a == big_inline(1)) {
			return a;
		}
		if (a == big_inline(-1)) {
			return big_inline((e.limbs[0] & 1) ? -1 : 1);
		}
		fa_out(2, (const uint8_t *)"BigInt exponent too large\n", 26);
		fa_exit(101);
	}
	uint32_t exponent = e.length == 0 ? 0 : e.limbs[0];
	int64_t result = big_inline(1);
	int64_t base = fa_bigint_copy(a);
	while (exponent != 0) {
		if (exponent & 1) {
			int64_t next = fa_bigint_multiply(result, base);
			fa_bigint_drop(result);
			result = next;
		}
		exponent >>= 1;
		if (exponent != 0) {
			int64_t squared = fa_bigint_multiply(base, base);
			fa_bigint_drop(base);
			base = squared;
		}
	}
	fa_bigint_drop(base);
	return result;
}

/// The nearest `Float` (`Number(a)` in JavaScript: ties to even).
FA_EXPORT double fa_bigint_to_f64(int64_t word) {
	if (word & 1) {
		return (double)(word >> 1);
	}
	FaBig *block = big_block(word);
	uint32_t length = block->length;
	uint32_t top_bits = 32 - big_leading_zeros(block->limbs[length - 1]);
	uint64_t bits = (uint64_t)(length - 1) * 32 + top_bits;
	double magnitude;
	if (bits <= 64) {
		uint64_t value = 0;
		for (uint32_t index = length; index > 0; index -= 1) {
			value = (value << 32) | block->limbs[index - 1];
		}
		magnitude = (double)value;
	} else if (bits > 1024) {
		magnitude = __builtin_inf();
	} else {
		// the top 64 bits, the lower ones folded into a sticky bit
		uint64_t drop = bits - 64;
		uint64_t top = 0;
		int sticky = 0;
		for (uint64_t bit = bits; bit > drop; bit -= 1) {
			uint64_t position = bit - 1;
			top = (top << 1) | ((block->limbs[position / 32] >> (position % 32)) & 1);
		}
		for (uint64_t limb = 0; limb < drop / 32 && !sticky; limb += 1) {
			sticky = block->limbs[limb] != 0;
		}
		if (!sticky && drop % 32 != 0) {
			sticky = (block->limbs[drop / 32] & (((uint32_t)1 << (drop % 32)) - 1)) != 0;
		}
		magnitude = (double)(top | (uint64_t)sticky);
		// times 2^drop (at most 2^960: one exact multiplication)
		union {
			uint64_t bits;
			double value;
		} scale = {(uint64_t)(1023 + drop) << 52};
		magnitude *= scale.value;
	}
	return block->negative ? -magnitude : magnitude;
}

/// The value as a 128-bit two's complement integer in `out` (low, high);
/// 0 when it doesn't fit in 128 signed bits.
FA_EXPORT int32_t fa_bigint_to_128(int64_t word, uint64_t *out) {
	if (word & 1) {
		int64_t value = word >> 1;
		out[0] = (uint64_t)value;
		out[1] = value < 0 ? UINT64_MAX_VALUE : 0;
		return 1;
	}
	FaBig *block = big_block(word);
	if (block->length > 4) {
		return 0;
	}
	uint32_t limbs[4] = {0, 0, 0, 0};
	for (uint32_t index = 0; index < block->length; index += 1) {
		limbs[index] = block->limbs[index];
	}
	uint64_t low = ((uint64_t)limbs[1] << 32) | limbs[0];
	uint64_t high = ((uint64_t)limbs[3] << 32) | limbs[2];
	if (block->negative) {
		// up to 2^127 in magnitude
		if (high > ((uint64_t)1 << 63) || (high == ((uint64_t)1 << 63) && low != 0)) {
			return 0;
		}
		low = ~low + 1;
		high = ~high + (low == 0);
	} else if (high >> 63) {
		return 0;
	}
	out[0] = low;
	out[1] = high;
	return 1;
}

/// Appends the decimal digits of a value to the text builder.
FA_EXPORT void fa_b_bigint(int64_t word) {
	if (word & 1) {
		fa_b_i64(word >> 1);
		return;
	}
	FaBig *block = big_block(word);
	uint32_t length = block->length;
	// chunks of 9 digits, least significant first
	size_t work_bytes = (size_t)length * 4;
	size_t digits_size = (size_t)length * 10 + 2;
	uint32_t *work = fa_alloc(work_bytes, 4);
	uint8_t *digits = fa_alloc(digits_size, 1);
	for (uint32_t index = 0; index < length; index += 1) {
		work[index] = block->limbs[index];
	}
	size_t position = digits_size;
	while (length > 0) {
		uint64_t rest = 0;
		for (uint32_t index = length; index > 0; index -= 1) {
			uint64_t current = (rest << 32) | work[index - 1];
			work[index - 1] = (uint32_t)(current / 1000000000u);
			rest = current % 1000000000u;
		}
		while (length > 0 && work[length - 1] == 0) {
			length -= 1;
		}
		uint32_t chunk = (uint32_t)rest;
		for (int digit = 0; digit < 9 && (length != 0 || chunk != 0); digit += 1) {
			digits[--position] = (uint8_t)('0' + chunk % 10);
			chunk /= 10;
		}
	}
	if (block->negative) {
		digits[--position] = '-';
	}
	append_simple(digits + position, digits_size - position);
	fa_free(digits, digits_size, 1);
	fa_free(work, work_bytes, 4);
}

// ── floats (values brief D-5, D-6) ──────────────────────────────────────

#if !defined(__wasm__)

static int is_nan(double value) {
	return value != value;
}

/// The shortest decimal digits that read back as `value` (finite, not
/// zero, positive): `digits` without trailing zeros, and the exponent `n`
/// of `0.d1d2… × 10^n`. Returns the number of digits.
static int shortest_digits(double value, char *digits, int *exponent, int float32) {
	char text[40];
	for (int precision = 1; precision <= 17; precision += 1) {
		snprintf(text, sizeof text, "%.*e", precision - 1, value);
		double back = strtod(text, 0);
		if (float32 ? (float)back == (float)value : back == value) {
			break;
		}
	}
	// `d.ddde±x`
	int count = 0;
	const char *cursor = text;
	for (; *cursor != 'e'; cursor += 1) {
		if (*cursor >= '0' && *cursor <= '9') {
			digits[count] = *cursor;
			count += 1;
		}
	}
	cursor += 1;
	int sign = *cursor == '-' ? -1 : 1;
	cursor += 1;
	int power = 0;
	for (; *cursor != 0; cursor += 1) {
		power = power * 10 + (*cursor - '0');
	}
	while (count > 1 && digits[count - 1] == '0') {
		count -= 1;
	}
	*exponent = sign * power + 1;
	return count;
}

/// The ECMAScript `Number::toString` of a double.
static void append_double(double value) {
	if (is_nan(value)) {
		append_c("NaN");
		return;
	}
	if (value == 0) {
		append_byte('0');
		return;
	}
	if (value < 0) {
		append_byte('-');
		value = -value;
	}
	if (value > 1.7976931348623157e308) {
		append_c("Infinity");
		return;
	}
	char digits[24];
	int n;
	int k = shortest_digits(value, digits, &n, 0);
	if (k <= n && n <= 21) {
		fa_b_bytes((const uint8_t *)digits, (size_t)k);
		for (int index = k; index < n; index += 1) {
			append_byte('0');
		}
	} else if (0 < n && n <= 21) {
		fa_b_bytes((const uint8_t *)digits, (size_t)n);
		append_byte('.');
		fa_b_bytes((const uint8_t *)digits + n, (size_t)(k - n));
	} else if (-6 < n && n <= 0) {
		append_c("0.");
		for (int index = 0; index < -n; index += 1) {
			append_byte('0');
		}
		fa_b_bytes((const uint8_t *)digits, (size_t)k);
	} else {
		append_byte((uint8_t)digits[0]);
		if (k > 1) {
			append_byte('.');
			fa_b_bytes((const uint8_t *)digits + 1, (size_t)(k - 1));
		}
		append_byte('e');
		append_byte(n - 1 < 0 ? '-' : '+');
		append_u64((uint64_t)(n - 1 < 0 ? 1 - n : n - 1), 0);
	}
}

/// Parses `<digits>e<power>` as a double.
static double decimal(int64_t scaled, int power) {
	char text[48];
	snprintf(text, sizeof text, "%llde%d", (long long)scaled, power);
	return strtod(text, 0);
}

/// `faFloat32ToString`: the shortest decimal that reads back as the same
/// Float32, then laid out like `Number::toString`.
static void append_float32(float value) {
	if (value == 0 || value != value || value - value != 0) {
		append_double((double)value);
		return;
	}
	double wide = (double)value;
	char text[40];
	for (int digits = 1; digits <= 9; digits += 1) {
		snprintf(text, sizeof text, "%.*e", digits - 1, wide);
		int64_t scaled = 0;
		const char *cursor = text;
		int negative = 0;
		if (*cursor == '-') {
			negative = 1;
			cursor += 1;
		}
		for (; *cursor != 'e'; cursor += 1) {
			if (*cursor >= '0' && *cursor <= '9') {
				scaled = scaled * 10 + (*cursor - '0');
			}
		}
		cursor += 1;
		int sign = *cursor == '-' ? -1 : 1;
		cursor += 1;
		int power = 0;
		for (; *cursor != 0; cursor += 1) {
			power = power * 10 + (*cursor - '0');
		}
		power = sign * power - (digits - 1);
		if (negative) {
			scaled = -scaled;
		}
		int found = 0;
		double best = 0;
		int64_t candidates[3] = {scaled, scaled - 1, scaled + 1};
		for (int index = 0; index < 3; index += 1) {
			double candidate = decimal(candidates[index], power);
			double distance = candidate - wide < 0 ? wide - candidate : candidate - wide;
			double best_distance = best - wide < 0 ? wide - best : best - wide;
			if ((float)candidate == value && (!found || distance < best_distance)) {
				best = candidate;
				found = 1;
			}
			if (found && index == 0) {
				break;
			}
		}
		if (found) {
			append_double(best);
			return;
		}
	}
	append_double(wide);
}

/// `faPercentageToString`: the fraction times 100, rounded to 15 significant digits.
static void append_percentage(double fraction) {
	double value = fraction * 100;
	if (value == 0 || value != value || value - value != 0) {
		append_double(value);
	} else {
		char text[40];
		snprintf(text, sizeof text, "%.14e", value);
		append_double(strtod(text, 0));
	}
	append_byte('%');
}

FA_EXPORT void fa_b_f64(double value) {
	append_double(value);
}

FA_EXPORT void fa_b_f32(float value) {
	append_float32(value);
}

FA_EXPORT void fa_b_percentage(double fraction) {
	append_percentage(fraction);
}

#else

static void append_host_float(double value, int32_t kind) {
	reserve(40);
	builder_length += (size_t)fa_host_format_float(value, kind, builder + builder_length);
}

FA_EXPORT void fa_b_f64(double value) {
	append_host_float(value, 0);
}

FA_EXPORT void fa_b_f32(float value) {
	append_host_float((double)value, 1);
}

FA_EXPORT void fa_b_percentage(double fraction) {
	append_host_float(fraction, 2);
	append_byte('%');
}

#endif

// ── strings ─────────────────────────────────────────────────────────────

/// A string in Fa literal notation (`faQuote`): quoted, with `\\`, `\"`,
/// `\n`, `\r`, `\t`, `\0`, `\xNN` for the other control characters, and
/// `{{`, `}}` for the braces.
static void quote_fa_bytes(const uint8_t *bytes, size_t length) {
	static const char hex[] = "0123456789ABCDEF";
	for (size_t index = 0; index < length; index += 1) {
		uint8_t byte = bytes[index];
		switch (byte) {
		case '\\':
			append_c("\\\\");
			break;
		case '"':
			append_c("\\\"");
			break;
		case '\n':
			append_c("\\n");
			break;
		case '\r':
			append_c("\\r");
			break;
		case '\t':
			append_c("\\t");
			break;
		case 0:
			append_c("\\0");
			break;
		case '{':
			append_c("{{");
			break;
		case '}':
			append_c("}}");
			break;
		default:
			if (byte < 0x20 || byte == 0x7f) {
				append_c("\\x");
				append_byte((uint8_t)hex[byte >> 4]);
				append_byte((uint8_t)hex[byte & 15]);
			} else {
				append_byte(byte);
			}
		}
	}
}

FA_EXPORT void fa_b_quoted(const uint8_t *bytes, size_t length) {
	append_byte('"');
	if (span_has_edges(bytes, length)) { string_span_parts(bytes, length, quote_fa_bytes); }
	else { quote_fa_bytes(bytes, length); }
	append_byte('"');
}

/// `[\"a\", \"b\"]`: an array of strings.
FA_EXPORT void fa_b_string_array(const FaString *items, size_t length) {
	append_byte('[');
	for (size_t index = 0; index < length; index += 1) {
		if (index > 0) {
			append_c(", ");
		}
		fa_b_quoted(string_bytes(&items[index]), string_length(&items[index]));
	}
	append_byte(']');
}

FA_EXPORT __attribute__((always_inline)) void fa_concat(const uint8_t *left, size_t left_length, size_t left_capacity,
                         const uint8_t *right, size_t right_length, size_t right_capacity, FaString *out) {
	size_t left_encoded = fa_string_encoded_length(left, left_length, left_capacity);
	size_t right_encoded = fa_string_encoded_length(right, right_length, right_capacity);
	size_t length = left_encoded + right_encoded;
	size_t flags = (left_length ? left_capacity : SIMPLE) & (right_length ? right_capacity : SIMPLE) & SIMPLE;
	if (!flags) { flags = simple_flag(left, left_length) & simple_flag(right, right_length); }
	if (length < left_encoded || length >= KIND_MASK) { out_of_memory(); }
	if (length <= 1) {
		*out = owned_copy(left_length == 0 ? right : left, length);
		return;
	}
	if (length <= INLINE_STRING_BYTES) {
		FaString small = {0};
		fa_string_copy_bytes((uint8_t *)&small, left, left_length, left_capacity);
		fa_string_copy_bytes((uint8_t *)&small + left_encoded, right, right_length, right_capacity);
		inline_string_tag(&small, length, flags);
		*out = small;
		fa_string_finish_known(out, flags);
		return;
	}
	size_t block = string_block(length);
	uint8_t *bytes = string_alloc(&block, flags);
	fa_string_copy_bytes(bytes, left, left_length, left_capacity);
	fa_string_copy_bytes(bytes + left_encoded, right, right_length, right_capacity);
	out->bytes = bytes;
	out->length = length;
	out->capacity = block | flags;
	fa_string_finish_known(out, flags);
}

/// A checked byte count for concatenation, bounded by the string layout.
FA_EXPORT size_t fa_string_size(size_t left, size_t right) {
	if (right >= KIND_MASK - left) { out_of_memory(); }
	return left + right;
}

/// Allocate a template once all its expressions have been evaluated. Its
/// caller fills exactly `length` bytes before exposing the owning string.
FA_EXPORT uint8_t *fa_string_allocate(size_t length, size_t simple, FaString *out) {
	*out = (FaString){0};
	if (length <= INLINE_STRING_BYTES) {
		inline_string_tag(out, length, simple);
		return (uint8_t *)out;
	}
	// the whole block of its size class: room for later appends
	size_t block = string_block(length);
	uint8_t *bytes = string_alloc(&block, simple);
	out->bytes = bytes;
	out->length = length;
	out->capacity = block | simple;
	return bytes;
}

/// The destination buffer belongs to the function frame, never to this header.
/// Immutable short values remain inline; potentially growing mutables retain
/// their frame pointer until they outgrow it.
FA_EXPORT __attribute__((always_inline)) uint8_t *fa_string_allocate_local(size_t length,
    size_t simple, uint8_t *buffer, FaString *out, int32_t mutable) {
	if (length <= FRAME_STRING_BYTES && (mutable || length > INLINE_STRING_BYTES)) {
		*out = (FaString){buffer, length, FRAME_STRING_BYTES | STRING_FRAME | simple};
		return buffer;
	}
	return fa_string_allocate(length, simple, out);
}

/// Copy into the destination's own frame buffer. Only a consumed heap value
/// above the frame budget may transfer its pointer; a frame pointer never does.
FA_EXPORT __attribute__((always_inline)) void fa_string_assign_local(const FaString *source, uint8_t *buffer,
    FaString *out, int32_t mutable, int32_t consume) {
	FaString previous = *source;
	FaString destination = *out;
	size_t physical = string_length(&previous);
	const uint8_t *bytes = string_bytes(&previous);
	size_t length = fa_string_encoded_length(bytes, physical, previous.capacity);
	int owned = consume && string_owned(previous.capacity);
	int drop_destination = string_owned(destination.capacity);
	if (owned && destination.bytes == previous.bytes) { drop_destination = 0; }
	if (owned && (length > FRAME_STRING_BYTES || (previous.capacity & KIND_MASK) > FRAME_STRING_BYTES)) {
		*out = previous;
		if (drop_destination) { fa_string_free(destination.bytes, destination.capacity); }
		return;
	}
	FaString result;
	uint8_t *target = fa_string_allocate_local(length, previous.capacity & STRING_FLAGS,
	                                         buffer, &result, mutable);
	size_t result_capacity = result.capacity;
	int publish = !(previous.capacity & SIMPLE) && string_owned(result_capacity);
	size_t units = publish ? fa_string_length(bytes, physical, previous.capacity) : 0;
	if ((previous.capacity & SIMPLE) || !span_has_edges(bytes, physical)) {
		__builtin_memmove(target, bytes, physical);
	} else { fa_string_copy_bytes(target, bytes, physical, previous.capacity); }
	if (publish) { fa_string_set_unit_count(target, length, result_capacity, units); }
	if (owned) { fa_string_free(previous.bytes, previous.capacity); }
	if (drop_destination) { fa_string_free(destination.bytes, destination.capacity); }
	if ((previous.capacity & STRING_SURROGATES) && (result.capacity & STRING_INLINE)) { fa_string_finish(&result); }
	*out = result;
}

/// Moving a header outside a frame-backed local must detach its bytes.
FA_EXPORT __attribute__((always_inline)) void fa_string_escape(const FaString *source, FaString *out) {
	FaString previous = *source;
	*out = (string_frame(previous.capacity) || string_view(previous.capacity))
	           ? owned_copy_with(string_bytes(&previous), string_length(&previous), previous.capacity & STRING_FLAGS)
	           : previous;
}

/// Generic assignment has already dropped the previous owning value (and
/// reset any resource flags). A mutator borrowing a frame header keeps its
/// buffer; every other destination receives an independent normal header.
FA_EXPORT __attribute__((always_inline)) void fa_string_assign_after_drop(const FaString *source, FaString *out) {
	if (string_frame(out->capacity)) {
		fa_string_assign_local(source, (uint8_t *)out->bytes, out, 1, 1);
	} else {
		fa_string_escape(source, out);
	}
}

/// Replace a string whose new parts are proven disjoint from its buffer.
/// Empty assignments retain capacity, like clearing a mutable collection.
FA_EXPORT __attribute__((always_inline)) uint8_t *fa_string_replace_buffer(size_t length, size_t simple, FaString *out) {
	int owned = string_owned(out->capacity);
	size_t previous_capacity = out->capacity;
	size_t capacity = previous_capacity & KIND_MASK;
	const uint8_t *previous = out->bytes;
	if (string_writable(out->capacity, length)) {
		if (!(simple & out->capacity & SIMPLE)) { string_changed(out); }
		out->length = length;
		out->capacity = capacity | simple;
		return (uint8_t *)previous;
	}
	uint8_t *bytes = fa_string_allocate(length, simple, out);
	if (owned) { fa_string_free(previous, previous_capacity); }
	return bytes;
}

/// A template whose expressions have already been evaluated. The parts are
/// borrowed snapshots, including inline values. Copy directly to the result
/// instead of filling and copying a shared text builder.
static __attribute__((always_inline)) void join_strings(const FaString *parts, size_t count,
                                                        const uint8_t *separator, size_t separator_length,
                                                        FaString *out) {
	size_t separator_encoded = fa_string_encoded_length(separator, separator_length, 0);
	if (separator_encoded && count > 1 && count - 1 >= KIND_MASK / separator_encoded) { out_of_memory(); }
	size_t length = count > 1 ? (count - 1) * separator_encoded : 0;
	size_t simple = count > 1 ? simple_flag(separator, separator_length) : SIMPLE;
	for (size_t i = 0; i < count; i += 1) {
		size_t part_length = fa_string_encoded_length(string_bytes(parts + i), string_length(parts + i), parts[i].capacity);
		if (part_length >= KIND_MASK - length) { out_of_memory(); }
		length += part_length;
		if (part_length) { simple &= parts[i].capacity; }
	}
	FaString result = {0};
	size_t block = string_block(length);
	uint8_t *target = length <= INLINE_STRING_BYTES ? (uint8_t *)&result : string_alloc(&block, simple);
	size_t offset = 0;
	for (size_t i = 0; i < count; i += 1) {
		if (i) {
			fa_string_copy_bytes(target + offset, separator, separator_length, 0);
			offset += separator_encoded;
		}
		size_t part_length = string_length(parts + i);
		fa_string_copy_bytes(target + offset, string_bytes(parts + i), part_length, parts[i].capacity);
		offset += fa_string_encoded_length(string_bytes(parts + i), part_length, parts[i].capacity);
	}
	if (!simple) { simple = text_flags(target, length); }
	if (length <= INLINE_STRING_BYTES) { inline_string_tag(&result, length, simple); }
	else { result.bytes = target; result.length = length; result.capacity = block | simple; }
	*out = result;
	fa_string_finish_known(out, simple);
}

FA_EXPORT __attribute__((always_inline)) void fa_strings_join(const FaString *parts, size_t count, FaString *out) {
	join_strings(parts, count, 0, 0, out);
}

/// `fa_string_append` when the block is full (or not owned): a new block,
/// twice as large.
static __attribute__((noinline)) void string_grow(FaString *string, const uint8_t *bytes,
                                                  size_t length, size_t simple) {
	// The caller reserves logical bytes. Recover the physical suffix extent
	// for copying and overlap checks without adding a hot-path argument.
	size_t physical = length - span_low_head(bytes, length);
	size_t old_length = string_length(string);
	size_t needed = old_length + length;
	if (needed < old_length || needed >= KIND_MASK) { out_of_memory(); }
	size_t capacity = string->capacity & KIND_MASK;
	int owned = string_owned(string->capacity);
	size_t grown = owned && capacity * 2 > needed ? capacity * 2 : needed;
	if (grown >= KIND_MASK) { grown = KIND_MASK - 1; }
	// a string that outgrows its inline room keeps growing, usually: one
	// 64-byte block instead of 32 then 64 (inline strings hold 23)
	if (grown < 64) {
		grown = 64;
	}
	grown = string_block(grown);
	uint8_t *block;
	if (owned && (bytes + physical <= string->bytes || bytes >= string->bytes + capacity)) {
		// a large block moves its pages instead of its bytes (`fa_heap_grow`)
		block = string_grow_block(string->bytes, string->capacity, old_length, &grown, simple);
		fa_string_copy_bytes(block + old_length, bytes, physical, simple);
	} else {
		block = string_alloc(&grown, simple);
		copy_bytes(block, string_bytes(string), old_length);
		fa_string_copy_bytes(block + old_length, bytes, physical, simple);
		if (owned) {
			fa_string_free(string->bytes, string->capacity);
		}
	}
	string->bytes = block;
	string->length = needed;
	string->capacity = grown | simple;
}

/// Canonical operands can form a new surrogate pair only at their join.
/// Heap flags may remain conservative; inline flags are recomputed exactly.
static void string_finish_append(FaString *string, size_t boundary) {
	if (string->capacity & STRING_INLINE) { fa_string_finish(string); return; }
	size_t length = string->length;
	uint8_t *bytes = (uint8_t *)(void *)string->bytes;
	if (boundary < 3 || length - boundary < 3 || bytes[boundary - 3] != 0xED ||
	    bytes[boundary - 2] < 0xA0 || bytes[boundary - 2] > 0xAF ||
	    bytes[boundary] != 0xED || bytes[boundary + 1] < 0xB0 || bytes[boundary + 1] > 0xBF) { return; }
	uint32_t high = ((uint32_t)(bytes[boundary - 2] & 15) << 6) | (bytes[boundary - 1] & 63);
	uint32_t low = ((uint32_t)(bytes[boundary + 1] & 15) << 6) | (bytes[boundary + 2] & 63);
	uint32_t code = 0x10000 + (high << 10) + low;
	uint8_t *pair = bytes + boundary - 3;
	pair[0] = (uint8_t)(0xF0 | (code >> 18));
	pair[1] = (uint8_t)(0x80 | ((code >> 12) & 63));
	pair[2] = (uint8_t)(0x80 | ((code >> 6) & 63));
	pair[3] = (uint8_t)(0x80 | (code & 63));
	__builtin_memmove(pair + 4, bytes + boundary + 3, length - boundary - 3);
	string->length = length - 2;
}

/// `text += suffix`: appends in place to the block of an owned string,
/// which grows geometrically, so that a loop of appends is linear. The
/// suffix may be a view of the text itself: it is copied before the old
/// block is freed. Simple stays simple when the suffix is.
/// `fa_string_append` off its common path: a suffix not known to be
/// simple, or a block without room.
static __attribute__((noinline)) void string_append_slow(FaString *string, const uint8_t *bytes,
                                                         size_t length, size_t suffix_capacity) {
	if (length == 0) {
		return;
	}
	size_t capacity = string->capacity;
	if (string_view(capacity) && span_has_edges(string->bytes, string->length)) {
		FaString result;
		fa_concat(string->bytes, string->length, capacity, bytes, length, suffix_capacity, &result);
		*string = result;
		return;
	}
	size_t old_length = string_length(string);
	size_t encoded = fa_string_encoded_length(bytes, length, suffix_capacity);
	size_t needed = old_length + encoded;
	size_t simple = (suffix_capacity & SIMPLE) ? SIMPLE
	                : length == 1 ? (bytes[0] < 0x80 ? SIMPLE : 0)
	                              : text_flags(bytes, length);
	if (old_length != 0) {
		simple = (simple & capacity & SIMPLE) | ((simple | capacity) & STRING_SURROGATES);
	}
	size_t units = 0;
	if (!(simple & SIMPLE)) {
		units = fa_string_length(string_bytes(string), old_length, capacity) +
		        fa_string_length(bytes, length, suffix_capacity);
	}
	if (string_writable(capacity, needed)) {
		if (!(simple & SIMPLE)) { string_changed(string); }
		fa_string_copy_bytes((uint8_t *)string->bytes + old_length, bytes, length, suffix_capacity);
		string->length = needed;
		string->capacity = (capacity & KIND_MASK) | simple;
		string_finish_append(string, old_length);
		fa_string_set_unit_count(string_bytes(string), string_length(string), string->capacity, units);
		return;
	}
	if (needed <= INLINE_STRING_BYTES) {
		if (capacity & STRING_INLINE) {
			fa_string_copy_bytes((uint8_t *)string + old_length, bytes, length, suffix_capacity);
			inline_string_tag(string, needed, simple);
		} else {
			FaString small = {0};
			copy_bytes((uint8_t *)&small, string_bytes(string), old_length);
			fa_string_copy_bytes((uint8_t *)&small + old_length, bytes, length, suffix_capacity);
			inline_string_tag(&small, needed, simple);
			if (string_owned(capacity)) { fa_string_free(string->bytes, capacity); }
			*string = small;
		}
		string_finish_append(string, old_length);
		fa_string_set_unit_count(string_bytes(string), string_length(string), string->capacity, units);
		return;
	}
	string_grow(string, bytes, encoded, simple);
	string_finish_append(string, old_length);
	fa_string_set_unit_count(string_bytes(string), string_length(string), string->capacity, units);
}

/// The lowering has already checked its common append edge. Do not
/// duplicate that predicate in the snapshot path: it would give the hot
/// condition two uses and prevent direct comparison branches on x86.
FA_EXPORT __attribute__((always_inline)) void fa_string_append_after_guard(FaString *string, const uint8_t *bytes, size_t length,
                                size_t suffix_capacity) {
	size_t capacity = string->capacity;
	if (length == 0) { return; }
	if (string_view(capacity) && span_has_edges(string->bytes, string->length)) {
		string_append_slow(string, bytes, length, suffix_capacity);
		return;
	}
	size_t old_length = string_length(string);
	size_t needed = old_length + length;
	// A non-simple short suffix retains a reserved heap block just as it
	// retains a frame buffer. Only previously inline/shared text becomes inline.
	if (string_writable(capacity, needed)) {
		string_append_slow(string, bytes, length, suffix_capacity);
		return;
	}
	if (needed <= INLINE_STRING_BYTES && (suffix_capacity & SIMPLE)) {
		size_t simple = suffix_capacity & SIMPLE;
		if (old_length != 0) { simple = (simple & capacity & SIMPLE) | (capacity & STRING_SURROGATES); }
		if (capacity & STRING_INLINE) {
			uint8_t tag = ((uint8_t *)string)[sizeof(FaString) - 1];
			copy_bytes((uint8_t *)string + old_length, bytes, length);
			((uint8_t *)string)[sizeof(FaString) - 1] = (uint8_t)(tag + length);
		} else {
			FaString small = {0};
			copy_bytes((uint8_t *)&small, string->bytes, old_length);
			copy_bytes((uint8_t *)&small + old_length, bytes, length);
			inline_string_tag(&small, needed, simple);
			if (string_owned(capacity)) { fa_string_free(string->bytes, capacity); }
			*string = small;
		}
		return;
	}
	string_append_slow(string, bytes, length, suffix_capacity);
}

FA_EXPORT __attribute__((always_inline)) void fa_string_append(FaString *string, const uint8_t *bytes, size_t length,
                                size_t suffix_capacity) {
	// Common path: an ASCII suffix into an owned/frame block with room.
	size_t kind = string->capacity & ~(STRING_FLAGS | STRING_FRAME);
	if (__builtin_expect((suffix_capacity & SIMPLE) && kind < (KIND_MASK & ~STRING_FRAME) && string->length + length <= kind, 1)) {
		copy_bytes((uint8_t *)string->bytes + string->length, bytes, length);
		string->length += length;
		return;
	}
	fa_string_append_after_guard(string, bytes, length, suffix_capacity);
}

/// `fa_string_append` out of line, for the slow path of an append to a
/// string in memory (a field, an item: `src/lower/text.rs`), so that the
/// caller's hot path keeps few values alive around one call.
FA_EXPORT __attribute__((noinline, cold)) void fa_string_append_cold(FaString *string, const uint8_t *bytes,
                                                                    size_t length, size_t suffix_capacity) {
	fa_string_append(string, bytes, length, suffix_capacity);
}

/// `string + suffix` off its common path: a new string of the exact size
/// (a sum is seldom grown further), the old block freed when owned.
static __attribute__((noinline)) void string_plus_slow(FaString *string, const uint8_t *bytes,
                                                       size_t length) {
	FaString result;
	fa_concat(string_bytes(string), string_length(string), string->capacity, bytes, length, 0, &result);
	if (string_owned(string->capacity)) {
		fa_string_free(string->bytes, string->capacity);
	}
	*string = result;
}

/// `left + suffix` where the generated code owns `left` (a local at its last
/// use): appended in place when its heap block has room, else a new string.
FA_EXPORT __attribute__((always_inline)) void fa_string_plus(FaString *string,
                                                             const uint8_t *bytes, size_t length,
                                                             size_t suffix_capacity) {
	size_t kind = string->capacity & ~(STRING_FLAGS | STRING_FRAME);
	if (__builtin_expect((suffix_capacity & SIMPLE) && kind < (KIND_MASK & ~STRING_FRAME) &&
	                         string->length + length <= kind,
	                     1)) {
		copy_bytes((uint8_t *)string->bytes + string->length, bytes, length);
		string->length += length;
		return;
	}
	if (length == 0) {
		return;
	}
	if (string_writable(string->capacity, string_length(string) + length)) {
		fa_string_append(string, bytes, length, suffix_capacity);
		return;
	}
	if ((string->capacity & STRING_INLINE) || string_length(string) + length <= INLINE_STRING_BYTES) {
		// short: inline, as an append makes it
		fa_string_append(string, bytes, length, suffix_capacity);
		return;
	}
	FaString copy = *string;
	string_plus_slow(&copy, bytes, length);
	*string = copy;
}

/// `fa_string_reserve` when the string has no room: a heap block twice as
/// large, or as large as needed.
static int string_reserve_virtual(FaString *string, size_t extra) {
	if (!fa_virtual_string_slices || !string_view(string->capacity) ||
	    !span_has_edges(string->bytes, string->length)) { return 0; }
	size_t length = fa_string_encoded_length(string->bytes, string->length, string->capacity);
	FaString result;
	fa_string_from(string->bytes, string->length, string->capacity, fa_string_size(length, extra), &result);
	*string = result;
	return 1;
}

static __attribute__((noinline)) void string_reserve_slow(FaString *string, size_t needed) {
	size_t capacity = string->capacity;
	size_t old_length = string_length(string);
	size_t units = capacity & SIMPLE ? old_length : fa_string_length(string_bytes(string), old_length, capacity);
	int owned = string_owned(capacity);
	size_t kind = capacity & KIND_MASK;
	size_t grown = owned && kind * 2 > needed ? kind * 2 : needed;
	// a string that outgrows its inline room keeps growing, usually: one
	// 64-byte block instead of 32 then 64 (inline strings hold 23)
	if (grown < 64) {
		grown = 64;
	}
	if (grown >= KIND_MASK) {
		out_of_memory();
	}
	grown = string_block(grown);
	uint8_t *block;
	if (owned) {
		block = string_grow_block(string->bytes, capacity, old_length, &grown, old_length ? capacity & STRING_FLAGS : SIMPLE);
	} else {
		block = string_alloc(&grown, old_length ? capacity & STRING_FLAGS : SIMPLE);
		copy_bytes(block, string_bytes(string), old_length);
	}
	string->bytes = block;
	string->length = old_length;
	string->capacity = grown | (old_length != 0 ? capacity & STRING_FLAGS : SIMPLE);
	fa_string_set_unit_count(block, old_length, string->capacity, units);
}

/// Room for `extra` more bytes in a string (`text += "… {value} …"` before
/// its parts, `reserve`): an inline string keeps its room when it is enough.
FA_EXPORT __attribute__((always_inline)) void fa_string_reserve(FaString *string, size_t extra) {
	if (string_reserve_virtual(string, extra)) { return; }
	size_t capacity = string->capacity;
	size_t kind = capacity & ~(STRING_FLAGS | STRING_FRAME);
	size_t needed = string_length(string) + extra;
	if (needed < extra) {
		out_of_memory();
	}
	if (__builtin_expect(needed <= INLINE_STRING_BYTES || (kind < (KIND_MASK & ~STRING_FRAME) && needed <= kind), 1)) {
		return;
	}
	FaString copy = *string;
	string_reserve_slow(&copy, needed);
	*string = copy;
}

/// Room for `extra` more bytes in a heap block the string owns (an inline
/// or static string moves to one, of the size needed), so that a loop
/// appending a byte per round stores them without a check
/// (`src/lower/appends.rs`).
static __attribute__((noinline)) void string_reserve_owned_slow(FaString *string, size_t extra);

FA_EXPORT void fa_string_reserve_owned(FaString *string,
                                                                     size_t extra) {
	if (string_reserve_virtual(string, extra)) { return; }
	size_t capacity = string->capacity;
	size_t needed = string_length(string) + extra;
	if (__builtin_expect(extra == 0 || (needed >= extra && string_writable(capacity, needed)), 1)) {
		return;
	}
	// a new empty string (`mutable word = ""` before its loop): a block
	if (string_length(string) == 0 && !string_owned(capacity) && extra < KIND_MASK) {
		size_t block = string_block(extra);
		string->bytes = string_alloc(&block, SIMPLE);
		string->length = 0;
		string->capacity = block | SIMPLE;
		return;
	}
	string_reserve_owned_slow(string, extra);
}

/// A counted append loop reserves one byte per remaining round. Keep a
/// short loop in the value itself, including a new static empty string;
/// its generated byte stores distinguish inline values from owned blocks.
FA_EXPORT __attribute__((always_inline)) void fa_string_reserve_inline(FaString *string,
                                                                        size_t extra) {
	if (string_reserve_virtual(string, extra)) { return; }
	size_t capacity = string->capacity;
	// A reservation made by the program remains available. In particular,
	// an empty loop must not change the value or its observable capacity.
	if (extra == 0) {
		return;
	}
	// Both heap blocks and frame buffers retain their existing room. The
	// common representation test includes both, without a separate tag branch.
	size_t kind = capacity & ~(STRING_FLAGS);
	if (kind != 0 && kind < KIND_MASK) {
		size_t needed = string->length + extra;
		if (needed < extra || needed > (kind & ~(STRING_FRAME))) {
			string_reserve_owned_slow(string, extra);
		}
		return;
	}
	size_t length = string_length(string);
	if (length <= INLINE_STRING_BYTES && extra <= INLINE_STRING_BYTES - length) {
		FaString small = {0};
		copy_bytes((uint8_t *)&small, string_bytes(string), length);
		inline_string_tag(&small, length, length == 0 ? SIMPLE : capacity & STRING_FLAGS);
		if (capacity & STRING_SURROGATES) { fa_string_finish(&small); }
		*string = small;
		return;
	}
	fa_string_reserve_owned(string, extra);
}

static __attribute__((noinline)) void string_reserve_owned_slow(FaString *string, size_t extra) {
	size_t capacity = string->capacity;
	size_t length = string_length(string);
	size_t needed = length + extra;
	if (needed < extra || needed >= KIND_MASK) {
		out_of_memory();
	}
	int owned = string_owned(capacity);
	size_t kind = capacity & KIND_MASK;
	if (string_writable(capacity, needed)) {
		return;
	}
	size_t units = capacity & SIMPLE ? length : fa_string_length(string_bytes(string), length, capacity);
	size_t grown = owned && kind * 2 > needed ? kind * 2 : needed;
	grown = string_block(grown);
	uint8_t *block;
	if (owned) {
		block = string_grow_block(string->bytes, capacity, length, &grown, length ? capacity & STRING_FLAGS : SIMPLE);
	} else {
		block = string_alloc(&grown, length ? capacity & STRING_FLAGS : SIMPLE);
		copy_bytes(block, string_bytes(string), length);
	}
	string->bytes = block;
	string->length = length;
	string->capacity = grown | (length != 0 ? capacity & STRING_FLAGS : SIMPLE);
	fa_string_set_unit_count(block, length, string->capacity, units);
}

/// `text.capacity`: the bytes a string holds without a new block (its
/// inline room, its heap block, its length when it shares static bytes).
FA_EXPORT __attribute__((always_inline)) size_t fa_string_room(size_t length, size_t capacity) {
	if (capacity & STRING_INLINE) {
		return INLINE_STRING_BYTES;
	}
	size_t kind = capacity & KIND_MASK;
	return (capacity & STRING_LITERAL) || kind == 0 || kind == KIND_MASK ? length : kind & ~(STRING_FRAME);
}

/// `String.from(text, capacity = n)`: a copy of the text with room for `n`
/// bytes (inline when they fit).
FA_EXPORT void fa_string_from(const uint8_t *bytes, size_t length, size_t capacity,
                              size_t room, FaString *out) {
	size_t encoded = fa_string_encoded_length(bytes, length, capacity);
	if (room <= encoded || room <= INLINE_STRING_BYTES) {
		*out = owned_copy_with(bytes, length, capacity & STRING_FLAGS);
		return;
	}
	if (room >= KIND_MASK) {
		out_of_memory();
	}
	size_t block = string_block(room);
	size_t units = capacity & SIMPLE ? length : fa_string_length(bytes, length, capacity);
	uint8_t *copy = string_alloc(&block, length == 0 ? SIMPLE : capacity & STRING_FLAGS);
	fa_string_copy_bytes(copy, bytes, length, capacity);
	out->bytes = copy;
	out->length = encoded;
	out->capacity = block | (length == 0 ? SIMPLE : capacity & STRING_FLAGS);
	fa_string_set_unit_count(copy, encoded, out->capacity, units);
}

static __attribute__((noinline)) int32_t string_equal_bytes(const uint8_t *left,
                                                          const uint8_t *right, size_t length) {
	// Load only bytes inside both strings. Fixed-size copies allow LLVM to
	// use unaligned word loads even on freestanding targets, and short keys
	// need no byte loop (in particular the four-byte names used by sorting).
	size_t index = 0;
	for (; length - index >= 8; index += 8) {
		uint64_t a, b;
		__builtin_memcpy(&a, left + index, 8);
		__builtin_memcpy(&b, right + index, 8);
		if (a != b) {
			return 0;
		}
	}
	if (length - index >= 4) {
		uint32_t a, b;
		__builtin_memcpy(&a, left + index, 4);
		__builtin_memcpy(&b, right + index, 4);
		if (a != b) { return 0; }
		index += 4;
	}
	if (length - index >= 2) {
		uint16_t a, b;
		__builtin_memcpy(&a, left + index, 2);
		__builtin_memcpy(&b, right + index, 2);
		if (a != b) { return 0; }
		index += 2;
	}
	if (index < length && left[index] != right[index]) { return 0; }
	return 1;
}

static __attribute__((always_inline)) int32_t string_equal_canonical(const uint8_t *left,
                                  size_t left_length, const uint8_t *right, size_t right_length) {
	if (left_length != right_length) { return 0; }
	if (left_length == 0) { return 1; }
	// Keep single-byte comparisons visible to the caller: literals and
	// compact character reads can then reduce to a load and a comparison.
	if (left_length == 1) { return left[0] == right[0]; }
	// 2 to 16 bytes (words, keys, views of a split text): two overlapping
	// loads from each side, all inside both strings, and no call.
	if (left_length <= 16) {
		if (left_length >= 8) {
			uint64_t a, b, c, d;
			__builtin_memcpy(&a, left, 8);
			__builtin_memcpy(&b, right, 8);
			__builtin_memcpy(&c, left + left_length - 8, 8);
			__builtin_memcpy(&d, right + left_length - 8, 8);
			return ((a ^ b) | (c ^ d)) == 0;
		}
		if (left_length >= 4) {
			uint32_t a, b, c, d;
			__builtin_memcpy(&a, left, 4);
			__builtin_memcpy(&b, right, 4);
			__builtin_memcpy(&c, left + left_length - 4, 4);
			__builtin_memcpy(&d, right + left_length - 4, 4);
			return ((a ^ b) | (c ^ d)) == 0;
		}
		uint16_t a, b;
		__builtin_memcpy(&a, left, 2);
		__builtin_memcpy(&b, right, 2);
		return a == b && left[left_length - 1] == right[left_length - 1];
	}
	return string_equal_bytes(left, right, left_length);
}

static uint32_t decode(const uint8_t *bytes, size_t length, size_t *index);

static uint16_t next_unit(const uint8_t *bytes, size_t length, size_t *offset, uint16_t *pending) {
	if (*pending) { uint16_t low = *pending; *pending = 0; return low; }
	uint32_t code = decode(bytes, length, offset);
	if (code < 0x10000) { return (uint16_t)code; }
	code -= 0x10000;
	*pending = (uint16_t)(0xDC00 + (code & 1023));
	return (uint16_t)(0xD800 + (code >> 10));
}

static int32_t compare_units(const uint8_t *left, size_t left_length,
                             const uint8_t *right, size_t right_length) {
	size_t a = 0, b = 0;
	uint16_t low_a = 0, low_b = 0;
	while ((a < left_length || low_a) && (b < right_length || low_b)) {
		uint16_t x = next_unit(left, left_length, &a, &low_a);
		uint16_t y = next_unit(right, right_length, &b, &low_b);
		if (x != y) { return x < y ? -1 : 1; }
	}
	return (a < left_length || low_a) ? 1 : (b < right_length || low_b) ? -1 : 0;
}

FA_EXPORT __attribute__((always_inline)) int32_t fa_string_equal(const uint8_t *left,
    size_t left_length, const uint8_t *right, size_t right_length) {
	if (span_has_edges(left, left_length) || span_has_edges(right, right_length)) {
		return compare_units(left, left_length, right, right_length) == 0;
	}
	return string_equal_canonical(left, left_length, right, right_length);
}

FA_EXPORT __attribute__((always_inline)) int32_t fa_string_equal_flags(const uint8_t *left,
    size_t left_length, size_t left_capacity, const uint8_t *right, size_t right_length,
    size_t right_capacity) {
	if (left_capacity & right_capacity & SIMPLE) {
		return string_equal_canonical(left, left_length, right, right_length);
	}
	return fa_string_equal(left, left_length, right, right_length);
}

/// Byte order is UTF-16 order until a surrogate or astral sequence differs.
FA_EXPORT int32_t fa_string_compare(const uint8_t *left, size_t left_length, const uint8_t *right,
                                    size_t right_length) {
	if (span_has_edges(left, left_length) || span_has_edges(right, right_length)) {
		return compare_units(left, left_length, right, right_length);
	}
	size_t length = left_length < right_length ? left_length : right_length;
	for (size_t index = 0; index < length; index += 1) {
		if (left[index] != right[index]) {
			size_t start = index;
			while (start && (left[start] & 0xC0) == 0x80) { start -= 1; }
			if (left[start] >= 0xED || right[start] >= 0xED) {
				return compare_units(left + start, left_length - start, right + start, right_length - start);
			}
			return left[index] < right[index] ? -1 : 1;
		}
	}
	return left_length == right_length ? 0 : (left_length < right_length ? -1 : 1);
}

// ── grapheme clusters ───────────────────────────────────────────────────

// UAX #29 revision 47, Unicode 17.0.0. The generated properties are used
// only by characters(); lengths, positions and iteration do not reference them.
typedef struct { uint32_t first, last; } Range;

static int in_ranges(uint32_t code, const Range *ranges, size_t count) {
	size_t low = 0;
	size_t high = count;
	while (low < high) {
		size_t middle = (low + high) / 2;
		if (code < ranges[middle].first) {
			high = middle;
		} else if (code > ranges[middle].last) {
			low = middle + 1;
		} else {
			return 1;
		}
	}
	return 0;
}

enum Break {
    OTHER = 0, CR, LF, CONTROL, EXTEND_KIND, ZWJ, REGIONAL, PREPEND,
    SPACING_MARK, HANGUL_L, HANGUL_V, HANGUL_T, HANGUL_LV, HANGUL_LVT,
};
#define GRAPHEME_KIND 15
#define EXTENDED_PICTOGRAPHIC 16
#define INDIC_CONSONANT 1
#define INDIC_EXTEND 2
#define INDIC_LINKER 3

#include "fa_grapheme_tables.h"

static uint8_t grapheme_properties(uint32_t code) {
    if (code < 0x80) { return code == '\r' ? CR : code == '\n' ? LF : code < 0x20 || code == 0x7F ? CONTROL : OTHER; }
    if (code >= 0xAC00 && code <= 0xD7A3) {
        return (code - 0xAC00) % 28 == 0 ? HANGUL_LV : HANGUL_LVT;
    }
    size_t low = 0, high = sizeof FA_GRAPHEME_RANGES / sizeof FA_GRAPHEME_RANGES[0];
    while (low < high) {
        size_t middle = low + (high - low) / 2;
        const GraphemeRange *range = &FA_GRAPHEME_RANGES[middle];
        if (code < range->first) { high = middle; }
        else if (code > range->last) { low = middle + 1; }
        else { return range->flags; }
    }
    return OTHER;
}

/// Decodes the code point at `*index` (the input is valid UTF-8) and moves past it.
static uint32_t decode(const uint8_t *bytes, size_t length, size_t *index) {
	uint8_t first = bytes[*index];
	if (fa_virtual_string_slices && *index == 0 && span_low_head(bytes, length)) {
		*index = 2;
		return span_low_unit(bytes);
	}
	if (fa_virtual_string_slices && first >= 0xF0 && first <= 0xF4 && length - *index == 3) {
		uint16_t unit = span_high_unit(bytes + *index);
		*index += 3;
		return unit;
	}
	size_t count = first < 0x80 ? 1 : first < 0xE0 ? 2 : first < 0xF0 ? 3 : 4;
	uint32_t code = count == 1 ? first : first & (0x7F >> count);
	for (size_t offset = 1; offset < count && *index + offset < length; offset += 1) {
		code = (code << 6) | (bytes[*index + offset] & 0x3F);
	}
	*index += count;
	return code;
}

/// Iteration decodes code points, independent of UAX #29 segmentation.
FA_EXPORT size_t fa_next_string_point(const uint8_t *bytes, size_t length, size_t offset) {
	(void)decode(bytes, length, &offset);
	return offset;
}

static size_t encode_point(uint8_t *out, uint32_t code) {
	if (code < 0x80) { out[0] = (uint8_t)code; return 1; }
	if (code < 0x800) {
		out[0] = (uint8_t)(0xC0 | (code >> 6));
		out[1] = (uint8_t)(0x80 | (code & 63));
		return 2;
	}
	if (code < 0x10000) {
		out[0] = (uint8_t)(0xE0 | (code >> 12));
		out[1] = (uint8_t)(0x80 | ((code >> 6) & 63));
		out[2] = (uint8_t)(0x80 | (code & 63));
		return 3;
	}
	out[0] = (uint8_t)(0xF0 | (code >> 18));
	out[1] = (uint8_t)(0x80 | ((code >> 12) & 63));
	out[2] = (uint8_t)(0x80 | ((code >> 6) & 63));
	out[3] = (uint8_t)(0x80 | (code & 63));
	return 4;
}

typedef struct {
	size_t position;
	uint32_t pending;
	uint32_t remaining;
} FaUtf8Cursor;

/// Four bytes of bounded cursor state encode one code point at a time.
/// The lowering retains a direct byte loop when the ASCII flag is set.
FA_EXPORT uint8_t fa_next_utf8_byte(const uint8_t *bytes, size_t length, FaUtf8Cursor *cursor) {
	if (!cursor->remaining) {
		uint32_t code = decode(bytes, length, &cursor->position);
		if (code >= 0xD800 && code <= 0xDFFF) { code = 0xFFFD; }
		uint8_t encoded[4] = {0};
		cursor->remaining = (uint32_t)encode_point(encoded, code);
		__builtin_memcpy(&cursor->pending, encoded, sizeof encoded);
	}
	uint8_t byte = (uint8_t)cursor->pending;
	cursor->pending >>= 8;
	cursor->remaining -= 1;
	return byte;
}

/// A previously bounds-checked byte of the UTF-8 encoding. Canonical WTF-8
/// and UTF-8 have equal byte positions; only cut heads shift them, and lone
/// surrogates replace three bytes with three bytes. No whole encoding is made.
static __attribute__((noinline, cold)) uint64_t string_utf8_byte_slow(
    const uint8_t *bytes, size_t length, size_t capacity, size_t position) {
	(void)capacity;
	static const uint8_t replacement[] = {0xEF, 0xBF, 0xBD};
	if (span_low_head(bytes, length)) {
		if (position < 3) { return replacement[position]; }
		position -= 1;
	}
	if (span_high_tail(bytes, length) && position >= length - 3) {
		return replacement[position - (length - 3)];
	}
	if (bytes[position] == 0xED && position + 2 < length && bytes[position + 1] >= 0xA0) { return 0xEF; }
	if (position && bytes[position - 1] == 0xED && bytes[position] >= 0xA0 && position + 1 < length) { return 0xBF; }
	if (position >= 2 && bytes[position - 2] == 0xED && bytes[position - 1] >= 0xA0) { return 0xBD; }
	return bytes[position];
}

/// Keep the plain byte load separate from replacement-code result merges.
/// A wide internal return retains that load's zero extension through phis.
FA_EXPORT __attribute__((always_inline)) uint64_t fa_string_utf8_byte(
    const uint8_t *bytes, size_t length, size_t capacity, size_t position) {
	if (__builtin_expect((capacity & SIMPLE) != 0, 1) || !(capacity & STRING_SURROGATES)) {
		return bytes[position];
	}
	return string_utf8_byte_slow(bytes, length, capacity, position);
}

/// Normalize a writable byte sequence, including joins across builder parts.
static size_t string_join_pairs(uint8_t *bytes, size_t length, size_t *flags) {
	size_t read = 0, written = 0;
	*flags = 0;
	while (read < length) {
		size_t start = read;
		uint32_t code = decode(bytes, length, &read);
		if (code >= 0xD800 && code <= 0xDBFF && read < length) {
			size_t next = read;
			uint32_t low = decode(bytes, length, &next);
			if (low >= 0xDC00 && low <= 0xDFFF) {
				written += encode_point(bytes + written, 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00);
				read = next;
				continue;
			}
		}
		if (code >= 0xD800 && code <= 0xDFFF) { *flags = STRING_SURROGATES; }
		__builtin_memmove(bytes + written, bytes + start, read - start);
		written += read - start;
	}
	*flags |= simple_flag(bytes, written);
	return written;
}

/// Canonical WTF-8 joins adjacent surrogate halves. Only a result being
/// written is passed here; borrowed static storage is never modified.
static __attribute__((noinline, cold)) void string_finish_slow(FaString *string) {
	uint8_t *bytes = (uint8_t *)(void *)string_bytes(string);
	size_t length = string_length(string);
	if (simple_flag(bytes, length)) {
		if (string->capacity & STRING_INLINE) { inline_string_tag(string, length, SIMPLE); }
		else {
			string_changed(string);
			string->capacity = (string->capacity & KIND_MASK) | SIMPLE;
		}
		return;
	}
	size_t flags;
	size_t written = string_join_pairs(bytes, length, &flags);
	if (string->capacity & STRING_INLINE) {
		__builtin_memset(bytes + written, 0, INLINE_STRING_BYTES - written);
		inline_string_tag(string, written, flags);
	}
	else {
		string_changed(string);
		string->length = written;
		string->capacity = (string->capacity & KIND_MASK) | flags;
		fa_string_set_unit_count(bytes, written, string->capacity, unit_count(bytes, written));
	}
}

FA_EXPORT int32_t fa_string_is_well_formed(const uint8_t *bytes, size_t length, size_t capacity) {
	return (capacity & SIMPLE) || !(text_flags(bytes, length) & STRING_SURROGATES);
}

/// Output uses the logical byte length: each lone surrogate becomes U+FFFD.
FA_EXPORT void fa_string_write_utf8(uint8_t *out, const uint8_t *bytes, size_t length, size_t capacity) {
	size_t encoded = fa_string_encoded_length(bytes, length, capacity);
	if (out != bytes || (!(capacity & SIMPLE) && span_has_edges(bytes, length))) {
		fa_string_copy_bytes(out, bytes, length, capacity);
	}
	if (capacity & SIMPLE) { return; }
	for (size_t i = 0; i + 2 < encoded; i += 1) {
		if (out[i] == 0xED && out[i + 1] >= 0xA0) {
			out[i] = 0xEF; out[i + 1] = 0xBF; out[i + 2] = 0xBD;
			i += 2;
		}
	}
}

/// GB3--GB13, in rule order. Indic state records a preceding consonant
/// and whether a linker has followed it through InCB Extend/Linker units.
static int grapheme_joins(uint8_t previous, uint8_t current, int regional_odd,
                          int emoji_zwj, int indic) {
    unsigned before = previous & GRAPHEME_KIND, next = current & GRAPHEME_KIND;
    if (before == CR && next == LF) { return 1; }
    if (before == CR || before == LF || before == CONTROL || next == CR || next == LF || next == CONTROL) { return 0; }
    if (before == HANGUL_L && (next == HANGUL_L || next == HANGUL_V || next == HANGUL_LV || next == HANGUL_LVT)) { return 1; }
    if ((before == HANGUL_LV || before == HANGUL_V) && (next == HANGUL_V || next == HANGUL_T)) { return 1; }
    if ((before == HANGUL_LVT || before == HANGUL_T) && next == HANGUL_T) { return 1; }
    if (next == EXTEND_KIND || next == ZWJ || next == SPACING_MARK || before == PREPEND) { return 1; }
    if ((current >> 5) == INDIC_CONSONANT && indic == 2) { return 1; }
    if (before == ZWJ && (current & EXTENDED_PICTOGRAPHIC) && emoji_zwj) { return 1; }
    return before == REGIONAL && next == REGIONAL && regional_odd;
}

static int indic_state(int state, uint8_t properties) {
    switch (properties >> 5) {
    case INDIC_CONSONANT: return 1;
    case INDIC_EXTEND: return state;
    case INDIC_LINKER: return state ? 2 : 0;
    default: return 0;
    }
}

/// The end of the extended grapheme cluster starting at index.
static size_t next_boundary(const uint8_t *bytes, size_t length, size_t index) {
    if (bytes[index] < 0x80 && (index + 1 >= length || bytes[index + 1] < 0x80)) {
        return index + 1 + (bytes[index] == '\r' && index + 1 < length && bytes[index + 1] == '\n');
    }
    uint8_t previous = grapheme_properties(decode(bytes, length, &index));
    int regional_odd = (previous & GRAPHEME_KIND) == REGIONAL;
    int emoji_run = (previous & EXTENDED_PICTOGRAPHIC) != 0, emoji_zwj = 0;
    int indic = indic_state(0, previous);
    while (index < length) {
        size_t next = index;
        uint8_t current = grapheme_properties(decode(bytes, length, &next));
        if (!grapheme_joins(previous, current, regional_odd, emoji_zwj, indic)) { break; }
        unsigned kind = current & GRAPHEME_KIND;
        regional_odd = kind == REGIONAL ? ((previous & GRAPHEME_KIND) == REGIONAL ? !regional_odd : 1) : 0;
        emoji_zwj = kind == ZWJ && emoji_run;
        if (current & EXTENDED_PICTOGRAPHIC) { emoji_run = 1; }
        else if (kind != EXTEND_KIND) { emoji_run = 0; }
        indic = indic_state(indic, current);
        previous = current;
        index = next;
    }
    return index;
}

/// The end of a grapheme cluster for explicit characters() consumers.
FA_EXPORT size_t fa_next_grapheme(const uint8_t *bytes, size_t length, size_t index) {
	return next_boundary(bytes, length, index);
}

/// The number of extended grapheme clusters, independent of String.length.
FA_EXPORT size_t fa_graphemes(const uint8_t *bytes, size_t length) {
	size_t count = 0;
	size_t index = 0;
	while (index < length) {
		// eight ASCII bytes without '\r', followed by an ASCII byte: eight clusters
		if (index + 8 < length && bytes[index + 8] < 0x80) {
			uint64_t word;
			copy_bytes((uint8_t *)&word, bytes + index, 8);
			uint64_t ones = 0x0101010101010101u;
			uint64_t highs = 0x8080808080808080u;
			uint64_t cr = word ^ (ones * '\r');
			if (((word | ((cr - ones) & ~cr)) & highs) == 0) {
				count += 8;
				index += 8;
				continue;
			}
		}
		index = next_boundary(bytes, length, index);
		count += 1;
	}
	return count;
}

/// `text += "… {value} …"`: appends the built text to a string.
FA_EXPORT void fa_b_append_into(FaString *string) {
	fa_string_append(string, builder, builder_length, builder_flag());
	builder_length = 0;
	builder_simple = 0;
}

static int bytes_at(const uint8_t *text, size_t text_length, size_t offset, const uint8_t *search,
                    size_t search_length) {
	if (offset + search_length > text_length) {
		return 0;
	}
	for (size_t index = 0; index < search_length; index += 1) {
		if (text[offset + index] != search[index]) {
			return 0;
		}
	}
	return 1;
}

/// The offset of the first `byte` at or after `offset`, or `length`:
/// sixteen bytes at a time, with bounded word and byte tails.
static size_t next_byte(const uint8_t *text, size_t length, size_t offset, uint8_t byte) {
	const uint64_t ones = 0x0101010101010101u;
	uint64_t pattern = ones * byte;
	// A bounded vector comparison skips twice as many bytes per iteration.
	// Inspect the comparison mask to locate the first matching byte.
	typedef uint8_t ByteVector __attribute__((vector_size(16)));
	ByteVector wanted = {byte, byte, byte, byte, byte, byte, byte, byte,
	                     byte, byte, byte, byte, byte, byte, byte, byte};
	for (; offset + 16 <= length; offset += 16) {
		ByteVector word;
		__builtin_memcpy(&word, text + offset, 16);
		ByteVector matches = word == wanted;
		uint64_t halves[2];
		__builtin_memcpy(halves, &matches, 16);
		uint64_t found = halves[0] ? halves[0] : halves[1];
		if (found) {
			return offset + (halves[0] ? 0 : 8) + (size_t)(__builtin_ctzll(found) >> 3);
		}
	}
	for (; offset + 8 <= length; offset += 8) {
		uint64_t word;
		__builtin_memcpy(&word, text + offset, 8);
		uint64_t difference = word ^ pattern;
		uint64_t found = (difference - ones) & ~difference & (ones * 0x80);
		if (found != 0) {
			return offset + (size_t)(__builtin_ctzll(found) >> 3);
		}
	}
	for (; offset < length; offset += 1) {
		if (text[offset] == byte) {
			return offset;
		}
	}
	return length;
}

/// The index of the first item equal to `byte` at or after `offset` in the
/// items of an array of bytes, or `length` (`src/lower/copies.rs`: a loop
/// whose body only acts on the items equal to a constant skips the others).
FA_EXPORT int64_t fa_bytes_next(const uint8_t *items, int64_t offset, int64_t length,
                                int32_t byte) {
	if (offset >= length) {
		return length;
	}
	return (int64_t)next_byte(items, (size_t)length, (size_t)offset, (uint8_t)byte);
}

/// The low bit selects the low half of a four-byte code point; otherwise
/// position/2 is the next physical byte. Copies are independent cursors.
typedef struct {
	const uint8_t *text;
	size_t length;
	size_t position;
	size_t units;
} UnitReader;

static int32_t read_unit(UnitReader *reader) {
	size_t offset = reader->position / 2;
	if (offset == reader->length) { return -1; }
	size_t next = offset;
	uint32_t code = decode(reader->text, reader->length, &next);
	reader->units += 1;
	if (code < 0x10000) { reader->position = next * 2; return (int32_t)code; }
	code -= 0x10000;
	if (reader->position & 1) {
		reader->position = next * 2;
		return (int32_t)(0xDC00 + (code & 1023));
	}
	reader->position = offset * 2 + 1;
	return (int32_t)(0xD800 + (code >> 10));
}

static int match_unit_range(UnitReader text, const uint8_t *search, size_t length, UnitReader *after) {
	UnitReader needle = {search, length, 0, 0};
	int32_t unit;
	while ((unit = read_unit(&needle)) >= 0) {
		if (read_unit(&text) != unit) { return 0; }
	}
	if (after) { *after = text; }
	return 1;
}

static int matches_units(UnitReader text, const uint8_t *search, size_t length) {
	return match_unit_range(text, search, length, 0);
}

/// Only a low half at the start or a high half at the end can match inside
/// a canonical pair. Every other canonical needle is self-synchronizing UTF-8.
static int search_needs_units(const uint8_t *search, size_t length) {
	if (span_has_edges(search, length)) { return 1; }
	return length >= 3 &&
	       ((search[0] == 0xED && search[1] >= 0xB0) ||
	        (search[length - 3] == 0xED && search[length - 2] >= 0xA0 && search[length - 2] < 0xB0));
}

static __attribute__((noinline)) int64_t find_units(const uint8_t *text, size_t length,
    const uint8_t *search, size_t search_length, int last) {
	UnitReader candidate = {text, length, 0, 0};
	int64_t found = -1;
	for (;;) {
		if (matches_units(candidate, search, search_length)) {
			found = (int64_t)candidate.units;
			if (!last) { return found; }
		}
		if (read_unit(&candidate) < 0) { return found; }
	}
}

/// The byte offset of the first (or last) complete canonical match, or -1.
/// Callers route needles that can cut a pair through the unit reader.
static int64_t find_bytes(const uint8_t *text, size_t text_length, const uint8_t *search,
                          size_t search_length, int last) {
	if (search_length > text_length) { return -1; }
	if (search_length == 0) { return last ? (int64_t)text_length : 0; }
	if (!last) {
		size_t offset = next_byte(text, text_length, 0, search[0]);
		while (offset + search_length <= text_length) {
			if (bytes_at(text, text_length, offset, search, search_length)) { return (int64_t)offset; }
			offset = next_byte(text, text_length, offset + 1, search[0]);
		}
		return -1;
	}
	uint8_t first = search[0];
	size_t count = text_length - search_length + 1;
	for (size_t step = 0; step < count; step += 1) {
		size_t offset = count - 1 - step;
		if (text[offset] == first && bytes_at(text, text_length, offset, search, search_length)) {
			return (int64_t)offset;
		}
	}
	return -1;
}

FA_EXPORT int32_t fa_string_includes(const uint8_t *text, size_t text_length, size_t capacity,
                                     const uint8_t *search, size_t search_length) {
	if (!(capacity & SIMPLE) && search_needs_units(search, search_length)) {
		return find_units(text, text_length, search, search_length, 0) >= 0;
	}
	return find_bytes(text, text_length, search, search_length, 0) >= 0;
}

FA_EXPORT int32_t fa_string_starts_with(const uint8_t *text, size_t text_length, size_t capacity,
                                        const uint8_t *search, size_t search_length) {
	if (!(capacity & SIMPLE) && search_needs_units(search, search_length)) {
		return matches_units((UnitReader){text, text_length, 0, 0}, search, search_length);
	}
	return bytes_at(text, text_length, 0, search, search_length);
}

FA_EXPORT int32_t fa_string_ends_with(const uint8_t *text, size_t text_length, size_t capacity,
                                      const uint8_t *search, size_t search_length) {
	if (!(capacity & SIMPLE) && search_needs_units(search, search_length)) {
		size_t count = fa_string_length(text, text_length, capacity);
		size_t needed = unit_count(search, search_length);
		if (needed > count) { return 0; }
		UnitReader tail = {text, text_length, 0, 0};
		for (size_t index = needed; index < count; index += 1) { (void)read_unit(&tail); }
		return matches_units(tail, search, search_length);
	}
	return search_length <= text_length &&
	       bytes_at(text, text_length, text_length - search_length, search, search_length);
}

/// `indexOf` (`last` 0) and `lastIndexOf` (1): the UTF-16 position of the
/// match, or -1 for `none`.
static __attribute__((noinline)) int64_t string_index_of(const uint8_t *text, size_t text_length,
                                                          size_t capacity, const uint8_t *search,
                                                          size_t search_length, int32_t last) {
	size_t simple = capacity & SIMPLE;
	if (!simple && search_needs_units(search, search_length)) {
		return find_units(text, text_length, search, search_length, last);
	}
	int64_t offset = find_bytes(text, text_length, search, search_length, last);
	if (offset < 0 || simple) {
		return offset;
	}
	return unit_count(text, (size_t)offset);
}

FA_EXPORT int64_t fa_string_index_of(const uint8_t *text, size_t text_length, size_t capacity,
                                     const uint8_t *search, size_t search_length, int32_t last) {
	// common path, inlined: one byte in a short simple text (`digits.indexOf(c)`)
	if ((capacity & SIMPLE) && search_length == 1 && !last && text_length <= 32) {
		for (size_t offset = 0; offset < text_length; offset += 1) {
			if (text[offset] == search[0]) {
				return (int64_t)offset;
			}
		}
		return -1;
	}
	return string_index_of(text, text_length, capacity, search, search_length, last);
}

/// Non-overlapping byte matches in an ASCII source. Unicode callers use
/// the code-unit cursor; an empty needle visits every code-unit boundary.
static void each_ascii_match(const uint8_t *text, size_t text_length,
    const uint8_t *search, size_t search_length, void (*found)(void *, size_t), void *context) {
	if (!search_length) {
		for (size_t offset = 0; offset <= text_length; offset += 1) { found(context, offset); }
		return;
	}
	size_t offset = next_byte(text, text_length, 0, search[0]);
	while (offset + search_length <= text_length) {
		if (bytes_at(text, text_length, offset, search, search_length)) {
			found(context, offset);
			offset += search_length;
		} else { offset += 1; }
		offset = next_byte(text, text_length, offset, search[0]);
	}
}

typedef struct {
	const uint8_t *text;
	size_t previous;
	size_t search_length;
	const uint8_t *replacement;
	size_t replacement_length;
	int all;
	int done;
} Replacing;

static void replace_unicode(const uint8_t *text, size_t length, size_t capacity,
    const uint8_t *search, size_t search_length, const uint8_t *replacement,
    size_t replacement_length, int all, FaString *out);

static void replace_one(void *context, size_t offset) {
	Replacing *replacing = context;
	if (replacing->done) {
		return;
	}
	fa_b_bytes(replacing->text + replacing->previous, offset - replacing->previous);
	fa_b_string(replacing->replacement, replacing->replacement_length, 0);
	replacing->previous = offset + replacing->search_length;
	replacing->done = !replacing->all;
}

/// `replace` (the first match, `all` 0) and `replaceAll`.
FA_EXPORT void fa_string_replace(const uint8_t *text, size_t text_length, size_t capacity,
                                 const uint8_t *search, size_t search_length,
                                 const uint8_t *replacement, size_t replacement_length,
                                 int32_t all, FaString *out) {
	if (!(capacity & SIMPLE)) {
		replace_unicode(text, text_length, capacity, search, search_length,
		                replacement, replacement_length, all, out);
		return;
	}
	fa_b_reset();
	Replacing replacing = {text, 0, search_length, replacement, replacement_length, all, 0};
	each_ascii_match(text, text_length, search, search_length, replace_one,
	           &replacing);
	fa_b_bytes(text + replacing.previous, text_length - replacing.previous);
	fa_b_take(out);
}

const void *fa_error_index(int64_t index, int64_t length);
int64_t fa_string_code_point_count(const uint8_t *text, size_t length, size_t capacity);

/// The text between two offsets, as a new string.
static FaString slice(const uint8_t *text, size_t start, size_t end) {
	return owned_copy(text + start, end - start);
}

/// The header of an array (`src/lower/arrays.rs`).
typedef struct {
	uint8_t *items;
	int64_t length;
	int64_t capacity;
} FaArrayHeader;

void *fa_array_new(int64_t length, int64_t capacity, int64_t item_size);
void fa_array_materialize(FaArrayHeader *array);

void *fa_array_push_slot(void *array, int64_t item_size);
void fa_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size);

static void *split_unicode(const uint8_t *text, size_t length, size_t capacity,
    const uint8_t *separator, size_t separator_length, int views, int nonempty, void *parts);
static size_t count_unicode_parts(const uint8_t *text, size_t length,
    const uint8_t *separator, size_t separator_length, size_t limit);

typedef struct {
	const uint8_t *text;
	size_t previous;
	size_t separator_length;
	void *parts;
	size_t simple;
	int views;
	int nonempty;
	uint64_t raw_parts;
	uint64_t raw_capacity;
} Splitting;

/// A part of a split text: a copy, or a view of the text's bytes (the
/// `KIND_MASK` kind: never freed, copied by allocating) when the backend
/// proved that the text outlives the parts and isn't changed while they live.
static FaString split_part(const Splitting *splitting, size_t start, size_t end) {
	if (splitting->views) {
		FaString view = {splitting->text + start, end - start, KIND_MASK | splitting->simple};
		return view;
	}
	return owned_copy_with(splitting->text + start, end - start, splitting->simple);
}

static void split_one(void *context, size_t offset) {
	Splitting *splitting = context;
	if (splitting->raw_capacity && ++splitting->raw_parts > splitting->raw_capacity) {
		if (splitting->raw_capacity > INT64_MAX / 2) { out_of_memory(); }
		splitting->raw_capacity *= 2;
	}
	if (!splitting->nonempty || splitting->previous != offset) {
		FaString *part = fa_array_push_slot(splitting->parts, sizeof(FaString));
		*part = split_part(splitting, splitting->previous, offset);
	}
	splitting->previous = offset + splitting->separator_length;
}

/// The grapheme clusters of a text, an `Array(String)`: the characters of
/// a simple text point into the ASCII table, the others are copies.
// The backend fixes this at link time. Lazy materialization is currently
// enabled only for a single-file program without parallel or foreign code.
int32_t fa_compact_characters = 1;

static void *ascii_units(const uint8_t *text, size_t length) {
	if (!fa_compact_characters) {
		FaArrayHeader *array = fa_array_new((int64_t)length, (int64_t)length, sizeof(FaString));
		FaString *items = (FaString *)array->items;
		for (size_t i = 0; i < length; i += 1) { items[i] = ascii_string(text[i]); items[i].capacity = SIMPLE; }
		return array;
	}
	// An owned byte snapshot, decoded into static ASCII characters on
	// access. Negative capacity marks this compact character collection.
	FaArrayHeader *characters = fa_array_new((int64_t)length, (int64_t)length, 1);
	copy_bytes(characters->items, text, length);
	characters->capacity = -1;
	return characters;
}

static void *characters_of(const uint8_t *text, size_t length, size_t capacity) {
	if (capacity & SIMPLE) {
		size_t cr = next_byte(text, length, 0, '\r');
		while (cr + 1 < length && text[cr + 1] != '\n') {
			cr = next_byte(text, length, cr + 1, '\r');
		}
		if (cr + 1 >= length) { return ascii_units(text, length); }
	}
	void *characters = fa_array_new(0, (int64_t)fa_graphemes(text, length), sizeof(FaString));
	for (size_t index = 0; index < length;) {
		size_t next = next_boundary(text, length, index);
		FaString *part = fa_array_push_slot(characters, sizeof(FaString));
		*part = slice(text, index, next);
		index = next;
	}
	return characters;
}

/// `text.characters()`.
FA_EXPORT void *fa_string_characters(const uint8_t *text, size_t length, size_t capacity) {
	return characters_of(text, length, capacity);
}

#if !defined(__wasm__)
typedef uint8_t ByteVector __attribute__((vector_size(16)));

/// How many bytes of a text are `byte`: 16 at a time, into per-lane counters
/// summed every 255 steps (before a lane can overflow).
static size_t count_byte(const uint8_t *text, size_t length, uint8_t byte) {
	size_t count = 0;
	size_t index = 0;
	while (length - index >= 16) {
		size_t steps = (length - index) / 16;
		if (steps > 255) { steps = 255; }
		ByteVector lanes = {0};
		for (size_t step = 0; step < steps; step += 1, index += 16) {
			ByteVector chunk;
			__builtin_memcpy(&chunk, text + index, 16);
			lanes -= (ByteVector)(chunk == byte);
		}
		for (int lane = 0; lane < 16; lane += 1) { count += lanes[lane]; }
	}
	for (; index < length; index += 1) { count += text[index] == byte; }
	return count;
}
#endif

static void *split_text(const uint8_t *text, size_t length, size_t capacity,
                        const uint8_t *separator, size_t separator_length, int views, int nonempty) {
	if (!(capacity & SIMPLE)) {
		return split_unicode(text, length, capacity, separator, separator_length, views, nonempty, 0);
	}
	if (separator_length == 0) {
		return ascii_units(text, length);
	}
	size_t room = 4;
	uint64_t raw_capacity = nonempty ? room : 0;
#if !defined(__wasm__)
	// a one-byte separator: room for every occurrence (each match is one, so
	// the array never grows), counted 16 bytes at a time
	if (separator_length == 1) {
		room = 1 + count_byte(text, length, separator[0]);
		raw_capacity = 0;
	}
#endif
	Splitting splitting = {text, 0, separator_length,
	                       fa_array_new(0, (int64_t)room, sizeof(FaString)), capacity & SIMPLE,
	                       views, nonempty, 1, raw_capacity};
	each_ascii_match(text, length, separator, separator_length, split_one,
	           &splitting);
	if (!nonempty || splitting.previous != length) {
		FaString *last = fa_array_push_slot(splitting.parts, sizeof(FaString));
		*last = split_part(&splitting, splitting.previous, length);
	}
	// The original temporary filter preserves the split buffer's capacity.
	// Native one-byte splits already reserve the full raw count above; on
	// other paths reproduce its geometric growth even for discarded parts.
	if (splitting.raw_capacity) {
		fa_array_reserve(splitting.parts, (int64_t)splitting.raw_capacity, sizeof(FaString));
	}
	return splitting.parts;
}

/// `text.split(separator)`; an empty separator splits into code units.
FA_EXPORT void *fa_string_split(const uint8_t *text, size_t length, size_t capacity,
                                const uint8_t *separator, size_t separator_length) {
	return split_text(text, length, capacity, separator, separator_length, 0, 0);
}

/// `text.split(separator)` whose parts are views of the text's bytes
/// (`src/lower/views.rs`): `text` points to bytes that outlive the parts
/// unchanged (for an inline text, its snapshot in the caller's frame). An
/// empty separator still gives code units.
FA_EXPORT void *fa_string_split_views(const uint8_t *text, size_t length, size_t capacity,
                                      const uint8_t *separator, size_t separator_length) {
	return split_text(text, length, capacity, separator, separator_length, 1, 0);
}

/// A split followed by a pure, index-independent nonempty-part filter.
/// Keep the capacity of the unfiltered split, just as in-place filter does.
FA_EXPORT void *fa_string_split_nonempty(const uint8_t *text, size_t length, size_t capacity,
                                         const uint8_t *separator, size_t separator_length) {
	return split_text(text, length, capacity, separator, separator_length, 0, 1);
}

FA_EXPORT void *fa_string_split_nonempty_views(const uint8_t *text, size_t length, size_t capacity,
                                               const uint8_t *separator, size_t separator_length) {
	return split_text(text, length, capacity, separator, separator_length, 1, 1);
}

/// `text.codePoints()`: an `Array(UInt32)`.
FA_EXPORT void *fa_string_code_points(const uint8_t *text, size_t length, size_t capacity) {
	size_t count = (size_t)fa_string_code_point_count(text, length, capacity);
	FaArrayHeader *points = fa_array_new((int64_t)count, (int64_t)count, sizeof(uint32_t));
	uint32_t *items = (uint32_t *)points->items;
	if (count == length) {
		for (size_t index = 0; index < length; index += 1) {
			items[index] = text[index];
		}
		return points;
	}
	size_t item = 0;
	for (size_t index = 0; index < length;) {
		items[item] = decode(text, length, &index);
		item += 1;
	}
	return points;
}

/// `text.codePoints()[index]` and `.get(index)` without the array: the code
/// point at a position (negative from the end), or -1 when there is none.
FA_EXPORT int64_t fa_string_code_point_at(const uint8_t *text, size_t length, size_t capacity,
                                          int64_t index) {
	if (capacity & SIMPLE) {
		if (index < 0) {
			index += (int64_t)length;
		}
		return (uint64_t)index < (uint64_t)length ? (int64_t)text[index] : -1;
	}
	if (index < 0) {
		index += fa_string_code_point_count(text, length, capacity);
		if (index < 0) {
			return -1;
		}
	}
	for (size_t offset = 0; offset < length; index -= 1) {
		uint32_t code = decode(text, length, &offset);
		if (index == 0) {
			return code;
		}
	}
	return -1;
}

/// `for code in text.codePoints()`: the code point at `*offset` (in the
/// text), moving the offset past it.
FA_EXPORT uint32_t fa_next_code_point(const uint8_t *text, size_t length, int64_t *offset) {
	size_t index = (size_t)*offset;
	uint8_t first = text[index];
	if (first < 0x80) {
		*offset = (int64_t)index + 1;
		return first;
	}
	uint32_t code = decode(text, length, &index);
	*offset = (int64_t)index;
	return code;
}

/// The size of `text.codePoints()`.
FA_EXPORT int64_t fa_string_code_point_count(const uint8_t *text, size_t length, size_t capacity) {
	if (capacity & SIMPLE) {
		return (int64_t)length;
	}
	int64_t count = 0;
	for (size_t index = 0; index < length; index += 1) {
		count += (text[index] & 0xC0) != 0x80;
	}
	return count + span_low_head(text, length);
}

/// Counts UTF-16 code units directly in WTF-8, with no Unicode tables.
static __attribute__((noinline)) size_t unit_count(const uint8_t *text, size_t length) {
	size_t count = 0;
	for (size_t i = 0; i < length; i += 1) {
		count += (text[i] & 0xC0) != 0x80;
		count += text[i] >= 0xF0;
	}
	return count + span_low_head(text, length) - span_high_tail(text, length);
}

static StringHeader *string_metadata(const uint8_t *text, size_t length, size_t capacity) {
	(void)length;
	if (!string_owned(capacity)) { return 0; }
	StringHeader *header = string_header(text);
	spin_lock(&header->lock);
	return header;
}

static __attribute__((noinline)) size_t string_length_slow(const uint8_t *text, size_t length) {
	const StringHeader *header = string_header(text);
	// SIZE_MAX is reserved for an unpublished construction. Writers alone
	// publish this pair; concurrent readers modify only the other fields.
	if (header->bytes > length) { __builtin_trap(); }
	return header->units + (length - header->bytes);
}

FA_EXPORT __attribute__((always_inline)) size_t fa_string_length(const uint8_t *text, size_t length, size_t capacity) {
	if (__builtin_expect((capacity & SIMPLE) != 0, 1)) { return length; }
	if (!(capacity & STRING_INLINE) && (capacity & STRING_LITERAL)) { return capacity & KIND_MASK; }
	// A borrowed view has no cache to mutate. Keep that fact visible to
	// callers, so a length query cannot obscure their representation.
	return string_owned(capacity) ? string_length_slow(text, length)
	                              : unit_count(text, length);
}

/// Publish a count after an exclusive write, including external arena
/// transfers. Equal byte lengths do not imply an unchanged unit count.
FA_EXPORT __attribute__((always_inline)) void fa_string_set_unit_count(
    const uint8_t *bytes, size_t length, size_t capacity, size_t units) {
	if ((capacity & SIMPLE) || !string_owned(capacity)) { return; }
	StringHeader *header = string_header(bytes);
	string_clear(header);
	header->units = units;
	header->breadcrumbs = 0;
	header->bytes = length;
}

FA_EXPORT __attribute__((always_inline)) uint8_t *fa_string_write_arena_count(
    uint8_t *cursor, size_t units, size_t flags) {
	if (flags & SIMPLE) { return cursor; }
	uint16_t count = (uint16_t)units;
	__builtin_memcpy(cursor, &count, sizeof count);
	return cursor + sizeof count;
}

/// Inline arena text adds a two-byte count only on the Unicode path.
/// An external node retains its existing owning descriptor and eager cache.
FA_EXPORT __attribute__((always_inline)) void fa_string_read_arena_view(
    const uint8_t *node, size_t byte_length, size_t flags, int32_t packed, int32_t external,
    FaString *out) {
	const uint8_t *bytes;
	size_t units;
	if (external) {
		const FaString *owner = (const FaString *)(const void *)(node + (packed ? 8 : 16));
		bytes = owner->bytes;
		if (packed) { byte_length = owner->length; }
		units = flags & SIMPLE ? byte_length : fa_string_length(bytes, byte_length, owner->capacity);
	} else {
		bytes = node + (packed ? 4 : 16);
		units = byte_length;
		if (!(flags & SIMPLE)) {
			uint16_t count;
			__builtin_memcpy(&count, bytes, sizeof count);
			units = count;
			bytes += sizeof count;
		}
	}
	*out = (FaString){bytes, byte_length, string_view_capacity(flags, units)};
}

/// A position is twice its byte offset, plus one for the low half of an
/// astral code point. Breadcrumbs can therefore land inside a pair.
static size_t unit_position(const uint8_t *text, size_t length, size_t capacity, size_t index) {
	if (capacity & SIMPLE) { return index * 2; }
	size_t position = 0;
	size_t offset = 0;
	StringHeader *header = string_metadata(text, length, capacity);
	if (header) {
		size_t units = string_length_slow(text, length);
		if (header->breadcrumbs && index / 64 >= header->breadcrumbs[0]) {
			string_clear_crumbs(header);
			header->conversions = 1;
		}
		if (!header->breadcrumbs && ++header->conversions >= 2 && units >= 64) {
			size_t entries = units / 64 + 1;
			size_t *crumbs = fa_alloc(entries * sizeof(size_t), sizeof(size_t));
			crumbs[0] = entries;
			while (offset < length) {
				size_t start = offset;
				uint32_t code = decode(text, length, &offset);
				position += 1;
				if (position % 64 == 0) {
					crumbs[position / 64] = code >= 0x10000 ? start * 2 + 1 : offset * 2;
				}
				if (code >= 0x10000) {
					position += 1;
					if (position % 64 == 0) { crumbs[position / 64] = offset * 2; }
				}
			}
			header->breadcrumbs = crumbs;
		}
		position = 0;
		offset = 0;
		if (header->breadcrumbs) {
			size_t breadcrumb = index < 64 ? 0 : header->breadcrumbs[index / 64];
			position = index - index % 64;
			offset = breadcrumb / 2;
			if (breadcrumb & 1) {
				if (position == index) { spin_unlock(&header->lock); return breadcrumb; }
				offset += 4;
				position += 1;
			}
		}
		spin_unlock(&header->lock);
	}
	while (position < index) {
		size_t start = offset;
		uint32_t code = decode(text, length, &offset);
		position += 1;
		if (code >= 0x10000) {
			if (position == index) { return start * 2 + 1; }
			position += 1;
		}
	}
	return offset * 2;
}

static uint16_t unit_at_position(const uint8_t *text, size_t length, size_t position) {
	size_t offset = position / 2;
	uint32_t code = decode(text, length, &offset);
	if (code < 0x10000) { return (uint16_t)code; }
	code -= 0x10000;
	return (uint16_t)((position & 1) ? 0xDC00 + (code & 1023) : 0xD800 + (code >> 10));
}

FA_EXPORT __attribute__((noinline)) int64_t string_code_unit_slow(const uint8_t *text,
    size_t length, size_t capacity, int64_t index) {
	size_t count = fa_string_length(text, length, capacity);
	if (index < 0) { index += (int64_t)count; }
	if ((uint64_t)index >= count) { return -1; }
	return unit_at_position(text, length, unit_position(text, length, capacity, (size_t)index));
}

/// The lowering keeps the checked ASCII load separate from this fallback,
/// so a missing-unit sentinel never enters the successful byte path.
FA_EXPORT __attribute__((noinline, cold)) int64_t string_ascii_unit_fallback(const uint8_t *text,
    size_t length, int64_t index) {
	if (index < 0) { index += (int64_t)length; }
	return (uint64_t)index < length ? text[index] : -1;
}

static void unit_string(uint16_t unit, FaString *out) {
	if (unit < 0x80) { *out = ascii_string((uint8_t)unit); return; }
	*out = (FaString){0};
	uint8_t *bytes = (uint8_t *)out;
	size_t length;
	if (unit < 0x800) {
		bytes[0] = (uint8_t)(0xC0 | (unit >> 6));
		bytes[1] = (uint8_t)(0x80 | (unit & 63));
		length = 2;
	} else {
		bytes[0] = (uint8_t)(0xE0 | (unit >> 12));
		bytes[1] = (uint8_t)(0x80 | ((unit >> 6) & 63));
		bytes[2] = (uint8_t)(0x80 | (unit & 63));
		length = 3;
	}
	inline_string_tag(out, length, unit >= 0xD800 && unit <= 0xDFFF ? STRING_SURROGATES : 0);
}

static __attribute__((noinline)) int32_t string_get_slow(const uint8_t *text, size_t length,
    size_t capacity, int64_t index, FaString *out) {
	int64_t unit = string_code_unit_slow(text, length, capacity, index);
	if (unit < 0) { *out = (FaString){0}; return 0; }
	unit_string((uint16_t)unit, out);
	return 1;
}

FA_EXPORT int32_t fa_string_get(const uint8_t *text, size_t length, size_t capacity, int64_t index,
                                FaString *out) {
	if (capacity & SIMPLE) {
		if (index < 0) { index += (int64_t)length; }
		if ((uint64_t)index >= length) { *out = (FaString){0}; return 0; }
		if ((capacity & ~STRING_FLAGS) == 0) {
			*out = (FaString){text + index, 1, SIMPLE};
		} else { *out = ascii_string(text[index]); }
		return 1;
	}
	return string_get_slow(text, length, capacity, index, out);
}

/// A range whose physical positions and UTF-16 width are already known.
/// Search cursors use this without rescanning an earlier part of a view.
static void unit_slice_positions(const uint8_t *text, size_t length, size_t capacity,
    size_t first, size_t last, size_t units, FaString *out, int view) {
	if (!units) {
		*out = (FaString){view ? text + first / 2 : (const uint8_t *)"", 0,
		                  view ? string_view_capacity(capacity, 0) : SIMPLE};
		return;
	}
	if (view == 1) {
		// Retain the two low-half bytes or the three high-half bytes. They
		// remain inside the owner and readers expose their logical units.
		if (!fa_virtual_string_slices) { __builtin_trap(); }
		size_t start = first / 2 + ((first & 1) ? 2 : 0);
		size_t stop = last / 2 + ((last & 1) ? 3 : 0);
		*out = (FaString){text + start, stop - start, string_view_capacity(capacity, units)};
		return;
	}
	if (!(first & 1) && !(last & 1)) {
		size_t bytes = last / 2 - first / 2;
		if (view || (string_static(capacity) && bytes > 1)) {
			size_t tag = view ? string_view_capacity(capacity, units)
			                  : (capacity & SIMPLE) ? SIMPLE : STRING_LITERAL | units;
			*out = (FaString){text + first / 2, bytes, tag};
		} else { *out = owned_copy_with(text + first / 2, bytes, capacity & STRING_FLAGS); }
		return;
	}
	// Three bytes for a cut low/high half, plus the untouched middle.
	size_t start = first / 2 + ((first & 1) ? 4 : 0);
	size_t stop = last / 2;
	size_t middle = stop > start ? stop - start : 0;
	size_t encoded_middle = fa_string_encoded_length(text + start, middle, capacity);
	size_t size = encoded_middle + ((first & 1) ? 3 : 0) + ((last & 1) ? 3 : 0);
	uint8_t *bytes = fa_string_allocate(size, STRING_SURROGATES, out);
	size_t offset = 0;
	if (first & 1) {
		FaString unit;
		unit_string(unit_at_position(text, length, first), &unit);
		copy_bytes(bytes, (uint8_t *)&unit, 3);
		offset = 3;
	}
	fa_string_copy_bytes(bytes + offset, text + start, middle, capacity);
	offset += encoded_middle;
	if (last & 1) {
		FaString unit;
		unit_string(unit_at_position(text, length, last & ~(size_t)1), &unit);
		copy_bytes(bytes + offset, (uint8_t *)&unit, 3);
	}
	if (string_owned(out->capacity)) {
		StringHeader *header = string_header(out->bytes);
		header->units = units;
		header->breadcrumbs = 0;
		header->conversions = 0;
		header->bytes = size;
	}
}

/// Copy a code-unit range, encoding a cut pair boundary as a lone surrogate.
static void unit_slice(const uint8_t *text, size_t length, size_t capacity,
                       size_t from, size_t to, FaString *out, int view) {
	if (capacity & SIMPLE) {
		size_t bytes = to - from;
		if (view || (string_static(capacity) && bytes > 1)) {
			*out = (FaString){text + from, bytes, view ? SIMPLE | KIND_MASK : SIMPLE};
		} else { *out = owned_copy_with(text + from, bytes, SIMPLE); }
		return;
	}
	if (from == to && !view) { *out = (FaString){(const uint8_t *)"", 0, SIMPLE}; return; }
	size_t first = unit_position(text, length, capacity, from);
	size_t last = from == to ? first : unit_position(text, length, capacity, to);
	unit_slice_positions(text, length, capacity, first, last, to - from, out, view);
}

/// Append the selected units directly into the destination builder. At most
/// the two cut ends need synthetic bytes; no temporary owning slice is built.
static void append_unit_range(const uint8_t *text, size_t length, size_t capacity,
    size_t first, size_t last) {
	if (first == last) { return; }
	uint8_t edge[3];
	size_t start = first / 2 + ((first & 1) ? 4 : 0);
	size_t stop = last / 2;
	if (first & 1) {
		size_t size = encode_point(edge, unit_at_position(text, length, first));
		fa_b_bytes(edge, size);
	}
	if (stop > start) { fa_b_string(text + start, stop - start, capacity); }
	if (last & 1) {
		size_t size = encode_point(edge, unit_at_position(text, length, last & ~(size_t)1));
		fa_b_bytes(edge, size);
	}
}

static void replace_unicode(const uint8_t *text, size_t length, size_t capacity,
    const uint8_t *search, size_t search_length, const uint8_t *replacement,
    size_t replacement_length, int all, FaString *out) {
	fa_b_reset();
	UnitReader candidate = {text, length, 0, 0};
	size_t previous = 0;
	for (;;) {
		UnitReader after;
		if (match_unit_range(candidate, search, search_length, &after)) {
			append_unit_range(text, length, capacity, previous, candidate.position);
			fa_b_string(replacement, replacement_length, 0);
			previous = after.position;
			if (!all) { break; }
			candidate = after;
			if (search_length) { continue; }
		}
		if (read_unit(&candidate) < 0) { break; }
	}
	append_unit_range(text, length, capacity, previous, length * 2);
	fa_b_take(out);
}

static size_t count_unicode_parts(const uint8_t *text, size_t length,
    const uint8_t *separator, size_t separator_length, size_t limit) {
	if (!separator_length) { return unit_count(text, length); }
	UnitReader candidate = {text, length, 0, 0};
	size_t count = 1;
	for (;;) {
		UnitReader after;
		if (match_unit_range(candidate, separator, separator_length, &after)) {
			if (++count > limit) { return count; }
			candidate = after;
			continue;
		}
		if (read_unit(&candidate) < 0) { return count; }
	}
}

static void split_unit_part(void *parts, const uint8_t *text, size_t length, size_t capacity,
    size_t first, size_t last, size_t units, int views) {
	FaString *part = fa_array_push_slot(parts, sizeof(FaString));
	unit_slice_positions(text, length, capacity, first, last, units, part, views ? 2 : 0);
}

static void *split_unicode(const uint8_t *text, size_t length, size_t capacity,
    const uint8_t *separator, size_t separator_length, int views, int nonempty, void *parts) {
	size_t room = 4;
	uint64_t raw_capacity = nonempty ? room : 0, raw_parts = 1;
	if (!separator_length) { room = fa_string_length(text, length, capacity); raw_capacity = 0; }
#if !defined(__wasm__)
	else if (separator_length == 1) { room = 1 + count_byte(text, length, separator[0]); raw_capacity = 0; }
#endif
	if (!parts) { parts = fa_array_new(0, (int64_t)room, sizeof(FaString)); }
	else { raw_capacity = 0; }
	UnitReader candidate = {text, length, 0, 0};
	size_t previous = 0, previous_units = 0;
	if (!separator_length) {
		while (read_unit(&candidate) >= 0) {
			split_unit_part(parts, text, length, capacity, previous, candidate.position, 1, views);
			previous = candidate.position;
		}
		return parts;
	}
	for (;;) {
		UnitReader after;
		if (match_unit_range(candidate, separator, separator_length, &after)) {
			if (raw_capacity && ++raw_parts > raw_capacity) { raw_capacity *= 2; }
			if (!nonempty || candidate.units != previous_units) {
				split_unit_part(parts, text, length, capacity, previous, candidate.position,
				                candidate.units - previous_units, views);
			}
			previous = after.position;
			previous_units = after.units;
			candidate = after;
			continue;
		}
		if (read_unit(&candidate) < 0) { break; }
	}
	if (!nonempty || candidate.units != previous_units) {
		split_unit_part(parts, text, length, capacity, previous, candidate.position,
		                candidate.units - previous_units, views);
	}
	if (raw_capacity) { fa_array_reserve(parts, (int64_t)raw_capacity, sizeof(FaString)); }
	return parts;
}

static __attribute__((always_inline)) const void *slice_bounds(int64_t count,
    int64_t start, int64_t end, int8_t flags, int64_t *from, int64_t *to) {
	*from = 0;
	*to = count;
	if (flags & 1) {
		if (start < -count || start > count) { return fa_error_index(start, count); }
		*from = start < 0 ? start + count : start;
	}
	if (flags & 2) {
		int inclusive = (flags & 4) != 0;
		if (end < -count || end > count - inclusive) { return fa_error_index(end, count); }
		*to = (end < 0 ? end + count : end) + inclusive;
	}
	if (*to < *from) { *to = *from; }
	return 0;
}

static const void *string_slice(const uint8_t *text, size_t length, size_t capacity,
                               int64_t start, int64_t end, int8_t flags, FaString *out,
                               int view) {
	int64_t from, to;
	const void *error = slice_bounds((int64_t)fa_string_length(text, length, capacity),
	                                start, end, flags, &from, &to);
	if (error) { return error; }
	unit_slice(text, length, capacity, (size_t)from, (size_t)to, out, view);
	return 0;
}

FA_EXPORT const void *fa_string_slice(const uint8_t *text, size_t length, size_t capacity,
                                      int64_t start, int64_t end, int8_t flags, FaString *out) {
	return string_slice(text, length, capacity, start, end, flags, out, 0);
}

FA_EXPORT const void *fa_string_slice_view(const uint8_t *text, size_t length, size_t capacity,
                                           int64_t start, int64_t end, int8_t flags, FaString *out) {
	return string_slice(text, length, capacity, start, end, flags, out, 1);
}

FA_EXPORT const void *fa_string_slice_region(const uint8_t *text, size_t length, size_t capacity,
                                             int64_t start, int64_t end, int8_t flags, FaString *out) {
	if (capacity & SIMPLE) {
		int64_t from, to;
		const void *error = slice_bounds((int64_t)length, start, end, flags, &from, &to);
		if (error) { return error; }
		size_t size = (size_t)(to - from);
		if (size > INLINE_STRING_BYTES) { *out = owned_copy_with(text + from, size, SIMPLE); }
		else { *out = (FaString){text + from, size, SIMPLE | KIND_MASK}; }
		return 0;
	}
	const void *error = string_slice(text, length, capacity, start, end, flags, out, 2);
	if (!error && string_length(out) > INLINE_STRING_BYTES &&
	    ((out->capacity & KIND_MASK) == KIND_MASK || string_counted_view(out->capacity))) {
		*out = owned_copy_with(string_bytes(out), string_length(out), out->capacity & STRING_FLAGS);
	}
	// Only a short immediate result may borrow a nonstatic source here.
	if (!error && string_length(out) <= INLINE_STRING_BYTES && !(out->capacity & STRING_INLINE)) {
		out->capacity = string_view_capacity(out->capacity, fa_string_length(string_bytes(out), string_length(out), out->capacity));
	}
	return error;
}

/// Appends the UTF-8 encoding of a code point.
static void append_code_point(uint32_t code) {
	if (code < 0x80) {
		append_byte((uint8_t)code);
	} else if (code < 0x800) {
		append_byte((uint8_t)(0xC0 | (code >> 6)));
		append_byte((uint8_t)(0x80 | (code & 0x3F)));
	} else if (code < 0x10000) {
		append_byte((uint8_t)(0xE0 | (code >> 12)));
		append_byte((uint8_t)(0x80 | ((code >> 6) & 0x3F)));
		append_byte((uint8_t)(0x80 | (code & 0x3F)));
	} else {
		append_byte((uint8_t)(0xF0 | (code >> 18)));
		append_byte((uint8_t)(0x80 | ((code >> 12) & 0x3F)));
		append_byte((uint8_t)(0x80 | ((code >> 6) & 0x3F)));
		append_byte((uint8_t)(0x80 | (code & 0x3F)));
	}
}

// ── case conversion ─────────────────────────────────────────────────────

// The full Unicode tables, generated from the JavaScript engine's by
// `case_tables.js`: native code converts case as JavaScript does.
#include "fa_case_tables.h"

#define COUNT_OF(table) (sizeof(table) / sizeof((table)[0]))

/// The single code point a code point maps to (itself when unchanged).
static uint32_t case_run(uint32_t code, const CaseRun *runs, size_t count) {
	size_t low = 0;
	size_t high = count;
	while (low < high) {
		size_t middle = (low + high) / 2;
		const CaseRun *run = &runs[middle];
		if (code < run->first) {
			high = middle;
		} else if (code >= run->first + (uint32_t)run->count * run->stride) {
			low = middle + 1;
		} else {
			if ((code - run->first) % run->stride == 0) {
				return (uint32_t)((int32_t)code + run->delta);
			}
			return code;
		}
	}
	return code;
}

/// The UTF-8 text a code point maps to when it maps to several, or null.
static const char *case_special(uint32_t code, const CaseSpecial *specials, size_t count) {
	size_t low = 0;
	size_t high = count;
	while (low < high) {
		size_t middle = (low + high) / 2;
		if (code < specials[middle].code) {
			high = middle;
		} else if (code > specials[middle].code) {
			low = middle + 1;
		} else {
			return specials[middle].text;
		}
	}
	return 0;
}

/// Whether a capital sigma at `start..end` ends a word (Unicode's
/// Final_Sigma): a cased letter before it and none after it, skipping the
/// case-ignorable code points.
static int final_sigma(const uint8_t *bytes, size_t length, size_t start, size_t end) {
	int before = 0;
	size_t index = start;
	while (index > 0) {
		size_t previous = index - 1;
		while (previous > 0 && (bytes[previous] & 0xC0) == 0x80) {
			previous -= 1;
		}
		size_t cursor = previous;
		uint32_t code = decode(bytes, length, &cursor);
		index = previous;
		if (!in_ranges(code, CASE_IGNORABLE, COUNT_OF(CASE_IGNORABLE))) {
			before = in_ranges(code, CASED, COUNT_OF(CASED));
			break;
		}
	}
	if (!before) {
		return 0;
	}
	index = end;
	while (index < length) {
		uint32_t code = decode(bytes, length, &index);
		if (!in_ranges(code, CASE_IGNORABLE, COUNT_OF(CASE_IGNORABLE))) {
			return !in_ranges(code, CASED, COUNT_OF(CASED));
		}
	}
	return 1;
}

/// `toUpperCase` (`upper` 1) and `toLowerCase` (0), with the full Unicode
/// tables: the mappings to several code points (`ß` → `SS`) and the final
/// sigma included. ASCII text takes a fast path.
FA_EXPORT void fa_string_case(const uint8_t *bytes, size_t length, int32_t upper, FaString *out) {
	fa_b_reset();
	const CaseRun *runs = upper ? UPPER_RUNS : LOWER_RUNS;
	size_t run_count = upper ? COUNT_OF(UPPER_RUNS) : COUNT_OF(LOWER_RUNS);
	const CaseSpecial *specials = upper ? UPPER_SPECIAL : LOWER_SPECIAL;
	size_t special_count = upper ? COUNT_OF(UPPER_SPECIAL) : COUNT_OF(LOWER_SPECIAL);
	size_t index = 0;
	while (index < length) {
		uint8_t byte = bytes[index];
		if (byte < 0x80) {
			if (upper && byte >= 'a' && byte <= 'z') {
				byte -= 0x20;
			} else if (!upper && byte >= 'A' && byte <= 'Z') {
				byte += 0x20;
			}
			append_byte(byte);
			index += 1;
			continue;
		}
		size_t start = index;
		uint32_t code = decode(bytes, length, &index);
		if (!upper && code == 0x3A3) {
			append_code_point(final_sigma(bytes, length, start, index) ? 0x3C2 : 0x3C3);
			continue;
		}
		const char *special = case_special(code, specials, special_count);
		if (special != 0) {
			append_c(special);
			continue;
		}
		append_code_point(case_run(code, runs, run_count));
	}
	fa_b_take(out);
}

/// The white space that `trim` removes (ECMAScript's WhiteSpace and LineTerminator).
static int is_space(uint32_t code) {
	return (code >= 0x09 && code <= 0x0D) || code == 0x20 || code == 0xA0 || code == 0x1680 ||
	       (code >= 0x2000 && code <= 0x200A) || code == 0x2028 || code == 0x2029 ||
	       code == 0x202F || code == 0x205F || code == 0x3000 || code == 0xFEFF;
}

/// `trim` (`sides` 3), `trimStart` (1), `trimEnd` (2): a view of the text.
FA_EXPORT void fa_string_trim(const uint8_t *bytes, size_t length, int32_t sides, FaString *out) {
	size_t start = 0;
	size_t end = length;
	if (sides & 1) {
		while (start < end) {
			size_t next = start;
			if (!is_space(decode(bytes, end, &next))) {
				break;
			}
			start = next;
		}
	}
	if (sides & 2) {
		while (end > start) {
			size_t previous = end - 1;
			while (previous > start && (bytes[previous] & 0xC0) == 0x80) {
				previous -= 1;
			}
			size_t cursor = previous;
			if (!is_space(decode(bytes, end, &cursor))) {
				break;
			}
			end = previous;
		}
	}
	*out = owned_copy(bytes + start, end - start);
}

/// `text.repeat(count)`.
FA_EXPORT void fa_string_repeat(const uint8_t *bytes, size_t length, int64_t count, FaString *out) {
	// Check the identity before any pointer-width conversion (wasm32 also
	// accepts an Integer repetition count much wider than size_t).
	if (length == 0 || count <= 0) {
		*out = (FaString){(const uint8_t *)"", 0, SIMPLE};
		return;
	}
	size_t encoded = fa_string_encoded_length(bytes, length, 0);
	if ((uint64_t)count > (KIND_MASK - 1) / encoded) { out_of_memory(); }
	fa_b_reset();
	for (int64_t index = 0; index < count; index += 1) {
		fa_b_string(bytes, length, 0);
	}
	fa_b_take(out);
}

/// `items.join(separator)` on an array of strings.
FA_EXPORT void fa_array_join(const void *header, const uint8_t *separator,
                             size_t separator_length, FaString *out) {
	const FaArrayHeader *array = header;
	fa_array_materialize((FaArrayHeader *)array);
	const FaString *items = (const FaString *)array->items;
	join_strings(items, (size_t)array->length, separator, separator_length, out);
}

static __attribute__((noinline)) size_t join_unit_length(const FaString *parts, size_t count,
                                                        size_t index, size_t bytes, size_t units) {
	for (; index < count; index += 1) {
		size_t physical = string_length(parts + index);
		size_t size = fa_string_encoded_length(string_bytes(parts + index), physical, parts[index].capacity);
		bytes = fa_string_size(bytes, size);
		units += fa_string_length(string_bytes(parts + index), physical, parts[index].capacity);
	}
	return units;
}

/// UTF-16 lengths are additive even when a join reunites surrogate halves.
/// Keep the ASCII loop as a single checked sum; a non-ASCII suffix also
/// measures units, retaining byte-size overflow checks without an output.
FA_EXPORT size_t fa_array_join_length(void *header, const uint8_t *separator, size_t separator_length) {
	FaArrayHeader *array = header;
	fa_array_materialize(array);
	size_t count = (size_t)array->length;
	size_t separator_encoded = fa_string_encoded_length(separator, separator_length, 0);
	if (separator_encoded && count > 1 && count - 1 >= KIND_MASK / separator_encoded) { out_of_memory(); }
	size_t length = count > 1 ? (count - 1) * separator_encoded : 0;
	const FaString *parts = (const FaString *)array->items;
	if (count > 1 && !simple_flag(separator, separator_length)) {
		return join_unit_length(parts, count, 0, length, (count - 1) * unit_count(separator, separator_length));
	}
	for (size_t i = 0; i < count; i += 1) {
		size_t size = string_length(parts + i);
		if (!(parts[i].capacity & SIMPLE)) { return join_unit_length(parts, count, i, length, length); }
		length = fa_string_size(length, size);
	}
	return length;
}

/// Padding repeats and cuts the filler at exactly the requested UTF-16 width.
static __attribute__((noinline)) void string_pad_slow(const uint8_t *bytes, size_t length, size_t capacity, int64_t width,
                             const uint8_t *filler, size_t filler_length, int32_t at_start,
                             FaString *out) {
	size_t units = fa_string_length(bytes, length, capacity);
	if ((uint64_t)width <= units || !filler_length) {
		*out = owned_copy_with(bytes, length, capacity & STRING_FLAGS);
		return;
	}
	if ((uint64_t)width >= KIND_MASK) { out_of_memory(); }
	size_t missing = (size_t)width - units;
	size_t filler_units = unit_count(filler, filler_length);
	size_t repeats = missing / filler_units;
	size_t remainder = missing % filler_units;
	UnitReader tail = {filler, filler_length, 0, 0};
	while (tail.units < remainder) { (void)read_unit(&tail); }
	size_t encoded = fa_string_encoded_length(bytes, length, capacity);
	size_t filler_encoded = fa_string_encoded_length(filler, filler_length, 0);
	if (repeats && filler_encoded > (KIND_MASK - 1 - encoded) / repeats) { out_of_memory(); }
	size_t tail_encoded = fa_string_encoded_length(filler, tail.position / 2, 0) + ((tail.position & 1) ? 3 : 0);
	size_t total = fa_string_size(encoded + repeats * filler_encoded, tail_encoded);
	fa_b_reset();
	reserve(total);
	if (!at_start) {
		fa_b_string(bytes, length, capacity);
	}
	for (size_t index = 0; index < repeats; index += 1) {
		fa_b_string(filler, filler_length, 0);
	}
	append_unit_range(filler, filler_length, 0, 0, tail.position);
	if (at_start) {
		fa_b_string(bytes, length, capacity);
	}
	fa_b_take(out);
}

FA_EXPORT __attribute__((always_inline)) void fa_string_pad(const uint8_t *bytes, size_t length, size_t capacity, int64_t width,
                             const uint8_t *filler, size_t filler_length, int32_t at_start,
                             FaString *out) {
	if (width <= 0) { *out = owned_copy_with(bytes, length, capacity & SIMPLE); return; }
	if (!(capacity & SIMPLE) || filler_length != 1 || filler[0] >= 128) {
		string_pad_slow(bytes, length, capacity, width, filler, filler_length, at_start, out);
		return;
	}
	if ((uint64_t)width <= length) { *out = owned_copy_with(bytes, length, SIMPLE); return; }
	if ((uint64_t)width >= KIND_MASK) { out_of_memory(); }
	size_t total = (size_t)width;
	size_t missing = total - length;
	FaString result = {0};
	size_t block = string_block(total);
	uint8_t *target = total <= INLINE_STRING_BYTES ? (uint8_t *)&result : string_alloc(&block, SIMPLE);
	if (at_start) {
		fill_bytes(target, filler[0], missing);
		copy_bytes(target + missing, bytes, length);
	} else {
		copy_bytes(target, bytes, length);
		fill_bytes(target + length, filler[0], missing);
	}
	if (total <= INLINE_STRING_BYTES) { inline_string_tag(&result, total, SIMPLE); }
	else { result.bytes = target; result.length = total; result.capacity = block | SIMPLE; }
	*out = result;
}

// ── UTF-8 input ─────────────────────────────────────────────────────────

typedef struct {
	size_t units;
	size_t error; // SIZE_MAX on success; otherwise the first invalid subpart
} Utf8Info;

/// Strict decoding and replacement decoding share the same byte consumption.
/// A bad continuation is left for the next subpart; valid continuations before
/// it belong to this one. In particular ED A0 80 produces three replacements.
static uint32_t utf8_decode(const uint8_t *bytes, size_t length, size_t *position, int *valid) {
	uint8_t first = bytes[(*position)++];
	*valid = 1;
	if (first < 128) { return first; }
	unsigned width;
	uint32_t code;
	if (first >= 0xC2 && first <= 0xDF) { width = 2; code = first & 31; }
	else if (first >= 0xE0 && first <= 0xEF) { width = 3; code = first & 15; }
	else if (first >= 0xF0 && first <= 0xF4) { width = 4; code = first & 7; }
	else { *valid = 0; return 0xFFFD; }
	for (unsigned index = 1; index < width; index += 1) {
		uint8_t lower = 0x80, upper = 0xBF;
		if (index == 1) {
			if (first == 0xE0) { lower = 0xA0; }
			if (first == 0xED) { upper = 0x9F; }
			if (first == 0xF0) { lower = 0x90; }
			if (first == 0xF4) { upper = 0x8F; }
		}
		if (*position == length || bytes[*position] < lower || bytes[*position] > upper) {
			*valid = 0;
			return 0xFFFD;
		}
		code = (code << 6) | (bytes[(*position)++] & 63);
	}
	return code;
}

#if !defined(__wasm__)
static unsigned utf8_lane_mask(ByteVector lanes) {
#if defined(__x86_64__)
	typedef char SignedByteVector __attribute__((vector_size(16)));
	return (unsigned)__builtin_ia32_pmovmskb128((SignedByteVector)lanes);
#else
	uint64_t words[2];
	__builtin_memcpy(words, &lanes, sizeof words);
	uint64_t low = (words[0] >> 7) & UINT64_C(0x0101010101010101);
	uint64_t high = (words[1] >> 7) & UINT64_C(0x0101010101010101);
	return (unsigned)((low * UINT64_C(0x0102040810204080)) >> 56)
	     | (unsigned)((high * UINT64_C(0x0102040810204080)) >> 56) << 8;
#endif
}

/// Validate a complete prefix of a 16-byte block. Leave a trailing partial
/// sequence for the next block; every vector therefore starts at a boundary.
static size_t utf8_vector(const uint8_t *bytes, size_t *units) {
	ByteVector chunk;
	__builtin_memcpy(&chunk, bytes, sizeof chunk);
	unsigned non_ascii = utf8_lane_mask(chunk);
	if (non_ascii == 0) { *units = 16; return 16; }
	unsigned continuation = utf8_lane_mask((chunk >= 0x80) & (chunk <= 0xBF));
	unsigned two = utf8_lane_mask((chunk >= 0xC2) & (chunk <= 0xDF));
	unsigned three = utf8_lane_mask((chunk >= 0xE0) & (chunk <= 0xEF));
	unsigned four = utf8_lane_mask((chunk >= 0xF0) & (chunk <= 0xF4));
	unsigned expected = ((two | three | four) << 1) | ((three | four) << 2) | (four << 3);
	unsigned invalid = non_ascii & ~(continuation | two | three | four);
	invalid |= continuation ^ expected;
	invalid |= (utf8_lane_mask(chunk == 0xE0) << 1) & utf8_lane_mask(chunk < 0xA0);
	invalid |= (utf8_lane_mask(chunk == 0xED) << 1) & utf8_lane_mask(chunk > 0x9F);
	invalid |= (utf8_lane_mask(chunk == 0xF0) << 1) & utf8_lane_mask(chunk < 0x90);
	invalid |= (utf8_lane_mask(chunk == 0xF4) << 1) & utf8_lane_mask(chunk > 0x8F);
	size_t tail = 15;
	while (tail > 12 && (bytes[tail] & 0xC0) == 0x80) { tail -= 1; }
	uint8_t lead = bytes[tail];
	size_t width = lead >= 0xF0 && lead <= 0xF4 ? 4
	             : lead >= 0xE0 && lead <= 0xEF ? 3
	             : lead >= 0xC2 && lead <= 0xDF ? 2 : 1;
	size_t consumed = tail + width > 16 ? tail : 16;
	unsigned mask = (1U << consumed) - 1;
	// A new lead at the tail cannot hide an unfinished preceding sequence.
	if ((invalid & mask) || (consumed < 16 && (expected & (1U << consumed)))) { return 0; }
	*units = consumed - (size_t)__builtin_popcount(continuation & mask)
	                  + (size_t)__builtin_popcount(four & mask);
	return consumed;
}
#endif

/// Validation computes the UTF-16 count and ASCII fact in the same pass.
/// Native targets use 16-byte vectors; wasm retains its portable word ASCII
/// path and scalar Unicode decoder without requiring the SIMD128 extension.
static Utf8Info utf8_validate(const uint8_t *bytes, size_t length) {
	Utf8Info result = {0, SIZE_MAX};
	size_t position = 0;
#if !defined(__wasm__)
	while (length - position >= 16) {
		size_t units;
		size_t consumed = utf8_vector(bytes + position, &units);
		if (!consumed) { break; }
		position += consumed;
		result.units += units;
	}
#else
	while (length - position >= 8) {
		uint64_t word;
		__builtin_memcpy(&word, bytes + position, sizeof word);
		if (word & UINT64_C(0x8080808080808080)) { break; }
		position += 8;
		result.units += 8;
	}
#endif
	while (position < length) {
		size_t start = position;
		int valid;
		uint32_t code = utf8_decode(bytes, length, &position, &valid);
		if (!valid) { result.error = start; return result; }
		result.units += 1 + (code > 0xFFFF);
	}
	return result;
}

static void utf8_copy_valid(const uint8_t *bytes, size_t length, size_t units, FaString *out) {
	uint8_t *target = fa_string_allocate(length, units == length ? SIMPLE : 0, out);
	copy_bytes(target, bytes, length);
	fa_string_set_unit_count(target, length, out->capacity, units);
}

static void utf8_copy_lossy(const uint8_t *bytes, size_t length, Utf8Info info, FaString *out) {
	if (info.error == SIZE_MAX) { utf8_copy_valid(bytes, length, info.units, out); return; }
	size_t encoded = info.error, units = info.units, position = info.error;
	while (position < length) {
		int valid;
		uint32_t code = utf8_decode(bytes, length, &position, &valid);
		encoded = fa_string_size(encoded, code < 128 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4);
		units += 1 + (code > 0xFFFF);
	}
	uint8_t *target = fa_string_allocate(encoded, 0, out);
	copy_bytes(target, bytes, info.error);
	size_t written = info.error;
	position = info.error;
	while (position < length) {
		int valid;
		uint32_t code = utf8_decode(bytes, length, &position, &valid);
		written += encode_point(target + written, code);
	}
	fa_string_set_unit_count(target, encoded, out->capacity, units);
}

/// External nonthrowing text inputs and String.fromUtf8Lossy share this path.
FA_EXPORT void fa_string_from_utf8_lossy(const uint8_t *bytes, size_t length, FaString *out) {
	utf8_copy_lossy(bytes, length, utf8_validate(bytes, length), out);
}

// ── arguments ───────────────────────────────────────────────────────────

static FaString *argument_items;
#if defined(__wasm__)
static uint8_t *argument_bytes;
static size_t argument_bytes_size;
#endif
static size_t argument_count;
static int arguments_have_owners;

static FaString argument_string(const uint8_t *bytes, size_t length) {
	Utf8Info info = utf8_validate(bytes, length);
	if (info.error == SIZE_MAX) {
		return (FaString){bytes, length, info.units == length ? SIMPLE : STRING_LITERAL | info.units};
	}
	FaString result;
	fa_string_from_utf8_lossy(bytes, length, &result);
	arguments_have_owners |= string_owned(result.capacity);
	return result;
}

#if !defined(__wasm__)
/// Keeps the user arguments: `argv` without the program (`argv[0]`).
FA_EXPORT __attribute__((noinline, cold)) void fa_arguments_init(int32_t argc, char **argv) {
	argument_count = argc > 1 ? (size_t)argc - 1 : 0;
	argument_items = fa_alloc(sizeof(FaString) * (argument_count + 1), sizeof(void *));
	for (size_t index = 0; index < argument_count; index += 1) {
		const char *text = argv[index + 1];
		argument_items[index] = argument_string((const uint8_t *)text, c_length(text));
	}
}
#else
/// Asks the host for the user arguments.
FA_EXPORT __attribute__((noinline, cold)) void fa_arguments_init(int32_t argc, char **argv) {
	(void)argc;
	(void)argv;
	size_t size = (size_t)fa_host_arguments_size();
	uint8_t *bytes = fa_alloc(size == 0 ? 1 : size, 1);
	argument_bytes = bytes;
	argument_bytes_size = size == 0 ? 1 : size;
	fa_host_arguments_copy(bytes);
	argument_count = 0;
	for (size_t index = 0; index < size; index += 1) {
		argument_count += bytes[index] == 0;
	}
	argument_items = fa_alloc(sizeof(FaString) * (argument_count + 1), sizeof(void *));
	size_t start = 0;
	size_t item = 0;
	for (size_t index = 0; index < size; index += 1) {
		if (bytes[index] == 0) {
			argument_items[item] = argument_string(bytes + start, index - start);
			item += 1;
			start = index + 1;
		}
	}
}
#endif

// ── arrays ──────────────────────────────────────────────────────────────

/// The offset of the items stored in the block of their header: a small
/// array is one allocation, its items right after the header (aligned on 16).
#define INLINE_ITEMS ((size_t)32)
#define INLINE_LIMIT ((size_t)96)

// The marker, the word after the header: 0 for a heap array whose items
// have a block of their own, 1 when they follow the header; then the kinds
// below. A region array (`ARRAY_REGION`) keeps the size of its items above
// its flags; the storage kinds are those of the other arrays.
#define ARRAY_REGION ((uint64_t)2)
#define ARRAY_REGION_HEAP ((uint64_t)4)
#define ARRAY_REGION_CLEANUP ((uint64_t)8)
#define ARRAY_REGION_LOCAL ((uint64_t)16)
#define ARRAY_REGION_SIZE_SHIFT 5
/// An immutable array of read-only data (`ARRAY_STATIC_MARKER` of
/// `src/lower/arrays.rs`): neither its items nor its blocks are freed.
#define ARRAY_STATIC ((uint64_t)32)
/// An array in a function's frame (`ARRAY_FRAME_MARKER`): its items follow
/// its header and are dropped, its block is never freed.
#define ARRAY_FRAME ((uint64_t)64)
#define ARRAY_BORROWED ((uint64_t)256)

static void region_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size);
void *fa_array_into_region(void *header, int64_t item_size);

/// Whether the items of an array are in the block of its header: the word
/// after the header says so (headers take 32 bytes, so that it exists).
static int inline_items(const FaArrayHeader *array) {
	uint64_t marker;
	__builtin_memcpy(&marker, (const uint8_t *)array + sizeof(FaArrayHeader), 8);
	return marker != 0 && array->items == (const uint8_t *)array + INLINE_ITEMS;
}

static uint64_t array_marker(const FaArrayHeader *array) {
	uint64_t marker;
	__builtin_memcpy(&marker, (const uint8_t *)array + sizeof(FaArrayHeader), 8);
	return marker;
}

FA_EXPORT __attribute__((always_inline)) int32_t fa_array_is_region(const FaArrayHeader *array) {
	return array != 0 && (array_marker(array) & ARRAY_REGION) != 0;
}

static void mark_items(FaArrayHeader *array, uint64_t marker) {
	__builtin_memcpy((uint8_t *)array + sizeof(FaArrayHeader), &marker, 8);
}

/// Where an array lives: 0 on the heap or in a region (its owner frees
/// it), 2 in read-only data, 3 in a function's frame.
FA_EXPORT __attribute__((always_inline)) int32_t fa_array_storage(const FaArrayHeader *array) {
	uint64_t marker = array_marker(array);
	if (marker & ARRAY_REGION) {
		return 0;
	}
	return marker & ARRAY_STATIC ? 2 : marker & ARRAY_FRAME ? 3 : 0;
}

/// Frees the block of the items of an array, unless it is the header's.
/// Called before the items move to another block: the marker is cleared,
/// since that block may happen to follow the header.
static void free_array_items(FaArrayHeader *array, int64_t item_size) {
	uint64_t marker = array_marker(array);
	if (marker & ARRAY_REGION) {
		if (marker & ARRAY_REGION_HEAP) {
			fa_free(array->items, (size_t)(array->capacity * item_size), 16);
			mark_items(array, marker & ~ARRAY_REGION_HEAP);
		}
		return;
	}
	if (inline_items(array)) {
		mark_items(array, 0);
	} else {
		fa_free(array->items, (size_t)(array->capacity < 0 ? array->length : array->capacity * item_size), 16);
	}
}

/// A new array of `length` items (not initialized), with room for `capacity`.
FA_EXPORT void *fa_array_new(int64_t length, int64_t capacity, int64_t item_size) {
	if (capacity < length) {
		capacity = length;
	}
	size_t bytes = (size_t)(capacity * item_size);
	if (bytes <= INLINE_LIMIT) {
		FaArrayHeader *array = fa_alloc(INLINE_ITEMS + bytes, 16);
		mark_items(array, 1);
		array->items = (uint8_t *)array + INLINE_ITEMS;
		array->length = length;
		array->capacity = capacity;
		return array;
	}
	FaArrayHeader *array = fa_alloc(INLINE_ITEMS, sizeof(int64_t));
	mark_items(array, 0);
	array->items = fa_alloc(bytes, 16);
	array->length = length;
	array->capacity = capacity;
	return array;
}

/// Select the final storage before constructing any items. The caller offers
/// a fixed, aligned buffer; no input-dependent stack allocation or copy-out.
FA_EXPORT __attribute__((always_inline)) void *fa_array_new_into(
    int64_t length, int64_t capacity, int64_t item_size,
    void *destination, size_t bytes) {
	if (capacity < length) { capacity = length; }
	if (destination != 0 && bytes >= INLINE_ITEMS && capacity >= 0 && item_size > 0 &&
	    (uint64_t)capacity <= (bytes - INLINE_ITEMS) / (uint64_t)item_size) {
		FaArrayHeader *array = destination;
		array->items = (uint8_t *)destination + INLINE_ITEMS;
		array->length = length;
		array->capacity = capacity;
		mark_items(array, ARRAY_FRAME);
		return array;
	}
	return fa_array_new(length, capacity, item_size);
}

/// Makes room for one more item and returns its slot (`push`).
static __attribute__((noinline)) void materialize_characters(FaArrayHeader *array, int64_t capacity) {
	if (capacity == 0) { array->capacity = 0; return; }
	if ((uint64_t)capacity > SIZE_MAX / sizeof(FaString) ||
	    (uint64_t)capacity > (uint64_t)INT64_MAX / sizeof(FaString)) { out_of_memory(); }
	FaString *items = fa_alloc((size_t)capacity * sizeof(FaString), 16);
	for (int64_t index = 0; index < array->length; index += 1) {
		items[index] = ascii_string(array->items[index]);
		items[index].capacity = SIMPLE;
	}
	free_array_items(array, 1);
	array->items = (uint8_t *)items;
	array->capacity = capacity;
}

FA_EXPORT __attribute__((always_inline)) void fa_array_materialize(FaArrayHeader *array) {
	if (__builtin_expect(array->capacity < 0, 0)) { materialize_characters(array, array->length); }
}

/// A borrowed string/character from an ordinary or compact array.
FA_EXPORT __attribute__((always_inline)) int32_t fa_string_array_get(const FaArrayHeader *array, int64_t index, FaString *out) {
	if (index < 0) { index += array->length; }
	if ((uint64_t)index >= (uint64_t)array->length) { return 0; }
	if (array->capacity < 0) {
		// A value consumer can use the byte itself, without a dependent
		// load through the static character table. Nothing is borrowed.
		FaString character = {0};
		((uint8_t *)&character)[0] = array->items[index];
		inline_string_tag(&character, 1, SIMPLE);
		*out = character;
	} else { *out = ((const FaString *)array->items)[index]; }
	return 1;
}

/// Makes room for one more item and returns its slot (`push`).
/// `fa_array_push_slot` on a full array: a block twice as large.
static __attribute__((noinline)) void array_grow(FaArrayHeader *array, int64_t item_size) {
	if (fa_array_is_region(array)) {
		if (array->capacity > INT64_MAX / 2) { out_of_memory(); }
		region_array_reserve(array, array->capacity < 4 ? 4 : array->capacity * 2, item_size);
		return;
	}
	if (array->capacity < 0) {
		if (array->length > INT64_MAX / 2) { out_of_memory(); }
		materialize_characters(array, array->length < 4 ? 4 : array->length * 2);
		return;
	}
	int64_t capacity = array->capacity < 4 ? 4 : array->capacity * 2;
	uint8_t *items;
	if (inline_items(array)) {
		items = fa_alloc((size_t)(capacity * item_size), 16);
		copy_bytes(items, array->items, (size_t)(array->length * item_size));
		mark_items(array, 0);
	} else {
		items = grow_block(array->items, (size_t)(array->capacity * item_size),
		                   (size_t)(array->length * item_size), (size_t)(capacity * item_size), 16);
	}
	array->items = items;
	array->capacity = capacity;
}

FA_EXPORT void *fa_array_push_slot(void *header, int64_t item_size) {
	FaArrayHeader *array = header;
	if (__builtin_expect(array->length >= array->capacity, 0)) {
		array_grow(array, item_size);
	}
	void *slot = array->items + array->length * item_size;
	array->length += 1;
	return slot;
}

void fa_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size);

/// `array_grow`, out of line, for the pushes the compiler writes inline
/// (`src/lower/arrays.rs`, `array_push`): their common path keeps the
/// header in registers, and only a full array calls this.
FA_EXPORT __attribute__((noinline)) void fa_array_grow(void *header, int64_t item_size) {
	array_grow(header, item_size);
}

/// Makes room for `count` more items, grown geometrically like the pushes.
static __attribute__((noinline)) void array_extend_room(FaArrayHeader *array, int64_t count,
                                                        int64_t item_size) {
	int64_t length = array->length;
	if (length > INT64_MAX - count || array->capacity > INT64_MAX / 2) {
		out_of_memory();
	}
	int64_t capacity = array->capacity < 2 ? 4 : array->capacity * 2;
	if (capacity < length + count) {
		capacity = length + count;
	}
	fa_array_reserve(array, capacity, item_size);
}

/// Makes room for `count` more items (`count >= 0`), grown geometrically
/// like the pushes: before a loop that pushes `count` times
/// (`src/lower/arrays.rs`, `reserve_for_pushes`). Inline: the room is
/// usually there.
FA_EXPORT __attribute__((always_inline)) void fa_array_room(FaArrayHeader *array, int64_t count,
                                                          int64_t item_size) {
	if (__builtin_expect(count > array->capacity - array->length, 0)) {
		array_extend_room(array, count, item_size);
	}
}

/// Appends `count` copies of a byte to an array of bytes: a loop pushing
/// the same byte `count` times (`src/lower/copies.rs`). Inline: short runs
/// are a few bounded stores.
FA_EXPORT __attribute__((always_inline)) void fa_array_fill(FaArrayHeader *array, int64_t count,
                                                          int32_t byte) {
	if (count <= 0) {
		return;
	}
	fa_array_room(array, count, 1);
	fill_bytes(array->items + array->length, (uint8_t)byte, (size_t)count);
	array->length += count;
}

/// Appends `count` items of `source`, from item `start` (bytewise): what a
/// loop pushing them one by one does (`src/lower/copies.rs`), with room
/// made once. The caller checked `0 <= start` and `start + count <=
/// source->length`. The items are read once the room is made, from the
/// block the source has then: the source may be the array itself. Inline:
/// most copies are short and find room.
FA_EXPORT __attribute__((always_inline)) void fa_array_extend(FaArrayHeader *array,
                                                            const FaArrayHeader *source,
                                                            int64_t start, int64_t count,
                                                            int64_t item_size) {
	if (count <= 0) {
		return;
	}
	fa_array_room(array, count, item_size);
	int64_t length = array->length;
	copy_bytes(array->items + length * item_size, source->items + start * item_size,
	           (size_t)(count * item_size));
	array->length = length + count;
}

/// `fa_array_grow` for a fresh child array of a root region
/// (`region_child_array` of `src/lower/trees.rs`), inline: a fresh region
/// literal has four physical slots, while its public capacity stays zero
/// until the first growth. Expose those slots here without entering the
/// general allocator's cold growth path.
FA_EXPORT __attribute__((always_inline)) void fa_array_region_grow(void *header,
                                                                  int64_t item_size) {
	FaArrayHeader *array = header;
	if (array->length == 0 && array->capacity == 0 &&
	    (array_marker(array) & ARRAY_REGION) &&
	    array->items == (uint8_t *)array + INLINE_ITEMS) {
		array->capacity = 4;
	} else {
		array_grow(array, item_size);
	}
}

/// The largest items moved into the block of their header by
/// `fa_array_compact`.
#define COMPACT_LIMIT ((size_t)512)

/// An array that the generated code owns and stores into a value it builds
/// (`compact_moved_array` of `src/lower/arrays.rs`): when its items are
/// small and in a block of their own, they move into a new block holding
/// the header and them, of their exact size. The array is unique, so that
/// the address of its header may change.
FA_EXPORT void *fa_array_compact(void *header, int64_t item_size) {
	FaArrayHeader *array = header;
	if (array_marker(array) & ARRAY_REGION_LOCAL) { return fa_array_into_region(header, item_size); }
	if (array->capacity < 0 || inline_items(array) || fa_array_is_region(array) ||
	    fa_array_storage(array) != 0) {
		return array;
	}
	size_t bytes = (size_t)(array->length * item_size);
	if (bytes > COMPACT_LIMIT) {
		return array;
	}
	FaArrayHeader *compact = fa_alloc(INLINE_ITEMS + bytes, 16);
	mark_items(compact, 1);
	compact->items = (uint8_t *)compact + INLINE_ITEMS;
	compact->length = array->length;
	compact->capacity = array->length;
	copy_bytes(compact->items, array->items, bytes);
	fa_free(array->items, (size_t)(array->capacity * item_size), 16);
	fa_free(array, INLINE_ITEMS, sizeof(int64_t));
	return compact;
}

/// A packed array (`src/lower/packed.rs`): the items of an `Array` of a
/// union whose arrays are only built by `push` and read by `for`, each
/// stored as the index of its member (a byte) followed by its data, with no
/// padding, one after the other. `length` counts the items, `capacity` is
/// the size of the block of the items in bytes and `used` the bytes in use.
typedef struct {
	uint8_t *items;
	int64_t length;
	int64_t capacity;
	int64_t used;
} FaPacked;

FA_EXPORT void *fa_packed_new(void) {
	FaPacked *array = fa_alloc(sizeof(FaPacked), sizeof(int64_t));
	array->items = 0;
	array->length = 0;
	array->capacity = 0;
	array->used = 0;
	return array;
}

/// `fa_packed_push` on a full block: one at least twice as large.
static __attribute__((noinline)) void packed_grow(FaPacked *array, int64_t bytes) {
	int64_t capacity = array->capacity < 64 ? 64 : array->capacity;
	while (capacity - array->used < bytes) {
		if (capacity > INT64_MAX / 2) { out_of_memory(); }
		capacity *= 2;
	}
	uint8_t *items = array->items == 0
	                     ? fa_alloc((size_t)capacity, 16)
	                     : grow_block(array->items, (size_t)array->capacity, (size_t)array->used,
	                                  (size_t)capacity, 16);
	array->items = items;
	array->capacity = capacity;
}

/// Room for an item of `bytes` bytes at the end (`push`): its address.
FA_EXPORT __attribute__((always_inline)) uint8_t *fa_packed_push(void *header, int64_t bytes) {
	FaPacked *array = header;
	if (__builtin_expect(array->capacity - array->used < bytes, 0)) {
		packed_grow(array, bytes);
	}
	uint8_t *slot = array->items + array->used;
	array->used += bytes;
	array->length += 1;
	return slot;
}

FA_EXPORT void fa_packed_free(void *header) {
	FaPacked *array = header;
	if (array->items != 0) {
		fa_free(array->items, (size_t)array->capacity, 16);
	}
	fa_free(array, sizeof(FaPacked), sizeof(int64_t));
}

/// Appends the items of `source` to `array` (bytewise).
FA_EXPORT void fa_array_append(FaArrayHeader *array, const FaArrayHeader *source,
                               int64_t item_size) {
	fa_array_materialize(array);
	fa_array_materialize((FaArrayHeader *)source);
	int64_t count = source->length;
	for (int64_t index = 0; index < count; index += 1) {
		void *slot = fa_array_push_slot(array, item_size);
		copy_bytes(slot, source->items + index * item_size, (size_t)item_size);
	}
}

const void *fa_error_index(int64_t index, int64_t length);

/// The position of a negative index counted from the end (`array[-1]`),
/// or -1 when it is out of bounds too. Out of line, so that LLVM's range
/// check elimination only sees the check `0 <= index < length` of the
/// common path.
FA_EXPORT __attribute__((noinline)) int64_t fa_index_from_end(int64_t index, int64_t length) {
	return index < 0 && index + length >= 0 ? index + length : -1;
}

/// `array[range]`: a new array (`faSliceBounds`): each bound in
/// `-length..=length` (an inclusive end in `-length..length`), else an
/// `IndexError`; a start after the end gives an empty slice. `flags`: bit 0
/// a start, bit 1 an end, bit 2 an inclusive end.
FA_EXPORT const void *fa_array_slice(const FaArrayHeader *array, int64_t start, int64_t end,
                                     int8_t flags, int64_t item_size, void **out) {
	fa_array_materialize((FaArrayHeader *)array);
	int64_t from, to;
	const void *error = slice_bounds(array->length, start, end, flags, &from, &to);
	if (error) { return error; }
	FaArrayHeader *slice = fa_array_new(to - from, to - from, item_size);
	copy_bytes(slice->items, array->items + from * item_size, (size_t)((to - from) * item_size));
	*out = slice;
	return 0;
}

typedef struct { FaArrayHeader header; uint64_t marker; } FaArrayView;
typedef struct { FaArrayHeader *value; FaArrayView view; } FaArrayReference;

/// A borrowed descriptor owns neither its items nor any allocation.
FA_EXPORT const void *fa_array_slice_view(FaArrayHeader *array, int64_t start, int64_t end,
                                         int8_t flags, int64_t item_size, FaArrayView *out) {
	fa_array_materialize(array);
	int64_t from, to;
	const void *error = slice_bounds(array->length, start, end, flags, &from, &to);
	if (error) { return error; }
	out->header.items = array->items + from * item_size;
	out->header.length = out->header.capacity = to - from;
	out->marker = ARRAY_FRAME | ARRAY_BORROWED;
	return 0;
}

FA_EXPORT FaArrayHeader **fa_array_reference(FaArrayHeader **place, FaArrayReference *out) {
	FaArrayHeader *array = *place;
	if (!array || !(array_marker(array) & ARRAY_BORROWED)) { return place; }
	out->view = *(const FaArrayView *)array;
	out->value = &out->view.header;
	return &out->value;
}

static int64_t storage_total(int64_t length, int64_t additional, int64_t item_size) {
    if (additional < 0 || length > INT64_MAX - additional || item_size < 0 ||
        (item_size > 0 && (uint64_t)(length + additional) > SIZE_MAX / (uint64_t)item_size)) { out_of_memory(); }
    return length + additional;
}

/// Makes room for `capacity` items (`Array.from(…, capacity = n)`).
FA_EXPORT void fa_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size) {
	if (fa_array_is_region(array)) {
		region_array_reserve(array, capacity, item_size);
		return;
	}
	if (array->capacity < 0) {
		if (capacity > array->length) { materialize_characters(array, capacity); }
		return;
	}
	if (capacity <= array->capacity) {
		return;
	}
	if (item_size > 0 && capacity > (int64_t)(SIZE_MAX / 2) / item_size) {
		out_of_memory();
	}
	uint8_t *items = fa_alloc((size_t)(capacity * item_size), 16);
	copy_bytes(items, array->items, (size_t)(array->length * item_size));
	free_array_items(array, item_size);
	array->items = items;
	array->capacity = capacity;
}

/// `target = source` on arrays: the items of the source copied into the
/// target's block, grown when needed (an array is a value).
FA_EXPORT void fa_array_assign(FaArrayHeader *target, const FaArrayHeader *source,
                               int64_t item_size) {
	// Only String/Character arrays have a compact negative-capacity form.
	// A different static element width lets LLVM remove both cold paths.
	if (item_size == sizeof(FaString)) {
		fa_array_materialize(target);
		fa_array_materialize((FaArrayHeader *)source);
	}
	if (target == source) {
		return;
	}
	if (target->capacity < source->length) {
		uint8_t *items = fa_alloc((size_t)(source->length * item_size), 16);
		free_array_items(target, item_size);
		target->items = items;
		target->capacity = source->length;
	}
	copy_bytes(target->items, source->items, (size_t)(source->length * item_size));
	target->length = source->length;
}

/// A copy of an array (its items copied bytewise).
FA_EXPORT FaArrayHeader *fa_array_copy(const FaArrayHeader *array, int64_t item_size) {
	if (item_size == sizeof(FaString) && array->capacity < 0) {
		FaArrayHeader *copy = fa_array_new(array->length, array->length, 1);
		copy_bytes(copy->items, array->items, (size_t)array->length);
		copy->capacity = -1;
		return copy;
	}
	FaArrayHeader *copy = fa_array_new(array->length, array->length, item_size);
	copy_bytes(copy->items, array->items, (size_t)(array->length * item_size));
	return copy;
}

/// Reverses the items of an array in place.
FA_EXPORT void fa_array_reverse(FaArrayHeader *array, int64_t item_size) {
	fa_array_materialize(array);
	if (array->length < 2) {
		return;
	}
	uint8_t swap[64];
	uint8_t *low = array->items;
	uint8_t *high = array->items + (array->length - 1) * item_size;
	while (low < high) {
		for (int64_t offset = 0; offset < item_size; offset += (int64_t)sizeof swap) {
			size_t chunk = (size_t)(item_size - offset) < sizeof swap ? (size_t)(item_size - offset)
			                                                          : sizeof swap;
			copy_bytes(swap, low + offset, chunk);
			copy_bytes(low + offset, high + offset, chunk);
			copy_bytes(high + offset, swap, chunk);
		}
		low += item_size;
		high -= item_size;
	}
}

/// Makes room for an item at `index` (`0..=length`) and returns its slot.
FA_EXPORT void *fa_array_insert_slot(FaArrayHeader *array, int64_t index, int64_t item_size) {
	fa_array_push_slot(array, item_size);
	uint8_t *at = array->items + index * item_size;
	size_t moved = (size_t)((array->length - 1 - index) * item_size);
	__builtin_memmove(at + item_size, at, moved);
	return at;
}

/// Removes the item at `index` (`0..length`).
FA_EXPORT void fa_array_remove(FaArrayHeader *array, int64_t index, int64_t item_size) {
	fa_array_materialize(array);
	uint8_t *at = array->items + index * item_size;
	size_t moved = (size_t)((array->length - 1 - index) * item_size);
	__builtin_memmove(at, at + item_size, moved);
	array->length -= 1;
}

/// Sorts an array in place, stably (a merge sort, as JavaScript's sort):
/// `compare(context, a, b)` is negative when `a` comes first.
FA_EXPORT __attribute__((always_inline)) void fa_array_sort(FaArrayHeader *array, int64_t item_size,
                             int32_t (*compare)(void *, const void *, const void *),
                             void *context) {
	fa_array_materialize(array);
	int64_t length = array->length;
	if (length < 2) {
		return;
	}
	size_t bytes = (size_t)(length * item_size);
	uint8_t *from = array->items;
	uint8_t *to = fa_alloc(bytes, 16);
	for (int64_t width = 1; width < length; width *= 2) {
		for (int64_t start = 0; start < length; start += 2 * width) {
			int64_t middle = start + width < length ? start + width : length;
			int64_t end = start + 2 * width < length ? start + 2 * width : length;
			int64_t left = start, right = middle, out = start;
			while (left < middle && right < end) {
				if (compare(context, from + right * item_size, from + left * item_size) < 0) {
					copy_bytes(to + out * item_size, from + right * item_size, (size_t)item_size);
					right += 1;
				} else {
					copy_bytes(to + out * item_size, from + left * item_size, (size_t)item_size);
					left += 1;
				}
				out += 1;
			}
			copy_bytes(to + out * item_size, from + left * item_size,
			           (size_t)((middle - left) * item_size));
			out += middle - left;
			copy_bytes(to + out * item_size, from + right * item_size,
			           (size_t)((end - right) * item_size));
		}
		uint8_t *swap = from;
		from = to;
		to = swap;
	}
	if (from != array->items) {
		copy_bytes(array->items, from, bytes);
		fa_free(from, bytes, 16);
	} else {
		fa_free(to, bytes, 16);
	}
}

/// `arguments`: a new `Array(String)` (its strings live as long as the program).
FA_EXPORT FaArrayHeader *fa_arguments(void) {
	size_t count = argument_count;
#if !defined(__wasm__)
	// The only writer takes a signed 32-bit argc and stores max(argc - 1, 0).
	// On wasm the count already fits in 32 bits. Preserve this invariant
	// across allocation calls so that the ordinary storage is provable.
	if (count > INT32_MAX) { __builtin_unreachable(); }
#endif
	FaArrayHeader *array = fa_array_new((int64_t)count, (int64_t)count,
	                                    (int64_t)sizeof(FaString));
	copy_bytes(array->items, (const uint8_t *)argument_items, sizeof(FaString) * count);
	if (arguments_have_owners) {
		FaString *items = (FaString *)(void *)array->items;
		for (size_t index = 0; index < count; index += 1) {
			if (string_owned(items[index].capacity)) {
				items[index].capacity = STRING_LITERAL | fa_string_length(items[index].bytes, items[index].length, items[index].capacity);
			}
		}
	}
	// Finish the header after the copy: the optimizer need not prove that
	// the allocator returned disjoint header and item storage.
	array->length = (int64_t)count;
	array->capacity = (int64_t)count;
	return array;
}

/// Frees an array whose items were dropped: its block and its header.
FA_EXPORT void fa_array_free(FaArrayHeader *array, int64_t item_size) {
	if (fa_array_is_region(array)) {
		free_array_items(array, item_size);
		return;
	}
	if (fa_array_storage(array) != 0) { return; }
	if (!inline_items(array)) {
		fa_free(array->items, (size_t)(array->capacity < 0 ? array->length : array->capacity * item_size), 16);
	}
	fa_free(array, INLINE_ITEMS, sizeof(int64_t));
}

// ── fixed-length byte blocks ────────────────────────────────────────────

typedef struct { uint8_t *items; size_t length; } FaBytesView;
typedef struct { void *value; FaBytesView view; } FaBytesReference;
_Static_assert(sizeof(FaBytesView) == 2 * sizeof(size_t), "two-word Bytes view");

// A separate byte payload can retain/adopt a String allocation prefix. Its
// metadata is uninitialized while it is Bytes, and is never inspected.
#define BYTES_STRING_BLOCK ((uint64_t)128)

static FaArrayHeader *bytes_allocate(size_t length) {
	if (length > SIZE_MAX - INLINE_ITEMS) { out_of_memory(); }
#if SIZE_MAX > INT64_MAX
	if (length > INT64_MAX) { out_of_memory(); }
#endif
	if (length <= INLINE_LIMIT) {
		// Padding makes a combined block adoptable as a String whose low
		// capacity bit is reserved. It never extends the logical byte range.
		return fa_array_new((int64_t)length, (int64_t)((length + 1) & ~(size_t)1), 1);
	}
	if (length >= KIND_MASK - sizeof(StringHeader) - 8192) {
		return fa_array_new((int64_t)length, (int64_t)length, 1);
	}
	FaArrayHeader *owner = fa_alloc(INLINE_ITEMS, sizeof(int64_t));
	size_t capacity = string_block(length);
	*owner = (FaArrayHeader){string_alloc_block(capacity, SIMPLE), (int64_t)length, (int64_t)capacity};
	mark_items(owner, BYTES_STRING_BLOCK);
	return owner;
}

static FaArrayHeader *bytes_adopt_string(uint8_t *bytes, size_t length, size_t capacity) {
	FaArrayHeader *owner = fa_alloc(INLINE_ITEMS, sizeof(int64_t));
	*owner = (FaArrayHeader){bytes, (int64_t)length, (int64_t)capacity};
	mark_items(owner, BYTES_STRING_BLOCK);
	return owner;
}

/// Owners keep the array block layout. A tagged pointer addresses only the
/// two-word borrowed descriptor: it has no capacity, marker or allocation.
static FaBytesView bytes_parts(const void *bytes) {
	uintptr_t address = (uintptr_t)bytes;
	if (address & 1) { return *(const FaBytesView *)(address & ~(uintptr_t)1); }
	const FaArrayHeader *owner = bytes;
	return (FaBytesView){owner->items, (size_t)owner->length};
}

FA_EXPORT __attribute__((always_inline)) void fa_bytes_array_view(const void *bytes, FaArrayHeader *out) {
	FaBytesView view = bytes_parts(bytes);
	*out = (FaArrayHeader){view.items, (int64_t)view.length, (int64_t)view.length};
	mark_items(out, ARRAY_FRAME);
}

FA_EXPORT void *fa_bytes_filled(int64_t length, uint8_t value) {
	if (length < 0) { out_of_memory(); }
#if SIZE_MAX < INT64_MAX
	if ((uint64_t)length > SIZE_MAX) { out_of_memory(); }
#endif
	FaArrayHeader *out = bytes_allocate((size_t)length);
	fill_bytes(out->items, value, (size_t)length);
	return out;
}

FA_EXPORT void *fa_bytes_from(const uint8_t *items, size_t length) {
	FaArrayHeader *out = bytes_allocate(length);
	copy_bytes(out->items, items, length);
	return out;
}

FA_EXPORT void *fa_bytes_copy(const void *bytes) {
	if (!bytes) { return 0; }
	FaBytesView view = bytes_parts(bytes);
	return fa_bytes_from(view.items, view.length);
}

FA_EXPORT void fa_bytes_free(void *bytes) {
	if (!bytes || ((uintptr_t)bytes & 1)) { return; }
	FaArrayHeader *owner = bytes;
	if (fa_array_storage(owner) != 0) { return; }
	if (array_marker(owner) == BYTES_STRING_BLOCK) {
		fa_free(string_header(owner->items), sizeof(StringHeader) + (size_t)owner->capacity, sizeof(size_t));
		fa_free(owner, INLINE_ITEMS, sizeof(int64_t));
	} else if (inline_items(owner)) {
		fa_free(owner, INLINE_ITEMS + (size_t)owner->capacity, 16);
	} else { fa_array_free(owner, 1); }
}

FA_EXPORT void *fa_bytes_own(void *bytes) {
	if (bytes && (((uintptr_t)bytes & 1) || fa_array_storage(bytes) != 0)) {
		return fa_bytes_copy(bytes);
	}
	return bytes;
}

/// Array results use its ordinary allocation/deallocation convention.
FA_EXPORT void *fa_bytes_to_array(void *bytes, int32_t move) {
	if (move && !((uintptr_t)bytes & 1) && fa_array_storage(bytes) == 0 &&
	    !fa_array_is_region(bytes) && array_marker(bytes) != BYTES_STRING_BLOCK) { return bytes; }
	FaBytesView view = bytes_parts(bytes);
	FaArrayHeader *array = fa_array_new((int64_t)view.length, (int64_t)view.length, 1);
	copy_bytes(array->items, view.items, view.length);
	if (move) { fa_bytes_free(bytes); }
	return array;
}

FA_EXPORT void *fa_bytes_from_array(void *array, int32_t move) {
	if (move && fa_array_storage(array) == 0 && !fa_array_is_region(array)) { return array; }
	FaArrayHeader *source = array;
	void *bytes = fa_bytes_from(source->items, (size_t)source->length);
	if (move) { fa_array_free(source, 1); }
	return bytes;
}

FA_EXPORT __attribute__((noinline)) void *fa_string_to_utf8(FaString *source, int32_t move) {
	const uint8_t *bytes = string_bytes(source);
	size_t length = string_length(source), capacity = source->capacity;
	if (move && string_owned(capacity)) {
		if (!(capacity & SIMPLE)) { string_clear(string_header(bytes)); }
		// Canonical owners never have virtual edges; replacements are the
		// same three bytes wide, so the block can be rewritten in place.
		fa_string_write_utf8((uint8_t *)bytes, bytes, length, capacity);
		*source = (FaString){0};
		return bytes_adopt_string((uint8_t *)bytes, length, capacity & KIND_MASK);
	}
	size_t encoded = fa_string_encoded_length(bytes, length, capacity);
	FaArrayHeader *out = bytes_allocate(encoded);
	fa_string_write_utf8(out->items, bytes, length, capacity);
	if (move) { *source = (FaString){0}; }
	return out;
}

const void *fa_error_utf8(int64_t position);

/// A consuming call frees the input on both the error and success paths.
FA_EXPORT __attribute__((noinline)) const void *fa_string_decode_bytes(void *bytes,
    int32_t move, int32_t lossy, FaString *out) {
	FaBytesView view = bytes_parts(bytes);
	Utf8Info info = utf8_validate(view.items, view.length);
	if (info.error != SIZE_MAX) {
		if (lossy) { utf8_copy_lossy(view.items, view.length, info, out); }
		if (move) { fa_bytes_free(bytes); }
		return lossy ? 0 : fa_error_utf8((int64_t)info.error);
	}
	if (move && !((uintptr_t)bytes & 1) && view.length > INLINE_STRING_BYTES &&
	    fa_array_storage(bytes) == 0 && !fa_array_is_region(bytes)) {
		FaArrayHeader *owner = bytes;
		size_t capacity = (size_t)owner->capacity;
		uint8_t *items = view.items;
		int adopt = array_marker(owner) == BYTES_STRING_BLOCK && capacity < KIND_MASK;
		if (!adopt && inline_items(owner) && !(capacity & 1)) {
			// The combined array header becomes the prefix. On wasm the
			// smaller prefix shifts at most 96 bytes within this same block.
			items = (uint8_t *)owner + sizeof(StringHeader);
			__builtin_memmove(items, view.items, view.length);
			capacity += INLINE_ITEMS - sizeof(StringHeader);
			adopt = 2;
		}
		if (adopt) {
			if (adopt == 1) { fa_free(owner, INLINE_ITEMS, sizeof(int64_t)); }
			size_t flags = info.units == view.length ? SIMPLE : 0;
			if (!flags) { string_init_header(string_header(items)); }
			*out = (FaString){items, view.length, capacity | flags};
			fa_string_set_unit_count(items, view.length, out->capacity, info.units);
			return 0;
		}
	}
	utf8_copy_valid(view.items, view.length, info.units, out);
	if (move) { fa_bytes_free(bytes); }
	return 0;
}

FA_EXPORT __attribute__((always_inline)) int32_t fa_bytes_equal(const void *left, const void *right) {
	FaBytesView a = bytes_parts(left), b = bytes_parts(right);
	return string_equal_canonical(a.items, a.length, b.items, b.length);
}

uint64_t fa_hash_bytes(const uint8_t *bytes, size_t length);
FA_EXPORT uint64_t fa_bytes_hash(const void *bytes) {
	FaBytesView view = bytes_parts(bytes);
	return fa_hash_bytes(view.items, view.length);
}

FA_EXPORT const void *fa_bytes_slice(const void *bytes, int64_t start, int64_t end,
    int8_t flags, void **out) {
	FaBytesView view = bytes_parts(bytes);
	int64_t from, to;
	const void *error = slice_bounds((int64_t)view.length, start, end, flags, &from, &to);
	if (error) { return error; }
	*out = fa_bytes_from(view.items + from, (size_t)(to - from));
	return 0;
}

FA_EXPORT const void *fa_bytes_slice_view(const void *bytes, int64_t start, int64_t end,
    int8_t flags, FaBytesView *out) {
	FaBytesView view = bytes_parts(bytes);
	int64_t from, to;
	const void *error = slice_bounds((int64_t)view.length, start, end, flags, &from, &to);
	if (error) { return error; }
	*out = (FaBytesView){view.items + from, (size_t)(to - from)};
	return 0;
}

FA_EXPORT __attribute__((always_inline)) void *fa_bytes_view_pointer(FaBytesView *view) {
	return (void *)((uintptr_t)view | 1);
}

FA_EXPORT __attribute__((always_inline)) void **fa_bytes_reference(void **place, FaBytesReference *snapshot) {
	if (!((uintptr_t)*place & 1)) { return place; }
	snapshot->view = bytes_parts(*place);
	snapshot->value = fa_bytes_view_pointer(&snapshot->view);
	return &snapshot->value;
}

/// The same non-overlapping ASCII search as `each_ascii_match`, but
/// stop as soon as the caller's item budget is exceeded. Large inputs do
/// not need a complete extra scan merely to choose ordinary heap storage.
static size_t count_split_parts(const uint8_t *text, size_t length,
                                const uint8_t *separator, size_t separator_length,
                                size_t limit) {
	size_t count = 1;
	size_t offset = 0;
	while (offset + separator_length <= length) {
		offset = next_byte(text, length, offset, separator[0]);
		if (offset + separator_length > length) { break; }
		if (bytes_at(text, length, offset, separator, separator_length)) {
			count += 1;
			if (count > limit) { return count; }
			offset += separator_length;
		} else { offset += 1; }
	}
	return count;
}

/// Construct directly in the caller's fixed destination when it fits.
/// Count first: a large result takes the heap from the start, so no items
/// are relocated or copied out of a partially filled frame buffer.
static void *split_text_into(const uint8_t *text, size_t length, size_t capacity,
                             const uint8_t *separator, size_t separator_length, int views,
                             void *destination, size_t bytes) {
	if (separator_length == 0 || destination == 0 || bytes < INLINE_ITEMS) {
		return split_text(text, length, capacity, separator, separator_length, views, 0);
	}
	size_t limit = (bytes - INLINE_ITEMS) / sizeof(FaString);
	// At most one part per non-overlapping separator, plus the last part.
	size_t count = !(capacity & SIMPLE) ? count_unicode_parts(text, length, separator, separator_length, limit)
	             : length / separator_length < limit ? length / separator_length + 1
	             : count_split_parts(text, length, separator, separator_length, limit);
	if (count > limit) {
		return split_text(text, length, capacity, separator, separator_length, views, 0);
	}
	FaArrayHeader *parts = destination;
	parts->items = (uint8_t *)destination + INLINE_ITEMS;
	parts->length = 0;
	parts->capacity = (int64_t)count;
	mark_items(parts, ARRAY_FRAME);
	if (!(capacity & SIMPLE)) {
		return split_unicode(text, length, capacity, separator, separator_length, views, 0, parts);
	}
	Splitting splitting = {text, 0, separator_length, parts, capacity & SIMPLE, views, 0, 1, 0};
	each_ascii_match(text, length, separator, separator_length, split_one, &splitting);
	FaString *last = fa_array_push_slot(parts, sizeof(FaString));
	*last = split_part(&splitting, splitting.previous, length);
	return parts;
}

/// `fa_string_split` whose array is built in the caller's storage when its
/// parts fit there (`src/lower/collection_storage.rs`).
FA_EXPORT void *fa_string_split_into(const uint8_t *text, size_t length, size_t capacity,
                                     const uint8_t *separator, size_t separator_length,
                                     void *destination, size_t bytes) {
	return split_text_into(text, length, capacity, separator, separator_length, 0, destination,
	                       bytes);
}

/// `fa_string_split_views` whose array is built in the caller's storage.
FA_EXPORT void *fa_string_split_views_into(const uint8_t *text, size_t length, size_t capacity,
                                           const uint8_t *separator, size_t separator_length,
                                           void *destination, size_t bytes) {
	return split_text_into(text, length, capacity, separator, separator_length, 1, destination,
	                       bytes);
}

// ── segmented storage (the growing collections) ────────────────────────

// A `GrowingArray` (and the entries of a `GrowingMap` / `GrowingSet`) never
// moves its items: they live in segments that are never reallocated
// (growing-collections.md, implementation notes; Zig's `SegmentedList`).
// With a first segment of `2^k` items, segment `s > 0` holds `2^(k+s-1)`
// items starting at index `2^(k+s-1)`, so item `i` is in segment
// `s = floor(log2(i >> k)) + 1` (0 when `i < 2^k`): one `clz`.

#define FA_SEGMENTS ((int32_t)(sizeof(void *) * 8))
#define FA_DIRECTORY_INITIAL (FA_INLINE_SEGMENTS + 8)
#define FA_INLINE_SEGMENTS 3

/// A `GrowingArray` (`src/lower/growing.rs`). `cursor` is the address of
/// item `length` and `end` the end of its segment when that segment is
/// allocated; else both are equal (null in a new array), so that `push` is
/// one comparison, one store and one increment until a segment is full.
typedef struct {
	int64_t length;
	int16_t segments;
	uint16_t storage;
	int32_t k;
	uint8_t *cursor;
	uint8_t *end;
	uintptr_t *segment;
	uintptr_t inline_segment[FA_INLINE_SEGMENTS];
} FaSegmented;

_Static_assert(sizeof(FaSegmented) == (sizeof(void *) == 8 ? 64 : 40), "compact growing header");

/// The directory always contains every pointer: the header's inline slots
/// until segment four, then a heap table with copies of those three slots.
static uint8_t *segmented_items(const FaSegmented *array, int64_t segment, int64_t item_size) {
	uint64_t first = segment == 0 ? 0 : (uint64_t)1 << (array->k + segment - 1);
	return (uint8_t *)(array->segment[segment] + (uintptr_t)first * (uintptr_t)item_size);
}

/// The directory stores integer addresses biased by each segment's start.
/// Unsigned arithmetic may wrap; only the restored in-bounds address is a pointer.
static uint8_t *segmented_address(const FaSegmented *array, int64_t segment,
                                   int64_t index, int64_t item_size) {
	return (uint8_t *)(array->segment[segment] + (uintptr_t)index * (uintptr_t)item_size);
}

static size_t segmented_directory_entries(const FaSegmented *array) {
	return array->segments <= FA_DIRECTORY_INITIAL
	           ? FA_DIRECTORY_INITIAL : FA_SEGMENTS;
}

static void segmented_free_directory(FaSegmented *array) {
	if (array->segment != array->inline_segment) {
		fa_free(array->segment, segmented_directory_entries(array) * sizeof(uint8_t *), sizeof(void *));
		array->segment = array->inline_segment;
	}
}

/// The segment of item `index`, and the index of its first item.
static __attribute__((always_inline)) inline int64_t segment_of(int64_t index, int32_t k,
                                                                int64_t *first) {
	uint64_t capacity = (uint64_t)1 << k;
	uint64_t high = (uint64_t)index >> k;
	int64_t segment = 63 - __builtin_clzll((high << 1) | 1);
	*first = (int64_t)(((capacity << segment) >> 1) & -capacity);
	return segment;
}

/// The number of items of a segment.
static int64_t segment_size(int64_t segment, int32_t k) {
	return (int64_t)1 << (segment == 0 ? k : k + segment - 1);
}

/// The address of item `index` (in bounds).
FA_EXPORT __attribute__((always_inline)) void *fa_segmented_at(const FaSegmented *array,
                                                               int64_t index,
                                                               int64_t item_size, int32_t k) {
	k = array->k < 0 ? k : array->k;
	int64_t first;
	int64_t segment = segment_of(index, k, &first);
	return segmented_address(array, segment, index, item_size);
}

/// A new empty `GrowingArray`.
FA_EXPORT FaSegmented *fa_segmented_new(void) {
	FaSegmented *array = fa_alloc(sizeof(FaSegmented), sizeof(int64_t));
	__builtin_memset(array, 0, sizeof(FaSegmented));
	array->k = -1;
	array->segment = array->inline_segment;
	return array;
}

FA_EXPORT FaSegmented *fa_segmented_local(void *buffer, int64_t length,
                                         int64_t item_size, int32_t k) {
	FaSegmented *array = buffer;
	__builtin_memset(array, 0, sizeof(FaSegmented));
	while (((uint64_t)1 << k) < (uint64_t)length) { k += 1; }
	array->storage = 3;
	array->length = length;
	array->k = k;
	array->segment = array->inline_segment;
	if (length) {
		uint8_t *items = (uint8_t *)(((uintptr_t)buffer + sizeof(FaSegmented) + 15) & ~(uintptr_t)15);
		array->segments = 1;
		array->inline_segment[0] = (uintptr_t)items;
		array->cursor = items + length * item_size;
		array->end = items + segment_size(0, k) * item_size;
	}
	return array;
}

/// Allocates a segment; only its pointer directory can ever move.
static void segmented_allocate(FaSegmented *array, int64_t item_size) {
    int32_t segment = array->segments;
    if (segment == 0 && array->storage == 3 && array->inline_segment[0] == 0) { array->storage = 4; }
    int32_t shift = segment == 0 ? array->k : array->k + segment - 1;
    if (segment >= FA_SEGMENTS || shift >= (segment == 0 ? 63 : 62) ||
        (item_size > 0 && (uint64_t)segment_size(segment, array->k) > SIZE_MAX / (uint64_t)item_size)) { out_of_memory(); }
    if (segment == FA_INLINE_SEGMENTS || segment == FA_DIRECTORY_INITIAL) {
        size_t entries = segment == FA_INLINE_SEGMENTS ? FA_DIRECTORY_INITIAL : FA_SEGMENTS;
        uintptr_t *directory = fa_alloc(entries * sizeof(uintptr_t), sizeof(uintptr_t));
        size_t previous = segment == FA_INLINE_SEGMENTS ? FA_INLINE_SEGMENTS : FA_DIRECTORY_INITIAL;
        copy_bytes((uint8_t *)directory, (const uint8_t *)array->segment, previous * sizeof(uint8_t *));
        if (segment > FA_INLINE_SEGMENTS) {
            fa_free(array->segment, FA_DIRECTORY_INITIAL * sizeof(uint8_t *), sizeof(void *));
        }
        array->segment = directory;
    }
    uint8_t *items = fa_alloc((size_t)segment_size(segment, array->k) * (size_t)item_size, 16);
    uint64_t first = segment == 0 ? 0 : (uint64_t)segment_size(segment, array->k);
    array->segment[segment] = (uintptr_t)items - (uintptr_t)first * (uintptr_t)item_size;
    array->segments += 1;
}

FA_EXPORT int64_t fa_segmented_capacity(const FaSegmented *array) {
    return array->segments == 0 ? 0 : segment_size(array->segments - 1, array->k) * (array->segments == 1 ? 1 : 2);
}

FA_EXPORT void fa_segmented_reserve(FaSegmented *array, int64_t additional, int64_t item_size, int32_t k) {
    int64_t needed = storage_total(array->length, additional, item_size);
    int64_t capacity = fa_segmented_capacity(array);
    if (array->length == 0 && array->segments > 0 && needed > segment_size(0, array->k)) {
        // Reset already invalidated references to the old items. Rebuild an
        // empty reservation as one block, without reducing total capacity.
        for (int32_t segment = array->storage == 3 ? 1 : 0; segment < array->segments; segment += 1) {
            fa_free(segmented_items(array, segment, item_size), (size_t)segment_size(segment, array->k) * (size_t)item_size, 16);
        }
        segmented_free_directory(array);
        array->segments = 0;
        if (array->storage == 3) { array->storage = 4; }
        if (needed < capacity) { needed = capacity; }
    } else if (needed <= capacity) { return; }
    if (array->segments == 0) {
        k = 0;
        while (k < 62 && ((uint64_t)1 << k) < (uint64_t)needed) { k += 1; }
        array->k = k;
    }
    while (fa_segmented_capacity(array) < needed) { segmented_allocate(array, item_size); }
    if (array->length == 0) {
        array->cursor = (uint8_t *)array->inline_segment[0];
        array->end = array->cursor + segment_size(0, array->k) * item_size;
    }
}

/// A literal uses one final segment, in the caller's buffer or on the heap.
FA_EXPORT void *fa_segmented_new_into(int64_t length, int64_t item_size, int32_t k,
                                     void *destination, size_t bytes) {
	if (length < 0 || item_size < 0 || k < 0 || k > 62) { out_of_memory(); }
	while (((uint64_t)1 << k) < (uint64_t)length) {
		if (k == 62) { out_of_memory(); }
		k += 1;
	}
	uint64_t capacity = (uint64_t)1 << k;
	size_t header = (sizeof(FaSegmented) + 15) & ~(size_t)15;
	if (destination && bytes >= header &&
	    (item_size == 0 || capacity <= (bytes - header) / (uint64_t)item_size)) {
		return fa_segmented_local(destination, length, item_size, k);
	}
	FaSegmented *array = fa_segmented_new();
	if (length) {
		fa_segmented_reserve(array, (int64_t)capacity, item_size, k);
		array->length = length;
		array->cursor += length * item_size;
	}
	return array;
}

/// Points the cursor at item `length`, in its segment, allocated when it
/// doesn't exist yet (`push` on a full segment; segments are kept by a
/// reset, so the next one may exist already).
FA_EXPORT __attribute__((noinline)) void fa_segmented_next(FaSegmented *array, int64_t item_size,
                                                          int32_t k) {
	k = array->k < 0 ? k : array->k;
	if (array->k < 0) { array->k = k; }
	int64_t index = array->length;
	int64_t first;
	int64_t segment = segment_of(index, k, &first);
	if (segment >= array->segments) {
		segmented_allocate(array, item_size);
	}
	array->cursor = segmented_address(array, segment, index, item_size);
	array->end = segmented_address(array, segment, first + segment_size(segment, k), item_size);
}

/// The slot of a new last item (`push`): a full segment is kept, and a new
/// one is added.
FA_EXPORT __attribute__((always_inline)) void *fa_segmented_push_slot(FaSegmented *array,
                                                                    int64_t item_size,
                                                                    int32_t k) {
	if (__builtin_expect(array->cursor == array->end, 0)) {
		fa_segmented_next(array, item_size, k);
	}
	uint8_t *slot = array->cursor;
	array->cursor = slot + item_size;
	array->length += 1;
	return slot;
}

/// `fa_segmented_append` across segments.
static __attribute__((noinline)) void segmented_append_runs(FaSegmented *array,
                                                           const uint8_t *items, int64_t count,
                                                           int64_t item_size, int32_t k) {
	k = array->k < 0 ? k : array->k;
	while (count > 0) {
		if (array->cursor == array->end) {
			fa_segmented_next(array, item_size, k);
		}
		int64_t room = (array->end - array->cursor) / item_size;
		int64_t run = count < room ? count : room;
		copy_bytes(array->cursor, items, (size_t)(run * item_size));
		array->cursor += run * item_size;
		array->length += run;
		items += run * item_size;
		count -= run;
	}
}

/// Appends `count` items stored contiguously at `items`: one copy per
/// segment they fill. `items` may be in this array: its items never move.
/// Inline: most copies fit in the current segment.
FA_EXPORT __attribute__((always_inline)) void fa_segmented_append(FaSegmented *array,
                                                                const uint8_t *items,
                                                                int64_t count, int64_t item_size,
                                                                int32_t k) {
    if (item_size == 0) {
        fa_segmented_reserve(array, count, item_size, k);
        array->length += count;
        return;
    }

	if (count > 0 && count * item_size <= array->end - array->cursor) {
		copy_bytes(array->cursor, items, (size_t)(count * item_size));
		array->cursor += count * item_size;
		array->length += count;
		return;
	}
	segmented_append_runs(array, items, count, item_size, k);
}

/// `fa_segmented_fill` across segments.
static __attribute__((noinline)) void segmented_fill_runs(FaSegmented *array, int64_t count,
                                                         int32_t byte, int32_t k) {
	k = array->k < 0 ? k : array->k;
	while (count > 0) {
		if (array->cursor == array->end) {
			fa_segmented_next(array, 1, k);
		}
		int64_t room = array->end - array->cursor;
		int64_t run = count < room ? count : room;
		fill_bytes(array->cursor, (uint8_t)byte, (size_t)run);
		array->cursor += run;
		array->length += run;
		count -= run;
	}
}

/// Appends `count` copies of a byte to a `GrowingArray` of bytes (a loop
/// pushing the same byte, `src/lower/copies.rs`). Inline: most
/// fills fit in the current segment.
FA_EXPORT __attribute__((always_inline)) void fa_segmented_fill(FaSegmented *array, int64_t count,
                                                              int32_t byte, int32_t k) {
	if (count > 0 && count <= array->end - array->cursor) {
		fill_bytes(array->cursor, (uint8_t)byte, (size_t)count);
		array->cursor += count;
		array->length += count;
		return;
	}
	segmented_fill_runs(array, count, byte, k);
}

/// Appends `count` items of a `GrowingArray` from item `start` (bytewise),
/// to an array (`target_k < 0`) or to a `GrowingArray`: one copy per run of
/// the source in a segment. The caller checked `0 <= start` and `start +
/// count <= source->length`; the source's items never move, even when the
/// target is the source itself.
static __attribute__((noinline)) void segmented_extend_runs(void *target, int32_t target_k,
                                                           const FaSegmented *source,
                                                           int64_t start, int64_t count,
                                                           int64_t item_size, int32_t k) {
	k = source->k < 0 ? k : source->k;
	if (target_k < 0) {
		fa_array_room((FaArrayHeader *)target, count, item_size);
	}
	while (count > 0) {
		int64_t first;
		int64_t segment = segment_of(start, k, &first);
		int64_t room = segment_size(segment, k) - (start - first);
		int64_t run = count < room ? count : room;
		const uint8_t *items = segmented_address(source, segment, start, item_size);
		if (target_k < 0) {
			FaArrayHeader *array = target;
			copy_bytes(array->items + array->length * item_size, items, (size_t)(run * item_size));
			array->length += run;
		} else {
			fa_segmented_append(target, items, run, item_size, target_k);
		}
		start += run;
		count -= run;
	}
}

FA_EXPORT __attribute__((always_inline)) void fa_segmented_extend_to(
	void *target, int32_t target_k, const FaSegmented *source, int64_t start, int64_t count,
	int64_t item_size, int32_t k) {
	if (count <= 0) { return; }
	// One allocated segment proves every checked source range contiguous.
	// The general path also handles retained or preallocated later segments.
	const uint8_t *items;
	if (source->segments <= 1) {
		items = (uint8_t *)source->inline_segment[0] + start * item_size;
	} else if ((uint64_t)start + (uint64_t)count <= ((uint64_t)1 << source->k)) {
		items = (uint8_t *)source->inline_segment[0] + start * item_size;
	} else {
		k = source->k;
		int64_t first;
		int64_t segment = segment_of(start, k, &first);
		if (start - first + count > segment_size(segment, k)) {
			segmented_extend_runs(target, target_k, source, start, count, item_size, k);
			return;
		}
		items = segmented_address(source, segment, start, item_size);
	}
	if (target_k < 0) {
		FaArrayHeader *array = target;
		fa_array_room(array, count, item_size);
		copy_bytes(array->items + array->length * item_size, items, (size_t)(count * item_size));
		array->length += count;
	} else {
		fa_segmented_append(target, items, count, item_size, target_k);
	}
}

/// The first item of the segment holding item `index`, and in `*end` the
/// end of that segment: a loop over the items walks a segment with a
/// pointer, and calls this only when it reaches the end of one.
FA_EXPORT __attribute__((noinline)) uint8_t *fa_segmented_run(const FaSegmented *array,
                                                              int64_t index, int64_t item_size,
                                                              int32_t k, uint8_t **end) {
	k = array->k < 0 ? k : array->k;
	int64_t first;
	int64_t segment = segment_of(index, k, &first);
	*end = segmented_address(array, segment, first + segment_size(segment, k), item_size);
	return segmented_address(array, segment, index, item_size);
}

/// The index of the first byte equal to `byte` at or after `offset` in a
/// `GrowingArray` of bytes, or `length` (a loop acting on those items
/// only). `*cursor` is the address of item `offset` and `*end` the end of
/// its segment, or both are equal when that segment must be found: the
/// loop walks the segments with them, and finds them at the match.
FA_EXPORT int64_t fa_segmented_next_byte(const FaSegmented *array, int64_t offset, int64_t length,
                                         int32_t byte, int32_t k, uint8_t **cursor,
                                         uint8_t **end) {
	while (offset < length) {
		if (*cursor == *end) {
			*cursor = fa_segmented_run(array, offset, 1, k, end);
		}
		int64_t room = *end - *cursor;
		int64_t count = length - offset < room ? length - offset : room;
		size_t found = next_byte(*cursor, (size_t)count, 0, (uint8_t)byte);
		if ((int64_t)found < count) {
			*cursor += found;
			return offset + (int64_t)found;
		}
		*cursor += count;
		offset += count;
	}
	return length;
}

/// `growing = []`: no item left (the caller dropped those owning memory),
/// the segments kept for the next pushes.
FA_EXPORT void fa_segmented_reset(FaSegmented *array, int64_t item_size, int32_t k) {
	array->length = 0;
	if (array->segments > 0) {
		// An allocated first segment always has an initialized k.
		k = array->k;
		array->cursor = (uint8_t *)array->inline_segment[0];
		array->end = array->cursor + segment_size(0, k) * item_size;
	} else {
		array->cursor = 0;
		array->end = 0;
	}
}

/// Frees a `GrowingArray` whose items were dropped.
FA_EXPORT void fa_segmented_free(FaSegmented *array, int64_t item_size, int32_t k) {
	if (array->storage == 2) { return; }
	k = array->k < 0 ? k : array->k;
	for (int64_t segment = array->storage == 3 ? 1 : 0; segment < array->segments; segment += 1) {
		fa_free(segmented_items(array, segment, item_size), (size_t)(segment_size(segment, k) * item_size), 16);
	}
	segmented_free_directory(array);
	if (!array->storage) { fa_free(array, sizeof(FaSegmented), sizeof(int64_t)); }
}

/// The items of a `GrowingArray` copied bytewise into a new array: a view
/// for the read-only methods (freed with `fa_array_free`, its items not
/// dropped), or a move when the `GrowingArray` is freed after.
FA_EXPORT FaArrayHeader *fa_segmented_flatten(const FaSegmented *array, int64_t item_size,
                                              int32_t k) {
	k = array->k < 0 ? k : array->k;
	FaArrayHeader *flat = fa_array_new(array->length, array->length, item_size);
	int64_t done = 0;
	for (int64_t segment = 0; done < array->length; segment += 1) {
		int64_t count = segment_size(segment, k);
		if (count > array->length - done) {
			count = array->length - done;
		}
		copy_bytes(flat->items + done * item_size, segmented_items(array, segment, item_size),
		           (size_t)(count * item_size));
		done += count;
	}
	return flat;
}

/// A `GrowingArray` taking the items of an array (moved bytewise; the
/// array's block and header are freed).
FA_EXPORT FaSegmented *fa_segmented_from_array(FaArrayHeader *flat, int64_t item_size,
                                               int32_t k) {
	fa_array_materialize(flat);
	FaSegmented *array = fa_segmented_new();
	fa_segmented_append(array, flat->items, flat->length, item_size, k);
	fa_array_free(flat, item_size);
	return array;
}

FA_EXPORT FaSegmented *fa_segmented_from_array_capacity(FaArrayHeader *flat, int64_t item_size, int32_t k, int64_t capacity) {
    fa_array_materialize(flat);
    FaSegmented *array = fa_segmented_new();
    if (capacity < flat->length) { capacity = flat->length; }
    fa_segmented_reserve(array, capacity, item_size, k);
    fa_segmented_append(array, flat->items, flat->length, item_size, k);
    fa_array_free(flat, item_size);
    return array;
}

// ── the allocator of wasm and of the freestanding target ───────────────

// Size classes over 64 KiB pages (`design/llvm-backend-plan.md`): a page
// holds blocks of one class, and starts with a header naming it, so that
// `fa_heap_free` finds the class of a block from its address alone. Freed
// blocks go to the free list of their class. Larger blocks take whole runs
// of pages ("spans"), returned to a list of free spans, merged with their
// free neighbors. Memory comes from `fa_pages` (`memory.grow`, `mmap`) and
// is never given back to the system.

#define PAGE_SIZE ((size_t)65536)
#define PAGE_HEADER ((size_t)16)
#define SMALL_LIMIT ((size_t)8192)
#define LARGE_CLASS 0xFFFFu
/// The class of a block with a mapping of its own (native Linux): blocks of
/// at least `MAPPED_LIMIT` bytes, which grow by moving their pages
/// (`fa_heap_grow`) and are given back to the system when freed.
#define MAPPED_CLASS 0xFFFEu
#define MAPPED_LIMIT ((size_t)1 << 20)

/// Anonymous mappings (`src/runtime.rs`): null where there are none.
void *fa_os_map(size_t length, int32_t protection);
void fa_os_unmap(void *start, size_t length);
void *fa_os_remap(void *start, size_t old_length, size_t length, void *at);

/// `count` new pages, aligned on `PAGE_SIZE`, or null (tier 0).
void *fa_pages(size_t count);

typedef struct {
	uint32_t class;
	uint32_t pages;
} PageHeader;

typedef struct FreeSpan {
	struct FreeSpan *next;
	size_t pages;
} FreeSpan;

typedef struct FreeBlock {
	struct FreeBlock *next;
} FreeBlock;

#define CLASS_COUNT 40
// each thread allocates from its own pages, and a block freed by another
// thread joins the free list of that thread
static FA_THREAD_LOCAL FreeBlock *free_blocks[CLASS_COUNT];
static FA_THREAD_LOCAL uint8_t *class_next[CLASS_COUNT];
static FA_THREAD_LOCAL uint8_t *class_end[CLASS_COUNT];
static FreeSpan *free_spans;
/// Guards `free_spans`, shared by the threads.
static int32_t spans_lock;

/// The class of a size, and its block size: multiples of 8 up to 128,
/// then four classes per power of two. A block whose size is a multiple of
/// 16 is aligned on 16 (the pages and their header are).
static uint32_t size_class(size_t size, size_t *block) {
	if (size <= 128) {
		size_t rounded = size <= 8 ? 8 : (size + 7) & ~(size_t)7;
		*block = rounded;
		return (uint32_t)(rounded / 8 - 1);
	}
	uint32_t class = 16;
	size_t base = 128;
	for (;;) {
		size_t step = base / 4;
		for (size_t part = 1; part <= 4; part += 1) {
			if (size <= base + step * part) {
				*block = base + step * part;
				return class;
			}
			class += 1;
		}
		base *= 2;
	}
}

/// Takes `pages` pages from the free spans (first fit), or from the system.
static uint8_t *take_pages_locked(size_t pages) {
	FreeSpan **link = &free_spans;
	while (*link != 0) {
		FreeSpan *span = *link;
		if (span->pages >= pages) {
			if (span->pages == pages) {
				*link = span->next;
			} else {
				FreeSpan *rest = (FreeSpan *)((uint8_t *)span + pages * PAGE_SIZE);
				rest->next = span->next;
				rest->pages = span->pages - pages;
				*link = rest;
			}
			return (uint8_t *)span;
		}
		link = &span->next;
	}
	// Native spans fit a 2 MiB huge page; wasm keeps its smaller refill.
	// Every unused page joins the free spans, including with huge pages.
	size_t minimum = sizeof(size_t) == 8 ? 32 : 16;
	size_t asked = pages < minimum ? minimum : pages;
	uint8_t *memory = fa_pages(asked);
	if (memory == 0) {
		return 0;
	}
	if (asked > pages) {
		FreeSpan *rest = (FreeSpan *)(memory + pages * PAGE_SIZE);
		rest->pages = asked - pages;
		rest->next = 0;
		// kept in address order
		FreeSpan **at = &free_spans;
		while (*at != 0 && (uint8_t *)*at < (uint8_t *)rest) {
			at = &(*at)->next;
		}
		rest->next = *at;
		*at = rest;
	}
	return memory;
}

/// Returns a span to the free spans, in address order, merged with its neighbors.
static void give_pages_locked(uint8_t *start, size_t pages) {
	FreeSpan *span = (FreeSpan *)start;
	span->pages = pages;
	FreeSpan *previous = 0;
	FreeSpan *next = free_spans;
	while (next != 0 && (uint8_t *)next < start) {
		previous = next;
		next = next->next;
	}
	span->next = next;
	if (next != 0 && start + pages * PAGE_SIZE == (uint8_t *)next) {
		span->pages += next->pages;
		span->next = next->next;
	}
	if (previous != 0 && (uint8_t *)previous + previous->pages * PAGE_SIZE == start) {
		previous->pages += span->pages;
		previous->next = span->next;
	} else if (previous != 0) {
		previous->next = span;
	} else {
		free_spans = span;
	}
}

static __attribute__((noinline)) uint8_t *take_pages(size_t pages) {
	spin_lock(&spans_lock);
	uint8_t *start = take_pages_locked(pages);
	spin_unlock(&spans_lock);
	return start;
}

static __attribute__((noinline)) void give_pages(uint8_t *start, size_t pages) {
	spin_lock(&spans_lock);
	give_pages_locked(start, pages);
	spin_unlock(&spans_lock);
}

/// A block of at least `size` bytes, aligned on `align` (8, or 16 for
/// 128-bit integers and the items of arrays); null when the system has no
/// more memory.
static __attribute__((noinline)) void *heap_alloc_slow(size_t size);

FA_EXPORT __attribute__((always_inline)) void *fa_heap_alloc(size_t size, size_t align) {
	if (align > 8) {
		size = size == 0 ? 16 : (size + 15) & ~(size_t)15;
	}
	// common path, inlined: a small block from the free list of its class
	if (size <= 128) {
		uint32_t class = size <= 8 ? 0 : (uint32_t)((size + 7) / 8 - 1);
		FreeBlock *free = free_blocks[class];
		if (__builtin_expect(free != 0, 1)) {
			free_blocks[class] = free->next;
			return free;
		}
		// Newly allocated recursive values usually have no freed block yet.
		// Keep the bump path beside the free-list path so constant sizes fold
		// the class and stride at each allocation site. Only a page refill
		// needs the out-of-line allocator.
		uint8_t *next = class_next[class];
		size_t block = (size_t)(class + 1) * 8;
		if (next != 0 && (size_t)(class_end[class] - next) >= block) {
			class_next[class] = next + block;
			return next;
		}
	}
	return heap_alloc_slow(size);
}

/// A new mapping of `length` bytes (a multiple of `PAGE_SIZE`) aligned on
/// `PAGE_SIZE`, readable and writable, or only reserved; null without one.
/// No huge pages: with `defrag` set to `madvise`, a fault in an advised
/// mapping may compact memory synchronously, which cost the strings and
/// arrays of the benchmarks more system time than the page faults it saved.
static uint8_t *map_aligned(size_t length, int32_t protection) {
	size_t alignment = PAGE_SIZE;
	uint8_t *start = fa_os_map(length + alignment, protection);
	if (start == 0) {
		return 0;
	}
	uint8_t *aligned = (uint8_t *)(((uintptr_t)start + alignment - 1) & ~(uintptr_t)(alignment - 1));
	if (aligned > start) {
		fa_os_unmap(start, (size_t)(aligned - start));
	}
	size_t tail = (size_t)((start + length + alignment) - (aligned + length));
	if (tail > 0) {
		fa_os_unmap(aligned + length, tail);
	}
	return aligned;
}

/// The length of the mapping of a block of `size` bytes.
static size_t mapped_length(size_t size) {
	return (size + PAGE_HEADER + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1);
}

/// `fa_heap_alloc` off its common path: a new block of a page, or pages.
static __attribute__((noinline)) void *heap_alloc_slow(size_t size) {
	if (size >= MAPPED_LIMIT && size <= (SIZE_MAX >> 18)) {
		size_t length = mapped_length(size);
		uint8_t *start = map_aligned(length, 3);
		if (start != 0) {
			PageHeader *header = (PageHeader *)start;
			header->class = MAPPED_CLASS;
			header->pages = (uint32_t)(length / PAGE_SIZE);
			return start + PAGE_HEADER;
		}
	}
	if (size > SMALL_LIMIT) {
		size_t pages = (size + PAGE_HEADER + PAGE_SIZE - 1) / PAGE_SIZE;
		if (pages == 0) {
			return 0;
		}
		uint8_t *start = take_pages(pages);
		if (start == 0) {
			return 0;
		}
		PageHeader *header = (PageHeader *)start;
		header->class = LARGE_CLASS;
		header->pages = (uint32_t)pages;
		return start + PAGE_HEADER;
	}
	size_t block;
	uint32_t class = size_class(size, &block);
	FreeBlock *free = free_blocks[class];
	if (free != 0) {
		free_blocks[class] = free->next;
		return free;
	}
	if (class_next[class] == 0 || (size_t)(class_end[class] - class_next[class]) < block) {
		uint8_t *page = take_pages(1);
		if (page == 0) {
			return 0;
		}
		PageHeader *header = (PageHeader *)page;
		header->class = class;
		header->pages = 1;
		class_next[class] = page + PAGE_HEADER;
		class_end[class] = page + PAGE_SIZE;
	}
	uint8_t *result = class_next[class];
	class_next[class] = result + block;
	return result;
}

/// Frees a block of `fa_heap_alloc`.
FA_EXPORT void fa_heap_free(void *bytes) {
	uint8_t *page = (uint8_t *)((uintptr_t)bytes & ~(uintptr_t)(PAGE_SIZE - 1));
	PageHeader *header = (PageHeader *)page;
	if (header->class == LARGE_CLASS) {
		give_pages(page, header->pages);
		return;
	}
	if (header->class == MAPPED_CLASS) {
		fa_os_unmap(page, (size_t)header->pages * PAGE_SIZE);
		return;
	}
	FreeBlock *block = bytes;
	block->next = free_blocks[header->class];
	free_blocks[header->class] = block;
}

/// `fa_grow` with the allocator of the runtime: a block with a mapping of its
/// own moves its pages to a larger mapping, without copying its bytes or
/// touching new pages; another block is copied into a new one.
FA_EXPORT void *fa_heap_grow(void *bytes, size_t old, size_t used, size_t size) {
	uint8_t *page = (uint8_t *)((uintptr_t)bytes & ~(uintptr_t)(PAGE_SIZE - 1));
	PageHeader *header = (PageHeader *)page;
	if (header->class == MAPPED_CLASS && size <= (SIZE_MAX >> 18)) {
		size_t length = mapped_length(size);
		size_t old_length = (size_t)header->pages * PAGE_SIZE;
		if (length <= old_length) {
			return bytes;
		}
		// a reservation, aligned, that the pages replace
		uint8_t *at = map_aligned(length, 0);
		if (at != 0) {
			uint8_t *moved = fa_os_remap(page, old_length, length, at);
			if (moved != 0) {
				((PageHeader *)moved)->pages = (uint32_t)(length / PAGE_SIZE);
				return moved + PAGE_HEADER;
			}
			fa_os_unmap(at, length);
		}
	}
	(void)old;
	void *grown = fa_heap_alloc(size, 16);
	if (grown == 0) {
		return 0;
	}
	copy_bytes(grown, bytes, used);
	fa_heap_free(bytes);
	return grown;
}

// ── root-owned recursive regions ──────────────────────────────────────

// Only the compiler's closed immutable-tree proof uses these entry points.
// Arena chunks have their own granularity, separate from the 64 KiB heap
// pages: an edge finds its root region by masking its address, without a
// per-node header. The first chunk also holds the region control block.
#define ARENA_CHUNK_SIZE ((size_t)1024)
typedef struct FaArena {
	struct FaArena *owner;
	struct FaArena *next;
	uint8_t *bump;
	uint8_t *end;
	struct FaArenaCleanup *cleanup;
} FaArena;

#define ARENA_HEADER ((sizeof(FaArena) + 15) & ~(size_t)15)

typedef struct FaArenaCleanup {
	struct FaArenaCleanup *next;
	void (*drop)(void *);
	void *node;
} FaArenaCleanup;

static FA_THREAD_LOCAL FaArena *tree_arena;
static FA_THREAD_LOCAL size_t tree_arena_depth;
// The bump and the end of the current arena, while it is current (its own
// `bump` and `end` are written back when it stops being current): one
// thread-local access each in `fa_arena_alloc`, no arena pointer to load and
// test. Both null without a current arena, so that any allocation refills.
static FA_THREAD_LOCAL uint8_t *tree_bump;
static FA_THREAD_LOCAL uint8_t *tree_end;
static size_t live_tree_arenas;

// Compressed links (`design/decisions.md`, "Compressed links for
// arena-allocated recursive values"; the compiler's `trees.rs`): on 64-bit
// Linux, the recursive edges of the arena types are 32-bit links, an
// address divided by 8, so every arena node lies in the first 32 GiB of the
// address space. The arena pages then come from one range reserved at the
// first one, from 2 GiB (below it: a non-PIE executable and its `brk` heap)
// to 32 GiB, or the largest power-of-two part of it the system grants (an
// address-space limit), at least 64 MiB. Pages are committed 1 MiB at a
// time and reused, never given back. No node is at address 0: link 0 is
// the absent edge. A full range is the out-of-memory stop.

/// Set by the compiler: whether arena links are compressed.
int32_t fa_compressed_links = 0;

/// A reservation of `length` bytes of address space at `start` exactly,
/// neither readable nor writable, or null.
void *fa_os_reserve(void *start, size_t length);
/// Makes `length` reserved bytes at `start` readable and writable; 0 on success.
int32_t fa_os_commit(void *start, size_t length);

#if UINTPTR_MAX > 0xFFFFFFFFu
#define ARENA_SPACE_START ((uintptr_t)1 << 31)
#define ARENA_SPACE_END ((uintptr_t)1 << 35)
#else
#define ARENA_SPACE_START ((uintptr_t)1 << 30)
#define ARENA_SPACE_END ((uintptr_t)1 << 31)
#endif
#define ARENA_SPACE_MINIMUM ((size_t)1 << 26)
#define ARENA_COMMIT ((size_t)1 << 20)

static uint8_t *arena_space_next;
static uint8_t *arena_space_committed;
static uint8_t *arena_space_end;
static FreeBlock *free_arena_pages;
static int32_t arena_space_lock;

static uint8_t *take_arena_page_locked(void) {
	if (free_arena_pages != 0) {
		FreeBlock *page = free_arena_pages;
		free_arena_pages = page->next;
		return (uint8_t *)page;
	}
	if (arena_space_end == 0) {
		uint8_t *start = (uint8_t *)ARENA_SPACE_START;
		size_t length = (size_t)(ARENA_SPACE_END - ARENA_SPACE_START);
		for (; length >= ARENA_SPACE_MINIMUM; length /= 2) {
			if (fa_os_reserve(start, length) == start) {
				arena_space_next = start;
				arena_space_committed = start;
				arena_space_end = start + length;
				break;
			}
		}
		if (arena_space_end == 0) {
			return 0;
		}
	}
	if (arena_space_next == arena_space_end) {
		static const uint8_t message[] =
			"stop: out of memory (the range of the arenas of recursive values is full)\n";
		fa_write(2, message, sizeof(message) - 1);
		fa_exit(101);
		__builtin_unreachable();
	}
	if (arena_space_next == arena_space_committed) {
		size_t room = (size_t)(arena_space_end - arena_space_committed);
		size_t step = room < ARENA_COMMIT ? room : ARENA_COMMIT;
		if (fa_os_commit(arena_space_committed, step) != 0) {
			return 0;
		}
		arena_space_committed += step;
	}
	uint8_t *page = arena_space_next;
	arena_space_next += ARENA_CHUNK_SIZE;
	return page;
}

// Ordinary arenas subdivide a heap page. Its first chunk holds a bitmap;
// the remaining chunks may belong to unrelated roots and are freed in
// any order. Only the last release gives the whole heap page back.
typedef struct FaArenaPage {
	struct FaArenaPage *next;
	struct FaArenaPage *previous;
	uint64_t free;
} FaArenaPage;

#define ARENA_PAGE_FREE ((UINT64_MAX >> (64 - PAGE_SIZE / ARENA_CHUNK_SIZE)) & ~(uint64_t)1)
static FaArenaPage *available_arena_pages;
static int32_t arena_pages_lock;

static void unlink_arena_page(FaArenaPage *page) {
	if (page->previous != 0) { page->previous->next = page->next; }
	else { available_arena_pages = page->next; }
	if (page->next != 0) { page->next->previous = page->previous; }
}

static uint8_t *take_small_arena_page(void) {
	spin_lock(&arena_pages_lock);
	FaArenaPage *page = available_arena_pages;
	if (page == 0) {
		page = (FaArenaPage *)take_pages(1);
		if (page == 0) { spin_unlock(&arena_pages_lock); return 0; }
		page->free = ARENA_PAGE_FREE;
		page->next = 0;
		page->previous = 0;
		available_arena_pages = page;
	}
	uint32_t slot = (uint32_t)__builtin_ctzll(page->free);
	page->free &= ~((uint64_t)1 << slot);
	if (page->free == 0) { unlink_arena_page(page); }
	spin_unlock(&arena_pages_lock);
	return (uint8_t *)page + slot * ARENA_CHUNK_SIZE;
}

static void give_small_arena_page(uint8_t *chunk) {
	FaArenaPage *page = (FaArenaPage *)((uintptr_t)chunk & ~(uintptr_t)(PAGE_SIZE - 1));
	uint32_t slot = (uint32_t)((chunk - (uint8_t *)page) / ARENA_CHUNK_SIZE);
	spin_lock(&arena_pages_lock);
	if (page->free == 0) {
		page->previous = 0;
		page->next = available_arena_pages;
		if (page->next != 0) { page->next->previous = page; }
		available_arena_pages = page;
	}
	page->free |= (uint64_t)1 << slot;
	int release = page->free == ARENA_PAGE_FREE;
	if (release) { unlink_arena_page(page); }
	spin_unlock(&arena_pages_lock);
	if (release) { give_pages((uint8_t *)page, 1); }
}

static __attribute__((noinline)) uint8_t *take_arena_page(void) {
	if (!fa_compressed_links) {
		return take_small_arena_page();
	}
	spin_lock(&arena_space_lock);
	uint8_t *page = take_arena_page_locked();
	spin_unlock(&arena_space_lock);
	return page;
}

static void give_arena_page(uint8_t *page) {
	if (!fa_compressed_links) {
		give_small_arena_page(page);
		return;
	}
	spin_lock(&arena_space_lock);
	((FreeBlock *)page)->next = free_arena_pages;
	free_arena_pages = (FreeBlock *)page;
	spin_unlock(&arena_space_lock);
}

FA_EXPORT __attribute__((always_inline)) void fa_arena_enter(void) {
	tree_arena_depth += 1;
}

static FaArena *arena_of(const void *node) {
	return node == 0 ? 0 : ((FaArena *)((uintptr_t)node & ~(uintptr_t)(ARENA_CHUNK_SIZE - 1)))->owner;
}

// Boxing a moved root extends its existing region. A borrowed root is
// deep-copied into the current/new region instead.
FA_EXPORT void fa_arena_enter_owned(const void *root) {
	if (tree_arena_depth == 0) {
		FaArena *arena = arena_of(root);
		tree_arena = arena;
		if (arena != 0) {
			tree_bump = arena->bump;
			tree_end = arena->end;
		}
	}
	tree_arena_depth += 1;
}

static void release_arena(FaArena *arena) {
	__atomic_fetch_sub(&live_tree_arenas, 1, __ATOMIC_RELAXED);
	FaArenaCleanup *cleanup = arena->cleanup;
	while (cleanup != 0) {
		cleanup->drop(cleanup->node);
		cleanup = cleanup->next;
	}
	while (arena != 0) {
		FaArena *next = arena->next;
		give_arena_page((uint8_t *)arena);
		arena = next;
	}
}

// Drops of intermediate roots inside a builder defer to its final root.
FA_EXPORT void fa_arena_drop(const void *node) {
	FaArena *arena = arena_of(node);
	if (arena != 0 && arena != tree_arena) { release_arena(arena); }
}

FA_EXPORT __attribute__((always_inline)) void fa_arena_leave(const void *root) {
	if (--tree_arena_depth == 0) {
		FaArena *arena = tree_arena;
		tree_arena = 0;
		if (arena != 0) {
			arena->bump = tree_bump;
			arena->end = tree_end;
			tree_bump = 0;
			tree_end = 0;
			if (arena_of(root) != arena) { release_arena(arena); }
		}
	}
}

static __attribute__((noinline)) void *arena_refill(size_t size, size_t align) {
	size_t offset = (ARENA_HEADER + align - 1) & ~(align - 1);
	FaArena *chunk = (FaArena *)take_arena_page();
	if (chunk == 0 || offset > ARENA_CHUNK_SIZE || size > ARENA_CHUNK_SIZE - offset) {
		static const uint8_t message[] = "stop: out of memory\n";
		fa_write(2, message, sizeof(message) - 1);
		fa_exit(101);
		__builtin_unreachable();
	}
	FaArena *arena = tree_arena;
	if (arena == 0) {
		arena = chunk;
		chunk->next = 0;
		chunk->cleanup = 0;
		tree_arena = arena;
		__atomic_fetch_add(&live_tree_arenas, 1, __ATOMIC_RELAXED);
	} else {
		chunk->next = arena->next;
		arena->next = chunk;
	}
	chunk->owner = arena;
	uint8_t *result = (uint8_t *)chunk + offset;
	tree_bump = result + size;
	tree_end = (uint8_t *)chunk + ARENA_CHUNK_SIZE;
	return result;
}

// All bumps remain eight-byte aligned. Larger ABI alignments add padding
// only when needed; common 24-byte nodes no longer consume 32 bytes.
FA_EXPORT __attribute__((always_inline)) void *fa_arena_alloc(size_t size, size_t align) {
	// at least 8 bytes: a node has an address of its own (without an arena,
	// bump and end are both null)
	size = size == 0 ? 8 : (size + 7) & ~(size_t)7;
	uint8_t *bump = tree_bump;
	size_t padding = align <= 8 ? 0 : -(uintptr_t)bump & (align - 1);
	size_t available = (size_t)(tree_end - bump);
	if (__builtin_expect(padding <= available && size <= available - padding, 1)) {
		uint8_t *result = bump + padding;
		tree_bump = result + size;
		return result;
	}
	return arena_refill(size, align);
}

// A private node can use signed 16-bit links, scaled by eight, when all
// nonconstant children are near its actual address. Positive codes at the
// start of the range are reserved for the root's constant members. The
// bounding interval may reject an extra node, but never accepts a collision.
static __attribute__((always_inline)) int arena_links_near(uintptr_t node,
                                                         uintptr_t first, uintptr_t last,
                                                         size_t constants) {
	if (first == UINTPTR_MAX) { return 1; }
	uintptr_t backwards = (uintptr_t)32768 * 8;
	uintptr_t forwards = (uintptr_t)32767 * 8;
	if ((first < node ? node - first > backwards : first - node > forwards)
	    || (last < node ? node - last > backwards : last - node > forwards)) { return 0; }
	return constants == 0 || last <= node || first > node + constants * 8;
}

// Reserve the wide layout first, so even a refill's real address is known.
// No allocation or user code runs before the unused tail is returned to the
// same active bump. The first word records the wide-layout flag until the
// constructor writes its full header. Owners and the compressed address domain
// do not change; layouts of the same size skip this API.
FA_EXPORT __attribute__((always_inline, assume_aligned(8))) void *fa_arena_alloc_near(size_t compact_size, size_t wide_size,
                                                                 size_t align, uintptr_t first,
                                                                 uintptr_t last, size_t constants) {
	uint8_t *node = fa_arena_alloc(wide_size, align);
	int near = arena_links_near((uintptr_t)node, first, last, constants);
	if (near) {
		size_t size = compact_size == 0 ? 8 : (compact_size + 7) & ~(size_t)7;
		tree_bump = node + size;
	}
	*(uint16_t *)node = near ? 0 : 0x4000;
	return node;
}

// Only nodes with an owning passive payload need a callback. Ordinary
// scalar/static/inline payloads have no per-node cleanup record at all.
FA_EXPORT FA_COLD void fa_arena_cleanup(void *node, void (*drop)(void *)) {
	FaArenaCleanup *cleanup = fa_arena_alloc(sizeof(FaArenaCleanup), _Alignof(FaArenaCleanup));
	cleanup->next = tree_arena->cleanup;
	cleanup->drop = drop;
	cleanup->node = node;
	tree_arena->cleanup = cleanup;
}

// Fresh child arrays of a proven builder keep the normal Array ABI, but
// their header and small buffers belong to the active root region. A buffer
// larger than one arena page uses the heap and one idempotent region cleanup.
static void region_array_cleanup(void *pointer) {
	FaArrayHeader *array = pointer;
	uint64_t marker = array_marker(array);
	free_array_items(array, (int64_t)(marker >> ARRAY_REGION_SIZE_SHIFT));
}

static void region_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size) {
	if (capacity <= array->capacity) { return; }
	if (capacity < 0 || item_size <= 0 ||
	    (uint64_t)capacity > SIZE_MAX / (uint64_t)item_size ||
	    (uint64_t)capacity > (uint64_t)INT64_MAX / (uint64_t)item_size) { out_of_memory(); }
	size_t bytes = (size_t)capacity * (size_t)item_size;
	uint64_t marker = array_marker(array);
	// Empty literals already have room for their first four items, without
	// exposing that room as their capacity until the first growth/reserve.
	if (!(marker & ARRAY_REGION_HEAP) && capacity <= 4 &&
	    array->items == (uint8_t *)array + INLINE_ITEMS) {
		array->capacity = capacity;
		return;
	}
	uint8_t *items;
	int heap = (marker & ARRAY_REGION_LOCAL) || bytes > ARENA_CHUNK_SIZE - ARENA_HEADER;
	if (!heap) {
		items = fa_arena_alloc(bytes, 16);
	} else {
		items = fa_alloc(bytes, 16);
		if (!(marker & (ARRAY_REGION_LOCAL | ARRAY_REGION_CLEANUP))) {
			fa_arena_cleanup(array, region_array_cleanup);
			marker |= ARRAY_REGION_CLEANUP;
		}
	}
	copy_bytes(items, array->items, (size_t)array->length * (size_t)item_size);
	free_array_items(array, item_size);
	marker &= ~ARRAY_REGION_HEAP;
	if (heap) { marker |= ARRAY_REGION_HEAP; }
	mark_items(array, marker);
	array->items = items;
	array->capacity = capacity;
}

FA_EXPORT __attribute__((always_inline)) void *fa_array_new_region(int64_t length, int64_t capacity, int64_t item_size) {
	if (capacity < length) { capacity = length; }
	if (length < 0 || capacity < 0 || item_size <= 0 ||
	    (uint64_t)item_size > UINT64_MAX >> ARRAY_REGION_SIZE_SHIFT ||
	    (uint64_t)capacity > SIZE_MAX / (uint64_t)item_size ||
	    (uint64_t)capacity > (uint64_t)INT64_MAX / (uint64_t)item_size) { out_of_memory(); }
	size_t room = (size_t)(capacity < 4 ? 4 : capacity);
	size_t bytes = room <= SIZE_MAX / (size_t)item_size ? room * (size_t)item_size : SIZE_MAX;
	int inline_buffer = bytes <= ARENA_CHUNK_SIZE - ARENA_HEADER - INLINE_ITEMS;
	FaArrayHeader *array = fa_arena_alloc(INLINE_ITEMS + (inline_buffer ? bytes : 0), 16);
	mark_items(array, ARRAY_REGION | (uint64_t)item_size << ARRAY_REGION_SIZE_SHIFT);
	array->items = inline_buffer ? (uint8_t *)array + INLINE_ITEMS : 0;
	array->length = 0;
	array->capacity = inline_buffer ? capacity : 0;
	if (!inline_buffer) { region_array_reserve(array, capacity, item_size); }
	array->length = length;
	return array;
}

// A fresh local child array keeps its header and first four slots in the
// builder frame. Larger temporary buffers use the ordinary heap, then move
// into the root region when a member takes ownership of the completed array.
FA_EXPORT __attribute__((always_inline)) void fa_array_init_region_local(FaArrayHeader *array, int64_t length,
                                          int64_t capacity, int64_t item_size) {
	if (capacity < length) { capacity = length; }
	if (length < 0 || capacity < 0 || item_size <= 0 ||
	    (uint64_t)item_size > UINT64_MAX >> ARRAY_REGION_SIZE_SHIFT ||
	    (uint64_t)capacity > SIZE_MAX / (uint64_t)item_size ||
	    (uint64_t)capacity > (uint64_t)INT64_MAX / (uint64_t)item_size) { out_of_memory(); }
	mark_items(array, ARRAY_REGION | ARRAY_REGION_LOCAL | (uint64_t)item_size << ARRAY_REGION_SIZE_SHIFT);
	array->items = (uint8_t *)array + INLINE_ITEMS;
	array->length = 0;
	array->capacity = capacity <= 4 ? capacity : 0;
	if (capacity > 4) { region_array_reserve(array, capacity, item_size); }
	array->length = length;
}

FA_EXPORT __attribute__((always_inline)) void *fa_array_into_region(void *header, int64_t item_size) {
	FaArrayHeader *source = header;
	uint64_t marker = array_marker(source);
	if ((marker & ARRAY_REGION) && !(marker & ARRAY_REGION_LOCAL)) { return source; }
	// The completed member may still expose its capacity. Shrinking an
	// immutable inline payload requires the compiler's separate closed proof.
	int64_t capacity = source->capacity;
	FaArrayHeader *array = fa_array_new_region(source->length, capacity, item_size);
	copy_bytes(array->items, source->items, (size_t)source->length * (size_t)item_size);
	free_array_items(source, item_size);
	if (!(marker & ARRAY_REGION_LOCAL)) { fa_free(source, INLINE_ITEMS, sizeof(int64_t)); }
	else {
		source->length = source->capacity = 0;
		source->items = (uint8_t *)source + INLINE_ITEMS;
	}
	return array;
}

// ── leak checks (tests) ─────────────────────────────────────────────────

/// The number of blocks allocated and not freed, counted by `fa_alloc` and
/// `fa_free` when the program is compiled with `Options::leak_check`.
size_t fa_live_blocks;

/// Frees the text builder of the calling thread (a thread of the parallel
/// pool, at the end of each loop: its blocks are counted by leak checks).
FA_EXPORT void fa_release_builder(void) {
	if (builder != 0) {
		fa_free(builder, builder_capacity, 1);
		builder = 0;
		builder_capacity = 0;
		builder_length = 0;
	}
}

/// At the normal end of a program compiled with `Options::leak_check`:
/// releases the runtime's own blocks, then reports the blocks the program
/// didn't free (`leak: <count> blocks` on the standard error, exit code 102).
FA_EXPORT void fa_leak_check(void) {
	fa_flush();
	if (builder != 0) {
		fa_free(builder, builder_capacity, 1);
		builder = 0;
		builder_capacity = 0;
	}
	if (argument_items != 0) {
		if (arguments_have_owners) {
			for (size_t index = 0; index < argument_count; index += 1) {
				if (string_owned(argument_items[index].capacity)) {
					fa_string_free(argument_items[index].bytes, argument_items[index].capacity);
				}
			}
		}
		fa_free(argument_items, sizeof(FaString) * (argument_count + 1), sizeof(void *));
		argument_items = 0;
		arguments_have_owners = 0;
	}
#if defined(__wasm__)
	if (argument_bytes != 0) {
		fa_free(argument_bytes, argument_bytes_size, 1);
		argument_bytes = 0;
		argument_bytes_size = 0;
	}
#endif
	if (fa_live_blocks != 0 || live_tree_arenas != 0) {
		fa_b_reset();
		append_c("leak: ");
		fa_b_u64((uint64_t)(fa_live_blocks + live_tree_arenas));
		append_c(" blocks");
		fa_b_line(2);
		fa_exit(102);
	}
}

// ── pools (collections/pools.md, implementation notes) ─────────────────

// A pool is a slab: chunks of one size (a power of two of pages), aligned
// on that size, so that the chunk of a slot is its address with the low
// bits cleared. A chunk starts with its header and its occupancy bitmap;
// its free slots form a list through the dead slots (the index of the next
// one), and the slots past `bumped` were never used. The pool keeps its
// chunks in a table (in order: iteration and `add` go to the first chunk
// with room), and a bitset of the chunks with a free slot. A chunk that
// becomes empty leaves the table: it is kept as the spare chunk, or freed.
// Elements never move. The generated code drops the elements (`delete`,
// `clear`, the drop of the pool) before the runtime frees their slots.

typedef struct {
	uint8_t *slots;
	uint32_t index;
	uint32_t live;
	uint32_t bumped;
	uint32_t free_head;
	uint64_t occupancy[];
} PoolChunk;

typedef struct {
	/// `pool.size`: the first field, read by the generated code.
	int64_t size;
	PoolChunk **chunks;
	uint64_t *available;
	PoolChunk *spare;
	uint32_t count;
	uint32_t table_capacity;
	uint32_t slot_size;
	uint32_t capacity;
	uint32_t pages;
	uint32_t slots_offset;
	uint32_t allocated;
	int64_t storage;
} FaPool;

/// A compact, read-only chunk needs no size alignment: only mutations
/// derive a chunk from its element's address. Those require a heap copy.
FA_EXPORT FaPool *fa_pool_local(void *buffer, int64_t length, int64_t item_size) {
	FaPool *pool = buffer;
	__builtin_memset(pool, 0, sizeof(FaPool));
	pool->storage = 3;
	pool->allocated = length ? 1 : 0;
	pool->size = length;
	pool->slot_size = item_size < 4 ? 4 : (uint32_t)item_size;
	pool->capacity = (uint32_t)length;
	pool->count = length ? 1 : 0;
	pool->table_capacity = 1;
	uint8_t *next = (uint8_t *)buffer + sizeof(FaPool);
	pool->chunks = (PoolChunk **)next;
	next += sizeof(PoolChunk *);
	next = (uint8_t *)(((uintptr_t)next + 7) & ~(uintptr_t)7);
	pool->available = (uint64_t *)next;
	*pool->available = 0;
	next += 8;
	PoolChunk *chunk = (PoolChunk *)next;
	*pool->chunks = chunk;
	size_t words = ((size_t)length + 63) / 64;
	__builtin_memset(chunk, 0, sizeof(PoolChunk) + words * 8);
	next += sizeof(PoolChunk) + words * 8;
	chunk->slots = (uint8_t *)(((uintptr_t)next + 15) & ~(uintptr_t)15);
	chunk->live = (uint32_t)length;
	chunk->bumped = (uint32_t)length;
	for (int64_t index = 0; index < length; index += 1) {
		chunk->occupancy[index / 64] |= (uint64_t)1 << (index % 64);
	}
	return pool;
}

/// A new empty pool of elements of `item_size` bytes.
FA_EXPORT FaPool *fa_pool_new(int64_t item_size) {
	FaPool *pool = fa_alloc(sizeof(FaPool), sizeof(int64_t));
	__builtin_memset(pool, 0, sizeof(FaPool));
	size_t slot = item_size < 4 ? 4 : (size_t)item_size;
	pool->slot_size = (uint32_t)slot;
	// at least 64 slots in a chunk
	size_t pages = 1;
	while (pages * PAGE_SIZE < sizeof(PoolChunk) + 16 + 64 * (slot + 1)) {
		pages *= 2;
	}
	size_t bytes = pages * PAGE_SIZE;
	size_t capacity = (bytes - sizeof(PoolChunk) - 16) * 8 / (slot * 8 + 1);
	size_t offset;
	for (;;) {
		offset = (sizeof(PoolChunk) + (capacity + 63) / 64 * 8 + 15) & ~(size_t)15;
		if (offset + capacity * slot <= bytes) {
			break;
		}
		capacity -= 1;
	}
	pool->capacity = (uint32_t)capacity;
	pool->pages = (uint32_t)pages;
	pool->slots_offset = (uint32_t)offset;
	return pool;
}

/// A new empty columnar (collections/columnars.md, implementation notes):
/// a pool whose slots are 4 bytes (they hold the free list), followed in
/// each chunk by one run per field, which the generated code places (it
/// chose the capacity, the pages and the offset of the slots).
FA_EXPORT FaPool *fa_columnar_new(int64_t capacity, int64_t pages, int64_t slots_offset) {
	FaPool *pool = fa_alloc(sizeof(FaPool), sizeof(int64_t));
	__builtin_memset(pool, 0, sizeof(FaPool));
	pool->slot_size = 4;
	pool->capacity = (uint32_t)capacity;
	pool->pages = (uint32_t)pages;
	pool->slots_offset = (uint32_t)slots_offset;
	return pool;
}

/// Pages aligned on their own size (a power of two of pages).
static uint8_t *take_aligned_pages(size_t pages) {
	if (pages == 1) {
		return take_pages(1);
	}
	uint8_t *start = take_pages(pages * 2 - 1);
	if (start == 0) {
		return 0;
	}
	size_t size = pages * PAGE_SIZE;
	uint8_t *aligned = (uint8_t *)(((uintptr_t)start + size - 1) & ~(uintptr_t)(size - 1));
	size_t before = (size_t)(aligned - start) / PAGE_SIZE;
	if (before > 0) {
		give_pages(start, before);
	}
	size_t after = pages - 1 - before;
	if (after > 0) {
		give_pages(aligned + size, after);
	}
	return aligned;
}

static PoolChunk *pool_chunk_of(const FaPool *pool, const void *slot) {
	return (PoolChunk *)((uintptr_t)slot & ~(uintptr_t)(pool->pages * PAGE_SIZE - 1));
}

/// A new chunk in the first free entry of the table (cold).
static __attribute__((noinline, cold)) PoolChunk *pool_new_chunk(FaPool *pool) {
	PoolChunk *chunk = pool->spare;
	pool->spare = 0;
	if (chunk == 0) {
		chunk = (PoolChunk *)take_aligned_pages(pool->pages);
		if (chunk == 0) {
			out_of_memory();
		}
		__atomic_fetch_add(&fa_live_blocks, 1, __ATOMIC_RELAXED);
	}
	size_t words = (pool->capacity + 63) / 64;
	__builtin_memset(chunk, 0, sizeof(PoolChunk) + words * 8);
	chunk->slots = (uint8_t *)chunk + pool->slots_offset;
	uint32_t index = 0;
	while (index < pool->count && pool->chunks[index] != 0) {
		index += 1;
	}
	if (index == pool->count) {
		if (pool->count == pool->table_capacity) {
			uint32_t grown = pool->table_capacity == 0 ? 4 : pool->table_capacity * 2;
			PoolChunk **chunks = fa_alloc(grown * sizeof(PoolChunk *), sizeof(void *));
			uint64_t *available = fa_alloc((grown + 63) / 64 * 8, 8);
			__builtin_memset(chunks, 0, grown * sizeof(PoolChunk *));
			__builtin_memset(available, 0, (grown + 63) / 64 * 8);
			if (pool->table_capacity > 0) {
				copy_bytes((uint8_t *)chunks, (const uint8_t *)pool->chunks,
				           pool->count * sizeof(PoolChunk *));
				copy_bytes((uint8_t *)available, (const uint8_t *)pool->available,
				           (pool->table_capacity + 63) / 64 * 8);
				fa_free(pool->chunks, pool->table_capacity * sizeof(PoolChunk *), sizeof(void *));
				fa_free(pool->available, (pool->table_capacity + 63) / 64 * 8, 8);
			}
			pool->chunks = chunks;
			pool->available = available;
			pool->table_capacity = grown;
		}
		pool->count += 1;
	}
	chunk->index = index;
	pool->chunks[index] = chunk;
	pool->allocated += 1;
	pool->available[index / 64] |= (uint64_t)1 << (index % 64);
	return chunk;
}

FA_EXPORT int64_t fa_pool_capacity(const FaPool *pool) {
    return ((int64_t)pool->allocated + (pool->spare != 0)) * pool->capacity;
}

FA_EXPORT void fa_pool_reserve(FaPool *pool, int64_t additional) {
    int64_t needed = storage_total(pool->size, additional, pool->slot_size);
    while (fa_pool_capacity(pool) < needed) { pool_new_chunk(pool); }
}

/// The slot of a new element (`add`): a free slot of the first chunk with
/// room, or a new chunk. The caller stores the element.
FA_EXPORT void *fa_pool_add_slot(FaPool *pool) {
	PoolChunk *chunk = 0;
	uint32_t words = (pool->count + 63) / 64;
	for (uint32_t word = 0; word < words; word += 1) {
		uint64_t bits = pool->available[word];
		if (bits != 0) {
			chunk = pool->chunks[word * 64 + (uint32_t)__builtin_ctzll(bits)];
			break;
		}
	}
	if (chunk == 0) {
		chunk = pool_new_chunk(pool);
	}
	uint32_t slot;
	if (chunk->free_head != 0) {
		slot = chunk->free_head - 1;
		uint32_t next;
		copy_bytes((uint8_t *)&next, chunk->slots + (size_t)slot * pool->slot_size, 4);
		chunk->free_head = next;
	} else {
		slot = chunk->bumped;
		chunk->bumped += 1;
	}
	chunk->occupancy[slot / 64] |= (uint64_t)1 << (slot % 64);
	chunk->live += 1;
	pool->size += 1;
	if (chunk->free_head == 0 && chunk->bumped == pool->capacity) {
		pool->available[chunk->index / 64] &= ~((uint64_t)1 << (chunk->index % 64));
	}
	return chunk->slots + (size_t)slot * pool->slot_size;
}

/// Literal slots are initialized directly in their final pool storage.
FA_EXPORT FaPool *fa_pool_new_into(int64_t length, int64_t item_size,
                                  void *destination, size_t bytes, uint64_t required) {
	if (destination && required <= bytes) { return fa_pool_local(destination, length, item_size); }
	FaPool *pool = fa_pool_new(item_size);
	for (int64_t index = 0; index < length; index += 1) { fa_pool_add_slot(pool); }
	return pool;
}

/// Frees the slot of an element already dropped or moved out (`delete`,
/// `take`, `deleteWhere`). A chunk left empty leaves the table.
FA_EXPORT void fa_pool_remove(FaPool *pool, void *element) {
	PoolChunk *chunk = pool_chunk_of(pool, element);
	uint32_t slot = (uint32_t)(((uint8_t *)element - chunk->slots) / pool->slot_size);
	chunk->occupancy[slot / 64] &= ~((uint64_t)1 << (slot % 64));
	copy_bytes((uint8_t *)element, (const uint8_t *)&chunk->free_head, 4);
	chunk->free_head = slot + 1;
	chunk->live -= 1;
	pool->size -= 1;
	uint32_t index = chunk->index;
	if (chunk->live == 0) {
		pool->chunks[index] = 0;
		pool->allocated -= 1;
		pool->available[index / 64] &= ~((uint64_t)1 << (index % 64));
		while (pool->count > 0 && pool->chunks[pool->count - 1] == 0) {
			pool->count -= 1;
		}
		if (pool->spare == 0) {
			pool->spare = chunk;
		} else {
			give_pages((uint8_t *)chunk, pool->pages);
			__atomic_fetch_sub(&fa_live_blocks, 1, __ATOMIC_RELAXED);
		}
		return;
	}
	pool->available[index / 64] |= (uint64_t)1 << (index % 64);
}

/// The position of the first element at or after `position` (a chunk index
/// in the high 32 bits, a slot in the low ones), in slot order, or
/// `UINT64_MAX` at the end. A chunk freed meanwhile (`deleteWhere`) is
/// skipped.
FA_EXPORT uint64_t fa_pool_seek(const FaPool *pool, uint64_t position) {
	uint32_t index = (uint32_t)(position >> 32);
	uint32_t slot = (uint32_t)position;
	while (index < pool->count) {
		PoolChunk *chunk = pool->chunks[index];
		if (chunk != 0) {
			uint32_t end = chunk->bumped;
			while (slot < end) {
				uint64_t word = chunk->occupancy[slot / 64] >> (slot % 64);
				if (word != 0) {
					slot += (uint32_t)__builtin_ctzll(word);
					if (slot >= end) {
						break;
					}
					return ((uint64_t)index << 32) | slot;
				}
				slot = (slot / 64 + 1) * 64;
			}
		}
		index += 1;
		slot = 0;
	}
	return UINT64_MAX;
}

/// The element at a position that `fa_pool_seek` returned.
FA_EXPORT void *fa_pool_at(const FaPool *pool, uint64_t position) {
	return pool->chunks[position >> 32]->slots + (size_t)(uint32_t)position * pool->slot_size;
}

static void pool_release_chunks(FaPool *pool) {
	for (uint32_t index = 0; index < pool->count; index += 1) {
		if (pool->chunks[index] != 0) {
			give_pages((uint8_t *)pool->chunks[index], pool->pages);
			__atomic_fetch_sub(&fa_live_blocks, 1, __ATOMIC_RELAXED);
		}
	}
	if (pool->table_capacity > 0) {
		__builtin_memset(pool->chunks, 0, pool->table_capacity * sizeof(PoolChunk *));
		__builtin_memset(pool->available, 0, (pool->table_capacity + 63) / 64 * 8);
	}
	pool->count = 0;
	pool->allocated = 0;
	pool->size = 0;
}

/// `clear`: frees every chunk but the spare one (the elements were dropped).
FA_EXPORT void fa_pool_clear(FaPool *pool) {
	pool_release_chunks(pool);
}

/// Frees a pool whose elements were dropped.
FA_EXPORT void fa_pool_free(FaPool *pool) {
	if (pool->storage) { return; }
	pool_release_chunks(pool);
	if (pool->spare != 0) {
		give_pages((uint8_t *)pool->spare, pool->pages);
		__atomic_fetch_sub(&fa_live_blocks, 1, __ATOMIC_RELAXED);
	}
	if (pool->table_capacity > 0) {
		fa_free(pool->chunks, pool->table_capacity * sizeof(PoolChunk *), sizeof(void *));
		fa_free(pool->available, (pool->table_capacity + 63) / 64 * 8, 8);
	}
	fa_free(pool, sizeof(FaPool), sizeof(int64_t));
}

// ── relations (advanced brief 8.5, relations.md) ────────────────────────
//
// A relation side is a field of the elements: a side holding one element
// (a direct side `(T in c)?`, a reverse one-to-one side, a symmetric side,
// an owner field) is the address of the target or null; a side holding
// several (a many-to-many direct side, a `Pool` reverse side) is a pointer
// to `FaLinks`, null while empty. The generated code passes the byte
// offsets of the sides in the elements. Elements never move (pools and
// columnars), so an address designates an element for its whole life.

/// A set of element addresses in insertion order (a JavaScript `Set` of
/// objects): `items` in order with null for a removed one, and an open
/// addressing index of positions once it holds more than a few.
typedef struct {
	/// `side.size`: the first field, read by the generated code.
	int64_t size;
	void **items;
	int64_t length;
	int64_t capacity;
	/// Positions + 1 (0: empty), or null while the set is small.
	uint32_t *index;
	int64_t mask;
} FaLinks;

#define LINKS_SMALL 8

static uint64_t address_hash(const void *pointer) {
	uint64_t value = (uint64_t)(uintptr_t)pointer;
	value ^= value >> 33;
	value *= 0xff51afd7ed558ccdull;
	value ^= value >> 33;
	return value;
}

static void links_insert_index(FaLinks *links, int64_t position) {
	uint64_t mask = (uint64_t)links->mask;
	uint64_t slot = address_hash(links->items[position]) & mask;
	while (links->index[slot] != 0) {
		slot = (slot + 1) & mask;
	}
	links->index[slot] = (uint32_t)(position + 1);
}

static void links_reindex(FaLinks *links) {
	if (links->index != 0) {
		fa_free(links->index, (size_t)(links->mask + 1) * sizeof(uint32_t), sizeof(uint32_t));
		links->index = 0;
	}
	if (links->capacity <= LINKS_SMALL) {
		return;
	}
	int64_t slots = 16;
	while (slots < links->capacity * 2) {
		slots *= 2;
	}
	links->mask = slots - 1;
	links->index = fa_alloc((size_t)slots * sizeof(uint32_t), sizeof(uint32_t));
	__builtin_memset(links->index, 0, (size_t)slots * sizeof(uint32_t));
	for (int64_t position = 0; position < links->length; position += 1) {
		if (links->items[position] != 0) {
			links_insert_index(links, position);
		}
	}
}

/// The position of an address in the set, or -1.
static int64_t links_find(const FaLinks *links, const void *item) {
	if (links == 0) {
		return -1;
	}
	if (links->index == 0) {
		for (int64_t position = 0; position < links->length; position += 1) {
			if (links->items[position] == item) {
				return position;
			}
		}
		return -1;
	}
	uint64_t mask = (uint64_t)links->mask;
	uint64_t slot = address_hash(item) & mask;
	for (;;) {
		uint32_t entry = links->index[slot];
		if (entry == 0) {
			return -1;
		}
		if (links->items[entry - 1] == item) {
			return (int64_t)entry - 1;
		}
		slot = (slot + 1) & mask;
	}
}

/// Whether the set holds the address (`side.includes(element)`).
FA_EXPORT int32_t fa_links_has(const FaLinks *links, const void *item) {
	return links_find(links, item) >= 0;
}

/// Adds an address to the set of a side (created when null): whether it
/// was missing.
FA_EXPORT int32_t fa_links_add(FaLinks **side, void *item) {
	FaLinks *links = *side;
	if (links == 0) {
		links = fa_alloc(sizeof(FaLinks), sizeof(int64_t));
		__builtin_memset(links, 0, sizeof(FaLinks));
		*side = links;
	} else if (links_find(links, item) >= 0) {
		return 0;
	}
	if (links->length == links->capacity) {
		if (links->length > 0 && links->size * 2 <= links->length) {
			// the removed ones leave first
			int64_t kept = 0;
			for (int64_t position = 0; position < links->length; position += 1) {
				if (links->items[position] != 0) {
					links->items[kept] = links->items[position];
					kept += 1;
				}
			}
			links->length = kept;
		} else {
			int64_t grown = links->capacity == 0 ? 4 : links->capacity * 2;
			void **items = fa_alloc((size_t)grown * sizeof(void *), sizeof(void *));
			if (links->capacity > 0) {
				copy_bytes((uint8_t *)items, (const uint8_t *)links->items,
				           (size_t)links->length * sizeof(void *));
				fa_free(links->items, (size_t)links->capacity * sizeof(void *), sizeof(void *));
			}
			links->items = items;
			links->capacity = grown;
		}
		links_reindex(links);
	}
	int64_t position = links->length;
	links->items[position] = item;
	links->length += 1;
	links->size += 1;
	if (links->index != 0) {
		links_insert_index(links, position);
	}
	return 1;
}

/// Removes an address from the set: whether it was there.
FA_EXPORT int32_t fa_links_delete(FaLinks *links, const void *item) {
	int64_t position = links_find(links, item);
	if (position < 0) {
		return 0;
	}
	links->items[position] = 0;
	links->size -= 1;
	if (links->index != 0) {
		// backward-shift deletion keeps the probe sequences whole
		uint64_t mask = (uint64_t)links->mask;
		uint64_t slot = address_hash(item) & mask;
		while (links->index[slot] != (uint32_t)(position + 1)) {
			slot = (slot + 1) & mask;
		}
		uint64_t hole = slot;
		for (;;) {
			slot = (slot + 1) & mask;
			uint32_t entry = links->index[slot];
			if (entry == 0) {
				break;
			}
			uint64_t home = address_hash(links->items[entry - 1]) & mask;
			if (((slot - home) & mask) >= ((slot - hole) & mask)) {
				links->index[hole] = entry;
				hole = slot;
			}
		}
		links->index[hole] = 0;
	}
	if (links->size == 0) {
		links->length = 0;
	}
	return 1;
}

/// Frees a set (null for an empty side).
FA_EXPORT void fa_links_free(FaLinks *links) {
	if (links == 0) {
		return;
	}
	if (links->index != 0) {
		fa_free(links->index, (size_t)(links->mask + 1) * sizeof(uint32_t), sizeof(uint32_t));
	}
	if (links->capacity > 0) {
		fa_free(links->items, (size_t)links->capacity * sizeof(void *), sizeof(void *));
	}
	fa_free(links, sizeof(FaLinks), sizeof(int64_t));
}

/// The position of the first address at or after `position`, in insertion
/// order, or `UINT64_MAX` at the end.
FA_EXPORT uint64_t fa_links_seek(const FaLinks *links, uint64_t position) {
	if (links == 0) {
		return UINT64_MAX_VALUE;
	}
	while (position < (uint64_t)links->length) {
		if (links->items[position] != 0) {
			return position;
		}
		position += 1;
	}
	return UINT64_MAX_VALUE;
}

/// The address at a position that `fa_links_seek` returned.
FA_EXPORT void *fa_links_at(const FaLinks *links, uint64_t position) {
	return links->items[position];
}

static void **side_of(void *element, int64_t offset) {
	return (void **)((uint8_t *)element + offset);
}

/// Notifies the source of the relation side at `offset` of `object`, when
/// `notify` (the sides are reactive) and something read it (reactivity,
/// below).
static void notify_side(int32_t notify, void *object, int64_t offset);

/// `holder.direct = target` (null: `none`): the two-sided update of a
/// one-to-many relation (`many`: the reverse sides are sets), of a
/// one-to-one one, or of a symmetric one (`reverse` is `direct`). With
/// `notify`, each side changed notifies its source.
FA_EXPORT void fa_relate_one(void *holder, int64_t direct, int64_t reverse, void *target,
                             int32_t many, int32_t notify) {
	void **slot = side_of(holder, direct);
	void *old = *slot;
	if (old == target) {
		return;
	}
	if (many) {
		if (old != 0) {
			fa_links_delete(*(FaLinks **)side_of(old, reverse), holder);
			notify_side(notify, old, reverse);
		}
		*slot = target;
		notify_side(notify, holder, direct);
		if (target != 0) {
			fa_links_add((FaLinks **)side_of(target, reverse), holder);
			notify_side(notify, target, reverse);
		}
		return;
	}
	if (old != 0) {
		*side_of(old, reverse) = 0;
		notify_side(notify, old, reverse);
	}
	if (target != 0) {
		// the previous holder of the target loses it
		void *previous = *side_of(target, reverse);
		if (previous != 0) {
			*side_of(previous, direct) = 0;
			notify_side(notify, previous, direct);
		}
		*side_of(target, reverse) = holder;
		notify_side(notify, target, reverse);
	}
	*slot = target;
	notify_side(notify, holder, direct);
}

/// `holder.direct.add(target)` of a many-to-many relation: whether the
/// link is new.
FA_EXPORT int32_t fa_relate_add(void *holder, int64_t direct, int64_t reverse, void *target,
                                int32_t notify) {
	if (!fa_links_add((FaLinks **)side_of(holder, direct), target)) {
		return 0;
	}
	fa_links_add((FaLinks **)side_of(target, reverse), holder);
	notify_side(notify, holder, direct);
	notify_side(notify, target, reverse);
	return 1;
}

/// `holder.direct.delete(target)` of a many-to-many relation: whether there
/// was a link.
FA_EXPORT int32_t fa_relate_delete(void *holder, int64_t direct, int64_t reverse, void *target,
                                   int32_t notify) {
	if (!fa_links_delete(*(FaLinks **)side_of(holder, direct), target)) {
		return 0;
	}
	fa_links_delete(*(FaLinks **)side_of(target, reverse), holder);
	notify_side(notify, holder, direct);
	notify_side(notify, target, reverse);
	return 1;
}

/// `holder.direct.clear()` of a many-to-many relation.
FA_EXPORT void fa_relate_clear(void *holder, int64_t direct, int64_t reverse, int32_t notify) {
	FaLinks *links = *(FaLinks **)side_of(holder, direct);
	if (links == 0) {
		return;
	}
	for (int64_t position = 0; position < links->length; position += 1) {
		void *target = links->items[position];
		if (target != 0) {
			fa_links_delete(*(FaLinks **)side_of(target, reverse), holder);
			notify_side(notify, target, reverse);
		}
	}
	fa_links_free(links);
	*side_of(holder, direct) = 0;
	notify_side(notify, holder, direct);
}

/// Unlinks an element about to leave its collection (8.5 `unlink`), from
/// one relation: `kind` 0 one-to-many, 1 one-to-one, 2 many-to-many, 3
/// symmetric, 4 owner field; `holder` 1 when the element holds the direct
/// side, 0 when it is a target. `owner` is the address of the owner field
/// (kind 4), or null.
FA_EXPORT void fa_unlink(void *element, int32_t kind, int32_t holder, int64_t direct,
                         int64_t reverse, void **owner, int32_t notify) {
	if (holder) {
		void *target = *side_of(element, direct);
		if (kind == 2) {
			FaLinks *links = (FaLinks *)target;
			for (int64_t position = 0; links != 0 && position < links->length; position += 1) {
				void *item = links->items[position];
				if (item != 0) {
					fa_links_delete(*(FaLinks **)side_of(item, reverse), element);
					notify_side(notify, item, reverse);
				}
			}
		} else if (target != 0) {
			if (kind == 0) {
				fa_links_delete(*(FaLinks **)side_of(target, reverse), element);
			} else {
				*side_of(target, reverse) = 0;
			}
			notify_side(notify, target, reverse);
		}
		return;
	}
	switch (kind) {
	case 0:
	case 2: {
		FaLinks *links = *(FaLinks **)side_of(element, reverse);
		for (int64_t position = 0; links != 0 && position < links->length; position += 1) {
			void *item = links->items[position];
			if (item == 0) {
				continue;
			}
			if (kind == 0) {
				*side_of(item, direct) = 0;
			} else {
				fa_links_delete(*(FaLinks **)side_of(item, direct), element);
			}
			notify_side(notify, item, direct);
		}
		break;
	}
	case 1: {
		void *other = *side_of(element, reverse);
		if (other != 0) {
			*side_of(other, direct) = 0;
			notify_side(notify, other, direct);
		}
		break;
	}
	case 3: {
		void *other = *side_of(element, direct);
		if (other != 0 && other != element) {
			*side_of(other, direct) = 0;
			notify_side(notify, other, direct);
		}
		break;
	}
	default:
		if (owner != 0 && *owner == element) {
			*owner = 0;
		}
	}
}

/// A map of addresses (an original element to its copy), to copy an owner
/// of related collections.
typedef struct {
	void **keys;
	void **values;
	int64_t mask;
} FaAddressMap;

/// A map for `count` addresses.
FA_EXPORT FaAddressMap *fa_address_map_new(int64_t count) {
	FaAddressMap *map = fa_alloc(sizeof(FaAddressMap), sizeof(int64_t));
	int64_t slots = 16;
	while (slots < count * 2) {
		slots *= 2;
	}
	map->mask = slots - 1;
	map->keys = fa_alloc((size_t)slots * sizeof(void *), sizeof(void *));
	map->values = fa_alloc((size_t)slots * sizeof(void *), sizeof(void *));
	__builtin_memset(map->keys, 0, (size_t)slots * sizeof(void *));
	return map;
}

/// Maps `key` to `value` (the map was made for enough addresses).
FA_EXPORT void fa_address_map_put(FaAddressMap *map, void *key, void *value) {
	uint64_t mask = (uint64_t)map->mask;
	uint64_t slot = address_hash(key) & mask;
	while (map->keys[slot] != 0 && map->keys[slot] != key) {
		slot = (slot + 1) & mask;
	}
	map->keys[slot] = key;
	map->values[slot] = value;
}

/// The value of `key`, or null.
FA_EXPORT void *fa_address_map_get(const FaAddressMap *map, const void *key) {
	if (key == 0) {
		return 0;
	}
	uint64_t mask = (uint64_t)map->mask;
	uint64_t slot = address_hash(key) & mask;
	while (map->keys[slot] != 0) {
		if (map->keys[slot] == key) {
			return map->values[slot];
		}
		slot = (slot + 1) & mask;
	}
	return 0;
}

/// A copy of a set of links with each address replaced by its value in the
/// map, the ones the map doesn't hold left out (null when empty).
FA_EXPORT FaLinks *fa_links_relink(const FaLinks *links, const FaAddressMap *map) {
	FaLinks *copy = 0;
	for (int64_t position = 0; links != 0 && position < links->length; position += 1) {
		void *mapped = fa_address_map_get(map, links->items[position]);
		if (mapped != 0) {
			fa_links_add(&copy, mapped);
		}
	}
	return copy;
}

FA_EXPORT void fa_address_map_free(FaAddressMap *map) {
	fa_free(map->keys, (size_t)(map->mask + 1) * sizeof(void *), sizeof(void *));
	fa_free(map->values, (size_t)(map->mask + 1) * sizeof(void *), sizeof(void *));
	fa_free(map, sizeof(FaAddressMap), sizeof(int64_t));
}

// ── reactivity (advanced brief 11, advanced/reactivity.md) ──────────────
//
// Push-pull signals, as `packages/codegen-js/runtime/reactive.js`: values
// stay plain, a reactive place has a side source (a version, its observers,
// its children created on demand: a field, an element or entry by key, the
// length or size). A derived value and a watch block are observers, made by
// the generated code with the functions that compute or run them. Every
// source and observer lives until the end of the program
// (`fa_reactive_release`): a disposed watch block only stops running.

typedef struct FaObserver FaObserver;
typedef struct FaSource FaSource;

typedef struct {
	int64_t kind;
	int64_t key;
	FaSource *source;
} FaChild;

struct FaSource {
	/// 0: a source, 1: a derived value (an observer), 2: a watch block.
	int64_t kind;
	int64_t version;
	FaObserver **observers;
	int64_t observer_count;
	int64_t observer_capacity;
	FaSource *parent;
	FaChild *children;
	int64_t child_count;
	int64_t child_capacity;
};

/// A derived value or a watch block. `run` computes a derived value (into
/// `value`, setting `changed`) or runs the body of a watch block (setting
/// its cleanup); `cleanup` runs and drops the cleanup of a watch block;
/// `release` drops what the generated code owns (the environment, the
/// value, the cleanup).
struct FaObserver {
	/// A derived value is the source of what reads it.
	FaSource source;
	void *value;
	void *env;
	void (*run)(FaObserver *);
	void (*cleanup)(FaObserver *);
	void (*release)(FaObserver *);
	FaSource **reads;
	int64_t *versions;
	int64_t read_count;
	int64_t read_capacity;
	int64_t sequence;
	int32_t dirty;
	int32_t computed;
	int32_t computing;
	int32_t pending;
	int32_t disposed;
	int32_t changed;
	int32_t watch;
	int32_t padding;
	int64_t value_size;
	/// The cleanup of a watch block: a function value `{ function,
	/// environment }`.
	void *cleanup_closure[2];
};

static FaObserver *reactive_observer = 0;
static FaObserver **reactive_pending = 0;
static int64_t reactive_pending_count = 0;
static int64_t reactive_pending_capacity = 0;
static int32_t reactive_flushing = 0;
static int64_t reactive_sequence = 0;
// everything made, released at the end
static void **reactive_made = 0;
static int64_t reactive_made_count = 0;
static int64_t reactive_made_capacity = 0;
// the sources of `reactive` fields: by object address and field
static FaChild *field_sources = 0;
static int64_t field_source_count = 0;
static int64_t field_source_capacity = 0;
/// The sources given to the parameters of the function being called
/// (D24), set by the caller and read by the callee as it starts.
static FaSource *argument_sources[16];

FA_EXPORT void fa_argument_source_set(int32_t index, FaSource *source) {
	argument_sources[index & 15] = source;
}

FA_EXPORT FaSource *fa_argument_source_get(int32_t index) {
	return argument_sources[index & 15];
}

/// Grows an array of `size`-byte items to hold one more.
static void *grow_items(void *items, int64_t count, int64_t *capacity, size_t size) {
	if (count < *capacity) {
		return items;
	}
	int64_t grown = *capacity == 0 ? 4 : *capacity * 2;
	void *larger = fa_alloc((size_t)grown * size, sizeof(void *));
	if (*capacity > 0) {
		copy_bytes((uint8_t *)larger, (const uint8_t *)items, (size_t)count * size);
		fa_free(items, (size_t)*capacity * size, sizeof(void *));
	}
	*capacity = grown;
	return larger;
}

static void free_items(void *items, int64_t capacity, size_t size) {
	if (capacity > 0) {
		fa_free(items, (size_t)capacity * size, sizeof(void *));
	}
}

static void remember(void *made) {
	reactive_made = grow_items(reactive_made, reactive_made_count, &reactive_made_capacity,
	                           sizeof(void *));
	reactive_made[reactive_made_count] = made;
	reactive_made_count += 1;
}

static void source_init(FaSource *source, FaSource *parent) {
	__builtin_memset(source, 0, sizeof(FaSource));
	source->parent = parent;
}

/// A new source, for a `reactive` value.
FA_EXPORT FaSource *fa_source_new(void) {
	FaSource *source = fa_alloc(sizeof(FaSource), sizeof(int64_t));
	source_init(source, 0);
	remember(source);
	return source;
}

/// The child of a source (`kind` 0: the length or size, 1: a field by
/// `key`, 2: an element or entry by `key`), created on demand; null below
/// a null source (a parameter given a plain value).
FA_EXPORT FaSource *fa_source_child(FaSource *source, int64_t kind, int64_t key) {
	if (source == 0) {
		return 0;
	}
	for (int64_t index = 0; index < source->child_count; index += 1) {
		FaChild *child = &source->children[index];
		if (child->kind == kind && child->key == key) {
			return child->source;
		}
	}
	FaSource *child = fa_alloc(sizeof(FaSource), sizeof(int64_t));
	source_init(child, source);
	remember(child);
	source->children = grow_items(source->children, source->child_count,
	                              &source->child_capacity, sizeof(FaChild));
	source->children[source->child_count] = (FaChild){kind, key, child};
	source->child_count += 1;
	return child;
}

/// The source of a `reactive` field of the object at `object`: kept outside
/// the object, so that a copy of it starts without dependents.
FA_EXPORT FaSource *fa_field_source(const void *object, int64_t field) {
	int64_t address = (int64_t)(uintptr_t)object;
	for (int64_t index = 0; index < field_source_count; index += 1) {
		FaChild *entry = &field_sources[index];
		if (entry->kind == address && entry->key == field) {
			return entry->source;
		}
	}
	FaSource *source = fa_source_new();
	field_sources = grow_items(field_sources, field_source_count, &field_source_capacity,
	                           sizeof(FaChild));
	field_sources[field_source_count] = (FaChild){address, field, source};
	field_source_count += 1;
	return source;
}

/// The key of the source of a relation side in `fa_field_source`: its byte
/// offset, apart from the indices of `reactive` fields.
#define FA_SIDE_KEY ((int64_t)1 << 62)

FA_EXPORT void fa_notify(FaSource *source);

static void notify_side(int32_t notify, void *object, int64_t offset) {
	if (!notify || object == 0) {
		return;
	}
	int64_t address = (int64_t)(uintptr_t)object;
	for (int64_t index = 0; index < field_source_count; index += 1) {
		FaChild *entry = &field_sources[index];
		if (entry->kind == address && entry->key == (FA_SIDE_KEY | offset)) {
			fa_notify(entry->source);
			return;
		}
	}
}

/// Records that the observer being computed read `source` (nothing outside
/// a derived value or a watch block, nor for a null source).
FA_EXPORT void fa_track(FaSource *source) {
	FaObserver *observer = reactive_observer;
	if (observer == 0 || source == 0) {
		return;
	}
	for (int64_t index = 0; index < observer->read_count; index += 1) {
		if (observer->reads[index] == source) {
			return;
		}
	}
	int64_t capacity = observer->read_capacity;
	observer->reads = grow_items(observer->reads, observer->read_count, &capacity,
	                             sizeof(FaSource *));
	capacity = observer->read_capacity;
	observer->versions = grow_items(observer->versions, observer->read_count, &capacity,
	                                sizeof(int64_t));
	observer->read_capacity = capacity;
	observer->reads[observer->read_count] = source;
	observer->versions[observer->read_count] = source->version;
	observer->read_count += 1;
	for (int64_t index = 0; index < source->observer_count; index += 1) {
		if (source->observers[index] == observer) {
			return;
		}
	}
	source->observers = grow_items(source->observers, source->observer_count,
	                               &source->observer_capacity, sizeof(FaObserver *));
	source->observers[source->observer_count] = observer;
	source->observer_count += 1;
}

/// Pushes a change to an observer: a derived value becomes dirty (and so do
/// its observers), a watch block pending, once.
static void mark(FaObserver *observer) {
	if (observer->watch) {
		if (!observer->pending && !observer->disposed) {
			observer->pending = 1;
			reactive_pending = grow_items(reactive_pending, reactive_pending_count,
			                              &reactive_pending_capacity, sizeof(FaObserver *));
			reactive_pending[reactive_pending_count] = observer;
			reactive_pending_count += 1;
		}
		return;
	}
	if (observer->dirty) {
		return;
	}
	observer->dirty = 1;
	for (int64_t index = 0; index < observer->source.observer_count; index += 1) {
		mark(observer->source.observers[index]);
	}
}

static void mark_observers(FaSource *source) {
	for (int64_t index = 0; index < source->observer_count; index += 1) {
		mark(source->observers[index]);
	}
}

static void bump(FaSource *source) {
	source->version += 1;
	mark_observers(source);
	for (int64_t index = 0; index < source->child_count; index += 1) {
		bump(source->children[index].source);
	}
}

/// A write of the place of `source`: bumps it and the sources below it,
/// then the whole values above it, and pushes the change to their
/// observers (nothing for a null source).
FA_EXPORT void fa_notify(FaSource *source) {
	if (source == 0) {
		return;
	}
	bump(source);
	for (FaSource *parent = source->parent; parent != 0; parent = parent->parent) {
		parent->version += 1;
		mark_observers(parent);
	}
}

/// Removes an observer from the observers of what it read.
static void unsubscribe(FaObserver *observer) {
	for (int64_t index = 0; index < observer->read_count; index += 1) {
		FaSource *source = observer->reads[index];
		for (int64_t at = 0; at < source->observer_count; at += 1) {
			if (source->observers[at] == observer) {
				source->observers[at] = source->observers[source->observer_count - 1];
				source->observer_count -= 1;
				break;
			}
		}
	}
	observer->read_count = 0;
}

static void refresh(FaObserver *node);

/// Whether a source an observer read changed since (the derived ones
/// brought up to date first).
static int32_t outdated(FaObserver *observer) {
	for (int64_t index = 0; index < observer->read_count; index += 1) {
		FaSource *source = observer->reads[index];
		if (source->kind == 1) {
			refresh((FaObserver *)source);
		}
		if (source->version != observer->versions[index]) {
			return 1;
		}
	}
	return 0;
}

static FA_NORETURN void reactive_stop(const char *reason) {
	fa_flush();
	fa_write(2, (const uint8_t *)"stop: ", 6);
	fa_write(2, (const uint8_t *)reason, c_length(reason));
	fa_write(2, (const uint8_t *)"\n", 1);
	fa_exit(101);
}

/// Brings a derived value up to date: computes it again when it was never
/// computed or when what it read changed; its version changes only when
/// its value does.
static void refresh(FaObserver *node) {
	if (!node->dirty) {
		return;
	}
	if (node->computed && !outdated(node)) {
		node->dirty = 0;
		return;
	}
	if (node->computing) {
		reactive_stop("a derived value depends on itself");
	}
	unsubscribe(node);
	FaObserver *saved = reactive_observer;
	reactive_observer = node;
	node->computing = 1;
	node->changed = 1;
	node->run(node);
	node->computing = 0;
	reactive_observer = saved;
	node->dirty = 0;
	node->computed = 1;
	if (node->changed) {
		node->source.version += 1;
	}
}

static FaObserver *observer_new(void (*run)(FaObserver *), void (*cleanup)(FaObserver *),
                                void (*release)(FaObserver *), void *env, int64_t value_size) {
	FaObserver *node = fa_alloc(sizeof(FaObserver), sizeof(int64_t));
	__builtin_memset(node, 0, sizeof(FaObserver));
	node->run = run;
	node->cleanup = cleanup;
	node->release = release;
	node->env = env;
	if (value_size > 0) {
		node->value = fa_alloc((size_t)value_size, 16);
		__builtin_memset(node->value, 0, (size_t)value_size);
		node->value_size = value_size;
	}
	remember(node);
	return node;
}

/// A `derived` value, computed when it is read; `value_size` bytes hold
/// its value.
FA_EXPORT FaObserver *fa_derived_new(void (*compute)(FaObserver *),
                                     void (*release)(FaObserver *), void *env,
                                     int64_t value_size) {
	FaObserver *node = observer_new(compute, 0, release, env, value_size);
	node->source.kind = 1;
	node->dirty = 1;
	return node;
}

/// Reads a derived value: brings it up to date and tracks it; returns the
/// address of its value.
FA_EXPORT void *fa_derived_read(FaObserver *node) {
	refresh(node);
	fa_track(&node->source);
	return node->value;
}

/// Runs a watch block: its previous cleanup (untracked), then its body
/// under tracking.
static void run_watch(FaObserver *node) {
	unsubscribe(node);
	FaObserver *saved = reactive_observer;
	reactive_observer = 0;
	node->cleanup(node);
	reactive_observer = node;
	node->run(node);
	reactive_observer = saved;
}

/// A `watch` block: runs at once, then again after a value it read changed.
FA_EXPORT FaObserver *fa_watch_new(void (*run)(FaObserver *), void (*cleanup)(FaObserver *),
                                   void (*release)(FaObserver *), void *env) {
	FaObserver *node = observer_new(run, cleanup, release, env, 0);
	node->source.kind = 2;
	node->watch = 1;
	reactive_sequence += 1;
	node->sequence = reactive_sequence;
	run_watch(node);
	return node;
}

/// Destroys a watch block (`subscription.stop()`, the end of its scope):
/// runs its cleanup and leaves the observers of what it read.
FA_EXPORT void fa_dispose(FaObserver *node) {
	if (node == 0 || node->disposed) {
		return;
	}
	node->disposed = 1;
	unsubscribe(node);
	FaObserver *saved = reactive_observer;
	reactive_observer = 0;
	node->cleanup(node);
	reactive_observer = saved;
}

/// Runs the pending watch blocks in declaration order, each once, then the
/// ones they made pending, for at most 100 rounds.
FA_EXPORT void fa_reactive_flush(void) {
	if (reactive_flushing || reactive_pending_count == 0) {
		return;
	}
	reactive_flushing = 1;
	int32_t rounds = 0;
	while (reactive_pending_count > 0) {
		rounds += 1;
		if (rounds > 100) {
			reactive_stop("the `watch` blocks kept changing what they read: stopped after 100 "
			              "rounds");
		}
		int64_t count = reactive_pending_count;
		FaObserver **batch = fa_alloc((size_t)count * sizeof(FaObserver *), sizeof(void *));
		copy_bytes((uint8_t *)batch, (const uint8_t *)reactive_pending,
		           (size_t)count * sizeof(FaObserver *));
		reactive_pending_count = 0;
		// declaration order (insertion sort: few blocks)
		for (int64_t index = 1; index < count; index += 1) {
			FaObserver *node = batch[index];
			int64_t at = index;
			while (at > 0 && batch[at - 1]->sequence > node->sequence) {
				batch[at] = batch[at - 1];
				at -= 1;
			}
			batch[at] = node;
		}
		for (int64_t index = 0; index < count; index += 1) {
			FaObserver *node = batch[index];
			node->pending = 0;
			if (!node->disposed && outdated(node)) {
				run_watch(node);
			}
		}
		fa_free(batch, (size_t)count * sizeof(FaObserver *), sizeof(void *));
	}
	reactive_flushing = 0;
}

/// The value block of a derived value, the environment of an observer, and
/// the cleanup slot of a watch block, for the generated code.
FA_EXPORT void *fa_observer_value(FaObserver *node) {
	return node->value;
}

FA_EXPORT void *fa_observer_env(FaObserver *node) {
	return node->env;
}

FA_EXPORT void **fa_observer_cleanup(FaObserver *node) {
	return node->cleanup_closure;
}

/// Whether a derived value was computed before (its value block holds one).
FA_EXPORT int32_t fa_observer_computed(FaObserver *node) {
	return node->computed;
}

/// Tells a derived value whether its new value differs from the previous one.
FA_EXPORT void fa_observer_changed(FaObserver *node, int32_t changed) {
	node->changed = changed;
}

/// Frees every source and observer, at the end of the program: the
/// generated `release` of an observer drops its environment and its value.
FA_EXPORT void fa_reactive_release(void) {
	for (int64_t index = reactive_made_count - 1; index >= 0; index -= 1) {
		FaSource *source = reactive_made[index];
		free_items(source->observers, source->observer_capacity, sizeof(FaObserver *));
		free_items(source->children, source->child_capacity, sizeof(FaChild));
		if (source->kind == 0) {
			fa_free(source, sizeof(FaSource), sizeof(int64_t));
			continue;
		}
		FaObserver *node = (FaObserver *)source;
		node->release(node);
		free_items(node->reads, node->read_capacity, sizeof(FaSource *));
		free_items(node->versions, node->read_capacity, sizeof(int64_t));
		if (node->value_size > 0) {
			fa_free(node->value, (size_t)node->value_size, 16);
		}
		fa_free(node, sizeof(FaObserver), sizeof(int64_t));
	}
	free_items(reactive_made, reactive_made_capacity, sizeof(void *));
	free_items(reactive_pending, reactive_pending_capacity, sizeof(FaObserver *));
	free_items(field_sources, field_source_capacity, sizeof(FaChild));
	reactive_made_count = 0;
}

// ── stops (values brief 2.7)────────────────────────────────────────────

/// Prints `stop: <reason>` and `  at <location>` and exits with 101.
/// `location` is a NUL-terminated `file:line:column`.
static FA_NORETURN void stop_with_builder(const char *location) {
	fa_flush();
	FaString reason;
	reason.bytes = builder;
	reason.length = builder_length;
	fa_write(2, (const uint8_t *)"stop: ", 6);
	fa_write(2, reason.bytes, reason.length);
	fa_write(2, (const uint8_t *)"\n  at ", 6);
	fa_write(2, (const uint8_t *)location, c_length(location));
	fa_write(2, (const uint8_t *)"\n", 1);
	fa_exit(101);
}

/// A site of a check: the NUL-terminated name of the type, then the
/// NUL-terminated location.
static const char *site_location(const char *site) {
	return site + c_length(site) + 1;
}

/// `integer overflow: <value> doesn't fit in <type>`, with the exact value
/// computed by the caller (signed when `is_signed`).
FA_EXPORT FA_COLD FA_NORETURN void fa_overflow(uint64_t low, uint64_t high, int32_t is_signed,
                                               const char *site) {
	fa_b_reset();
	append_c("integer overflow: ");
	if (is_signed) {
		fa_b_i128(low, high);
	} else {
		fa_b_u128(low, high);
	}
	append_c(" doesn't fit in ");
	append_c(site);
	stop_with_builder(site_location(site));
}

/// An overflow whose exact value isn't computable in 128 bits.
FA_EXPORT FA_COLD FA_NORETURN void fa_overflow_unknown(const char *site) {
	fa_b_reset();
	append_c("integer overflow: the value doesn't fit in ");
	append_c(site);
	stop_with_builder(site_location(site));
}

/// A float that doesn't convert to an integer type (`Integer(x)`): NaN,
/// an infinity or a value out of range.
FA_EXPORT FA_COLD FA_NORETURN void fa_float_overflow(double value, const char *site) {
	fa_b_reset();
	append_c("integer overflow: ");
	fa_b_f64(value);
	append_c(" doesn't fit in ");
	append_c(site);
	stop_with_builder(site_location(site));
}

/// A false `assert`: `site` is the source of the condition, then the location.
FA_EXPORT FA_COLD FA_NORETURN void fa_assert_failed(const char *site) {
	fa_b_reset();
	append_c("assertion failed: ");
	append_c(site);
	stop_with_builder(site_location(site));
}

#if !defined(__wasm__)
// A stack overflow is a stop (values brief D-26): on native, the guard page
// below the stack raises SIGSEGV, handled on an alternate stack. Any other
// segmentation fault is a crash, a bug of the compiler (or of foreign code):
// reported with its address, then raised again with the default action, so
// that the process ends by the signal. Linux layouts of `stack_t`, of
// glibc's `struct sigaction` and of the start of `siginfo_t`.
typedef struct {
	void *pointer;
	int32_t flags;
	size_t size;
} AlternateStack;

typedef struct {
	void (*handler)(int, void *, void *);
	uint64_t mask[16];
	int32_t flags;
	void (*restorer)(void);
} SignalAction;

typedef struct {
	int32_t number;
	int32_t error;
	int32_t code;
	int32_t padding;
	uintptr_t address;
} SignalInformation;

int sigaltstack(const AlternateStack *stack, AlternateStack *old);
int sigaction(int signal, const SignalAction *action, SignalAction *old);
FA_NORETURN void _exit(int status);
int raise(int signal);
// the size limit of the main thread's stack (one system call; glibc's
// `pthread_getattr_np` reads `/proc/self/maps`, 90 000 instructions)
typedef struct {
	uint64_t current;
	uint64_t maximum;
} ResourceLimit;
__attribute__((weak)) int getrlimit(int resource, ResourceLimit *limit);

static uint8_t signal_stack[16384];
/// The lowest address of the main thread's stack (0: unknown), estimated
/// from the stack pointer at the start of the program: the arguments and
/// the environment above it make it up to [`STACK_ABOVE`] lower.
static uintptr_t stack_low;
/// How far below the stack a fault counts as a stack overflow: the guard
/// gap the kernel keeps below a growing stack (256 pages by default).
#define STACK_GUARD_GAP ((uintptr_t)1 << 20)
/// How far above the estimated lowest address the real one may be.
#define STACK_ABOVE ((uintptr_t)8 << 20)

static void on_fault(int signal, void *information, void *context) {
	(void)signal;
	(void)context;
	const SignalInformation *info = (const SignalInformation *)information;
	fa_flush();
	// raised by the kernel (`code > 0`) at an address of the guard below
	// the stack
	uintptr_t address = info->address;
	if (info->code > 0 && stack_low != 0 && address < stack_low + STACK_ABOVE &&
	    address + STACK_GUARD_GAP >= stack_low) {
		static const char message[] = "stop: stack overflow\n";
		fa_write(2, (const uint8_t *)message, sizeof message - 1);
		_exit(101);
	}
	static const char prefix[] = "crash: segmentation fault";
	fa_write(2, (const uint8_t *)prefix, sizeof prefix - 1);
	if (info->code > 0) {
		char text[32] = " at 0x";
		size_t length = 6;
		int started = 0;
		for (int shift = 60; shift >= 0; shift -= 4) {
			unsigned digit = (unsigned)(address >> shift) & 15;
			if (digit != 0 || started || shift == 0) {
				text[length++] = (char)(digit < 10 ? '0' + digit : 'a' + digit - 10);
				started = 1;
			}
		}
		fa_write(2, (const uint8_t *)text, length);
	}
	static const char suffix[] = " (a bug of the compiler or of foreign code)\n";
	fa_write(2, (const uint8_t *)suffix, sizeof suffix - 1);
	// the default action, when the faulting instruction runs again (or now,
	// for a signal sent by a process)
	SignalAction action = {0, {0}, 0, 0};
	sigaction(11 /* SIGSEGV */, &action, 0);
	if (info->code <= 0) {
		raise(11);
	}
}

/// Reports a stack overflow as a stop (native with the C library only).
FA_EXPORT void fa_guard_stack(void) {
	ResourceLimit limit = {0, 0};
	// (the address of a local: a frame address would make every function
	// this is inlined into keep a frame pointer)
	uintptr_t here = (uintptr_t)&limit;
	// `RLIMIT_STACK`; an unlimited stack (all ones) has no guard to tell
	if (getrlimit != 0 && getrlimit(3, &limit) == 0 && limit.current < here) {
		stack_low = here - (uintptr_t)limit.current;
	}
	AlternateStack stack = {signal_stack, 0, sizeof signal_stack};
	sigaltstack(&stack, 0);
	SignalAction action = {on_fault, {0}, 0x08000000 /* SA_ONSTACK */ | 4 /* SA_SIGINFO */, 0};
	sigaction(11 /* SIGSEGV */, &action, 0);
}
#endif

/// A false `assert` of a comparison: the built text holds its operands.
FA_EXPORT FA_COLD FA_NORETURN void fa_assert_failed_operands(const char *site) {
	FaString operands;
	fa_b_take(&operands);
	append_c("assertion failed: ");
	append_c(site);
	append_c("\n       \xE2\x86\xB3 ");
	fa_b_bytes(string_bytes(&operands), string_length(&operands));
	stop_with_builder(site_location(site));
}

// ── errors (values brief 15) ────────────────────────────────────────────

// An error, like every value of a named union, is a pointer to a box: the
// global id of its member (an `int64_t`), then its data. The built-in errors
// have fixed ids, and their data are the fields of their payload, sorted by
// name (the layout of an object): the generated code builds and reads the
// same boxes. `fa_format_union`, generated into every program, appends the
// `toString` of any box.

enum {
	INDEX_ERROR = 1,
	KEY_NOT_FOUND = 2,
	DIVISION_BY_ZERO = 3,
	ARITHMETIC_ERROR = 4,
	PARSE_ERROR = 5,
};

typedef struct {
	int64_t id;
} BareError;

typedef struct {
	int64_t id;
	FaString message;
} MessageError;

typedef struct {
	int64_t id;
	FaString input;
	FaString message;
} ParseError;

typedef struct {
	int64_t id;
	int64_t index;
	int64_t length;
} IndexError;

void fa_format_union(const void *box, int32_t nested);

/// An error that escaped the program: `error: <toString>`, exit code 1.
FA_EXPORT FA_COLD FA_NORETURN void fa_uncaught(const void *error) {
	fa_b_reset();
	fa_format_union(error, 1);
	fa_flush();
	fa_write(2, (const uint8_t *)"error: ", 7);
	fa_write(2, builder, builder_length);
	fa_write(2, (const uint8_t *)"\n", 1);
	fa_exit(1);
}

static const BareError division_by_zero = {DIVISION_BY_ZERO};

/// `DivisionByZero`.
FA_EXPORT FA_COLD const void *fa_error_division_by_zero(void) {
	return &division_by_zero;
}

/// An `ArithmeticError` with the built text as its message.
static const void *arithmetic_error_from_builder(void) {
	MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
	error->id = ARITHMETIC_ERROR;
	fa_b_take(&error->message);
	return error;
}

/// `ArithmeticError { message = "<value> doesn't fit in <type>" }`.
FA_EXPORT FA_COLD const void *fa_error_conversion(uint64_t low, uint64_t high, int32_t is_signed,
                                                  const char *site) {
	fa_b_reset();
	if (is_signed) {
		fa_b_i128(low, high);
	} else {
		fa_b_u128(low, high);
	}
	append_c(" doesn't fit in ");
	append_c(site);
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "negative exponent: <exponent>" }`.
FA_EXPORT FA_COLD const void *fa_error_negative_exponent(int64_t exponent) {
	fa_b_reset();
	append_c("negative exponent: ");
	fa_b_i64(exponent);
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "negative exponent: <exponent>" }` of a `BigInt`.
FA_EXPORT FA_COLD const void *fa_error_big_negative_exponent(int64_t exponent) {
	fa_b_reset();
	append_c("negative exponent: ");
	fa_b_bigint(exponent);
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "<value> doesn't fit in <type>" }` of a `BigInt`.
FA_EXPORT FA_COLD const void *fa_error_big_conversion(int64_t value, const char *site) {
	fa_b_reset();
	fa_b_bigint(value);
	append_c(" doesn't fit in ");
	append_c(site);
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "<value> is not a finite number" }` and
/// `"<value> doesn't fit in Integer"` (`Integer.round` and friends).
FA_EXPORT FA_COLD const void *fa_error_float(double value, int32_t finite) {
	fa_b_reset();
	fa_b_f64(value);
	append_c(finite ? " doesn't fit in Integer" : " is not a finite number");
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "shift amount <n> is outside 0..<bits>" }`.
FA_EXPORT FA_COLD const void *fa_error_shift(int64_t amount, int64_t bits) {
	fa_b_reset();
	append_c("shift amount ");
	fa_b_i64(amount);
	append_c(" is outside 0..");
	fa_b_i64(bits);
	return arithmetic_error_from_builder();
}

/// `ArithmeticError { message = "integer overflow: <exact> doesn't fit in <type>" }`
/// (`checkedAdd` and friends).
FA_EXPORT FA_COLD const void *fa_error_checked(uint64_t low, uint64_t high, const char *site) {
	fa_b_reset();
	append_c("integer overflow: ");
	fa_b_i128(low, high);
	append_c(" doesn't fit in ");
	append_c(site);
	return arithmetic_error_from_builder();
}

/// `IndexError { index, length }`.
FA_EXPORT FA_COLD const void *fa_error_index(int64_t index, int64_t length) {
	IndexError *error = fa_alloc(sizeof(IndexError), sizeof(int64_t));
	error->id = INDEX_ERROR;
	error->index = index;
	error->length = length;
	return error;
}

FA_EXPORT FA_COLD const void *fa_error_utf8(int64_t position) {
	struct { uint64_t id; int64_t position; } *error = fa_alloc(16, sizeof(int64_t));
	error->id = 12; // Utf8Error, builtins/errors.rs
	error->position = position;
	return error;
}

/// `ParseError { input, message }` with the built text as its message.
static const void *parse_error(const uint8_t *bytes, size_t length) {
	ParseError *error = fa_alloc(sizeof(ParseError), sizeof(int64_t));
	error->id = PARSE_ERROR;
	error->input = owned_copy(bytes, length);
	fa_b_take(&error->message);
	return error;
}

/// `Integer.parse(text)`: an optional sign then ASCII digits, within the
/// range of `Integer`; the error (`ParseError`) or null.
FA_EXPORT const void *fa_parse_integer(const uint8_t *bytes, size_t length, int64_t *out) {
	size_t index = 0;
	int negative = 0;
	if (length > 0 && (bytes[0] == '+' || bytes[0] == '-')) {
		negative = bytes[0] == '-';
		index = 1;
	}
	int valid = index < length;
	for (size_t position = index; position < length; position += 1) {
		valid &= bytes[position] >= '0' && bytes[position] <= '9';
	}
	if (!valid) {
		fa_b_reset();
		if (length == 0) {
			append_c("the text is empty");
		} else {
			// `JSON.stringify(text)`, then the message
			append_byte('"');
			for (size_t position = 0; position < length; position += 1) {
				uint8_t byte = bytes[position];
				if (byte == '"' || byte == '\\') {
					append_byte('\\');
				}
				append_byte(byte);
			}
			append_c("\" is not an integer");
		}
		return parse_error(bytes, length);
	}
	uint64_t magnitude = 0;
	int overflow = 0;
	for (; index < length; index += 1) {
		uint64_t digit = (uint64_t)(bytes[index] - '0');
		if (magnitude > (UINT64_MAX_VALUE - digit) / 10) {
			overflow = 1;
			break;
		}
		magnitude = magnitude * 10 + digit;
	}
	uint64_t limit = negative ? (uint64_t)1 << 63 : ((uint64_t)1 << 63) - 1;
	if (overflow || magnitude > limit) {
		fa_b_reset();
		fa_b_bytes(bytes, length);
		append_c(" doesn't fit in Integer");
		return parse_error(bytes, length);
	}
	*out = negative ? (int64_t)(0 - magnitude) : (int64_t)magnitude;
	return 0;
}

/// `base ** exponent` on `Int64` (the exponent is not negative): square and
/// multiply, each step checked when `checked` (`OVERFLOW_NATIVE`).
FA_EXPORT int64_t fa_pow_i64(int64_t base, int64_t exponent, int32_t checked, const char *site) {
	int64_t result = 1;
	for (;;) {
		if (exponent & 1) {
			if (__builtin_mul_overflow(result, base, &result) && checked) {
				fa_overflow_unknown(site);
			}
		}
		exponent >>= 1;
		if (exponent == 0) {
			return result;
		}
		// a later step multiplies by this square (the exponent isn't 0), and
		// the result is never 0 unless the base is: the overflow is real
		if (__builtin_mul_overflow(base, base, &base) && checked) {
			fa_overflow_unknown(site);
		}
	}
}

#if defined(__wasm__)
FA_IMPORT("pow") double fa_host_pow(double base, double exponent);

/// `llvm.pow`, which wasm lowers to a call of `pow`: JavaScript's `Math.pow`,
/// as on the JavaScript backend.
__attribute__((weak)) double pow(double base, double exponent) {
	return fa_host_pow(base, exponent);
}

/// `llvm.round` (halves away from zero), which wasm lowers to a call of
/// `round`: written with `f64.trunc`.
__attribute__((weak)) double round(double value) {
	double truncated = __builtin_trunc(value);
	double rest = value - truncated;
	if (rest >= 0.5) {
		return truncated + 1.0;
	}
	if (rest <= -0.5) {
		return truncated - 1.0;
	}
	return truncated;
}
#endif

// ── maps and sets ───────────────────────────────────────────────────────

// A `Map` or a `Set` is a pointer to a `FaTable` (`src/lower/tables.rs`):
// its entries, each `{ key, value }` (a set has no value), and an
// open-addressing index from a hash to an entry, probed linearly and at
// most half full.
//
// - **Index.** A slot holds the low 32 bits of the key's hash, which give its
//   home slot and filter the candidates before an entry is read, and the
//   entry's position + 1 (0: an empty slot), in one 64-bit word. Beyond 2^31
//   slots, a slot holds the full hash and the position (16 bytes). A deletion
//   shifts the following slots of its run back (no tombstones): a lookup stops
//   at the first empty slot, and an unsuccessful lookup gives the slot where
//   the key goes. The entries store no hash: the index grows from itself, and
//   from the keys (`hash_at`) only when it becomes wide.
// - **Entries.** They never move (references into a map's values stay valid):
//   they live in chunks of 32 entries, found through a directory holding each
//   chunk's pointer and the bitmap of its live entries. The chunks with a free
//   entry form a list (through the directory); an insertion takes the lowest
//   free entry of the first one, so that a table without deletions keeps its
//   insertion order.
//
// - **Dense entries.** When the program never keeps a reference into the
//   values of maps of a type across an insertion (`src/lower/tables.rs`,
//   `dense`), their entries live in one contiguous array that moves when it
//   grows: an entry is one load from the table instead of two (the chunk
//   directory then only holds the live bits and the free list). The generated
//   code says so with `TABLE_DENSE` in `entry_size`, a constant at each call.
//
// The generated code hashes the keys and compares them (`equals`, or bitwise
// over `key_size` bytes when null: the integers), and moves keys and values in
// and out of the entries.

/// A chunk of 32 entries, in the directory.
typedef struct {
	uint8_t *entries;
	// bit i: entry i is live
	uint32_t live;
	// the next chunk with a free entry + 1 (0: none), while this one has one
	uint32_t next;
} TableChunk;

typedef struct {
	TableChunk *chunks;
	int64_t length;
	// the positions below which entries may be live (iteration stops there)
	int64_t used;
	// the entries allocated (32 per chunk)
	int64_t capacity;
	void *index;
	int64_t mask;
	// the capacity of the directory, in chunks
	int64_t directory;
	// the first chunk with a free entry + 1 (0: none)
	int64_t free;
	// dense entries: the contiguous array, and the entries it has room for
	uint8_t *entries;
	int64_t room;
	// 0: the heap, 2: read-only data, 3: a block of a function's frame. The
	// last two are immutable, and none of their blocks is freed.
	int64_t storage;
} FaTable;

/// In `entry_size`: the entries are dense (see above).
#define TABLE_DENSE ((int64_t)1 << 40)

static __attribute__((always_inline)) inline int64_t table_entry_bytes(int64_t entry_size) {
	return entry_size & (TABLE_DENSE - 1);
}

#define TABLE_CHUNK_BITS 5
#define TABLE_CHUNK_SLOTS (1 << TABLE_CHUNK_BITS)
#define TABLE_CHUNK_FULL 0xffffffffu

/// The bytes of a chunk (entries without bytes still get an allocation).
static size_t table_chunk_bytes(int64_t entry_size) {
	int64_t bytes = table_entry_bytes(entry_size);
	return bytes > 0 ? (size_t)TABLE_CHUNK_SLOTS * (size_t)bytes : 16;
}

/// A wide index slot (more than 2^31 slots).
typedef struct {
	uint64_t hash;
	uint64_t position;
} WideSlot;

/// Up to 2^31 slots, the low 32 bits of a hash give the home slot, and at
/// most half of the slots name entries: the position + 1 fits in 32 bits.
static __attribute__((always_inline)) inline int table_narrow(int64_t mask) {
	return (uint64_t)mask <= 0x7fffffffull;
}

static __attribute__((always_inline)) inline size_t table_slot_size(int64_t mask) {
	return table_narrow(mask) ? 8 : 16;
}

/// The entry at a position.
static __attribute__((always_inline)) inline uint8_t *entry_at(const FaTable *table,
                                                               int64_t position,
                                                               int64_t entry_size) {
	if (entry_size & TABLE_DENSE) {
		return table->entries + position * table_entry_bytes(entry_size);
	}
	return table->chunks[position >> TABLE_CHUNK_BITS].entries
	       + (position & (TABLE_CHUNK_SLOTS - 1)) * entry_size;
}

static __attribute__((always_inline)) inline int table_keys_equal(
	const uint8_t *stored, const void *key, int32_t (*equals)(const void *, const void *),
	int64_t key_size);

/// In `entry_size`: the entries live in the probe array itself (see below).
#define TABLE_INLINE ((int64_t)1 << 41)
/// Inline entries with a 32-bit integer key. Zero marks an empty slot;
/// the actual zero key lives in one extra entry after the probe array.
#define TABLE_INLINE_KEY32 ((int64_t)1 << 42)

// - **Entries in the probe array** (`TABLE_INLINE`, `src/lower/tables.rs`):
//   for a dense map of small entries (integer keys, fields of at most 4
//   bytes, at most 12 bytes), a slot is the low 32 bits of the hash (0: an
//   empty slot; a hash whose low bits are 0 is stored as 2^31, which keeps
//   its home slot) followed by the entry. A position is a slot: a lookup
//   reads one array. For a 32-bit key, the slot holds only key and value
//   (8 bytes): key zero marks an empty slot, and the actual zero key lives
//   in one extra slot whose presence is `capacity` (otherwise unused for
//   inline entries). Rebuilds and shifts hash the keys they move, instead
//   of storing a hash beside every key. Deleting is a backward shift when the generated
//   code releases the entry (after it read the entry), which moves the
//   entries of the run: positions change only then, and at growth.

static __attribute__((always_inline)) inline int64_t inline_stride(int64_t entry_size) {
	return (entry_size & TABLE_INLINE_KEY32 ? 0 : 4) + table_entry_bytes(entry_size);
}

static __attribute__((always_inline)) inline int64_t inline_offset(int64_t entry_size) {
	return entry_size & TABLE_INLINE_KEY32 ? 0 : 4;
}

static __attribute__((always_inline)) inline int64_t inline_slots(int64_t mask, int64_t entry_size) {
	return mask + 1 + ((entry_size & TABLE_INLINE_KEY32) != 0);
}

static __attribute__((always_inline)) inline uint8_t *inline_slot(const FaTable *table,
                                                                  int64_t slot,
                                                                  int64_t entry_size) {
	return (uint8_t *)table->index + slot * inline_stride(entry_size);
}

static __attribute__((always_inline)) inline uint32_t inline_tag_of(uint64_t hash) {
	uint32_t tag = (uint32_t)hash;
	return tag == 0 ? 0x80000000u : tag;
}

static __attribute__((always_inline)) inline uint32_t inline_tag(const uint8_t *slot) {
	uint32_t tag;
	__builtin_memcpy(&tag, slot, 4);
	return tag;
}

static __attribute__((always_inline)) inline void inline_set_tag(uint8_t *slot, uint32_t tag) {
	__builtin_memcpy(slot, &tag, 4);
}

/// The same hash as the generated hash of a 32-bit integer key
/// (`lower/tables.rs`): its unsigned bits, multiplied, then rotated right
/// by 24. Both steps are bijective, so hash zero identifies only key zero.
static __attribute__((always_inline)) inline uint64_t inline_key_hash(uint32_t key) {
	uint64_t product = (uint64_t)key * 0x9E3779B97F4A7C15ull;
#if defined(__clang__)
	return __builtin_rotateright64(product, 24);
#else
	return (product >> 24) | (product << 40);
#endif
}

static __attribute__((always_inline)) inline uint64_t inline_home(uint32_t key_or_tag,
                                                                uint64_t mask,
                                                                int64_t entry_size) {
	uint64_t hash = entry_size & TABLE_INLINE_KEY32 ? inline_key_hash(key_or_tag) : key_or_tag;
	return hash & mask;
}

/// The slot of a key, or the complement of the empty slot where it goes.
static __attribute__((always_inline)) inline int64_t
inline_lookup(const FaTable *table, uint64_t hash, const void *key,
              int32_t (*equals)(const void *, const void *), int64_t entry_size,
              int64_t key_size) {
	uint64_t mask = (uint64_t)table->mask;
	uint64_t slot = hash & mask;
	if (entry_size & TABLE_INLINE_KEY32) {
		uint32_t wanted = inline_tag(key);
		if (wanted == 0) {
			int64_t extra = table->mask + 1;
			return table->capacity != 0 ? extra : ~extra;
		}
		for (;;) {
			uint32_t found = inline_tag(inline_slot(table, (int64_t)slot, entry_size));
			if (found == wanted) {
				return (int64_t)slot;
			}
			if (found == 0) {
				return ~(int64_t)slot;
			}
			slot = (slot + 1) & mask;
		}
	}
	uint32_t tag = inline_tag_of(hash);
	for (;;) {
		const uint8_t *at = inline_slot(table, (int64_t)slot, entry_size);
		uint32_t found = inline_tag(at);
		if (found == 0) {
			return ~(int64_t)slot;
		}
		if (found == tag && table_keys_equal(at + 4, key, equals, key_size)) {
			return (int64_t)slot;
		}
		slot = (slot + 1) & mask;
	}
}

/// Empties a slot: the following slots of its run shift back over it when
/// their home is not after it.
static __attribute__((always_inline)) inline void inline_shift(FaTable *table, uint64_t hole,
                                                               int64_t entry_size) {
	uint64_t mask = (uint64_t)table->mask;
	if ((entry_size & TABLE_INLINE_KEY32) && hole == mask + 1) {
		table->capacity = 0;
		table->length -= 1;
		return;
	}
	int64_t stride = inline_stride(entry_size);
	// read once: the slots moved below can't be the table's own fields
	uint8_t *slots = (uint8_t *)table->index;
	for (uint64_t next = (hole + 1) & mask;; next = (next + 1) & mask) {
		uint8_t *at = slots + (int64_t)next * stride;
		uint32_t found = inline_tag(at);
		if (found == 0) {
			break;
		}
		uint64_t home = inline_home(found, mask, entry_size);
		if (((next - home) & mask) >= ((next - hole) & mask)) {
			__builtin_memcpy(slots + (int64_t)hole * stride, at, (size_t)stride);
			hole = next;
		}
	}
	inline_set_tag(slots + (int64_t)hole * stride, 0);
	table->length -= 1;
}

/// A new array of `slots` empty slots (a power of two).
static void inline_new_slots(FaTable *table, int64_t slots, int64_t entry_size) {
	if (slots - 1 > 0x7fffffffll) {
		out_of_memory();
	}
	int64_t count = inline_slots(slots - 1, entry_size);
	int64_t stride = inline_stride(entry_size);
	if ((uint64_t)count > SIZE_MAX / (uint64_t)stride) {
		out_of_memory();
	}
	size_t bytes = (size_t)count * (size_t)stride;
	table->index = fa_alloc(bytes, 8);
	__builtin_memset(table->index, 0, bytes);
	table->mask = slots - 1;
	table->used = count;
}

static void inline_free_slots(void *slots, int64_t mask, int64_t entry_size) {
	fa_free(slots, (size_t)(inline_slots(mask, entry_size) * inline_stride(entry_size)), 8);
}

/// Moves the entries into `slots` slots.
static FA_COLD void inline_grow(FaTable *table, int64_t slots, int64_t entry_size) {
	uint8_t *old = (uint8_t *)table->index;
	int64_t old_mask = table->mask;
	int64_t stride = inline_stride(entry_size);
	inline_new_slots(table, slots, entry_size);
	uint64_t mask = (uint64_t)table->mask;
	for (int64_t slot = 0; slot <= old_mask; slot += 1) {
		const uint8_t *from = old + slot * stride;
		uint32_t tag = inline_tag(from);
		if (tag != 0) {
			uint64_t at = inline_home(tag, mask, entry_size);
			while (inline_tag(inline_slot(table, (int64_t)at, entry_size)) != 0) {
				at = (at + 1) & mask;
			}
			copy_bytes(inline_slot(table, (int64_t)at, entry_size), from, (size_t)stride);
		}
	}
	if (entry_size & TABLE_INLINE_KEY32) {
		copy_bytes(inline_slot(table, slots, entry_size), old + (old_mask + 1) * stride,
		           (size_t)stride);
	}
	inline_free_slots(old, old_mask, entry_size);
}

/// The entry at a position, for the generated code.
FA_EXPORT __attribute__((always_inline)) uint8_t *fa_table_entry(const FaTable *table,
                                                                 int64_t position,
                                                                 int64_t entry_size) {
	if (entry_size & TABLE_INLINE) {
		return inline_slot(table, position, entry_size) + inline_offset(entry_size);
	}
	return entry_at(table, position, entry_size);
}

/// Whether the entry at a position (below `used`) is live.
FA_EXPORT __attribute__((always_inline)) int32_t fa_table_live(const FaTable *table,
                                                               int64_t position,
                                                               int64_t entry_size) {
	if (entry_size & TABLE_INLINE) {
		if ((entry_size & TABLE_INLINE_KEY32) && position == table->mask + 1) {
			return table->capacity != 0;
		}
		return inline_tag(inline_slot(table, position, entry_size)) != 0;
	}
	return (table->chunks[position >> TABLE_CHUNK_BITS].live
	        >> (position & (TABLE_CHUNK_SLOTS - 1))) & 1;
}

/// `key_size` of a table of strings (`src/lower/tables.rs`).
#define STRING_KEY ((int64_t)-1)

/// `==` of two strings read in place (`strings_equal` of the backend): two
/// inline strings compare in three words (their padding is zero), without
/// their `SIMPLE` flag; other strings by their lengths and bytes.
static __attribute__((always_inline)) inline int string_keys_equal(const uint8_t *stored,
                                                                   const void *key) {
	FaString a;
	FaString b;
	__builtin_memcpy(&a, stored, sizeof(FaString));
	__builtin_memcpy(&b, key, sizeof(FaString));
	if (a.capacity & b.capacity & STRING_INLINE) {
		return ((uintptr_t)a.bytes == (uintptr_t)b.bytes) & (a.length == b.length) &
		       (((a.capacity ^ b.capacity) & ~SIMPLE) == 0);
	}
	const FaString *left = (const FaString *)stored;
	const FaString *right = (const FaString *)key;
	return fa_string_equal(string_bytes(left), string_length(left), string_bytes(right),
	                       string_length(right));
}

/// Whether a stored key equals a key.
static __attribute__((always_inline)) inline int table_keys_equal(
	const uint8_t *stored, const void *key, int32_t (*equals)(const void *, const void *),
	int64_t key_size) {
	if (equals != 0) {
		return equals(stored, key);
	}
	if (key_size == STRING_KEY) {
		return string_keys_equal(stored, key);
	}
	const uint8_t *other = (const uint8_t *)key;
	switch (key_size) {
	case 1:
		return stored[0] == other[0];
	case 2: {
		uint16_t a;
		uint16_t b;
		__builtin_memcpy(&a, stored, 2);
		__builtin_memcpy(&b, other, 2);
		return a == b;
	}
	case 4: {
		uint32_t a;
		uint32_t b;
		__builtin_memcpy(&a, stored, 4);
		__builtin_memcpy(&b, other, 4);
		return a == b;
	}
	case 8: {
		uint64_t a;
		uint64_t b;
		__builtin_memcpy(&a, stored, 8);
		__builtin_memcpy(&b, other, 8);
		return a == b;
	}
	default:
		for (int64_t byte = 0; byte < key_size; byte += 1) {
			if (stored[byte] != other[byte]) {
				return 0;
			}
		}
		return 1;
	}
}

/// The position of the entry of a key, or the complement of the empty slot
/// where it goes (negative); `*at` the index slot either way.
static __attribute__((always_inline)) inline int64_t
table_lookup(const FaTable *table, uint64_t hash, const void *key,
             int32_t (*equals)(const void *, const void *), int64_t entry_size,
             int64_t key_size, uint64_t *at) {
	uint64_t mask = (uint64_t)table->mask;
	uint64_t slot = hash & mask;
	if (__builtin_expect(table_narrow(table->mask), 1)) {
		const uint64_t *index = (const uint64_t *)table->index;
		uint32_t tag = (uint32_t)hash;
		for (;;) {
			uint64_t found = index[slot];
			if (found == 0) {
				*at = slot;
				return ~(int64_t)slot;
			}
			if ((uint32_t)found == tag) {
				int64_t position = (int64_t)(found >> 32) - 1;
				if (table_keys_equal(entry_at(table, position, entry_size), key, equals, key_size)) {
					*at = slot;
					return position;
				}
			}
			slot = (slot + 1) & mask;
		}
	}
	const WideSlot *index = (const WideSlot *)table->index;
	for (;;) {
		WideSlot found = index[slot];
		if (found.position == 0) {
			*at = slot;
			return ~(int64_t)slot;
		}
		if (found.hash == hash) {
			int64_t position = (int64_t)found.position - 1;
			if (table_keys_equal(entry_at(table, position, entry_size), key, equals, key_size)) {
				*at = slot;
				return position;
			}
		}
		slot = (slot + 1) & mask;
	}
}

/// The position of the entry of a key, or the complement of the empty slot
/// where it goes (negative).
FA_EXPORT __attribute__((always_inline)) int64_t
fa_table_find(const FaTable *table, uint64_t hash, const void *key,
              int32_t (*equals)(const void *, const void *), int64_t entry_size,
              int64_t key_size) {
	if (entry_size & TABLE_INLINE) {
		return inline_lookup(table, hash, key, equals, entry_size, key_size);
	}
	uint64_t slot;
	return table_lookup(table, hash, key, equals, entry_size, key_size, &slot);
}

/// Empties an index slot: the following slots of its run shift back over
/// it when their home is not after it (no tombstones).
static __attribute__((always_inline)) inline void table_shift(FaTable *table, uint64_t hole) {
	uint64_t mask = (uint64_t)table->mask;
	if (__builtin_expect(table_narrow(table->mask), 1)) {
		uint64_t *index = (uint64_t *)table->index;
		for (uint64_t next = (hole + 1) & mask;; next = (next + 1) & mask) {
			uint64_t found = index[next];
			if (found == 0) {
				break;
			}
			uint64_t home = found & mask;
			if (((next - home) & mask) >= ((next - hole) & mask)) {
				index[hole] = found;
				hole = next;
			}
		}
		index[hole] = 0;
		return;
	}
	WideSlot *index = (WideSlot *)table->index;
	for (uint64_t next = (hole + 1) & mask;; next = (next + 1) & mask) {
		WideSlot found = index[next];
		if (found.position == 0) {
			break;
		}
		uint64_t home = found.hash & mask;
		if (((next - home) & mask) >= ((next - hole) & mask)) {
			index[hole] = found;
			hole = next;
		}
	}
	index[hole].hash = 0;
	index[hole].position = 0;
}

/// Removes a key from the index, in the walk that finds it: its position, or
/// a negative number when it is absent. The entry stays live until the
/// generated code dropped (or moved out) its key and value and released it.
FA_EXPORT __attribute__((always_inline)) int64_t
fa_table_remove(FaTable *table, uint64_t hash, const void *key,
                int32_t (*equals)(const void *, const void *), int64_t entry_size,
                int64_t key_size) {
	if (entry_size & TABLE_INLINE) {
		// the entry is read, then released (`fa_table_release` shifts)
		int64_t position = inline_lookup(table, hash, key, equals, entry_size, key_size);
		return position >= 0 ? position : -1;
	}
	uint64_t slot;
	int64_t position = table_lookup(table, hash, key, equals, entry_size, key_size, &slot);
	if (position >= 0) {
		table_shift(table, slot);
	}
	return position;
}

/// Records an entry in the first empty slot from the home of its hash
/// (`slot`, when known: an empty slot of that run).
static __attribute__((always_inline)) inline void table_place(FaTable *table, uint64_t hash,
                                                              int64_t position, int64_t slot) {
	uint64_t mask = (uint64_t)table->mask;
	if (table_narrow(table->mask)) {
		uint64_t *index = (uint64_t *)table->index;
		uint64_t at = slot >= 0 ? (uint64_t)slot : hash & mask;
		while (index[at] != 0) {
			at = (at + 1) & mask;
		}
		index[at] = (uint32_t)hash | ((uint64_t)(position + 1) << 32);
		return;
	}
	WideSlot *index = (WideSlot *)table->index;
	uint64_t at = slot >= 0 ? (uint64_t)slot : hash & mask;
	while (index[at].position != 0) {
		at = (at + 1) & mask;
	}
	index[at].hash = hash;
	index[at].position = (uint64_t)(position + 1);
}

/// A new empty index of `slots` slots (a power of two).
static void table_new_index(FaTable *table, int64_t slots) {
	size_t width = table_slot_size(slots - 1);
	if ((uint64_t)slots > SIZE_MAX / width) {
		out_of_memory();
	}
	size_t bytes = (size_t)slots * width;
	table->index = fa_alloc(bytes, 8);
	__builtin_memset(table->index, 0, bytes);
	table->mask = slots - 1;
}

static void table_free_index(void *index, int64_t mask) {
	fa_free(index, (size_t)(mask + 1) * table_slot_size(mask), 8);
}

/// Rebuilds the index with more slots (a power of two): from the old index
/// while the hashes it holds are enough, else from the keys of the live entries.
static FA_COLD void table_grow_index(FaTable *table, int64_t slots,
                                     uint64_t (*hash_at)(const void *), int64_t entry_size) {
	void *old = table->index;
	int64_t old_mask = table->mask;
	int narrow = table_narrow(old_mask);
	table_new_index(table, slots);
	if (narrow && !table_narrow(slots - 1)) {
		for (int64_t position = 0; position < table->used; position += 1) {
			if (fa_table_live(table, position, entry_size)) {
				table_place(table, hash_at(entry_at(table, position, entry_size)), position, -1);
			}
		}
	} else if (narrow) {
		const uint64_t *index = (const uint64_t *)old;
		for (int64_t slot = 0; slot <= old_mask; slot += 1) {
			uint64_t found = index[slot];
			if (found != 0) {
				table_place(table, (uint32_t)found, (int64_t)(found >> 32) - 1, -1);
			}
		}
	} else {
		const WideSlot *index = (const WideSlot *)old;
		for (int64_t slot = 0; slot <= old_mask; slot += 1) {
			if (index[slot].position != 0) {
				table_place(table, index[slot].hash, (int64_t)index[slot].position - 1, -1);
			}
		}
	}
	table_free_index(old, old_mask);
}

/// The number of index slots for `capacity` entries.
static int64_t index_slots(int64_t capacity) {
	// Both the doubled request and the next power of two must fit i64.
	if (capacity < 0 || capacity > INT64_MAX / 4) {
		out_of_memory();
	}
	int64_t slots = 8;
	while (slots < capacity * 2) {
		slots *= 2;
	}
	return slots;
}

/// A new empty table, its index sized for `capacity` entries (the chunks
/// come with the first entries).
FA_EXPORT FaTable *fa_table_new(int64_t capacity, int64_t entry_size) {
	FaTable *table = fa_alloc(sizeof(FaTable), sizeof(int64_t));
	table->chunks = 0;
	table->length = 0;
	table->used = 0;
	table->capacity = 0;
	table->directory = 0;
	table->free = 0;
	table->entries = 0;
	table->room = 0;
	table->storage = 0;
	if (entry_size & TABLE_INLINE) {
		inline_new_slots(table, index_slots(capacity), entry_size);
		return table;
	}
	table_new_index(table, index_slots(capacity));
	return table;
}

/// An empty table with room for `capacity` entries in a block of the frame
/// of its function (an immutable literal, `table_local_bytes` of
/// `src/lower/tables.rs` gives the size of the block): its header, then its
/// probe array for inline entries, else its directory, its index and its
/// chunks of entries (one array for dense entries). The entries are then
/// added like those of any table, without growing anything.
FA_EXPORT FaTable *fa_table_local(void *buffer, int64_t capacity, int64_t entry_size) {
	FaTable *table = buffer;
	__builtin_memset(table, 0, sizeof(FaTable));
	table->storage = 3;
	uint8_t *next = (uint8_t *)buffer + sizeof(FaTable);
	int64_t slots = index_slots(capacity);
	table->index = next;
	table->mask = slots - 1;
	if (entry_size & TABLE_INLINE) {
		int64_t count = inline_slots(slots - 1, entry_size);
		__builtin_memset(next, 0, (size_t)(count * inline_stride(entry_size)));
		table->used = count;
		return table;
	}
	__builtin_memset(next, 0, (size_t)slots * sizeof(uint64_t));
	next += (size_t)slots * sizeof(uint64_t);
	int64_t chunks = (capacity + TABLE_CHUNK_SLOTS - 1) >> TABLE_CHUNK_BITS;
	table->chunks = (TableChunk *)next;
	table->directory = chunks;
	next += (size_t)chunks * sizeof(TableChunk);
	next = (uint8_t *)(((uintptr_t)next + 15) & ~(uintptr_t)15);
	if (entry_size & TABLE_DENSE) {
		table->entries = next;
		table->room = chunks * TABLE_CHUNK_SLOTS;
	}
	for (int64_t chunk = 0; chunk < chunks; chunk += 1) {
		table->chunks[chunk].entries =
			entry_size & TABLE_DENSE ? 0 : next + (size_t)chunk * table_chunk_bytes(entry_size);
		table->chunks[chunk].live = 0;
		table->chunks[chunk].next = chunk + 1 < chunks ? (uint32_t)(chunk + 2) : 0;
	}
	table->free = chunks > 0 ? 1 : 0;
	table->capacity = chunks * TABLE_CHUNK_SLOTS;
	return table;
}

/// `map.capacity`, `set.capacity`: the entries it holds without a rehash
/// (the index is at most half full; the entries never move).
FA_EXPORT int64_t fa_table_capacity(const FaTable *table) {
	return (table->mask + 1) / 2;
}

/// Dense entries: room for `entries` of them (a multiple of a chunk) in the
/// contiguous array, which moves (doubling) when it grows.
static FA_COLD void table_dense_room(FaTable *table, int64_t entries, int64_t bytes) {
	if (entries <= table->room) {
		return;
	}
	if (table->room > INT64_MAX / 2) {
		out_of_memory();
	}
	int64_t room = table->room == 0 ? TABLE_CHUNK_SLOTS : table->room * 2;
	if (room < entries) {
		room = entries;
	}
	uint64_t width = bytes > 0 ? (uint64_t)bytes : 1;
	if ((uint64_t)room > SIZE_MAX / width) {
		out_of_memory();
	}
	uint8_t *grown = fa_alloc((size_t)room * width, 16);
	if (table->entries != 0) {
		copy_bytes(grown, table->entries, (size_t)table->capacity * (size_t)bytes);
		fa_free(table->entries, (size_t)table->room * width, 16);
	}
	table->entries = grown;
	table->room = room;
}

/// Allocates one more chunk of entries, all free: the first of the list.
static FA_COLD void table_new_chunk(FaTable *table, int64_t entry_size) {
	int64_t chunk = table->capacity >> TABLE_CHUNK_BITS;
	if (chunk == table->directory) {
		int64_t directory = table->directory == 0 ? 4 : table->directory * 2;
		TableChunk *chunks = fa_alloc((size_t)directory * sizeof(TableChunk), sizeof(void *));
		if (table->chunks != 0) {
			copy_bytes((uint8_t *)chunks, (const uint8_t *)table->chunks,
			           (size_t)chunk * sizeof(TableChunk));
			fa_free(table->chunks, (size_t)table->directory * sizeof(TableChunk), sizeof(void *));
		}
		table->chunks = chunks;
		table->directory = directory;
	}
	if (entry_size & TABLE_DENSE) {
		table_dense_room(table, table->capacity + TABLE_CHUNK_SLOTS, table_entry_bytes(entry_size));
		table->chunks[chunk].entries = 0;
	} else {
		table->chunks[chunk].entries = fa_alloc(table_chunk_bytes(entry_size), 16);
	}
	table->chunks[chunk].live = 0;
	table->chunks[chunk].next = (uint32_t)table->free;
	table->free = chunk + 1;
	table->capacity += TABLE_CHUNK_SLOTS;
}

/// `map.reserve(additional)`, `set.reserve(additional)`: room for
/// `additional` more entries without a rehash nor a new block. The index
/// grows, and the chunks of the entries to come are allocated: the entries
/// that are there never move.
FA_EXPORT void fa_table_reserve(FaTable *table, int64_t additional, int64_t entry_size,
                                uint64_t (*hash_at)(const void *)) {
	if (additional <= 0) {
		return;
	}
	if (additional > INT64_MAX - table->length) {
		out_of_memory();
	}
	int64_t wanted = table->length + additional;
	if (wanted < table->length ||
	    wanted > (int64_t)(SIZE_MAX / 4) / (table_entry_bytes(entry_size) + 16)) {
		out_of_memory();
	}
	int64_t slots = index_slots(wanted);
	if (entry_size & TABLE_INLINE) {
		if (slots > table->mask + 1) {
			inline_grow(table, slots, entry_size);
		}
		return;
	}
	if (slots > table->mask + 1) {
		table_grow_index(table, slots, hash_at, entry_size);
	}
	int64_t capacity = (wanted + TABLE_CHUNK_SLOTS - 1) & ~(int64_t)(TABLE_CHUNK_SLOTS - 1);
	if (capacity <= table->capacity) {
		return;
	}
	if (entry_size & TABLE_DENSE) {
		table_dense_room(table, capacity, table_entry_bytes(entry_size));
	}
	while (table->capacity < capacity) {
		table_new_chunk(table, entry_size);
	}
	// the chunks with a free entry, lowest first: a table without deletions
	// keeps its insertion order
	table->free = 0;
	for (int64_t chunk = (table->capacity >> TABLE_CHUNK_BITS) - 1; chunk >= 0; chunk -= 1) {
		if (table->chunks[chunk].live != TABLE_CHUNK_FULL) {
			table->chunks[chunk].next = (uint32_t)table->free;
			table->free = chunk + 1;
		}
	}
}

/// A new live entry for a key that is absent (`probe`: the result of its
/// lookup, or 0 when unknown): its key and value are the generated code's to
/// write. `hash_at` hashes a key in an entry, for the index becoming wide.
FA_EXPORT __attribute__((always_inline)) void *
fa_table_append(FaTable *table, uint64_t hash, int64_t entry_size, int64_t probe,
                uint64_t (*hash_at)(const void *)) {
	if (entry_size & TABLE_INLINE) {
		int64_t at = probe < 0 ? ~probe : -1;
		if (__builtin_expect((table->length + 1) * 2 > table->mask + 1, 0)) {
			inline_grow(table, (table->mask + 1) * 2, entry_size);
			at = -1;
		}
		if ((entry_size & TABLE_INLINE_KEY32) && hash == 0) {
			table->capacity = 1;
			table->length += 1;
			return inline_slot(table, table->mask + 1, entry_size);
		}
		uint64_t mask = (uint64_t)table->mask;
		if (at < 0) {
			at = (int64_t)(hash & mask);
			while (inline_tag(inline_slot(table, at, entry_size)) != 0) {
				at = (int64_t)(((uint64_t)at + 1) & mask);
			}
		}
		uint8_t *slot = inline_slot(table, at, entry_size);
		if (!(entry_size & TABLE_INLINE_KEY32)) {
			inline_set_tag(slot, inline_tag_of(hash));
		}
		table->length += 1;
		return slot + inline_offset(entry_size);
	}
	int64_t slot = probe < 0 ? ~probe : -1;
	if (__builtin_expect((table->length + 1) * 2 > table->mask + 1, 0)) {
		table_grow_index(table, (table->mask + 1) * 2, hash_at, entry_size);
		slot = -1;
	}
	if (__builtin_expect(table->free == 0, 0)) {
		table_new_chunk(table, entry_size);
	}
	int64_t chunk = table->free - 1;
	TableChunk *free = &table->chunks[chunk];
	uint32_t bit = (uint32_t)__builtin_ctz(~free->live);
	free->live |= 1u << bit;
	if (free->live == TABLE_CHUNK_FULL) {
		table->free = free->next;
	}
	int64_t position = (chunk << TABLE_CHUNK_BITS) | bit;
	if (position >= table->used) {
		table->used = position + 1;
	}
	table_place(table, hash, position, slot);
	table->length += 1;
	return entry_at(table, position, entry_size);
}

/// Marks the entry at a position dead, its key out of the index and its key
/// and value dropped or moved out.
FA_EXPORT __attribute__((always_inline)) void fa_table_release(FaTable *table, int64_t position,
                                                              int64_t entry_size) {
	if (entry_size & TABLE_INLINE) {
		inline_shift(table, (uint64_t)position, entry_size);
		return;
	}
	TableChunk *chunk = &table->chunks[position >> TABLE_CHUNK_BITS];
	if (chunk->live == TABLE_CHUNK_FULL) {
		chunk->next = (uint32_t)table->free;
		table->free = (position >> TABLE_CHUNK_BITS) + 1;
	}
	chunk->live &= ~(1u << (position & (TABLE_CHUNK_SLOTS - 1)));
	table->length -= 1;
}

/// Removes the entry at a position (its key and value were dropped or moved
/// out), `hash` the hash of its key: its slot is found again in the run.
FA_EXPORT __attribute__((always_inline)) void fa_table_kill(FaTable *table, uint64_t hash,
                                                            int64_t position, int64_t entry_size) {
	if (entry_size & TABLE_INLINE) {
		inline_shift(table, (uint64_t)position, entry_size);
		return;
	}
	uint64_t mask = (uint64_t)table->mask;
	uint64_t hole = hash & mask;
	if (__builtin_expect(table_narrow(table->mask), 1)) {
		const uint64_t *index = (const uint64_t *)table->index;
		uint64_t named = (uint64_t)(position + 1) << 32;
		while ((index[hole] & 0xffffffff00000000ull) != named) {
			hole = (hole + 1) & mask;
		}
	} else {
		const WideSlot *index = (const WideSlot *)table->index;
		while (index[hole].position != (uint64_t)(position + 1)) {
			hole = (hole + 1) & mask;
		}
	}
	table_shift(table, hole);
	fa_table_release(table, position, entry_size);
}

/// Empties a table whose keys and values were dropped (its chunks are kept,
/// all free, in order).
FA_EXPORT void fa_table_clear(FaTable *table, int64_t entry_size) {
	if (entry_size & TABLE_INLINE) {
		__builtin_memset(table->index, 0,
		                 (size_t)(inline_slots(table->mask, entry_size) * inline_stride(entry_size)));
		table->length = 0;
		table->capacity = 0;
		return;
	}
	int64_t chunks = table->capacity >> TABLE_CHUNK_BITS;
	for (int64_t chunk = 0; chunk < chunks; chunk += 1) {
		table->chunks[chunk].live = 0;
		table->chunks[chunk].next = chunk + 1 < chunks ? (uint32_t)(chunk + 2) : 0;
	}
	table->free = chunks > 0 ? 1 : 0;
	table->length = 0;
	table->used = 0;
	__builtin_memset(table->index, 0, (size_t)(table->mask + 1) * table_slot_size(table->mask));
}

/// Frees a table whose keys and values were dropped.
FA_EXPORT void fa_table_free(FaTable *table, int64_t entry_size) {
	if (table->storage != 0) {
		return;
	}
	if (entry_size & TABLE_INLINE) {
		inline_free_slots(table->index, table->mask, entry_size);
		fa_free(table, sizeof(FaTable), sizeof(int64_t));
		return;
	}
	int64_t chunks = table->capacity >> TABLE_CHUNK_BITS;
	if (entry_size & TABLE_DENSE) {
		if (table->entries != 0) {
			int64_t bytes = table_entry_bytes(entry_size);
			fa_free(table->entries, (size_t)(table->room * (bytes > 0 ? bytes : 1)), 16);
		}
		chunks = 0;
	}
	for (int64_t chunk = 0; chunk < chunks; chunk += 1) {
		fa_free(table->chunks[chunk].entries, table_chunk_bytes(entry_size), 16);
	}
	if (table->chunks != 0) {
		fa_free(table->chunks, (size_t)table->directory * sizeof(TableChunk), sizeof(void *));
	}
	table_free_index(table->index, table->mask);
	fa_free(table, sizeof(FaTable), sizeof(int64_t));
}

/// A copy of a table, bytewise, with the same positions and index (the
/// generated code then copies the live keys and values that own memory).
FA_EXPORT FaTable *fa_table_copy(const FaTable *table, int64_t entry_size) {
	FaTable *copy = fa_alloc(sizeof(FaTable), sizeof(int64_t));
	*copy = *table;
	copy->storage = 0;
	if (entry_size & TABLE_INLINE) {
		size_t bytes = (size_t)(inline_slots(table->mask, entry_size) * inline_stride(entry_size));
		copy->index = fa_alloc(bytes, 8);
		copy_bytes((uint8_t *)copy->index, (const uint8_t *)table->index, bytes);
		return copy;
	}
	int64_t chunks = table->capacity >> TABLE_CHUNK_BITS;
	int dense = (entry_size & TABLE_DENSE) != 0;
	if (chunks > 0) {
		copy->chunks = fa_alloc((size_t)table->directory * sizeof(TableChunk), sizeof(void *));
		for (int64_t chunk = 0; chunk < chunks; chunk += 1) {
			copy->chunks[chunk] = table->chunks[chunk];
			if (!dense) {
				copy->chunks[chunk].entries = fa_alloc(table_chunk_bytes(entry_size), 16);
				copy_bytes(copy->chunks[chunk].entries, table->chunks[chunk].entries,
				           table_chunk_bytes(entry_size));
			}
		}
	}
	if (dense && table->entries != 0) {
		int64_t bytes = table_entry_bytes(entry_size);
		size_t room = (size_t)(table->room * (bytes > 0 ? bytes : 1));
		copy->entries = fa_alloc(room, 16);
		copy_bytes(copy->entries, table->entries, (size_t)(table->capacity * bytes));
	}
	size_t bytes = (size_t)(table->mask + 1) * table_slot_size(table->mask);
	copy->index = fa_alloc(bytes, 8);
	copy_bytes((uint8_t *)copy->index, (const uint8_t *)table->index, bytes);
	return copy;
}

/// The hash of bytes (FNV-1a, then mixed).
/// The hash of a string's bytes, eight bytes at a time (tables are
/// insertion-ordered: the hash is never observable).
typedef struct {
	const uint8_t *bytes;
	size_t physical;
	int low;
	int high;
	uint8_t prefix[3];
	uint8_t suffix[3];
} StringByteSpan;

static uint64_t span_word(const StringByteSpan *span, size_t index, size_t width) {
	uint64_t word = 0;
	for (size_t i = 0; i < width; i += 1) {
		size_t at = index + i;
		uint8_t byte;
		if (span->low && at < 3) { byte = span->prefix[at]; }
		else {
			at -= (size_t)span->low;
			byte = span->high && at >= span->physical - 3
			           ? span->suffix[at - (span->physical - 3)] : span->bytes[at];
		}
		word |= (uint64_t)byte << (i * 8);
	}
	return word;
}

static __attribute__((always_inline)) uint64_t hash_byte_range(
    const uint8_t *bytes, size_t length, const StringByteSpan *span) {
	const uint64_t multiplier = 0x9E3779B97F4A7C15ull;
	uint64_t hash = 0xcbf29ce484222325ull ^ (length * multiplier);
	size_t index = 0;
	for (; index + 8 <= length; index += 8) {
		uint64_t word;
		if (span) { word = span_word(span, index, 8); }
		else { __builtin_memcpy(&word, bytes + index, 8); }
		hash = (hash ^ word) * multiplier;
		hash ^= hash >> 29;
	}
	if (index < length) {
		// the last 1 to 7 bytes, as one word
		uint64_t word = 0;
		size_t rest = length - index;
		if (rest >= 4) {
			uint32_t low;
			uint32_t high;
			if (span) {
				low = (uint32_t)span_word(span, index, 4);
				high = (uint32_t)span_word(span, length - 4, 4);
			} else {
				__builtin_memcpy(&low, bytes + index, 4);
				__builtin_memcpy(&high, bytes + length - 4, 4);
			}
			word = low | ((uint64_t)high << 32);
		} else {
			if (span) {
				word = span_word(span, index, 1) | (span_word(span, index + rest / 2, 1) << 8) |
				       (span_word(span, length - 1, 1) << 16);
			} else {
				word = bytes[index] | ((uint64_t)bytes[index + rest / 2] << 8) |
				       ((uint64_t)bytes[length - 1] << 16);
			}
		}
		hash = (hash ^ word) * multiplier;
	}
	hash ^= hash >> 33;
	hash *= 0xff51afd7ed558ccdull;
	hash ^= hash >> 33;
	return hash;
}

FA_EXPORT uint64_t fa_hash_bytes(const uint8_t *bytes, size_t length) {
	return hash_byte_range(bytes, length, 0);
}

static __attribute__((noinline)) uint64_t hash_string_span(const uint8_t *bytes, size_t length) {
	StringByteSpan span = {bytes, length, span_low_head(bytes, length), span_high_tail(bytes, length), {0}, {0}};
	if (span.low) { encode_point(span.prefix, span_low_unit(bytes)); }
	if (span.high) { encode_point(span.suffix, span_high_unit(bytes + length - 3)); }
	return hash_byte_range(bytes, length + (size_t)span.low, &span);
}

/// The same hash, operating on an inline string's value without spilling
/// its bytes to a temporary. External strings keep the byte-slice path.
FA_EXPORT __attribute__((always_inline)) uint64_t fa_hash_string_bits(size_t first, size_t second, size_t third) {
	if (!(third & STRING_INLINE)) {
		const uint8_t *bytes = (const uint8_t *)(uintptr_t)first;
		if (!(third & SIMPLE) && span_has_edges(bytes, second)) { return hash_string_span(bytes, second); }
		return fa_hash_bytes(bytes, second);
	}
	size_t length = (third >> INLINE_LENGTH_SHIFT) & 31;
	const uint64_t multiplier = 0x9E3779B97F4A7C15ull;
	uint64_t hash = 0xcbf29ce484222325ull ^ (length * multiplier);
	uint64_t a, b, c;
#if UINTPTR_MAX > UINT32_MAX
	a = first;
	b = second;
	c = third;
#else
	a = first | ((uint64_t)second << 32);
	b = third;
	c = 0;
#endif
	if (length >= 8) {
		hash = (hash ^ a) * multiplier;
		hash ^= hash >> 29;
	}
	if (length >= 16) {
		hash = (hash ^ b) * multiplier;
		hash ^= hash >> 29;
	}
	size_t rest = length & 7;
	if (rest != 0) {
		uint64_t tail = length >= 16 ? c : length >= 8 ? b : a;
		uint64_t word;
		if (rest >= 4) {
			word = (uint32_t)tail | ((uint64_t)(uint32_t)(tail >> ((rest - 4) * 8)) << 32);
		} else {
			word = (tail & 255) | (((tail >> ((rest / 2) * 8)) & 255) << 8) |
			       (((tail >> ((rest - 1) * 8)) & 255) << 16);
		}
		hash = (hash ^ word) * multiplier;
	}
	hash ^= hash >> 33;
	hash *= 0xff51afd7ed558ccdull;
	hash ^= hash >> 33;
	return hash;
}

/// `KeyNotFound { key }`, the key's text being built.
FA_EXPORT FA_COLD const void *fa_error_key_not_found(void) {
	MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
	error->id = KEY_NOT_FOUND;
	fa_b_take(&error->message);
	return error;
}

// ── concurrency (advanced brief 14, `src/lower/parallel.rs`) ────────────
//
// Threads come from the C library (`pthread_create`), declared weak: the
// freestanding target has none, and runs everything on one thread (a
// `for parallel` runs its iterations in order, a wait that no other thread
// can end stops the program). WebAssembly has no threads either, and no
// `for parallel` (`CONC001`).
//
// - **Handles** (`AtomicInteger`, `AtomicBoolean`, `Mutex(T)`, `Channel(T)`):
//   a heap block starting with a `FaShared` header (an atomic count of the
//   handles designating it, its size, and the drop of what it holds),
//   copied by `fa_shared_retain` and dropped by `fa_shared_release`.
// - **`for parallel`** (`fa_parallel_for`): a pool of one thread per core
//   but one, started at the first parallel loop; the calling thread takes
//   part. The iterations are split in chunks taken in order from an atomic
//   counter. A nested parallel loop, or one started while another runs,
//   runs on its own thread.
// - **Mutexes and channels** park the waiting thread on a condition
//   variable of the C library.

/// `pthread_mutex_t` and `pthread_cond_t` (at most 64 bytes on the C
/// libraries of the 64-bit targets; zeroed means initialized on glibc and musl).
typedef struct {
	_Alignas(16) uint8_t bytes[64];
} FaSystemLock;

#if defined(__wasm__)
// one thread: nothing waits, nothing is woken
static int has_threads(void) {
	return 0;
}
static void system_lock(FaSystemLock *lock) {
	(void)lock;
}
static void system_unlock(FaSystemLock *lock) {
	(void)lock;
}
static int pthread_cond_wait(FaSystemLock *condition, FaSystemLock *mutex) {
	(void)condition;
	(void)mutex;
	return 0;
}
static int pthread_cond_broadcast(FaSystemLock *condition) {
	(void)condition;
	return 0;
}
static int pthread_cond_signal(FaSystemLock *condition) {
	(void)condition;
	return 0;
}
#else
__attribute__((weak)) int pthread_create(unsigned long *thread, const void *attributes,
                                         void *(*start)(void *), void *argument);
__attribute__((weak)) int pthread_mutex_lock(FaSystemLock *mutex);
__attribute__((weak)) int pthread_mutex_unlock(FaSystemLock *mutex);
__attribute__((weak)) int pthread_cond_wait(FaSystemLock *condition, FaSystemLock *mutex);
__attribute__((weak)) int pthread_cond_broadcast(FaSystemLock *condition);
__attribute__((weak)) int pthread_cond_signal(FaSystemLock *condition);
__attribute__((weak)) long sysconf(int name);
__attribute__((weak)) int sched_yield(void);

/// Whether the program can start threads (the C library is there).
static int has_threads(void) {
	return pthread_create != 0;
}

static void system_lock(FaSystemLock *lock) {
	if (pthread_mutex_lock != 0) {
		pthread_mutex_lock(lock);
	}
}

static void system_unlock(FaSystemLock *lock) {
	if (pthread_mutex_unlock != 0) {
		pthread_mutex_unlock(lock);
	}
}
#endif

/// Stops the program: a wait that nothing can end.
static FA_COLD FA_NORETURN void deadlock(const char *what) {
	fa_flush();
	fa_write(2, (const uint8_t *)"stop: deadlock: ", 16);
	fa_write(2, (const uint8_t *)what, c_length(what));
	fa_write(2, (const uint8_t *)"\n", 1);
	fa_exit(101);
}

/// Whether another thread may run Fa code while this one waits blocking
/// (a parallel loop runs): otherwise nothing can end the wait.
static int others_may_wake(void);

// ── handles

typedef struct {
	int64_t count;
	int64_t size;
	/// Drops what the block holds (called with the block), or null.
	void (*drop)(void *block);
	int64_t reserved;
} FaShared;

/// A new block of `size` bytes (a header first, the rest zeroed) with one handle.
static void *shared_new(size_t size, void (*drop)(void *)) {
	FaShared *shared = fa_alloc(size, 16);
	if (shared == 0) {
		out_of_memory();
	}
	__builtin_memset(shared, 0, size);
	shared->count = 1;
	shared->size = (int64_t)size;
	shared->drop = drop;
	return shared;
}

/// A copy of a handle: the same block.
FA_EXPORT void *fa_shared_retain(void *block) {
	if (block != 0) {
		__atomic_fetch_add(&((FaShared *)block)->count, 1, __ATOMIC_RELAXED);
	}
	return block;
}

/// A handle dropped: the last one drops what the block holds, and frees it.
FA_EXPORT void fa_shared_release(void *block) {
	if (block == 0) {
		return;
	}
	FaShared *shared = block;
	if (__atomic_fetch_sub(&shared->count, 1, __ATOMIC_ACQ_REL) != 1) {
		return;
	}
	if (shared->drop != 0) {
		shared->drop(block);
	}
	fa_free(block, (size_t)shared->size, 16);
}

// ── atomics: the generated code reads and writes `value` with atomic
// instructions (`FA_ATOMIC_VALUE` in `src/lower/parallel.rs`)

typedef struct {
	FaShared shared;
	int64_t value;
} FaAtomicBlock;

_Static_assert(__builtin_offsetof(FaAtomicBlock, value) == 32, "the offset of an atomic's value");

FA_EXPORT void *fa_atomic_new(int64_t value) {
	FaAtomicBlock *block = shared_new(sizeof(FaAtomicBlock), 0);
	block->value = value;
	return block;
}

// ── tasks and the event loop (design/async.md §3, `src/lower/tasks.rs`)
//
// Only asynchronous programs use this part (`Program::asynchronous`): a
// program in synchronous mode never calls it, and the optimizer drops it.
//
// - **A task** is the frame of an LLVM switched-resume coroutine (or a
//   *finished block* with the same layout: a null resume pointer, a destroy
//   function, then the header). Its promise, at `TASK_AT` bytes, starts
//   with a `FaTask` header, followed by the result. A frame is done when
//   its resume pointer is null (`llvm.coro.done`).
// - **The executor** of a thread (the main thread for now) runs the tasks
//   of its ready queue (an intrusive FIFO, no atomics) one at a time: a
//   *turn* resumes a task, then the tasks its end wakes (the trampoline of
//   `next_frame`, no stack growth). A *tick* runs at most 61 turns, then
//   fires the expired timers (a 4-ary min-heap), takes the wakes of the
//   other threads (a Vyukov MPSC queue plus an `eventfd`) and polls epoll:
//   timers and other threads can't be starved by ready tasks.
// - **The coop budget** (`fa_coop_budget`, 128 per turn): an `await` of a
//   primitive that is ready takes one unit, and at 0 the task goes to the
//   tail of the queue although ready; a loop back-edge of an async instance
//   takes one unit too, and at 0 the task yields when the turn lasted more
//   than 2 ms (the lead's amendment A2).
// - **Waits** (`FaWait`) live in the waiting frame: a timer, a place in the
//   line of a mutex or a channel. A wait allocates nothing, and the destroy
//   edge of the suspension point cancels it (`fa_timer_cancel`,
//   `fa_mutex_cancel`, `fa_channel_cancel`).
// - **Mutexes and channels** keep their waiting tasks in FIFO order and hand
//   the lock or the value over directly (no barging); a thread that waits
//   blocking uses the condition variable, as before.

/// The monotonic clock in nanoseconds, and a blocking sleep (tier 0).
int64_t fa_clock(void);
void fa_nanosleep(int64_t nanoseconds);
/// `llvm.coro.resume` and `llvm.coro.destroy` of a frame (generated IR: the
/// resume and destroy functions of a coroutine use LLVM's `fastcc`).
void fa_resume(void *frame);
void fa_destroy(void *frame);

/// The header of a task, at `TASK_AT` bytes in its frame (the promise).
typedef struct FaTask {
	/// The frame awaiting it, or its join node with the low bit set, or null.
	void *parent;
	/// The links of the ready queue, of the remote queue (`next` only), of
	/// the finished tasks of a join node (`next` only).
	struct FaTask *next;
	struct FaTask *prev;
	uint32_t state;
	uint32_t flags;
} FaTask;

#define TASK_AT 16

enum {
	TASK_CREATED = 0,
	TASK_QUEUED = 1,
	TASK_WAITING = 2,
	TASK_DONE = 3,
	TASK_RUNNING = 4,
	/// In the remote queue of its executor.
	TASK_REMOTE = 5,
};

enum {
	/// Started and not kept: the program waits for it, and its frame is
	/// freed at its end.
	TASK_DETACHED = 1,
	/// Its result is in the promise, not moved out yet (dropped with it).
	TASK_RESULT = 2,
	/// In the yielded queue of its executor (not the woken one).
	TASK_YIELDED = 4,
};

/// The wait of a suspended task (an `alloca` of its frame).
typedef struct FaWait {
	void *frame;
	struct FaExecutor *executor;
	/// The line of a mutex or a channel (circular).
	struct FaWait *next;
	struct FaWait *prev;
	/// A channel: the value sent, or where the value received goes.
	void *item;
	/// Where the program waits, for the deadlock message.
	const char *site;
	int64_t deadline;
	int64_t sequence;
	/// The place in the timer heap, -1 outside.
	int64_t index;
	/// `WAIT_*`.
	int32_t status;
	/// Woken by another thread, through the remote queue.
	int32_t remote;
	/// A timer that calls this when it fires instead of waking its task.
	void (*fired)(struct FaWait *wait);
} FaWait;

/// The size the generated code reserves for a wait (`[16 x i64]`).
_Static_assert(sizeof(FaWait) <= 128, "the size of a wait");

enum {
	WAIT_WAITING = 0,
	/// Handed over: the value sent or received, the lock taken.
	WAIT_DONE = 1,
	WAIT_CLOSED = 2,
};

/// The result of an `await` of a primitive (`fa_*_async`).
enum {
	/// Done now, go on.
	AWAIT_READY = 0,
	/// The task waits: suspend; the status of the wait says how it ended.
	AWAIT_SUSPEND = 1,
	/// Done now, but the coop budget is spent: the task was queued, suspend.
	AWAIT_YIELD = 2,
	/// The channel is closed.
	AWAIT_CLOSED = 3,
};

/// A join node (`Task.all`, `Task.any`, `Task.settled`): the tasks whose
/// parent it is, those that ended, and the coroutine waiting for them.
typedef struct {
	void *waiter;
	int64_t remaining;
	FaTask *head;
	FaTask *tail;
} FaJoin;

_Static_assert(sizeof(FaJoin) <= 64, "the size of a join node");

/// An entry of the timer heap: the deadline inline, so that the four
/// children of a node are one cache line and a sift reads no wait.
typedef struct {
	int64_t deadline;
	FaWait *wait;
} FaTimer;

typedef struct FaExecutor {
	/// The ready queues (`design/decisions.md`, "Async scheduler: wakes
	/// before yields"): the woken tasks (`head`, `tail`: timers, I/O
	/// readiness, remote wakes, a finished task's waiter, new tasks) and the
	/// yielded ones (a spent coop budget, `Task.yield`, a back-edge yield);
	/// `ready` counts both, `woken` the first one.
	FaTask *head;
	FaTask *tail;
	FaTask *yield_head;
	FaTask *yield_tail;
	int64_t ready;
	int64_t woken;
	/// The timers: a 4-ary min-heap by (deadline, sequence).
	FaTimer *timers;
	int64_t timer_count;
	int64_t timer_room;
	int64_t sequence;
	/// The task of the program's top level, and the detached tasks alive.
	void *root;
	int64_t detached;
	/// The other threads that may wake a task of this executor (the jobs
	/// of the blocking pool in flight; atomic): while there are some,
	/// waiting for them is no deadlock.
	int64_t sources;
	/// The tasks waiting for a file descriptor (`fa_io_wait`).
	int64_t io_waiting;
	/// The other threads inside `fa_remote_source_end` (atomic): the
	/// executor is freed once none is.
	int64_t ending;
	/// The remote queue: other threads push at `remote_tail`, this thread
	/// pops at `remote_head` (Vyukov's intrusive MPSC queue).
	FaTask *remote_head;
	FaTask *remote_tail;
	FaTask remote_stub;
	/// Set while the thread waits in epoll (atomic): a remote wake writes
	/// to `event` then.
	int32_t parked;
	int32_t epoll;
	int32_t event;
	/// Dev profiles: warn about long turns.
	int32_t dev;
	/// The turns since epoll was last polled.
	int64_t turns;
	/// A task yielded in this tick (`Task.yield()`, the coop budget): the
	/// descriptors are polled before the next tick, so that I/O comes in
	/// between the turns of tasks that yield (as Tokio defers `yield_now`).
	int32_t yielded;
	/// The time the budget was first spent in this turn (0: not yet).
	int64_t spent_at;
	/// Where the root task and any task last waited on a primitive (before
	/// `fa_run_main`, `root` is null: only the root's ramp runs).
	const char *root_site;
	const char *last_site;
	/// The sites already warned about (dev).
	const char *warned[32];
	int32_t warned_count;
} FaExecutor;

static FA_THREAD_LOCAL FaExecutor *current_executor;
/// The trampoline: the frame to resume after the current one suspends.
static FA_THREAD_LOCAL void *next_frame;
/// A detached task that ended: its frame is freed after it suspends.
static FA_THREAD_LOCAL void *frame_to_free;
/// The coop budget of the current turn (read by the generated code).
FA_THREAD_LOCAL int32_t fa_coop_budget;
/// Above 0 inside a blocking call of an async function: `start` runs the
/// task to its end at once (the generated code changes it).
FA_THREAD_LOCAL int32_t fa_blocking_depth;
/// Where the running task is, in dev profiles: `function, file:line`.
FA_THREAD_LOCAL const char *fa_turn_site;

#define COOP_BUDGET 128
#define EVENT_INTERVAL 61
#define LONG_TURN ((int64_t)50000000)
#define COOP_SLICE ((int64_t)2000000)

static FaTask *task_of(void *frame) {
	return (FaTask *)((uint8_t *)frame + TASK_AT);
}

static void *frame_of(FaTask *task) {
	return (uint8_t *)task - TASK_AT;
}

static int frame_done(void *frame) {
	return *(void **)frame == 0;
}

static uint32_t task_state(FaTask *task) {
	return __atomic_load_n(&task->state, __ATOMIC_ACQUIRE);
}

static void set_task_state(FaTask *task, uint32_t state) {
	__atomic_store_n(&task->state, state, __ATOMIC_RELEASE);
}

#if !defined(__wasm__)
// epoll and eventfd of the C library (none when freestanding)
typedef struct {
	uint32_t events;
	uint64_t data;
}
#if defined(__x86_64__)
__attribute__((packed))
#endif
FaEpollEvent;
__attribute__((weak)) int epoll_create1(int flags);
__attribute__((weak)) int epoll_ctl(int epoll, int operation, int fd, FaEpollEvent *event);
__attribute__((weak)) int epoll_wait(int epoll, FaEpollEvent *events, int count, int timeout);
__attribute__((weak)) int eventfd(unsigned int value, int flags);
__attribute__((weak)) long read(int fd, void *bytes, size_t count);
__attribute__((weak)) long write(int fd, const void *bytes, size_t count);
__attribute__((weak)) int close(int fd);
#endif

static FaExecutor *executor(void) {
	FaExecutor *e = current_executor;
	if (e != 0) {
		return e;
	}
	e = fa_alloc(sizeof(FaExecutor), 16);
	__builtin_memset(e, 0, sizeof(FaExecutor));
	e->remote_head = &e->remote_stub;
	e->remote_tail = &e->remote_stub;
	e->epoll = -1;
	e->event = -1;
#if !defined(__wasm__)
	if (epoll_create1 != 0 && eventfd != 0 && has_threads()) {
		// EPOLL_CLOEXEC, EFD_CLOEXEC | EFD_NONBLOCK
		e->epoll = epoll_create1(02000000);
		e->event = eventfd(0, 02000000 | 04000);
		if (e->epoll >= 0 && e->event >= 0) {
			FaEpollEvent event = {1 /* EPOLLIN */, 0};
			epoll_ctl(e->epoll, 1 /* EPOLL_CTL_ADD */, e->event, &event);
		}
	}
#endif
	fa_coop_budget = COOP_BUDGET;
	current_executor = e;
	return e;
}

// ── the ready queues
//
// Each tick serves the woken tasks first, then the yielded ones; when both
// queues hold tasks at its start, the woken ones take at most half of its
// turns, so that a flood of events can't starve the tasks that yielded.
// FIFO inside each queue.

static void queue_push(FaTask **head, FaTask **tail, FaTask *task) {
	task->next = 0;
	task->prev = *tail;
	if (*tail != 0) {
		(*tail)->next = task;
	} else {
		*head = task;
	}
	*tail = task;
}

static void queue_unlink(FaTask **head, FaTask **tail, FaTask *task) {
	if (task->prev != 0) {
		task->prev->next = task->next;
	} else {
		*head = task->next;
	}
	if (task->next != 0) {
		task->next->prev = task->prev;
	} else {
		*tail = task->prev;
	}
	task->next = 0;
	task->prev = 0;
}

/// A task woken by an event (or new): at the tail of the woken queue.
static void ready_push(FaExecutor *e, FaTask *task) {
	task->flags &= ~(uint32_t)TASK_YIELDED;
	queue_push(&e->head, &e->tail, task);
	e->ready += 1;
	e->woken += 1;
	set_task_state(task, TASK_QUEUED);
}

/// A task that lets the others run: at the tail of the yielded queue.
static void yield_push(FaExecutor *e, FaTask *task) {
	task->flags |= TASK_YIELDED;
	queue_push(&e->yield_head, &e->yield_tail, task);
	e->ready += 1;
	set_task_state(task, TASK_QUEUED);
}

/// Takes a queued task out of its queue.
static void ready_unlink(FaExecutor *e, FaTask *task) {
	if (task->flags & TASK_YIELDED) {
		queue_unlink(&e->yield_head, &e->yield_tail, task);
		task->flags &= ~(uint32_t)TASK_YIELDED;
	} else {
		queue_unlink(&e->head, &e->tail, task);
		e->woken -= 1;
	}
	e->ready -= 1;
}

/// The next task of a tick: a woken one while `woken_turns` is under
/// `woken_limit`, else a yielded one, else a woken one; null when none.
static FaTask *ready_pop(FaExecutor *e, int64_t *woken_turns, int64_t woken_limit) {
	FaTask *task;
	if (e->head != 0 && (*woken_turns < woken_limit || e->yield_head == 0)) {
		task = e->head;
		*woken_turns += 1;
	} else {
		task = e->yield_head;
	}
	if (task != 0) {
		ready_unlink(e, task);
	}
	return task;
}

/// The turns of a tick, and how many the woken tasks may take.
static int64_t tick_turns(FaExecutor *e, int64_t *woken_limit) {
	int64_t count = e->ready < EVENT_INTERVAL ? e->ready : EVENT_INTERVAL;
	// half of the 61 turns of a tick
	*woken_limit = e->woken > 0 && e->yield_head != 0 ? EVENT_INTERVAL / 2 : count;
	return count;
}

// ── the remote queue

/// Wakes a task of another thread's executor.
static void remote_push(FaExecutor *e, FaTask *task) {
	set_task_state(task, TASK_REMOTE);
	__atomic_store_n(&task->next, (FaTask *)0, __ATOMIC_RELAXED);
	FaTask *previous = __atomic_exchange_n(&e->remote_tail, task, __ATOMIC_ACQ_REL);
	__atomic_store_n(&previous->next, task, __ATOMIC_RELEASE);
#if !defined(__wasm__)
	if (__atomic_exchange_n(&e->parked, 0, __ATOMIC_SEQ_CST) != 0 && e->event >= 0) {
		uint64_t one = 1;
		write(e->event, &one, sizeof one);
	}
#endif
}

/// Another thread will wake a task of `e` (a job of the blocking pool).
FA_EXPORT void fa_remote_source_start(FaExecutor *e) {
	__atomic_fetch_add(&e->sources, 1, __ATOMIC_ACQ_REL);
}

/// That thread won't (after its wake, if any): the executor rechecks.
FA_EXPORT void fa_remote_source_end(FaExecutor *e) {
	// the executor may end once `sources` is 0: `ending` keeps it alive
	// until this thread is done with it
	__atomic_fetch_add(&e->ending, 1, __ATOMIC_SEQ_CST);
	__atomic_fetch_sub(&e->sources, 1, __ATOMIC_SEQ_CST);
#if !defined(__wasm__)
	if (__atomic_exchange_n(&e->parked, 0, __ATOMIC_SEQ_CST) != 0 && e->event >= 0) {
		uint64_t one = 1;
		write(e->event, &one, sizeof one);
	}
#endif
	__atomic_fetch_sub(&e->ending, 1, __ATOMIC_SEQ_CST);
}

/// The next task of the remote queue (this thread's), or null (none, or
/// one being pushed: it is seen at the next look).
static FaTask *remote_pop(FaExecutor *e) {
	FaTask *head = e->remote_head;
	FaTask *next = __atomic_load_n(&head->next, __ATOMIC_ACQUIRE);
	if (head == &e->remote_stub) {
		if (next == 0) {
			return 0;
		}
		e->remote_head = next;
		head = next;
		next = __atomic_load_n(&next->next, __ATOMIC_ACQUIRE);
	}
	if (next != 0) {
		e->remote_head = next;
		return head;
	}
	if (__atomic_load_n(&e->remote_tail, __ATOMIC_ACQUIRE) != head) {
		return 0;
	}
	FaTask *stub = &e->remote_stub;
	__atomic_store_n(&stub->next, (FaTask *)0, __ATOMIC_RELAXED);
	FaTask *previous = __atomic_exchange_n(&e->remote_tail, stub, __ATOMIC_ACQ_REL);
	__atomic_store_n(&previous->next, stub, __ATOMIC_RELEASE);
	next = __atomic_load_n(&head->next, __ATOMIC_ACQUIRE);
	if (next != 0) {
		e->remote_head = next;
		return head;
	}
	return 0;
}

/// Moves the tasks woken by other threads to the ready queue.
static void drain_remote(FaExecutor *e) {
	FaTask *task;
	while ((task = remote_pop(e)) != 0) {
		ready_push(e, task);
	}
}

/// Wakes the task of a wait: at the tail of its executor's ready queue.
static void wake(FaWait *wait) {
	FaTask *task = task_of(wait->frame);
	FaExecutor *e = wait->executor;
	if (e == current_executor) {
		ready_push(e, task);
		return;
	}
	wait->remote = 1;
	remote_push(e, task);
}

/// After a wait handed over by another thread was cancelled: its task is
/// in the remote queue, or in the ready queue (taken out of both).
static void forget_remote_wake(FaWait *wait) {
	if (!wait->remote) {
		return;
	}
	FaExecutor *e = current_executor;
	drain_remote(e);
	FaTask *task = task_of(wait->frame);
	if (task_state(task) == TASK_QUEUED) {
		ready_unlink(e, task);
	}
}

// ── tasks

/// Spends a unit of the coop budget at an `await` that is ready: whether
/// the task must let the others run first.
static int coop_spent(void) {
	fa_coop_budget -= 1;
	if (fa_coop_budget > 0) {
		return 0;
	}
	if (current_executor != 0) {
		current_executor->yielded = 1;
	}
	return 1;
}

/// Queues a task created by `start` (its frame is at its initial suspension).
FA_EXPORT void fa_task_spawn(void *frame, int32_t detached) {
	FaExecutor *e = executor();
	FaTask *task = task_of(frame);
	if (detached) {
		task->flags |= TASK_DETACHED;
		e->detached += 1;
	}
	ready_push(e, task);
}

/// `await child` in the task of `frame`, the child not done: the child
/// wakes it at its end.
FA_EXPORT void fa_task_await(void *child, void *frame) {
	task_of(child)->parent = frame;
	set_task_state(task_of(frame), TASK_WAITING);
	FaExecutor *e = current_executor;
	if (e != 0 && (frame == e->root || e->root == 0)) {
		e->root_site = 0;
	}
}

static void join_finished(FaJoin *join, FaTask *task);

/// The end of a task (its result is in its promise), right before its
/// final suspension: wakes what awaits it.
FA_EXPORT void fa_task_finish(void *frame) {
	FaTask *task = task_of(frame);
	task->flags |= TASK_RESULT;
	set_task_state(task, TASK_DONE);
	if (task->flags & TASK_DETACHED) {
		current_executor->detached -= 1;
		frame_to_free = frame;
		return;
	}
	void *parent = task->parent;
	if (parent == 0) {
		return;
	}
	task->parent = 0;
	if ((uintptr_t)parent & 1) {
		join_finished((FaJoin *)((uintptr_t)parent & ~(uintptr_t)1), task);
		return;
	}
	next_frame = parent;
}

/// The drop of a task (the end of its scope, `cancel()`): a task not done
/// is cancelled at its suspension point (its cleanups run), and its frame
/// freed. Null and 1 (a cancelled task) are no tasks.
FA_EXPORT void fa_task_drop(void *frame) {
	if ((uintptr_t)frame <= 1) {
		return;
	}
	FaTask *task = task_of(frame);
	FaExecutor *e = current_executor;
	if (e != 0) {
		if (task_state(task) == TASK_REMOTE) {
			drain_remote(e);
		}
		if (task_state(task) == TASK_QUEUED) {
			ready_unlink(e, task);
		}
	}
	fa_destroy(frame);
}

/// `await Task.yield()`, or a coop yield: the task goes to the tail of the queue.
FA_EXPORT void fa_task_yield(void *frame) {
	FaExecutor *e = executor();
	e->yielded = 1;
	yield_push(e, task_of(frame));
}

/// Spends a unit of the coop budget at a loop back-edge of an async
/// instance (A2): 1 when the task must yield (the budget is spent and the
/// turn lasted more than 2 ms).
FA_EXPORT int32_t fa_coop_back_edge(void) {
	FaExecutor *e = current_executor;
	fa_coop_budget = COOP_BUDGET;
	if (e == 0) {
		return 0;
	}
	int64_t now = fa_clock();
	if (e->spent_at == 0) {
		e->spent_at = now;
		return 0;
	}
	return now - e->spent_at > COOP_SLICE;
}

// ── timers

static int timer_before(FaTimer a, FaTimer b) {
	return a.deadline < b.deadline ||
	       (a.deadline == b.deadline && a.wait->sequence < b.wait->sequence);
}

static void timer_place(FaExecutor *e, int64_t index, FaTimer timer) {
	e->timers[index] = timer;
	timer.wait->index = index;
}

static void timer_up(FaExecutor *e, int64_t index) {
	FaTimer timer = e->timers[index];
	while (index > 0) {
		int64_t parent = (index - 1) / 4;
		if (!timer_before(timer, e->timers[parent])) {
			break;
		}
		timer_place(e, index, e->timers[parent]);
		index = parent;
	}
	timer_place(e, index, timer);
}

static void timer_down(FaExecutor *e, int64_t index) {
	FaTimer timer = e->timers[index];
	for (;;) {
		int64_t first = index * 4 + 1;
		if (first >= e->timer_count) {
			break;
		}
		int64_t best = first;
		int64_t last = first + 4 < e->timer_count ? first + 4 : e->timer_count;
		for (int64_t child = first + 1; child < last; child += 1) {
			if (timer_before(e->timers[child], e->timers[best])) {
				best = child;
			}
		}
		if (!timer_before(e->timers[best], timer)) {
			break;
		}
		timer_place(e, index, e->timers[best]);
		index = best;
	}
	timer_place(e, index, timer);
}

static void timer_remove(FaExecutor *e, int64_t index) {
	FaWait *removed = e->timers[index].wait;
	e->timer_count -= 1;
	if (index < e->timer_count) {
		FaTimer last = e->timers[e->timer_count];
		timer_place(e, index, last);
		if (index > 0 && timer_before(last, e->timers[(index - 1) / 4])) {
			timer_up(e, index);
		} else {
			timer_down(e, index);
		}
	}
	removed->index = -1;
}

/// `await sleep(milliseconds)` in the task of `frame`: it waits in `wait`
/// until the deadline (a sleep of 0 or less lets the others run first).
static void timer_insert(FaExecutor *e, FaWait *wait, int64_t deadline);

FA_EXPORT void fa_sleep_async(FaWait *wait, void *frame, int64_t milliseconds) {
	FaExecutor *e = executor();
	wait->frame = frame;
	wait->executor = e;
	wait->index = -1;
	wait->fired = 0;
	if (milliseconds <= 0) {
		yield_push(e, task_of(frame));
		return;
	}
	timer_insert(e, wait, fa_clock() + milliseconds * 1000000);
	set_task_state(task_of(frame), TASK_WAITING);
	if (frame == e->root || e->root == 0) {
		e->root_site = 0;
	}
}

/// Puts a wait in the timer heap of `e`, until `deadline`.
static void timer_insert(FaExecutor *e, FaWait *wait, int64_t deadline) {
	wait->executor = e;
	if (e->timer_count == e->timer_room) {
		int64_t room = e->timer_room < 16 ? 16 : e->timer_room * 2;
		FaTimer *timers = fa_alloc((size_t)room * sizeof(FaTimer), 64);
		for (int64_t index = 0; index < e->timer_count; index += 1) {
			timers[index] = e->timers[index];
		}
		if (e->timer_room > 0) {
			fa_free(e->timers, (size_t)e->timer_room * sizeof(FaTimer), 64);
		}
		e->timers = timers;
		e->timer_room = room;
	}
	wait->deadline = deadline;
	wait->sequence = e->sequence;
	e->sequence += 1;
	e->timer_count += 1;
	FaTimer timer = {wait->deadline, wait};
	timer_place(e, e->timer_count - 1, timer);
	timer_up(e, e->timer_count - 1);
}

/// The destroy edge of a sleep: its timer is removed.
FA_EXPORT void fa_timer_cancel(FaWait *wait) {
	if (wait->index >= 0) {
		timer_remove(wait->executor, wait->index);
	}
}

/// `sleep(milliseconds)` blocking.
FA_EXPORT void fa_sleep(int64_t milliseconds) {
	if (milliseconds > 0) {
		fa_nanosleep(milliseconds * 1000000);
	}
}

static void fire_timers(FaExecutor *e, int64_t now) {
	while (e->timer_count > 0 && e->timers[0].deadline <= now) {
		FaWait *wait = e->timers[0].wait;
		timer_remove(e, 0);
		if (wait->fired != 0) {
			wait->fired(wait);
		} else {
			ready_push(e, task_of(wait->frame));
		}
	}
}

// ── join nodes

/// Prepares a join node.
FA_EXPORT void fa_join_init(FaJoin *join) {
	__builtin_memset(join, 0, sizeof(FaJoin));
}

static void join_push(FaJoin *join, FaTask *task) {
	task->next = 0;
	if (join->tail != 0) {
		join->tail->next = task;
	} else {
		join->head = task;
	}
	join->tail = task;
}

/// Adds a task to a join node (a cancelled task, 1, is left out).
FA_EXPORT void fa_join_add(FaJoin *join, void *frame) {
	if ((uintptr_t)frame <= 1) {
		return;
	}
	FaTask *task = task_of(frame);
	if (frame_done(frame)) {
		join_push(join, task);
		return;
	}
	task->parent = (void *)((uintptr_t)join | 1);
	join->remaining += 1;
}

static void join_finished(FaJoin *join, FaTask *task) {
	join_push(join, task);
	join->remaining -= 1;
	if (join->waiter != 0) {
		next_frame = join->waiter;
		join->waiter = 0;
	}
}

/// The next task of a join node that ended (its frame); 1 when every task
/// ended; null when the coroutine of `frame` must wait for one.
FA_EXPORT void *fa_join_next(FaJoin *join, void *frame) {
	FaTask *task = join->head;
	if (task != 0) {
		join->head = task->next;
		if (join->head == 0) {
			join->tail = 0;
		}
		task->next = 0;
		return frame_of(task);
	}
	if (join->remaining == 0) {
		return (void *)1;
	}
	if (frame == 0) {
		return 0;
	}
	join->waiter = frame;
	set_task_state(task_of(frame), TASK_WAITING);
	return 0;
}

/// A task that isn't done is awaited in a blocking call (A13).
FA_EXPORT FA_COLD FA_NORETURN void fa_task_stuck(void) {
	deadlock("a blocking call waits for a task that isn't done");
}

// ── the event loop

static void poll_events(FaExecutor *e, int64_t timeout);

// ── file descriptors (epoll, edge-triggered: readiness only)
//
// A descriptor is registered once (`fa_io_register`) for reading and
// writing; an operation tries its system call first, and on `EAGAIN` the
// task waits (`fa_io_wait`) until epoll says the descriptor is ready again.
// Cancelling a wait forgets the interest (`fa_io_cancel`).

typedef struct FaIo {
	int32_t fd;
	/// `IO_READABLE`, `IO_WRITABLE`: what epoll said since the last `EAGAIN`.
	uint32_t ready;
	FaWait *reader;
	FaWait *writer;
	FaExecutor *executor;
} FaIo;

#define IO_READABLE 1u
#define IO_WRITABLE 4u

#if !defined(__wasm__)
/// A descriptor became ready: its waiting tasks are woken.
static void io_ready(FaIo *io, uint32_t events) {
	// EPOLLIN, EPOLLRDHUP, EPOLLHUP, EPOLLERR
	if (events & (0x001u | 0x2000u | 0x010u | 0x008u)) {
		io->ready |= IO_READABLE;
		if (io->reader != 0) {
			io->reader->status = WAIT_DONE;
			io->executor->io_waiting -= 1;
			ready_push(io->executor, task_of(io->reader->frame));
			io->reader = 0;
		}
	}
	// EPOLLOUT, EPOLLHUP, EPOLLERR
	if (events & (0x004u | 0x010u | 0x008u)) {
		io->ready |= IO_WRITABLE;
		if (io->writer != 0) {
			io->writer->status = WAIT_DONE;
			io->executor->io_waiting -= 1;
			ready_push(io->executor, task_of(io->writer->frame));
			io->writer = 0;
		}
	}
}
#endif

/// Registers a non-blocking descriptor with the thread's epoll; null when
/// there is no epoll (the operations block then).
FA_EXPORT FaIo *fa_io_register(int32_t fd) {
#if !defined(__wasm__)
	FaExecutor *e = executor();
	if (e->epoll < 0) {
		return 0;
	}
	FaIo *io = fa_alloc(sizeof(FaIo), 16);
	__builtin_memset(io, 0, sizeof(FaIo));
	io->fd = fd;
	io->ready = IO_READABLE | IO_WRITABLE;
	io->executor = e;
	// EPOLLIN | EPOLLOUT | EPOLLRDHUP | EPOLLET
	FaEpollEvent event = {0x001u | 0x004u | 0x2000u | (1u << 31), (uint64_t)(uintptr_t)io};
	if (epoll_ctl(e->epoll, 1 /* EPOLL_CTL_ADD */, fd, &event) != 0) {
		fa_free(io, sizeof(FaIo), 16);
		return 0;
	}
	return io;
#else
	(void)fd;
	return 0;
#endif
}

/// After `EAGAIN`: the task of `frame` waits in `wait` until the descriptor
/// is readable (or writable) again: `AWAIT_SUSPEND`.
FA_EXPORT int32_t fa_io_wait(FaIo *io, int32_t writable, FaWait *wait, void *frame) {
	uint32_t bit = writable ? IO_WRITABLE : IO_READABLE;
	// epoll is only read between turns, so the readiness seen came before the
	// system call that failed: it is stale, and new data makes a new edge
	io->ready &= ~bit;
	wait->frame = frame;
	wait->executor = io->executor;
	wait->status = WAIT_WAITING;
	wait->remote = 0;
	wait->index = -1;
	if (writable) {
		io->writer = wait;
	} else {
		io->reader = wait;
	}
	io->executor->io_waiting += 1;
	set_task_state(task_of(frame), TASK_WAITING);
	return AWAIT_SUSPEND;
}

/// The destroy edge of a wait for a descriptor: the interest is forgotten.
FA_EXPORT void fa_io_cancel(FaIo *io, FaWait *wait) {
	if (io->reader == wait) {
		io->reader = 0;
		io->executor->io_waiting -= 1;
	}
	if (io->writer == wait) {
		io->writer = 0;
		io->executor->io_waiting -= 1;
	}
}

/// The descriptor is closed (by its owner): forgets it.
FA_EXPORT void fa_io_close(FaIo *io) {
	if (io == 0) {
		return;
	}
#if !defined(__wasm__)
	FaEpollEvent event = {0, 0};
	epoll_ctl(io->executor->epoll, 2 /* EPOLL_CTL_DEL */, io->fd, &event);
#endif
	fa_free(io, sizeof(FaIo), 16);
}

/// Waits for the other threads (epoll) at most `timeout` nanoseconds (-1:
/// no limit, 0: only looks).
static void poll_events(FaExecutor *e, int64_t timeout) {
#if !defined(__wasm__)
	if (e->epoll >= 0) {
		int milliseconds = timeout < 0 ? -1 : (int)((timeout + 999999) / 1000000);
		if (timeout != 0) {
			__atomic_store_n(&e->parked, 1, __ATOMIC_SEQ_CST);
			if (__atomic_load_n(&e->remote_head->next, __ATOMIC_ACQUIRE) != 0 ||
			    __atomic_load_n(&e->remote_tail, __ATOMIC_ACQUIRE) != e->remote_head) {
				milliseconds = 0;
			}
			// the last source ended before `parked` was set (it wrote no
			// event): don't wait for it (the caller checks again)
			if (timeout < 0 && e->io_waiting == 0 &&
			    __atomic_load_n(&e->sources, __ATOMIC_SEQ_CST) == 0) {
				milliseconds = 0;
			}
		}
		FaEpollEvent events[256];
		int count = epoll_wait(e->epoll, events, 256, milliseconds);
		__atomic_store_n(&e->parked, 0, __ATOMIC_SEQ_CST);
		for (int index = 0; index < count; index += 1) {
			if (events[index].data == 0) {
				uint64_t value;
				read(e->event, &value, sizeof value);
			} else {
				io_ready((FaIo *)(uintptr_t)events[index].data, events[index].events);
			}
		}
		drain_remote(e);
		return;
	}
#else
	(void)e;
#endif
	if (timeout > 0) {
		fa_nanosleep(timeout);
	}
}

static void warn_long_turn(FaExecutor *e, int64_t elapsed) {
	const char *site = fa_turn_site;
	for (int32_t index = 0; index < e->warned_count; index += 1) {
		if (e->warned[index] == site) {
			return;
		}
	}
	if (e->warned_count < 32) {
		e->warned[e->warned_count] = site;
		e->warned_count += 1;
	}
	fa_flush();
	fa_b_reset();
	append_c("warning: a task kept the thread for ");
	fa_b_i64(elapsed / 1000000);
	append_c(" ms (");
	append_c(site != 0 ? site : "a task");
	append_c("): await Task.yield() or move the work to a worker");
	fa_b_line(2);
}

/// One turn: resumes a task, then the tasks its end wakes (the trampoline).
static void turn(FaExecutor *e, void *frame) {
	fa_coop_budget = COOP_BUDGET;
	e->spent_at = 0;
	int64_t start = e->dev ? fa_clock() : 0;
	while (frame != 0) {
		set_task_state(task_of(frame), TASK_RUNNING);
		fa_resume(frame);
		if (frame_to_free != 0) {
			void *ended = frame_to_free;
			frame_to_free = 0;
			fa_destroy(ended);
		}
		frame = next_frame;
		next_frame = 0;
	}
	if (e->dev) {
		int64_t elapsed = fa_clock() - start;
		if (elapsed > LONG_TURN) {
			warn_long_turn(e, elapsed);
		}
	}
}

static FA_COLD FA_NORETURN void deadlock_stop(FaExecutor *e) {
	fa_flush();
	fa_b_reset();
	append_c("stop: deadlock: every task waits");
	if (e->root_site != 0) {
		append_c(" (main waits on ");
		append_c(e->root_site);
		append_c(")");
	} else if (e->last_site != 0) {
		append_c(" (a task waits on ");
		append_c(e->last_site);
		append_c(")");
	}
	fa_b_line(2);
	fa_exit(101);
}

/// A tick: at most 61 turns, then the timers, the other threads, and the
/// wait for them when nothing is ready.
static void tick(FaExecutor *e) {
	int64_t woken_limit;
	int64_t woken_turns = 0;
	int64_t count = tick_turns(e, &woken_limit);
	for (int64_t done = 0; count > 0; count -= 1, done += 1) {
		// the expired timers every 8 turns: woken, they run in this tick
		if (done % 8 == 7 && e->timer_count > 0) {
			fire_timers(e, fa_clock());
		}
		FaTask *task = ready_pop(e, &woken_turns, woken_limit);
		if (task == 0) {
			break;
		}
		turn(e, frame_of(task));
		e->turns += 1;
	}
	int64_t now = e->timer_count > 0 ? fa_clock() : 0;
	if (e->timer_count > 0) {
		fire_timers(e, now);
	}
	drain_remote(e);
	if (e->ready > 0) {
		// the descriptors every 61 turns, or after a yield (a system call)
		if (e->epoll >= 0 && (e->turns >= EVENT_INTERVAL || e->yielded)) {
			e->turns = 0;
			e->yielded = 0;
			poll_events(e, 0);
		}
		return;
	}
	e->turns = 0;
	if (frame_done(e->root) && e->detached == 0 &&
	    __atomic_load_n(&e->sources, __ATOMIC_ACQUIRE) == 0) {
		return;
	}
	if (e->timer_count > 0) {
		int64_t wait = e->timers[0].deadline - now;
		poll_events(e, wait > 0 ? wait : 0);
		return;
	}
	if (__atomic_load_n(&e->sources, __ATOMIC_ACQUIRE) > 0 || e->io_waiting > 0) {
		poll_events(e, -1);
		return;
	}
	deadlock_stop(e);
}

/// Runs the program's top level (`root`, a frame after its ramp) and the
/// tasks it starts, until it ends and no detached task is left; then frees
/// the root frame and the executor. `dev`: warn about long turns.
FA_EXPORT void fa_run_main(void *root, int32_t dev) {
	FaExecutor *e = executor();
	e->root = root;
	e->dev = dev;
	while (!frame_done(root) || e->detached > 0 ||
	       __atomic_load_n(&e->sources, __ATOMIC_ACQUIRE) > 0) {
		tick(e);
	}
	fa_destroy(root);
	if (e->timer_room > 0) {
		fa_free(e->timers, (size_t)e->timer_room * sizeof(FaTimer), 64);
	}
#if !defined(__wasm__)
	if (e->epoll >= 0) {
		close(e->epoll);
	}
	if (e->event >= 0) {
		close(e->event);
	}
#endif
	while (__atomic_load_n(&e->ending, __ATOMIC_SEQ_CST) != 0) {
#if !defined(__wasm__)
		if (sched_yield != 0) {
			sched_yield();
		}
#endif
	}
	current_executor = 0;
	fa_free(e, sizeof(FaExecutor), 16);
}

#if defined(__wasm__)
// ── the event loop of wasm (design/async.md §4, step W1)
//
// wasm has no threads and no way to block: the host's event loop drives the
// executor. `_start` sets the root up (`fa_wasm_start`) and returns; the
// loader then calls `fa_run` (generated, exported: `fa_wasm_run`) from a
// macrotask while tasks are ready (that macrotask is the coop yield), and
// when the timer it was asked for fires (the import `fa.timer`). The timers
// stay in the executor's heap, read with the host's clock, so that a
// cancelled sleep needs nothing from the host (a stale host timer runs a
// batch with nothing to do). `fa_wake` (generated, exported:
// `fa_wasm_wake`) queues the task of a wait the host completed. Once the
// root and the detached tasks are done, `finish` (the end of `main`: flush,
// leak check, exit code) runs.

/// Asks the host to call `fa_run` in `milliseconds` (the latest `token`
/// wins: an earlier timer is stale).
__attribute__((import_module("fa"), import_name("timer"))) void fa_host_timer(int32_t token, double milliseconds);

static void (*wasm_finish)(void);
static int32_t wasm_timer_token;

/// What `fa_wasm_run` gives the host.
enum {
	/// Nothing is ready: a timer was asked for.
	RUN_IDLE = 0,
	/// Tasks are ready: call `fa_run` again from a macrotask.
	RUN_READY = 1,
	/// The program has ended.
	RUN_DONE = 2,
};

/// `_start` of an asynchronous program: the root task (after its ramp) and
/// the end of `main`; the host runs the rest.
FA_EXPORT void fa_wasm_start(void *root, int32_t dev, void (*finish)(void)) {
	FaExecutor *e = executor();
	e->root = root;
	e->dev = dev;
	wasm_finish = finish;
}

/// One batch of the event loop: the expired timers, at most 61 turns, the
/// timers again; then what the host should do.
FA_EXPORT int32_t fa_wasm_run(void) {
	FaExecutor *e = current_executor;
	if (e == 0) {
		return RUN_DONE;
	}
	if (e->timer_count > 0) {
		fire_timers(e, fa_clock());
	}
	int64_t woken_limit;
	int64_t woken_turns = 0;
	int64_t count = tick_turns(e, &woken_limit);
	for (int64_t done = 0; count > 0; count -= 1, done += 1) {
		// the expired timers every 8 turns: woken, they run in this tick
		if (done % 8 == 7 && e->timer_count > 0) {
			fire_timers(e, fa_clock());
		}
		FaTask *task = ready_pop(e, &woken_turns, woken_limit);
		if (task == 0) {
			break;
		}
		turn(e, frame_of(task));
	}
	int64_t now = e->timer_count > 0 ? fa_clock() : 0;
	if (e->timer_count > 0) {
		fire_timers(e, now);
	}
	if (e->ready > 0) {
		return RUN_READY;
	}
	if (frame_done(e->root) && e->detached == 0) {
		fa_destroy(e->root);
		if (e->timer_room > 0) {
			fa_free(e->timers, (size_t)e->timer_room * sizeof(FaWait *), sizeof(void *));
		}
		current_executor = 0;
		fa_free(e, sizeof(FaExecutor), 16);
		wasm_finish();
		return RUN_DONE;
	}
	if (e->timer_count > 0) {
		int64_t wait = e->timers[0].deadline - now;
		wasm_timer_token += 1;
		fa_host_timer(wasm_timer_token, wait > 0 ? (double)wait / 1e6 : 0.0);
		return RUN_IDLE;
	}
	deadlock_stop(e);
}

/// Queues the task of a wait that the host completed (`fa.fetch` and the
/// sockets, later).
FA_EXPORT void fa_wasm_wake(FaWait *wait) {
	wait->status = WAIT_DONE;
	ready_push(current_executor, task_of(wait->frame));
}
#endif

// ── the lines of waiting tasks (mutexes, channels)

/// Puts a wait at the end of a line, for the task of `frame`.
static void line_push(FaWait **line, FaWait *wait, void *frame, const char *site) {
	FaExecutor *e = executor();
	wait->frame = frame;
	wait->executor = e;
	wait->site = site;
	wait->status = WAIT_WAITING;
	wait->remote = 0;
	wait->index = -1;
	if (*line == 0) {
		wait->next = wait;
		wait->prev = wait;
		*line = wait;
	} else {
		FaWait *first = *line;
		wait->next = first;
		wait->prev = first->prev;
		first->prev->next = wait;
		first->prev = wait;
	}
	set_task_state(task_of(frame), TASK_WAITING);
	e->last_site = site;
	if (frame == e->root || e->root == 0) {
		e->root_site = site;
	}
}

static void line_remove(FaWait **line, FaWait *wait) {
	if (wait->next == wait) {
		*line = 0;
	} else {
		wait->prev->next = wait->next;
		wait->next->prev = wait->prev;
		if (*line == wait) {
			*line = wait->next;
		}
	}
	wait->next = 0;
	wait->prev = 0;
}

static FaWait *line_pop(FaWait **line) {
	FaWait *first = *line;
	if (first != 0) {
		line_remove(line, first);
	}
	return first;
}

// ── the blocking pool (files, stdin, DNS: design/async.md §3.3)
//
// A job runs on a thread of the pool (up to 64, started on demand, gone
// after 10 s without work) while its task waits; its end wakes the task
// through the remote queue. A job cancelled while it runs is abandoned: the
// pool thread releases it at its end (its side effects may still happen, A8).
// Without threads (freestanding), jobs run at once.

typedef struct FaJob {
	/// Runs on a thread of the pool.
	void (*run)(struct FaJob *job);
	/// Frees the job and what it owns: called by the side that ends last.
	void (*release)(struct FaJob *job);
	FaWait *wait;
	FaExecutor *executor;
	struct FaJob *next;
	int32_t state;
} FaJob;

enum { JOB_QUEUED, JOB_RUNNING, JOB_DONE, JOB_ABANDONED };

#if !defined(__wasm__)

#define POOL_THREADS 64
#define POOL_IDLE_SECONDS 10

typedef struct {
	int64_t seconds;
	int64_t nanoseconds;
} FaTimespec;

__attribute__((weak)) int pthread_cond_timedwait(FaSystemLock *condition, FaSystemLock *mutex,
                                                 const FaTimespec *deadline);
__attribute__((weak)) int clock_gettime(int clock, FaTimespec *time);
__attribute__((weak)) int pthread_detach(unsigned long thread);

static struct {
	FaSystemLock lock;
	FaSystemLock wake;
	FaJob *head;
	FaJob *tail;
	int32_t threads;
	int32_t idle;
} blocking;

static void *blocking_thread(void *unused) {
	(void)unused;
	system_lock(&blocking.lock);
	for (;;) {
		while (blocking.head == 0) {
			blocking.idle += 1;
			int timed_out = 0;
			if (pthread_cond_timedwait != 0 && clock_gettime != 0) {
				FaTimespec deadline;
				clock_gettime(0 /* CLOCK_REALTIME */, &deadline);
				deadline.seconds += POOL_IDLE_SECONDS;
				timed_out = pthread_cond_timedwait(&blocking.wake, &blocking.lock, &deadline) != 0;
			} else {
				pthread_cond_wait(&blocking.wake, &blocking.lock);
			}
			blocking.idle -= 1;
			if (timed_out && blocking.head == 0) {
				blocking.threads -= 1;
				system_unlock(&blocking.lock);
				return 0;
			}
		}
		FaJob *job = blocking.head;
		blocking.head = job->next;
		if (blocking.head == 0) {
			blocking.tail = 0;
		}
		job->state = JOB_RUNNING;
		system_unlock(&blocking.lock);
		job->run(job);
		system_lock(&blocking.lock);
		FaExecutor *e = job->executor;
		if (job->state == JOB_ABANDONED) {
			system_unlock(&blocking.lock);
			job->release(job);
		} else {
			job->state = JOB_DONE;
			job->wait->status = WAIT_DONE;
			wake(job->wait);
			system_unlock(&blocking.lock);
		}
		fa_remote_source_end(e);
		system_lock(&blocking.lock);
	}
}

/// Runs a job on the pool for the task of `frame`, which waits in `wait`:
/// `AWAIT_SUSPEND`, or `AWAIT_READY` when it ran at once (no threads).
FA_EXPORT int32_t fa_blocking_submit(FaJob *job, FaWait *wait, void *frame) {
	if (!has_threads()) {
		job->run(job);
		job->state = JOB_DONE;
		wait->status = WAIT_DONE;
		return AWAIT_READY;
	}
	FaExecutor *e = executor();
	wait->frame = frame;
	wait->executor = e;
	wait->status = WAIT_WAITING;
	wait->remote = 0;
	wait->index = -1;
	job->wait = wait;
	job->executor = e;
	job->state = JOB_QUEUED;
	job->next = 0;
	fa_remote_source_start(e);
	set_task_state(task_of(frame), TASK_WAITING);
	system_lock(&blocking.lock);
	if (blocking.tail != 0) {
		blocking.tail->next = job;
	} else {
		blocking.head = job;
	}
	blocking.tail = job;
	if (blocking.idle > 0) {
		pthread_cond_signal(&blocking.wake);
	} else if (blocking.threads < POOL_THREADS) {
		blocking.threads += 1;
		unsigned long thread;
		pthread_create(&thread, 0, blocking_thread, 0);
		if (pthread_detach != 0) {
			pthread_detach(thread);
		}
	}
	system_unlock(&blocking.lock);
	return AWAIT_SUSPEND;
}

/// The destroy edge of a job's wait: a queued job is removed and released,
/// a running one abandoned (the pool thread releases it), a done one
/// released (its wake forgotten).
FA_EXPORT void fa_blocking_cancel(FaJob *job) {
	FaExecutor *e = job->executor;
	system_lock(&blocking.lock);
	if (job->state == JOB_QUEUED) {
		FaJob **link = &blocking.head;
		FaJob *previous = 0;
		while (*link != job) {
			previous = *link;
			link = &(*link)->next;
		}
		*link = job->next;
		if (blocking.tail == job) {
			blocking.tail = previous;
		}
		system_unlock(&blocking.lock);
		job->release(job);
		fa_remote_source_end(e);
		return;
	}
	if (job->state == JOB_RUNNING) {
		job->state = JOB_ABANDONED;
		system_unlock(&blocking.lock);
		return;
	}
	system_unlock(&blocking.lock);
	forget_remote_wake(job->wait);
	job->release(job);
}

#else

FA_EXPORT int32_t fa_blocking_submit(FaJob *job, FaWait *wait, void *frame) {
	(void)frame;
	job->run(job);
	job->state = JOB_DONE;
	wait->status = WAIT_DONE;
	return AWAIT_READY;
}

FA_EXPORT void fa_blocking_cancel(FaJob *job) {
	job->release(job);
}

#endif

// ── sockets and files (std.net, std.fs: design/async.md §5, `src/lower/io.rs`)
//
// - **A socket** (`TcpListener`, `TcpStream`) is a block of the handles
//   (`FaShared`: copied by `fa_shared_retain`, though the analysis keeps it
//   move-only) holding a non-blocking descriptor, closed with the last
//   handle (or by `close()`). A listener listens as soon as it is built; a
//   failure is kept and thrown by its first `accept()`.
// - **Blocking** operations (synchronous instances) try the system call and
//   wait with `poll` on `EAGAIN`. **Awaited** ones try it, and on `EAGAIN`
//   the task waits for epoll (`fa_io_wait`; the descriptor is registered
//   with the executor of the thread at its first await): the generated code
//   calls the operation again when the task is resumed. An operation that
//   ends at once spends a unit of the coop budget (`AWAIT_YIELD` when it is
//   spent). Errors are `NetError { message }`.
// - **Files** run in place when blocking, and on the blocking pool when
//   awaited (`FaFileJob`: the job owns copies of its inputs and its result;
//   the error is built on the task's thread). Errors are `FileError { message, path }`.
// - Hosts are IPv4 addresses and `localhost` (no name resolution yet).

// the ids of built-in errors: their position in `ERRORS` of the analyzer's
// `builtins/errors.rs` plus one (checked by `builtin_error_ids_match_the_runtime`)
#define WORKER_TIMEOUT 8
#define FILE_ERROR 9
#define NET_ERROR 11

#define E_INTR 4
#define E_IO 5
#define E_BADF 9
#define E_AGAIN 11
#define E_INVAL 22
#define E_NOSYS 38
#define E_CONNABORTED 103
#define E_INPROGRESS 115
#define POLL_IN 1
#define POLL_OUT 4

typedef struct {
	int32_t fd;
	int16_t events;
	int16_t revents;
} FaPollFd;

typedef struct {
	FaShared shared;
	int32_t fd;
	/// The error of a listener that couldn't listen (`errno`; -1: an unknown host).
	int32_t error;
	int64_t port;
	FaIo *io;
} FaSocket;

/// Error payloads use the same name-sorted layout as ordinary Fa objects.
typedef struct {
	int64_t id;
	FaString message;
	FaString path;
} FileErrorBox;

#if !defined(__wasm__)
__attribute__((weak)) int socket(int domain, int type, int protocol);
__attribute__((weak)) int bind(int fd, const void *address, uint32_t length);
__attribute__((weak)) int listen(int fd, int backlog);
__attribute__((weak)) int accept4(int fd, void *address, uint32_t *length, int flags);
__attribute__((weak)) int connect(int fd, const void *address, uint32_t length);
__attribute__((weak)) int getsockname(int fd, void *address, uint32_t *length);
__attribute__((weak)) int setsockopt(int fd, int level, int name, const void *value, uint32_t length);
__attribute__((weak)) int getsockopt(int fd, int level, int name, void *value, uint32_t *length);
__attribute__((weak)) long send(int fd, const void *bytes, size_t count, int flags);
__attribute__((weak)) int poll(FaPollFd *fds, unsigned long count, int timeout);
__attribute__((weak)) int *__errno_location(void);
__attribute__((weak)) char *strerror(int code);
__attribute__((weak)) int open(const char *path, int flags, ...);

static int last_error(void) {
	return __errno_location != 0 ? *__errno_location() : E_IO;
}

static int network_available(void) {
	return socket != 0 && poll != 0;
}

/// Waits (blocking) until a descriptor is readable or writable.
static void wait_fd(int fd, int16_t events) {
	FaPollFd entry = {fd, events, 0};
	poll(&entry, 1, -1);
}
#endif

/// The text of an error code.
static void append_error(int code) {
#if !defined(__wasm__)
	if (code == -1) {
		append_c("unknown host (an IPv4 address or localhost)");
		return;
	}
	if (code == E_NOSYS || code == E_BADF) {
		append_c(code == E_NOSYS ? "no network on this target" : "the socket is closed");
		return;
	}
	if (strerror != 0) {
		append_c(strerror(code));
		return;
	}
#endif
	append_c("error ");
	fa_b_i64(code);
}

/// `NetError { message = "<what>: <the error's text>" }`.
static const void *net_error(int code, const char *what) {
	fa_b_reset();
	append_c(what);
	append_c(": ");
	append_error(code);
	MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
	error->id = NET_ERROR;
	fa_b_take(&error->message);
	return error;
}

/// The code of a file operation whose error has a text of the host (wasm).
#define E_HOST (-2)

/// `FileError { message, path }`: the text of `code`, or `text` (`E_HOST`).
static const void *file_error(int code, const uint8_t *text, size_t text_length,
                              const uint8_t *path, size_t length) {
	FileErrorBox *error = fa_alloc(sizeof(FileErrorBox), sizeof(int64_t));
	error->id = FILE_ERROR;
	fa_b_reset();
	if (code == E_HOST) {
		fa_b_bytes(text, text_length);
	} else {
		append_error(code);
	}
	fa_b_take(&error->message);
	error->path = owned_copy(path, length);
	return error;
}

#if !defined(__wasm__)
/// The IPv4 address of a host (network order): a dotted address, or
/// `localhost`; 0 when it is neither.
static int parse_host(const uint8_t *host, size_t length, uint32_t *address) {
	static const char local[] = "localhost";
	if (length == sizeof(local) - 1) {
		int same = 1;
		for (size_t index = 0; index < length; index += 1) {
			same &= host[index] == (uint8_t)local[index];
		}
		if (same) {
			*address = 0x0100007Fu;
			return 1;
		}
	}
	uint32_t parts[4] = {0, 0, 0, 0};
	size_t part = 0;
	size_t digits = 0;
	for (size_t index = 0; index < length; index += 1) {
		uint8_t byte = host[index];
		if (byte == '.') {
			if (digits == 0 || part == 3) {
				return 0;
			}
			part += 1;
			digits = 0;
		} else if (byte >= '0' && byte <= '9' && digits < 3) {
			parts[part] = parts[part] * 10 + (byte - '0');
			digits += 1;
		} else {
			return 0;
		}
	}
	if (part != 3 || digits == 0 || parts[0] > 255 || parts[1] > 255 || parts[2] > 255 ||
	    parts[3] > 255) {
		return 0;
	}
	*address = parts[0] | parts[1] << 8 | parts[2] << 16 | parts[3] << 24;
	return 1;
}

typedef struct {
	uint16_t family;
	uint16_t port;
	uint32_t address;
	uint8_t zero[8];
} FaSocketAddress;

static FaSocketAddress socket_address(uint32_t address, int64_t port) {
	FaSocketAddress result;
	__builtin_memset(&result, 0, sizeof result);
	result.family = 2; // AF_INET
	result.port = (uint16_t)(((port & 0xFF) << 8) | ((port >> 8) & 0xFF));
	result.address = address;
	return result;
}
#endif

static void socket_close(FaSocket *socket_block) {
	FaIo *io = socket_block->io;
	if (io != 0) {
		socket_block->io = 0;
		if (io->executor == current_executor) {
			fa_io_close(io);
		} else {
			// closing the descriptor removes it from that epoll
			fa_free(io, sizeof(FaIo), 16);
		}
	}
#if !defined(__wasm__)
	if (socket_block->fd >= 0) {
		close(socket_block->fd);
	}
#endif
	socket_block->fd = -1;
}

static void socket_drop(void *block) {
	socket_close(block);
}

static FaSocket *socket_new(int32_t fd) {
	FaSocket *socket_block = shared_new(sizeof(FaSocket), socket_drop);
	socket_block->fd = fd;
	return socket_block;
}

/// `close()` of a socket: its descriptor is closed now, its block with its handle.
FA_EXPORT void fa_tcp_close(FaSocket *socket_block) {
	socket_close(socket_block);
}

/// `TcpListener { host, port }`: listens at once (a failure is kept).
FA_EXPORT FaSocket *fa_tcp_listen(const uint8_t *host, size_t length, int64_t port) {
	FaSocket *listener = socket_new(-1);
#if !defined(__wasm__)
	uint32_t address = 0;
	if (!network_available()) {
		listener->error = E_NOSYS;
		return listener;
	}
	if (length > 0 && !parse_host(host, length, &address)) {
		listener->error = -1;
		return listener;
	}
	if (port < 0 || port > 65535) {
		listener->error = E_INVAL;
		return listener;
	}
	// SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC
	int fd = socket(2, 1 | 04000 | 02000000, 0);
	if (fd < 0) {
		listener->error = last_error();
		return listener;
	}
	int one = 1;
	setsockopt(fd, 1 /* SOL_SOCKET */, 2 /* SO_REUSEADDR */, &one, sizeof one);
	FaSocketAddress bound = socket_address(address, port);
	if (bind(fd, &bound, sizeof bound) != 0 || listen(fd, 4096) != 0) {
		listener->error = last_error();
		close(fd);
		return listener;
	}
	uint32_t size = sizeof bound;
	if (getsockname(fd, &bound, &size) == 0) {
		port = (int64_t)(((bound.port & 0xFF) << 8) | (bound.port >> 8));
	}
	listener->fd = fd;
	listener->port = port;
#else
	(void)host;
	(void)length;
	(void)port;
	listener->error = E_NOSYS;
#endif
	return listener;
}

/// `listener.port`.
FA_EXPORT int64_t fa_tcp_port(const FaSocket *listener) {
	return listener->port;
}

/// The registration of a socket with the executor of this thread.
static FaIo *socket_io(FaSocket *socket_block) {
	FaExecutor *e = executor();
	if (socket_block->io != 0 && socket_block->io->executor == e) {
		return socket_block->io;
	}
	if (socket_block->io != 0) {
		fa_free(socket_block->io, sizeof(FaIo), 16);
	}
	socket_block->io = fa_io_register(socket_block->fd);
	return socket_block->io;
}

/// An operation that ended at once: it spends a unit of the coop budget.
static int32_t io_done(void *frame) {
	if (coop_spent()) {
		yield_push(executor(), task_of(frame));
		return AWAIT_YIELD;
	}
	return AWAIT_READY;
}

/// After `EAGAIN` in a task: it waits for the descriptor (`AWAIT_SUSPEND`),
/// or tries again (`AWAIT_READY`). Without epoll, it waits blocking.
static int32_t io_wait(FaSocket *socket_block, int32_t writable, FaWait *wait, void *frame) {
	FaIo *io = socket_io(socket_block);
#if !defined(__wasm__)
	if (io == 0) {
		wait_fd(socket_block->fd, writable ? POLL_OUT : POLL_IN);
		return AWAIT_READY;
	}
#endif
	if (io == 0) {
		return AWAIT_READY;
	}
	FaWait *taken = writable ? io->writer : io->reader;
	if (taken != 0 && taken != wait) {
		// another task waits on this side: try again after the others
		wait->frame = frame;
		wait->executor = io->executor;
		yield_push(io->executor, task_of(frame));
		return AWAIT_SUSPEND;
	}
	return fa_io_wait(io, writable, wait, frame);
}

/// The destroy edge of an awaited operation on a socket: its wait is
/// cancelled, and the socket it made (`out`: accepted, being connected) closed.
FA_EXPORT void fa_tcp_cancel(FaSocket *socket_block, FaWait *wait, FaSocket **out) {
	if (socket_block != 0 && socket_block->io != 0) {
		fa_io_cancel(socket_block->io, wait);
	}
	if (out != 0 && *out != 0) {
		if ((*out)->io != 0) {
			fa_io_cancel((*out)->io, wait);
		}
		fa_shared_release(*out);
		*out = 0;
	}
}

/// One try: 0 done, 1 `EAGAIN`, 2 failed (`*error`).
static int try_accept(FaSocket *listener, FaSocket **out, const void **error) {
	if (listener->fd < 0) {
		*error = net_error(listener->error == 0 ? E_BADF : listener->error, "can't listen");
		return 2;
	}
#if !defined(__wasm__)
	for (;;) {
		int fd = accept4(listener->fd, 0, 0, 04000 | 02000000);
		if (fd >= 0) {
			int one = 1;
			setsockopt(fd, 6 /* IPPROTO_TCP */, 1 /* TCP_NODELAY */, &one, sizeof one);
			*out = socket_new(fd);
			return 0;
		}
		int code = last_error();
		if (code == E_INTR || code == E_CONNABORTED) {
			continue;
		}
		if (code == E_AGAIN) {
			return 1;
		}
		*error = net_error(code, "accept");
		return 2;
	}
#else
	(void)out;
	*error = net_error(E_NOSYS, "accept");
	return 2;
#endif
}

/// `listener.accept()` blocking: the error, or null (`*out` the stream).
FA_EXPORT const void *fa_tcp_accept(FaSocket *listener, FaSocket **out) {
	const void *error = 0;
	while (try_accept(listener, out, &error) == 1) {
#if !defined(__wasm__)
		wait_fd(listener->fd, POLL_IN);
#endif
	}
	return error;
}

/// `await listener.accept()` (`AWAIT_*`; `*out` or `*error` once ready).
FA_EXPORT int32_t fa_tcp_accept_async(FaSocket *listener, FaWait *wait, void *frame,
                                      FaSocket **out, const void **error) {
	for (;;) {
		int result = try_accept(listener, out, error);
		if (result == 0) {
			return io_done(frame);
		}
		if (result == 2) {
			return AWAIT_READY;
		}
		if (io_wait(listener, 0, wait, frame) == AWAIT_SUSPEND) {
			return AWAIT_SUSPEND;
		}
	}
}

/// The first step of a connection: 0 connected, 1 in progress, 2 failed.
static int connect_start(const uint8_t *host, size_t length, int64_t port, FaSocket **out,
                         const void **error) {
#if !defined(__wasm__)
	uint32_t address;
	if (!network_available()) {
		*error = net_error(E_NOSYS, "connect");
		return 2;
	}
	if (!parse_host(host, length, &address)) {
		*error = net_error(-1, "connect");
		return 2;
	}
	if (port < 0 || port > 65535) {
		*error = net_error(E_INVAL, "connect");
		return 2;
	}
	int fd = socket(2, 1 | 04000 | 02000000, 0);
	if (fd < 0) {
		*error = net_error(last_error(), "connect");
		return 2;
	}
	int one = 1;
	setsockopt(fd, 6 /* IPPROTO_TCP */, 1 /* TCP_NODELAY */, &one, sizeof one);
	FaSocketAddress target = socket_address(address, port);
	int result;
	do {
		result = connect(fd, &target, sizeof target);
	} while (result != 0 && last_error() == E_INTR);
	*out = socket_new(fd);
	if (result == 0) {
		return 0;
	}
	int code = last_error();
	if (code == E_INPROGRESS || code == E_AGAIN) {
		return 1;
	}
	fa_shared_release(*out);
	*out = 0;
	*error = net_error(code, "connect");
	return 2;
#else
	(void)host;
	(void)length;
	(void)port;
	(void)out;
	*error = net_error(E_NOSYS, "connect");
	return 2;
#endif
}

/// A connection in progress: 0 connected, 1 still in progress, 2 failed.
static int connect_check(FaSocket **out, const void **error) {
#if !defined(__wasm__)
	FaPollFd entry = {(*out)->fd, POLL_OUT, 0};
	if (poll(&entry, 1, 0) == 0) {
		return 1;
	}
	int code = 0;
	uint32_t size = sizeof code;
	getsockopt((*out)->fd, 1 /* SOL_SOCKET */, 4 /* SO_ERROR */, &code, &size);
	if (code == 0) {
		return 0;
	}
	fa_shared_release(*out);
	*out = 0;
	*error = net_error(code, "connect");
#else
	(void)out;
	*error = net_error(E_NOSYS, "connect");
#endif
	return 2;
}

/// `TcpStream.connect(host, port)` blocking.
FA_EXPORT const void *fa_tcp_connect(const uint8_t *host, size_t length, int64_t port,
                                     FaSocket **out) {
	const void *error = 0;
	*out = 0;
	if (connect_start(host, length, port, out, &error) == 1) {
		while (connect_check(out, &error) == 1) {
#if !defined(__wasm__)
			wait_fd((*out)->fd, POLL_OUT);
#endif
		}
	}
	return error;
}

/// `await TcpStream.connect(host, port)`: `*out` is null at the first call,
/// the socket being connected at the next ones.
FA_EXPORT int32_t fa_tcp_connect_async(const uint8_t *host, size_t length, int64_t port,
                                       FaWait *wait, void *frame, FaSocket **out,
                                       const void **error) {
	int state = *out == 0 ? connect_start(host, length, port, out, error) : 1;
	for (;;) {
		if (state == 0) {
			return io_done(frame);
		}
		if (state == 2) {
			return AWAIT_READY;
		}
		state = connect_check(out, error);
		if (state == 1 && io_wait(*out, 1, wait, frame) == AWAIT_SUSPEND) {
			return AWAIT_SUSPEND;
		}
	}
}

/// One read: 0 done (`*out` bytes, 0 at the end), 1 `EAGAIN`, 2 failed.
static int try_read(FaSocket *stream, FaArrayHeader *buffer, int64_t *out, const void **error) {
	if (stream->fd < 0) {
		*error = net_error(E_BADF, "read");
		return 2;
	}
#if !defined(__wasm__)
	for (;;) {
		long count = read(stream->fd, buffer->items, (size_t)buffer->length);
		if (count >= 0) {
			*out = count;
			return 0;
		}
		int code = last_error();
		if (code == E_INTR) {
			continue;
		}
		if (code == E_AGAIN) {
			return 1;
		}
		*error = net_error(code, "read");
		return 2;
	}
#else
	(void)buffer;
	(void)out;
	*error = net_error(E_NOSYS, "read");
	return 2;
#endif
}

/// `stream.read(buffer)` blocking.
FA_EXPORT const void *fa_tcp_read(FaSocket *stream, FaArrayHeader *buffer, int64_t *out) {
	const void *error = 0;
	while (try_read(stream, buffer, out, &error) == 1) {
#if !defined(__wasm__)
		wait_fd(stream->fd, POLL_IN);
#endif
	}
	return error;
}

/// `await stream.read(buffer)`.
FA_EXPORT int32_t fa_tcp_read_async(FaSocket *stream, FaArrayHeader *buffer, FaWait *wait,
                                    void *frame, int64_t *out, const void **error) {
	for (;;) {
		int result = try_read(stream, buffer, out, error);
		if (result == 0) {
			return io_done(frame);
		}
		if (result == 2) {
			return AWAIT_READY;
		}
		if (io_wait(stream, 0, wait, frame) == AWAIT_SUSPEND) {
			return AWAIT_SUSPEND;
		}
	}
}

/// Writes from `*written` to `count`: 0 all written, 1 `EAGAIN`, 2 failed.
static int try_write(FaSocket *stream, const FaArrayHeader *bytes, int64_t count,
                     int64_t *written, const void **error) {
	if (stream->fd < 0) {
		*error = net_error(E_BADF, "write");
		return 2;
	}
	if (count < 0 || count > bytes->length) {
		count = bytes->length;
	}
#if !defined(__wasm__)
	while (*written < count) {
		// MSG_NOSIGNAL: a closed connection is an error, not a signal
		long sent = send(stream->fd, bytes->items + *written, (size_t)(count - *written), 0x4000);
		if (sent >= 0) {
			*written += sent;
			continue;
		}
		int code = last_error();
		if (code == E_INTR) {
			continue;
		}
		if (code == E_AGAIN) {
			return 1;
		}
		*error = net_error(code, "write");
		return 2;
	}
	return 0;
#else
	(void)written;
	*error = net_error(E_NOSYS, "write");
	return 2;
#endif
}

/// `stream.write(bytes, count)` blocking (`count` -1: every byte).
FA_EXPORT const void *fa_tcp_write(FaSocket *stream, const FaArrayHeader *bytes, int64_t count) {
	const void *error = 0;
	int64_t written = 0;
	while (try_write(stream, bytes, count, &written, &error) == 1) {
#if !defined(__wasm__)
		wait_fd(stream->fd, POLL_OUT);
#endif
	}
	return error;
}

/// `await stream.write(bytes, count)`: `*written` counts the bytes written
/// by the previous calls (0 at the first one).
FA_EXPORT int32_t fa_tcp_write_async(FaSocket *stream, const FaArrayHeader *bytes, int64_t count,
                                     FaWait *wait, void *frame, int64_t *written,
                                     const void **error) {
	for (;;) {
		int result = try_write(stream, bytes, count, written, error);
		if (result == 0) {
			return io_done(frame);
		}
		if (result == 2) {
			return AWAIT_READY;
		}
		if (io_wait(stream, 1, wait, frame) == AWAIT_SUSPEND) {
			return AWAIT_SUSPEND;
		}
	}
}

// ── files

enum { FILE_READ = 0, FILE_READ_BYTES = 1, FILE_WRITE = 2, FILE_WRITE_BYTES = 3 };

/// A file operation, run in place or on the blocking pool.
typedef struct {
	FaJob job;
	int32_t kind;
	/// The error (`errno`), 0 when it worked.
	int32_t code;
	char *path;
	size_t path_length;
	/// What was read, or the bytes to write (owned).
	uint8_t *bytes;
	size_t length;
	size_t room;
} FaFileJob;

static void file_buffer_free(FaFileJob *file) {
	if (!file->room) { return; }
	if (file->kind < FILE_WRITE) {
		fa_free(string_header(file->bytes), sizeof(StringHeader) + file->room, sizeof(size_t));
	} else { fa_free(file->bytes, file->room, 16); }
}

static uint8_t *file_buffer_allocate(FaFileJob *file, size_t room) {
	if (file->kind >= FILE_WRITE) { return fa_alloc(room, 16); }
	if (room > SIZE_MAX - sizeof(StringHeader)) { out_of_memory(); }
	return (uint8_t *)fa_alloc(sizeof(StringHeader) + room, sizeof(size_t)) + sizeof(StringHeader);
}

static void file_job_run(FaJob *job) {
	FaFileJob *file = (FaFileJob *)job;
#if !defined(__wasm__)
	if (open == 0) {
		file->code = E_NOSYS;
		return;
	}
	if (file->kind >= FILE_WRITE) {
		// O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC
		int fd = open(file->path, 01 | 0100 | 01000 | 02000000, 0666);
		if (fd < 0) {
			file->code = last_error();
			return;
		}
		size_t done = 0;
		while (done < file->length) {
			long count = write(fd, file->bytes + done, file->length - done);
			if (count < 0) {
				if (last_error() == E_INTR) {
					continue;
				}
				file->code = last_error();
				break;
			}
			done += (size_t)count;
		}
		if (close(fd) != 0 && file->code == 0) {
			file->code = last_error();
		}
		return;
	}
	int fd = open(file->path, 02000000 /* O_RDONLY | O_CLOEXEC */);
	if (fd < 0) {
		file->code = last_error();
		return;
	}
	for (;;) {
		if (file->length == file->room) {
			if (file->room > (SIZE_MAX - sizeof(StringHeader)) / 2) { out_of_memory(); }
			size_t room = file->room < 4096 ? 4096 : file->room * 2;
			uint8_t *bytes = file_buffer_allocate(file, room);
			if (file->room > 0) {
				copy_bytes(bytes, file->bytes, file->length);
				file_buffer_free(file);
			}
			file->bytes = bytes;
			file->room = room;
		}
		long count = read(fd, file->bytes + file->length, file->room - file->length);
		if (count < 0) {
			if (last_error() == E_INTR) {
				continue;
			}
			file->code = last_error();
			break;
		}
		if (count == 0) {
			break;
		}
		file->length += (size_t)count;
	}
	close(fd);
#else
	// the host's `node:fs`; what was read (or the error's text) replaces
	// the bytes to write
	int32_t size = 0;
	int32_t failed = fa_host_file_run(file->kind >= FILE_WRITE ? FILE_WRITE : file->kind, (const uint8_t *)file->path, file->path_length,
	                                  file->bytes, file->length, &size);
	file_buffer_free(file);
	file->bytes = 0;
	file->length = 0;
	file->room = 0;
	if (size > 0) {
		file->room = file->kind < FILE_WRITE ? string_block((size_t)size) : (size_t)size;
		file->bytes = file_buffer_allocate(file, file->room);
		fa_host_file_take(file->bytes);
		file->length = (size_t)size;
	}
	if (failed) {
		file->code = E_HOST;
	}
#endif
}

static void file_job_release(FaJob *job) {
	FaFileJob *file = (FaFileJob *)job;
	fa_free(file->path, file->path_length + 1, 1);
	file_buffer_free(file);
	fa_free(file, sizeof(FaFileJob), 16);
}

/// A file operation on `path` (copied, and the bytes to write).
FA_EXPORT FaFileJob *fa_file_job(int32_t kind, const uint8_t *path, size_t length,
                                 const uint8_t *bytes, size_t byte_count) {
	FaFileJob *file = fa_alloc(sizeof(FaFileJob), 16);
	__builtin_memset(file, 0, sizeof(FaFileJob));
	file->job.run = file_job_run;
	file->job.release = file_job_release;
	file->kind = kind;
	size_t path_length = fa_string_encoded_length(path, length, 0);
	file->path = fa_alloc(fa_string_size(path_length, 1), 1);
	fa_string_write_utf8((uint8_t *)file->path, path, length, 0);
	file->path[path_length] = 0;
	file->path_length = path_length;
	if (kind == FILE_WRITE && byte_count > 0) {
		size_t encoded = fa_string_encoded_length(bytes, byte_count, 0);
		file->bytes = fa_alloc(encoded, 16);
		fa_string_write_utf8(file->bytes, bytes, byte_count, 0);
		file->length = encoded;
		file->room = encoded;
	} else if (kind == FILE_WRITE_BYTES && byte_count > 0) {
		file->bytes = fa_alloc(byte_count, 16);
		copy_bytes(file->bytes, bytes, byte_count);
		file->length = file->room = byte_count;
	}
	return file;
}

/// Runs a file operation in place (blocking).
FA_EXPORT void fa_file_run(FaFileJob *file) {
	file_job_run(&file->job);
}

/// Runs a file operation on the blocking pool for the task of `frame`.
FA_EXPORT int32_t fa_file_submit(FaFileJob *file, FaWait *wait, void *frame) {
	return fa_blocking_submit(&file->job, wait, frame);
}

/// The destroy edge of an awaited file operation.
FA_EXPORT void fa_file_cancel(FaFileJob *file) {
	fa_blocking_cancel(&file->job);
}

/// The end of a file operation: its error, or null and its result in `out`
/// (a string for `read`, an array of bytes for `readBytes`); frees the job.
FA_EXPORT const void *fa_file_finish(FaFileJob *file, void *out) {
	const void *error = 0;
	if (file->code != 0) {
		error = file_error(file->code, file->bytes, file->length, (const uint8_t *)file->path,
		                   file->path_length);
	} else if (file->kind == FILE_READ) {
		Utf8Info info = utf8_validate(file->bytes, file->length);
		if (info.error != SIZE_MAX) {
			static const uint8_t message[] = "invalid UTF-8";
			error = file_error(E_HOST, message, sizeof message - 1,
			                   (const uint8_t *)file->path, file->path_length);
		} else if (file->length <= INLINE_STRING_BYTES || file->room >= KIND_MASK) {
			utf8_copy_valid(file->bytes, file->length, info.units, (FaString *)out);
		} else {
			size_t flags = info.units == file->length ? SIMPLE : 0;
			if (!flags) { string_init_header(string_header(file->bytes)); }
			*(FaString *)out = (FaString){file->bytes, file->length, file->room | flags};
			fa_string_set_unit_count(file->bytes, file->length, file->room | flags, info.units);
			file->bytes = 0;
			file->room = 0;
		}
	} else if (file->kind == FILE_READ_BYTES) {
		*(void **)out = file->room ? (void *)bytes_adopt_string(file->bytes, file->length, file->room)
		                          : fa_bytes_filled(0, 0);
		file->bytes = 0;
		file->room = 0;
	}
	file_job_release(&file->job);
	return error;
}

// ── JSON (`std.json`, `src/lower/json.rs`)
//
// `Json.parse(text)` reads the text into a tree of `FaJson` nodes (an arena
// that `fa_json_free` frees), then checks the tree against the schema of
// the expected type, which the compiler writes, with the messages of
// `faJsonParse` of the JavaScript runtime; the generated code then builds
// the value from the tree, which fits. `Json.stringify` is generated code
// appending to the text builder (`fa_b_json_string`, `fa_b_json_number`).
//
// A schema, one letter per type:
// - `b` a boolean, `s` a string, `f` a number, `n` null;
// - `i<min>;<max>;` an integer (a bound is empty when absent);
// - `S<length>;<bytes>`, `T`, `F`, `I<value>;` a literal;
// - `a<item>` an array, `m<value>` an object read as a map;
// - `o<count>;` an object, then `<length>;<name><0|1: optional><schema>`
//   per field, in declaration order;
// - `u<count>;` an anonymous union, then its members: the first that fits.
//
// The members of an object are in the order of JavaScript's
// `Object.entries`: the array indexes first, ascending, then the other keys.

#define JSON_ERROR 10
#define JSON_MAX_DEPTH 1000

enum { JSON_NULL, JSON_BOOLEAN, JSON_NUMBER, JSON_STRING, JSON_ARRAY, JSON_OBJECT };

/// A number is `JSON_INTEGRAL` when its value is an integer, and also
/// `JSON_FITS` when `integer` holds it.
#define JSON_INTEGRAL 1
#define JSON_FITS 2

typedef struct FaJson FaJson;
struct FaJson {
	int32_t kind;
	/// A boolean's value; a number's `JSON_INTEGRAL` and `JSON_FITS`.
	int32_t flags;
	int64_t integer;
	double number;
	/// A string's bytes, unescaped (in the text or in the arena).
	const uint8_t *bytes;
	size_t length;
	/// The key of a member of an object.
	const uint8_t *key;
	size_t key_length;
	/// The items of an array, the members of an object.
	FaJson *items;
	size_t count;
};

typedef struct JsonChunk {
	struct JsonChunk *next;
	size_t size;
} JsonChunk;

#define JSON_CHUNK_HEADER ((sizeof(JsonChunk) + 15) & ~(size_t)15)
#define JSON_CHUNK 65536

/// A parsed text: its root, then the arena of its nodes.
typedef struct {
	FaJson root;
	JsonChunk *chunks;
	uint8_t *free;
	size_t room;
} FaJsonDocument;

static void *json_alloc(FaJsonDocument *document, size_t size) {
	size = (size + 15) & ~(size_t)15;
	if (size > document->room) {
		size_t chunk = size + JSON_CHUNK_HEADER > JSON_CHUNK ? size + JSON_CHUNK_HEADER : JSON_CHUNK;
		JsonChunk *block = fa_alloc(chunk, 16);
		block->next = document->chunks;
		block->size = chunk;
		document->chunks = block;
		document->free = (uint8_t *)block + JSON_CHUNK_HEADER;
		document->room = chunk - JSON_CHUNK_HEADER;
	}
	void *bytes = document->free;
	document->free += size;
	document->room -= size;
	return bytes;
}

/// Frees a parsed text.
FA_EXPORT void fa_json_free(FaJsonDocument *document) {
	JsonChunk *chunk = document->chunks;
	while (chunk != 0) {
		JsonChunk *next = chunk->next;
		fa_free(chunk, chunk->size, 16);
		chunk = next;
	}
	fa_free(document, sizeof(FaJsonDocument), 16);
}

typedef struct {
	const uint8_t *text;
	size_t length;
	size_t at;
	FaJsonDocument *document;
	/// The items of the arrays and objects being read, copied to the arena
	/// once complete.
	FaJson *stack;
	size_t stack_length;
	size_t stack_room;
	int depth;
	/// The error: its text, then a detail (quoted by `quote`, unless 0).
	const char *error;
	const uint8_t *detail;
	size_t detail_length;
	uint8_t quote;
} JsonReader;

static int json_fail(JsonReader *reader, const char *error, const uint8_t *detail,
                     size_t detail_length, uint8_t quote) {
	reader->error = error;
	reader->detail = detail;
	reader->detail_length = detail_length;
	reader->quote = quote;
	return 0;
}

static void json_space(JsonReader *reader) {
	while (reader->at < reader->length) {
		uint8_t byte = reader->text[reader->at];
		if (byte != ' ' && byte != '\t' && byte != '\n' && byte != '\r') {
			return;
		}
		reader->at += 1;
	}
}

static void json_push(JsonReader *reader, const FaJson *node) {
	if (reader->stack_length == reader->stack_room) {
		size_t room = reader->stack_room < 64 ? 64 : reader->stack_room * 2;
		FaJson *stack = fa_alloc(room * sizeof(FaJson), 16);
		if (reader->stack_room > 0) {
			copy_bytes((uint8_t *)stack, (const uint8_t *)reader->stack,
			           reader->stack_length * sizeof(FaJson));
			fa_free(reader->stack, reader->stack_room * sizeof(FaJson), 16);
		}
		reader->stack = stack;
		reader->stack_room = room;
	}
	reader->stack[reader->stack_length] = *node;
	reader->stack_length += 1;
}

/// The items pushed since `base`, moved to the arena.
static void json_collect(JsonReader *reader, size_t base, FaJson *out) {
	size_t count = reader->stack_length - base;
	out->count = count;
	out->items = 0;
	if (count > 0) {
		out->items = json_alloc(reader->document, count * sizeof(FaJson));
		copy_bytes((uint8_t *)out->items, (const uint8_t *)(reader->stack + base),
		           count * sizeof(FaJson));
	}
	reader->stack_length = base;
}

static int json_digit(uint8_t byte) {
	return byte >= '0' && byte <= '9';
}

static int json_hex(uint8_t byte) {
	if (byte >= '0' && byte <= '9') {
		return byte - '0';
	}
	if (byte >= 'a' && byte <= 'f') {
		return byte - 'a' + 10;
	}
	if (byte >= 'A' && byte <= 'F') {
		return byte - 'A' + 10;
	}
	return -1;
}

/// The 4 hex digits at `at`, or -1.
static int32_t json_code_unit(JsonReader *reader, size_t at) {
	if (at + 4 > reader->length) {
		return -1;
	}
	int32_t unit = 0;
	for (size_t index = 0; index < 4; index += 1) {
		int digit = json_hex(reader->text[at + index]);
		if (digit < 0) {
			return -1;
		}
		unit = unit * 16 + digit;
	}
	return unit;
}

static size_t json_utf8(uint32_t code, uint8_t *out) {
	if (code < 0x80) {
		out[0] = (uint8_t)code;
		return 1;
	}
	if (code < 0x800) {
		out[0] = (uint8_t)(0xC0 | (code >> 6));
		out[1] = (uint8_t)(0x80 | (code & 0x3F));
		return 2;
	}
	if (code < 0x10000) {
		out[0] = (uint8_t)(0xE0 | (code >> 12));
		out[1] = (uint8_t)(0x80 | ((code >> 6) & 0x3F));
		out[2] = (uint8_t)(0x80 | (code & 0x3F));
		return 3;
	}
	out[0] = (uint8_t)(0xF0 | (code >> 18));
	out[1] = (uint8_t)(0x80 | ((code >> 12) & 0x3F));
	out[2] = (uint8_t)(0x80 | ((code >> 6) & 0x3F));
	out[3] = (uint8_t)(0x80 | (code & 0x3F));
	return 4;
}

/// A string at `at` (its opening quote): its bytes, unescaped.
static int json_string(JsonReader *reader, const uint8_t **bytes, size_t *length) {
	size_t start = reader->at + 1;
	size_t at = start;
	int escaped = 0;
	for (;;) {
		if (at >= reader->length) {
			return json_fail(reader, "Unterminated string", 0, 0, 0);
		}
		uint8_t byte = reader->text[at];
		if (byte == '"') {
			break;
		}
		if (byte < 0x20) {
			return json_fail(reader, "Unterminated string", 0, 0, 0);
		}
		if (byte == '\\') {
			escaped = 1;
			at += 2;
			continue;
		}
		at += 1;
	}
	reader->at = at + 1;
	if (!escaped) {
		*bytes = reader->text + start;
		*length = at - start;
		return 1;
	}
	// an escape is never shorter than what it stands for
	uint8_t *out = json_alloc(reader->document, at - start);
	size_t written = 0;
	size_t index = start;
	int lone_surrogates = 0;
	while (index < at) {
		uint8_t byte = reader->text[index];
		if (byte != '\\') {
			out[written] = byte;
			written += 1;
			index += 1;
			continue;
		}
		uint8_t escape = reader->text[index + 1];
		index += 2;
		switch (escape) {
		case '"':
		case '\\':
		case '/':
			out[written++] = escape;
			break;
		case 'b':
			out[written++] = '\b';
			break;
		case 'f':
			out[written++] = '\f';
			break;
		case 'n':
			out[written++] = '\n';
			break;
		case 'r':
			out[written++] = '\r';
			break;
		case 't':
			out[written++] = '\t';
			break;
		case 'u': {
			int32_t unit = json_code_unit(reader, index);
			if (unit < 0 || index + 4 > at) {
				return json_fail(reader, "\\u must be followed by 4 hex digits", 0, 0, 0);
			}
			index += 4;
			uint32_t code = (uint32_t)unit;
			if (unit >= 0xD800 && unit <= 0xDBFF && index + 6 <= at && reader->text[index] == '\\' &&
			    reader->text[index + 1] == 'u') {
				int32_t low = json_code_unit(reader, index + 2);
				if (low >= 0xDC00 && low <= 0xDFFF) {
					code = 0x10000 + (((uint32_t)unit - 0xD800) << 10) + ((uint32_t)low - 0xDC00);
					index += 6;
				}
			}
			lone_surrogates |= code >= 0xD800 && code <= 0xDFFF;
			written += json_utf8(code, out + written);
			break;
		}
		default:
			return json_fail(reader, "Invalid escape character ", reader->text + index - 1, 1, 0);
		}
	}
	*bytes = out;
	if (lone_surrogates) {
		size_t flags;
		written = string_join_pairs(out, written, &flags);
	}
	*length = written;
	return 1;
}

#if defined(__wasm__)
// `Number(text)` of the host.
FA_IMPORT("parseNumber") double fa_host_parse_number(const uint8_t *text, size_t length);
#endif

/// The double of the text of a JSON number.
static double json_double(const uint8_t *text, size_t length) {
#if defined(__wasm__)
	return fa_host_parse_number(text, length);
#else
	char small[64];
	char *copy = length < sizeof small ? small : fa_alloc(length + 1, 1);
	copy_bytes((uint8_t *)copy, text, length);
	copy[length] = 0;
	double value = strtod(copy, 0);
	if (copy != small) {
		fa_free(copy, length + 1, 1);
	}
	return value;
#endif
}

static int json_number(JsonReader *reader, FaJson *out) {
	const uint8_t *text = reader->text;
	size_t start = reader->at;
	size_t at = start;
	int negative = 0;
	if (text[at] == '-') {
		negative = 1;
		at += 1;
	}
	if (at >= reader->length || !json_digit(text[at])) {
		return json_fail(reader, "Invalid number", 0, 0, 0);
	}
	uint64_t magnitude = 0;
	int exact = 1;
	if (text[at] == '0') {
		at += 1;
	} else {
		while (at < reader->length && json_digit(text[at])) {
			uint64_t digit = (uint64_t)(text[at] - '0');
			if (magnitude > (UINT64_MAX_VALUE - digit) / 10) {
				exact = 0;
			} else {
				magnitude = magnitude * 10 + digit;
			}
			at += 1;
		}
	}
	int whole = 1;
	if (at < reader->length && text[at] == '.') {
		at += 1;
		if (at >= reader->length || !json_digit(text[at])) {
			return json_fail(reader, "Invalid digits after decimal point", 0, 0, 0);
		}
		while (at < reader->length && json_digit(text[at])) {
			at += 1;
		}
		whole = 0;
	}
	if (at < reader->length && (text[at] == 'e' || text[at] == 'E')) {
		at += 1;
		if (at < reader->length && (text[at] == '+' || text[at] == '-')) {
			at += 1;
		}
		if (at >= reader->length || !json_digit(text[at])) {
			return json_fail(reader, "Unable to parse JSON string", 0, 0, 0);
		}
		while (at < reader->length && json_digit(text[at])) {
			at += 1;
		}
		whole = 0;
	}
	reader->at = at;
	out->kind = JSON_NUMBER;
	uint64_t limit = negative ? (uint64_t)1 << 63 : ((uint64_t)1 << 63) - 1;
	if (whole && exact && magnitude <= limit) {
		out->integer = negative ? (int64_t)(0 - magnitude) : (int64_t)magnitude;
		out->flags = JSON_INTEGRAL | JSON_FITS;
		out->number = negative && magnitude == 0 ? -0.0 : (double)out->integer;
		return 1;
	}
	double value = json_double(text + start, at - start);
	out->number = value;
	out->flags = 0;
	if (value - value == 0) {
		if (value >= -9223372036854775808.0 && value < 9223372036854775808.0) {
			int64_t integer = (int64_t)value;
			if ((double)integer == value) {
				out->integer = integer;
				out->flags = JSON_INTEGRAL | JSON_FITS;
			}
		} else {
			// beyond 2^63, every double is an integer
			out->flags = JSON_INTEGRAL;
		}
	}
	return 1;
}

static int json_identifier_byte(uint8_t byte) {
	return (byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') || json_digit(byte) ||
	       byte == '_' || byte == '$';
}

static int json_same(const uint8_t *bytes, size_t length, const char *text) {
	size_t text_length = c_length(text);
	if (length != text_length) {
		return 0;
	}
	for (size_t index = 0; index < length; index += 1) {
		if (bytes[index] != (uint8_t)text[index]) {
			return 0;
		}
	}
	return 1;
}

static int json_value(JsonReader *reader, FaJson *out);

/// The array index a key stands for (`Object.entries` lists them first),
/// or -1.
static int64_t json_index(const uint8_t *key, size_t length) {
	if (length == 0 || length > 10 || (length > 1 && key[0] == '0')) {
		return -1;
	}
	int64_t value = 0;
	for (size_t index = 0; index < length; index += 1) {
		if (!json_digit(key[index])) {
			return -1;
		}
		value = value * 10 + (key[index] - '0');
	}
	return value < 4294967295 ? value : -1;
}

/// The members in the order of `Object.entries`: the array indexes first,
/// ascending (stable), then the other keys.
static void json_order(JsonReader *reader, FaJson *object) {
	size_t count = object->count;
	size_t indexes = 0;
	for (size_t index = 0; index < count; index += 1) {
		indexes += json_index(object->items[index].key, object->items[index].key_length) >= 0;
	}
	if (indexes == 0) {
		return;
	}
	FaJson *ordered = json_alloc(reader->document, count * sizeof(FaJson));
	size_t sorted = 0;
	size_t rest = indexes;
	for (size_t index = 0; index < count; index += 1) {
		FaJson *member = &object->items[index];
		int64_t position = json_index(member->key, member->key_length);
		if (position < 0) {
			ordered[rest] = *member;
			rest += 1;
			continue;
		}
		// insertion: after the equal or lower indexes
		size_t at = sorted;
		while (at > 0 && json_index(ordered[at - 1].key, ordered[at - 1].key_length) > position) {
			ordered[at] = ordered[at - 1];
			at -= 1;
		}
		ordered[at] = *member;
		sorted += 1;
	}
	object->items = ordered;
}

static int json_array(JsonReader *reader, FaJson *out) {
	reader->at += 1;
	out->kind = JSON_ARRAY;
	size_t base = reader->stack_length;
	json_space(reader);
	if (reader->at < reader->length && reader->text[reader->at] == ']') {
		reader->at += 1;
		json_collect(reader, base, out);
		return 1;
	}
	for (;;) {
		FaJson item = {0};
		if (!json_value(reader, &item)) {
			return 0;
		}
		json_push(reader, &item);
		json_space(reader);
		if (reader->at < reader->length && reader->text[reader->at] == ',') {
			reader->at += 1;
			json_space(reader);
			if (reader->at < reader->length && reader->text[reader->at] == ']') {
				return json_fail(reader, "Unexpected comma at the end of array expression", 0, 0, 0);
			}
			continue;
		}
		if (reader->at < reader->length && reader->text[reader->at] == ']') {
			reader->at += 1;
			json_collect(reader, base, out);
			return 1;
		}
		return json_fail(reader, "Expected ']'", 0, 0, 0);
	}
}

static int json_object(JsonReader *reader, FaJson *out) {
	reader->at += 1;
	out->kind = JSON_OBJECT;
	size_t base = reader->stack_length;
	json_space(reader);
	if (reader->at < reader->length && reader->text[reader->at] == '}') {
		reader->at += 1;
		json_collect(reader, base, out);
		return 1;
	}
	if (reader->at >= reader->length || reader->text[reader->at] != '"') {
		return json_fail(reader, "Expected '}'", 0, 0, 0);
	}
	for (;;) {
		FaJson member = {0};
		if (!json_string(reader, &member.key, &member.key_length)) {
			return 0;
		}
		json_space(reader);
		if (reader->at >= reader->length || reader->text[reader->at] != ':') {
			return json_fail(reader, "Expected ':' before value in object property definition", 0, 0,
			                 0);
		}
		reader->at += 1;
		const uint8_t *key = member.key;
		size_t key_length = member.key_length;
		if (!json_value(reader, &member)) {
			return 0;
		}
		member.key = key;
		member.key_length = key_length;
		json_push(reader, &member);
		json_space(reader);
		if (reader->at < reader->length && reader->text[reader->at] == ',') {
			reader->at += 1;
			json_space(reader);
			if (reader->at >= reader->length || reader->text[reader->at] != '"') {
				return json_fail(reader, "Property name must be a string literal", 0, 0, 0);
			}
			continue;
		}
		if (reader->at < reader->length && reader->text[reader->at] == '}') {
			reader->at += 1;
			json_collect(reader, base, out);
			json_order(reader, out);
			return 1;
		}
		return json_fail(reader, "Expected '}'", 0, 0, 0);
	}
}

static int json_value(JsonReader *reader, FaJson *out) {
	json_space(reader);
	if (reader->at >= reader->length) {
		return json_fail(reader, "Unexpected EOF", 0, 0, 0);
	}
	const uint8_t *text = reader->text;
	uint8_t byte = text[reader->at];
	if (byte == '{' || byte == '[') {
		if (reader->depth >= JSON_MAX_DEPTH) {
			return json_fail(reader, "Exceeded the maximum depth", 0, 0, 0);
		}
		reader->depth += 1;
		int done = byte == '{' ? json_object(reader, out) : json_array(reader, out);
		reader->depth -= 1;
		return done;
	}
	if (byte == '"') {
		out->kind = JSON_STRING;
		return json_string(reader, &out->bytes, &out->length);
	}
	if (byte == '-' || json_digit(byte)) {
		return json_number(reader, out);
	}
	if (byte == '}' || byte == ']' || byte == ',' || byte == ':' || byte == '.') {
		return json_fail(reader, "Unexpected token ", text + reader->at, 1, '\'');
	}
	if (json_identifier_byte(byte)) {
		size_t start = reader->at;
		size_t end = start;
		while (end < reader->length && json_identifier_byte(text[end])) {
			end += 1;
		}
		const uint8_t *word = text + start;
		size_t length = end - start;
		if (json_same(word, length, "true") || json_same(word, length, "false")) {
			out->kind = JSON_BOOLEAN;
			out->flags = length == 4;
		} else if (json_same(word, length, "null")) {
			out->kind = JSON_NULL;
		} else {
			return json_fail(reader, "Unexpected identifier ", word, length, '"');
		}
		reader->at = end;
		return 1;
	}
	// a character, whole
	size_t length = 1;
	if (byte >= 0xF0) {
		length = 4;
	} else if (byte >= 0xE0) {
		length = 3;
	} else if (byte >= 0xC0) {
		length = 2;
	}
	if (reader->at + length > reader->length) {
		length = reader->length - reader->at;
	}
	return json_fail(reader, "Unrecognized token ", text + reader->at, length, '\'');
}

// ── checking a value against a schema

typedef struct {
	/// Where the value being checked is: `the value.tags[0]`.
	uint8_t *path;
	size_t length;
	size_t room;
	/// The error, once found (a `JsonError`).
	const void *error;
} JsonCheck;

static void json_path(JsonCheck *check, const uint8_t *bytes, size_t length) {
	if (check->length + length > check->room) {
		size_t room = check->room * 2;
		while (room < check->length + length) {
			room *= 2;
		}
		uint8_t *path = fa_alloc(room, 1);
		copy_bytes(path, check->path, check->length);
		fa_free(check->path, check->room, 1);
		check->path = path;
		check->room = room;
	}
	copy_bytes(check->path + check->length, bytes, length);
	check->length += length;
}

/// Reads `<digits>;` (an empty number: `present` is 0).
static int64_t json_schema_number(const uint8_t **schema, int *present) {
	const uint8_t *cursor = *schema;
	int negative = 0;
	if (*cursor == '-') {
		negative = 1;
		cursor += 1;
	}
	uint64_t value = 0;
	*present = *cursor != ';';
	while (*cursor != ';') {
		value = value * 10 + (uint64_t)(*cursor - '0');
		cursor += 1;
	}
	*schema = cursor + 1;
	return negative ? (int64_t)(0 - value) : (int64_t)value;
}

/// The schema after this one.
static const uint8_t *json_skip(const uint8_t *schema) {
	int present;
	switch (*schema) {
	case 'i':
		schema += 1;
		json_schema_number(&schema, &present);
		json_schema_number(&schema, &present);
		return schema;
	case 'S': {
		schema += 1;
		int64_t length = json_schema_number(&schema, &present);
		return schema + length;
	}
	case 'I':
		schema += 1;
		json_schema_number(&schema, &present);
		return schema;
	case 'a':
	case 'm':
		return json_skip(schema + 1);
	case 'o': {
		schema += 1;
		int64_t count = json_schema_number(&schema, &present);
		for (int64_t field = 0; field < count; field += 1) {
			int64_t length = json_schema_number(&schema, &present);
			schema = json_skip(schema + length + 1);
		}
		return schema;
	}
	case 'u': {
		schema += 1;
		int64_t count = json_schema_number(&schema, &present);
		for (int64_t member = 0; member < count; member += 1) {
			schema = json_skip(schema);
		}
		return schema;
	}
	case 'N': {
		schema += 1;
		int64_t count = json_schema_number(&schema, &present);
		for (int64_t member = 0; member < count; member += 1) {
			int64_t length = json_schema_number(&schema, &present);
			uint8_t kind = schema[length];
			schema += length + 1;
			if (kind != 'e') {
				schema = json_skip(schema);
			}
		}
		return schema;
	}
	case 'E': {
		schema += 1;
		int64_t count = json_schema_number(&schema, &present);
		for (int64_t member = 0; member < count; member += 1) {
			int64_t length = json_schema_number(&schema, &present);
			schema += length;
			if (*schema == 'p') {
				schema += 1;
				json_schema_number(&schema, &present);
			} else {
				schema += 1;
			}
		}
		return schema;
	}
	default:
		return schema + 1;
	}
}

static int json_check(JsonCheck *check, const FaJson *node, const uint8_t *schema, int report);
FA_EXPORT const FaJson *fa_json_field(const FaJson *node, const uint8_t *name, size_t length);

/// A member of the schema of a named union (`N<count>;` then, for each,
/// `<length>;<name><kind>` and the schema of its data unless `kind` is
/// `e`): its name, its kind (`e` no data, `l` a literal, `r` an object next
/// to `"type"`, `v` data under `"value"`), its data's schema.
typedef struct {
	const uint8_t *name;
	size_t length;
	uint8_t kind;
	const uint8_t *data;
} JsonMember;

static const uint8_t *json_member_at(const uint8_t *cursor, JsonMember *member) {
	int present;
	int64_t length = json_schema_number(&cursor, &present);
	member->name = cursor;
	member->length = (size_t)length;
	member->kind = cursor[length];
	member->data = cursor + length + 1;
	return member->kind == 'e' ? member->data : json_skip(member->data);
}

/// The member of an enumeration a value is (its position in the schema),
/// or -1: its pinned value (a number), else its name (a string).
static int64_t json_enum_of(const FaJson *node, const uint8_t *schema) {
	int present;
	const uint8_t *cursor = schema + 1;
	int64_t count = json_schema_number(&cursor, &present);
	for (int64_t index = 0; index < count; index += 1) {
		int64_t length = json_schema_number(&cursor, &present);
		const uint8_t *name = cursor;
		cursor += length;
		if (*cursor == 'p') {
			cursor += 1;
			int64_t value = json_schema_number(&cursor, &present);
			if (node->kind == JSON_NUMBER && (node->flags & JSON_FITS) && node->integer == value) {
				return index;
			}
			continue;
		}
		cursor += 1;
		if (node->kind == JSON_STRING && node->length == (size_t)length) {
			int same = 1;
			for (size_t at = 0; at < (size_t)length; at += 1) {
				same &= node->bytes[at] == name[at];
			}
			if (same) {
				return index;
			}
		}
	}
	return -1;
}

/// The member of a named union a value is (its position in the schema),
/// or -1: a literal it equals, else the member its `"type"` names.
static int64_t json_member_of(const FaJson *node, const uint8_t *schema, const FaJson **tag) {
	int present;
	const uint8_t *cursor = schema + 1;
	int64_t count = json_schema_number(&cursor, &present);
	*tag = node->kind == JSON_OBJECT ? fa_json_field(node, (const uint8_t *)"type", 4) : 0;
	int64_t found = -1;
	for (int64_t index = 0; index < count && found < 0; index += 1) {
		JsonMember member;
		cursor = json_member_at(cursor, &member);
		if (member.kind == 'l') {
			JsonCheck check = {0};
			if (json_check(&check, node, member.data, 0)) {
				found = index;
			}
			continue;
		}
		if (*tag != 0 && (*tag)->kind == JSON_STRING && (*tag)->length == member.length) {
			int same = 1;
			for (size_t at = 0; at < member.length; at += 1) {
				same &= (*tag)->bytes[at] == member.name[at];
			}
			if (same) {
				found = index;
			}
		}
	}
	return found;
}

/// What a value is, in the words of `faJsonParse`.
static const char *json_found(const FaJson *node) {
	switch (node->kind) {
	case JSON_NULL:
		return "null";
	case JSON_BOOLEAN:
		return "boolean";
	case JSON_NUMBER:
		return "number";
	case JSON_STRING:
		return "string";
	case JSON_ARRAY:
		return "an array";
	default:
		return "object";
	}
}

FA_EXPORT void fa_b_json_string(const uint8_t *bytes, size_t length);
FA_EXPORT const FaJson *fa_json_field(const FaJson *node, const uint8_t *name, size_t length);

/// The error of the value being checked: `<path><text>`, built by the
/// caller after `json_error_start`.
static void json_error_start(JsonCheck *check) {
	fa_b_reset();
	fa_b_bytes(check->path, check->length);
}

static int json_error_end(JsonCheck *check) {
	MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
	error->id = JSON_ERROR;
	fa_b_take(&error->message);
	check->error = error;
	return 0;
}

/// `<path> should be <expected>, not <found>`; `expected` is a C string,
/// or the literal of `schema` (`literal`).
static int json_mismatch(JsonCheck *check, const FaJson *node, const char *expected,
                         const uint8_t *literal, int report) {
	if (!report) {
		return 0;
	}
	json_error_start(check);
	append_c(" should be ");
	if (literal != 0) {
		int present;
		switch (*literal) {
		case 'S': {
			const uint8_t *cursor = literal + 1;
			int64_t length = json_schema_number(&cursor, &present);
			fa_b_json_string(cursor, (size_t)length);
			break;
		}
		case 'I': {
			const uint8_t *cursor = literal + 1;
			int64_t value = json_schema_number(&cursor, &present);
			fa_b_i64(value);
			break;
		}
		default:
			append_c(*literal == 'T' ? "true" : "false");
		}
	} else {
		append_c(expected);
	}
	append_c(", not ");
	append_c(json_found(node));
	return json_error_end(check);
}

static int json_check(JsonCheck *check, const FaJson *node, const uint8_t *schema, int report) {
	int present;
	switch (*schema) {
	case 'b':
		return node->kind == JSON_BOOLEAN || json_mismatch(check, node, "a boolean", 0, report);
	case 's':
		return node->kind == JSON_STRING || json_mismatch(check, node, "a string", 0, report);
	case 'f':
		return node->kind == JSON_NUMBER || json_mismatch(check, node, "a number", 0, report);
	case 'n':
		return node->kind == JSON_NULL || json_mismatch(check, node, "null", 0, report);
	case 'i': {
		const uint8_t *cursor = schema + 1;
		int has_min;
		int has_max;
		int64_t min = json_schema_number(&cursor, &has_min);
		int64_t max = json_schema_number(&cursor, &has_max);
		if (node->kind != JSON_NUMBER || !(node->flags & JSON_INTEGRAL)) {
			return json_mismatch(check, node, "an integer", 0, report);
		}
		if (!(node->flags & JSON_FITS)) {
			if (report) {
				json_error_start(check);
				append_c(" doesn't fit in an Integer");
				json_error_end(check);
			}
			return 0;
		}
		if ((has_min && node->integer < min) || (has_max && node->integer > max)) {
			if (report) {
				int low = has_min && node->integer < min;
				json_error_start(check);
				append_c(low ? " should be at least " : " should be at most ");
				fa_b_i64(low ? min : max);
				append_c(", not ");
				fa_b_i64(node->integer);
				json_error_end(check);
			}
			return 0;
		}
		return 1;
	}
	case 'S': {
		const uint8_t *cursor = schema + 1;
		int64_t length = json_schema_number(&cursor, &present);
		if (node->kind == JSON_STRING && node->length == (size_t)length) {
			int same = 1;
			for (size_t index = 0; index < node->length; index += 1) {
				same &= node->bytes[index] == cursor[index];
			}
			if (same) {
				return 1;
			}
		}
		return json_mismatch(check, node, 0, schema, report);
	}
	case 'T':
	case 'F':
		return (node->kind == JSON_BOOLEAN && node->flags == (*schema == 'T')) ||
		       json_mismatch(check, node, 0, schema, report);
	case 'I': {
		const uint8_t *cursor = schema + 1;
		int64_t value = json_schema_number(&cursor, &present);
		return (node->kind == JSON_NUMBER && (node->flags & JSON_FITS) && node->integer == value) ||
		       json_mismatch(check, node, 0, schema, report);
	}
	case 'a': {
		if (node->kind != JSON_ARRAY) {
			return json_mismatch(check, node, "an array", 0, report);
		}
		size_t length = check->length;
		for (size_t index = 0; index < node->count; index += 1) {
			uint8_t digits[24];
			size_t start = decimal_u64(index, 0, digits);
			json_path(check, (const uint8_t *)"[", 1);
			json_path(check, digits + start, sizeof digits - start);
			json_path(check, (const uint8_t *)"]", 1);
			int fits = json_check(check, &node->items[index], schema + 1, report);
			check->length = length;
			if (!fits) {
				return 0;
			}
		}
		return 1;
	}
	case 'm': {
		if (node->kind != JSON_OBJECT) {
			return json_mismatch(check, node, "an object", 0, report);
		}
		size_t length = check->length;
		for (size_t index = 0; index < node->count; index += 1) {
			const FaJson *member = &node->items[index];
			json_path(check, (const uint8_t *)".", 1);
			json_path(check, member->key, member->key_length);
			int fits = json_check(check, member, schema + 1, report);
			check->length = length;
			if (!fits) {
				return 0;
			}
		}
		return 1;
	}
	case 'o': {
		if (node->kind != JSON_OBJECT) {
			return json_mismatch(check, node, "an object", 0, report);
		}
		const uint8_t *cursor = schema + 1;
		int64_t count = json_schema_number(&cursor, &present);
		size_t length = check->length;
		for (int64_t field = 0; field < count; field += 1) {
			int64_t name_length = json_schema_number(&cursor, &present);
			const uint8_t *name = cursor;
			int optional = cursor[name_length] == '1';
			const uint8_t *field_schema = cursor + name_length + 1;
			cursor = json_skip(field_schema);
			const FaJson *member = fa_json_field(node, name, (size_t)name_length);
			if (member == 0) {
				if (optional) {
					continue;
				}
				if (report) {
					json_error_start(check);
					append_c(" has no field `");
					fa_b_bytes(name, (size_t)name_length);
					append_c("`");
					json_error_end(check);
				}
				return 0;
			}
			json_path(check, (const uint8_t *)".", 1);
			json_path(check, name, (size_t)name_length);
			int fits = json_check(check, member, field_schema, report);
			check->length = length;
			if (!fits) {
				return 0;
			}
		}
		return 1;
	}
	case 'u': {
		const uint8_t *cursor = schema + 1;
		int64_t count = json_schema_number(&cursor, &present);
		for (int64_t member = 0; member < count; member += 1) {
			if (json_check(check, node, cursor, 0)) {
				return 1;
			}
			cursor = json_skip(cursor);
		}
		return json_mismatch(check, node, "one of the members of its union", 0, report);
	}
	case 'E': {
		if (json_enum_of(node, schema) >= 0) {
			return 1;
		}
		if (report) {
			// `<path> should be one of "red", "green", not string` (or the values)
			json_error_start(check);
			append_c(" should be one of ");
			const uint8_t *cursor = schema + 1;
			int64_t count = json_schema_number(&cursor, &present);
			for (int64_t member = 0; member < count; member += 1) {
				int64_t length = json_schema_number(&cursor, &present);
				const uint8_t *name = cursor;
				cursor += length;
				if (member > 0) {
					append_c(", ");
				}
				if (*cursor == 'p') {
					cursor += 1;
					fa_b_i64(json_schema_number(&cursor, &present));
				} else {
					cursor += 1;
					fa_b_json_string(name, (size_t)length);
				}
			}
			append_c(", not ");
			append_c(json_found(node));
			json_error_end(check);
		}
		return 0;
	}
	case 'N': {
		const FaJson *tag;
		int64_t index = json_member_of(node, schema, &tag);
		if (index < 0) {
			if (tag == 0 || tag->kind != JSON_STRING) {
				return json_mismatch(check, node, "one of the members of its union", 0, report);
			}
			if (report) {
				// `<path>.type should be one of "A", "B", not "C"`
				json_error_start(check);
				append_c(".type should be one of ");
				const uint8_t *cursor = schema + 1;
				int64_t count = json_schema_number(&cursor, &present);
				int first = 1;
				for (int64_t member = 0; member < count; member += 1) {
					JsonMember found;
					cursor = json_member_at(cursor, &found);
					if (found.kind == 'l') {
						continue;
					}
					if (!first) {
						append_c(", ");
					}
					first = 0;
					fa_b_json_string(found.name, found.length);
				}
				append_c(", not ");
				fa_b_json_string(tag->bytes, tag->length);
				json_error_end(check);
			}
			return 0;
		}
		const uint8_t *cursor = schema + 1;
		json_schema_number(&cursor, &present);
		JsonMember member;
		for (int64_t at = 0; at <= index; at += 1) {
			cursor = json_member_at(cursor, &member);
		}
		switch (member.kind) {
		case 'e':
		case 'l':
			return 1;
		case 'r':
			return json_check(check, node, member.data, report);
		default: {
			const FaJson *value = fa_json_field(node, (const uint8_t *)"value", 5);
			if (value == 0) {
				if (report) {
					json_error_start(check);
					append_c(" has no field `value`");
					json_error_end(check);
				}
				return 0;
			}
			size_t length = check->length;
			json_path(check, (const uint8_t *)".value", 6);
			int fits = json_check(check, value, member.data, report);
			check->length = length;
			return fits;
		}
		}
	}
	default:
		if (report) {
			json_error_start(check);
			append_c(" has a type JSON can't describe");
			json_error_end(check);
		}
		return 0;
	}
}

static const void *json_parse_error(JsonReader *reader) {
	fa_b_reset();
	append_c("JSON Parse error: ");
	append_c(reader->error);
	if (reader->detail != 0) {
		if (reader->quote != 0) {
			append_byte(reader->quote);
		}
		fa_b_bytes(reader->detail, reader->detail_length);
		if (reader->quote != 0) {
			append_byte(reader->quote);
		}
	}
	MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
	error->id = JSON_ERROR;
	fa_b_take(&error->message);
	return error;
}

/// `Json.parse(text)`: reads the text and checks it against `schema`; the
/// error (a `JsonError`), or null and the parsed text in `out`.
FA_EXPORT const void *fa_json_parse(const uint8_t *text, size_t length, const uint8_t *schema,
                                    FaJsonDocument **out) {
	FaJsonDocument *document = fa_alloc(sizeof(FaJsonDocument), 16);
	__builtin_memset(document, 0, sizeof(FaJsonDocument));
	JsonReader reader = {0};
	reader.text = text;
	reader.length = length;
	reader.document = document;
	int read = json_value(&reader, &document->root);
	if (read) {
		json_space(&reader);
		if (reader.at < reader.length) {
			read = json_fail(&reader, "Unable to parse JSON string", 0, 0, 0);
		}
	}
	if (reader.stack_room > 0) {
		fa_free(reader.stack, reader.stack_room * sizeof(FaJson), 16);
	}
	if (!read) {
		fa_json_free(document);
		return json_parse_error(&reader);
	}
	JsonCheck check = {0};
	check.room = 64;
	check.path = fa_alloc(check.room, 1);
	json_path(&check, (const uint8_t *)"the value", 9);
	json_check(&check, &document->root, schema, 1);
	fa_free(check.path, check.room, 1);
	if (check.error != 0) {
		fa_json_free(document);
		return check.error;
	}
	*out = document;
	return 0;
}

/// Binary JSON input is strict UTF-8. Parse the validated bytes directly;
/// only decoded string tokens become String values.
FA_EXPORT const void *fa_json_parse_bytes(const void *bytes, const uint8_t *schema,
                                         FaJsonDocument **out) {
	FaBytesView view = bytes_parts(bytes);
	Utf8Info info = utf8_validate(view.items, view.length);
	if (info.error != SIZE_MAX) {
		static const uint8_t message[] = "invalid UTF-8 in JSON input";
		MessageError *error = fa_alloc(sizeof(MessageError), sizeof(int64_t));
		error->id = JSON_ERROR;
		error->message = owned_copy_with(message, sizeof message - 1, SIMPLE);
		return error;
	}
	return fa_json_parse(view.items, view.length, schema, out);
}

/// The member of an enumeration a value (that fits) is: its position in
/// the schema.
FA_EXPORT int64_t fa_json_enum(const FaJson *node, const uint8_t *schema) {
	return json_enum_of(node, schema);
}

/// The member of a named union a value (that fits) is: its position in
/// the schema.
FA_EXPORT int64_t fa_json_member(const FaJson *node, const uint8_t *schema) {
	const FaJson *tag;
	return json_member_of(node, schema, &tag);
}

/// Whether a value fits a schema (the member of a union it is read as).
FA_EXPORT int32_t fa_json_fits(const FaJson *node, const uint8_t *schema) {
	JsonCheck check = {0};
	return json_check(&check, node, schema, 0);
}

FA_EXPORT int32_t fa_json_boolean(const FaJson *node) {
	return node->flags;
}

FA_EXPORT int64_t fa_json_integer(const FaJson *node) {
	return node->integer;
}

FA_EXPORT double fa_json_number(const FaJson *node) {
	return node->number;
}

FA_EXPORT void fa_json_string(const FaJson *node, FaString *out) {
	*out = owned_copy(node->bytes, node->length);
}

FA_EXPORT int64_t fa_json_count(const FaJson *node) {
	return (int64_t)node->count;
}

FA_EXPORT const FaJson *fa_json_item(const FaJson *node, int64_t index) {
	return &node->items[index];
}

/// The key of a member of an object.
FA_EXPORT void fa_json_key(const FaJson *member, FaString *out) {
	*out = owned_copy(member->key, member->key_length);
}

/// The member of an object named `name` (the last one, as `JSON.parse`
/// keeps it), or null.
FA_EXPORT const FaJson *fa_json_field(const FaJson *node, const uint8_t *name, size_t length) {
	for (size_t index = node->count; index > 0; index -= 1) {
		const FaJson *member = &node->items[index - 1];
		if (member->key_length == length) {
			int same = 1;
			for (size_t at = 0; at < length; at += 1) {
				same &= member->key[at] == name[at];
			}
			if (same) {
				return member;
			}
		}
	}
	return 0;
}

/// A string of `JSON.stringify`: quoted, `\"`, `\\`, `\b`, `\f`, `\n`,
/// `\r`, `\t`, and `\u00xx` for the other control characters.
static void quote_json_bytes(const uint8_t *bytes, size_t length) {
	static const char hex[] = "0123456789abcdef";
	size_t start = 0;
	for (size_t index = 0; index < length; index += 1) {
		uint8_t byte = bytes[index];
		if (byte >= 0x20 && byte != '"' && byte != '\\') {
			continue;
		}
		fa_b_bytes(bytes + start, index - start);
		start = index + 1;
		switch (byte) {
		case '"':
			append_c("\\\"");
			break;
		case '\\':
			append_c("\\\\");
			break;
		case '\b':
			append_c("\\b");
			break;
		case '\f':
			append_c("\\f");
			break;
		case '\n':
			append_c("\\n");
			break;
		case '\r':
			append_c("\\r");
			break;
		case '\t':
			append_c("\\t");
			break;
		default:
			append_c("\\u00");
			append_byte((uint8_t)hex[byte >> 4]);
			append_byte((uint8_t)hex[byte & 15]);
		}
	}
	fa_b_bytes(bytes + start, length - start);
}

static void quote_json_surrogates(const uint8_t *bytes, size_t length) {
	static const char hex[] = "0123456789abcdef";
	size_t start = 0;
	for (size_t index = 0; index + 2 < length; index += 1) {
		if (bytes[index] != 0xED || bytes[index + 1] < 0xA0) { continue; }
		quote_json_bytes(bytes + start, index - start);
		uint16_t unit = (uint16_t)(0xD000 | ((bytes[index + 1] & 63) << 6) | (bytes[index + 2] & 63));
		append_c("\\u");
		for (int shift = 12; shift >= 0; shift -= 4) { append_byte((uint8_t)hex[(unit >> shift) & 15]); }
		index += 2;
		start = index + 1;
	}
	quote_json_bytes(bytes + start, length - start);
}

FA_EXPORT void fa_b_json_string_flags(const uint8_t *bytes, size_t length, size_t capacity) {
	append_byte('"');
	if (!(capacity & SIMPLE) && span_has_edges(bytes, length)) {
		string_span_parts(bytes, length, quote_json_surrogates);
	} else if (capacity & STRING_SURROGATES) { quote_json_surrogates(bytes, length); }
	else { quote_json_bytes(bytes, length); }
	append_byte('"');
}

FA_EXPORT void fa_b_json_string(const uint8_t *bytes, size_t length) {
	fa_b_json_string_flags(bytes, length, STRING_SURROGATES);
}

/// A number of `JSON.stringify`: `null` when it isn't finite.
FA_EXPORT void fa_b_json_number(double value) {
	if (value - value != 0) {
		append_c("null");
	} else {
		fa_b_f64(value);
	}
}

// ── workers (asynchronous programs: design/async.md §3.4, A9, A10;
// `src/lower/tasks.rs`)
//
// - **A worker** is a block of the handles (`FaWorker`: a `FaShared` header,
//   then the worker's state at `WORKER_STATE`) and an OS thread, which runs
//   the calls of its queue one at a time, in order (A9), each with the
//   synchronous instance of the method (a blocking call: an `await` in it
//   waits, a `start` runs at once). Its last handle stops it.
// - **A call** (`FaCall`) is a message: the generated `run` (the method,
//   given the state, the copied arguments and the call's frame), and a
//   frame laid out like a task's (`WORKER_FRAME` bytes after the call),
//   whose promise receives the result. Awaited or started, the frame is the
//   task: its resume function (generated, `fastcc`) is resumed once the
//   worker ended the call (through the caller's remote queue), and ends the
//   task. A blocking caller waits on the worker's condition variable.
// - **Cancelling** a task of a call (its drop): a queued call is removed (its
//   arguments dropped), a running one is abandoned (the worker drops its
//   result), a done one is disposed of (A8).
// - A call holds a handle on its worker (released by the caller's side),
//   so that the block outlives the tasks of its calls whatever the order of
//   the drops; the worker thread never releases one (no thread joins itself).
// - `stop()` (or the drop of the last handle) cancels the queued detached
//   calls, lets the others run, ends the thread and drops the state (A10).
// - Without threads (freestanding), a call runs at once on the caller's thread.

#define WORKER_STATE 256
#define WORKER_FRAME 144

enum { CALL_QUEUED, CALL_RUNNING, CALL_DONE, CALL_ABANDONED };

typedef struct FaCall {
	struct FaCall *next;
	struct FaWorker *worker;
	/// The executor of an awaited or started call; null for a blocking caller.
	FaExecutor *owner;
	/// Runs the method on the worker's thread: `run(state, arguments, frame)`.
	void (*run)(void *state, void *arguments, void *frame);
	/// Drops the arguments and frees their block (a call cancelled while queued).
	void (*drop_arguments)(void *arguments);
	/// Drops the result if it wasn't taken, then frees the call (`fa_call_free`).
	void (*dispose)(void *frame);
	void *arguments;
	int64_t size;
	int32_t state;
	/// The owners of a job of a pool (its task, its worker's side).
	int32_t refs;
	/// A job of a pool: its pool, when a worker took it, its result as the
	/// worker writes it (`raw_size` bytes), the timer of its timeout, and
	/// what writes `WorkerTimeout` into its promise or drops a result.
	struct FaWorkerPool *pool;
	int64_t started_at;
	void *scratch;
	int64_t raw_size;
	struct FaWait *timer;
	void (*timeout_error)(void *frame);
	void (*drop_result)(void *result);
} FaCall;

_Static_assert(sizeof(FaCall) <= WORKER_FRAME, "the header of a call");

typedef struct FaWorker {
	FaShared shared;
	FaSystemLock lock;
	/// Signalled when the queue gets a call, and when a call ends.
	FaSystemLock changed;
	FaCall *head;
	FaCall *tail;
	FaCall *running;
	/// Drops the state (at `WORKER_STATE`), given the block.
	void (*drop_state)(void *block);
	unsigned long thread;
	int32_t started;
	int32_t stopping;
	int32_t stopped;
	/// Replaced in its pool (a call timed out): its thread ends its call,
	/// then exits and releases the worker (it holds a handle).
	int32_t detached;
	/// The pool of a worker of a pool, which takes its jobs from it.
	struct FaWorkerPool *pool;
} FaWorker;

/// The worker whose thread this is (its own calls run in place).
static FA_THREAD_LOCAL FaWorker *current_worker;

_Static_assert(sizeof(FaWorker) <= WORKER_STATE, "the header of a worker");

#if !defined(__wasm__)
__attribute__((weak)) int pthread_join(unsigned long thread, void **result);
#endif

/// The workers alive (atomic): while there are some, a blocking wait on a
/// channel or a mutex may be ended by them.
static int64_t workers_alive;

static FaCall *call_of(void *frame) {
	return (FaCall *)((uint8_t *)frame - WORKER_FRAME);
}

#if !defined(__wasm__)
/// The end of a call on the worker's thread (its lock held): wakes its caller.
static void call_ended(FaCall *call) {
	FaTask *task = task_of((uint8_t *)call + WORKER_FRAME);
	task->flags |= TASK_RESULT;
	call->state = CALL_DONE;
	if (call->owner != 0) {
		FaExecutor *owner = call->owner;
		remote_push(owner, task);
		fa_remote_source_end(owner);
	}
}

static void *worker_thread(void *argument) {
	FaWorker *worker = argument;
	// every call is a blocking call of the method's synchronous instance
	fa_blocking_depth = 1;
	system_lock(&worker->lock);
	for (;;) {
		while (worker->head == 0 && !worker->stopping) {
			pthread_cond_wait(&worker->changed, &worker->lock);
		}
		FaCall *call = worker->head;
		if (call == 0) {
			break;
		}
		worker->head = call->next;
		if (worker->head == 0) {
			worker->tail = 0;
		}
		call->state = CALL_RUNNING;
		worker->running = call;
		system_unlock(&worker->lock);
		call->run((uint8_t *)worker + WORKER_STATE, call->arguments,
		          (uint8_t *)call + WORKER_FRAME);
		system_lock(&worker->lock);
		worker->running = 0;
		if (call->state == CALL_ABANDONED) {
			system_unlock(&worker->lock);
			task_of((uint8_t *)call + WORKER_FRAME)->flags |= TASK_RESULT;
			call->dispose((uint8_t *)call + WORKER_FRAME);
			system_lock(&worker->lock);
		} else {
			call_ended(call);
		}
		pthread_cond_broadcast(&worker->changed);
	}
	int32_t detached = worker->detached;
	system_unlock(&worker->lock);
	// its text builder (counted by leak checks)
	fa_release_builder();
	if (detached) {
		__atomic_fetch_sub(&workers_alive, 1, __ATOMIC_ACQ_REL);
		fa_shared_release(worker);
	}
	return 0;
}
#endif

/// Stops a worker: the queued calls are cancelled, the running one waited
/// for, the thread ended (A10). Again: nothing.
FA_EXPORT void fa_worker_stop(FaWorker *worker) {
	if (worker == 0 || worker->stopped) {
		return;
	}
	worker->stopped = 1;
	if (!worker->started) {
		return;
	}
	if (worker->pool != 0) {
		// a worker of a pool: the pool's shutdown made it stop
#if !defined(__wasm__)
		if (pthread_join != 0) {
			pthread_join(worker->thread, 0);
		}
#endif
		__atomic_fetch_sub(&workers_alive, 1, __ATOMIC_ACQ_REL);
		return;
	}
	// the queued detached calls are cancelled (nothing awaits them); the
	// tasks of kept ones are awaited or dropped by their owner (CONC019):
	// they run
	system_lock(&worker->lock);
	FaCall *cancelled = 0;
	FaCall **link = &worker->head;
	FaCall *previous = 0;
	while (*link != 0) {
		FaCall *call = *link;
		if (task_of((uint8_t *)call + WORKER_FRAME)->flags & TASK_DETACHED) {
			*link = call->next;
			call->next = cancelled;
			cancelled = call;
		} else {
			previous = call;
			link = &call->next;
		}
	}
	worker->tail = previous;
	worker->stopping = 1;
	pthread_cond_broadcast(&worker->changed);
	system_unlock(&worker->lock);
	while (cancelled != 0) {
		FaCall *next = cancelled->next;
		FaExecutor *owner = cancelled->owner;
		cancelled->drop_arguments(cancelled->arguments);
		if (owner != 0) {
			owner->detached -= 1;
		}
		fa_free(cancelled, (size_t)cancelled->size, 16);
		if (owner != 0) {
			fa_remote_source_end(owner);
		}
		fa_shared_release(worker);
		cancelled = next;
	}
#if !defined(__wasm__)
	if (pthread_join != 0) {
		pthread_join(worker->thread, 0);
	}
#endif
	__atomic_fetch_sub(&workers_alive, 1, __ATOMIC_ACQ_REL);
}

__attribute__((weak)) int pthread_detach(unsigned long thread);

/// Detaches a worker replaced in its pool: its thread ends the call it
/// runs, then exits and releases the worker.
FA_EXPORT void fa_worker_detach(FaWorker *worker) {
	if (!worker->started || worker->stopped) {
		return;
	}
	fa_shared_retain(worker);
	system_lock(&worker->lock);
	worker->detached = 1;
	worker->stopping = 1;
	pthread_cond_broadcast(&worker->changed);
	system_unlock(&worker->lock);
	worker->stopped = 1;
	if (pthread_detach != 0) {
		pthread_detach(worker->thread);
	}
}

static void worker_drop(void *block) {
	FaWorker *worker = block;
	fa_worker_stop(worker);
	if (worker->drop_state != 0) {
		worker->drop_state(block);
	}
}

/// A new worker of a state of `size` bytes (stored by the generated code at
/// `WORKER_STATE`, before `fa_worker_start`).
FA_EXPORT FaWorker *fa_worker_new(int64_t size, void (*drop_state)(void *block)) {
	FaWorker *worker = shared_new(WORKER_STATE + (size_t)size, worker_drop);
	worker->drop_state = drop_state;
	return worker;
}

/// Starts the thread of a worker (none without threads: its calls run on
/// the caller's thread).
FA_EXPORT void fa_worker_start(FaWorker *worker) {
	(void)worker;
	if (!has_threads()) {
		return;
	}
#if !defined(__wasm__)
	worker->started = 1;
	__atomic_fetch_add(&workers_alive, 1, __ATOMIC_ACQ_REL);
	pthread_create(&worker->thread, 0, worker_thread, worker);
#endif
}

/// A new call whose frame takes `frame_size` bytes: its frame (the
/// generated code fills its resume and destroy functions and its header).
FA_EXPORT void *fa_call_new(int64_t frame_size) {
	size_t size = WORKER_FRAME + (size_t)frame_size;
	FaCall *call = fa_alloc(size, 16);
	__builtin_memset(call, 0, WORKER_FRAME + TASK_AT + sizeof(FaTask));
	call->size = (int64_t)size;
	return (uint8_t *)call + WORKER_FRAME;
}

/// Frees a call (from `dispose`, after its result was dropped).
FA_EXPORT void fa_call_free(void *frame) {
	FaCall *call = call_of(frame);
	fa_free(call, (size_t)call->size, 16);
}

/// Queues a call of `worker` (`frame` from `fa_call_new`). `mode`: 0 a
/// blocking caller, which waits here for the end of the call; 1 an awaited
/// or kept task; 2 a detached task.
FA_EXPORT void fa_worker_call(FaWorker *worker, void *frame,
                              void (*run)(void *state, void *arguments, void *frame),
                              void *arguments, void (*drop_arguments)(void *arguments),
                              void (*dispose)(void *frame), int32_t mode) {
	FaCall *call = call_of(frame);
	call->worker = worker;
	call->run = run;
	call->arguments = arguments;
	call->drop_arguments = drop_arguments;
	call->dispose = dispose;
	call->next = 0;
	fa_shared_retain(worker);
	FaTask *task = task_of(frame);
	if (!worker->started || worker->stopped || worker == current_worker) {
		// no thread (or the worker's own thread, running a pool's job): the call runs now
		call->state = CALL_RUNNING;
		int32_t depth = fa_blocking_depth;
		fa_blocking_depth = 1;
		run((uint8_t *)worker + WORKER_STATE, arguments, frame);
		fa_blocking_depth = depth;
		task->flags |= TASK_RESULT;
		call->state = CALL_DONE;
		*(void **)frame = 0;
		set_task_state(task, TASK_DONE);
		if (mode == 2) {
			call->dispose(frame);
			fa_shared_release(worker);
		}
		return;
	}
	if (mode != 0) {
		FaExecutor *e = executor();
		call->owner = e;
		fa_remote_source_start(e);
		set_task_state(task, TASK_WAITING);
		if (mode == 2) {
			task->flags |= TASK_DETACHED;
			e->detached += 1;
		}
	}
	system_lock(&worker->lock);
	call->state = CALL_QUEUED;
	if (worker->tail != 0) {
		worker->tail->next = call;
	} else {
		worker->head = call;
	}
	worker->tail = call;
	pthread_cond_broadcast(&worker->changed);
	if (mode == 0) {
		while (call->state != CALL_DONE) {
			pthread_cond_wait(&worker->changed, &worker->lock);
		}
		*(void **)frame = 0;
		set_task_state(task, TASK_DONE);
	}
	system_unlock(&worker->lock);
}

/// The resume of a call's frame (from its generated resume function): the
/// call ended, on the caller's thread: the task ends.
FA_EXPORT void fa_call_resume(void *frame) {
	*(void **)frame = 0;
	fa_task_finish(frame);
	// `fa_task_finish` counts a detached task's end on this executor
}

/// The destroy of a call's frame (its task dropped, or awaited and its
/// result taken): see the cancellation above.
FA_EXPORT void fa_call_destroy(void *frame) {
	FaCall *call = call_of(frame);
	FaWorker *worker = call->worker;
	if (worker == 0 || !worker->started) {
		call->dispose(frame);
		fa_shared_release(worker);
		return;
	}
	system_lock(&worker->lock);
	if (call->state == CALL_QUEUED) {
		FaCall **link = &worker->head;
		FaCall *previous = 0;
		while (*link != call) {
			previous = *link;
			link = &(*link)->next;
		}
		*link = call->next;
		if (worker->tail == call) {
			worker->tail = previous;
		}
		system_unlock(&worker->lock);
		call->drop_arguments(call->arguments);
		FaExecutor *owner = call->owner;
		fa_free(call, (size_t)call->size, 16);
		if (owner != 0) {
			fa_remote_source_end(owner);
		}
		fa_shared_release(worker);
		return;
	}
	if (call->state == CALL_RUNNING) {
		// the worker's thread disposes of it
		call->state = CALL_ABANDONED;
		FaExecutor *owner = call->owner;
		system_unlock(&worker->lock);
		if (owner != 0) {
			fa_remote_source_end(owner);
		}
		fa_shared_release(worker);
		return;
	}
	system_unlock(&worker->lock);
	// done: its wake may still be in the remote queue (`fa_task_drop`
	// took it out of the ready queue already)
	FaExecutor *e = current_executor;
	FaTask *task = task_of(frame);
	if (e != 0 && call->owner == e && task_state(task) == TASK_REMOTE) {
		drain_remote(e);
		if (task_state(task) == TASK_QUEUED) {
			ready_unlink(e, task);
		}
	}
	call->dispose(frame);
	fa_shared_release(worker);
}

// ── worker pools (`WorkerPool(T)`: `design/decisions.md`, `src/lower/tasks.rs`)
//
// A pool is a block of the handles (`FaWorkerPool`, then the pool's copy of
// the workers' state at `POOL_STATE`) holding its workers and a queue of
// jobs under a lock. The workers' threads pull the jobs themselves: a job
// is the submitted function (its closure) run with the worker, on the
// worker's thread (its calls of the worker's methods run in place), and a
// frame laid out like a task's, which is the job's task: the caller's
// executor ends it through its remote queue.
//
// - **The queue** counts the jobs submitted that no worker took yet; with a
//   `maxQueueSize`, a blocking `submit` waits on a condition variable, an
//   awaited one in a line of the pool (woken remotely), until a worker
//   takes a job.
// - **A timeout** is a timer of the caller's executor: when it fires while
//   the job ran longer than `timeout`, the job is abandoned (its result,
//   when the worker ends it, dropped there), the task ends with
//   `WorkerTimeout`, and the worker is replaced by a new one made from a
//   fresh copy of the state (`spawn`); the old one is detached: it ends the
//   job, then its thread exits and frees it.
// - **A job** has two owners, the task and the worker's side, counted
//   (`refs`); its result is written by the worker in a scratch area, copied
//   into the promise when the job ends in time.

#define POOL_STATE 384

typedef struct FaWorkerPool {
	FaShared shared;
	FaSystemLock lock;
	/// Signalled when a job is queued, and when the pool stops.
	FaSystemLock jobs_changed;
	/// Signalled when a worker takes a job (room for a blocking `submit`).
	FaSystemLock room_changed;
	FaWorker **workers;
	int64_t count;
	FaCall *head;
	FaCall *tail;
	/// The jobs queued (submitted, not taken), and `maxQueueSize` (0: no limit).
	int64_t queued;
	int64_t max_queue;
	/// `timeout` in milliseconds (0: none).
	int64_t timeout;
	/// The awaited `submit`s waiting for room (`FaWait`, circular).
	FaWait *room;
	/// A new worker of the pool, made from a copy of the state (generated).
	FaWorker *(*spawn)(void *pool);
	/// Drops the pool's copy of the state, given the block.
	void (*drop_state)(void *block);
	int32_t shut;
} FaWorkerPool;

_Static_assert(sizeof(FaWorkerPool) <= POOL_STATE, "the header of a pool");

static void workers_shut(FaWorkerPool *pool) {
	if (pool->shut) {
		return;
	}
	system_lock(&pool->lock);
	pool->shut = 1;
	pthread_cond_broadcast(&pool->jobs_changed);
	system_unlock(&pool->lock);
	for (int64_t index = 0; index < pool->count; index += 1) {
		fa_worker_stop(pool->workers[index]);
	}
}

static void workers_drop(void *block) {
	FaWorkerPool *pool = block;
	workers_shut(pool);
	for (int64_t index = 0; index < pool->count; index += 1) {
		fa_shared_release(pool->workers[index]);
	}
	fa_free(pool->workers, (size_t)pool->count * sizeof(void *), sizeof(void *));
	if (pool->drop_state != 0) {
		pool->drop_state(block);
	}
}

/// A new pool of `count` workers (made by `fa_workers_fill`, once the
/// state is stored at `POOL_STATE`); `timeout` in milliseconds, 0 for none.
FA_EXPORT FaWorkerPool *fa_workers_new(int64_t count, int64_t timeout, int64_t max_queue,
                                       int64_t state_size, void (*drop_state)(void *block),
                                       FaWorker *(*spawn)(void *pool)) {
	if (count < 1) {
		count = 1;
	}
	FaWorkerPool *pool = shared_new(POOL_STATE + (size_t)state_size, workers_drop);
	pool->drop_state = drop_state;
	pool->spawn = spawn;
	pool->timeout = timeout > 0 ? timeout : 0;
	pool->max_queue = max_queue > 0 ? max_queue : 0;
	pool->workers = fa_alloc((size_t)count * sizeof(void *), sizeof(void *));
	pool->count = count;
	return pool;
}

/// Starts the workers of a pool.
FA_EXPORT void fa_workers_fill(FaWorkerPool *pool) {
	for (int64_t index = 0; index < pool->count; index += 1) {
		pool->workers[index] = pool->spawn(pool);
	}
}

/// A worker took a job (the pool's lock held): its place in the queue goes
/// to the first awaited `submit` waiting for room, or is freed.
static void workers_room(FaWorkerPool *pool) {
	FaWait *waiting = line_pop(&pool->room);
	if (waiting != 0) {
		waiting->status = WAIT_DONE;
		wake(waiting);
		return;
	}
	pool->queued -= 1;
	pthread_cond_broadcast(&pool->room_changed);
}

/// An awaited `submit` for the task of `frame`: a place in the queue
/// (`AWAIT_READY`), or it waits in `wait` until one is handed over.
FA_EXPORT int32_t fa_workers_reserve(FaWorkerPool *pool, FaWait *wait, void *frame) {
	executor();
	system_lock(&pool->lock);
	if (pool->max_queue == 0 || pool->queued < pool->max_queue) {
		pool->queued += 1;
		system_unlock(&pool->lock);
		return AWAIT_READY;
	}
	line_push(&pool->room, wait, frame, "WorkerPool.submit (the queue is full)");
	system_unlock(&pool->lock);
	return AWAIT_SUSPEND;
}

/// A blocking `submit`: a place in the queue, waiting on the calling thread
/// while it is full (the workers take the jobs themselves).
FA_EXPORT void fa_workers_reserve_now(FaWorkerPool *pool) {
	system_lock(&pool->lock);
	while (pool->max_queue > 0 && pool->queued >= pool->max_queue) {
		pthread_cond_wait(&pool->room_changed, &pool->lock);
	}
	pool->queued += 1;
	system_unlock(&pool->lock);
}

/// The destroy edge of a `submit` waiting for room.
FA_EXPORT void fa_workers_unreserve(FaWorkerPool *pool, FaWait *wait) {
	system_lock(&pool->lock);
	if (wait->status == WAIT_DONE) {
		workers_room(pool);
	} else {
		line_remove(&pool->room, wait);
	}
	system_unlock(&pool->lock);
}

static void *job_frame(FaCall *job) {
	return (uint8_t *)job + WORKER_FRAME;
}

/// Releases one owner of a job: the last one drops its result (unless
/// taken) and frees it.
static void job_release(FaCall *job) {
	if (__atomic_fetch_sub(&job->refs, 1, __ATOMIC_ACQ_REL) == 1) {
		job->dispose(job_frame(job));
	}
}

#if !defined(__wasm__)
static void *pool_worker_thread(void *argument) {
	FaWorker *worker = argument;
	FaWorkerPool *pool = worker->pool;
	current_worker = worker;
	// every job is a blocking call of the submitted function
	fa_blocking_depth = 1;
	system_lock(&pool->lock);
	for (;;) {
		while (pool->head == 0 && !worker->stopping && !pool->shut) {
			pthread_cond_wait(&pool->jobs_changed, &pool->lock);
		}
		FaCall *job = pool->head;
		if (job == 0 || worker->stopping) {
			break;
		}
		pool->head = job->next;
		if (pool->head == 0) {
			pool->tail = 0;
		}
		job->state = CALL_RUNNING;
		job->worker = worker;
		job->started_at = fa_clock();
		workers_room(pool);
		system_unlock(&pool->lock);
		job->run(worker, job->arguments, job->scratch);
		system_lock(&pool->lock);
		if (job->state == CALL_ABANDONED) {
			// cancelled, or timed out: its result goes
			system_unlock(&pool->lock);
			job->drop_result(job->scratch);
			job_release(job);
			system_lock(&pool->lock);
			continue;
		}
		void *frame = job_frame(job);
		copy_bytes((uint8_t *)frame + TASK_AT + sizeof(FaTask), job->scratch,
		           (size_t)job->raw_size);
		task_of(frame)->flags |= TASK_RESULT;
		job->state = CALL_DONE;
		FaExecutor *owner = job->owner;
		remote_push(owner, task_of(frame));
		fa_remote_source_end(owner);
		job_release(job);
	}
	int32_t detached = worker->detached;
	system_unlock(&pool->lock);
	fa_release_builder();
	if (detached) {
		__atomic_fetch_sub(&workers_alive, 1, __ATOMIC_ACQ_REL);
		fa_shared_release(worker);
		fa_shared_release(pool);
	}
	return 0;
}
#endif

/// Starts the thread of a worker of a pool (none without threads: its jobs
/// run on the submitting thread).
FA_EXPORT void fa_worker_start_in_pool(FaWorker *worker, FaWorkerPool *pool) {
	worker->pool = pool;
	if (!has_threads()) {
		return;
	}
#if !defined(__wasm__)
	worker->started = 1;
	__atomic_fetch_add(&workers_alive, 1, __ATOMIC_ACQ_REL);
	pthread_create(&worker->thread, 0, pool_worker_thread, worker);
#endif
}

/// A new job whose frame takes `frame_size` bytes and whose result
/// `raw_size`: its frame.
FA_EXPORT void *fa_job_new(int64_t frame_size, int64_t raw_size) {
	size_t frame = ((size_t)frame_size + 15) & ~(size_t)15;
	size_t scratch = ((size_t)raw_size + 15) & ~(size_t)15;
	size_t size = WORKER_FRAME + frame + scratch + sizeof(FaWait);
	FaCall *job = fa_alloc(size, 16);
	__builtin_memset(job, 0, WORKER_FRAME + TASK_AT + sizeof(FaTask));
	job->size = (int64_t)size;
	job->scratch = (uint8_t *)job + WORKER_FRAME + frame;
	job->timer = (FaWait *)((uint8_t *)job->scratch + scratch);
	job->timer->index = -1;
	job->timer->status = WAIT_CLOSED;
	job->raw_size = raw_size;
	job->refs = 2;
	return (uint8_t *)job + WORKER_FRAME;
}

static void job_timer_fired(FaWait *timer);

/// Arms (or arms again) the timer of a job, at `deadline`.
static void job_arm(FaCall *job, int64_t deadline) {
	FaWait *timer = job->timer;
	timer->frame = job_frame(job);
	timer->item = job;
	timer->fired = job_timer_fired;
	timer->status = WAIT_WAITING;
	timer_insert(job->owner, timer, deadline);
}

static void job_disarm(FaCall *job) {
	FaWait *timer = job->timer;
	if (timer->status == WAIT_WAITING && timer->index >= 0) {
		timer_remove(timer->executor, timer->index);
	}
	timer->status = WAIT_CLOSED;
}

/// Queues a job (`frame` from `fa_job_new`; its place in the queue was
/// reserved): the closure (`arguments`) runs with a free worker of the pool.
FA_EXPORT void fa_workers_enqueue(FaWorkerPool *pool, void *frame,
                                  void (*run)(void *worker, void *closure, void *result),
                                  void *closure, void (*drop_closure)(void *closure),
                                  void (*dispose)(void *frame), void (*timeout_error)(void *frame),
                                  void (*drop_result)(void *result)) {
	FaCall *job = call_of(frame);
	FaExecutor *e = executor();
	job->pool = pool;
	job->run = (void (*)(void *, void *, void *))run;
	job->arguments = closure;
	job->drop_arguments = drop_closure;
	job->dispose = dispose;
	job->timeout_error = timeout_error;
	job->drop_result = drop_result;
	job->owner = e;
	fa_shared_retain(pool);
	FaTask *task = task_of(frame);
	if (!has_threads()) {
		// no threads: the job runs now, with the first worker
		run(pool->workers[0], closure, job->scratch);
		copy_bytes((uint8_t *)frame + TASK_AT + sizeof(FaTask), job->scratch,
		           (size_t)job->raw_size);
		task->flags |= TASK_RESULT;
		job->state = CALL_DONE;
		*(void **)frame = 0;
		set_task_state(task, TASK_DONE);
		pool->queued -= 1;
		job->refs = 1;
		return;
	}
	fa_remote_source_start(e);
	set_task_state(task, TASK_WAITING);
	system_lock(&pool->lock);
	job->state = CALL_QUEUED;
	job->next = 0;
	if (pool->tail != 0) {
		pool->tail->next = job;
	} else {
		pool->head = job;
	}
	pool->tail = job;
	pthread_cond_signal(&pool->jobs_changed);
	system_unlock(&pool->lock);
	if (pool->timeout > 0) {
		job_arm(job, fa_clock() + pool->timeout * 1000000);
	}
}

/// The timer of a job (the caller's thread): while the job is queued, or
/// ran for less than the timeout, it is armed again; else the job times out.
static void job_timer_fired(FaWait *timer) {
	FaCall *job = timer->item;
	FaWorkerPool *pool = job->pool;
	int64_t now = fa_clock();
	int64_t limit = pool->timeout * 1000000;
	timer->status = WAIT_CLOSED;
	system_lock(&pool->lock);
	if (job->state == CALL_QUEUED) {
		system_unlock(&pool->lock);
		job_arm(job, now + limit);
		return;
	}
	if (job->state != CALL_RUNNING) {
		system_unlock(&pool->lock);
		return;
	}
	if (now - job->started_at < limit) {
		system_unlock(&pool->lock);
		job_arm(job, job->started_at + limit);
		return;
	}
	// timed out: abandoned to its worker, which is replaced
	job->state = CALL_ABANDONED;
	FaWorker *old = job->worker;
	system_unlock(&pool->lock);
	void *frame = job_frame(job);
	job->timeout_error(frame);
	task_of(frame)->flags |= TASK_RESULT;
	fa_remote_source_end(job->owner);
	FaWorker *fresh = pool->spawn(pool);
	for (int64_t index = 0; index < pool->count; index += 1) {
		if (pool->workers[index] == old) {
			pool->workers[index] = fresh;
		}
	}
	// the old worker ends the job, then its thread exits and frees it
	fa_shared_retain(old);
	fa_shared_retain(pool);
	system_lock(&pool->lock);
	old->detached = 1;
	old->stopping = 1;
	pthread_cond_broadcast(&pool->jobs_changed);
	system_unlock(&pool->lock);
	old->stopped = 1;
#if !defined(__wasm__)
	if (pthread_detach != 0) {
		pthread_detach(old->thread);
	}
#endif
	fa_shared_release(old);
	ready_push(job->owner, task_of(frame));
}

/// The resume of a job's frame (its generated resume function): the job
/// ended, or timed out: the task ends.
FA_EXPORT void fa_job_resume(void *frame) {
	job_disarm(call_of(frame));
	*(void **)frame = 0;
	fa_task_finish(frame);
}

/// The destroy of a job's frame (its task dropped, or its result taken).
FA_EXPORT void fa_job_destroy(void *frame) {
	FaCall *job = call_of(frame);
	FaWorkerPool *pool = job->pool;
	job_disarm(job);
	system_lock(&pool->lock);
	switch (job->state) {
	case CALL_QUEUED: {
		FaCall **link = &pool->head;
		FaCall *previous = 0;
		while (*link != job) {
			previous = *link;
			link = &(*link)->next;
		}
		*link = job->next;
		if (pool->tail == job) {
			pool->tail = previous;
		}
		workers_room(pool);
		system_unlock(&pool->lock);
		job->drop_arguments(job->arguments);
		fa_remote_source_end(job->owner);
		job_release(job);
		break;
	}
	case CALL_RUNNING:
		// its worker drops the result
		job->state = CALL_ABANDONED;
		system_unlock(&pool->lock);
		fa_remote_source_end(job->owner);
		break;
	default: {
		// done (its wake may still be in the remote queue), or timed out
		system_unlock(&pool->lock);
		FaExecutor *e = current_executor;
		FaTask *task = task_of(frame);
		if (e != 0 && job->owner == e && task_state(task) == TASK_REMOTE) {
			drain_remote(e);
			if (task_state(task) == TASK_QUEUED) {
				ready_unlink(e, task);
			}
		}
		break;
	}
	}
	job_release(job);
	fa_shared_release(pool);
}

/// `pool.shutdown()`: the workers take the queued jobs, then stop.
FA_EXPORT void fa_workers_shutdown(FaWorkerPool *pool) {
	workers_shut(pool);
}

/// `WorkerTimeout`.
static const BareError worker_timeout = {WORKER_TIMEOUT};

FA_EXPORT FA_COLD const void *fa_error_worker_timeout(void) {
	return &worker_timeout;
}

// ── mutexes

typedef struct {
	FaShared shared;
	int32_t locked;
	/// The threads waiting blocking (on `released`).
	int32_t waiting;
	/// The tasks waiting, in order (`FaWait`, circular): `unlock` hands
	/// the lock over to the first one.
	FaWait *waiters;
	FaSystemLock lock;
	FaSystemLock released;
	/// The value (`FA_MUTEX_VALUE` in `src/lower/parallel.rs`).
	_Alignas(16) uint8_t value[];
} FaMutex;

_Static_assert(__builtin_offsetof(FaMutex, value) == 176, "the offset of a mutex's value");

/// A new mutex holding `size` bytes of value, dropped by `drop` (a
/// function of the block): the generated code writes the value.
FA_EXPORT void *fa_mutex_new(int64_t size, void (*drop)(void *)) {
	return shared_new(sizeof(FaMutex) + (size_t)size, drop);
}

/// `lock()`: waits until the mutex is free, and takes it. The guard keeps
/// the block alive (a handle).
FA_EXPORT void *fa_mutex_lock(void *block) {
	FaMutex *mutex = block;
	fa_shared_retain(block);
	if (__atomic_exchange_n(&mutex->locked, 1, __ATOMIC_ACQUIRE) == 0) {
		return block;
	}
	if (!others_may_wake()) {
		deadlock("`lock()` on a mutex this thread holds");
	}
	system_lock(&mutex->lock);
	mutex->waiting += 1;
	while (__atomic_exchange_n(&mutex->locked, 1, __ATOMIC_ACQUIRE) != 0) {
		pthread_cond_wait(&mutex->released, &mutex->lock);
	}
	mutex->waiting -= 1;
	system_unlock(&mutex->lock);
	return block;
}

/// `await mutex.lock()` in the task of `frame`: `AWAIT_READY` when taken
/// (the guard is the block), `AWAIT_SUSPEND` when the task waits in line in
/// `wait` (the lock is handed over to it, `WAIT_DONE`), `AWAIT_YIELD` when
/// taken but the coop budget is spent.
FA_EXPORT int32_t fa_mutex_lock_async(void *block, FaWait *wait, void *frame, const char *site) {
	FaMutex *mutex = block;
	fa_shared_retain(block);
	system_lock(&mutex->lock);
	if (mutex->waiters == 0 && __atomic_exchange_n(&mutex->locked, 1, __ATOMIC_ACQUIRE) == 0) {
		system_unlock(&mutex->lock);
		wait->status = WAIT_DONE;
		wait->remote = 0;
		if (coop_spent()) {
			fa_task_yield(frame);
			return AWAIT_YIELD;
		}
		return AWAIT_READY;
	}
	line_push(&mutex->waiters, wait, frame, site);
	system_unlock(&mutex->lock);
	return AWAIT_SUSPEND;
}

/// `tryLock()`: the guard, or null when the mutex is locked.
FA_EXPORT void *fa_mutex_try_lock(void *block) {
	FaMutex *mutex = block;
	if (__atomic_exchange_n(&mutex->locked, 1, __ATOMIC_ACQUIRE) != 0) {
		return 0;
	}
	return fa_shared_retain(block);
}

/// The end of a guard: hands the lock over to the first task waiting, or
/// releases it; then releases the guard's handle.
FA_EXPORT void fa_mutex_unlock(void *block) {
	if (block == 0) {
		return;
	}
	FaMutex *mutex = block;
	system_lock(&mutex->lock);
	FaWait *next = line_pop(&mutex->waiters);
	if (next != 0) {
		// still locked: the lock is the waiting task's now
		next->status = WAIT_DONE;
		wake(next);
	} else {
		__atomic_store_n(&mutex->locked, 0, __ATOMIC_RELEASE);
		if (mutex->waiting > 0) {
			pthread_cond_signal(&mutex->released);
		}
	}
	system_unlock(&mutex->lock);
	fa_shared_release(block);
}

/// The destroy edge of `await mutex.lock()`: leaves the line, or releases
/// the lock handed over.
FA_EXPORT void fa_mutex_cancel(void *block, FaWait *wait) {
	FaMutex *mutex = block;
	system_lock(&mutex->lock);
	if (wait->status == WAIT_WAITING) {
		line_remove(&mutex->waiters, wait);
		system_unlock(&mutex->lock);
		fa_shared_release(block);
		return;
	}
	system_unlock(&mutex->lock);
	forget_remote_wake(wait);
	fa_mutex_unlock(block);
}

// ── channels

typedef struct {
	FaShared shared;
	FaSystemLock lock;
	FaSystemLock changed;
	/// At most this many values.
	int64_t capacity;
	int64_t item_size;
	/// The ring of values: `length` values from `head`, `room` places.
	uint8_t *items;
	int64_t head;
	int64_t length;
	int64_t room;
	int32_t closed;
	/// The threads waiting blocking (on `changed`).
	int32_t waiting;
	/// Drops a value (called with a pointer to it), or null.
	void (*drop_item)(void *item);
	/// The tasks waiting to send (their value in `item`) and to receive
	/// (their place in `item`), in order.
	FaWait *senders;
	FaWait *receivers;
} FaChannel;

static void channel_drop(void *block) {
	FaChannel *channel = block;
	for (int64_t index = 0; index < channel->length; index += 1) {
		int64_t at = (channel->head + index) % channel->room;
		if (channel->drop_item != 0) {
			channel->drop_item(channel->items + at * channel->item_size);
		}
	}
	if (channel->room > 0) {
		fa_free(channel->items, (size_t)(channel->room * channel->item_size), 16);
	}
}

/// `Channel(T) { capacity = n }` (`capacity` < 0: unbounded).
FA_EXPORT void *fa_channel_new(int64_t capacity, int64_t item_size, void (*drop_item)(void *)) {
	FaChannel *channel = shared_new(sizeof(FaChannel), channel_drop);
	channel->capacity = capacity < 0 ? (int64_t)(UINT64_MAX_VALUE >> 1) : capacity;
	channel->item_size = item_size < 1 ? 1 : item_size;
	channel->drop_item = drop_item;
	return channel;
}

/// Appends a value (the lock held, room checked).
static void channel_push(FaChannel *channel, const void *item) {
	if (channel->length == channel->room) {
		int64_t room = channel->room < 4 ? 4 : channel->room * 2;
		uint8_t *items = fa_alloc((size_t)(room * channel->item_size), 16);
		if (items == 0) {
			out_of_memory();
		}
		for (int64_t index = 0; index < channel->length; index += 1) {
			int64_t at = (channel->head + index) % channel->room;
			copy_bytes(items + index * channel->item_size, channel->items + at * channel->item_size,
			           (size_t)channel->item_size);
		}
		if (channel->room > 0) {
			fa_free(channel->items, (size_t)(channel->room * channel->item_size), 16);
		}
		channel->items = items;
		channel->head = 0;
		channel->room = room;
	}
	int64_t at = (channel->head + channel->length) % channel->room;
	copy_bytes(channel->items + at * channel->item_size, item, (size_t)channel->item_size);
	channel->length += 1;
}

/// Takes the first value into `out` (the lock held, a value there).
static void channel_pop(FaChannel *channel, void *out) {
	copy_bytes(out, channel->items + channel->head * channel->item_size, (size_t)channel->item_size);
	channel->head = (channel->head + 1) % channel->room;
	channel->length -= 1;
}

static void channel_changed(FaChannel *channel) {
	if (channel->waiting > 0) {
		pthread_cond_broadcast(&channel->changed);
	}
}

/// Sends a value now if a task waits for one or there is room (the lock
/// held): 1 when sent.
static int channel_send_now(FaChannel *channel, const void *item) {
	FaWait *receiver = line_pop(&channel->receivers);
	if (receiver != 0) {
		copy_bytes(receiver->item, item, (size_t)channel->item_size);
		receiver->status = WAIT_DONE;
		wake(receiver);
		return 1;
	}
	if (channel->length < channel->capacity) {
		channel_push(channel, item);
		channel_changed(channel);
		return 1;
	}
	return 0;
}

/// Receives a value now if there is one or a task waits to send one (the
/// lock held): 1 when received into `out`.
static int channel_receive_now(FaChannel *channel, void *out) {
	if (channel->length > 0) {
		channel_pop(channel, out);
		// the first task waiting to send takes the place freed
		FaWait *sender = line_pop(&channel->senders);
		if (sender != 0) {
			channel_push(channel, sender->item);
			sender->status = WAIT_DONE;
			wake(sender);
		}
		channel_changed(channel);
		return 1;
	}
	FaWait *sender = line_pop(&channel->senders);
	if (sender != 0) {
		copy_bytes(out, sender->item, (size_t)channel->item_size);
		sender->status = WAIT_DONE;
		wake(sender);
		channel_changed(channel);
		return 1;
	}
	return 0;
}

/// `send(value)`, with the value at `item`: 1 when sent (the channel owns
/// it), 0 when the channel is closed (the caller still owns it). With
/// `wait`, waits while the channel is full; else returns 0 when full.
FA_EXPORT int32_t fa_channel_send(void *block, const void *item, int32_t wait) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	for (;;) {
		if (channel->closed) {
			system_unlock(&channel->lock);
			return 0;
		}
		if (channel_send_now(channel, item)) {
			system_unlock(&channel->lock);
			return 1;
		}
		if (!wait) {
			system_unlock(&channel->lock);
			return 0;
		}
		if (!others_may_wake()) {
			deadlock("`send` on a full channel that no other thread receives from");
		}
		channel->waiting += 1;
		pthread_cond_wait(&channel->changed, &channel->lock);
		channel->waiting -= 1;
	}
}

/// `await channel.send(value)` in the task of `frame`, with the value at
/// `item` (in the frame): `AWAIT_READY` or `AWAIT_YIELD` when sent,
/// `AWAIT_CLOSED`, or `AWAIT_SUSPEND` when the task waits in line in `wait`
/// (`WAIT_DONE`: sent, `WAIT_CLOSED`: the channel was closed).
FA_EXPORT int32_t fa_channel_send_async(void *block, void *item, FaWait *wait, void *frame,
                                        const char *site) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	if (channel->closed) {
		system_unlock(&channel->lock);
		return AWAIT_CLOSED;
	}
	if (channel_send_now(channel, item)) {
		system_unlock(&channel->lock);
		wait->status = WAIT_DONE;
		wait->remote = 0;
		if (coop_spent()) {
			fa_task_yield(frame);
			return AWAIT_YIELD;
		}
		return AWAIT_READY;
	}
	wait->item = item;
	line_push(&channel->senders, wait, frame, site);
	system_unlock(&channel->lock);
	return AWAIT_SUSPEND;
}

/// `receive()`: 1 with the next value in `out`, 0 once the channel is
/// closed and empty. With `wait`, waits for a value; else returns 0 when
/// none is ready.
FA_EXPORT int32_t fa_channel_receive(void *block, void *out, int32_t wait) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	for (;;) {
		if (channel_receive_now(channel, out)) {
			system_unlock(&channel->lock);
			return 1;
		}
		if (!wait || channel->closed) {
			system_unlock(&channel->lock);
			return 0;
		}
		if (!others_may_wake()) {
			deadlock("`receive` on an empty channel that no other thread sends to");
		}
		channel->waiting += 1;
		pthread_cond_wait(&channel->changed, &channel->lock);
		channel->waiting -= 1;
	}
}

/// `await channel.receive()` in the task of `frame`, the value going to
/// `out` (in the frame): as `fa_channel_send_async`.
FA_EXPORT int32_t fa_channel_receive_async(void *block, void *out, FaWait *wait, void *frame,
                                           const char *site) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	if (channel_receive_now(channel, out)) {
		system_unlock(&channel->lock);
		wait->status = WAIT_DONE;
		wait->remote = 0;
		if (coop_spent()) {
			fa_task_yield(frame);
			return AWAIT_YIELD;
		}
		return AWAIT_READY;
	}
	if (channel->closed) {
		system_unlock(&channel->lock);
		return AWAIT_CLOSED;
	}
	wait->item = out;
	line_push(&channel->receivers, wait, frame, site);
	system_unlock(&channel->lock);
	return AWAIT_SUSPEND;
}

/// The destroy edge of an `await` of `send` or `receive`: leaves the line.
/// Returns the status of the wait: `WAIT_DONE` when the value was handed
/// over (a value received must be dropped by the caller).
FA_EXPORT int32_t fa_channel_cancel(void *block, FaWait *wait) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	int32_t status = wait->status;
	if (status == WAIT_WAITING) {
		// in one of the two lines
		FaWait **line = &channel->senders;
		for (FaWait *at = channel->receivers; at != 0;) {
			if (at == wait) {
				line = &channel->receivers;
				break;
			}
			at = at->next;
			if (at == channel->receivers) {
				break;
			}
		}
		line_remove(line, wait);
	}
	system_unlock(&channel->lock);
	if (status != WAIT_WAITING) {
		forget_remote_wake(wait);
	}
	return status;
}

/// The status of a wait after its task resumed (`WAIT_DONE` or `WAIT_CLOSED`).
FA_EXPORT int32_t fa_wait_status(FaWait *wait) {
	return __atomic_load_n(&wait->status, __ATOMIC_ACQUIRE);
}

/// `close()`: no value can be sent anymore; the tasks waiting to receive
/// get nothing, those waiting to send keep their value.
FA_EXPORT void fa_channel_close(void *block) {
	FaChannel *channel = block;
	system_lock(&channel->lock);
	channel->closed = 1;
	FaWait *wait;
	while ((wait = line_pop(&channel->receivers)) != 0) {
		wait->status = WAIT_CLOSED;
		wake(wait);
	}
	while ((wait = line_pop(&channel->senders)) != 0) {
		wait->status = WAIT_CLOSED;
		wake(wait);
	}
	channel_changed(channel);
	system_unlock(&channel->lock);
}

// ── for parallel

#if !defined(__wasm__)

/// Runs the iterations `[start, end)`; returns the error of the first one
/// that throws (and its index in `failed`), or null.
typedef void *(*FaParallelBody)(void *environment, int64_t start, int64_t end, int64_t *failed);

typedef struct {
	FaParallelBody body;
	void *environment;
	int64_t count;
	int64_t chunk;
	/// The start of the next chunk (atomic).
	int64_t next;
	/// An iteration failed: the chunks not started are skipped (atomic).
	int32_t failed;
	/// The threads of the pool that may join, and those that did (atomic).
	int32_t slots;
	int32_t joined;
	int32_t error_lock;
	void *error;
	int64_t error_index;
	void (*drop_error)(void *error);
} ParallelJob;

static struct {
	FaSystemLock lock;
	FaSystemLock wake;
	/// The job being run (atomic), its generation (under `lock`).
	ParallelJob *job;
	int64_t generation;
	/// The threads of the pool looking at `job` (atomic).
	int32_t busy;
	/// A parallel loop is running (atomic): another one runs on its thread.
	int32_t running;
	int32_t started;
} pool;

/// Set in the threads of the pool, and in a thread running a parallel loop.
static FA_THREAD_LOCAL int32_t in_parallel;

static void record_error(ParallelJob *job, void *error, int64_t index) {
	__atomic_store_n(&job->failed, 1, __ATOMIC_RELAXED);
	spin_lock(&job->error_lock);
	void *dropped = error;
	if (job->error == 0 || index < job->error_index) {
		dropped = job->error;
		job->error = error;
		job->error_index = index;
	}
	spin_unlock(&job->error_lock);
	if (dropped != 0 && job->drop_error != 0) {
		job->drop_error(dropped);
	}
}

static void run_chunks(ParallelJob *job) {
	for (;;) {
		if (__atomic_load_n(&job->failed, __ATOMIC_RELAXED)) {
			return;
		}
		int64_t start = __atomic_fetch_add(&job->next, job->chunk, __ATOMIC_RELAXED);
		if (start >= job->count) {
			return;
		}
		int64_t end = job->count - start < job->chunk ? job->count : start + job->chunk;
		int64_t failed = start;
		void *error = job->body(job->environment, start, end, &failed);
		if (error != 0) {
			record_error(job, error, failed);
		}
	}
}

static void *pool_thread(void *unused) {
	(void)unused;
	in_parallel = 1;
	int64_t seen = 0;
	for (;;) {
		pthread_mutex_lock(&pool.lock);
		while (pool.generation == seen) {
			pthread_cond_wait(&pool.wake, &pool.lock);
		}
		seen = pool.generation;
		pthread_mutex_unlock(&pool.lock);
		__atomic_fetch_add(&pool.busy, 1, __ATOMIC_SEQ_CST);
		ParallelJob *job = __atomic_load_n(&pool.job, __ATOMIC_SEQ_CST);
		if (job != 0 && __atomic_fetch_add(&job->joined, 1, __ATOMIC_RELAXED) < job->slots) {
			run_chunks(job);
			// the blocks of this thread are counted by leak checks
			fa_release_builder();
		}
		__atomic_fetch_sub(&pool.busy, 1, __ATOMIC_SEQ_CST);
	}
	return 0;
}

/// The number of processors (1 when unknown).
static int64_t processors(void) {
	long count = sysconf != 0 ? sysconf(84 /* _SC_NPROCESSORS_ONLN */) : 1;
	return count < 1 ? 1 : count;
}

/// `for parallel` over `count` iterations, on at most `threads` threads
/// (0: one per processor). Returns the error of the failed iteration with
/// the smallest index, the others dropped with `drop_error`; null when
/// every iteration ran.
FA_EXPORT void *fa_parallel_for(int64_t count, FaParallelBody body, void *environment,
                                int64_t threads, void (*drop_error)(void *)) {
	if (count <= 0) {
		return 0;
	}
	int64_t failed = 0;
	if (!has_threads() || count == 1 || in_parallel) {
		return body(environment, 0, count, &failed);
	}
	int64_t available = processors();
	if (threads <= 0 || threads > available) {
		threads = available;
	}
	if (threads <= 1 || __atomic_exchange_n(&pool.running, 1, __ATOMIC_ACQUIRE) != 0) {
		return body(environment, 0, count, &failed);
	}
	if (!pool.started) {
		pool.started = 1;
		for (int64_t index = 1; index < available; index += 1) {
			unsigned long thread;
			pthread_create(&thread, 0, pool_thread, 0);
		}
	}
	ParallelJob job;
	__builtin_memset(&job, 0, sizeof job);
	job.body = body;
	job.environment = environment;
	job.count = count;
	job.slots = (int32_t)(threads - 1);
	// small chunks balance the work, large ones cost fewer atomic operations
	job.chunk = count / (threads * 16);
	if (job.chunk < 1) {
		job.chunk = 1;
	}
	job.drop_error = drop_error;
	in_parallel = 1;
	__atomic_store_n(&pool.job, &job, __ATOMIC_SEQ_CST);
	pthread_mutex_lock(&pool.lock);
	pool.generation += 1;
	pthread_cond_broadcast(&pool.wake);
	pthread_mutex_unlock(&pool.lock);
	run_chunks(&job);
	__atomic_store_n(&pool.job, 0, __ATOMIC_SEQ_CST);
	// the threads still running a chunk
	while (__atomic_load_n(&pool.busy, __ATOMIC_SEQ_CST) != 0) {
		if (sched_yield != 0) {
			sched_yield();
		}
	}
	in_parallel = 0;
	__atomic_store_n(&pool.running, 0, __ATOMIC_RELEASE);
	return job.error;
}

static int others_may_wake(void) {
	return has_threads() && (in_parallel || __atomic_load_n(&pool.running, __ATOMIC_ACQUIRE) ||
	                         __atomic_load_n(&workers_alive, __ATOMIC_ACQUIRE) > 0);
}

#else

static int others_may_wake(void) {
	// one thread
	return has_threads();
}

#endif

// ── compiler builtins ───────────────────────────────────────────────────

// The 128-bit multiplication and divisions that LLVM lowers to library
// calls: wasm32 has no compiler-rt, and the freestanding target no libgcc.
// Weak, so that the C library's own win on native. Written with 64-bit
// operations only, so that they don't call themselves.

typedef unsigned __int128 u128;
typedef __int128 i128;

static u128 make(uint64_t low, uint64_t high) {
	return ((u128)high << 64) | low;
}

/// The full product of two 64-bit numbers.
static u128 multiply_64(uint64_t a, uint64_t b) {
	uint64_t a_low = (uint32_t)a, a_high = a >> 32;
	uint64_t b_low = (uint32_t)b, b_high = b >> 32;
	uint64_t low_low = a_low * b_low;
	uint64_t high_low = a_high * b_low;
	uint64_t low_high = a_low * b_high;
	uint64_t high_high = a_high * b_high;
	uint64_t middle = (low_low >> 32) + (uint32_t)high_low + (uint32_t)low_high;
	uint64_t low = (middle << 32) | (uint32_t)low_low;
	uint64_t high = high_high + (high_low >> 32) + (low_high >> 32) + (middle >> 32);
	return make(low, high);
}

__attribute__((weak)) i128 __multi3(i128 a, i128 b) {
	uint64_t a_low = (uint64_t)a, a_high = (uint64_t)((u128)a >> 64);
	uint64_t b_low = (uint64_t)b, b_high = (uint64_t)((u128)b >> 64);
	u128 product = multiply_64(a_low, b_low);
	uint64_t high = (uint64_t)(product >> 64) + a_low * b_high + a_high * b_low;
	return (i128)make((uint64_t)product, high);
}

/// Unsigned division with remainder, bit by bit.
static u128 divide_u128(u128 dividend, u128 divisor, u128 *remainder) {
	uint64_t quotient_low = 0, quotient_high = 0;
	uint64_t rest_low = 0, rest_high = 0;
	uint64_t divisor_low = (uint64_t)divisor, divisor_high = (uint64_t)(divisor >> 64);
	uint64_t dividend_low = (uint64_t)dividend, dividend_high = (uint64_t)(dividend >> 64);
	for (int bit = 127; bit >= 0; bit -= 1) {
		// rest = rest << 1 | the next bit of the dividend
		rest_high = (rest_high << 1) | (rest_low >> 63);
		uint64_t next = bit >= 64 ? (dividend_high >> (bit - 64)) & 1 : (dividend_low >> bit) & 1;
		rest_low = (rest_low << 1) | next;
		if (rest_high > divisor_high || (rest_high == divisor_high && rest_low >= divisor_low)) {
			uint64_t borrow = rest_low < divisor_low;
			rest_low -= divisor_low;
			rest_high -= divisor_high + borrow;
			if (bit >= 64) {
				quotient_high |= (uint64_t)1 << (bit - 64);
			} else {
				quotient_low |= (uint64_t)1 << bit;
			}
		}
	}
	*remainder = make(rest_low, rest_high);
	return make(quotient_low, quotient_high);
}

__attribute__((weak)) u128 __udivti3(u128 a, u128 b) {
	u128 remainder;
	return divide_u128(a, b, &remainder);
}

__attribute__((weak)) u128 __umodti3(u128 a, u128 b) {
	u128 remainder;
	divide_u128(a, b, &remainder);
	return remainder;
}

static u128 magnitude(i128 value) {
	return value < 0 ? 0 - (u128)value : (u128)value;
}

__attribute__((weak)) i128 __divti3(i128 a, i128 b) {
	u128 remainder;
	u128 quotient = divide_u128(magnitude(a), magnitude(b), &remainder);
	return (a < 0) != (b < 0) ? (i128)(0 - quotient) : (i128)quotient;
}

__attribute__((weak)) i128 __modti3(i128 a, i128 b) {
	u128 remainder;
	divide_u128(magnitude(a), magnitude(b), &remainder);
	return a < 0 ? (i128)(0 - remainder) : (i128)remainder;
}
