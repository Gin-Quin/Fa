// Generated from @gin-quin/codec; SPDX-License-Identifier: MIT.
// Source: https://github.com/Gin-Quin/codec (local Fa integration).
// Source SHA-256: c464d3434bbfb992d2116280701c4dda494d08ecbd29c8994912b4c741ea9251
// Regenerate: bun scripts/build-fa-runtime.ts <destination>
// @fa-helper faCodec deps:
export const faCodec = (() => {
// source/binary.ts
var bit8 = 128;
var bits7 = 127;
var defaultBufferSize = 128;
var minInt32 = -2147483648;
var maxInt32 = 2147483647;
var maxUint32 = 4294967295;
var maxDirectAsciiStringSize = 64;
var textEncoder = new TextEncoder;
var textDecoder = new TextDecoder;
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
  const value = new Uint8Array(decoder.arr.buffer, decoder.arr.byteOffset + decoder.pos, byteLength);
  decoder.pos += byteLength;
  return value;
}
function writeFloat32(encoder, value) {
  ensureCapacity(encoder, 4);
  if (encoder.view.byteLength < encoder.pos + 4) {
    encoder.view = new DataView(encoder.buffer.buffer, encoder.buffer.byteOffset, encoder.buffer.byteLength);
  }
  encoder.view.setFloat32(encoder.pos, value, true);
  encoder.pos += 4;
}
function readFloat32(decoder) {
  if (decoder.pos + 4 > decoder.arr.length) {
    throw new Error("Unexpected end of array");
  }
  if (decoder.view.byteLength < decoder.pos + 4) {
    decoder.view = new DataView(decoder.arr.buffer, decoder.arr.byteOffset, decoder.arr.byteLength);
  }
  const value = decoder.view.getFloat32(decoder.pos, true);
  decoder.pos += 4;
  return value;
}
function writeFloat64(encoder, value) {
  ensureCapacity(encoder, 8);
  if (encoder.view.byteLength < encoder.pos + 8) {
    encoder.view = new DataView(encoder.buffer.buffer, encoder.buffer.byteOffset, encoder.buffer.byteLength);
  }
  encoder.view.setFloat64(encoder.pos, value, true);
  encoder.pos += 8;
}
function readFloat64(decoder) {
  if (decoder.pos + 8 > decoder.arr.length) {
    throw new Error("Unexpected end of array");
  }
  if (decoder.view.byteLength < decoder.pos + 8) {
    decoder.view = new DataView(decoder.arr.buffer, decoder.arr.byteOffset, decoder.arr.byteLength);
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
      buffer[pos++] = bit8 | remaining & bits7;
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
    buffer[pos++] = bit8 | bits7 & value;
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
    writeVarUint(encoder, (value << 1 ^ value >> 31) >>> 0);
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
  return value >>> 1 ^ -(value & 1) | 0;
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
    buffer[pos++] = bit8 | bits7 & byteLength;
    byteLength = Math.floor(byteLength / 128);
  }
  buffer[pos++] = bits7 & byteLength;
  for (let index = 0;index < value.length; index++) {
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
      buffer[pos++] = bit8 | bits7 & byteLength;
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
function readStringBytes(decoder, byteLength) {
  if (byteLength <= maxDirectAsciiStringSize) {
    let value = "";
    for (let index = 0;index < byteLength; index++) {
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

// source/context.ts
function transformForEncoding(context, name, value) {
  if (!context?.transforms)
    throw new TypeError(`Encoding transform ${name} requires a transform writer`);
  return context.transforms.write(name, value);
}
function transformForDecoding(context, name, value) {
  if (!context?.transforms)
    throw new TypeError(`Decoding transform ${name} requires a transform reader`);
  return context.transforms.read(name, value);
}

// source/schema.ts
function array(element) {
  return { _type: "array", element };
}
function object(fields) {
  return { _type: "object", fields };
}
function map(element) {
  return { _type: "map", element };
}
function mapEntries(key, value) {
  return { _type: "mapEntries", key, value };
}
function transform(name, schema) {
  if (typeof name !== "string" || name.length === 0) {
    throw new TypeError("Transform names must be nonempty strings");
  }
  return { _type: "transform", name, schema };
}
function reference(name) {
  if (typeof name !== "string" || name.length === 0) {
    throw new TypeError("Reference names must be nonempty strings");
  }
  return { _type: "reference", name };
}
function bigint(maxBytes = 128) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) {
    throw new RangeError("bigint maxBytes must be a positive safe integer");
  }
  return { _type: "bigint", maxBytes };
}
function set(element) {
  return { _type: "set", element };
}
function tuple(...elements) {
  return { _type: "tuple", elements };
}
function literal(value) {
  return { _type: "literal", value };
}
function optional(schema) {
  return { _type: "optional", schema };
}
function schemaOf(value, override) {
  return discoverSchema(value, new WeakSet, override);
}
function union(discriminantTypeOrOptionsOrVariants, variants) {
  if (Array.isArray(discriminantTypeOrOptionsOrVariants)) {
    return { _type: "untaggedUnion", variants: discriminantTypeOrOptionsOrVariants };
  }
  if (typeof discriminantTypeOrOptionsOrVariants === "string") {
    return {
      _type: "union",
      tagName: "type",
      type: discriminantTypeOrOptionsOrVariants,
      variants: variants ?? {}
    };
  }
  const options = discriminantTypeOrOptionsOrVariants;
  return {
    _type: "union",
    tagName: options.tagName,
    type: options.tagType,
    variants: options.variants
  };
}
function discoverSchema(value, stack, override) {
  const custom = override?.(value);
  if (custom !== undefined) {
    return custom;
  }
  switch (typeof value) {
    case "undefined":
      return "undefined";
    case "string":
      return "string";
    case "boolean":
      return "boolean";
    case "bigint":
      return bigint(Math.max(128, getVarBigIntByteLength(value)));
    case "number":
      return discoverNumberSchema(value);
    case "function":
    case "symbol":
      throw new TypeError(`Cannot discover schema for ${typeof value} values`);
    case "object":
      if (value === null) {
        return "null";
      }
      return discoverObjectLikeSchema(value, stack, override);
  }
}
function discoverNumberSchema(value) {
  if (!Number.isSafeInteger(value) || Object.is(value, -0)) {
    return "float64";
  }
  return value >= 0 ? "uint" : "int";
}
function discoverObjectLikeSchema(value, stack, override) {
  if (value instanceof Date) {
    return "date";
  }
  if (value instanceof Uint8Array) {
    return "uint8Array";
  }
  if (stack.has(value)) {
    throw new TypeError("Cannot discover schema for cyclic values");
  }
  stack.add(value);
  try {
    if (Array.isArray(value)) {
      return discoverArraySchema(value, stack, override);
    }
    if (value instanceof Set) {
      return discoverSetSchema(value, stack, override);
    }
    if (value instanceof Map) {
      let keySchema;
      let valueSchema;
      for (const [key, element] of value) {
        keySchema = joinSchemas(keySchema, discoverSchema(key, stack, override));
        valueSchema = joinSchemas(valueSchema, discoverSchema(element, stack, override));
      }
      return mapEntries(keySchema ?? "unknown", valueSchema ?? "unknown");
    }
    if (isPlainRecord(value)) {
      return discoverRecordSchema(value, stack, override);
    }
  } finally {
    stack.delete(value);
  }
  throw new TypeError(`Cannot discover schema for ${getObjectTypeName(value)} values`);
}
function discoverArraySchema(value, stack, override) {
  let elementSchema;
  for (let index = 0;index < value.length; index++) {
    elementSchema = joinSchemas(elementSchema, discoverSchema(value[index], stack, override));
  }
  return array(elementSchema ?? "unknown");
}
function discoverSetSchema(value, stack, override) {
  let elementSchema;
  for (const element of value) {
    elementSchema = joinSchemas(elementSchema, discoverSchema(element, stack, override));
  }
  return set(elementSchema ?? "unknown");
}
function discoverRecordSchema(value, stack, override) {
  if (Object.getOwnPropertySymbols(value).some((key) => Object.prototype.propertyIsEnumerable.call(value, key))) {
    throw new TypeError("Cannot discover schema for objects with enumerable symbol keys");
  }
  const fields = {};
  const keys = Object.keys(value);
  for (let index = 0;index < keys.length; index++) {
    const key = keys[index];
    setOwnProperty(fields, key, discoverSchema(value[key], stack, override));
  }
  return object(fields);
}
function joinSchemas(left, right) {
  if (!left) {
    return right;
  }
  if (schemasEqual(left, right)) {
    return left;
  }
  if (left === "unknown" || right === "unknown") {
    return "unknown";
  }
  if (left === "undefined") {
    return optional(right);
  }
  if (right === "undefined") {
    return optional(left);
  }
  if (isOptionalSchemaValue(left)) {
    return optional(joinSchemas(left.schema, isOptionalSchemaValue(right) ? right.schema : right));
  }
  if (isOptionalSchemaValue(right)) {
    return optional(joinSchemas(left, right.schema));
  }
  if (typeof left === "function" || typeof right === "function") {
    return schemasEqual(left, right) ? left : "unknown";
  }
  if (typeof left === "string" || typeof right === "string") {
    return joinPrimitiveSchemas(left, right);
  }
  if (left._type !== right._type) {
    return "unknown";
  }
  switch (left._type) {
    case "literal":
      return literalValuesEqual(left.value, right.value) ? left : "unknown";
    case "array":
      return array(joinSchemas(left.element, right.element));
    case "object":
      return joinObjectSchemas(left, right);
    case "map":
      return map(joinSchemas(left.element, right.element));
    case "mapEntries": {
      const other = right;
      return mapEntries(joinSchemas(left.key, other.key), joinSchemas(left.value, other.value));
    }
    case "transform": {
      const other = right;
      return left.name === other.name ? transform(left.name, joinSchemas(left.schema, other.schema)) : "unknown";
    }
    case "reference":
      return left.name === right.name ? left : "unknown";
    case "bigint":
      return bigint(Math.max(left.maxBytes, right.maxBytes));
    case "set":
      return set(joinSchemas(left.element, right.element));
    case "tuple":
      return joinTupleSchemas(left, right);
    case "union":
    case "untaggedUnion":
      return schemasEqual(left, right) ? left : "unknown";
  }
}
function joinPrimitiveSchemas(left, right) {
  if (left === right) {
    return left;
  }
  if (isIntegerNumberSchema(left) && isIntegerNumberSchema(right)) {
    return "int";
  }
  if (isNumberSchema(left) && isNumberSchema(right)) {
    return "float64";
  }
  return "unknown";
}
function joinObjectSchemas(left, right) {
  const fields = {};
  const leftKeys = Object.keys(left.fields);
  for (let index = 0;index < leftKeys.length; index++) {
    const key = leftKeys[index];
    const leftSchema = left.fields[key];
    setOwnProperty(fields, key, Object.hasOwn(right.fields, key) ? joinSchemas(leftSchema, right.fields[key]) : optional(leftSchema));
  }
  const rightKeys = Object.keys(right.fields);
  for (let index = 0;index < rightKeys.length; index++) {
    const key = rightKeys[index];
    if (!Object.hasOwn(left.fields, key)) {
      setOwnProperty(fields, key, optional(right.fields[key]));
    }
  }
  return object(fields);
}
function joinTupleSchemas(left, right) {
  if (left.elements.length !== right.elements.length) {
    return "unknown";
  }
  const elements = new Array(left.elements.length);
  for (let index = 0;index < left.elements.length; index++) {
    elements[index] = joinSchemas(left.elements[index], right.elements[index]);
  }
  return { _type: "tuple", elements };
}
function isIntegerNumberSchema(schema) {
  return schema === "int" || schema === "uint" || schema === "int32" || schema === "uint32" || schema === "int64" || schema === "uint64";
}
function isNumberSchema(schema) {
  return isIntegerNumberSchema(schema) || schema === "float32" || schema === "float64";
}
function isOptionalSchemaValue(schema) {
  return typeof schema === "object" && schema !== null && schema._type === "optional";
}
function isPlainRecord(value) {
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}
function getObjectTypeName(value) {
  return Object.prototype.toString.call(value).slice(8, -1);
}
function getVarBigIntByteLength(value) {
  if (value < 0n) {
    value = -value;
  }
  let byteLength = 1;
  value >>= 6n;
  while (value > 0n) {
    byteLength++;
    value >>= 7n;
  }
  return byteLength;
}
function isSchema(value) {
  return isConcreteSchema(value, new WeakSet);
}
function isConcreteSchema(value, stack) {
  if (typeof value === "string") {
    return isPrimitiveType(value);
  }
  if (typeof value !== "object" || value === null || stack.has(value)) {
    return false;
  }
  stack.add(value);
  try {
    const candidate = value;
    switch (candidate._type) {
      case "literal":
        return true;
      case "array":
        return isConcreteSchema(candidate.element, stack);
      case "object":
        return isSchemaRecord(candidate.fields, stack);
      case "optional":
        return isConcreteSchema(candidate.schema, stack);
      case "union":
        return isUnionSchemaValue(candidate, stack);
      case "untaggedUnion": {
        const variants = candidate.variants;
        return Array.isArray(variants) && variants.every((variant) => isConcreteSchema(variant, stack));
      }
      case "map":
        return isConcreteSchema(candidate.element, stack);
      case "mapEntries": {
        const entry = candidate;
        return isConcreteSchema(entry.key, stack) && isConcreteSchema(entry.value, stack);
      }
      case "transform": {
        const entry = candidate;
        return typeof entry.name === "string" && entry.name.length > 0 && isConcreteSchema(entry.schema, stack);
      }
      case "reference": {
        const name = candidate.name;
        return typeof name === "string" && name.length > 0;
      }
      case "bigint": {
        const maxBytes = candidate.maxBytes;
        return Number.isSafeInteger(maxBytes) && maxBytes >= 1;
      }
      case "set":
        return isConcreteSchema(candidate.element, stack);
      case "tuple": {
        const elements = candidate.elements;
        return Array.isArray(elements) && elements.every((element) => isConcreteSchema(element, stack));
      }
      default:
        return false;
    }
  } finally {
    stack.delete(value);
  }
}
function isPrimitiveType(value) {
  switch (value) {
    case "string":
    case "int":
    case "int32":
    case "int64":
    case "uint":
    case "uint32":
    case "uint64":
    case "uint8Array":
    case "boolean":
    case "date":
    case "float32":
    case "float64":
    case "unknown":
    case "schema":
    case "null":
    case "undefined":
      return true;
  }
  return false;
}
function isSchemaRecord(value, stack) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const entries = Object.values(value);
  for (let index = 0;index < entries.length; index++) {
    if (!isConcreteSchema(entries[index], stack)) {
      return false;
    }
  }
  return true;
}
function isUnionSchemaValue(value, stack) {
  const schema = value;
  if (typeof schema.tagName !== "string" || !isUnionDiscriminantType(schema.type) || !schema.variants || typeof schema.variants !== "object" || Array.isArray(schema.variants)) {
    return false;
  }
  const variantSchemas = Object.values(schema.variants);
  for (let index = 0;index < variantSchemas.length; index++) {
    if (!isSchemaRecord(variantSchemas[index], stack)) {
      return false;
    }
  }
  return true;
}
function isUnionDiscriminantType(value) {
  return value === "string" || value === "int" || value === "uint" || value === "int32" || value === "uint32" || value === "int64" || value === "uint64";
}
function schemasEqual(left, right) {
  if (left === right) {
    return true;
  }
  if (typeof left === "function" || typeof right === "function") {
    return false;
  }
  if (typeof left === "string" || typeof right === "string" || left._type !== right._type) {
    return false;
  }
  switch (left._type) {
    case "literal":
      return literalValuesEqual(left.value, right.value);
    case "array":
      return schemasEqual(left.element, right.element);
    case "object":
      return schemaRecordsEqual(left.fields, right.fields);
    case "optional":
      return schemasEqual(left.schema, right.schema);
    case "union":
      return unionSchemasEqual(left, right);
    case "untaggedUnion": {
      const rightVariants = right.variants;
      if (left.variants.length !== rightVariants.length) {
        return false;
      }
      for (let index = 0;index < left.variants.length; index++) {
        if (!schemasEqual(left.variants[index], rightVariants[index])) {
          return false;
        }
      }
      return true;
    }
    case "map":
      return schemasEqual(left.element, right.element);
    case "mapEntries": {
      const other = right;
      return schemasEqual(left.key, other.key) && schemasEqual(left.value, other.value);
    }
    case "transform": {
      const other = right;
      return left.name === other.name && schemasEqual(left.schema, other.schema);
    }
    case "reference":
      return left.name === right.name;
    case "bigint":
      return left.maxBytes === right.maxBytes;
    case "set":
      return schemasEqual(left.element, right.element);
    case "tuple": {
      const rightElements = right.elements;
      if (left.elements.length !== rightElements.length) {
        return false;
      }
      for (let index = 0;index < left.elements.length; index++) {
        if (!schemasEqual(left.elements[index], rightElements[index])) {
          return false;
        }
      }
      return true;
    }
  }
}
function setOwnProperty(target, key, value) {
  if (key === "__proto__") {
    Object.defineProperty(target, key, {
      value,
      enumerable: true,
      configurable: true,
      writable: true
    });
  } else {
    target[key] = value;
  }
}
function literalValuesEqual(left, right) {
  return literalValuesEqualInner(left, right, new WeakMap);
}
function literalValuesEqualInner(left, right, seen) {
  if (Object.is(left, right)) {
    return true;
  }
  if (left === null || right === null || typeof left !== "object" || typeof right !== "object") {
    return false;
  }
  if (hasSeenPair(seen, left, right)) {
    return true;
  }
  if (left instanceof Date || right instanceof Date) {
    return left instanceof Date && right instanceof Date && left.getTime() === right.getTime();
  }
  if (left instanceof Uint8Array || right instanceof Uint8Array) {
    return left instanceof Uint8Array && right instanceof Uint8Array && bytesEqual(left, right);
  }
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
      return false;
    }
    for (let index = 0;index < left.length; index++) {
      if (!literalValuesEqualInner(left[index], right[index], seen)) {
        return false;
      }
    }
    return true;
  }
  if (left instanceof Set || right instanceof Set) {
    return left instanceof Set && right instanceof Set && setsEqual(left, right, seen);
  }
  if (left instanceof Map || right instanceof Map) {
    return left instanceof Map && right instanceof Map && mapsEqual(left, right, seen);
  }
  if (Object.getPrototypeOf(left) !== Object.getPrototypeOf(right)) {
    return false;
  }
  const leftKeys = getEnumerableOwnKeys(left);
  const rightKeys = getEnumerableOwnKeys(right);
  if (leftKeys.length !== rightKeys.length) {
    return false;
  }
  for (const key of leftKeys) {
    if (!Object.hasOwn(right, key) || !literalValuesEqualInner(left[key], right[key], seen)) {
      return false;
    }
  }
  return true;
}
function getEnumerableOwnKeys(value) {
  const symbolKeys = Object.getOwnPropertySymbols(value).filter((key) => Object.prototype.propertyIsEnumerable.call(value, key));
  return [...Object.keys(value), ...symbolKeys];
}
function hasSeenPair(seen, left, right) {
  const existing = seen.get(left);
  if (existing?.has(right)) {
    return true;
  }
  if (existing) {
    existing.add(right);
  } else {
    seen.set(left, new WeakSet([right]));
  }
  return false;
}
function bytesEqual(left, right) {
  if (left.byteLength !== right.byteLength) {
    return false;
  }
  for (let index = 0;index < left.byteLength; index++) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
}
function setsEqual(left, right, seen) {
  if (left.size !== right.size) {
    return false;
  }
  const unmatched = [...right];
  for (const leftValue of left) {
    const index = unmatched.findIndex((rightValue) => literalValuesEqualInner(leftValue, rightValue, seen));
    if (index === -1) {
      return false;
    }
    unmatched.splice(index, 1);
  }
  return true;
}
function mapsEqual(left, right, seen) {
  if (left.size !== right.size) {
    return false;
  }
  const unmatched = [...right];
  for (const [leftKey, leftValue] of left) {
    const index = unmatched.findIndex(([rightKey, rightValue]) => literalValuesEqualInner(leftKey, rightKey, seen) && literalValuesEqualInner(leftValue, rightValue, seen));
    if (index === -1) {
      return false;
    }
    unmatched.splice(index, 1);
  }
  return true;
}
function schemaRecordsEqual(left, right) {
  const leftKeys = Object.keys(left);
  const rightKeys = Object.keys(right);
  if (leftKeys.length !== rightKeys.length) {
    return false;
  }
  for (const key of leftKeys) {
    if (!Object.hasOwn(right, key) || !schemasEqual(left[key], right[key])) {
      return false;
    }
  }
  return true;
}
function unionSchemasEqual(left, right) {
  return left.tagName === right.tagName && left.type === right.type && unionVariantMapsEqual(left.variants, right.variants);
}
function unionVariantMapsEqual(left, right) {
  const leftKeys = Object.keys(left);
  const rightKeys = Object.keys(right);
  if (leftKeys.length !== rightKeys.length) {
    return false;
  }
  for (const key of leftKeys) {
    if (!Object.hasOwn(right, key) || !schemaRecordsEqual(left[key], right[key])) {
      return false;
    }
  }
  return true;
}

// source/dynamic.ts
function writeSchema(schema, value, encoder) {
  if (typeof schema === "function") {
    schema = schema();
  }
  if (typeof schema === "string") {
    switch (schema) {
      case "string":
        writeVarString(encoder, value);
        break;
      case "int":
      case "int32":
      case "int64":
        writeVarInt(encoder, value);
        break;
      case "uint":
      case "uint32":
      case "uint64":
        writeVarUint(encoder, value);
        break;
      case "uint8Array":
        writeVarUint8Array(encoder, value);
        break;
      case "boolean":
        writeUint8(encoder, value ? 1 : 0);
        break;
      case "date":
        writeDate(encoder, value);
        break;
      case "float32":
        writeFloat32(encoder, value);
        break;
      case "float64":
        writeFloat64(encoder, value);
        break;
      case "unknown":
        writeUnknown(value, encoder);
        break;
      case "schema":
        writeSchemaValue(value, encoder);
        break;
      case "null":
      case "undefined":
        break;
    }
  } else {
    switch (schema._type) {
      case "literal":
        writeLiteral(schema, value);
        break;
      case "array":
        writeArray(schema, value, encoder);
        break;
      case "object":
        writeObject(schema, value, encoder);
        break;
      case "optional":
        writeOptional(schema, value, encoder);
        break;
      case "union":
        writeUnion(schema, value, encoder);
        break;
      case "untaggedUnion":
        writeUntaggedUnion(schema, value, encoder);
        break;
      case "map":
        writeMap(schema, value, encoder);
        break;
      case "mapEntries":
        writeMapEntries(schema, value, encoder);
        break;
      case "transform":
        writeSchema(schema.schema, transformForEncoding(encoder.context, schema.name, value), encoder);
        break;
      case "reference":
        writeReference(encoder, schema.name, value);
        break;
      case "bigint":
        writeBigInt(schema, value, encoder);
        break;
      case "set":
        writeSet(schema, value, encoder);
        break;
      case "tuple":
        writeTuple(schema, value, encoder);
        break;
    }
  }
}
function writeLiteral(schema, value) {
  if (!literalValuesEqual(value, schema.value)) {
    throw new Error("Value does not match literal schema");
  }
}
function writeArray(schema, value, encoder) {
  writeVarUint(encoder, value.length);
  for (let index = 0;index < value.length; index++) {
    writeSchema(schema.element, value[index], encoder);
  }
}
function writeObject(schema, value, encoder) {
  for (const [key, fieldSchema] of Object.entries(schema.fields)) {
    writeSchema(fieldSchema, value[key], encoder);
  }
}
function writeOptional(schema, value, encoder) {
  if (value === undefined) {
    writeUint8(encoder, 0);
  } else {
    writeUint8(encoder, 1);
    writeSchema(schema.schema, value, encoder);
  }
}
function writeUnion(schema, value, encoder) {
  const discriminantValue = value[schema.tagName];
  writeSchema(schema.type, discriminantValue, encoder);
  const variantSchema = schema.variants[discriminantValue];
  if (!Object.hasOwn(schema.variants, discriminantValue) || !variantSchema) {
    throw new Error(`Unknown union variant: ${discriminantValue}`);
  }
  for (const [key, fieldSchema] of Object.entries(variantSchema)) {
    if (key !== schema.tagName) {
      writeSchema(fieldSchema, value[key], encoder);
    }
  }
}
function writeUntaggedUnion(schema, value, encoder) {
  const variantIndex = findUntaggedUnionVariant(schema.variants, value, encoder.context);
  if (variantIndex === -1) {
    throw new Error("No matching union variant");
  }
  writeVarUint(encoder, variantIndex);
  writeSchema(schema.variants[variantIndex], value, encoder);
}
function writeMap(schema, value, encoder) {
  const entries = Object.entries(value);
  writeVarUint(encoder, entries.length);
  for (let index = 0;index < entries.length; index++) {
    const entry = entries[index];
    writeVarString(encoder, entry[0]);
    writeSchema(schema.element, entry[1], encoder);
  }
}
function writeMapEntries(schema, value, encoder) {
  writeVarUint(encoder, value.size);
  for (const [key, element] of value) {
    writeSchema(schema.key, key, encoder);
    writeSchema(schema.value, element, encoder);
  }
}
function writeBigInt(schema, value, encoder) {
  writeVarBigInt(encoder, value, schema.maxBytes);
}
function writeSet(schema, value, encoder) {
  writeVarUint(encoder, value.size);
  for (const element of value) {
    writeSchema(schema.element, element, encoder);
  }
}
function writeTuple(schema, value, encoder) {
  for (let index = 0;index < schema.elements.length; index++) {
    writeSchema(schema.elements[index], value[index], encoder);
  }
}
function writeSchemaValue(schema, encoder) {
  if (typeof schema === "function") {
    throw new Error("Cannot encode lazy schema functions");
  }
  if (typeof schema === "string") {
    writePrimitiveSchemaValue(schema, encoder);
    return;
  }
  if (typeof schema !== "object" || schema === null) {
    throw new Error("Cannot encode invalid schema");
  }
  switch (schema._type) {
    case "literal":
      writeLiteralSchemaValue(schema, encoder);
      break;
    case "array":
      writeUint8(encoder, 9 /* Array */);
      writeSchemaValue(schema.element, encoder);
      break;
    case "object":
      writeUint8(encoder, 10 /* Object */);
      writeSchemaFields(schema.fields, encoder);
      break;
    case "optional":
      writeUint8(encoder, 11 /* Optional */);
      writeSchemaValue(schema.schema, encoder);
      break;
    case "union": {
      writeUint8(encoder, 12 /* Union */);
      writeVarString(encoder, schema.tagName);
      writeSchemaValue(schema.type, encoder);
      const variantKeys = Object.keys(schema.variants);
      writeVarUint(encoder, variantKeys.length);
      for (let index = 0;index < variantKeys.length; index++) {
        const variant = variantKeys[index];
        writeVarString(encoder, variant);
        writeSchemaFields(schema.variants[variant], encoder);
      }
      break;
    }
    case "untaggedUnion":
      writeUint8(encoder, 13 /* UntaggedUnion */);
      writeVarUint(encoder, schema.variants.length);
      for (const variant of schema.variants) {
        writeSchemaValue(variant, encoder);
      }
      break;
    case "map":
      writeUint8(encoder, 14 /* Map */);
      writeSchemaValue(schema.element, encoder);
      break;
    case "reference":
      writeUint8(encoder, 28 /* Reference */);
      writeVarString(encoder, schema.name);
      break;
    case "transform":
      writeUint8(encoder, 29 /* Transform */);
      writeVarString(encoder, schema.name);
      writeSchemaValue(schema.schema, encoder);
      break;
    case "mapEntries":
      writeUint8(encoder, 30 /* MapEntries */);
      writeSchemaValue(schema.key, encoder);
      writeSchemaValue(schema.value, encoder);
      break;
    case "bigint":
      writeUint8(encoder, 8 /* BigInt */);
      writeVarUint(encoder, schema.maxBytes);
      break;
    case "set":
      writeUint8(encoder, 15 /* Set */);
      writeSchemaValue(schema.element, encoder);
      break;
    case "tuple":
      writeUint8(encoder, 16 /* Tuple */);
      writeVarUint(encoder, schema.elements.length);
      for (const element of schema.elements) {
        writeSchemaValue(element, encoder);
      }
      break;
    default:
      throw new Error(`Unknown schema type: ${String(schema._type)}`);
  }
}
function writePrimitiveSchemaValue(schema, encoder) {
  switch (schema) {
    case "string":
      writeUint8(encoder, 0 /* String */);
      break;
    case "int":
      writeUint8(encoder, 1 /* Int */);
      break;
    case "int32":
      writeUint8(encoder, 24 /* Int32 */);
      break;
    case "int64":
      writeUint8(encoder, 26 /* Int64 */);
      break;
    case "uint":
      writeUint8(encoder, 2 /* Uint */);
      break;
    case "uint32":
      writeUint8(encoder, 25 /* Uint32 */);
      break;
    case "uint64":
      writeUint8(encoder, 27 /* Uint64 */);
      break;
    case "uint8Array":
      writeUint8(encoder, 3 /* Uint8Array */);
      break;
    case "boolean":
      writeUint8(encoder, 4 /* Boolean */);
      break;
    case "date":
      writeUint8(encoder, 5 /* Date */);
      break;
    case "float32":
      writeUint8(encoder, 6 /* Float32 */);
      break;
    case "float64":
      writeUint8(encoder, 7 /* Float64 */);
      break;
    case "unknown":
      writeUint8(encoder, 17 /* Unknown */);
      break;
    case "schema":
      writeUint8(encoder, 18 /* Schema */);
      break;
    case "null":
      writeUint8(encoder, 19 /* Null */);
      break;
    case "undefined":
      writeUint8(encoder, 20 /* Undefined */);
      break;
    default:
      throw new Error(`Unknown primitive schema: ${String(schema)}`);
  }
}
function writeLiteralSchemaValue(schema, encoder) {
  if (schema.value === null) {
    writeUint8(encoder, 22 /* LiteralNull */);
    return;
  }
  if (schema.value === undefined) {
    writeUint8(encoder, 23 /* LiteralUndefined */);
    return;
  }
  writeUint8(encoder, 21 /* Literal */);
  writeUnknown(schema.value, encoder);
}
function writeSchemaFields(fields, encoder) {
  const keys = Object.keys(fields);
  writeVarUint(encoder, keys.length);
  for (let index = 0;index < keys.length; index++) {
    const key = keys[index];
    writeVarString(encoder, key);
    writeSchemaValue(fields[key], encoder);
  }
}
function readSchemaValue(decoder) {
  const tag = readUint8(decoder);
  switch (tag) {
    case 0 /* String */:
      return "string";
    case 1 /* Int */:
      return "int";
    case 24 /* Int32 */:
      return "int32";
    case 26 /* Int64 */:
      return "int64";
    case 2 /* Uint */:
      return "uint";
    case 25 /* Uint32 */:
      return "uint32";
    case 27 /* Uint64 */:
      return "uint64";
    case 3 /* Uint8Array */:
      return "uint8Array";
    case 4 /* Boolean */:
      return "boolean";
    case 5 /* Date */:
      return "date";
    case 6 /* Float32 */:
      return "float32";
    case 7 /* Float64 */:
      return "float64";
    case 8 /* BigInt */:
      return bigint(readVarUint(decoder));
    case 9 /* Array */:
      return { _type: "array", element: readSchemaValue(decoder) };
    case 10 /* Object */:
      return { _type: "object", fields: readSchemaFields(decoder) };
    case 11 /* Optional */:
      return { _type: "optional", schema: readSchemaValue(decoder) };
    case 12 /* Union */:
      return readUnionSchemaValue(decoder);
    case 13 /* UntaggedUnion */:
      return readUntaggedUnionSchemaValue(decoder);
    case 14 /* Map */:
      return { _type: "map", element: readSchemaValue(decoder) };
    case 28 /* Reference */:
      return reference(readVarString(decoder));
    case 29 /* Transform */:
      return transform(readVarString(decoder), readSchemaValue(decoder));
    case 30 /* MapEntries */:
      return {
        _type: "mapEntries",
        key: readSchemaValue(decoder),
        value: readSchemaValue(decoder)
      };
    case 15 /* Set */:
      return { _type: "set", element: readSchemaValue(decoder) };
    case 16 /* Tuple */:
      return readTupleSchemaValue(decoder);
    case 17 /* Unknown */:
      return "unknown";
    case 18 /* Schema */:
      return "schema";
    case 19 /* Null */:
      return "null";
    case 20 /* Undefined */:
      return "undefined";
    case 21 /* Literal */:
      return literal(readUnknown(decoder));
    case 22 /* LiteralNull */:
      return literal(null);
    case 23 /* LiteralUndefined */:
      return literal(undefined);
    default:
      throw new Error(`Unknown schema tag: ${tag}`);
  }
}
function readSchemaFields(decoder) {
  const length = readVarUint(decoder);
  const fields = {};
  for (let index = 0;index < length; index++) {
    setOwnProperty(fields, readVarString(decoder), readSchemaValue(decoder));
  }
  return fields;
}
function readUnionSchemaValue(decoder) {
  const tagName = readVarString(decoder);
  const type = readUnionDiscriminantSchemaValue(decoder);
  const length = readVarUint(decoder);
  const variants = {};
  for (let index = 0;index < length; index++) {
    setOwnProperty(variants, readVarString(decoder), readSchemaFields(decoder));
  }
  return { _type: "union", tagName, type, variants };
}
function readUnionDiscriminantSchemaValue(decoder) {
  const schema = readSchemaValue(decoder);
  if (schema === "string" || schema === "int" || schema === "uint" || schema === "int32" || schema === "uint32" || schema === "int64" || schema === "uint64") {
    return schema;
  }
  throw new Error("Invalid union discriminant schema");
}
function readUntaggedUnionSchemaValue(decoder) {
  const length = readVarUint(decoder);
  const variants = new Array(length);
  for (let index = 0;index < length; index++) {
    variants[index] = readSchemaValue(decoder);
  }
  return { _type: "untaggedUnion", variants };
}
function readTupleSchemaValue(decoder) {
  const length = readVarUint(decoder);
  const elements = new Array(length);
  for (let index = 0;index < length; index++) {
    elements[index] = readSchemaValue(decoder);
  }
  return { _type: "tuple", elements };
}
function writeUnknown(value, encoder) {
  const discoveredSchema = schemaOf(value, encoder.context?.schemaOf);
  writeSchemaValue(discoveredSchema, encoder);
  writeSchema(discoveredSchema, value, encoder);
}
function readUnknown(decoder) {
  return readSchema(readSchemaValue(decoder), decoder);
}
function readSchema(schema, decoder) {
  if (typeof schema === "function") {
    schema = schema();
  }
  if (typeof schema === "string") {
    return readPrimitive(schema, decoder);
  }
  switch (schema._type) {
    case "literal":
      return schema.value;
    case "array":
      return readArray(schema, decoder);
    case "object":
      return readObject(schema, decoder);
    case "optional":
      return readOptional(schema, decoder);
    case "union":
      return readUnion(schema, decoder);
    case "untaggedUnion":
      return readUntaggedUnion(schema, decoder);
    case "map":
      return readMap(schema, decoder);
    case "reference":
      return readReference(decoder, schema.name);
    case "transform":
      return transformForDecoding(decoder.context, schema.name, readSchema(schema.schema, decoder));
    case "mapEntries": {
      const length = readVarUint(decoder);
      const result = new Map;
      for (let index = 0;index < length; index++) {
        result.set(readSchema(schema.key, decoder), readSchema(schema.value, decoder));
      }
      return result;
    }
    case "bigint":
      return readBigInt(schema, decoder);
    case "set":
      return readSet(schema, decoder);
    case "tuple":
      return readTuple(schema, decoder);
  }
}
function readPrimitive(type, decoder) {
  switch (type) {
    case "string":
      return readVarString(decoder);
    case "int":
    case "int32":
    case "int64":
      return readVarInt(decoder);
    case "uint":
    case "uint32":
    case "uint64":
      return readVarUint(decoder);
    case "uint8Array":
      return readVarUint8Array(decoder);
    case "boolean":
      return readUint8(decoder) === 1;
    case "date":
      return readDate(decoder);
    case "float32":
      return readFloat32(decoder);
    case "float64":
      return readFloat64(decoder);
    case "unknown":
      return readUnknown(decoder);
    case "schema":
      return readSchemaValue(decoder);
    case "null":
      return null;
    case "undefined":
      return;
  }
}
function readArray(schema, decoder) {
  const length = readVarUint(decoder);
  const result = new Array(length);
  for (let index = 0;index < length; index++) {
    result[index] = readSchema(schema.element, decoder);
  }
  return result;
}
function readObject(schema, decoder) {
  const result = {};
  for (const [key, fieldSchema] of Object.entries(schema.fields)) {
    setOwnProperty(result, key, readSchema(fieldSchema, decoder));
  }
  return result;
}
function readOptional(schema, decoder) {
  if (readUint8(decoder) === 0) {
    return;
  }
  return readSchema(schema.schema, decoder);
}
function readUnion(schema, decoder) {
  const discriminantValue = readPrimitive(schema.type, decoder);
  const variantSchema = schema.variants[discriminantValue];
  if (!Object.hasOwn(schema.variants, discriminantValue) || !variantSchema) {
    throw new Error(`Unknown union variant: ${discriminantValue}`);
  }
  const result = {
    [schema.tagName]: discriminantValue
  };
  for (const [key, fieldSchema] of Object.entries(variantSchema)) {
    if (key !== schema.tagName) {
      setOwnProperty(result, key, readSchema(fieldSchema, decoder));
    }
  }
  return result;
}
function readUntaggedUnion(schema, decoder) {
  const variantIndex = readVarUint(decoder);
  const variantSchema = schema.variants[variantIndex];
  if (!variantSchema) {
    throw new Error(`Unknown union variant: ${variantIndex}`);
  }
  return readSchema(variantSchema, decoder);
}
function readMap(schema, decoder) {
  const length = readVarUint(decoder);
  const result = {};
  for (let index = 0;index < length; index++) {
    const key = readVarString(decoder);
    setOwnProperty(result, key, readSchema(schema.element, decoder));
  }
  return result;
}
function readBigInt(schema, decoder) {
  return readVarBigInt(decoder, schema.maxBytes);
}
function readSet(schema, decoder) {
  const length = readVarUint(decoder);
  const result = new Set;
  for (let index = 0;index < length; index++) {
    result.add(readSchema(schema.element, decoder));
  }
  return result;
}
function readTuple(schema, decoder) {
  const result = new Array(schema.elements.length);
  for (let index = 0;index < schema.elements.length; index++) {
    result[index] = readSchema(schema.elements[index], decoder);
  }
  return result;
}
function findUntaggedUnionVariant(variants, value, context) {
  for (let index = 0;index < variants.length; index++) {
    if (matchesSchema(variants[index], value, context)) {
      return index;
    }
  }
  return -1;
}
function matchesSchema(schema, value, context) {
  if (typeof schema === "function") {
    return matchesSchema(schema(), value, context);
  }
  if (typeof schema === "string") {
    return matchesPrimitive(schema, value);
  }
  switch (schema._type) {
    case "literal":
      return literalValuesEqual(value, schema.value);
    case "array":
      if (!Array.isArray(value)) {
        return false;
      }
      for (const element of value) {
        if (!matchesSchema(schema.element, element, context)) {
          return false;
        }
      }
      return true;
    case "object":
      if (!isRecordValue(value)) {
        return false;
      }
      for (const [key, fieldSchema] of Object.entries(schema.fields)) {
        if (!matchesSchema(fieldSchema, value[key], context)) {
          return false;
        }
      }
      return true;
    case "optional":
      return value === undefined || matchesSchema(schema.schema, value, context);
    case "union":
      return matchesTaggedUnionSchema(schema, value, context);
    case "untaggedUnion":
      return findUntaggedUnionVariant(schema.variants, value, context) !== -1;
    case "map":
      if (!isRecordValue(value)) {
        return false;
      }
      for (const element of Object.values(value)) {
        if (!matchesSchema(schema.element, element, context)) {
          return false;
        }
      }
      return true;
    case "reference": {
      const references = context?.references;
      if (!references?.is) {
        throw new TypeError("Matching reference variants requires references.is");
      }
      return references.is(schema.name, value);
    }
    case "transform": {
      const transforms = context?.transforms;
      if (!transforms?.is)
        throw new TypeError("Matching transform variants requires transforms.is");
      return transforms.is(schema.name, value);
    }
    case "mapEntries":
      if (!(value instanceof Map))
        return false;
      for (const [key, element] of value) {
        if (!matchesSchema(schema.key, key, context) || !matchesSchema(schema.value, element, context)) {
          return false;
        }
      }
      return true;
    case "bigint":
      return typeof value === "bigint";
    case "set":
      if (!(value instanceof Set)) {
        return false;
      }
      for (const element of value) {
        if (!matchesSchema(schema.element, element, context)) {
          return false;
        }
      }
      return true;
    case "tuple":
      if (!Array.isArray(value) || value.length !== schema.elements.length) {
        return false;
      }
      for (let index = 0;index < schema.elements.length; index++) {
        if (!matchesSchema(schema.elements[index], value[index], context)) {
          return false;
        }
      }
      return true;
  }
}
function matchesPrimitive(schema, value) {
  switch (schema) {
    case "string":
      return typeof value === "string";
    case "int":
    case "int32":
    case "int64":
      return Number.isSafeInteger(value);
    case "uint":
    case "uint32":
    case "uint64":
      return Number.isSafeInteger(value) && value >= 0;
    case "uint8Array":
      return value instanceof Uint8Array;
    case "boolean":
      return typeof value === "boolean";
    case "date":
      return value instanceof Date;
    case "float32":
    case "float64":
      return typeof value === "number";
    case "unknown":
      return true;
    case "schema":
      return isSchema(value);
    case "null":
      return value === null;
    case "undefined":
      return value === undefined;
  }
}
function matchesTaggedUnionSchema(schema, value, context) {
  if (!isRecordValue(value)) {
    return false;
  }
  const variantSchema = schema.variants[value[schema.tagName]];
  if (!Object.hasOwn(schema.variants, value[schema.tagName]) || !variantSchema) {
    return false;
  }
  for (const [key, fieldSchema] of Object.entries(variantSchema)) {
    if (key !== schema.tagName && !matchesSchema(fieldSchema, value[key], context)) {
      return false;
    }
  }
  return true;
}
function isRecordValue(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value) && !(value instanceof Date) && !(value instanceof Set) && !(value instanceof Uint8Array);
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
      if (context !== undefined)
        destination.context = context;
      try {
        write(destination, value);
        return destination.pos - start;
      } finally {
        destination.context = previous;
      }
    },
    decode: (buffer, context) => {
      const source = buffer instanceof Uint8Array ? decoding ? createDecoder(buffer) : decoder : buffer;
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
    }
  };
}

// source/dynamic-codec.ts
function createDynamicCodec(schema) {
  return createStreamCodec((encoder, value) => writeSchema(schema, value, encoder), (decoder) => readSchema(schema, decoder));
}

// integrations/fa.ts
var faCodec = {
  createCodec: createDynamicCodec,
  createDecoder,
  createEncoder,
  toUint8Array,
  readSchema,
  readSchemaValue,
  writeSchema,
  writeSchemaValue,
  array,
  bigint,
  isSchema,
  literal,
  map,
  mapEntries,
  object,
  optional,
  reference,
  schemaOf,
  set,
  transform,
  tuple,
  union
};
return faCodec;
})();
