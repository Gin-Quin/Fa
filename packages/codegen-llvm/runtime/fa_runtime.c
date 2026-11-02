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

#if defined(__wasm__)
#define FA_IMPORT(name) __attribute__((import_module("fa"), import_name(name)))
// Writes the ECMAScript `Number::toString` of the value (of `Math.fround`
// for a Float32 printed as one, `kind` 1; of a Percentage, `kind` 2) into
// `out` (at least 32 bytes) and returns its length.
FA_IMPORT("formatFloat") int32_t fa_host_format_float(double value, int32_t kind, uint8_t *out);
// The UTF-8 bytes of the user arguments, each followed by a NUL byte.
FA_IMPORT("argumentsSize") int32_t fa_host_arguments_size(void);
FA_IMPORT("argumentsCopy") void fa_host_arguments_copy(uint8_t *out);
#else
int snprintf(char *buffer, size_t size, const char *format, ...);
double strtod(const char *text, char **end);
#endif

// ── values ──────────────────────────────────────────────────────────────

/// A string: UTF-8 bytes, `{ ptr, usize, usize }` in the generated code.
/// The capacity word holds a flag in its top bit, `SIMPLE`: every byte is
/// ASCII other than '\r', so that every byte is a grapheme cluster and a
/// position is a byte offset. The flag may be missing from a simple string
/// (it is only an optimization), never set on another one. The other bits
/// (`KIND_MASK`) are the size of the heap block that the string owns, 0 for
/// a string that lives as long as the program (a literal, an argument, a
/// character of the ASCII table), and `KIND_MASK` itself for a view of
/// another string's bytes (the variable of a `for` over characters), which
/// a copy must allocate.
typedef struct {
	const uint8_t *bytes;
	size_t length;
	size_t capacity;
} FaString;

#define SIMPLE ((size_t)1 << (sizeof(size_t) * 8 - 1))
#define KIND_MASK (~SIMPLE)

/// Whether a string with this capacity word owns its heap block.
static int string_owned(size_t capacity) {
	size_t kind = capacity & KIND_MASK;
	return kind != 0 && kind != KIND_MASK;
}

/// `SIMPLE` when every byte is ASCII other than '\r', else 0.
static size_t simple_flag(const uint8_t *bytes, size_t length) {
	const uint64_t ones = 0x0101010101010101u;
	uint64_t seen = 0;
	size_t index = 0;
	for (; index + 8 <= length; index += 8) {
		uint64_t word;
		__builtin_memcpy(&word, bytes + index, 8);
		uint64_t cr = word ^ (ones * '\r');
		// the high bit of a byte: not ASCII, or (all bytes being ASCII) a '\r'
		seen |= word | ((cr - ones) & ~cr);
	}
	for (; index < length; index += 1) {
		seen |= bytes[index] == '\r' ? 0x80 : bytes[index];
	}
	return (seen & 0x8080808080808080u) == 0 ? SIMPLE : 0;
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
	string.capacity = byte == '\r' ? 0 : SIMPLE;
	return string;
}

/// Stops the program: no memory left (as `fa_alloc` does).
static FA_NORETURN void out_of_memory(void) {
	static const char message[] = "stop: out of memory\n";
	fa_write(2, (const uint8_t *)message, sizeof(message) - 1);
	fa_exit(101);
}

static void copy_bytes(uint8_t *to, const uint8_t *from, size_t length) {
	// up to 16 bytes (words, characters): two overlapping loads and stores
	// in place, without a call
	if (length <= 16) {
		if (length >= 8) {
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

static size_t c_length(const char *text) {
	size_t length = 0;
	while (text[length] != 0) {
		length += 1;
	}
	return length;
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

// ── the text builder ────────────────────────────────────────────────────

// One builder serves `console.log` lines, interpolations and `toString`:
// the generated code evaluates every part before it starts building, so
// building is never nested.
static FA_THREAD_LOCAL uint8_t *builder;
static FA_THREAD_LOCAL size_t builder_length;
static FA_THREAD_LOCAL size_t builder_capacity;

static void reserve(size_t extra) {
	if (builder_length + extra <= builder_capacity) {
		return;
	}
	size_t capacity = builder_capacity < 256 ? 256 : builder_capacity * 2;
	while (capacity < builder_length + extra) {
		capacity *= 2;
	}
	uint8_t *bytes = fa_alloc(capacity, 1);
	copy_bytes(bytes, builder, builder_length);
	if (builder != 0) {
		fa_free(builder, builder_capacity, 1);
	}
	builder = bytes;
	builder_capacity = capacity;
}

FA_EXPORT void fa_b_reset(void) {
	builder_length = 0;
}

FA_EXPORT void fa_b_bytes(const uint8_t *bytes, size_t length) {
	reserve(length);
	copy_bytes(builder + builder_length, bytes, length);
	builder_length += length;
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
	append_byte('\n');
	fa_out(fd, builder, builder_length);
	builder_length = 0;
}

static FaString owned_copy_with(const uint8_t *bytes, size_t length, size_t simple);

/// A new string owning a copy of `length` bytes (the empty string and the
/// ASCII characters are static).
static FaString owned_copy(const uint8_t *bytes, size_t length) {
	return owned_copy_with(bytes, length, 0);
}

/// `owned_copy` of bytes known to be simple (`SIMPLE`), or to be scanned (0).
static FaString owned_copy_with(const uint8_t *bytes, size_t length, size_t simple) {
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
	uint8_t *copy = fa_alloc(length, 1);
	copy_bytes(copy, bytes, length);
	string.bytes = copy;
	string.length = length;
	string.capacity = length | (simple ? simple : simple_flag(copy, length));
	return string;
}

/// The built text as a new string.
FA_EXPORT void fa_b_take(FaString *out) {
	*out = owned_copy(builder, builder_length);
	builder_length = 0;
}

// ── integers ────────────────────────────────────────────────────────────

static void append_u64(uint64_t value, int negative) {
	char digits[24];
	size_t position = sizeof digits;
	do {
		position -= 1;
		digits[position] = (char)('0' + (int)(value % 10));
		value /= 10;
	} while (value != 0);
	if (negative) {
		position -= 1;
		digits[position] = '-';
	}
	fa_b_bytes((const uint8_t *)digits + position, sizeof digits - position);
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
	fa_b_bytes((const uint8_t *)digits + position, sizeof digits - position);
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
	append_c(value ? "true" : "false");
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
FA_EXPORT void fa_b_quoted(const uint8_t *bytes, size_t length) {
	static const char hex[] = "0123456789ABCDEF";
	append_byte('"');
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
	append_byte('"');
}

/// `[\"a\", \"b\"]`: an array of strings.
FA_EXPORT void fa_b_string_array(const FaString *items, size_t length) {
	append_byte('[');
	for (size_t index = 0; index < length; index += 1) {
		if (index > 0) {
			append_c(", ");
		}
		fa_b_quoted(items[index].bytes, items[index].length);
	}
	append_byte(']');
}

FA_EXPORT void fa_concat(const uint8_t *left, size_t left_length, const uint8_t *right,
                         size_t right_length, FaString *out) {
	size_t length = left_length + right_length;
	if (length <= 1) {
		*out = owned_copy(left_length == 0 ? right : left, length);
		return;
	}
	uint8_t *bytes = fa_alloc(length, 1);
	copy_bytes(bytes, left, left_length);
	copy_bytes(bytes + left_length, right, right_length);
	out->bytes = bytes;
	out->length = length;
	out->capacity = length | simple_flag(bytes, length);
}

/// `fa_string_append` when the block is full (or not owned): a new block,
/// twice as large.
static __attribute__((noinline)) void string_grow(FaString *string, const uint8_t *bytes,
                                                  size_t length, size_t simple) {
	size_t old_length = string->length;
	size_t needed = old_length + length;
	size_t capacity = string->capacity & KIND_MASK;
	int owned = string_owned(string->capacity);
	size_t grown = owned && capacity * 2 > needed ? capacity * 2 : needed;
	if (grown < 16) {
		grown = 16;
	}
	uint8_t *block = fa_alloc(grown, 1);
	copy_bytes(block, string->bytes, old_length);
	copy_bytes(block + old_length, bytes, length);
	if (owned) {
		fa_free((void *)string->bytes, capacity, 1);
	}
	string->bytes = block;
	string->length = needed;
	string->capacity = grown | simple;
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
	size_t old_length = string->length;
	size_t needed = old_length + length;
	size_t simple = (suffix_capacity & SIMPLE) ? SIMPLE
	                : length == 1 ? (bytes[0] < 0x80 && bytes[0] != '\r' ? SIMPLE : 0)
	                              : simple_flag(bytes, length);
	if (old_length != 0) {
		simple &= capacity;
	}
	if (string_owned(capacity) && needed <= (capacity & KIND_MASK)) {
		copy_bytes((uint8_t *)string->bytes + old_length, bytes, length);
		string->length = needed;
		string->capacity = (capacity & KIND_MASK) | simple;
		return;
	}
	string_grow(string, bytes, length, simple);
}

FA_EXPORT void fa_string_append(FaString *string, const uint8_t *bytes, size_t length,
                                size_t suffix_capacity) {
	// common path, inlined: a simple suffix into an owned block with room
	// (the flag of the string stays as it is: simple stays simple)
	size_t capacity = string->capacity;
	size_t old_length = string->length;
	size_t needed = old_length + length;
	size_t kind = capacity & KIND_MASK;
	if (__builtin_expect((suffix_capacity & SIMPLE) && needed <= kind && kind != KIND_MASK, 1)) {
		copy_bytes((uint8_t *)string->bytes + old_length, bytes, length);
		string->length = needed;
		return;
	}
	string_append_slow(string, bytes, length, suffix_capacity);
}

FA_EXPORT int32_t fa_string_equal(const uint8_t *left, size_t left_length, const uint8_t *right,
                                  size_t right_length) {
	if (left_length != right_length) {
		return 0;
	}
	for (size_t index = 0; index < left_length; index += 1) {
		if (left[index] != right[index]) {
			return 0;
		}
	}
	return 1;
}

/// Orders two strings by their bytes (code points): -1, 0 or 1.
FA_EXPORT int32_t fa_string_compare(const uint8_t *left, size_t left_length, const uint8_t *right,
                                    size_t right_length) {
	size_t length = left_length < right_length ? left_length : right_length;
	for (size_t index = 0; index < length; index += 1) {
		if (left[index] != right[index]) {
			return left[index] < right[index] ? -1 : 1;
		}
	}
	return left_length == right_length ? 0 : (left_length < right_length ? -1 : 1);
}

// ── grapheme clusters ───────────────────────────────────────────────────

// An approximation of the extended grapheme clusters of UAX #29, enough for
// Latin, Greek, Cyrillic, CJK, Hangul, combining marks, emoji sequences and
// flags: the full tables come with the Unicode library of the standard
// library. ASCII text takes a fast path.

typedef struct {
	uint32_t first;
	uint32_t last;
} Range;

// Grapheme_Cluster_Break = Extend or SpacingMark (approximated by blocks of
// combining marks), and ZWNJ.
static const Range EXTEND[] = {
    {0x0300, 0x036F}, {0x0483, 0x0489}, {0x0591, 0x05BD}, {0x05BF, 0x05BF}, {0x05C1, 0x05C2},
    {0x05C4, 0x05C5}, {0x05C7, 0x05C7}, {0x0610, 0x061A}, {0x064B, 0x065F}, {0x0670, 0x0670},
    {0x06D6, 0x06DC}, {0x06DF, 0x06E4}, {0x06E7, 0x06E8}, {0x06EA, 0x06ED}, {0x0711, 0x0711},
    {0x0730, 0x074A}, {0x07A6, 0x07B0}, {0x07EB, 0x07F3}, {0x0816, 0x082D}, {0x0859, 0x085B},
    {0x0898, 0x089F}, {0x08CA, 0x08E1}, {0x08E3, 0x0903}, {0x093A, 0x093C}, {0x093E, 0x094F},
    {0x0951, 0x0957}, {0x0962, 0x0963}, {0x0981, 0x0983}, {0x09BC, 0x09BC}, {0x09BE, 0x09CD},
    {0x09D7, 0x09D7}, {0x09E2, 0x09E3}, {0x0A01, 0x0A03}, {0x0A3C, 0x0A51}, {0x0A70, 0x0A71},
    {0x0A75, 0x0A75}, {0x0A81, 0x0A83}, {0x0ABC, 0x0ABC}, {0x0ABE, 0x0ACD}, {0x0AE2, 0x0AE3},
    {0x0B01, 0x0B03}, {0x0B3C, 0x0B3C}, {0x0B3E, 0x0B57}, {0x0B62, 0x0B63}, {0x0B82, 0x0B82},
    {0x0BBE, 0x0BCD}, {0x0BD7, 0x0BD7}, {0x0C00, 0x0C04}, {0x0C3C, 0x0C3C}, {0x0C3E, 0x0C56},
    {0x0C62, 0x0C63}, {0x0C81, 0x0C83}, {0x0CBC, 0x0CBC}, {0x0CBE, 0x0CD6}, {0x0CE2, 0x0CE3},
    {0x0D00, 0x0D03}, {0x0D3B, 0x0D3C}, {0x0D3E, 0x0D4D}, {0x0D57, 0x0D57}, {0x0D62, 0x0D63},
    {0x0D81, 0x0D83}, {0x0DCA, 0x0DDF}, {0x0DF2, 0x0DF3}, {0x0E31, 0x0E31}, {0x0E33, 0x0E3A},
    {0x0E47, 0x0E4E}, {0x0EB1, 0x0EB1}, {0x0EB3, 0x0EBC}, {0x0EC8, 0x0ECE}, {0x0F18, 0x0F19},
    {0x0F35, 0x0F35}, {0x0F37, 0x0F37}, {0x0F39, 0x0F39}, {0x0F3E, 0x0F3F}, {0x0F71, 0x0F84},
    {0x0F86, 0x0F87}, {0x0F8D, 0x0FBC}, {0x0FC6, 0x0FC6}, {0x102B, 0x103E}, {0x1056, 0x1059},
    {0x105E, 0x1060}, {0x1062, 0x1064}, {0x1067, 0x106D}, {0x1071, 0x1074}, {0x1082, 0x108D},
    {0x108F, 0x108F}, {0x109A, 0x109D}, {0x135D, 0x135F}, {0x1712, 0x1715}, {0x1732, 0x1734},
    {0x1752, 0x1753}, {0x1772, 0x1773}, {0x17B4, 0x17D3}, {0x17DD, 0x17DD}, {0x180B, 0x180D},
    {0x180F, 0x180F}, {0x1885, 0x1886}, {0x18A9, 0x18A9}, {0x1920, 0x193B}, {0x1A17, 0x1A1B},
    {0x1A55, 0x1A7F}, {0x1AB0, 0x1AFF}, {0x1B00, 0x1B04}, {0x1B34, 0x1B44}, {0x1B6B, 0x1B73},
    {0x1B80, 0x1B82}, {0x1BA1, 0x1BAD}, {0x1BE6, 0x1BF3}, {0x1C24, 0x1C37}, {0x1CD0, 0x1CD2},
    {0x1CD4, 0x1CE8}, {0x1CED, 0x1CED}, {0x1CF4, 0x1CF4}, {0x1CF7, 0x1CF9}, {0x1DC0, 0x1DFF},
    {0x200C, 0x200C}, {0x20D0, 0x20F0}, {0x2CEF, 0x2CF1}, {0x2D7F, 0x2D7F}, {0x2DE0, 0x2DFF},
    {0x302A, 0x302F}, {0x3099, 0x309A}, {0xA66F, 0xA672}, {0xA674, 0xA67D}, {0xA69E, 0xA69F},
    {0xA6F0, 0xA6F1}, {0xA802, 0xA802}, {0xA806, 0xA806}, {0xA80B, 0xA80B}, {0xA823, 0xA827},
    {0xA82C, 0xA82C}, {0xA880, 0xA881}, {0xA8B4, 0xA8C5}, {0xA8E0, 0xA8F1}, {0xA8FF, 0xA8FF},
    {0xA926, 0xA92D}, {0xA947, 0xA953}, {0xA980, 0xA983}, {0xA9B3, 0xA9C0}, {0xAA29, 0xAA36},
    {0xAA43, 0xAA43}, {0xAA4C, 0xAA4D}, {0xAAEB, 0xAAEF}, {0xAAF5, 0xAAF6}, {0xABE3, 0xABEA},
    {0xABEC, 0xABED}, {0xFB1E, 0xFB1E}, {0xFE00, 0xFE0F}, {0xFE20, 0xFE2F}, {0xFF9E, 0xFF9F},
    {0x1F3FB, 0x1F3FF}, {0xE0020, 0xE007F}, {0xE0100, 0xE01EF},
};

// Extended_Pictographic, approximated by the emoji blocks.
static const Range PICTOGRAPHIC[] = {
    {0x00A9, 0x00A9}, {0x00AE, 0x00AE}, {0x203C, 0x203C}, {0x2049, 0x2049}, {0x2122, 0x2122},
    {0x2139, 0x2139}, {0x2194, 0x21AA}, {0x231A, 0x23FF}, {0x24C2, 0x24C2}, {0x25AA, 0x25FE},
    {0x2600, 0x27BF}, {0x2934, 0x2935}, {0x2B05, 0x2B55}, {0x3030, 0x3030}, {0x303D, 0x303D},
    {0x3297, 0x3297}, {0x3299, 0x3299}, {0x1F000, 0x1F0FF}, {0x1F10D, 0x1F1AD}, {0x1F201, 0x1F3FA},
    {0x1F400, 0x1FAFF}, {0x1FC00, 0x1FFFD},
};

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
	OTHER,
	CR,
	LF,
	CONTROL,
	EXTEND_KIND,
	ZWJ,
	REGIONAL,
	PICTO,
	HANGUL_L,
	HANGUL_V,
	HANGUL_T,
	HANGUL_LV,
	HANGUL_LVT,
};

static enum Break break_kind(uint32_t code) {
	if (code == '\r') {
		return CR;
	}
	if (code == '\n') {
		return LF;
	}
	if (code < 0x20 || (code >= 0x7F && code < 0xA0) || code == 0x2028 || code == 0x2029) {
		return CONTROL;
	}
	if (code < 0x300) {
		return code == 0xA9 || code == 0xAE ? PICTO : OTHER;
	}
	if (code == 0x200D) {
		return ZWJ;
	}
	if (code >= 0x1F1E6 && code <= 0x1F1FF) {
		return REGIONAL;
	}
	if ((code >= 0x1100 && code <= 0x115F) || (code >= 0xA960 && code <= 0xA97C)) {
		return HANGUL_L;
	}
	if ((code >= 0x1160 && code <= 0x11A7) || (code >= 0xD7B0 && code <= 0xD7C6)) {
		return HANGUL_V;
	}
	if ((code >= 0x11A8 && code <= 0x11FF) || (code >= 0xD7CB && code <= 0xD7FB)) {
		return HANGUL_T;
	}
	if (code >= 0xAC00 && code <= 0xD7A3) {
		return (code - 0xAC00) % 28 == 0 ? HANGUL_LV : HANGUL_LVT;
	}
	if (in_ranges(code, EXTEND, sizeof EXTEND / sizeof EXTEND[0])) {
		return EXTEND_KIND;
	}
	if (in_ranges(code, PICTOGRAPHIC, sizeof PICTOGRAPHIC / sizeof PICTOGRAPHIC[0])) {
		return PICTO;
	}
	return OTHER;
}

/// Decodes the code point at `*index` (the input is valid UTF-8) and moves past it.
static uint32_t decode(const uint8_t *bytes, size_t length, size_t *index) {
	uint8_t first = bytes[*index];
	size_t count = first < 0x80 ? 1 : first < 0xE0 ? 2 : first < 0xF0 ? 3 : 4;
	uint32_t code = count == 1 ? first : first & (0x7F >> count);
	for (size_t offset = 1; offset < count && *index + offset < length; offset += 1) {
		code = (code << 6) | (bytes[*index + offset] & 0x3F);
	}
	*index += count;
	return code;
}

/// Whether two code points of these kinds stay in one cluster (GB3 to GB13),
/// with the state of the cluster so far.
static int joins(enum Break previous, enum Break kind, int regional_run, int emoji_sequence) {
	if (previous == CR && kind == LF) {
		return 1;
	}
	if (previous == CR || previous == LF || previous == CONTROL || kind == CR || kind == LF ||
	    kind == CONTROL) {
		return 0;
	}
	if (previous == HANGUL_L &&
	    (kind == HANGUL_L || kind == HANGUL_V || kind == HANGUL_LV || kind == HANGUL_LVT)) {
		return 1;
	}
	if ((previous == HANGUL_LV || previous == HANGUL_V) && (kind == HANGUL_V || kind == HANGUL_T)) {
		return 1;
	}
	if ((previous == HANGUL_LVT || previous == HANGUL_T) && kind == HANGUL_T) {
		return 1;
	}
	if (kind == EXTEND_KIND || kind == ZWJ) {
		return 1;
	}
	if (previous == ZWJ && kind == PICTO && emoji_sequence) {
		return 1;
	}
	return previous == REGIONAL && kind == REGIONAL && regional_run % 2 == 1;
}

/// The end of the grapheme cluster that starts at `index` (`index < length`).
static size_t next_boundary(const uint8_t *bytes, size_t length, size_t index) {
	// ASCII followed by ASCII: one cluster per byte, except "\r\n"
	if (bytes[index] < 0x80 && (index + 1 >= length || bytes[index + 1] < 0x80)) {
		return index + 1 + (bytes[index] == '\r' && index + 1 < length && bytes[index + 1] == '\n');
	}
	enum Break previous = break_kind(decode(bytes, length, &index));
	int regional_run = previous == REGIONAL;
	int emoji_sequence = previous == PICTO;
	while (index < length) {
		size_t next = index;
		enum Break kind = break_kind(decode(bytes, length, &next));
		if (!joins(previous, kind, regional_run, emoji_sequence)) {
			break;
		}
		index = next;
		regional_run = kind == REGIONAL ? regional_run + 1 : 0;
		if (kind == PICTO) {
			emoji_sequence = 1;
		} else if (kind != EXTEND_KIND && kind != ZWJ) {
			emoji_sequence = 0;
		}
		previous = kind;
	}
	return index;
}

/// The end of the grapheme cluster at `index`: `for character in text`.
FA_EXPORT size_t fa_next_grapheme(const uint8_t *bytes, size_t length, size_t index) {
	return next_boundary(bytes, length, index);
}

/// `text.length`: the number of grapheme clusters.
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
	fa_string_append(string, builder, builder_length, simple_flag(builder, builder_length));
	builder_length = 0;
}

/// Whether a byte offset is a grapheme cluster boundary.
static int is_boundary(const uint8_t *bytes, size_t length, size_t offset) {
	size_t index = 0;
	while (index < offset) {
		index = next_boundary(bytes, length, index);
	}
	return index == offset;
}

/// The number of grapheme clusters before a boundary.
static size_t grapheme_index(const uint8_t *bytes, size_t length, size_t offset) {
	size_t count = 0;
	for (size_t index = 0; index < offset; index = next_boundary(bytes, length, index)) {
		count += 1;
	}
	return count;
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

/// A walk over the grapheme boundaries of a text, for offsets that never
/// decrease: each check costs amortized constant time, where `is_boundary`
/// walks from the start. Every offset of a simple text is a boundary.
typedef struct {
	const uint8_t *bytes;
	size_t length;
	size_t index;
	size_t simple;
} Boundaries;

static int at_boundary(Boundaries *boundaries, size_t offset) {
	const uint8_t *bytes = boundaries->bytes;
	// between two ASCII bytes other than "\r\n", always a boundary
	if (boundaries->simple || offset == 0 || offset >= boundaries->length ||
	    (bytes[offset] < 0x80 && bytes[offset - 1] < 0x80 &&
	     !(bytes[offset - 1] == '\r' && bytes[offset] == '\n'))) {
		return 1;
	}
	while (boundaries->index < offset) {
		boundaries->index = next_boundary(bytes, boundaries->length, boundaries->index);
	}
	return boundaries->index == offset;
}

/// The offset of the first `byte` at or after `offset`, or `length`:
/// eight bytes at a time (the lowest byte that "has a zero byte" flags is
/// exact).
static size_t next_byte(const uint8_t *text, size_t length, size_t offset, uint8_t byte) {
	const uint64_t ones = 0x0101010101010101u;
	uint64_t pattern = ones * byte;
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

/// The byte offset of the first (or last) match of `search` whose two ends
/// are grapheme boundaries, or -1 (`faFindMatches`).
static int64_t find(const uint8_t *text, size_t text_length, size_t simple, const uint8_t *search,
                    size_t search_length, int last) {
	if (search_length > text_length) {
		return -1;
	}
	if (simple && search_length > 0 && !last) {
		// every offset is a boundary: the bytes alone decide
		size_t offset = next_byte(text, text_length, 0, search[0]);
		while (offset + search_length <= text_length) {
			if (bytes_at(text, text_length, offset, search, search_length)) {
				return (int64_t)offset;
			}
			offset = next_byte(text, text_length, offset + 1, search[0]);
		}
		return -1;
	}
	if (simple && search_length > 0) {
		// every offset is a boundary: the bytes alone decide
		uint8_t first = search[0];
		size_t count = text_length - search_length + 1;
		for (size_t step = 0; step < count; step += 1) {
			size_t offset = last ? count - 1 - step : step;
			if (text[offset] == first &&
			    bytes_at(text, text_length, offset, search, search_length)) {
				return (int64_t)offset;
			}
		}
		return -1;
	}
	int64_t found = -1;
	Boundaries starts = {text, text_length, 0, simple};
	Boundaries ends = {text, text_length, 0, simple};
	for (size_t offset = 0; offset + search_length <= text_length; offset += 1) {
		if (bytes_at(text, text_length, offset, search, search_length) &&
		    at_boundary(&starts, offset) && at_boundary(&ends, offset + search_length)) {
			found = (int64_t)offset;
			if (!last) {
				break;
			}
		}
	}
	return found;
}

FA_EXPORT int32_t fa_string_includes(const uint8_t *text, size_t text_length, size_t capacity,
                                     const uint8_t *search, size_t search_length) {
	return find(text, text_length, capacity & SIMPLE, search, search_length, 0) >= 0;
}

FA_EXPORT int32_t fa_string_starts_with(const uint8_t *text, size_t text_length, size_t capacity,
                                        const uint8_t *search, size_t search_length) {
	return bytes_at(text, text_length, 0, search, search_length) &&
	       ((capacity & SIMPLE) || is_boundary(text, text_length, search_length));
}

FA_EXPORT int32_t fa_string_ends_with(const uint8_t *text, size_t text_length, size_t capacity,
                                      const uint8_t *search, size_t search_length) {
	return search_length <= text_length &&
	       bytes_at(text, text_length, text_length - search_length, search, search_length) &&
	       ((capacity & SIMPLE) || is_boundary(text, text_length, text_length - search_length));
}

/// `indexOf` (`last` 0) and `lastIndexOf` (1): the grapheme position of the
/// match, or -1 for `none`.
static __attribute__((noinline)) int64_t string_index_of(const uint8_t *text, size_t text_length,
                                                          size_t capacity, const uint8_t *search,
                                                          size_t search_length, int32_t last) {
	size_t simple = capacity & SIMPLE;
	int64_t offset = find(text, text_length, simple, search, search_length, last);
	if (offset < 0 || simple) {
		return offset;
	}
	return (int64_t)grapheme_index(text, text_length, (size_t)offset);
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

/// The non-overlapping matches of `search` whose ends are grapheme
/// boundaries (`faFindMatches`): calls `found(offset)` for each, in order.
static void each_match(const uint8_t *text, size_t text_length, size_t simple,
                       const uint8_t *search, size_t search_length,
                       void (*found)(void *, size_t), void *context) {
	size_t offset = 0;
	if (simple && search_length > 0) {
		// every offset is a boundary: jump from one first byte to the next
		offset = next_byte(text, text_length, 0, search[0]);
		while (offset + search_length <= text_length) {
			if (bytes_at(text, text_length, offset, search, search_length)) {
				found(context, offset);
				offset += search_length;
			} else {
				offset += 1;
			}
			offset = next_byte(text, text_length, offset, search[0]);
		}
		return;
	}
	Boundaries starts = {text, text_length, 0, simple};
	Boundaries ends = {text, text_length, 0, simple};
	while (offset + search_length <= text_length) {
		if (bytes_at(text, text_length, offset, search, search_length) &&
		    at_boundary(&starts, offset) && at_boundary(&ends, offset + search_length)) {
			found(context, offset);
			offset += search_length == 0 ? 1 : search_length;
		} else {
			offset += 1;
		}
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

static void replace_one(void *context, size_t offset) {
	Replacing *replacing = context;
	if (replacing->done) {
		return;
	}
	fa_b_bytes(replacing->text + replacing->previous, offset - replacing->previous);
	fa_b_bytes(replacing->replacement, replacing->replacement_length);
	replacing->previous = offset + replacing->search_length;
	replacing->done = !replacing->all;
}

/// `replace` (the first match, `all` 0) and `replaceAll`.
FA_EXPORT void fa_string_replace(const uint8_t *text, size_t text_length, size_t capacity,
                                 const uint8_t *search, size_t search_length,
                                 const uint8_t *replacement, size_t replacement_length,
                                 int32_t all, FaString *out) {
	fa_b_reset();
	Replacing replacing = {text, 0, search_length, replacement, replacement_length, all, 0};
	each_match(text, text_length, capacity & SIMPLE, search, search_length, replace_one,
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

void *fa_array_push_slot(void *array, int64_t item_size);

typedef struct {
	const uint8_t *text;
	size_t previous;
	size_t separator_length;
	void *parts;
	size_t simple;
} Splitting;

static void split_one(void *context, size_t offset) {
	Splitting *splitting = context;
	FaString *part = fa_array_push_slot(splitting->parts, sizeof(FaString));
	*part = owned_copy_with(splitting->text + splitting->previous, offset - splitting->previous,
	                        splitting->simple);
	splitting->previous = offset + splitting->separator_length;
}

/// The grapheme clusters of a text, an `Array(String)`: the characters of
/// a simple text point into the ASCII table, the others are copies.
static void *characters_of(const uint8_t *text, size_t length, size_t capacity) {
	if (capacity & SIMPLE) {
		void *characters = fa_array_new((int64_t)length, (int64_t)length, sizeof(FaString));
		FaString *items = (FaString *)((FaArrayHeader *)characters)->items;
		for (size_t index = 0; index < length; index += 1) {
			items[index] = ascii_string(text[index]);
		}
		return characters;
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

/// `text.split(separator)`; an empty separator splits into characters.
FA_EXPORT void *fa_string_split(const uint8_t *text, size_t length, size_t capacity,
                                const uint8_t *separator, size_t separator_length) {
	if (separator_length == 0) {
		return characters_of(text, length, capacity);
	}
	Splitting splitting = {text, 0, separator_length, fa_array_new(0, 4, sizeof(FaString)),
	                       capacity & SIMPLE};
	each_match(text, length, capacity & SIMPLE, separator, separator_length, split_one,
	           &splitting);
	FaString *last = fa_array_push_slot(splitting.parts, sizeof(FaString));
	*last = owned_copy_with(text + splitting.previous, length - splitting.previous,
	                        capacity & SIMPLE);
	return splitting.parts;
}

/// `text.codePoints()`: an `Array(UInt32)`.
FA_EXPORT void *fa_string_code_points(const uint8_t *text, size_t length, size_t capacity) {
	size_t count = 0;
	if (capacity & SIMPLE) {
		count = length;
	} else {
		for (size_t index = 0; index < length; index += 1) {
			count += (text[index] & 0xC0) != 0x80;
		}
	}
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
	return count;
}

/// `fa_string_get` on a text that isn't simple: walks the boundaries. A
/// character of a static text is a view of it, also static.
static __attribute__((noinline)) int32_t string_get_slow(const uint8_t *text, size_t length,
                                                         size_t capacity, int64_t index,
                                                         FaString *out) {
	if (index < 0) {
		index += (int64_t)fa_graphemes(text, length);
	}
	if (index < 0) {
		return 0;
	}
	size_t offset = 0;
	for (int64_t position = 0; offset < length; position += 1) {
		size_t next = next_boundary(text, length, offset);
		if (position == index) {
			if ((capacity & KIND_MASK) == 0 && next - offset > 1) {
				out->bytes = text + offset;
				out->length = next - offset;
				out->capacity = 0;
			} else {
				*out = slice(text, offset, next);
			}
			return 1;
		}
		offset = next;
	}
	return 0;
}

/// `text.get(index)`: the character at a grapheme position (negative from
/// the end); 0 when there is none. Constant time on a simple text.
FA_EXPORT int32_t fa_string_get(const uint8_t *text, size_t length, size_t capacity, int64_t index,
                                FaString *out) {
	if (capacity & SIMPLE) {
		if (index < 0) {
			index += (int64_t)length;
		}
		if ((uint64_t)index >= (uint64_t)length) {
			return 0;
		}
		*out = ascii_string(text[index]);
		return 1;
	}
	return string_get_slow(text, length, capacity, index, out);
}

/// `text.length`: the number of grapheme clusters (constant time on a simple text).
FA_EXPORT size_t fa_string_length(const uint8_t *text, size_t length, size_t capacity) {
	return capacity & SIMPLE ? length : fa_graphemes(text, length);
}

/// `text[start..end]` (`faStringSlice`): each bound in `-length..=length`
/// (an inclusive end in `-length..length`), counted in grapheme clusters,
/// else an `IndexError`; a start after the end gives `""`. `flags`: bit 0 a
/// start, bit 1 an end, bit 2 an inclusive end. A slice of a static text is
/// a view of it.
FA_EXPORT const void *fa_string_slice(const uint8_t *text, size_t length, size_t capacity,
                                      int64_t start, int64_t end, int8_t flags, FaString *out) {
	int simple = (capacity & SIMPLE) != 0;
	int64_t count = simple ? (int64_t)length : (int64_t)fa_graphemes(text, length);
	int64_t from = 0;
	int64_t to = count;
	if (flags & 1) {
		if (start < -count || start > count) {
			return fa_error_index(start, count);
		}
		from = start < 0 ? start + count : start;
	}
	if (flags & 2) {
		int inclusive = (flags & 4) != 0;
		if (end < -count || end > (inclusive ? count - 1 : count)) {
			return fa_error_index(end, count);
		}
		to = (end < 0 ? end + count : end) + inclusive;
	}
	if (to < from) {
		to = from;
	}
	size_t first = (size_t)from;
	size_t last = (size_t)to;
	if (!simple) {
		// the byte offsets of the two boundaries
		size_t offset = 0;
		size_t position = 0;
		for (; position < (size_t)from; position += 1) {
			offset = next_boundary(text, length, offset);
		}
		first = offset;
		for (; position < (size_t)to; position += 1) {
			offset = next_boundary(text, length, offset);
		}
		last = offset;
	}
	if ((capacity & KIND_MASK) == 0 && last - first > 1) {
		out->bytes = text + first;
		out->length = last - first;
		out->capacity = capacity & SIMPLE;
	} else {
		*out = slice(text, first, last);
	}
	return 0;
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

/// `toUpperCase` (`upper` 1) and `toLowerCase` (0): ASCII, Latin-1, Greek
/// and Cyrillic, and `ß` → `SS` (the full Unicode tables come with the
/// standard library).
FA_EXPORT void fa_string_case(const uint8_t *bytes, size_t length, int32_t upper, FaString *out) {
	fa_b_reset();
	size_t index = 0;
	while (index < length) {
		uint32_t code = decode(bytes, length, &index);
		if (upper) {
			if ((code >= 'a' && code <= 'z') || (code >= 0xE0 && code <= 0xFE && code != 0xF7) ||
			    (code >= 0x3B1 && code <= 0x3C9 && code != 0x3C2) || (code >= 0x430 && code <= 0x44F)) {
				code -= 0x20;
			} else if (code >= 0x450 && code <= 0x45F) {
				code -= 0x50;
			} else if (code == 0xFF) {
				code = 0x178;
			} else if (code == 0xB5) {
				code = 0x39C;
			} else if (code == 0x3C2) {
				code = 0x3A3;
			} else if (code == 0xDF) {
				append_c("SS");
				continue;
			}
		} else {
			if ((code >= 'A' && code <= 'Z') || (code >= 0xC0 && code <= 0xDE && code != 0xD7) ||
			    (code >= 0x391 && code <= 0x3A9 && code != 0x3A2) || (code >= 0x410 && code <= 0x42F)) {
				code += 0x20;
			} else if (code >= 0x400 && code <= 0x40F) {
				code += 0x50;
			} else if (code == 0x178) {
				code = 0xFF;
			}
		}
		append_code_point(code);
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
	fa_b_reset();
	for (int64_t index = 0; index < count; index += 1) {
		fa_b_bytes(bytes, length);
	}
	fa_b_take(out);
}

/// `items.join(separator)` on an array of strings.
FA_EXPORT void fa_array_join(const void *header, const uint8_t *separator,
                             size_t separator_length, FaString *out) {
	const FaArrayHeader *array = header;
	const FaString *items = (const FaString *)array->items;
	fa_b_reset();
	for (int64_t index = 0; index < array->length; index += 1) {
		if (index > 0) {
			fa_b_bytes(separator, separator_length);
		}
		fa_b_bytes(items[index].bytes, items[index].length);
	}
	fa_b_take(out);
}

/// `padStart` (`at_start` 1) and `padEnd` (0), in grapheme clusters.
FA_EXPORT void fa_string_pad(const uint8_t *bytes, size_t length, int64_t width,
                             const uint8_t *filler, size_t filler_length, int32_t at_start,
                             FaString *out) {
	int64_t missing = width - (int64_t)fa_graphemes(bytes, length);
	fa_b_reset();
	if (!at_start) {
		fa_b_bytes(bytes, length);
	}
	size_t position = 0;
	for (int64_t index = 0; index < missing && filler_length > 0; index += 1) {
		size_t next = next_boundary(filler, filler_length, position);
		fa_b_bytes(filler + position, next - position);
		position = next == filler_length ? 0 : next;
	}
	if (at_start) {
		fa_b_bytes(bytes, length);
	}
	fa_b_take(out);
}

// ── arguments ───────────────────────────────────────────────────────────

static FaString *argument_items;
#if defined(__wasm__)
static uint8_t *argument_bytes;
#endif
static size_t argument_count;

#if !defined(__wasm__)
/// Keeps the user arguments: `argv` without the program (`argv[0]`).
FA_EXPORT void fa_arguments_init(int32_t argc, char **argv) {
	argument_count = argc > 1 ? (size_t)argc - 1 : 0;
	argument_items = fa_alloc(sizeof(FaString) * (argument_count + 1), sizeof(void *));
	for (size_t index = 0; index < argument_count; index += 1) {
		const char *text = argv[index + 1];
		argument_items[index].bytes = (const uint8_t *)text;
		argument_items[index].length = c_length(text);
		argument_items[index].capacity =
		    simple_flag((const uint8_t *)text, argument_items[index].length);
	}
}
#else
/// Asks the host for the user arguments.
FA_EXPORT void fa_arguments_init(int32_t argc, char **argv) {
	(void)argc;
	(void)argv;
	size_t size = (size_t)fa_host_arguments_size();
	uint8_t *bytes = fa_alloc(size == 0 ? 1 : size, 1);
	argument_bytes = bytes;
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
			argument_items[item].bytes = bytes + start;
			argument_items[item].length = index - start;
			argument_items[item].capacity = simple_flag(bytes + start, index - start);
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

/// Whether the items of an array are in the block of its header: the word
/// after the header says so (headers take 32 bytes, so that it exists).
static int inline_items(const FaArrayHeader *array) {
	uint64_t marker;
	__builtin_memcpy(&marker, (const uint8_t *)array + sizeof(FaArrayHeader), 8);
	return marker != 0 && array->items == (const uint8_t *)array + INLINE_ITEMS;
}

static void mark_items(FaArrayHeader *array, uint64_t marker) {
	__builtin_memcpy((uint8_t *)array + sizeof(FaArrayHeader), &marker, 8);
}

/// Frees the block of the items of an array, unless it is the header's.
/// Called before the items move to another block: the marker is cleared,
/// since that block may happen to follow the header.
static void free_array_items(FaArrayHeader *array, int64_t item_size) {
	if (inline_items(array)) {
		mark_items(array, 0);
	} else {
		fa_free(array->items, (size_t)(array->capacity * item_size), 16);
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

/// Makes room for one more item and returns its slot (`push`).
/// `fa_array_push_slot` on a full array: a block twice as large.
static __attribute__((noinline)) void array_grow(FaArrayHeader *array, int64_t item_size) {
	int64_t capacity = array->capacity < 4 ? 4 : array->capacity * 2;
	uint8_t *items = fa_alloc((size_t)(capacity * item_size), 16);
	copy_bytes(items, array->items, (size_t)(array->length * item_size));
	free_array_items(array, item_size);
	array->items = items;
	array->capacity = capacity;
}

FA_EXPORT void *fa_array_push_slot(void *header, int64_t item_size) {
	FaArrayHeader *array = header;
	if (__builtin_expect(array->length == array->capacity, 0)) {
		array_grow(array, item_size);
	}
	void *slot = array->items + array->length * item_size;
	array->length += 1;
	return slot;
}

/// Appends the items of `source` to `array` (bytewise).
FA_EXPORT void fa_array_append(FaArrayHeader *array, const FaArrayHeader *source,
                               int64_t item_size) {
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
	int64_t length = array->length;
	int64_t from = 0;
	int64_t to = length;
	if (flags & 1) {
		if (start < -length || start > length) {
			return fa_error_index(start, length);
		}
		from = start < 0 ? start + length : start;
	}
	if (flags & 2) {
		int inclusive = (flags & 4) != 0;
		if (end < -length || end > (inclusive ? length - 1 : length)) {
			return fa_error_index(end, length);
		}
		to = (end < 0 ? end + length : end) + inclusive;
	}
	if (to < from) {
		to = from;
	}
	FaArrayHeader *slice = fa_array_new(to - from, to - from, item_size);
	copy_bytes(slice->items, array->items + from * item_size, (size_t)((to - from) * item_size));
	*out = slice;
	return 0;
}

/// Makes room for `capacity` items (`Array.from(…, capacity = n)`).
FA_EXPORT void fa_array_reserve(FaArrayHeader *array, int64_t capacity, int64_t item_size) {
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
	FaArrayHeader *copy = fa_array_new(array->length, array->length, item_size);
	copy_bytes(copy->items, array->items, (size_t)(array->length * item_size));
	return copy;
}

/// Reverses the items of an array in place.
FA_EXPORT void fa_array_reverse(FaArrayHeader *array, int64_t item_size) {
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
	uint8_t *at = array->items + index * item_size;
	size_t moved = (size_t)((array->length - 1 - index) * item_size);
	__builtin_memmove(at, at + item_size, moved);
	array->length -= 1;
}

/// Sorts an array in place, stably (a merge sort, as JavaScript's sort):
/// `compare(context, a, b)` is negative when `a` comes first.
FA_EXPORT void fa_array_sort(FaArrayHeader *array, int64_t item_size,
                             int32_t (*compare)(void *, const void *, const void *),
                             void *context) {
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
	FaArrayHeader *array = fa_array_new((int64_t)argument_count, (int64_t)argument_count,
	                                    (int64_t)sizeof(FaString));
	copy_bytes(array->items, (const uint8_t *)argument_items, sizeof(FaString) * argument_count);
	return array;
}

/// Frees an array whose items were dropped: its block and its header.
FA_EXPORT void fa_array_free(FaArrayHeader *array, int64_t item_size) {
	if (!inline_items(array)) {
		fa_free(array->items, (size_t)(array->capacity * item_size), 16);
	}
	fa_free(array, INLINE_ITEMS, sizeof(int64_t));
}

// ── segmented storage (the growing collections) ────────────────────────

// A `GrowingArray` (and the entries of a `GrowingMap` / `GrowingSet`) never
// moves its items: they live in segments that are never reallocated
// (growing-collections.md, implementation notes; Zig's `SegmentedList`).
// With a first segment of `2^k` items, segment `s > 0` holds `2^(k+s-1)`
// items starting at index `2^(k+s-1)`, so item `i` is in segment
// `s = floor(log2(i >> k)) + 1` (0 when `i < 2^k`): one `clz`.

#define FA_SEGMENTS 48

/// A `GrowingArray` (`src/lower/growing.rs`).
typedef struct {
	int64_t length;
	int64_t segments;
	uint8_t *segment[FA_SEGMENTS];
} FaSegmented;

/// The segment of item `index`, and the index of its first item.
static __attribute__((always_inline)) inline int64_t segment_of(int64_t index, int32_t k,
                                                                int64_t *first) {
	uint64_t high = (uint64_t)index >> k;
	if (high == 0) {
		*first = 0;
		return 0;
	}
	int64_t segment = 64 - __builtin_clzll(high);
	*first = (int64_t)1 << (k + segment - 1);
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
	int64_t first;
	int64_t segment = segment_of(index, k, &first);
	return array->segment[segment] + (index - first) * item_size;
}

/// A new empty `GrowingArray`.
FA_EXPORT FaSegmented *fa_segmented_new(void) {
	FaSegmented *array = fa_alloc(sizeof(FaSegmented), sizeof(int64_t));
	__builtin_memset(array, 0, sizeof(FaSegmented));
	return array;
}

/// The slot of a new last item (`push`): a full segment is kept, and a new
/// one is added.
FA_EXPORT void *fa_segmented_push_slot(FaSegmented *array, int64_t item_size, int32_t k) {
	int64_t index = array->length;
	int64_t first;
	int64_t segment = segment_of(index, k, &first);
	if (segment >= array->segments) {
		array->segment[segment] = fa_alloc((size_t)(segment_size(segment, k) * item_size), 16);
		array->segments = segment + 1;
	}
	array->length = index + 1;
	return array->segment[segment] + (index - first) * item_size;
}

/// Frees a `GrowingArray` whose items were dropped.
FA_EXPORT void fa_segmented_free(FaSegmented *array, int64_t item_size, int32_t k) {
	for (int64_t segment = 0; segment < array->segments; segment += 1) {
		fa_free(array->segment[segment], (size_t)(segment_size(segment, k) * item_size), 16);
	}
	fa_free(array, sizeof(FaSegmented), sizeof(int64_t));
}

/// The items of a `GrowingArray` copied bytewise into a new array: a view
/// for the read-only methods (freed with `fa_array_free`, its items not
/// dropped), or a move when the `GrowingArray` is freed after.
FA_EXPORT FaArrayHeader *fa_segmented_flatten(const FaSegmented *array, int64_t item_size,
                                              int32_t k) {
	FaArrayHeader *flat = fa_array_new(array->length, array->length, item_size);
	int64_t done = 0;
	for (int64_t segment = 0; done < array->length; segment += 1) {
		int64_t count = segment_size(segment, k);
		if (count > array->length - done) {
			count = array->length - done;
		}
		copy_bytes(flat->items + done * item_size, array->segment[segment],
		           (size_t)(count * item_size));
		done += count;
	}
	return flat;
}

/// A `GrowingArray` taking the items of an array (moved bytewise; the
/// array's block and header are freed).
FA_EXPORT FaSegmented *fa_segmented_from_array(FaArrayHeader *flat, int64_t item_size,
                                               int32_t k) {
	FaSegmented *array = fa_segmented_new();
	for (int64_t index = 0; index < flat->length; index += 1) {
		copy_bytes(fa_segmented_push_slot(array, item_size, k), flat->items + index * item_size,
		           (size_t)item_size);
	}
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
	// at least 1 MiB at a time: fewer system calls, and the rest is free
	size_t asked = pages < 16 ? 16 : pages;
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

FA_EXPORT void *fa_heap_alloc(size_t size, size_t align) {
	if (align > 8) {
		size = (size + 15) & ~(size_t)15;
	}
	// common path, inlined: a small block from the free list of its class
	if (size <= 128) {
		uint32_t class = size <= 8 ? 0 : (uint32_t)((size + 7) / 8 - 1);
		FreeBlock *free = free_blocks[class];
		if (__builtin_expect(free != 0, 1)) {
			free_blocks[class] = free->next;
			return free;
		}
	}
	return heap_alloc_slow(size);
}

/// `fa_heap_alloc` off its common path: a new block of a page, or pages.
static __attribute__((noinline)) void *heap_alloc_slow(size_t size) {
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
	if (class_next[class] + block > class_end[class] || class_next[class] == 0) {
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
	FreeBlock *block = bytes;
	block->next = free_blocks[header->class];
	free_blocks[header->class] = block;
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
		fa_free(argument_items, sizeof(FaString) * (argument_count + 1), sizeof(void *));
		argument_items = 0;
	}
#if defined(__wasm__)
	if (argument_bytes != 0) {
		fa_free(argument_bytes, 1, 1);
		argument_bytes = 0;
	}
#endif
	if (fa_live_blocks != 0) {
		fa_b_reset();
		append_c("leak: ");
		fa_b_u64((uint64_t)fa_live_blocks);
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
} FaPool;

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
	pool->available[index / 64] |= (uint64_t)1 << (index % 64);
	return chunk;
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
	pool->size = 0;
}

/// `clear`: frees every chunk but the spare one (the elements were dropped).
FA_EXPORT void fa_pool_clear(FaPool *pool) {
	pool_release_chunks(pool);
}

/// Frees a pool whose elements were dropped.
FA_EXPORT void fa_pool_free(FaPool *pool) {
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

/// `holder.direct = target` (null: `none`): the two-sided update of a
/// one-to-many relation (`many`: the reverse sides are sets), of a
/// one-to-one one, or of a symmetric one (`reverse` is `direct`).
FA_EXPORT void fa_relate_one(void *holder, int64_t direct, int64_t reverse, void *target,
                             int32_t many) {
	void **slot = side_of(holder, direct);
	void *old = *slot;
	if (old == target) {
		return;
	}
	if (many) {
		if (old != 0) {
			fa_links_delete(*(FaLinks **)side_of(old, reverse), holder);
		}
		*slot = target;
		if (target != 0) {
			fa_links_add((FaLinks **)side_of(target, reverse), holder);
		}
		return;
	}
	if (old != 0) {
		*side_of(old, reverse) = 0;
	}
	if (target != 0) {
		// the previous holder of the target loses it
		void *previous = *side_of(target, reverse);
		if (previous != 0) {
			*side_of(previous, direct) = 0;
		}
		*side_of(target, reverse) = holder;
	}
	*slot = target;
}

/// `holder.direct.add(target)` of a many-to-many relation: whether the
/// link is new.
FA_EXPORT int32_t fa_relate_add(void *holder, int64_t direct, int64_t reverse, void *target) {
	if (!fa_links_add((FaLinks **)side_of(holder, direct), target)) {
		return 0;
	}
	fa_links_add((FaLinks **)side_of(target, reverse), holder);
	return 1;
}

/// `holder.direct.delete(target)` of a many-to-many relation: whether there
/// was a link.
FA_EXPORT int32_t fa_relate_delete(void *holder, int64_t direct, int64_t reverse, void *target) {
	if (!fa_links_delete(*(FaLinks **)side_of(holder, direct), target)) {
		return 0;
	}
	fa_links_delete(*(FaLinks **)side_of(target, reverse), holder);
	return 1;
}

/// `holder.direct.clear()` of a many-to-many relation.
FA_EXPORT void fa_relate_clear(void *holder, int64_t direct, int64_t reverse) {
	FaLinks *links = *(FaLinks **)side_of(holder, direct);
	if (links == 0) {
		return;
	}
	for (int64_t position = 0; position < links->length; position += 1) {
		void *target = links->items[position];
		if (target != 0) {
			fa_links_delete(*(FaLinks **)side_of(target, reverse), holder);
		}
	}
	fa_links_free(links);
	*side_of(holder, direct) = 0;
}

/// Unlinks an element about to leave its collection (8.5 `unlink`), from
/// one relation: `kind` 0 one-to-many, 1 one-to-one, 2 many-to-many, 3
/// symmetric, 4 owner field; `holder` 1 when the element holds the direct
/// side, 0 when it is a target. `owner` is the address of the owner field
/// (kind 4), or null.
FA_EXPORT void fa_unlink(void *element, int32_t kind, int32_t holder, int64_t direct,
                         int64_t reverse, void **owner) {
	if (holder) {
		void *target = *side_of(element, direct);
		if (kind == 2) {
			FaLinks *links = (FaLinks *)target;
			for (int64_t position = 0; links != 0 && position < links->length; position += 1) {
				void *item = links->items[position];
				if (item != 0) {
					fa_links_delete(*(FaLinks **)side_of(item, reverse), element);
				}
			}
		} else if (target != 0) {
			if (kind == 0) {
				fa_links_delete(*(FaLinks **)side_of(target, reverse), element);
			} else {
				*side_of(target, reverse) = 0;
			}
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
		}
		break;
	}
	case 1: {
		void *other = *side_of(element, reverse);
		if (other != 0) {
			*side_of(other, direct) = 0;
		}
		break;
	}
	case 3: {
		void *other = *side_of(element, direct);
		if (other != 0 && other != element) {
			*side_of(other, direct) = 0;
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
// below the stack raises SIGSEGV, handled on an alternate stack. Linux
// layouts of `stack_t` and of glibc's `struct sigaction`.
typedef struct {
	void *pointer;
	int32_t flags;
	size_t size;
} AlternateStack;

typedef struct {
	void (*handler)(int);
	uint64_t mask[16];
	int32_t flags;
	void (*restorer)(void);
} SignalAction;

int sigaltstack(const AlternateStack *stack, AlternateStack *old);
int sigaction(int signal, const SignalAction *action, SignalAction *old);
FA_NORETURN void _exit(int status);

static uint8_t signal_stack[16384];

static void on_fault(int signal) {
	(void)signal;
	fa_flush();
	static const char message[] = "stop: stack overflow\n";
	fa_write(2, (const uint8_t *)message, sizeof message - 1);
	_exit(101);
}

/// Reports a stack overflow as a stop (native with the C library only).
FA_EXPORT void fa_guard_stack(void) {
	AlternateStack stack = {signal_stack, 0, sizeof signal_stack};
	sigaltstack(&stack, 0);
	SignalAction action = {on_fault, {0}, 0x08000000 /* SA_ONSTACK */, 0};
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
	fa_b_bytes(operands.bytes, operands.length);
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
// its entries in insertion order (as the JavaScript runtime iterates them),
// each `{ u64 hash, i64 live, key, value }` (a set has no value), and an
// open-addressing index from a hash to an entry. A deleted entry stays dead
// in place until the entries are compacted, when they are full. The
// generated code hashes the keys and compares them (`equals`), and moves
// keys and values in and out of the entries.

typedef struct {
	uint8_t *entries;
	int64_t length;
	int64_t used;
	int64_t capacity;
	int64_t *index;
	int64_t mask;
	// a `GrowingMap` / `GrowingSet`: `entries` is a `FaSegmented` of
	// entries, which never move (references into them stay valid)
	int64_t growing;
} FaTable;

#define TABLE_SEGMENT_BITS 2

#define ENTRY_HASH(entry) (*(uint64_t *)(entry))
#define ENTRY_LIVE(entry) (*(int64_t *)((entry) + 8))
#define ENTRY_KEY(entry) ((entry) + 16)

/// The entry at a position.
static __attribute__((always_inline)) inline uint8_t *entry_at(const FaTable *table,
                                                               int64_t position,
                                                               int64_t entry_size, int growing) {
	if (growing) {
		return fa_segmented_at((const FaSegmented *)table->entries, position, entry_size,
		                       TABLE_SEGMENT_BITS);
	}
	return table->entries + position * entry_size;
}

/// The entry at a position, for the generated code of growing tables.
FA_EXPORT __attribute__((always_inline)) uint8_t *fa_table_entry(const FaTable *table,
                                                                 int64_t position,
                                                                 int64_t entry_size) {
	return entry_at(table, position, entry_size, 1);
}

/// Rebuilds the index of the entries (`slots` a power of two).
static void table_index(FaTable *table, int64_t slots, int64_t entry_size) {
	if (table->index != 0) {
		fa_free(table->index, (size_t)((table->mask + 1) * 8), 8);
	}
	table->index = fa_alloc((size_t)(slots * 8), 8);
	__builtin_memset(table->index, 0, (size_t)(slots * 8));
	table->mask = slots - 1;
	for (int64_t position = 0; position < table->used; position += 1) {
		uint8_t *entry = entry_at(table, position, entry_size, table->growing != 0);
		if (ENTRY_LIVE(entry)) {
			int64_t slot = (int64_t)(ENTRY_HASH(entry) & (uint64_t)table->mask);
			while (table->index[slot] != 0) {
				slot = (slot + 1) & table->mask;
			}
			table->index[slot] = position + 1;
		}
	}
}

/// The number of index slots for `capacity` entries.
static int64_t index_slots(int64_t capacity) {
	int64_t slots = 8;
	while (slots < capacity * 2) {
		slots *= 2;
	}
	return slots;
}

/// A new empty table with room for `capacity` entries.
FA_EXPORT FaTable *fa_table_new(int64_t capacity, int64_t entry_size) {
	FaTable *table = fa_alloc(sizeof(FaTable), sizeof(int64_t));
	if (capacity < 4) {
		capacity = 4;
	}
	table->entries = fa_alloc((size_t)(capacity * entry_size), 16);
	table->length = 0;
	table->used = 0;
	table->capacity = capacity;
	table->index = 0;
	table->growing = 0;
	table_index(table, index_slots(capacity), entry_size);
	return table;
}

/// A new empty growing table (`GrowingMap`, `GrowingSet`), its index sized
/// for `capacity` entries.
FA_EXPORT FaTable *fa_table_new_growing(int64_t capacity, int64_t entry_size) {
	FaTable *table = fa_alloc(sizeof(FaTable), sizeof(int64_t));
	if (capacity < 4) {
		capacity = 4;
	}
	table->entries = (uint8_t *)fa_segmented_new();
	table->length = 0;
	table->used = 0;
	table->capacity = capacity;
	table->index = 0;
	table->growing = 1;
	table_index(table, index_slots(capacity), entry_size);
	return table;
}

/// The position of the entry of a key, or -1.
static __attribute__((always_inline)) inline int64_t
table_find(const FaTable *table, uint64_t hash, const void *key,
           int32_t (*equals)(const void *, const void *), int64_t entry_size, int growing) {
	int64_t slot = (int64_t)(hash & (uint64_t)table->mask);
	for (;;) {
		int64_t found = table->index[slot];
		if (found == 0) {
			return -1;
		}
		if (found > 0) {
			uint8_t *entry = entry_at(table, found - 1, entry_size, growing);
			if (ENTRY_HASH(entry) == hash && equals(ENTRY_KEY(entry), key)) {
				return found - 1;
			}
		}
		slot = (slot + 1) & table->mask;
	}
}

FA_EXPORT __attribute__((always_inline)) int64_t
fa_table_find(const FaTable *table, uint64_t hash, const void *key,
              int32_t (*equals)(const void *, const void *), int64_t entry_size) {
	return table_find(table, hash, key, equals, entry_size, 0);
}

FA_EXPORT __attribute__((always_inline)) int64_t
fa_table_find_growing(const FaTable *table, uint64_t hash, const void *key,
                      int32_t (*equals)(const void *, const void *), int64_t entry_size) {
	return table_find(table, hash, key, equals, entry_size, 1);
}

/// Makes room for one more entry: the dead entries are compacted away, and
/// the entries grow when more than half of them are live. A growing table
/// only grows its index: its entries never move.
static void table_reserve(FaTable *table, int64_t entry_size) {
	if (table->used < table->capacity) {
		return;
	}
	if (table->growing) {
		table->capacity *= 2;
		table_index(table, index_slots(table->capacity), entry_size);
		return;
	}
	int64_t capacity = table->capacity;
	if (table->length * 2 > table->used) {
		capacity *= 2;
	}
	uint8_t *entries = fa_alloc((size_t)(capacity * entry_size), 16);
	int64_t count = 0;
	for (int64_t position = 0; position < table->used; position += 1) {
		uint8_t *entry = table->entries + position * entry_size;
		if (ENTRY_LIVE(entry)) {
			copy_bytes(entries + count * entry_size, entry, (size_t)entry_size);
			count += 1;
		}
	}
	fa_free(table->entries, (size_t)(table->capacity * entry_size), 16);
	table->entries = entries;
	table->used = count;
	table->capacity = capacity;
	int64_t slots = table->mask + 1;
	while (slots < capacity * 2) {
		slots *= 2;
	}
	table_index(table, slots, entry_size);
}

/// A new live entry for a key that is absent, at the end of the order: its
/// key and value are the generated code's to write.
FA_EXPORT void *fa_table_append(FaTable *table, uint64_t hash, int64_t entry_size) {
	table_reserve(table, entry_size);
	uint8_t *entry;
	if (table->growing) {
		entry = fa_segmented_push_slot((FaSegmented *)table->entries, entry_size,
		                               TABLE_SEGMENT_BITS);
	} else {
		entry = table->entries + table->used * entry_size;
	}
	ENTRY_HASH(entry) = hash;
	ENTRY_LIVE(entry) = 1;
	int64_t slot = (int64_t)(hash & (uint64_t)table->mask);
	while (table->index[slot] > 0) {
		slot = (slot + 1) & table->mask;
	}
	table->index[slot] = table->used + 1;
	table->used += 1;
	table->length += 1;
	return entry;
}

/// Marks an entry dead (its key and value were dropped or moved out).
FA_EXPORT void fa_table_kill(FaTable *table, int64_t position, int64_t entry_size) {
	uint8_t *entry = entry_at(table, position, entry_size, table->growing != 0);
	int64_t slot = (int64_t)(ENTRY_HASH(entry) & (uint64_t)table->mask);
	while (table->index[slot] != position + 1) {
		slot = (slot + 1) & table->mask;
	}
	table->index[slot] = -1;
	ENTRY_LIVE(entry) = 0;
	table->length -= 1;
}

/// Empties a table whose keys and values were dropped.
FA_EXPORT void fa_table_clear(FaTable *table) {
	table->length = 0;
	table->used = 0;
	if (table->growing) {
		((FaSegmented *)table->entries)->length = 0;
	}
	__builtin_memset(table->index, 0, (size_t)((table->mask + 1) * 8));
}

/// Frees a table whose keys and values were dropped.
FA_EXPORT void fa_table_free(FaTable *table, int64_t entry_size) {
	if (table->growing) {
		fa_segmented_free((FaSegmented *)table->entries, entry_size, TABLE_SEGMENT_BITS);
	} else {
		fa_free(table->entries, (size_t)(table->capacity * entry_size), 16);
	}
	fa_free(table->index, (size_t)((table->mask + 1) * 8), 8);
	fa_free(table, sizeof(FaTable), sizeof(int64_t));
}

/// A copy of the live entries of a table, bytewise (the generated code then
/// copies the keys and values that own memory).
FA_EXPORT FaTable *fa_table_copy(const FaTable *table, int64_t entry_size) {
	int growing = table->growing != 0;
	int64_t capacity = table->length < 4 ? 4 : table->length;
	FaTable *copy = growing ? fa_table_new_growing(capacity, entry_size)
	                        : fa_table_new(capacity, entry_size);
	int64_t count = 0;
	for (int64_t position = 0; position < table->used; position += 1) {
		uint8_t *entry = entry_at(table, position, entry_size, growing);
		if (ENTRY_LIVE(entry)) {
			uint8_t *slot = growing ? fa_segmented_push_slot((FaSegmented *)copy->entries,
			                                                 entry_size, TABLE_SEGMENT_BITS)
			                        : copy->entries + count * entry_size;
			copy_bytes(slot, entry, (size_t)entry_size);
			count += 1;
		}
	}
	copy->used = count;
	copy->length = count;
	table_index(copy, copy->mask + 1, entry_size);
	return copy;
}

/// The hash of bytes (FNV-1a, then mixed).
/// The hash of a string's bytes, eight bytes at a time (tables are
/// insertion-ordered: the hash is never observable).
FA_EXPORT uint64_t fa_hash_bytes(const uint8_t *bytes, size_t length) {
	const uint64_t multiplier = 0x9E3779B97F4A7C15ull;
	uint64_t hash = 0xcbf29ce484222325ull ^ (length * multiplier);
	size_t index = 0;
	for (; index + 8 <= length; index += 8) {
		uint64_t word;
		__builtin_memcpy(&word, bytes + index, 8);
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
			__builtin_memcpy(&low, bytes + index, 4);
			__builtin_memcpy(&high, bytes + length - 4, 4);
			word = low | ((uint64_t)high << 32);
		} else {
			word = bytes[index] | ((uint64_t)bytes[index + rest / 2] << 8) |
			       ((uint64_t)bytes[length - 1] << 16);
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

typedef struct FaExecutor {
	/// The ready queue.
	FaTask *head;
	FaTask *tail;
	int64_t ready;
	/// The timers: a 4-ary min-heap by (deadline, sequence).
	FaWait **timers;
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

// ── the ready queue

static void ready_push(FaExecutor *e, FaTask *task) {
	task->next = 0;
	task->prev = e->tail;
	if (e->tail != 0) {
		e->tail->next = task;
	} else {
		e->head = task;
	}
	e->tail = task;
	e->ready += 1;
	set_task_state(task, TASK_QUEUED);
}

static FaTask *ready_pop(FaExecutor *e) {
	FaTask *task = e->head;
	if (task == 0) {
		return 0;
	}
	e->head = task->next;
	if (e->head != 0) {
		e->head->prev = 0;
	} else {
		e->tail = 0;
	}
	e->ready -= 1;
	task->next = 0;
	task->prev = 0;
	return task;
}

static void ready_unlink(FaExecutor *e, FaTask *task) {
	if (task->prev != 0) {
		task->prev->next = task->next;
	} else {
		e->head = task->next;
	}
	if (task->next != 0) {
		task->next->prev = task->prev;
	} else {
		e->tail = task->prev;
	}
	task->next = 0;
	task->prev = 0;
	e->ready -= 1;
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
	__atomic_fetch_sub(&e->sources, 1, __ATOMIC_ACQ_REL);
#if !defined(__wasm__)
	if (__atomic_exchange_n(&e->parked, 0, __ATOMIC_SEQ_CST) != 0 && e->event >= 0) {
		uint64_t one = 1;
		write(e->event, &one, sizeof one);
	}
#endif
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
	return fa_coop_budget <= 0;
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
	ready_push(executor(), task_of(frame));
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

static int timer_before(const FaWait *a, const FaWait *b) {
	return a->deadline < b->deadline || (a->deadline == b->deadline && a->sequence < b->sequence);
}

static void timer_place(FaExecutor *e, int64_t index, FaWait *wait) {
	e->timers[index] = wait;
	wait->index = index;
}

static void timer_up(FaExecutor *e, int64_t index) {
	FaWait *wait = e->timers[index];
	while (index > 0) {
		int64_t parent = (index - 1) / 4;
		if (!timer_before(wait, e->timers[parent])) {
			break;
		}
		timer_place(e, index, e->timers[parent]);
		index = parent;
	}
	timer_place(e, index, wait);
}

static void timer_down(FaExecutor *e, int64_t index) {
	FaWait *wait = e->timers[index];
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
		if (!timer_before(e->timers[best], wait)) {
			break;
		}
		timer_place(e, index, e->timers[best]);
		index = best;
	}
	timer_place(e, index, wait);
}

static void timer_remove(FaExecutor *e, int64_t index) {
	FaWait *removed = e->timers[index];
	e->timer_count -= 1;
	if (index < e->timer_count) {
		FaWait *last = e->timers[e->timer_count];
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
FA_EXPORT void fa_sleep_async(FaWait *wait, void *frame, int64_t milliseconds) {
	FaExecutor *e = executor();
	wait->frame = frame;
	wait->executor = e;
	wait->index = -1;
	if (milliseconds <= 0) {
		ready_push(e, task_of(frame));
		return;
	}
	if (e->timer_count == e->timer_room) {
		int64_t room = e->timer_room < 16 ? 16 : e->timer_room * 2;
		FaWait **timers = fa_alloc((size_t)room * sizeof(FaWait *), sizeof(void *));
		for (int64_t index = 0; index < e->timer_count; index += 1) {
			timers[index] = e->timers[index];
		}
		if (e->timer_room > 0) {
			fa_free(e->timers, (size_t)e->timer_room * sizeof(FaWait *), sizeof(void *));
		}
		e->timers = timers;
		e->timer_room = room;
	}
	wait->deadline = fa_clock() + milliseconds * 1000000;
	wait->sequence = e->sequence;
	e->sequence += 1;
	e->timer_count += 1;
	timer_place(e, e->timer_count - 1, wait);
	timer_up(e, e->timer_count - 1);
	set_task_state(task_of(frame), TASK_WAITING);
	if (frame == e->root || e->root == 0) {
		e->root_site = 0;
	}
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
	while (e->timer_count > 0 && e->timers[0]->deadline <= now) {
		FaWait *wait = e->timers[0];
		timer_remove(e, 0);
		ready_push(e, task_of(wait->frame));
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
/// is readable (or writable): `AWAIT_SUSPEND`, or `AWAIT_READY` when epoll
/// said it is ready since (try again).
FA_EXPORT int32_t fa_io_wait(FaIo *io, int32_t writable, FaWait *wait, void *frame) {
	uint32_t bit = writable ? IO_WRITABLE : IO_READABLE;
	if (io->ready & bit) {
		// the readiness came before this `EAGAIN` was seen: clear it, try once more
		io->ready &= ~bit;
		poll_events(io->executor, 0);
		if (io->ready & bit) {
			return AWAIT_READY;
		}
	}
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
		}
		FaEpollEvent events[64];
		int count = epoll_wait(e->epoll, events, 64, milliseconds);
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
	int64_t count = e->ready < EVENT_INTERVAL ? e->ready : EVENT_INTERVAL;
	for (; count > 0; count -= 1) {
		FaTask *task = ready_pop(e);
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
		// the descriptors every 61 turns (a system call)
		if (e->epoll >= 0 && e->turns >= EVENT_INTERVAL) {
			e->turns = 0;
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
		int64_t wait = e->timers[0]->deadline - now;
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
		fa_free(e->timers, (size_t)e->timer_room * sizeof(FaWait *), sizeof(void *));
	}
#if !defined(__wasm__)
	if (e->epoll >= 0) {
		close(e->epoll);
	}
	if (e->event >= 0) {
		close(e->event);
	}
#endif
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
	int64_t count = e->ready < EVENT_INTERVAL ? e->ready : EVENT_INTERVAL;
	for (; count > 0; count -= 1) {
		FaTask *task = ready_pop(e);
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
		int64_t wait = e->timers[0]->deadline - now;
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
	return has_threads() && (in_parallel || __atomic_load_n(&pool.running, __ATOMIC_ACQUIRE));
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
