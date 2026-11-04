// Strings (values brief section 7, D-42, D-43; operations/string).
//
// A `String` (and a `Character`) is a JavaScript string. Lengths, positions
// and searches count grapheme clusters, through one shared `Intl.Segmenter`,
// with a fast path for ASCII text without `\r` (where every code unit is a
// grapheme cluster: `\r\n` is one cluster). A search never matches part of a
// grapheme cluster. `==` is `===` (no normalization), `+` is `+`, and the
// methods that are the same as JavaScript's (`toUpperCase`, `trim`, `repeat`,
// `join`...) are called directly.

import { faNormalizeIndex, faSliceBounds } from "./collections.js";
import { IndexError } from "./errors.js";

// @fa-helper faSegmenter
/** @type {Intl.Segmenter | undefined} */
let faSegmenterInstance;

/**
 * The shared grapheme segmenter, created on first use.
 * @returns {Intl.Segmenter}
 */
export function faSegmenter() {
	if (faSegmenterInstance === undefined) {
		faSegmenterInstance = new Intl.Segmenter("und", { granularity: "grapheme" });
	}
	return faSegmenterInstance;
}

// @fa-helper faIsSimpleText
/** The encoder and the buffer of `faIsSimpleText` (created at the first long text). */
let faAsciiEncoder;
/** @type {Uint8Array} */
let faAsciiBuffer;
/** The last long text checked, and its answer: a loop slicing one text checks it once. */
let faSimpleLastText = "";
let faSimpleLastAnswer = true;

/**
 * Whether every UTF-16 code unit of the text is a grapheme cluster: ASCII
 * without `\r` (`\r\n` is a single cluster). A long text is encoded in UTF-8
 * by chunks of 16 KiB (`encodeInto` is native and vectorized, about fifteen
 * times faster than the regular expression): a chunk is ASCII when each of
 * its code units took one byte.
 * @param {string} text
 * @returns {boolean}
 */
export function faIsSimpleText(text) {
	if (text.length < 256) {
		return /^[\x00-\x0c\x0e-\x7f]*$/.test(text);
	}
	if (text === faSimpleLastText) {
		return faSimpleLastAnswer;
	}
	if (faAsciiEncoder === undefined) {
		faAsciiEncoder = new TextEncoder();
		faAsciiBuffer = new Uint8Array(16384);
	}
	let simple = true;
	for (let start = 0; start < text.length && simple; start += 16384) {
		const chunk = text.length <= 16384 ? text : text.slice(start, start + 16384);
		const { read, written } = faAsciiEncoder.encodeInto(chunk, faAsciiBuffer);
		simple = read === chunk.length && written === chunk.length;
	}
	faSimpleLastText = text;
	faSimpleLastAnswer = simple && text.indexOf("\r") === -1;
	return faSimpleLastAnswer;
}

// @fa-helper faCharacters deps: faSegmenter faIsSimpleText
/**
 * `text.characters()`: the grapheme clusters of the text. Also what a `for`
 * over a string iterates.
 * @param {string} text
 * @returns {string[]}
 */
export function faCharacters(text) {
	if (faIsSimpleText(text)) {
		return text.split("");
	}
	const characters = [];
	for (const { segment } of faSegmenter().segment(text)) {
		characters.push(segment);
	}
	return characters;
}

// @fa-helper faBoundaries deps: faSegmenter
/**
 * The UTF-16 offsets of the grapheme cluster boundaries, from `0` to
 * `text.length` included.
 * @param {string} text
 * @returns {number[]}
 */
export function faBoundaries(text) {
	const boundaries = [];
	for (const { index } of faSegmenter().segment(text)) {
		boundaries.push(index);
	}
	boundaries.push(text.length);
	return boundaries;
}

// @fa-helper faStringLength deps: faSegmenter faIsSimpleText
/**
 * `text.length`: the number of grapheme clusters.
 * @param {string} text
 * @returns {number}
 */
export function faStringLength(text) {
	if (faIsSimpleText(text)) {
		return text.length;
	}
	let length = 0;
	for (const _ of faSegmenter().segment(text)) {
		length += 1;
	}
	return length;
}

// @fa-helper faStringIndex deps: faCharacters faIsSimpleText faNormalizeIndex
/**
 * `text[index]` with an unproven index: the `Character` at that grapheme
 * position, negative indices counting from the end; throws `IndexError`.
 * @param {string} text
 * @param {number} index
 * @returns {string}
 */
export function faStringIndex(text, index) {
	if (faIsSimpleText(text)) {
		return text[faNormalizeIndex(text.length, index)];
	}
	const characters = faCharacters(text);
	return characters[faNormalizeIndex(characters.length, index)];
}

// @fa-helper faStringGet deps: faCharacters faIsSimpleText
/**
 * `text.get(index)`: the `Character` at that position, or `none`
 * (`undefined`); negative indices count from the end.
 * @param {string} text
 * @param {number} index
 * @returns {string | undefined}
 */
export function faStringGet(text, index) {
	const characters = faIsSimpleText(text) ? text : faCharacters(text);
	const position = index < 0 ? index + characters.length : index;
	return position >= 0 && position < characters.length ? characters[position] : undefined;
}

// @fa-helper faStringSlice deps: faCharacters faIsSimpleText faSliceBounds
/**
 * `text[start..end]` (`inclusive` for `..=`): a copy of the grapheme
 * clusters in the range. A missing bound is `undefined`. Bounds go from
 * `-length` to `length`; outside, throws `IndexError`; a start after the end
 * gives `""` (D-42).
 * @param {string} text
 * @param {number | undefined} start
 * @param {number | undefined} end
 * @param {boolean} inclusive
 * @returns {string}
 */
export function faStringSlice(text, start, end, inclusive) {
	if (faIsSimpleText(text)) {
		const [from, to] = faSliceBounds(text.length, start, end, inclusive);
		return text.substring(from, to);
	}
	const characters = faCharacters(text);
	const [from, to] = faSliceBounds(characters.length, start, end, inclusive);
	return characters.slice(from, to).join("");
}

// @fa-helper faFindMatches deps: faBoundaries faIsSimpleText
/**
 * The UTF-16 offsets of the matches of `search` in `text` whose two ends are
 * grapheme cluster boundaries, without overlaps, from the start (or only the
 * last one when `last`). An empty search matches at every boundary.
 * @param {string} text
 * @param {string} search
 * @param {boolean} all whether to find every match, or only the first one
 * @param {boolean} [last] whether to find only the last match
 * @returns {number[]}
 */
export function faFindMatches(text, search, all, last = false) {
	/** @type {number[]} */
	const matches = [];
	if (search === "" && all && !last) {
		// Every boundary (`indexOf("", position)` would stop moving at the end).
		return faIsSimpleText(text)
			? Array.from({ length: text.length + 1 }, (_, index) => index)
			: faBoundaries(text);
	}
	if (faIsSimpleText(text)) {
		if (last) {
			const position = text.lastIndexOf(search);
			return position === -1 ? matches : [position];
		}
		let position = text.indexOf(search);
		while (position !== -1) {
			matches.push(position);
			if (!all) {
				break;
			}
			position = text.indexOf(search, position + Math.max(search.length, 1));
		}
		return matches;
	}
	if (search === "") {
		return [last ? text.length : 0];
	}
	const isBoundary = new Set(faBoundaries(text));
	let position = text.indexOf(search);
	while (position !== -1) {
		if (isBoundary.has(position) && isBoundary.has(position + search.length)) {
			matches.push(position);
			if (!all && !last) {
				break;
			}
			position = text.indexOf(search, position + search.length);
		} else {
			position = text.indexOf(search, position + 1);
		}
	}
	return last && matches.length > 0 ? [matches[matches.length - 1]] : matches;
}

// @fa-helper faGraphemeIndex deps: faBoundaries faIsSimpleText
/**
 * The grapheme position of a UTF-16 offset that is a boundary.
 * @param {string} text
 * @param {number} offset
 * @returns {number}
 */
export function faGraphemeIndex(text, offset) {
	if (faIsSimpleText(text)) {
		return offset;
	}
	return faBoundaries(text).indexOf(offset);
}

// @fa-helper faStringIncludes deps: faFindMatches
/**
 * `text.includes(search)`.
 * @param {string} text
 * @param {string} search
 * @returns {boolean}
 */
export function faStringIncludes(text, search) {
	return faFindMatches(text, search, false).length > 0;
}

// @fa-helper faStringStartsWith deps: faBoundaries faIsSimpleText
/**
 * `text.startsWith(search)`.
 * @param {string} text
 * @param {string} search
 * @returns {boolean}
 */
export function faStringStartsWith(text, search) {
	if (!text.startsWith(search)) {
		return false;
	}
	return faIsSimpleText(text) || faBoundaries(text).includes(search.length);
}

// @fa-helper faStringEndsWith deps: faBoundaries faIsSimpleText
/**
 * `text.endsWith(search)`.
 * @param {string} text
 * @param {string} search
 * @returns {boolean}
 */
export function faStringEndsWith(text, search) {
	if (!text.endsWith(search)) {
		return false;
	}
	return faIsSimpleText(text) || faBoundaries(text).includes(text.length - search.length);
}

// @fa-helper faStringIndexOf deps: faFindMatches faGraphemeIndex
/**
 * `text.indexOf(search)`: the grapheme position of the first match, or
 * `none` (`undefined`).
 * @param {string} text
 * @param {string} search
 * @returns {number | undefined}
 */
export function faStringIndexOf(text, search) {
	const matches = faFindMatches(text, search, false);
	return matches.length === 0 ? undefined : faGraphemeIndex(text, matches[0]);
}

// @fa-helper faStringLastIndexOf deps: faFindMatches faGraphemeIndex
/**
 * `text.lastIndexOf(search)`: the grapheme position of the last match, or
 * `none` (`undefined`).
 * @param {string} text
 * @param {string} search
 * @returns {number | undefined}
 */
export function faStringLastIndexOf(text, search) {
	const matches = faFindMatches(text, search, false, true);
	return matches.length === 0 ? undefined : faGraphemeIndex(text, matches[0]);
}

// @fa-helper faReplaceMatches
/**
 * Replaces the given matches of `search` (UTF-16 offsets, in order).
 * @param {string} text
 * @param {string} search
 * @param {string} replacement
 * @param {number[]} matches
 * @returns {string}
 */
export function faReplaceMatches(text, search, replacement, matches) {
	let result = "";
	let previous = 0;
	for (const position of matches) {
		result += text.substring(previous, position) + replacement;
		previous = position + search.length;
	}
	return result + text.substring(previous);
}

// @fa-helper faStringReplace deps: faFindMatches faReplaceMatches
/**
 * `text.replace(search, replacement)`: the first match only. The replacement
 * is plain text (no `$&` patterns, unlike JavaScript's `replace`).
 * @param {string} text
 * @param {string} search
 * @param {string} replacement
 * @returns {string}
 */
export function faStringReplace(text, search, replacement) {
	return faReplaceMatches(text, search, replacement, faFindMatches(text, search, false));
}

// @fa-helper faStringReplaceAll deps: faFindMatches faReplaceMatches
/**
 * `text.replaceAll(search, replacement)`: every match, without overlaps.
 * @param {string} text
 * @param {string} search
 * @param {string} replacement
 * @returns {string}
 */
export function faStringReplaceAll(text, search, replacement) {
	return faReplaceMatches(text, search, replacement, faFindMatches(text, search, true));
}

// @fa-helper faStringSplit deps: faCharacters faFindMatches faIsSimpleText
/**
 * `text.split(separator)`; an empty separator splits into characters. In a
 * simple text every position is a boundary: JavaScript's `split` is the
 * same, without the list of matches.
 * @param {string} text
 * @param {string} separator
 * @returns {string[]}
 */
export function faStringSplit(text, separator) {
	if (separator === "") {
		return faCharacters(text);
	}
	if (faIsSimpleText(text)) {
		return text.split(separator);
	}
	const parts = [];
	let previous = 0;
	for (const position of faFindMatches(text, separator, true)) {
		parts.push(text.substring(previous, position));
		previous = position + separator.length;
	}
	parts.push(text.substring(previous));
	return parts;
}

// @fa-helper faPadding deps: faCharacters faStringLength
/**
 * The filler that pads `text` to `length` grapheme clusters.
 * @param {string} text
 * @param {number} length
 * @param {string} filler
 * @returns {string}
 */
export function faPadding(text, length, filler) {
	const missing = length - faStringLength(text);
	if (missing <= 0 || filler === "") {
		return "";
	}
	const characters = faCharacters(filler);
	let padding = "";
	for (let index = 0; index < missing; index += 1) {
		padding += characters[index % characters.length];
	}
	return padding;
}

// @fa-helper faStringPadStart deps: faIsSimpleText faPadding
/**
 * `text.padStart(length, filler = " ")`, in grapheme clusters.
 * @param {string} text
 * @param {number} length
 * @param {string} [filler]
 * @returns {string}
 */
export function faStringPadStart(text, length, filler = " ") {
	if (faIsSimpleText(text) && faIsSimpleText(filler)) {
		return text.padStart(length, filler);
	}
	return faPadding(text, length, filler) + text;
}

// @fa-helper faStringPadEnd deps: faIsSimpleText faPadding
/**
 * `text.padEnd(length, filler = " ")`, in grapheme clusters.
 * @param {string} text
 * @param {number} length
 * @param {string} [filler]
 * @returns {string}
 */
export function faStringPadEnd(text, length, filler = " ") {
	if (faIsSimpleText(text) && faIsSimpleText(filler)) {
		return text.padEnd(length, filler);
	}
	return text + faPadding(text, length, filler);
}

// @fa-helper faStringCodePoints
/**
 * `text.codePoints()`.
 * @param {string} text
 * @returns {number[]}
 */
export function faStringCodePoints(text) {
	return Array.from(text, (character) => /** @type {number} */ (character.codePointAt(0)));
}

// @fa-helper faStringBytes
/** The encoder of `faStringBytes` (created at the first non-ASCII text). */
let faBytesEncoder;

/**
 * `text.bytes()`: the UTF-8 bytes of the text. An ASCII text is read with
 * `charCodeAt`, any other is encoded once.
 * @param {string} text
 * @returns {number[]}
 */
export function faStringBytes(text) {
	const bytes = new Array(text.length);
	for (let index = 0; index < text.length; index++) {
		const code = text.charCodeAt(index);
		if (code >= 0x80) {
			if (faBytesEncoder === undefined) {
				faBytesEncoder = new TextEncoder();
			}
			return Array.from(faBytesEncoder.encode(text));
		}
		bytes[index] = code;
	}
	return bytes;
}

// @fa-helper faStringBytesView deps: faStringBytes
/** The text and the bytes of the last `faStringBytesView`. */
let faBytesViewText = "";
/** @type {number[]} */
let faBytesViewBytes = [];

/**
 * `text.bytes()` only read (`text.bytes()[i]`, `.get(i)`, `.length`, a
 * `for`): the bytes of the last text asked for are kept, so that a loop
 * reading the bytes of one text encodes it once. The array must not change.
 * @param {string} text
 * @returns {number[]}
 */
export function faStringBytesView(text) {
	if (text !== faBytesViewText) {
		faBytesViewBytes = faStringBytes(text);
		faBytesViewText = text;
	}
	return faBytesViewBytes;
}

// @fa-helper faStringCompare
/**
 * Compares two strings by Unicode code point (`<`, `<=`, `>`, `>=` on
 * strings): negative, zero or positive. JavaScript's `<` compares UTF-16 code
 * units, which only differs when the first difference involves a surrogate
 * and a unit above it (`U+E000`…`U+FFFF`, or another surrogate): only then
 * are code points read. A loop on the units, not a regular expression test
 * of both strings: comparators call it millions of times.
 * @param {string} a
 * @param {string} b
 * @returns {number}
 */
export function faStringCompare(a, b) {
	const length = a.length < b.length ? a.length : b.length;
	for (let index = 0; index < length; index++) {
		const x = a.charCodeAt(index);
		const y = b.charCodeAt(index);
		if (x !== y) {
			if (x >= 0xd800 && y >= 0xd800 && (x <= 0xdfff || y <= 0xdfff)) {
				for (let start = 0; ;) {
					const p = /** @type {number} */ (a.codePointAt(start));
					const q = /** @type {number} */ (b.codePointAt(start));
					if (p !== q) {
						return p < q ? -1 : 1;
					}
					start += p > 0xffff ? 2 : 1;
				}
			}
			return x < y ? -1 : 1;
		}
	}
	return a.length < b.length ? -1 : a.length > b.length ? 1 : 0;
}

// @fa-helper faStringCompareLocale
/**
 * `String.compareLocale(a, b, locale)`: `-1`, `0` or `1`.
 * @param {string} a
 * @param {string} b
 * @param {string} [locale]
 * @returns {number}
 */
export function faStringCompareLocale(a, b, locale) {
	return Math.sign(a.localeCompare(b, locale)) + 0;
}
