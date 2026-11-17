// Reuse the existing runtime test platform and its allocation counters.
#define main fa_existing_runtime_tests
#include "../../../../packages/codegen-llvm/runtime/test/executor_test.c"
#undef main

static void measure_split(const FaString *source, const uint8_t *separator, size_t length) {
	long blocks = allocated;
	long bytes = allocated_bytes;
	FaArrayHeader *parts = fa_string_split_views(source->bytes, source->length,
		source->capacity, separator, length);
	FaString *items = (FaString *)parts->items;
	printf("blocks=%ld bytes=%ld first_owns=%d first_units=%zu\n",
		allocated - blocks, allocated_bytes - bytes,
		string_owned(items[0].capacity),
		fa_string_length(string_bytes(items), string_length(items), items[0].capacity));
	for (int64_t index = 0; index < parts->length; index += 1) {
		if (string_owned(items[index].capacity)) {
			fa_string_free(items[index].bytes, items[index].capacity);
		}
	}
	// Account for the complete combined allocation on this malloc platform.
	if (inline_items(parts)) {
		fa_free(parts, INLINE_ITEMS + (size_t)parts->capacity * sizeof(FaString), 16);
	} else {
		fa_array_free(parts, sizeof(FaString));
	}
}

int main(void) {
	uint8_t bytes[165];
	for (size_t index = 0; index < 80; index += 1) {
		bytes[index * 2] = 0xC3;
		bytes[index * 2 + 1] = 0xA9;
	}
	copy_bytes(bytes + 160, (const uint8_t *)"\xF0\x9F\x98\x80x", 5);
	FaString source = owned_copy(bytes, sizeof bytes);
	measure_split(&source, (const uint8_t *)"\xF0\x9F\x98\x80", 4);
	measure_split(&source, (const uint8_t *)"\xED\xB8\x80", 3);
	fa_string_free(source.bytes, source.capacity);
	CHECK(allocated == 0 && allocated_bytes == 0);
}
