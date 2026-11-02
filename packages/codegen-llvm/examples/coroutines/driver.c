// The hand-written runtime of the coroutine spike (`coroutines/spike.rs`):
// the allocator of the coroutine frames, the run queue of the tasks, and, on
// native, the `main` that times the benchmarks of the IR module. On wasm,
// `bench.mjs` times them. It compiles with `cc` (native) and with clang
// `--target=wasm32 -nostdlib` (freestanding: no libc).

#include <stdint.h>

// --- allocator: free lists of 16-byte size classes over a static arena ---

#define ARENA_SIZE (64u << 20)
#define CLASSES 64
#define HEADER 16

static _Alignas(16) unsigned char arena[ARENA_SIZE];
static uintptr_t arena_used;
static void *free_lists[CLASSES];

// counters read by the benchmarks
uint64_t fa_alloc_count;
uint64_t fa_last_alloc_size;

void *fa_alloc(uint64_t size) {
	fa_alloc_count += 1;
	fa_last_alloc_size = size;
	uint64_t class_index = (size + 15) / 16;
	if (class_index >= CLASSES) {
		// a large block, never reused (`fa_free` leaks it)
		class_index = CLASSES;
	}
	void *block = class_index < CLASSES ? free_lists[class_index] : 0;
	if (block) {
		free_lists[class_index] = *(void **)block;
		return block;
	}
	uintptr_t total = HEADER + (class_index < CLASSES ? class_index * 16 : (size + 15) / 16 * 16);
	if (arena_used + total > ARENA_SIZE) {
		__builtin_trap();
	}
	unsigned char *start = arena + arena_used;
	arena_used += total;
	*(uint64_t *)start = class_index;
	return start + HEADER;
}

void fa_free(void *block) {
	uint64_t class_index = *(uint64_t *)((unsigned char *)block - HEADER);
	if (class_index == CLASSES) {
		return;
	}
	*(void **)block = free_lists[class_index];
	free_lists[class_index] = block;
}

// --- run queue: a ring of coroutine handles ---

#define QUEUE_SIZE (1u << 20)
static void *queue[QUEUE_SIZE];
static uint32_t queue_head, queue_tail;

void fa_queue_push(void *handle) {
	if (queue_tail - queue_head == QUEUE_SIZE) {
		__builtin_trap();
	}
	queue[queue_tail++ & (QUEUE_SIZE - 1)] = handle;
}

void *fa_queue_pop(void) {
	if (queue_head == queue_tail) {
		return 0;
	}
	return queue[queue_head++ & (QUEUE_SIZE - 1)];
}

#ifndef __wasm__

#include <stdio.h>
#include <stdlib.h>
#include <time.h>

// the benchmarks of the IR module (`spike.rs`, `BENCHMARKS`)
int64_t bench_gen_opaque(int64_t);
int64_t bench_gen_inline(int64_t);
int64_t bench_sm_direct(int64_t);
int64_t bench_sm_indirect(int64_t);
int64_t bench_sm_inline(int64_t);
int64_t bench_task_yield(int64_t);
int64_t bench_task_sync(int64_t);
int64_t bench_task_spawn(int64_t);

static const struct {
	const char *name;
	int64_t (*run)(int64_t);
} benchmarks[] = {
	{"gen_opaque", bench_gen_opaque}, {"gen_inline", bench_gen_inline},
	{"sm_direct", bench_sm_direct},   {"sm_indirect", bench_sm_indirect},
	{"sm_inline", bench_sm_inline},   {"task_yield", bench_task_yield},
	{"task_sync", bench_task_sync},   {"task_spawn", bench_task_spawn},
};

static double now(void) {
	struct timespec time;
	clock_gettime(CLOCK_MONOTONIC, &time);
	return time.tv_sec * 1e9 + time.tv_nsec;
}

// `driver <n> [name]`: prints `name n result allocations frame_size nanoseconds`
int main(int argc, char **argv) {
	int64_t n = argc > 1 ? atoll(argv[1]) : 1000;
	for (unsigned index = 0; index < sizeof benchmarks / sizeof *benchmarks; index += 1) {
		const char *name = benchmarks[index].name;
		if (argc > 2) {
			const char *wanted = argv[2], *candidate = name;
			while (*wanted && *wanted == *candidate) {
				wanted += 1;
				candidate += 1;
			}
			if (*wanted || *candidate) {
				continue;
			}
		}
		fa_alloc_count = 0;
		fa_last_alloc_size = 0;
		double start = now();
		int64_t result = benchmarks[index].run(n);
		double elapsed = now() - start;
		printf("%s %lld %lld %llu %llu %.0f\n", name, (long long)n, (long long)result,
		       (unsigned long long)fa_alloc_count, (unsigned long long)fa_last_alloc_size,
		       elapsed);
	}
	return 0;
}

#endif
