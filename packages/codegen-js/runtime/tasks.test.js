// Tasks on JavaScript (design/async.md §4): start, await, cancellation,
// combinators, and the waits of mutexes and channels.

import { describe, expect, test } from "bun:test";

import { FaChannel, FaMutex, faUnlock } from "./concurrency.js";
import { faRunTests } from "./main.js";
import { FaStop } from "./stops.js";
import {
	Cancelled,
	ChannelClosed,
	faAsyncValue,
	faCallValue,
	faCancel,
	faCancelSignal,
	faCombine,
	faFinished,
	faJoin,
	faLock,
	faReceive,
	faResult,
	faSched,
	faSend,
	faSleep,
	faStart,
	faStartValue,
	FaTask,
	faTestTask,
	faWaitShared,
	faYield,
} from "./tasks.js";

/** A root task, for the code of a test. */
function root() {
	return new FaTask(null);
}

describe("start and await", () => {
	test("a started task begins in a microtask, not at once", async () => {
		const self = root();
		const order = [];
		const task = faStart(self, async () => {
			order.push("task");
			return 42;
		});
		order.push("starter");
		expect(await faJoin(self, task)).toBe(42);
		expect(order).toEqual(["starter", "task"]);
	});

	test("an error is stored, then thrown by the await", async () => {
		const self = root();
		const error = new ChannelClosed();
		const task = faStart(self, async () => {
			throw error;
		});
		await expect(faJoin(self, task)).rejects.toBe(error);
	});

	test("tasks take turns at their yields", async () => {
		const self = root();
		const order = [];
		/** @param {FaTask} task @param {string} name */
		const count = async (task, name) => {
			for (let round = 1; round <= 3; round += 1) {
				order.push(`${name}${round}`);
				await faYield(task);
			}
		};
		const a = faStart(self, count, "a");
		const b = faStart(self, count, "b");
		await faJoin(self, a);
		await faJoin(self, b);
		expect(order).toEqual(["a1", "b1", "a2", "b2", "a3", "b3"]);
	});

	test("a blocking start runs at once and gives a finished task", () => {
		const task = faFinished((x) => x + 1, 1);
		expect(task.state).toBe(1);
		expect(faResult(task)).toBe(2);
	});
});

describe("cancellation", () => {
	test("cancel ends the wait at once, and a later await throws Cancelled", async () => {
		const self = root();
		let finallyRan = false;
		const task = faStart(self, async (/** @type {FaTask} */ task) => {
			try {
				await faSleep(task, 10_000);
			} finally {
				finallyRan = true;
			}
		});
		await faYield(self);
		faCancel(task);
		expect(task.state).toBe(2);
		await expect(faJoin(self, task)).rejects.toBeInstanceOf(Cancelled);
		await faYield(self);
		expect(finallyRan).toBe(true);
	});

	test("cancelling a task cancels the tasks it owns", async () => {
		const self = root();
		/** @type {FaTask | undefined} */
		let inner;
		const outer = faStart(self, async (/** @type {FaTask} */ task) => {
			inner = faStart(task, async (/** @type {FaTask} */ task) => faSleep(task, 10_000));
			await faJoin(task, inner);
		});
		await faYield(self);
		await faYield(self);
		faCancel(outer);
		expect(inner?.state).toBe(2);
	});

	test("the waits throw the cancel signal when they resume", async () => {
		const self = root();
		const sleeping = faSleep(self, 10_000);
		faCancel(self);
		await expect(sleeping).rejects.toBe(faCancelSignal);
	});
});

describe("combinators", () => {
	test("all gives every result in order, any the first, settled every outcome", async () => {
		const self = root();
		/** @param {FaTask} task @param {number} value @param {number} delay */
		const later = async (task, value, delay) => {
			await faSleep(task, delay);
			if (value < 0) {
				throw new ChannelClosed();
			}
			return value;
		};
		const all = faCombine(self, [faStart(self, later, 1, 20), faStart(self, later, 2, 5)], 0);
		expect(await faJoin(self, all)).toEqual([1, 2]);
		const losers = [faStart(self, later, 3, 50), faStart(self, later, 4, 1)];
		expect(await faJoin(self, faCombine(self, losers, 1))).toBe(4);
		expect(losers[0].state).toBe(2);
		const settled = faCombine(self, [faStart(self, later, 5, 1), faStart(self, later, -1, 1)], 2);
		const outcomes = await faJoin(self, settled);
		expect(outcomes[0]).toBe(5);
		expect(outcomes[1]).toBeInstanceOf(ChannelClosed);
	});

	test("all fails with the first error and cancels the others", async () => {
		const self = root();
		const slow = faStart(self, async (/** @type {FaTask} */ task) => faSleep(task, 10_000));
		const failing = faStart(self, async () => {
			throw new ChannelClosed();
		});
		await expect(faJoin(self, faCombine(self, [slow, failing], 0))).rejects.toBeInstanceOf(
			ChannelClosed,
		);
		expect(slow.state).toBe(2);
	});
});

describe("channels and mutexes", () => {
	test("a value sent to a waiting receiver is handed over", async () => {
		const self = root();
		const channel = new FaChannel(1);
		const receiving = faStart(self, async (/** @type {FaTask} */ task) =>
			faReceive(task, channel, "here"),
		);
		await faYield(self);
		await faSend(self, channel, "value", "here");
		expect(channel.items.length - channel.head).toBe(0);
		expect(await faJoin(self, receiving)).toBe("value");
	});

	test("senders wait while the channel is full, in order", async () => {
		const self = root();
		const channel = new FaChannel(1);
		const sending = [1, 2, 3].map((value) =>
			faStart(self, async (/** @type {FaTask} */ task) => faSend(task, channel, value, "here")),
		);
		await faYield(self);
		await faYield(self);
		const received = [];
		for (let index = 0; index < 3; index += 1) {
			received.push(await faReceive(self, channel, "here"));
		}
		expect(received).toEqual([1, 2, 3]);
		for (const task of sending) {
			await faJoin(self, task);
		}
	});

	test("closing a channel ends its waits", async () => {
		const self = root();
		const channel = new FaChannel(1);
		const receiving = faStart(self, async (/** @type {FaTask} */ task) =>
			faReceive(task, channel, "here"),
		);
		await faYield(self);
		channel.close();
		expect(await faJoin(self, receiving)).toBeUndefined();
		await expect(faSend(self, channel, 1, "here")).rejects.toBeInstanceOf(ChannelClosed);
	});

	test("a mutex hands its lock over in order", async () => {
		const self = root();
		const mutex = new FaMutex({ order: "" });
		/** @param {FaTask} task @param {string} name */
		const visit = async (task, name) => {
			const guard = await faLock(task, mutex, "here");
			try {
				guard.value.order += name;
				await faYield(task);
			} finally {
				faUnlock(guard);
			}
		};
		const tasks = ["a", "b", "c"].map((name) => faStart(self, visit, name));
		await faJoin(self, faCombine(self, tasks, 0));
		expect(mutex.value.order).toBe("abc");
		expect(mutex.locked).toBe(false);
	});
});

describe("the coop budget", () => {
	test("128 awaits without waiting give the thread back with a macrotask", async () => {
		const self = root();
		let fired = false;
		setImmediate(() => {
			fired = true;
		});
		faSched.resolutions = 0;
		for (let index = 0; index < 130 && !fired; index += 1) {
			await faYield(self);
		}
		expect(fired).toBe(true);
	});
});

describe("tests that await", () => {
	test("run one after the other as tasks; a failed assert fails only its test", async () => {
		/** @type {string[]} */
		const order = [];
		const tests = [
			{
				name: "first",
				file: "a.test.fa",
				line: 1,
				modifier: undefined,
				async: true,
				run: async (/** @type {FaTask} */ task) => {
					order.push("first starts");
					await faYield(task);
					order.push("first ends");
				},
			},
			{
				name: "failing",
				file: "a.test.fa",
				line: 2,
				modifier: undefined,
				async: true,
				run: async (/** @type {FaTask} */ task) => {
					await faYield(task);
					throw new FaStop("assertion failed: x == 1", "a.test.fa:2:1");
				},
			},
			{
				name: "blocking",
				file: "a.test.fa",
				line: 3,
				modifier: undefined,
				run: () => order.push("blocking"),
			},
		];
		const code = await faRunTests([["a.test", tests]], () => {}, faTestTask);
		expect(code).toBe(1);
		expect(order).toEqual(["first starts", "first ends", "blocking"]);
	});
});

describe("values of async functions", () => {
	test("an awaited value runs its async instance, a blocking call the other", async () => {
		const self = root();
		const value = faAsyncValue(
			(/** @type {number} */ x) => `sync ${x}`,
			async (/** @type {FaTask} */ task, /** @type {number} */ x) => {
				await faYield(task);
				return `async ${x}`;
			},
		);
		expect(await faCallValue(self, value, 1)).toBe("async 1");
		expect(value(2)).toBe("sync 2");
		expect(await faCallValue(self, (/** @type {number} */ x) => x + 1, 2)).toBe(3);
	});

	test("a started value waits for its turn", async () => {
		const self = root();
		/** @type {string[]} */
		const order = [];
		const value = faAsyncValue(
			() => order.push("sync"),
			async () => {
				order.push("async");
				return 7;
			},
		);
		const task = faStartValue(self, value);
		order.push("starter");
		expect(await faJoin(self, task)).toBe(7);
		expect(order).toEqual(["starter", "async"]);
	});
});

describe("faWaitShared", () => {
	/**
	 * A task that waits for a shared word, which another "thread" changes
	 * and notifies a little later.
	 * @returns {Promise<number>} how long it waited, in milliseconds
	 */
	const waitForChange = async () => {
		const cell = new Int32Array(new SharedArrayBuffer(4));
		const task = new FaTask(null);
		setTimeout(() => {
			Atomics.store(cell, 0, 1);
			Atomics.notify(cell, 0);
		}, 5);
		const start = performance.now();
		await faWaitShared(task, cell, 0, 0, "test", 0);
		expect(Atomics.load(cell, 0)).toBe(1);
		return performance.now() - start;
	};

	test("wakes when the word is notified (Atomics.waitAsync)", async () => {
		expect(await waitForChange()).toBeLessThan(500);
	});

	test("without Atomics.waitAsync, a timer tries again", async () => {
		/** @type {any} */
		const atomics = Atomics;
		const waitAsync = atomics.waitAsync;
		delete atomics.waitAsync;
		try {
			const cell = new Int32Array(new SharedArrayBuffer(4));
			const task = new FaTask(null);
			let attempts = 0;
			setTimeout(() => Atomics.store(cell, 0, 1), 5);
			while (Atomics.load(cell, 0) === 0) {
				await faWaitShared(task, cell, 0, 0, "test", attempts);
				attempts += 1;
			}
			expect(attempts).toBeGreaterThan(0);
		} finally {
			atomics.waitAsync = waitAsync;
		}
	});

	test("a cancelled task stops waiting", async () => {
		const cell = new Int32Array(new SharedArrayBuffer(4));
		const task = new FaTask(null);
		const waiting = faWaitShared(task, cell, 0, 0, "test", 0);
		task.state = 2;
		task.wake?.();
		await expect(waiting).rejects.toBe(faCancelSignal);
	});

	test("a cancelled waiter doesn't keep the program alive", () => {
		const script = `
			import { FaTask, faWaitShared } from "${import.meta.dir}/tasks.js";
			const task = new FaTask(null);
			const cell = new Int32Array(new SharedArrayBuffer(4));
			const waiting = faWaitShared(task, cell, 0, 0, "test", 0).catch(() => {});
			task.state = 2;
			task.wake();
			await waiting;
		`;
		const start = performance.now();
		const result = Bun.spawnSync([process.execPath, "-e", script]);
		expect(result.exitCode).toBe(0);
		expect(performance.now() - start).toBeLessThan(800);
	});
});
