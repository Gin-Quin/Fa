// `console.log` and `console.error` (advanced brief 22.2, D39): the values
// in Fa notation, separated by a space, then a newline. The backend passes
// strings already formatted by a typed formatter where the JavaScript
// representation isn't enough (a `Float32`, a `Percentage`...): a string at
// the top level prints raw, so a formatted value prints unchanged.

import { faFormat } from "./format.js";

// @fa-helper faLog deps: faFormat
/**
 * `console.log(values...)` on stdout.
 * @param {...unknown} values
 * @returns {void}
 */
export function faLog(...values) {
	console.log(values.map(faFormat).join(" "));
}

// @fa-helper faLogError deps: faFormat
/**
 * `console.error(values...)` on stderr.
 * @param {...unknown} values
 * @returns {void}
 */
export function faLogError(...values) {
	console.error(values.map(faFormat).join(" "));
}
