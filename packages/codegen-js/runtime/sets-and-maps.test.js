import { describe, expect, test } from "bun:test";

import {
	faAdd,
	faEvery,
	faFind,
	FaHashMap,
	faIterable,
	faMapEmptyLike,
	faMapFilter,
	faMapFind,
	faMapMap,
	FaRange,
	faRangeValues,
	faReduce,
	FaSet,
	faSetAdd,
	faSetFilter,
	faSome,
} from "./collections.js";
import { faParseFloat } from "./floats.js";
import { FaStop } from "./stops.js";

describe("pools", () => {
	test("add stores the element and returns it: the reference is the object", () => {
		const pool = new Set();
		const first = { name: "A" };
		const second = { name: "A" };
		expect(faAdd(pool, first)).toBe(first);
		expect(faAdd(pool, second)).toBe(second);
		expect(pool.size).toBe(2);
		const ordered = [];
		expect(faAdd(ordered, first)).toBe(first);
		expect(ordered).toEqual([first]);
	});
});

describe("sets", () => {
	test("add says whether the value was added (D37)", () => {
		const set = new Set([1]);
		expect(faSetAdd(set, 1)).toBe(false);
		expect(faSetAdd(set, 2)).toBe(true);
		const objects = new FaSet([{ a: 1 }]);
		expect(faSetAdd(objects, { a: 1 })).toBe(false);
		expect(faSetAdd(objects, { a: 2 })).toBe(true);
	});

	test("filter keeps the kind of the set", () => {
		const keyed = new FaSet([{ id: 1 }, { id: 2 }], { key: (value) => value.id });
		const filtered = faSetFilter(keyed, (value) => value.id > 1);
		expect(filtered).toBeInstanceOf(FaSet);
		expect([...filtered]).toEqual([{ id: 2 }]);
		expect(faSetFilter(new Set([1, 2, 3]), (value) => value % 2 === 1)).toEqual(new Set([1, 3]));
	});

	test("find, some, every and reduce walk the values", () => {
		const set = new Set([1, 2, 3]);
		expect(faFind(set, (value) => value > 1)).toBe(2);
		expect(faFind(set, (value) => value > 5)).toBeUndefined();
		expect(faSome(set, (value) => value === 3)).toBe(true);
		expect(faEvery(set, (value) => value > 1)).toBe(false);
		expect(faReduce(set, (total, value) => total + value, 10)).toBe(16);
	});
});

describe("maps", () => {
	const map = new Map([
		["a", 1],
		["b", 2],
	]);

	test("callbacks receive the key, then the value", () => {
		expect(faSome(map, (key, value) => key === "b" && value === 2, true)).toBe(true);
		expect(faEvery(map, (_key, value) => value > 1, true)).toBe(false);
		expect(faReduce(map, (total, key, value) => total + key + value, "", true)).toBe("a1b2");
		expect(faMapFind(map, (_key, value) => value === 2)).toEqual(["b", 2]);
		expect(faMapFind(map, () => false)).toBeUndefined();
	});

	test("filter and map give a map of the same kind", () => {
		expect(faMapFilter(map, (key) => key === "a")).toEqual(new Map([["a", 1]]));
		expect(faMapMap(map, (key, value) => `${key}${value}`)).toEqual(
			new Map([
				["a", "a1"],
				["b", "b2"],
			]),
		);
		const hashed = new FaHashMap([[[0, 0], "origin"]]);
		expect(faMapEmptyLike(hashed)).toBeInstanceOf(FaHashMap);
		expect(faMapMap(hashed, (_key, value) => value.length).get([0, 0])).toBe(6);
	});
});

describe("the sources of from", () => {
	test("a range gives its integers, a string its characters", () => {
		expect([...faIterable(new FaRange(1, 4, false))]).toEqual([1, 2, 3]);
		expect([...faIterable("ab")]).toEqual(["a", "b"]);
		const array = [1];
		expect(faIterable(array)).toBe(array);
	});

	test("a range without start stops a loop", () => {
		expect(() => faRangeValues(new FaRange(undefined, 3, false), "main.fa:1:1")).toThrow(FaStop);
	});
});

describe("Float.parse", () => {
	test("reads decimal floats", () => {
		expect(faParseFloat("2.5")).toBe(2.5);
		expect(faParseFloat("-1e3")).toBe(-1000);
		expect(faParseFloat(".5")).toBe(0.5);
		expect(faParseFloat("Infinity")).toBe(Infinity);
		expect(Number.isNaN(faParseFloat("NaN"))).toBe(true);
	});

	test("rejects what Number() would accept", () => {
		for (const text of ["", " 1", "0x10", "1_000", "1.5.2", "abc"]) {
			expect(() => faParseFloat(text)).toThrow();
		}
	});
});
