# Concurrency

Concurrency is a first-class citizen in Fa, designed to help you build high-performance applications that can handle multiple tasks simultaneously.

## What is concurrency?

Concurrency is the ability to handle multiple tasks at the same time, though not necessarily simultaneously. It allows your program to make progress on multiple fronts, improving responsiveness and throughput. Concurrency can be achieved through:

- **Threads**: multiple execution contexts running in parallel on different CPU cores,
- **Asynchronous operations**: non-blocking operations using an event loop in a single thread.

Since Fa compiles to native, WebAssembly and JavaScript, some concurrency features are only available on specific targets.

On native targets, Fa uses [Tokio](https://tokio.rs) as its asynchronous runtime.

When targeting JavaScript, Fa uses a custom library built on top of [Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API), which allows for parallelism across multiple threads.

## Async programming

One of Fa's most powerful features is its **colorless async** design. Unlike many languages that distinguish between synchronous and asynchronous functions (creating "colored" functions), Fa allows any function defined as `async` to be called in both synchronous and asynchronous contexts.

### Why colorless is better

Colored async functions can force the developer to write two versions of the same function, one synchronous and one asynchronous. This leads to code duplication and increased complexity.

Fa handles this natively, by generating the synchronous and/or asynchronous version of a function when it is used.

### Defining async functions

Like in JavaScript and Rust, a function that *can be asynchronous* is defined with the `async` keyword.

**This does not mean that the function is always asynchronous**, as it can be called as a synchronous function. It only means that the function can be awaited, or started in the background.

```fa
async function fetchData = (url: String): Response throws HttpError => {
	let response = try await httpClient.get(url)
	response
}
```

### Calling async functions

An `async` function can be called in three ways:

1. **Blocking**: the function is called synchronously, blocking the current thread until it completes.
2. **Awaited**: the function is called with the `await` keyword. The current function is suspended until the result is available, and the thread can execute other tasks in the meantime. Only an `async` function can use `await`.
3. **Started**: the function is called with the `start` keyword. It starts executing in the background, and the caller continues with the next statements. `start` returns a `Task`, which can be awaited later.

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

### How colorless functions are compiled

When targeting JavaScript, an `async` function can be emitted in two versions: a synchronous one (`fetchData___sync`) and an asynchronous one (`fetchData___async`).

Versions are generated **lazily**: a version is only emitted if it is actually called somewhere in the program. A function that is only ever awaited produces only `fetchData___async`, and a function that is only ever called synchronously produces only `fetchData___sync`.

At the bottom of the call chain, the platform primitives (network, file system, timers, ...) must also provide the requested version. Some primitives have no synchronous implementation on some targets (for example, network requests on the browser's main thread). In that case, calling the function synchronously is a **compile error** for this target, never a runtime failure:

```
error: `fetchData` cannot be called synchronously on target `web`
  fetchData → httpClient.get → fetch (async-only on `web`)
  help: use `await fetchData(…)` or `start fetchData(…)`
```

## Multithreading

Fa provides several approaches to multithreading, each suited for different use cases.

### 1. Parallel iterations

**Available on native targets only.**

Use the `parallel` keyword in `for` loops to automatically parallelize iterations:

```fa
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

-- Sequential processing
for number in numbers {
	expensiveComputation(number)
}

-- Parallel processing: automatically uses the available CPU cores
for parallel number in numbers {
	expensiveComputation(number)
}

-- Parallel processing with a custom number of threads
for parallel(threads = 2) number in numbers {
	expensiveComputation(number)
}

-- Parallel mapping: collect the results of each iteration
let results = numbers.map(parallel (number) => expensiveComputation(number))
```

Iterations running in parallel cannot modify a shared value, unless it's a thread-safe type (see [thread safety](#thread-safety-and-synchronization)). To collect results, use a parallel `map`.

### 2. Worker files

Workers are instantiable threads defined in files with the `.worker.fa` extension. They encapsulate thread logic and can be used like any other type.

#### Defining a worker

```fa
-- MathWorker.worker.fa
export type = {
	numbers: Array(Integer)

	function sum = (self): Integer => self.numbers.reduce((total, n) => total + n, 0)

	function average = (self): Float => self.sum() / self.numbers.length

	async function processLarge = (self, data: Array(Integer)): Array(Integer) => data.map((x) => x * x)
}
```

#### Using workers

```fa
async function main = () => {
	-- Create worker instances
	let worker1 = MathWorker([1, 2, 3, 4, 5])
	let worker2 = MathWorker([6, 7, 8, 9, 10])

	-- Call worker methods (they run in separate threads)
	let sums = await [
		start worker1.sum()
		start worker2.sum()
	]

	let result1 = await worker1.processLarge([100, 200, 300])
	let result2 = await worker2.processLarge([400, 500, 600])

	console.log("Total sum: {sums[0] + sums[1]}")
	console.log("Processed results: {result1} {result2}")
}
```

### 3. Worker pool

To manage multiple workers efficiently, Fa provides a built-in worker pool:

```fa
use { WorkerPool } = std.concurrency

async function main = () => {
	-- Create a worker pool with 4 workers
	mutable workers = WorkerPool(DataProcessor, count = 4)

	let tasks = [
		{ data = [1, 2, 3] }
		{ data = [4, 5, 6] }
		{ data = [7, 8, 9] }
		{ data = [10, 11, 12] }
	]

	-- Submit tasks to the pool
	let futures = tasks.map((task) => workers.submit((worker) => worker.process(task.data)))

	-- Wait for all tasks to complete
	let results = await futures

	console.log("All results: {results}")

	-- Clean up
	workers.shutdown()
}

-- Advanced pool configuration
let pool = WorkerPool(
	of = DataProcessor
	count = 8
	maxQueueSize = 1000
	timeout = 30000 -- 30 seconds
)
```

## Concurrency patterns

### Producer-consumer

```fa
use { Channel } = std.concurrency

async function producer = (mutable channel: Channel(String)) => {
	for index in 1..=100 {
		await channel.send("Item {index}")
		await sleep(100) -- simulate work
	}
	channel.close()
}

async function consumer = (mutable channel: Channel(String), id: String) => {
	while let item = await channel.receive() {
		console.log("Consumer {id} received: {item}")
		await sleep(50) -- simulate processing
	}
}

async function main = () => {
	mutable channel = Channel(String) { capacity = 10 }

	-- Start the producer and the consumers in the background
	let producerTask = start producer(channel)
	let consumer1 = start consumer(channel, "1")
	let consumer2 = start consumer(channel, "2")

	-- Wait for all of them to complete
	await [producerTask, consumer1, consumer2]
}
```

### Fan-out / fan-in

```fa
async function fanOut = (input: Array(T), workers: Integer): Array(Channel(T)) => {
	mutable channels: Array(Channel(T)) = []
	for worker in 0..workers {
		channels.push(Channel(T)())
	}

	for item, index in input {
		let channelIndex = index modulo workers
		await channels[channelIndex].send(item)
	}

	-- Close all channels
	for channel in channels {
		channel.close()
	}

	channels
}

async function fanIn = (mutable channels: Array(Channel(T))): Channel(T) => {
	mutable output = Channel(T)()

	for channel in channels {
		-- Start a background task for each input channel
		start do {
			while let item = await channel.receive() {
				await output.send(item)
			}
		}
	}

	output
}
```

## Thread safety and synchronization

Synchronization primitives are declared `mutable`, like any value that changes. Only thread-safe types (`Mutex`, atomics, `Channel`) can be shared mutably between threads or tasks: sharing any other mutable value is a compile error.

### Mutexes and atomic operations

```fa
use { Mutex, AtomicInteger } = std.concurrency

mutable counter = AtomicInteger(0)
mutable sharedData = Mutex({ value = 0 })

async function incrementCounter = () => {
	-- Atomic operations are thread-safe
	counter.increment()

	-- Mutex for more complex operations
	mutable guard = await sharedData.lock()
	guard.value += 1
	-- automatically unlocked when `guard` goes out of scope
}

-- Using atomic operations
let count = counter.load()
counter.store(42)
counter.compareAndSwap(42, 100)

-- Atomic operations with memory ordering
counter.increment(Ordering.Relaxed)
counter.store(42, Ordering.SeqCst)
```

## Platform-specific considerations

### Native and WebAssembly targets

On native targets, you have access to:

- parallel iterations with the `parallel` keyword,
- full threading capabilities,
- native performance.

### JavaScript target

When targeting JavaScript, concurrency is handled through:

- Web Workers for true parallelism,
- async/await for non-blocking operations,
- SharedArrayBuffer for shared memory (where available),
- an automatic fallback to async patterns when threading is not available.

## Best practices

1. **Use the right tool for the job**:
   - parallel iterations for CPU-bound data processing,
   - workers for long-running background tasks,
   - async/await for I/O-bound operations.

2. **Avoid shared mutable state**:
   - prefer message passing over shared memory,
   - use immutable data when possible,
   - use Fa's built-in synchronization primitives.

3. **Handle errors explicitly**: errors of async functions are typed like any other error, and are propagated with `try await` or handled with `catch`.

   ```fa
   async function safeOperation = (): Result => {
   	let result = await riskyAsyncOperation() catch error => Result.default()
   	result
   }
   ```

4. **Monitor resource usage**:
   - don't create an unlimited number of workers,
   - use worker pools for better resource management,
   - set appropriate timeouts for operations.

5. **Test concurrent code thoroughly**:
   - use stress testing to find race conditions,
   - test with different numbers of workers,
   - verify that resources are properly cleaned up.
