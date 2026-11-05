// `Fields` values (types brief section 3.9): a number up to 32 flags, a
// bigint above, one bit per flag in declaration order. The generated code
// does the bit operations itself; the helpers take the table of the dotted
// paths of the flags, by bit (`["read", "nested.a"]`).

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
