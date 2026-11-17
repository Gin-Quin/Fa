// helpers: faCopyAs, faHashMix, faHashString, faHashNumber, faSides, faHash, FaSet, faIsBytes, faBytesEqual, faDeepEquals, faKeyEquals, FaHashMap, faCopyShell, faSharedHandle, faCopy, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const copy$convert$1 = (value, target, key, work) => { target[key] = faCopy(value); };
const copy$convert$3 = (value, target, key, work) => { target[key] = faCopy(value); };
const copy$convert$2 = (value, target, key, work) => { work.push([copy$convert$3, value, target, key]); };
const copy$convert = (value, target, key, work) => { if (value === undefined) { work.push([copy$convert$1, value, target, key]); return; } if (typeof value === "object") { work.push([copy$convert$2, value, target, key]); return; } };

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
		snapshot = faCopyAs(snapshot.next, copy$convert);
	}
}

faRun(main);
