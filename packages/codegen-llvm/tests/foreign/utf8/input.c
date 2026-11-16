#include <stdint.h>

const char *fa_test_utf8_text(int32_t index) {
	static const char *const cases[] = {
		"\xEF\xBB\xBF\xC3\xA9\xF0\x9F\x98\x80",
		"\xF0\x9F", "\xED\xA0\x80", "\xE1\x80x", "\xC0\xAF",
		"\xF4\x90\x80\x80", "\xEF\xBB\xBF\xE2\x82", "\x80z", "",
		"a long C string requiring an owned replacement buffer: \xE1\x80x"
	};
	return cases[index];
}
