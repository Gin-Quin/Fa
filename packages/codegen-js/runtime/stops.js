// Stops: the failures of runtime checks (values brief section 2.7, D-26).
//
// A stop is not an error: no compiled `catch` intercepts it (the lowering of
// `catch` only keeps `FaError` instances, see `faCaught`). The entry point
// (`faRun`) reports it on stderr and exits with code 101.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

// @fa-helper FaStop
/**
 * The exception thrown by a stop. It extends `Error` so that an uncaught stop
 * on the web is reported with a stack trace by the browser console.
 */
export class FaStop extends Error {
	/**
	 * @param {string} reason what failed, like `integer overflow: 128 doesn't fit in Int8`
	 * @param {string | undefined} location `file:line:column` of the check
	 */
	constructor(reason, location) {
		super(location === undefined ? reason : `${reason}\n  at ${location}`);
		this.name = "FaStop";
		/** @type {string} */
		this.reason = reason;
		/** @type {string | undefined} */
		this.location = location;
	}
}

// @fa-helper faStop deps: FaStop
/**
 * Stops the program: throws a `FaStop`.
 * @param {string} reason
 * @param {string} [location] `file:line:column`
 * @returns {never}
 */
export function faStop(reason, location) {
	throw new FaStop(reason, location);
}

// @fa-helper faOverflow deps: faStop
/**
 * Stops the program on an integer overflow.
 * @param {number | bigint} value the exact value when known
 * @param {string} type the Fa type that the value doesn't fit in
 * @param {string} [location]
 * @returns {never}
 */
export function faOverflow(value, type, location) {
	return faStop(`integer overflow: ${value} doesn't fit in ${type}`, location);
}

// @fa-helper faStopMessage
/**
 * The text a stop prints on stderr: `stop: <reason>`, then `  at <location>`.
 * @param {FaStop} stop
 * @returns {string}
 */
export function faStopMessage(stop) {
	return stop.location === undefined
		? `stop: ${stop.reason}`
		: `stop: ${stop.reason}\n  at ${stop.location}`;
}

// @fa-helper faAssertFailed deps: faStop
/**
 * A false `assert` outside a test: stops with `assertion failed: <source>`,
 * followed by the operands of its top-level comparison when given.
 * @param {string} source the source text of the condition
 * @param {string} [location]
 * @param {string} [operands] like `left: 5, right: 6`
 * @returns {never}
 */
export function faAssertFailed(source, location, operands) {
	return faStop(
		operands === undefined
			? `assertion failed: ${source}`
			: `assertion failed: ${source}\n       ↳ ${operands}`,
		location,
	);
}

// @fa-helper faIsStackOverflow
/**
 * Whether a JavaScript exception is the engine's call-stack overflow, which
 * the entry point converts into a stop (D-26).
 * @param {unknown} exception
 * @returns {boolean}
 */
export function faIsStackOverflow(exception) {
	return exception instanceof RangeError && /call stack/i.test(exception.message);
}
