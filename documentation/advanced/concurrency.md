# Concurrency

Concurrency is a first-class citizen in Fa, designed to help you build high-performance applications that can handle multiple tasks simultaneously. As a language that strives to be among the top performers, Fa provides elegant and powerful concurrency primitives that abstract away the complexity while maintaining excellent performance characteristics.

## What is Concurrency?

Concurrency is the ability to handle multiple tasks at the same time, though not necessarily simultaneously. It allows your program to make progress on multiple fronts, improving responsiveness and throughput. Concurrency can be achieved through:

- **Threads**: Multiple execution contexts running in parallel on different CPU cores
- **Asynchronous operations**: Non-blocking operations using an event loop in a single thread

Since Fa transpiles to both Rust and JavaScript, some concurrency methods are available only for specific targets, allowing you to leverage the strengths of each platform.

When targeting Rust, Fa uses [Tokio](https://tokio.rs), a popular asynchronous runtime for Rust that provides a robust set of tools for building concurrent applications.

When targeting Javascript, Fa uses a custom library built on top of [Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API), which allows for efficient parallelism across multiple threads.

## Async Programming in Fa

One of Fa's most powerful features is its **colorless async** design. Unlike many languages that distinguish between synchronous and asynchronous functions (creating "colored" functions), Fa allows any function defined as `async` to be called in both synchronous and asynchronous contexts.

### Why Colorless is Better

Colorful async functions can force the developer to write two versions of the same function, one synchronous and one asynchronous. This can lead to code duplication and increased complexity.

Fa handles this natively, allowing any function (async or not) to be called in both synchronous and asynchronous contexts, by generating the synchronous and/or asynchronous version of the function when used.

### Defining Async Functions

Like in **JavaScript** and **Rust**, a function that *can be asynchronous* is defined with the `async` keyword.

**This does not mean that the function is always asynchronous**, as it can be called as a synchronous function. It only means that the function is callable in an asynchronous context, i.e. with the `await` or `run` keyword.

```fa
-- Async function with explicit await
async function fetchData(url: String): Response {
    response = await httpClient.get(url)
    return response
}

-- Async function with implicit await (Fa will handle it)
async function fetchData(url: String): Response {
    return httpClient.get(url)
}

-- Calling async functions
let data = fetchData("https:--api.example.com/data")  -- Blocks in sync context
let data = await fetchData("https:--api.example.com/data")  -- Non-blocking in async context
```

### Calling Async Functions

A function defined as `async` can be called in three possible ways:

1. **Blocking**: The function is called synchronously, blocking the current thread until it completes.
2. **Awaited**: The function is called asynchronously with the `await` keyword, allowing the current thread to continue executing other tasks while waiting for the function to complete.
3. **Running**: The function starts executing asynchronously immediately with the `run` keyword, and the caller continues executing the next statements without waiting for the function to complete.

```fa
async function fetchData(url: String): Response {
    response = await httpClient.get(url)
    return response
}

-- 1. Running synchronously
data = fetchData("https:--api.example.com/data")

console.log("This will be printed once data is fetched")

-- 2. Awaited
await fetchData("https:--api.example.com/data")

console.log("This will be printed once data is fetched, but without blocking the current thread")

-- 3. Running
dataFetching = start fetchData("https:--api.example.com/data")

console.log("This will be printed immediately, before the data is fetched")

data = await dataFetching
```

## Multithreading in Fa

Fa provides several approaches to achieve multithreading, each suited for different use cases:

### 1. Parallel Iterations

**Available only when transpiling to Rust**

Use the `parallel` keyword in for loops to automatically parallelize iterations using the Rayon library internally:

```fa
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let results = []

-- Sequential processing
for number in numbers {
    results.push(expensiveComputation(number))
}

-- Parallel processing - automatically uses available CPU cores
for parallel number in numbers {
    results.push(expensiveComputation(number))
}

-- Parallel with custom chunk size
for parallel(threads = 2) number in numbers {
    results.push(expensiveComputation(number))
}

-- Parallel mapping
let doubled = numbers.map(parallel (x) => x * 2)
```

### 2. Worker Files

Workers are instantiable threads defined in files with the `.worker.fa` extension. They provide a clean way to encapsulate thread logic and can be imported and instantiated like any other type.

#### Defining a Worker

```fa
-- mathWorker.worker.fa
type MathWorker = {
    numbers: Array<Integer>
}

-- Worker initialization
function initialize(numbers: Array<Integer>): MathWorker {
    return MathWorker { numbers }
}

-- Worker methods
function sum(self): Integer {
    return self.numbers.reduce((a, b) => a + b, 0)
}

function average(self): Float {
    return self.sum() / self.numbers.length
}

-- Async worker method
async function processLarge(self, data: Array<Integer>): Array<Integer> {
    return data.map(x => x * x)
}
```

#### Using Workers

```fa
import MathWorker from "./mathWorker.worker.fa"

function main() {
    numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

    -- Create worker instances
    worker1 = MathWorker([1, 2, 3, 4, 5])
    worker2 = MathWorker([6, 7, 8, 9, 10])

    -- Call worker methods (these run in separate threads)
    sums = await [
        worker1.sum()
        worker2.sum()
    ]

    -- Async worker calls
    result1 = await worker1.processLarge([100, 200, 300])
    result2 = await worker2.processLarge([400, 500, 600])

    print("Total sum: " + (sum1 + sum2))
    print("Processed results: " + result1 + result2)
}
```

### 3. Worker Pool

For managing multiple workers efficiently, Fa provides a built-in worker pool:

```fa
import { WorkerPool } from "std/concurrency"
import DataProcessor from "./dataProcessor.worker.fa"

function main() {
    -- Create a worker pool with 4 workers
    workers = WorkerPool(DataProcessor, count = 4)

    tasks = [
        { data = [1, 2, 3] }
        { data = [4, 5, 6] }
        { data = [7, 8, 9] }
        { data = [10, 11, 12] }
    ]

    -- Submit tasks to the pool
    futures = tasks.map(task => workers.submit(worker => worker.process(task.data)))

    -- Wait for all tasks to complete
    results = await futures

    print("All results: " + results)

    -- Clean up
    workers.shutdown()
}

-- Advanced pool configuration
pool = WorkerPool(
		of = DataProcessor
    count = 8
    maxQueueSize = 1000
    timeout = 30000 -- 30 seconds
)
```

## Concurrency Patterns

### Producer-Consumer Pattern

```fa
import { Channel } from "std/concurrency"

async function producer(channel: Channel(String)) {
    for i in 1..100 {
        await channel.send("Item " + i)
        await sleep(100)  -- Simulate work
    }
    channel.close()
}

async function consumer(channel: Channel(String), id: String) {
    while let Some(item) = await channel.receive() {
        print("Consumer " + id + " received: " + item)
        await sleep(50)  -- Simulate processing
    }
}

async function main() {
    channel = Channel(String) { capacity = 10 }

    -- Start producer and consumers
    producerTask = producer(channel)
    consumer1 = consumer(channel, "1")
    consumer2 = consumer(channel, "2")

    -- Wait for all to complete
    await [producerTask, consumer1, consumer2]
}
```

### Fan-out/Fan-in Pattern

```fa
async function fanOut<T>(input: Array<T>, workers: Integer): Array<Channel<T>> {
    channels = []
    for i in 0..workers {
        channels.push(Channel<T>())
    }

    for parallel (index, item) in input.enumerate() {
        channelIndex = index modulo workers
        await channels[channelIndex].send(item)
    }

    -- Close all channels
    for channel in channels {
        channel.close()
    }

    channels
}

async function fanIn<T>(channels: Array<Channel<T>>): Channel<T> {
    output = Channel<T>()

    for channel in channels {
        -- Start a task for each input channel
        spawn async {
            while item = await channel.receive()? {
                await output.send(item)
            }
        }
    }

    output
}
```

## Thread Safety and Synchronization

### Mutexes and Atomic Operations

```fa
import { Mutex, AtomicInteger } from "std/concurrency"

let counter = AtomicInteger(0)
let sharedData = Mutex({ value: 0 })

async function incrementCounter() {
    -- Atomic operations are thread-safe
    counter.increment()

    -- Mutex for more complex operations
    guard = await sharedData.lock()
    guard.value += 1
    -- Automatically unlocked when guard goes out of scope
}

-- Using atomic operations
let count = counter.load()
counter.store(42)
counter.compareAndSwap(42, 100)

-- Atomic operations with memory ordering
counter.increment(Ordering.Relaxed)
counter.store(42, Ordering.SeqCst)
```

## Platform-Specific Considerations

### Rust Target

When transpiling to Rust, you have access to:
- Parallel iterations with `parallel` keyword
- Full threading capabilities
- Zero-cost abstractions
- Native performance

### JavaScript Target

When transpiling to JavaScript, concurrency is handled through:
- Web Workers for true parallelism
- Async/await for non-blocking operations
- SharedArrayBuffer for shared memory (where available)
- Automatic fallback to async patterns when threading is not available

## Best Practices

1. **Use the right tool for the job**:
   - Parallel iterations for CPU-bound data processing
   - Workers for long-running background tasks
   - Async/await for I/O-bound operations

2. **Avoid shared mutable state**:
   - Prefer message passing over shared memory
   - Use immutable data structures when possible
   - Utilize Fa's built-in synchronization primitives

3. **Handle errors gracefully**:
   ```fa
   async function safeOperation() {
       try {
           result = await riskyAsyncOperation()
           return Ok(result)
       } catch error {
           return Err(error)
       }
   }
   ```

4. **Monitor resource usage**:
   - Don't create unlimited workers
   - Use worker pools for better resource management
   - Set appropriate timeouts for operations

5. **Test concurrent code thoroughly**:
   - Use stress testing to find race conditions
   - Test with different numbers of workers
   - Verify proper cleanup of resources

Fa's concurrency model provides the tools you need to build fast, scalable applications while maintaining code clarity and safety. Whether you're processing large datasets, building web servers, or creating real-time applications, Fa's concurrency features will help you achieve optimal performance.
