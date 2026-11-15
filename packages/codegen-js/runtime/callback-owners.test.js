import { expect, test } from "bun:test";
import { FaHashMap, FaSet } from "./collections.js";
import { faDropOwned, faOwnShared, faReleaseShared } from "./ownership.js";

test("a copied Set keeps its key callback alive after the original scope", () => {
	let finalized = 0;
	const captured = faOwnShared({ suffix: "!" }, () => { finalized += 1; });
	const key = faOwnShared((value) => value + captured.suffix, () => faReleaseShared(captured));
	const original = new FaSet(["a"], { key });
	const copied = original.emptyCopy();
	for (const value of original) copied.add(value);
	faDropOwned(original);
	expect(finalized).toBe(0);
	expect(copied.has("a")).toBe(true);
	faDropOwned(copied);
	copied.dropCallbacks();
	expect(finalized).toBe(1);
});

test("a copied HashMap retains hash and equality environments", () => {
	let finalized = 0;
	const hash = faOwnShared((key) => key.value, () => { finalized += 1; });
	const equals = faOwnShared((left, right) => left.value === right.value, () => { finalized += 1; });
	const original = new FaHashMap([[{ value: 1 }, 2]], hash, equals);
	const copied = original.emptyCopy();
	for (const [key, value] of original) copied.set(key, value);
	original.dropCallbacks();
	original.dropCallbacks();
	expect(finalized).toBe(0);
	expect(copied.get({ value: 1 })).toBe(2);
	faDropOwned(copied);
	expect(finalized).toBe(2);
});
