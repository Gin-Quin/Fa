// Foreign code (documentation/advanced/foreign-code.md, JavaScript notes):
// C functions called through `bun:ffi` on Bun and `Deno.dlopen` on Deno.
//
// The compiler gives each `@extern` function one symbol table entry with
// Bun's type names (`i32`, `u64`, `f64`, `bool`, `ptr`, `function`,
// `void`); on Deno they are renamed (`ptr` is `pointer`). A pointer is a
// number on Bun and a pointer object on Deno; `NULL` is `undefined`, which
// is Fa's `none`. `Int64` and `UInt64` values are `bigint`s, like Fa's.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

// @fa-helper faFfiRuntime
/**
 * The FFI of the running JavaScript runtime.
 * @returns {{ kind: "bun", ffi: any } | { kind: "deno", ffi: any }}
 */
export function faFfiRuntime() {
	const deno = /** @type {any} */ (globalThis).Deno;
	if (deno !== undefined && typeof deno.dlopen === "function") {
		return { kind: "deno", ffi: deno };
	}
	const meta = /** @type {any} */ (import.meta);
	if (typeof meta.require === "function") {
		return { kind: "bun", ffi: meta.require("bun:ffi") };
	}
	throw new Error("C functions can only be called on Bun or Deno");
}

// @fa-helper faFfiType
/**
 * The name of a C type for the running runtime (Bun's names are the reference).
 * @param {string} kind `bun` or `deno`
 * @param {string} type
 * @returns {string}
 */
export function faFfiType(kind, type) {
	if (kind === "bun") {
		return type;
	}
	return type === "ptr" ? "pointer" : type;
}

// @fa-helper faFfiLibraries
/**
 * The paths of the libraries to search, in order: `build.link` (a name
 * `sqlite3` is `libsqlite3.so`, or `.dylib` on macOS), then the C library.
 * @param {string[]} libraries
 * @returns {string[]}
 */
export function faFfiLibraries(libraries) {
	const process = /** @type {any} */ (globalThis).process;
	const deno = /** @type {any} */ (globalThis).Deno;
	const platform = deno?.build?.os ?? process?.platform ?? "linux";
	const mac = platform === "darwin";
	const paths = libraries.map((name) =>
		name.includes("/") || name.includes(".") ? name : `lib${name}.${mac ? "dylib" : "so"}`,
	);
	paths.push(mac ? "libSystem.B.dylib" : "libc.so.6", mac ? "libm.dylib" : "libm.so.6");
	return paths;
}

// @fa-helper faFfiBind deps: faFfiRuntime faFfiType faFfiLibraries
/**
 * A C function: the symbol `symbol` of the first library that has it.
 * @param {string[]} libraries the libraries of `build.link`
 * @param {string} symbol
 * @param {string[]} args the C types of the parameters
 * @param {string} returns the C type of the result
 * @returns {(...values: any[]) => any}
 */
export function faFfiBind(libraries, symbol, args, returns) {
	const { kind, ffi } = faFfiRuntime();
	/** @type {unknown} */
	let problem;
	for (const path of faFfiLibraries(libraries)) {
		try {
			if (kind === "bun") {
				const library = ffi.dlopen(path, { [symbol]: { args, returns } });
				const call = library.symbols[symbol];
				return returns === "ptr"
					? (/** @type {any[]} */ ...values) => call(...values) ?? undefined
					: call;
			}
			const library = ffi.dlopen(path, {
				[symbol]: {
					parameters: args.map((type) => faFfiType(kind, type)),
					result: faFfiType(kind, returns),
				},
			});
			const call = library.symbols[symbol];
			return (/** @type {any[]} */ ...values) => {
				const converted = values.map((value, index) =>
					args[index] === "ptr" && ArrayBuffer.isView(value)
						? ffi.UnsafePointer.of(value)
						: (value ?? (args[index] === "ptr" ? null : value)),
				);
				return call(...converted) ?? undefined;
			};
		} catch (error) {
			problem = error;
		}
	}
	throw new Error(`the C function \`${symbol}\` was not found: ${String(problem)}`);
}

// @fa-helper faCString
/**
 * A string as a C string: its UTF-8 bytes and a final zero byte.
 * @param {string} text
 * @returns {Uint8Array}
 */
export function faCString(text) {
	return new TextEncoder().encode(`${text}\0`);
}

// @fa-helper faCArrayOf
/**
 * A typed array holding `count` values of a C type.
 * @param {string} type
 * @param {number} count
 * @returns {any}
 */
export function faCArrayOf(type, count) {
	switch (type) {
		case "i8":
			return new Int8Array(count);
		case "u8":
		case "bool":
			return new Uint8Array(count);
		case "i16":
			return new Int16Array(count);
		case "u16":
			return new Uint16Array(count);
		case "i32":
			return new Int32Array(count);
		case "u32":
			return new Uint32Array(count);
		case "i64":
			return new BigInt64Array(count);
		case "u64":
		case "ptr":
			return new BigUint64Array(count);
		case "f32":
			return new Float32Array(count);
		default:
			return new Float64Array(count);
	}
}

// @fa-helper faCArray deps: faCArrayOf
/**
 * The items of an array copied into a typed array, for C.
 * @param {any[]} items
 * @param {string} type the C type of the items
 * @returns {any}
 */
export function faCArray(items, type) {
	const typed = faCArrayOf(type, items.length);
	for (let index = 0; index < items.length; index += 1) {
		const item = items[index];
		typed[index] = type === "bool" ? (item ? 1 : 0) : item;
	}
	return typed;
}

// @fa-helper faCArrayBack
/**
 * Copies back what C wrote into the items of an array.
 * @param {any[]} items
 * @param {any} typed
 * @param {string} type
 */
export function faCArrayBack(items, typed, type) {
	for (let index = 0; index < items.length; index += 1) {
		items[index] = type === "bool" ? typed[index] !== 0 : typed[index];
	}
}

// @fa-helper faCBox deps: faCArrayOf
/**
 * A value in memory, whose address is given to C.
 * @param {any} value
 * @param {string} type
 * @returns {any}
 */
export function faCBox(value, type) {
	const box = faCArrayOf(type, 1);
	if (type === "ptr") {
		box[0] = value === undefined ? 0n : BigInt(value);
	} else {
		box[0] = type === "bool" ? (value ? 1 : 0) : value;
	}
	return box;
}

// @fa-helper faCUnbox
/**
 * The value C wrote into a box.
 * @param {any} box
 * @param {string} type
 * @returns {any}
 */
export function faCUnbox(box, type) {
	const value = box[0];
	if (type === "ptr") {
		return value === 0n ? undefined : Number(value);
	}
	return type === "bool" ? value !== 0 : value;
}

// @fa-helper faCCallback deps: faFfiRuntime faFfiType
/**
 * A Fa function as a C function pointer, valid until `close()`.
 * @param {(...values: any[]) => any} callback
 * @param {string[]} args
 * @param {string} returns
 * @returns {{ pointer: any, close: () => void }}
 */
export function faCCallback(callback, args, returns) {
	const { kind, ffi } = faFfiRuntime();
	if (kind === "bun") {
		const made = new ffi.JSCallback(callback, { args, returns });
		return { pointer: made.ptr, close: () => made.close() };
	}
	const made = new ffi.UnsafeCallback(
		{
			parameters: args.map((type) => faFfiType(kind, type)),
			result: faFfiType(kind, returns),
		},
		callback,
	);
	return { pointer: made.pointer, close: () => made.close() };
}

// @fa-helper faCSize
/**
 * The size of a C type, in bytes.
 * @param {string} type
 * @returns {number}
 */
export function faCSize(type) {
	switch (type) {
		case "i8":
		case "u8":
		case "bool":
			return 1;
		case "i16":
		case "u16":
			return 2;
		case "i32":
		case "u32":
		case "f32":
			return 4;
		default:
			return 8;
	}
}

// @fa-helper faPointerView deps: faFfiRuntime
/**
 * A `DataView` over the memory a pointer designates.
 * @param {any} pointer
 * @param {number} size
 * @returns {DataView}
 */
export function faPointerView(pointer, size) {
	const { kind, ffi } = faFfiRuntime();
	const buffer =
		kind === "bun"
			? ffi.toArrayBuffer(pointer, 0, size)
			: ffi.UnsafePointerView.getArrayBuffer(pointer, size);
	return new DataView(buffer);
}

// @fa-helper faPointerRead deps: faPointerView faCSize faFfiRuntime
/**
 * `pointer.read()`: the value of a C type at `pointer`, `index` items after it.
 * @param {any} pointer
 * @param {string} type
 * @param {number} [index]
 * @returns {any}
 */
export function faPointerRead(pointer, type, index = 0) {
	const size = faCSize(type);
	const view = faPointerView(pointer, size * (index + 1));
	const offset = size * index;
	const little = true;
	switch (type) {
		case "i8":
			return view.getInt8(offset);
		case "u8":
			return view.getUint8(offset);
		case "bool":
			return view.getUint8(offset) !== 0;
		case "i16":
			return view.getInt16(offset, little);
		case "u16":
			return view.getUint16(offset, little);
		case "i32":
			return view.getInt32(offset, little);
		case "u32":
			return view.getUint32(offset, little);
		case "i64":
			return view.getBigInt64(offset, little);
		case "u64":
			return view.getBigUint64(offset, little);
		case "f32":
			return view.getFloat32(offset, little);
		case "f64":
			return view.getFloat64(offset, little);
		default: {
			const address = view.getBigUint64(offset, little);
			if (address === 0n) {
				return undefined;
			}
			const { kind, ffi } = faFfiRuntime();
			return kind === "bun" ? Number(address) : ffi.UnsafePointer.create(address);
		}
	}
}

// @fa-helper faPointerWrite deps: faPointerView faCSize faFfiRuntime
/**
 * `pointer.write(value)`.
 * @param {any} pointer
 * @param {string} type
 * @param {any} value
 */
export function faPointerWrite(pointer, type, value) {
	const view = faPointerView(pointer, faCSize(type));
	const little = true;
	switch (type) {
		case "i8":
			return view.setInt8(0, value);
		case "u8":
			return view.setUint8(0, value);
		case "bool":
			return view.setUint8(0, value ? 1 : 0);
		case "i16":
			return view.setInt16(0, value, little);
		case "u16":
			return view.setUint16(0, value, little);
		case "i32":
			return view.setInt32(0, value, little);
		case "u32":
			return view.setUint32(0, value, little);
		case "i64":
			return view.setBigInt64(0, value, little);
		case "u64":
			return view.setBigUint64(0, value, little);
		case "f32":
			return view.setFloat32(0, value, little);
		case "f64":
			return view.setFloat64(0, value, little);
		default: {
			const { kind, ffi } = faFfiRuntime();
			const address =
				value === undefined
					? 0n
					: kind === "bun"
						? BigInt(value)
						: BigInt(ffi.UnsafePointer.value(value));
			return view.setBigUint64(0, address, little);
		}
	}
}

// @fa-helper faPointerOffset deps: faFfiRuntime faCSize
/**
 * `pointer.offset(count)`: the pointer `count` items of a C type after it.
 * @param {any} pointer
 * @param {string} type
 * @param {number} count
 * @returns {any}
 */
export function faPointerOffset(pointer, type, count) {
	const bytes = faCSize(type) * count;
	const { kind, ffi } = faFfiRuntime();
	return kind === "bun" ? pointer + bytes : ffi.UnsafePointer.offset(pointer, bytes);
}

// @fa-helper faPointerToArray deps: faPointerRead
/**
 * `pointer.toArray(count)`: `count` items copied into an array.
 * @param {any} pointer
 * @param {string} type
 * @param {number} count
 * @returns {any[]}
 */
export function faPointerToArray(pointer, type, count) {
	const items = [];
	for (let index = 0; index < count; index += 1) {
		items.push(faPointerRead(pointer, type, index));
	}
	return items;
}

// @fa-helper faCStringRead deps: faFfiRuntime
/**
 * `text.toString()`: a C string copied into a string.
 * @param {any} pointer
 * @returns {string}
 */
export function faCStringRead(pointer) {
	const { kind, ffi } = faFfiRuntime();
	return kind === "bun"
		? new ffi.CString(pointer).toString()
		: ffi.UnsafePointerView.getCString(pointer);
}
