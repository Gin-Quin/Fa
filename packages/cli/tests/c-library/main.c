/* A C program using the Fa library of `farm/` through its generated header
 * (`packages/cli/tests/c_library.rs`): linked with `libfarm.a`, then with
 * `libfarm.so`. Its output is compared with `expected.txt`. */
#include <pthread.h>
#include <stdio.h>
#include <string.h>

#include "farm.h"

/* A call on another thread: an error, and its message. */
static void *on_thread(void *message) {
	int32_t quotient = 0;
	int status = farm_divide(1, 0, &quotient);
	snprintf(message, 200, "%d %s", status, farm_last_error_message());
	return NULL;
}

int main(void) {
	/* the module values were initialized when the library was loaded */
	printf("squares: %lld\n", (long long)farm_sum_of_squares());
	fflush(stdout);
	farm_greet("C");
	printf("after greet\n");
	fflush(stdout);

	/* a Fa value behind a pointer */
	farm_Farm *farm = farm_create_farm("Green Acres");
	printf("name length: %lld\n", (long long)farm_name_length(farm));
	int64_t count = -1;
	int status = farm_animal_count(farm, &count);
	printf("count status: %d (empty: %d), count: %lld, message: %s\n", status,
	       status == FARM_FARM_ERROR_EMPTY, (long long)count, farm_last_error_message());
	status = farm_add_animals(farm, 7);
	printf("add status: %d\n", status);
	status = farm_animal_count(farm, &count);
	printf("count status: %d, count: %lld\n", status, (long long)count);
	status = farm_add_animals(farm, 200);
	printf("add status: %d (too many: %d), message: %s\n", status,
	       status == FARM_FARM_ERROR_TOO_MANY, farm_last_error_message());
	status = farm_animal_count(farm, NULL);
	printf("count without a result: %d\n", status);
	status = farm_rename(farm, "Blue Hills");
	printf("rename: %d, length %lld\n", status, (long long)farm_name_length(farm));
	/* a message longer than its buffer: cut before a character */
	char name[2003] = "xy";
	for (int at = 2; at < 2002; at += 2) {
		name[at] = (char)0xc3;
		name[at + 1] = (char)0xa9;
	}
	name[2002] = 0;
	status = farm_rename(farm, name);
	const char *long_message = farm_last_error_message();
	size_t length = strlen(long_message);
	printf("long name: %d (%d), message of %zu bytes, ending with a whole character: %d\n", status,
	       status == FARM_FARM_ERROR_BAD_NAME, length,
	       (unsigned char)long_message[length - 2] == 0xc3);
	farm_destroy_farm(farm);

	/* optional pointers */
	farm_Farm *maybe = farm_maybe_farm(true);
	printf("maybe: %s, length %lld\n", maybe ? "a farm" : "none", (long long)farm_name_length(maybe));
	farm_destroy_farm(maybe);
	printf("maybe not: %s\n", farm_maybe_farm(false) ? "a farm" : "none");

	/* C structs by value, in registers and in memory */
	farm_Point point = farm_scale((farm_Point){.x = 3, .y = 1.25f}, 4);
	printf("scaled: %d %.2f\n", point.x, point.y);
	printf("sum: %.1f\n", farm_sum((farm_Triple){.a = 1.0, .b = 2.0, .c = 3.5}));
	farm_Triple triple = farm_make_triple(1.5);
	printf("triple: %.1f %.1f %.1f\n", triple.a, triple.b, triple.c);

	/* errors with data, and results through a pointer */
	int32_t quotient = 0;
	status = farm_divide(17, 5, &quotient);
	printf("divide: %d %d\n", status, quotient);
	status = farm_divide(1, 0, &quotient);
	printf("divide by zero: %d (%d), %s\n", status, status == FARM_MATH_ERROR_DIVISION_BY_ZERO,
	       farm_last_error_message());
	status = farm_divide(-4, 2, &quotient);
	printf("negative: %d (%d), %s\n", status, status == FARM_MATH_ERROR_NEGATIVE,
	       farm_last_error_message());

	/* scalars */
	printf("even: %d %d\n", farm_is_even(10), farm_is_even(7));
	printf("half: %.2f\n", farm_half(5.0f));

	/* another thread: its own last error */
	char message[200];
	pthread_t thread;
	pthread_create(&thread, NULL, on_thread, message);
	pthread_join(thread, NULL);
	printf("thread: %s; here: %s\n", message, farm_last_error_message());
	return 0;
}
