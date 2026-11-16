import { describe, expect, test } from "bun:test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
	faJsonParse,
	faJsonStringify,
	faMonotonic,
	faReadBytes,
	faReadBytesAsync,
	faReadFile,
	faReadFileAsync,
	faWriteFile,
	faWriteFileAsync,
	FileError,
	JsonError,
} from "./system.js";
import { faCancel, faCancelSignal, FaTask } from "./tasks.js";

describe("faReadFile", () => {
	test("reads a whole file as UTF-8", () => {
		const directory = mkdtempSync(join(tmpdir(), "fa-system-"));
		const path = join(directory, "scores.csv");
		writeFileSync(path, "Ana, 90\nBão, 70\n");
		expect(faReadFile(path)).toBe("Ana, 90\nBão, 70\n");
	});

	test("throws a FileError for a missing file", () => {
		expect(() => faReadFile("/nonexistent/fa/file.txt")).toThrow(FileError);
		try {
			faReadFile("/nonexistent/fa/file.txt");
		} catch (error) {
			expect(/** @type {FileError} */ (error).path).toBe("/nonexistent/fa/file.txt");
			expect(/** @type {FileError} */ (error).message).toBe("No such file or directory");
		}
	});
});

describe("bytes, writes and awaited files", () => {
	const directory = mkdtempSync(join(tmpdir(), "fa-system-"));

	test("writes a file, then reads its bytes", () => {
		const path = join(directory, "written.txt");
		faWriteFile(path, "héllo");
		expect(faReadFile(path)).toBe("héllo");
		expect(faReadBytes(path)).toEqual(new Uint8Array([104, 195, 169, 108, 108, 111]));
	});

	test("awaited forms read and write while other code runs", async () => {
		const task = new FaTask(null);
		const path = join(directory, "awaited.txt");
		await faWriteFileAsync(task, path, "one\ntwo");
		expect(await faReadFileAsync(task, path)).toBe("one\ntwo");
		expect((await faReadBytesAsync(task, path)).length).toBe(7);
	});

	test("errors carry the system's text", async () => {
		const task = new FaTask(null);
		expect(() => faReadBytes(directory)).toThrow(FileError);
		try {
			await faReadFileAsync(task, join(directory, "missing"));
			throw new Error("not reached");
		} catch (error) {
			expect(/** @type {FileError} */ (error).message).toBe("No such file or directory");
		}
	});

	test("a cancelled task stops waiting for its file", async () => {
		const task = new FaTask(null);
		const reading = faReadFileAsync(task, join(directory, "written.txt"));
		faCancel(task);
		await expect(reading).rejects.toBe(faCancelSignal);
	});
});

describe("faMonotonic", () => {
	test("gives growing nanoseconds", () => {
		const first = faMonotonic();
		const second = faMonotonic();
		expect(Number.isInteger(first)).toBe(true);
		expect(second).toBeGreaterThanOrEqual(first);
	});
});

describe("faJsonParse", () => {
	const user = [
		"o",
		[
			["name", "s", false],
			["age", ["i", "0", null], false],
			["email", ["u", ["s", "n"]], true],
		],
	];

	test("writes and reads a Fields value as the paths of its flags", () => {
		const names = ["foo", "bar", "nested.baz1", "nested.baz2"];
		const schema = ["F", names, ["foo", "bar", "nested", "nested.baz1", "nested.baz2"]];
		expect(faJsonStringify(0b1001, schema)).toBe('["foo","nested.baz2"]');
		expect(faJsonParse('["bar", "nested"]', schema)).toBe(0b1110);
		expect(faJsonParse("[]", schema)).toBe(0);
		const wide = Array.from({ length: 40 }, (_, index) => `f${index}`);
		expect(faJsonParse('["f39"]', ["F", wide, wide])).toBe(1n << 39n);
		expect(() => faJsonParse('["qux"]', schema)).toThrow(JsonError);
	});

	test("reads a value of the expected type", () => {
		const value = faJsonParse('{"age": 36, "name": "Ada", "extra": 1}', user);
		expect(value).toEqual({ name: "Ada", age: 36, email: undefined });
		expect(Object.keys(value)).toEqual(["name", "age", "email"]);
		expect(faJsonParse('{"name": "Ada", "age": 1, "email": null}', user).email).toBe(undefined);
		expect(faJsonParse("[1, 2]", ["a", ["I", null, null]])).toEqual([1n, 2n]);
		expect(faJsonParse('{"a": 1.5}', ["m", "f"])).toEqual(new Map([["a", 1.5]]));
		expect(
			faJsonParse('"red"', [
				"u",
				[
					["l", "red"],
					["l", "blue"],
				],
			]),
		).toBe("red");
	});

	test("throws a JsonError when the JSON doesn't fit", () => {
		expect(() => faJsonParse("{", user)).toThrow(JsonError);
		expect(() => faJsonParse('{"name": "Ada"}', user)).toThrow("has no field `age`");
		expect(() => faJsonParse('{"name": "Ada", "age": -1}', user)).toThrow(
			"the value.age should be at least 0, not -1",
		);
		expect(() => faJsonParse('{"name": "Ada", "age": 1.5}', user)).toThrow(
			"the value.age should be an integer, not number",
		);
		expect(() => faJsonParse("1e300", ["i", null, null])).toThrow(JsonError);
		expect(() => faJsonParse('"green"', ["u", [["l", "red"]]])).toThrow(JsonError);
	});
});

describe("faJsonStringify", () => {
	test("object unions select by fields and literal discriminants", () => {
		const forward = [
			"o",
			[
				["kind", ["l", "forward"]],
				["z", "s"],
				["a", "s"],
			],
		];
		const reverse = [
			"o",
			[
				["a", "s"],
				["z", "s"],
				["kind", ["l", "reverse"]],
			],
		];
		const schema = ["u", [forward, reverse]];
		expect(faJsonStringify({ a: "a", z: "z", kind: "reverse" }, schema)).toBe(
			'{"a":"a","z":"z","kind":"reverse"}',
		);
		expect(faJsonStringify({ a: "a", z: "z", kind: "forward" }, schema)).toBe(
			'{"kind":"forward","z":"z","a":"a"}',
		);
		const distinct = [
			"u",
			[
				["o", [["z", "s"]]],
				["o", [["a", "s"]]],
			],
		];
		expect(faJsonStringify({ a: "a" }, distinct)).toBe('{"a":"a"}');
	});
	test("deep recursive values use bounded stack space", () => {
		/** @type {any} */
		let value;
		for (let index = 0; index < 100000; index++) value = { next: value };
		const text = faJsonStringify(value);
		expect(text).toBe('{"next":'.repeat(99999) + "{}" + "}".repeat(99999));
	});
	test("writes Fa values", () => {
		expect(faJsonStringify({ name: "Ada", email: undefined, tags: ["a"] })).toBe(
			'{"name":"Ada","tags":["a"]}',
		);
		expect(faJsonStringify([1, undefined, 2n, Number.NaN])).toBe("[1,null,2,null]");
		expect(faJsonStringify(new Map([["a", true]]))).toBe('{"a":true}');
		expect(faJsonStringify(new Set(["x"]))).toBe('["x"]');
		// a member of a named union: tagged by `"type"` (design/decisions.md)
		expect(faJsonStringify({ $t: "Shape.Circle", radius: 1 })).toBe('{"type":"Circle","radius":1}');
		expect(faJsonStringify({ $t: "Shape.Label", $v: "a" })).toBe('{"type":"Label","value":"a"}');
		// by a schema: an enumeration's names or pinned values, fields in order
		const color = [
			"E",
			[
				["red", 0, false],
				["green", 1, false],
			],
		];
		expect(faJsonStringify([1, 0], ["a", color])).toBe('["green","red"]');
		expect(faJsonStringify(9, ["E", [["archived", 9, true]]])).toBe("9");
		const point = [
			"o",
			[
				["y", "f", false, null],
				["x", "f", false, null],
			],
		];
		expect(faJsonStringify({ x: 1, y: 2 }, point)).toBe('{"y":2,"x":1}');
		expect(faJsonStringify(undefined)).toBe("null");
		expect(faJsonStringify('say "hi"')).toBe('"say \\"hi\\""');
	});
});

// Both file paths keep raw bytes; only the Text variants decode strictly.
describe("UTF-8 file boundaries", () => {
	test("strict reads reject invalid bytes, while binary reads preserve them", async () => {
		const { faWriteBytes, faWriteBytesAsync } = await import("./system.js");
		const directory = mkdtempSync(join(tmpdir(), "fa-utf8-"));
		const path = join(directory, "invalid");
		const bytes = new Uint8Array([97, 255, 240, 159]);
		faWriteBytes(path, bytes);
		expect(faReadBytes(path)).toEqual(bytes);
		expect(() => faReadFile(path)).toThrow(FileError);
		const task = new FaTask(null);
		await expect(faReadFileAsync(task, path)).rejects.toBeInstanceOf(FileError);
		await faWriteBytesAsync(task, path, bytes);
		expect(await faReadBytesAsync(task, path)).toEqual(bytes);
	});
	test("text writes replace lone surrogates and reads retain BOM", async () => {
		const directory = mkdtempSync(join(tmpdir(), "fa-utf8-"));
		const path = join(directory, "text");
		faWriteFile(path, "\ufeffé日\ud800");
		expect(faReadFile(path)).toBe("\ufeffé日�");
		const task = new FaTask(null);
		await faWriteFileAsync(task, path, "\ufeff😀\udc00");
		expect(await faReadFileAsync(task, path)).toBe("\ufeff😀�");
	});
});

describe("JSON byte inputs", () => {
	test("byte parser agrees with native JSON, including escapes and numeric limits", async () => {
		const { faJsonReadBytes } = await import("./system.js");
		const encoder = new TextEncoder();
		for (const text of [
			"null",
			"true",
			"false",
			"-0",
			"1e400",
			"1.25e-4",
			'"é日😀"',
			'"\\ud800"',
			'"\\\"\\\\\\n"',
			'{"__proto__":1,"constructor":2}',
			'{"é":[null,true,false,{},[],1,-2,1.5,"😀"]}',
			" \r\n\t[1,2] ",
		]) {
			expect(faJsonReadBytes(encoder.encode(text))).toEqual(JSON.parse(text));
		}
		const deep = "[".repeat(10000) + "0" + "]".repeat(10000);
		let value = faJsonReadBytes(encoder.encode(deep));
		for (let depth = 0; depth < 10000; depth++) value = value[0];
		expect(value).toBe(0);
	});
	test("malformed JSON and invalid UTF-8 are rejected", async () => {
		const { faJsonReadBytes } = await import("./system.js");
		const encoder = new TextEncoder();
		for (const text of [
			"",
			"01",
			"+1",
			".1",
			"NaN",
			"undefined",
			"[1,]",
			'{"a":1,}',
			"[",
			"{",
			"[1 2]",
			"{} []",
			'"unterminated',
			'"\\q"',
			'"\n"',
			"\ufeffnull",
		]) {
			expect(() => faJsonParse(encoder.encode(text), "n", faJsonReadBytes)).toThrow(JsonError);
		}
		for (const bytes of [[34, 255, 34], [34, 237, 160, 128, 34], [255], [34, 240, 159, 34]]) {
			expect(() => faJsonParse(new Uint8Array(bytes), "s", faJsonReadBytes)).toThrow(JsonError);
		}
	});
	test("byte JSON inputs can hold arrays whose schema checks every byte", async () => {
		const { faJsonReadBytes } = await import("./system.js");
		const value = [0, 127, 255];
		const schema = ["a", ["i", "0", "255"]];
		expect(faJsonStringify(value, schema)).toBe("[0,127,255]");
		expect(faJsonParse(new TextEncoder().encode("[0,127,255]"), schema, faJsonReadBytes)).toEqual(
			value,
		);
		for (const text of ["[256]", "[-1]", "[0.5]", '["1"]', "{}", "null"]) {
			expect(() => faJsonParse(text, schema)).toThrow(JsonError);
		}
	});
});
