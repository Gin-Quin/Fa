import { faDeleteWhere, faFind, faSetFilter, faSetTake } from "./collections.js";
import { faAssign } from "./copy.js";

// @fa-helper faReferenceCellTag
/** An internal primitive place, distinguishable from every Fa object. */
export const faReferenceCellTag = Symbol("fa:reference");

// @fa-helper faReferenceValue deps: faReferenceCellTag
/**
 * Object references are the object itself; only primitive places use cells.
 * @param {any} reference
 * @returns {any}
 */
export function faReferenceValue(reference) {
	return reference?.[faReferenceCellTag] ? reference.$v : reference;
}

// @fa-helper faReferenceWrite deps: faReferenceCellTag faAssign
/**
 * Writes an erased reference without replacing a designated object's identity.
 * @param {any} reference
 * @param {any} value
 * @returns {void}
 */
export function faReferenceWrite(reference, value) {
	if (reference[faReferenceCellTag]) reference.$v = value;
	else faAssign(reference, value);
}

// @fa-helper faFindReference deps: faFind
/**
 * A generic pool lookup preserves the object or the exact primitive slot.
 * @param {any} collection
 * @param {(value: any) => boolean} predicate
 * @returns {any}
 */
export function faFindReference(collection, predicate) {
	if (collection[Symbol.toStringTag] !== "fa:ValuePool") return faFind(collection, predicate);
	const slot = collection.slotOf(predicate);
	return slot === undefined ? undefined : collection.reference(slot);
}

// @fa-helper faBorrowSlot deps: faReferenceCellTag
/**
 * Evaluate a place once and reuse its primitive carrier across calls. Object
 * places need no carrier. Value pools own their slot cache so deletion can
 * retire it; array/field/map caches live no longer than their container.
 * @param {any} container
 * @param {any} key
 * @param {boolean} map
 * @param {boolean} [object]
 * @returns {any}
 */
export function faBorrowSlot(container, key, map, object = false) {
	const value = map ? container.get(key) : container[key];
	if (container[Symbol.toStringTag] === "fa:ValuePool") return container.reference(key);
	if (object) return value;
	const state = /** @type {any} */ (faBorrowSlot);
	state.cache ??= new WeakMap();
	let slots = state.cache.get(container);
	if (slots === undefined) {
		slots = new Map();
		state.cache.set(container, slots);
	}
	let reference = slots.get(key);
	if (reference !== undefined) return reference;
	// Bound the cache of a map whose keys are repeatedly removed or replaced.
	if (map && slots.size > Math.max(32, container.size * 2)) {
		for (const old of slots.keys()) if (!container.has(old)) slots.delete(old);
		if (slots.size > Math.max(32, container.size * 2)) slots.clear();
	}
	reference = {
		[faReferenceCellTag]: true,
		$k: key,
		get $v() {
			return map ? container.get(key) : container[key];
		},
		set $v(value) {
			if (map) container.set(key, value);
			else container[key] = value;
		},
	};
	slots.set(key, reference);
	return reference;
}

// @fa-helper faErasedPoolRemove deps: faSetTake
/**
 * Remove the exact slot through an erased Pool<Item> parameter.
 * @param {any} collection
 * @param {any} reference
 * @param {boolean} take
 * @returns {any}
 */
export function faErasedPoolRemove(collection, reference, take) {
	if (collection[Symbol.toStringTag] === "fa:ValuePool")
		return take ? collection.take(reference.$k) : collection.delete(reference.$k);
	return take ? faSetTake(collection, reference) : collection.delete(reference);
}

// @fa-helper faErasedPoolFilter deps: faDeleteWhere faSetFilter
/**
 * Keep the representation of a generic pool when selecting its elements.
 * @param {any} collection
 * @param {(value: any) => boolean} predicate
 * @param {boolean} remove
 * @returns {any}
 */
export function faErasedPoolFilter(collection, predicate, remove) {
	if (collection[Symbol.toStringTag] === "fa:ValuePool")
		return remove ? collection.deleteWhere(predicate) : collection.filter(predicate);
	return remove ? faDeleteWhere(collection, predicate) : faSetFilter(collection, predicate);
}
