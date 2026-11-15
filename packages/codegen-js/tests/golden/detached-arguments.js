// helpers: faFormatArray, faFormatObject, FaTask, faIsError, faCancelSignal, faSched, faCancel, faEnd, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faReport, faFatal, faSettle, faRunTask, faStartCall, faDetach, faLater, faMacrotask, faReady, faAlive, faYield, faWhenDone, faRunAsync

const format = (value) => faFormatObject(value, [["value", String]]);
const format$1 = (value) => faFormatArray(value, format);
const copy$Box = (value) => ({ value: value.value });
const copy$Box$array = (value) => value.map(copy$Box);

function report(value) {
	console.log(format(value));
}

function collect(...values) {
	console.log(format$1(values));
}

async function main($task) {
	let values = [{ value: 1 }];
	faDetach(faStartCall($task, report, copy$Box(values[0])));
	faDetach(faStartCall($task, collect, ...copy$Box$array(values)));
	const $t0 = values[0];
	$t0.value = 2;
	await faYield($task);
}

faRunAsync(main);
