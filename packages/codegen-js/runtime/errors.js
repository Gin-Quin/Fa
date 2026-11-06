// Fa errors (values brief section 15, D-24, D-92; types brief D50).
//
// Errors are ordinary control flow in Fa, so they are JavaScript exceptions
// that don't capture a stack: every member of an `Errors` type compiles to a
// class extending `FaError` (not `Error`). `throw` is `throw new Member(...)`,
// `try` is plain JavaScript propagation, and `catch` keeps only `FaError`
// instances:
//
//     try { $t = <left> } catch ($e) { const error = faCaught($e); $t = <handler> }
//
// so stops (`FaStop`) and foreign JavaScript exceptions go through.

import { faFormatError } from "./format.js";

// @fa-helper FaError deps: faFormatError
/**
 * The base class of every Fa error. A generated member class sets the static
 * `faName` to its qualified name (`ApiError.Timeout`) and assigns its fields
 * in declaration order in its constructor; `toString` prints them in Fa
 * notation. A member whose fields need a typed formatter (a `Float32`, an
 * enumeration...) overrides `toString`.
 */
export class FaError {
	/** @type {string | undefined} */
	static faName;
	/** @type {string | undefined} */
	static faId;
	/** @type {Map<string, Function>} */
	static types = new Map();

	/** @param {string} id @returns {string} */
	static register(id) {
		FaError.types.set(id, this);
		return id;
	}

	/**
	 * What recognizes a Fa error (`faIsError`): its qualified name.
	 * @returns {string}
	 */
	get [Symbol.for("fa.error")]() {
		return /** @type {any} */ (this.constructor).faName ?? this.constructor.name;
	}

	/** @returns {string} */
	get [Symbol.for("fa.error.type")]() {
		const type = /** @type {any} */ (this.constructor);
		return type.faId ?? type.faName ?? type.name;
	}

	/** @returns {string} */
	toString() {
		return faFormatError(this);
	}
}

// @fa-helper faCaught deps: FaError
/**
 * The first statement of a compiled `catch`: returns the Fa error, rethrows
 * anything else (stops, JavaScript exceptions of foreign code).
 * @param {unknown} exception
 * @returns {FaError}
 */
export function faCaught(exception) {
	if (exception instanceof FaError) {
		return exception;
	}
	throw exception;
}

// @fa-helper IndexError deps: FaError
/** An index or a slice bound outside the collection (or string). */
export class IndexError extends FaError {
	static faName = this.register("IndexError");

	/**
	 * @param {number} index the index that was out of bounds
	 * @param {number} length the length of the collection
	 */
	constructor(index, length) {
		super();
		/** @type {number} */
		this.index = index;
		/** @type {number} */
		this.length = length;
	}
}

// @fa-helper KeyNotFound deps: FaError
/** A map read with a key that isn't present. */
export class KeyNotFound extends FaError {
	static faName = this.register("KeyNotFound");

	/** @param {string} key the `toString()` of the key */
	constructor(key) {
		super();
		/** @type {string} */
		this.key = key;
	}
}

// @fa-helper DivisionByZero deps: FaError
/** An integer `//` or `%%` by zero. */
export class DivisionByZero extends FaError {
	static faName = this.register("DivisionByZero");
}

// @fa-helper ArithmeticError deps: FaError
/**
 * A conversion, a `checked*` method, a negative exponent or a shift amount
 * that doesn't fit.
 */
export class ArithmeticError extends FaError {
	static faName = this.register("ArithmeticError");

	/** @param {string} message */
	constructor(message) {
		super();
		/** @type {string} */
		this.message = message;
	}
}

// @fa-helper ParseError deps: FaError
/** Text that doesn't parse, like `Integer.parse("4x")`. */
export class ParseError extends FaError {
	static faName = this.register("ParseError");

	/**
	 * @param {string} input the text that was parsed
	 * @param {string} message what is wrong with it
	 */
	constructor(input, message) {
		super();
		/** @type {string} */
		this.input = input;
		/** @type {string} */
		this.message = message;
	}
}
