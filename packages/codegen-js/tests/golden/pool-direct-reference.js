// helpers: FaValuePool, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function main() {
	let values = new FaValuePool([1]);
	const $t2 = values;
	const $t0 = values;
	const $t1 = $t0.add(1);
	$t2.delete($t1);
	const $t5 = values;
	const $t3 = values;
	const $t4 = $t3.add(7);
	console.log(`${$t5.take($t4)} ${values.size}`);
}

faRun(main);
