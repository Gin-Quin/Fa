import { describe, expect, test } from "bun:test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { FileError, JsonError, faJsonParse, faJsonStringify, faReadFile } from "./system.js";

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
			expect(/** @type {FileError} */ (error).message).toBe("no such file");
		}
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

	test("reads a value of the expected type", () => {
		const value = faJsonParse('{"age": 36, "name": "Ada", "extra": 1}', user);
		expect(value).toEqual({ name: "Ada", age: 36, email: undefined });
		expect(Object.keys(value)).toEqual(["name", "age", "email"]);
		expect(faJsonParse('{"name": "Ada", "age": 1, "email": null}', user).email).toBe(undefined);
		expect(faJsonParse("[1, 2]", ["a", ["I", null, null]])).toEqual([1n, 2n]);
		expect(faJsonParse('{"a": 1.5}', ["m", "f"])).toEqual(new Map([["a", 1.5]]));
		expect(faJsonParse('"red"', ["u", [["l", "red"], ["l", "blue"]]])).toBe("red");
	});

	test("throws a JsonError when the JSON doesn't fit", () => {
		expect(() => faJsonParse("{", user)).toThrow(JsonError);
		expect(() => faJsonParse('{"name": "Ada"}', user)).toThrow("has no field `age`");
		expect(() => faJsonParse('{"name": "Ada", "age": -1}', user)).toThrow("the value.age should be at least 0, not -1");
		expect(() => faJsonParse('{"name": "Ada", "age": 1.5}', user)).toThrow(
			"the value.age should be an integer, not number",
		);
		expect(() => faJsonParse("1e300", ["i", null, null])).toThrow(JsonError);
		expect(() => faJsonParse('"green"', ["u", [["l", "red"]]])).toThrow(JsonError);
	});
});

describe("faJsonStringify", () => {
	test("writes Fa values", () => {
		expect(faJsonStringify({ name: "Ada", email: undefined, tags: ["a"] })).toBe(
			'{"name":"Ada","tags":["a"]}',
		);
		expect(faJsonStringify([1, undefined, 2n, Number.NaN])).toBe("[1,null,2,null]");
		expect(faJsonStringify(new Map([["a", true]]))).toBe('{"a":true}');
		expect(faJsonStringify(new Set(["x"]))).toBe('["x"]');
		expect(faJsonStringify({ $t: "Shape.Circle", radius: 1 })).toBe('{"radius":1}');
		expect(faJsonStringify(undefined)).toBe("null");
		expect(faJsonStringify('say "hi"')).toBe('"say \\"hi\\""');
	});
});
