import { describe, expect, test } from "bun:test";

import { IndexError } from "./errors.js";
import {
	faBoundaries,
	faCharacters,
	faIsSimpleText,
	faStringCodePoints,
	faStringCompare,
	faStringCompareLocale,
	faStringEndsWith,
	faStringGet,
	faStringIncludes,
	faStringIndex,
	faStringIndexOf,
	faStringLastIndexOf,
	faStringLength,
	faStringPadEnd,
	faStringPadStart,
	faStringReplace,
	faStringReplaceAll,
	faStringSlice,
	faStringSplit,
	faStringStartsWith,
} from "./strings.js";

const FAMILY = "👨‍👩‍👧‍👦"; // man, ZWJ, woman, ZWJ, girl, ZWJ, boy: one cluster
const FRANCE = "🇫🇷";
const GERMANY = "🇩🇪";
const E_ACUTE = "é"; // e + combining acute accent
const THUMBS = "👍🏽"; // with a skin tone modifier

/**
 * The error thrown by `run`.
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

describe("simple text fast path", () => {
	test("ASCII without \\r is simple", () => {
		expect(faIsSimpleText("")).toBe(true);
		expect(faIsSimpleText("hello world\n\t~")).toBe(true);
		expect(faIsSimpleText("\x00\x7f")).toBe(true);
		expect(faIsSimpleText("a\r\nb")).toBe(false);
		expect(faIsSimpleText("a\rb")).toBe(false);
		expect(faIsSimpleText("é")).toBe(false);
		expect(faIsSimpleText("\x80")).toBe(false);
	});

	test("long texts (encoded by chunks) agree with the regular expression", () => {
		const reference = (/** @type {string} */ text) => /^[\x00-\x0c\x0e-\x7f]*$/.test(text);
		for (const length of [255, 256, 1000, 16383, 16384, 16385, 40000]) {
			const ascii = "abcdefghij;\n\t".repeat(Math.ceil(length / 13)).slice(0, length);
			expect(faIsSimpleText(ascii)).toBe(true);
			for (const position of [0, 1, length >> 1, 16383, 16384, length - 1]) {
				if (position >= length) {
					continue;
				}
				for (const intruder of ["\r", "\x80", "é", "\u{1F600}", "\ud800", "\udc00", "\uffff"]) {
					const text = ascii.slice(0, position) + intruder + ascii.slice(position + 1);
					expect([length, position, intruder, faIsSimpleText(text)]).toEqual([
						length,
						position,
						intruder,
						reference(text),
					]);
				}
			}
		}
	});
});

describe("length counts grapheme clusters", () => {
	test("documented values (T-STR-14)", () => {
		expect(faStringLength("héllo")).toBe(5);
		expect(faStringLength(FRANCE)).toBe(1);
		expect(faStringLength(E_ACUTE)).toBe(1);
	});

	test("ASCII, empty, emoji sequences, combining marks, CRLF", () => {
		expect(faStringLength("")).toBe(0);
		expect(faStringLength("hello")).toBe(5);
		expect(faStringLength(FAMILY)).toBe(1);
		expect(FAMILY.length).toBe(11);
		expect(faStringLength(THUMBS)).toBe(1);
		expect(faStringLength(`${FRANCE}${GERMANY}`)).toBe(2);
		expect(faStringLength(`a${FAMILY}b`)).toBe(3);
		expect(faStringLength("é̂̃")).toBe(1);
		expect(faStringLength("a\r\nb")).toBe(3);
		expect(faStringLength("a\rb")).toBe(3);
		expect(faStringLength("a\n\rb")).toBe(4);
		expect(faStringLength("한국어")).toBe(3);
		expect(faStringLength("각")).toBe(1); // conjoining jamo
		expect(faStringLength("😀")).toBe(1);
		expect(faStringLength("First line\nSecond line")).toBe(22);
	});

	test("characters (T-STR-20, T-STR-27)", () => {
		expect(faCharacters("aé🇫🇷")).toEqual(["a", "é", FRANCE]);
		expect(faCharacters("ab")).toEqual(["a", "b"]);
		expect(faCharacters("")).toEqual([]);
		expect(faCharacters(`x${FAMILY}${E_ACUTE}`)).toEqual(["x", FAMILY, E_ACUTE]);
		expect(faCharacters("a\r\nb")).toEqual(["a", "\r\n", "b"]);
	});

	test("boundaries are UTF-16 offsets", () => {
		expect(faBoundaries("abc")).toEqual([0, 1, 2, 3]);
		expect(faBoundaries(`a${E_ACUTE}b`)).toEqual([0, 1, 3, 4]);
		expect(faBoundaries("")).toEqual([0]);
	});
});

describe("indexing", () => {
	test("documented values (T-STR-15, T-STR-18)", () => {
		expect(faStringIndex("Fa", 0)).toBe("F");
		expect(faStringIndex("Fa", -1)).toBe("a");
		expect(faStringGet("abc", 10)).toBeUndefined();
		expect(errorOf(() => faStringIndex("abc", 10))).toBeInstanceOf(IndexError);
	});

	test("graphemes and negative indices", () => {
		const text = `a${FAMILY}${E_ACUTE}${FRANCE}`;
		expect(faStringIndex(text, 0)).toBe("a");
		expect(faStringIndex(text, 1)).toBe(FAMILY);
		expect(faStringIndex(text, 2)).toBe(E_ACUTE);
		expect(faStringIndex(text, 3)).toBe(FRANCE);
		expect(faStringIndex(text, -1)).toBe(FRANCE);
		expect(faStringIndex(text, -4)).toBe("a");
		const error = errorOf(() => faStringIndex(text, 4));
		expect(error).toBeInstanceOf(IndexError);
		expect(error.index).toBe(4);
		expect(error.length).toBe(4);
		expect(String(error)).toBe("IndexError { index = 4, length = 4 }");
		expect(errorOf(() => faStringIndex(text, -5)).index).toBe(-5);
		expect(errorOf(() => faStringIndex("", 0))).toBeInstanceOf(IndexError);
	});

	test("get", () => {
		expect(faStringGet(`${FRANCE}x`, 0)).toBe(FRANCE);
		expect(faStringGet(`${FRANCE}x`, -1)).toBe("x");
		expect(faStringGet(`${FRANCE}x`, 2)).toBeUndefined();
		expect(faStringGet(`${FRANCE}x`, -3)).toBeUndefined();
		expect(faStringGet("", 0)).toBeUndefined();
		expect(faStringGet("abc", 1)).toBe("b");
	});
});

describe("slicing", () => {
	test("documented values (T-STR-16, T-STR-17, T-STR-29)", () => {
		const s = "hello";
		expect(faStringSlice(s, 1, 3, false)).toBe("el");
		expect(faStringSlice(s, 1, 3, true)).toBe("ell");
		expect(faStringSlice(s, 1, undefined, false)).toBe("ello");
		expect(faStringSlice(s, undefined, 2, false)).toBe("he");
		expect(faStringSlice(s, -3, undefined, false)).toBe("llo");
		expect(faStringSlice(s, 3, 1, false)).toBe("");
		expect(faStringSlice("hello world", 0, 5, false)).toBe("hello");
		expect(faStringSlice("hello world", 6, undefined, false)).toBe("world");
	});

	test("bounds from -length to length, inclusive ends below length", () => {
		expect(faStringSlice("abc", 0, 3, false)).toBe("abc");
		expect(faStringSlice("abc", 3, 3, false)).toBe("");
		expect(faStringSlice("abc", -3, -1, false)).toBe("ab");
		expect(faStringSlice("abc", 0, -1, true)).toBe("abc");
		expect(faStringSlice("abc", 0, 2, true)).toBe("abc");
		expect(faStringSlice("abc", undefined, undefined, false)).toBe("abc");
		expect(errorOf(() => faStringSlice("abc", 0, 4, false))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faStringSlice("abc", 0, 3, true))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faStringSlice("abc", -4, undefined, false))).toBeInstanceOf(IndexError);
		expect(errorOf(() => faStringSlice("abc", 4, undefined, false)).index).toBe(4);
		expect(faStringSlice("abc", 5 - 5, 1, false)).toBe("a");
	});

	test("graphemes", () => {
		const text = `${FRANCE}${E_ACUTE}x${FAMILY}`;
		expect(faStringSlice(text, 0, 2, false)).toBe(`${FRANCE}${E_ACUTE}`);
		expect(faStringSlice(text, 1, -1, false)).toBe(`${E_ACUTE}x`);
		expect(faStringSlice(text, -1, undefined, false)).toBe(FAMILY);
		expect(faStringSlice(text, 2, 0, false)).toBe("");
		expect(errorOf(() => faStringSlice(text, 0, 5, false)).length).toBe(4);
	});
});

describe("searching never matches part of a grapheme cluster", () => {
	test("documented values (T-STR-23, T-STR-26)", () => {
		const t = "hello world";
		expect(faStringIncludes(t, "world")).toBe(true);
		expect(faStringStartsWith(t, "hello")).toBe(true);
		expect(faStringEndsWith(t, "world")).toBe(true);
		expect(faStringIndexOf(t, "world")).toBe(6);
		expect(faStringIndexOf(t, "planet")).toBeUndefined();
		expect(faStringIncludes(`${E_ACUTE}x`, "e")).toBe(false);
		expect(faStringIndexOf(`${FRANCE}${GERMANY}`, GERMANY)).toBe(1);
	});

	test("includes", () => {
		expect(faStringIncludes("", "")).toBe(true);
		expect(faStringIncludes("abc", "")).toBe(true);
		expect(faStringIncludes(E_ACUTE, "")).toBe(true);
		expect(faStringIncludes(`a${E_ACUTE}`, E_ACUTE)).toBe(true);
		expect(faStringIncludes(`a${E_ACUTE}`, "́")).toBe(false);
		expect(faStringIncludes(FAMILY, "👨")).toBe(false);
		expect(faStringIncludes(`👨 ${FAMILY}`, "👨")).toBe(true);
		expect(faStringIncludes(THUMBS, "👍")).toBe(false);
		// The regional indicators 🇷🇩 (Rwanda?) straddle 🇫🇷 and 🇩🇪.
		expect(faStringIncludes(`${FRANCE}${GERMANY}`, "\u{1F1F7}\u{1F1E9}")).toBe(false);
		expect(faStringIncludes("a\r\nb", "\r")).toBe(false);
		expect(faStringIncludes("a\r\nb", "\r\n")).toBe(true);
	});

	test("a rejected match doesn't hide a later valid one", () => {
		expect(faStringIndexOf(`${E_ACUTE}e`, "e")).toBe(1);
		expect(faStringIndexOf(`${FAMILY}👨`, "👨")).toBe(1);
		expect(faStringLastIndexOf(`e${E_ACUTE}`, "e")).toBe(0);
	});

	test("startsWith and endsWith", () => {
		expect(faStringStartsWith(E_ACUTE, "e")).toBe(false);
		expect(faStringStartsWith(`${E_ACUTE}x`, E_ACUTE)).toBe(true);
		expect(faStringStartsWith("abc", "")).toBe(true);
		expect(faStringStartsWith("abc", "abcd")).toBe(false);
		expect(faStringEndsWith(FAMILY, "👦")).toBe(false);
		expect(faStringEndsWith(`x${FAMILY}`, FAMILY)).toBe(true);
		expect(faStringEndsWith("abc", "")).toBe(true);
		expect(faStringEndsWith("é", "é")).toBe(true);
		expect(faStringEndsWith("a\r\n", "\n")).toBe(false);
	});

	test("indexOf and lastIndexOf count graphemes", () => {
		const text = `${FRANCE}ab${FAMILY}ab`;
		expect(faStringIndexOf(text, "ab")).toBe(1);
		expect(faStringLastIndexOf(text, "ab")).toBe(4);
		expect(faStringIndexOf(text, FAMILY)).toBe(3);
		expect(faStringLastIndexOf(text, "zz")).toBeUndefined();
		expect(faStringIndexOf(text, "")).toBe(0);
		expect(faStringLastIndexOf(text, "")).toBe(6);
		expect(faStringLastIndexOf("abcabc", "abc")).toBe(3);
		expect(faStringLastIndexOf("abcabc", "")).toBe(6);
		expect(faStringIndexOf("aaa", "aa")).toBe(0);
		expect(faStringLastIndexOf("aaa", "aa")).toBe(1);
	});
});

describe("replace and split", () => {
	test("documented values (T-STR-24, T-STR-27)", () => {
		expect(faStringReplace("hello-world", "-", " ")).toBe("hello world");
		expect(faStringReplaceAll("a-b-c", "-", " ")).toBe("a b c");
		expect(faStringSplit("a,b,c", ",")).toEqual(["a", "b", "c"]);
		expect(faStringSplit("a😀", "")).toEqual(["a", "😀"]);
	});

	test("the replacement is plain text", () => {
		expect(faStringReplace("a-b", "-", "$&$&")).toBe("a$&$&b");
		expect(faStringReplaceAll("a-b-c", "-", "$'")).toBe("a$'b$'c");
		expect(faStringReplace(`${E_ACUTE}-x`, "-", "$1")).toBe(`${E_ACUTE}$1x`);
	});

	test("graphemes", () => {
		expect(faStringReplace(`${E_ACUTE}e`, "e", "E")).toBe(`${E_ACUTE}E`);
		expect(faStringReplaceAll(`e${E_ACUTE}e`, "e", "E")).toBe(`E${E_ACUTE}E`);
		expect(faStringReplaceAll(`${FAMILY}👨`, "👨", "M")).toBe(`${FAMILY}M`);
		expect(faStringReplaceAll("aaa", "aa", "b")).toBe("ba");
		expect(faStringReplaceAll("abc", "x", "y")).toBe("abc");
		expect(faStringReplace("abc", "x", "y")).toBe("abc");
	});

	test("an empty search", () => {
		expect(faStringReplace("abc", "", "-")).toBe("-abc");
		expect(faStringReplaceAll("abc", "", "-")).toBe("-a-b-c-");
		expect(faStringReplaceAll(`${E_ACUTE}${FRANCE}`, "", "-")).toBe(`-${E_ACUTE}-${FRANCE}-`);
		expect(faStringReplaceAll("", "", "-")).toBe("-");
	});

	test("split", () => {
		expect(faStringSplit("a,b,", ",")).toEqual(["a", "b", ""]);
		expect(faStringSplit("", ",")).toEqual([""]);
		expect(faStringSplit("", "")).toEqual([]);
		expect(faStringSplit("abc", ",")).toEqual(["abc"]);
		expect(faStringSplit("a, b, c", ", ")).toEqual(["a", "b", "c"]);
		expect(faStringSplit(`${E_ACUTE}e${E_ACUTE}`, "e")).toEqual([E_ACUTE, E_ACUTE]);
		expect(faStringSplit(`x${FAMILY}y`, "")).toEqual(["x", FAMILY, "y"]);
		expect(faStringSplit(`${FRANCE},${GERMANY}`, ",")).toEqual([FRANCE, GERMANY]);
	});
});

describe("padding counts graphemes", () => {
	test("documented values (T-STR-25)", () => {
		expect(faStringPadStart("7", 3, "0")).toBe("007");
		expect(faStringPadEnd("7", 3)).toBe("7  ");
		expect(faStringPadStart("7", 3)).toBe("  7");
	});

	test("graphemes in the text and in the filler", () => {
		expect(faStringPadStart(E_ACUTE, 3, "*")).toBe(`**${E_ACUTE}`);
		expect(faStringPadEnd(FRANCE, 3, "ab")).toBe(`${FRANCE}ab`);
		expect(faStringPadStart("x", 4, FAMILY)).toBe(`${FAMILY}${FAMILY}${FAMILY}x`);
		expect(faStringPadEnd("x", 4, `${FRANCE}-`)).toBe(`x${FRANCE}-${FRANCE}`);
		expect(faStringPadStart("abc", 2, "0")).toBe("abc");
		expect(faStringPadStart(E_ACUTE, 1, "*")).toBe(E_ACUTE);
		expect(faStringPadStart("é", 5, "")).toBe("é");
		expect(faStringPadStart("ab", 5, "xyz")).toBe("xyzab");
		expect(faStringPadEnd("ab", 6, "xyz")).toBe("abxyzx");
	});
});

describe("code points and comparison", () => {
	test("codePoints (T-STR-27)", () => {
		expect(faStringCodePoints("abc")).toEqual([97, 98, 99]);
		expect(faStringCodePoints("a😀")).toEqual([97, 0x1f600]);
		expect(faStringCodePoints(E_ACUTE)).toEqual([0x65, 0x301]);
		expect(faStringCodePoints("")).toEqual([]);
	});

	test("comparison is by code point (T-STR-21)", () => {
		expect(faStringCompare("abc", "abd")).toBe(-1);
		expect(faStringCompare("Z", "a")).toBe(-1);
		expect(faStringCompare("￿", "\u{10000}")).toBe(-1);
		expect("￿" < "\u{10000}").toBe(false);
		expect(faStringCompare("\u{10000}", "￿")).toBe(1);
		expect(faStringCompare("abc", "abc")).toBe(0);
		expect(faStringCompare("ab", "abc")).toBe(-1);
		expect(faStringCompare("abc", "ab")).toBe(1);
		expect(faStringCompare("", "")).toBe(0);
		expect(faStringCompare("😀a", "😀b")).toBe(-1);
		expect(faStringCompare("😀", "😀")).toBe(0);
		expect(faStringCompare("\u{1F600}", "\u{1F601}")).toBe(-1);
		expect(faStringCompare("😀", "😀x")).toBe(-1);
		expect(faStringCompare("", "😀")).toBe(-1);
		expect(faStringCompare("😀", "")).toBe(1);
	});

	test("comparison agrees with the code points on surrogates, paired or not", () => {
		/** The order of the code points (a lone surrogate is its own code point). */
		const reference = (/** @type {string} */ a, /** @type {string} */ b) => {
			const x = Array.from(a, (c) => /** @type {number} */ (c.codePointAt(0)));
			const y = Array.from(b, (c) => /** @type {number} */ (c.codePointAt(0)));
			for (let index = 0; index < Math.min(x.length, y.length); index++) {
				if (x[index] !== y[index]) {
					return x[index] < y[index] ? -1 : 1;
				}
			}
			return x.length < y.length ? -1 : x.length > y.length ? 1 : 0;
		};
		const pieces = ["a", "z", "퟿", "\ud800", "\udbff", "\udc00", "\udfff", "", "￿"];
		const all = [...pieces, "\u{10000}", "\u{1F600}", "\u{10FFFF}"];
		const strings = [""];
		for (const first of all) {
			strings.push(first);
			for (const second of all) {
				strings.push(first + second);
			}
		}
		for (const a of strings) {
			for (const b of strings) {
				expect([a, b, faStringCompare(a, b)]).toEqual([a, b, reference(a, b)]);
			}
		}
	});

	test("equality has no normalization (T-STR-22)", () => {
		expect("é" === E_ACUTE).toBe(false);
	});

	test("compareLocale (T-STR-28)", () => {
		expect(faStringCompareLocale("a", "b")).toBeLessThan(0);
		expect(faStringCompareLocale("b", "a")).toBe(1);
		expect(faStringCompareLocale("a", "a")).toBe(0);
		expect(faStringCompareLocale("a", "B", "en")).toBe(-1);
	});
});

describe("the JavaScript methods Fa uses directly", () => {
	test("case mapping, trimming and repeating (T-STR-25, T-STR-30)", () => {
		expect("ß".toUpperCase()).toBe("SS");
		expect("  padded  ".trim()).toBe("padded");
		expect("ab".repeat(3)).toBe("ababab");
		expect("a" + "b" + "c").toBe("abc");
		expect(["a", "b"].join(", ")).toBe("a, b");
	});
});
