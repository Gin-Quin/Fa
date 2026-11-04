// Workers on JavaScript (design/async.md, step J2): the answers of a
// worker's calls. The threads themselves are tested by the programs
// (`async-workers.fa`, `async-worker-pool.fa`, `async-worker-stress.fa`).

import { describe, expect, test } from "bun:test";

import { FaAtomic, FaChannel, FaMutex, faUnlock } from "./concurrency.js";
import { IndexError } from "./errors.js";
import { FaStop } from "./stops.js";
import { faCancelSignal } from "./tasks.js";
import { faRevive, faShare, faWorkerAnswer, faWorkerError } from "./workers.js";

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

	test("a channel can't be given to a worker yet", () => {
		expect(() => faShare([new FaChannel(1)])).toThrow(FaStop);
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
