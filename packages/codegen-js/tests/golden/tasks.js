// helpers: faQuote, faFormatError, FaError, faCaught, FaStop, faStop, faOverflow, faCheckInteger, faFormatArray, faFormatObject, faFormatMember, faFormatTagged, faFormatMap, faFormatSet, faFormatRange, faFormatNested, faFormat, FaChannel, faSched, faLater, faMacrotask, faCancelSignal, faAlive, faCoop, FaTask, faIsError, faCancel, faEnd, faExit, faStopMessage, faIsStackOverflow, faWriteError, faReport, faFatal, faSettle, faRunTask, faStart, faDetach, faReady, faWhenDone, Cancelled, faResult, faJoin, faCombine, faYield, faSleep, faSleepSync, ChannelClosed, faRemove, faSend, faReceive, faRunAsync, faWaiting, faReceiveNow, faSendNow

async function pause___async($task, milliseconds) {
	await faSleep($task, milliseconds);
	return milliseconds;
}
function pause___sync(milliseconds) {
	faSleepSync(milliseconds);
	return milliseconds;
}

async function drain($task, channel) {
	let $t0;
	let total = 0;
	while (true) {
		const value = ($t0 = faReceiveNow(channel)) !== faWaiting ? $t0 : await faReceive($task, channel, "tasks.fa:15:20");
		if (value === undefined) break;
		if (--faSched.n < 0) await faCoop($task);
		total = faCheckInteger(total + value, "tasks.fa:16:3");
	}
	return total;
}

function blocking() {
	return pause___sync(1);
}

async function main($task) {
	let channel = new FaChannel(2);
	const draining = faStart($task, drain, channel);
	for (let value = 1; value <= 3; value++) {
		if (--faSched.n < 0) await faCoop($task);
		try {
			const $t0 = channel;
			const $t1 = value;
			faSendNow($t0, $t1) || await faSend($task, $t0, $t1, "tasks.fa:26:2");
		} catch ($e) {
			const error = faCaught($e);
			break;
		}
	}
	channel.close();
	console.log(String(await faJoin($task, draining)));
	console.log(String(faCheckInteger(await pause___async($task, 2) + blocking(), "tasks.fa:30:13")));
	const both = await faJoin($task, faCombine($task, [faStart($task, pause___async, 3), faStart($task, pause___async, 4)], 0));
	console.log(faFormat(both));
	faDetach(faStart($task, async ($task) => {
		await faYield($task);
		console.log("detached");
	}));
}

faRunAsync(main);
