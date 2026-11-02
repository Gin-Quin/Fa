// helpers: FaStop, faStop, faOverflow, faInt64ToInteger, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function xorshift(x) {
	let v = x;
	v = v ^ BigInt.asUintN(64, v << 13n);
	v = v ^ v >> 7n;
	return v ^ BigInt.asUintN(64, v << 17n);
}

function below(random, bound) {
	return faInt64ToInteger(random % BigInt(bound), "sized-bigints.fa:11:80");
}

function main() {
	const signed = -40n;
	const r = xorshift(12345n);
	console.log(`${r} ${below(r, 1000)} ${r / 1000n} ${BigInt.asIntN(64, signed << 2n)} ${signed >> 3n} ${BigInt.asUintN(64, ~r)}`);
}

faRun(main);
