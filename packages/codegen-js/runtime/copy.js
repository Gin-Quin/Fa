// Copies and in-place writes (advanced brief 6.5, types brief D60, values
// brief D-81).
//
// Objects and collections are values in Fa: `let b = a` copies. The backend
// generates `copy$T` per destination type, which builds the object with the
// fields of `T` in declaration order (one hidden class per type) and reads
// only those fields (a narrowing copy). `faCopy` is the generic version for
// erased generic code: it keeps the properties of the source, in the
// source's order (which is the declaration order when the value was built by
// compiled code). Strings and primitives are immutable in JavaScript and
// shared; functions are shared; set elements and map keys are read-only in Fa
// and shared.

import { FaHashMap, FaSet } from "./collections.js";

// @fa-helper faCopyShell deps: FaHashMap FaSet
/**
 * An empty container of the same kind as `source`: an array, a `Map`, a
 * `FaHashMap` or `FaSet` with the same hash and key function, a `Set`, or an
 * object with the same prototype.
 * @param {object} source
 * @returns {any}
 */
export function faCopyShell(source) {
	if (Array.isArray(source)) {
		return [];
	}
	if (source instanceof FaHashMap || source instanceof FaSet) {
		return source.emptyCopy();
	}
	if (source instanceof Map) {
		return new Map();
	}
	if (source instanceof Set) {
		return new Set();
	}
	return Object.create(Object.getPrototypeOf(source));
}

// @fa-helper faCopy deps: faCopyShell FaHashMap FaSet
/**
 * A deep structural copy of a Fa value. Iterative: long recursive chains
 * (linked lists) don't overflow the stack.
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function faCopy(value) {
	if (typeof value !== "object" || value === null) {
		return value;
	}
	/** @type {any[]} */
	const pending = [];
	/**
	 * @param {any} item
	 * @returns {any}
	 */
	const copyLater = (item) => {
		if (typeof item !== "object" || item === null) {
			return item;
		}
		const shell = faCopyShell(item);
		pending.push(item, shell);
		return shell;
	};
	const root = copyLater(value);
	while (pending.length > 0) {
		const target = pending.pop();
		const source = pending.pop();
		if (Array.isArray(source)) {
			for (let index = 0; index < source.length; index += 1) {
				target.push(copyLater(source[index]));
			}
		} else if (source instanceof Map || source instanceof FaHashMap) {
			for (const [key, item] of source) {
				target.set(key, copyLater(item));
			}
		} else if (source instanceof Set || source instanceof FaSet) {
			for (const item of source) {
				target.add(item);
			}
		} else {
			for (const key of Object.keys(source)) {
				target[key] = copyLater(source[key]);
			}
		}
	}
	return root;
}

// @fa-helper faCopyInto
/**
 * `local = place` for an array local of primitives that nothing else sees
 * (no loop iterates it, no function captures it): the items of `source` are
 * copied into `target`'s own storage when the lengths match, which saves an
 * allocation per assignment in a loop; otherwise a new copy.
 * @template T
 * @param {T[]} target
 * @param {T[]} source
 * @returns {T[]} the value to assign: `target`, or a copy of `source`
 */
export function faCopyInto(target, source) {
	const length = source.length;
	if (target === undefined || target.length !== length) {
		return source.slice();
	}
	for (let index = 0; index < length; index++) {
		target[index] = source[index];
	}
	return target;
}

// @fa-helper faAssign deps: faCopy FaHashMap FaSet
/**
 * Writes a whole value into an existing object or collection, in place, so
 * that the live references to it (and to its nested objects of the same
 * shape) see the new value (D60). Collections get their contents replaced;
 * objects are assigned field by field, recursively where the old and the new
 * field values are objects of the same kind (same prototype, same union tag
 * `$t`), and replaced by a copy otherwise.
 * @template T
 * @param {T} target
 * @param {T} source
 * @returns {T} the target
 */
export function faAssign(target, source) {
	const into = /** @type {any} */ (target);
	const from = /** @type {any} */ (source);
	if (into === from) {
		return target;
	}
	if (Array.isArray(into)) {
		into.length = 0;
		for (const item of from) {
			into.push(faCopy(item));
		}
	} else if (into instanceof Map || into instanceof FaHashMap) {
		into.clear();
		for (const [key, item] of from) {
			into.set(key, faCopy(item));
		}
	} else if (into instanceof Set || into instanceof FaSet) {
		into.clear();
		for (const item of from) {
			into.add(item);
		}
	} else {
		for (const key of Object.keys(from)) {
			const old = into[key];
			const next = from[key];
			const sameKind =
				typeof old === "object" &&
				old !== null &&
				typeof next === "object" &&
				next !== null &&
				Object.getPrototypeOf(old) === Object.getPrototypeOf(next) &&
				old.$t === next.$t;
			if (sameKind) {
				faAssign(old, next);
			} else {
				into[key] = faCopy(next);
			}
		}
	}
	return target;
}
