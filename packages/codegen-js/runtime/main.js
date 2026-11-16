// The entry point of a compiled program (values brief 2.7 and 15, D-26,
// D-59; advanced brief 17, "exit code").
//
//     faRun(() => { ...top-level statements, or a call of the entry function... });
//
// - a normal end exits with the returned `Integer`, or 0;
// - an escaped error prints `error: <toString>` on stderr and exits with 1;
// - a stop (a `FaStop`, or the engine's call-stack overflow) prints
//   `stop: <reason>` and `  at <location>` on stderr and exits with 101.
//
// On Bun and Node the exit goes through `process.exit`. On the web,
// there is no process: the report goes to the console, the program stops
// running, and the exit code is ignored.

import { faIsStackOverflow, FaStop, faStopMessage } from "./stops.js";

// @fa-helper faOmitted
/**
 * The argument a call passes for an omitted parameter that is optional and
 * has a default, when a later argument is given: `undefined` is `none`
 * there, which the call may give explicitly (design/decisions.md).
 */
export const faOmitted = Symbol("fa.omitted");

// @fa-helper faWriteError
/**
 * Writes a line on stderr (the console on the web).
 * @param {string} text
 * @returns {void}
 */
export function faWriteError(text) {
	const process = /** @type {any} */ (globalThis).process;
	if (process?.stderr?.write !== undefined) {
		process.stderr.write(`${text}\n`);
	} else {
		console.error(text);
	}
}

// @fa-helper faExit
/**
 * Exits the process with a code, on server runtimes; does nothing on the
 * web. A zero code doesn't call `process.exit`, so that pending output is
 * flushed normally.
 * @param {number} code
 * @returns {void}
 */
export function faExit(code) {
	const process = /** @type {any} */ (globalThis).process;
	if (code !== 0 && typeof process?.exit === "function") {
		process.exit(code);
	}
}

// @fa-helper faIsError
/**
 * Whether an exception is a Fa error: an instance of a member class of an
 * `Errors` type, whose `Symbol.for("fa.error")` property is its qualified
 * name (see `FaError`), never a field of the value. Recognized without the
 * `FaError` class, so that a program that throws no error doesn't contain it,
 * nor the formatting helpers its `toString` needs.
 * @param {unknown} exception
 * @returns {boolean}
 */
export function faIsError(exception) {
	return typeof (/** @type {any} */ (exception)?.[Symbol.for("fa.error")]) === "string";
}

// @fa-helper faReport deps: FaStop faIsError faIsStackOverflow faStopMessage faWriteError
/**
 * Reports an exception that escaped the program and returns the exit code:
 * 101 for a stop (or a call-stack overflow), 1 for a Fa error. Any other
 * JavaScript exception is rethrown.
 * @param {unknown} exception
 * @returns {number}
 */
export function faReport(exception) {
	if (exception instanceof FaStop) {
		faWriteError(faStopMessage(exception));
		return 101;
	}
	if (faIsError(exception)) {
		faWriteError(`error: ${String(exception)}`);
		return 1;
	}
	if (faIsStackOverflow(exception)) {
		faWriteError("stop: stack overflow");
		return 101;
	}
	if (
		exception instanceof RangeError &&
		/^(Invalid (?:string|array|typed array) length|(?:Array buffer allocation failed|Out of memory))(?::.*)?$/i.test(exception.message)
	) {
		faWriteError("stop: out of memory");
		return 101;
	}
	throw exception;
}

// @fa-helper faRun deps: faReport faExit
/**
 * Runs the program and exits with its code.
 * @param {() => unknown} main the top-level statements, or a call of the entry function
 * @returns {number} the exit code (useful on the web, where nothing exits)
 */
export function faRun(main) {
	let code = 0;
	try {
		const result = main();
		if (typeof result === "number" || typeof result === "bigint") {
			code = Number(result);
		}
	} catch (exception) {
		code = faReport(exception);
	}
	faExit(code);
	return code;
}

// @fa-helper faRunTests deps: FaStop faStopMessage faIsError faIsStackOverflow
/**
 * Runs the tests of a program (`fa test`, advanced brief 18.2) and returns
 * the exit code: 0 when every test that ran passed, 1 otherwise.
 *
 * - When a test is `only`, only the focused tests run; `skip` never runs.
 * - Before each test, `init` initializes the module-level values again (D33).
 * - A false `assert` or an escaping error fails the test; a stop fails it
 *   too, and the remaining tests of its file are reported as not run.
 * - A test that awaits (`async: true`, in an asynchronous program) is an
 *   async function taking its task: `runAsync` (`faTestTask`) runs it as a
 *   task, and the runner waits for it before the next test; the result is
 *   then a promise of the exit code.
 * - `fa test` runs each test file in a process of its own: the argument
 *   `--fa-test-file=<module>` runs the tests of that file only (a `test
 *   only` of another file still hides them), and the counts are printed
 *   as the last line, after `FA_TEST_SUMMARY`, for `fa` to add them up.
 * @param {Array<[string, Array<{ name: string, file: string, line: number, modifier: string | undefined, run: Function, async?: boolean }>]>} files
 *   the tests of each test file, by module
 * @param {() => void} init
 * @param {(run: Function) => Promise<void>} [runAsync]
 * @param {string[]} [argv] the arguments of the program
 * @returns {number | Promise<number>}
 */
export function faRunTests(files, init, runAsync, argv = []) {
	const all = files.flatMap(([, tests]) => tests);
	const focused = all.some((test) => test.modifier === "only");
	const option = argv.find((argument) => argument.startsWith("--fa-test-file="));
	const worker = option?.slice("--fa-test-file=".length);
	const tests =
		worker === undefined
			? all
			: files.filter(([module]) => module === worker).flatMap(([, tests]) => tests);
	let passed = 0;
	let failed = 0;
	let skipped = 0;
	let notRun = 0;
	/** @type {string | undefined} */
	let stopped;
	/** @param {{ name: string }} test */
	const pass = (test) => {
		passed += 1;
		console.log(`✓ ${test.name}`);
	};
	/**
	 * @param {{ name: string, file: string }} test
	 * @param {string} where
	 * @param {unknown} exception
	 */
	const fail = (test, where, exception) => {
		failed += 1;
		let report;
		if (exception instanceof FaStop && exception.reason.startsWith("assertion failed: ")) {
			const condition = exception.reason.slice("assertion failed: ".length);
			report = `  assert ${condition.replace(/\n {7}/g, "\n         ")}`;
		} else if (exception instanceof FaStop) {
			report = `  ${faStopMessage(exception)}`;
			stopped = test.file;
		} else if (faIsError(exception)) {
			report = `  error: ${String(exception)}`;
		} else if (faIsStackOverflow(exception)) {
			report = "  stop: stack overflow";
			stopped = test.file;
		} else {
			throw exception;
		}
		console.log(`✗ ${test.name} (${where})\n${report}`);
	};
	/**
	 * Runs the tests from `index` on.
	 * @param {number} index
	 * @returns {number | Promise<number>}
	 */
	const step = (index) => {
		for (; index < tests.length; index += 1) {
			const test = tests[index];
			const where = `${test.file}:${test.line}`;
			if (test.file === stopped) {
				notRun += 1;
				console.log(`- ${test.name} (${where}): not run`);
				continue;
			}
			if (test.modifier === "skip" || (focused && test.modifier !== "only")) {
				skipped += 1;
				continue;
			}
			let result;
			try {
				init();
				result = test.async && runAsync !== undefined ? runAsync(test.run) : test.run();
			} catch (exception) {
				fail(test, where, exception);
				continue;
			}
			if (result instanceof Promise) {
				const next = index + 1;
				return result.then(
					() => {
						pass(test);
						return step(next);
					},
					(exception) => {
						fail(test, where, exception);
						return step(next);
					},
				);
			}
			pass(test);
		}
		if (worker !== undefined) {
			// FA_TEST_SUMMARY: read by `fa test`, which prints the total
			console.log(`\u0001fa-test-summary ${passed} ${failed} ${skipped} ${notRun}`);
			return failed > 0 ? 1 : 0;
		}
		const parts = [`${passed} passed`, `${failed} failed`];
		if (skipped > 0) {
			parts.push(`${skipped} skipped`);
		}
		if (notRun > 0) {
			parts.push(`${notRun} not run`);
		}
		console.log(`\n${parts.join(", ")}`);
		return failed > 0 ? 1 : 0;
	};
	return step(0);
}
