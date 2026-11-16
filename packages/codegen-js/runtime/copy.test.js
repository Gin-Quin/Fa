import { describe, expect, test } from "bun:test";

import { FaHashMap, FaSet } from "./collections.js";
import { FaAtomic, FaChannel, FaMutex } from "./concurrency.js";
import { faAssign, faCopy, faCopyAs, faCopyGraph, faCopyInto } from "./copy.js";

describe("copies", () => {
	test("copied set items can be removed and changed independently", () => {
		for (const source of [new Set([{ items: [1] }]), new FaSet([{ items: [1] }])]) {
			const copied = faCopy(source);
			const item = copied.values().next().value;
			copied.delete(item);
			item.items.push(2);
			expect(source.values().next().value.items).toEqual([1]);
		}
	});

	test("copied structural map keys are complete before insertion", () => {
		const key = { items: [1] };
		const source = new FaHashMap([[key, [2]]]);
		const copied = faCopy(source);
		expect(copied.get({ items: [1] })).toEqual([2]);
		expect(copied.keys().next().value).not.toBe(key);
		expect(copied.keys().next().value.items).not.toBe(key.items);
	});

	test("converting plans copy deep recursive values and project only retained data", () => {
		/** @type {any} */
		let source;
		for (let index = 0; index < 100000; index++)
			source = { value: index, next: source, omitted: [index] };
		/** @type {(value: any, target: any, key: any, work: any[]) => void} */
		const plan = (value, target, key, work) => {
			if (value === undefined) {
				target[key] = undefined;
				return;
			}
			const result = { value: BigInt(value.value), next: undefined };
			target[key] = result;
			work.push([plan, value.next, result, "next"]);
		};
		const copied = faCopyAs(source, plan);
		let count = 0;
		for (
			let left = source, right = copied;
			left !== undefined;
			left = left.next, right = right.next
		) {
			if (left === right || right.value !== BigInt(left.value) || "omitted" in right)
				throw new Error("invalid converting copy");
			count++;
		}
		expect(count).toBe(100000);
		expect(typeof source.value).toBe("number");
	});

	test("converting plans finish a map key before hashing it", () => {
		const source = new Map([[{ id: 1 }, [2]]]);
		/** @type {(value: any, target: any, key: any, work: any[]) => void} */
		const keyPlan = (value, target, key, work) => {
			const result = { id: undefined };
			target[key] = result;
			work.push([
				(value, target, key) => {
					target[key] = BigInt(value);
				},
				value.id,
				result,
				"id",
			]);
		};
		const result = faCopyAs(source, (value, target, key, work) => {
			const result = new FaHashMap();
			target[key] = result;
			for (const [sourceKey, sourceValue] of value) {
				const entry = { key: undefined, value: sourceValue.slice() };
				work.push([() => result.set(entry.key, entry.value)]);
				work.push([keyPlan, sourceKey, entry, "key"]);
			}
		});
		expect(result.get({ id: 1n })).toEqual([2]);
		expect(result.get({ id: 1n })).not.toBe(source.values().next().value);
	});

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
