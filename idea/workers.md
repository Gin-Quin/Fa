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

A worker is defined in a `{workerName}.worker.fa` file. It must export either:

- a function,
- or a module containing multiple functions (in this case, these functions are called **actions**).

Example:

```fa
// add.worker.fa
export (a: Number, b: Number) => a + b
```

Every function exported by a worker is considered **asynchronous**, even if it's a synchronous function.

Like every asynchronous function, you have to either `await` or explicitly `run` an action.

:::tip
The `await` keyword means we want to wait for the action to finish before continuing.
The `run` keyword means we want to run the action in the background and continue immediately.
:::

```fa
// main.fa
sum = await add(1, 2); // the sum computation is done in another thread
```
