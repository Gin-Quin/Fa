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

/// A string: UTF-8 bytes, `{ ptr, usize }` in the generated code.
typedef struct {
	const uint8_t *bytes;
	size_t length;
} FaString;

/// An array: `{ ptr, usize }` in the generated code.
typedef struct {
	void *items;
	size_t length;
} FaArray;

static void copy_bytes(uint8_t *to, const uint8_t *from, size_t length) {
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

#if defined(__wasm__)
// exported, so that the loader flushes the output after a trap
__attribute__((export_name("flush")))
#endif
FA_EXPORT void fa_flush(void) {
	if (output_length > 0) {
		fa_write(1, output, output_length);
		output_length = 0;
	}
}

FA_EXPORT void fa_out(int32_t fd, const uint8_t *bytes, size_t length) {
	if (fd != 1) {
		fa_flush();
		fa_write(fd, bytes, length);
		return;
	}
	if (output_length + length > OUTPUT_CAPACITY) {
		fa_flush();
		if (length > OUTPUT_CAPACITY) {
			fa_write(1, bytes, length);
			return;
		}
	}
	copy_bytes(output + output_length, bytes, length);
	output_length += length;
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
static uint8_t *builder;
static size_t builder_length;
static size_t builder_capacity;

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

/// The built text as a new string.
FA_EXPORT void fa_b_take(FaString *out) {
	uint8_t *bytes = fa_alloc(builder_length == 0 ? 1 : builder_length, 1);
	copy_bytes(bytes, builder, builder_length);
	out->bytes = bytes;
	out->length = builder_length;
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
	uint8_t *bytes = fa_alloc(length == 0 ? 1 : length, 1);
	copy_bytes(bytes, left, left_length);
	copy_bytes(bytes + left_length, right, right_length);
	out->bytes = bytes;
	out->length = length;
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
	for (size_t index = 0; index < length; index = next_boundary(bytes, length, index)) {
		count += 1;
	}
	return count;
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

/// The byte offset of the first (or last) match of `search` whose two ends
/// are grapheme boundaries, or -1 (`faFindMatches`).
static int64_t find(const uint8_t *text, size_t text_length, const uint8_t *search,
                    size_t search_length, int last) {
	int64_t found = -1;
	for (size_t offset = 0; offset + search_length <= text_length; offset += 1) {
		if (bytes_at(text, text_length, offset, search, search_length) &&
		    is_boundary(text, text_length, offset) &&
		    is_boundary(text, text_length, offset + search_length)) {
			found = (int64_t)offset;
			if (!last) {
				break;
			}
		}
	}
	return found;
}

FA_EXPORT int32_t fa_string_includes(const uint8_t *text, size_t text_length,
                                     const uint8_t *search, size_t search_length) {
	return find(text, text_length, search, search_length, 0) >= 0;
}

FA_EXPORT int32_t fa_string_starts_with(const uint8_t *text, size_t text_length,
                                        const uint8_t *search, size_t search_length) {
	return bytes_at(text, text_length, 0, search, search_length) &&
	       is_boundary(text, text_length, search_length);
}

FA_EXPORT int32_t fa_string_ends_with(const uint8_t *text, size_t text_length,
                                      const uint8_t *search, size_t search_length) {
	return search_length <= text_length &&
	       bytes_at(text, text_length, text_length - search_length, search, search_length) &&
	       is_boundary(text, text_length, text_length - search_length);
}

/// `indexOf` (`last` 0) and `lastIndexOf` (1): the grapheme position of the
/// match, or -1 for `none`.
FA_EXPORT int64_t fa_string_index_of(const uint8_t *text, size_t text_length,
                                     const uint8_t *search, size_t search_length, int32_t last) {
	int64_t offset = find(text, text_length, search, search_length, last);
	return offset < 0 ? -1 : (int64_t)grapheme_index(text, text_length, (size_t)offset);
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
	out->bytes = bytes + start;
	out->length = end - start;
}

/// `text.repeat(count)`.
FA_EXPORT void fa_string_repeat(const uint8_t *bytes, size_t length, int64_t count, FaString *out) {
	fa_b_reset();
	for (int64_t index = 0; index < count; index += 1) {
		fa_b_bytes(bytes, length);
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
	}
}
#else
/// Asks the host for the user arguments.
FA_EXPORT void fa_arguments_init(int32_t argc, char **argv) {
	(void)argc;
	(void)argv;
	size_t size = (size_t)fa_host_arguments_size();
	uint8_t *bytes = fa_alloc(size == 0 ? 1 : size, 1);
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
			item += 1;
			start = index + 1;
		}
	}
}
#endif

// ── arrays ──────────────────────────────────────────────────────────────

/// The header of an array (`src/lower/arrays.rs`).
typedef struct {
	uint8_t *items;
	int64_t length;
	int64_t capacity;
} FaArrayHeader;

/// A new array of `length` items (not initialized), with room for `capacity`.
FA_EXPORT FaArrayHeader *fa_array_new(int64_t length, int64_t capacity, int64_t item_size) {
	FaArrayHeader *array = fa_alloc(sizeof(FaArrayHeader), sizeof(int64_t));
	if (capacity < length) {
		capacity = length;
	}
	size_t bytes = (size_t)(capacity * item_size);
	array->items = fa_alloc(bytes == 0 ? 1 : bytes, 16);
	array->length = length;
	array->capacity = capacity;
	return array;
}

/// Makes room for one more item and returns its slot (`push`).
FA_EXPORT void *fa_array_push_slot(FaArrayHeader *array, int64_t item_size) {
	if (array->length == array->capacity) {
		int64_t capacity = array->capacity < 4 ? 4 : array->capacity * 2;
		uint8_t *items = fa_alloc((size_t)(capacity * item_size), 16);
		copy_bytes(items, array->items, (size_t)(array->length * item_size));
		fa_free(array->items, (size_t)(array->capacity * item_size), 16);
		array->items = items;
		array->capacity = capacity;
	}
	void *slot = array->items + array->length * item_size;
	array->length += 1;
	return slot;
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

/// `arguments`: an `Array(String)`.
FA_EXPORT FaArrayHeader *fa_arguments(void) {
	FaArrayHeader *array = fa_alloc(sizeof(FaArrayHeader), sizeof(int64_t));
	array->items = (uint8_t *)argument_items;
	array->length = (int64_t)argument_count;
	array->capacity = (int64_t)argument_count;
	return array;
}

// ── stops (values brief 2.7) ────────────────────────────────────────────

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
	error->input.bytes = bytes;
	error->input.length = length;
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
