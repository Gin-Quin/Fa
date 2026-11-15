// Tasks on JavaScript (design/async.md §4, step J1): `start`, `await`,
// `cancel`, `Task.all/any/settled`, `Task.yield`, `sleep`, and the waits of
// mutexes and channels.
//
// - An async instance of a function is a JavaScript `async function` whose
//   first parameter is its task (`$task`); `await f()` of a function that may
//   suspend is `await f___async($task, …)`.
// - A `FaTask` is the `Task` value. `start` begins it in a microtask
//   (`faStart`), never synchronously. Its outcome is stored in it: a value, a
//   Fa error, or cancelled; no promise of the runtime ever rejects with a Fa
//   error. A stop (or a foreign exception) in a task ends the program.
// - Cancellation (`faCancel`) is synchronous: it marks the task, ends its
//   current wait (a timer is cleared, a place in a line is left) and cancels
//   the tasks it owns. The waits of the runtime check the mark when they
//   resume and throw `faCancelSignal`, which no `catch` keeps: the task's
//   `finally` blocks (guards, resources) run, and nothing else.
// - The coop budget (A2): an await that resolves without waiting counts, and
//   every 128 of them a macrotask lets timers and I/O run (`faReady`); the
//   loops of async instances check the clock every 128 iterations and give
//   the thread back after 4 ms (`faCoop`).
// - Blocking calls (asynchronous mode off, synchronous instances): `start`
//   runs the task at once and gives a finished task (`faFinished`), `await
//   task` reads it (`faResult`), `sleep` blocks the thread (`faSleepSync`).
// - The program ends when `main` and the detached tasks are done
//   (`faRunAsync`); when every task waits and nothing can wake them, it stops
//   with `deadlock`.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { FaGuard, faUnlock } from "./concurrency.js";
import { FaError } from "./errors.js";
import { faExit, faIsError, faReport, faWriteError } from "./main.js";
import { faDropOwned, faReleaseShared, faRetainShared, faUniqueDrop } from "./ownership.js";
import { faStop } from "./stops.js";

// @fa-helper faSched
/**
 * The state of the scheduler of the thread.
 * @type {{ n: number, resolutions: number, turn: number, depth: number, live: number, idle: (() => void) | null }}
 */
export const faSched = {
	/** the loop iterations of async instances until the next clock check */
	n: 128,
	/** the awaits resolved without waiting since the last macrotask */
	resolutions: 0,
	/** when the current macrotask was first measured (`performance.now()`), -1 when unknown */
	turn: -1,
	/** above zero: inside a blocking call, `start` runs the task at once */
	depth: 0,
	/** the detached tasks not done */
	live: 0,
	/** called when the last detached task ends */
	idle: null,
};

// @fa-helper faCancelSignal
/**
 * What a cancelled task throws when it resumes: not a Fa error, so that no
 * `catch` keeps it, and its `finally` blocks run.
 */
export const faCancelSignal = Object.freeze({ faCancelled: true });

// @fa-helper Cancelled deps: FaError
/** The error of awaiting a cancelled task. */
export class Cancelled extends FaError {
	static faName = "Cancelled";
}

// @fa-helper ChannelClosed deps: FaError
/** The error of `send` on a closed channel. */
export class ChannelClosed extends FaError {
	static faName = "ChannelClosed";
}

// @fa-helper faLater
/** @type {MessageChannel | undefined} the channel of the macrotasks on the web */
let faLaterChannel;
/** @type {Array<() => void>} */
const faLaterQueue = [];

/**
 * Runs a callback in a macrotask: `setImmediate` on Bun and Node, a
 * `MessageChannel` on the web (not throttled like `setTimeout`).
 * @param {() => void} callback
 * @returns {void}
 */
export function faLater(callback) {
	/** @type {any} */
	const host = globalThis;
	if (typeof host.setImmediate === "function") {
		host.setImmediate(callback);
		return;
	}
	if (faLaterChannel === undefined) {
		faLaterChannel = new MessageChannel();
		faLaterChannel.port1.onmessage = () => faLaterQueue.shift()?.();
	}
	faLaterQueue.push(callback);
	faLaterChannel.port2.postMessage(null);
}

// @fa-helper faMacrotask deps: faLater faSched
/**
 * A promise resolved in a macrotask: the coop yield.
 * @returns {Promise<void>}
 */
export function faMacrotask() {
	faSched.resolutions = 0;
	return new Promise((resolve) => faLater(resolve));
}

// @fa-helper faReady deps: faSched faMacrotask
/**
 * An await that resolves without waiting: every 128 of them, a macrotask.
 * @returns {Promise<void> | undefined}
 */
export function faReady() {
	if (++faSched.resolutions < 128) {
		return undefined;
	}
	return faMacrotask();
}

// @fa-helper faAlive deps: faCancelSignal
/**
 * After a wait: throws `faCancelSignal` when the task was cancelled meanwhile.
 * @param {FaTask} task
 * @returns {void}
 */
export function faAlive(task) {
	if (task.state === 2) {
		throw faCancelSignal;
	}
}

// @fa-helper faCoop deps: faSched faLater faMacrotask faAlive
/**
 * The check of a loop of an async instance, every 128 iterations: once the
 * current macrotask lasted 4 ms, the task lets the others (timers, I/O) run.
 * @param {FaTask} task
 * @returns {Promise<void>}
 */
export async function faCoop(task) {
	const sched = faSched;
	sched.n = 128;
	const now = performance.now();
	if (sched.turn < 0) {
		sched.turn = now;
		faLater(() => {
			sched.turn = -1;
		});
		return;
	}
	if (now - sched.turn < 4) {
		return;
	}
	await faMacrotask();
	faAlive(task);
}

// @fa-helper FaTask deps: faUniqueDrop faDrop
/**
 * A task: its outcome once it ended, what it waits on, the tasks it owns.
 */
export class FaTask {
	[faUniqueDrop]() {
		faDrop(this);
	}

	/** @param {FaTask | null} parent the task owning it */
	constructor(parent) {
		/** @type {number} 0: running or waiting, 1: done, 2: cancelled */
		this.state = 0;
		/** @type {any} */
		this.value = undefined;
		/** @type {any} the Fa error it ended with */
		this.error = undefined;
		/** @type {boolean} */
		this.failed = false;
		/** @type {(() => void) | null} ends its current wait (when cancelled) */
		this.wake = null;
		/** @type {Array<() => void> | null} called when it ends */
		this.waiters = null;
		/** @type {FaTask | null} */
		this.parent = parent;
		/** @type {Set<FaTask> | null} the tasks it owns, not awaited yet */
		this.children = null;
		/** @type {boolean} */
		this.detached = false;
		/** @type {string} what it waits on (for the deadlock message) */
		this.where = "";
		if (parent !== null) {
			(parent.children ??= new Set()).add(this);
		}
	}
}

// @fa-helper faWhenDone
/**
 * Calls `callback` when the task has ended (at once if it has).
 * @param {FaTask} task
 * @param {() => void} callback
 * @returns {void}
 */
export function faWhenDone(task, callback) {
	if (task.state !== 0) {
		callback();
	} else {
		(task.waiters ??= []).push(callback);
	}
}

// @fa-helper faEnd deps: faSched faCancel
/**
 * A task has ended: the tasks it still owns are cancelled, its owner
 * forgets it, and what waits for it runs.
 * @param {FaTask} task
 * @returns {void}
 */
export function faEnd(task) {
	task.wake = null;
	if (task.children !== null) {
		const children = task.children;
		task.children = null;
		for (const child of children) {
			faCancel(child);
		}
	}
	task.parent?.children?.delete(task);
	if (task.detached) {
		task.detached = false;
		faSched.live -= 1;
		if (faSched.live === 0 && faSched.idle !== null) {
			faSched.idle();
		}
	}
	const waiters = task.waiters;
	task.waiters = null;
	if (waiters !== null) {
		for (const waiter of waiters) {
			waiter();
		}
	}
}

// @fa-helper faCancel deps: faEnd
/**
 * `task.cancel()`, and the drop of a task: it stops at its current wait.
 * A later `await` of it throws `Cancelled`.
 * @param {FaTask} task
 * @returns {void}
 */
export function faCancel(task) {
	if (task.state === 0) {
		task.state = 2;
		const wake = task.wake;
		task.wake = null;
		if (wake !== null) {
			wake();
		}
		faEnd(task);
	} else if (task.state === 1) {
		task.state = 2;
		task.value = undefined;
		task.error = undefined;
		task.failed = false;
	}
}

// @fa-helper faDrop deps: faCancel
/**
 * The end of the scope of a task that wasn't awaited: it is cancelled
 * (nothing for a task that has ended).
 * @param {FaTask | undefined} task
 * @returns {void}
 */
export function faDrop(task) {
	if (task !== undefined && task.state === 0) {
		faCancel(task);
	}
}

// @fa-helper faDropTasks deps: FaTask faDrop
/**
 * The end of the scope of a value holding tasks (an array, a map, an
 * object, an optional…): each task still running is cancelled, in order.
 * @param {unknown} value
 * @returns {void}
 */
export function faDropTasks(value) {
	/** @type {unknown[]} */
	const pending = [value];
	while (pending.length > 0) {
		const item = /** @type {any} */ (pending.shift());
		if (item instanceof FaTask) {
			faDrop(item);
		} else if (typeof item === "object" && item !== null) {
			const parts =
				typeof item[Symbol.iterator] === "function" ? Array.from(item) : Object.values(item);
			pending.unshift(...parts);
		}
	}
}

// @fa-helper faFatal deps: faReport faExit
/**
 * A stop (or a foreign exception) in a task: the program ends.
 * @param {unknown} exception
 * @returns {void}
 */
export function faFatal(exception) {
	faExit(faReport(exception));
}

// @fa-helper faSettle deps: faCancelSignal faIsError faFatal faEnd
/**
 * The body of a task ended, with a value or an exception.
 * @param {FaTask} task
 * @param {any} outcome
 * @param {boolean} thrown
 * @returns {void}
 */
export function faSettle(task, outcome, thrown) {
	if (thrown) {
		if (outcome === faCancelSignal) {
			return;
		}
		if (!faIsError(outcome)) {
			faFatal(outcome);
			return;
		}
	}
	if (task.state !== 0) {
		return;
	}
	task.state = 1;
	if (thrown) {
		task.failed = true;
		task.error = outcome;
	} else {
		task.value = outcome;
	}
	faEnd(task);
}

// @fa-helper faRunTask deps: faSettle
/**
 * Runs the body of a task, unless it was cancelled before its first turn.
 * @param {FaTask} task
 * @param {Function} body
 * @param {any[]} args
 * @param {boolean} withTask whether `body` takes the task first (an async instance)
 * @returns {void}
 */
export function faRunTask(task, body, args, withTask) {
	if (task.state !== 0) {
		return;
	}
	let result;
	try {
		result = withTask ? body(task, ...args) : body(...args);
	} catch (exception) {
		faSettle(task, exception, true);
		return;
	}
	if (result instanceof Promise) {
		result.then(
			(value) => faSettle(task, value, false),
			(exception) => faSettle(task, exception, true),
		);
	} else {
		faSettle(task, result, false);
	}
}

// @fa-helper faStart deps: FaTask faRunTask
/**
 * `start f(…)` of an async instance (`f___async(task, …)`), or `start do`:
 * the task begins in a microtask.
 * @param {FaTask | null} parent the task starting it
 * @param {Function} body
 * @param {...any} args
 * @returns {FaTask}
 */
export function faStart(parent, body, ...args) {
	const task = new FaTask(parent);
	queueMicrotask(() => faRunTask(task, body, args, true));
	return task;
}

// @fa-helper faStartOwned deps: faStart faWhenDone
/**
 * A detached start-do owns its captures until the body's finally blocks
 * finish. Cancellation before its first turn releases them without a call.
 * @param {FaTask | null} parent
 * @param {Function} body
 * @param {() => void} release
 * @returns {FaTask}
 */
export function faStartOwned(parent, body, release) {
	let entered = false;
	let owned = true;
	const finish = () => {
		if (!owned) return;
		owned = false;
		release();
	};
	const task = faStart(parent, async (self) => {
		entered = true;
		try {
			return await body(self);
		} finally {
			finish();
		}
	});
	faWhenDone(task, () => {
		if (!entered) finish();
	});
	return task;
}

// @fa-helper faFinishedOwned deps: faFinished
/** @param {Function} body @param {() => void} release @returns {FaTask} */
export function faFinishedOwned(body, release) {
	try {
		return faFinished(body);
	} finally {
		release();
	}
}

// @fa-helper faStartOwnedDynamic deps: faSched faFinishedOwned faStartOwned
/** @param {Function} asynchronous @param {Function} blocking @param {() => void} release @returns {FaTask} */
export function faStartOwnedDynamic(asynchronous, blocking, release) {
	return faSched.depth > 0
		? faFinishedOwned(blocking, release)
		: faStartOwned(null, asynchronous, release);
}

// @fa-helper faStartCall deps: FaTask faRunTask
/**
 * `start f(…)` of a function that has no async instance (it can't
 * suspend, or it is a function value): it runs in a microtask.
 * @param {FaTask | null} parent
 * @param {Function} body
 * @param {...any} args
 * @returns {FaTask}
 */
export function faStartCall(parent, body, ...args) {
	const task = new FaTask(parent);
	queueMicrotask(() => faRunTask(task, body, args, false));
	return task;
}

// @fa-helper faBlocking deps: faSched
/**
 * A blocking call of an `async` function that starts tasks without
 * suspending: its `start`s run their tasks at once (the blocking depth).
 * Awaited, the same function is a plain call, and its tasks start later.
 * @param {Function} f
 * @param {...any} args
 * @returns {any}
 */
export function faBlocking(f, ...args) {
	faSched.depth += 1;
	try {
		return f(...args);
	} finally {
		faSched.depth -= 1;
	}
}

// @fa-helper faFinished deps: FaTask faIsError
/**
 * `start` in a blocking call: the task runs to its end at once, and the
 * finished task holds its outcome.
 * @param {Function} body
 * @param {...any} args
 * @returns {FaTask}
 */
export function faFinished(body, ...args) {
	const task = new FaTask(null);
	try {
		task.value = body(...args);
	} catch (exception) {
		if (!faIsError(exception)) {
			throw exception;
		}
		task.failed = true;
		task.error = exception;
	}
	task.state = 1;
	return task;
}

// @fa-helper faStartDynamic deps: faSched faFinished faStart faStartCall
/**
 * `start` in a function that isn't `async`: at once in a blocking call, in
 * a microtask otherwise (design/async.md §1.1, the blocking depth).
 * @param {Function | undefined} asyncBody the async instance (taking the task first), if any
 * @param {Function} syncBody
 * @param {...any} args
 * @returns {FaTask}
 */
export function faStartDynamic(asyncBody, syncBody, ...args) {
	if (faSched.depth > 0) {
		return faFinished(syncBody, ...args);
	}
	return asyncBody === undefined
		? faStartCall(null, syncBody, ...args)
		: faStart(null, asyncBody, ...args);
}

// @fa-helper faAsyncValue
/**
 * The value of an `async` function that may suspend, or of an async lambda:
 * its synchronous instance (what a blocking call runs), carrying its async
 * instance as `$async` (design/async.md §2.2: both instances).
 * @template {Function} F
 * @param {F} sync
 * @param {Function} instance the async instance, taking the task first
 * @returns {F}
 */
export function faAsyncValue(sync, instance) {
	const value = /** @type {any} */ (sync);
	value.$async = instance;
	return sync;
}

// @fa-helper faKeepFunctionResult deps: faReleaseShared
/**
 * Invokes one already retained Function owner. A suspended call keeps its
 * environment until its real finally blocks run, including after cancellation.
 * @param {Function} f
 * @param {() => any} invoke
 * @returns {any}
 */
export function faKeepFunctionResult(f, invoke) {
	let result;
	try {
		result = invoke();
	} catch (error) {
		faReleaseShared(f);
		throw error;
	}
	if (result instanceof Promise) return result.finally(() => faReleaseShared(f));
	faReleaseShared(f);
	return result;
}

// @fa-helper faCallValue deps: faRetainShared faKeepFunctionResult
/**
 * `await f(…)` of a value of an `async` function type, in an async
 * instance: its async instance when it has one, a plain call otherwise (a
 * function that isn't async, or can't suspend).
 * @param {FaTask} task
 * @param {Function} f
 * @param {...any} args
 * @returns {any}
 */
export function faCallValue(task, f, ...args) {
	faRetainShared(f);
	return faKeepFunctionResult(f, () => {
		const instance = /** @type {any} */ (f).$async;
		return instance === undefined ? f(...args) : instance(task, ...args);
	});
}

// @fa-helper faStartValue deps: FaTask faRunTask faWhenDone faRetainShared faReleaseShared faKeepFunctionResult
/**
 * `start f(…)` of a value of an `async` function type: a task of its async
 * instance, or of the plain call.
 * @param {FaTask | null} parent
 * @param {Function} f
 * @param {...any} args
 * @returns {FaTask}
 */
export function faStartValue(parent, f, ...args) {
	faRetainShared(f);
	const task = new FaTask(parent);
	let entered = false;
	faWhenDone(task, () => {
		if (!entered) faReleaseShared(f);
	});
	queueMicrotask(() =>
		faRunTask(
			task,
			(self) => {
				entered = true;
				return faKeepFunctionResult(f, () => {
					const instance = /** @type {any} */ (f).$async;
					return instance === undefined ? f(...args) : instance(self, ...args);
				});
			},
			[],
			true,
		),
	);
	return task;
}

// @fa-helper faStartValueDynamic deps: faSched faFinished faStartValue faRetainShared faReleaseShared
/**
 * [`faStartValue`] in a function that isn't `async` (the blocking depth).
 * @param {Function} f
 * @param {...any} args
 * @returns {FaTask}
 */
export function faStartValueDynamic(f, ...args) {
	if (faSched.depth <= 0) return faStartValue(null, f, ...args);
	faRetainShared(f);
	try {
		return faFinished(f, ...args);
	} finally {
		faReleaseShared(f);
	}
}

// @fa-helper faDetach deps: faSched
/**
 * A detached task (a `start` statement): no scope owns it, and the program
 * waits for it before exiting.
 * @param {FaTask} task
 * @returns {void}
 */
export function faDetach(task) {
	task.parent?.children?.delete(task);
	task.parent = null;
	if (task.state === 0 && !task.detached) {
		task.detached = true;
		faSched.live += 1;
	}
}

// @fa-helper faResult deps: Cancelled
/**
 * The outcome of a task that ended: its value, or its error thrown.
 * @param {FaTask} task
 * @returns {any}
 */
export function faResult(task) {
	task.parent?.children?.delete(task);
	if (task.state === 2) {
		throw new Cancelled();
	}
	if (task.state === 0) {
		throw new Cancelled();
	}
	if (task.failed) {
		throw task.error;
	}
	return task.value;
}

// @fa-helper faJoin deps: faWhenDone faReady faAlive faResult
/**
 * `await task` in an async instance.
 * @param {FaTask} self the task awaiting
 * @param {FaTask} task the task awaited
 * @returns {Promise<any>}
 */
export async function faJoin(self, task) {
	if (task.state === 0) {
		await new Promise((resolve) => {
			faWhenDone(task, () => resolve(undefined));
			self.wake = () => resolve(undefined);
			self.where = "a task";
		});
		self.wake = null;
	} else {
		await faReady();
	}
	faAlive(self);
	return faResult(task);
}

// @fa-helper faCombine deps: FaTask faWhenDone faSettle Cancelled
/**
 * `Task.all`, `Task.any`, `Task.settled`: a task owning the tasks given.
 * @param {FaTask | null} parent
 * @param {FaTask[]} tasks
 * @param {number} mode 0: all, 1: any, 2: settled
 * @returns {FaTask}
 */
export function faCombine(parent, tasks, mode) {
	const join = new FaTask(parent);
	for (const task of tasks) {
		task.parent?.children?.delete(task);
		if (task.state === 0) {
			task.parent = join;
			(join.children ??= new Set()).add(task);
		}
	}
	let remaining = tasks.length;
	/** @param {FaTask} task */
	const outcome = (task) =>
		task.state === 2 ? new Cancelled() : task.failed ? task.error : task.value;
	if (remaining === 0) {
		faSettle(join, mode === 1 ? new Cancelled() : [], mode === 1);
		return join;
	}
	for (const task of tasks) {
		faWhenDone(task, () => {
			if (join.state !== 0) {
				return;
			}
			remaining -= 1;
			const failed = task.state === 2 || task.failed;
			if (mode === 1) {
				faSettle(join, outcome(task), failed);
			} else if (mode === 0 && failed) {
				faSettle(join, outcome(task), true);
			} else if (remaining === 0) {
				faSettle(join, tasks.map(outcome), false);
			}
		});
	}
	return join;
}

// @fa-helper faYield deps: faReady faAlive
/**
 * `await Task.yield()`: the other tasks run first.
 * @param {FaTask} task
 * @returns {Promise<void>}
 */
export async function faYield(task) {
	await faReady();
	faAlive(task);
}

// @fa-helper faSleep deps: faAlive
/**
 * `await sleep(milliseconds)`.
 * @param {FaTask} task
 * @param {number} milliseconds
 * @returns {Promise<void>}
 */
export async function faSleep(task, milliseconds) {
	await new Promise((resolve) => {
		const timer = setTimeout(resolve, milliseconds);
		task.wake = () => {
			clearTimeout(timer);
			resolve(undefined);
		};
	});
	task.wake = null;
	faAlive(task);
}

// @fa-helper faSleepSync
/**
 * `sleep(milliseconds)` called blocking (Bun and Node): the thread waits.
 * @param {number} milliseconds
 * @returns {void}
 */
export function faSleepSync(milliseconds) {
	if (milliseconds > 0) {
		Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
	}
}

// @fa-helper faRemove
/**
 * Removes a waiter from a line.
 * @param {any[]} line
 * @param {any} waiter
 * @returns {void}
 */
export function faRemove(line, waiter) {
	const index = line.indexOf(waiter);
	if (index >= 0) {
		line.splice(index, 1);
	}
}

// @fa-helper faLockWait deps: FaGuard faUnlock faReady faAlive faRemove faCancelSignal faWaitShared
/**
 * `await mutex.lock()`: the guard, when every task that asked before has
 * released it.
 * @param {FaTask} task
 * @param {any} mutex
 * @param {string} where
 * @returns {Promise<FaGuard>}
 */
export async function faLockWait(task, mutex, where) {
	// shared with a worker: the shared lock, tried again when its word changes
	if (mutex.cell != null) {
		for (let attempt = 0; !mutex.acquire(); attempt += 1) {
			await faWaitShared(task, mutex.cell, 0, 1, `Mutex.lock at ${where}`, attempt);
		}
		return new FaGuard(mutex);
	}
	if (!mutex.locked) {
		mutex.locked = true;
		const guard = new FaGuard(mutex);
		try {
			await faReady();
			faAlive(task);
			return guard;
		} catch (error) {
			faUnlock(guard);
			throw error;
		}
	}
	/** @type {FaGuard | undefined} */
	const guard = await new Promise((resolve) => {
		const waiter = { resolve };
		mutex.waiters.push(waiter);
		task.wake = () => {
			faRemove(mutex.waiters, waiter);
			resolve(undefined);
		};
		task.where = `Mutex.lock at ${where}`;
	});
	task.wake = null;
	if (task.state === 2) {
		if (guard !== undefined) {
			faUnlock(guard);
		}
		throw faCancelSignal;
	}
	return /** @type {FaGuard} */ (guard);
}

// @fa-helper faLock deps: faLockWait faRetainShared faReleaseShared
/** @param {FaTask} task @param {any} mutex @param {string} where @returns {Promise<FaGuard>} */
export async function faLock(task, mutex, where) {
	faRetainShared(mutex);
	try {
		return await faLockWait(task, mutex, where);
	} finally {
		faReleaseShared(mutex);
	}
}

// @fa-helper faSendWait deps: ChannelClosed faReady faAlive faRemove faWaitShared
/**
 * `await channel.send(value)`: waits while the channel is full; throws
 * `ChannelClosed` when it is (or gets) closed.
 * @param {FaTask} task
 * @param {any} channel
 * @param {any} value
 * @param {string} where
 * @param {{owned:boolean}} receipt
 * @returns {Promise<void>}
 */
export async function faSendWait(task, channel, value, where, receipt) {
	// shared with a worker: tried again when its version word changes
	if (channel.cell != null) {
		for (let attempt = 0; ; attempt += 1) {
			const version = Atomics.load(channel.cell, 3);
			const sent = channel.pollSend(value);
			if (sent === 1) {
				receipt.owned = false;
				return;
			}
			if (sent === -1) {
				throw new ChannelClosed();
			}
			await faWaitShared(task, channel.cell, 3, version, `Channel.send at ${where}`, attempt);
		}
	}
	if (channel.closed) {
		throw new ChannelClosed();
	}
	if (channel.offer(value)) {
		receipt.owned = false;
		await faReady();
		faAlive(task);
		return;
	}
	const sent = await new Promise((resolve) => {
		const waiter = { resolve, value };
		channel.senders.push(waiter);
		task.wake = () => {
			faRemove(channel.senders, waiter);
			resolve(false);
		};
		task.where = `Channel.send at ${where}`;
	});
	task.wake = null;
	if (sent) receipt.owned = false;
	faAlive(task);
	if (!sent) {
		throw new ChannelClosed();
	}
}

// @fa-helper faSend deps: faSendWait faRetainShared faReleaseShared faDropOwned
/** @param {FaTask} task @param {any} channel @param {any} value @param {string} where @returns {Promise<void>} */
export async function faSend(task, channel, value, where) {
	const receipt = { owned: true };
	faRetainShared(channel);
	try {
		await faSendWait(task, channel, value, where, receipt);
	} finally {
		if (receipt.owned) faDropOwned(value);
		faReleaseShared(channel);
	}
}

// @fa-helper faReceiveWait deps: faReady faAlive faRemove faWaitShared faWaiting faDropOwned
/**
 * `await channel.receive()`: the next value, `none` once the channel is
 * closed and empty.
 * @param {FaTask} task
 * @param {any} channel
 * @param {string} where
 * @returns {Promise<any>}
 */
export async function faReceiveWait(task, channel, where) {
	// shared with a worker: tried again when its version word changes
	if (channel.cell != null) {
		for (let attempt = 0; ; attempt += 1) {
			const version = Atomics.load(channel.cell, 3);
			const value = channel.pollReceive(faWaiting);
			if (value !== faWaiting) {
				return value;
			}
			await faWaitShared(task, channel.cell, 3, version, `Channel.receive at ${where}`, attempt);
		}
	}
	if (channel.head < channel.items.length || channel.closed) {
		const value = channel.tryReceive();
		await faReady();
		if (task.state === 2) faDropOwned(value);
		faAlive(task);
		return value;
	}
	const value = await new Promise((resolve) => {
		const waiter = { resolve };
		channel.receivers.push(waiter);
		task.wake = () => {
			faRemove(channel.receivers, waiter);
			resolve(undefined);
		};
		task.where = `Channel.receive at ${where}`;
	});
	task.wake = null;
	if (task.state === 2) faDropOwned(value);
	faAlive(task);
	return value;
}

// @fa-helper faReceive deps: faReceiveWait faRetainShared faReleaseShared
/** @param {FaTask} task @param {any} channel @param {string} where @returns {Promise<any>} */
export async function faReceive(task, channel, where) {
	faRetainShared(channel);
	try {
		return await faReceiveWait(task, channel, where);
	} finally {
		faReleaseShared(channel);
	}
}

// @fa-helper faWaitShared deps: faAlive
/**
 * A task waiting for a word of memory shared with a worker (a mutex's lock,
 * a channel's version) to change from `value`, or to be cancelled (it then
 * throws `faCancelSignal`): `Atomics.waitAsync`, woken by the
 * `Atomics.notify` of every unlock, send and receive, and by a timeout once
 * a second (a change can't be missed, the caller tries again); cancelling
 * the task notifies the word, so that its wait ends at once. Where
 * `waitAsync` is missing (Firefox), a timer whose delay doubles from 1 ms
 * to 16 ms with each `attempt`.
 * @param {FaTask} task
 * @param {Int32Array} cell
 * @param {number} index
 * @param {number} value
 * @param {string} where what it waits on, for deadlock reports
 * @param {number} attempt how many times the caller tried already
 * @returns {Promise<void>}
 */
export async function faWaitShared(task, cell, index, value, where, attempt) {
	/** @type {any} */
	const atomics = Atomics;
	await new Promise((resolve) => {
		task.where = where;
		if (typeof atomics.waitAsync === "function") {
			const result = atomics.waitAsync(cell, index, value, 1000);
			if (!result.async) {
				// already changed
				resolve(undefined);
				return;
			}
			// cancelled: the word is notified, so that this wait settles at
			// once and doesn't keep the program alive (the other waiters
			// check their condition again and wait anew)
			task.wake = () => {
				Atomics.notify(cell, index);
				resolve(undefined);
			};
			result.value.then(resolve);
			return;
		}
		const timer = setTimeout(resolve, Math.min(16, 2 ** Math.min(attempt, 4)));
		task.wake = () => {
			clearTimeout(timer);
			resolve(undefined);
		};
	});
	task.wake = null;
	faAlive(task);
}

// @fa-helper faOnWorker
/** @returns {boolean} whether this code runs on a worker's thread */
export function faOnWorker() {
	/** @type {any} */
	const host = globalThis;
	return (
		host.process?.getBuiltinModule?.("node:worker_threads")?.isMainThread === false ||
		typeof host.WorkerGlobalScope !== "undefined"
	);
}

// @fa-helper faSendSyncWait deps: faWaitSync faOnWorker ChannelClosed
/**
 * `channel.send(value)` in a blocking call: on the thread of a worker, a
 * channel shared with it is waited for (`Atomics.wait` on its version);
 * anywhere else nothing could make room (`faWaitSync`).
 * @param {any} channel
 * @param {any} value
 * @param {string} where
 * @param {{owned:boolean}} receipt
 * @returns {void}
 */
export function faSendSyncWait(channel, value, where, receipt) {
	if (channel?.cell != null && faOnWorker()) {
		for (;;) {
			const version = Atomics.load(channel.cell, 3);
			const sent = channel.pollSend(value);
			if (sent === 1) {
				receipt.owned = false;
				return;
			}
			if (sent === -1) {
				throw new ChannelClosed();
			}
			Atomics.wait(channel.cell, 3, version, 20);
		}
	}
	faWaitSync("Channel.send", where);
}

// @fa-helper faSendSync deps: faSendSyncWait faRetainShared faReleaseShared faDropOwned
/** @param {any} channel @param {any} value @param {string} where */
export function faSendSync(channel, value, where) {
	const receipt = { owned: true };
	faRetainShared(channel);
	try {
		faSendSyncWait(channel, value, where, receipt);
	} finally {
		if (receipt.owned) faDropOwned(value);
		faReleaseShared(channel);
	}
}

// @fa-helper faReceiveSync deps: faWaitSync faOnWorker faWaiting
/**
 * `channel.receive()` in a blocking call: on the thread of a worker, a
 * channel shared with it is waited for (`Atomics.wait` on its version);
 * anywhere else nothing could send (`faWaitSync`).
 * @param {any} channel
 * @param {string} where
 * @returns {any}
 */
export function faReceiveSync(channel, where) {
	if (channel?.cell != null && faOnWorker()) {
		for (;;) {
			const version = Atomics.load(channel.cell, 3);
			const value = channel.pollReceive(faWaiting);
			if (value !== faWaiting) {
				return value;
			}
			Atomics.wait(channel.cell, 3, version, 20);
		}
	}
	return faWaitSync("Channel.receive", where);
}

// @fa-helper faWaitSync deps: faStop
/**
 * A wait in a blocking call that no other code can end (one thread): the
 * program stops with `deadlock`.
 * @param {string} what like `Channel.receive`
 * @param {string} where
 * @returns {never}
 */
export function faWaitSync(what, where) {
	return faStop(`deadlock: every task waits (main waits on ${what})`, where);
}

// @fa-helper faLockSync deps: FaGuard faWaitSync
/**
 * `await mutex.lock()` in a blocking call: on the thread of a worker, a
 * mutex shared with it is waited for (`Atomics.wait`, step J2); anywhere
 * else nothing could release it (`faWaitSync`).
 * @param {any} mutex
 * @param {string} where
 * @returns {FaGuard}
 */
export function faLockSync(mutex, where) {
	/** @type {any} */
	const host = globalThis;
	const onWorker =
		host.process?.getBuiltinModule?.("node:worker_threads")?.isMainThread === false ||
		typeof host.WorkerGlobalScope !== "undefined";
	if (mutex?.cell != null && onWorker) {
		while (!mutex.acquire()) {
			Atomics.wait(mutex.cell, 0, 1, 20);
		}
		return new FaGuard(mutex);
	}
	return faWaitSync("Mutex.lock", where);
}

// @fa-helper faRunAsync deps: FaTask faSched faRunTask faWhenDone faReport faExit faWriteError
/**
 * Runs an asynchronous program: `main` (an async function taking its task)
 * is the root task; the program exits once it and the detached tasks are
 * done, with its code. When every task waits and nothing can wake them (the
 * event loop is empty), it stops with `deadlock`.
 * @param {(task: FaTask) => Promise<unknown>} main
 * @returns {void}
 */
export function faRunAsync(main) {
	const root = new FaTask(null);
	/** @type {any} */
	const host = globalThis;
	const process = host.process;
	let ended = false;
	/** @param {number} code */
	const end = (code) => {
		if (!ended) {
			ended = true;
			faExit(code);
		}
	};
	faWhenDone(root, () => {
		if (root.failed) {
			end(faReport(root.error));
			return;
		}
		const value = root.value;
		const code = typeof value === "number" || typeof value === "bigint" ? Number(value) : 0;
		if (faSched.live === 0) {
			end(code);
		} else {
			faSched.idle = () => end(code);
		}
	});
	if (typeof process?.on === "function") {
		process.on("beforeExit", () => {
			if (ended) {
				return;
			}
			ended = true;
			const waiting =
				root.state === 0 ? `main waits on ${root.where || "a task"}` : "detached tasks wait";
			faWriteError(`stop: deadlock: every task waits (${waiting})`);
			process.exit(101);
		});
	}
	faRunTask(root, main, [], true);
}

// @fa-helper faTestTask deps: FaTask faSched faEnd
/**
 * Runs a test that awaits (`fa test`, `faRunTests`): its body is an async
 * function taking its task. The promise ends once the test and the
 * detached tasks it started are done, rejected with what the test threw
 * (an `assert` that failed, a Fa error); the tasks it still owns are
 * cancelled.
 * @param {(task: FaTask) => Promise<unknown>} run
 * @returns {Promise<void>}
 */
export async function faTestTask(run) {
	const task = new FaTask(null);
	try {
		await run(task);
	} finally {
		if (task.state === 0) {
			task.state = 1;
			faEnd(task);
		}
		if (faSched.live > 0) {
			await new Promise((resolve) => {
				faSched.idle = () => {
					faSched.idle = null;
					resolve(undefined);
				};
			});
		}
	}
}

// @fa-helper faPromise deps: FaTask faWhenDone faRunTask Cancelled
/**
 * The Promise API of an exported async function, for JavaScript callers:
 * the call runs as a task; the promise rejects with its Fa error.
 * @param {Function} body the async instance
 * @param {any[]} args
 * @returns {Promise<any>}
 */
export function faPromise(body, args) {
	const task = new FaTask(null);
	return new Promise((resolve, reject) => {
		faWhenDone(task, () => {
			if (task.state === 2) {
				reject(new Cancelled());
			} else if (task.failed) {
				reject(task.error);
			} else {
				resolve(task.value);
			}
		});
		faRunTask(task, body, args, true);
	});
}

// @fa-helper faWaiting
/** What the ready paths of the waits give when the task would wait. */
export const faWaiting = Symbol("fa.waiting");

// @fa-helper faReceiveNow deps: faSched faWaiting
/**
 * `await channel.receive()` when a value (or the end) is there: no `await`
 * at all (an `await` that doesn't wait is a plain call, A1). `faWaiting`
 * when the task would wait, or when the coop budget is spent (`faReceive`
 * then gives the thread back).
 * @param {any} channel
 * @returns {any}
 */
export function faReceiveNow(channel) {
	if (
		faSched.resolutions >= 127 ||
		channel.cell != null ||
		(channel.head === channel.items.length && !channel.closed)
	) {
		return faWaiting;
	}
	faSched.resolutions += 1;
	return channel.tryReceive();
}

// @fa-helper faSendNow deps: faSched
/**
 * `await channel.send(value)` when the channel has room: whether it was
 * sent without waiting (`faSend` otherwise).
 * @param {any} channel
 * @param {any} value
 * @returns {boolean}
 */
export function faSendNow(channel, value) {
	if (
		faSched.resolutions >= 127 ||
		channel.cell != null ||
		channel.closed ||
		!channel.offer(value)
	) {
		return false;
	}
	faSched.resolutions += 1;
	return true;
}

// @fa-helper faLockNow deps: faSched faWaiting FaGuard
/**
 * `await mutex.lock()` when it is free: the guard, or `faWaiting` (`faLock`
 * then).
 * @param {any} mutex
 * @returns {any}
 */
export function faLockNow(mutex) {
	if (mutex.cell != null) {
		return faWaiting;
	}
	if (faSched.resolutions >= 127 || mutex.locked) {
		return faWaiting;
	}
	faSched.resolutions += 1;
	mutex.locked = true;
	return new FaGuard(mutex);
}
