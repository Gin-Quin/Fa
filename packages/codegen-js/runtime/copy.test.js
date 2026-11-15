import { describe, expect, test } from "bun:test";

import { FaHashMap } from "./collections.js";
import { FaAtomic, FaChannel, FaMutex } from "./concurrency.js";
import { faAssign, faCopy, faCopyGraph, faCopyInto } from "./copy.js";

describe("copies", () => {
	test("structural and relation-graph copies retain shared synchronization handles", () => {
		const values = [new FaAtomic(0), new FaMutex({ value: 1 }), new FaChannel(1)];
		const source = { values };
		for (const copy of [faCopy(source), faCopyGraph(source)]) {
			expect(copy).not.toBe(source);
			expect(copy.values).not.toBe(values);
			for (let index = 0; index < values.length; index++)
				expect(copy.values[index]).toBe(values[index]);
		}
	});
	test("faCopy copies nested objects and collections", () => {
		const value = { name: "A", pets: [{ name: "Rex" }], tags: new Set(["x"]) };
		const copy = faCopy(value);
		expect(copy).toEqual(value);
		expect(copy.pets).not.toBe(value.pets);
		expect(copy.pets[0]).not.toBe(value.pets[0]);
	});

	test("copies and in-place writes traverse deep recursive values iteratively", () => {
		/** @type {any} */
		let source;
		for (let index = 0; index < 100000; index++) source = { value: index, next: source };
		const copy = faCopy(source);
		const nested = copy.next;
		faAssign(copy, source);
		expect(copy.next).toBe(nested);
		let count = 0;
		for (
			let left = source, right = copy;
			left !== undefined;
			left = left.next, right = right.next
		) {
			if (right === left) throw new Error("shared a copied node");
			if (right.value !== left.value) throw new Error("copied a wrong field");
			count++;
		}
		expect(count).toBe(100000);
	});

	test("faCopyInto reuses the target's storage when the lengths match", () => {
		const target = [1, 2];
		expect(faCopyInto(target, [3, 4])).toBe(target);
		expect(target).toEqual([3, 4]);
		const longer = faCopyInto(target, [5, 6, 7]);
		expect(longer).not.toBe(target);
		expect(longer).toEqual([5, 6, 7]);
	});
});

describe("in-place writes (D60)", () => {
	test("an object keeps its identity, and its nested objects of the same kind", () => {
		const pet = { name: "Rex" };
		const target = { name: "Ana", pet, pets: [pet] };
		const source = { name: "Bao", pet: { name: "Max" }, pets: [{ name: "Fido" }] };
		expect(faAssign(target, source)).toBe(target);
		expect(target.name).toBe("Bao");
		expect(target.pet).toBe(pet);
		expect(pet.name).toBe("Max");
		expect(target.pets[0]).not.toBe(source.pets[0]);
		expect(target.pets[0].name).toBe("Fido");
	});

	test("a union member of another tag is replaced by a copy", () => {
		const target = { shape: { $t: "Shape.Circle", radius: 1 } };
		const square = { $t: "Shape.Square", side: 2 };
		faAssign(target, { shape: square });
		expect(target.shape).toEqual(square);
		expect(target.shape).not.toBe(square);
	});

	test("collections get their contents replaced", () => {
		const array = [1, 2, 3];
		faAssign(array, [4]);
		expect(array).toEqual([4]);
		const map = new Map([["a", 1]]);
		faAssign(map, new Map([["b", 2]]));
		expect([...map]).toEqual([["b", 2]]);
		const hashed = new FaHashMap([[{ k: 1 }, 1]]);
		faAssign(hashed, new FaHashMap([[{ k: 2 }, 2]]));
		expect(hashed.size).toBe(1);
		const set = new Set([1]);
		faAssign(set, new Set([2, 3]));
		expect([...set]).toEqual([2, 3]);
		expect(faAssign(set, set)).toBe(set);
	});
});
