import { describe, expect, test } from "bun:test";

import { faOwnShared, faReleaseShared } from "./ownership.js";
import {
	faAlive,
	faAsyncValue,
	faCallValue,
	faCancel,
	faSched,
	faStartValue,
	faStartValueDynamic,
	FaTask,
	faWhenDone,
} from "./tasks.js";

function deferred() {
	let resolve;
	const promise = new Promise((done) => {
		resolve = done;
	});
	return { promise, resolve };
}
function ended(task) {
	return new Promise((done) => faWhenDone(task, () => done(undefined)));
}

describe("Function environments of tasks", () => {
	test("a queued call retains before its first turn and through suspension", async () => {
		let destroyed = 0;
		const began = deferred();
		const resume = deferred();
		const f = faOwnShared(
			faAsyncValue(
				() => 0,
				async () => {
					began.resolve();
					await resume.promise;
					expect(destroyed).toBe(0);
					return 7;
				},
			),
			() => {
				destroyed++;
			},
		);
		const task = faStartValue(null, f);
		faReleaseShared(f);
		expect(destroyed).toBe(0);
		await began.promise;
		expect(destroyed).toBe(0);
		resume.resolve();
		await ended(task);
		expect(task.value).toBe(7);
		expect(destroyed).toBe(1);
	});
	test("cancellation before the first turn releases without invoking", async () => {
		let destroyed = 0;
		let invoked = 0;
		const f = faOwnShared(
			() => {
				invoked++;
				return 7;
			},
			() => {
				destroyed++;
			},
		);
		const task = faStartValue(null, f);
		faReleaseShared(f);
		faCancel(task);
		expect(destroyed).toBe(1);
		await Promise.resolve();
		expect(invoked).toBe(0);
		expect(destroyed).toBe(1);
	});
	test("cancellation during an await waits for the actual finally", async () => {
		let destroyed = 0;
		const began = deferred();
		const resume = deferred();
		const finished = deferred();
		const f = faOwnShared(
			faAsyncValue(
				() => 0,
				async (self) => {
					began.resolve();
					try {
						await resume.promise;
						faAlive(self);
					} finally {
						expect(destroyed).toBe(0);
						finished.resolve();
					}
				},
			),
			() => {
				destroyed++;
			},
		);
		const task = faStartValue(null, f);
		faReleaseShared(f);
		await began.promise;
		faCancel(task);
		expect(destroyed).toBe(0);
		resume.resolve();
		await finished.promise;
		await Promise.resolve();
		await Promise.resolve();
		expect(destroyed).toBe(1);
	});
	test("an awaited call retains the function until its result resolves", async () => {
		let destroyed = 0;
		const resume = deferred();
		const f = faOwnShared(
			faAsyncValue(
				() => 0,
				async () => {
					await resume.promise;
					return 9;
				},
			),
			() => {
				destroyed++;
			},
		);
		const result = faCallValue(new FaTask(null), f);
		faReleaseShared(f);
		expect(destroyed).toBe(0);
		resume.resolve();
		expect(await result).toBe(9);
		expect(destroyed).toBe(1);
	});
	test("a blocking dynamic call keeps its owner through reentrant release", () => {
		let destroyed = 0;
		const f = faOwnShared(
			() => {
				faReleaseShared(f);
				expect(destroyed).toBe(0);
				return 11;
			},
			() => {
				destroyed++;
			},
		);
		const depth = faSched.depth;
		faSched.depth = 1;
		try {
			expect(faStartValueDynamic(f).value).toBe(11);
		} finally {
			faSched.depth = depth;
		}
		expect(destroyed).toBe(1);
	});
	test("a synchronous awaited value preserves its plain result", () => {
		let destroyed = 0;
		const f = faOwnShared(
			() => {
				faReleaseShared(f);
				expect(destroyed).toBe(0);
				return 13;
			},
			() => {
				destroyed++;
			},
		);
		expect(faCallValue(new FaTask(null), f)).toBe(13);
		expect(destroyed).toBe(1);
	});
});
