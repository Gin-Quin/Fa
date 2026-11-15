// helpers: faHashMix, faHashString, faHashNumber, faSides, faHash, FaSet, faDeepEquals, faKeyEquals, FaHashMap, faCopyShell, faSharedHandle, faCopy, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function main() {
	let head = undefined;
	for (let index = 0; index < 3; index++) {
		head = { value: index, next: head };
	}
	let snapshot = faCopy(head);
	if (!(head === undefined)) {
		head.value = -1;
	}
	while (!(snapshot === undefined)) {
		console.log(String(snapshot.value));
		snapshot = snapshot.next;
	}
}

faRun(main);
