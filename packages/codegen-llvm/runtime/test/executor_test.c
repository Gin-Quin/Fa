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
	allocated += 1;
	return malloc(size);
}

void fa_free(void *bytes, size_t size, size_t align) {
	(void)size;
	(void)align;
	allocated -= 1;
	free(bytes);
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

int main(void) {
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
	fa_release_builder();
	CHECK(allocated == 0);
	printf("the event loop tests passed\n");
	return 0;
}
