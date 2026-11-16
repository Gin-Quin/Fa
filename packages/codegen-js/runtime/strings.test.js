import { describe, expect, test } from "bun:test";

import { faIndex, faSliceBounds } from "./collections.js";
import { IndexError } from "./errors.js";
import {
	faCharacters,
	faCodePointCount,
	faFromUtf8,
	faInvalidUtf8,
	faIsWellFormed,
	faStringCodePoints,
	faStringCompareLocale,
	faUtf8Encoder,
	faUtf8Length,
	faUtf8LossyDecoder,
	faUtf8Values,
	Utf8Error,
} from "./strings.js";

const FRANCE = "🇫🇷";
const GERMANY = "🇩🇪";
const ACCENT = "e\u0301";
const FAMILY = "👨‍👩‍👧‍👦";

// Native operations have no Fa wrapper. Exercise their Fa bounds boundary
// here; generated-program tests cover lowering and evaluation order.
describe("UTF-16 string operations", () => {
	test("length counts code units, including CRLF and combining marks", () => {
		for (const [text, units] of [
			["", 0],
			["hello", 5],
			["héllo", 5],
			["日本語", 3],
			["😀", 2],
			[FRANCE, 4],
			[ACCENT, 2],
			["\r\n", 2],
			[FAMILY, 11],
		]) {
			expect(text.length).toBe(units);
		}
	});
	test("checked indexing and negative indices", () => {
		expect(faIndex("hé😀", 0)).toBe("h");
		expect(faIndex("hé😀", -1).charCodeAt(0)).toBe(0xde00);
		expect(faIndex("hé😀", 2).charCodeAt(0)).toBe(0xd83d);
		for (const index of [-5, 4]) expect(() => faIndex("hé😀", index)).toThrow(IndexError);
		expect(() => faIndex("", 0)).toThrow(IndexError);
	});
	test("get uses at and returns undefined outside the string", () => {
		expect("hé😀".at(-1)?.charCodeAt(0)).toBe(0xde00);
		expect("hé😀".at(4)).toBeUndefined();
		expect("".at(0)).toBeUndefined();
	});
	test("slices can cut and rejoin a surrogate pair", () => {
		const face = "😀";
		const half = face.slice(0, 1);
		expect(half.length).toBe(1);
		expect(faIsWellFormed(half)).toBe(false);
		expect(half + face.slice(1)).toBe(face);
		expect(face.slice(1, 0)).toBe("");
		let rebuilt = half;
		rebuilt += face[1];
		expect(rebuilt).toBe(face);
	});
	test("slice bounds include negative and inverted ranges", () => {
		expect(faSliceBounds(4, -2, -1, true)).toEqual([2, 4]);
		expect(faSliceBounds(4, 3, 1, false)).toEqual([3, 3]);
		expect(faSliceBounds(0, undefined, undefined, false)).toEqual([0, 0]);
		for (const bounds of [
			[-5, 2, false],
			[0, 5, false],
			[0, 4, true],
		]) {
			expect(() => faSliceBounds(4, ...bounds)).toThrow(IndexError);
		}
	});
	test("searches can match a combining mark or half a pair", () => {
		expect(ACCENT.includes("e")).toBe(true);
		expect(ACCENT.startsWith("e")).toBe(true);
		expect(ACCENT.endsWith("\u0301")).toBe(true);
		expect("😀".includes("😀"[0])).toBe(true);
		expect((FRANCE + GERMANY).indexOf(GERMANY)).toBe(4);
		expect("é日😀x😀".lastIndexOf("😀")).toBe(5);
		expect("".indexOf("")).toBe(0);
		expect("é".indexOf("x")).toBe(-1);
	});
	test("split and replace use code units", () => {
		expect("a😀".split("").length).toBe(3);
		expect("".split("")).toEqual([]);
		expect(ACCENT.replace("e", "a")).toBe("a\u0301");
		expect("😀".replaceAll("😀"[0], "a")).toBe("a\ude00");
		expect("a-b-c".replaceAll("-", " ")).toBe("a b c");
		expect("ab".replaceAll("", "-")).toBe("-a-b-");
	});
	test("padding counts code units and may cut a filler pair", () => {
		expect("é".padStart(3, "*")).toBe("**é");
		expect(FRANCE.padEnd(5, "ab")).toBe(`${FRANCE}a`);
		expect("x".padStart(2, "😀")).toBe("\ud83dx");
		expect("ab".padEnd(6, "xyz")).toBe("abxyzx");
		expect("é".padStart(5, "")).toBe("é");
	});
	test("order is by units and equality has no normalization", () => {
		expect("\uffff" < "\u{10000}").toBe(false);
		expect("😀" < "\ue000").toBe(true);
		expect("é" === ACCENT).toBe(false);
	});
	test("case mapping, trimming, repeating, joining", () => {
		expect("ß".toUpperCase()).toBe("SS");
		expect("  é  ".trim()).toBe("é");
		expect("日".repeat(3)).toBe("日日日");
		expect(["é", "😀"].join(", ")).toBe("é, 😀");
	});
});

describe("explicit Unicode views", () => {
	test("characters alone segments graphemes", () => {
		expect(faCharacters(FRANCE + GERMANY)).toEqual([FRANCE, GERMANY]);
		expect(faCharacters(ACCENT + FAMILY + "👍🏽\r\n")).toEqual([ACCENT, FAMILY, "👍🏽", "\r\n"]);
		expect(faCharacters("abc")).toEqual(["a", "b", "c"]);
		expect(faCharacters("")).toEqual([]);
	});
	test("codePoints and the count agree, including lone surrogates", () => {
		for (const text of [
			"",
			"abc",
			ACCENT,
			FRANCE,
			FAMILY,
			"é日😀",
			"\ud800x\udc00",
			"\ud800\udc00",
		]) {
			const points = Array.from(text, (point) => point.codePointAt(0));
			expect(faStringCodePoints(text)).toEqual(points);
			expect(faCodePointCount(text)).toBe(points.length);
		}
	});
	test("well-formed fallback agrees with the native method", () => {
		const pieces = ["", "a", "é", "日", "😀", "\ud800", "\udbff", "\udc00", "\udfff"];
		for (const a of pieces)
			for (const b of pieces) {
				expect(faIsWellFormed(a + b)).toBe((a + b).isWellFormed());
			}
	});
	test("compareLocale", () => {
		expect(faStringCompareLocale("a", "b")).toBe(-1);
		expect(faStringCompareLocale("b", "a")).toBe(1);
		expect(faStringCompareLocale("a", "a")).toBe(0);
	});
});

describe("UTF-8", () => {
	test("encoder, browser length and allocation-free iterator agree", () => {
		for (const text of [
			"",
			"abc",
			"héllo",
			"日本語",
			"😀",
			FRANCE,
			FAMILY,
			ACCENT,
			"\ufeff\0\r\n",
			"\ud800",
			"\udc00",
			"a\ud800b\udc00",
			"😀".repeat(200),
		]) {
			const bytes = faUtf8Encoder.encode(text);
			expect(bytes).toBeInstanceOf(Uint8Array);
			expect(faUtf8Length(text)).toBe(bytes.length);
			expect(faUtf8Length(text)).toBe(Buffer.byteLength(text));
			expect([...faUtf8Values(text)]).toEqual([...bytes]);
		}
	});
	test("lone surrogates encode as U+FFFD", () => {
		expect([...faUtf8Encoder.encode("😀"[0])]).toEqual([239, 191, 189]);
	});
	test("strict decoding preserves BOM and accepts all scalar boundaries", () => {
		const text = "\ufeff\0\u007f\u0080\u07ff\u0800\ud7ff\ue000\uffff\u{10000}\u{10ffff}";
		const bytes = faUtf8Encoder.encode(text);
		expect(faFromUtf8(bytes)).toBe(text);
		expect(faUtf8LossyDecoder.decode(bytes)).toBe(text);
		expect(faInvalidUtf8(bytes)).toBe(-1);
	});
	test("malformed sequences at the start, middle and truncated end", () => {
		const invalid = [
			[255],
			[128],
			[192, 128],
			[193, 191],
			[224, 128, 128],
			[237, 160, 128],
			[240, 128, 128, 128],
			[244, 144, 128, 128],
			[245, 128, 128, 128],
			[194],
			[226, 130],
			[240, 159, 152],
			[226, 65, 128],
			[240, 159, 65, 128],
		];
		for (const sequence of invalid)
			for (const prefix of [[], [97], [195, 169, 230, 151, 165]]) {
				const bytes = new Uint8Array([...prefix, ...sequence]);
				expect(faInvalidUtf8(bytes)).toBe(prefix.length);
				expect(() => faFromUtf8(bytes)).toThrow(Utf8Error);
				try {
					faFromUtf8(bytes);
				} catch (error) {
					expect(error.position).toBe(prefix.length);
				}
			}
	});
	test("lossy decoding replaces invalid bytes and preserves a BOM", () => {
		expect(faUtf8LossyDecoder.decode(new Uint8Array([239, 187, 191, 104, 255]))).toBe("\ufeffh�");
		expect(faUtf8LossyDecoder.decode(new Uint8Array([237, 160, 128]))).toBe("���");
	});
});
