// helpers: FaStop, faStop, faOverflow, faCheckInteger, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const limit = 4;

function inside(x) {
	return x * x <= limit;
}

function sieve(size) {
	let marks = [];
	for (let $t0 = 0; $t0 < size; $t0++) {
		marks.push(true);
	}
	let count = 0;
	for (let i = 2, $t1 = marks.length; i < $t1; i++) {
		if (marks[i]) {
			count = faCheckInteger(count + 1, "hot-loops.fa:15:4");
			if (i <= (marks.length - marks.length % i) / i) {
				let multiple = faCheckInteger(i * i, "hot-loops.fa:17:24");
				while (multiple < marks.length) {
					marks[multiple] = false;
					multiple = faCheckInteger(multiple + i, "hot-loops.fa:20:6");
				}
			}
		}
	}
	return count;
}

function checksum(values) {
	const modulus = 1000000007;
	let sum = 0;
	for (const value of values) {
		sum = faCheckInteger(faCheckInteger(sum * 31, "hot-loops.fa:32:10") + value, "hot-loops.fa:32:10") % modulus;
	}
	return sum;
}

function halves(a, b) {
	return a / b | 0;
}

function mix(a, b) {
	return Math.imul((a ^ b) >>> 0, 16777619) >>> 0;
}

function main() {
	console.log(`${inside(1.5)} ${sieve(100)} ${checksum([1, 2, 3])} ${halves(7, 2)} ${mix(1, 2)}`);
}

faRun(main);
