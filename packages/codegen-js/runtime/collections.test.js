import { describe, expect, test } from "bun:test";

import {
	faArrayDelete,
	faArrayDeleteWhere,
	faArrayGet,
	faArrayIncludes,
	faArrayIndexOf,
	faArrayInsert,
	faArraySlice,
	faArraySort,
	faArraySortInPlace,
	faArrayTake,
	faComparator,
	faDeleteWhere,
	FaHashMap,
	faIndex,
	faIndexSet,
	faInlineClear,
	faInlineDelete,
	faInlinePush,
	faMapGetOrInsert,
	faMapIndex,
	faMapTake,
	faNormalizeIndex,
	FaRange,
	faRangeIncludes,
	faRangeValues,
	FaSet,
	faSetDifference,
	faSetEmptyLike,
	faSetIntersection,
	faSetIsSubsetOf,
	faSetTake,
	faSetUnion,
	faSliceBounds,
	faSortFloats,
	faSortIntegers,
	faSortStrings,
	faSumIntegers,
	FaValuePool,
} from "./collections.js";
import { faCopy } from "./copy.js";
import { faDeepEquals } from "./equality.js";
import { CapacityError, IndexError, KeyNotFound } from "./errors.js";
import { faFormat } from "./format.js";
import { FaStop } from "./stops.js";

/**
 * The exception thrown by `run`.
 * @param {() => unknown} run
 * @returns {any}
 */
function errorOf(run) {
	try {
		run();
	} catch (exception) {
		return exception;
	}
	throw new Error("no exception");
}

describe("indexing", () => {
	test("negative indices count from the end", () => {
		const items = [10, 20, 30];
		expect(faIndex(items, 0)).toBe(10);
		expect(faIndex(items, 2)).toBe(30);
		expect(faIndex(items, -1)).toBe(30);
		expect(faIndex(items, -3)).toBe(10);
		expect(faNormalizeIndex(3, -2)).toBe(1);
	});

	test("out of bounds throws IndexError { index, length } (T-ERR-6)", () => {
		const items = [1, 2, 3];
		for (const index of [3, -4, 100, -100]) {
			const error = errorOf(() => faIndex(items, index));
			expect(error).toBeInstanceOf(IndexError);
			expect(error.index).toBe(index);
			expect(error.length).toBe(3);
		}
		expect(String(errorOf(() => faIndex([1], 2)))).toBe("IndexError { index = 2, length = 1 }");
		expect(errorOf(() => faIndex([], 0))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faIndex([], -1))).toBeInstanceOf(IndexError);
	});

	test("an IndexError is not a JavaScript Error (no stack capture)", () => {
		const error = errorOf(() => faIndex([], 0));
		expect(error instanceof Error).toBe(false);
		expect(error.stack).toBeUndefined();
	});

	test("writing (T-ERR-7)", () => {
		const xs = [1, 2];
		faIndexSet(xs, 1, 0);
		expect(errorOf(() => faIndexSet(xs, 5, 0))).toBeInstanceOf(IndexError);
		faIndexSet(xs, -2, 7);
		expect(xs).toEqual([7, 0]);
	});

	test("checked writes destroy the replaced or rejected owner once", () => {
		const first = { id: 1 };
		const next = { id: 2 };
		const rejected = { id: 3 };
		const items = [first];
		const destroyed = [];
		const drop = (value) => destroyed.push(value.id);
		faIndexSet(items, -1, next, drop);
		expect(items).toEqual([next]);
		expect(errorOf(() => faIndexSet(items, 9, rejected, drop))).toBeInstanceOf(IndexError);
		expect(items).toEqual([next]);
		expect(destroyed).toEqual([1, 3]);
	});

	test("get returns none (undefined)", () => {
		const items = ["a", "b"];
		expect(faArrayGet(items, 1)).toBe("b");
		expect(faArrayGet(items, -1)).toBe("b");
		expect(faArrayGet(items, -2)).toBe("a");
		expect(faArrayGet(items, 2)).toBeUndefined();
		expect(faArrayGet(items, -3)).toBeUndefined();
		expect(faArrayGet([], 0)).toBeUndefined();
	});
});

describe("slicing (D40)", () => {
	test("bounds", () => {
		expect(faSliceBounds(5, undefined, undefined, false)).toEqual([0, 5]);
		expect(faSliceBounds(5, 1, 3, false)).toEqual([1, 3]);
		expect(faSliceBounds(5, 1, 3, true)).toEqual([1, 4]);
		expect(faSliceBounds(5, -2, undefined, false)).toEqual([3, 5]);
		expect(faSliceBounds(5, 5, undefined, false)).toEqual([5, 5]);
		expect(faSliceBounds(5, -5, 5, false)).toEqual([0, 5]);
		expect(faSliceBounds(5, undefined, -1, true)).toEqual([0, 5]);
	});

	test("a start after the end is empty", () => {
		expect(faSliceBounds(5, 4, 2, false)).toEqual([4, 4]);
		expect(faArraySlice([1, 2, 3, 4, 5], 4, 2, false)).toEqual([]);
		expect(faArraySlice([1, 2, 3], 3, 3, false)).toEqual([]);
	});

	test("out of -length..=length throws IndexError", () => {
		const items = [1, 2, 3];
		expect(errorOf(() => faArraySlice(items, 4, undefined, false))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faArraySlice(items, -4, undefined, false))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faArraySlice(items, 0, 4, false))).toBeInstanceOf(IndexError);
		// an inclusive end must be an index
		expect(errorOf(() => faArraySlice(items, 0, 3, true))).toBeInstanceOf(IndexError);
		expect(faArraySlice(items, 0, 2, true)).toEqual([1, 2, 3]);
		const error = errorOf(() => faArraySlice([1, 2], 2, 5, false));
		expect(error.index).toBe(5);
		expect(error.length).toBe(2);
	});

	test("a slice is a new array, with copied items when asked", () => {
		const items = [{ x: 1 }, { x: 2 }];
		const shallow = faArraySlice(items, 0, undefined, false);
		expect(shallow).not.toBe(items);
		expect(shallow[0]).toBe(items[0]);
		const deep = faArraySlice(items, 0, 1, false, (item) => ({ x: item.x }));
		expect(deep).toEqual([{ x: 1 }]);
		expect(deep[0]).not.toBe(items[0]);
	});
});

describe("insert, delete, take, deleteWhere", () => {
	test("insert accepts 0..=length", () => {
		const items = [1, 2];
		faArrayInsert(items, 0, 0);
		faArrayInsert(items, 3, 3);
		faArrayInsert(items, 2, 9);
		expect(items).toEqual([0, 1, 9, 2, 3]);
		expect(errorOf(() => faArrayInsert(items, 6, 0))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faArrayInsert(items, -1, 0))).toBeInstanceOf(IndexError);
	});

	test("delete and take, negative indices included", () => {
		const items = ["a", "b", "c", "d"];
		faArrayDelete(items, 1);
		expect(items).toEqual(["a", "c", "d"]);
		expect(faArrayTake(items, -1)).toBe("d");
		expect(items).toEqual(["a", "c"]);
		expect(errorOf(() => faArrayDelete(items, 2))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faArrayTake(items, -3))).toBeInstanceOf(IndexError);
		expect(items).toEqual(["a", "c"]);
	});

	test("deleteWhere compacts in place", () => {
		const items = [1, 2, 3, 4, 5, 6];
		const same = items;
		faArrayDeleteWhere(items, (item) => item % 2 === 0);
		expect(items).toBe(same);
		expect(items).toEqual([1, 3, 5]);
		faArrayDeleteWhere(items, () => true);
		expect(items).toEqual([]);
	});
});

describe("sorting (D35)", () => {
	test("natural order never sorts numbers as strings", () => {
		const items = [10, 9, 1, -5, 100];
		faArraySortInPlace(items);
		expect(items).toEqual([-5, 1, 9, 10, 100]);
		expect(faArraySort([3n, -1n, 20n])).toEqual([-1n, 3n, 20n]);
	});

	test("strings by code point, tuples lexicographically", () => {
		expect(faArraySort(["b", "a", "\u{10000}", "￿", "B"])).toEqual([
			"B",
			"a",
			"b",
			"￿",
			"\u{10000}",
		]);
		expect(
			faArraySort([
				[2, "a"],
				[1, "b"],
				[1, "a"],
				[1, "a", 0],
			]),
		).toEqual([
			[1, "a"],
			[1, "a", 0],
			[1, "b"],
			[2, "a"],
		]);
	});

	test("sort is a copy, sortInPlace is in place", () => {
		const items = [3, 1, 2];
		expect(faArraySort(items)).toEqual([1, 2, 3]);
		expect(items).toEqual([3, 1, 2]);
		faArraySortInPlace(items, (a, b) => b - a);
		expect(items).toEqual([3, 2, 1]);
	});

	test("stable, with Float, bigint and NaN results of comparators", () => {
		const people = [
			{ name: "a", age: 30 },
			{ name: "b", age: 20 },
			{ name: "c", age: 30 },
			{ name: "d", age: 20 },
		];
		const sorted = faArraySort(people, (x, y) => x.age - y.age);
		expect(sorted.map((p) => p.name)).toEqual(["b", "d", "a", "c"]);
		const floats = faArraySort([0.5, 0.25, 0.75], (x, y) => (x - y) / 10);
		expect(floats).toEqual([0.25, 0.5, 0.75]);
		expect(faArraySort([3n, 1n, 2n], (x, y) => x - y)).toEqual([1n, 2n, 3n]);
		// NaN counts as 0: everything is equal, the order is kept
		expect(faArraySort([3, 1, 2], () => NaN)).toEqual([3, 1, 2]);
		expect(faComparator(() => -0.001)(1, 2)).toBe(-1);
		expect(faComparator(() => 5n)(1, 2)).toBe(1);
	});

	test("the merge sort is stable and agrees with Array.prototype.sort", () => {
		let state = 11;
		const next = () => (state = (state * 48271) % 2147483647);
		for (const length of [0, 1, 2, 15, 16, 17, 31, 32, 33, 100, 1000, 4097]) {
			for (const keys of [3, 1000]) {
				const items = Array.from({ length }, (_, index) => ({ key: next() % keys, index }));
				const expected = items.slice().sort((a, b) => a.key - b.key);
				const sorted = items.slice();
				faArraySortInPlace(sorted, (a, b) => BigInt(a.key - b.key));
				expect(sorted).toEqual(expected);
				const descending = items.slice();
				faArraySortInPlace(descending, (a, b) => (b.key - a.key) / 7);
				expect(descending).toEqual(items.slice().sort((a, b) => b.key - a.key));
			}
		}
		// already sorted, and reversed
		const ascending = Array.from({ length: 100 }, (_, index) => index);
		faArraySortInPlace(ascending, (a, b) => a - b);
		expect(ascending).toEqual(Array.from({ length: 100 }, (_, index) => index));
		faArraySortInPlace(ascending, (a, b) => b - a);
		expect(ascending).toEqual(Array.from({ length: 100 }, (_, index) => 99 - index));
	});

	test("a comparator that throws leaves the array as it was", () => {
		const items = Array.from({ length: 50 }, (_, index) => 50 - index);
		let calls = 0;
		expect(() =>
			faArraySortInPlace(items, (a, b) => {
				calls += 1;
				if (calls === 70) {
					throw new Error("stop");
				}
				return a - b;
			}),
		).toThrow("stop");
		expect(items).toEqual(Array.from({ length: 50 }, (_, index) => 50 - index));
	});

	test("direct natural sorts agree with faCompare", () => {
		let state = 7;
		const next = () => (state = (state * 48271) % 2147483647);
		const integers = Array.from({ length: 2000 }, () => (next() % 2001) - 1000);
		integers.push(9007199254740991, -9007199254740991, 0, 0);
		const expected = integers.slice();
		faArraySortInPlace(expected);
		expect(faSortIntegers(integers)).toBe(integers);
		expect(integers).toEqual(expected);
		const floats = [0.5, -0.25, Infinity, -Infinity, 1e-300, 3, -0, 0, 2.5];
		const expectedFloats = floats.slice();
		faArraySortInPlace(expectedFloats);
		expect(faSortFloats(floats)).toEqual(expectedFloats);
		const strings = ["b", "a", "\u{10000}", "\uffff", "B", "", "ab", "\ud800"];
		const expectedStrings = strings.slice();
		faArraySortInPlace(expectedStrings);
		expect(faSortStrings(strings)).toEqual(expectedStrings);
		expect(faSortIntegers([])).toEqual([]);
	});
});

describe("includes and indexOf use Fa equality (D-65)", () => {
	test("NaN is never found, -0 finds 0", () => {
		expect(faArrayIncludes([1, NaN], NaN)).toBe(false);
		expect(faArrayIncludes([0], -0)).toBe(true);
		expect(faArrayIndexOf([NaN, 2], NaN)).toBeUndefined();
		expect(faArrayIndexOf([NaN, 2], 2)).toBe(1);
	});

	test("objects and tuples compare deeply", () => {
		const points = [
			{ x: 1, y: 2 },
			{ x: 3, y: 4 },
		];
		expect(faArrayIncludes(points, { x: 3, y: 4 })).toBe(true);
		expect(faArrayIncludes(points, { x: 3, y: 5 })).toBe(false);
		expect(faArrayIndexOf(points, { x: 3, y: 4 })).toBe(1);
		expect(faArrayIncludes([[1, "a"]], [1, "a"])).toBe(true);
		expect(faArrayIncludes([[NaN]], [NaN])).toBe(false);
	});

	test("a generated equality", () => {
		const byId = (/** @type {any} */ a, /** @type {any} */ b) => a.id === b.id;
		expect(faArrayIncludes([{ id: 1, n: "a" }], { id: 1, n: "b" }, byId)).toBe(true);
		expect(faArrayIndexOf([{ id: 2 }, { id: 1 }], { id: 1 }, byId)).toBe(1);
	});
});

describe("sum", () => {
	test("checks every partial sum at ±(2^53 − 1)", () => {
		expect(faSumIntegers([])).toBe(0);
		expect(faSumIntegers([1, 2, 3])).toBe(6);
		expect(faSumIntegers([9007199254740990, 1])).toBe(9007199254740991);
		expect(errorOf(() => faSumIntegers([9007199254740991, 1]))).toBeInstanceOf(FaStop);
		expect(errorOf(() => faSumIntegers([-9007199254740991, -1]))).toBeInstanceOf(FaStop);
	});
});

describe("ranges", () => {
	test("includes", () => {
		expect(faRangeIncludes(new FaRange(1, 3, false), 2)).toBe(true);
		expect(faRangeIncludes(new FaRange(1, 3, false), 3)).toBe(false);
		expect(faRangeIncludes(new FaRange(1, 3, true), 3)).toBe(true);
		expect(faRangeIncludes(new FaRange(1, 3, false), 0)).toBe(false);
		expect(faRangeIncludes(new FaRange(5, undefined, false), 1e15)).toBe(true);
		expect(faRangeIncludes(new FaRange(undefined, 10, false), -1e15)).toBe(true);
		expect(faRangeIncludes(new FaRange(undefined, undefined, false), 0)).toBe(true);
	});

	test("values", () => {
		expect([...faRangeValues(new FaRange(1, 4, false))]).toEqual([1, 2, 3]);
		expect([...faRangeValues(new FaRange(1, 4, true))]).toEqual([1, 2, 3, 4]);
		expect([...faRangeValues(new FaRange(4, 1, false))]).toEqual([]);
		expect([...faRangeValues(new FaRange(4, 4, true))]).toEqual([4]);
		expect([...faRangeValues(new FaRange(-2, 0, false))]).toEqual([-2, -1]);
	});

	test("an inclusive end at the maximum terminates", () => {
		const max = 9007199254740991;
		expect([...faRangeValues(new FaRange(max - 2, max, true))]).toEqual([max - 2, max - 1, max]);
	});

	test("an open range stops past the maximum", () => {
		const max = 9007199254740991;
		const values = faRangeValues(new FaRange(max - 1, undefined, false), "main.fa:1:1");
		expect(values.next().value).toBe(max - 1);
		expect(values.next().value).toBe(max);
		const stop = errorOf(() => values.next());
		expect(stop).toBeInstanceOf(FaStop);
		expect(stop.location).toBe("main.fa:1:1");
	});
});

describe("FaHashMap", () => {
	test("keys compare structurally", () => {
		const map = new FaHashMap();
		map.set({ x: 1, y: 2 }, "a");
		map.set([1, "b"], "tuple");
		expect(map.get({ x: 1, y: 2 })).toBe("a");
		expect(map.get({ y: 2, x: 1 })).toBe("a");
		expect(map.has({ x: 1, y: 3 })).toBe(false);
		expect(map.get([1, "b"])).toBe("tuple");
		map.set({ x: 1, y: 2 }, "b");
		expect(map.size).toBe(2);
		expect(map.get({ x: 1, y: 2 })).toBe("b");
	});

	test("floats in keys use SameValueZero", () => {
		const map = new FaHashMap();
		map.set([NaN], 1);
		map.set([-0], 2);
		expect(map.get([NaN])).toBe(1);
		expect(map.get([0])).toBe(2);
		expect(map.size).toBe(2);
		map.set(NaN, 3);
		expect(map.get(NaN)).toBe(3);
	});

	test("keeps the insertion order, primitive keys included", () => {
		const map = new FaHashMap([
			[[2], "two"],
			[1, "one"],
			[[3], "three"],
		]);
		map.set([2], "TWO");
		expect([...map]).toEqual([
			[[2], "TWO"],
			[1, "one"],
			[[3], "three"],
		]);
		map.delete([2]);
		map.set([2], "again");
		expect([...map.keys()]).toEqual([1, [3], [2]]);
		expect([...map.values()]).toEqual(["one", "three", "again"]);
	});

	test("collisions", () => {
		const map = new FaHashMap(undefined, () => 7);
		for (let index = 0; index < 5; index += 1) {
			map.set({ index }, index);
		}
		expect(map.size).toBe(5);
		expect(map.get({ index: 3 })).toBe(3);
		expect(map.delete({ index: 3 })).toBe(true);
		expect(map.delete({ index: 3 })).toBe(false);
		expect(map.has({ index: 3 })).toBe(false);
		expect(map.size).toBe(4);
		for (const index of [0, 1, 2]) {
			map.delete({ index });
		}
		expect([...map]).toEqual([[{ index: 4 }, 4]]);
		expect(map.get({ index: 4 })).toBe(4);
		map.clear();
		expect(map.size).toBe(0);
		expect(map.get({ index: 4 })).toBeUndefined();
	});

	test("has the JavaScript Map interface", () => {
		const map = new FaHashMap([[{ a: 1 }, 1]]);
		/** @type {unknown[]} */
		const seen = [];
		map.forEach((value, key, self) => seen.push(value, key, self === map));
		expect(seen).toEqual([1, { a: 1 }, true]);
		expect([...map.entries()]).toEqual([[{ a: 1 }, 1]]);
		expect(new Map(map).size).toBe(1);
	});
});

describe("map helpers", () => {
	test("map[key] throws KeyNotFound with the key's toString()", () => {
		const scores = new Map([["ana", 3]]);
		expect(faMapIndex(scores, "ana")).toBe(3);
		const error = errorOf(() => faMapIndex(scores, "bao"));
		expect(error).toBeInstanceOf(KeyNotFound);
		expect(error.key).toBe("bao");
		expect(String(error)).toBe('KeyNotFound { key = "bao" }');
		const byPoint = new FaHashMap([[{ x: 1 }, 1]]);
		expect(errorOf(() => faMapIndex(byPoint, { x: 2 })).key).toBe("{ x = 2 }");
		expect(faMapIndex(new Map([[1, undefined]]), 1)).toBeUndefined();
	});

	test("getOrInsert and take (D37)", () => {
		const counts = new Map();
		expect(faMapGetOrInsert(counts, "a", 0)).toBe(0);
		counts.set("a", counts.get("a") + 1);
		expect(faMapGetOrInsert(counts, "a", 0)).toBe(1);
		expect(faMapTake(counts, "a")).toBe(1);
		expect(counts.size).toBe(0);
		expect(faMapTake(counts, "a")).toBeUndefined();
		const hashed = new FaHashMap([[[1, 2], "x"]]);
		expect(faMapTake(hashed, [1, 2])).toBe("x");
		expect(hashed.size).toBe(0);
	});

	test("deleteWhere on maps and sets", () => {
		const map = new Map([
			["a", 1],
			["b", 2],
			["c", 3],
		]);
		faDeleteWhere(map, (_key, value) => value % 2 === 1);
		expect([...map]).toEqual([["b", 2]]);
		const set = new FaSet([{ n: 1 }, { n: 2 }]);
		faDeleteWhere(set, (value) => value.n === 1);
		expect([...set]).toEqual([{ n: 2 }]);
		const plain = new Set([1, 2, 3]);
		faDeleteWhere(plain, (value) => value > 1);
		expect([...plain]).toEqual([1]);
	});
});

describe("FaSet (D33)", () => {
	test("elements compare structurally", () => {
		const set = new FaSet([{ x: 1 }, { x: 1 }, { x: 2 }]);
		expect(set.size).toBe(2);
		expect(set.has({ x: 1 })).toBe(true);
		expect(set.has({ x: 3 })).toBe(false);
		expect(set.delete({ x: 1 })).toBe(true);
		expect(set.delete({ x: 1 })).toBe(false);
		expect([...set]).toEqual([{ x: 2 }]);
	});

	test("the first equal element is kept", () => {
		const first = { id: 1, name: "first" };
		const set = new FaSet([first, { id: 1, name: "second" }], { key: (user) => user.id });
		expect(set.size).toBe(1);
		expect([...set][0]).toBe(first);
		expect(set.has({ id: 1, name: "other" })).toBe(true);
		expect(faSetTake(set, { id: 1, name: "" })).toBe(first);
		expect(set.size).toBe(0);
		expect(faSetTake(set, { id: 1, name: "" })).toBeUndefined();
	});

	test("has the JavaScript Set interface", () => {
		const set = new FaSet([[1, 2]]);
		expect([...set.values()]).toEqual([[1, 2]]);
		expect([...set.keys()]).toEqual([[1, 2]]);
		expect([...set.entries()]).toEqual([
			[
				[1, 2],
				[1, 2],
			],
		]);
		/** @type {unknown[]} */
		const seen = [];
		set.forEach((value, same) => seen.push(value === same));
		expect(seen).toEqual([true]);
		set.clear();
		expect(set.size).toBe(0);
	});

	test("an empty set of the same kind keeps the key function", () => {
		const set = new FaSet([], { key: (/** @type {string} */ text) => text.toLowerCase() });
		const empty = /** @type {FaSet<string>} */ (faSetEmptyLike(set));
		empty.add("A");
		expect(empty.has("a")).toBe(true);
		expect(faSetEmptyLike(new Set([1]))).toEqual(new Set());
	});

	test("set operations keep the receiver's kind", () => {
		const a = new Set([1, 2, 3]);
		const b = new Set([2, 3, 4]);
		expect([...faSetUnion(a, b)]).toEqual([1, 2, 3, 4]);
		expect([...faSetIntersection(a, b)]).toEqual([2, 3]);
		expect([...faSetDifference(a, b)]).toEqual([1]);
		expect(faSetIsSubsetOf(new Set([2, 3]), a)).toBe(true);
		expect(faSetIsSubsetOf(a, b)).toBe(false);
		const caseless = new FaSet(["A", "b"], { key: (/** @type {string} */ t) => t.toLowerCase() });
		const other = new Set(["a", "C"]);
		expect([...faSetIntersection(caseless, other)]).toEqual(["A"]);
		expect([...faSetDifference(caseless, other)]).toEqual(["b"]);
		expect(faSetUnion(caseless, other).size).toBe(3);
		expect(faSetUnion(caseless, other)).toBeInstanceOf(FaSet);
		expect(
			faSetIsSubsetOf(
				new FaSet(["B"], { key: (/** @type {string} */ t) => t.toLowerCase() }),
				caseless,
			),
		).toBe(true);
	});
});

describe("FaValuePool", () => {
	test("elements have slots: references read and write them, removals take them", () => {
		const pool = new FaValuePool([3, 7, 7]);
		const slot = pool.slotOf((value) => value === 7);
		expect(slot).toBe(1);
		pool.set(slot, 8);
		const added = pool.add(5);
		expect([...pool]).toEqual([3, 8, 7, 5]);
		expect(pool.take(added)).toBe(5);
		expect(pool.delete(slot)).toBe(true);
		expect(pool.slotOf((value) => value > 100)).toBeUndefined();
		pool.deleteWhere((value) => value === 3);
		expect([...pool]).toEqual([7]);
		expect(pool.size).toBe(1);
	});

	test("is printed, copied and compared by its values", () => {
		const pool = new FaValuePool(["a", "b"]);
		expect(faFormat(pool)).toBe('["a", "b"]');
		const copy = faCopy(pool);
		expect(copy).toBeInstanceOf(FaValuePool);
		expect(faDeepEquals(copy, pool)).toBe(true);
		copy.add("c");
		expect(pool.size).toBe(2);
		expect(faDeepEquals(copy, pool)).toBe(false);
		expect([...pool.filter((value) => value === "b")]).toEqual(["b"]);
	});
});

describe("inline arrays", () => {
	test("append preserves identities, order and duplicates; overflow leaves the prefix unchanged", () => {
		const first = { value: 1 };
		const values = [first];
		faInlinePush(values, 3, first);
		faInlinePush(values, 3, { value: 2 });
		expect(values[0]).toBe(first);
		expect(values[1]).toBe(first);
		expect(errorOf(() => faInlinePush(values, 3, { value: 3 }))).toBeInstanceOf(CapacityError);
		expect(values).toEqual([first, first, { value: 2 }]);
		expect(errorOf(() => faInlinePush([], 0, first))).toBeInstanceOf(CapacityError);
	});

	test("failed append and removal destroy only the owners they remove", () => {
		const dropped = [];
		const drop = (item) => dropped.push(item);
		const values = ["kept", "deleted"];
		expect(errorOf(() => faInlinePush(values, 2, "overflow", drop))).toBeInstanceOf(CapacityError);
		faInlineDelete(values, 1, drop);
		faInlineClear(values, drop);
		expect(values).toEqual([]);
		expect(dropped).toEqual(["overflow", "deleted", "kept"]);
	});
});
