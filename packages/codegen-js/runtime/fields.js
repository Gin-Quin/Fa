// `Fields` values (types brief section 3.9): a number up to 32 flags, a
// bigint above, one bit per flag in declaration order. The generated code
// does the bit operations itself; the helpers take the table of the dotted
// paths of the flags, by bit (`["read", "nested.a"]`).

import { faCopy } from "./copy.js";

// @fa-helper faFieldsToStringArray
/**
 * `set.toStringArray()`: the paths of the flags of a set, in bit order.
 * @param {number | bigint} value
 * @param {readonly string[]} names
 * @returns {string[]}
 */
export function faFieldsToStringArray(value, names) {
	/** @type {string[]} */
	const flags = [];
	if (typeof value === "bigint") {
		for (let index = 0; index < names.length; index++) {
			if (((value >> BigInt(index)) & 1n) === 1n) {
				flags.push(/** @type {string} */ (names[index]));
			}
		}
		return flags;
	}
	for (let index = 0; index < names.length; index++) {
		if (((value >>> index) & 1) === 1) {
			flags.push(/** @type {string} */ (names[index]));
		}
	}
	return flags;
}

// @fa-helper faFieldAt
/**
 * `value[key]` in an `@for` over a key set: the field at a dotted path.
 * @param {any} value
 * @param {string} path
 * @returns {unknown}
 */
export function faFieldAt(value, path) {
	let current = value;
	for (const name of path.split(".")) {
		current = current[name];
	}
	return current;
}

// @fa-helper faFormatFields deps: faFieldsToStringArray
/**
 * Prints a set as a key set: `{ read, nested { a, b } }`, `{}` when empty.
 * @param {number | bigint} value
 * @param {readonly string[]} names
 * @returns {string}
 */
export function faFormatFields(value, names) {
	/** @typedef {[string, FieldsEntry[] | undefined]} FieldsEntry */
	/** @type {FieldsEntry[]} */
	const root = [];
	for (const path of faFieldsToStringArray(value, names)) {
		const parts = path.split(".");
		let level = root;
		for (let index = 0; index < parts.length - 1; index++) {
			// the flags of a group are next to each other
			let group = level[level.length - 1];
			if (group === undefined || group[0] !== parts[index] || group[1] === undefined) {
				group = [/** @type {string} */ (parts[index]), []];
				level.push(group);
			}
			level = /** @type {FieldsEntry[]} */ (group[1]);
		}
		level.push([/** @type {string} */ (parts[parts.length - 1]), undefined]);
	}
	/**
	 * @param {FieldsEntry[]} level
	 * @returns {string}
	 */
	const write = (level) =>
		level.length === 0
			? "{}"
			: `{ ${level
					.map(([name, members]) => (members === undefined ? name : `${name} ${write(members)}`))
					.join(", ")} }`;
	return write(root);
}

// @fa-helper faPick deps: faCopy
/**
 * `pick(value, keys)` with a key set only known at run time (a key-set
 * parameter of a generic function): a new object holding a copy of the
 * fields at the dotted paths, in their order.
 * @param {any} value
 * @param {readonly string[]} paths
 * @returns {any}
 */
export function faPick(value, paths) {
	/** @type {any} */
	const result = {};
	for (const path of paths) {
		const parts = path.split(".");
		let source = value;
		let target = result;
		for (let index = 0; index < parts.length - 1; index++) {
			const name = /** @type {string} */ (parts[index]);
			source = source[name];
			target[name] ??= {};
			target = target[name];
		}
		const name = /** @type {string} */ (parts[parts.length - 1]);
		target[name] = faCopy(source[name]);
	}
	return result;
}

// @fa-helper faOmit deps: faCopy
/**
 * `omit(value, keys)` with a key set only known at run time: a copy of the
 * value without the fields at the dotted paths.
 * @param {any} value
 * @param {readonly string[]} paths
 * @returns {any}
 */
export function faOmit(value, paths) {
	/** @type {any} */
	const result = faCopy(value);
	for (const path of paths) {
		const parts = path.split(".");
		let target = result;
		for (let index = 0; index < parts.length - 1; index++) {
			target = target[/** @type {string} */ (parts[index])];
		}
		delete target[/** @type {string} */ (parts[parts.length - 1])];
	}
	return result;
}
