// Printing (`toString`, interpolation, `console.log`): values brief section
// 2.8, D-5, D-6.
//
// - Top level: a string is its raw text; any other value its Fa notation.
// - Nested in a composite: strings quoted with Fa escapes, integers and
//   floats like JavaScript (`String(x)`), `none`, `{ x = 1, y = 2 }` in
//   declaration order, `[1, 2]`, `(1, "one")`, `Shape.Circle { radius = 1 }`,
//   `1..3`. A map prints like its literal, `[("a", 1)]`, a set like an array.
//
// `faFormat` decides from the JavaScript representation alone, which is
// enough for most values. Where it isn't (a `Float32`, a `Percentage`, an
// enumeration, a tuple, a named union member, relation fields to leave out),
// the backend composes the typed formatters below (`faFormatArray(items,
// faFloat32ToString)`...), passing the formatter of each part.

/** @typedef {(value: any) => string} FaFormatter */

// @fa-helper faQuote
/**
 * A string in Fa literal notation: quoted, with `\\`, `\"`, `\n`, `\r`,
 * `\t`, `\0`, `\xNN` for the other control characters, and `{{`, `}}` for
 * the braces (a lone brace starts an interpolation in Fa).
 * @param {string} text
 * @returns {string}
 */
export function faQuote(text) {
	const escaped = text.replace(/[\x00-\x1f\x7f"\\{}]/g, (character) => {
		switch (character) {
			case "\\":
				return "\\\\";
			case '"':
				return '\\"';
			case "\n":
				return "\\n";
			case "\r":
				return "\\r";
			case "\t":
				return "\\t";
			case "\0":
				return "\\0";
			case "{":
				return "{{";
			case "}":
				return "}}";
			default:
				return `\\x${character.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0")}`;
		}
	});
	return `"${escaped}"`;
}

// @fa-helper faFormat deps: faFormatNested
/**
 * The top-level `toString` of a value: a string is its raw text, anything
 * else its Fa notation.
 * @param {unknown} value
 * @returns {string}
 */
export function faFormat(value) {
	return typeof value === "string" ? value : faFormatNested(value);
}

// @fa-helper faFormatNested deps: faQuote faFormatArray faFormatObject faFormatTagged faFormatMap faFormatSet faFormatRange
/**
 * The Fa notation of a value nested in a composite, decided from its
 * JavaScript representation: strings are quoted, `undefined` is `none`,
 * arrays print `[...]`, maps `[(key, value)]`, sets `[...]`, ranges
 * `a..b`, errors their `toString()`, other objects `{ field = value }`
 * (without the properties starting with `$`: union tags, private fields).
 * @param {unknown} value
 * @returns {string}
 */
export function faFormatNested(value) {
	switch (typeof value) {
		case "string":
			return faQuote(value);
		case "undefined":
			return "none";
		case "object":
			if (value === null) {
				return "none";
			}
			if (Array.isArray(value)) {
				return faFormatArray(value);
			}
			// recognized by their tags, so that printing doesn't bring the
			// collection classes into programs that don't use them
			switch (/** @type {any} */ (value)[Symbol.toStringTag]) {
				case "Map":
				case "fa:HashMap":
					return faFormatMap(/** @type {Iterable<[unknown, unknown]>} */ (value));
				case "Set":
				case "fa:Set":
				case "fa:ValuePool":
					return faFormatSet(/** @type {Iterable<unknown>} */ (value));
				case "fa:Range":
					return faFormatRange(/** @type {any} */ (value));
			}
			if (typeof (/** @type {any} */ (value).constructor?.faName) === "string") {
				// a Fa error (see `faIsError`)
				return String(value);
			}
			if (typeof (/** @type {any} */ (value).$t) === "string") {
				return faFormatTagged(value);
			}
			return faFormatObject(value);
		case "function":
			return "<function>";
		default:
			return String(value);
	}
}

// @fa-helper faFormatOptional deps: faFormatNested
/**
 * An optional: `none`, or the value with its formatter.
 * @template T
 * @param {T | undefined} value
 * @param {(value: T) => string} [format]
 * @returns {string}
 */
export function faFormatOptional(value, format = faFormatNested) {
	return value === undefined ? "none" : format(value);
}

// @fa-helper faFormatArray deps: faFormatNested
/**
 * An array (or growing array): `[1, 2, 3]`, `[]`.
 * @template T
 * @param {Iterable<T>} items
 * @param {(item: T) => string} [format] the formatter of the items
 * @returns {string}
 */
export function faFormatArray(items, format = faFormatNested) {
	const parts = [];
	for (const item of items) {
		parts.push(format(item));
	}
	return `[${parts.join(", ")}]`;
}

// @fa-helper faFormatTuple deps: faFormatNested
/**
 * A tuple: `(1, "one")`.
 * @param {unknown[]} tuple
 * @param {FaFormatter[]} [formats] the formatter of each item
 * @returns {string}
 */
export function faFormatTuple(tuple, formats) {
	const parts = tuple.map((item, index) => (formats?.[index] ?? faFormatNested)(item));
	return `(${parts.join(", ")})`;
}

// @fa-helper faFormatObject deps: faFormatNested
/**
 * An object: `{ x = 1, y = 2 }`, `{}`. `fields` lists the fields to print, in
 * declaration order, with their formatters (`[["x", faFormatNested]]`), so
 * that relation fields and private fields are left out; without it, every
 * own property not starting with `$`, in property order.
 * @param {object} object
 * @param {[string, FaFormatter][]} [fields]
 * @returns {string}
 */
export function faFormatObject(object, fields) {
	const record = /** @type {Record<string, unknown>} */ (object);
	const parts = [];
	if (fields === undefined) {
		for (const key of Object.keys(record)) {
			if (!key.startsWith("$")) {
				parts.push(`${key} = ${faFormatNested(record[key])}`);
			}
		}
	} else {
		for (const [key, format] of fields) {
			parts.push(`${key} = ${format(record[key])}`);
		}
	}
	return parts.length === 0 ? "{}" : `{ ${parts.join(", ")} }`;
}

// @fa-helper faFormatMember
/**
 * A member of a named union or of an `Errors` type: its qualified name,
 * then its payload: `Shape.Point` (none), `Shape.Circle { radius = 1 }` (an
 * object, already formatted), `UserId.User(12)` (any other value, already
 * formatted).
 * @param {string} name the qualified name
 * @param {string} [payload] the formatted payload
 * @param {boolean} [object] whether the payload is an object
 * @returns {string}
 */
export function faFormatMember(name, payload, object = false) {
	if (payload === undefined) {
		return name;
	}
	return object ? `${name} ${payload}` : `${name}(${payload})`;
}

// @fa-helper faFormatTagged deps: faFormatNested faFormatObject faFormatMember
/**
 * A member of a named union, as compiled: `{ $t: "Shape.Circle", radius: 1 }`
 * (data fields), `{ $t: "UserId.User", $v: 12 }` (data that is not an object)
 * or `{ $t: "Shape.Point" }` (no data). The tag is the qualified name.
 * @param {{ $t: string, $v?: unknown }} member
 * @returns {string}
 */
export function faFormatTagged(member) {
	if (Object.hasOwn(member, "$v")) {
		return faFormatMember(member.$t, faFormatNested(member.$v));
	}
	const hasFields = Object.keys(member).some((key) => !key.startsWith("$"));
	return faFormatMember(member.$t, hasFields ? faFormatObject(member) : undefined, true);
}

// @fa-helper faFormatError deps: faQuote
/**
 * The `toString()` of an error: its qualified name (the static `faName` of
 * its class), then its fields: `IndexError { index = 5, length = 3 }`,
 * `DivisionByZero`. The fields of the runtime's errors are numbers and
 * strings, so this formatter doesn't bring the whole printer into a
 * program; a generated error class with composite fields overrides
 * `toString` with `faFormatErrorFields`.
 * @param {object} error
 * @returns {string}
 */
export function faFormatError(error) {
	const type = /** @type {{ faName?: string, name: string }} */ (error.constructor);
	const record = /** @type {Record<string, unknown>} */ (error);
	const parts = [];
	for (const key of Object.keys(record)) {
		if (!key.startsWith("$")) {
			const value = record[key];
			const text =
				typeof value === "string" ? faQuote(value) : value === undefined ? "none" : String(value);
			parts.push(`${key} = ${text}`);
		}
	}
	const name = type.faName ?? type.name;
	return parts.length === 0 ? name : `${name} { ${parts.join(", ")} }`;
}

// @fa-helper faFormatErrorFields deps: faFormatObject faFormatMember
/**
 * The `toString()` of an error whose fields hold composite values (objects,
 * collections, unions): each field in Fa notation.
 * @param {object} error
 * @returns {string}
 */
export function faFormatErrorFields(error) {
	const type = /** @type {{ faName?: string, name: string }} */ (error.constructor);
	const name = type.faName ?? type.name;
	const hasFields = Object.keys(error).some((key) => !key.startsWith("$"));
	return faFormatMember(name, hasFields ? faFormatObject(error) : undefined, true);
}

// @fa-helper faFormatMap deps: faFormatNested
/**
 * A map, like its literal: `[("a", 1), ("b", 2)]`, `[]`.
 * @template K, V
 * @param {Iterable<[K, V]>} map
 * @param {(key: K) => string} [formatKey]
 * @param {(value: V) => string} [formatValue]
 * @returns {string}
 */
export function faFormatMap(map, formatKey = faFormatNested, formatValue = faFormatNested) {
	const parts = [];
	for (const [key, value] of map) {
		parts.push(`(${formatKey(key)}, ${formatValue(value)})`);
	}
	return `[${parts.join(", ")}]`;
}

// @fa-helper faFormatSet deps: faFormatArray faFormatNested
/**
 * A set, like its literal: `[1, 2]`, `[]`.
 * @template T
 * @param {Iterable<T>} set
 * @param {(item: T) => string} [format]
 * @returns {string}
 */
export function faFormatSet(set, format = faFormatNested) {
	return faFormatArray(set, format);
}

// @fa-helper faFormatRange
/**
 * A range: `1..3`, `0..=10`, `5..`, `..10`, `..=10`, `..`.
 * @param {{ start?: number, end?: number, includesEnd: boolean }} range
 * @returns {string}
 */
export function faFormatRange(range) {
	const start = range.start === undefined ? "" : String(range.start);
	if (range.end === undefined) {
		return `${start}..`;
	}
	return `${start}${range.includesEnd ? "..=" : ".."}${range.end}`;
}
