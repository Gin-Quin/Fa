// helpers: faQuote, faFormatError, FaError, IndexError, faSliceBounds, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function part(text) {
	const $t0 = faSliceBounds(text.length, 0, 1, false);
	let word = text.slice($t0[0], $t0[1]);
	const $t1 = faSliceBounds(text.length, 1, 3, false);
	word = text.slice($t1[0], $t1[1]);
	return word;
}

function main() {
	console.log(part("a😀z"));
}

faRun(main);
