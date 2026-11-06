// Foreign code (documentation/advanced/foreign-code.md, JavaScript notes):
// C functions called through `bun:ffi` on Bun.
//
// The compiler gives each `@extern` function one symbol table entry with
// Bun's type names (`i32`, `u64`, `f64`, `bool`, `ptr`, `function`,
// `void`). A pointer is a number; `NULL` is `undefined`, which is Fa's
// `none`. `Int64` and `UInt64` values are `bigint`s, like Fa's.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

// @fa-helper faFfiRuntime
/**
 * The FFI of the running JavaScript runtime: `bun:ffi`.
 * @returns {any}
 */
export function faFfiRuntime() {
	const meta = /** @type {any} */ (import.meta);
	if (typeof meta.require === "function") {
		return meta.require("bun:ffi");
	}
	throw new Error("C functions can only be called on Bun");
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
	const platform = process?.platform ?? "linux";
	const mac = platform === "darwin";
	const paths = libraries.map((name) =>
		name.includes("/") || name.includes(".") ? name : `lib${name}.${mac ? "dylib" : "so"}`,
	);
	paths.push(mac ? "libSystem.B.dylib" : "libc.so.6", mac ? "libm.dylib" : "libm.so.6");
	return paths;
}

// @fa-helper faFfiBind deps: faFfiRuntime faFfiLibraries
/**
 * A C function: the symbol `symbol` of the first library that has it.
 * @param {string[]} libraries the libraries of `build.link`
 * @param {string} symbol
 * @param {string[]} args the C types of the parameters
 * @param {string} returns the C type of the result
 * @returns {(...values: any[]) => any}
 */
export function faFfiBind(libraries, symbol, args, returns) {
	const ffi = faFfiRuntime();
	/** @type {unknown} */
	let problem;
	for (const path of faFfiLibraries(libraries)) {
		try {
			const library = ffi.dlopen(path, { [symbol]: { args, returns } });
			const call = library.symbols[symbol];
			return returns === "ptr"
				? (/** @type {any[]} */ ...values) => call(...values) ?? undefined
				: call;
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
 * @param {any} type
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

// @fa-helper faCCallback deps: faFfiRuntime
/**
 * A Fa function as a C function pointer, valid until `close()`.
 * @param {(...values: any[]) => any} callback
 * @param {string[]} args
 * @param {string} returns
 * @returns {{ pointer: any, close: () => void }}
 */
export function faCCallback(callback, args, returns) {
	const ffi = faFfiRuntime();
	const made = new ffi.JSCallback(callback, { args, returns });
	return { pointer: made.ptr, close: () => made.close() };
}

// @fa-helper faCSize
/**
 * The size of a C type, in bytes.
 * @param {any} type
 * @returns {number}
 */
export function faCSize(type) {
	if (typeof type === "object") {
		return type.array === undefined ? type.size : type.count * type.stride;
	}
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
	return new DataView(faFfiRuntime().toArrayBuffer(pointer, 0, size));
}

// @fa-helper faPointerRead deps: faPointerView faCSize
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
			return address === 0n ? undefined : Number(address);
		}
	}
}

// @fa-helper faPointerWrite deps: faPointerView faCSize
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
			const address = value === undefined ? 0n : BigInt(value);
			return view.setBigUint64(0, address, little);
		}
	}
}

// @fa-helper faPointerOffset deps: faCSize
/**
 * `pointer.offset(count)`: the pointer `count` items of a C type after it.
 * @param {any} pointer
 * @param {string} type
 * @param {number} count
 * @returns {any}
 */
export function faPointerOffset(pointer, type, count) {
	return pointer + faCSize(type) * count;
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
	const ffi = faFfiRuntime();
	return new ffi.CString(pointer).toString();
}

// @fa-helper faPointerMemory
/**
 * Opaque Fa values live behind real C addresses; their JavaScript values
 * stay rooted until free. C-compatible values live in the C block itself.
 * @type {{ values: Map<number, { value: any }>, malloc: ((...values: any[]) => any) | null, free: ((...values: any[]) => any) | null }}
 */
export const faPointerMemory = { values: new Map(), malloc: null, free: null };

// @fa-helper faPointerFree deps: faPointerMemory faFfiBind
/**
 * Destroys an owned Fa value, then releases its C allocation.
 * @param {number} pointer
 * @param {((value: any) => void) | undefined} [drop]
 */
export function faPointerFree(pointer, drop) {
	const block = faPointerMemory.values.get(pointer);
	faPointerMemory.values.delete(pointer);
	try {
		if (block !== undefined && drop !== undefined) drop(block.value);
	} finally {
		faPointerMemory.free ??= faFfiBind([], "free", ["ptr"], "void");
		faPointerMemory.free(pointer);
	}
}

// @fa-helper faPointerAllocate deps: faPointerMemory faFfiBind faCSize faPointerView faCWrite faPointerFree
/**
 * Moves a value into a C allocation. A null spec denotes opaque Fa data.
 * @param {any} value
 * @param {any} spec
 * @returns {number}
 */
export function faPointerAllocate(value, spec) {
	faPointerMemory.malloc ??= faFfiBind([], "malloc", ["u64"], "ptr");
	const size = Math.max(1, spec === null ? 8 : faCSize(spec));
	const pointer = faPointerMemory.malloc(BigInt(size));
	if (pointer === undefined) throw new Error("out of memory");
	try {
		if (spec === null) faPointerMemory.values.set(pointer, { value });
		else faCWrite(faPointerView(pointer, size), 0, spec, value);
	} catch (error) {
		faPointerFree(pointer);
		throw error;
	}
	return pointer;
}

// @fa-helper faPointerReadValue deps: faPointerMemory faCRead faPointerView faCSize
/**
 * Reads the underlying value; the compiler copies opaque data for read().
 * @param {number} pointer
 * @param {any} spec
 * @param {boolean} bigint
 * @returns {any}
 */
export function faPointerReadValue(pointer, spec, bigint) {
	if (spec !== null) return faCRead(faPointerView(pointer, faCSize(spec)), 0, spec, bigint);
	const block = faPointerMemory.values.get(pointer);
	if (block === undefined) throw new Error("unknown Fa allocation");
	return block.value;
}

// @fa-helper faPointerWriteValue deps: faPointerMemory faCWrite faPointerView faCSize
/**
 * Replaces a value, destroying the previous value after the new one was evaluated.
 * @param {number} pointer
 * @param {any} spec
 * @param {any} value
 * @param {((value: any) => void) | undefined} [drop]
 */
export function faPointerWriteValue(pointer, spec, value, drop) {
	if (spec !== null) {
		faCWrite(faPointerView(pointer, faCSize(spec)), 0, spec, value);
		return;
	}
	const block = faPointerMemory.values.get(pointer);
	if (block === undefined) throw new Error("unknown Fa allocation");
	if (drop !== undefined) drop(block.value);
	faPointerMemory.values.set(pointer, { value });
}

// @fa-helper faCReference deps: faCRead faCWrite
/**
 * A view of a C struct or fixed array, including writes through nested fields.
 * @param {DataView} view
 * @param {number} offset
 * @param {any} spec
 * @param {boolean} bigint
 * @returns {any}
 */
export function faCReference(view, offset, spec, bigint) {
	if (typeof spec === "string") return faCRead(view, offset, spec, bigint);
	const array = spec.array !== undefined;
	const target = array
		? new Array(spec.count).fill(undefined)
		: Object.fromEntries(spec.fields.map(([name]) => [name, undefined]));
	const field = (key) => {
		if (typeof key !== "string") return undefined;
		if (!array) return spec.fields.find(([name]) => name === key);
		const index = Number(key);
		return Number.isInteger(index) && index >= 0 && index < spec.count && String(index) === key
			? [key, spec.array, index * spec.stride, spec.bigint]
			: undefined;
	};
	return new Proxy(target, {
		get(target, key, receiver) {
			const part = field(key);
			return part === undefined
				? Reflect.get(target, key, receiver)
				: faCReference(view, offset + part[2], part[1], part[3]);
		},
		set(target, key, value) {
			const part = field(key);
			if (part === undefined) return Reflect.set(target, key, value);
			faCWrite(view, offset + part[2], part[1], value);
			return true;
		},
	});
}

// @fa-helper faPointerValue deps: faPointerReadValue faCReference faPointerView faCSize
/**
 * pointer.value borrows the stored value rather than copying it.
 * @param {number} pointer
 * @param {any} spec
 * @param {boolean} bigint
 * @returns {any}
 */
export function faPointerValue(pointer, spec, bigint) {
	return spec === null
		? faPointerReadValue(pointer, spec, bigint)
		: faCReference(faPointerView(pointer, faCSize(spec)), 0, spec, bigint);
}

// @fa-helper faPointerCell deps: faPointerValue faPointerWriteValue
/**
 * An accessor place for assignments and references to pointer.value.
 * Assignment lowering destroys replaced resources before invoking the setter.
 * @param {number} pointer
 * @param {any} spec
 * @param {boolean} bigint
 * @returns {{ value: any }}
 */
export function faPointerCell(pointer, spec, bigint) {
	return {
		get value() {
			return faPointerValue(pointer, spec, bigint);
		},
		set value(value) {
			faPointerWriteValue(pointer, spec, value);
		},
	};
}

// @fa-helper faPointerToValues deps: faCRead faPointerView faCSize
/**
 * Copies C values, keeping the Fa representation of 64-bit integers.
 * @param {number} pointer
 * @param {any} spec
 * @param {boolean} bigint
 * @param {number} count
 * @returns {any[]}
 */
export function faPointerToValues(pointer, spec, bigint, count) {
	if (count === 0) return [];
	const size = faCSize(spec);
	const view = faPointerView(pointer, size * count);
	const result = [];
	for (let index = 0; index < count; index += 1) {
		result.push(faCRead(view, index * size, spec, bigint));
	}
	return result;
}

// @fa-helper faCWrite
/**
 * Writes a Fa value into memory with the C layout `spec` (from the
 * compiler: a C type name, a struct `{ size, fields }`, or an array
 * `{ array, count, stride, bigint }`), little-endian.
 * @param {DataView} view
 * @param {number} offset
 * @param {any} spec
 * @param {any} value
 * @returns {void}
 */
export function faCWrite(view, offset, spec, value) {
	if (typeof spec === "string") {
		switch (spec) {
			case "bool":
				view.setUint8(offset, value ? 1 : 0);
				return;
			case "i8":
				view.setInt8(offset, value);
				return;
			case "u8":
				view.setUint8(offset, value);
				return;
			case "i16":
				view.setInt16(offset, value, true);
				return;
			case "u16":
				view.setUint16(offset, value, true);
				return;
			case "i32":
				view.setInt32(offset, value, true);
				return;
			case "u32":
				view.setUint32(offset, value, true);
				return;
			case "i64":
				view.setBigInt64(offset, BigInt(value), true);
				return;
			case "u64":
				view.setBigUint64(offset, BigInt(value), true);
				return;
			case "f32":
				view.setFloat32(offset, value, true);
				return;
			case "f64":
				view.setFloat64(offset, value, true);
				return;
			default:
				// a pointer: a number, `none` is `NULL`
				view.setBigUint64(offset, value === undefined ? 0n : BigInt(value), true);
				return;
		}
	}
	if (spec.array !== undefined) {
		for (let index = 0; index < spec.count; index += 1) {
			faCWrite(view, offset + index * spec.stride, spec.array, value[index]);
		}
		return;
	}
	for (const [name, field, at] of spec.fields) {
		faCWrite(view, offset + at, field, value[name]);
	}
}

// @fa-helper faCRead
/**
 * Reads a Fa value from memory with the C layout `spec` (see `faCWrite`):
 * an `Int64` stays a `bigint` when `bigint`, else it is a number.
 * @param {DataView} view
 * @param {number} offset
 * @param {any} spec
 * @param {boolean} bigint
 * @returns {any}
 */
export function faCRead(view, offset, spec, bigint) {
	if (typeof spec === "string") {
		switch (spec) {
			case "bool":
				return view.getUint8(offset) !== 0;
			case "i8":
				return view.getInt8(offset);
			case "u8":
				return view.getUint8(offset);
			case "i16":
				return view.getInt16(offset, true);
			case "u16":
				return view.getUint16(offset, true);
			case "i32":
				return view.getInt32(offset, true);
			case "u32":
				return view.getUint32(offset, true);
			case "i64": {
				const value = view.getBigInt64(offset, true);
				return bigint ? value : Number(value);
			}
			case "u64": {
				const value = view.getBigUint64(offset, true);
				return bigint ? value : Number(value);
			}
			case "f32":
				return view.getFloat32(offset, true);
			case "f64":
				return view.getFloat64(offset, true);
			default: {
				const address = view.getBigUint64(offset, true);
				return address === 0n ? undefined : Number(address);
			}
		}
	}
	if (spec.array !== undefined) {
		const items = [];
		for (let index = 0; index < spec.count; index += 1) {
			items.push(faCRead(view, offset + index * spec.stride, spec.array, spec.bigint));
		}
		return items;
	}
	/** @type {Record<string, any>} */
	const value = {};
	for (const [name, field, at, big] of spec.fields) {
		value[name] = faCRead(view, offset + at, field, big);
	}
	return value;
}

// @fa-helper faFfiBindStruct deps: faFfiRuntime faFfiLibraries faCWrite faCRead
/**
 * A C function taking or returning a `CStruct` by value, which `bun:ffi`
 * can't pass: its address from the first library that has `symbol`, and a
 * shim compiled by TinyCC (`cc`) from `source` (`fa_shim(f, p0…, out)`)
 * that calls it with the arguments read from memory, under the platform's
 * C ABI.
 * @param {string[]} libraries the libraries of `build.link`
 * @param {string} symbol
 * @param {string} source the C source of the shim
 * @param {any[]} params the C layouts of the parameters (`faCWrite`)
 * @param {[any, number, boolean]} returns the layout of the result, its
 *   size, and whether it is a `bigint`
 * @returns {(...values: any[]) => any}
 */
export function faFfiBindStruct(libraries, symbol, source, params, returns) {
	const ffi = faFfiRuntime();
	/** @type {number | undefined} */
	let address;
	/** @type {unknown} */
	let problem;
	for (const path of faFfiLibraries(libraries)) {
		try {
			const library = ffi.dlopen(path, { [symbol]: { args: [], returns: "void" } });
			address = library.symbols[symbol].ptr;
			break;
		} catch (error) {
			problem = error;
		}
	}
	if (address === undefined) {
		throw new Error(`the C function \`${symbol}\` was not found: ${String(problem)}`);
	}
	/** @type {any} */
	const process = /** @type {any} */ (globalThis).process;
	const fs = process.getBuiltinModule("node:fs");
	const os = process.getBuiltinModule("node:os");
	// `cc` reads a file: one per source, named by its hash
	let hash = 0;
	for (let index = 0; index < source.length; index += 1) {
		hash = (Math.imul(hash, 31) + source.charCodeAt(index)) | 0;
	}
	const file = `${os.tmpdir()}/fa-shim-${(hash >>> 0).toString(16)}-${source.length}.c`;
	fs.writeFileSync(file, source);
	const shim = ffi.cc({
		source: file,
		symbols: { fa_shim: { args: Array(params.length + 2).fill("ptr"), returns: "void" } },
	}).symbols.fa_shim;
	const [result, resultSize, bigint] = returns;
	return (/** @type {any[]} */ ...values) => {
		const pointers = params.map((spec, index) => {
			const value = values[index];
			const size = typeof spec === "string" ? 8 : spec.size;
			const memory = new Uint8Array(Math.max(size, 8));
			// a value already in memory (a C string, an array): its address
			const given = ArrayBuffer.isView(value) ? ffi.ptr(value) : value;
			faCWrite(new DataView(memory.buffer), 0, spec, given);
			return memory;
		});
		const out = new Uint8Array(Math.max(resultSize, 8));
		shim(address, ...pointers.map((memory) => ffi.ptr(memory)), ffi.ptr(out));
		if (result === "void") {
			return undefined;
		}
		return faCRead(new DataView(out.buffer), 0, result, bigint);
	};
}
