// Generated from @gin-quin/codec; SPDX-License-Identifier: MIT.
// Source: https://github.com/Gin-Quin/codec (local Fa integration).
// Source SHA-256: 6fcc9177ebb76fae3c92ef03c657a99c3fa064b8124d4242ab82ce9418623b95
// Regenerate: bun scripts/build-fa-runtime.ts <destination>
// @fa-helper faCodec deps:
export const faCodec = (() => {
	var __defProp = Object.defineProperty;
	var __returnValue = (v) => v;
	function __exportSetter(name, newValue) {
		this[name] = __returnValue.bind(null, newValue);
	}
	var __export = (target, all) => {
		for (var name in all)
			__defProp(target, name, {
				get: all[name],
				enumerable: true,
				configurable: true,
				set: __exportSetter.bind(all, name),
			});
	};

	// source/runtime.ts
	var exports_runtime = {};
	__export(exports_runtime, {
		Decoder: () => Decoder,
		Encoder: () => Encoder,
		createDecoder: () => createDecoder,
		createEncoder: () => createEncoder,
		ensureCapacity: () => ensureCapacity,
		fromFunctions: () => createStreamCodec,
		readCachedVarString: () => readCachedVarString,
		readDate: () => readDate,
		readFloat32: () => readFloat32,
		readFloat64: () => readFloat64,
		readReference: () => readReference,
		readUint8: () => readUint8,
		readUint8Array: () => readUint8Array,
		readVarBigInt: () => readVarBigInt,
		readVarInt: () => readVarInt,
		readVarString: () => readVarString,
		readVarUint: () => readVarUint,
		readVarUint8Array: () => readVarUint8Array,
		toUint8Array: () => toUint8Array,
		writeDate: () => writeDate,
		writeFloat32: () => writeFloat32,
		writeFloat64: () => writeFloat64,
		writeReference: () => writeReference,
		writeUint8: () => writeUint8,
		writeUint8Array: () => writeUint8Array,
		writeVarBigInt: () => writeVarBigInt,
		writeVarInt: () => writeVarInt,
		writeVarString: () => writeVarString,
		writeVarUint: () => writeVarUint,
		writeVarUint8Array: () => writeVarUint8Array,
	});

	// source/binary.ts
	var bit8 = 128;
	var bits7 = 127;
	var defaultBufferSize = 128;
	var minInt32 = -2147483648;
	var maxInt32 = 2147483647;
	var maxUint32 = 4294967295;
	var maxDirectAsciiStringSize = 64;
	var textEncoder = new TextEncoder();
	var textDecoder = new TextDecoder();
	var stringBuffer = new Uint8Array(30000);
	var maxStringBufferSize = stringBuffer.length / 3;

	class Encoder {
		buffer;
		view;
		pos;
		fixed;
		context;
		constructor(initialSize = defaultBufferSize, offset = 0, context) {
			if (!Number.isSafeInteger(offset) || offset < 0) {
				throw new RangeError("Encoder offset must be a nonnegative safe integer");
			}
			this.fixed = initialSize instanceof Uint8Array;
			this.buffer = typeof initialSize === "number" ? new Uint8Array(initialSize) : initialSize;
			if (offset > this.buffer.length) {
				throw new RangeError("Encoder offset is outside its byte window");
			}
			this.view = new DataView(this.buffer.buffer, this.buffer.byteOffset, this.buffer.byteLength);
			this.pos = offset;
			this.context = context;
		}
	}

	class Decoder {
		arr;
		view;
		pos;
		context;
		constructor(uint8Array, pos = 0, context) {
			this.arr = uint8Array;
			this.view = new DataView(uint8Array.buffer, uint8Array.byteOffset, uint8Array.byteLength);
			this.pos = pos;
			this.context = context;
		}
	}
	function createEncoder(initialSize, offset = 0, context) {
		return new Encoder(initialSize, offset, context);
	}
	function createDecoder(uint8Array, context) {
		return new Decoder(uint8Array, 0, context);
	}
	function writeReference(encoder, name, value) {
		const references = encoder.context?.references;
		if (!references) {
			throw new TypeError(`Encoding reference ${name} requires a reference writer`);
		}
		const id = references.write(name, value);
		if (!Number.isSafeInteger(id) || id < 0) {
			throw new RangeError("Reference identifiers must be nonnegative safe integers");
		}
		writeVarUint(encoder, id);
	}
	function readReference(decoder, name) {
		const references = decoder.context?.references;
		if (!references) {
			throw new TypeError(`Decoding reference ${name} requires a reference reader`);
		}
		return references.read(name, readVarUint(decoder));
	}
	function toUint8Array(encoder) {
		return encoder.buffer.slice(0, encoder.pos);
	}
	function ensureCapacity(encoder, byteLength) {
		const required = encoder.pos + byteLength;
		if (required <= encoder.buffer.length) {
			return;
		}
		if (encoder.fixed) {
			throw new RangeError("Encoded value exceeds the target byte window");
		}
		let nextLength = Math.max(encoder.buffer.length * 2, defaultBufferSize);
		while (nextLength < required) {
			nextLength *= 2;
		}
		const next = new Uint8Array(nextLength);
		next.set(encoder.buffer);
		encoder.buffer = next;
		encoder.view = new DataView(next.buffer);
	}
	function writeUint8(encoder, value) {
		ensureCapacity(encoder, 1);
		encoder.buffer[encoder.pos++] = value;
	}
	function readUint8(decoder) {
		if (decoder.pos >= decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		return decoder.arr[decoder.pos++];
	}
	function writeUint8Array(encoder, value) {
		ensureCapacity(encoder, value.byteLength);
		encoder.buffer.set(value, encoder.pos);
		encoder.pos += value.byteLength;
	}
	function readUint8Array(decoder, byteLength) {
		if (decoder.pos + byteLength > decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		const value = new Uint8Array(
			decoder.arr.buffer,
			decoder.arr.byteOffset + decoder.pos,
			byteLength,
		);
		decoder.pos += byteLength;
		return value;
	}
	function writeFloat32(encoder, value) {
		ensureCapacity(encoder, 4);
		if (encoder.view.byteLength < encoder.pos + 4) {
			encoder.view = new DataView(
				encoder.buffer.buffer,
				encoder.buffer.byteOffset,
				encoder.buffer.byteLength,
			);
		}
		encoder.view.setFloat32(encoder.pos, value, true);
		encoder.pos += 4;
	}
	function readFloat32(decoder) {
		if (decoder.pos + 4 > decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		if (decoder.view.byteLength < decoder.pos + 4) {
			decoder.view = new DataView(
				decoder.arr.buffer,
				decoder.arr.byteOffset,
				decoder.arr.byteLength,
			);
		}
		const value = decoder.view.getFloat32(decoder.pos, true);
		decoder.pos += 4;
		return value;
	}
	function writeFloat64(encoder, value) {
		ensureCapacity(encoder, 8);
		if (encoder.view.byteLength < encoder.pos + 8) {
			encoder.view = new DataView(
				encoder.buffer.buffer,
				encoder.buffer.byteOffset,
				encoder.buffer.byteLength,
			);
		}
		encoder.view.setFloat64(encoder.pos, value, true);
		encoder.pos += 8;
	}
	function readFloat64(decoder) {
		if (decoder.pos + 8 > decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		if (decoder.view.byteLength < decoder.pos + 8) {
			decoder.view = new DataView(
				decoder.arr.buffer,
				decoder.arr.byteOffset,
				decoder.arr.byteLength,
			);
		}
		const value = decoder.view.getFloat64(decoder.pos, true);
		decoder.pos += 8;
		return value;
	}
	function writeVarUint(encoder, value) {
		if (value >= 0 && value <= bits7) {
			ensureCapacity(encoder, 1);
			encoder.buffer[encoder.pos++] = value;
			return;
		}
		if (value >= 0 && value <= maxUint32) {
			ensureCapacity(encoder, 5);
			const buffer = encoder.buffer;
			let pos = encoder.pos;
			let remaining = value >>> 0;
			while (remaining > bits7) {
				buffer[pos++] = bit8 | (remaining & bits7);
				remaining >>>= 7;
			}
			buffer[pos++] = remaining;
			encoder.pos = pos;
			return;
		}
		ensureCapacity(encoder, 8);
		const buffer = encoder.buffer;
		let pos = encoder.pos;
		while (value > bits7) {
			buffer[pos++] = bit8 | (bits7 & value);
			value = Math.floor(value / 128);
		}
		buffer[pos++] = bits7 & value;
		encoder.pos = pos;
	}
	function readVarUint(decoder) {
		const buffer = decoder.arr;
		let pos = decoder.pos;
		const length = buffer.length;
		if (pos >= length) {
			throw new Error("Unexpected end of array");
		}
		let byte = buffer[pos++];
		let value = byte & bits7;
		if (byte < bit8) {
			decoder.pos = pos;
			return value;
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 7;
		if (byte < bit8) {
			decoder.pos = pos;
			return value;
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 14;
		if (byte < bit8) {
			decoder.pos = pos;
			return value;
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 21;
		if (byte < bit8) {
			decoder.pos = pos;
			return value;
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value += (byte & bits7) * 268435456;
		decoder.pos = pos;
		if (byte < bit8) {
			return value;
		}
		return readVarUintSlow(decoder, value, 34359738368);
	}
	function readVarUintSlow(decoder, value, multiplier) {
		const length = decoder.arr.length;
		while (decoder.pos < length) {
			const byte = decoder.arr[decoder.pos++];
			value += (byte & bits7) * multiplier;
			multiplier *= 128;
			if (byte < bit8) {
				return value;
			}
			if (value > Number.MAX_SAFE_INTEGER) {
				throw new Error("Integer out of range");
			}
		}
		throw new Error("Unexpected end of array");
	}
	function writeVarInt(encoder, value) {
		if (value >= minInt32 && value <= maxInt32) {
			writeVarUint(encoder, ((value << 1) ^ (value >> 31)) >>> 0);
			return;
		}
		let remaining = value >= 0 ? BigInt(value) << 1n : (BigInt(-value) << 1n) - 1n;
		while (remaining > 0x7fn) {
			writeUint8(encoder, bit8 | Number(remaining & 0x7fn));
			remaining >>= 7n;
		}
		writeUint8(encoder, Number(remaining));
	}
	function readVarInt(decoder) {
		const buffer = decoder.arr;
		let pos = decoder.pos;
		const length = buffer.length;
		if (pos >= length) {
			throw new Error("Unexpected end of array");
		}
		let byte = buffer[pos++];
		let value = byte & bits7;
		if (byte < bit8) {
			decoder.pos = pos;
			return zigZagDecodeUint32(value);
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 7;
		if (byte < bit8) {
			decoder.pos = pos;
			return zigZagDecodeUint32(value);
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 14;
		if (byte < bit8) {
			decoder.pos = pos;
			return zigZagDecodeUint32(value);
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value |= (byte & bits7) << 21;
		if (byte < bit8) {
			decoder.pos = pos;
			return zigZagDecodeUint32(value);
		}
		if (pos >= length) {
			decoder.pos = pos;
			throw new Error("Unexpected end of array");
		}
		byte = buffer[pos++];
		value += (byte & bits7) * 268435456;
		decoder.pos = pos;
		if (byte < bit8) {
			if (value <= maxUint32) {
				return zigZagDecodeUint32(value);
			}
			return zigZagDecodeSafeBigInt(BigInt(value));
		}
		return readVarIntSlow(decoder, BigInt(value), 35n);
	}
	function readVarIntSlow(decoder, value, shift) {
		const length = decoder.arr.length;
		while (decoder.pos < length) {
			const byte = decoder.arr[decoder.pos++];
			value |= BigInt(byte & bits7) << shift;
			shift += 7n;
			if (byte < bit8) {
				return zigZagDecodeSafeBigInt(value);
			}
		}
		throw new Error("Unexpected end of array");
	}
	function zigZagDecodeUint32(value) {
		return ((value >>> 1) ^ -(value & 1)) | 0;
	}
	function zigZagDecodeSafeBigInt(value) {
		const decoded = (value & 1n) === 0n ? value >> 1n : -((value >> 1n) + 1n);
		const result = Number(decoded);
		if (!Number.isSafeInteger(result)) {
			throw new Error("Integer out of range");
		}
		return result;
	}
	function zigZagDecodeBigInt(value) {
		return (value & 1n) === 0n ? value >> 1n : -((value >> 1n) + 1n);
	}
	function writeVarBigInt(encoder, value, maxBytes = 128) {
		validateMaxBytes(maxBytes);
		let remaining = value >= 0n ? value << 1n : (-value << 1n) - 1n;
		let byteLength = 1;
		while (remaining > 0x7fn) {
			if (byteLength >= maxBytes) {
				throw new Error(`BigInt exceeds maxBytes (${maxBytes})`);
			}
			writeUint8(encoder, bit8 | Number(remaining & 0x7fn));
			remaining >>= 7n;
			byteLength++;
		}
		writeUint8(encoder, Number(remaining));
	}
	function readVarBigInt(decoder, maxBytes = 128) {
		validateMaxBytes(maxBytes);
		let byte = readUint8(decoder);
		let value = BigInt(byte & bits7);
		let shift = 7n;
		let byteLength = 1;
		while ((byte & bit8) > 0) {
			if (byteLength >= maxBytes) {
				throw new Error(`BigInt exceeds maxBytes (${maxBytes})`);
			}
			byte = readUint8(decoder);
			byteLength++;
			value |= BigInt(byte & bits7) << shift;
			shift += 7n;
		}
		return zigZagDecodeBigInt(value);
	}
	function validateMaxBytes(maxBytes) {
		if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) {
			throw new RangeError("maxBytes must be a positive safe integer");
		}
	}
	function writeDate(encoder, value) {
		const time = value.getTime();
		if (Number.isNaN(time)) {
			throw new Error("Cannot encode invalid Date");
		}
		writeFloat64(encoder, time);
	}
	function readDate(decoder) {
		const value = new Date(readFloat64(decoder));
		if (Number.isNaN(value.getTime())) {
			throw new Error("Cannot decode invalid Date");
		}
		return value;
	}
	function writeVarUint8Array(encoder, value) {
		writeVarUint(encoder, value.byteLength);
		writeUint8Array(encoder, value);
	}
	function readVarUint8Array(decoder) {
		return readUint8Array(decoder, readVarUint(decoder));
	}
	function writeVarString(encoder, value) {
		const start = encoder.pos;
		ensureCapacity(encoder, value.length + 8);
		const buffer = encoder.buffer;
		let pos = encoder.pos;
		let byteLength = value.length;
		while (byteLength > bits7) {
			buffer[pos++] = bit8 | (bits7 & byteLength);
			byteLength = Math.floor(byteLength / 128);
		}
		buffer[pos++] = bits7 & byteLength;
		for (let index = 0; index < value.length; index++) {
			const code = value.charCodeAt(index);
			if (code > 127) {
				encoder.pos = start;
				writeVarStringUtf8(encoder, value);
				return;
			}
			buffer[pos++] = code;
		}
		encoder.pos = pos;
	}
	function writeVarStringUtf8(encoder, value) {
		if (value.length < maxStringBufferSize) {
			const written = textEncoder.encodeInto(value, stringBuffer).written;
			ensureCapacity(encoder, written + 8);
			const buffer = encoder.buffer;
			let pos = encoder.pos;
			let byteLength = written;
			while (byteLength > bits7) {
				buffer[pos++] = bit8 | (bits7 & byteLength);
				byteLength = Math.floor(byteLength / 128);
			}
			buffer[pos++] = bits7 & byteLength;
			buffer.set(stringBuffer.subarray(0, written), pos);
			encoder.pos = pos + written;
			return;
		}
		writeVarUint8Array(encoder, textEncoder.encode(value));
	}
	function readVarString(decoder) {
		const byteLength = readVarUint(decoder);
		if (decoder.pos + byteLength > decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		return readStringBytes(decoder, byteLength);
	}
	function readCachedVarString(decoder, cache, index) {
		const byteLength = readVarUint(decoder);
		if (decoder.pos + byteLength > decoder.arr.length) {
			throw new Error("Unexpected end of array");
		}
		const cached = cache[index];
		if (
			cached !== undefined &&
			cached.bytes.byteLength === byteLength &&
			bytesEqual(decoder.arr, decoder.pos, cached.bytes)
		) {
			decoder.pos += byteLength;
			return cached.value;
		}
		const start = decoder.pos;
		const value = readStringBytes(decoder, byteLength);
		cache[index] = {
			bytes: decoder.arr.slice(start, start + byteLength),
			value,
		};
		return value;
	}
	function readStringBytes(decoder, byteLength) {
		if (byteLength <= maxDirectAsciiStringSize) {
			let value = "";
			for (let index = 0; index < byteLength; index++) {
				const byte = decoder.arr[decoder.pos + index];
				if (byte > 127) {
					value = textDecoder.decode(decoder.arr.subarray(decoder.pos, decoder.pos + byteLength));
					decoder.pos += byteLength;
					return value;
				}
				value += String.fromCharCode(byte);
			}
			decoder.pos += byteLength;
			return value;
		}
		const value = textDecoder.decode(decoder.arr.subarray(decoder.pos, decoder.pos + byteLength));
		decoder.pos += byteLength;
		return value;
	}
	function bytesEqual(buffer, pos, bytes) {
		for (let index = 0; index < bytes.byteLength; index++) {
			if (buffer[pos + index] !== bytes[index]) {
				return false;
			}
		}
		return true;
	}
	// source/codec.ts
	function createStreamCodec(write, read) {
		const encoder = createEncoder();
		const decoder = createDecoder(new Uint8Array(0));
		let encoding = false;
		let decoding = false;
		return {
			encode: (value, context) => {
				const destination = encoding ? createEncoder() : encoder;
				const previous = encoding;
				encoding = true;
				destination.pos = 0;
				destination.context = context;
				try {
					write(destination, value);
					return toUint8Array(destination);
				} finally {
					destination.context = undefined;
					encoding = previous;
				}
			},
			encodeInto: (value, target, offset = 0, context) => {
				if (target instanceof Encoder && offset !== 0) {
					throw new RangeError("Use the encoder position instead of an encodeInto offset");
				}
				const destination = target instanceof Encoder ? target : createEncoder(target, offset);
				const start = destination.pos;
				const previous = destination.context;
				if (context !== undefined) destination.context = context;
				try {
					write(destination, value);
					return destination.pos - start;
				} finally {
					destination.context = previous;
				}
			},
			decode: (buffer, context) => {
				const source =
					buffer instanceof Uint8Array ? (decoding ? createDecoder(buffer) : decoder) : buffer;
				const previous = decoding;
				const previousContext = source.context;
				decoding = true;
				if (buffer instanceof Uint8Array) {
					source.arr = buffer;
					source.view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
					source.pos = 0;
					source.context = context;
				} else if (context !== undefined) {
					source.context = context;
				}
				try {
					return read(source);
				} finally {
					source.context = previousContext;
					decoding = previous;
				}
			},
		};
	}
	// integrations/fa.ts
	var faCodec = exports_runtime;
	return faCodec;
})();
