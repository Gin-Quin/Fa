import { describe, expect, test } from "bun:test";

import { faArrayClear, faArrayDelete, faArrayDeleteWhere, faArrayTake } from "./collections.js";
import { FaChannel, FaMutex, faUnlock } from "./concurrency.js";
import { FaTcpStream } from "./net.js";
import { faDropOwned, faOwnShared, faReleaseShared, faRetainShared } from "./ownership.js";
import {
	faDerived,
	faDispose,
	faFlush,
	faNotify,
	faRead,
	faSource,
	faTrack,
	faWatch,
} from "./reactive.js";
import {
	faCancel,
	faCancelSignal,
	faLock,
	faReceive,
	faSend,
	faSleep,
	faStart,
	faStartOwned,
	FaTask,
} from "./tasks.js";

describe("shared handle owners", () => {
	test("a copied guard keeps its mutex locked until its last owner leaves", () => {
		const mutex = new FaMutex(41);
		const guard = mutex.tryLock();
		const copy = faRetainShared(guard);
		faUnlock(copy);
		expect(mutex.tryLock()).toBeUndefined();
		expect(guard.value).toBe(41);
		guard.value += 1;
		faUnlock(guard);
		const next = mutex.tryLock();
		expect(next.value).toBe(42);
		faUnlock(next);
		faReleaseShared(mutex);
	});

	test("a guard keeps the local mutex value alive after its handle leaves", () => {
		let destroyed = 0;
		const value = faOwnShared({ id: 7 }, () => {
			destroyed += 1;
		});
		const mutex = new FaMutex(value);
		const guard = mutex.tryLock();
		faReleaseShared(mutex);
		expect(destroyed).toBe(0);
		expect(guard.value.id).toBe(7);
		faUnlock(guard);
		expect(destroyed).toBe(1);
	});

	test("the last local channel drops only the values it still owns", () => {
		const destroyed = [];
		const first = faOwnShared({ id: 1 }, (value) => {
			destroyed.push(value.id);
		});
		const second = faOwnShared({ id: 2 }, (value) => {
			destroyed.push(value.id);
		});
		const channel = new FaChannel();
		expect(channel.trySend(first)).toBe(true);
		expect(channel.trySend(second)).toBe(true);
		const copy = faRetainShared(channel);
		const received = channel.tryReceive();
		faReleaseShared(copy);
		expect(destroyed).toEqual([]);
		faReleaseShared(channel);
		expect(destroyed).toEqual([2]);
		faReleaseShared(received);
		expect(destroyed).toEqual([2, 1]);
	});

	test("a realm's last channel handle does not close a shared queue", () => {
		const channel = new FaChannel();
		channel.trySend(3);
		channel.share();
		faReleaseShared(channel);
		expect(Atomics.load(channel.cell, 1)).toBe(0);
		expect(channel.pollReceive()).toBe(3);
	});

	test("copied subscriptions stop on their last drop; explicit stop stays global", () => {
		const source = faSource();
		let runs = 0;
		let cleanups = 0;
		const subscription = faWatch(() => {
			faTrack(source);
			runs += 1;
			return () => {
				cleanups += 1;
			};
		});
		const copy = faRetainShared(subscription);
		faReleaseShared(copy);
		faNotify(source);
		faFlush();
		expect([runs, cleanups]).toEqual([2, 1]);
		const stoppedCopy = faRetainShared(subscription);
		faDispose(stoppedCopy);
		faNotify(source);
		faFlush();
		expect([runs, cleanups]).toEqual([2, 2]);
		faReleaseShared(stoppedCopy);
		faReleaseShared(subscription);
		expect(cleanups).toBe(2);
	});

	test("a socket closes only once when its last owner leaves", () => {
		let ended = 0;
		const raw = {
			on() {},
			unref() {},
			end() {
				ended += 1;
			},
		};
		const socket = new FaTcpStream(raw, true);
		const copy = faRetainShared(socket);
		faReleaseShared(copy);
		expect(ended).toBe(0);
		faReleaseShared(socket);
		faReleaseShared(socket);
		expect(ended).toBe(1);
	});

	test("an erased function releases its registered environment exactly once", () => {
		let released = 0;
		const closure = faOwnShared(
			() => 4,
			() => {
				released += 1;
			},
		);
		const copy = faRetainShared(closure);
		faDropOwned({ callbacks: [copy] });
		expect(released).toBe(0);
		faDropOwned(closure);
		expect(released).toBe(1);
	});

	test("an erased unique task cancels without walking its parent cycle", async () => {
		const parent = new FaTask(null);
		const child = faStart(parent, async () => 4);
		faDropOwned(child);
		expect(child.state).toBe(2);
		expect(parent.state).toBe(0);
		await Promise.resolve();
	});

	test("a cancelled start-do releases its captures before its first turn", async () => {
		let releases = 0;
		let called = false;
		const task = faStartOwned(
			null,
			() => {
				called = true;
			},
			() => {
				releases += 1;
			},
		);
		faCancel(task);
		expect(releases).toBe(1);
		await Promise.resolve();
		expect(called).toBe(false);
		expect(releases).toBe(1);
	});

	test("a running start-do releases its captures after its body's finally", async () => {
		const order = [];
		let entered;
		const started = new Promise((resolve) => {
			entered = resolve;
		});
		const task = faStartOwned(
			null,
			async (self) => {
				entered();
				try {
					await faSleep(self, 10000);
				} finally {
					order.push("body");
				}
			},
			() => {
				order.push("captures");
			},
		);
		await started;
		faCancel(task);
		await Promise.resolve();
		await Promise.resolve();
		await Promise.resolve();
		expect(order).toEqual(["body", "captures"]);
	});

	test("a derived value releases its captured function and cached resource", () => {
		let destroyed = 0;
		const resource = faOwnShared({ id: 1 }, () => {
			destroyed += 1;
		});
		const compute = faOwnShared(
			() => faRetainShared(resource),
			() => faDropOwned(resource),
		);
		const derived = faDerived(compute);
		expect(faRead(derived).id).toBe(1);
		faReleaseShared(derived);
		expect(destroyed).toBe(1);
	});

	test("a failed or cancelled send drops its value exactly once", async () => {
		let destroyed = 0;
		const closed = new FaChannel(1);
		closed.close();
		expect(
			closed.trySend(
				faOwnShared({}, () => {
					destroyed += 1;
				}),
			),
		).toBe(false);
		expect(destroyed).toBe(1);
		const channel = new FaChannel(0);
		const self = new FaTask(null);
		const pending = faSend(
			self,
			channel,
			faOwnShared({}, () => {
				destroyed += 1;
			}),
			"test",
		);
		faReleaseShared(channel);
		faCancel(self);
		await expect(pending).rejects.toBe(faCancelSignal);
		expect(destroyed).toBe(2);
		faReleaseShared(closed);
	});

	test("a receiver cancelled during handoff drops the handed value", async () => {
		let destroyed = 0;
		const channel = new FaChannel(1);
		const self = new FaTask(null);
		const pending = faReceive(self, channel, "test");
		channel.trySend(
			faOwnShared({}, () => {
				destroyed += 1;
			}),
		);
		faCancel(self);
		await expect(pending).rejects.toBe(faCancelSignal);
		expect(destroyed).toBe(1);
		faReleaseShared(channel);
	});

	test("a cancelled lock handoff releases its guard and mutex owner", async () => {
		let destroyed = 0;
		const mutex = new FaMutex(
			faOwnShared({}, () => {
				destroyed += 1;
			}),
		);
		const first = mutex.tryLock();
		const self = new FaTask(null);
		const pending = faLock(self, mutex, "test");
		faUnlock(first);
		faCancel(self);
		await expect(pending).rejects.toBe(faCancelSignal);
		const next = mutex.tryLock();
		expect(next).not.toBeUndefined();
		faUnlock(next);
		faReleaseShared(mutex);
		expect(destroyed).toBe(1);
	});

	test("deleting and clearing arrays drops removed owners; take transfers one", () => {
		const destroyed = [];
		const make = (id) =>
			faOwnShared({ id }, () => {
				destroyed.push(id);
			});
		const values = [make(1), make(2), make(3), make(4)];
		faArrayDelete(values, 1, faDropOwned);
		expect(destroyed).toEqual([2]);
		const kept = faArrayTake(values, 0);
		faArrayDeleteWhere(values, (item) => item.id === 3, faDropOwned);
		expect(destroyed).toEqual([2, 3]);
		faArrayClear(values, faDropOwned);
		expect(destroyed).toEqual([2, 3, 4]);
		faReleaseShared(kept);
		expect(destroyed).toEqual([2, 3, 4, 1]);
	});
});

