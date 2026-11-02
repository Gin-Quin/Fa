// Integers (values brief section 6, D-4, D-9, D-49, D-50, D-51, D-74, D-76).
//
// Representations:
// - `Integer`, the sized types up to 32 bits and the ranged integers are
//   JavaScript numbers. An `Integer` node that may exceed ±(2^53 − 1) is
//   checked with `faCheckInteger`; a sink of a sized or ranged type with
//   `faCheckRange`.
// - `Int64`, `UInt64`, `Int128`, `UInt128` and `BigInt` are JavaScript bigints;
//   the sized ones are checked with `faCheckWidth`.
//
// The helpers that take a type take its storage (`bits`, `signed`) for the
// wrapping and bitwise operations, and its range (`min`, `max`, numbers or
// bigints) for the saturating and checked ones. `Integer` itself is
// `bits = 64` with a number: it computes on 64 bits through `BigInt`, then
// goes back to a number with the safe-integer check, which may stop (D-51).

import { ArithmeticError, DivisionByZero, ParseError } from "./errors.js";
import { faRound } from "./floats.js";
import { faOverflow, faStop } from "./stops.js";

// @fa-helper faCheckInteger deps: faOverflow
/**
 * Checks an `Integer` result against the JavaScript range, ±(2^53 − 1):
 * stops when it doesn't fit, returns it otherwise.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faCheckInteger(value, location) {
	if (value > 9007199254740991 || value < -9007199254740991) {
		faOverflow(value, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return value;
}

// @fa-helper faCheckRange deps: faOverflow
/**
 * Checks a value at a sink of a sized or ranged integer type: stops when it
 * is outside `min..=max`, returns it otherwise.
 * @template {number | bigint} T
 * @param {T} value
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type the name of the type, like `Int8` or `Integer(min = 0)`
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckRange(value, min, max, type, location) {
	if (value < min || value > max) {
		faOverflow(value, type, location);
	}
	return value;
}

// @fa-helper faCheckWidth deps: faOverflow
/**
 * Checks a bigint result of an `Int64`, `UInt64`, `Int128` or `UInt128`
 * operation against its width: stops when it doesn't fit.
 * @param {bigint} value
 * @param {number} bits 64 or 128
 * @param {boolean} signed
 * @param {string} type
 * @param {string} [location]
 * @returns {bigint}
 */
export function faCheckWidth(value, bits, signed, type, location) {
	if ((signed ? BigInt.asIntN(bits, value) : BigInt.asUintN(bits, value)) !== value) {
		faOverflow(value, type, location);
	}
	return value;
}

// @fa-helper faFloorQuotient
/**
 * `a // b` on numbers with a divisor proven non-zero: rounds toward
 * negative infinity (D-49); `+ 0` turns `-0` into `0`.
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloorQuotient(a, b) {
	const r = a % b;
	const q = (a - r) / b;
	return r !== 0 && r < 0 !== b < 0 ? q - 1 : q + 0;
}

// @fa-helper faFloorDivide deps: DivisionByZero faFloorQuotient
/**
 * `a // b` on numbers: throws `DivisionByZero` when `b` is 0.
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloorDivide(a, b) {
	if (b === 0) {
		throw new DivisionByZero();
	}
	return faFloorQuotient(a, b);
}

// @fa-helper faFloorRemainder
/**
 * `a %% b` on numbers with a divisor proven non-zero: the remainder has the
 * sign of the divisor (D-49).
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloorRemainder(a, b) {
	const r = a % b;
	return r !== 0 && r < 0 !== b < 0 ? r + b : r + 0;
}

// @fa-helper faFloorModulo deps: DivisionByZero faFloorRemainder
/**
 * `a %% b` on numbers: throws `DivisionByZero` when `b` is 0.
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloorModulo(a, b) {
	if (b === 0) {
		throw new DivisionByZero();
	}
	return faFloorRemainder(a, b);
}

// @fa-helper faBigFloorQuotient
/**
 * `a // b` on bigints with a divisor proven non-zero.
 * @param {bigint} a
 * @param {bigint} b
 * @returns {bigint}
 */
export function faBigFloorQuotient(a, b) {
	const q = a / b;
	const r = a % b;
	return r !== 0n && r < 0n !== b < 0n ? q - 1n : q;
}

// @fa-helper faBigFloorDivide deps: DivisionByZero faBigFloorQuotient
/**
 * `a // b` on bigints: throws `DivisionByZero` when `b` is 0.
 * @param {bigint} a
 * @param {bigint} b
 * @returns {bigint}
 */
export function faBigFloorDivide(a, b) {
	if (b === 0n) {
		throw new DivisionByZero();
	}
	return faBigFloorQuotient(a, b);
}

// @fa-helper faBigFloorRemainder
/**
 * `a %% b` on bigints with a divisor proven non-zero.
 * @param {bigint} a
 * @param {bigint} b
 * @returns {bigint}
 */
export function faBigFloorRemainder(a, b) {
	const r = a % b;
	return r !== 0n && r < 0n !== b < 0n ? r + b : r;
}

// @fa-helper faBigFloorModulo deps: DivisionByZero faBigFloorRemainder
/**
 * `a %% b` on bigints: throws `DivisionByZero` when `b` is 0.
 * @param {bigint} a
 * @param {bigint} b
 * @returns {bigint}
 */
export function faBigFloorModulo(a, b) {
	if (b === 0n) {
		throw new DivisionByZero();
	}
	return faBigFloorRemainder(a, b);
}

// @fa-helper faPower deps: ArithmeticError
/**
 * `a ** b` on number integers. Exact while the result is a safe integer;
 * beyond, it returns the approximate `a ** b`, which the overflow check that
 * follows rejects. A negative exponent throws `ArithmeticError`.
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faPower(a, b) {
	if (b < 0) {
		throw new ArithmeticError(`negative exponent: ${b}`);
	}
	let result = 1;
	let base = a;
	let exponent = b;
	for (;;) {
		if (exponent % 2 === 1) {
			result *= base;
			if (result > 9007199254740991 || result < -9007199254740991) {
				return a ** b;
			}
		}
		exponent = Math.floor(exponent / 2);
		if (exponent === 0) {
			return result + 0;
		}
		base *= base;
		if (base > 9007199254740991) {
			return a ** b;
		}
	}
}

// @fa-helper faBigPower deps: ArithmeticError
/**
 * `a ** b` on bigints. A negative exponent throws `ArithmeticError`.
 * @param {bigint} a
 * @param {bigint} b
 * @returns {bigint}
 */
export function faBigPower(a, b) {
	if (b < 0n) {
		throw new ArithmeticError(`negative exponent: ${b}`);
	}
	return a ** b;
}

// @fa-helper faConvertInteger deps: ArithmeticError
/**
 * An unproven integer conversion (`Int8(x)`, `Integer(max = 100)(x)`):
 * throws `ArithmeticError` when the value is outside `min..=max`. The value
 * keeps its representation (`Number(...)` or `BigInt(...)` is added around
 * the call when it changes).
 * @template {number | bigint} T
 * @param {T} value
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @returns {T}
 */
export function faConvertInteger(value, min, max, type) {
	if (value < min || value > max) {
		throw new ArithmeticError(`${value} doesn't fit in ${type}`);
	}
	return value;
}

// @fa-helper faBigToInteger deps: ArithmeticError faOverflow
/**
 * `Integer(b)` with `b: BigInt`: throws `ArithmeticError` outside the 64-bit
 * range of `Integer`; stops beyond ±(2^53 − 1), the JavaScript range.
 * @param {bigint} value
 * @param {string} [location]
 * @returns {number}
 */
export function faBigToInteger(value, location) {
	if (value < -9223372036854775808n || value > 9223372036854775807n) {
		throw new ArithmeticError(`${value} doesn't fit in Integer`);
	}
	if (value < -9007199254740991n || value > 9007199254740991n) {
		faOverflow(value, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return Number(value);
}

// @fa-helper faInt64ToInteger deps: faOverflow
/**
 * An `Int64` (a bigint) where an `Integer` (a number) is expected: always
 * fits natively, stops beyond ±(2^53 − 1) on JavaScript (types brief D2).
 * @param {bigint} value
 * @param {string} [location]
 * @returns {number}
 */
export function faInt64ToInteger(value, location) {
	if (value < -9007199254740991n || value > 9007199254740991n) {
		faOverflow(value, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return Number(value);
}

// @fa-helper faFloatToIntegerResult deps: ArithmeticError faOverflow
/**
 * The end of `Integer.round`, `floor`, `ceil` and `truncate`: throws
 * `ArithmeticError` for NaN, an infinity or a value outside the 64-bit range
 * of `Integer`; stops beyond ±(2^53 − 1), the JavaScript range.
 * @param {number} rounded the rounded float
 * @param {number} value the float before rounding, for the message
 * @param {string} [location]
 * @returns {number}
 */
export function faFloatToIntegerResult(rounded, value, location) {
	if (Number.isNaN(rounded) || !Number.isFinite(rounded)) {
		throw new ArithmeticError(`${value} is not a finite number`);
	}
	if (rounded >= 9223372036854775808 || rounded < -9223372036854775808) {
		throw new ArithmeticError(`${value} doesn't fit in Integer`);
	}
	if (rounded > 9007199254740991 || rounded < -9007199254740991) {
		faOverflow(rounded, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return rounded + 0;
}

// @fa-helper faIntegerRound deps: faFloatToIntegerResult faRound
/**
 * `Integer.round(x)`: halves away from zero.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faIntegerRound(value, location) {
	return faFloatToIntegerResult(faRound(value), value, location);
}

// @fa-helper faIntegerFloor deps: faFloatToIntegerResult
/**
 * `Integer.floor(x)`.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faIntegerFloor(value, location) {
	return faFloatToIntegerResult(Math.floor(value), value, location);
}

// @fa-helper faIntegerCeil deps: faFloatToIntegerResult
/**
 * `Integer.ceil(x)`.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faIntegerCeil(value, location) {
	return faFloatToIntegerResult(Math.ceil(value), value, location);
}

// @fa-helper faIntegerTruncate deps: faFloatToIntegerResult
/**
 * `Integer.truncate(x)`.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faIntegerTruncate(value, location) {
	return faFloatToIntegerResult(Math.trunc(value), value, location);
}

// @fa-helper faFloatToInteger deps: faOverflow
/**
 * `Integer(x)` on a float (design/decisions.md, "Float to integer
 * conversion"): truncates; NaN or a value out of range stops the program,
 * like an overflow. The analyzer removes the call when the range of `x` is
 * known.
 * @param {number} value
 * @param {string} [location]
 * @returns {number}
 */
export function faFloatToInteger(value, location) {
	const truncated = Math.trunc(value);
	if (!(truncated <= 9007199254740991 && truncated >= -9007199254740991)) {
		faOverflow(value, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return truncated + 0;
}

// @fa-helper faParseInteger deps: ParseError faParseBigInteger
/**
 * `Integer.parse(text)` (and the parse of the other number-represented
 * integer types): an optional sign then ASCII decimal digits, nothing else
 * (no spaces, no `_`, no prefix). Throws `ParseError` otherwise, or when the
 * value doesn't fit in `min..=max` (by default the JavaScript range of
 * `Integer`, ±(2^53 − 1)).
 * @param {string} text
 * @param {number} [min]
 * @param {number} [max]
 * @param {string} [type]
 * @returns {number}
 */
export function faParseInteger(
	text,
	min = -9007199254740991,
	max = 9007199254740991,
	type = "Integer",
) {
	const value = faParseBigInteger(text);
	if (value < min || value > max) {
		const javascript =
			type === "Integer" && value >= -9223372036854775808n && value <= 9223372036854775807n;
		throw new ParseError(
			text,
			javascript
				? `${text} doesn't fit in Integer on JavaScript (±(2^53 − 1))`
				: `${text} doesn't fit in ${type}`,
		);
	}
	return Number(value);
}

// @fa-helper faParseBigInteger deps: ParseError
/**
 * Parses an integer into a bigint: `BigInt.parse`, and the parse of `Int64`,
 * `UInt64` and the 128-bit types when `min` and `max` are given.
 * @param {string} text
 * @param {bigint} [min]
 * @param {bigint} [max]
 * @param {string} [type]
 * @returns {bigint}
 */
export function faParseBigInteger(text, min, max, type = "BigInt") {
	if (!/^[+-]?[0-9]+$/.test(text)) {
		throw new ParseError(
			text,
			text === "" ? "the text is empty" : `${JSON.stringify(text)} is not an integer`,
		);
	}
	const value = BigInt(text);
	if ((min !== undefined && value < min) || (max !== undefined && value > max)) {
		throw new ParseError(text, `${text} doesn't fit in ${type}`);
	}
	return value;
}

// @fa-helper faWrap
/**
 * Wraps an exact integer (at most 2^53 in magnitude) to a storage of up to
 * 32 bits.
 * @param {number} value
 * @param {number} bits 8, 16 or 32
 * @param {boolean} signed
 * @returns {number}
 */
export function faWrap(value, bits, signed) {
	if (bits === 32) {
		return signed ? value | 0 : value >>> 0;
	}
	const shift = 32 - bits;
	return signed ? (value << shift) >> shift : value & ((1 << bits) - 1);
}

// @fa-helper faWrapBig
/**
 * Wraps a bigint to a storage of `bits` bits.
 * @param {bigint} value
 * @param {number} bits
 * @param {boolean} signed
 * @returns {bigint}
 */
export function faWrapBig(value, bits, signed) {
	return signed ? BigInt.asIntN(bits, value) : BigInt.asUintN(bits, value);
}

// @fa-helper faFromBig64 deps: faOverflow
/**
 * Back from a 64-bit bigint computation to an `Integer` number, with the
 * safe-integer check (D-51).
 * @param {bigint} value
 * @param {string} [location]
 * @returns {number}
 */
export function faFromBig64(value, location) {
	if (value < -9007199254740991n || value > 9007199254740991n) {
		faOverflow(value, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return Number(value);
}

// @fa-helper faWrappingResult deps: ArithmeticError faWrap faWrapBig faFromBig64
/**
 * The wrapping operations: `op` is `+`, `-`, `*` or `**`.
 * @template {number | bigint} T
 * @param {string} op
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faWrappingResult(op, a, b, bits, signed, location) {
	if (typeof a === "bigint" && typeof b === "bigint") {
		if (op === "**") {
			if (b < 0n) {
				throw new ArithmeticError(`negative exponent: ${b}`);
			}
			let result = 1n;
			let base = faWrapBig(a, bits, signed);
			for (let exponent = b; exponent > 0n; exponent >>= 1n) {
				if (exponent & 1n) {
					result = faWrapBig(result * base, bits, signed);
				}
				base = faWrapBig(base * base, bits, signed);
			}
			return /** @type {T} */ (result);
		}
		const exact = op === "+" ? a + b : op === "-" ? a - b : a * b;
		return /** @type {T} */ (faWrapBig(exact, bits, signed));
	}
	const x = /** @type {number} */ (a);
	const y = /** @type {number} */ (b);
	if (bits > 32) {
		const wrapped = faWrappingResult(op, BigInt(x), BigInt(y), 64, true);
		return /** @type {T} */ (faFromBig64(wrapped, location));
	}
	if (op === "**") {
		if (y < 0) {
			throw new ArithmeticError(`negative exponent: ${y}`);
		}
		let result = 1;
		let base = x;
		for (let exponent = y; exponent > 0; exponent = Math.floor(exponent / 2)) {
			if (exponent % 2 === 1) {
				result = faWrap(bits === 32 ? Math.imul(result, base) : result * base, bits, signed);
			}
			base = faWrap(bits === 32 ? Math.imul(base, base) : base * base, bits, signed);
		}
		return /** @type {T} */ (faWrap(result, bits, signed));
	}
	const exact = op === "+" ? x + y : op === "-" ? x - y : bits === 32 ? Math.imul(x, y) : x * y;
	return /** @type {T} */ (faWrap(exact, bits, signed));
}

// @fa-helper faWrappingAdd deps: faWrappingResult
/**
 * `a.wrappingAdd(b)`: wraps modulo 2^bits of the storage type.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faWrappingAdd(a, b, bits, signed, location) {
	return faWrappingResult("+", a, b, bits, signed, location);
}

// @fa-helper faWrappingSub deps: faWrappingResult
/**
 * `a.wrappingSub(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faWrappingSub(a, b, bits, signed, location) {
	return faWrappingResult("-", a, b, bits, signed, location);
}

// @fa-helper faWrappingMul deps: faWrappingResult
/**
 * `a.wrappingMul(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faWrappingMul(a, b, bits, signed, location) {
	return faWrappingResult("*", a, b, bits, signed, location);
}

// @fa-helper faWrappingPow deps: faWrappingResult
/**
 * `a.wrappingPow(b)`; a negative exponent throws `ArithmeticError`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faWrappingPow(a, b, bits, signed, location) {
	return faWrappingResult("**", a, b, bits, signed, location);
}

// @fa-helper faExactResult deps: ArithmeticError
/**
 * The exact value of `a op b` (`op` is `+`, `-`, `*` or `**`): a number when
 * it is a safe integer, a bigint otherwise. A power whose magnitude is beyond
 * 2^256 gives ±2^256, which is outside every sized type.
 * @param {string} op
 * @param {number | bigint} a
 * @param {number | bigint} b
 * @returns {number | bigint}
 */
export function faExactResult(op, a, b) {
	if (op === "**" && b < 0) {
		throw new ArithmeticError(`negative exponent: ${b}`);
	}
	if (typeof a === "number" && typeof b === "number") {
		const result = op === "+" ? a + b : op === "-" ? a - b : op === "*" ? a * b : a ** b;
		if (Number.isSafeInteger(result)) {
			if (op !== "**") {
				return result + 0;
			}
			// `**` is not always correctly rounded: recompute exactly.
			const exact = faExactResult(op, BigInt(a), BigInt(b));
			return Number(exact);
		}
	}
	const x = BigInt(a);
	const y = BigInt(b);
	if (op === "+") {
		return x + y;
	}
	if (op === "-") {
		return x - y;
	}
	if (op === "*") {
		return x * y;
	}
	if (x === 0n) {
		return y === 0n ? 1n : 0n;
	}
	if (x === 1n) {
		return 1n;
	}
	if (x === -1n) {
		return y % 2n === 0n ? 1n : -1n;
	}
	if (y > 256n) {
		return x < 0n && y % 2n === 1n ? -(2n ** 256n) : 2n ** 256n;
	}
	return x ** y;
}

// @fa-helper faSaturatingResult deps: faExactResult faOverflow
/**
 * The saturating operations: the exact result clamped to `min..=max` (the
 * receiver's type), in the representation of `a`. On an `Integer`, a result
 * beyond ±(2^53 − 1) stops (D-51).
 * @template {number | bigint} T
 * @param {string} op
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} [location]
 * @returns {T}
 */
export function faSaturatingResult(op, a, b, min, max, location) {
	const exact = faExactResult(op, a, b);
	const clamped = exact < min ? min : exact > max ? max : exact;
	if (typeof a === "bigint") {
		return /** @type {T} */ (BigInt(clamped));
	}
	if (clamped < -9007199254740991 || clamped > 9007199254740991) {
		faOverflow(clamped, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return /** @type {T} */ (Number(clamped));
}

// @fa-helper faSaturatingAdd deps: faSaturatingResult
/**
 * `a.saturatingAdd(b)`: clamps to the bounds of the receiver's type.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} [location]
 * @returns {T}
 */
export function faSaturatingAdd(a, b, min, max, location) {
	return faSaturatingResult("+", a, b, min, max, location);
}

// @fa-helper faSaturatingSub deps: faSaturatingResult
/**
 * `a.saturatingSub(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} [location]
 * @returns {T}
 */
export function faSaturatingSub(a, b, min, max, location) {
	return faSaturatingResult("-", a, b, min, max, location);
}

// @fa-helper faSaturatingMul deps: faSaturatingResult
/**
 * `a.saturatingMul(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} [location]
 * @returns {T}
 */
export function faSaturatingMul(a, b, min, max, location) {
	return faSaturatingResult("*", a, b, min, max, location);
}

// @fa-helper faSaturatingPow deps: faSaturatingResult
/**
 * `a.saturatingPow(b)`; a negative exponent throws `ArithmeticError`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} [location]
 * @returns {T}
 */
export function faSaturatingPow(a, b, min, max, location) {
	return faSaturatingResult("**", a, b, min, max, location);
}

// @fa-helper faCheckedResult deps: faExactResult ArithmeticError faOverflow
/**
 * The checked operations: throws `ArithmeticError` when the exact result is
 * outside `min..=max` (the receiver's type). On an `Integer`, a result that
 * fits natively but not in ±(2^53 − 1) stops (D-51).
 * @template {number | bigint} T
 * @param {string} op
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckedResult(op, a, b, min, max, type, location) {
	const exact = faExactResult(op, a, b);
	if (exact < min || exact > max) {
		throw new ArithmeticError(`integer overflow: ${exact} doesn't fit in ${type}`);
	}
	if (typeof a === "bigint") {
		return /** @type {T} */ (BigInt(exact));
	}
	if (exact < -9007199254740991 || exact > 9007199254740991) {
		faOverflow(exact, "Integer (±(2^53 − 1) on JavaScript)", location);
	}
	return /** @type {T} */ (Number(exact));
}

// @fa-helper faCheckedAdd deps: faCheckedResult
/**
 * `a.checkedAdd(b)`: throws `ArithmeticError` when the result doesn't fit.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckedAdd(a, b, min, max, type, location) {
	return faCheckedResult("+", a, b, min, max, type, location);
}

// @fa-helper faCheckedSub deps: faCheckedResult
/**
 * `a.checkedSub(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckedSub(a, b, min, max, type, location) {
	return faCheckedResult("-", a, b, min, max, type, location);
}

// @fa-helper faCheckedMul deps: faCheckedResult
/**
 * `a.checkedMul(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckedMul(a, b, min, max, type, location) {
	return faCheckedResult("*", a, b, min, max, type, location);
}

// @fa-helper faCheckedPow deps: faCheckedResult
/**
 * `a.checkedPow(b)`; a negative exponent throws `ArithmeticError` too.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number | bigint} min
 * @param {number | bigint} max
 * @param {string} type
 * @param {string} [location]
 * @returns {T}
 */
export function faCheckedPow(a, b, min, max, type, location) {
	return faCheckedResult("**", a, b, min, max, type, location);
}

// @fa-helper faShiftAmount deps: ArithmeticError
/**
 * Checks a shift amount: it must be in `0..bits` (D-50).
 * @param {number | bigint} amount
 * @param {number} bits
 * @returns {number}
 */
export function faShiftAmount(amount, bits) {
	if (amount < 0 || amount >= bits) {
		throw new ArithmeticError(`shift amount ${amount} is outside 0..${bits}`);
	}
	return Number(amount);
}

// @fa-helper faBitwiseResult deps: faShiftAmount faWrap faWrapBig faFromBig64
/**
 * The bitwise methods: `op` is `and`, `or`, `xor`, `not`, `shiftLeft` or
 * `shiftRight`. The result has the storage type of the receiver (D-50).
 * Shifts are logical on unsigned types and arithmetic on signed ones;
 * `shiftLeft` drops the bits shifted out.
 * @template {number | bigint} T
 * @param {string} op
 * @param {T} a
 * @param {T | number} b the other operand, or the shift amount
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faBitwiseResult(op, a, b, bits, signed, location) {
	if (typeof a === "bigint") {
		const y = typeof b === "bigint" ? b : 0n;
		switch (op) {
			case "and":
				return /** @type {T} */ (a & y);
			case "or":
				return /** @type {T} */ (a | y);
			case "xor":
				return /** @type {T} */ (a ^ y);
			case "not":
				return /** @type {T} */ (faWrapBig(~a, bits, signed));
			case "shiftLeft":
				return /** @type {T} */ (faWrapBig(a << BigInt(faShiftAmount(b, bits)), bits, signed));
			default:
				return /** @type {T} */ (a >> BigInt(faShiftAmount(b, bits)));
		}
	}
	const x = /** @type {number} */ (a);
	if (bits > 32) {
		const other = op === "shiftLeft" || op === "shiftRight" || op === "not" ? b : BigInt(b);
		const result = faBitwiseResult(op, BigInt(x), other, 64, true);
		return /** @type {T} */ (faFromBig64(result, location));
	}
	const y = /** @type {number} */ (b);
	switch (op) {
		case "and":
			return /** @type {T} */ (faWrap(x & y, bits, signed));
		case "or":
			return /** @type {T} */ (faWrap(x | y, bits, signed));
		case "xor":
			return /** @type {T} */ (faWrap(x ^ y, bits, signed));
		case "not":
			return /** @type {T} */ (faWrap(~x, bits, signed));
		case "shiftLeft":
			return /** @type {T} */ (faWrap(x << faShiftAmount(y, bits), bits, signed));
		default: {
			const amount = faShiftAmount(y, bits);
			return /** @type {T} */ (signed ? x >> amount : x >>> amount);
		}
	}
}

// @fa-helper faBitAnd deps: faBitwiseResult
/**
 * `a.bitAnd(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faBitAnd(a, b, bits, signed, location) {
	return faBitwiseResult("and", a, b, bits, signed, location);
}

// @fa-helper faBitOr deps: faBitwiseResult
/**
 * `a.bitOr(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faBitOr(a, b, bits, signed, location) {
	return faBitwiseResult("or", a, b, bits, signed, location);
}

// @fa-helper faBitXor deps: faBitwiseResult
/**
 * `a.bitXor(b)`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {T} b
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faBitXor(a, b, bits, signed, location) {
	return faBitwiseResult("xor", a, b, bits, signed, location);
}

// @fa-helper faBitNot deps: faBitwiseResult
/**
 * `a.bitNot()`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faBitNot(a, bits, signed, location) {
	return faBitwiseResult("not", a, 0, bits, signed, location);
}

// @fa-helper faShiftLeft deps: faBitwiseResult
/**
 * `a.shiftLeft(amount)`: throws `ArithmeticError` when the amount is outside
 * `0..bits`.
 * @template {number | bigint} T
 * @param {T} a
 * @param {number | bigint} amount
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faShiftLeft(a, amount, bits, signed, location) {
	return faBitwiseResult("shiftLeft", a, amount, bits, signed, location);
}

// @fa-helper faShiftRight deps: faBitwiseResult
/**
 * `a.shiftRight(amount)`: arithmetic on signed types, logical on unsigned
 * ones.
 * @template {number | bigint} T
 * @param {T} a
 * @param {number | bigint} amount
 * @param {number} bits
 * @param {boolean} signed
 * @param {string} [location]
 * @returns {T}
 */
export function faShiftRight(a, amount, bits, signed, location) {
	return faBitwiseResult("shiftRight", a, amount, bits, signed, location);
}

// @fa-helper faClamp
/**
 * `x.clamp(min, max)` on integers (numbers or bigints) and floats; NaN stays
 * NaN.
 * @template {number | bigint} T
 * @param {T} value
 * @param {T} min
 * @param {T} max
 * @returns {T}
 */
export function faClamp(value, min, max) {
	return value < min ? min : value > max ? max : value;
}

// @fa-helper faRangeOverflow deps: faStop
/**
 * Stops a `for i in a..` loop that would go past the maximum of `Integer`
 * (D-40); on JavaScript, ±(2^53 − 1).
 * @param {string} [location]
 * @returns {never}
 */
export function faRangeOverflow(location) {
	return faStop(
		"integer overflow: the loop goes past the maximum of Integer (2^53 − 1 on JavaScript)",
		location,
	);
}
