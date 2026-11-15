import { expect, test } from "bun:test";

import { faAssign, faCopy, faCopyGraph } from "./copy.js";
import { faBytesEqual, faEquals, faHash } from "./equality.js";
import { faFormat, faFormatBytes } from "./format.js";

test("byte equality includes length and contents, and distinguishes arrays", () => {
	const data = new Uint8Array([1, 2]);
	expect(faBytesEqual(data, data.slice())).toBe(true);
	expect(faBytesEqual(data, new Uint8Array([1]))).toBe(false);
	expect(faBytesEqual(data, new Uint8Array([1, 3]))).toBe(false);
	expect(faEquals(data, [1, 2])).toBe(false);
	expect(faEquals(data, data.slice())).toBe(true);
	expect(faEquals({ data }, { data: data.slice() })).toBe(true);
	expect(faHash(data)).toBe(faHash(data.slice()));
});

test("byte copies own their buffer, including in erased objects and graphs", () => {
	const data = new Uint8Array([1, 2]);
	for (const copy of [faCopy(data), faCopy({ data }).data, faCopyGraph({ data }).data]) {
		expect(copy).toBeInstanceOf(Uint8Array);
		copy[0] = 255;
		expect(data[0]).toBe(1);
	}
	const target = new Uint8Array(2);
	faAssign(target, data);
	expect(target).toEqual(data);
	data[0] = 9;
	expect(target[0]).toBe(1);
});

test("byte notation never decodes text", () => {
	const data = new Uint8Array([195, 169, 255]);
	expect(faFormatBytes(data)).toBe("[195, 169, 255]");
	expect(faFormat(data)).toBe("[195, 169, 255]");
	expect(faFormat({ data })).toBe("{ data = [195, 169, 255] }");
	expect(faFormatBytes(new Uint8Array())).toBe("[]");
});
