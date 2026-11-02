// The wasm side of the coroutine spike (`spike.rs`): runs the benchmarks of
// the module, as `driver.c`'s `main` does on native.
//
//   node bench.mjs spike.wasm <n> [name]
//
// Prints `name n result allocations frame_size nanoseconds` per benchmark.

import { readFileSync } from "node:fs";

const [file, count = "1000", only] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(readFileSync(file), {});
const exports = instance.exports;
const n = BigInt(count);

function counter(name) {
	return new BigUint64Array(exports.memory.buffer, exports[name].value, 1);
}

const names = [
	"gen_opaque",
	"gen_inline",
	"sm_direct",
	"sm_indirect",
	"sm_inline",
	"task_yield",
	"task_sync",
	"task_spawn",
];
for (const name of names) {
	if (only !== undefined && only !== name) {
		continue;
	}
	counter("fa_alloc_count")[0] = 0n;
	counter("fa_last_alloc_size")[0] = 0n;
	const start = performance.now();
	const result = exports[`bench_${name}`](n);
	const elapsed = (performance.now() - start) * 1e6;
	const allocations = counter("fa_alloc_count")[0];
	const size = counter("fa_last_alloc_size")[0];
	console.log(`${name} ${n} ${result} ${allocations} ${size} ${elapsed.toFixed(0)}`);
}
