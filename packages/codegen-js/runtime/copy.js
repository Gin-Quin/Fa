// Copies and in-place writes (advanced brief 6.5, types brief D60, values
// brief D-81).
//
// Ordinary values retain value semantics; structures move or use explicit `copy`. The backend
// generates `copy$T` per destination type, which builds the object with the
// fields of `T` in declaration order (one hidden class per type) and reads
// only those fields (a narrowing copy). `faCopy` is the generic version for
// erased generic code: it keeps the properties of the source, in the
// source's order (which is the declaration order when the value was built by
// compiled code). Strings and primitives are immutable in JavaScript and
// shared; functions and synchronization handles share their identity.

import { FaHashMap, faIsBytes, FaSet } from "./collections.js";
import { faSharedHandle } from "./concurrency.js";
import { faSides } from "./relations.js";

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
	// a pool of values (`FaValuePool`), recognized by its tag
	const shell = /** @type {any} */ (source);
	if (
		source instanceof FaHashMap ||
		source instanceof FaSet ||
		shell[Symbol.toStringTag] === "fa:ValuePool"
	) {
		return shell.emptyCopy();
	}
	if (source instanceof Map) {
		return new Map();
	}
	if (source instanceof Set) {
		return new Set();
	}
	return Object.create(Object.getPrototypeOf(source));
}

// @fa-helper faCopyAs deps:
/**
 * Execute a generated converting-copy plan without recursive JavaScript calls.
 * Jobs either copy one value into a destination property or commit a completed
 * map/set entry after its children have been copied.
 * @param {any} value
 * @param {(value: any, target: any, key: any, work: any[]) => void} plan
 * @returns {any}
 */
export function faCopyAs(value, plan) {
	const result = { value: undefined };
	/** @type {any[]} */
	const work = [[plan, value, result, "value"]];
	while (work.length > 0) {
		const job = work.pop();
		if (job.length === 1) job[0]();
		else job[0](job[1], job[2], job[3], work);
	}
	return result.value;
}

// @fa-helper faCopy deps: faCopyAs faCopyShell FaHashMap FaSet faSides faSharedHandle faIsBytes
/**
 * A deep structural copy of a Fa value. Children finish before a map/set
 * entry is hashed; all traversal is iterative, including recursive keys.
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function faCopy(value) {
	/** @type {(value: any, target: any, key: any, work: any[]) => void} */
	const plan = (source, owner, key, work) => {
		if (typeof source !== "object" || source === null || source[faSharedHandle] === true) {
			owner[key] = source;
			return;
		}
		if (faIsBytes(source)) {
			owner[key] = source.slice();
			return;
		}
		const target = faCopyShell(source);
		owner[key] = target;
		if (Array.isArray(source)) {
			target.length = source.length;
			for (let index = source.length - 1; index >= 0; index--)
				work.push([plan, source[index], target, index]);
		} else if (source instanceof Map || source instanceof FaHashMap) {
			const entries = Array.from(source);
			for (let index = entries.length - 1; index >= 0; index--) {
				const entry = { key: undefined, value: undefined };
				work.push([() => target.set(entry.key, entry.value)]);
				work.push([plan, entries[index][1], entry, "value"]);
				work.push([plan, entries[index][0], entry, "key"]);
			}
		} else if (
			source instanceof Set ||
			source instanceof FaSet ||
			source[Symbol.toStringTag] === "fa:ValuePool"
		) {
			const entries = Array.from(source);
			for (let index = entries.length - 1; index >= 0; index--) {
				const entry = { value: undefined };
				work.push([() => target.add(entry.value)]);
				work.push([plan, entries[index], entry, "value"]);
			}
		} else {
			const sides = source[faSides];
			const keys = Object.keys(source);
			// Establish the source's property order before scheduling children.
			for (const key of keys) target[key] = undefined;
			for (let index = keys.length - 1; index >= 0; index--) {
				const key = keys[index];
				if (sides !== undefined && sides.includes(key))
					target[key] = source[key] instanceof Set ? new Set() : undefined;
				else work.push([plan, source[key], target, key]);
			}
			if (sides !== undefined) target[faSides] = sides;
		}
	};
	return faCopyAs(value, plan);
}

// @fa-helper faUnlinked deps: faCopy faSides
/**
 * A value of a type parameter read from a place and stored (erased generic
 * code shares values): an element with relation sides is copied without
 * its links (D18), any other value is itself.
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function faUnlinked(value) {
	const element = /** @type {any} */ (value);
	if (typeof element === "object" && element !== null && element[faSides] !== undefined) {
		return faCopy(value);
	}
	return value;
}

// @fa-helper faCopyGraph deps: faCopyShell FaHashMap FaSet faSides faSharedHandle faIsBytes
/**
 * A copy of an owner of related collections (advanced brief 8.5, copies):
 * every object is copied once, and every reference to an object of the
 * value (a relation side, an element of a `Set`) points at its copy, so the
 * links of the copy join the copied elements. Iterative.
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function faCopyGraph(value) {
	/** @type {Map<object, any>} */
	const copies = new Map();
	/** @type {any[]} */
	const pending = [];
	/**
	 * @param {any} item
	 * @returns {any}
	 */
	const copyOf = (item) => {
		if (typeof item !== "object" || item === null || item[faSharedHandle] === true) {
			return item;
		}
		if (faIsBytes(item)) return item.slice();
		let copy = copies.get(item);
		if (copy === undefined) {
			copy = faCopyShell(item);
			copies.set(item, copy);
			pending.push(item, copy);
		}
		return copy;
	};
	const root = copyOf(value);
	while (pending.length > 0) {
		const target = pending.pop();
		const source = pending.pop();
		if (Array.isArray(source)) {
			for (let index = 0; index < source.length; index += 1) {
				target.push(copyOf(source[index]));
			}
		} else if (source instanceof Map || source instanceof FaHashMap) {
			for (const [key, item] of source) {
				target.set(key, copyOf(item));
			}
		} else if (
			source instanceof Set ||
			source instanceof FaSet ||
			source[Symbol.toStringTag] === "fa:ValuePool"
		) {
			for (const item of source) {
				target.add(copyOf(item));
			}
		} else {
			for (const key of Object.keys(source)) {
				target[key] = copyOf(source[key]);
			}
			if (source[faSides] !== undefined) {
				target[faSides] = source[faSides];
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

// @fa-helper faAssign deps: faCopy FaHashMap FaSet faIsBytes
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
	/** @type {any[]} */
	const pending = [target, source];
	while (pending.length > 0) {
		const from = pending.pop();
		const into = pending.pop();
		if (into === from) continue;
		if (faIsBytes(into)) {
			into.set(from);
		} else if (Array.isArray(into)) {
			into.length = 0;
			for (const item of from) {
				into.push(faCopy(item));
			}
		} else if (into instanceof Map || into instanceof FaHashMap) {
			into.clear();
			for (const [key, item] of from) {
				into.set(key, faCopy(item));
			}
		} else if (
			into instanceof Set ||
			into instanceof FaSet ||
			into[Symbol.toStringTag] === "fa:ValuePool"
		) {
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
					(!faIsBytes(old) || old.length === next.length) &&
					old.$t === next.$t;
				if (sameKind) {
					pending.push(old, next);
				} else {
					into[key] = faCopy(next);
				}
			}
		}
	}
	return target;
}
