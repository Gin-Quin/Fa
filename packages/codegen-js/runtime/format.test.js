import { describe, expect, test } from "bun:test";

import { FaHashMap, FaRange, FaSet } from "./collections.js";
import {
	ArithmeticError,
	Cancelled,
	DivisionByZero,
	FaError,
	IndexError,
	ParseError,
} from "./errors.js";
import { faFloat32ToString, faPercentageToString } from "./floats.js";
import {
	faFormat,
	faFormatArray,
	faFormatError,
	faFormatMap,
	faFormatMember,
	faFormatNested,
	faFormatObject,
	faFormatOptional,
	faFormatRange,
	faFormatSet,
	faFormatTuple,
	faQuote,
} from "./format.js";

describe("top level (values brief 2.8, D-5)", () => {
	test("strings are raw, other values in Fa notation", () => {
		expect(faFormat("hello")).toBe("hello");
		expect(faFormat('a"b')).toBe('a"b');
		expect(faFormat(-128)).toBe("-128");
		expect(faFormat(12n)).toBe("12");
		expect(faFormat(2)).toBe("2");
		expect(faFormat(0.5)).toBe("0.5");
		expect(faFormat(-0)).toBe("0");
		expect(faFormat(NaN)).toBe("NaN");
		expect(faFormat(-Infinity)).toBe("-Infinity");
		expect(faFormat(1e21)).toBe("1e+21");
		expect(faFormat(true)).toBe("true");
		expect(faFormat(undefined)).toBe("none");
	});

	test("golden strings of the documentation", () => {
		expect(faFormat({ name: "John" })).toBe('{ name = "John" }');
		expect(faFormat([2, 3, 4])).toBe("[2, 3, 4]");
		expect(faFormat([])).toBe("[]");
		expect(faFormat({})).toBe("{}");
		expect(faFormat({ x: 1, y: 2 })).toBe("{ x = 1, y = 2 }");
		expect(faFormat([{ a: [1, "b"] }, undefined])).toBe('[{ a = [1, "b"] }, none]');
	});
});

describe("nested strings are quoted with Fa escapes", () => {
	test("escapes", () => {
		expect(faQuote("plain")).toBe('"plain"');
		expect(faQuote('a"b')).toBe('"a\\"b"');
		expect(faQuote("back\\slash")).toBe('"back\\\\slash"');
		expect(faQuote("line\nbreak\r\ttab")).toBe('"line\\nbreak\\r\\ttab"');
		expect(faQuote("\0")).toBe('"\\0"');
		expect(faQuote("\x01\x1f\x7f")).toBe('"\\x01\\x1F\\x7F"');
		expect(faQuote("{x}")).toBe('"{{x}}"');
		expect(faQuote("é👨‍👩‍👧")).toBe('"é👨‍👩‍👧"');
		expect(faFormatNested("a")).toBe('"a"');
		expect(faFormat(["a\n"])).toBe('["a\\n"]');
	});
});

describe("typed formatters", () => {
	test("optionals", () => {
		expect(faFormatOptional(undefined)).toBe("none");
		expect(faFormatOptional("a")).toBe('"a"');
		expect(faFormatOptional(Math.fround(0.1), faFloat32ToString)).toBe("0.1");
		expect(faFormatOptional(undefined, faFloat32ToString)).toBe("none");
	});

	test("arrays of Float32 and percentages", () => {
		expect(faFormatArray([Math.fround(0.1), Math.fround(1.5)], faFloat32ToString)).toBe(
			"[0.1, 1.5]",
		);
		expect(faFormatArray([0.1, 0.125], faPercentageToString)).toBe("[10%, 12.5%]");
		expect(faFormatArray(new Set([1, 2]))).toBe("[1, 2]");
	});

	test("tuples", () => {
		expect(faFormatTuple([1, "one"])).toBe('(1, "one")');
		expect(faFormatTuple([Math.fround(0.1), 2], [faFloat32ToString])).toBe("(0.1, 2)");
		expect(faFormatTuple([])).toBe("()");
	});

	test("objects in declaration order, relation fields left out", () => {
		const user = { id: 1, name: "Ana", friends: [] };
		expect(
			faFormatObject(user, [
				["name", faFormatNested],
				["id", faFormatNested],
			]),
		).toBe('{ name = "Ana", id = 1 }');
		expect(faFormatObject({ $t: 3, radius: 1 })).toBe("{ radius = 1 }");
		expect(faFormatObject({}, [])).toBe("{}");
		expect(faFormatObject({ ratio: 0.5 }, [["ratio", faPercentageToString]])).toBe(
			"{ ratio = 50% }",
		);
	});

	test("union members", () => {
		expect(faFormatMember("Shape.Point")).toBe("Shape.Point");
		expect(faFormatMember("Shape.Circle", faFormatObject({ radius: 1 }), true)).toBe(
			"Shape.Circle { radius = 1 }",
		);
		expect(faFormatMember("UserId.User", "12")).toBe("UserId.User(12)");
		expect(faFormatMember("Name.Text", faQuote("a"))).toBe('Name.Text("a")');
	});

	test("maps and sets like their literals", () => {
		expect(
			faFormatMap(
				new Map([
					["a", 1],
					["b", 2],
				]),
			),
		).toBe('[("a", 1), ("b", 2)]');
		expect(faFormatMap(new Map())).toBe("[]");
		expect(faFormat(new Map([[1, "Ana"]]))).toBe('[(1, "Ana")]');
		expect(faFormat(new FaHashMap([[{ x: 1 }, [2]]]))).toBe("[({ x = 1 }, [2])]");
		expect(faFormatMap(new Map([[1, 0.5]]), String, faPercentageToString)).toBe("[(1, 50%)]");
		expect(faFormatSet(new Set(["a"]))).toBe('["a"]');
		expect(faFormat(new FaSet([{ a: 1 }]))).toBe("[{ a = 1 }]");
		expect(faFormat(new Set())).toBe("[]");
	});

	test("ranges", () => {
		expect(faFormatRange(new FaRange(1, 3, false))).toBe("1..3");
		expect(faFormatRange(new FaRange(0, 10, true))).toBe("0..=10");
		expect(faFormatRange(new FaRange(5, undefined, false))).toBe("5..");
		expect(faFormatRange(new FaRange(undefined, 10, false))).toBe("..10");
		expect(faFormatRange(new FaRange(undefined, 10, true))).toBe("..=10");
		expect(faFormatRange(new FaRange(undefined, undefined, false))).toBe("..");
		expect(faFormat(new FaRange(-3, -1, false))).toBe("-3..-1");
		expect(faFormat([new FaRange(1, 2, false)])).toBe("[1..2]");
	});
});

describe("errors (T-ERR-8, types brief D50)", () => {
	test("built-in errors", () => {
		expect(String(new IndexError(5, 3))).toBe("IndexError { index = 5, length = 3 }");
		expect(String(new DivisionByZero())).toBe("DivisionByZero");
		expect(String(new Cancelled())).toBe("Cancelled");
		expect(String(new ArithmeticError("128 doesn't fit in Int8"))).toBe(
			`ArithmeticError { message = "128 doesn't fit in Int8" }`,
		);
		expect(String(new ParseError("4x", "not a number"))).toBe(
			'ParseError { input = "4x", message = "not a number" }',
		);
		expect(`${new DivisionByZero()}`).toBe("DivisionByZero");
	});

	test("generated members", () => {
		class NetworkError extends FaError {
			static faName = "ApiError.NetworkError";

			/** @param {string} message */
			constructor(message) {
				super();
				this.message = message;
			}
		}
		class Timeout extends FaError {
			static faName = "ApiError.Timeout";
		}
		expect(String(new NetworkError("Connection lost"))).toBe(
			'ApiError.NetworkError { message = "Connection lost" }',
		);
		expect(String(new Timeout())).toBe("ApiError.Timeout");
		expect(faFormat(new Timeout())).toBe("ApiError.Timeout");
		expect(faFormat([new Timeout()])).toBe("[ApiError.Timeout]");
		expect(faFormatError(new Timeout())).toBe("ApiError.Timeout");
		expect(`failed: ${new Timeout()}`).toBe("failed: ApiError.Timeout"); // T-ERR-5
	});
});
