// Handwritten xorshift64 representation study; not a compiler implementation.
// Matches benchmarks/sum/main.fa, including its unsigned remainder by 1000.
import assert from "node:assert/strict";

function xorshiftBigInt(value) {
	value ^= BigInt.asUintN(64, value << 13n);
	value ^= value >> 7n;
	return value ^ BigInt.asUintN(64, value << 17n);
}

function xorshiftWords(lo, hi) {
	hi = (hi ^ ((hi << 13) | (lo >>> 19))) >>> 0;
	lo = (lo ^ (lo << 13)) >>> 0;
	lo = (lo ^ ((lo >>> 7) | (hi << 25))) >>> 0;
	hi = (hi ^ (hi >>> 7)) >>> 0;
	hi = (hi ^ ((hi << 17) | (lo >>> 15))) >>> 0;
	lo = (lo ^ (lo << 17)) >>> 0;
	return { lo, hi };
}

function bigint(count) {
	let value = 12345n;
	let checksum = 0;
	for (let i = 0; i < count; i++) {
		value = xorshiftBigInt(value);
		checksum += Number(value % 1000n);
	}
	return { lo: Number(value & 0xffffffffn), hi: Number(value >> 32n), checksum };
}

function words(count) {
	let lo = 12345;
	let hi = 0;
	let checksum = 0;
	for (let i = 0; i < count; i++) {
		({ lo, hi } = xorshiftWords(lo, hi));
		// Each intermediate is an exact integer below 2^53.
		checksum += ((hi % 1000) * 4294967296 + lo) % 1000;
	}
	return { lo, hi, checksum };
}

function scalar(count) {
	let lo = 12345;
	let hi = 0;
	let checksum = 0;
	for (let i = 0; i < count; i++) {
		hi = (hi ^ ((hi << 13) | (lo >>> 19))) >>> 0;
		lo = (lo ^ (lo << 13)) >>> 0;
		lo = (lo ^ ((lo >>> 7) | (hi << 25))) >>> 0;
		hi = (hi ^ (hi >>> 7)) >>> 0;
		hi = (hi ^ ((hi << 17) | (lo >>> 15))) >>> 0;
		lo = (lo ^ (lo << 17)) >>> 0;
		checksum += ((hi % 1000) * 4294967296 + lo) % 1000;
	}
	return { lo, hi, checksum };
}

const variants = { bigint, words, scalar };
const name = process.argv[2] ?? "bigint";
const count = Number(process.argv[3] ?? 2000000);
assert(Number.isSafeInteger(count) && count > 0);
assert(Object.hasOwn(variants, name));

// Independent BigInt oracle, with difficult starting bit patterns too.
for (const seed of [0n, 1n, 12345n, 0xffffffffffffffffn, 0x8000000000000000n]) {
	let value = seed;
	let lo = Number(seed & 0xffffffffn);
	let hi = Number(seed >> 32n);
	for (let i = 0; i < 10000; i++) {
		value = xorshiftBigInt(value);
		({ lo, hi } = xorshiftWords(lo, hi));
		assert.equal((BigInt(hi) << 32n) | BigInt(lo), value);
		assert.equal(((hi % 1000) * 4294967296 + lo) % 1000, Number(value % 1000n));
	}
}
const expected = bigint(10000);
assert.deepEqual(words(10000), expected);
assert.deepEqual(scalar(10000), expected);
const run = variants[name];
for (let i = 0; i < 5; i++) run(100000);
const start = performance.now();
const result = run(count);
const milliseconds = performance.now() - start;
console.log(JSON.stringify({ name, count, milliseconds, ...result }));
