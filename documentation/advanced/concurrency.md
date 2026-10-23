# Concurrency

Concurrency is a first-class citizen in Fa, designed to help you build high-performance applications that can handle multiple tasks at the same time.

## What is concurrency?

Concurrency is the ability to handle multiple tasks at the same time, though not necessarily simultaneously. It allows your program to make progress on multiple fronts, improving responsiveness and throughput.

Fa gives you three tools, each suited to a different kind of work:

- **async functions**, for work that waits (network, file system, timers),
- **parallel iterations**, for work that computes,
- **workers**, for long-running background tasks with their own state.

Since Fa compiles to native code, WebAssembly and JavaScript, some of these tools are only available on specific targets (see [what each target supports](#what-each-target-supports)).

When compiling to native code, Fa uses [Tokio](https://tokio.rs) for asynchronous work and [Rayon](https://docs.rs/rayon) for parallel iterations. When compiling to JavaScript, Fa uses the event loop and a library built on top of [Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API).

## Async programming

One of Fa's most powerful features is its **colorless async** design. Unlike many languages that distinguish between synchronous and asynchronous functions (creating "colored" functions), Fa allows any function defined as `async` to be called both synchronously and asynchronously.

### Why colorless is better

Colored async functions divide code in two: an asynchronous function can only be awaited from another asynchronous function, which forces developers to write two versions of the same function, one synchronous and one asynchronous. This leads to code duplication and increased complexity.

Fa handles this natively, by generating the synchronous and/or asynchronous version of a function when it is used.

### Defining async functions

Like in JavaScript and Rust, a function that *can be asynchronous* is defined with the `async` keyword.

**This does not mean that the function is always asynchronous**, as it can be called as a synchronous function. It only means that the function *may* suspend, and can therefore be awaited, or started in the background.

```fa
async function fetchData = (url: String): Response throws HttpError => {
	let response = try await httpClient.get(url)
	response
}
```

### Calling async functions

An `async` function can be called in three ways:

1. **Blocking**: the function is called synchronously, blocking the current thread until it completes.
2. **Awaited**: the function is called with the `await` keyword. The current function is suspended until the result is available, and the thread can execute other tasks in the meantime. `await` can only be used inside an `async` function.
3. **Started**: the function is called with the `start` keyword. It starts executing in the background, and the caller continues with the next statements. `start` returns a `Task(T)`, which can be awaited later.

```fa
async function main = () throws HttpError => {
	-- 1. Blocking
	let data1 = try fetchData("https://api.example.com/data")
	console.log("This is printed once the data is fetched")

	-- 2. Awaited
	let data2 = try await fetchData("https://api.example.com/data")
	console.log("This is printed once the data is fetched, without blocking the thread")

	-- 3. Started
	let task = start fetchData("https://api.example.com/data") -- type: Task(Response)
	console.log("This is printed immediately, before the data is fetched")

	let data3 = try await task
}
```

The return type of a function is always the type of its result (`Response` here). Whether the function is called synchronously or asynchronously is decided by the caller.

`start` also accepts a block, to run several statements in the background:

```fa
start do {
	let report = try await buildReport()
	try await sendReport(report)
}
```

### How colorless functions are compiled

When targeting JavaScript, an `async` function can be emitted in two versions: a synchronous one (`fetchData___sync`) and an asynchronous one (`fetchData___async`).

Versions are generated **lazily**: a version is only emitted if it is actually called somewhere in the program. A function that is only ever awaited produces only `fetchData___async`, and a function that is only ever called synchronously produces only `fetchData___sync`.

At the bottom of the call chain, the platform primitives (network, file system, timers, ...) must also provide the requested version. Some primitives have no synchronous implementation on some targets (for example, network requests on the browser's main thread). In that case, calling the function synchronously is a **compile error** for this target, never a runtime failure:

```
error: `fetchData` cannot be called synchronously on target `web`
  fetchData → httpClient.get → fetch (async-only on `web`)
  help: use `await fetchData(…)` or `start fetchData(…)`
```

## Tasks

A `Task(T)` is the handle of a computation started with `start`. It can be awaited, stored, or cancelled.

### Cancelling a task

A task you no longer need should be cancelled, rather than abandoned:

```fa
let task = start slowOperation()

if userCancelled {
	task.cancel()
}
```

### Awaiting several tasks

The `Task` type provides combinators to wait for several tasks at once:

```fa
let responses = try await Task.all(tasks) -- every result, or the first error
let fastest = try await Task.any(tasks) -- the result of the first task to finish
let outcomes = await Task.settled(tasks) -- the outcome of every task, errors included
```

For example, to fetch two resources at the same time:

```fa
async function loadPage = (): Array(Response) throws HttpError => {
	try await Task.all([
		start fetchData("https://api.example.com/header")
		start fetchData("https://api.example.com/content")
	])
}
```

## Errors

Async functions declare and propagate their errors exactly like every other [function](/values/errors): with `throws` in the signature, `try` to propagate, and `catch` to handle. There is no rejected promise and no unhandled error.

`try` is written before `await`:

```fa
async function loadProfile = (id: Integer): Profile throws HttpError | ParseError => {
	let response = try await fetchData("/users/{id}")
	try parseProfile(response.body)
}
```

Errors are handled with `catch`, as in synchronous code:

```fa
async function showProfile = (id: Integer) => {
	let profile = await loadProfile(id) catch error => when error is {
		HttpError => defaultProfile
		ParseError => {
			console.log("Bad payload: {error.message}")
			defaultProfile
		}
	}

	console.log(profile.name)
}
```

The errors of a started function are raised when its task is awaited: `try await task` propagates them, and `await task catch error => ...` handles them.

## Multithreading

Fa provides several approaches to multithreading, each suited to different use cases.

### 1. Parallel iterations

**Available on the native target only.**

Use the `parallel` keyword in `for` loops to spread the iterations across the CPU cores:

```fa
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

-- Sequential processing
mutable sequentialResults: Array(Integer) = []
for number in numbers {
	sequentialResults.push(expensiveComputation(number))
}

-- Parallel processing: each iteration writes to its own index
mutable results = numbers -- a copy, with one slot per number
for parallel number, index in numbers {
	results[index] = expensiveComputation(number)
}

-- Parallel processing with a custom number of threads
for parallel(threads = 2) number, index in numbers {
	results[index] = expensiveComputation(number)
}

-- Parallel methods: `parallel()` returns a view whose operations run in parallel
let doubled = numbers.parallel().map((number) => number * 2)
let total = numbers.parallel().filter((number) => number > 0).sum()
```

`parallel()` works on every collection. The operations of the view document their own guarantees: `map` and `filter` keep the order of the elements, and `reduce` requires an associative function.

Iterations running in parallel cannot modify a shared value, unless:

- each iteration only writes to its own index (`results[index] = ...`): the compiler proves that the writes are disjoint,
- or the value is a thread-safe type (see [thread safety](#thread-safety-and-synchronization)).

The compiler rejects any other write to a shared value, including `results.push(...)`: `push` invalidates the array, and two iterations could push at the same time. This is why the parallel example above writes to an index instead.

### 2. Workers

Workers are instantiable threads with their own state, defined in files with the `.worker.fa` extension. Like every Fa file, a worker file has a [single export](/architecture/modules): a type, whose methods run on the worker's thread.

#### Defining a worker

```fa
-- MathWorker.worker.fa
export type = {
	numbers: Array(Integer)

	function sum = (self): Integer => self.numbers.reduce((total, number) => total + number, 0)

	function average = (self): Float => {
		let count = self.numbers.length
		if count != 0 { self.sum() / count } else { 0 }
	}

	function processLarge = (self, data: Array(Integer)): Array(Integer) => data.map((x) => x * x)
}
```

#### Using workers

Creating a value of a worker type starts a new thread. From the caller's side, every method of a worker behaves like an `async` function: it can be called, awaited, or started.

```fa
async function main = () => {
	-- Create worker instances: each one runs in its own thread
	mutable worker1 = MathWorker { numbers = [1, 2, 3, 4, 5] }
	mutable worker2 = MathWorker { numbers = [6, 7, 8, 9, 10] }

	-- Call worker methods: they run in parallel, in separate threads
	let sums = await Task.all([
		start worker1.sum()
		start worker2.sum()
	])

	let result1 = await worker1.processLarge([100, 200, 300])
	let result2 = await worker2.processLarge([400, 500, 600])

	console.log("Total sum: {sums[0] + sums[1]}")
	console.log("Processed results: {result1} {result2}")

	-- Stop the workers' threads
	worker1.stop()
	worker2.stop()
}
```

`stop()` stops a worker's thread. A worker is also stopped when it is destroyed, at the end of its scope.

#### Values are copied across threads

Arguments and results are **copied** across the thread boundary. This is not a special rule for concurrency: it is Fa's ordinary copy-on-assignment rule (see [memory management](/advanced/memory-management)), and it makes workers safe by construction, since two threads never share the same data.

[References](/advanced/owned-references) and [relations](/advanced/relations) cannot cross a worker boundary: passing one to a worker method is a compile error. [Requirements](/advanced/requirements) do not cross worker boundaries either.

### 3. Worker pools

To manage multiple workers efficiently, Fa provides a built-in worker pool:

```fa
use { WorkerPool } = std.concurrency

async function main = () => {
	-- Create a pool of 4 workers
	mutable workers = WorkerPool(DataProcessor) { count = 4 }

	let inputs = [
		[1, 2, 3]
		[4, 5, 6]
		[7, 8, 9]
		[10, 11, 12]
	]

	-- Submit the tasks to the pool
	mutable pending: Array(Task(Array(Integer))) = []
	for data in inputs {
		pending.push(workers.submit((worker) => worker.process(data)))
	}

	-- Wait for all the tasks to complete
	let results = await Task.all(pending)
	console.log("All results: {results}")

	-- Stop all the workers
	workers.shutdown()
}
```

A pool accepts more options:

```fa
mutable pool = WorkerPool(DataProcessor) {
	count = 8
	maxQueueSize = 1000
	timeout = 30000 -- milliseconds
}
```

## Channels

A channel moves values between tasks or threads. It is the preferred way for concurrent code to communicate.

### Producer-consumer

```fa
use { Channel } = std.concurrency

async function producer = (channel: Channel(String)) mutates channel => {
	for index in 1..=100 {
		await channel.send("Item {index}")
		await sleep(100) -- simulate work
	}
	channel.close()
}

async function consumer = (channel: Channel(String), id: String) mutates channel => {
	while let item = await channel.receive() {
		console.log("Consumer {id} received: {item}")
		await sleep(50) -- simulate processing
	}
}

async function main = () => {
	mutable channel = Channel(String) { capacity = 10 }

	-- Start the producer and the consumers, then wait for all of them
	await Task.all([
		start producer(channel)
		start consumer(channel, "1")
		start consumer(channel, "2")
	])
}
```

`receive` returns an optional, which is `none` once the channel is closed and empty, so the `while let` loop ends when the producer is done. See [loops](/control-flow/loops).

Giving channels a capacity applies back-pressure: when a channel is full, `send` waits until a consumer receives a value.

### Fan-out and fan-in

Several consumers can receive from the same channel: each value is received by exactly one of them (**fan-out**). Several producers can send into the same channel (**fan-in**).

```fa
async function processJobs = (jobs: Channel(Job), reports: Channel(Report)) mutates jobs, reports => {
	while let job = await jobs.receive() {
		await reports.send(process(job))
	}
}

async function main = () => {
	mutable jobs = Channel(Job) { capacity = 16 }
	mutable reports = Channel(Report) { capacity = 16 }

	-- Fan-out: three tasks take jobs from the same channel,
	-- fan-in: and send their reports into the same channel
	let processing = Task.all([
		start processJobs(jobs, reports)
		start processJobs(jobs, reports)
		start processJobs(jobs, reports)
	])

	-- Close the reports once every job is processed
	start do {
		await processing
		reports.close()
	}

	for job in loadJobs() {
		await jobs.send(job)
	}
	jobs.close()

	while let report = await reports.receive() {
		console.log(report)
	}
}
```

## Thread safety and synchronization

Synchronization primitives (`Mutex`, atomics, `Channel`) are declared `mutable`, like any value that changes. They are **thread-safe types**: only thread-safe types can be shared mutably between threads or tasks, and sharing any other mutable value is a compile error.

### Mutexes

A `Mutex` protects a value. `lock()` waits until the value is available, and returns a guard giving access to it:

```fa
use { Mutex } = std.concurrency

type Stats = {
	visits: Integer
	errors: Integer
}

async function recordVisit = (stats: Mutex(Stats)) mutates stats => {
	do {
		use guard = await stats.lock()
		guard.visits += 1
	} -- the lock is released here, when `guard` leaves its scope
}

async function main = () => {
	mutable stats = Mutex(Stats) { value = { visits = 0, errors = 0 } }

	-- both tasks share `stats`, which is allowed because `Mutex` is thread-safe
	await Task.all([
		start recordVisit(stats)
		start recordVisit(stats)
	])
}
```

A guard is an ordinary value, so it is released at the end of its scope by the same rule as every other value. There is no `unlock`: keep the scope of a guard as small as possible.

### Atomics

Atomics are thread-safe numbers, modified without a lock:

```fa
use { AtomicInteger } = std.concurrency

mutable counter = AtomicInteger { value = 0 }

counter.increment()
let count = counter.load()
counter.store(42)
counter.compareAndSwap(42, 100)

-- With an explicit memory ordering
counter.increment(Ordering.relaxed)
counter.store(42, Ordering.sequential)
```

## Requirements and concurrency

[Requirements](/advanced/requirements) cross `start`: a task started in the background runs on the same thread, and sees the values provided where it was started.

```fa
function announce = (message: String) requires logger: Logger => {
	logger.info(message)
}

async function main = () => {
	provide Logger = {
		info = (message) => console.log(message)
	}

	let task = start announce("Server started") -- `announce` receives the provided `Logger`
	await task
}
```

Requirements do **not** cross worker boundaries. A worker only receives copies of the arguments it is given: to give it a value, pass it as an argument.

## What each target supports

| Feature | `native` | `wasm` | JavaScript (`web`, `bun`, `node`, `deno`) |
| --- | --- | --- | --- |
| `async` / `await` / `start` | yes | yes | yes |
| Workers, channels | yes | yes | yes (Web Workers) |
| `for parallel` | yes | no | no |
| `Mutex`, atomics | yes | with shared memory | with `SharedArrayBuffer` |

Code using a feature that is not available on a target fails to compile for this target, rather than silently degrading. To branch on the target, use a [compile-time](/advanced/metaprogramming) `@if`:

```fa
@if target is "native" {
	for parallel item, index in items {
		results[index] = process(item)
	}
} @else {
	for item, index in items {
		results[index] = process(item)
	}
}
```

## Best practices

1. **Use the right tool for the job**:
	- `async` / `await` for I/O-bound operations,
	- parallel iterations for CPU-bound data processing,
	- workers for long-running background tasks.

2. **Send messages rather than sharing state**:
	- prefer channels over shared memory,
	- use immutable data when possible: Fa's copy semantics make this the path of least resistance,
	- use Fa's built-in synchronization primitives when sharing is needed.

3. **Handle errors explicitly**: errors of async functions are typed like any other error, and are propagated with `try await` or handled with `catch`.

4. **Bound your concurrency**:
	- use a worker pool rather than creating an unlimited number of workers,
	- give channels a capacity, so that a slow consumer applies back-pressure,
	- set appropriate timeouts for operations.

5. **Cancel what you start**: a `Task` you no longer need should be cancelled, not abandoned.

6. **Test concurrent code under load**: deterministic tests catch logic errors, stress tests catch contention. Test with different numbers of workers, and verify that resources are properly released.
