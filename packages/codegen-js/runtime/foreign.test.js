import { describe, expect, test } from "bun:test";

import {
	faCArray,
	faCArrayBack,
	faCBox,
	faCCallback,
	faCString,
	faCStringRead,
	faCUnbox,
	faFfiBind,
	faFfiLibraries,
	faPointerOffset,
	faPointerAllocate,
	faPointerFree,
	faPointerMemory,
	faPointerRead,
	faPointerReadValue,
	faPointerToArray,
	faPointerValue,
	faPointerWrite,
	faPointerWriteValue,
} from "./foreign.js";

describe("conversions", () => {
	test("a string is zero-terminated UTF-8", () => {
		expect([...faCString("é")]).toEqual([0xc3, 0xa9, 0]);
	});

	test("arrays are copied into typed arrays, and back", () => {
		const items = [3, 1, 2];
		const typed = faCArray(items, "i32");
		expect(typed).toBeInstanceOf(Int32Array);
		typed[0] = 9;
		faCArrayBack(items, typed, "i32");
		expect(items).toEqual([9, 1, 2]);
		expect([...faCArray([true, false], "bool")]).toEqual([1, 0]);
	});

	test("a box holds one value", () => {
		expect(faCUnbox(faCBox(5n, "i64"), "i64")).toBe(5n);
		expect(faCUnbox(faCBox(undefined, "ptr"), "ptr")).toBeUndefined();
		expect(faCUnbox(faCBox(true, "bool"), "bool")).toBe(true);
	});

	test("the libraries end with the C library", () => {
		const libraries = faFfiLibraries(["sqlite3", "./local.so"]);
		expect(libraries[0]).toMatch(/^libsqlite3\.(so|dylib)$/);
		expect(libraries[1]).toBe("./local.so");
		expect(libraries.length).toBe(4);
	});
});

describe.if(process.platform === "linux")("calls", () => {
	test("owned pointer allocations use C memory and release opaque values", () => {
		const scalar = faPointerAllocate(41, "i64");
		expect(faPointerReadValue(scalar, "i64", false)).toBe(41);
		expect(faPointerReadValue(scalar, "i64", true)).toBe(41n);
		faPointerWriteValue(scalar, "i64", 42);
		expect(faPointerRead(scalar, "i64")).toBe(42n);
		faPointerFree(scalar);
		const destroyed = [];
		const opaque = faPointerAllocate({ name: "farm" }, null);
		faPointerValue(opaque, null, false).name = "goats";
		expect(faPointerReadValue(opaque, null, false)).toEqual({ name: "goats" });
		faPointerWriteValue(opaque, null, { name: "cows" }, (value) => destroyed.push(value.name));
		faPointerFree(opaque, (value) => destroyed.push(value.name));
		expect(destroyed).toEqual(["goats", "cows"]);
		expect(faPointerMemory.values.has(opaque)).toBe(false);
	});

	test("pointer.value writes through nested C structs and fixed arrays", () => {
		const inner = { size: 4, fields: [["x", "i32", 0, false]] };
		const items = { array: "i16", count: 2, stride: 2, bigint: false };
		const spec = { size: 8, fields: [["point", inner, 0, false], ["items", items, 4, false]] };
		const pointer = faPointerAllocate({ point: { x: 7 }, items: [1, 2] }, spec);
		const value = faPointerValue(pointer, spec, false);
		value.point.x = -3;
		value.items[1] = 42;
		expect([...value.items]).toEqual([1, 42]);
		expect(faPointerReadValue(pointer, spec, false)).toEqual({ point: { x: -3 }, items: [1, 42] });
		faPointerFree(pointer);
	});

	test("C functions of the C library", () => {
		const strlen = faFfiBind([], "strlen", ["ptr"], "u64");
		expect(strlen(faCString("farm"))).toBe(4n);
		const abs = faFfiBind([], "abs", ["i32"], "i32");
		expect(abs(-3)).toBe(3);
		const getenv = faFfiBind([], "getenv", ["ptr"], "ptr");
		expect(getenv(faCString("FA_SURELY_NOT_SET"))).toBeUndefined();
		expect(() => faFfiBind([], "fa_no_such_symbol", [], "void")).toThrow();
	});

	test("pointers read and write memory", () => {
		const malloc = faFfiBind([], "malloc", ["u64"], "ptr");
		const free = faFfiBind([], "free", ["ptr"], "void");
		const block = malloc(16n);
		faPointerWrite(block, "i32", 7);
		faPointerWrite(faPointerOffset(block, "i32", 1), "i32", -2);
		expect(faPointerRead(block, "i32")).toBe(7);
		expect(faPointerToArray(block, "i32", 2)).toEqual([7, -2]);
		free(block);
		const strdup = faFfiBind([], "strdup", ["ptr"], "ptr");
		expect(faCStringRead(strdup(faCString("goat")))).toBe("goat");
	});

	test("callbacks", () => {
		const qsort = faFfiBind([], "qsort", ["ptr", "u64", "u64", "function"], "void");
		const items = faCArray([3, 1, 2], "i32");
		const compare = faCCallback(
			(a, b) => faPointerRead(a, "i32") - faPointerRead(b, "i32"),
			["ptr", "ptr"],
			"i32",
		);
		qsort(items, 3n, 4n, compare.pointer);
		compare.close();
		expect([...items]).toEqual([1, 2, 3]);
	});
});
