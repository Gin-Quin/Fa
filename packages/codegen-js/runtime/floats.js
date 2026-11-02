// Floats and percentages (values brief sections 6.10, 6.12, 6.14, D-5, D-6).
//
// A `Float` (`Float64`) is a JavaScript number and prints with `String(x)`
// (ECMAScript `Number::toString`, `-0` prints `0`); `==` is `===` (NaN is
// never equal, `-0 == 0`). A `Float32` is a number rounded with `Math.fround`
// after every operation. A `Percentage` is a number holding the fraction.

import { ParseError } from "./errors.js";

// @fa-helper faRound
/**
 * `x.round()`: halfway cases away from zero (`2.5` → `3`, `-2.5` → `-3`),
 * unlike `Math.round`.
 * @param {number} value
 * @returns {number}
 */
export function faRound(value) {
	return value < 0 ? -Math.round(-value) : Math.round(value);
}

// @fa-helper faParseFloat deps: ParseError
/**
 * `Float.parse(text)`: an optional sign, digits with an optional fraction
 * and exponent (`1`, `-2.5`, `.5`, `1e-3`), or `NaN`, `Infinity`,
 * `-Infinity`; nothing else (no spaces, no `_`, no hexadecimal), unlike
 * `Number(text)` which reads `""` as `0`.
 * @param {string} text
 * @returns {number}
 */
export function faParseFloat(text) {
	if (
		/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text) ||
		/^[+-]?Infinity$/.test(text) ||
		text === "NaN"
	) {
		return Number(text);
	}
	throw new ParseError(text, `"${text}" is not a number`);
}

// @fa-helper faFloatFloorDivide
/**
 * `a // b` on floats: `floor(a / b)` (IEEE: an infinity or NaN for a zero
 * divisor).
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloatFloorDivide(a, b) {
	return Math.floor(a / b);
}

// @fa-helper faFloatFloorModulo
/**
 * `a %% b` on floats: `a - floor(a / b) * b`, with the sign of the divisor
 * (NaN for a zero divisor).
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function faFloatFloorModulo(a, b) {
	return a - Math.floor(a / b) * b;
}

// @fa-helper faFloat32ToString
/**
 * Prints a `Float32`: the shortest decimal that reads back as the same
 * `Float32` (`0.1`, not `0.10000000149011612`), laid out like
 * `Number::toString` (D-5).
 * @param {number} value a value of `Math.fround`
 * @returns {string}
 */
export function faFloat32ToString(value) {
	if (value === 0 || !Number.isFinite(value)) {
		return String(value);
	}
	// For each number of significant digits, the nearest decimal, then its
	// two neighbors (the rounding interval of a Float32 isn't always
	// symmetric around it).
	for (let digits = 1; digits <= 9; digits += 1) {
		const [mantissa, exponent] = value.toExponential(digits - 1).split("e");
		const scaled = Number(mantissa.replace(".", ""));
		const power = Number(exponent) - (digits - 1);
		/** @type {number | undefined} */
		let best;
		for (const candidate of [scaled, scaled - 1, scaled + 1]) {
			const decimal = Number(`${candidate}e${power}`);
			if (
				Math.fround(decimal) === value &&
				(best === undefined || Math.abs(decimal - value) < Math.abs(best - value))
			) {
				best = decimal;
			}
			if (best !== undefined && candidate === scaled) {
				break;
			}
		}
		if (best !== undefined) {
			return String(best);
		}
	}
	return String(value);
}

// @fa-helper faPercentageToString
/**
 * Prints a `Percentage`: the fraction times 100, rounded to 15 significant
 * digits, then `%` (`10%`, not `10.000000000000002%`, D-6).
 * @param {number} fraction
 * @returns {string}
 */
export function faPercentageToString(fraction) {
	const value = fraction * 100;
	if (value === 0 || !Number.isFinite(value)) {
		return `${String(value)}%`;
	}
	return `${String(Number(value.toPrecision(15)))}%`;
}
