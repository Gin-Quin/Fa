// Sixteen dense integer arms: emitted if-chain versus a switch.
import assert from "node:assert/strict";

function chain(value) {
	if (value === 0) return 31;
	else if (value === 1) return 71;
	else if (value === 2) return 11;
	else if (value === 3) return 53;
	else if (value === 4) return 97;
	else if (value === 5) return 19;
	else if (value === 6) return 43;
	else if (value === 7) return 83;
	else if (value === 8) return 47;
	else if (value === 9) return 23;
	else if (value === 10) return 89;
	else if (value === 11) return 37;
	else if (value === 12) return 67;
	else if (value === 13) return 17;
	else if (value === 14) return 59;
	else return 29;
}

function switched(value) {
	switch (value) {
		case 0:
			return 31;
		case 1:
			return 71;
		case 2:
			return 11;
		case 3:
			return 53;
		case 4:
			return 97;
		case 5:
			return 19;
		case 6:
			return 43;
		case 7:
			return 83;
		case 8:
			return 47;
		case 9:
			return 23;
		case 10:
			return 89;
		case 11:
			return 37;
		case 12:
			return 67;
		case 13:
			return 17;
		case 14:
			return 59;
		default:
			return 29;
	}
}

function run(count, choose) {
	let state = 12345;
	let checksum = 0;
	for (let i = 0; i < count; i++) {
		state ^= state << 13;
		state ^= state >>> 17;
		state ^= state << 5;
		checksum += choose(state & 15);
	}
	return checksum;
}

const name = process.argv[2] ?? "chain";
const count = Number(process.argv[3] ?? 10000000);
const choose = name === "chain" ? chain : switched;
for (let i = 0; i < 16; i++) assert.equal(chain(i), switched(i));
for (let i = 0; i < 5; i++) run(100000, choose);
const start = performance.now();
const checksum = run(count, choose);
console.log(JSON.stringify({ name, count, milliseconds: performance.now() - start, checksum }));
