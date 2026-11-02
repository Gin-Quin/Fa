// helpers: FaStop, faStop, faOverflow, faCheckInteger, faCheckRange, faCheckWidth, faFloorQuotient, faInt64ToInteger, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function wide(a, b) {
	return a + b;
}

function average(a, b) {
	return faFloorQuotient(a + b, 2);
}

function narrow(a, b) {
	return faCheckRange(a + b, -128, 127, "Int8", "checks.fa:4:47");
}

function grow(x) {
	return faCheckInteger(faCheckInteger(x * 2, "checks.fa:5:42") + 1, "checks.fa:5:42");
}

function small(i) {
	return i * i + i;
}

function big(x, y) {
	return faCheckWidth(x * y, 64, true, "Int64", "checks.fa:7:47");
}

function mixed(x, n) {
	return faCheckWidth(x + BigInt(n), 64, true, "Int64", "checks.fa:8:51");
}

function toNumber(x) {
	return faInt64ToInteger(x, "checks.fa:9:44");
}

function divide(a, b) {
	return faCheckInteger(faFloorQuotient(a, b), "checks.fa:10:65");
}

function main() {
	console.log(`${wide(1, 2)} ${average(3, 4)} ${narrow(5, 6)} ${grow(7)} ${small(8)}`);
	console.log(`${big(9n, 10n)} ${mixed(11n, 12)} ${toNumber(13n)} ${divide(14, 3)}`);
}

faRun(main);
