// A test of the event loop of the runtime (the "tasks and the event loop"
// section of `fa_runtime.c`), built with the C library and AddressSanitizer
// by `runtime/test/run.sh` (and `tests/runtime.rs`): the coroutines are
// hand-made state machines with the layout of an LLVM frame (the resume and
// destroy functions, then the task header at 16 bytes), and the platform
// layer of `src/runtime.rs` is written here with the C library.
//
//   packages/codegen-llvm/runtime/test/run.sh

#include "../fa_runtime.c"
#include "fa_grapheme_cases.h"

int printf(const char *format, ...);
int fflush(void *stream);
void *malloc(size_t size);
void *realloc(void *pointer, size_t size);
void *aligned_alloc(size_t align, size_t size);
void free(void *pointer);
void abort(void);
__attribute__((noreturn)) void exit(int code);
int nanosleep(const void *request, void *remaining);

// ── the platform layer

void fa_write(int32_t fd, const uint8_t *bytes, size_t length) {
	write(fd, bytes, length);
}

void fa_exit(int32_t code) {
	exit(code);
}

static long allocated;
static long allocated_bytes;

void *fa_alloc(size_t size, size_t align) {
	(void)align;
	__atomic_fetch_add(&allocated, 1, __ATOMIC_RELAXED);
	__atomic_fetch_add(&allocated_bytes, (long)size, __ATOMIC_RELAXED);
	return malloc(size);
}

void fa_free(void *bytes, size_t size, size_t align) {
	(void)size;
	(void)align;
	__atomic_fetch_sub(&allocated, 1, __ATOMIC_RELAXED);
	__atomic_fetch_sub(&allocated_bytes, (long)size, __ATOMIC_RELAXED);
	free(bytes);
}

void *fa_grow(void *bytes, size_t old, size_t used, size_t size) {
	(void)old;
	(void)used;
	return realloc(bytes, size);
}

// Like fa_pages below, refuse refills: allocator tests provide their pages
// explicitly and check that a full page returns failure without overrunning it.
void *fa_os_map(size_t length, int32_t protection) {
	(void)length;
	(void)protection;
	return 0;
}

void fa_os_unmap(void *start, size_t length) {
	(void)start;
	(void)length;
	abort(); // the test platform never creates a mapping
}

void *fa_os_remap(void *start, size_t old_length, size_t length, void *at) {
	(void)start;
	(void)old_length;
	(void)length;
	(void)at;
	return 0;
}

void *fa_os_reserve(void *start, size_t length) {
	(void)start;
	(void)length;
	return 0;
}

int32_t fa_os_commit(void *start, size_t length) {
	(void)start;
	(void)length;
	return -1;
}

// generated with the program (the printing of unions)
void fa_format_union(const void *box, int32_t nested) {
	(void)box;
	(void)nested;
}

void *fa_pages(size_t count) {
	(void)count;
	return 0;
}

int64_t fa_clock(void) {
	FaTimespec time;
	clock_gettime(1, &time);
	return time.seconds * 1000000000 + time.nanoseconds;
}

void fa_nanosleep(int64_t nanoseconds) {
	int64_t time[2] = {nanoseconds / 1000000000, nanoseconds % 1000000000};
	nanosleep(time, 0);
}

// ── hand-made coroutines

typedef struct Frame {
	void (*resume)(struct Frame *frame);
	void (*destroy)(struct Frame *frame);
	_Alignas(16) FaTask task;
	int step;
	int id;
	int rounds;
	int64_t value;
	FaWait wait;
	FaJoin join;
	void *channel;
	void *child;
	struct Frame **frames;
	int count;
	int cancelled;
} Frame;

_Static_assert(__builtin_offsetof(Frame, task) == TASK_AT, "the header of a frame");

void fa_resume(void *frame) {
	((Frame *)frame)->resume(frame);
}

void fa_destroy(void *frame) {
	((Frame *)frame)->destroy(frame);
}

#define CHECK(condition)                                                                           \
	do {                                                                                           \
		if (!(condition)) {                                                                        \
			printf("check failed at line %d: %s\n", __LINE__, #condition);                         \
			fflush(0);                                                                             \
			abort();                                                                               \
		}                                                                                          \
	} while (0)

static char trace[4096];
static size_t trace_length;

static void note(const char *text) {
	while (*text != 0 && trace_length + 1 < sizeof trace) {
		trace[trace_length] = *text;
		trace_length += 1;
		text += 1;
	}
	trace[trace_length] = 0;
}

static void note_number(int64_t number) {
	char digits[24];
	int length = 0;
	if (number < 0) {
		note("-");
		number = -number;
	}
	do {
		digits[length] = (char)('0' + number % 10);
		length += 1;
		number /= 10;
	} while (number != 0);
	char text[24];
	for (int index = 0; index < length; index += 1) {
		text[index] = digits[length - 1 - index];
	}
	text[length] = 0;
	note(text);
}

static void free_frame(Frame *frame) {
	fa_free(frame, sizeof(Frame), 16);
}

/// The end of a coroutine: its result stored, the final suspension.
static void finish(Frame *frame) {
	fa_task_finish(frame);
	frame->resume = 0;
}

static void destroy_frame(Frame *frame) {
	if (frame->resume != 0) {
		// cancelled at a suspension point: the cleanups of the step
		frame->cancelled = 1;
		if (frame->step == 100) {
			fa_timer_cancel(&frame->wait);
		} else if (frame->step == 200) {
			fa_channel_cancel(frame->channel, &frame->wait);
		} else if (frame->step == 300) {
			fa_task_drop(frame->child);
		}
	}
	free_frame(frame);
}

static Frame *new_frame(void (*resume)(Frame *), int id) {
	Frame *frame = fa_alloc(sizeof(Frame), 16);
	__builtin_memset(frame, 0, sizeof(Frame));
	frame->resume = resume;
	frame->destroy = destroy_frame;
	frame->task.state = TASK_RUNNING;
	frame->id = id;
	return frame;
}

// ── yields: tasks take turns in FIFO order

static void yielder(Frame *frame) {
	if (frame->step < frame->rounds) {
		note_number(frame->id);
		frame->step += 1;
		fa_task_yield(frame);
		return;
	}
	finish(frame);
}

static void yield_root(Frame *frame) {
	switch (frame->step) {
	case 0:
		for (int index = 0; index < 3; index += 1) {
			Frame *child = new_frame(yielder, index + 1);
			child->rounds = 2;
			fa_task_spawn(child, 1);
		}
		// waits on a timer while they run
		frame->step = 100;
		fa_sleep_async(&frame->wait, frame, 5);
		return;
	default:
		finish(frame);
	}
}

static void test_yields(void) {
	trace_length = 0;
	Frame *root = new_frame(yield_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
	CHECK(__builtin_strcmp(trace, "123123") == 0);
}

// ── timers fire in deadline order, ties in sleeping order

static void sleeper(Frame *frame) {
	if (frame->step == 0) {
		frame->step = 100;
		fa_sleep_async(&frame->wait, frame, frame->rounds);
		return;
	}
	note_number(frame->id);
	finish(frame);
}

static void timer_root(Frame *frame) {
	static const int delays[] = {30, 10, 20, 10, 0};
	if (frame->step == 0) {
		frame->step = 1;
		for (int index = 0; index < 5; index += 1) {
			Frame *child = new_frame(sleeper, index + 1);
			child->rounds = delays[index];
			fa_task_spawn(child, 1);
		}
		// a sleeper cancelled before its deadline
		Frame *never = new_frame(sleeper, 9);
		never->rounds = 1000;
		never->resume(never);
		fa_task_drop(never);
		// waits for the sleepers
		frame->step = 100;
		fa_sleep_async(&frame->wait, frame, 50);
		return;
	}
	finish(frame);
}

static void test_timers(void) {
	trace_length = 0;
	Frame *root = new_frame(timer_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
	CHECK(__builtin_strcmp(trace, "52431") == 0);
}

// ── a channel fed by other threads: remote wakes

#define SENT 20000
#define SENDERS 4

int pthread_join(unsigned long thread, void **result);

/// The executor of the main thread, for the threads of the tests.
static FaExecutor *current_executor_of_test;

static void *send_values(void *channel) {
	// as a thread of a parallel loop (the main thread can still receive)
	in_parallel = 1;
	for (int64_t value = 1; value <= SENT; value += 1) {
		CHECK(fa_channel_send(channel, &value, 1) == 1);
	}
	return 0;
}

static int received_total;

static void remote_root(Frame *frame) {
	for (;;) {
		if (frame->step == 200) {
			// resumed: the value was handed over (or received before a yield)
			CHECK(fa_wait_status(&frame->wait) == WAIT_DONE);
			frame->rounds += 1;
			frame->step = 0;
		}
		if (frame->rounds == SENT * SENDERS) {
			received_total = frame->rounds;
			finish(frame);
			return;
		}
		int32_t result =
		    fa_channel_receive_async(frame->channel, &frame->value, &frame->wait, frame, "receive");
		if (result == AWAIT_READY) {
			frame->rounds += 1;
			continue;
		}
		CHECK(result == AWAIT_SUSPEND || result == AWAIT_YIELD);
		frame->step = 200;
		return;
	}
}

static void *send_and_end(void *channel) {
	send_values(channel);
	fa_remote_source_end(current_executor_of_test);
	return 0;
}

static void test_remote_wakes(void) {
	void *channel = fa_channel_new(2, sizeof(int64_t), 0);
	Frame *root = new_frame(remote_root, 0);
	root->channel = channel;
	current_executor_of_test = executor();
	unsigned long threads[SENDERS];
	for (int index = 0; index < SENDERS; index += 1) {
		// the threads may wake the root: no deadlock while they run
		fa_remote_source_start(current_executor_of_test);
		pthread_create(&threads[index], 0, send_and_end, channel);
	}
	root->resume(root);
	fa_run_main(root, 0);
	for (int index = 0; index < SENDERS; index += 1) {
		pthread_join(threads[index], 0);
	}
	CHECK(received_total == SENT * SENDERS);
	fa_shared_release(channel);
}

// ── cancelling a task waiting in the line of a channel, and one awaiting another

static void receiver(Frame *frame) {
	if (frame->step == 0) {
		frame->step = 200;
		CHECK(fa_channel_receive_async(frame->channel, &frame->value, &frame->wait, frame,
		                               "receive") == AWAIT_SUSPEND);
		return;
	}
	note("received");
	finish(frame);
}

static void awaiter(Frame *frame) {
	if (frame->step == 0) {
		frame->step = 300;
		fa_task_await(frame->child, frame);
		return;
	}
	finish(frame);
}

static void cancel_root(Frame *frame) {
	switch (frame->step) {
	case 0: {
		Frame *waiting = new_frame(receiver, 1);
		waiting->channel = frame->channel;
		Frame *outer = new_frame(awaiter, 2);
		outer->child = waiting;
		fa_task_spawn(waiting, 0);
		fa_task_spawn(outer, 0);
		frame->child = outer;
		frame->step = 1;
		fa_task_yield(frame);
		return;
	}
	case 1:
		// both wait now: the drop cancels the outer task, which drops the inner one
		fa_task_drop(frame->child);
		frame->child = 0;
		{
			int64_t value = 7;
			// nobody waits anymore: the value stays in the channel
			CHECK(fa_channel_send(frame->channel, &value, 0) == 1);
		}
		finish(frame);
		return;
	}
}

static void test_cancel(void) {
	trace_length = 0;
	void *channel = fa_channel_new(1, sizeof(int64_t), 0);
	Frame *root = new_frame(cancel_root, 0);
	root->channel = channel;
	root->resume(root);
	fa_run_main(root, 0);
	CHECK(trace_length == 0);
	fa_shared_release(channel);
}

// ── a mutex hands its lock over in order

static void locker(Frame *frame) {
	switch (frame->step) {
	case 0:
		frame->step = 1;
		if (fa_mutex_lock_async(frame->channel, &frame->wait, frame, "lock") == AWAIT_SUSPEND) {
			return;
		}
		// fallthrough
	case 1:
		note_number(frame->id);
		frame->step = 2;
		fa_task_yield(frame);
		return;
	default:
		fa_mutex_unlock(frame->channel);
		finish(frame);
	}
}

static void mutex_root(Frame *frame) {
	if (frame->step == 0) {
		frame->step = 1;
		for (int index = 0; index < 4; index += 1) {
			Frame *child = new_frame(locker, index + 1);
			child->channel = frame->channel;
			fa_task_spawn(child, 1);
		}
		fa_task_yield(frame);
		return;
	}
	finish(frame);
}

static void test_mutex(void) {
	trace_length = 0;
	void *mutex = fa_mutex_new(8, 0);
	Frame *root = new_frame(mutex_root, 0);
	root->channel = mutex;
	root->resume(root);
	fa_run_main(root, 0);
	CHECK(__builtin_strcmp(trace, "1234") == 0);
	fa_shared_release(mutex);
}

// ── join nodes

static void joined(Frame *frame) {
	if (frame->step == 0) {
		frame->step = 100;
		fa_sleep_async(&frame->wait, frame, frame->rounds);
		return;
	}
	finish(frame);
}

static void join_root(Frame *frame) {
	static const int delays[] = {20, 5, 10};
	if (frame->step == 0) {
		fa_join_init(&frame->join);
		frame->frames = fa_alloc(sizeof(Frame *) * 3, 8);
		for (int index = 0; index < 3; index += 1) {
			Frame *child = new_frame(joined, index + 1);
			child->rounds = delays[index];
			fa_task_spawn(child, 0);
			frame->frames[index] = child;
			fa_join_add(&frame->join, child);
		}
		frame->step = 1;
	}
	for (;;) {
		void *ended = fa_join_next(&frame->join, frame);
		if (ended == 0) {
			return;
		}
		if (ended == (void *)1) {
			break;
		}
		note_number(((Frame *)ended)->id);
	}
	for (int index = 0; index < 3; index += 1) {
		fa_task_drop(frame->frames[index]);
	}
	fa_free(frame->frames, sizeof(Frame *) * 3, 8);
	finish(frame);
}

static void test_join(void) {
	trace_length = 0;
	Frame *root = new_frame(join_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
	CHECK(__builtin_strcmp(trace, "231") == 0);
}

// ── the blocking pool

typedef struct {
	FaJob job;
	int64_t input;
	int64_t output;
} SquareJob;

static void square_run(FaJob *job) {
	SquareJob *square = (SquareJob *)job;
	fa_nanosleep(1000000);
	square->output = square->input * square->input;
}

static void square_release(FaJob *job) {
	fa_free(job, sizeof(SquareJob), 8);
}

static SquareJob *new_square(int64_t input) {
	SquareJob *square = fa_alloc(sizeof(SquareJob), 8);
	__builtin_memset(square, 0, sizeof(SquareJob));
	square->job.run = square_run;
	square->job.release = square_release;
	square->input = input;
	return square;
}

static void pool_task(Frame *frame) {
	if (frame->step == 0) {
		SquareJob *square = new_square(frame->id);
		frame->child = square;
		frame->step = 400;
		if (fa_blocking_submit(&square->job, &frame->wait, frame) == AWAIT_SUSPEND) {
			return;
		}
	}
	SquareJob *square = frame->child;
	frame->value = square->output;
	square_release(&square->job);
	frame->child = 0;
	note_number(frame->value);
	finish(frame);
}

static void destroy_pool_task(Frame *frame) {
	if (frame->resume != 0 && frame->child != 0) {
		fa_blocking_cancel(frame->child);
	}
	free_frame(frame);
}

static void pool_root(Frame *frame) {
	if (frame->step == 0) {
		frame->frames = fa_alloc(sizeof(Frame *) * 4, 8);
		for (int index = 0; index < 4; index += 1) {
			Frame *child = new_frame(pool_task, index + 2);
			child->destroy = destroy_pool_task;
			child->resume(child);
			frame->frames[index] = child;
		}
		// one cancelled while its job runs or waits
		fa_task_drop(frame->frames[3]);
		frame->frames[3] = 0;
		fa_join_init(&frame->join);
		for (int index = 0; index < 3; index += 1) {
			fa_join_add(&frame->join, frame->frames[index]);
		}
		frame->step = 1;
	}
	for (;;) {
		void *ended = fa_join_next(&frame->join, frame);
		if (ended == 0) {
			return;
		}
		if (ended == (void *)1) {
			break;
		}
	}
	for (int index = 0; index < 3; index += 1) {
		fa_task_drop(frame->frames[index]);
	}
	fa_free(frame->frames, sizeof(Frame *) * 4, 8);
	finish(frame);
}

static void test_pool(void) {
	trace_length = 0;
	Frame *root = new_frame(pool_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
	// the squares of 2, 3 and 4 (4, 9, 16), in any order
	CHECK(trace_length == 4);
}

// ── file descriptors: a task waits until a pipe is readable

int pipe2(int fds[2], int flags);

static int pipe_fds[2];

static void *write_later(void *unused) {
	(void)unused;
	fa_nanosleep(3000000);
	write(pipe_fds[1], "fa", 2);
	return 0;
}

static void reader(Frame *frame) {
	FaIo *io = frame->child;
	for (;;) {
		char bytes[8];
		long count = read(io->fd, bytes, sizeof bytes);
		if (count > 0) {
			frame->value += count;
			note("read");
			fa_io_close(io);
			frame->child = 0;
			finish(frame);
			return;
		}
		frame->step = 500;
		if (fa_io_wait(io, 0, &frame->wait, frame) == AWAIT_SUSPEND) {
			note("wait ");
			return;
		}
	}
}

static void test_io(void) {
	trace_length = 0;
	CHECK(pipe2(pipe_fds, 04000 /* O_NONBLOCK */) == 0);
	Frame *root = new_frame(reader, 0);
	root->child = fa_io_register(pipe_fds[0]);
	CHECK(root->child != 0);
	unsigned long thread;
	pthread_create(&thread, 0, write_later, 0);
	root->resume(root);
	fa_run_main(root, 0);
	pthread_join(thread, 0);
	close(pipe_fds[0]);
	close(pipe_fds[1]);
	CHECK(__builtin_strcmp(trace, "wait read") == 0);
}

static void check_unsigned_decimal(uint64_t value) {
	char expected[32];
	int length = snprintf(expected, sizeof expected, "%llu", (unsigned long long)value);
	FaString actual;
	fa_string_u64(value, &actual);
	CHECK(!string_owned(actual.capacity));
	CHECK(string_length(&actual) == (size_t)length);
	CHECK(fa_string_equal(string_bytes(&actual), (size_t)length, (const uint8_t *)expected, (size_t)length));
	uint8_t digits[24];
	size_t offset = fa_decimal_u64(value, digits);
	CHECK(offset <= sizeof digits && sizeof digits - offset == (size_t)length);
	CHECK(fa_string_equal(digits + offset, (size_t)length, (const uint8_t *)expected, (size_t)length));
}

static void check_signed_decimal(int64_t value) {
	char expected[32];
	int length = snprintf(expected, sizeof expected, "%lld", (long long)value);
	FaString actual;
	fa_string_i64(value, &actual);
	CHECK(!string_owned(actual.capacity));
	CHECK(string_length(&actual) == (size_t)length);
	CHECK(fa_string_equal(string_bytes(&actual), (size_t)length, (const uint8_t *)expected, (size_t)length));
	uint8_t digits[24];
	size_t offset = fa_decimal_i64(value, digits);
	CHECK(offset <= sizeof digits && sizeof digits - offset == (size_t)length);
	CHECK(fa_string_equal(digits + offset, (size_t)length, (const uint8_t *)expected, (size_t)length));
}

static void test_integer_strings(void) {
	long before = allocated;
	const int64_t signed_edges[] = {INT64_MIN, INT64_MIN + 1, -100, -99, -10, -1, 0, 1, 9, 10, 99, 100, INT64_MAX - 1, INT64_MAX};
	for (size_t i = 0; i < sizeof signed_edges / sizeof signed_edges[0]; i += 1) { check_signed_decimal(signed_edges[i]); }
	check_unsigned_decimal(UINT64_MAX);
	for (uint64_t value = (uint64_t)UINT32_MAX - 1; value <= (uint64_t)UINT32_MAX + 1; value += 1) {
		check_unsigned_decimal(value);
		check_signed_decimal((int64_t)value);
		check_signed_decimal(-(int64_t)value);
	}
	for (uint64_t power = 1;;) {
		check_unsigned_decimal(power - 1);
		check_unsigned_decimal(power);
		check_unsigned_decimal(power + 1);
		if (power > UINT64_MAX / 10) { break; }
		power *= 10;
	}
	uint64_t random = 0x123456789abcdef0ull;
	for (size_t i = 0; i < 10000; i += 1) {
		random ^= random << 13;
		random ^= random >> 7;
		random ^= random << 17;
		check_unsigned_decimal(random);
		int64_t signed_value = (int64_t)(random >> 1);
		check_signed_decimal(random & 1 ? -signed_value : signed_value);
	}
	CHECK(allocated == before);
}

static void test_string_padding(void) {
	long before = allocated;
	const uint8_t text[] = "sample";
	for (int64_t width = -2; width <= 80; width += 1) {
		for (int start = 0; start <= 1; start += 1) {
			uint8_t expected[80];
			size_t length = width > 6 ? (size_t)width : 6;
			size_t padding = length - 6;
			for (size_t i = 0; i < length; i += 1) { expected[i] = '_'; }
			__builtin_memcpy(expected + (start ? padding : 0), text, 6);
			FaString actual;
			fa_string_pad(text, 6, SIMPLE, width, (const uint8_t *)"_", 1, start, &actual);
			CHECK(string_length(&actual) == length);
			CHECK(fa_string_equal(string_bytes(&actual), length, expected, length));
			if (string_owned(actual.capacity)) { fa_string_free(actual.bytes, actual.capacity); }
		}
	}
	FaString actual;
	fa_string_pad(text, 6, SIMPLE, INT64_MIN, (const uint8_t *)"_", 1, 1, &actual);
	CHECK(fa_string_equal(string_bytes(&actual), string_length(&actual), text, 6));
	// The general path counts code units and cycles a multi-character filler.
	fa_string_pad((const uint8_t *)"\xc3\xa9", 2, 0, 4, (const uint8_t *)"ab", 2, 1, &actual);
	CHECK(fa_string_equal(string_bytes(&actual), string_length(&actual), (const uint8_t *)"aba\xc3\xa9", 5));
	if (string_owned(actual.capacity)) { fa_string_free(actual.bytes, actual.capacity); }
	fa_release_builder();
	CHECK(allocated == before);
}

static void test_inline_strings(void) {
	uint8_t input[128];
	for (size_t i = 0; i < sizeof input; i += 1) { input[i] = (uint8_t)('a' + i % 26); }
	for (size_t length = 0; length <= INLINE_STRING_BYTES; length += 1) {
		long before = allocated;
		FaString original = owned_copy(input, length);
		FaString copy = original;
		CHECK(allocated == before);
		CHECK(!string_owned(original.capacity));
		CHECK(string_length(&copy) == length);
		CHECK(fa_string_equal(string_bytes(&copy), length, input, length));
		CHECK(fa_hash_string_bits((uintptr_t)copy.bytes, copy.length, copy.capacity) == fa_hash_bytes(input, length));
		if (copy.capacity & STRING_INLINE) {
			for (size_t i = length; i < INLINE_STRING_BYTES; i += 1) { CHECK(((const uint8_t *)&copy)[i] == 0); }
		}
		fa_string_append(&copy, input, 1, SIMPLE);
		CHECK(string_length(&original) == length);
		CHECK(fa_string_equal(string_bytes(&original), length, input, length));
		if (length < INLINE_STRING_BYTES) { CHECK(allocated == before); }
		if (string_owned(copy.capacity)) { fa_string_free(copy.bytes, copy.capacity); }
		CHECK(allocated == before);
	}
	FaString text = owned_copy(input, 0);
	long before = allocated;
	for (size_t i = 0; i < INLINE_STRING_BYTES; i += 1) {
		fa_string_append(&text, input + i, 1, SIMPLE);
		CHECK(allocated == before);
		CHECK(string_length(&text) == i + 1);
		CHECK(fa_string_equal(string_bytes(&text), i + 1, input, i + 1));
	}
	fa_string_append(&text, input + INLINE_STRING_BYTES, 1, SIMPLE);
	CHECK(allocated == before + 1);
	CHECK(fa_string_equal(string_bytes(&text), INLINE_STRING_BYTES + 1, input, INLINE_STRING_BYTES + 1));
	fa_string_free(text.bytes, text.capacity);
	CHECK(allocated == before);
	for (size_t length = 1; length < sizeof input; length += 1) {
		FaString text;
		fa_string_own(input, length, KIND_MASK, &text);
		CHECK(fa_hash_string_bits((uintptr_t)text.bytes, text.length, text.capacity) == fa_hash_bytes(input, length));
		if (length <= INLINE_STRING_BYTES) { CHECK(!string_owned(text.capacity)); }
		if (string_owned(text.capacity)) { fa_string_free(text.bytes, text.capacity); }
	}
	CHECK(allocated == before);
}

static void test_nonempty_splits(void) {
	long before = allocated;
	const char *texts[] = {"", " ", "       ", "a b c", "  a  b c  ", "x::y::::z::",
	                       "e\xcc\x81 e\xcc\x81\r\n", "a long fragment beyond twenty three bytes :: second::"};
	const char *separators[] = {" ", "::", "e", "\r", ""};
	for (size_t text = 0; text < sizeof texts / sizeof texts[0]; text += 1) {
		for (size_t separator = 0; separator < sizeof separators / sizeof separators[0]; separator += 1) {
			const uint8_t *bytes = (const uint8_t *)texts[text];
			const uint8_t *sep = (const uint8_t *)separators[separator];
			size_t length = __builtin_strlen(texts[text]);
			size_t sep_length = __builtin_strlen(separators[separator]);
			for (int views = 0; views <= 1; views += 1) {
				FaArrayHeader *plain = split_text(bytes, length, 0, sep, sep_length, views, 0);
				FaArrayHeader *fused = views
					? fa_string_split_nonempty_views(bytes, length, 0, sep, sep_length)
					: fa_string_split_nonempty(bytes, length, 0, sep, sep_length);
				fa_array_materialize(plain);
				fa_array_materialize(fused);
				CHECK(fused->capacity == plain->capacity);
				FaString *source = (FaString *)plain->items;
				FaString *result = (FaString *)fused->items;
				int64_t kept = 0;
				for (int64_t index = 0; index < plain->length; index += 1) {
					size_t bytes = string_length(&source[index]);
					if (bytes) {
						CHECK(kept < fused->length);
						CHECK(fa_string_equal(string_bytes(&source[index]), bytes,
						                      string_bytes(&result[kept]), string_length(&result[kept])));
						kept += 1;
					}
					if (string_owned(source[index].capacity)) {
						fa_string_free(source[index].bytes, source[index].capacity);
					}
				}
				CHECK(kept == fused->length);
				for (int64_t index = 0; index < fused->length; index += 1) {
					if (string_owned(result[index].capacity)) {
						fa_string_free(result[index].bytes, result[index].capacity);
					}
				}
				fa_array_free(plain, sizeof(FaString));
				fa_array_free(fused, sizeof(FaString));
			}
		}
	}
	CHECK(allocated == before);
}

static void test_region_slice_views(void) {
	long before = allocated;
	uint8_t input[128];
	for (size_t index = 0; index < sizeof input; index += 1) { input[index] = (uint8_t)('a' + index % 26); }
	FaString source = owned_copy_with(input, sizeof input, SIMPLE);
	for (size_t length = 0; length <= sizeof input - 3; length += 1) {
		FaString view;
		CHECK(fa_string_slice_region(source.bytes, source.length, source.capacity, 3, (int64_t)length + 3, 3, &view) == 0);
		CHECK(string_length(&view) == length && (view.capacity & SIMPLE));
		CHECK(fa_string_equal(string_bytes(&view), length, input + 3, length));
		if (length <= INLINE_STRING_BYTES) {
			CHECK(view.bytes == source.bytes + 3 && (view.capacity & KIND_MASK) == KIND_MASK);
		} else {
			CHECK(string_owned(view.capacity));
			fa_string_free(view.bytes, view.capacity);
		}
		FaString copied;
		CHECK(fa_string_slice(source.bytes, source.length, source.capacity, 3, (int64_t)length + 3, 3, &copied) == 0);
		CHECK(copied.capacity & SIMPLE);
		CHECK(fa_string_equal(string_bytes(&copied), length, input + 3, length));
		if (string_owned(copied.capacity)) { fa_string_free(copied.bytes, copied.capacity); }
	}
	fa_string_free(source.bytes, source.capacity);
	FaString unicode;
	const uint8_t clusters[] = "e\xcc\x81" "e\xcc\x81" "e\xcc\x81";
	CHECK(fa_string_slice_region(clusters, 9, 0, 1, 3, 3, &unicode) == 0);
	CHECK(unicode.bytes == clusters + 1 && unicode.length == 3 && !(unicode.capacity & SIMPLE));
	CHECK(fa_string_length(unicode.bytes, unicode.length, unicode.capacity) == 2);
	CHECK(fa_string_slice_region((const uint8_t *)"\r\nx", 3, 0, 0, 1, 3, &unicode) == 0);
	CHECK(unicode.length == 1 && !(unicode.capacity & SIMPLE));
	CHECK(fa_string_length(unicode.bytes, unicode.length, unicode.capacity) == 1);
	CHECK(allocated == before);
}

static void test_utf16_strings(void) {
	fa_release_builder();
	long before = allocated;
	long bytes_before = allocated_bytes;
	const uint8_t text[] = "a\xc3\xa9\xe6\x97\xa5\xf0\x9f\x98\x80z";
	const uint16_t units[] = {97, 233, 0x65E5, 0xD83D, 0xDE00, 122};
	CHECK(fa_string_length(text, sizeof(text) - 1, 0) == 6);
	CHECK(simple_flag((const uint8_t *)"\r\nx", 3) == SIMPLE);
	const uint8_t ascii_units[] = "abc";
	for (int64_t i = 0; i < 3; i += 1) {
		CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, i) == ascii_units[i]);
		CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, i - 3) == ascii_units[i]);
	}
	CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, 3) == -1);
	CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, -4) == -1);
	CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, INT64_MIN) == -1);
	CHECK(string_code_unit_slow(ascii_units, 3, SIMPLE, INT64_MAX) == -1);
	CHECK(string_code_unit_slow(ascii_units, 0, SIMPLE, 0) == -1);
	CHECK(string_code_unit_slow(ascii_units, 0, SIMPLE, -1) == -1);
	for (int64_t i = 0; i < 6; i += 1) {
		CHECK(string_code_unit_slow(text, sizeof(text) - 1, 0, i) == units[i]);
		CHECK(string_code_unit_slow(text, sizeof(text) - 1, 0, i - 6) == units[i]);
		FaString unit;
		CHECK(fa_string_get(text, sizeof(text) - 1, 0, i, &unit));
		CHECK(!string_owned(unit.capacity));
		CHECK(fa_string_length(string_bytes(&unit), string_length(&unit), unit.capacity) == 1);
		CHECK(fa_string_is_well_formed(string_bytes(&unit), string_length(&unit), unit.capacity) == (i != 3 && i != 4));
	}
	for (size_t from = 0; from <= 6; from += 1) {
		for (size_t to = from; to <= 6; to += 1) {
			FaString part;
			CHECK(!fa_string_slice(text, sizeof(text) - 1, 0, (int64_t)from, (int64_t)to, 3, &part));
			CHECK(fa_string_length(string_bytes(&part), string_length(&part), part.capacity) == to - from);
			for (size_t i = from; i < to; i += 1) {
				CHECK(string_code_unit_slow(string_bytes(&part), string_length(&part), part.capacity, (int64_t)(i - from)) == units[i]);
			}
			if (string_owned(part.capacity)) { fa_string_free(part.bytes, part.capacity); }
		}
	}
	FaString high, low, joined;
	fa_string_repeat((const uint8_t *)"", 0, INT64_MAX, &joined);
	CHECK(string_length(&joined) == 0);
	CHECK(fa_string_get(text, sizeof(text) - 1, 0, 3, &high));
	CHECK(fa_string_get(text, sizeof(text) - 1, 0, 4, &low));
	fa_concat(string_bytes(&high), 3, high.capacity, string_bytes(&low), 3, low.capacity, &joined);
	CHECK(string_length(&joined) == 4);
	CHECK(fa_string_equal(string_bytes(&joined), 4, text + 6, 4));
	fa_string_append(&high, string_bytes(&low), 3, low.capacity);
	CHECK(string_length(&high) == 4 && !(high.capacity & STRING_SURROGATES));
	uint8_t encoded[3];
	fa_string_write_utf8(encoded, string_bytes(&low), 3, low.capacity);
	CHECK(__builtin_memcmp(encoded, "\xef\xbf\xbd", 3) == 0);
	CHECK(fa_string_compare((const uint8_t *)"\xf0\x90\x80\x80", 4, (const uint8_t *)"\xee\x80\x80", 3) < 0);
	CHECK(fa_string_compare(string_bytes(&low), 3, (const uint8_t *)"\xee\x80\x80", 3) < 0);
	FaString counted;
	CHECK(!fa_string_slice_view(text, sizeof(text) - 1, 0, 1, 3, 3, &counted));
	CHECK(counted.bytes == text + 1 && counted.length == 5);
	CHECK(string_counted_view(counted.capacity) && (counted.capacity & KIND_MASK) == 2);
	CHECK(fa_string_length(counted.bytes, counted.length, counted.capacity) == 2);
	CHECK(!string_owned(counted.capacity) && !string_static(counted.capacity));
	FaString snapshot;
	CHECK(fa_string_reference(0, &snapshot) == 0);
	CHECK(fa_string_reference(&counted, &snapshot) == &snapshot);
	counted.length = 0;
	CHECK(snapshot.bytes == text + 1 && snapshot.length == 5);
	FaString whole = {(const uint8_t *)"owner", 5, SIMPLE};
	CHECK(fa_string_reference(&whole, &snapshot) == &whole);
	// Virtual cut spans keep only the bytes needed to recover each half:
	// bytes 2..4 for a low head, bytes 0..3 for a high tail. No read may
	// leave those physical spans, and owning copies write canonical WTF-8.
	for (uint32_t code = 0x10000; code <= 0x10FFFF; code += 1021) {
		uint8_t scalar[4], copied[4] = {0, 0, 0, 0x77}, expected_half[3];
		CHECK(encode_point(scalar, code) == 4);
		uint16_t high_unit = (uint16_t)(0xD800 + ((code - 0x10000) >> 10));
		uint16_t low_unit = (uint16_t)(0xDC00 + ((code - 0x10000) & 1023));
		size_t position = 0;
		CHECK(decode(scalar, 3, &position) == high_unit && position == 3);
		position = 0;
		CHECK(decode(scalar + 2, 2, &position) == low_unit && position == 2);
		CHECK(unit_count(scalar, 3) == 1 && unit_count(scalar + 2, 2) == 1);
		CHECK(fa_string_encoded_length(scalar, 3, STRING_COUNTED_VIEW | 1) == 3);
		CHECK(fa_string_encoded_length(scalar + 2, 2, STRING_COUNTED_VIEW | 1) == 3);
		fa_string_copy_bytes(copied, scalar, 3, STRING_COUNTED_VIEW | 1);
		CHECK(encode_point(expected_half, high_unit) == 3);
		CHECK(__builtin_memcmp(copied, expected_half, 3) == 0);
		CHECK(copied[3] == 0x77);
		fa_string_copy_bytes(copied, scalar + 2, 2, STRING_COUNTED_VIEW | 1);
		CHECK(encode_point(expected_half, low_unit) == 3);
		CHECK(__builtin_memcmp(copied, expected_half, 3) == 0);
	}
	const uint8_t both_edges[] = {0x98, 0x80, 'x', 0xF0, 0x9F, 0x98};
	uint8_t canonical_edges[7];
	CHECK(unit_count(both_edges, sizeof both_edges) == 3);
	fa_string_copy_bytes(canonical_edges, both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3);
	CHECK(__builtin_memcmp(canonical_edges, "\xED\xB8\x80x\xED\xA0\xBD", 7) == 0);
	FaString owned_edges;
	fa_string_own(both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3, &owned_edges);
	CHECK(string_length(&owned_edges) == 7);
	CHECK(__builtin_memcmp(string_bytes(&owned_edges), canonical_edges, 7) == 0);
	CHECK(fa_string_equal(both_edges, sizeof both_edges, canonical_edges, 7));
	CHECK(fa_string_compare(both_edges, sizeof both_edges, canonical_edges, 7) == 0);
	CHECK(fa_string_compare(both_edges, sizeof both_edges, (const uint8_t *)"z", 1) > 0);
	CHECK(fa_hash_string_bits((size_t)(uintptr_t)both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3) == fa_hash_bytes(canonical_edges, 7));
	FaString edge_view = {both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3};
	FaString result = {0};
	fa_string_escape(&edge_view, &result);
	CHECK(!string_view(result.capacity) && fa_string_equal(string_bytes(&result), string_length(&result), canonical_edges, 7));
	uint8_t view_frame[FRAME_STRING_BYTES];
	fa_string_assign_local(&edge_view, view_frame, &result, 1, 1);
	CHECK(string_frame(result.capacity) && result.length == 7);
	CHECK(fa_string_equal(result.bytes, result.length, canonical_edges, 7));
	fa_string_from(both_edges, sizeof both_edges, edge_view.capacity, 128, &result);
	CHECK(result.length == 7 && string_owned(result.capacity));
	fa_string_free(result.bytes, result.capacity);
	fa_b_reset();
	fa_b_string(both_edges, sizeof both_edges, edge_view.capacity);
	fa_b_take(&result);
	CHECK(fa_string_equal(string_bytes(&result), string_length(&result), canonical_edges, 7));
	fa_b_reset();
	fa_b_quoted(both_edges, sizeof both_edges);
	fa_b_take(&result);
	CHECK(string_length(&result) == 9 && string_code_unit_slow(string_bytes(&result), 9, result.capacity, 1) == 0xDE00);
	fa_b_reset();
	fa_b_json_string(both_edges, sizeof both_edges);
	fa_b_take(&result);
	CHECK(string_length(&result) == 15);
	CHECK(__builtin_memcmp(string_bytes(&result), "\"\\ude00x\\ud83d\"", 15) == 0);
	CHECK(fa_string_is_well_formed(string_bytes(&result), string_length(&result), result.capacity));
	fa_concat(both_edges, sizeof both_edges, edge_view.capacity, both_edges, sizeof both_edges, edge_view.capacity, &result);
	CHECK(fa_string_length(string_bytes(&result), string_length(&result), result.capacity) == 6);
	fa_string_append(&result, both_edges, sizeof both_edges, edge_view.capacity);
	CHECK(fa_string_length(string_bytes(&result), string_length(&result), result.capacity) == 9);
	// A self-borrowed virtual suffix survives both in-place appends and
	// allocation, even when normalization changes its original end bytes.
	for (size_t reserved = 0; reserved <= 256; reserved += 256) {
		uint8_t original[67] = {0xF0, 0x9F, 0x98, 0x80};
		__builtin_memset(original + 4, 'x', 60);
		copy_bytes(original + 64, (const uint8_t *)"\xED\xA0\xBD", 3);
		FaString self;
		fa_string_from(original, sizeof original, STRING_SURROGATES, reserved, &self);
		const uint8_t *suffix = self.bytes + 2;
		FaString expected;
		fa_concat(self.bytes, self.length, self.capacity, suffix, self.length - 2, STRING_COUNTED_VIEW | 62, &expected);
		fa_string_append(&self, suffix, self.length - 2, STRING_COUNTED_VIEW | 62);
		CHECK(fa_string_equal(self.bytes, self.length, string_bytes(&expected), string_length(&expected)));
		CHECK(fa_string_length(self.bytes, self.length, self.capacity) == 125);
		fa_string_free(self.bytes, self.capacity);
		fa_string_free(expected.bytes, expected.capacity);
	}
	for (size_t middle = 0; middle <= 80; middle += 1) {
		uint8_t physical[85] = {0x98, 0x80};
		__builtin_memset(physical + 2, 'x', middle);
		copy_bytes(physical + middle + 2, (const uint8_t *)"\xF0\x9F\x98", 3);
		size_t physical_length = middle + 5;
		size_t tag = STRING_COUNTED_VIEW | (middle + 2);
		uint8_t canonical[86];
		fa_string_copy_bytes(canonical, physical, physical_length, tag);
		uint64_t hash = fa_hash_bytes(canonical, physical_length + 1);
		CHECK(fa_hash_string_bits((size_t)(uintptr_t)physical, physical_length, tag) == hash);
		// The excluded half's bits have no effect on the String's value.
		physical[0] = 0xB8;
		physical[physical_length - 1] = 0x9F;
		CHECK(fa_string_equal_flags(physical, physical_length, tag, canonical, physical_length + 1, STRING_SURROGATES));
		CHECK(fa_hash_string_bits((size_t)(uintptr_t)physical, physical_length, tag) == hash);
		uint8_t inplace[86];
		copy_bytes(inplace, physical, physical_length);
		fa_string_copy_bytes(inplace, inplace, physical_length, tag);
		CHECK(__builtin_memcmp(inplace, canonical, physical_length + 1) == 0);
	}
	CHECK(!fa_string_is_well_formed(both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3));
	fa_string_write_utf8(canonical_edges, both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3);
	CHECK(__builtin_memcmp(canonical_edges, "\xEF\xBF\xBDx\xEF\xBF\xBD", 7) == 0);
	fa_virtual_string_slices = 0;
	CHECK(fa_string_encoded_length(both_edges, sizeof both_edges, STRING_COUNTED_VIEW | 3) == sizeof both_edges);
	fa_virtual_string_slices = 1;
	// Explicit cuts retain the owner's bytes even when both ends cut pairs
	// and the logical result is too large for inline storage.
	uint8_t paired[320];
	for (size_t i = 0; i < sizeof paired; i += 4) {
		copy_bytes(paired + i, (const uint8_t *)"\xF0\x9F\x98\x80", 4);
	}
	long view_blocks = allocated;
	long view_bytes = allocated_bytes;
	FaString cut, nested_cut;
	CHECK(!fa_string_slice_view(paired, sizeof paired, STRING_LITERAL | 160, 1, 159, 3, &cut));
	CHECK(cut.bytes == paired + 2 && cut.length == 317);
	CHECK(fa_string_length(cut.bytes, cut.length, cut.capacity) == 158);
	CHECK(fa_string_encoded_length(cut.bytes, cut.length, cut.capacity) == 318);
	CHECK(!fa_string_slice_view(cut.bytes, cut.length, cut.capacity, 0, 1, 3, &nested_cut));
	CHECK(nested_cut.bytes == paired + 2 && nested_cut.length == 2);
	CHECK(string_code_unit_slow(nested_cut.bytes, nested_cut.length, nested_cut.capacity, 0) == 0xDE00);
	CHECK(allocated == view_blocks && allocated_bytes == view_bytes);
	// ASCII compact storage has no extra field. Unicode counts are bounded
	// unsigned LEB128 values and preserve the full representation range.
	uint8_t metadata[16] = "ascii";
	CHECK(fa_string_view_count_size(5, SIMPLE) == 0);
	CHECK(fa_string_write_view_count(metadata, 5, SIMPLE) == metadata);
	CHECK(metadata[0] == 'a');
	CHECK(fa_string_read_packed_view(metadata, 5, SIMPLE, &counted) == metadata + 5);
	CHECK(counted.bytes == metadata && counted.length == 5 && counted.capacity == (SIMPLE | KIND_MASK));
	const size_t counts[] = {0, 1, 127, 128, 16383, 16384, KIND_MASK - 1};
	const size_t sizes[] = {1, 1, 1, 2, 2, 3, (sizeof(size_t) * 8 - 4 + 6) / 7};
	for (size_t i = 0; i < sizeof counts / sizeof counts[0]; i += 1) {
		uint8_t *end = fa_string_write_view_count(metadata, counts[i], 0);
		CHECK((size_t)(end - metadata) == sizes[i]);
		CHECK(fa_string_view_count_size(counts[i], 0) == sizes[i]);
		CHECK(fa_string_read_packed_view(metadata, 0, 0, &counted) == end);
		CHECK(string_counted_view(counted.capacity) && (counted.capacity & KIND_MASK) == counts[i]);
	}
	// Arena text preserves its four-byte compact or sixteen-byte ordinary
	// ASCII prefix. Only Unicode carries the additional two-byte count.
	uint8_t arena[32] = {0};
	for (int32_t packed = 0; packed <= 1; packed += 1) {
		uint8_t *payload = arena + (packed ? 4 : 16);
		CHECK(fa_string_write_arena_count(payload, 5, SIMPLE) == payload);
		fa_string_read_arena_view(arena, 5, SIMPLE, packed, 0, &counted);
		CHECK(counted.bytes == payload && counted.length == 5);
		CHECK(counted.capacity == (SIMPLE | KIND_MASK));
		CHECK(fa_string_write_arena_count(payload, 3, 0) == payload + 2);
		fa_string_read_arena_view(arena, 7, 0, packed, 0, &counted);
		CHECK(counted.bytes == payload + 2 && counted.length == 7);
		CHECK(string_counted_view(counted.capacity) && (counted.capacity & KIND_MASK) == 3);
	}
	// Materializing a conservative view makes the inline surrogate flag exact.
	FaString view = {(const uint8_t *)"\xc3\xa9", 2, STRING_SURROGATES | KIND_MASK};
	FaString materialized = {0};
	uint8_t frame[FRAME_STRING_BYTES];
	fa_string_assign_local(&view, frame, &materialized, 0, 0);
	CHECK((materialized.capacity & STRING_INLINE) && !(materialized.capacity & STRING_SURROGATES));
	FaString ascii = owned_copy_with((const uint8_t *)"abcdefghijklmnopqrstuvwxyz012345", 32, SIMPLE);
	uint8_t *replacement = fa_string_replace_buffer(96, SIMPLE, &ascii);
	fill_bytes(replacement, 'x', 96);
	CHECK(string_length(&ascii) == 96 && (ascii.capacity & SIMPLE));
	fa_string_free(ascii.bytes, ascii.capacity);
	// A pair crosses every second breadcrumb; query both halves in reverse order.
	uint8_t long_text[640];
	size_t bytes = 0;
	uint16_t expected[260];
	size_t count = 0;
	for (size_t i = 0; i < 4; i += 1) {
		for (size_t j = 0; j < 63; j += 1) { long_text[bytes++] = 'x'; expected[count++] = 'x'; }
		copy_bytes(long_text + bytes, text + 6, 4); bytes += 4;
		expected[count++] = 0xD83D; expected[count++] = 0xDE00;
	}
	FaString heap;
	fa_string_from(long_text, bytes, 0, 4096, &heap);
	CHECK(string_owned(heap.capacity));
	CHECK(fa_string_length(heap.bytes, bytes, heap.capacity) == count);
	CHECK(string_header(heap.bytes)->breadcrumbs == 0);
	CHECK(string_code_unit_slow(heap.bytes, bytes, heap.capacity, 0) == 'x');
	CHECK(string_header(heap.bytes)->breadcrumbs == 0);
	for (size_t i = count; i > 0; i -= 1) { CHECK(string_code_unit_slow(heap.bytes, bytes, heap.capacity, (int64_t)i - 1) == expected[i - 1]); }
	CHECK(string_header(heap.bytes)->breadcrumbs != 0);
	size_t *old_crumbs = string_header(heap.bytes)->breadcrumbs;
	size_t base_bytes = string_header(heap.bytes)->bytes;
	size_t base_units = string_header(heap.bytes)->units;
	fa_string_append(&heap, (const uint8_t *)"!", 1, SIMPLE);
	CHECK(fa_string_length(heap.bytes, heap.length, heap.capacity) == count + 1);
	CHECK(string_header(heap.bytes)->breadcrumbs == old_crumbs);
	CHECK(string_header(heap.bytes)->bytes == base_bytes && string_header(heap.bytes)->units == base_units);
	uint8_t ascii_tail[128];
	__builtin_memset(ascii_tail, '!', sizeof ascii_tail);
	fa_string_append(&heap, ascii_tail, sizeof ascii_tail, SIMPLE);
	CHECK(string_code_unit_slow(heap.bytes, heap.length, heap.capacity, (int64_t)count + 100) == '!');
	CHECK(string_header(heap.bytes)->breadcrumbs[0] == (count + 129) / 64 + 1);
	CHECK(string_header(heap.bytes)->bytes == base_bytes && string_header(heap.bytes)->units == base_units);
	fa_string_free(heap.bytes, heap.capacity);
	// Equal byte lengths can have different unit counts. A replacement
	// publishes even when its pointer, byte length and capacity are unchanged.
	uint8_t euros[48], accents[48];
	for (size_t i = 0; i < sizeof euros; i += 3) { copy_bytes(euros + i, (const uint8_t *)"€", 3); }
	for (size_t i = 0; i < sizeof accents; i += 2) { copy_bytes(accents + i, (const uint8_t *)"é", 2); }
	FaString rewritten;
	fa_string_from(euros, sizeof euros, 0, 128, &rewritten);
	CHECK(fa_string_length(rewritten.bytes, 48, rewritten.capacity) == 16);
	uint8_t *same_block = fa_string_replace_buffer(48, 0, &rewritten);
	copy_bytes(same_block, accents, sizeof accents);
	fa_string_finish(&rewritten);
	CHECK(rewritten.bytes == same_block && fa_string_length(rewritten.bytes, 48, rewritten.capacity) == 24);
	same_block = fa_string_replace_buffer(48, 0, &rewritten);
	__builtin_memset(same_block, 'x', 48);
	fa_string_finish(&rewritten);
	CHECK(rewritten.capacity & SIMPLE);
	fa_string_append(&rewritten, (const uint8_t *)"é", 2, 0);
	CHECK(fa_string_length(rewritten.bytes, rewritten.length, rewritten.capacity) == 49);
	fa_string_free(rewritten.bytes, rewritten.capacity);
	// Non-ASCII appends normalize only their join, even as the block grows.
	FaString growing = owned_copy((const uint8_t *)"é", 2);
	for (size_t i = 0; i < 10000; i += 1) {
		fa_string_append(&growing, (const uint8_t *)"é", 2, 0);
	}
	CHECK(fa_string_length(growing.bytes, growing.length, growing.capacity) == 10001);
	fa_string_append(&growing, string_bytes(&low), 3, low.capacity);
	CHECK(!fa_string_is_well_formed(growing.bytes, growing.length, growing.capacity));
	FaString last_high;
	CHECK(fa_string_get(text, sizeof(text) - 1, 0, 3, &last_high));
	fa_string_append(&growing, string_bytes(&last_high), 3, last_high.capacity);
	fa_string_append(&growing, string_bytes(&low), 3, low.capacity);
	CHECK(growing.length == 20009);
	CHECK(string_code_unit_slow(growing.bytes, growing.length, growing.capacity, -2) == 0xD83D);
	CHECK(string_code_unit_slow(growing.bytes, growing.length, growing.capacity, -1) == 0xDE00);
	fa_string_free(growing.bytes, growing.capacity);
	fa_release_builder();
	CHECK(allocated == before);
	CHECK(allocated_bytes == bytes_before);
}

static void test_byte_blocks(void) {
	long before = allocated, bytes_before = allocated_bytes;
	CHECK(sizeof(FaBytesView) == sizeof(void *) * 2);
	FaArrayHeader *owner = fa_bytes_filled(160, 255);
	FaBytesView first, second;
	CHECK(!fa_bytes_slice_view(owner, 1, 159, 3, &first));
	void *view = fa_bytes_view_pointer(&first);
	CHECK(bytes_parts(view).items == owner->items + 1 && bytes_parts(view).length == 158);
	FaBytesReference snapshot;
	void **saved = fa_bytes_reference(&view, &snapshot);
	CHECK(!fa_bytes_slice_view(owner, -2, -1, 3, &second));
	view = fa_bytes_view_pointer(&second);
	CHECK(bytes_parts(*saved).length == 158 && bytes_parts(view).length == 1);
	bytes_parts(*saved).items[0] = 7;
	CHECK(owner->items[1] == 7);
	FaArrayHeader *detached = fa_bytes_copy(*saved);
	CHECK(fa_bytes_equal(detached, *saved));
	CHECK(fa_bytes_hash(detached) == fa_bytes_hash(*saved));
	detached->items[0] = 8;
	CHECK(owner->items[1] == 7 && !fa_bytes_equal(detached, *saved));
	fa_bytes_free(*saved);
	fa_bytes_free(view);
	fa_bytes_free(detached);
	fa_bytes_free(owner);
	const uint8_t raw_low[] = {0x98, 0x80};
	const uint8_t wtf8_low[] = {0xED, 0xB8, 0x80};
	void *raw = fa_bytes_from(raw_low, sizeof raw_low);
	void *encoded = fa_bytes_from(wtf8_low, sizeof wtf8_low);
	CHECK(!fa_bytes_equal(raw, encoded));
	fa_bytes_free(raw);
	fa_bytes_free(encoded);
	void *empty = fa_bytes_filled(0, 0);
	CHECK(!fa_bytes_slice_view(empty, 0, 0, 3, &first));
	CHECK(bytes_parts(fa_bytes_view_pointer(&first)).length == 0);
	fa_bytes_free(empty);
	CHECK(allocated == before && allocated_bytes == bytes_before);
}

static void test_utf8_byte_conversions(void) {
	fa_release_builder();
	long before = allocated, bytes_before = allocated_bytes;
	uint8_t raw[180];
	for (size_t i = 0; i < sizeof raw; i += 6) { copy_bytes(raw + i, (const uint8_t *)"\xC3\xA9\xF0\x9F\x98\x80", 6); }
	for (size_t count = 0; count <= sizeof raw; count += 6) {
		void *bytes = fa_bytes_from(raw, count);
		uint8_t *original = bytes_parts(bytes).items;
		FaString text;
		CHECK(!fa_string_decode_bytes(bytes, 1, 0, &text));
		CHECK(fa_string_length(string_bytes(&text), string_length(&text), text.capacity) == count / 2);
		if (count > INLINE_LIMIT) { CHECK(text.bytes == original); }
		if (count > 128) {
			CHECK(string_code_unit_slow(text.bytes, count, text.capacity, 0) == 233);
			CHECK(string_code_unit_slow(text.bytes, count, text.capacity, 1) == 0xD83D);
		}
		const uint8_t *owned = string_bytes(&text);
		int was_owned = string_owned(text.capacity);
		void *encoded = fa_string_to_utf8(&text, 1);
		if (was_owned) { CHECK(bytes_parts(encoded).items == owned); }
		CHECK(bytes_parts(encoded).length == count);
		CHECK(__builtin_memcmp(bytes_parts(encoded).items, raw, count) == 0);
		fa_bytes_free(encoded);
	}
	void *bad = fa_bytes_from((const uint8_t *)"a\xF0\x9F", 3);
	FaString output;
	const int64_t *error = fa_string_decode_bytes(bad, 1, 0, &output);
	CHECK(error && error[0] == 13 && error[1] == 1);
	fa_free((void *)error, 16, sizeof(int64_t));
	bad = fa_bytes_from((const uint8_t *)"\xED\xA0\x80", 3);
	CHECK(!fa_string_decode_bytes(bad, 1, 1, &output));
	CHECK(string_length(&output) == 9 && fa_string_length(string_bytes(&output), 9, output.capacity) == 3);
	FaString surrogate;
	unit_string(0xD83D, &surrogate);
	FaString long_surrogate;
	fa_string_from(string_bytes(&surrogate), 3, surrogate.capacity, 160, &long_surrogate);
	void *replacement = fa_string_to_utf8(&long_surrogate, 1);
	CHECK(bytes_parts(replacement).length == 3);
	CHECK(__builtin_memcmp(bytes_parts(replacement).items, "\xEF\xBF\xBD", 3) == 0);
	fa_bytes_free(replacement);
	CHECK(allocated == before && allocated_bytes == bytes_before);
}

static void test_allocator_fast_paths(void) {
	const size_t sizes[] = {0, 1, 8, 16, 24, 32, 128};
	for (size_t at = 0; at < sizeof sizes / sizeof sizes[0]; at += 1) {
		for (size_t align = 8; align <= 16; align += 8) {
			size_t size = align > 8 ? (sizes[at] == 0 ? 16 : (sizes[at] + 15) & ~(size_t)15) : sizes[at];
			size_t block;
			uint32_t class = size_class(size, &block);
			uint8_t *page = aligned_alloc(PAGE_SIZE, PAGE_SIZE);
			CHECK(page != 0);
			PageHeader *header = (PageHeader *)page;
			header->class = class;
			header->pages = 1;
			class_next[class] = page + PAGE_HEADER;
			class_end[class] = page + PAGE_SIZE;
			uint8_t *next = class_next[class];
			while ((size_t)(class_end[class] - next) >= block) {
				CHECK(fa_heap_alloc(sizes[at], align) == next);
				CHECK((uintptr_t)next % align == 0);
				next += block;
			}
			// The test platform refuses page refills. A full page must take
			// that path without an out-of-bounds pointer calculation.
			CHECK(fa_heap_alloc(sizes[at], align) == 0);
			fa_heap_free(page + PAGE_HEADER);
			CHECK(fa_heap_alloc(sizes[at], align) == page + PAGE_HEADER);
			CHECK(free_blocks[class] == 0);
			class_next[class] = class_end[class] = 0;
			free(page);
		}
	}
}

static int arena_cleanups;

static void release_region_payload(void *node) {
	fa_free(*(void **)node, 64, 16);
	arena_cleanups += 1;
}

static void test_tree_regions(void) {
	// Supply pages without the test platform's (deliberately failing) refill.
	uint8_t *memory = aligned_alloc(PAGE_SIZE, PAGE_SIZE * 6);
	CHECK(memory != 0 && free_spans == 0);
	give_pages(memory, 6);
	fa_arena_enter();
	uint64_t *first = fa_arena_alloc(32, 16);
	first[0] = 123;
	FaArena *owner = arena_of(first);
	for (size_t i = 0; i < PAGE_SIZE / 16 * 2; i += 1) {
		uint64_t *node = fa_arena_alloc(16, 16);
		CHECK((uintptr_t)node % 16 == 0 && arena_of(node) == owner);
		node[0] = i;
	}
	CHECK(first[0] == 123 && live_tree_arenas == 1);
	fa_arena_drop(first); // an intermediate root cannot free an active region
	CHECK(first[0] == 123 && live_tree_arenas == 1);
	fa_arena_leave(first);
	CHECK(tree_arena == 0 && tree_arena_depth == 0);
	fa_arena_enter();
	uint64_t *second = fa_arena_alloc(16, 16);
	second[0] = 456;
	CHECK(arena_of(second) != owner && live_tree_arenas == 2);
	fa_arena_enter();
	CHECK(arena_of(fa_arena_alloc(16, 16)) == arena_of(second));
	fa_arena_leave(second);
	fa_arena_leave(second);
	fa_arena_drop(first);
	CHECK(second[0] == 456 && live_tree_arenas == 1);
	fa_arena_enter_owned(second);
	void *boxed = fa_arena_alloc(16, 16);
	CHECK(arena_of(boxed) == arena_of(second));
	fa_arena_leave(boxed);
	fa_arena_drop(boxed);
	CHECK(live_tree_arenas == 0);
	// Returning a leaf after discarding a tree releases the unused region.
	fa_arena_enter();
	(void)fa_arena_alloc(16, 16);
	fa_arena_leave(0);
	CHECK(live_tree_arenas == 0 && tree_arena_depth == 0);
	long before = allocated;
	fa_arena_enter();
	void **payload = fa_arena_alloc(sizeof(void *), _Alignof(void *));
	*payload = fa_alloc(64, 16);
	fa_arena_cleanup(payload, release_region_payload);
	fa_arena_leave(payload);
	CHECK(arena_cleanups == 0 && allocated == before + 1);
	fa_arena_drop(payload);
	CHECK(arena_cleanups == 1 && allocated == before && live_tree_arenas == 0);
	fa_arena_enter();
	uint8_t *packed = fa_arena_alloc(24, 8);
	CHECK((uint8_t *)fa_arena_alloc(24, 8) == packed + 24);
	__builtin_memset(packed, 0x5a, 24);
	// Mix sizes and ABI alignments across page boundaries. Neither padding
	// nor refills may overwrite earlier allocations or lose their owner.
	for (size_t i = 0; i < PAGE_SIZE / 16; i += 1) {
		size_t align = (size_t)1 << (i % 6);
		size_t size = i % 37 + 1;
		uint8_t *node = fa_arena_alloc(size, align);
		CHECK((uintptr_t)node % align == 0 && arena_of(node) == arena_of(packed));
		CHECK((uintptr_t)node % ARENA_CHUNK_SIZE + size <= ARENA_CHUNK_SIZE);
		__builtin_memset(node, 0xa5, size);
		for (size_t byte = 0; byte < 24; byte += 1) { CHECK(packed[byte] == 0x5a); }
	}
	fa_arena_leave(packed);
	fa_arena_drop(packed);
	CHECK(live_tree_arenas == 0 && tree_arena_depth == 0);
	// A compact reservation returns its unused wide tail to this bump.
	fa_arena_enter();
	uint8_t *near_child = fa_arena_alloc(8, 8);
	uint8_t *near_node = fa_arena_alloc_near(8, 16, 8,
	    (uintptr_t)near_child, (uintptr_t)near_child, 0);
	CHECK(*(uint16_t *)near_node == 0);
	CHECK((uint8_t *)fa_arena_alloc(8, 8) == near_node + 8);
	__builtin_memset(near_node, 0x6b, 8);
	// Far links keep their full reservation, rather than wrapping.
	uint8_t *wide_node = fa_arena_alloc_near(8, 16, 8, 1, 1, 0);
	CHECK(*(uint16_t *)wide_node == 0x4000);
	CHECK((uint8_t *)fa_arena_alloc(8, 8) == wide_node + 16);
	CHECK(near_node[0] == 0x6b && near_node[7] == 0x6b);
	CHECK(arena_links_near(400000, 400000 - 32768 * 8, 400000 + 32767 * 8, 0));
	CHECK(!arena_links_near(400000, 400000 - 32768 * 8 - 8, 400000, 0));
	CHECK(!arena_links_near(400000, 400000, 400000 + 32767 * 8 + 8, 0));
	CHECK(!arena_links_near(400000, 400008, 400008, 1));
	CHECK(arena_links_near(400000, UINTPTR_MAX, 0, 1));
	fa_arena_leave(near_child);
	fa_arena_drop(near_child);
	CHECK(live_tree_arenas == 0 && tree_arena_depth == 0);
	// Fresh child array buffers share a root region. Their ordinary ABI and
	// observed capacity stay unchanged; large buffers get one heap cleanup.
	before = allocated;
	fa_arena_enter();
	FaArrayHeader *children = fa_array_new_region(0, 0, sizeof(uint64_t));
	CHECK(fa_array_is_region(children) && children->capacity == 0 && children->length == 0);
	for (uint64_t value = 0; value < 100; value += 1) {
		*(uint64_t *)fa_array_push_slot(children, sizeof(uint64_t)) = value;
	}
	int buffer_fits = 128 * sizeof(uint64_t) <= ARENA_CHUNK_SIZE - ARENA_HEADER;
	CHECK(allocated == before + (buffer_fits ? 0 : 1));
	CHECK((arena_of(children)->cleanup == 0) == buffer_fits);
	for (uint64_t value = 0; value < 100; value += 1) { CHECK(((uint64_t *)children->items)[value] == value); }
	fa_array_reserve(children, 20000, sizeof(uint64_t));
	CHECK(children->capacity == 20000 && allocated == before + 1);
	FaArenaCleanup *registered = arena_of(children)->cleanup;
	CHECK(registered != 0);
	fa_array_reserve(children, 40000, sizeof(uint64_t));
	CHECK(children->capacity == 40000 && allocated == before + 1);
	CHECK(arena_of(children)->cleanup == registered);
	for (uint64_t value = 0; value < 100; value += 1) { CHECK(((uint64_t *)children->items)[value] == value); }
	fa_arena_leave(children);
	FaArrayHeader *copied = fa_array_copy(children, sizeof(uint64_t));
	CHECK(!fa_array_is_region(copied));
	fa_arena_drop(children);
	CHECK(live_tree_arenas == 0 && copied->length == 100);
	for (uint64_t value = 0; value < 100; value += 1) { CHECK(((uint64_t *)copied->items)[value] == value); }
	fa_array_free(copied, sizeof(uint64_t));
	CHECK(allocated == before);
	// A local may be discarded before its region, including after a large
	// reservation. The later callback is idempotent and never frees a page.
	fa_arena_enter();
	children = fa_array_new_region(2, 20000, sizeof(uint64_t));
	CHECK(children->length == 2 && allocated == before + 1);
	fa_array_free(children, sizeof(uint64_t));
	fa_array_free(children, sizeof(uint64_t));
	CHECK(allocated == before);
	fa_arena_leave(children);
	fa_arena_drop(children);
	CHECK(live_tree_arenas == 0 && allocated == before);
	// A builder's temporary header and four slots live in its frame. Moving
	// the completed array into a member detaches it before the frame is reused.
	uint64_t packet[8];
	FaArrayHeader *local = (FaArrayHeader *)packet;
	fa_arena_enter();
	fa_array_init_region_local(local, 0, 0, sizeof(uint64_t));
	for (uint64_t index = 0; index < 10; index += 1) {
		// a push of the generated code: only the growth is the runtime's
		if (local->length >= local->capacity) { fa_array_region_grow(local, sizeof(uint64_t)); }
		((uint64_t *)local->items)[local->length] = index + 123;
		local->length += 1;
	}
	CHECK(allocated == before + 1 && tree_arena == 0);
	children = fa_array_into_region(local, sizeof(uint64_t));
	CHECK(children != local && allocated == before && children->length == 10);
	CHECK(local->length == 0 && !(array_marker(children) & ARRAY_REGION_LOCAL));
	__builtin_memset(packet, 0xcc, sizeof(packet));
	for (uint64_t index = 0; index < 10; index += 1) { CHECK(((uint64_t *)children->items)[index] == index + 123); }
	fa_arena_leave(children);
	fa_arena_drop(children);
	CHECK(live_tree_arenas == 0 && allocated == before);
	// Discarding the local frees a large temporary buffer immediately.
	fa_arena_enter();
	fa_array_init_region_local(local, 0, 100, sizeof(uint64_t));
	CHECK(allocated == before + 1 && tree_arena == 0);
	fa_array_free(local, sizeof(uint64_t));
	CHECK(allocated == before);
	fa_arena_leave(0);
	// Neighboring roots use independent 4 KiB chunks of the same heap page.
	// Interleaved releases/reuse must not return a page with live neighbors.
	uint64_t *roots[40];
	for (uint64_t index = 0; index < 40; index += 1) {
		fa_arena_enter();
		roots[index] = fa_arena_alloc(24, 8);
		roots[index][0] = index;
		fa_arena_leave(roots[index]);
	}
	for (size_t index = 1; index < 40; index += 2) { fa_arena_drop(roots[index]); }
	for (uint64_t index = 0; index < 40; index += 2) { CHECK(roots[index][0] == index); }
	for (uint64_t index = 1; index < 40; index += 2) {
		fa_arena_enter();
		roots[index] = fa_arena_alloc(24, 8);
		roots[index][0] = index + 100;
		fa_arena_leave(roots[index]);
	}
	for (uint64_t index = 0; index < 40; index += 1) {
		CHECK(roots[index][0] == (index % 2 == 0 ? index : index + 100));
		fa_arena_drop(roots[index]);
	}
	CHECK(live_tree_arenas == 0 && available_arena_pages == 0);
	// Compressed arenas use the same chunk granularity in their common
	// reserved address range, independent of heap-page subdivision.
	uint8_t *compressed = aligned_alloc(ARENA_CHUNK_SIZE, ARENA_CHUNK_SIZE * 40);
	CHECK(compressed != 0 && free_arena_pages == 0);
	fa_compressed_links = 1;
	arena_space_next = compressed;
	arena_space_committed = compressed + ARENA_CHUNK_SIZE * 40;
	arena_space_end = arena_space_committed;
	for (uint64_t index = 0; index < 40; index += 1) {
		fa_arena_enter();
		roots[index] = fa_arena_alloc(24, 8);
		roots[index][0] = index;
		fa_arena_leave(roots[index]);
		CHECK((uint8_t *)arena_of(roots[index]) == compressed + index * ARENA_CHUNK_SIZE);
	}
	for (size_t index = 40; index > 0; index -= 1) { fa_arena_drop(roots[index - 1]); }
	CHECK(live_tree_arenas == 0);
	fa_compressed_links = 0;
	arena_space_next = arena_space_committed = arena_space_end = 0;
	free_arena_pages = 0;
	free(compressed);
	CHECK((uint8_t *)free_spans == memory && free_spans->pages == 6 && free_spans->next == 0);
	free_spans = 0;
	free(memory);
}

static Utf8Info scalar_utf8_info(const uint8_t *bytes, size_t length) {
	Utf8Info info = {0, SIZE_MAX};
	size_t position = 0;
	while (position < length) {
		size_t start = position;
		int valid;
		uint32_t code = utf8_decode(bytes, length, &position, &valid);
		if (!valid) { info.error = start; break; }
		info.units += 1 + (code > 0xFFFF);
	}
	return info;
}

static void test_utf8_input(void) {
	long before = allocated, bytes_before = allocated_bytes;
	struct { const char *input; const char *output; size_t error; } cases[] = {
		{"", "", SIZE_MAX}, {"ASCII", "ASCII", SIZE_MAX},
		{"\xC3\xA9\xE6\x97\xA5\xF0\x9F\x98\x80", "\xC3\xA9\xE6\x97\xA5\xF0\x9F\x98\x80", SIZE_MAX},
		{"\xEF\xBB\xBF", "\xEF\xBB\xBF", SIZE_MAX},
		{"\xF0\x9F", "\xEF\xBF\xBD", 0},
		{"\xED\xA0\x80", "\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD", 0},
		{"\xE1\x80x", "\xEF\xBF\xBDx", 0},
		{"\xC0\xAF", "\xEF\xBF\xBD\xEF\xBF\xBD", 0},
		{"\xF4\x90\x80\x80", "\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD", 0},
		{"\xEF\xBB\xBF\xE2\x82", "\xEF\xBB\xBF\xEF\xBF\xBD", 3},
		{"\x80z", "\xEF\xBF\xBDz", 0},
		{"\xE0\x9F\xBF", "\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD", 0},
		{"\xF0\x8F\xBF\xBF", "\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD\xEF\xBF\xBD", 0},
		{"a\xE2\x82\xACz\xE2\x82", "a\xE2\x82\xACz\xEF\xBF\xBD", 5},
	};
	for (size_t item = 0; item < sizeof cases / sizeof *cases; item += 1) {
		for (size_t prefix = 0; prefix < 32; prefix += 1) {
			for (size_t suffix = 0; suffix <= 32; suffix += 32) {
				size_t input_length = c_length(cases[item].input);
				size_t output_length = c_length(cases[item].output);
				size_t length = prefix + input_length + suffix;
				uint8_t *input = malloc(length ? length : 1);
				fill_bytes(input, 'a', prefix);
				copy_bytes(input + prefix, (const uint8_t *)cases[item].input, input_length);
				fill_bytes(input + prefix + input_length, 'z', suffix);
				Utf8Info actual = utf8_validate(input, length);
				Utf8Info scalar = scalar_utf8_info(input, length);
				CHECK(actual.units == scalar.units && actual.error == scalar.error);
				CHECK(actual.error == (cases[item].error == SIZE_MAX ? SIZE_MAX : prefix + cases[item].error));
				FaString result;
				fa_string_from_utf8_lossy(input, length, &result);
				const uint8_t *text = string_bytes(&result);
				CHECK(string_length(&result) == prefix + output_length + suffix);
				CHECK(string_equal_bytes(text + prefix, (const uint8_t *)cases[item].output, output_length));
				CHECK(fa_string_length(text, string_length(&result), result.capacity)
				      == prefix + unit_count((const uint8_t *)cases[item].output, output_length) + suffix);
				CHECK(fa_string_is_well_formed(text, string_length(&result), result.capacity));
				FaUtf8Cursor cursor = {0};
				for (size_t index = 0; index < string_length(&result); index += 1) {
					CHECK(fa_next_utf8_byte(text, string_length(&result), &cursor) == text[index]);
				}
				CHECK(cursor.remaining == 0 && cursor.position == string_length(&result));
				if (string_owned(result.capacity)) { fa_string_free(result.bytes, result.capacity); }
				free(input);
			}
		}
	}
	// Exercise each lane and each byte value against the scalar decoder, with
	// valid multibyte sequences on both sides of the mutated lane.
	uint8_t data[64];
	for (size_t index = 0; index < sizeof data; index += 4) {
		copy_bytes(data + index, (const uint8_t *)"\xF0\x9F\x98\x80", 4);
	}
	for (size_t index = 0; index < sizeof data; index += 1) {
		uint8_t old = data[index];
		for (unsigned byte = 0; byte < 256; byte += 1) {
			data[index] = (uint8_t)byte;
			for (size_t length = 16; length <= sizeof data; length += 1) {
				Utf8Info actual = utf8_validate(data, length);
				Utf8Info scalar = scalar_utf8_info(data, length);
				if (actual.units != scalar.units || actual.error != scalar.error) {
					printf("UTF-8 vector mismatch lane=%zu byte=%u length=%zu actual=%zu/%zu scalar=%zu/%zu\n",
					       index, byte, length, actual.units, actual.error, scalar.units, scalar.error);
					fflush(0);
				}
				CHECK(actual.units == scalar.units && actual.error == scalar.error);
			}
		}
		data[index] = old;
	}
	const uint8_t *pair = (const uint8_t *)"\xF0\x9F\x98\x80\xF0\x9F\x98\x80";
	const uint8_t *spans[] = {(const uint8_t *)"\xED\xA0\xBD", pair, pair + 2, pair + 2};
	size_t lengths[] = {3, 3, 2, 5};
	for (size_t index = 0; index < 4; index += 1) {
		size_t count = index == 3 ? 6 : 3;
		FaUtf8Cursor cursor = {0};
		for (size_t byte = 0; byte < count; byte += 1) {
			CHECK(fa_next_utf8_byte(spans[index], lengths[index], &cursor) == (byte % 3 == 0 ? 0xEF : byte % 3 == 1 ? 0xBF : 0xBD));
			CHECK(fa_string_utf8_byte(spans[index], lengths[index], STRING_COUNTED_VIEW | (index == 3 ? 2 : 1), byte)
			      == (byte % 3 == 0 ? 0xEF : byte % 3 == 1 ? 0xBF : 0xBD));
		}
		CHECK(cursor.remaining == 0 && cursor.position == lengths[index]);
		uint8_t copied[8] = {0};
		fa_string_copy_block(copied, spans[index], lengths[index], STRING_COUNTED_VIEW | (index == 3 ? 2 : 1));
		CHECK(unit_count(copied, count) == (index == 3 ? 2 : 1));
		size_t capacity = STRING_COUNTED_VIEW | (index == 3 ? 2 : 1);
		CHECK(fa_string_index_of(spans[index], lengths[index], capacity, copied, count, 0) == 0);
		CHECK(fa_string_index_of(copied, count, capacity, spans[index], lengths[index], 1) == 0);
		CHECK(fa_string_includes(spans[index], lengths[index], capacity, copied, count));
		CHECK(fa_string_starts_with(spans[index], lengths[index], capacity, copied, count));
		CHECK(fa_string_ends_with(spans[index], lengths[index], capacity, copied, count));
		CHECK(fa_string_code_point_count(spans[index], lengths[index], capacity) == (index == 3 ? 2 : 1));
	}
	FaFileJob *file = fa_file_job(FILE_WRITE, pair + 2, 2, pair, 3);
	CHECK(file->path_length == 3 && file->path[3] == 0 && file->length == 3);
	CHECK(string_equal_bytes((const uint8_t *)file->path, (const uint8_t *)"\xEF\xBF\xBD", 3));
	CHECK(string_equal_bytes(file->bytes, (const uint8_t *)"\xEF\xBF\xBD", 3));
	file_job_release(&file->job);
	CHECK(allocated == before && allocated_bytes == bytes_before);
}

static void drop_test_string_parts(FaArrayHeader *parts) {
	CHECK(parts->capacity >= 0);
	FaString *items = (FaString *)(void *)parts->items;
	for (int64_t index = 0; index < parts->length; index += 1) {
		if (string_owned(items[index].capacity)) { fa_string_free(items[index].bytes, items[index].capacity); }
	}
	fa_array_free(parts, sizeof(FaString));
}

static void test_unicode_transform_counts(void) {
	fa_release_builder();
	long before = allocated;
	uint8_t bytes[204];
	for (size_t index = 0; index < 200; index += 2) { copy_bytes(bytes + index, (const uint8_t *)"\xC3\xA9", 2); }
	copy_bytes(bytes + 200, (const uint8_t *)"\xF0\x9F\x98\x80", 4);
	FaString source;
	fa_string_from(bytes, sizeof bytes, 0, 512, &source);
	FaString high, low;
	unit_string(0xD83D, &high);
	unit_string(0xDE00, &low);
	FaArrayHeader *parts = fa_string_split_views(source.bytes, source.length, source.capacity,
	                                             string_bytes(&low), string_length(&low));
	CHECK(parts->length == 2);
	FaString *items = (FaString *)(void *)parts->items;
	CHECK(!string_owned(items[0].capacity));
	CHECK(items[0].bytes == source.bytes && items[0].length == 203);
	CHECK(fa_string_length(items[0].bytes, items[0].length, items[0].capacity) == 101);
	CHECK(!fa_string_is_well_formed(items[0].bytes, items[0].length, items[0].capacity));
	drop_test_string_parts(parts);
	parts = fa_string_split_views(source.bytes, source.length, source.capacity, (const uint8_t *)"x", 1);
	items = (FaString *)(void *)parts->items;
	CHECK(parts->length == 1 && string_counted_view(items[0].capacity));
	CHECK(items[0].bytes == source.bytes && fa_string_length(items[0].bytes, items[0].length, items[0].capacity) == 102);
	drop_test_string_parts(parts);
	parts = fa_string_split_views(source.bytes, source.length, source.capacity, (const uint8_t *)"", 0);
	CHECK(parts->length == 102);
	items = (FaString *)(void *)parts->items;
	for (int64_t index = 0; index < parts->length; index += 1) {
		CHECK(fa_string_length(string_bytes(items + index), string_length(items + index), items[index].capacity) == 1);
	}
	drop_test_string_parts(parts);
	FaString result;
	fa_string_replace(source.bytes, source.length, source.capacity, string_bytes(&low), string_length(&low), (const uint8_t *)"", 0, 1, &result);
	CHECK(fa_string_length(string_bytes(&result), string_length(&result), result.capacity) == 101);
	CHECK(!fa_string_is_well_formed(string_bytes(&result), string_length(&result), result.capacity));
	if (string_owned(result.capacity)) { fa_string_free(result.bytes, result.capacity); }
	fa_string_pad(string_bytes(&high), string_length(&high), high.capacity, 2, string_bytes(&low), string_length(&low), 0, &result);
	CHECK(fa_string_equal(string_bytes(&result), string_length(&result), bytes + 200, 4));
	CHECK(fa_string_is_well_formed(string_bytes(&result), string_length(&result), result.capacity));
	if (string_owned(result.capacity)) { fa_string_free(result.bytes, result.capacity); }
	CHECK(fa_string_length(source.bytes, source.length, source.capacity) == 102);
	FaArrayHeader *points = fa_string_code_points(bytes + 202, 2, STRING_COUNTED_VIEW | 1);
	CHECK(points->length == 1 && ((const uint32_t *)(const void *)points->items)[0] == 0xDE00);
	CHECK(fa_string_code_point_at(bytes + 202, 2, STRING_COUNTED_VIEW | 1, -1) == 0xDE00);
	fa_array_free(points, sizeof(uint32_t));
	fa_string_free(source.bytes, source.capacity);
	fa_release_builder();
	CHECK(allocated == before);
}

static void test_canonical_split_views_without_edges(void) {
	long before = allocated;
	const uint8_t text[] = "é  😀 xx";
	fa_virtual_string_slices = 0;
	for (int mode = 0; mode < 3; mode += 1) {
		_Alignas(16) uint8_t buffer[512];
		FaArrayHeader *parts = mode == 0
		    ? fa_string_split_views(text, sizeof text - 1, 0, (const uint8_t *)" ", 1)
		    : mode == 1
		        ? fa_string_split_nonempty_views(text, sizeof text - 1, 0, (const uint8_t *)" ", 1)
		        : fa_string_split_views_into(text, sizeof text - 1, 0, (const uint8_t *)" ", 1, buffer, sizeof buffer);
		CHECK(parts->length == (mode == 1 ? 3 : 4));
		FaString *items = (FaString *)(void *)parts->items;
		const size_t expected[] = {1, 0, 2, 2};
		for (int64_t index = 0; index < parts->length; index += 1) {
			size_t expected_index = (size_t)index + (mode == 1 && index > 0);
			CHECK(!string_owned(items[index].capacity));
			CHECK(fa_string_is_well_formed(string_bytes(items + index), string_length(items + index), items[index].capacity));
			CHECK(fa_string_length(string_bytes(items + index), string_length(items + index), items[index].capacity) == expected[expected_index]);
		}
		drop_test_string_parts(parts);
	}
	fa_virtual_string_slices = 1;
	CHECK(allocated == before);
}

static void test_arguments(void) {
	char *argv[] = {"program", "one", "two"};
	for (int32_t argc = -1; argc <= 3; argc += 1) {
		long before = allocated;
		fa_arguments_init(argc, argv);
		FaArrayHeader *array = fa_arguments();
		int64_t count = argc > 1 ? argc - 1 : 0;
		CHECK(array->length == count && array->capacity >= 0);
		FaString first;
		CHECK(fa_string_array_get(array, 0, &first) == (count != 0));
		if (count != 0) { CHECK(fa_string_equal(string_bytes(&first), string_length(&first), (const uint8_t *)"one", 3)); }
		fa_array_free(array, sizeof(FaString));
		fa_free(argument_items, sizeof(FaString) * (argument_count + 1), sizeof(void *));
		argument_items = 0;
		argument_count = 0;
		CHECK(allocated == before);
	}
	long before = allocated, bytes_before = allocated_bytes;
	char *external[] = {"program", "\xC3\xA9\xF0\x9F\x98\x80", "\xE1\x80x",
	                    "a long argument that needs owned replacement bytes: \xED\xA0\x80", "ASCII", ""};
	fa_arguments_init(6, external);
	CHECK(arguments_have_owners);
	CHECK(fa_string_length(argument_items[0].bytes, argument_items[0].length, argument_items[0].capacity) == 3);
	for (unsigned visit = 0; visit < 2; visit += 1) {
		FaArrayHeader *array = fa_arguments();
		FaString *items = (FaString *)(void *)array->items;
		CHECK(fa_string_length(string_bytes(items + 1), string_length(items + 1), items[1].capacity) == 2);
		CHECK(!string_owned(items[2].capacity));
		for (size_t index = 0; index < argument_count; index += 1) {
			if (string_owned(items[index].capacity)) { fa_string_free(items[index].bytes, items[index].capacity); }
		}
		fa_array_free(array, sizeof(FaString));
	}
	for (size_t index = 0; index < argument_count; index += 1) {
		if (string_owned(argument_items[index].capacity)) { fa_string_free(argument_items[index].bytes, argument_items[index].capacity); }
	}
	fa_free(argument_items, sizeof(FaString) * (argument_count + 1), sizeof(void *));
	argument_items = 0;
	argument_count = 0;
	arguments_have_owners = 0;
	CHECK(allocated == before && allocated_bytes == bytes_before);
}

static void test_strict_file_text(void) {
	fa_release_builder();
	long before = allocated, bytes_before = allocated_bytes;
	const char *inputs[] = {"\xC3\xA9\xE6\x97\xA5\xF0\x9F\x98\x80", "ASCII",
	                       "a long text that needs an owning Unicode prefix: \xC3\xA9\xF0\x9F\x98\x80",
	                       "\xE1\x80x", "\xED\xA0\x80", "\xF0\x9F"};
	for (size_t index = 0; index < sizeof inputs / sizeof *inputs; index += 1) {
		size_t length = c_length(inputs[index]);
		FaFileJob *file = fa_file_job(FILE_READ, (const uint8_t *)"path", 4, 0, 0);
		file->room = length ? string_block(length) : 0;
		file->bytes = file->room ? file_buffer_allocate(file, file->room) : 0;
		file->length = length;
		copy_bytes(file->bytes, (const uint8_t *)inputs[index], length);
		const uint8_t *original = file->bytes;
		FaString result = {0};
		const FileErrorBox *error = fa_file_finish(file, &result);
		if (index < 3) {
			CHECK(!error);
			if (length > INLINE_STRING_BYTES) { CHECK(result.bytes == original); }
			CHECK(fa_string_length(string_bytes(&result), string_length(&result), result.capacity)
			      == unit_count((const uint8_t *)inputs[index], length));
			CHECK(string_equal_bytes(string_bytes(&result), (const uint8_t *)inputs[index], length));
			if (string_owned(result.capacity)) { fa_string_free(result.bytes, result.capacity); }
		} else {
			CHECK(error && error->id == FILE_ERROR);
			CHECK(fa_string_equal(string_bytes(&error->message), string_length(&error->message), (const uint8_t *)"invalid UTF-8", 13));
			if (string_owned(error->message.capacity)) { fa_string_free(error->message.bytes, error->message.capacity); }
			if (string_owned(error->path.capacity)) { fa_string_free(error->path.bytes, error->path.capacity); }
			fa_free((void *)error, sizeof *error, sizeof(int64_t));
		}
	}
	fa_release_builder();
	CHECK(allocated == before && allocated_bytes == bytes_before);
}

static void test_character_views(void) {
	uint8_t source[200];
	for (size_t i = 0; i < sizeof source; i += 1) { source[i] = (uint8_t)('a' + i % 26); }
	for (size_t length = 0; length <= sizeof source; length += 1) {
		long before = allocated;
		FaArrayHeader *view = fa_string_characters(source, length, SIMPLE);
		CHECK(view->capacity < 0 && view->length == (int64_t)length);
		FaString character;
		CHECK(!fa_string_array_get(view, (int64_t)length, &character));
		if (length > 0) {
			uint8_t last = source[length - 1];
			source[length - 1] = '!';
			CHECK(fa_string_array_get(view, -1, &character));
			CHECK(string_length(&character) == 1 && string_bytes(&character)[0] == last);
			source[length - 1] = last;
		}
		if (length % 2 == 0) {
			fa_array_materialize(view);
			CHECK(view->capacity >= 0);
			for (size_t i = 0; i < length; i += 1) {
				CHECK(fa_string_array_get(view, (int64_t)i, &character));
				CHECK(string_bytes(&character)[0] == source[i]);
			}
		}
		fa_array_free(view, sizeof(FaString));
		CHECK(allocated == before);
	}
}

static void test_short_copies(void) {
	// Exact allocation ends catch overreads; varied alignment and lengths
	// cover every short-copy branch and the transition to memcpy.
	for (size_t length = 0; length <= 65; length += 1) {
		for (size_t offset = 0; offset < 16; offset += 1) {
			uint8_t *source = malloc(length + offset + 1);
			uint8_t *target = malloc(length + offset + 1);
			for (size_t i = 0; i <= length + offset; i += 1) {
				source[i] = (uint8_t)(i * 17);
				target[i] = 0xa5;
			}
			copy_bytes(target + offset + 1, source + offset + 1, length);
			for (size_t i = 0; i <= offset; i += 1) { CHECK(target[i] == 0xa5); }
			for (size_t i = offset + 1; i <= offset + length; i += 1) { CHECK(target[i] == source[i]); }
			uint8_t byte = (uint8_t)(offset * 17);
			fill_bytes(target + offset + 1, byte, length);
			for (size_t i = 0; i <= offset; i += 1) { CHECK(target[i] == 0xa5); }
			for (size_t i = offset + 1; i <= offset + length; i += 1) { CHECK(target[i] == byte); }
			const uint8_t *text = source + offset + 1;
			for (unsigned needle = 0; needle <= 255; needle += 1) {
				for (size_t start = 0; start <= length; start += length / 2 + 1) {
					size_t expected = start;
					while (expected < length && text[expected] != needle) { expected += 1; }
					CHECK(next_byte(text, length, start, (uint8_t)needle) == expected);
				}
			}
			free(source);
			free(target);
		}
	}
}

static void test_string_equal(void) {
	// End exactly at the allocation boundary so ASan detects overreads in
	// short tails. Also exercise unaligned inputs and every mismatching byte.
	for (size_t length = 0; length <= 65; length += 1) {
		for (size_t offset = 0; offset < 8; offset += 1) {
			uint8_t *left = malloc(length + offset + 1);
			uint8_t *right = malloc(length + offset + 1);
			for (size_t i = 0; i <= length; i += 1) {
				left[offset + i] = right[offset + i] = (uint8_t)(i * 17);
			}
			const uint8_t *a = left + offset + 1;
			uint8_t *b = right + offset + 1;
			CHECK(fa_string_equal(a, length, b, length));
			for (size_t i = 0; i < length; i += 1) {
				b[i] ^= 1;
				CHECK(!fa_string_equal(a, length, b, length));
				b[i] ^= 1;
			}
			if (length > 0) { CHECK(!fa_string_equal(a, length, b, length - 1)); }
			free(left);
			free(right);
		}
	}
}

static void test_string_keys(void) {
	uint8_t ascii[65];
	__builtin_memset(ascii, 'x', sizeof ascii);
	for (size_t length = 0; length <= sizeof ascii; length += 1) {
		FaString owner;
		fa_string_own(ascii, length, SIMPLE, &owner);
		FaString view = {ascii, length, SIMPLE | KIND_MASK};
		FaString unmarked = {ascii, length, KIND_MASK};
		CHECK(string_keys_equal((const uint8_t *)&owner, &view));
		CHECK(string_keys_equal((const uint8_t *)&view, &owner));
		CHECK(string_keys_equal((const uint8_t *)&owner, &unmarked));
		if (length) {
			ascii[length - 1] = 'y';
			CHECK(!string_keys_equal((const uint8_t *)&owner, &view));
			ascii[length - 1] = 'x';
		}
		if (string_owned(owner.capacity)) { fa_string_free(owner.bytes, owner.capacity); }
	}
	const uint8_t physical[] = {0x98, 0x80, 'x', 0xF0, 0x9F, 0x98};
	const uint8_t canonical[] = {0xED, 0xB8, 0x80, 'x', 0xED, 0xA0, 0xBD};
	FaString edge = {physical, sizeof physical, STRING_COUNTED_VIEW | 3};
	FaString owner;
	fa_string_own(canonical, sizeof canonical, STRING_SURROGATES, &owner);
	CHECK(string_keys_equal((const uint8_t *)&owner, &edge));
	CHECK(string_keys_equal((const uint8_t *)&edge, &owner));
	FaString view = {ascii, sizeof physical, SIMPLE | KIND_MASK};
	CHECK(!string_keys_equal((const uint8_t *)&view, &edge));
	CHECK(!string_keys_equal((const uint8_t *)&edge, &view));
	if (string_owned(owner.capacity)) { fa_string_free(owner.bytes, owner.capacity); }
}

static void test_reserving_cut_string_views(void) {
	for (size_t middle = 0; middle <= 80; middle += 40) {
		uint8_t physical[85] = {0x98, 0x80};
		__builtin_memset(physical + 2, 'x', middle);
		copy_bytes(physical + middle + 2, (const uint8_t *)"\xF0\x9F\x98", 3);
		size_t length = middle + 5;
		size_t flags = STRING_COUNTED_VIEW | (middle + 2);
		uint8_t canonical[86];
		fa_string_copy_bytes(canonical, physical, length, flags);
		for (int mode = 0; mode < 3; mode += 1) {
			for (size_t extra = 0; extra <= 128; extra += 64) {
				FaString string = {physical, length, flags};
				if (mode == 0) { fa_string_reserve(&string, extra); }
				else if (mode == 1) { fa_string_reserve_owned(&string, extra); }
				else { fa_string_reserve_inline(&string, extra); }
				CHECK(fa_string_equal(string_bytes(&string), string_length(&string), canonical, length + 1));
				CHECK(fa_string_length(string_bytes(&string), string_length(&string), string.capacity) == middle + 2);
				CHECK(fa_string_room(string_length(&string), string.capacity) >= length + 1 + extra);
				if (string_owned(string.capacity)) { fa_string_free(string.bytes, string.capacity); }
			}
		}
	}
}

typedef struct {
	int64_t key;
	int64_t value;
} TestTableEntry;

static int32_t table_test_equals(const void *left, const void *right) {
	return *(const int64_t *)left == *(const int64_t *)right;
}

// Every key collides when set: the same hash, so the same tag and home.
static int table_test_colliding;

static uint64_t table_test_hash(const void *key) {
	return table_test_colliding ? 7 : (uint64_t)*(const int64_t *)key * 0x9e3779b97f4a7c15ull;
}

static int64_t table_test_find(FaTable *table, int64_t key) {
	return fa_table_find(table, table_test_hash(&key), &key, table_test_equals,
	                     sizeof(TestTableEntry), 0);
}

static TestTableEntry *table_test_insert(FaTable *table, int64_t key) {
	int64_t probe = table_test_find(table, key);
	CHECK(probe < 0);
	TestTableEntry *entry = fa_table_append(table, table_test_hash(&key), sizeof(TestTableEntry),
	                                        probe, 0, table_test_hash);
	entry->key = key;
	entry->value = key * 3;
	return entry;
}

// Every third key is removed in the walk that finds it, the others found then killed.
static void table_test_kill(FaTable *table, int64_t key) {
	int64_t position = table_test_find(table, key);
	CHECK(position >= 0 && fa_table_live(table, position, sizeof(TestTableEntry)));
	if (key % 3 == 0) {
		int64_t removed = fa_table_remove(table, table_test_hash(&key), &key, table_test_equals,
		                                  sizeof(TestTableEntry), 0);
		CHECK(removed == position);
		CHECK(table_test_find(table, key) < 0);
		fa_table_release(table, position, sizeof(TestTableEntry));
	} else {
		fa_table_kill(table, table_test_hash(&key), position, sizeof(TestTableEntry));
	}
	CHECK(table_test_find(table, key) < 0 && !fa_table_live(table, position, sizeof(TestTableEntry)));
	CHECK(fa_table_remove(table, table_test_hash(&key), &key, table_test_equals,
	                      sizeof(TestTableEntry), 0) < 0);
}

static void test_tables(void) {
	long before = allocated;
	for (int colliding = 0; colliding < 2; colliding += 1) {
		table_test_colliding = colliding;
		FaTable *table = fa_table_new(4, sizeof(TestTableEntry));
		TestTableEntry *first = table_test_insert(table, 0);
		for (int64_t key = 1; key < 200; key += 1) {
			TestTableEntry *entry = table_test_insert(table, key);
			// without deletions, the positions follow the insertions
			CHECK(entry == (TestTableEntry *)fa_table_entry(table, key, sizeof(TestTableEntry)));
		}
		CHECK(table->length == 200 && table->used == 200 && table->mask + 1 == 512);
		// growth moves no entry
		CHECK(first == (TestTableEntry *)fa_table_entry(table, 0, sizeof(TestTableEntry)));
		first->value = 42;
		// deletions in the middle of runs shift the following slots back
		for (int64_t key = 1; key < 199; key += 2) {
			table_test_kill(table, key);
			CHECK(table_test_find(table, key + 1) >= 0);
		}
		for (int64_t key = 0; key < 200; key += 1) {
			int64_t position = table_test_find(table, key);
			CHECK((position >= 0) == (key % 2 == 0 || key == 199));
			if (position >= 0) {
				TestTableEntry *entry =
					(TestTableEntry *)fa_table_entry(table, position, sizeof(TestTableEntry));
				CHECK(entry->key == key && fa_table_live(table, position, sizeof(TestTableEntry)));
			}
		}
		// the dead entries are reused, lowest first in a chunk
		int64_t used = table->used;
		TestTableEntry *reused = table_test_insert(table, 201);
		CHECK(table_test_find(table, 201) % 32 == 1);
		for (int64_t key = 202; key < 300; key += 1) {
			table_test_insert(table, key);
		}
		CHECK(table->used == used && first->value == 42 && reused->key == 201);
		FaTable *copy = fa_table_copy(table, sizeof(TestTableEntry));
		CHECK(copy->length == table->length && copy->used == table->used);
		for (int64_t key = 0; key < 300; key += 1) {
			CHECK(table_test_find(copy, key) == table_test_find(table, key));
		}
		table_test_kill(copy, 0);
		CHECK(table_test_find(table, 0) == 0);
		table_test_insert(copy, 1000);
		CHECK(table_test_find(table, 1000) < 0);
		fa_table_free(copy, sizeof(TestTableEntry));
		int64_t capacity = table->capacity;
		fa_table_clear(table, sizeof(TestTableEntry));
		CHECK(table->length == 0 && table->used == 0 && table->capacity == capacity);
		CHECK(table_test_find(table, 0) < 0);
		// a cleared table fills its chunks in order again
		for (int64_t key = 0; key < 100; key += 1) {
			table_test_insert(table, key);
			CHECK(table_test_find(table, key) == key);
		}
		CHECK(table->capacity == capacity);
		fa_table_free(table, sizeof(TestTableEntry));
	}
	// A fixed-size live set under churn neither grows the entries nor the index.
	table_test_colliding = 0;
	FaTable *table = fa_table_new(4, sizeof(TestTableEntry));
	table_test_insert(table, -1);
	for (int64_t key = 0; key < 10000; key += 1) {
		table_test_insert(table, key);
		table_test_kill(table, key);
		CHECK(table->length == 1 && table->mask + 1 == 8);
		CHECK(table->capacity == 32 && table->used == 2);
	}
	CHECK(table_test_find(table, -1) == 0);
	// `reserve` grows the index, from its own hashes, and allocates the
	// chunks of the entries to come: no entry moves
	CHECK(fa_table_capacity(table) == 4);
	TestTableEntry *kept = (TestTableEntry *)fa_table_entry(table, 0, sizeof(TestTableEntry));
	fa_table_reserve(table, 100, sizeof(TestTableEntry), table_test_hash);
	CHECK(fa_table_capacity(table) >= 101 && table->mask + 1 == 256 && table->capacity == 128);
	CHECK(table_test_find(table, -1) == 0 && kept->key == -1);
	fa_table_reserve(table, 10, sizeof(TestTableEntry), table_test_hash);
	CHECK(table->mask + 1 == 256 && table->capacity == 128);
	// a full chunk leaves the list of chunks with a free entry, and returns to it
	for (int64_t key = 0; key < 70; key += 1) {
		table_test_insert(table, key);
	}
	CHECK(table->capacity == 128 && table->free == 3);
	table_test_kill(table, 5);
	CHECK(table->free == 1 && table->chunks[0].next == 3);
	CHECK(table_test_find(table, 5) < 0);
	table_test_insert(table, 5);
	CHECK(table_test_find(table, 5) == 6 && table->free == 3);
	fa_table_free(table, sizeof(TestTableEntry));
	// Bitwise keys: an equal tag is not an equal key.
	FaTable *bitwise = fa_table_new(4, sizeof(TestTableEntry));
	for (int64_t key = 0; key < 50; key += 1) {
		int64_t probe = fa_table_find(bitwise, 7, &key, 0, sizeof(TestTableEntry), 8);
		CHECK(probe < 0);
		TestTableEntry *entry = fa_table_append(bitwise, 7, sizeof(TestTableEntry), probe, 0, 0);
		entry->key = key;
	}
	for (int64_t key = 0; key < 60; key += 1) {
		int64_t position = fa_table_find(bitwise, 7, &key, 0, sizeof(TestTableEntry), 8);
		CHECK(key < 50 ? position == key : position < 0);
	}
	fa_table_free(bitwise, sizeof(TestTableEntry));
	// The wide index (beyond 2^31 slots), with small arrays: every probe stays
	// in a run at the start of the index.
	WideSlot wide[8] = {{0, 0}};
	TestTableEntry entries[32] = {{10, 0}, {20, 0}, {30, 0}};
	TableChunk chunk = {(uint8_t *)entries, 7, 0};
	FaTable header = {0};
	header.chunks = &chunk;
	header.index = wide;
	header.mask = 0xffffffffll;
	header.used = 3;
	header.capacity = 32;
	header.directory = 1;
	header.length = 3;
	header.free = 1;
	// homes 0, 0 and 1: slots 0, 1 and 2
	uint64_t hashes[3] = {0x100000000ull, 0x200000000ull, 1};
	for (int64_t position = 0; position < 3; position += 1) {
		table_place(&header, hashes[position], position, -1);
	}
	CHECK(wide[0].position == 1 && wide[1].position == 2 && wide[2].position == 3);
	for (int64_t position = 0; position < 3; position += 1) {
		int64_t key = entries[position].key;
		CHECK(fa_table_find(&header, hashes[position], &key, 0, sizeof(TestTableEntry), 8)
		      == position);
	}
	int64_t absent = 40;
	CHECK(fa_table_find(&header, 0x300000000ull, &absent, 0, sizeof(TestTableEntry), 8) == ~3ll);
	// the full hash is compared: the same low bits are another key
	CHECK(fa_table_find(&header, 0x100000001ull, &absent, 0, sizeof(TestTableEntry), 8) == ~3ll);
	fa_table_kill(&header, hashes[0], 0, sizeof(TestTableEntry));
	CHECK(wide[0].position == 2 && wide[1].position == 3 && wide[2].position == 0);
	CHECK(header.length == 2 && chunk.live == 6);
	int64_t key = 30;
	CHECK(fa_table_find(&header, 1, &key, 0, sizeof(TestTableEntry), 8) == 2);
	// a narrow index slot keeps the position above the 32 bits of the tag
	uint64_t narrow[8] = {0};
	header.index = narrow;
	header.mask = 7;
	table_place(&header, 0xffffffff, 1, -1);
	CHECK(narrow[7] == (0xffffffffull | (2ull << 32)));
	CHECK(allocated == before);
}

// Inline 32-bit keys use every bit pattern, including zero and signed extrema.
typedef struct {
	uint32_t key;
	uint32_t value;
} TestKey32Entry;

static uint64_t key32_hash(const void *key) {
	uint32_t bits;
	__builtin_memcpy(&bits, key, sizeof bits);
	return inline_key_hash(bits);
}

static int64_t key32_find(FaTable *table, uint32_t key, int64_t entry_size) {
	return fa_table_find(table, key32_hash(&key), &key, 0, entry_size, 4);
}

static void key32_insert(FaTable *table, uint32_t key, int64_t entry_size, int known_probe) {
	int64_t probe = key32_find(table, key, entry_size);
	CHECK(probe < 0);
	TestKey32Entry *entry = fa_table_append(table, key32_hash(&key), entry_size,
	                                      known_probe ? probe : 0, key, key32_hash);
	entry->key = key;
	entry->value = key ^ 0xa5a5a5a5u;
}

static void key32_check(FaTable *table, const uint32_t *keys, const uint8_t *present,
                       int count, int64_t entry_size) {
	int64_t length = 0;
	for (int index = 0; index < count; index += 1) {
		int64_t at = key32_find(table, keys[index], entry_size);
		CHECK((at >= 0) == (present[index] != 0));
		if (at >= 0) {
			TestKey32Entry *entry = (TestKey32Entry *)fa_table_entry(table, at, entry_size);
			CHECK(entry->key == keys[index] && entry->value == (keys[index] ^ 0xa5a5a5a5u));
			length += 1;
		}
	}
	CHECK(table->length == length);
	int64_t live = 0;
	for (int64_t at = 0; at < table->used; at += 1) {
		live += fa_table_live(table, at, entry_size);
	}
	CHECK(live == length);
}

static void test_inline_key32(void) {
	long before = allocated;
	for (int compact = 0; compact < 2; compact += 1) {
		int64_t entry_size = sizeof(TestKey32Entry) | TABLE_INLINE |
		                     (compact ? TABLE_INLINE_KEY32 : 0);
		CHECK(inline_stride(entry_size) == (compact ? 8 : 12));
		FaTable *table = fa_table_new(4, entry_size);
		// Both ordinary zero and the reserved sentinel remain valid keys.
		// Grow with the sentinel present, then clear and insert both again.
		for (int round = 0; round < 2; round += 1) {
			key32_insert(table, 0, entry_size, round);
			key32_insert(table, UINT32_MAX, entry_size, !round);
			if (compact) {
				CHECK(key32_find(table, 0, entry_size) < table->mask + 1);
				CHECK(key32_find(table, UINT32_MAX, entry_size) == table->mask + 1);
			}
			fa_table_reserve(table, 128, entry_size, key32_hash);
			const uint32_t sentinel_keys[] = {0, UINT32_MAX};
			const uint8_t both_present[] = {1, 1};
			key32_check(table, sentinel_keys, both_present, 2, entry_size);
			FaTable *copy = fa_table_copy(table, entry_size);
			key32_check(copy, sentinel_keys, both_present, 2, entry_size);
			fa_table_free(copy, entry_size);
			fa_table_clear(table, entry_size);
			CHECK(key32_find(table, 0, entry_size) < 0);
			CHECK(key32_find(table, UINT32_MAX, entry_size) < 0);
		}
		uint32_t keys[256];
		uint8_t present[256] = {0};
		for (int index = 0; index < 256; index += 1) {
			keys[index] = index == 0 ? 0 : index == 1 ? 0x80000000u :
			              index == 2 ? 0xffffffffu : index == 3 ? 0x7fffffffu :
			              (uint32_t)index - 3;
		}
		uint32_t random = 12345;
		for (int round = 0; round < 4000; round += 1) {
			random ^= random << 13;
			random ^= random >> 17;
			random ^= random << 5;
			int index = (int)(random & 255);
			uint32_t key = keys[index];
			if (present[index]) {
				int64_t at = fa_table_remove(table, key32_hash(&key), &key, 0, entry_size, 4);
				CHECK(at >= 0);
				TestKey32Entry *entry = (TestKey32Entry *)fa_table_entry(table, at, entry_size);
				CHECK(entry->key == key && entry->value == (key ^ 0xa5a5a5a5u));
				fa_table_release(table, at, entry_size);
			} else {
				key32_insert(table, key, entry_size, round % 2);
			}
			present[index] = !present[index];
			if (round % 37 == 0) {
				key32_check(table, keys, present, 256, entry_size);
			}
			if (round == 1000) {
				fa_table_reserve(table, 1000, entry_size, key32_hash);
				key32_check(table, keys, present, 256, entry_size);
			}
			if (round % 401 == 0) {
				FaTable *copy = fa_table_copy(table, entry_size);
				key32_check(copy, keys, present, 256, entry_size);
				fa_table_clear(copy, entry_size);
				CHECK(copy->length == 0 && key32_find(copy, 0, entry_size) < 0);
				key32_insert(copy, 0, entry_size, 1);
				CHECK(key32_find(copy, 0, entry_size) >= 0);
				key32_check(table, keys, present, 256, entry_size);
				fa_table_free(copy, entry_size);
			}
			if (round == 2000) {
				fa_table_clear(table, entry_size);
				__builtin_memset(present, 0, sizeof present);
			}
		}
		key32_check(table, keys, present, 256, entry_size);
		fa_table_free(table, entry_size);

		// A run wraps from the final slot to the start. Delete its middle,
		// then its beginning and end; every surviving key must remain found.
		table = fa_table_new(32, entry_size);
		uint32_t collision = 1;
		uint32_t colliding[24];
		uint8_t kept[24];
		for (int index = 0; index < 24; index += 1) {
			while ((inline_key_hash(collision) & 63) != 63) { collision += 1; }
			colliding[index] = collision++;
			kept[index] = 1;
			key32_insert(table, colliding[index], entry_size, 1);
		}
		CHECK(key32_find(table, colliding[0], entry_size) == 63);
		CHECK(key32_find(table, colliding[1], entry_size) == 0);
		const int removed[] = {12, 0, 23, 1, 22, 11, 13};
		for (int index = 0; index < (int)(sizeof removed / sizeof removed[0]); index += 1) {
			uint32_t key = colliding[removed[index]];
			int64_t at = key32_find(table, key, entry_size);
			CHECK(at >= 0);
			fa_table_kill(table, key32_hash(&key), at, entry_size);
			kept[removed[index]] = 0;
			key32_check(table, colliding, kept, 24, entry_size);
		}
		fa_table_free(table, entry_size);
		CHECK(allocated == before);
	}
}

// Wide values are aligned even behind the hash tags of 8/16-bit keys.
static uint64_t scalar_table_hash(uint64_t key, int bits) {
	if (bits == 64) { return inline_key64_hash(key); }
	if (bits == 32) { return inline_key_hash((uint32_t)key); }
	uint64_t product = key * 0x9E3779B97F4A7C15ull;
	return product ^ (product >> 32);
}

static int64_t scalar_table_find(FaTable *table, uint64_t key, int bits, int64_t size) {
	uint64_t storage[3] = {key, 0, 0};
	return fa_table_find(table, scalar_table_hash(key, bits), storage, 0, size, bits / 8);
}

static void scalar_table_insert(FaTable *table, uint64_t key, int bits, int64_t size, int hint) {
	int64_t position = scalar_table_find(table, key, bits, size);
	CHECK(position < 0);
	uint8_t *entry = fa_table_append(table, scalar_table_hash(key, bits), size,
	                                 hint ? position : 0, key, 0);
	CHECK(((uintptr_t)entry & 7) == 0);
	__builtin_memcpy(entry, &key, (size_t)(bits / 8));
	uint64_t value = key ^ 0xa5a5f0f0f00fa5a5ull;
	__builtin_memcpy(entry + 8, &value, 8);
}

static void scalar_table_check(FaTable *table, const uint64_t *keys, const uint8_t *present,
                               int count, int bits, int64_t size) {
	int64_t length = 0;
	for (int index = 0; index < count; index += 1) {
		int64_t position = scalar_table_find(table, keys[index], bits, size);
		CHECK((position >= 0) == (present[index] != 0));
		if (position >= 0) {
			uint8_t *entry = fa_table_entry(table, position, size);
			CHECK(((uintptr_t)entry & 7) == 0);
			uint64_t key = 0, value;
			__builtin_memcpy(&key, entry, (size_t)(bits / 8));
			__builtin_memcpy(&value, entry + 8, 8);
			CHECK(key == keys[index] && value == (key ^ 0xa5a5f0f0f00fa5a5ull));
			length += 1;
		}
	}
	CHECK(table->length == length);
	int64_t live = 0;
	for (int64_t position = 0; position < table->used; position += 1) {
		live += fa_table_live(table, position, size);
	}
	CHECK(live == length);
}

static void test_inline_scalar64(void) {
	long before = allocated;
	const int widths[] = {8, 16, 32, 64};
	for (int mode = 0; mode < 4; mode += 1) {
		int bits = widths[mode];
		int64_t size = 16 | TABLE_INLINE | (bits == 32 ? TABLE_INLINE_KEY32 :
		                                    bits == 64 ? TABLE_INLINE_KEY64 : 0);
		CHECK(inline_offset(size) == (bits < 32 ? 8 : 0));
		CHECK(inline_stride(size) == (bits < 32 ? 24 : 16));
		for (int local = 0; local < 2; local += 1) {
			FaTable header;
			FaTable *table = local ? fa_table_new_local(&header, 4, size) : fa_table_new(4, size);
			uint64_t keys[128];
			uint8_t present[128] = {0};
			uint64_t maximum = bits == 64 ? UINT64_MAX : ((uint64_t)1 << bits) - 1;
			for (int index = 0; index < 128; index += 1) {
				keys[index] = index == 0 ? 0 : index == 1 ? maximum :
				              index == 2 ? maximum / 2 + 1 : index == 3 ? maximum / 2 :
				              bits == 64 ? ((uint64_t)(index - 3) << 40) | (uint64_t)(index - 3) :
				              (uint64_t)(index - 3);
			}
			uint32_t random = 98765;
			for (int round = 0; round < 4000; round += 1) {
				random ^= random << 13; random ^= random >> 17; random ^= random << 5;
				int index = (int)(random & 127);
				uint64_t key = keys[index];
				if (present[index]) {
					uint64_t storage[3] = {key, 0, 0};
					int64_t position = fa_table_remove(table, scalar_table_hash(key, bits), storage,
					                                   0, size, bits / 8);
					CHECK(position >= 0);
					fa_table_release(table, position, size);
				} else { scalar_table_insert(table, key, bits, size, round & 1); }
				present[index] = !present[index];
				if (round % 37 == 0) { scalar_table_check(table, keys, present, 128, bits, size); }
				if (round % 401 == 0) {
					FaTable *copy = fa_table_copy(table, size);
					CHECK(copy->storage == 0);
					scalar_table_check(copy, keys, present, 128, bits, size);
					fa_table_clear(copy, size);
					CHECK(copy->length == 0 && scalar_table_find(copy, maximum, bits, size) < 0);
					fa_table_free(copy, size);
				}
				if (round == 1000) { fa_table_reserve(table, 1000, size, 0); }
				if (round == 2000) {
					fa_table_clear(table, size);
					__builtin_memset(present, 0, sizeof present);
				}
			}
			scalar_table_check(table, keys, present, 128, bits, size);
			fa_table_free(table, size);
			CHECK(allocated == before);
		}
	}
	// A 64-bit run crosses slot 63 -> 0; shifting must recompute full-width homes.
	int64_t size = 16 | TABLE_INLINE | TABLE_INLINE_KEY64;
	FaTable *table = fa_table_new(32, size);
	uint64_t keys[24], next = 1ull << 48;
	uint8_t present[24];
	for (int index = 0; index < 24; index += 1) {
		while ((scalar_table_hash(next, 64) & 63) != 63) { next += 1; }
		keys[index] = next++; present[index] = 1;
		scalar_table_insert(table, keys[index], 64, size, 1);
	}
	CHECK(scalar_table_find(table, keys[0], 64, size) == 63);
	CHECK(scalar_table_find(table, keys[1], 64, size) == 0);
	for (int index = 0; index < 24; index += 1) {
		int removed = (index * 7) % 24;
		int64_t position = scalar_table_find(table, keys[removed], 64, size);
		CHECK(position >= 0);
		fa_table_kill(table, scalar_table_hash(keys[removed], 64), position, size);
		present[removed] = 0;
		scalar_table_check(table, keys, present, 24, 64, size);
	}
	fa_table_free(table, size);
	CHECK(allocated == before);
}

static void test_negative_probe_deletions(void) {
	long before = allocated;
	for (int compact = 0; compact < 2; compact += 1) {
		int64_t entry_size = sizeof(TestKey32Entry) | TABLE_INLINE |
		                     (compact ? TABLE_INLINE_KEY32 : 0);
		for (int with_zero = 0; with_zero < 2; with_zero += 1) {
			FaTable *base = fa_table_new(32, entry_size);
			uint32_t keys[24];
			uint32_t next = 1;
			for (int index = 0; index < 24; index += 1) {
				while ((inline_key_hash(next) & 63) != 63) { next += 1; }
				keys[index] = next++;
				key32_insert(base, keys[index], entry_size, 1);
			}
			if (with_zero) {
				key32_insert(base, 0, entry_size, 1);
				key32_insert(base, UINT32_MAX, entry_size, 1);
			}
			for (uint32_t query = 0; query < 258; query += 1) {
				uint32_t missing = query == 256 ? 0xffffffffu :
				                   query == 257 ? 0x80000000u : query;
				if (key32_find(base, missing, entry_size) >= 0) { continue; }
				FaTable *copy = fa_table_copy(base, entry_size);
				int64_t hint = key32_find(copy, missing, entry_size);
				for (int index = 0; index < 24 + 2 * with_zero; index += 1) {
					uint32_t removed = index == 24 ? 0 : index == 25 ? UINT32_MAX : keys[(index * 7) % 24];
					int64_t position = key32_find(copy, removed, entry_size);
					CHECK(position >= 0);
					hint = fa_table_release_probe(copy, position, entry_size,
					                              hint, key32_hash(&missing));
					CHECK(hint == key32_find(copy, missing, entry_size));
				}
				TestKey32Entry *entry = fa_table_append(copy, key32_hash(&missing),
				                                      entry_size, hint, missing, key32_hash);
				entry->key = missing;
				entry->value = 42;
				CHECK(key32_find(copy, missing, entry_size) >= 0);
				fa_table_free(copy, entry_size);
			}
			fa_table_free(base, entry_size);
		}
	}
	CHECK(allocated == before);
}

static void test_local_map_headers(void) {
	long before = allocated;
	int64_t entry_size = sizeof(TestKey32Entry) | TABLE_INLINE | TABLE_INLINE_KEY32;
	FaTable local;
	for (int round = 0; round < 64; round += 1) {
		FaTable *table = fa_table_new_local(&local, 4, entry_size);
		CHECK(table == &local && table->storage == 1);
		key32_insert(table, 0, entry_size, 1);
		key32_insert(table, UINT32_MAX, entry_size, 1);
		for (uint32_t key = 1; key < 128; key += 1) { key32_insert(table, key, entry_size, 1); }
		fa_table_reserve(table, 1000, entry_size, key32_hash);
		CHECK(table->storage == 1 && key32_find(table, UINT32_MAX, entry_size) >= 0);
		FaTable *copy = fa_table_copy(table, entry_size);
		CHECK(copy->storage == 0 && key32_find(copy, 0, entry_size) >= 0);
		fa_table_free(copy, entry_size);
		fa_table_clear(table, entry_size);
		CHECK(table->storage == 1 && table->length == 0);
		key32_insert(table, 42, entry_size, 1);
		CHECK(key32_find(table, 42, entry_size) >= 0);
		fa_table_free(table, entry_size);
		CHECK(allocated == before);
	}
}

static void test_inline_loop_reservations(void) {
	long before = allocated;
	for (size_t prefix = 0; prefix <= 5; prefix += 1) {
		for (size_t rounds = 0; rounds <= 80; rounds += 1) {
			FaString text = {(const uint8_t *)"start", prefix, SIMPLE};
			fa_string_reserve_inline(&text, rounds);
			if (prefix + rounds <= INLINE_STRING_BYTES) {
				CHECK(allocated == before);
				CHECK(rounds == 0 || (text.capacity & STRING_INLINE));
			}
			for (size_t round = 0; round < rounds; round += 1) {
				fa_string_append(&text, (const uint8_t *)"x", 1, SIMPLE);
			}
			CHECK(string_length(&text) == prefix + rounds);
			CHECK(__builtin_memcmp(string_bytes(&text), "start", prefix) == 0);
			for (size_t at = prefix; at < prefix + rounds; at += 1) {
				CHECK(string_bytes(&text)[at] == 'x');
			}
			if (string_owned(text.capacity)) {
				fa_string_free(text.bytes, text.capacity);
			}
			CHECK(allocated == before);
		}
	}
	FaString reserved = {(const uint8_t *)"short", 5, SIMPLE};
	fa_string_reserve_owned(&reserved, 128);
	const uint8_t *buffer = reserved.bytes;
	size_t capacity = reserved.capacity;
	fa_string_reserve_inline(&reserved, 0);
	CHECK(reserved.bytes == buffer && reserved.capacity == capacity);
	fa_string_reserve_inline(&reserved, 5);
	CHECK(reserved.bytes == buffer && reserved.capacity == capacity);
	fa_string_free(reserved.bytes, reserved.capacity);
	CHECK(allocated == before);
	// A two-byte character exceeds the one-byte reservation. Normal append
	// grows safely, then the remaining rounds are reserved once more.
	FaString text = {(const uint8_t *)"", 0, SIMPLE};
	fa_string_reserve_inline(&text, 20);
	for (size_t round = 0; round < 20; round += 1) {
		fa_string_append(&text, (const uint8_t *)"é", 2, 0);
		fa_string_reserve_inline(&text, 19 - round);
	}
	CHECK(string_length(&text) == 40 && !(text.capacity & SIMPLE));
	for (size_t at = 0; at < 40; at += 2) {
		CHECK(string_bytes(&text)[at] == 0xc3 && string_bytes(&text)[at + 1] == 0xa9);
	}
	CHECK(string_owned(text.capacity));
	fa_string_free(text.bytes, text.capacity);
	CHECK(allocated == before);
}

static void test_frame_strings(void) {
	long before = allocated;
	uint8_t first[64];
	uint8_t second[64];
	uint8_t original_bytes[100];
	for (size_t at = 0; at < sizeof original_bytes; at += 1) { original_bytes[at] = (uint8_t)('a' + at % 26); }
	FaString original = {original_bytes, 40, SIMPLE};
	FaString text = {0};
	FaString copy = {0};
	fa_string_assign_local(&original, first, &text, 1, 0);
	CHECK(allocated == before && string_frame(text.capacity) && text.bytes == first);
	CHECK(string_length(&text) == 40 && __builtin_memcmp(first, original_bytes, 40) == 0);
	CHECK(fa_string_room(text.length, text.capacity) == 64);
	fa_string_assign_local(&text, second, &copy, 0, 1);
	CHECK(allocated == before && string_frame(copy.capacity) && copy.bytes == second);
	__builtin_memset(first, 'x', 40);
	CHECK(__builtin_memcmp(second, original_bytes, 40) == 0);
	for (size_t length = 40; length < 80; length += 1) {
		fa_string_append(&text, (const uint8_t *)"x", 1, SIMPLE);
		CHECK(string_length(&text) == length + 1);
		if (length < 64) { CHECK(allocated == before && text.bytes == first); }
	}
	CHECK(string_owned(text.capacity) && allocated == before + 1);
	// A borrowed self-assignment copies before freeing the previous heap block.
	fa_string_assign_local(&text, first, &text, 1, 0);
	CHECK(string_length(&text) == 80 && string_owned(text.capacity) && allocated == before + 1);
	// An owned self-assignment transfers the existing block exactly once.
	const uint8_t *heap = text.bytes;
	fa_string_assign_local(&text, first, &text, 1, 1);
	CHECK(text.bytes == heap && allocated == before + 1);
	FaString slice = {text.bytes + 1, 60, SIMPLE | KIND_MASK};
	fa_string_assign_local(&slice, first, &text, 1, 0);
	CHECK(string_frame(text.capacity) && text.bytes == first && string_length(&text) == 60);
	CHECK(allocated == before);
	slice = (FaString){text.bytes + 1, 40, SIMPLE | KIND_MASK};
	fa_string_assign_local(&slice, first, &text, 1, 0);
	CHECK(string_length(&text) == 40 && string_frame(text.capacity));
	FaString consumed = owned_copy_with(original_bytes, 40, SIMPLE);
	CHECK(allocated == before + 1);
	fa_string_assign_local(&consumed, first, &text, 1, 1);
	CHECK(allocated == before && __builtin_memcmp(first, original_bytes, 40) == 0);
	fa_string_reserve_inline(&text, 1);
	CHECK(text.bytes == first && string_frame(text.capacity) && allocated == before);
	fa_string_plus(&text, (const uint8_t *)"é", 2, 0);
	CHECK(text.bytes == first && string_length(&text) == 42 && !(text.capacity & SIMPLE));
	CHECK(allocated == before);
	original.length = 10;
	fa_string_assign_local(&original, second, &copy, 0, 0);
	CHECK(copy.capacity & STRING_INLINE);
	CHECK(string_length(&copy) == 10 && allocated == before);
	original.length = 80;
	fa_string_assign_local(&original, second, &copy, 0, 0);
	CHECK(string_owned(copy.capacity) && string_length(&copy) == 80);
	fa_string_free(copy.bytes, copy.capacity);
	CHECK(allocated == before);
	// Taking a frame value into an unknown owner detaches its bytes.
	FaString detached;
	fa_string_escape(&text, &detached);
	CHECK(string_owned(detached.capacity) && !string_frame(detached.capacity));
	CHECK(string_length(&detached) == 42 && allocated == before + 1);
	__builtin_memset(first, '?', 40);
	CHECK(__builtin_memcmp(string_bytes(&detached), original_bytes, 40) == 0);
	fa_string_free(detached.bytes, detached.capacity);
	// Mutating through a borrowed frame header also retains its buffer.
	FaString incoming = owned_copy_with(original_bytes, 40, SIMPLE);
	fa_string_assign_after_drop(&incoming, &text);
	CHECK(string_frame(text.capacity) && text.bytes == first && allocated == before);
	CHECK(__builtin_memcmp(first, original_bytes, 40) == 0);
	// Non-simple short appends retain a mutable's frame buffer too.
	original.length = 0;
	fa_string_assign_local(&original, first, &text, 1, 0);
	fa_string_append(&text, (const uint8_t *)"é", 2, 0);
	CHECK(string_frame(text.capacity) && text.bytes == first && string_length(&text) == 2);
	CHECK(allocated == before);
	// An explicit reservation above 64 must survive moving its value.
	FaString reserved;
	fa_string_from(original_bytes, 40, SIMPLE, 128, &reserved);
	const uint8_t *reserved_buffer = reserved.bytes;
	fa_string_assign_local(&reserved, first, &text, 1, 1);
	CHECK(text.bytes == reserved_buffer && fa_string_room(text.length, text.capacity) >= 128);
	CHECK(allocated == before + 1);
	// A short Unicode append must not throw away an explicit reservation.
	fa_string_replace_buffer(0, SIMPLE, &text);
	fa_string_append(&text, (const uint8_t *)"é\r\n", 4, 0);
	CHECK(text.bytes == reserved_buffer && fa_string_room(text.length, text.capacity) >= 128);
	CHECK(string_length(&text) == 4 && allocated == before + 1);
	fa_string_free(text.bytes, text.capacity);
	CHECK(allocated == before);
}

// ── workers: calls on a worker's thread, blocking, detached, kept and
// cancelled at random (queued, running or done), freed once

static int64_t worker_results;

// the frame of a call: a task's layout, its result an `int64_t` at 48
static void call_resume(void *frame) {
	fa_call_resume(frame);
}

static void call_destroy(void *frame) {
	fa_call_destroy(frame);
}

static void call_dispose(void *frame) {
	fa_call_free(frame);
}

static void call_run(void *state, void *arguments, void *frame) {
	int64_t *count = state;
	*count += 1;
	int64_t value = *(int64_t *)arguments;
	// a little work, so that calls are queued while others run
	for (volatile int spin = 0; spin < 2000; spin += 1) {
	}
	*(int64_t *)((uint8_t *)frame + TASK_AT + sizeof(FaTask)) = value * 2;
	fa_free(arguments, sizeof(int64_t), 16);
}

static void call_drop_arguments(void *arguments) {
	fa_free(arguments, sizeof(int64_t), 16);
}

static void *new_call(int64_t value, void **arguments) {
	void *frame = fa_call_new(TASK_AT + sizeof(FaTask) + 16);
	((void **)frame)[0] = (void *)call_resume;
	((void **)frame)[1] = (void *)call_destroy;
	int64_t *argument = fa_alloc(sizeof(int64_t), 16);
	*argument = value;
	*arguments = argument;
	return frame;
}

static void post(FaWorker *worker, int64_t value, int32_t mode, void **frame) {
	void *arguments;
	*frame = new_call(value, &arguments);
	fa_worker_call(worker, *frame, call_run, arguments, call_drop_arguments, call_dispose, mode);
}

static FaWorker *test_worker;
static void *kept[300];
static uint64_t random_state = 12345;

static uint64_t next_random(void) {
	random_state = random_state * 6364136223846793005u + 1442695040888963407u;
	return random_state >> 33;
}

static void worker_root(Frame *frame) {
	// detached calls: the program waits for them
	for (int index = 0; index < 50; index += 1) {
		void *call;
		post(test_worker, index, 2, &call);
	}
	// kept calls, half of them cancelled at once
	for (int index = 0; index < 300; index += 1) {
		post(test_worker, index, 1, &kept[index]);
		if (next_random() % 2 == 0) {
			fa_task_drop(kept[index]);
			kept[index] = 0;
		}
	}
	finish(frame);
}

static void test_workers(void) {
	test_worker = fa_worker_new(sizeof(int64_t), 0);
	*(int64_t *)((uint8_t *)test_worker + WORKER_STATE) = 0;
	fa_worker_start(test_worker);
	// blocking calls
	for (int index = 0; index < 100; index += 1) {
		void *frame;
		post(test_worker, index, 0, &frame);
		worker_results += *(int64_t *)((uint8_t *)frame + TASK_AT + sizeof(FaTask));
		fa_destroy(frame);
	}
	CHECK(worker_results == 9900);
	Frame *root = new_frame(worker_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
	// the kept calls not cancelled: done (the program waited for their
	// wakes), dropped before or after the worker stops
	for (int index = 0; index < 300; index += 1) {
		if (kept[index] != 0 && next_random() % 2 == 0) {
			fa_task_drop(kept[index]);
			kept[index] = 0;
		}
	}
	fa_shared_release(test_worker);
	for (int index = 0; index < 300; index += 1) {
		if (kept[index] != 0) {
			fa_task_drop(kept[index]);
		}
	}
	CHECK(__atomic_load_n(&workers_alive, __ATOMIC_ACQUIRE) == 0);
}

// ── wakes before yields: the woken queue first, at most half of a tick
// when both queues hold tasks (design/decisions.md)

static void queues_root(Frame *frame) {
	FaExecutor *e = executor();
	Frame *tasks[45];
	for (int index = 0; index < 45; index += 1) {
		tasks[index] = new_frame(finish, index);
	}
	// two yielded tasks, then a woken one: the woken one first
	yield_push(e, &tasks[0]->task);
	yield_push(e, &tasks[1]->task);
	ready_push(e, &tasks[2]->task);
	int64_t limit;
	int64_t woken_turns = 0;
	CHECK(tick_turns(e, &limit) == 3);
	CHECK(ready_pop(e, &woken_turns, limit) == &tasks[2]->task);
	CHECK(ready_pop(e, &woken_turns, limit) == &tasks[0]->task);
	CHECK(ready_pop(e, &woken_turns, limit) == &tasks[1]->task);
	CHECK(e->ready == 0 && e->woken == 0);
	// 40 woken tasks and 3 yielded ones: the woken take 30 turns, then the
	// yielded ones run, then the rest of the woken ones
	for (int index = 0; index < 3; index += 1) {
		yield_push(e, &tasks[40 + index]->task);
	}
	for (int index = 0; index < 40; index += 1) {
		ready_push(e, &tasks[index]->task);
	}
	// a queued task taken out (a drop) leaves its queue
	ready_unlink(e, &tasks[41]->task);
	ready_unlink(e, &tasks[5]->task);
	woken_turns = 0;
	int64_t count = tick_turns(e, &limit);
	CHECK(count == 41 && limit == EVENT_INTERVAL / 2);
	int order[41];
	for (int64_t index = 0; index < count; index += 1) {
		FaTask *task = ready_pop(e, &woken_turns, limit);
		order[index] = ((Frame *)frame_of(task))->id;
	}
	CHECK(order[29] == 30 && order[30] == 40 && order[31] == 42 && order[32] == 31);
	CHECK(e->ready == 0 && e->head == 0 && e->yield_head == 0);
	for (int index = 0; index < 45; index += 1) {
		free_frame(tasks[index]);
	}
	finish(frame);
}

static void test_queues(void) {
	Frame *root = new_frame(queues_root, 0);
	root->resume(root);
	fa_run_main(root, 0);
}

static void test_collection_destinations(void) {
	long before = allocated;
	_Alignas(16) uint8_t small[128];
	FaArrayHeader *fresh = fa_array_new_into(0, 12, sizeof(int64_t), small, sizeof(small));
	CHECK(fresh == (FaArrayHeader *)small && fa_array_storage(fresh) == 3);
	CHECK(fresh->capacity == 12 && allocated == before);
	fa_array_free(fresh, sizeof(int64_t));
	fresh = fa_array_new_into(0, 13, sizeof(int64_t), small, sizeof(small));
	CHECK(fresh != (FaArrayHeader *)small && fa_array_storage(fresh) == 0);
	fa_array_free(fresh, sizeof(int64_t));
	CHECK(allocated == before);
	FaArrayHeader *parts = fa_string_split_into((const uint8_t *)"a bb", 4, SIMPLE,
	                                          (const uint8_t *)" ", 1, small, sizeof(small));
	CHECK(parts == (FaArrayHeader *)small && parts->length == 2);
	CHECK(fa_array_storage(parts) == 3 && allocated == before);
	CHECK(string_length(&((FaString *)parts->items)[1]) == 2);
	fa_array_free(parts, sizeof(FaString));
	CHECK(allocated == before);
	parts = fa_string_split_into((const uint8_t *)"a b c d e f g h", 15, SIMPLE,
	                             (const uint8_t *)" ", 1, small, sizeof(small));
	CHECK(parts != (FaArrayHeader *)small && parts->length == 8);
	CHECK(fa_array_storage(parts) == 0);
	fa_array_free(parts, sizeof(FaString));
	CHECK(allocated == before);
	_Alignas(16) uint8_t storage[2048];
	FaSegmented *returned = fa_segmented_new_into(3, sizeof(int64_t), 3, storage, sizeof(storage));
	CHECK(returned == (FaSegmented *)storage && returned->length == 3);
	CHECK(returned->storage == 3 && allocated == before);
	fa_segmented_free(returned, sizeof(int64_t), 3);
	returned = fa_segmented_new_into(20, sizeof(int64_t), 3, small, sizeof(small));
	CHECK(returned != (FaSegmented *)small && returned->length == 20);
	CHECK(returned->storage == 0 && returned->segments == 1);
	fa_segmented_free(returned, sizeof(int64_t), 3);
	FaPool *returned_pool = fa_pool_new_into(3, sizeof(int64_t), storage, sizeof(storage), sizeof(storage));
	CHECK(returned_pool == (FaPool *)storage && returned_pool->size == 3);
	fa_pool_free(returned_pool);
	CHECK(allocated == before);
	FaSegmented *growing = fa_segmented_local(storage, 20, sizeof(int64_t), 3);
	for (int64_t i = 0; i < 20; i += 1) {
		*(int64_t *)fa_segmented_at(growing, i, sizeof(int64_t), 3) = i;
	}
	CHECK(*(int64_t *)fa_segmented_at(growing, 19, sizeof(int64_t), 3) == 19);
	void *frame_first = fa_segmented_at(growing, 0, sizeof(int64_t), 3);
	fa_segmented_reserve(growing, 40000, sizeof(int64_t), 3);
	CHECK(frame_first == fa_segmented_at(growing, 0, sizeof(int64_t), 3));
	CHECK(*(int64_t *)fa_segmented_at(growing, 19, sizeof(int64_t), 3) == 19);
	fa_segmented_reset(growing, sizeof(int64_t), 3);
	fa_segmented_reserve(growing, 512, sizeof(int64_t), 3);
	fa_segmented_free(growing, sizeof(int64_t), 3);
	growing = fa_segmented_local(storage, 0, sizeof(int64_t), 3);
	fa_segmented_reserve(growing, 100, sizeof(int64_t), 3);
	fa_segmented_free(growing, sizeof(int64_t), 3);
	CHECK(allocated == before);
	FaPool *pool = fa_pool_local(storage, 2, sizeof(int64_t));
	*(int64_t *)fa_pool_at(pool, 0) = 42;
	CHECK(fa_pool_seek(pool, 0) == 0 && *(int64_t *)fa_pool_at(pool, 0) == 42);
	fa_pool_free(pool);
	// a table in a frame: chunks of entries, dense entries, entries in the
	// probe array; its entries are added without a block of the heap
	for (int layout = 0; layout < 3; layout += 1) {
		int64_t entry_size = layout == 0 ? (int64_t)sizeof(TestTableEntry)
		                     : layout == 1 ? (int64_t)sizeof(TestTableEntry) | TABLE_DENSE
		                                   : (int64_t)sizeof(TestKey32Entry) | TABLE_INLINE | TABLE_INLINE_KEY32;
		FaTable *table = fa_table_local(storage, 4, entry_size);
		CHECK(table->storage == 3 && table->length == 0);
		for (int64_t key = 0; key < 4; key += 1) {
			if (layout == 2) {
				uint32_t narrow = (uint32_t)key;
				TestKey32Entry *entry = fa_table_append(table, inline_key_hash(narrow), entry_size, 0, narrow, 0);
				entry->key = narrow;
				entry->value = narrow + 10;
			} else {
				TestTableEntry *entry = fa_table_append(table, table_test_hash(&key), entry_size, 0, 0, table_test_hash);
				entry->key = key;
				entry->value = key + 10;
			}
		}
		CHECK(table->length == 4);
		for (int64_t key = 0; key < 5; key += 1) {
			uint32_t narrow = (uint32_t)key;
			int64_t position = layout == 2
				? fa_table_find(table, inline_key_hash(narrow), &narrow, 0, entry_size, 4)
				: fa_table_find(table, table_test_hash(&key), &key, table_test_equals, entry_size, 0);
			CHECK((position >= 0) == (key < 4));
			if (position >= 0 && layout == 2) {
				CHECK(((TestKey32Entry *)fa_table_entry(table, position, entry_size))->value == narrow + 10);
			} else if (position >= 0) {
				CHECK(((TestTableEntry *)fa_table_entry(table, position, entry_size))->value == key + 10);
			}
		}
		// a copy is an ordinary table of the heap
		FaTable *copy = fa_table_copy(table, entry_size);
		CHECK(copy->storage == 0 && copy->length == 4 && allocated > before);
		fa_table_free(copy, entry_size);
		fa_table_free(table, entry_size);
		CHECK(allocated == before);
	}
}

// Storage hints must allocate ahead, preserve addresses, and free every block.
static void test_storage_hints(void) {
    long before = allocated;
    CHECK(sizeof(FaSegmented) <= 64);
    FaSegmented *a = fa_segmented_new();
    FaSegmented *b = fa_segmented_new();
    fa_segmented_reserve(a, 1000, 1, 6);
    fa_segmented_reserve(b, 17, 8, 3);
    CHECK(a->k == 10 && b->k == 5);
    CHECK(fa_segmented_capacity(a) == 1024 && fa_segmented_capacity(b) == 32);
    long reserved = allocated;
    for (int64_t i = 0; i < 1000; i += 1) { *(uint8_t *)fa_segmented_push_slot(a, 1, 6) = (uint8_t)i; }
    for (int64_t i = 0; i < 17; i += 1) { *(int64_t *)fa_segmented_push_slot(b, 8, 3) = i; }
    CHECK(allocated == reserved);
    uint8_t *first = fa_segmented_at(a, 0, 1, 6);
    fa_segmented_reserve(a, 10000, 1, 6);
    reserved = allocated;
    for (int64_t i = 1000; i < 11000; i += 1) { *(uint8_t *)fa_segmented_push_slot(a, 1, 6) = (uint8_t)i; }
    CHECK(allocated == reserved && first == fa_segmented_at(a, 0, 1, 6));
    for (int64_t i = 0; i < 11000; i += 1) { CHECK(*(uint8_t *)fa_segmented_at(a, i, 1, 6) == (uint8_t)i); }
    CHECK(*(int64_t *)fa_segmented_at(b, 16, 8, 3) == 16);
    fa_segmented_reset(a, 1, 6);
    CHECK(fa_segmented_capacity(a) >= 11000);
    fa_segmented_free(a, 1, 6);
    fa_segmented_free(b, 8, 3);

    // The first three pointers remain inline and are mirrored in the heap
    // directory. Eleven slots cover segments 0..10 before its one reallocation.
    long bytes_before = allocated_bytes;
    FaSegmented *directory = fa_segmented_new();
    int64_t *kept = 0;
    uintptr_t *small = 0;
    for (int64_t i = 0; i <= 8192; i += 1) {
        int64_t *slot = fa_segmented_push_slot(directory, sizeof(int64_t), 3);
        *slot = i;
        if (i == 0) { kept = slot; }
        if (i == 32) {
            CHECK(directory->segments == 4);
            small = directory->segment;
            CHECK(small != directory->inline_segment);
            for (int slot = 0; slot < FA_INLINE_SEGMENTS; slot += 1) {
                CHECK(directory->segment[slot] == directory->inline_segment[slot]);
            }
            CHECK(allocated_bytes - bytes_before == (long)(sizeof(FaSegmented) + 64 * sizeof(int64_t) + FA_DIRECTORY_INITIAL * sizeof(void *)));
        }
        if (i == 8191) { CHECK(directory->segments == 11 && directory->segment == small); }
    }
    CHECK(directory->segments == 12 && directory->segment != small);
    CHECK(kept == fa_segmented_at(directory, 0, sizeof(int64_t), 3) && *kept == 0);
    CHECK(allocated_bytes - bytes_before == (long)(sizeof(FaSegmented) + 16384 * sizeof(int64_t) + FA_SEGMENTS * sizeof(void *)));
    for (int64_t i = 0; i <= 8192; i += 1) {
        CHECK(*(int64_t *)fa_segmented_at(directory, i, sizeof(int64_t), 3) == i);
    }
    // Rebuilding an empty reservation releases its old directory, so sized
    // deallocation never has to infer a directory size from a reset count.
    fa_segmented_reset(directory, sizeof(int64_t), 3);
    fa_segmented_reserve(directory, 20000, sizeof(int64_t), 3);
    CHECK(directory->segments == 1 && directory->segment == directory->inline_segment);
    fa_segmented_free(directory, sizeof(int64_t), 3);
    CHECK(allocated_bytes == bytes_before);
    directory = fa_segmented_new();
    for (int64_t i = 0; i < 40; i += 1) { *(int64_t *)fa_segmented_push_slot(directory, sizeof(int64_t), 3) = i; }
    fa_segmented_free(directory, sizeof(int64_t), 3);
    CHECK(allocated_bytes == bytes_before); // free the eleven-entry directory too
    for (int k = 0; k <= 6; k += 1) {
        FaSegmented *formula = fa_segmented_new();
        fa_segmented_reserve(formula, (int64_t)1 << k, sizeof(int64_t), 3);
        for (int64_t i = 0; i < 5000; i += 1) {
            *(int64_t *)fa_segmented_push_slot(formula, sizeof(int64_t), 3) = i;
        }
        for (int64_t i = 0; i < 5000; i += 1) {
            CHECK(*(int64_t *)fa_segmented_at(formula, i, sizeof(int64_t), 3) == i);
        }
        fa_segmented_free(formula, sizeof(int64_t), 3);
    }
    CHECK(allocated_bytes == bytes_before);
    // chunks of entries, then dense entries: a reservation allocates the
    // index and the entries, before and after deletions
    for (int dense = 0; dense < 2; dense += 1) {
        int64_t entry_size = (int64_t)sizeof(TestTableEntry) | (dense ? TABLE_DENSE : 0);
        FaTable *table = fa_table_new(4, entry_size);
        fa_table_reserve(table, 100, entry_size, table_test_hash);
        CHECK(fa_table_capacity(table) >= 100);
        reserved = allocated;
        for (int64_t i = 0; i < 100; i += 1) {
            TestTableEntry *entry = fa_table_append(table, table_test_hash(&i), entry_size, 0, 0, table_test_hash);
            entry->key = i;
            // without deletions, the positions follow the insertions
            CHECK(entry == (TestTableEntry *)fa_table_entry(table, i, entry_size));
        }
        CHECK(allocated == reserved);
        for (int64_t i = 0; i < 50; i += 1) { fa_table_kill(table, table_test_hash(&i), i, entry_size); }
        fa_table_reserve(table, 100, entry_size, table_test_hash);
        CHECK(fa_table_capacity(table) >= 150);
        reserved = allocated;
        for (int64_t i = 100; i < 200; i += 1) {
            TestTableEntry *entry = fa_table_append(table, table_test_hash(&i), entry_size, 0, 0, table_test_hash);
            entry->key = i;
        }
        CHECK(allocated == reserved && table->length == 150);
        for (int64_t i = 0; i < 200; i += 1) {
            CHECK((fa_table_find(table, table_test_hash(&i), &i, table_test_equals, entry_size, 0) >= 0) == (i >= 50));
        }
        fa_table_free(table, entry_size);
    }
    FaString string = owned_copy_with((const uint8_t *)"hello", 5, SIMPLE);
    fa_string_reserve(&string, 100);
    CHECK(fa_string_room(string.length, string.capacity) >= 105);
    reserved = allocated;
    fa_string_append(&string, (const uint8_t *)"!", 1, SIMPLE);
    CHECK(allocated == reserved && string_length(&string) == 6);
    size_t string_capacity = fa_string_room(string.length, string.capacity);
    fa_string_append(&string, (const uint8_t *)"é", 2, 0);
    CHECK(allocated == reserved && fa_string_room(string.length, string.capacity) == string_capacity);
    fa_string_free(string.bytes, string.capacity);
    CHECK(allocated == before);
}

static void test_grapheme_conformance(void) {
	for (size_t i = 0; i < sizeof FA_GRAPHEME_CASES / sizeof FA_GRAPHEME_CASES[0]; i += 1) {
		const GraphemeCase *test = &FA_GRAPHEME_CASES[i];
		size_t position = 0;
		for (size_t cluster = 0; cluster < test->count; cluster += 1) {
			position = next_boundary(test->bytes, test->length, position);
			if (position != test->ends[cluster]) {
				printf("grapheme case %zu, cluster %zu: expected %u, got %zu\n", i, cluster, test->ends[cluster], position);
				CHECK(position == test->ends[cluster]);
			}
		}
		CHECK(position == test->length);
		CHECK(fa_graphemes(test->bytes, test->length) == test->count);
	}
	FaArrayHeader *characters = characters_of((const uint8_t *)"a\r\nb", 4, SIMPLE);
	CHECK(characters->length == 3 && characters->capacity >= 0);
	FaString *items = (FaString *)characters->items;
	CHECK(string_length(&items[1]) == 2);
	for (int64_t i = 0; i < characters->length; i += 1) {
		if (string_owned(items[i].capacity)) { fa_string_free(items[i].bytes, items[i].capacity); }
	}
	fa_array_free(characters, sizeof(FaString));
}

static void test_array_view_snapshots(void) {
	long before = allocated_bytes;
	FaArrayHeader *owner = fa_array_new(3, 32, sizeof(int64_t));
	int64_t *items = (int64_t *)owner->items;
	items[0] = 1; items[1] = 2; items[2] = 3;
	FaArrayView current;
	CHECK(!fa_array_slice_view(owner, 0, 1, 3, sizeof(int64_t), &current));
	FaArrayHeader *place = &current.header;
	FaArrayReference kept;
	FaArrayHeader **saved = fa_array_reference(&place, &kept);
	CHECK(*saved != place && (*saved)->items == owner->items && (*saved)->length == 1);
	CHECK(fa_array_reference(&owner, &kept) == &owner);
	CHECK(!fa_array_slice_view(owner, 2, 3, 3, sizeof(int64_t), &current));
	CHECK(*(int64_t *)(*saved)->items == 1 && *(int64_t *)place->items == 3);
	const void *error = fa_array_slice_view(owner, 4, 5, 3, sizeof(int64_t), &current);
	CHECK(error != 0);
	fa_free((void *)error, sizeof(IndexError), sizeof(int64_t));
	// Bounds failures must leave the previous designation untouched.
	CHECK(*(int64_t *)place->items == 3 && place->length == 1);
	fa_array_free(owner, sizeof(int64_t));
	CHECK(allocated_bytes == before);
}

// The grammar's depth bound is independent of the C/wasm call stack. Check
// both the complete returned tree and frame cleanup on deep syntax failures.
static void test_json_depth(void) {
	fa_release_builder();
	long before = allocated;
	for (int mode = 0; mode < 3; mode++) {
		size_t depth = mode == 0 ? 999 : mode == 1 ? 1000 : 500;
		uint8_t text[2 * JSON_MAX_DEPTH + 64];
		const uint8_t prefix[] = "{\"data\":";
		size_t length = sizeof prefix - 1;
		copy_bytes(text, prefix, length);
		for (size_t i = 0; i < depth; i++) text[length++] = '[';
		text[length++] = '0';
		if (mode == 2) text[length++] = ',';
		for (size_t i = 0; i < depth; i++) text[length++] = ']';
		text[length++] = '}';
		FaJsonDocument *document = 0;
		MessageError *error = (MessageError *)fa_json_parse(text, length,
		    (const uint8_t *)"o0;", &document);
		if (mode == 0) {
			CHECK(error == 0 && document != 0);
			const FaJson *node = document->root.items;
			for (size_t i = 0; i < depth; i++) {
				CHECK(node->kind == JSON_ARRAY && node->count == 1);
				node = node->items;
			}
			CHECK(node->kind == JSON_NUMBER && node->integer == 0);
			fa_json_free(document);
		} else {
			CHECK(error != 0 && document == 0);
			const char *expected = mode == 1 ? "JSON Parse error: Exceeded the maximum depth" :
			    "JSON Parse error: Unexpected comma at the end of array expression";
			CHECK(fa_string_equal(string_bytes(&error->message), string_length(&error->message),
			    (const uint8_t *)expected, c_length(expected)));
			if (string_owned(error->message.capacity)) {
				fa_string_free(error->message.bytes, error->message.capacity);
			}
			fa_free(error, sizeof(MessageError), sizeof(int64_t));
		}
		fa_release_builder();
		CHECK(allocated == before);
	}
}

int main(void) {
	test_json_depth();
	test_grapheme_conformance();
	test_array_view_snapshots();
	test_storage_hints();
	test_collection_destinations();
	test_tables();
	test_inline_key32();
	test_inline_scalar64();
	test_negative_probe_deletions();
	test_local_map_headers();
	test_inline_loop_reservations();
	test_frame_strings();
	test_integer_strings();
	test_string_padding();
	test_utf16_strings();
	test_byte_blocks();
	test_utf8_byte_conversions();
	test_inline_strings();
	test_nonempty_splits();
	test_region_slice_views();
	test_tree_regions();
	test_utf8_input();
	test_unicode_transform_counts();
	test_canonical_split_views_without_edges();
	test_arguments();
	test_strict_file_text();
	test_character_views();
	test_allocator_fast_paths();
	test_short_copies();
	test_string_equal();
	test_string_keys();
	test_reserving_cut_string_views();
	printf("test_yields\n");
	test_yields();
	printf("test_timers\n");
	test_timers();
	printf("test_remote_wakes\n");
	test_remote_wakes();
	printf("test_cancel\n");
	test_cancel();
	printf("test_mutex\n");
	test_mutex();
	printf("test_join\n");
	test_join();
	printf("test_pool\n");
	test_pool();
	printf("test_io\n");
	test_io();
	printf("test_workers\n");
	test_workers();
	printf("test_queues\n");
	test_queues();
	fa_release_builder();
	CHECK(allocated == 0);
	printf("the event loop tests passed\n");
	return 0;
}
