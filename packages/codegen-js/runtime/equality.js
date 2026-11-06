// Deep structural equality, hashing and natural order (values brief section
// 13, D-65; types brief D17, D35, D63; collections `==`).
//
// The backend generates an `eq$T`, a `hash$T` and a comparator per static
// type where it can; these generic versions serve erased generic code, the
// default `FaHashMap` and `FaSet`, and the types whose JavaScript
// representation is enough to decide (`==` doesn't depend on Fa types that
// share a representation).

import { FaHashMap, FaSet } from "./collections.js";
import { faSides } from "./relations.js";
import { faStringCompare } from "./strings.js";

// @fa-helper faDeepEquals deps: FaHashMap FaSet faSides
/**
 * Deep structural equality of two Fa values. Numbers compare with `===`
 * (NaN is never equal, `-0` equals `0`), or with SameValueZero when
 * `sameValueZero` (map keys and set elements: NaN finds NaN). A bigint equals
 * a number of the same value (D-69). Arrays (and tuples) compare item by
 * item; maps by keys and values; sets by elements (with the right-hand set's
 * key function); objects by their own properties, which must be the same set,
 * and by prototype (so a `FaRange` or an error class only equals its own
 * kind); a field set to `none` equals an absent field. Iterative: long
 * recursive chains don't overflow the stack.
 * @param {unknown} a
 * @param {unknown} b
 * @param {boolean} sameValueZero
 * @returns {boolean}
 */
export function faDeepEquals(a, b, sameValueZero) {
	/** @type {unknown[]} */
	const pending = [a, b];
	while (pending.length > 0) {
		const y = /** @type {any} */ (pending.pop());
		const x = /** @type {any} */ (pending.pop());
		// a composite equals itself only when it holds no NaN: no identity
		// shortcut, except for SameValueZero (where NaN finds NaN)
		if (x === y && (sameValueZero || typeof x !== "object" || x === null)) {
			continue;
		}
		const typeX = typeof x;
		const typeY = typeof y;
		if (typeX === "number" && typeY === "number") {
			if (sameValueZero && x !== x && y !== y) {
				continue;
			}
			return false;
		}
		if (typeX === "bigint" || typeY === "bigint") {
			if ((typeX === "number" || typeY === "number") && x == y) {
				continue;
			}
			return false;
		}
		if (typeX !== "object" || typeY !== "object" || x === null || y === null) {
			return false;
		}
		if (Array.isArray(x)) {
			if (!Array.isArray(y) || x.length !== y.length) {
				return false;
			}
			for (let index = 0; index < x.length; index += 1) {
				pending.push(x[index], y[index]);
			}
			continue;
		}
		if (x instanceof Map || x instanceof FaHashMap) {
			if (!(y instanceof Map || y instanceof FaHashMap) || x.size !== y.size) {
				return false;
			}
			for (const [key, value] of x) {
				if (!y.has(key)) {
					return false;
				}
				pending.push(value, y.get(key));
			}
			continue;
		}
		// pools of values (`FaValuePool`): their values in order
		if (x[Symbol.toStringTag] === "fa:ValuePool") {
			if (y?.[Symbol.toStringTag] !== "fa:ValuePool") {
				return false;
			}
			const values = Array.from(y);
			if (x.size !== values.length) {
				return false;
			}
			let index = 0;
			for (const value of x) {
				pending.push(value, values[index]);
				index += 1;
			}
			continue;
		}
		if (x instanceof Set || x instanceof FaSet) {
			if (!(y instanceof Set || y instanceof FaSet) || x.size !== y.size) {
				return false;
			}
			// a pool or a columnar of objects (a `Set` of them; a Fa set of
			// objects is a `FaSet`): its elements in order, compared by value
			// (identity is never observable in Fa)
			const first = x.values().next().value;
			if (
				!(x instanceof FaSet) &&
				!(y instanceof FaSet) &&
				typeof first === "object" &&
				first !== null
			) {
				const values = Array.from(y);
				let index = 0;
				for (const value of x) {
					pending.push(value, values[index]);
					index += 1;
				}
				continue;
			}
			for (const value of x) {
				if (!y.has(value)) {
					return false;
				}
			}
			continue;
		}
		if (Object.getPrototypeOf(x) !== Object.getPrototypeOf(y)) {
			const errorType = x[Symbol.for("fa.error.type")];
			if (typeof errorType !== "string" || errorType !== y[Symbol.for("fa.error.type")]) {
				return false;
			}
		}
		// An absent optional field is `none`, like a field set to `undefined`.
		// Relation sides are not data (D21).
		const sides = x[faSides];
		for (const key of Object.keys(x)) {
			if (sides === undefined || !sides.includes(key)) {
				pending.push(x[key], y[key]);
			}
		}
		for (const key of Object.keys(y)) {
			if (y[key] !== undefined && !Object.hasOwn(x, key)) {
				return false;
			}
		}
	}
	return true;
}

// @fa-helper faEquals deps: faDeepEquals
/**
 * Fa `==` on any two values of the same type: deep, IEEE on floats (a
 * composite holding NaN doesn't equal itself, so identity isn't enough).
 * @param {unknown} a
 * @param {unknown} b
 * @returns {boolean}
 */
export function faEquals(a, b) {
	return (a === b && (typeof a !== "object" || a === null)) || faDeepEquals(a, b, false);
}

// @fa-helper faKeyEquals deps: faDeepEquals
/**
 * The equality of map keys and set elements: deep, SameValueZero on floats
 * (NaN finds NaN, `-0` and `0` are the same key).
 * @param {unknown} a
 * @param {unknown} b
 * @returns {boolean}
 */
export function faKeyEquals(a, b) {
	return a === b || faDeepEquals(a, b, true);
}

// @fa-helper faHashMix
/**
 * The final mix of a 32-bit hash (MurmurHash3's `fmix32`).
 * @param {number} hash
 * @returns {number} an unsigned 32-bit integer
 */
export function faHashMix(hash) {
	let h = hash | 0;
	h ^= h >>> 16;
	h = Math.imul(h, 0x85ebca6b);
	h ^= h >>> 13;
	h = Math.imul(h, 0xc2b2ae35);
	h ^= h >>> 16;
	return h >>> 0;
}

// @fa-helper faHashString
/**
 * FNV-1a over the UTF-16 code units of a string.
 * @param {string} text
 * @returns {number} an unsigned 32-bit integer
 */
export function faHashString(text) {
	let h = 0x811c9dc5;
	for (let index = 0; index < text.length; index += 1) {
		h ^= text.charCodeAt(index);
		h = Math.imul(h, 0x01000193);
	}
	return h >>> 0;
}

// @fa-helper faHashNumber deps: faHashMix faHashString
/**
 * The hash of a number: consistent with SameValueZero (`-0` and `0` hash
 * the same, every NaN hashes the same), and the same for a safe integer as
 * for the bigint of the same value.
 * @param {number} value
 * @returns {number}
 */
export function faHashNumber(value) {
	if (Number.isSafeInteger(value)) {
		const low = value >>> 0;
		const high = Math.floor(value / 4294967296) | 0;
		return faHashMix(low ^ Math.imul(high, 0x9e3779b1));
	}
	if (value !== value) {
		return 0x7ff80000;
	}
	return faHashMix(faHashString(String(value)));
}

// @fa-helper faHash deps: faHashMix faHashNumber faHashString FaHashMap FaSet faSides
/**
 * A 32-bit structural hash consistent with `faKeyEquals`: two equal keys
 * have the same hash. Arrays hash in order; objects, maps and sets don't
 * depend on the order of their properties or elements.
 * @param {unknown} value
 * @returns {number} an unsigned 32-bit integer
 */
export function faHash(value) {
	switch (typeof value) {
		case "number":
			return faHashNumber(value);
		case "bigint":
			return value >= -9007199254740991n && value <= 9007199254740991n
				? faHashNumber(Number(value))
				: faHashMix(faHashString(value.toString()) ^ 0x5bd1e995);
		case "string":
			return faHashString(value);
		case "boolean":
			return value ? 0x3c6ef372 : 0x1b873593;
		case "object": {
			if (value === null) {
				return 0x6a09e667;
			}
			if (Array.isArray(value)) {
				let h = 0x9e3779b9 ^ value.length;
				for (const item of value) {
					h = Math.imul(h ^ faHash(item), 0x01000193);
				}
				return faHashMix(h);
			}
			let sum = 0;
			if (value instanceof Map || value instanceof FaHashMap) {
				for (const [key, item] of value) {
					sum = (sum + faHashMix(Math.imul(faHash(key), 31) + faHash(item))) | 0;
				}
				return faHashMix(sum ^ 0x85ebca6b);
			}
			if (value instanceof FaSet) {
				for (const item of value) {
					sum = (sum + faHash(value.keyOf(item))) | 0;
				}
				return faHashMix(sum ^ 0xc2b2ae35);
			}
			if (value instanceof Set) {
				for (const item of value) {
					sum = (sum + faHash(item)) | 0;
				}
				return faHashMix(sum ^ 0xc2b2ae35);
			}
			const record = /** @type {Record<string, unknown>} */ (value);
			const sides = /** @type {any} */ (value)[faSides];
			for (const key of Object.keys(record)) {
				if (record[key] === undefined || (sides !== undefined && sides.includes(key))) {
					continue; // like an absent optional field; relation sides are not data
				}
				sum = (sum + faHashMix(Math.imul(faHashString(key), 31) + faHash(record[key]))) | 0;
			}
			return faHashMix(sum);
		}
		default:
			return 0x6a09e667;
	}
}

// @fa-helper faCompare deps: faStringCompare
/**
 * The natural order (`sort()` without a comparator, D35): numbers and
 * bigints by value (NaN counts as equal to everything), strings by code
 * point, arrays (tuples) lexicographically.
 * @param {unknown} a
 * @param {unknown} b
 * @returns {number}
 */
export function faCompare(a, b) {
	if (typeof a === "string" && typeof b === "string") {
		return faStringCompare(a, b);
	}
	if (Array.isArray(a) && Array.isArray(b)) {
		const length = Math.min(a.length, b.length);
		for (let index = 0; index < length; index += 1) {
			const order = faCompare(a[index], b[index]);
			if (order !== 0) {
				return order;
			}
		}
		return a.length - b.length;
	}
	const x = /** @type {number} */ (a);
	const y = /** @type {number} */ (b);
	return x < y ? -1 : x > y ? 1 : 0;
}
