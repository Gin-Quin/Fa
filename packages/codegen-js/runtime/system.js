// `std.fs`, `std.time` and `std.json` (advanced brief D9). `std.time.now()`
// compiles to `Date.now()` and needs no helper.
//
// - `File.read(path)`, `File.readBytes(path)` and `File.write(path, text)`
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
import { faAlive } from "./tasks.js";

/** @typedef {import("./tasks.js").FaTask} FaTask */

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

// @fa-helper faReadFile deps: faFileCall
/**
 * `File.read(path)`: the whole file, as UTF-8.
 * @param {string} path
 * @returns {string}
 */
export function faReadFile(path) {
	return faFileCall(path, (fs) => fs.readFileSync(path, "utf8"));
}

// @fa-helper faReadBytes deps: faFileCall
/**
 * `File.readBytes(path)`: the whole file, as an `Array(Byte)`.
 * @param {string} path
 * @returns {number[]}
 */
export function faReadBytes(path) {
	return faFileCall(path, (fs) => Array.from(fs.readFileSync(path)));
}

// @fa-helper faWriteFile deps: faFileCall
/**
 * `File.write(path, text)`: creates or replaces the file, in UTF-8.
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

// @fa-helper faReadFileAsync deps: faFileAwait
/**
 * `await File.read(path)`.
 * @param {FaTask} task
 * @param {string} path
 * @returns {Promise<string>}
 */
export function faReadFileAsync(task, path) {
	return faFileAwait(task, path, (fs) => fs.readFile(path, "utf8"));
}

// @fa-helper faReadBytesAsync deps: faFileAwait
/**
 * `await File.readBytes(path)`.
 * @param {FaTask} task
 * @param {string} path
 * @returns {Promise<number[]>}
 */
export function faReadBytesAsync(task, path) {
	return faFileAwait(task, path, async (fs) => Array.from(await fs.readFile(path)));
}

// @fa-helper faWriteFileAsync deps: faFileAwait
/**
 * `await File.write(path, text)`.
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
