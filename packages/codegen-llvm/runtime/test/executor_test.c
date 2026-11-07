// A test of the event loop of the runtime (the "tasks and the event loop"
// section of `fa_runtime.c`), built with the C library and AddressSanitizer
// by `runtime/test/run.sh` (and `tests/runtime.rs`): the coroutines are
// hand-made state machines with the layout of an LLVM frame (the resume and
// destroy functions, then the task header at 16 bytes), and the platform
// layer of `src/runtime.rs` is written here with the C library.
//
//   packages/codegen-llvm/runtime/test/run.sh

#include "../fa_runtime.c"

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

void *fa_alloc(size_t size, size_t align) {
	(void)align;
	__atomic_fetch_add(&allocated, 1, __ATOMIC_RELAXED);
	return malloc(size);
}

void fa_free(void *bytes, size_t size, size_t align) {
	(void)size;
	(void)align;
	__atomic_fetch_sub(&allocated, 1, __ATOMIC_RELAXED);
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
			if (string_owned(actual.capacity)) { fa_free((void *)actual.bytes, actual.capacity & KIND_MASK, 1); }
		}
	}
	FaString actual;
	fa_string_pad(text, 6, SIMPLE, INT64_MIN, (const uint8_t *)"_", 1, 1, &actual);
	CHECK(fa_string_equal(string_bytes(&actual), string_length(&actual), text, 6));
	// The general path still counts graphemes and cycles a multi-character filler.
	fa_string_pad((const uint8_t *)"\xc3\xa9", 2, 0, 4, (const uint8_t *)"ab", 2, 1, &actual);
	CHECK(fa_string_equal(string_bytes(&actual), string_length(&actual), (const uint8_t *)"aba\xc3\xa9", 5));
	if (string_owned(actual.capacity)) { fa_free((void *)actual.bytes, actual.capacity & KIND_MASK, 1); }
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
		if (string_owned(copy.capacity)) { fa_free((void *)copy.bytes, copy.capacity & KIND_MASK, 1); }
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
	fa_free((void *)text.bytes, text.capacity & KIND_MASK, 1);
	CHECK(allocated == before);
	for (size_t length = 1; length < sizeof input; length += 1) {
		FaString text;
		fa_string_own(input, length, KIND_MASK, &text);
		CHECK(fa_hash_string_bits((uintptr_t)text.bytes, text.length, text.capacity) == fa_hash_bytes(input, length));
		if (length <= INLINE_STRING_BYTES) { CHECK(!string_owned(text.capacity)); }
		if (string_owned(text.capacity)) { fa_free((void *)text.bytes, text.capacity & KIND_MASK, 1); }
	}
	CHECK(allocated == before);
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
		CHECK((uintptr_t)node % PAGE_SIZE + size <= PAGE_SIZE);
		__builtin_memset(node, 0xa5, size);
		for (size_t byte = 0; byte < 24; byte += 1) { CHECK(packed[byte] == 0x5a); }
	}
	fa_arena_leave(packed);
	fa_arena_drop(packed);
	CHECK(live_tree_arenas == 0 && tree_arena_depth == 0);
	CHECK((uint8_t *)free_spans == memory && free_spans->pages == 6 && free_spans->next == 0);
	free_spans = 0;
	free(memory);
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
	                                        probe, table_test_hash);
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
	// `reserve` grows the index only, from its own hashes: no entry moves
	CHECK(fa_table_capacity(table) == 4);
	TestTableEntry *kept = (TestTableEntry *)fa_table_entry(table, 0, sizeof(TestTableEntry));
	fa_table_reserve(table, 100, sizeof(TestTableEntry), table_test_hash);
	CHECK(fa_table_capacity(table) >= 101 && table->mask + 1 == 256 && table->capacity == 32);
	CHECK(table_test_find(table, -1) == 0 && kept->key == -1);
	fa_table_reserve(table, 10, sizeof(TestTableEntry), table_test_hash);
	CHECK(table->mask + 1 == 256);
	// a full chunk leaves the list of chunks with a free entry, and returns to it
	for (int64_t key = 0; key < 70; key += 1) {
		table_test_insert(table, key);
	}
	CHECK(table->capacity == 96 && table->free == 3);
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
		TestTableEntry *entry = fa_table_append(bitwise, 7, sizeof(TestTableEntry), probe, 0);
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
	                                      known_probe ? probe : 0, key32_hash);
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
		uint32_t keys[256];
		uint8_t present[256] = {0};
		for (int index = 0; index < 256; index += 1) {
			keys[index] = index == 0 ? 0 : index == 1 ? 0x80000000u :
			              index == 2 ? 0xffffffffu : (uint32_t)index - 2;
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
				fa_free((void *)text.bytes, text.capacity & KIND_MASK, 1);
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
	fa_free((void *)reserved.bytes, reserved.capacity & KIND_MASK, 1);
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
	fa_free((void *)text.bytes, text.capacity & KIND_MASK, 1);
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
	fa_free((void *)copy.bytes, copy.capacity & KIND_MASK, 1);
	CHECK(allocated == before);
	// Taking a frame value into an unknown owner detaches its bytes.
	FaString detached;
	fa_string_escape(&text, &detached);
	CHECK(string_owned(detached.capacity) && !string_frame(detached.capacity));
	CHECK(string_length(&detached) == 42 && allocated == before + 1);
	__builtin_memset(first, '?', 40);
	CHECK(__builtin_memcmp(string_bytes(&detached), original_bytes, 40) == 0);
	fa_free((void *)detached.bytes, detached.capacity & KIND_MASK, 1);
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
	fa_free((void *)text.bytes, text.capacity & KIND_MASK, 1);
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

int main(void) {
	test_tables();
	test_inline_key32();
	test_inline_loop_reservations();
	test_frame_strings();
	test_integer_strings();
	test_string_padding();
	test_inline_strings();
	test_tree_regions();
	test_arguments();
	test_character_views();
	test_allocator_fast_paths();
	test_short_copies();
	test_string_equal();
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
