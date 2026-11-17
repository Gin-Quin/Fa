// `std.fs`, `std.time` and `std.json` (advanced brief D9). `std.time.now()`
// compiles to `Date.now()` and needs no helper.
//
// - `File.read/readText(path)` and `File.write/writeText(path, value)`
//   work through the `node:fs` module of Bun, Node and Deno
//   (`process.getBuiltinModule`): blocking with its `*Sync` functions,
//   awaited with `fs.promises` (`faFileAwait`: the other tasks run
//   meanwhile). Errors are `FileError`s with the system's text, as natively
//   (`faSystemMessage`). They don't exist on `web` (a compile error).
// - `std.time.monotonic()` is `performance.now()` in nanoseconds.
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
import { faUtf8Decoder } from "./strings.js";
import { faAlive } from "./tasks.js";

/** @typedef {import("./tasks.js").FaTask} FaTask */

// @fa-helper FileError deps: FaError
/** A file that can't be read (`std.fs`). */
export class FileError extends FaError {
	static faName = this.register("FileError");

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
	static faName = this.register("JsonError");

	/** @param {string} message what is wrong, and where */
	constructor(message) {
		super();
		/** @type {string} */
		this.message = message;
	}
}

// @fa-helper faSystemMessage
/**
 * The system's text of an error of `node:fs` or `node:net` (the text of
 * `strerror`, as natively), from its code.
 * @param {any} error
 * @returns {string}
 */
export function faSystemMessage(error) {
	/** @type {Record<string, string>} */
	const texts = {
		ENOENT: "No such file or directory",
		EISDIR: "Is a directory",
		ENOTDIR: "Not a directory",
		EACCES: "Permission denied",
		EPERM: "Operation not permitted",
		EEXIST: "File exists",
		ENOSPC: "No space left on device",
		EMFILE: "Too many open files",
		EROFS: "Read-only file system",
		ECONNREFUSED: "Connection refused",
		ECONNRESET: "Connection reset by peer",
		EADDRINUSE: "Address already in use",
		EADDRNOTAVAIL: "Cannot assign requested address",
		EPIPE: "Broken pipe",
		ETIMEDOUT: "Connection timed out",
		EHOSTUNREACH: "No route to host",
		ENETUNREACH: "Network is unreachable",
		ENOTCONN: "Transport endpoint is not connected",
	};
	const code = error?.code;
	if (typeof code === "string" && texts[code] !== undefined) {
		return texts[code];
	}
	if (error?.name === "NotFound") {
		return texts.ENOENT;
	}
	return String(error?.message ?? error);
}

// @fa-helper faFs
/**
 * The `node:fs` module of Bun, Node and Deno, or `undefined` (web).
 * @returns {any}
 */
export function faFs() {
	/** @type {any} */
	const host = globalThis;
	return host.process?.getBuiltinModule?.("node:fs");
}

// @fa-helper faFileCall deps: FileError faFs faSystemMessage
/**
 * Runs a blocking operation of `node:fs` on `path`; its errors become a
 * `FileError`.
 * @template T
 * @param {string} path
 * @param {(fs: any) => T} operation
 * @returns {T}
 */
export function faFileCall(path, operation) {
	const fs = faFs();
	if (fs === undefined) {
		throw new FileError(path, "this platform has no file system");
	}
	try {
		return operation(fs);
	} catch (error) {
		throw new FileError(path, faSystemMessage(error));
	}
}

// @fa-helper faReadFile deps: faFileCall faUtf8Decoder
/**
 * `File.readText(path)`: the whole file, strictly decoded from UTF-8.
 * @param {string} path
 * @returns {string}
 */
export function faReadFile(path) {
	return faFileCall(path, (fs) => faUtf8Decoder.decode(fs.readFileSync(path)));
}

// @fa-helper faReadBytes deps: faFileCall
/**
 * `File.read(path)`: the whole file, as Bytes.
 * @param {string} path
 * @returns {Uint8Array}
 */
export function faReadBytes(path) {
	return faFileCall(path, (fs) => new Uint8Array(fs.readFileSync(path)));
}

// @fa-helper faWriteFile deps: faFileCall
/**
 * `File.writeText(path, text)`: creates or replaces the file, in UTF-8.
 * @param {string} path
 * @param {string} text
 * @returns {void}
 */
export function faWriteFile(path, text) {
	faFileCall(path, (fs) => fs.writeFileSync(path, text));
}

// @fa-helper faFileAwait deps: FileError faFs faSystemMessage faAlive
/**
 * An awaited operation of `node:fs` (`fs.promises`): the other tasks run
 * meanwhile. Cancelling the task ends its wait at once (the operation's
 * result is dropped, as natively).
 * @template T
 * @param {FaTask} task
 * @param {string} path
 * @param {(promises: any) => Promise<T>} operation
 * @returns {Promise<T>}
 */
export async function faFileAwait(task, path, operation) {
	const fs = faFs();
	if (fs === undefined) {
		throw new FileError(path, "this platform has no file system");
	}
	/** @type {{ value?: T, error?: any, failed: boolean }} */
	const outcome = { failed: false };
	await new Promise((resolve) => {
		task.wake = () => resolve(undefined);
		operation(fs.promises).then(
			(value) => {
				outcome.value = value;
				resolve(undefined);
			},
			(error) => {
				outcome.failed = true;
				outcome.error = error;
				resolve(undefined);
			},
		);
	});
	task.wake = null;
	faAlive(task);
	if (outcome.failed) {
		throw new FileError(path, faSystemMessage(outcome.error));
	}
	return /** @type {T} */ (outcome.value);
}

// @fa-helper faReadFileAsync deps: faFileAwait faUtf8Decoder
/**
 * `await File.readText(path)`.
 * @param {FaTask} task
 * @param {string} path
 * @returns {Promise<string>}
 */
export function faReadFileAsync(task, path) {
	return faFileAwait(task, path, async (fs) => faUtf8Decoder.decode(await fs.readFile(path)));
}

// @fa-helper faReadBytesAsync deps: faFileAwait
/**
 * `await File.read(path)`.
 * @param {FaTask} task
 * @param {string} path
 * @returns {Promise<Uint8Array>}
 */
export function faReadBytesAsync(task, path) {
	return faFileAwait(task, path, async (fs) => new Uint8Array(await fs.readFile(path)));
}

// @fa-helper faWriteFileAsync deps: faFileAwait
/**
 * `await File.writeText(path, text)`.
 * @param {FaTask} task
 * @param {string} path
 * @param {string} text
 * @returns {Promise<void>}
 */
export function faWriteFileAsync(task, path, text) {
	return faFileAwait(task, path, (fs) => fs.writeFile(path, text));
}

// @fa-helper faMonotonic
/**
 * `std.time.monotonic()`: nanoseconds of a monotonic clock.
 * @returns {number}
 */
export function faMonotonic() {
	return Math.round(performance.now() * 1e6);
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
 * - `["o", [[name, schema, optional, initial], …]]` an object, its fields
 *   in declaration order: a field with a default value (`initial`, a
 *   function giving it, else `null`) or optional may be absent (its default
 *   value, else `none`), extra fields are ignored;
 * - `["u", [schema, …]]` an anonymous union: the first member that fits;
 * - `["E", [[name, value, pinned], …]]` an enumeration: its pinned
 *   values (numbers), else its member names (strings);
 * - `["F", [path, …], [name, …]]` a `Fields` value: the array of the
 *   dotted paths of its flags (`path`: the path of each bit; `name`: the
 *   accepted names, flags and groups, a group standing for all its flags),
 *   a number up to 32 flags, a bigint above;
 * - `["N", [[name, tag, kind, data], …]]` a named union, its members
 *   tagged by a `"type"` property holding their name: `"e"` a member
 *   without data (`{ "type": "Empty" }`), `"l"` a literal (`data`, written
 *   as itself), `"r"` an object (`data` its schema; its fields next to
 *   `"type"`), `"w"` an object with its own `type` field and `"v"` other
 *   data (under `"value"`); a member of an `Errors` type has its class and
 *   its fields next (`"x"`: its fields next to `"type"`, `"V"`: under
 *   `"value"`).
 *
 * @param {any} text
 * @param {any} schema
 * @param {(text: any) => any} [read]
 * @returns {any}
 */
export function faJsonParse(text, schema, read = JSON.parse) {
	let data;
	try {
		data = read(text);
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
			if (schema[2] !== undefined && data.length > schema[2]) {
				throw new JsonError(`${where} exceeds its inline array capacity ${schema[2]}`);
			}
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
			for (const [name, field, optional, initial] of schema[1]) {
				// a field named like a property of `Object.prototype` has a `$`
				const property = name in Object.prototype ? `${name}$` : name;
				const item = Object.hasOwn(data, name) ? data[name] : undefined;
				if (item === undefined) {
					if (typeof initial === "function") {
						value[property] = initial();
						continue;
					}
					if (!optional) throw new JsonError(`${where} has no field \`${name}\``);
					value[property] = undefined;
				} else {
					value[property] = faJsonConvert(item, field, `${where}.${name}`);
				}
			}
			return value;
		}
		case "F": {
			if (!Array.isArray(data)) throw fail("an array of flags");
			/** @type {string[]} */
			const names = schema[1];
			const big = names.length > 32;
			let bits = big ? 0n : 0;
			/** @type {string[]} */
			const accepted = schema[2];
			for (const [position, flag] of data.entries()) {
				if (typeof flag !== "string" || !accepted.includes(flag)) {
					const found = flag === null ? "null" : Array.isArray(flag) ? "an array" : typeof flag;
					throw new JsonError(
						`${where}[${position}] should be one of ${accepted.map((name) => JSON.stringify(name)).join(", ")}, not ${found}`,
					);
				}
				const prefix = `${flag}.`;
				for (const [index, name] of names.entries()) {
					if (name === flag || name.startsWith(prefix)) {
						if (big) bits = /** @type {bigint} */ (bits) | (1n << BigInt(index));
						else bits = /** @type {number} */ (bits | (1 << index)) >>> 0;
					}
				}
			}
			return bits;
		}
		case "E": {
			const pinned = schema[1].some((/** @type {any} */ member) => member[2]);
			for (const [name, value] of schema[1]) {
				if (pinned ? data === value : data === name) return value;
			}
			const accepted = schema[1].map((/** @type {any} */ member) =>
				pinned ? String(member[1]) : JSON.stringify(member[0]),
			);
			throw fail(`one of ${accepted.join(", ")}`);
		}
		case "N": {
			const tagged = schema[1].filter((/** @type {any} */ member) => member[2] !== "l");
			for (const [, , kind, literal] of schema[1]) {
				if (kind === "l" && data === literal) return literal;
			}
			const type =
				data !== null && typeof data === "object" && !Array.isArray(data) ? data.type : undefined;
			if (typeof type !== "string") throw fail("one of the members of its union");
			const member = tagged.find((/** @type {any} */ member) => member[0] === type);
			if (member === undefined) {
				const names = tagged.map((/** @type {any} */ member) => JSON.stringify(member[0]));
				throw new JsonError(
					`${where}.type should be one of ${names.join(", ")}, not ${JSON.stringify(type)}`,
				);
			}
			const [, tag, kind, field, error, names] = member;
			if (kind === "x" || kind === "V") {
				/** @type {Record<string, any>} */
				let fields = {};
				if (field !== null && kind === "x") fields = faJsonConvert(data, field, where);
				if (field !== null && kind === "V") {
					if (data.value === undefined) throw new JsonError(`${where} has no field \`value\``);
					const value = faJsonConvert(data.value, field, `${where}.value`);
					fields = names.length === 1 && names[0] === "$v" ? { $v: value } : value;
				}
				return new error(
					...names.map(
						(/** @type {string} */ name) => fields[name in Object.prototype ? `${name}$` : name],
					),
				);
			}
			if (kind === "e") return Object.freeze({ $t: tag });
			if (kind === "r") return Object.assign({ $t: tag }, faJsonConvert(data, field, where));
			if (data.value === undefined) throw new JsonError(`${where} has no field \`value\``);
			const value = faJsonConvert(data.value, field, `${where}.value`);
			return kind === "w" ? Object.assign({ $t: tag }, value) : { $t: tag, $v: value };
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
 * `Json.stringify(value)`: the JSON text of a Fa value, written by the
 * schema of its type (the format of `faJsonParse`): enumerations as their
 * pinned values or names, named unions and errors tagged by `"type"`,
 * fields in declaration order. Without a schema, by the value's shape.
 * @param {any} value
 * @param {any} [schema]
 * @returns {string}
 */
export function faJsonStringify(value, schema) {
	return faJsonWrite(value, schema ?? null) ?? "null";
}

/**
 * Whether a value is one of the type a schema describes (a member of an
 * anonymous union).
 * @param {any} value
 * @param {any} schema
 * @returns {boolean}
 */
function faJsonIs(value, schema) {
	if (typeof schema === "string") {
		switch (schema) {
			case "b":
				return typeof value === "boolean";
			case "s":
				return typeof value === "string";
			case "f":
				return typeof value === "number";
			case "n":
				return value === undefined;
		}
	}
	switch (schema[0]) {
		case "i":
			return typeof value === "number" && Number.isInteger(value);
		case "I":
			return typeof value === "bigint";
		case "l":
			return value === schema[1];
		case "a":
			return Array.isArray(value);
		case "m":
			return value instanceof Map || value?.[Symbol.toStringTag] === "fa:HashMap";
		case "o":
			return (
				value !== null &&
				typeof value === "object" &&
				!Array.isArray(value) &&
				!("$t" in value) &&
				schema[1].every((/** @type {any} */ [name, field]) => {
					const property = name in Object.prototype ? `${name}$` : name;
					return Object.hasOwn(value, property) && faJsonIs(value[property], field);
				})
			);
		case "F":
			return typeof value === (schema[1].length > 32 ? "bigint" : "number");
		case "E":
			return schema[1].some((/** @type {any} */ member) => member[1] === value);
		case "N":
			return schema[1].some((/** @type {any} */ member) => faJsonMember(value, member));
		case "u":
			return schema[1].some((/** @type {any} */ member) => faJsonIs(value, member));
	}
	return false;
}

/**
 * Whether a value is a member of a named union (its schema's entry).
 * @param {any} value
 * @param {any} member
 * @returns {boolean}
 */
function faJsonMember(value, member) {
	const [, tag, kind, , error] = member;
	if (kind === "l") return value === member[3];
	if (kind === "x" || kind === "V") return value instanceof error;
	return value !== null && typeof value === "object" && value.$t === tag;
}

/**
 * @param {any} value
 * @param {any} schema
 * @returns {string | undefined} `undefined` for `none` (left out of objects)
 */
function faJsonWrite(value, schema) {
	if (schema !== null) {
		const written = faJsonWriteAs(value, schema);
		if (written !== null) return written;
	}
	if (value === undefined || typeof value === "function") return undefined;
	/** @type {(string | { value: any, record?: boolean })[]} */
	const pending = [{ value }];
	const parts = [];
	while (pending.length > 0) {
		const frame = /** @type {string | { value: any, record?: boolean }} */ (pending.pop());
		if (typeof frame === "string") {
			parts.push(frame);
			continue;
		}
		const item = frame.value;
		if (item === null || typeof item !== "object") {
			parts.push(typeof item === "bigint" ? item.toString() : (JSON.stringify(item) ?? "null"));
			continue;
		}
		if (!frame.record && typeof item.$t === "string") {
			const type = JSON.stringify(item.$t.slice(item.$t.lastIndexOf(".") + 1));
			parts.push(`{"type":${type}`);
			if ("$v" in item) {
				if (item.$v === undefined || typeof item.$v === "function") {
					parts.push("}");
				} else {
					parts.push(',"value":');
					pending.push("}", { value: item.$v });
				}
				continue;
			}
			if (Object.hasOwn(item, "type")) {
				parts.push(',"value":');
				pending.push("}", { value: item, record: true });
				continue;
			}
		} else if (Array.isArray(item) || item instanceof Set) {
			const items = Array.isArray(item) ? item : Array.from(item);
			parts.push("[");
			pending.push("]");
			for (let index = items.length - 1; index >= 0; index--) {
				pending.push({ value: items[index] });
				if (index > 0) pending.push(",");
			}
			continue;
		} else {
			parts.push("{");
		}
		const map = item instanceof Map || item[Symbol.toStringTag] === "fa:HashMap";
		/** @type {[string, any][]} */
		const entries = Array.from(map ? item.entries() : Object.entries(item));
		const fields = entries.filter(
			([key, value]) =>
				(map || !key.startsWith("$")) && value !== undefined && typeof value !== "function",
		);
		pending.push("}");
		for (let index = fields.length - 1; index >= 0; index--) {
			const [key, value] = fields[index];
			const name = !map && key.endsWith("$") ? key.slice(0, -1) : String(key);
			pending.push({ value }, `${JSON.stringify(name)}:`);
			if (index > 0) pending.push(",");
		}
		if (!frame.record && typeof item.$t === "string" && fields.length > 0) parts.push(",");
	}
	return parts.join("");
}

/**
 * A value written by its schema: `null` when the schema doesn't decide
 * (the value's shape does).
 * @param {any} value
 * @param {any} schema
 * @returns {string | undefined | null}
 */
function faJsonWriteAs(value, schema) {
	if (typeof schema === "string" || value === undefined) return null;
	switch (schema[0]) {
		case "a":
			if (!Array.isArray(value)) return null;
			return `[${value.map((item) => faJsonWrite(item, schema[1]) ?? "null").join(",")}]`;
		case "m": {
			if (!(value instanceof Map || value?.[Symbol.toStringTag] === "fa:HashMap")) return null;
			const parts = [];
			for (const [key, item] of value.entries()) {
				const written = faJsonWrite(item, schema[1]);
				if (written !== undefined) parts.push(`${JSON.stringify(String(key))}:${written}`);
			}
			return `{${parts.join(",")}}`;
		}
		case "o":
			return faJsonFields(value, schema[1], "{");
		case "u": {
			const member = schema[1].find((/** @type {any} */ member) => faJsonIs(value, member));
			return member === undefined ? null : faJsonWrite(value, member);
		}
		case "F": {
			/** @type {string[]} */
			const names = schema[1];
			const flags = names.filter((_, index) =>
				typeof value === "bigint"
					? ((value >> BigInt(index)) & 1n) === 1n
					: ((value >>> index) & 1) === 1,
			);
			return JSON.stringify(flags);
		}
		case "E": {
			const member = schema[1].find((/** @type {any} */ member) => member[1] === value);
			if (member === undefined) return null;
			return member[2] ? String(member[1]) : JSON.stringify(member[0]);
		}
		case "N": {
			const member = schema[1].find((/** @type {any} */ member) => faJsonMember(value, member));
			if (member === undefined) return null;
			const [name, , kind, data, , names] = member;
			const type = `{"type":${JSON.stringify(name)}`;
			switch (kind) {
				case "l":
					return JSON.stringify(data);
				case "e":
					return `${type}}`;
				case "r":
				case "x":
					return data === null ? `${type}}` : faJsonFields(value, data[1], `${type},`);
				case "w":
					return `${type},"value":${faJsonFields(value, data[1], "{")}}`;
				case "V": {
					const inner =
						names.length === 1 && names[0] === "$v"
							? faJsonWrite(value.$v, data)
							: faJsonFields(value, data[1], "{");
					return `${type},"value":${inner ?? "null"}}`;
				}
				default:
					return `${type},"value":${faJsonWrite(value.$v, data) ?? "null"}}`;
			}
		}
	}
	return null;
}

/**
 * The fields of an object in the order of its schema, after `open` (`{`,
 * or the `"type"` of a member and a comma), then `}`.
 * @param {any} value
 * @param {any[]} fields
 * @param {string} open
 * @returns {string}
 */
function faJsonFields(value, fields, open) {
	const parts = [];
	for (const [name, field] of fields) {
		const written = faJsonWrite(value[name in Object.prototype ? `${name}$` : name], field);
		if (written !== undefined) parts.push(`${JSON.stringify(name)}:${written}`);
	}
	if (parts.length === 0 && open !== "{") return `${open.slice(0, -1)}}`;
	return `${open}${parts.join(",")}}`;
}

// @fa-helper faWriteBytes deps: faFileCall
/** @param {string} path @param {Uint8Array} bytes @returns {void} */
export function faWriteBytes(path, bytes) {
	faFileCall(path, (fs) => fs.writeFileSync(path, bytes));
}

// @fa-helper faWriteBytesAsync deps: faFileAwait
/** @param {FaTask} task @param {string} path @param {Uint8Array} bytes @returns {Promise<void>} */
export function faWriteBytesAsync(task, path, bytes) {
	return faFileAwait(task, path, (fs) => fs.writeFile(path, bytes));
}

// @fa-helper faJsonReadBytes deps: faUtf8Decoder
/** Parse bytes directly. Decode only JSON string tokens, never the input
 * document. JSON.parse validates escapes and scalar spelling.
 * An explicit container stack also accepts deeply nested documents.
 * @param {string | Uint8Array} bytes @returns {any}
 */
export function faJsonReadBytes(bytes) {
	if (typeof bytes === "string") return JSON.parse(bytes);
	let index = 0;
	const fail = () => {
		throw new SyntaxError(`Invalid JSON at byte ${index}`);
	};
	const space = () => {
		while (bytes[index] === 32 || bytes[index] === 9 || bytes[index] === 10 || bytes[index] === 13)
			index++;
	};
	const string = () => {
		const start = index++;
		while (index < bytes.length) {
			const byte = bytes[index++];
			if (byte === 34) return JSON.parse(faUtf8Decoder.decode(bytes.subarray(start, index)));
			if (byte === 92) index++;
		}
		return fail();
	};
	/** @type {{value: any, object: boolean, state: number, key: string}[]} */
	const stack = [];
	let root;
	let hasRoot = false;
	/** @param {any} value */
	const append = (value) => {
		const frame = stack[stack.length - 1];
		if (frame === undefined) {
			if (hasRoot) fail();
			root = value;
			hasRoot = true;
		} else if (frame.object) {
			Object.defineProperty(frame.value, frame.key, {
				value,
				writable: true,
				enumerable: true,
				configurable: true,
			});
			frame.state = 3;
		} else {
			frame.value.push(value);
			frame.state = 3;
		}
	};
	while (true) {
		space();
		const frame = stack[stack.length - 1];
		if (frame === undefined && hasRoot) {
			if (index !== bytes.length) fail();
			return root;
		}
		if (frame !== undefined) {
			const close = frame.object ? 125 : 93;
			if ((frame.state === 0 || frame.state === 3) && bytes[index] === close) {
				index++;
				stack.pop();
				continue;
			}
			if (frame.state === 3) {
				if (bytes[index++] !== 44) fail();
				frame.state = 1;
				space();
			}
			if (frame.object && frame.state !== 2) {
				if (bytes[index] !== 34) fail();
				frame.key = string();
				space();
				if (bytes[index++] !== 58) fail();
				frame.state = 2;
				space();
			}
		}
		const byte = bytes[index];
		if (byte === 123 || byte === 91) {
			const object = byte === 123;
			const value = object ? {} : [];
			append(value);
			stack.push({ value, object, state: 0, key: "" });
			index++;
		} else if (byte === 34) append(string());
		else {
			const start = index;
			while (
				index < bytes.length &&
				bytes[index] !== 44 &&
				bytes[index] !== 93 &&
				bytes[index] !== 125 &&
				bytes[index] !== 32 &&
				bytes[index] !== 9 &&
				bytes[index] !== 10 &&
				bytes[index] !== 13
			)
				index++;
			if (index === start) fail();
			let token = "";
			for (let offset = start; offset < index; offset++) {
				if (bytes[offset] > 127) fail();
				token += String.fromCharCode(bytes[offset]);
			}
			const value = JSON.parse(token);
			if (typeof value === "object" && value !== null) fail();
			append(value);
		}
	}
}
