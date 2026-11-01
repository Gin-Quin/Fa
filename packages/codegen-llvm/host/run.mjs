// Runs a Fa program compiled for `wasm32-unknown-unknown`, with Node or Bun:
//
//   node host/run.mjs program.wasm
//
// The module imports its few host functions from the `fa` module, and exports
// its memory and `_start`. A browser host provides the same imports.

import { readFileSync, writeSync } from "node:fs";

class Exit extends Error {
	constructor(code) {
		super(`exit ${code}`);
		this.code = code;
	}
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
	},
};

const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), imports);
memory = instance.exports.memory;
try {
	instance.exports._start();
} catch (error) {
	if (!(error instanceof Exit)) throw error;
	process.exitCode = error.code;
}
