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
	faPointerRead,
	faPointerToArray,
	faPointerWrite,
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
