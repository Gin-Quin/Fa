// Collections (types brief section 5, D31, D33, D35, D37, D40; values brief
// section 12).
//
// - An `Array` (and a `GrowingArray`, and a tuple) is a JavaScript array. A
//   proven index is a plain `array[i]`; the helpers here check the others.
// - A `Map` with primitive keys is a JavaScript `Map`; with object or tuple
//   keys, a `FaHashMap`, which has the same interface.
// - A `Set` of primitives without `key` is a JavaScript `Set`; any other set
//   is a `FaSet`, which has the interface of a JavaScript `Set`.
// - A `Range` value is a `FaRange`.

import { faCompare, faEquals, faHash, faKeyEquals } from "./equality.js";
import { CapacityError, IndexError, KeyNotFound } from "./errors.js";
import { faFormat } from "./format.js";
import { faCheckInteger, faRangeOverflow } from "./integers.js";
import { faStop } from "./stops.js";
import { faCharacters, faStringCompare } from "./strings.js";

// @fa-helper faInlinePush deps: CapacityError
/**
 * Appends only when the bounded initialized prefix has room.
 * @template T
 * @param {T[]} array
 * @param {number} capacity
 * @param {T} value
 */
export function faInlinePush(array, capacity, value) {
	if (array.length >= capacity) throw new CapacityError();
	array.push(value);
}

// @fa-helper faNormalizeIndex deps: IndexError
/**
 * Checks an index in `-length..length` and makes it non-negative (negative
 * indices count from the end); throws `IndexError` outside.
 * @param {number} length
 * @param {number} index
 * @returns {number}
 */
export function faNormalizeIndex(length, index) {
	const position = index < 0 ? index + length : index;
	if (position < 0 || position >= length) {
		throw new IndexError(index, length);
	}
	return position;
}

// @fa-helper faSliceBounds deps: IndexError
/**
 * The non-negative `[from, to)` bounds of a slice `start..end` (`inclusive`
 * for `..=`, `undefined` for a missing bound). Each bound must be in
 * `-length..=length` (an inclusive end in `-length..length`), otherwise it
 * throws `IndexError` (D40); a start after the end gives an empty slice.
 * @param {number} length
 * @param {number | undefined} start
 * @param {number | undefined} end
 * @param {boolean} inclusive
 * @returns {[number, number]}
 */
export function faSliceBounds(length, start, end, inclusive) {
	let from = 0;
	let to = length;
	if (start !== undefined) {
		if (start < -length || start > length) {
			throw new IndexError(start, length);
		}
		from = start < 0 ? start + length : start;
	}
	if (end !== undefined) {
		if (end < -length || end > (inclusive ? length - 1 : length)) {
			throw new IndexError(end, length);
		}
		to = (end < 0 ? end + length : end) + (inclusive ? 1 : 0);
	}
	return [from, Math.max(from, to)];
}

// @fa-helper faIndex deps: faNormalizeIndex
/**
 * `array[index]` with an unproven index; throws `IndexError`.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @returns {T}
 */
export function faIndex(array, index) {
	return array[faNormalizeIndex(array.length, index)];
}

// @fa-helper faIndexSet deps: faNormalizeIndex
/**
 * `array[index] = value` with an unproven index; throws `IndexError`.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @param {T} value
 * @returns {void}
 */
export function faIndexSet(array, index, value) {
	array[faNormalizeIndex(array.length, index)] = value;
}

// @fa-helper faArrayGet
/**
 * `array.get(index)`: the item, or `none` (`undefined`) when the index is out
 * of bounds; negative indices count from the end.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @returns {T | undefined}
 */
export function faArrayGet(array, index) {
	const position = index < 0 ? index + array.length : index;
	return position >= 0 && position < array.length ? array[position] : undefined;
}

// @fa-helper faArraySlice deps: faSliceBounds
/**
 * `array[start..end]`: a new array (throws `IndexError` like
 * `faSliceBounds`). `copy` copies the items that own memory (objects are
 * values in Fa, not shared).
 * @template T
 * @param {T[]} array
 * @param {number | undefined} start
 * @param {number | undefined} end
 * @param {boolean} inclusive
 * @param {(item: T) => T} [copy]
 * @returns {T[]}
 */
export function faArraySlice(array, start, end, inclusive, copy) {
	const [from, to] = faSliceBounds(array.length, start, end, inclusive);
	const slice = array.slice(from, to);
	if (copy !== undefined) {
		for (let index = 0; index < slice.length; index += 1) {
			slice[index] = copy(slice[index]);
		}
	}
	return slice;
}

// @fa-helper faArrayInsert deps: IndexError
/**
 * `array.insert(index, item)`: `index` in `0..=length` (D31), otherwise
 * throws `IndexError`.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @param {T} item
 * @returns {void}
 */
export function faArrayInsert(array, index, item) {
	if (index < 0 || index > array.length) {
		throw new IndexError(index, array.length);
	}
	array.splice(index, 0, item);
}

// @fa-helper faArrayDelete deps: faNormalizeIndex
/**
 * `array.delete(index)`; throws `IndexError` like indexing.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @returns {void}
 */
export function faArrayDelete(array, index) {
	array.splice(faNormalizeIndex(array.length, index), 1);
}

// @fa-helper faArrayTake deps: faNormalizeIndex
/**
 * `array.take(index)`: removes and returns the item; throws `IndexError` like
 * indexing.
 * @template T
 * @param {T[]} array
 * @param {number} index
 * @returns {T}
 */
export function faArrayTake(array, index) {
	return array.splice(faNormalizeIndex(array.length, index), 1)[0];
}

// @fa-helper faArrayDeleteWhere
/**
 * `array.deleteWhere(predicate)`: removes the matching items in place, in one
 * pass (no new array).
 * @template T
 * @param {T[]} array
 * @param {(item: T) => boolean} predicate
 * @returns {void}
 */
export function faArrayDeleteWhere(array, predicate) {
	let kept = 0;
	let index = 0;
	try {
		for (; index < array.length; index += 1) {
			const item = array[index];
			if (!predicate(item)) {
				array[kept] = item;
				kept += 1;
			}
		}
	} finally {
		// an error of the predicate: the items not tested yet stay, after the kept ones
		for (let rest = index; rest < array.length; rest += 1) {
			array[kept] = array[rest];
			kept += 1;
		}
		array.length = kept;
	}
}

// @fa-helper faComparator
/**
 * Turns a Fa comparator (its result's sign decides; `Integer`, `Float` or a
 * bigint; NaN counts as 0, D35) into a JavaScript one.
 * @template T
 * @param {(a: T, b: T) => number | bigint} compare
 * @returns {(a: T, b: T) => number}
 */
export function faComparator(compare) {
	return (a, b) => {
		const order = compare(a, b);
		return order > 0 ? 1 : order < 0 ? -1 : 0;
	};
}

// @fa-helper faArraySortInPlace deps: faCompare
/**
 * `array.sortInPlace(compare)`: stable; without a comparator, the natural
 * order (never JavaScript's string sort). A merge sort written in JavaScript
 * rather than `Array.prototype.sort`: the engine inlines the comparator
 * into it (Bun's native sort calls it from C++, a quarter slower on a
 * million objects). The comparator's sign decides (`Integer`, `Float` or a
 * bigint; NaN counts as 0, D35). The items are sorted in a copy, written
 * back at the end: a comparator that throws leaves the array as it was.
 * @template T
 * @param {T[]} array
 * @param {(a: T, b: T) => number | bigint} [compare]
 * @returns {void}
 */
export function faArraySortInPlace(array, compare = faCompare) {
	const length = array.length;
	if (length < 2) {
		return;
	}
	let source = array.slice();
	// runs of 16 items, by insertion
	for (let low = 0; low < length; low += 16) {
		const high = low + 16 < length ? low + 16 : length;
		for (let index = low + 1; index < high; index++) {
			const item = source[index];
			let position = index - 1;
			while (position >= low && compare(source[position], item) > 0) {
				source[position + 1] = source[position];
				position--;
			}
			source[position + 1] = item;
		}
	}
	// merged by pairs: an item of the right run goes first only when smaller
	let target = array.slice();
	for (let width = 16; width < length; width *= 2) {
		for (let low = 0; low < length; low += 2 * width) {
			const middle = low + width < length ? low + width : length;
			const high = low + 2 * width < length ? low + 2 * width : length;
			let left = low;
			let right = middle;
			let next = low;
			if (middle < high && compare(source[middle - 1], source[middle]) > 0) {
				while (left < middle && right < high) {
					target[next++] =
						compare(source[right], source[left]) < 0 ? source[right++] : source[left++];
				}
			}
			while (left < middle) {
				target[next++] = source[left++];
			}
			while (right < high) {
				target[next++] = source[right++];
			}
		}
		const merged = target;
		target = source;
		source = merged;
	}
	for (let index = 0; index < length; index++) {
		array[index] = source[index];
	}
}

// @fa-helper faArraySort deps: faArraySortInPlace
/**
 * `array.sort(compare)`: a sorted copy (shallow: the backend copies the items
 * when they own memory).
 * @template T
 * @param {T[]} array
 * @param {(a: T, b: T) => number | bigint} [compare]
 * @returns {T[]}
 */
export function faArraySort(array, compare) {
	const sorted = array.slice();
	faArraySortInPlace(sorted, compare);
	return sorted;
}

// @fa-helper faSortIntegers
/**
 * `numbers.sortInPlace()` on an array of number-represented integers (never
 * NaN): sorted as a `Float64Array`, a numeric sort without comparator calls
 * (ten times faster than `sort` with one), then copied back. Equal integers
 * are the same value, so the stability of `sort` can't be observed (`-0`,
 * which the typed sort puts before `0`, is the integer `0`: conversions and
 * printing make it `0`).
 * @param {number[]} array
 * @returns {number[]} the array
 */
export function faSortIntegers(array) {
	const sorted = Float64Array.from(array).sort();
	for (let index = 0; index < sorted.length; index++) {
		array[index] = sorted[index];
	}
	return array;
}

// @fa-helper faSortFloats
/**
 * `floats.sortInPlace()`: the natural order of floats, `a - b` (a NaN
 * operand gives NaN, which `sort` reads as equal, as `faCompare` does).
 * @param {number[]} array
 * @returns {number[]} the array
 */
export function faSortFloats(array) {
	return array.sort((a, b) => a - b);
}

// @fa-helper faSortStrings deps: faArraySortInPlace faStringCompare
/**
 * `texts.sortInPlace()`: by code point, without `faCompare`'s type tests.
 * @param {string[]} array
 * @returns {string[]} the array
 */
export function faSortStrings(array) {
	faArraySortInPlace(array, faStringCompare);
	return array;
}

// @fa-helper faArrayIncludes deps: faEquals
/**
 * `array.includes(value)` and `value is in array`: an item `==` to the value
 * (so NaN is never found, D-65).
 * @template T
 * @param {T[]} array
 * @param {T} value
 * @param {(a: T, b: T) => boolean} [equals] the generated `==` of the items
 * @returns {boolean}
 */
export function faArrayIncludes(array, value, equals) {
	if (equals === undefined && (typeof value !== "object" || value === null)) {
		return array.indexOf(value) !== -1;
	}
	const same = equals ?? faEquals;
	for (const item of array) {
		if (same(item, value)) {
			return true;
		}
	}
	return false;
}

// @fa-helper faArrayIndexOf deps: faEquals
/**
 * `array.indexOf(value)`: the index of the first item `==` to the value, or
 * `none` (`undefined`).
 * @template T
 * @param {T[]} array
 * @param {T} value
 * @param {(a: T, b: T) => boolean} [equals]
 * @returns {number | undefined}
 */
export function faArrayIndexOf(array, value, equals) {
	if (equals === undefined && (typeof value !== "object" || value === null)) {
		const index = array.indexOf(value);
		return index === -1 ? undefined : index;
	}
	const same = equals ?? faEquals;
	for (let index = 0; index < array.length; index += 1) {
		if (same(array[index], value)) {
			return index;
		}
	}
	return undefined;
}

// @fa-helper faSumIntegers deps: faCheckInteger
/**
 * `values.sum()` on `Integer` items: `0` when empty; every partial sum is
 * checked against ±(2^53 − 1) (D39).
 * @param {Iterable<number>} values
 * @param {string} [location]
 * @returns {number}
 */
export function faSumIntegers(values, location) {
	let sum = 0;
	for (const value of values) {
		sum = faCheckInteger(sum + value, location);
	}
	return sum;
}

// @fa-helper FaRange
/**
 * A `Range` value: `{ start: Integer?, end: Integer?, includesEnd: Boolean }`
 * (`undefined` for a missing bound). `for` over a range literal compiles to
 * a plain loop; a `FaRange` only exists when a range is a value.
 */
export class FaRange {
	/**
	 * @param {number | undefined} start
	 * @param {number | undefined} end
	 * @param {boolean} includesEnd
	 */
	constructor(start, end, includesEnd) {
		/** @type {number | undefined} */
		this.start = start;
		/** @type {number | undefined} */
		this.end = end;
		/** @type {boolean} */
		this.includesEnd = includesEnd;
	}

	/** The tag that printing recognizes, without depending on this class. */
	get [Symbol.toStringTag]() {
		return "fa:Range";
	}
}

// @fa-helper faRangeIncludes
/**
 * `value is in range`: missing bounds are unbounded.
 * @param {FaRange} range
 * @param {number} value
 * @returns {boolean}
 */
export function faRangeIncludes(range, value) {
	if (range.start !== undefined && value < range.start) {
		return false;
	}
	if (range.end === undefined) {
		return true;
	}
	return range.includesEnd ? value <= range.end : value < range.end;
}

// @fa-helper faRangeValues deps: faRangeOverflow faStop
/**
 * The values of a `for i in range` over a range value (a range literal
 * compiles to a plain loop instead). A range without start stops the
 * program (design/decisions.md). Without an end, the loop stops the program when it would go
 * past the maximum of `Integer` (D-40); an inclusive end at the maximum
 * terminates.
 * @param {FaRange} range
 * @param {string} [location]
 * @returns {IterableIterator<number>}
 */
export function faRangeValues(range, location) {
	if (range.start === undefined) {
		faStop("the range has no start: a `for` loop needs one", location);
	}
	let next = /** @type {number} */ (range.start);
	const end = range.end;
	const includesEnd = range.includesEnd;
	let done = false;
	return {
		[Symbol.iterator]() {
			return this;
		},
		next() {
			if (done || (end !== undefined && (includesEnd ? next > end : next >= end))) {
				done = true;
				return { done: true, value: undefined };
			}
			if (next > 9007199254740991) {
				faRangeOverflow(location);
			}
			const value = next;
			if (end !== undefined && includesEnd && value === end) {
				done = true;
			}
			next += 1;
			return { done: false, value };
		},
	};
}

// @fa-helper FaHashMap deps: faHash faKeyEquals
/**
 * A map compared by Fa `==` on its keys (deep, floats with SameValueZero),
 * for object and tuple keys. It has the interface of a JavaScript `Map`, and
 * keeps the insertion order. Primitive keys are found through a JavaScript
 * `Map` (SameValueZero already); the others through a JavaScript `Map` from
 * their 32-bit structural hash to an entry `{ key, value }`, or to an array
 * of entries when several keys share a hash.
 * @template K, V
 */
export class FaHashMap {
	/**
	 * Every entry in insertion order, by its primitive key, or by the entry
	 * itself for the other keys (an entry is never a key).
	 * @type {Map<unknown, {key: K, value: V}>}
	 */
	#entries = new Map();
	/** @type {Map<number, {key: K, value: V} | {key: K, value: V}[]>} */
	#buckets = new Map();

	/**
	 * @param {Iterable<[K, V]>} [entries]
	 * @param {(key: K) => number} [hash] the generated hash of the key type
	 * @param {(a: K, b: K) => boolean} [equals] the generated `==` of the key type
	 */
	constructor(entries, hash = faHash, equals = faKeyEquals) {
		/** @type {(key: K) => number} */
		this.hash = hash;
		/** @type {(a: K, b: K) => boolean} */
		this.equals = equals;
		if (entries !== undefined) {
			for (const [key, value] of entries) {
				this.set(key, value);
			}
		}
	}

	/** The tag that printing recognizes, without depending on this class. */
	get [Symbol.toStringTag]() {
		return "fa:HashMap";
	}

	/**
	 * An empty map with the same hash and equality.
	 * @returns {FaHashMap<K, V>}
	 */
	emptyCopy() {
		return new FaHashMap(undefined, this.hash, this.equals);
	}

	/** @returns {number} */
	get size() {
		return this.#entries.size;
	}

	/**
	 * The entry of a key.
	 * @param {K} key
	 * @returns {{key: K, value: V} | undefined}
	 */
	#entry(key) {
		if (typeof key !== "object" || key === null) {
			return this.#entries.get(key);
		}
		const bucket = this.#buckets.get(this.hash(key));
		if (bucket === undefined) {
			return undefined;
		}
		if (!Array.isArray(bucket)) {
			return this.equals(bucket.key, key) ? bucket : undefined;
		}
		return bucket.find((entry) => this.equals(entry.key, key));
	}

	/**
	 * @param {K} key
	 * @returns {boolean}
	 */
	has(key) {
		return this.#entry(key) !== undefined;
	}

	/**
	 * @param {K} key
	 * @returns {V | undefined}
	 */
	get(key) {
		return this.#entry(key)?.value;
	}

	/**
	 * Sets the value of a key; an existing key keeps its place (and its
	 * first spelling).
	 * @param {K} key
	 * @param {V} value
	 * @returns {this}
	 */
	set(key, value) {
		const existing = this.#entry(key);
		if (existing !== undefined) {
			existing.value = value;
			return this;
		}
		const entry = { key, value };
		if (typeof key !== "object" || key === null) {
			this.#entries.set(key, entry);
			return this;
		}
		const hash = this.hash(key);
		const bucket = this.#buckets.get(hash);
		if (bucket === undefined) {
			this.#buckets.set(hash, entry);
		} else if (Array.isArray(bucket)) {
			bucket.push(entry);
		} else {
			this.#buckets.set(hash, [bucket, entry]);
		}
		this.#entries.set(entry, entry);
		return this;
	}

	/**
	 * @param {K} key
	 * @returns {boolean} whether the key was present
	 */
	delete(key) {
		if (typeof key !== "object" || key === null) {
			return this.#entries.delete(key);
		}
		const hash = this.hash(key);
		const bucket = this.#buckets.get(hash);
		if (bucket === undefined) {
			return false;
		}
		let entry;
		if (!Array.isArray(bucket)) {
			if (!this.equals(bucket.key, key)) {
				return false;
			}
			entry = bucket;
			this.#buckets.delete(hash);
		} else {
			const index = bucket.findIndex((candidate) => this.equals(candidate.key, key));
			if (index === -1) {
				return false;
			}
			entry = bucket[index];
			bucket.splice(index, 1);
			if (bucket.length === 1) {
				this.#buckets.set(hash, bucket[0]);
			}
		}
		this.#entries.delete(entry);
		return true;
	}

	/** @returns {void} */
	clear() {
		this.#entries.clear();
		this.#buckets.clear();
	}

	/** @returns {Generator<[K, V]>} */
	*entries() {
		for (const entry of this.#entries.values()) {
			yield [entry.key, entry.value];
		}
	}

	/** @returns {Generator<K>} */
	*keys() {
		for (const entry of this.#entries.values()) {
			yield entry.key;
		}
	}

	/** @returns {Generator<V>} */
	*values() {
		for (const entry of this.#entries.values()) {
			yield entry.value;
		}
	}

	/** @returns {Generator<[K, V]>} */
	[Symbol.iterator]() {
		return this.entries();
	}

	/**
	 * @param {(value: V, key: K, map: FaHashMap<K, V>) => void} callback
	 * @returns {void}
	 */
	forEach(callback) {
		for (const entry of this.#entries.values()) {
			callback(entry.value, entry.key, this);
		}
	}
}

// @fa-helper FaSet deps: FaHashMap faHash faKeyEquals
/**
 * A set that is not a plain JavaScript `Set` of primitives: elements compared
 * by Fa `==` (objects, tuples), or by a `key` function (`Set.from(users,
 * key = (user) => user.id)`, D33). It has the interface of a JavaScript `Set`
 * (`has`, `add`, `delete`, `size`, iteration), so that the code using a set
 * doesn't depend on its kind.
 * @template T
 */
export class FaSet {
	/** @type {FaHashMap<unknown, T>} */
	#elements;

	/**
	 * @param {Iterable<T>} [values]
	 * @param {{ key?: (value: T) => unknown, hash?: (key: any) => number, equals?: (a: any, b: any) => boolean }} [options]
	 */
	constructor(values, options = {}) {
		/** @type {((value: T) => unknown) | undefined} */
		this.key = options.key;
		this.#elements = new FaHashMap(
			undefined,
			options.hash ?? faHash,
			options.equals ?? faKeyEquals,
		);
		if (values !== undefined) {
			for (const value of values) {
				this.add(value);
			}
		}
	}

	/** The tag that printing recognizes, without depending on this class. */
	get [Symbol.toStringTag]() {
		return "fa:Set";
	}

	/**
	 * An empty set with the same key function, hash and equality.
	 * @returns {FaSet<T>}
	 */
	emptyCopy() {
		return new FaSet(undefined, {
			key: this.key,
			hash: this.#elements.hash,
			equals: this.#elements.equals,
		});
	}

	/**
	 * What identifies a value in this set: its key, or the value itself.
	 * @param {T} value
	 * @returns {unknown}
	 */
	keyOf(value) {
		return this.key === undefined ? value : this.key(value);
	}

	/** @returns {number} */
	get size() {
		return this.#elements.size;
	}

	/**
	 * @param {T} value
	 * @returns {boolean}
	 */
	has(value) {
		return this.#elements.has(this.keyOf(value));
	}

	/**
	 * Adds the value unless an equal one is already there (which is kept).
	 * @param {T} value
	 * @returns {this}
	 */
	add(value) {
		const key = this.keyOf(value);
		if (!this.#elements.has(key)) {
			this.#elements.set(key, value);
		}
		return this;
	}

	/**
	 * @param {T} value
	 * @returns {boolean} whether it was present
	 */
	delete(value) {
		return this.#elements.delete(this.keyOf(value));
	}

	/**
	 * Removes and returns the element equal to the value, or `undefined`.
	 * @param {T} value
	 * @returns {T | undefined}
	 */
	take(value) {
		const key = this.keyOf(value);
		const element = this.#elements.get(key);
		return this.#elements.delete(key) ? element : undefined;
	}

	/** @returns {void} */
	clear() {
		this.#elements.clear();
	}

	/** @returns {Generator<T>} */
	values() {
		return this.#elements.values();
	}

	/** @returns {Generator<T>} */
	keys() {
		return this.#elements.values();
	}

	/** @returns {Generator<[T, T]>} */
	*entries() {
		for (const value of this.#elements.values()) {
			yield [value, value];
		}
	}

	/** @returns {Generator<T>} */
	[Symbol.iterator]() {
		return this.#elements.values();
	}

	/**
	 * @param {(value: T, same: T, set: FaSet<T>) => void} callback
	 * @returns {void}
	 */
	forEach(callback) {
		for (const value of this.#elements.values()) {
			callback(value, value, this);
		}
	}
}

// @fa-helper faSetEmptyLike deps: FaSet
/**
 * An empty set of the same kind as `set` (same key function).
 * @template T
 * @param {Set<T> | FaSet<T>} set
 * @returns {Set<T> | FaSet<T>}
 */
export function faSetEmptyLike(set) {
	return set instanceof FaSet ? set.emptyCopy() : new Set();
}

// @fa-helper faSetTake deps: FaSet
/**
 * `set.take(value)`: removes and returns the element equal to the value, or
 * `none` (`undefined`).
 * @template T
 * @param {Set<T> | FaSet<T>} set
 * @param {T} value
 * @returns {T | undefined}
 */
export function faSetTake(set, value) {
	if (set instanceof FaSet) {
		return set.take(value);
	}
	return set.delete(value) ? value : undefined;
}

// @fa-helper faSetUnion deps: faSetEmptyLike
/**
 * `a.union(b)`: a new set, keyed like `a` (the receiver).
 * @template T
 * @param {Set<T> | FaSet<T>} a
 * @param {Set<T> | FaSet<T>} b
 * @returns {Set<T> | FaSet<T>}
 */
export function faSetUnion(a, b) {
	const result = faSetEmptyLike(a);
	for (const value of a) {
		result.add(value);
	}
	for (const value of b) {
		result.add(value);
	}
	return result;
}

// @fa-helper faSetIntersection deps: faSetEmptyLike
/**
 * `a.intersection(b)`: the elements of `a` that are in `b`, compared with
 * `a`'s key function.
 * @template T
 * @param {Set<T> | FaSet<T>} a
 * @param {Set<T> | FaSet<T>} b
 * @returns {Set<T> | FaSet<T>}
 */
export function faSetIntersection(a, b) {
	const other = faSetEmptyLike(a);
	for (const value of b) {
		other.add(value);
	}
	const result = faSetEmptyLike(a);
	for (const value of a) {
		if (other.has(value)) {
			result.add(value);
		}
	}
	return result;
}

// @fa-helper faSetDifference deps: faSetEmptyLike
/**
 * `a.difference(b)`: the elements of `a` that are not in `b`, compared with
 * `a`'s key function.
 * @template T
 * @param {Set<T> | FaSet<T>} a
 * @param {Set<T> | FaSet<T>} b
 * @returns {Set<T> | FaSet<T>}
 */
export function faSetDifference(a, b) {
	const other = faSetEmptyLike(a);
	for (const value of b) {
		other.add(value);
	}
	const result = faSetEmptyLike(a);
	for (const value of a) {
		if (!other.has(value)) {
			result.add(value);
		}
	}
	return result;
}

// @fa-helper faSetIsSubsetOf deps: faSetEmptyLike
/**
 * `a.isSubsetOf(b)`, compared with `a`'s key function.
 * @template T
 * @param {Set<T> | FaSet<T>} a
 * @param {Set<T> | FaSet<T>} b
 * @returns {boolean}
 */
export function faSetIsSubsetOf(a, b) {
	const other = faSetEmptyLike(a);
	for (const value of b) {
		other.add(value);
	}
	for (const value of a) {
		if (!other.has(value)) {
			return false;
		}
	}
	return true;
}

// @fa-helper faDeleteWhere deps: FaHashMap
/**
 * `deleteWhere(predicate)` on a `Map` (`predicate(key, value)`) or a `Set`
 * (`predicate(value)`), of any kind.
 * @param {Map<any, any> | FaHashMap<any, any> | Set<any> | FaSet<any>} collection
 * @param {(a: any, b: any) => boolean} predicate
 * @returns {void}
 */
export function faDeleteWhere(collection, predicate) {
	const removed = [];
	try {
		if (collection instanceof Map || collection instanceof FaHashMap) {
			for (const [key, value] of collection) {
				if (predicate(key, value)) {
					removed.push(key);
				}
			}
		} else {
			for (const value of collection) {
				if (predicate(value, undefined)) {
					removed.push(value);
				}
			}
		}
	} finally {
		// an error of the predicate: the entries it matched before are deleted
		for (const item of removed) {
			collection.delete(item);
		}
	}
}

// @fa-helper faMapIndex deps: KeyNotFound faFormat
/**
 * `map[key]` with a key not proven present: throws `KeyNotFound` with the
 * `toString()` of the key.
 * @template K, V
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {K} key
 * @param {(key: K) => string} [format] the generated `toString` of the key type
 * @returns {V}
 */
export function faMapIndex(map, key, format = faFormat) {
	if (!map.has(key)) {
		throw new KeyNotFound(format(key));
	}
	return /** @type {V} */ (map.get(key));
}

// @fa-helper faMapGetOrInsert
/**
 * `map.getOrInsert(key, value)`: the value of the key, after inserting
 * `value` when the key is absent.
 * @template K, V
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {K} key
 * @param {V} value
 * @returns {V}
 */
export function faMapGetOrInsert(map, key, value) {
	if (map.has(key)) {
		return /** @type {V} */ (map.get(key));
	}
	map.set(key, value);
	return value;
}

// @fa-helper faMapTake
/**
 * `map.take(key)`: removes the key and returns its value, or `none`
 * (`undefined`) when absent (D37).
 * @template K, V
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {K} key
 * @returns {V | undefined}
 */
export function faMapTake(map, key) {
	if (!map.has(key)) {
		return undefined;
	}
	const value = map.get(key);
	map.delete(key);
	return value;
}

// @fa-helper faSetAdd
/**
 * `set.add(value)`: whether the value was added (`false` when an equal one
 * is already there, D37).
 * @template T
 * @param {Set<T> | FaSet<T>} set
 * @param {T} value
 * @returns {boolean}
 */
export function faSetAdd(set, value) {
	const size = set.size;
	set.add(value);
	return set.size !== size;
}

// @fa-helper faAdd
/**
 * `pool.add(element)`, `columnar.add(element)`: stores the element (a pool or
 * a columnar of objects is a `Set` of them, an ordered columnar an array)
 * and returns it: a reference into a pool is the object itself.
 * @template T
 * @param {Set<T> | T[]} collection
 * @param {T} element
 * @returns {T}
 */
export function faAdd(collection, element) {
	if (Array.isArray(collection)) {
		collection.push(element);
	} else {
		collection.add(element);
	}
	return element;
}

// @fa-helper faSetFilter deps: faSetEmptyLike
/**
 * `set.filter(predicate)`: a new set of the same kind.
 * @template T
 * @param {Set<T> | FaSet<T>} set
 * @param {(value: T) => boolean} predicate
 * @returns {Set<T> | FaSet<T>}
 */
export function faSetFilter(set, predicate) {
	const result = faSetEmptyLike(set);
	for (const value of set) {
		if (predicate(value)) {
			result.add(value);
		}
	}
	return result;
}

// @fa-helper faFind
/**
 * `set.find(predicate)`: the first match, or `none`.
 * @template T
 * @param {Iterable<T>} values
 * @param {(value: T) => boolean} predicate
 * @returns {T | undefined}
 */
export function faFind(values, predicate) {
	for (const value of values) {
		if (predicate(value)) {
			return value;
		}
	}
	return undefined;
}

// @fa-helper faSome
/**
 * `collection.some(predicate)` on a set, or on a map (`(key, value)`).
 * @param {Iterable<any>} values
 * @param {(...values: any[]) => boolean} predicate
 * @param {boolean} [entries] whether the values are `[key, value]` pairs
 * @returns {boolean}
 */
export function faSome(values, predicate, entries = false) {
	for (const value of values) {
		if (entries ? predicate(value[0], value[1]) : predicate(value)) {
			return true;
		}
	}
	return false;
}

// @fa-helper faEvery
/**
 * `collection.every(predicate)` on a set, or on a map (`(key, value)`).
 * @param {Iterable<any>} values
 * @param {(...values: any[]) => boolean} predicate
 * @param {boolean} [entries] whether the values are `[key, value]` pairs
 * @returns {boolean}
 */
export function faEvery(values, predicate, entries = false) {
	for (const value of values) {
		if (!(entries ? predicate(value[0], value[1]) : predicate(value))) {
			return false;
		}
	}
	return true;
}

// @fa-helper faReduce
/**
 * `collection.reduce(combine, initial)` on a set, or on a map
 * (`(accumulator, key, value)`).
 * @template A
 * @param {Iterable<any>} values
 * @param {(accumulator: A, ...values: any[]) => A} combine
 * @param {A} initial
 * @param {boolean} [entries] whether the values are `[key, value]` pairs
 * @returns {A}
 */
export function faReduce(values, combine, initial, entries = false) {
	let accumulator = initial;
	for (const value of values) {
		accumulator = entries ? combine(accumulator, value[0], value[1]) : combine(accumulator, value);
	}
	return accumulator;
}

// @fa-helper faMapEmptyLike deps: FaHashMap
/**
 * An empty map of the same kind (and key equality) as `map`.
 * @param {Map<any, any> | FaHashMap<any, any>} map
 * @returns {Map<any, any> | FaHashMap<any, any>}
 */
export function faMapEmptyLike(map) {
	return map instanceof FaHashMap ? map.emptyCopy() : new Map();
}

// @fa-helper faMapFilter deps: faMapEmptyLike
/**
 * `map.filter((key, value) => …)`: a new map of the same kind.
 * @template K, V
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {(key: K, value: V) => boolean} predicate
 * @returns {Map<K, V> | FaHashMap<K, V>}
 */
export function faMapFilter(map, predicate) {
	const result = faMapEmptyLike(map);
	for (const [key, value] of map) {
		if (predicate(key, value)) {
			result.set(key, value);
		}
	}
	return result;
}

// @fa-helper faMapMap deps: faMapEmptyLike
/**
 * `map.map((key, value) => …)`: the same keys, the new values.
 * @template K, V, R
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {(key: K, value: V) => R} transform
 * @returns {Map<K, R> | FaHashMap<K, R>}
 */
export function faMapMap(map, transform) {
	const result = faMapEmptyLike(map);
	for (const [key, value] of map) {
		result.set(key, transform(key, value));
	}
	return result;
}

// @fa-helper faMapFind
/**
 * `map.find((key, value) => …)`: the first matching `(key, value)`, or `none`.
 * @template K, V
 * @param {Map<K, V> | FaHashMap<K, V>} map
 * @param {(key: K, value: V) => boolean} predicate
 * @returns {[K, V] | undefined}
 */
export function faMapFind(map, predicate) {
	for (const [key, value] of map) {
		if (predicate(key, value)) {
			return [key, value];
		}
	}
	return undefined;
}

// @fa-helper faIterable deps: FaRange faRangeValues faCharacters
/**
 * The items of the source of a `from`: a range gives its integers, a string
 * its characters, any other collection itself.
 * @param {any} source
 * @returns {Iterable<any>}
 */
export function faIterable(source) {
	if (typeof source === "string") {
		return faCharacters(source);
	}
	return source instanceof FaRange ? faRangeValues(source) : source;
}

// @fa-helper FaValuePool
/**
 * A `Pool` (or `GrowingPool`) of values that are not objects
 * (`Pool<Integer>`): its elements by slot, in a `Map` from a slot number to
 * the value, iterated as its values. A reference to an element is the pool
 * and its slot (`get`, `set`): `add` gives the slot, `slotOf` finds one,
 * `delete` and `take` take one. Printed, copied and compared by its tag.
 * @template T
 */
export class FaValuePool {
	/** @type {Map<number, T>} */
	#slots = new Map();
	#next = 0;

	/** @param {Iterable<T>} [values] */
	constructor(values) {
		if (values !== undefined) {
			for (const value of values) {
				this.add(value);
			}
		}
	}

	/** The tag that printing, copies and `==` recognize. */
	get [Symbol.toStringTag]() {
		return "fa:ValuePool";
	}

	/** @returns {number} */
	get size() {
		return this.#slots.size;
	}

	/**
	 * Stores a value.
	 * @param {T} value
	 * @returns {number} its slot
	 */
	add(value) {
		const slot = this.#next;
		this.#next += 1;
		this.#slots.set(slot, value);
		return slot;
	}

	/**
	 * Stores a value.
	 * @param {T} value
	 * @returns {T} the value
	 */
	addValue(value) {
		this.add(value);
		return value;
	}

	/**
	 * @param {number} slot
	 * @returns {T | undefined}
	 */
	get(slot) {
		return this.#slots.get(slot);
	}

	/**
	 * @param {number} slot
	 * @param {T} value
	 * @returns {this}
	 */
	set(slot, value) {
		this.#slots.set(slot, value);
		return this;
	}

	/**
	 * @param {number} slot
	 * @returns {boolean}
	 */
	delete(slot) {
		return this.#slots.delete(slot);
	}

	/**
	 * Removes an element.
	 * @param {number} slot
	 * @returns {T | undefined} its value
	 */
	take(slot) {
		const value = this.#slots.get(slot);
		this.#slots.delete(slot);
		return value;
	}

	/** @returns {void} */
	clear() {
		this.#slots.clear();
	}

	/**
	 * The slot of the first value the predicate accepts, or `undefined`.
	 * @param {(value: T) => boolean} predicate
	 * @returns {number | undefined}
	 */
	slotOf(predicate) {
		for (const [slot, value] of this.#slots) {
			if (predicate(value)) {
				return slot;
			}
		}
		return undefined;
	}

	/**
	 * @param {(value: T) => boolean} predicate
	 * @returns {void}
	 */
	deleteWhere(predicate) {
		for (const [slot, value] of this.#slots) {
			if (predicate(value)) {
				this.#slots.delete(slot);
			}
		}
	}

	/**
	 * A new pool of the values the predicate accepts.
	 * @param {(value: T) => boolean} predicate
	 * @returns {FaValuePool<T>}
	 */
	filter(predicate) {
		const kept = new FaValuePool();
		for (const value of this.#slots.values()) {
			if (predicate(value)) {
				kept.add(value);
			}
		}
		return kept;
	}

	/** @returns {FaValuePool<T>} */
	emptyCopy() {
		return new FaValuePool();
	}

	/** @returns {IterableIterator<T>} */
	values() {
		return this.#slots.values();
	}

	/** @returns {IterableIterator<T>} */
	[Symbol.iterator]() {
		return this.#slots.values();
	}
}
