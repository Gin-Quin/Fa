// Workers on JavaScript (design/async.md, step J2): the answers of a
// worker's calls. The threads themselves are tested by the programs
// (`async-workers.fa`, `async-worker-pool.fa`, `async-worker-stress.fa`).

import { describe, expect, test } from "bun:test";

import { IndexError } from "./errors.js";
import { faCancelSignal } from "./tasks.js";
import { faWorkerAnswer, faWorkerError } from "./workers.js";

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
