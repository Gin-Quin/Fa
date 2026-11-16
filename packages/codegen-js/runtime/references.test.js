import { expect, test } from "bun:test";

import { FaValuePool } from "./collections.js";
import { faBorrowSlot, faReferenceValue, faReferenceWrite } from "./references.js";

test("primitive slot carriers are reused across calls and forward writes", () => {
	const values = [7, 7];
	const reference = faBorrowSlot(values, 0, false);
	for (let index = 0; index < 100; index++) {
		expect(faBorrowSlot(values, 0, false)).toBe(reference);
		faReferenceWrite(reference, index);
	}
	expect(values).toEqual([99, 7]);
	const map = new Map([["one", 7]]);
	const entry = faBorrowSlot(map, "one", true);
	expect(faBorrowSlot(map, "one", true)).toBe(entry);
	faReferenceWrite(entry, 9);
	expect(map.get("one")).toBe(9);
});

test("pool carriers retain slot identity and are retired with their slots", () => {
	const pool = new FaValuePool();
	const first = pool.add(7);
	const second = pool.add(7);
	const reference = faBorrowSlot(pool, second, true);
	expect(faBorrowSlot(pool, second, true)).toBe(reference);
	faReferenceWrite(reference, 9);
	expect(pool.get(first)).toBe(7);
	expect(pool.get(second)).toBe(9);
	pool.delete(second);
	const third = pool.add(11);
	expect(faBorrowSlot(pool, third, true)).not.toBe(reference);
	expect(faReferenceValue(faBorrowSlot(pool, third, true))).toBe(11);
});

test("erased object references are bare objects and replace data in place", () => {
	const object = { value: 1 };
	const reference = faBorrowSlot([object], 0, false, true);
	expect(reference).toBe(object);
	expect(faReferenceValue(reference)).toBe(object);
	faReferenceWrite(reference, { value: 9 });
	expect(object.value).toBe(9);
});

test("a reference to an optional primitive remains distinct from no reference", () => {
	const values = [undefined];
	const reference = faBorrowSlot(values, 0, false);
	expect(reference).not.toBeUndefined();
	expect(faReferenceValue(reference)).toBeUndefined();
	expect(faReferenceValue(undefined)).toBeUndefined();
	faReferenceWrite(reference, 3);
	expect(values[0]).toBe(3);
});

test("a union slot keeps its carrier while its value changes representation", () => {
	const values = [{ value: 7 }];
	const reference = faBorrowSlot(values, 0, false, false);
	faReferenceWrite(reference, 9);
	expect(values).toEqual([9]);
	expect(faReferenceValue(reference)).toBe(9);
	expect(faBorrowSlot(values, 0, false, false)).toBe(reference);
});
