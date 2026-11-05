import { describe, expect, test } from "bun:test";

import { faFieldsToStringArray, faFormatFields } from "./fields.js";

const names = ["foo", "bar", "nested.baz1", "nested.baz2"];

describe("faFieldsToStringArray", () => {
	test("lists the flags in bit order", () => {
		expect(faFieldsToStringArray(0b1111, names)).toEqual(names);
		expect(faFieldsToStringArray(0b0101, names)).toEqual(["foo", "nested.baz1"]);
		expect(faFieldsToStringArray(0, names)).toEqual([]);
	});

	test("reads bit 31 and the bits of a bigint", () => {
		const wide = Array.from({ length: 40 }, (_, index) => `flag${index}`);
		expect(faFieldsToStringArray(2 ** 31, wide.slice(0, 32))).toEqual(["flag31"]);
		expect(faFieldsToStringArray((1n << 39n) | 1n, wide)).toEqual(["flag0", "flag39"]);
	});
});

describe("faFormatFields", () => {
	test("prints a key set with its groups", () => {
		expect(faFormatFields(0b1111, names)).toBe("{ foo, bar, nested { baz1, baz2 } }");
		expect(faFormatFields(0b0100, names)).toBe("{ nested { baz1 } }");
		expect(faFormatFields(0, names)).toBe("{}");
		expect(faFormatFields(0b11n, names)).toBe("{ foo, bar }");
	});
});
