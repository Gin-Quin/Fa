// The loader of a Fa program compiled to WebAssembly (`fa build --target=wasm`):
//
//   node program.mjs arguments…     (or bun)
//
// It instantiates `program.wasm`, next to it, with the few functions the
// module imports from `fa`, and runs `_start`. The compiler keeps the parts
// between `// @import <name>` and `// @end` only when the module imports
// `<name>`. The program's exit code is
// the process's.
//
// An asynchronous program (design/async.md §4, step W1) exports `fa_run`:
// `_start` returns once the top level waits, and the loader runs a batch of
// the event loop (`fa_run`) from a macrotask while tasks are ready (that
// macrotask is the coop yield: timers and I/O of the host run between), and
// when the timer the program asked for (`fa.timer`) fires. The program ends
// when its top level and its detached tasks are done.

import { readFileSync, writeFileSync, writeSync } from "node:fs";

const WASM = new URL("__FA_WASM__", import.meta.url);

class Exit extends Error {
	constructor(code) {
		super(`exit ${code}`);
		this.code = code;
	}
}

const encoder = new TextEncoder();
// @import argumentsSize
const userArguments = encoder.encode(
	process.argv
		.slice(2)
		.map((argument) => `${argument}\0`)
		.join(""),
);
// @end

// @import formatFloat
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
// @end

// @import fileRun
// the files of `std.fs`: the bytes a file operation read, or the text of
// its error, until the program takes them (`fileTake`)
const decoder = new TextDecoder();
let pending = new Uint8Array(0);

// `faSystemMessage` of the JavaScript runtime: the text of `strerror`.
function systemMessage(error) {
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
	};
	return texts[error?.code] ?? String(error?.message ?? error);
}
// @end

let memory;
const imports = {
	fa: {
		write(fd, bytes, length) {
			writeSync(fd, new Uint8Array(memory.buffer, bytes, length));
		},
		exit(code) {
			throw new Exit(code);
		},
		// @import formatFloat
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
		// @end
		// @import argumentsSize
		argumentsSize() {
			return userArguments.length;
		},
		argumentsCopy(out) {
			new Uint8Array(memory.buffer, out, userArguments.length).set(userArguments);
		},
		// @end
		// @import pow
		pow: Math.pow,
		// @end
		// @import clock
		clock: () => performance.now(),
		// @end
		// @import now
		now: () => Date.now(),
		// @end
		// @import parseNumber
		parseNumber(text, length) {
			return Number(new TextDecoder().decode(new Uint8Array(memory.buffer, text, length)));
		},
		// @end
		// @import fileRun
		// 0: read the text, 1: read the bytes, 2: write the bytes
		fileRun(kind, path, pathLength, bytes, length, size) {
			const name = decoder.decode(new Uint8Array(memory.buffer, path, pathLength));
			let failed = 0;
			try {
				if (kind === 2) {
					writeFileSync(name, new Uint8Array(memory.buffer, bytes, length));
					pending = new Uint8Array(0);
				} else {
					pending = readFileSync(name);
				}
			} catch (error) {
				failed = 1;
				pending = encoder.encode(systemMessage(error));
			}
			new DataView(memory.buffer).setInt32(size, pending.length, true);
			return failed;
		},
		fileTake(out) {
			new Uint8Array(memory.buffer, out, pending.length).set(pending);
			pending = new Uint8Array(0);
		},
		// @end
		// @import timer
		// the latest timer replaces the earlier ones (`token` counts them)
		timer(token, milliseconds) {
			clearTimeout(timer);
			timer = setTimeout(step, milliseconds);
		},
		// @end
	},
};

// the event loop of an asynchronous program
let timer;
let running = false;

/** Runs a callback in a macrotask. */
function later(callback) {
	if (typeof setImmediate === "function") {
		setImmediate(callback);
	} else {
		const channel = new MessageChannel();
		channel.port1.onmessage = () => {
			channel.port1.close();
			callback();
		};
		channel.port2.postMessage(null);
	}
}

/** A batch of the event loop; another one in a macrotask while tasks are ready. */
function step() {
	if (running) {
		return;
	}
	running = true;
	let status = 2;
	try {
		status = instance.exports.fa_run();
	} catch (error) {
		ended(error);
	} finally {
		running = false;
	}
	if (status === 1) {
		later(step);
	} else if (status === 2) {
		clearTimeout(timer);
		checkMemory();
	}
}

/** The end of the program by an exception of `_start` or `fa_run`. */
function ended(error) {
	if (error instanceof Exit) {
		process.exitCode = error.code;
		clearTimeout(timer);
		checkMemory();
	} else if (error instanceof RangeError && /call stack|stack overflow/i.test(error.message)) {
		instance.exports.flush?.();
		writeSync(2, "stop: stack overflow\n");
		process.exitCode = 101;
		clearTimeout(timer);
	} else {
		throw error;
	}
}

const module = await WebAssembly.compile(readFileSync(WASM));
// the C functions of foreign code the module imports from `env` (those no C
// linked with it defines): exported by `env.mjs`, next to this loader, which
// may also export `attach(exports)` to reach the module's memory
const hosted = WebAssembly.Module.imports(module).filter((entry) => entry.module === "env");
let host;
if (hosted.length > 0) {
	const wanted = hosted.map((entry) => `\`${entry.name}\``).join(", ");
	try {
		host = await import(new URL("./env.mjs", import.meta.url).href);
	} catch (error) {
		throw new Error(
			`the program calls the C functions ${wanted}, which the host gives: export them from \`env.mjs\`, next to ${new URL(import.meta.url).pathname} (${error})`,
		);
	}
	imports.env = {};
	for (const { name } of hosted) {
		if (typeof host[name] !== "function") {
			throw new Error(`\`env.mjs\` doesn't export the C function \`${name}\``);
		}
		imports.env[name] = host[name];
	}
}
const instance = await WebAssembly.instantiate(module, imports);
memory = instance.exports.memory;
host?.attach?.(instance.exports);
let started = false;
try {
	instance.exports._start();
	started = true;
} catch (error) {
	ended(error);
}
if (started && instance.exports.fa_run !== undefined) {
	later(step);
} else if (started) {
	checkMemory();
}

/** The tests' memory cap: the size the memory grew to, in bytes. */
function checkMemory() {
	const limit = Number(process.env.FA_MEMORY_LIMIT ?? Infinity);
	if (memory.buffer.byteLength > limit) {
		writeSync(2, `memory: ${memory.buffer.byteLength} bytes, over the limit of ${limit}\n`);
		process.exitCode = 103;
	}
}
