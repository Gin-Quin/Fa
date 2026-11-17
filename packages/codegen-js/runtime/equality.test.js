import { describe, expect, test } from "bun:test";

import { FaHashMap, FaRange, FaSet } from "./collections.js";
import { faAssign, faCopy, faCopyInto } from "./copy.js";
import {
	faCompare,
	faDeepEquals,
	faEquals,
	faHash,
	faHashNumber,
	faHashString,
	faKeyEquals,
} from "./equality.js";
import { ArithmeticError, DivisionByZero, IndexError } from "./errors.js";

describe("equality (values brief section 13, types brief D17, D63)", () => {
	test("primitives", () => {
		expect(faEquals(1, 1)).toBe(true);
		expect(faEquals(1, 2)).toBe(false);
		expect(faEquals("é", "é")).toBe(false); // no normalization (D-43)
		expect(faEquals(true, true)).toBe(true);
		expect(faEquals(undefined, undefined)).toBe(true);
		expect(faEquals(undefined, 0)).toBe(false);
		expect(faEquals(0, "0")).toBe(false);
	});

	test("floats: IEEE for ==, SameValueZero for keys", () => {
		expect(faEquals(NaN, NaN)).toBe(false);
		expect(faEquals(-0, 0)).toBe(true);
		expect(faEquals([NaN], [NaN])).toBe(false);
		expect(faEquals({ x: NaN }, { x: NaN })).toBe(false);
		expect(faKeyEquals(NaN, NaN)).toBe(true);
		expect(faKeyEquals([NaN, -0], [NaN, 0])).toBe(true);
		expect(faKeyEquals({ x: NaN }, { x: NaN })).toBe(true);
		expect(faKeyEquals(NaN, 1)).toBe(false);
	});

	test("nested set and map hashes follow complete values independent of order", () => {
		const a = new FaSet([{ n: 1 }, { n: 2 }]);
		const b = new FaSet([{ n: 2 }, { n: 1 }]);
		expect(faKeyEquals(a, b)).toBe(true);
		expect(faHash(a)).toBe(faHash(b));
		const sets = new FaSet([a, b]);
		expect(sets.size).toBe(1);
		expect(sets.has(new FaSet([{ n: 1 }, { n: 3 }]))).toBe(false);
		const maps = new FaSet([
			new Map([
				[1, "a"],
				[2, "b"],
			]),
			new Map([
				[2, "b"],
				[1, "a"],
			]),
		]);
		expect(maps.size).toBe(1);
		expect(
			maps.delete(
				new Map([
					[1, "a"],
					[2, "b"],
				]),
			),
		).toBe(true);
	});

	test("bigints equal numbers of the same value (D-69)", () => {
		expect(faEquals(5n, 5)).toBe(true);
		expect(faEquals(5, 5n)).toBe(true);
		expect(faEquals(5n, 6)).toBe(false);
		expect(faEquals(2n ** 60n, 2 ** 60)).toBe(true);
		expect(faEquals(9007199254740993n, 9007199254740992)).toBe(false);
		expect(faEquals(5n, "5")).toBe(false);
		expect(faEquals([1n], [1])).toBe(true);
	});

	test("arrays and tuples, item by item", () => {
		expect(faEquals([1, [2, 3]], [1, [2, 3]])).toBe(true);
		expect(faEquals([1, 2], [1, 2, 3])).toBe(false);
		expect(faEquals([1, 2], [2, 1])).toBe(false);
		expect(faEquals([], [])).toBe(true);
		expect(faEquals([1], { 0: 1 })).toBe(false);
	});

	test("objects, whatever the property order", () => {
		expect(faEquals({ x: 1, y: { z: "a" } }, { y: { z: "a" }, x: 1 })).toBe(true);
		expect(faEquals({ x: 1 }, { x: 1, y: 2 })).toBe(false);
		expect(faEquals({ x: 1, y: 2 }, { x: 1 })).toBe(false);
		expect(faEquals({}, {})).toBe(true);
	});

	test("an absent optional field equals a none field", () => {
		expect(faEquals({ x: 1, name: undefined }, { x: 1 })).toBe(true);
		expect(faEquals({ x: 1 }, { x: 1, name: undefined })).toBe(true);
		expect(faEquals({ x: 1, name: "a" }, { x: 1 })).toBe(false);
		expect(faHash({ x: 1, name: undefined })).toBe(faHash({ x: 1 }));
	});

	test("union members by tag", () => {
		expect(faEquals({ $t: 1, radius: 2 }, { $t: 1, radius: 2 })).toBe(true);
		expect(faEquals({ $t: 1, radius: 2 }, { $t: 2, radius: 2 })).toBe(false);
		expect(faEquals({ $t: 3, v: 12 }, { $t: 3, v: 12 })).toBe(true);
	});

	test("maps by keys and values, sets by elements, without order", () => {
		expect(
			faEquals(
				new Map([
					["a", 1],
					["b", 2],
				]),
				new Map([
					["b", 2],
					["a", 1],
				]),
			),
		).toBe(true);
		expect(faEquals(new Map([["a", 1]]), new Map([["a", 2]]))).toBe(false);
		expect(faEquals(new Map([["a", 1]]), new Map([["b", 1]]))).toBe(false);
		expect(faEquals(new Set([1, 2]), new Set([2, 1]))).toBe(true);
		expect(faEquals(new Set([1, 2]), new Set([1, 3]))).toBe(false);
		expect(faEquals(new Set([1]), new Map([[1, 1]]))).toBe(false);
		const a = new FaHashMap([
			[{ k: 1 }, [1]],
			[{ k: 2 }, [2]],
		]);
		const b = new FaHashMap([
			[{ k: 2 }, [2]],
			[{ k: 1 }, [1]],
		]);
		expect(faEquals(a, b)).toBe(true);
		b.set({ k: 1 }, [3]);
		expect(faEquals(a, b)).toBe(false);
		expect(faEquals(new FaSet([{ x: 1 }, { x: 2 }]), new FaSet([{ x: 2 }, { x: 1 }]))).toBe(true);
		expect(faEquals(new FaSet([{ x: 1 }]), new FaSet([{ x: 2 }]))).toBe(false);
	});

	test("ranges and errors only equal their own kind", () => {
		expect(faEquals(new FaRange(1, 3, false), new FaRange(1, 3, false))).toBe(true);
		expect(faEquals(new FaRange(1, 3, false), new FaRange(1, 3, true))).toBe(false);
		expect(faEquals(new FaRange(1, 3, false), { start: 1, end: 3, includesEnd: false })).toBe(
			false,
		);
		expect(faEquals(new IndexError(1, 2), new IndexError(1, 2))).toBe(true);
		expect(faEquals(new IndexError(1, 2), new IndexError(1, 3))).toBe(false);
		expect(faEquals(new DivisionByZero(), new DivisionByZero())).toBe(true);
		expect(faEquals(new ArithmeticError("a"), new DivisionByZero())).toBe(false);
	});

	test("long recursive chains don't overflow the stack", () => {
		/** @type {any} */
		let a = undefined;
		/** @type {any} */
		let b = undefined;
		for (let index = 0; index < 200_000; index += 1) {
			a = { value: index, next: a };
			b = { value: index, next: b };
		}
		expect(faEquals(a, b)).toBe(true);
		b.next.next.value = -1;
		expect(faEquals(a, b)).toBe(false);
		expect(faDeepEquals(faCopy(a), a, false)).toBe(true);
	});
});

describe("collections compare by value, never by identity", () => {
	test("a pool of objects (a Set of them): its elements in order", () => {
		const a = new Set([{ name: "ant" }, { name: "cat" }]);
		const b = new Set([{ name: "ant" }, { name: "cat" }]);
		expect(faEquals(a, b)).toBe(true);
		expect(faEquals(a, new Set([{ name: "ant" }, { name: "dog" }]))).toBe(false);
		expect(faEquals(new Set([1, 2]), new Set([2, 1]))).toBe(true);
	});
});

describe("hashing", () => {
	test("is a 32-bit unsigned integer", () => {
		for (const value of [0, -1, 1.5, NaN, "a", "", true, [1], { a: 1 }, 2n ** 100n, undefined]) {
			const hash = faHash(value);
			expect(Number.isInteger(hash) && hash >= 0 && hash < 2 ** 32).toBe(true);
		}
	});

	test("equal keys hash the same", () => {
		expect(faHashNumber(-0)).toBe(faHashNumber(0));
		expect(faHashNumber(NaN)).toBe(faHashNumber(-NaN));
		expect(faHash(5n)).toBe(faHash(5));
		expect(faHash(-(2n ** 40n))).toBe(faHash(-(2 ** 40)));
		expect(faHash({ x: 1, y: [2, "a"] })).toBe(faHash({ y: [2, "a"], x: 1 }));
		expect(faHash(new Set([1, 2, 3]))).toBe(faHash(new Set([3, 2, 1])));
		expect(
			faHash(
				new Map([
					["a", 1],
					["b", 2],
				]),
			),
		).toBe(
			faHash(
				new Map([
					["b", 2],
					["a", 1],
				]),
			),
		);
		expect(faHash(new FaSet([{ a: 1 }, { a: 2 }]))).toBe(faHash(new FaSet([{ a: 2 }, { a: 1 }])));
	});

	test("spreads usual keys", () => {
		const hashes = new Set();
		for (let index = 0; index < 10_000; index += 1) {
			hashes.add(faHash([index, index * 2]));
		}
		expect(hashes.size).toBeGreaterThan(9_990);
		expect(faHash([1, 2])).not.toBe(faHash([2, 1]));
		expect(faHashString("ab")).not.toBe(faHashString("ba"));
		expect(faHash(1)).not.toBe(faHash(2));
		expect(faHash(2 ** 32)).not.toBe(faHash(0));
	});
});

describe("natural order (D35)", () => {
	test("numbers, bigints, strings, tuples", () => {
		expect(faCompare(1, 2)).toBe(-1);
		expect(faCompare(2, 1)).toBe(1);
		expect(faCompare(2, 2)).toBe(0);
		expect(faCompare(NaN, 1)).toBe(0);
		expect(faCompare(1n, 2n)).toBe(-1);
		expect(faCompare("￿", "\u{10000}")).toBe(1);
		expect(faCompare([1, "b"], [1, "a"])).toBe(1);
		expect(faCompare([1], [1, 0])).toBeLessThan(0);
	});
});

describe("copies (types brief 3.2.7, D60, values brief D-81)", () => {
	test("deep, with the source's property order", () => {
		const source = { name: "Ana", position: { x: 1, y: 2 }, tags: ["a"], score: 3 };
		const copy = faCopy(source);
		expect(copy).toEqual(source);
		expect(Object.keys(copy)).toEqual(["name", "position", "tags", "score"]);
		expect(copy.position).not.toBe(source.position);
		expect(copy.tags).not.toBe(source.tags);
		copy.position.x = 9;
		expect(source.position.x).toBe(1);
	});

	test("primitives and functions are shared", () => {
		const f = () => 1;
		expect(faCopy(3)).toBe(3);
		expect(faCopy("a")).toBe("a");
		expect(faCopy(undefined)).toBeUndefined();
		expect(faCopy({ f }).f).toBe(f);
	});

	test("collections keep their kind; keys and elements are shared", () => {
		const key = { k: 1 };
		const map = new FaHashMap([[key, { v: 1 }]]);
		const copied = faCopy(map);
		expect(copied).toBeInstanceOf(FaHashMap);
		expect([...copied.keys()][0]).toBe(key);
		expect(copied.get({ k: 1 })).toEqual({ v: 1 });
		expect(copied.get({ k: 1 })).not.toBe(map.get(key));
		const set = new FaSet([{ name: "A" }]);
		const setCopy = faCopy(set);
		expect(setCopy).toBeInstanceOf(FaSet);
		expect(setCopy.has({ name: "A" })).toBe(true);
		const plain = faCopy(new Map([["a", [1]]]));
		expect(plain).toBeInstanceOf(Map);
		expect(plain.get("a")).toEqual([1]);
		expect(faCopy(new Set([1, 2]))).toEqual(new Set([1, 2]));
		const range = faCopy(new FaRange(1, 2, false));
		expect(range).toBeInstanceOf(FaRange);
		expect(range).toEqual(new FaRange(1, 2, false));
	});

	test("an error keeps its class", () => {
		const error = faCopy(new IndexError(3, 1));
		expect(error).toBeInstanceOf(IndexError);
		expect(String(error)).toBe("IndexError { index = 3, length = 1 }");
	});

	test("long chains don't overflow the stack", () => {
		/** @type {any} */
		let list = undefined;
		for (let index = 0; index < 200_000; index += 1) {
			list = { value: index, next: list };
		}
		const copy = faCopy(list);
		expect(copy.value).toBe(199_999);
		expect(copy.next).not.toBe(list.next);
	});
});

describe("in-place writes (D60)", () => {
	test("a live reference sees the new value", () => {
		const users = [{ name: "Ana", address: { city: "Paris" } }];
		const reference = users[0];
		const address = reference.address;
		faAssign(users[0], { name: "Bao", address: { city: "Lyon" } });
		expect(reference.name).toBe("Bao");
		expect(address.city).toBe("Lyon");
		expect(users[0]).toBe(reference);
	});

	test("the source is copied, not shared", () => {
		const target = { items: [1], point: { x: 1 } };
		const source = { items: [2, 3], point: { x: 2 } };
		faAssign(target, source);
		source.items.push(4);
		source.point.x = 5;
		expect(target).toEqual({ items: [2, 3], point: { x: 2 } });
	});

	test("a union field changing member is replaced", () => {
		const target = { shape: { $t: 1, radius: 1 } };
		const circle = target.shape;
		faAssign(target, { shape: { $t: 2, width: 3 } });
		expect(target.shape).toEqual({ $t: 2, width: 3 });
		expect(circle).toEqual({ $t: 1, radius: 1 });
	});

	test("collections get their contents replaced", () => {
		const array = [1, 2, 3];
		expect(faAssign(array, [4])).toBe(array);
		expect(array).toEqual([4]);
		const map = new Map([["a", 1]]);
		faAssign(map, new Map([["b", 2]]));
		expect([...map]).toEqual([["b", 2]]);
		const set = new FaSet([{ a: 1 }]);
		faAssign(set, new FaSet([{ a: 2 }]));
		expect([...set]).toEqual([{ a: 2 }]);
		const same = { x: 1 };
		expect(faAssign(same, same)).toBe(same);
	});
});

describe("faCopyInto", () => {
	test("reuses the target when the lengths match", () => {
		const target = [1, 2, 3];
		const source = [4, 5, 6];
		expect(faCopyInto(target, source)).toBe(target);
		expect(target).toEqual([4, 5, 6]);
		source[0] = 9;
		expect(target[0]).toBe(4);
		expect(faCopyInto(target, target)).toBe(target);
	});

	test("copies the source otherwise", () => {
		const source = [1, 2];
		for (const target of [[0], [0, 0, 0], [], undefined]) {
			const result = faCopyInto(/** @type {number[]} */ (target), source);
			expect(result).toEqual([1, 2]);
			expect(result).not.toBe(source);
			expect(result).not.toBe(target);
		}
	});
});
