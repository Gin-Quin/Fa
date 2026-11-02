// `std.fs`, `std.time` and `std.json` (advanced brief D9). `std.time.now()`
// compiles to `Date.now()` and needs no helper.
//
// - `File.read(path)` reads a whole file as UTF-8, blocking, through the
//   `node:fs` module of Bun, Node and Deno (`process.getBuiltinModule`), or
//   `Deno.readTextFileSync`. It doesn't exist on `web` (a compile error).
// - `Json.parse(text)` is directed by the type the program expects: the
//   compiler passes a schema of that type (`faJsonParse`), and the value is
//   checked and converted while it is read.
// - `Json.stringify(value)` writes Fa values: `none` is `null` (an absent
//   field is left out), maps with `String` keys are objects, sets are arrays,
//   and the `$` properties of the runtime are skipped.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { FaError } from "./errors.js";

// @fa-helper FileError deps: FaError
/** A file that can't be read (`std.fs`). */
export class FileError extends FaError {
	static faName = "FileError";

	/**
	 * @param {string} path the file
	 * @param {string} message what went wrong
	 */
	constructor(path, message) {
		super();
		/** @type {string} */
		this.path = path;
		/** @type {string} */
		this.message = message;
	}
}

// @fa-helper JsonError deps: FaError
/** Malformed JSON, or JSON that doesn't fit the expected type (`std.json`). */
export class JsonError extends FaError {
	static faName = "JsonError";

	/** @param {string} message what is wrong, and where */
	constructor(message) {
		super();
		/** @type {string} */
		this.message = message;
	}
}

// @fa-helper faReadFile deps: FileError
/**
 * `File.read(path)`: the whole file, as UTF-8.
 * @param {string} path
 * @returns {string}
 */
export function faReadFile(path) {
	/** @type {any} */
	const host = globalThis;
	try {
		const fs = host.process?.getBuiltinModule?.("node:fs");
		if (fs !== undefined) {
			return fs.readFileSync(path, "utf8");
		}
		if (host.Deno !== undefined) {
			return host.Deno.readTextFileSync(path);
		}
	} catch (error) {
		const code = /** @type {any} */ (error)?.code;
		const message =
			code === "ENOENT" || /** @type {any} */ (error)?.name === "NotFound"
				? "no such file"
				: String(/** @type {any} */ (error)?.message ?? error);
		throw new FileError(path, message);
	}
	throw new FileError(path, "this platform has no file system");
}

// @fa-helper faJsonParse deps: JsonError
/**
 * `Json.parse(text)`: the value of the expected type, described by `schema`:
 *
 * - `"b"` a boolean, `"s"` a string, `"f"` a float, `"n"` `none` (`null`);
 * - `["i", min, max]` an integer number, `["I", min, max]` a bigint (the
 *   bounds are strings, `null` without a bound);
 * - `["l", value]` a literal (string, boolean or number);
 * - `["a", item]` an array, `["m", value]` a map with `String` keys;
 * - `["o", [[name, schema, optional], …]]` an object, its fields in
 *   declaration order: an optional field may be absent (`none`), extra
 *   fields are ignored;
 * - `["u", [schema, …]]` an anonymous union: the first member that fits.
 *
 * @param {string} text
 * @param {any} schema
 * @returns {any}
 */
export function faJsonParse(text, schema) {
	let data;
	try {
		data = JSON.parse(text);
	} catch (error) {
		throw new JsonError(String(/** @type {any} */ (error)?.message ?? error));
	}
	return faJsonConvert(data, schema, "the value");
}

/**
 * @param {any} data
 * @param {any} schema
 * @param {string} where
 * @returns {any}
 */
function faJsonConvert(data, schema, where) {
	const fail = (/** @type {string} */ expected) => {
		const found = data === null ? "null" : Array.isArray(data) ? "an array" : typeof data;
		return new JsonError(`${where} should be ${expected}, not ${found}`);
	};
	if (typeof schema === "string") {
		switch (schema) {
			case "b":
				if (typeof data === "boolean") return data;
				throw fail("a boolean");
			case "s":
				if (typeof data === "string") return data;
				throw fail("a string");
			case "f":
				if (typeof data === "number") return data;
				throw fail("a number");
			case "n":
				if (data === null) return undefined;
				throw fail("null");
		}
	}
	switch (schema[0]) {
		case "i":
		case "I": {
			if (typeof data !== "number" || !Number.isInteger(data)) {
				throw fail("an integer");
			}
			const big = schema[0] === "I";
			if (!big && !Number.isSafeInteger(data)) {
				throw new JsonError(`${where} doesn't fit in an Integer on JavaScript`);
			}
			const value = BigInt(data);
			if (schema[1] !== null && value < BigInt(schema[1])) {
				throw new JsonError(`${where} should be at least ${schema[1]}, not ${data}`);
			}
			if (schema[2] !== null && value > BigInt(schema[2])) {
				throw new JsonError(`${where} should be at most ${schema[2]}, not ${data}`);
			}
			return big ? value : data;
		}
		case "l":
			if (data === schema[1]) return data;
			throw fail(JSON.stringify(schema[1]));
		case "a":
			if (!Array.isArray(data)) throw fail("an array");
			return data.map((item, index) => faJsonConvert(item, schema[1], `${where}[${index}]`));
		case "m": {
			if (data === null || typeof data !== "object" || Array.isArray(data)) {
				throw fail("an object");
			}
			const map = new Map();
			for (const [key, item] of Object.entries(data)) {
				map.set(key, faJsonConvert(item, schema[1], `${where}.${key}`));
			}
			return map;
		}
		case "o": {
			if (data === null || typeof data !== "object" || Array.isArray(data)) {
				throw fail("an object");
			}
			/** @type {Record<string, any>} */
			const value = {};
			for (const [name, field, optional] of schema[1]) {
				const item = data[name];
				if (item === undefined) {
					if (!optional) throw new JsonError(`${where} has no field \`${name}\``);
					value[name] = undefined;
				} else {
					value[name] = faJsonConvert(item, field, `${where}.${name}`);
				}
			}
			return value;
		}
		case "u": {
			for (const member of schema[1]) {
				try {
					return faJsonConvert(data, member, where);
				} catch (error) {
					if (!(error instanceof JsonError)) throw error;
				}
			}
			throw fail("one of the members of its union");
		}
	}
	throw new JsonError(`${where} has a type JSON can't describe`);
}

// @fa-helper faJsonStringify
/**
 * `Json.stringify(value)`: the JSON text of a Fa value.
 * @param {any} value
 * @returns {string}
 */
export function faJsonStringify(value) {
	return faJsonWrite(value) ?? "null";
}

/**
 * @param {any} value
 * @returns {string | undefined} `undefined` for `none` (left out of objects)
 */
function faJsonWrite(value) {
	switch (typeof value) {
		case "undefined":
			return undefined;
		case "bigint":
			return value.toString();
		case "number":
			return Number.isFinite(value) ? String(value) : "null";
		case "string":
		case "boolean":
			return JSON.stringify(value);
		case "function":
			return undefined;
	}
	if (value === null) return "null";
	if (Array.isArray(value) || value instanceof Set) {
		return `[${Array.from(value, (item) => faJsonWrite(item) ?? "null").join(",")}]`;
	}
	/** @type {Iterable<[any, any]>} */
	const entries =
		value instanceof Map || value[Symbol.toStringTag] === "fa:HashMap"
			? value.entries()
			: Object.entries(value);
	const parts = [];
	for (const [key, item] of entries) {
		if (typeof key === "string" && key.startsWith("$")) continue;
		const written = faJsonWrite(item);
		if (written !== undefined) {
			parts.push(`${JSON.stringify(String(key))}:${written}`);
		}
	}
	return `{${parts.join(",")}}`;
}
