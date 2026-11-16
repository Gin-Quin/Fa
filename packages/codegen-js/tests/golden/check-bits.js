// helpers: faQuote, faFormatError, FaError, faCaught, FaStop, faStop, faOverflow, faCheckInteger, faCheckRange, DivisionByZero, faFloorQuotient, faFloorDivide, ArithmeticError, faPower, faConvertInteger, faFloatToIntegerResult, faRound, faIntegerRound, faWrap, faWrapBig, faFromBig64, faShiftAmount, faBitwiseResult, faShiftLeft, IndexError, faNormalizeIndex, faIndex, faIndexSet, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function indices(items, i) {
	let duplicate = items.slice();
	for (let j = 0, $t0 = duplicate.length; j < $t0; j++) {
		duplicate[j] = faCheckInteger(duplicate[j] + 1, "check-bits.fa:6:18");
	}
	faIndexSet(duplicate, i, 0);
	return faCheckInteger(faIndex(duplicate, i) + duplicate.length, "check-bits.fa:9:2");
}

function shifts(x, amount) {
	const proven = x << 3;
	const unproven = faShiftLeft(x, amount, 32, true, "check-bits.fa:13:21");
	return faCheckRange(proven + unproven, -2147483648, 2147483647, "Int32", "check-bits.fa:14:2");
}

function conversions(x, f) {
	const small = x >= 0 && x < 100 ? x : 0;
	const any = faConvertInteger(x, -128, 127, "Int8");
	const rounded = faRound(2.5) + 0;
	const computed = faIntegerRound(f, "check-bits.fa:20:21");
	return faCheckRange(small + any + faConvertInteger(faCheckInteger(rounded + computed, "check-bits.fa:21:26"), -128, 127, "Int8"), -128, 127, "Int8", "check-bits.fa:21:2");
}

function powers(base, exponent) {
	const literal = faPower(base, 2);
	const unknown = faCheckInteger(faPower(base, exponent), "check-bits.fa:25:20");
	return faCheckInteger(literal + unknown, "check-bits.fa:26:2");
}

function divisions(a, b, c) {
	const proven = (a / b | 0) + a % b;
	const unproven = faFloorDivide(a, c);
	return proven + unproven;
}

function main() {
	let $t0;
	try {
		$t0 = indices([1, 2], 0);
	} catch ($e) {
		const e = faCaught($e);
		$t0 = -1;
	}
	console.log($t0);
	let $t1;
	try {
		$t1 = shifts(1, 2);
	} catch ($e) {
		const e = faCaught($e);
		$t1 = -1;
	}
	let $t2;
	try {
		$t2 = conversions(5, 1.5);
	} catch ($e) {
		const e = faCaught($e);
		$t2 = -1;
	}
	let $t3;
	try {
		$t3 = powers(2, 3);
	} catch ($e) {
		const e = faCaught($e);
		$t3 = -1;
	}
	console.log(`${$t1} ${$t2} ${$t3}`);
	let $t4;
	try {
		$t4 = divisions(7, 2, 3);
	} catch ($e) {
		const e = faCaught($e);
		$t4 = -1;
	}
	console.log($t4);
}

faRun(main);
