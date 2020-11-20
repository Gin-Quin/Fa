#pragma once
// #include "Type.hpp"
#include <cstring>
#include <cstdlib>
#include <cstdio>


// static unsigned int utf8_length(const char* str) {
// 	char c;
// 	int p = 0;
// 	unsigned int length = 0;

// 	while ((c = str[p])) {
// 		length++;

// 		if ((c & 128) == 0)
// 			p++;
// 		else if ((c & 32) == 0)
// 			p += 2;
// 		else if ((c & 16) == 0)
// 			p += 3;
// 		else if ((c & 8) == 0)
// 			p += 4;
// 		else // 10xxxxxx should never happen if valid utf8
// 			p++;  // but we prevent infinite loops nonetheless
// 	}

// 	return length;
// }

// static inline unsigned int find_capacity() {

// }


struct MyString {
	union {
		const char* constant;
		char* dynamic;
		void* buffer;
	} __data { 0 };
	unsigned int __size { 0 };
	unsigned int __capacity { 0 };
	unsigned int __length { 0 };

	~MyString() {
		if (__capacity)
			free(__data.buffer);
	}

	MyString() {
		// puts("Empty constructor");
	}

	MyString(const char* from) {
		// puts("Const char* constructor");
		__data.constant = from;
		char c;
		while ((c = from[__size])) {
			__length++;
			if ((c & 128) == 0) __size++;
			else if ((c & 32) == 0) __size += 2;
			else if ((c & 16) == 0) __size += 3;
			else if ((c & 8) == 0) __size += 4;
			else __size++;  // should not happen if valid utf8 (10xxxxxx case)
		}
	}

	MyString(const MyString& other) {
		// puts("Copy constructor");
		__size = other.__size;
		__length = other.__length;
		__capacity = other.__capacity;

		// we copy the dynamic data from the other MyString
		if (__capacity) {
			__data.buffer = malloc(__capacity);
			memcpy(__data.buffer, (const void*) other.__data.buffer, __size + 1);
		}

		// or we reference to the same constant MyString
		else {
			__data.constant = other.__data.constant;
		}
	}

	MyString(MyString&& other) {
		// puts("Move constructor");
		__size = other.__size;
		__length = other.__length;
		__capacity = other.__capacity;
		__data.buffer = other.__data.buffer;
		other.__capacity = 0;

		// we copy the dynamic data from the other MyString
		// if (__capacity) {
		// 	__data.buffer = malloc(__capacity);
		// 	memcpy(__data.buffer, (const void*) other.__data.buffer, __size + 1);
		// }

		// or we reference to the same constant MyString
		// else {
		// }
	}

	MyString& operator=(const char* from) {
		if (__capacity) {
			free(__data.buffer);
			__capacity = 0;
		}
		__data.constant = from;
		__size = 0;
		__length = 0;
		char c;
		while ((c = from[__size])) {
			__length++;
			if ((c & 128) == 0) __size++;
			else if ((c & 32) == 0) __size += 2;
			else if ((c & 16) == 0) __size += 3;
			else if ((c & 8) == 0) __size += 4;
			else __size++;  // should not happen if valid utf8 (10xxxxxx case)
		}
		return *this;
	}

	MyString& operator=(MyString& other) {
		// puts(" = from other");
		__size = other.__size;
		__length = other.__length;

		// we copy the dynamic data from the other MyString
		if (other.__capacity) {
			if (__capacity == 0) {
				__capacity = __size + 1;
				// __capacity = 8;
				// while (__capacity <= totalSize) __capacity *= 2;
				__data.buffer = malloc(__capacity);
			}
			else if (__capacity <= __size) {
				__capacity = __size + 1;
				// __capacity = 8;
				// while (__capacity <= totalSize) __capacity *= 2;
				__data.buffer = realloc(__data.buffer, __capacity);
			}
			memcpy(__data.dynamic, other.__data.constant, __size);
		}

		// or we reference to the same constant MyString
		else {
			if (__capacity) {
				free(__data.buffer);
				__capacity = 0;
			}
			__data.constant = other.__data.constant;
		}

		return *this;
	}

	// - Methods
	// MyString& append(const char* str) {
	// 	if (str[0] == 0) return *this;

	// 	// we find the constant MyString's size & length
	// 	unsigned int size = 0, length = 0;
	// 	char c;
	// 	while ((c = str[size])) {
	// 			__size++;
	// 		if (c & 128 == 0) length++;
	// 		else if (c & 32 == 0) length += 2;
	// 		else if (c & 16 == 0) length += 3;
	// 		else if (c & 8 == 0) length += 4;
	// 		else length++;  // should not happen if valid utf8 (10xxxxxx case)
	// 	}

	// 	if (__capacity == 0) {
	// 		__capacity = __size + size + 1;

	// 	}
		

	// 	return *this;
	// }

	MyString& append(const MyString& other) {
		auto totalSize = __size + other.__size;

		if (__capacity) {
			if (__capacity <= totalSize) {
				while (__capacity <= totalSize)
					__capacity *= 2;
				__data.buffer = realloc(__data.buffer, __capacity);
			}
		}

		else {
			__capacity = totalSize + 1;
			// while (__capacity <= totalSize)
			// 	__capacity *= 2;
			const char* data = __data.constant;
			__data.buffer = malloc(__capacity);
			strcpy(__data.dynamic, data);
		}

		strcpy(__data.dynamic + __size, other.__data.constant);
		__size = totalSize;
		__length += other.__length;
		return *this;
	}

	// - Operations
	inline MyString& operator+=(const MyString& other) { return this->append(other); }
	// inline MyString& operator+=(const char* str) { return this->append(str); }
	// inline MyString& operator+=(Integer i) { return this->append(i); }

	// - Getters
	inline unsigned int size() { return __size; }
	inline const char* data() { return __data.constant; }
};


// -- OPERATOR OVERLOADING
// MyString& operator+(const MyString& s1, const MyString& s2) {
	
// }