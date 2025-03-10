# Workers

Like any Javascript runtime, Fa uses an event loop to handle **concurrency**.

To allow **parallel execution**, Fa uses **workers** (aka threads).

:::tip
*Concurrency* and *parallelism* are two different ways to achieve asynchronous execution.

**Parallelism** is about using the multiple cores of a CPU to do multiple works simultaneously (with one work not preventing the other ones from being done).

In Fa, parallelism is achieved using **workers** (aka threads) or by running sub-processes.

**Concurrency** is about allowing a task to be interrupted and resumed later, to allow other tasks to run in the meantime.

Concurrency is achieved using an event loop.

Unlike parallelism, concurrency doesn't require multiple cores.
:::

Fa can handle:

- One-shot actions (like `add(1, 2)`), using ephemeral `worker pools`,
- Long-running threads, using persistent `workers`.

## Workers

In Fa, any module can be treated as a worker using the `Worker` or `WorkerPool` constructor.

Example:

```fa
// /workers/math.fa
export module {
  add = (a: Number, b: Number) => a + b
}
```

```fa
// /main.fa
export () => {
  // create a new worker from the math module
  mathWorker = Worker(workers.math)
  
  // call the `add` action on the worker
  // we have to use an "await" because we're using a worker
  // so the action is asynchronous
  let sum = await mathWorker.add(1, 2)
  console.log(sum)

  // we can close the worker manually
  // but it will also close automatically when the worker is out of scope
  mathWorker.close()
}
```

Every function exported by a worker is considered **asynchronous**, even if it's a synchronous function.

Like every asynchronous function, you have to either `await` or explicitly `run` an action.

:::tip
The `await` keyword means we want to wait for the action to finish before continuing.
The `run` keyword means we want to run the action in the background and continue immediately.
:::

## Worker pools

You can create a pool of workers from a worker module the same way you create a single worker. The difference is that a pool of workers can grow and shrink dynamically the number of treads used (even shrinking to zero threads if no task is available).

```fa
// /main.fa
export () => {
  // create a worker pool from the math module
  mathWorkerPool = WorkerPool(workers.math)

  // run the actions in parallel
  use await all [
    mathWorkerPool.add(1, 2)
    mathWorkerPool.add(3, 4)
    mathWorkerPool.add(5, 6)
    mathWorkerPool.add(7, 8)
    mathWorkerPool.add(9, 10)
    mathWorkerPool.add(11, 12)
  ] >> results

  console.log(results)
}
```

**Worker pools** are intended to be used for CPU-bound tasks. You should keep I/O operations (like reading files or making network requests) on the main thread, and pass the results to the worker pool.


## Using Javascript workers

This syntax is quite close to the one used in Javascript. There is some sugar to make it easier to use, but not much. Javascript does not have a native concept of worker pools, but it's possible to create one by hand.


## Native parallelism

For the **native and wasm platforms only**, Fa introduces two other kinds of parallelism using the `parallel` keyword:

- **Parallel iterators** with the `for parallel` keyword.
- **Parallel scopes** with the `parallel {}` block.

When exporting to Javascript, the `parallel` keyword is ignored, and the iteration will be done in the main thread.

:::tip
Because native parallelism is not available for all platforms, you should consider first using the `Worker` or `WorkerPool` syntax. Only use native parallelism if:

- you want to target primarily native platforms,
- you verified via benchmarks that native parallelism is faster than worker pools.

Internally, Fa uses the **rayon** crate to achieve native parallelism.
:::

### Parallel iterators

```fa
// this will print the numbers 0 to 99 in random order.
for parallel 0..100 >> (index) {
    println(index)
}
```

### Parallel scopes

A `parallel scope` is a block of code prefixed by the `parallel` keyword.

Inside a parallel scope, you can define `run` blocks that will be executed in parallel.

The parallel block will wait for all the `run` blocks to finish before finishing itself.

```fa
// this will print the numbers 0 to 99 in random order.
parallel {
  // start running a block of code
  run {
    expansiveTask()
    followUpTask()
  }

  // instantly start running another block of code
  run {
    anotherExpansiveTask()
  }
}

console.log("All runs finished")
```
