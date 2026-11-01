import { describe, expect, test } from "bun:test";

import { ArithmeticError, DivisionByZero, ParseError } from "./errors.js";
import {
	faBigFloorDivide,
	faBigFloorModulo,
	faBigPower,
	faBigToInteger,
	faBitAnd,
	faBitNot,
	faBitOr,
	faBitXor,
	faCheckedAdd,
	faCheckedMul,
	faCheckedPow,
	faCheckedSub,
	faCheckInteger,
	faCheckRange,
	faCheckWidth,
	faClamp,
	faConvertInteger,
	faExactResult,
	faFloatToInteger,
	faFloorDivide,
	faFloorModulo,
	faInt64ToInteger,
	faIntegerCeil,
	faIntegerFloor,
	faIntegerRound,
	faIntegerTruncate,
	faParseBigInteger,
	faParseInteger,
	faPower,
	faRangeOverflow,
	faSaturatingAdd,
	faSaturatingMul,
	faSaturatingPow,
	faSaturatingSub,
	faShiftLeft,
	faShiftRight,
	faWrap,
	faWrapBig,
	faWrappingAdd,
	faWrappingMul,
	faWrappingPow,
	faWrappingSub,
} from "./integers.js";
import { FaStop } from "./stops.js";

const MAX = Number.MAX_SAFE_INTEGER;
const INT64_MIN = -(2n ** 63n);
const INT64_MAX = 2n ** 63n - 1n;
const UINT64_MAX = 2n ** 64n - 1n;

/**
 * The stop thrown by `run`.
 * @param {() => unknown} run
 * @returns {FaStop}
 */
function stopOf(run) {
	try {
		run();
	} catch (exception) {
		if (exception instanceof FaStop) {
			return exception;
		}
		throw exception;
	}
	throw new Error("no stop");
}

/**
 * The Fa error thrown by `run`.
 * @param {() => unknown} run
 * @returns {any}
 */
function errorOf(run) {
	try {
		run();
	} catch (exception) {
		return exception;
	}
	throw new Error("no error");
}

describe("overflow checks", () => {
	test("faCheckInteger accepts the safe range", () => {
		for (const value of [0, 1, -1, 42, MAX, -MAX, MAX - 1, 2 ** 52]) {
			expect(faCheckInteger(value)).toBe(value);
		}
	});

	test("faCheckInteger stops beyond ±(2^53 − 1)", () => {
		for (const value of [MAX + 1, -MAX - 1, 2 ** 60, -(2 ** 60), Infinity, -Infinity]) {
			expect(() => faCheckInteger(value, "main.fa:1:1")).toThrow(FaStop);
		}
		const stop = stopOf(() => faCheckInteger(2 ** 53, "main.fa:3:7"));
		expect(stop.reason).toBe(
			"integer overflow: 9007199254740992 doesn't fit in Integer (±(2^53 − 1) on JavaScript)",
		);
		expect(stop.location).toBe("main.fa:3:7");
	});

	test("faCheckRange checks sized and ranged types (T-NUM-13)", () => {
		expect(faCheckRange(100 + 27, -128, 127, "Int8")).toBe(127);
		expect(faCheckRange(-128, -128, 127, "Int8")).toBe(-128);
		expect(stopOf(() => faCheckRange(100 + 28, -128, 127, "Int8")).reason).toBe(
			"integer overflow: 128 doesn't fit in Int8",
		);
		expect(stopOf(() => faCheckRange(-129, -128, 127, "Int8")).reason).toBe(
			"integer overflow: -129 doesn't fit in Int8",
		);
		expect(faCheckRange(255, 0, 255, "UInt8")).toBe(255);
		expect(() => faCheckRange(256, 0, 255, "UInt8")).toThrow(FaStop);
		expect(() => faCheckRange(-1, 0, 255, "UInt8")).toThrow(FaStop);
		expect(faCheckRange(174, 0, 200, "Integer(min = 0, max = 200)")).toBe(174);
		expect(stopOf(() => faCheckRange(19, 20, MAX, "Integer(min = 20)")).reason).toBe(
			"integer overflow: 19 doesn't fit in Integer(min = 20)",
		);
		expect(faCheckRange(2147483647, -2147483648, 2147483647, "Int32")).toBe(2147483647);
		expect(() => faCheckRange(2147483648, -2147483648, 2147483647, "Int32")).toThrow(FaStop);
	});

	test("faCheckRange works on bigints", () => {
		expect(faCheckRange(5n, 0n, 10n, "UInt64(max = 10)")).toBe(5n);
		expect(() => faCheckRange(11n, 0n, 10n, "UInt64(max = 10)")).toThrow(FaStop);
	});

	test("faCheckWidth checks 64-bit and 128-bit bigints", () => {
		expect(faCheckWidth(INT64_MAX, 64, true, "Int64")).toBe(INT64_MAX);
		expect(faCheckWidth(INT64_MIN, 64, true, "Int64")).toBe(INT64_MIN);
		expect(stopOf(() => faCheckWidth(INT64_MAX + 1n, 64, true, "Int64")).reason).toBe(
			"integer overflow: 9223372036854775808 doesn't fit in Int64",
		);
		expect(() => faCheckWidth(INT64_MIN - 1n, 64, true, "Int64")).toThrow(FaStop);
		expect(faCheckWidth(UINT64_MAX, 64, false, "UInt64")).toBe(UINT64_MAX);
		expect(() => faCheckWidth(-1n, 64, false, "UInt64")).toThrow(FaStop);
		expect(() => faCheckWidth(UINT64_MAX + 1n, 64, false, "UInt64")).toThrow(FaStop);
		expect(faCheckWidth(2n ** 127n - 1n, 128, true, "Int128")).toBe(2n ** 127n - 1n);
		expect(() => faCheckWidth(2n ** 127n, 128, true, "Int128")).toThrow(FaStop);
		expect(faCheckWidth(2n ** 128n - 1n, 128, false, "UInt128")).toBe(2n ** 128n - 1n);
		expect(() => faCheckWidth(2n ** 128n, 128, false, "UInt128")).toThrow(FaStop);
	});

	test("Int64 arithmetic is exact beyond 2^53 (T-NUM-44)", () => {
		const x = 9007199254740993n;
		expect(String(faCheckWidth(x + 1n, 64, true, "Int64"))).toBe("9007199254740994");
	});

	test("faRangeOverflow stops", () => {
		expect(() => faRangeOverflow("main.fa:1:1")).toThrow(FaStop);
	});
});

describe("floor division and remainder", () => {
	test("documented values (T-NUM-7, T-NUM-41)", () => {
		expect(faFloorDivide(22, 7)).toBe(3);
		expect(faFloorDivide(-7, 2)).toBe(-4);
		expect(faFloorModulo(7, 3)).toBe(1);
		expect(faFloorModulo(-7, 3)).toBe(2);
		expect(faFloorModulo(7, -3)).toBe(-2);
		expect(faFloorDivide(10, 4)).toBe(2);
		expect(faFloorDivide(10, 2)).toBe(5);
	});

	test("every sign combination keeps a == (a // b) * b + a %% b", () => {
		for (const a of [-13, -12, -7, -1, 0, 1, 7, 12, 13]) {
			for (const b of [-5, -3, -1, 1, 3, 5]) {
				const q = faFloorDivide(a, b);
				const r = faFloorModulo(a, b);
				expect(q * b + r).toBe(a);
				expect(q).toBe(Math.floor(a / b) + 0);
				if (r !== 0) {
					expect(Math.sign(r)).toBe(Math.sign(b));
				}
				expect(Math.abs(r)).toBeLessThan(Math.abs(b));
			}
		}
	});

	test("exact near 2^53 where the documented formulas fail (D-74)", () => {
		// Math.floor(a / b) rounds: (2^53 − 1) / 2 is 4503599627370495.5.
		expect(faFloorDivide(MAX, 2)).toBe(4503599627370495);
		expect(faFloorModulo(MAX, 2)).toBe(1);
		expect(faFloorDivide(-MAX, 2)).toBe(-4503599627370496);
		expect(faFloorModulo(-MAX, 2)).toBe(1);
		expect(faFloorDivide(MAX, 3)).toBe(3002399751580330);
		expect(faFloorModulo(MAX, 3)).toBe(1);
		// ((a % b) + b) % b overflows the safe range for a large divisor.
		expect(faFloorModulo(-1, MAX)).toBe(MAX - 1);
		expect(faFloorDivide(-1, MAX)).toBe(-1);
		expect(faFloorModulo(MAX - 1, -MAX)).toBe(-1);
		expect(faFloorDivide(MAX - 1, -MAX)).toBe(-1);
		expect(faFloorDivide(MAX, MAX)).toBe(1);
		expect(faFloorDivide(-MAX, 1)).toBe(-MAX);
		expect(faFloorDivide(-MAX, -1)).toBe(MAX);
		expect(faFloorModulo(-MAX, -1)).toBe(0);
		for (const a of [MAX, MAX - 1, -MAX, -MAX + 1, 2 ** 52 + 1, -(2 ** 52) - 1]) {
			for (const b of [2, 3, 7, -2, -3, -7, 1000000007, -1000000007]) {
				const q = BigInt(faFloorDivide(a, b));
				const r = BigInt(faFloorModulo(a, b));
				expect(q).toBe(faBigFloorDivide(BigInt(a), BigInt(b)));
				expect(r).toBe(faBigFloorModulo(BigInt(a), BigInt(b)));
			}
		}
	});

	test("never returns -0 (D-76)", () => {
		expect(Object.is(faFloorDivide(0, -5), 0)).toBe(true);
		expect(Object.is(faFloorModulo(-4, 2), 0)).toBe(true);
		expect(Object.is(faFloorModulo(4, -2), 0)).toBe(true);
		expect(Object.is(faFloorModulo(0, -3), 0)).toBe(true);
		expect(1 / faFloorDivide(0, -5)).toBe(Infinity);
	});

	test("division by zero throws DivisionByZero (T-NUM-19)", () => {
		expect(() => faFloorDivide(10, 0)).toThrow(DivisionByZero);
		expect(() => faFloorModulo(10, 0)).toThrow(DivisionByZero);
		expect(() => faFloorDivide(0, 0)).toThrow(DivisionByZero);
		expect(() => faFloorModulo(10, -0)).toThrow(DivisionByZero);
		expect(() => faBigFloorDivide(10n, 0n)).toThrow(DivisionByZero);
		expect(() => faBigFloorModulo(10n, 0n)).toThrow(DivisionByZero);
		expect(String(errorOf(() => faFloorDivide(1, 0)))).toBe("DivisionByZero");
	});

	test("bigints (T-NUM-28)", () => {
		expect(faBigFloorDivide(-7n, 2n)).toBe(-4n);
		expect(faBigFloorModulo(-7n, 2n)).toBe(1n);
		expect(faBigFloorDivide(7n, -2n)).toBe(-4n);
		expect(faBigFloorModulo(7n, -2n)).toBe(-1n);
		expect(faBigFloorDivide(-8n, 2n)).toBe(-4n);
		expect(faBigFloorModulo(-8n, 2n)).toBe(0n);
		expect(faBigFloorDivide(7n, 2n)).toBe(3n);
		expect(faBigFloorDivide(-(10n ** 30n) - 1n, 10n ** 15n)).toBe(-(10n ** 15n) - 1n);
		expect(faBigFloorModulo(-(10n ** 30n) - 1n, 10n ** 15n)).toBe(10n ** 15n - 1n);
		// Int64.minValue // -1 is 2^63: the backend checks it with faCheckWidth.
		expect(() => faCheckWidth(faBigFloorDivide(INT64_MIN, -1n), 64, true, "Int64")).toThrow(FaStop);
		expect(faBigFloorModulo(INT64_MIN, -1n)).toBe(0n);
	});

	test("Int32.minValue // -1 overflows at the sink (T-NUM-50)", () => {
		const q = faFloorDivide(-2147483648, -1);
		expect(q).toBe(2147483648);
		expect(() => faCheckRange(q, -2147483648, 2147483647, "Int32")).toThrow(FaStop);
	});
});

describe("exponentiation", () => {
	test("documented values (T-NUM-9, T-NUM-10)", () => {
		expect(faPower(2, faPower(3, 2))).toBe(512);
		expect(faPower(-2, 2)).toBe(4);
		expect(faPower(-3, 2)).toBe(9);
		expect(faPower(0, 0)).toBe(1);
		expect(faPower(5, 0)).toBe(1);
		expect(faPower(0, 5)).toBe(0);
		expect(faPower(1, 1e15)).toBe(1);
		expect(faPower(-1, 1e15)).toBe(1);
		expect(faPower(-1, 1e15 + 1)).toBe(-1);
		expect(faPower(-2, 3)).toBe(-8);
	});

	test("exact up to 2^53 − 1", () => {
		expect(faPower(3, 33)).toBe(5559060566555523);
		expect(BigInt(faPower(3, 33))).toBe(3n ** 33n);
		expect(faPower(2, 52)).toBe(2 ** 52);
		expect(faPower(-3, 33)).toBe(-5559060566555523);
		expect(faPower(7, 18)).toBe(1628413597910449);
		expect(BigInt(faPower(7, 18))).toBe(7n ** 18n);
		for (let base = 2; base < 40; base += 1) {
			for (let exponent = 0; exponent < 60; exponent += 1) {
				const exact = BigInt(base) ** BigInt(exponent);
				if (exact > BigInt(MAX)) {
					expect(() => faCheckInteger(faPower(base, exponent))).toThrow(FaStop);
					break;
				}
				expect(BigInt(faPower(base, exponent))).toBe(exact);
			}
		}
	});

	test("beyond the safe range, the check stops (T-NUM-88)", () => {
		expect(() => faCheckInteger(faPower(2, 60))).toThrow(FaStop);
		expect(() => faCheckInteger(faPower(2, 53))).toThrow(FaStop);
		expect(() => faCheckInteger(faPower(-2, 53))).toThrow(FaStop);
		expect(() => faCheckInteger(faPower(10, 400))).toThrow(FaStop);
		expect(faCheckInteger(faPower(-2, 52))).toBe(2 ** 52);
	});

	test("a negative exponent throws ArithmeticError", () => {
		expect(() => faPower(2, -1)).toThrow(ArithmeticError);
		expect(String(errorOf(() => faPower(2, -1)))).toBe(
			'ArithmeticError { message = "negative exponent: -1" }',
		);
		expect(() => faBigPower(2n, -1n)).toThrow(ArithmeticError);
	});

	test("bigints (T-NUM-27)", () => {
		expect(String(faBigPower(3n, 40n))).toBe("12157665459056928801");
		expect(faBigPower(0n, 0n)).toBe(1n);
		expect(faBigPower(-2n, 3n)).toBe(-8n);
	});
});

describe("conversions", () => {
	test("faConvertInteger (T-NUM-36)", () => {
		expect(faConvertInteger(3, -128, 127, "Int8")).toBe(3);
		expect(faConvertInteger(-128, -128, 127, "Int8")).toBe(-128);
		expect(() => faConvertInteger(300, -128, 127, "Int8")).toThrow(ArithmeticError);
		expect(String(errorOf(() => faConvertInteger(300, -128, 127, "Int8")))).toBe(
			`ArithmeticError { message = "300 doesn't fit in Int8" }`,
		);
		expect(() => faConvertInteger(-1, 0, 65535, "UInt16")).toThrow(ArithmeticError);
		expect(faConvertInteger(100, 0, 100, "Integer(max = 100)")).toBe(100);
		expect(() => faConvertInteger(101, -MAX, 100, "Integer(max = 100)")).toThrow(ArithmeticError);
		expect(faConvertInteger(2n ** 40n, -128n, 2n ** 63n, "Int64")).toBe(2n ** 40n);
		expect(Number(faConvertInteger(255n, 0, 255, "UInt8"))).toBe(255);
		expect(() => faConvertInteger(256n, 0, 255, "UInt8")).toThrow(ArithmeticError);
	});

	test("faBigToInteger", () => {
		expect(faBigToInteger(12n)).toBe(12);
		expect(faBigToInteger(-12n)).toBe(-12);
		expect(faBigToInteger(BigInt(MAX))).toBe(MAX);
		expect(faBigToInteger(-BigInt(MAX))).toBe(-MAX);
		expect(() => faBigToInteger(BigInt(MAX) + 1n)).toThrow(FaStop);
		expect(() => faBigToInteger(INT64_MAX)).toThrow(FaStop);
		expect(() => faBigToInteger(INT64_MAX + 1n)).toThrow(ArithmeticError);
		expect(() => faBigToInteger(INT64_MIN - 1n)).toThrow(ArithmeticError);
		expect(() => faBigToInteger(10n ** 22n)).toThrow(ArithmeticError);
	});

	test("faInt64ToInteger (types brief D2)", () => {
		expect(faInt64ToInteger(5n)).toBe(5);
		expect(faInt64ToInteger(BigInt(MAX))).toBe(MAX);
		expect(faInt64ToInteger(-BigInt(MAX))).toBe(-MAX);
		expect(() => faInt64ToInteger(BigInt(MAX) + 1n)).toThrow(FaStop);
		expect(() => faInt64ToInteger(INT64_MIN)).toThrow(FaStop);
	});

	test("Integer.round, floor, ceil, truncate (T-NUM-33, T-NUM-35)", () => {
		expect(faIntegerRound(2.7)).toBe(3);
		expect(faIntegerFloor(2.7)).toBe(2);
		expect(faIntegerRound(-2.5)).toBe(-3);
		expect(faIntegerRound(2.5)).toBe(3);
		expect(faIntegerRound(0.5)).toBe(1);
		expect(faIntegerRound(-0.5)).toBe(-1);
		expect(faIntegerRound(0.49999999999999994)).toBe(0);
		expect(faIntegerCeil(2.1)).toBe(3);
		expect(faIntegerCeil(-2.1)).toBe(-2);
		expect(faIntegerTruncate(-2.9)).toBe(-2);
		expect(faIntegerTruncate(2.9)).toBe(2);
		expect(Object.is(faIntegerRound(-0.4), 0)).toBe(true);
		expect(Object.is(faIntegerCeil(-0.5), 0)).toBe(true);
		expect(Object.is(faIntegerTruncate(-0.5), 0)).toBe(true);
	});

	test("Integer.round throws on NaN, infinities and out of range", () => {
		for (const convert of [faIntegerRound, faIntegerFloor, faIntegerCeil, faIntegerTruncate]) {
			expect(() => convert(NaN)).toThrow(ArithmeticError);
			expect(() => convert(Infinity)).toThrow(ArithmeticError);
			expect(() => convert(-Infinity)).toThrow(ArithmeticError);
			expect(() => convert(1e19)).toThrow(ArithmeticError);
			expect(() => convert(9223372036854775808)).toThrow(ArithmeticError);
			expect(() => convert(-1e19)).toThrow(ArithmeticError);
		}
		expect(String(errorOf(() => faIntegerRound(NaN)))).toBe(
			'ArithmeticError { message = "NaN is not a finite number" }',
		);
	});

	test("Integer.round stops between 2^53 and 2^63 on JavaScript", () => {
		expect(() => faIntegerRound(1e17)).toThrow(FaStop);
		expect(() => faIntegerFloor(2 ** 53)).toThrow(FaStop);
		expect(() => faIntegerFloor(-9223372036854775808)).toThrow(FaStop);
		expect(faIntegerFloor(MAX)).toBe(MAX);
		expect(faIntegerFloor(-MAX)).toBe(-MAX);
	});

	test("faFloatToInteger truncates and stops on NaN or out of range", () => {
		expect(faFloatToInteger(2.9)).toBe(2);
		expect(faFloatToInteger(-2.9)).toBe(-2);
		expect(Object.is(faFloatToInteger(-0.5), 0)).toBe(true);
		expect(faFloatToInteger(MAX)).toBe(MAX);
		expect(() => faFloatToInteger(NaN)).toThrow(FaStop);
		expect(() => faFloatToInteger(Infinity)).toThrow(FaStop);
		expect(() => faFloatToInteger(-Infinity)).toThrow(FaStop);
		expect(() => faFloatToInteger(2 ** 53)).toThrow(FaStop);
		expect(() => faFloatToInteger(-1e300)).toThrow(FaStop);
	});
});

describe("Integer.parse", () => {
	test("valid integers", () => {
		expect(faParseInteger("42")).toBe(42);
		expect(faParseInteger("-42")).toBe(-42);
		expect(faParseInteger("+42")).toBe(42);
		expect(faParseInteger("007")).toBe(7);
		expect(Object.is(faParseInteger("-0"), 0)).toBe(true);
		expect(faParseInteger("9007199254740991")).toBe(MAX);
		expect(faParseInteger("-9007199254740991")).toBe(-MAX);
	});

	test("invalid text throws ParseError", () => {
		for (const text of [
			"",
			" 42",
			"42 ",
			"4x",
			"0x2A",
			"1_000",
			"1.5",
			"1e3",
			"--1",
			"+",
			"-",
			"٤٢",
			"Infinity",
		]) {
			const error = errorOf(() => faParseInteger(text));
			expect(error).toBeInstanceOf(ParseError);
			expect(error.input).toBe(text);
		}
		expect(String(errorOf(() => faParseInteger("")))).toBe(
			'ParseError { input = "", message = "the text is empty" }',
		);
		expect(String(errorOf(() => faParseInteger("4x")))).toBe(
			'ParseError { input = "4x", message = "\\"4x\\" is not an integer" }',
		);
	});

	test("out of range", () => {
		expect(errorOf(() => faParseInteger("9007199254740992")).message).toBe(
			"9007199254740992 doesn't fit in Integer on JavaScript (±(2^53 − 1))",
		);
		expect(errorOf(() => faParseInteger("9223372036854775808")).message).toBe(
			"9223372036854775808 doesn't fit in Integer",
		);
		expect(faParseInteger("127", -128, 127, "Int8")).toBe(127);
		expect(errorOf(() => faParseInteger("128", -128, 127, "Int8")).message).toBe(
			"128 doesn't fit in Int8",
		);
	});

	test("bigints", () => {
		expect(faParseBigInteger("123456789012345678901234567890")).toBe(
			123456789012345678901234567890n,
		);
		expect(faParseBigInteger("9223372036854775807", INT64_MIN, INT64_MAX, "Int64")).toBe(INT64_MAX);
		expect(() => faParseBigInteger("9223372036854775808", INT64_MIN, INT64_MAX, "Int64")).toThrow(
			ParseError,
		);
		expect(() => faParseBigInteger("12n")).toThrow(ParseError);
	});
});

describe("wrapping", () => {
	test("UInt8 (T-NUM-15)", () => {
		expect(faWrappingAdd(250, 10, 8, false)).toBe(4);
		expect(faWrappingSub(0, 1, 8, false)).toBe(255);
		expect(faWrappingMul(16, 16, 8, false)).toBe(0);
		expect(faWrappingMul(255, 255, 8, false)).toBe(1);
		expect(faWrappingPow(2, 8, 8, false)).toBe(0);
		expect(faWrappingPow(3, 5, 8, false)).toBe(243);
		expect(faWrappingPow(3, 6, 8, false)).toBe(729 % 256);
	});

	test("Int8, Int16, Int32", () => {
		expect(faWrappingAdd(127, 1, 8, true)).toBe(-128);
		expect(faWrappingSub(-128, 1, 8, true)).toBe(127);
		expect(faWrappingMul(-128, -1, 8, true)).toBe(-128);
		expect(faWrappingAdd(32767, 1, 16, true)).toBe(-32768);
		expect(faWrappingAdd(65535, 1, 16, false)).toBe(0);
		expect(faWrappingAdd(2147483647, 1, 32, true)).toBe(-2147483648);
		expect(faWrappingMul(2147483647, 2147483647, 32, true)).toBe(1);
		expect(faWrappingMul(65536, 65536, 32, true)).toBe(0);
		expect(faWrappingMul(123456789, 987654321, 32, true)).toBe(
			Number(BigInt.asIntN(32, 123456789n * 987654321n)),
		);
		expect(faWrappingMul(4294967295, 4294967295, 32, false)).toBe(1);
		expect(faWrappingAdd(4294967295, 1, 32, false)).toBe(0);
		expect(faWrappingSub(0, 1, 32, false)).toBe(4294967295);
		expect(faWrappingPow(3, 40, 32, false)).toBe(Number(BigInt.asUintN(32, 3n ** 40n)));
		expect(faWrappingPow(-3, 41, 32, true)).toBe(Number(BigInt.asIntN(32, (-3n) ** 41n)));
		expect(faWrappingPow(7, 4000000000, 32, false)).toBe(
			Number(modularPower(7n, 4000000000n, 2n ** 32n)),
		);
	});

	test("Integer wraps on 64 bits then checks the safe range (D-51)", () => {
		expect(faWrappingAdd(1, 2, 64, true)).toBe(3);
		expect(faWrappingMul(-1, MAX, 64, true)).toBe(-MAX);
		// (2^53 − 1)^2 wrapped to 64 bits is −2^54 + 1: not safe, stops.
		expect(() => faWrappingMul(MAX, MAX, 64, true)).toThrow(FaStop);
		// 2^32 · 2^32 = 2^64 wraps to 0.
		expect(faWrappingMul(2 ** 32, 2 ** 32, 64, true)).toBe(0);
		expect(faWrappingPow(2, 64, 64, true)).toBe(0);
		expect(faWrappingPow(3, 2, 64, true)).toBe(9);
		expect(() => faWrappingAdd(MAX, 1, 64, true)).toThrow(FaStop);
	});

	test("bigint types wrap with asIntN and asUintN", () => {
		expect(faWrappingAdd(INT64_MAX, 1n, 64, true)).toBe(INT64_MIN);
		expect(faWrappingSub(0n, 1n, 64, false)).toBe(UINT64_MAX);
		expect(faWrappingMul(INT64_MAX, 2n, 64, true)).toBe(-2n);
		expect(faWrappingPow(3n, 100n, 64, false)).toBe(BigInt.asUintN(64, 3n ** 100n));
		expect(faWrappingPow(3n, 10n ** 18n, 64, false)).toBe(modularPower(3n, 10n ** 18n, 2n ** 64n));
		expect(faWrappingAdd(2n ** 127n - 1n, 1n, 128, true)).toBe(-(2n ** 127n));
	});

	test("a negative exponent throws ArithmeticError", () => {
		expect(() => faWrappingPow(2, -1, 8, true)).toThrow(ArithmeticError);
		expect(() => faWrappingPow(2n, -1n, 64, true)).toThrow(ArithmeticError);
		expect(() => faWrappingPow(2, -1, 64, true)).toThrow(ArithmeticError);
	});

	test("faWrap and faWrapBig", () => {
		expect(faWrap(256, 8, false)).toBe(0);
		expect(faWrap(-1, 8, false)).toBe(255);
		expect(faWrap(128, 8, true)).toBe(-128);
		expect(faWrap(-129, 8, true)).toBe(127);
		expect(faWrap(2 ** 32, 32, false)).toBe(0);
		expect(faWrap(2 ** 31, 32, true)).toBe(-(2 ** 31));
		expect(faWrap(MAX, 32, false)).toBe(4294967295);
		expect(faWrap(MAX, 16, true)).toBe(-1);
		expect(faWrapBig(2n ** 64n, 64, false)).toBe(0n);
		expect(faWrapBig(-1n, 128, false)).toBe(2n ** 128n - 1n);
	});
});

describe("saturating", () => {
	test("UInt8 (T-NUM-15)", () => {
		expect(faSaturatingAdd(250, 10, 0, 255)).toBe(255);
		expect(faSaturatingSub(5, 10, 0, 255)).toBe(0);
		expect(faSaturatingMul(16, 16, 0, 255)).toBe(255);
		expect(faSaturatingPow(2, 8, 0, 255)).toBe(255);
		expect(faSaturatingPow(2, 7, 0, 255)).toBe(128);
		expect(faSaturatingAdd(1, 2, 0, 255)).toBe(3);
	});

	test("signed and ranged types clamp to the receiver's bounds", () => {
		expect(faSaturatingAdd(100, 100, -128, 127)).toBe(127);
		expect(faSaturatingSub(-100, 100, -128, 127)).toBe(-128);
		expect(faSaturatingMul(-100, 100, -128, 127)).toBe(-128);
		expect(faSaturatingPow(-2, 9, -128, 127)).toBe(-128);
		expect(faSaturatingAdd(90, 20, 0, 100)).toBe(100);
		expect(faSaturatingMul(2147483647, 2147483647, -2147483648, 2147483647)).toBe(2147483647);
		expect(faSaturatingPow(10, 1000, 0, 4294967295)).toBe(4294967295);
		expect(faSaturatingPow(-10, 1001, -2147483648, 2147483647)).toBe(-2147483648);
	});

	test("Integer saturates at 64 bits, then stops beyond 2^53 on JavaScript", () => {
		expect(faSaturatingAdd(1, 2, INT64_MIN, INT64_MAX)).toBe(3);
		expect(faSaturatingMul(2 ** 26, 2 ** 26, INT64_MIN, INT64_MAX)).toBe(2 ** 52);
		expect(() => faSaturatingMul(MAX, MAX, INT64_MIN, INT64_MAX)).toThrow(FaStop);
		expect(() => faSaturatingAdd(MAX, 1, INT64_MIN, INT64_MAX)).toThrow(FaStop);
		expect(faSaturatingAdd(MAX, 1, -MAX, MAX)).toBe(MAX);
	});

	test("bigint types", () => {
		expect(faSaturatingAdd(INT64_MAX, 1n, INT64_MIN, INT64_MAX)).toBe(INT64_MAX);
		expect(faSaturatingSub(0n, 1n, 0n, UINT64_MAX)).toBe(0n);
		expect(faSaturatingPow(2n, 10n ** 9n, 0n, UINT64_MAX)).toBe(UINT64_MAX);
		expect(faSaturatingPow(-2n, 10n ** 9n + 1n, INT64_MIN, INT64_MAX)).toBe(INT64_MIN);
		expect(faSaturatingMul(3n, 4n, INT64_MIN, INT64_MAX)).toBe(12n);
	});

	test("a negative exponent throws ArithmeticError", () => {
		expect(() => faSaturatingPow(2, -1, -128, 127)).toThrow(ArithmeticError);
	});
});

describe("checked", () => {
	test("UInt8 (T-NUM-16)", () => {
		expect(() => faCheckedAdd(250, 10, 0, 255, "UInt8")).toThrow(ArithmeticError);
		expect(String(errorOf(() => faCheckedAdd(250, 10, 0, 255, "UInt8")))).toBe(
			`ArithmeticError { message = "integer overflow: 260 doesn't fit in UInt8" }`,
		);
		expect(faCheckedAdd(250, 5, 0, 255, "UInt8")).toBe(255);
		expect(() => faCheckedSub(0, 1, 0, 255, "UInt8")).toThrow(ArithmeticError);
		expect(faCheckedMul(15, 17, 0, 255, "UInt8")).toBe(255);
		expect(() => faCheckedMul(16, 16, 0, 255, "UInt8")).toThrow(ArithmeticError);
		expect(faCheckedPow(2, 7, 0, 255, "UInt8")).toBe(128);
		expect(() => faCheckedPow(2, 8, 0, 255, "UInt8")).toThrow(ArithmeticError);
		expect(() => faCheckedPow(2, -1, 0, 255, "UInt8")).toThrow(ArithmeticError);
	});

	test("Int32 products beyond 2^53 are decided exactly", () => {
		expect(() => faCheckedMul(2147483647, 2147483647, -2147483648, 2147483647, "Int32")).toThrow(
			ArithmeticError,
		);
		expect(faCheckedMul(-65536, 32768, -2147483648, 2147483647, "Int32")).toBe(-2147483648);
	});

	test("Integer throws beyond Int64, stops beyond 2^53 on JavaScript", () => {
		expect(faCheckedAdd(1, 2, INT64_MIN, INT64_MAX, "Integer")).toBe(3);
		expect(() => faCheckedMul(MAX, MAX, INT64_MIN, INT64_MAX, "Integer")).toThrow(ArithmeticError);
		expect(() => faCheckedAdd(MAX, 1, INT64_MIN, INT64_MAX, "Integer")).toThrow(FaStop);
		expect(() => faCheckedPow(2, 63, INT64_MIN, INT64_MAX, "Integer")).toThrow(ArithmeticError);
		expect(() => faCheckedPow(2, 62, INT64_MIN, INT64_MAX, "Integer")).toThrow(FaStop);
		expect(faCheckedPow(-2, 52, INT64_MIN, INT64_MAX, "Integer")).toBe(2 ** 52);
	});

	test("bigint types", () => {
		expect(faCheckedAdd(INT64_MAX - 1n, 1n, INT64_MIN, INT64_MAX, "Int64")).toBe(INT64_MAX);
		expect(() => faCheckedAdd(INT64_MAX, 1n, INT64_MIN, INT64_MAX, "Int64")).toThrow(
			ArithmeticError,
		);
		expect(() => faCheckedPow(2n, 10n ** 12n, 0n, UINT64_MAX, "UInt64")).toThrow(ArithmeticError);
	});

	test("faExactResult", () => {
		expect(faExactResult("+", MAX, 1)).toBe(BigInt(MAX) + 1n);
		expect(faExactResult("+", 1, 2)).toBe(3);
		expect(faExactResult("*", MAX, MAX)).toBe(BigInt(MAX) ** 2n);
		expect(faExactResult("**", 3, 33)).toBe(5559060566555523);
		expect(faExactResult("**", 0, 0)).toBe(1);
		expect(faExactResult("**", 0n, 5n)).toBe(0n);
		expect(faExactResult("**", -1n, 10n ** 20n + 1n)).toBe(-1n);
		expect(faExactResult("**", 2, 1000)).toBe(2n ** 256n);
		expect(faExactResult("**", -2, 1001)).toBe(-(2n ** 256n));
		expect(Object.is(faExactResult("*", 0, -5), 0)).toBe(true);
	});
});

describe("bitwise methods", () => {
	test("UInt8 (T-NUM-37, T-NUM-39)", () => {
		const a = 0b1100;
		const b = 0b1010;
		expect(faBitAnd(a, b, 8, false)).toBe(8);
		expect(faBitOr(a, b, 8, false)).toBe(14);
		expect(faBitXor(a, b, 8, false)).toBe(6);
		expect(faBitNot(a, 8, false)).toBe(243);
		expect(faShiftLeft(a, 2, 8, false)).toBe(48);
		expect(faShiftRight(a, 2, 8, false)).toBe(3);
		expect(faShiftLeft(0b1000_0000, 1, 8, false)).toBe(0);
		expect(faShiftRight(0b1000_0000, 7, 8, false)).toBe(1);
	});

	test("signed shifts are arithmetic (T-NUM-38)", () => {
		expect(faShiftRight(-8, 1, 8, true)).toBe(-4);
		expect(faShiftRight(-1, 7, 8, true)).toBe(-1);
		expect(faShiftLeft(64, 1, 8, true)).toBe(-128);
		expect(faShiftLeft(-1, 7, 8, true)).toBe(-128);
		expect(faBitNot(0, 8, true)).toBe(-1);
		expect(faBitAnd(-1, 0x7f, 8, true)).toBe(127);
		expect(faBitXor(-128, -1, 8, true)).toBe(127);
	});

	test("32-bit types", () => {
		expect(faBitNot(0, 32, false)).toBe(4294967295);
		expect(faBitAnd(4294967295, 4294967295, 32, false)).toBe(4294967295);
		expect(faBitOr(2147483648, 1, 32, false)).toBe(2147483649);
		expect(faShiftRight(4294967295, 31, 32, false)).toBe(1);
		expect(faShiftRight(-2147483648, 31, 32, true)).toBe(-1);
		expect(faShiftLeft(1, 31, 32, true)).toBe(-2147483648);
		expect(faShiftLeft(1, 31, 32, false)).toBe(2147483648);
		expect(faBitXor(0x0f0f0f0f, 0xffffffff, 32, false)).toBe(0xf0f0f0f0);
	});

	test("16-bit types", () => {
		expect(faBitNot(0, 16, false)).toBe(65535);
		expect(faShiftLeft(0xffff, 4, 16, false)).toBe(0xfff0);
		expect(faShiftLeft(0x4000, 1, 16, true)).toBe(-32768);
	});

	test("shift amounts outside 0..width throw ArithmeticError (D-50)", () => {
		expect(() => faShiftLeft(1, 8, 8, false)).toThrow(ArithmeticError);
		expect(() => faShiftLeft(1, -1, 8, false)).toThrow(ArithmeticError);
		expect(() => faShiftRight(1, 32, 32, true)).toThrow(ArithmeticError);
		expect(() => faShiftLeft(1, 64, 64, true)).toThrow(ArithmeticError);
		expect(() => faShiftLeft(1n, 64, 64, true)).toThrow(ArithmeticError);
		expect(() => faShiftRight(1n, 128, 128, false)).toThrow(ArithmeticError);
		expect(String(errorOf(() => faShiftLeft(1, 8, 8, false)))).toBe(
			'ArithmeticError { message = "shift amount 8 is outside 0..8" }',
		);
		expect(faShiftLeft(1, 7, 8, false)).toBe(128);
		expect(faShiftLeft(1, 0, 8, false)).toBe(1);
	});

	test("Integer goes through 64 bits, then the safe-integer check", () => {
		expect(faBitAnd(12, 10, 64, true)).toBe(8);
		expect(faBitOr(-8, 3, 64, true)).toBe(-5);
		expect(faBitXor(5, -1, 64, true)).toBe(-6);
		expect(() => faBitXor(MAX, -1, 64, true)).toThrow(FaStop);
		expect(faBitNot(5, 64, true)).toBe(-6);
		expect(faShiftRight(-8, 1, 64, true)).toBe(-4);
		expect(faShiftLeft(1, 52, 64, true)).toBe(2 ** 52);
		expect(() => faShiftLeft(1, 53, 64, true)).toThrow(FaStop);
		expect(() => faBitNot(MAX, 64, true)).toThrow(FaStop);
		expect(() => faBitAnd(-MAX, -2, 64, true)).toThrow(FaStop);
		expect(faShiftLeft(-1, 1, 64, true)).toBe(-2);
	});

	test("bigint types", () => {
		expect(faBitAnd(0xffn, 0x0fn, 64, false)).toBe(0x0fn);
		expect(faBitNot(0n, 64, false)).toBe(UINT64_MAX);
		expect(faBitNot(0n, 64, true)).toBe(-1n);
		expect(faShiftLeft(1n, 63, 64, true)).toBe(INT64_MIN);
		expect(faShiftLeft(1n, 63, 64, false)).toBe(2n ** 63n);
		expect(faShiftLeft(3n, 63, 64, false)).toBe(2n ** 63n);
		expect(faShiftRight(INT64_MIN, 63, 64, true)).toBe(-1n);
		expect(faShiftRight(UINT64_MAX, 63, 64, false)).toBe(1n);
		expect(faShiftRight(UINT64_MAX, 63n, 64, false)).toBe(1n);
		expect(faBitXor(2n ** 127n, 1n, 128, false)).toBe(2n ** 127n + 1n);
	});
});

describe("clamp", () => {
	test("integers, floats and bigints (T-NUM-40)", () => {
		expect(faClamp(15, 0, 10)).toBe(10);
		expect(faClamp(-5, 0, 10)).toBe(0);
		expect(faClamp(5, 0, 10)).toBe(5);
		expect(faClamp(1.5, 0.0, 1.0)).toBe(1);
		expect(faClamp(15n, 0n, 10n)).toBe(10n);
		expect(faClamp(NaN, 0, 1)).toBeNaN();
	});
});

/**
 * `base ** exponent % modulus` by squaring.
 * @param {bigint} base
 * @param {bigint} exponent
 * @param {bigint} modulus
 * @returns {bigint}
 */
function modularPower(base, exponent, modulus) {
	let result = 1n;
	let b = base % modulus;
	for (let e = exponent; e > 0n; e >>= 1n) {
		if (e & 1n) {
			result = (result * b) % modulus;
		}
		b = (b * b) % modulus;
	}
	return result;
}
