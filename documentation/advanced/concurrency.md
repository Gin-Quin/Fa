# Concurrency

Fa gives you three ways to do more than one thing at a time:

- **async functions**, for work that waits (I/O, network, timers),
- **parallel iteration**, for work that computes,
- **workers**, for long-running background tasks.

Because Fa targets both native code and JavaScript, some of these are target-specific. When compiling to native, Fa uses [Tokio](https://tokio.rs) for asynchronous work and [Rayon](https://docs.rs/rayon) for parallel iteration. When compiling to JavaScript, it uses the event loop and [Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API).

## Colourless async

In most languages, `async` divides your code in two: an async function can only be called from another async function, so you end up writing everything twice.

In Fa, a function marked `async` can be called from **any** context. The compiler generates whichever version is needed at each call site.

```fa
async function fetchData = (url: String): Response | HttpError => {
  return await httpClient.get(url)
}
```

`async` does not mean the function is always asynchronous. It means the function *may* suspend, and is therefore callable with `await` or `start`.

## Three ways to call

```fa
-- 1. Blocking: run it synchronously, waiting for the result
let data = fetchData("https://api.example.com/data")!
console.log("printed once the data has arrived")

-- 2. Awaited: suspend this task, let the runtime do other work
let data = await fetchData("https://api.example.com/data")!
console.log("printed once the data has arrived, without blocking the thread")

-- 3. Started: begin now, collect the result later
let pending = start fetchData("https://api.example.com/data")
console.log("printed immediately")
let data = await pending!
```

`start` returns a `Task(T)`, which you can await, cancel, or store.

```fa
let task = start slowOperation()

if userCancelled {
  task.cancel()
}
```

## Awaiting several tasks

Awaiting an array of tasks waits for all of them:

```fa
let responses = await [
  start fetchData(firstUrl)
  start fetchData(secondUrl)
]
```

Other combinators:

```fa
let first = await Task.any(tasks)      -- the first one to finish
let all = await Task.all(tasks)        -- every result, or the first error
let settled = await Task.settled(tasks) -- every result, errors included
```

## Errors

Async functions return errors the same way every other function does — as values. There is no `try`, no `catch`, and no rejected promise.

```fa
async function loadProfile = (id: Int): Profile | HttpError | ParseError => {
  let response = await fetchData("/users/{id}")!
  let profile = Json.parse(Profile, response.body)!
  return profile
}
```

Handle them with `when`, exactly as you would in synchronous code:

```fa
let profile = when await loadProfile(42) is {
  Profile >> value => value
  HttpError => defaultProfile
  ParseError >> error => {
    console.log("bad payload: {error.message}")
    defaultProfile
  }
}
```

## Parallel iteration

**Native target only.**

`parallel` spreads a loop's iterations across CPU cores:

```fa
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

-- sequential
mutable results = Array(Int, capacity = numbers.length) {}
for numbers >> number {
  results.push(expensiveComputation(number))
}

-- parallel: each iteration writes to its own slot
mutable results = Array(Int, length = numbers.length) { }
for parallel numbers >> number, index {
  results[index] = expensiveComputation(number)
}
```

Control the number of threads:

```fa
for parallel(threads = 2) numbers >> number, index {
  results[index] = expensiveComputation(number)
}
```

Mapping in parallel:

```fa
let doubled = numbers.map(parallel (x: Int) => x * 2)
```

The compiler rejects a parallel loop whose body writes to a shared value in an unsynchronised way — including `results.push`, which is why the example above writes to an index instead.

## Workers

A worker is a thread with its own state, defined in a file ending in `.worker.fa`. Like every Fa file, a worker file has a [single export](../architecture/modules).

```fa
-- mathWorker.worker.fa
export = {
  numbers: Array(Int)

  initialize(numbers: Array(Int)): Self => Self { numbers }

  sum(self): Int => self.numbers.reduce((a: Int, b: Int) => a + b, 0)

  average(self): Decimal => self.sum() / self.numbers.length

  async processLarge(self, data: Array(Int)): Array(Int) => data.map((x: Int) => x * x)
}
```

Use it like any other type. Every method call runs on the worker's thread:

```fa
function main = () => {
  mutable worker1 = mathWorker([1, 2, 3, 4, 5])
  mutable worker2 = mathWorker([6, 7, 8, 9, 10])

  let sums = await [
    start worker1.sum()
    start worker2.sum()
  ]

  console.log("Total: {sums[0] + sums[1]}")

  worker1.stop()
  worker2.stop()
}
```

Arguments and results are **copied** across the thread boundary. This is not a restriction imposed for concurrency — it is Fa's ordinary [copy-on-assignment](./memory-management) rule, which is what makes workers safe by construction. References and relations cannot cross a worker boundary, and the compiler says so.

### Worker pools

```fa
use { WorkerPool } = std.concurrency

function main = () => {
  mutable workers = WorkerPool(dataProcessor, count = 4)

  let tasks = [
    [1, 2, 3]
    [4, 5, 6]
    [7, 8, 9]
    [10, 11, 12]
  ]

  let results = await tasks.map((data: Array(Int)) =>
    workers.submit((worker: dataProcessor) => worker.process(data))
  )

  console.log("All results: {results}")

  workers.shutdown()
}
```

With more configuration:

```fa
mutable pool = WorkerPool(
  dataProcessor
  count = 8
  maxQueueSize = 1000
  timeout = 30000 -- milliseconds
)
```

## Channels

A channel moves values between tasks.

```fa
use { Channel } = std.concurrency

async function producer = (mutable channel: Channel(String)) => {
  for 1..=100 >> index {
    await channel.send("Item {index}")!
    await sleep(100)
  }
  channel.close()
}

async function consumer = (mutable channel: Channel(String), id: String) => {
  while await channel.receive() >> item {
    console.log("Consumer {id} received: {item}")
    await sleep(50)
  }
}

async function main = () => {
  mutable channel = Channel(String, capacity = 10)

  await [
    start producer(channel)
    start consumer(channel, "1")
    start consumer(channel, "2")
  ]
}
```

`receive` returns an optional, so the `while ... >>` loop ends when the channel is closed. See [extracting in a while](../control-flow/loops#extracting-in-a-while).

### Fan-out and fan-in

```fa
async function fanOut = (Item: Type, input: Array(Item), workers: Int(min = 1)): Array(Channel(Item)) => {
  mutable channels = Array(Channel(Item), capacity = workers) {}

  for 0..workers {
    channels.push(Channel(Item, capacity = 16))
  }

  for input >> item, index {
    await channels[index %% workers].send(item)!
  }

  for channels >> channel {
    channel.close()
  }

  return channels
}

async function fanIn = (Item: Type, channels: Array(Channel(Item))): Channel(Item) => {
  mutable output = Channel(Item, capacity = 16)

  for channels >> channel {
    start async {
      while await channel.receive() >> item {
        await output.send(item)!
      }
    }
  }

  return output
}
```

## Shared state

Prefer messages to shared memory. When you do need it:

```fa
use { Mutex, AtomicInt } = std.concurrency

mutable counter = AtomicInt(0)
mutable sharedData = Mutex({ value = 0 })

async function incrementCounter = () => {
  counter.increment()

  do {
    use guard = await sharedData.lock()
    guard.value += 1
  } -- the lock is released here, when `guard` leaves scope
}
```

```fa
let count = counter.load()
counter.store(42)
counter.compareAndSwap(42, 100)

counter.increment(Ordering.relaxed)
counter.store(42, Ordering.sequential)
```

A `Mutex` guard is an ordinary value, so it is released at the end of its scope by the same rule as everything else. There is no `unlock`.

## What each target supports

| Feature | Native | WebAssembly | JavaScript |
| --- | --- | --- | --- |
| `async` / `await` / `start` | yes | yes | yes |
| Workers, channels | yes | yes | yes (Web Workers) |
| `for parallel` | yes | no | no |
| `Mutex`, atomics | yes | with shared memory | with `SharedArrayBuffer` |

Code using an unavailable feature fails to compile for that target rather than silently degrading. Check `@target` when you need to branch:

```fa
@if @target is "native" {
  for parallel items >> item, index { results[index] = process(item) }
} @else {
  for items >> item, index { results[index] = process(item) }
}
```

## Guidelines

1. **Match the tool to the work.** `async` for waiting, `for parallel` for computing, workers for long-running background tasks.
2. **Send messages rather than sharing state.** Fa's copy semantics make this the path of least resistance.
3. **Bound your concurrency.** Use a worker pool rather than spawning without limit, and give channels a capacity so a slow consumer applies back-pressure.
4. **Cancel what you start.** A `Task` you no longer need should be cancelled, not abandoned.
5. **Test under load.** Deterministic tests catch logic errors; stress tests catch contention.
