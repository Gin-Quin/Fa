/* The Fa library loaded with `dlopen` after the program's start, as
 * Python's `ctypes` loads it (`packages/cli/tests/c_library.rs`): its
 * thread-local variables use the dynamic model, its module values are
 * initialized when it is loaded and dropped when it is unloaded. */
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>

int main(int argc, char **argv) {
	if (argc < 2) {
		return 2;
	}
	void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
	if (library == NULL) {
		printf("dlopen failed: %s\n", dlerror());
		return 1;
	}
	int64_t (*sum_of_squares)(void) = (int64_t(*)(void))dlsym(library, "farm_sum_of_squares");
	int (*divide)(int32_t, int32_t, int32_t *) =
	    (int (*)(int32_t, int32_t, int32_t *))dlsym(library, "farm_divide");
	const char *(*last_error_message)(void) =
	    (const char *(*)(void))dlsym(library, "farm_last_error_message");
	if (sum_of_squares == NULL || divide == NULL || last_error_message == NULL) {
		printf("a symbol is missing\n");
		return 1;
	}
	int32_t quotient = 0;
	int status = divide(1, 0, &quotient);
	printf("%lld %d %s\n", (long long)sum_of_squares(), status, last_error_message());
	/* an internal symbol of the library is not visible */
	printf("internal: %s\n", dlsym(library, "fa_flush") == NULL ? "hidden" : "visible");
	dlclose(library);
	printf("unloaded\n");
	return 0;
}
