// helpers: faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function pick(index) {
	if (index < 0) {
		return undefined;
	} else if (index === 0) {
		return 0;
	} else {
		return 2;
	}
}

function main() {
	const first = pick(1);
	const second = pick(1);
	console.log(`${first === second} ${first !== second} ${pick(-1) === pick(-1)}`);
}

faRun(main);
