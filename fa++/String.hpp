#pragma once
#include <string>

using std::string;




struct String : string {
	size_t __length { size_t(-1) };

	// default (1)
	String() : string() {}

	// copy (2)
	String(const String& str) : string(str) {
	}

	// substring (3)
	String(const String& str, size_t pos, size_t len = npos) : string(str, pos, len) {
	}

	// from c-String (4)	
	String(const char* s) : string(s) {
		// __length = 0;
		// char c;
		// int p = 0;

		// while ((c = s[p])) {
		// 	__length++;
		// 	if ((c & 128) == 0)     p ++;
		// 	else if ((c & 32) == 0) p += 2;
		// 	else if ((c & 16) == 0) p += 3;
		// 	else if ((c & 8) == 0)  p += 4;
		// 	else p++; // 10xxxxxx should not happen in valid utf8
		// }
	}

	// from buffer (5)	
	String(const char* s, size_t n) : string(s, n) {
		// calculate_length();
	}

	// fill (6)	
	String(size_t n, char c) : string(n, c) {
		// calculate_length();
	}

	// move (9)	 
	String(String&& str) noexcept : string(str) {
		// __length = str.length();
	}

	size_t length() noexcept {		
		if (__length == -1) {
			const char* s = data();
			char c;
			size_t p = 0;
			while (__length++, (c = s[p])) {
				if ((c & 128) == 0)     p ++;
				else if ((c & 32) == 0) p += 2;
				else if ((c & 16) == 0) p += 3;
				else if ((c & 8) == 0)  p += 4;
				else p++; // 10xxxxxx should not happen in valid utf8
			}
		}
		return __length;
	}

	inline void calculate_length() {
		size_t p = 0;
		__length = 0;
		char c;

		while ((c = data()[p])) {
			__length++;
			if ((c & 128) == 0)     p ++;
			else if ((c & 32) == 0) p += 2;
			else if ((c & 16) == 0) p += 3;
			else if ((c & 8) == 0)  p += 4;
			else p++; // 10xxxxxx should not happen in valid utf8
		}
	}


};