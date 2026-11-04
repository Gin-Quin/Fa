// Workers on JavaScript (design/async.md §3.4, step J2): a worker is a
// thread of its own, `worker_threads` on Bun and Node, a Web Worker on the
// web, and its calls are messages.
//
// - `MathWorker { … }` starts a thread running `$worker.js` (written next to
//   `main.js`): it initializes the module-level values of the program, then
//   imports the worker's module and keeps the state it was sent
//   (`faWorkerServe`). The state, the arguments and the results cross by
//   structured clone: they are copies, as natively.
// - Its calls run one at a time, in the order they were sent (A9), with the
//   synchronous instance of the method (`Type$method$sync` when it may
//   suspend): an `await` in the worker's code waits there.
// - A call is awaited (`faWorkerCall`) or started (`faWorkerStart`: the
//   message is sent at once, so later changes of the arguments don't reach
//   it); a blocking call is `CONC010` (only threads of `native` can wait).
//   Cancelling a call that hasn't run yet removes it from the worker's line;
//   a call that runs ends there, and its result is dropped.
// - A Fa error thrown by a call is rebuilt with its class (the classes the
//   method may throw are given by the call); a stop in a worker ends the
//   program, as natively.
// - `stop()` ends the thread once its calls are done; an idle worker
//   doesn't keep the process alive.
// - `WorkerPool(T) { count, state, maxQueueSize, timeout }`: `count` workers
//   made from copies of `state`; `await pool.submit(work)` waits while
//   `maxQueueSize` works are queued, and gives the task of the work, which
//   runs once a worker is free. With a `timeout`, a work that takes longer
//   fails with `WorkerTimeout`: its worker's thread is terminated and
//   replaced by one made from a fresh copy of `state`. The submitted
//   function runs on the main thread; its calls of the worker run on the
//   worker's thread.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { FaError } from "./errors.js";
import { faExit, faIsError, faWriteError } from "./main.js";
import { FaStop, faStopMessage } from "./stops.js";
import {
	faAlive,
	faCallValue,
	faCancelSignal,
	faEnd,
	faRunTask,
	faSettle,
	FaTask,
	faWhenDone,
} from "./tasks.js";

// @fa-helper WorkerTimeout deps: FaError
/** A work of a worker pool that took longer than the pool's `timeout`. */
export class WorkerTimeout extends FaError {
	static faName = "WorkerTimeout";
}

// @fa-helper FaWorker deps: faExit faWriteError
/** A worker: its thread, and the calls sent to it that haven't answered. */
export class FaWorker {
	/**
	 * Starts the thread of a worker.
	 * @param {any} state the worker's state (copied to its thread)
	 */
	constructor(state) {
		/** @type {Map<number, (answer: any) => void>} */
		this.calls = new Map();
		/** @type {number} */
		this.next = 0;
		/** @type {boolean} */
		this.stopping = false;
		/** @type {boolean} */
		this.dead = false;
		/** @type {any} */
		const host = globalThis;
		const threads = host.process?.getBuiltinModule?.("node:worker_threads");
		const url = new URL("./$worker.js", import.meta.url);
		/** @param {any} answer */
		const receive = (answer) => {
			if (answer.stop !== undefined) {
				faWriteError(answer.stop);
				faExit(101);
				host.process?.exit?.(101);
				return;
			}
			const resolve = this.calls.get(answer.id);
			if (resolve !== undefined) {
				this.calls.delete(answer.id);
				resolve(answer);
			}
			this.settled();
		};
		if (threads !== undefined) {
			this.thread = new threads.Worker(url);
			this.thread.on("message", receive);
			this.thread.on("error", (/** @type {any} */ error) => {
				faWriteError(`stop: a worker failed: ${error?.message ?? error}`);
				host.process?.exit?.(101);
			});
		} else {
			this.thread = new host.Worker(url, { type: "module" });
			this.thread.onmessage = (/** @type {any} */ event) => receive(event.data);
		}
		this.thread.postMessage({ kind: 0, state });
		this.thread.unref?.();
	}

	/**
	 * Sends a call; `answer` receives `{ value }`, `{ error }` or `{ cancelled }`.
	 * @param {string} method `<URL of the module>#<export of the method>`
	 * @param {any[]} args
	 * @param {(answer: any) => void} answer
	 * @returns {number} the call's id
	 */
	send(method, args, answer) {
		const id = this.next++;
		if (this.dead) {
			answer({ id, cancelled: true });
			return id;
		}
		if (this.calls.size === 0) {
			this.thread.ref?.();
		}
		this.calls.set(id, answer);
		this.thread.postMessage({ kind: 1, id, method, args });
		return id;
	}

	/** A call answered: an idle worker lets the process end, a stopped one ends. */
	settled() {
		if (this.calls.size > 0) {
			return;
		}
		if (this.stopping) {
			this.terminate();
		} else {
			this.thread.unref?.();
		}
	}

	/** Ends the thread at once; the calls waiting for it are cancelled. */
	terminate() {
		if (this.dead) {
			return;
		}
		this.dead = true;
		this.thread.terminate();
		const calls = [...this.calls.values()];
		this.calls.clear();
		for (const resolve of calls) {
			resolve({ cancelled: true });
		}
	}
}

// @fa-helper faWorkerError deps: FaError
/**
 * A Fa error thrown by a worker's call, rebuilt on this thread: an instance
 * of its class among `classes` (the errors the method may throw), or of a
 * class of the same name.
 * @param {{ name: string, fields: Record<string, any> }} error
 * @param {Function[]} classes
 * @returns {any}
 */
export function faWorkerError(error, classes) {
	/** @type {any} */
	let found = classes.find((/** @type {any} */ type) => type.faName === error.name);
	if (found === undefined) {
		found = class extends FaError {
			static faName = error.name;
		};
	}
	return Object.assign(Object.create(found.prototype), error.fields);
}

// @fa-helper faWorkerAnswer deps: faWorkerError faCancelSignal
/**
 * The outcome of an answered call: its value, or its error thrown.
 * @param {any} answer
 * @param {Function[]} classes
 * @returns {any}
 */
export function faWorkerAnswer(answer, classes) {
	if (answer.cancelled) {
		throw faCancelSignal;
	}
	if (answer.error !== undefined) {
		throw faWorkerError(answer.error, classes);
	}
	return answer.value;
}

// @fa-helper faWorkerCall deps: faWorkerAnswer faAlive
/**
 * `await worker.method(…)`: the call runs on the worker's thread while the
 * other tasks run.
 * @param {FaTask} task
 * @param {FaWorker} worker
 * @param {string} method
 * @param {any[]} args
 * @param {Function[]} classes the errors the method may throw
 * @returns {Promise<any>}
 */
export async function faWorkerCall(task, worker, method, args, classes) {
	/** @type {any} */
	let answer;
	await new Promise((resolve) => {
		const id = worker.send(method, args, (/** @type {any} */ received) => {
			answer = received;
			resolve(undefined);
		});
		task.wake = () => {
			worker.calls.delete(id);
			worker.thread.postMessage({ kind: 2, id });
			worker.settled();
			resolve(undefined);
		};
		task.where = "a worker";
	});
	task.wake = null;
	faAlive(task);
	return faWorkerAnswer(answer, classes);
}

// @fa-helper faWorkerStart deps: FaTask faWorkerAnswer faSettle
/**
 * `start worker.method(…)`: the message is sent at once (the arguments are
 * copied now), and the task ends with the call's answer.
 * @param {FaTask | null} parent
 * @param {FaWorker} worker
 * @param {string} method
 * @param {any[]} args
 * @param {Function[]} classes
 * @returns {FaTask}
 */
export function faWorkerStart(parent, worker, method, args, classes) {
	const task = new FaTask(parent);
	const id = worker.send(method, args, (/** @type {any} */ answer) => {
		let value;
		try {
			value = faWorkerAnswer(answer, classes);
		} catch (exception) {
			faSettle(task, exception, true);
			return;
		}
		faSettle(task, value, false);
	});
	// cancelled before it ran: the worker skips it
	task.wake = () => {
		worker.calls.delete(id);
		worker.thread.postMessage({ kind: 2, id });
		worker.settled();
	};
	return task;
}

// @fa-helper faWorkerStop
/**
 * `worker.stop()`: its thread ends once the calls sent are done.
 * @param {FaWorker} worker
 * @returns {void}
 */
export function faWorkerStop(worker) {
	worker.stopping = true;
	worker.settled();
}

// @fa-helper faWorkerServe deps: faIsError FaStop faStopMessage
/**
 * The thread of a worker (`$worker.js`): initializes the module-level
 * values (`init`), then runs the calls it receives one at a time.
 * @param {() => void} init
 * @returns {void}
 */
export function faWorkerServe(init) {
	/** @type {any} */
	const host = globalThis;
	const threads = host.process?.getBuiltinModule?.("node:worker_threads");
	const port = threads !== undefined ? threads.parentPort : host;
	/** @type {Map<string, any>} the modules of the methods called */
	const modules = new Map();
	/** @type {any} */
	let state;
	/** @type {Set<number>} */
	const cancelled = new Set();
	/** @type {Promise<void>} */
	let line = Promise.resolve();
	/** @param {any} message */
	const run = async (message) => {
		if (message.kind === 0) {
			init();
			state = message.state;
			return;
		}
		const id = message.id;
		if (cancelled.delete(id)) {
			port.postMessage({ id, cancelled: true });
			return;
		}
		// `<URL of the module>#<export of the method>`
		const split = message.method.lastIndexOf("#");
		const url = message.method.slice(0, split);
		const name = message.method.slice(split + 1);
		let module = modules.get(url);
		if (module === undefined) {
			module = await import(url);
			modules.set(url, module);
		}
		const method = module[`${name}$sync`] ?? module[name];
		try {
			const value = method(state, ...message.args);
			port.postMessage({ id, value });
		} catch (exception) {
			if (faIsError(exception)) {
				const error = /** @type {any} */ (exception);
				port.postMessage({ id, error: { name: error.constructor.faName, fields: { ...error } } });
			} else if (exception instanceof FaStop) {
				port.postMessage({ id, stop: faStopMessage(exception) });
			} else if (exception instanceof RangeError) {
				port.postMessage({ id, stop: "stop: stack overflow" });
			} else {
				throw exception;
			}
		}
	};
	/** @param {any} message */
	const receive = (message) => {
		if (message.kind === 2) {
			cancelled.add(message.id);
			return;
		}
		line = line.then(() => run(message));
	};
	if (threads !== undefined) {
		port.on("message", receive);
	} else {
		host.onmessage = (/** @type {any} */ event) => receive(event.data);
	}
}

// @fa-helper FaWorkerPool deps: FaWorker
/** A worker pool: its workers, the free ones, and the works waiting for one. */
export class FaWorkerPool {
	/**
	 * @param {number} count
	 * @param {any} state copied into each worker
	 * @param {number | undefined} maxQueueSize
	 * @param {number | undefined} timeout milliseconds
	 */
	constructor(count, state, maxQueueSize, timeout) {
		/** @type {any} */
		this.state = state;
		/** @type {number} */
		this.maxQueueSize = maxQueueSize ?? Infinity;
		/** @type {number | undefined} */
		this.timeout = timeout;
		/** @type {FaWorker[]} */
		this.workers = [];
		/** @type {FaWorker[]} */
		this.free = [];
		/** @type {Array<() => void>} the works waiting for a worker */
		this.queue = [];
		/** @type {Array<() => void>} the submitters waiting for room */
		this.waiting = [];
		/** @type {boolean} */
		this.closed = false;
		for (let index = 0; index < count; index++) {
			const worker = new FaWorker(state);
			this.workers.push(worker);
			this.free.push(worker);
		}
	}
}

// @fa-helper faPoolSubmit deps: FaWorker FaTask WorkerTimeout faAlive faCallValue faRunTask faWhenDone faEnd
/**
 * `await pool.submit(work)`: waits while the queue is full, then gives the
 * task of the work, which runs once a worker is free.
 * @param {FaTask} task
 * @param {FaWorkerPool} pool
 * @param {Function} work an `async (worker) => …` function value
 * @returns {Promise<FaTask>}
 */
export async function faPoolSubmit(task, pool, work) {
	while (pool.queue.length >= pool.maxQueueSize) {
		await new Promise((resolve) => {
			pool.waiting.push(() => resolve(undefined));
			task.wake = () => resolve(undefined);
			task.where = "WorkerPool.submit";
		});
		task.wake = null;
		faAlive(task);
	}
	const job = new FaTask(task);
	const dispatch = () => {
		while (pool.free.length > 0 && pool.queue.length > 0) {
			const next = /** @type {() => void} */ (pool.queue.shift());
			next();
			pool.waiting.shift()?.();
		}
	};
	pool.queue.push(() => {
		const worker = /** @type {FaWorker} */ (pool.free.pop());
		/** @type {ReturnType<typeof setTimeout> | undefined} */
		let timer;
		const release = () => {
			if (timer !== undefined) {
				clearTimeout(timer);
			}
			if (!worker.dead) {
				pool.free.push(worker);
				if (pool.closed) {
					worker.stopping = true;
					worker.settled();
				}
			}
			dispatch();
		};
		if (pool.timeout !== undefined) {
			timer = setTimeout(() => {
				if (job.state !== 0) {
					return;
				}
				// the work took too long: its worker is replaced
				const index = pool.workers.indexOf(worker);
				const fresh = new FaWorker(pool.state);
				pool.workers[index] = fresh;
				pool.free.push(fresh);
				// its call waiting for the old thread ends (cancelled)
				worker.terminate();
				job.wake = null;
				job.state = 1;
				job.failed = true;
				job.error = new WorkerTimeout();
				faEnd(job);
			}, pool.timeout);
		}
		faWhenDone(job, release);
		faRunTask(job, (/** @type {FaTask} */ self) => faCallValue(self, work, worker), [], true);
	});
	dispatch();
	return job;
}

// @fa-helper faPoolShutdown
/**
 * `pool.shutdown()`: the workers end once the works sent to them are done;
 * the works still queued are cancelled.
 * @param {FaWorkerPool} pool
 * @returns {void}
 */
export function faPoolShutdown(pool) {
	pool.closed = true;
	pool.queue = [];
	for (const worker of pool.free) {
		worker.stopping = true;
		worker.settled();
	}
}
