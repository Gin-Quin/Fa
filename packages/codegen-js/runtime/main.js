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
// On Bun, Node and Deno the exit goes through `process.exit`. On the web,
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
 * `Errors` type, which sets the static `faName`. Recognized without the
 * `FaError` class, so that a program that throws no error doesn't contain it,
 * nor the formatting helpers its `toString` needs.
 * @param {unknown} exception
 * @returns {boolean}
 */
export function faIsError(exception) {
	return (
		typeof exception === "object" &&
		exception !== null &&
		typeof (/** @type {any} */ (exception).constructor?.faName) === "string"
	);
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
