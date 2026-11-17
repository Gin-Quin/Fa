// Errors, stops and the entry point (values brief 2.7 and 15, D-24, D-26,
// D-92): `catch` lowering, exit codes and messages, run in a child process.

import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { DivisionByZero, faCaught, FaError, IndexError, KeyNotFound } from "./errors.js";
import { faFloorDivide } from "./integers.js";
import {
	faAssertFailed,
	faIsStackOverflow,
	faOverflow,
	FaStop,
	faStop,
	faStopMessage,
} from "./stops.js";

/**
 * The exception thrown by `run`.
 * @param {() => unknown} run
 * @returns {any}
 */
function errorOf(run) {
	try {
		run();
	} catch (exception) {
		return exception;
	}
	throw new Error("no exception");
}

/**
 * Runs a module importing the runtime in a child process.
 * @param {string} code its body; `$` is the runtime directory
 * @param {string} [engine] the JavaScript engine to run
 * @returns {{ code: number, stdout: string, stderr: string }}
 */
function runProgram(code, engine = process.execPath) {
	const directory = mkdtempSync(join(tmpdir(), "fa-runtime-"));
	try {
		const file = join(directory, "program.mjs");
		writeFileSync(file, code.replaceAll("$/", `${import.meta.dir}/`));
		const result = Bun.spawnSync([engine, file], { stdout: "pipe", stderr: "pipe" });
		return {
			code: result.exitCode ?? -1,
			stdout: result.stdout.toString(),
			stderr: result.stderr.toString(),
		};
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
}

describe("the lowering of catch (D-24)", () => {
	test("keeps Fa errors", () => {
		let handled;
		try {
			faFloorDivide(1, 0);
		} catch ($e) {
			handled = faCaught($e);
		}
		expect(handled).toBeInstanceOf(DivisionByZero);
	});

	test("rethrows stops and foreign JavaScript exceptions (T-ERR-27)", () => {
		const stop = new FaStop("integer overflow: 300 doesn't fit in UInt8", "main.fa:1:1");
		expect(errorOf(() => faCaught(stop))).toBe(stop);
		const foreign = new TypeError("x is not a function");
		expect(errorOf(() => faCaught(foreign))).toBe(foreign);
		expect(errorOf(() => faCaught("thrown string"))).toBe("thrown string");
	});

	test("errors are FaError, not Error; stops are Error", () => {
		expect(new IndexError(1, 0)).toBeInstanceOf(FaError);
		expect(new KeyNotFound("a") instanceof Error).toBe(false);
		expect(new FaStop("x", undefined)).toBeInstanceOf(Error);
	});
});

describe("stops (D-26)", () => {
	test("messages", () => {
		const stop = errorOf(() => faOverflow(300, "UInt8", "main.fa:3:9"));
		expect(stop).toBeInstanceOf(FaStop);
		expect(faStopMessage(stop)).toBe(
			"stop: integer overflow: 300 doesn't fit in UInt8\n  at main.fa:3:9",
		);
		expect(faStopMessage(errorOf(() => faStop("out of memory")))).toBe("stop: out of memory");
		expect(faStopMessage(errorOf(() => faAssertFailed("total == 10", "t.fa:2:1")))).toBe(
			"stop: assertion failed: total == 10\n  at t.fa:2:1",
		);
		expect(errorOf(() => faAssertFailed("a == b", undefined, "left: 5, right: 6")).reason).toBe(
			"assertion failed: a == b\n       ↳ left: 5, right: 6",
		);
	});

	test("a call-stack overflow is recognized", () => {
		/** @param {number} n @returns {number} */
		const deep = (n) => deep(n + 1) + 1;
		expect(faIsStackOverflow(errorOf(() => deep(0)))).toBe(true);
		expect(faIsStackOverflow(new RangeError("Invalid array length"))).toBe(false);
		expect(faIsStackOverflow(new TypeError("call stack"))).toBe(false);
	});
});

describe("the entry point", () => {
	test("a normal end exits with 0, or the returned Integer", () => {
		const done = runProgram(`
			import { faRun } from "$/main.js";
			import { faFormat } from "$/format.js";
			faRun(() => console.log(["Hello", 1, [1, "a"], undefined].map(faFormat).join(" ")));
		`);
		expect(done).toEqual({ code: 0, stdout: 'Hello 1 [1, "a"] none\n', stderr: "" });
		const returned = runProgram(`
			import { faRun } from "$/main.js";
			faRun(() => 3);
		`);
		expect(returned.code).toBe(3);
		expect(runProgram(`import { faRun } from "$/main.js"; faRun(() => 4n);`).code).toBe(4);
	});

	test("the tests of one file run alone, and their counts end the output", () => {
		const files = `[
			["a.test", [
				{ name: "one", file: "a.test.fa", line: 1, run: () => {} },
				{ name: "two", file: "a.test.fa", line: 2, modifier: "skip", run: () => {} },
			]],
			["b.test", [{ name: "three", file: "b.test.fa", line: 1, run: () => { throw new Error("no"); } }]],
		]`;
		const program = (/** @type {string[]} */ argv) => `
			import { faRun, faRunTests } from "$/main.js";
			faRun(() => faRunTests(${files}, () => {}, undefined, ${JSON.stringify(argv)}));
		`;
		expect(runProgram(program(["--fa-test-file=a.test"]))).toEqual({
			code: 0,
			stdout: "✓ one\n\u0001fa-test-summary 1 0 1 0\n",
			stderr: "",
		});
		// every file, without the argument
		const all = runProgram(program([]));
		expect(all.stdout.startsWith("✓ one\n")).toBe(true);
		expect(all.code).not.toBe(0);
	});

	test("an escaped error prints `error: <toString>` and exits with 1", () => {
		const result = runProgram(`
			import { faRun } from "$/main.js";
			import { faIndex } from "$/collections.js";
			faRun(() => { console.log("before"); faIndex([1], 2); console.log("after"); });
		`);
		expect(result).toEqual({
			code: 1,
			stdout: "before\n",
			stderr: "error: IndexError { index = 2, length = 1 }\n",
		});
	});

	test("a stop prints its reason and location and exits with 101, through catch", () => {
		const result = runProgram(`
			import { faRun } from "$/main.js";
			import { faCaught } from "$/errors.js";
			import { faCheckInteger } from "$/integers.js";
			faRun(() => {
				try {
					faCheckInteger(9007199254740991 + 1, "main.fa:2:13");
				} catch ($e) {
					faCaught($e);
					console.log("caught");
				}
			});
		`);
		expect(result.code).toBe(101);
		expect(result.stdout).toBe("");
		expect(result.stderr).toBe(
			"stop: integer overflow: 9007199254740992 doesn't fit in Integer (±(2^53 − 1) on JavaScript)\n  at main.fa:2:13\n",
		);
	});

	test("a call-stack overflow is a stop", () => {
		const result = runProgram(`
			import { faRun } from "$/main.js";
			const deep = (n) => deep(n + 1) + 1;
			faRun(() => deep(0));
		`);
		expect(result).toEqual({ code: 101, stdout: "", stderr: "stop: stack overflow\n" });
	});

	test("a foreign JavaScript exception is not caught by the runtime", () => {
		const result = runProgram(`
			import { faRun } from "$/main.js";
			faRun(() => { null.x; });
		`);
		// the runtime's own exit code (1 on Bun and Node), with its report
		expect(result.code).not.toBe(0);
		expect(result.code).not.toBe(101);
		expect(result.stderr).toContain("TypeError");
		expect(result.stderr.startsWith("error: ")).toBe(false);
		expect(result.stderr.startsWith("stop: ")).toBe(false);
	});
});

describe("allocation failures", () => {
	test("Node size failures from string and byte operations become stops", () => {
		for (const expression of [
			'"abc".repeat(600000000)',
			'"abc".padStart(1200000000, "é")',
			'"abc".padEnd(1200000000, "😀")',
			"new Uint8Array(Number.MAX_SAFE_INTEGER)",
		]) {
			const result = runProgram(
				`
				import { faRun } from "$/main.js";
				faRun(() => { ${expression}; });
			`,
				`${process.env.HOME}/.local/bin/node`,
			);
			expect(result).toEqual({ code: 101, stdout: "", stderr: "stop: out of memory\n" });
		}
	});
	for (const message of [
		"Invalid string length",
		"Invalid array length",
		"Invalid typed array length: 9007199254740991",
		"Array buffer allocation failed",
		"Out of memory",
	]) {
		test(`${message} is an allocation stop`, () => {
			const result = runProgram(`
				import { faRun, faReport } from "$/main.js";
				import { faCaught } from "$/errors.js";
				faRun(() => {
					try { throw new RangeError(${JSON.stringify(message)}); }
					catch (error) { faCaught(error); }
				});
			`);
			expect(result).toEqual({ code: 101, stdout: "", stderr: "stop: out of memory\n" });
		});
	}
	test("an unrelated RangeError retains its host report", () => {
		const result = runProgram(`
			import { faRun } from "$/main.js";
			faRun(() => { throw new RangeError("foreign range"); });
		`);
		expect(result.code).toBe(1);
		expect(result.stderr).toContain("RangeError: foreign range");
	});
});
