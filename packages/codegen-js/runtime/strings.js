// Strings are JavaScript strings: positions and order count UTF-16 units.
// Only explicit Unicode views and UTF-8 conversions need runtime support.

import { FaError } from "./errors.js";

// @fa-helper faSegmenter
/** The shared grapheme segmenter. */
export const faSegmenter = new Intl.Segmenter("und", { granularity: "grapheme" });

// @fa-helper faCharacters deps: faSegmenter
/** @param {string} text @returns {string[]} */
export function faCharacters(text) {
	return Array.from(faSegmenter.segment(text), ({ segment }) => segment);
}

// @fa-helper faStringCodePoints
/** @param {string} text @returns {number[]} */
export function faStringCodePoints(text) {
	return Array.from(text, (point) => /** @type {number} */ (point.codePointAt(0)));
}

// @fa-helper faCodePointCount
/** @param {string} text @returns {number} */
export function faCodePointCount(text) {
	let count = 0;
	for (let index = 0; index < text.length; count++) {
		index += /** @type {number} */ (text.codePointAt(index)) > 0xffff ? 2 : 1;
	}
	return count;
}

// @fa-helper faIsWellFormed
/** ES2022 browser fallback; Bun and Node use the native method.
 * @param {string} text @returns {boolean}
 */
export function faIsWellFormed(text) {
	for (let index = 0; index < text.length; index++) {
		const unit = text.charCodeAt(index);
		if (unit >= 0xd800 && unit <= 0xdbff) {
			const next = text.charCodeAt(++index);
			if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
		} else if (unit >= 0xdc00 && unit <= 0xdfff) {
			return false;
		}
	}
	return true;
}

// @fa-helper faUtf8Length
/** Browser byte count, including replacement of each lone surrogate.
 * @param {string} text @returns {number}
 */
export function faUtf8Length(text) {
	let length = 0;
	for (let index = 0; index < text.length; index++) {
		const unit = text.charCodeAt(index);
		if (unit < 0x80) length++;
		else if (unit < 0x800) length += 2;
		else if (
			unit >= 0xd800 &&
			unit <= 0xdbff &&
			text.charCodeAt(index + 1) >= 0xdc00 &&
			text.charCodeAt(index + 1) <= 0xdfff
		) {
			length += 4;
			index++;
		} else length += 3;
	}
	return length;
}

// @fa-helper faUtf8Encoder
/** The shared encoder; encode replaces lone surrogates with U+FFFD. */
export const faUtf8Encoder = new TextEncoder();

// @fa-helper faUtf8Decoder
/** Strict decoding preserves a UTF-8 BOM as U+FEFF. */
export const faUtf8Decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

// @fa-helper faUtf8LossyDecoder
/** Lossy decoding preserves a UTF-8 BOM too. */
export const faUtf8LossyDecoder = new TextDecoder("utf-8", { ignoreBOM: true });

// @fa-helper Utf8Error deps: FaError
/** The start of the first malformed UTF-8 sequence. */
export class Utf8Error extends FaError {
	static faName = this.register("Utf8Error");
	/** @param {number} position */
	constructor(position) {
		super();
		this.position = position;
	}
}

// @fa-helper faInvalidUtf8
/** Scan only after strict decoding fails. A malformed or truncated sequence
 * is reported at its lead byte, including an invalid continuation.
 * @param {Uint8Array} bytes @returns {number}
 */
export function faInvalidUtf8(bytes) {
	for (let index = 0; index < bytes.length;) {
		const start = index;
		const lead = bytes[index++];
		if (lead < 0x80) continue;
		const size =
			lead >= 0xc2 && lead <= 0xdf
				? 2
				: lead >= 0xe0 && lead <= 0xef
					? 3
					: lead >= 0xf0 && lead <= 0xf4
						? 4
						: 0;
		if (!size || start + size > bytes.length) return start;
		const second = bytes[index];
		if (
			second < 0x80 ||
			second > 0xbf ||
			(lead === 0xe0 && second < 0xa0) ||
			(lead === 0xed && second > 0x9f) ||
			(lead === 0xf0 && second < 0x90) ||
			(lead === 0xf4 && second > 0x8f)
		)
			return start;
		for (let offset = 1; offset < size; offset++, index++) {
			if (bytes[index] < 0x80 || bytes[index] > 0xbf) return start;
		}
	}
	return -1;
}

// @fa-helper faFromUtf8 deps: faUtf8Decoder faInvalidUtf8 Utf8Error
/** @param {Uint8Array} bytes @returns {string} */
export function faFromUtf8(bytes) {
	try {
		return faUtf8Decoder.decode(bytes);
	} catch {
		throw new Utf8Error(faInvalidUtf8(bytes));
	}
}

// @fa-helper faStringCompareLocale
/** @param {string} a @param {string} b @param {string} [locale] @returns {number} */
export function faStringCompareLocale(a, b, locale) {
	return Math.sign(a.localeCompare(b, locale)) + 0;
}
