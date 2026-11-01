import { describe, expect, test } from "bun:test";

import {
	faFloat32ToString,
	faFloatFloorDivide,
	faFloatFloorModulo,
	faPercentageToString,
	faRound,
} from "./floats.js";

describe("Float64 printing is String(x) (D-5)", () => {
	test("golden strings of the values brief (T-NUM-2, T-NUM-6, T-NUM-22..25)", () => {
		expect(String(0.5)).toBe("0.5");
		expect(String(1.5e9)).toBe("1500000000");
		expect(String(2.5e-3)).toBe("0.0025");
		expect(String(1e6)).toBe("1000000");
		expect(String(22 / 7)).toBe("3.142857142857143");
		expect(String(4 / 2)).toBe("2");
		expect(String(0.1 + 0.2)).toBe("0.30000000000000004");
		expect(String(1e21)).toBe("1e+21");
		expect(String(1e-7)).toBe("1e-7");
		expect(String(123456789012345680000.0)).toBe("123456789012345680000");
		expect(String(1 / 0)).toBe("Infinity");
		expect(String(-1 / 0)).toBe("-Infinity");
		expect(String(0 / 0)).toBe("NaN");
		expect(String(-0)).toBe("0");
		expect(`${-0}`).toBe("0");
	});

	test("NaN and -0 equality (T-NUM-23)", () => {
		const n = 0 / 0;
		expect(n === n).toBe(false);
		expect(-0 === 0).toBe(true);
	});
});

describe("faRound: halves away from zero", () => {
	test("documented values (T-NUM-34)", () => {
		expect(faRound(2.5)).toBe(3);
		expect(faRound(-2.5)).toBe(-3);
		expect(faRound(2.4)).toBe(2);
		expect(faRound(-2.4)).toBe(-2);
		expect(faRound(0.5)).toBe(1);
		expect(faRound(-0.5)).toBe(-1);
		expect(faRound(1.5)).toBe(2);
		expect(faRound(-1.5)).toBe(-2);
	});

	test("edge cases", () => {
		expect(faRound(0.49999999999999994)).toBe(0);
		expect(faRound(-0.49999999999999994)).toBe(-0);
		expect(Object.is(faRound(-0.4), -0)).toBe(true);
		expect(Object.is(faRound(0), 0)).toBe(true);
		expect(Object.is(faRound(-0), -0)).toBe(true);
		expect(faRound(NaN)).toBeNaN();
		expect(faRound(Infinity)).toBe(Infinity);
		expect(faRound(-Infinity)).toBe(-Infinity);
		expect(faRound(2 ** 51 + 0.5)).toBe(2 ** 51 + 1);
		expect(faRound(-(2 ** 51) - 0.5)).toBe(-(2 ** 51) - 1);
		expect(faRound(4503599627370497)).toBe(4503599627370497);
		expect(faRound(1e300)).toBe(1e300);
		expect(faRound(Math.fround(2.5))).toBe(3);
	});
});

describe("float floor division and remainder", () => {
	test("documented values (T-NUM-8)", () => {
		expect(faFloatFloorDivide(7.5, 2.0)).toBe(3);
		expect(faFloatFloorModulo(-7.5, 2.0)).toBe(0.5);
		expect(faFloatFloorModulo(7.5, 2.0)).toBe(1.5);
		expect(faFloatFloorModulo(7.5, -2.0)).toBe(-0.5);
		expect(faFloatFloorDivide(-7.5, 2.0)).toBe(-4);
	});

	test("a zero divisor gives infinities and NaN, never throws", () => {
		expect(faFloatFloorDivide(1, 0)).toBe(Infinity);
		expect(faFloatFloorDivide(-1, 0)).toBe(-Infinity);
		expect(faFloatFloorDivide(0, 0)).toBeNaN();
		expect(faFloatFloorModulo(1, 0)).toBeNaN();
		expect(faFloatFloorModulo(NaN, 2)).toBeNaN();
	});

	test("Float32 results are rounded by the caller", () => {
		const a = Math.fround(7.3);
		const b = Math.fround(2.1);
		const r = Math.fround(faFloatFloorModulo(a, b));
		expect(r).toBe(Math.fround(a - Math.floor(a / b) * b));
	});
});

describe("faFloat32ToString: shortest Float32 round trip (D-5)", () => {
	test("golden strings (T-NUM-26)", () => {
		expect(faFloat32ToString(Math.fround(0.1))).toBe("0.1");
		expect(String(Math.fround(0.1))).toBe("0.10000000149011612");
		expect(faFloat32ToString(Math.fround(0.5))).toBe("0.5");
		expect(faFloat32ToString(Math.fround(1))).toBe("1");
		expect(faFloat32ToString(Math.fround(3.14159))).toBe("3.14159");
		expect(faFloat32ToString(Math.fround(16777216))).toBe("16777216");
		expect(faFloat32ToString(Math.fround(1e21))).toBe("1e+21");
		expect(faFloat32ToString(Math.fround(1e-7))).toBe("1e-7");
		expect(faFloat32ToString(Math.fround(-2.5))).toBe("-2.5");
		expect(faFloat32ToString(Math.fround(1 / 3))).toBe("0.33333334");
		expect(faFloat32ToString(Math.fround(3.4028234663852886e38))).toBe("3.4028235e+38");
		expect(faFloat32ToString(Math.fround(1.401298464324817e-45))).toBe("1e-45");
		expect(faFloat32ToString(Math.fround(1.1754943508222875e-38))).toBe("1.1754944e-38");
	});

	test("special values", () => {
		expect(faFloat32ToString(NaN)).toBe("NaN");
		expect(faFloat32ToString(Infinity)).toBe("Infinity");
		expect(faFloat32ToString(-Infinity)).toBe("-Infinity");
		expect(faFloat32ToString(-0)).toBe("0");
		expect(faFloat32ToString(0)).toBe("0");
	});

	test("every printed value reads back as the same Float32, and is short", () => {
		let seed = 12345;
		const random = () => {
			seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
			return seed;
		};
		const bits = new Uint32Array(1);
		const view = new Float32Array(bits.buffer);
		for (let index = 0; index < 20000; index += 1) {
			bits[0] = random();
			const value = view[0];
			if (!Number.isFinite(value)) {
				continue;
			}
			const text = faFloat32ToString(value);
			expect(Math.fround(Number(text))).toBe(value);
			const digits = text
				.replace(/^-/, "")
				.replace(/e.*$/, "")
				.replace(".", "")
				.replace(/^0+|0+$/g, "");
			expect(digits.length).toBeLessThanOrEqual(9);
		}
	});
});

describe("faPercentageToString (D-6)", () => {
	test("golden strings (T-NUM-29, T-NUM-31, T-NUM-32, T-NUM-56)", () => {
		expect(faPercentageToString(0.5)).toBe("50%");
		expect(faPercentageToString(0.125)).toBe("12.5%");
		expect(faPercentageToString(0.2 + 0.3)).toBe("50%");
		expect(faPercentageToString(0.5 * 0.5)).toBe("25%");
		expect(faPercentageToString((30 + 10) / 100)).toBe("40%");
		expect(faPercentageToString(10 / 100)).toBe("10%");
		expect(faPercentageToString((1 + 2) / 100)).toBe("3%");
		expect(faPercentageToString(-0.5)).toBe("-50%");
	});

	test("rounding to 15 significant digits", () => {
		expect(String(0.07 * 100)).toBe("7.000000000000001");
		expect(String(0.29 * 100)).toBe("28.999999999999996");
		expect(faPercentageToString(0.1)).toBe("10%");
		expect(faPercentageToString(0.07)).toBe("7%");
		expect(faPercentageToString(0.29)).toBe("29%");
		expect(faPercentageToString(0.333)).toBe("33.3%");
		expect(faPercentageToString(1 / 3)).toBe("33.3333333333333%");
		expect(faPercentageToString(1e6 / 100)).toBe("1000000%");
		expect(faPercentageToString(1e20)).toBe("1e+22%");
		expect(faPercentageToString(1e-9)).toBe("1e-7%");
	});

	test("special values", () => {
		expect(faPercentageToString(0)).toBe("0%");
		expect(faPercentageToString(-0)).toBe("0%");
		expect(faPercentageToString(NaN)).toBe("NaN%");
		expect(faPercentageToString(Infinity)).toBe("Infinity%");
		expect(faPercentageToString(-Infinity)).toBe("-Infinity%");
	});
});
