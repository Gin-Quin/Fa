# Concurrency

Concurrency is a first-class citizen in Fa, designed to help you build high-performance applications that can handle multiple tasks at the same time.

## What is concurrency?

Concurrency is the ability to handle multiple tasks at the same time, though not necessarily simultaneously. It allows your program to make progress on multiple fronts, improving responsiveness and throughput.

Fa gives you three tools, each suited to a different kind of work:

- **async functions**, for work that waits (network, file system, timers),
- **parallel iterations**, for work that computes,
- **workers**, for long-running background tasks with their own state.

Since Fa compiles to native code, WebAssembly and JavaScript, some of these tools are only available on specific targets (see [what each target supports](#what-each-target-supports)).

Tasks started with `start` all run on the thread that started them, one at a time, as in JavaScript: they take turns at each `await`. Threads only come from [workers](#2-workers) and [parallel iterations](#1-parallel-iterations).

## Async programming

One of Fa's most powerful features is its **colorless async** design. Unlike many languages that distinguish between synchronous and asynchronous functions (creating "colored" functions), Fa allows any function defined as `async` to be called both synchronously and asynchronously.

### Why colorless is better

Colored async functions divide code in two: an asynchronous function can only be awaited from another asynchronous function, which forces developers to write two versions of the same function, one synchronous and one asynchronous. This leads to code duplication and increased complexity.

Fa handles this natively: the compiler generates the synchronous and/or the asynchronous version of an `async` function, depending on how it is called.

### Defining async functions

Like in JavaScript and Rust, a function that *can be asynchronous* is defined with the `async` keyword.

**This does not mean that the function is always asynchronous**, as it can be called as a synchronous function. It only means that the function *may* suspend, and can therefore be awaited, or started in the background.

```fa
async function fetchData = (url: String): Response throws HttpError => {
	let response = try await std.http.get(url)
	response
}
```

Only `async` functions have two versions. A function without `async` is always synchronous: when it calls an `async` function without `await`, it blocks. To let a caller suspend instead of blocking, make it `async` too: `async` goes up the call chain as far as you want the code to suspend, and no further.

### Calling async functions

An `async` function can be called in three ways:

1. **Blocking**: the function is called synchronously, blocking the current thread until it completes.
2. **Awaited**: the function is called with the `await` keyword. The current function is suspended until the result is available, and the thread can execute other tasks in the meantime.
3. **Started**: the function is called with the `start` keyword. It runs in the background, and the caller continues with the next statements. `start` returns a [`Task`](#tasks), which can be awaited later.

```fa
async function main = () throws HttpError => {
	-- 1. Blocking (on targets where `std.http.get` can block, see below)
	let data1 = try fetchData("https://api.example.com/data")
	console.log("This is printed once the data is fetched")

	-- 2. Awaited
	let data2 = try await fetchData("https://api.example.com/data")
	console.log("This is printed once the data is fetched, without blocking the thread")

	-- 3. Started
	let task = start fetchData("https://api.example.com/data") -- type: Task<Response> throws HttpError
	console.log("This is printed immediately, before the data is fetched")

	let data3 = try await task
}
```

The return type of a function is always the type of its result (`Response` here). Whether the function is called synchronously or asynchronously is decided by the caller.

The rules:

- `await` is only allowed in an **async context**: the body of an `async function`, an async lambda, a `start do` block, a test, and the top level of a standalone file.
- `await` takes a call of an `async` function, or a `Task` value. Awaiting a call of a function that isn't `async` is an error: call it directly.
- `start` takes any call, `async` or not, or a `do` block. A started task does not run right away: it starts when the current code waits (an `await` whose result isn't ready yet) or returns to the event loop. Other tasks *may* run at any `await`, and *do* run when the code actually waits: an awaited call that finishes without waiting is a plain call.
- A **blocking call** runs the synchronous version of the function. While it runs, every `await` inside it blocks, and every `start` runs its call to the end right away and gives a finished task. The whole thread is blocked, including the other tasks of the thread. Awaiting, inside a blocking call, a task that was started outside of it is an error, since that task needs the thread the call blocks.

`start` also accepts a block, to run several statements in the background. A [`start do`](/control-flow/do#start-do) block is an async context, in any function:

```fa
start do {
	let report = await buildReport() catch error => emptyReport
	await sendReport(report) catch error => console.log("The report was not sent: {error.toString()}")
}
```

This task is not kept, so it is [detached](#tasks): it handles its own errors.

### Async function types

`async` is part of a [function type](/functions/defining-functions#function-types):

```fa
type Handler = async (request: Request) => Response

async function handleHome = (request: Request): Response => {
	let page = await loadPage("home") catch error => return Response.notFound()
	Response.html(page)
}

let handler: Handler = handleHome
let response = await handler(request)
```

- A function type without `async` can't hold an `async` function: a call through it would have to block, which the caller didn't ask for.
- A function type with `async` can hold a function without `async`: awaiting it simply calls it.
- A lambda is `async` when its expected function type is: `server.get("/", (request) => await render(request))`.
- Higher-order functions are not async-polymorphic: `items.map(fetchData)` is an error, since `map` takes a function type without `async`. `items.map((item) => fetchData(item))` calls `fetchData` blocking, item by item. To fetch several items at once, start one task per item and combine them with [`Task.all`](#awaiting-several-tasks).

### Blocking calls and targets

At the bottom of the call chain, the platform primitives (network, file system, timers, ...) must provide the version the caller asks for. Some primitives have no synchronous implementation on some targets: network requests on JavaScript, for example, can only be awaited. Calling a function synchronously when its call chain reaches such a primitive is a **compile error** for this target, never a runtime failure:

```
error: `fetchData` cannot be called blocking on target `web`
  fetchData → std.http.get → fetch (async-only on `web`)
  help: use `await fetchData(…)` or `start fetchData(…)`
```

| Primitive | `native` | `wasm` | `web` | `bun`, `node` |
| --- | --- | --- | --- | --- |
| File system, standard input | blocking or async | blocking (through the loader) | not available | blocking or async |
| Network (HTTP client and server, sockets) | blocking or async | async | async | async |
| Timers (`sleep`) | blocking or async | async | async | blocking or async |
| Console output | blocking | blocking | blocking | blocking |
| Waiting on a channel, a mutex or a worker | blocking or async | async | async | async |

The last row means that on JavaScript and WebAssembly, a blocking call can't wait for a channel, a mutex or a worker: only native threads can wait without stopping every other task.

Whether a function can be called blocking on a target is part of its **interface**: the compiler infers it from the function's body, shows it in the function's documentation, and a library that removes it from a function makes a [breaking change](/guides/deploy-a-library#version-it).

## Tasks

A closure never consumes what it captures. Even a nonescaping closure cannot await or move a captured Task: pass that Task as an explicit parameter from its owning scope. Task-owning parameters transfer; structure and resource parameters still borrow.

```fa
async function work = (): Integer => 1
async function outer = (): Integer => {
	let task = start work()
	async function finish = (): Integer => await task -- error: MEM002
	await finish()
}
```

Make the transfer visible at the call:

```fa
async function work = (): Integer => 1
async function finish = (task: Task<Integer>): Integer => await task catch _error => -1
async function outer = (): Integer => {
	let task = start work()
	await finish(task)
}
```


`start` evaluates the written arguments and omitted parameter defaults when the task is created, in source order; only its body waits for the task's first turn. An error from a default is an error of the `start` expression, and already evaluated owned arguments are destroyed if creation fails. Cancelling before the first turn does not undo defaults that already ran.

```fa
async function greet = (name: String = "Ana"): String => name
let task = start greet() -- binds the default now; runs the body on its first turn
console.log(await task)
```

A `Task` is the handle of a computation started with `start`. Its type declares its result and its errors, like a function type: starting `fetchData` gives a `Task<Response> throws HttpError`.

```fa
async function loadAll = (urls: Array<String>): Array<Response> throws HttpError => {
	mutable tasks: Array<Task<Response> throws HttpError> = []
	for url in urls {
		tasks.push(start fetchData(url))
	}
	try await Task.all(tasks)
}
```

A task is **structured**:

- A task can't be copied: assigning it, passing it to `Task.all` or storing it in a collection moves it, and the previous name can't be used anymore.
- `await task` waits for the result and consumes the task.
- **Leaving its scope cancels a task and waits for it.** A task that is still running when the value holding it is destroyed is cancelled, so nothing keeps running unnoticed.
- A `start` whose task is not kept (a `start` statement) is **detached**: it runs to its end, independently of the scope that started it. Since nobody awaits it, it can't have `throws` (handle its errors with `catch` inside), and like an [escaping closure](/functions/defining-functions#closures), it owns its arguments and captures under the [storage rules](/advanced/memory-management#copies-moves-unique-values-and-shared-handles): ordinary values copy, collections move at last use or require explicit `copy`, and unique captures are forbidden (`MEM003`). So it uses no local value and no requirement, and doesn't reserve the module-level values it modifies: other code sees its changes after each `await`. The program waits for its detached tasks before it exits.

### Cancelling a task

A task you no longer need should be cancelled, rather than abandoned:

```fa
let task = start slowOperation()

if userCancelled {
	task.cancel()
}
```

- Cancellation is cooperative: a task stops at a suspension point (an `await`). The values it owns are destroyed as at the end of a scope, and the tasks it owns are cancelled in turn.
- `cancel()` waits for the task to stop. Since tasks share one thread, a task that is cancelled is never running at that moment: it stops at the `await` where it is suspended, immediately.
- Awaiting a cancelled task throws `Cancelled`. The compiler adds `Cancelled` to the errors of an `await` only when `cancel()` may have been called on that task.

### Tasks and references

Parameters are references, also for a started function: while a task runs, it uses the values passed to it. The effects of the started function apply to the caller's references at the `start`: the references they invalidate are invalidated there, as for a call. Then, until the task ends (it is awaited, cancelled, or its scope ends), a value can't be modified if the task reads it, and can't be used at all if the task modifies it (see [reference checking](/advanced/reference-checking#tasks)):

```fa
async function main = () => {
	mutable scores: Array<Integer> = [1, 2, 3]

	let task = start uploadScores(scores) -- `uploadScores` reads `scores` in the background
	scores.push(4) -- error: `scores` is read by `task`: await it first

	await task
	scores.push(4) -- valid: the task is finished
}
```

A task uses:

- the arguments of the started call,
- the local values a `start do` block uses,
- the module-level values in the effects of the started function (`mutates people.size`),
- its [requirements](/advanced/requirements#requirements-and-concurrency).

**Thread-safe values are the exception**: the methods of a `Mutex`, an atomic or a `Channel` don't declare `mutates self`, so several tasks can use the same one at once (see [thread safety](#thread-safety-and-synchronization)).

A task can't outlive the values it uses. Its data must belong to the caller, or to the task itself:

```fa
async function startUpload = (): Task<Void> => {
	let scores = [1, 2, 3]
	start uploadScores(scores) -- error: the task uses `scores`, which is destroyed when `startUpload` returns
}

async function startUploadOwned = (): Task<Void> => start do {
	let scores = [1, 2, 3] -- owned by the task
	await uploadScores(scores)
}
```

A task that uses no local value, because it only receives copies or owns its data, can be returned, stored and awaited anywhere.

At each `await`, other tasks may run and modify the module-level values. So after an `await`, the references into module-level `mutable` values must be declared again, and what the compiler knew about them ([narrowing](/control-flow/conditionals#when-narrowing-ends), ranges) is forgotten. The same goes at each new iteration of a loop of an `async` function or of a `start do` block, which may let other tasks run (see [letting other tasks run](#letting-other-tasks-run)).

### Letting other tasks run

Tasks take turns: a task keeps the thread until it waits. So that one long task can't stop the others (a request computing for a long time must not block the other requests of a server), a loop of an `async` function or of a `start do` block lets the other tasks of its thread run when its turn has lasted more than a few milliseconds: an iteration of the loop then waits for them, like `await Task.yield()`.

```fa
async function handleReport = (rows: Array<Row>): Report => {
	mutable report = Report { total = 0 }
	for row in rows {
		report.total += compute(row) -- other requests are served between iterations, when this one takes long
	}
	report
}
```

A loop of a function that isn't `async`, and every loop of a blocking call, never lets other tasks run: synchronous code runs at full speed, and keeps the thread until it ends. To let other tasks run from such code, `await Task.yield()` from an `async` function, or move the work to a [worker](#2-workers) or a [parallel iteration](#1-parallel-iterations).

| Member | |
| --- | --- |
| `task.cancel()` | cancels the task (see [cancelling a task](#cancelling-a-task)) |
| `Task.yield()` | async: lets the other tasks of the thread run, then continues; does nothing in a blocking call |
| `Task.all`, `Task.any`, `Task.settled` | combine tasks (see [awaiting several tasks](#awaiting-several-tasks)) |

### Awaiting several tasks

The `Task` type provides combinators. They are plain functions that take tasks and return a new `Task`, which is awaited like any other:

```fa
let responses = try await Task.all(tasks)   -- every result, or the first error
let fastest = try await Task.any(tasks)     -- the outcome of the first task to finish
let outcomes = await Task.settled(tasks)    -- the outcome of every task, errors included
```

| Combinator | From an `Array<Task<T> throws E>` | Notes |
| --- | --- | --- |
| `Task.all` | `Task<Array<T>> throws E` | results in the order of the tasks; on the first error, the other tasks are cancelled |
| `Task.any` | `Task<T> throws E` | the result or the error of the first task to finish; the other tasks are cancelled |
| `Task.settled` | `Task<Array<T \| E>>` | never fails: each outcome is a result or an error, checked with `is` |

The tasks are moved into the combinator. For example, to fetch two resources at the same time:

```fa
async function loadPage = (): Array<Response> throws HttpError => {
	try await Task.all([
		start fetchData("https://api.example.com/header")
		start fetchData("https://api.example.com/content")
	])
}
```

### Synchronous programs

A program that never starts a task nor awaits something that may wait doesn't need an event loop: the compiler builds it as one blocking call of its entry, with only the synchronous version of its functions. When the `asynchronous` option of the [build configuration](/architecture/configuration#build) is not set, the compiler decides from the program itself:

- a program that starts a task (`start`), or awaits a call that may wait (a timer, the network, a channel, a mutex, a worker), is asynchronous;
- a program whose `async` functions never wait (`async function main` that only computes) is not.

`fa build -v` says which mode a program builds in, and why. Forced off (`asynchronous = false` or `--asynchronous=false`), a program that starts tasks still works, but each task runs to its end as soon as it is started: the compiler warns that the output order may change.

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
			console.log("Bad payload: {error.toString()}")
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
mutable sequentialResults: Array<Integer> = []
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

The iterations run in any order, on several threads. The body of a parallel loop follows these rules:

- **Reads**: it can read any value that no iteration modifies.
- **Writes**: it can only write
	- through the loop variable, when iterating over a `mutable` collection: each iteration owns its element (except the elements of a `Set` and the keys of a `Map`, which are read-only),
	- to its own index, `target[index]` or a path below it (`target[index].score`), where `index` is the index of the loop and `target` is an `Array` or a `Columnar`. When the compiler can't prove that `index` is in bounds of `target` (here it can: `results` is a copy of `numbers`), the write is `try target[index] = ...`,
	- to a [thread-safe](#thread-safety-and-synchronization) value,
	- to the values declared inside the body, which belong to the iteration.
- **Calls**: a function called in the body follows the same rules through its effects: it can't modify a module-level value or a requirement unless it is thread-safe. Reading requirements is allowed. `console` output is allowed (the lines of different iterations may interleave).
- **Not allowed**: `break`, `return` and `await` (the body is not an async context), and writing a [reactive](/advanced/reactivity) value. `continue` ends the current iteration.
- **Errors**: the body may throw. When an iteration fails, the iterations that haven't started are skipped, the running ones finish, and the error of the failed iteration with the smallest index is raised.
- **Index**: the index binding is only available on collections with positions (`Array`, `Columnar`, `GrowingArray`).

The compiler rejects any other write to a shared value, including `results.push(...)`: `push` mutates the whole array, and two iterations could push at the same time. This is why the parallel example above writes to an index instead.

`parallel()` works on every collection, on the targets that support `for parallel`. Its callbacks follow the rules of a parallel body. `map` and `filter` keep the order of the elements. `reduce` groups the elements in an unspecified way, so its function should be associative: the compiler can't check it, and with a function that isn't, the result depends on the grouping.

`parallel()` creates a view for one operation called immediately: `map`, `filter`, `reduce`, `some`, `every` or `sum`. It cannot be saved in a variable. `filter` keeps the collection kind. A `reduce` callback combines two items, so its accumulator has the item type. On a map, the view visits `(key, value)` entries and provides `map`, `filter`, `some` and `every`; `map` and `filter` keep the keys. Convert `map.values()` to an array for a parallel `reduce`.

### 2. Workers

Workers are instantiable threads with their own state, defined in files with the `.worker.fa` extension. Like every Fa file, a worker file has a [single export](/architecture/modules): a type, whose methods run on the worker's thread.

#### Defining a worker

```fa
-- MathWorker.worker.fa
export type = {
	numbers: Array<Integer>

	function sum = (self): Integer => self.numbers.reduce((total, number) => total + number, 0)

	function average = (self): Float => {
		let count = self.numbers.length
		if count != 0 { self.sum() / count } else { 0 }
	}

	function processLarge = (self, data: Array<Integer>): Array<Integer> => data.map((x) => x * x)
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
	let firstSum = start worker1.sum()
	let secondSum = start worker2.sum()
	let total = (await firstSum) + (await secondSum)

	let result1 = await worker1.processLarge([100, 200, 300])
	let result2 = await worker2.processLarge([400, 500, 600])

	console.log("Total sum: {total}")
	console.log("Processed results: {result1} {result2}")

	-- Stop the workers' threads
	worker1.stop()
	worker2.stop()
}
```

The rules:

- The value the caller holds is a **unique handle** to the worker. It moves, `copy` is `MEM003`, and a parameter borrows it (unlike a Task-consuming parameter). WorkerPool, socket/listener, guard and subscription owners follow the same unique lifetime rule. Only Mutex, Channel and atomics share identity when copied.
- **Calls are queued**: a worker runs one call at a time, in the order the calls were made, so its state is never accessed by two threads at once. A method declared with `mutates self` modifies the worker's state, and needs a `mutable` handle.
- The caller can't read or write the fields of a worker: it only calls its methods. The fields are given when the worker is created.
- A started call uses the handle, like any started call uses its arguments: the worker can't be stopped or moved until the task is awaited or cancelled.
- `stop()` waits for the call in progress, cancels the queued ones (their tasks fail with `Cancelled`) and stops the thread; the handle can't be used afterwards. A worker is also stopped when its handle is destroyed, at the end of its scope. A worker type can't declare a method named `stop`.
- The code of a worker can't use a module-level `mutable` value unless its type is thread-safe; immutable module-level values are available.
- Methods of a worker type can't declare `requires`: requirements don't cross threads.
- A method's errors reach the caller like those of any `async` function.

#### Values are copied across threads

Arguments and results crossing a worker boundary require an independent value, a common `Mutex` protecting shared state, or a proven exclusive transfer invalidating the source and its aliases. Keeping a collection usable on both sides requires `copy` at the visible transfer site. Unique resources can transfer only with an exclusivity proof; a Task never crosses, even inside a resource, a shared payload or a closure (`CONC008`). There is no automatic field locking. A value borrowed from `Guard.value` cannot escape its protection, nor can an older unprotected alias bypass that protection. See [memory management](/advanced/memory-management#closures).

[References](/advanced/owned-references) and [relations](/advanced/relations) cannot cross a worker boundary: passing one to a worker method is a compile error. [Requirements](/advanced/requirements) do not cross worker boundaries either.

[Thread-safe values](#thread-safety-and-synchronization) are the only shared ones: a mutex, an atomic or a channel passed to a worker is the same mutex, atomic or channel on both sides.

### 3. Worker pools

To manage multiple workers efficiently, Fa provides a built-in worker pool:

```fa
use { WorkerPool, WorkerTimeout } = std.concurrency

async function main = () => {
	-- Create a pool of 4 workers, each one starting from a copy of `state`
	mutable workers = WorkerPool<DataProcessor> {
		count = 4
		state = DataProcessor { factor = 2 }
	}

	let inputs = [
		[1, 2, 3]
		[4, 5, 6]
		[7, 8, 9]
		[10, 11, 12]
	]

	-- Submit the tasks to the pool
	mutable pending: Array<Task<Array<Integer>> throws WorkerTimeout> = []
	for data in inputs {
		let snapshot = copy data
		pending.push(await workers.submit((worker) => await worker.process(snapshot)))
	}

	-- Wait for all the tasks to complete
	let results = await Task.all(pending) catch error => []
	console.log("All results: {results}")

	-- Stop all the workers
	workers.shutdown()
}
```

`state` is the initial state of each worker: every worker of the pool starts with its own independent copy of it. This is a repeated-copy operation, including after a restart: unique state is forbidden (`MEM003`). An existing collection-valued state needs an explicit `copy` at construction (`MEM004`); the pool's documented per-worker copies are then part of that operation, never a one-time transfer. It may be left out when every field of the worker type has a default value, and the workers then start from `DataProcessor {}`.

`submit` takes an async function that receives a free worker of the pool, and gives its task. It is `async`: it waits while the pool's queue is full (`maxQueueSize`), so `await workers.submit(…)` gives the task, not its result; called without `await`, it waits on the calling thread. The function is stored until a worker is free, so it owns immutable captures under the [escaping-closure rules](/functions/defining-functions#closures), with explicit copies for borrowed structure data and no unique captures.

A pool accepts more options:

```fa
mutable pool = WorkerPool<DataProcessor> {
	count = 8
	state = DataProcessor { factor = 3 }
	maxQueueSize = 1000 -- `submit` waits while 1000 calls are queued
	timeout = 30000 -- milliseconds
}
```

With a `timeout`, a call that takes longer is cancelled, its worker is restarted from a fresh copy of `state`, and its task fails with `WorkerTimeout` (from `std.concurrency`). Without one, `submit` never fails with `WorkerTimeout`. `shutdown()` waits for the queued calls, then stops the workers.

The task returned by `submit` also fails with the errors of its submitted function. A local pool initialized without `timeout` and never replaced gives tasks without `WorkerTimeout`; these still satisfy a task type that lists `WorkerTimeout`.

## Channels

A channel moves values between tasks or threads. It is the preferred way for concurrent code to communicate.

### Producer-consumer

```fa
use { Channel } = std.concurrency
use { sleep } = std.time

async function producer = (channel: Channel<String>) => {
	for index in 1..=100 {
		await channel.send("Item {index}") catch error => return -- the channel was closed
		await sleep(100) -- simulate work
	}
	channel.close()
}

async function consumer = (channel: Channel<String>, id: String) => {
	while let item = await channel.receive() {
		console.log("Consumer {id} received: {item}")
		await sleep(50) -- simulate processing
	}
}

async function main = () => {
	mutable channel = Channel<String> { capacity = 10 }

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
async function processJobs = (jobs: Channel<Job>, reports: Channel<Report>) => {
	while let job = await jobs.receive() {
		await reports.send(process(job)) catch error => return
	}
}

async function main = () => {
	mutable jobs = Channel<Job> { capacity = 16 }
	mutable reports = Channel<Report> { capacity = 16 }

	-- Fan-out: three tasks take jobs from the same channel,
	-- fan-in: and send their reports into the same channel.
	-- The reports are closed once every job is processed.
	let processing = start do {
		await Task.all([
			start processJobs(jobs, reports)
			start processJobs(jobs, reports)
			start processJobs(jobs, reports)
		])
		reports.close()
	}

	for job in loadJobs() {
		await jobs.send(job) catch error => break
	}
	jobs.close()

	while let report = await reports.receive() {
		console.log(report)
	}

	await processing
}
```

### The `Channel` type

| Member | |
| --- | --- |
| `Channel<T> { capacity = 16 }` | creates a channel; without `capacity`, it is unbounded |
| `send(value)` | async: waits while the channel is full; `throws ChannelClosed` when the channel is closed |
| `receive()` | async: the next value, or `none` once the channel is closed and empty |
| `trySend(value)` | sends without waiting; returns `false` when the channel is full or closed |
| `tryReceive()` | receives without waiting; `none` when no value is ready |
| `close()` | no value can be sent anymore; the values already sent can still be received |

A value sent through a channel is copied (moved when it isn't used afterwards). `T` can't contain references or relations.

## Thread safety and synchronization

The **thread-safe types** are:

- `Mutex<T>`, `AtomicInteger`, `AtomicBoolean` and `Channel<T>`, from `std.concurrency`,
- objects and tuples whose fields are all thread-safe.

They are the only values that can be shared mutably between tasks or threads; sharing any other mutable value is a compile error. They work differently from other values:

- **They are handles.** A copy of a thread-safe value designates the same mutex, atomic or channel, which lives as long as one of its handles does. This is how a worker and its caller share a channel.
- **Their methods don't declare `mutates self`.** Like Rust's `&self` methods on a `Mutex`, they modify the shared state from an immutable reference. So several tasks and threads can use the same one at once, and a function receiving one doesn't declare `mutates`.
- **They are declared `mutable`** where they are created, like every value that changes: declaring one with `let` is an error.
- **They hold owned data.** The `T` of a `Mutex<T>` or a `Channel<T>` can't contain references or relations.

Immutable values need none of this: a value that no thread modifies can be read by every thread.

### Mutexes

A `Mutex` protects a value. Its handle is shared, but each guard returned by `lock()` or `tryLock()` is a unique owner of that acquisition: it moves, cannot be copied, and is borrowed in a parameter. `lock()` waits until the value is available, and returns a guard giving access to it:

```fa
use { Mutex } = std.concurrency

type Stats = {
	visits: Integer
	errors: Integer
}

async function recordVisit = (stats: Mutex<Stats>) => {
	do {
		mutable guard = await stats.lock()
		guard.value.visits += 1
	} -- the lock is released here, when `guard` leaves its scope
}

async function main = () => {
	mutable stats = Mutex<Stats> { value = { visits = 0, errors = 0 } }

	-- both tasks share `stats`, which is allowed because `Mutex` is thread-safe
	await Task.all([
		start recordVisit(stats)
		start recordVisit(stats)
	])
}
```

A guard is an ordinary value that owns the lock: it is released at the end of its scope, by the same rule as every other value. `guard.value` is a reference to the protected value, valid while the guard lives; it is written through a `mutable` guard. There is no `unlock`: keep the scope of a guard as small as possible.

`lock()` is async: between tasks of the same thread, it waits without blocking the thread, and a task may `await` while it holds a guard. `tryLock()` returns the guard without waiting, or `none` when the mutex is locked.

### Atomics

Atomics are thread-safe numbers and booleans, modified without a lock:

```fa
use { AtomicInteger, Ordering } = std.concurrency

mutable counter = AtomicInteger { value = 0 }

counter.increment()
let count = counter.load()
counter.store(42)
let swapped = counter.compareAndSwap(42, 100) -- `true` if the value was 42

-- With an explicit memory ordering
counter.increment(ordering = Ordering.relaxed)
counter.store(42, ordering = Ordering.sequential)
```

| Member | |
| --- | --- |
| `load()`, `store(value)` | read and write the value |
| `increment()`, `decrement()`, `add(amount)` | `AtomicInteger` only; return the new value |
| `swap(value)` | writes the value and returns the previous one |
| `compareAndSwap(expected, value)` | writes `value` if the current value is `expected`; returns whether it did |

Every method takes an optional `ordering`: `Ordering.relaxed`, `Ordering.acquire`, `Ordering.release`, `Ordering.acquireRelease` or `Ordering.sequential` (the default).

## Requirements and concurrency

[Requirements](/advanced/requirements) cross `start`: a task started in the background runs on the same thread, and sees the values provided where it was started. A task uses its requirements like its arguments, so a task that is kept can't outlive the block that provides them, and a detached task can't use requirements.

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

The body of a parallel loop can read the requirements, but not modify them unless they are thread-safe.

Requirements do **not** cross worker boundaries. A worker only receives copies of the arguments it is given: to give it a value, pass it as an argument.

## Reactivity and concurrency

[Reactive values](/advanced/reactivity) belong to the thread that declares them. Tasks of that thread can read and write them; pending `watch` blocks run before a task suspends at an `await`. A reactive value can't be read or written in a parallel loop, and a worker only receives a plain copy of it.

## What each target supports

| Feature | `native` | `wasm` | JavaScript (`web`, `bun`, `node`) |
| --- | --- | --- | --- |
| `async` / `await` / `start` | yes | yes | yes |
| Blocking calls | see [the primitives](#blocking-calls-and-targets) | see [the primitives](#blocking-calls-and-targets) | see [the primitives](#blocking-calls-and-targets) |
| Workers, channels | yes | yes | yes (Web Workers) |
| `for parallel`, `parallel()` | yes | no | no |
| `Mutex`, atomics between tasks | yes | yes | yes |
| `Mutex`, atomics, channels shared with a worker | yes | with shared memory, flat values only | with `SharedArrayBuffer`, flat values only |

A flat value contains no collection, string or recursive union: on JavaScript and WebAssembly, only such a value can live in memory shared between threads.

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

On the web, sharing these handles with a worker requires cross-origin isolation: serve `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Without it, passing a shared handle stops the program and names the required headers.

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

5. **Cancel what you start**: a `Task` you no longer need should be cancelled, rather than left to run until the end of its scope.

6. **Test concurrent code under load**: deterministic tests catch logic errors, stress tests catch contention. Test with different numbers of workers, and verify that resources are properly released.

:::implementation
## Implementation details

### Instances of an `async` function

The checker requests, for each `async` function, a synchronous instance, an asynchronous instance, or both, depending on how it is called; each instance is generated only if requested, and cached separately. A non-async function has one instance. The checker computes, per target, whether the synchronous instance can exist (its call chain reaches no async-only primitive, and on JavaScript no channel, lock, worker call or `await` of an outside task); this set of targets is part of the function's interface and of its interface hash.

- **Synchronous instance**: `await f()` becomes a call of `f`'s synchronous instance; `await task` returns the stored result of a task started in the same blocking call; `start f()` runs `f`'s synchronous instance immediately and builds a finished task (result or error). On native, waiting on a channel, a mutex or a worker parks the thread.
- **Blocking mode** is dynamic: a non-async function called during a blocking call may contain `start`. A thread-local "blocking depth" counter, incremented around each blocking call of an `async` function, makes `start` run eagerly. The checker requires, for a non-async function that contains `start f()` and can be reached during a blocking call on a target, that `f` can be called blocking on that target.
- **Function values** of an `async` function type carry both entry points (a pair of pointers on native, two properties on JavaScript); an entry the target can't provide is a stub that is never called, since the checker rejects the blocking call. A non-async function stored in an `async` function type gets an asynchronous entry that calls it and returns a finished task.
- **Primitives**: the standard library declares, per target and with `@if target`, a synchronous and an asynchronous implementation of each primitive, through an internal intrinsic reserved to the standard library. A missing implementation is what makes a blocking call a compile error.

### Tasks

- A task frame outlives the call that started it: it is a heap block owned by the `Task` value (move-only), freed when the task is awaited, cancelled or destroyed. A detached task owns its frame, registered in the thread's executor.
- Each thread runs one executor: a ready queue of tasks. `start` pushes the task on the queue; the queue runs when the current code suspends or returns to the event loop. The program's exit waits for the detached tasks of the main thread.
- **Cancellation** destroys the frame at its current suspension point: the generated code runs the scope-exit destruction of every live value of the frame (the same code as a normal scope exit), then cancels the tasks the frame owns. Since only one task of a thread runs at a time, `cancel()` is synchronous.
- `Task.all`, `Task.any`, `Task.settled` are runtime functions that take ownership of the frames and register a waker on each.

### Native

The strategy is detailed in `design/compilation-with-llvm.md`, and decided for async in `design/async.md` (LLVM switched-resume coroutines, epoll first, a blocking pool, a 4-ary timer heap, a coop budget): where they differ, `design/async.md` wins.

- **Async instance**: a resumable state machine, generated with LLVM coroutines (switched-resume lowering) or by hand. The runtime (`fa-runtime`) wraps it in a task with a poll / wake / drop ABI. Each thread runs the runtime's own single-threaded executor (a ready queue and a timer wheel), so tasks never move between threads and their frames need no synchronization. Frames come from a per-thread size-class allocator. I/O goes through `io_uring` on Linux, `kqueue` on macOS and the BSDs, and IOCP on Windows.
- **`for parallel`**: the body is outlined into a function, and the loop becomes `fa_parallel_for(count, body, env)`, where `body` runs a range of indices: the runtime's fork-join pool (one thread per core, work-stealing deques) splits the range lazily, and the calling thread takes part. The proof that writes are disjoint is what makes the outlined body free of synchronization. Errors are collected per chunk; the smallest failing index wins.
- **Workers**: a thread with its own executor and its own copy of the immutable module-level values. A call is a message (the copied arguments and a completion slot) on the worker's queue, and the result is copied back.
- **Thread-safe handles**: reference-counted (atomic counts) heap blocks. `Mutex` is an async-aware lock (a waiter queue that wakes tasks of any thread); atomics are LLVM atomic instructions with the requested ordering.

### JavaScript

- **Async instance**: `name___async`, an `async function` of JavaScript taking its task first (`name___async($task, …)`); `await` becomes `await`. **Sync instance**: `name___sync`, a plain function. Each is emitted only when a call needs it, and a function with one instance keeps its plain name; an exported one is also a Promise API (`name`) for JavaScript callers, and the other modules import `name$sync` and `name$async`. On `bun` and `node`, the blocking primitives map to the synchronous APIs (`readFileSync`) or, for timers, to `Atomics.wait` on a private `SharedArrayBuffer`. An `await` of a mutex or a channel that is ready doesn't `await` at all.
- **`start`** binds written arguments and omitted defaults immediately in source order, with cleanup if binding fails; only the body begins in a microtask (`faStart`). A synchronous default-entry/portable owner recipe keeps defaults separate from the scheduled body. This is the specified creation timing on every target; the legacy JavaScript binder must be updated in ownership slice O24. The `Task` object holds its outcome (a value, a Fa error, or cancelled), the end of its current wait, and the tasks it owns.
- **Detached calls** store arguments and captures at the `start`, including fields, indexed values, spread elements and method receivers. Ordinary values copy, structures move or need visible `copy`, and unique captures are forbidden. Only the started call owns these stored values: nested calls evaluating its arguments retain ordinary borrowing. Worker transport already snapshots its arguments and needs no additional copy. A `start do` uses the analysis's capture list. Synchronization handles keep their shared identity; module-level values are read by the block rather than captured.
- **Guards** bound to locals unlock at block exit. An awaited `lock()` or a `tryLock()` used temporarily, including as a positional or named argument, unlocks at the end of its statement. A generated `try`/`finally` also releases it if evaluation throws or leaves the scope; a binding initialized inside that `try` is declared outside it. A fresh guard result whose value is discarded is passed directly to `faUnlock` at the discard point.
- **Cancellation**: `cancel()` marks the task, ends its current wait (a timer is cleared, a place in a line is left) and cancels the tasks it owns, synchronously; the waits of the runtime check the mark when they resume and throw a signal that no `catch` keeps, so that only the `finally` blocks of the task (its scope exits) run. Errors are Fa values, never rejected promises.
- **Coop budget**: an `await` that resolves without waiting counts, and every 128 of them a macrotask lets timers and I/O run; the loops of async instances check the clock every 128 iterations and give the thread back once the current macrotask lasted 4 ms.
- **WebAssembly**: the same coroutines as natively; the host's event loop drives the executor (`fa_run`, run from a macrotask while tasks are ready, and when the timer the program asked for with `fa.timer` fires).
- **Workers**: Web Workers on `web`, `worker_threads` on `node`, `Worker` on `bun`; each worker file is a separate bundle. Arguments and results use generated independent-copy or exclusive-transfer operations; no Task is transferable. Shared synchronization handles retain category identity (never `structuredClone` on Fa objects). A channel, mutex or atomic shared with a worker lives in a `SharedArrayBuffer`. Production payloads use build-time generated binary codecs and category ownership operations behind a lock and a version word: a task waits with `Atomics.waitAsync`, a blocking worker with `Atomics.wait`, and updates notify waiters. Mutexes use the same locking scheme. The legacy JSON payload path is superseded and is replaced by ownership slices O30–O32; no runtime schema interpretation or function compilation is permitted.
- **Queued worker cancellation**: sending a call snapshots its arguments immediately in a prepared message. The caller commits one live call at a time, from a microtask, after the previous answer. Cancelling before that commit discards the prepared call; cancelling during an import is checked again before entering the method. This uses no extra argument copy or shared memory, and a method already executing may finish with its result discarded.
- `for parallel` and `parallel()` don't exist on JavaScript.

### Analysis

- A file depends on the target when it reads `target` or `configuration`, uses a primitive absent on some targets, or makes a blocking call unavailable on some targets. `await`, `start`, and calls that never wait do not themselves cause target dependence.

:::
