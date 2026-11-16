import { expect, test } from "bun:test";

import { FaValuePool } from "./collections.js";
import { faAssign } from "./copy.js";
import { faMapAssign, faReferenceKey, faReferenceRead, faReferenceResult, faReferenceSet } from "./references.js";

test("container/key pairs forward primitive writes without carriers", () => {
	const values = [7, 7];
	for (let index = 0; index < 100; index++) {
		faReferenceSet(values, 0, index);
		expect(faReferenceRead(values, 0)).toBe(index);
	}
	expect(values).toEqual([99, 7]);
	const map = new Map([["one", 7]]);
	faReferenceSet(map, "one", 9);
	expect(faReferenceRead(map, "one")).toBe(9);
	expect(Object.hasOwn(faReferenceRead, "cache")).toBe(false);
});

test("pool locators retain exact slot identity among duplicate values", () => {
	const pool = new FaValuePool();
	const first = pool.add(7);
	const second = pool.add(7);
	faReferenceSet(pool, second, 9);
	expect(pool.get(first)).toBe(7);
	expect(pool.get(second)).toBe(9);
	pool.delete(second);
	const third = pool.add(11);
	expect(third).not.toBe(second);
	expect(faReferenceRead(pool, third)).toBe(11);
	expect(faReferenceRead(pool, second)).toBeUndefined();
});

test("object references remain bare objects with in-place writes", () => {
	const object = { value: 1 };
	const values = [object];
	const reference = values[0];
	expect(reference).toBe(object);
	faAssign(reference, { value: 9 });
	expect(object.value).toBe(9);
	expect(values[0]).toBe(reference);
});

test("absence uses the container and cannot collide with an optional slot or map key", () => {
	const values = [undefined];
	expect(values).not.toBeUndefined();
	expect(faReferenceRead(values, 0)).toBeUndefined();
	expect(faReferenceRead(undefined, 0)).toBeUndefined();
	faReferenceSet(values, 0, 3);
	expect(values[0]).toBe(3);
	const map = new Map([[undefined, 7]]);
	expect(faReferenceRead(map, undefined)).toBe(7);
	faReferenceSet(map, undefined, 9);
	expect(map.get(undefined)).toBe(9);
});

test("a mixed-union locator keeps the slot while its current value is an object", () => {
	const values = [{ value: 1 }];
	faReferenceSet(values, 0, 9);
	expect(values[0]).toBe(9);
	expect(faReferenceRead(values, 0)).toBe(9);
});

test("generic pool bodies can use ordinary object and primitive operations", () => {
	const object = { value: 1 };
	const objects = new Set();
	objects.add(object);
	expect([...objects][0]).toBe(object);
	const numbers = new FaValuePool();
	const slot = numbers.add(7);
	expect(numbers.slotOf(() => true)).toBe(slot);
	faReferenceSet(numbers, slot, 9);
	expect([...numbers]).toEqual([9]);
	expect(numbers.slotOf(() => false)).toBeUndefined();
});

test("general results are captured before another call can overwrite the scratch key", () => {
	const values = [{ left: 1, right: 2 }];
	const container = faReferenceResult(values[0], "left");
	const key = faReferenceKey;
	faReferenceResult(values[0], "right");
	faReferenceSet(container, key, 9);
	expect(values).toEqual([{ left: 9, right: 2 }]);
	expect(faReferenceRead(container, key)).toBe(9);
});

test("map replacement preserves existing object references and inserts new slots", () => {
 const node = { value: 7 };
 const values = new Map([["kept", node]]);
 faMapAssign(values, "kept", { value: 9 });
 expect(values.get("kept")).toBe(node);
 expect(node.value).toBe(9);
 faMapAssign(values,"new", { value: 11 });
 expect(values.get("new")).toEqual({value:11});
});
