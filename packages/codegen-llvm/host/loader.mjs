// The loader of a Fa program compiled to WebAssembly (`fa build --target=wasm`):
//
//   node program.mjs arguments…     (or bun, or deno run --allow-read)
//
// It instantiates `program.wasm`, next to it, with the few functions the
// module imports from `fa`, and runs `_start`. The program's exit code is
// the process's.

import { readFileSync, writeSync } from "node:fs";

const WASM = new URL("__FA_WASM__", import.meta.url);

class Exit extends Error {
	constructor(code) {
		super(`exit ${code}`);
		this.code = code;
	}
}

const encoder = new TextEncoder();
const userArguments = encoder.encode(
	process.argv
		.slice(2)
		.map((argument) => `${argument}\0`)
		.join(""),
);

// `faFloat32ToString` of the JavaScript runtime: the shortest decimal that
// reads back as the same Float32.
function float32ToString(value) {
	if (value === 0 || !Number.isFinite(value)) {
		return String(value);
	}
	for (let digits = 1; digits <= 9; digits += 1) {
		const [mantissa, exponent] = value.toExponential(digits - 1).split("e");
		const scaled = Number(mantissa.replace(".", ""));
		const power = Number(exponent) - (digits - 1);
		let best;
		for (const candidate of [scaled, scaled - 1, scaled + 1]) {
			const decimal = Number(`${candidate}e${power}`);
			if (
				Math.fround(decimal) === value &&
				(best === undefined || Math.abs(decimal - value) < Math.abs(best - value))
			) {
				best = decimal;
			}
			if (best !== undefined && candidate === scaled) {
				break;
			}
		}
		if (best !== undefined) {
			return String(best);
		}
	}
	return String(value);
}

// `faPercentageToString`, without the `%` (the runtime adds it).
function percentageToString(fraction) {
	const value = fraction * 100;
	if (value === 0 || !Number.isFinite(value)) {
		return String(value);
	}
	return String(Number(value.toPrecision(15)));
}

let memory;
const imports = {
	fa: {
		write(fd, bytes, length) {
			writeSync(fd, new Uint8Array(memory.buffer, bytes, length));
		},
		exit(code) {
			throw new Exit(code);
		},
		formatFloat(value, kind, out) {
			const text =
				kind === 1
					? float32ToString(value)
					: kind === 2
						? percentageToString(value)
						: String(value);
			const bytes = encoder.encode(text);
			new Uint8Array(memory.buffer, out, bytes.length).set(bytes);
			return bytes.length;
		},
		argumentsSize() {
			return userArguments.length;
		},
		argumentsCopy(out) {
			new Uint8Array(memory.buffer, out, userArguments.length).set(userArguments);
		},
		pow: Math.pow,
	},
};

const { instance } = await WebAssembly.instantiate(readFileSync(WASM), imports);
memory = instance.exports.memory;
try {
	instance.exports._start();
} catch (error) {
	if (error instanceof Exit) {
		process.exitCode = error.code;
	} else if (error instanceof RangeError && /call stack|stack overflow/i.test(error.message)) {
		instance.exports.flush?.();
		writeSync(2, "stop: stack overflow\n");
		process.exitCode = 101;
	} else {
		throw error;
	}
}
// the tests' memory cap: the size the memory grew to, in bytes
const limit = Number(process.env.FA_MEMORY_LIMIT ?? Infinity);
if (memory.buffer.byteLength > limit) {
	writeSync(2, `memory: ${memory.buffer.byteLength} bytes, over the limit of ${limit}\n`);
	process.exitCode = 103;
}
