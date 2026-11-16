// Workers on JavaScript (design/async.md, step J2): the answers of a
// worker's calls. The threads themselves are tested by the programs
// (`async-workers.fa`, `async-worker-pool.fa`, `async-worker-stress.fa`).

import { describe, expect, test } from "bun:test";

import { FaAtomic, FaChannel, FaMutex, faUnlock } from "./concurrency.js";
import { faEquals } from "./equality.js";
import { FaError, IndexError } from "./errors.js";
import { FaStop } from "./stops.js";
import { faCancel, faCancelSignal } from "./tasks.js";
import {
	faRevive,
	faShare,
	FaWorker,
	faWorkerAnswer,
	faWorkerError,
	faWorkerStart,
} from "./workers.js";

describe("queued worker calls", () => {
	function setup() {
		/** @type {any[]} */
		const messages = [];
		/** @type {FaWorker} */
		const worker = Object.assign(Object.create(FaWorker.prototype), {
			calls: new Map(),
			next: 0,
			waiting: [],
			waitingHead: 0,
			active: null,
			scheduled: false,
			dead: false,
			stopping: false,
			thread: {
				postMessage: (/** @type {any} */ message) => messages.push(structuredClone(message)),
			},
		});
		return { worker, messages };
	}

	test("cancellation before the starter suspends never commits the call", async () => {
		const { worker, messages } = setup();
		const args = [{ value: 1 }];
		const task = faWorkerStart(null, worker, "test#add", args, []);
		args[0].value = 9;
		faCancel(task);
		await Promise.resolve();
		expect(messages.map((message) => message.kind)).toEqual([1, 2]);
		expect(messages[0].args).toEqual([{ value: 1 }]);
		expect(worker.calls.size).toBe(0);
		expect(worker.waiting).toEqual([]);
	});

	test("queued calls wait for the running call and cancelled ones are skipped", async () => {
		const { worker, messages } = setup();
		faWorkerStart(null, worker, "test#first", [], []);
		await Promise.resolve();
		const cancelled = faWorkerStart(null, worker, "test#second", [], []);
		faWorkerStart(null, worker, "test#third", [], []);
		faCancel(cancelled);
		await Promise.resolve();
		expect(messages.filter((message) => message.kind === 3).map((message) => message.id)).toEqual([
			0,
		]);
		worker.calls.delete(0);
		worker.active = null;
		worker.dispatch();
		await Promise.resolve();
		expect(messages.filter((message) => message.kind === 3).map((message) => message.id)).toEqual([
			0, 2,
		]);
	});
});

describe("faWorkerError", () => {
	test("rebuilds an error with its class", () => {
		const error = faWorkerError({ name: "IndexError", fields: { index: 10, length: 5 } }, [
			IndexError,
		]);
		expect(error).toBeInstanceOf(IndexError);
		expect(error.index).toBe(10);
		expect(error.length).toBe(5);
	});

	test("an error of another class keeps its name", () => {
		const error = faWorkerError({ name: "Parse.Bad", fields: { line: 3 } }, []);
		expect(error.constructor.faName).toBe("Parse.Bad");
		expect(error.line).toBe(3);
	});

	test("same displayed names keep their separate declaring classes", () => {
		class Left extends FaError {
			static faName = "Problem.Bad";
			static faId = this.register("worker-test:left");
		}
		class Right extends FaError {
			static faName = "Problem.Bad";
			static faId = this.register("worker-test:right");
			toString() {
				return `right ${/** @type {any} */ (this).value}`;
			}
		}
		const message = structuredClone({
			name: "Problem.Bad",
			id: "worker-test:right",
			fields: { value: 3 },
		});
		const known = faWorkerError(message, [Left, Right]);
		const erased = faWorkerError(message, []);
		for (const error of [known, erased]) {
			expect(error).toBeInstanceOf(Right);
			expect(error).not.toBeInstanceOf(Left);
			expect(error.toString()).toBe("right 3");
			expect(error[Symbol.for("fa.error.type")]).toBe("worker-test:right");
		}
		class OtherModule extends FaError {
			static faId = "worker-test:right";
		}
		const sameType = Object.assign(new OtherModule(), { value: 3 });
		expect(faEquals(known, sameType)).toBe(true);
		expect(faEquals(known, Object.assign(new Left(), { value: 3 }))).toBe(false);
	});

	test("an unavailable class retains its type and typed display", () => {
		const error = faWorkerError(
			{
				name: "Problem.Bad",
				id: "worker-test:unavailable",
				display: "Problem.Bad { ratio = 1.2 }",
				fields: { ratio: 1.2000000476837158 },
			},
			[],
		);
		expect(error).toBeInstanceOf(FaError);
		expect(error[Symbol.for("fa.error.type")]).toBe("worker-test:unavailable");
		expect(error.toString()).toBe("Problem.Bad { ratio = 1.2 }");
		expect(error.ratio).toBe(1.2000000476837158);
	});
});

describe("thread-safe values given to a worker", () => {
	test("an atomic and a mutex share their memory through a structured clone", () => {
		const hits = new FaAtomic(5);
		const flag = new FaAtomic(false);
		const total = new FaMutex({ count: 1, label: "a", none: undefined, big: 2n });
		const message = [hits, flag, { total }];
		faShare(message);
		const [otherHits, otherFlag, { total: otherTotal }] = faRevive(structuredClone(message));
		expect(otherHits).toBeInstanceOf(FaAtomic);
		otherHits.add(10);
		expect(hits.load()).toBe(15);
		expect(hits.increment()).toBe(16);
		expect(otherHits.compareAndSwap(16, 1)).toBe(true);
		expect(hits.swap(3)).toBe(1);
		otherFlag.store(true);
		expect(flag.load()).toBe(true);
		const guard = otherTotal.tryLock();
		expect(guard.value).toEqual({ count: 1, label: "a", none: undefined, big: 2n });
		expect(total.tryLock()).toBeUndefined();
		guard.value = { count: 2, label: "b", none: undefined, big: 3n };
		faUnlock(guard);
		const mine = total.tryLock();
		expect(mine.value.count).toBe(2);
		expect(mine.value.big).toBe(3n);
		faUnlock(mine);
	});

	test("a channel shares its queue through a structured clone", () => {
		const channel = new FaChannel(2);
		channel.trySend(1n);
		faShare([channel]);
		const [other] = faRevive(structuredClone([channel]));
		expect(other).toBeInstanceOf(FaChannel);
		expect(other.trySend(undefined)).toBe(true);
		expect(channel.trySend(3)).toBe(false);
		expect(other.tryReceive()).toBe(1n);
		expect(channel.trySend("three")).toBe(true);
		expect(channel.tryReceive()).toBeUndefined();
		expect(other.tryReceive()).toBe("three");
		expect(channel.tryReceive()).toBeUndefined();
		other.close();
		expect(channel.trySend(4)).toBe(false);
		expect(channel.pollReceive(FaChannel)).toBeUndefined();
	});

	test("without shared memory, giving one to a worker stops with what to do", () => {
		const shared = globalThis.SharedArrayBuffer;
		// @ts-ignore a web page that isn't cross-origin isolated
		delete globalThis.SharedArrayBuffer;
		try {
			expect(() => faShare([new FaChannel(1)])).toThrow(FaStop);
			expect(() => faShare([new FaAtomic(1)])).toThrow(FaStop);
		} finally {
			globalThis.SharedArrayBuffer = shared;
		}
	});
});

describe("faWorkerAnswer", () => {
	test("gives the value, throws the error, or the cancellation", () => {
		expect(faWorkerAnswer({ value: [1, 2] }, [])).toEqual([1, 2]);
		expect(() =>
			faWorkerAnswer({ error: { name: "IndexError", fields: { index: 1, length: 0 } } }, [
				IndexError,
			]),
		).toThrow(IndexError);
		expect(() => faWorkerAnswer({ cancelled: true }, [])).toThrow();
		try {
			faWorkerAnswer({ cancelled: true }, []);
		} catch (exception) {
			expect(exception).toBe(faCancelSignal);
		}
	});
});

test("shared mutexes and channels preserve Bytes as Uint8Array", () => {
	const bytes = new Uint8Array([0, 195, 169, 255]);
	const mutex = new FaMutex({ bytes });
	const channel = new FaChannel(1);
	channel.trySend(bytes);
	faShare([mutex, channel]);
	const [otherMutex, otherChannel] = faRevive(structuredClone([mutex, channel]));
	const guard = otherMutex.tryLock();
	expect(guard.value.bytes).toEqual(bytes);
	guard.value.bytes[0] = 42;
	faUnlock(guard);
	const mine = mutex.tryLock();
	expect(mine.value.bytes[0]).toBe(42);
	expect(mine.value.bytes).toBeInstanceOf(Uint8Array);
	faUnlock(mine);
	expect(otherChannel.tryReceive()).toEqual(bytes);
});
