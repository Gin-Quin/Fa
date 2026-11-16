import { faAssign } from "./copy.js";

// @fa-helper faReferenceKey
/** The second half of the most recently returned general reference. */
export let faReferenceKey;

// @fa-helper faReferenceResult deps: faReferenceKey
/**
 * Publish after every cleanup. A caller saves the key immediately after calling.
 * @param {any} container
 * @param {any} key
 * @returns {any}
 */
export function faReferenceResult(container, key) {
	faReferenceKey = key;
	return container;
}

// @fa-helper faReferenceRead
/**
 * Read a general container/key locator. Known local places use direct accesses.
 * @param {any} container
 * @param {any} key
 * @returns {any}
 */
export function faReferenceRead(container, key) {
	if (container === undefined) return undefined;
	const tag = container[Symbol.toStringTag];
	if (container instanceof Set || tag === "fa:Set") return key;
	return container instanceof Map || tag === "fa:ValuePool" || tag === "fa:HashMap"
		? container.get(key)
		: container[key];
}

// @fa-helper faReferenceSet
/**
 * Write a general locator. Its declared slot type decides replacement semantics.
 * @param {any} container
 * @param {any} key
 * @param {any} value
 * @returns {void}
 */
export function faReferenceSet(container, key, value) {
	const tag = container[Symbol.toStringTag];
	if (container instanceof Map || tag === "fa:ValuePool" || tag === "fa:HashMap")
		container.set(key, value);
	else container[key] = value;
}

// @fa-helper faMapAssign deps: faAssign
/**
 * Replace a map value while preserving bare references to an existing object.
 * The caller applies the ordinary storage copy/move rules to value.
 * @param {any} map
 * @param {any} key
 * @param {any} value
 */
export function faMapAssign(map, key, value) {
 const previous = map.get(key);
 if (previous !== null && typeof previous === "object" && value !== null && typeof value === "object") faAssign(previous, value);
 else map.set(key, value);
}
