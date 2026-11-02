// helpers: FaSource, faSource, faFieldSource, faReactive, faTrack, faMark, faNotify, faChanged, FaDerivedNode, faDerived, FaStop, faStop, faUnsubscribe, faOutdated, faRefresh, faRead, FaWatchNode, faRunWatch, faWatch, faDispose, faFlush, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const number$s = faSource();
const values$s = faSource();
const channel$s = faSource();
let channel;

function addAll(numbers, extra) {
	for (const value of extra) {
		numbers.push(value);
	}
}
function addAll$r(numbers, extra, numbers$s, extra$s) {
	for (const value of (faTrack(extra$s), extra)) {
		faChanged(numbers.push(value), numbers$s?.length());
	}
}

function main() {
	const multiplier = 2;
	let number = 3;
	const multiplied = faDerived(() => {
		return (faTrack(number$s), number) * multiplier;
	});
	faWatch(() => {
		console.log(`Multiplied is ${faRead(multiplied)}`);
	});
	faFlush();
	const $t0 = number;
	number = 5;
	if ($t0 !== number) {
		faNotify(number$s);
	}
	faFlush();
	let values = [1, 2, 3];
	faWatch(() => {
		console.log(`Length ${(faTrack(values$s.length()), values.length)}`);
	});
	faFlush();
	values[0] = 10;
	faNotify(values$s.index(0));
	faFlush();
	addAll$r(values, [4], values$s, null);
	faFlush();
	let plain = [1];
	addAll(plain, [2]);
	let counter = { label: "clicks", count: 1 };
	channel = "general";
	const subscription = faWatch(() => {
		const channel$copy = channel;
		console.log(`${(faTrack(channel$s), channel$copy)} ${(faTrack(faFieldSource(counter, "count")), counter.count)}`);
		return () => console.log(`leave ${channel$copy}`);
	});
	faFlush();
	const $t1 = counter.count;
	counter.count = 2;
	if ($t1 !== counter.count) {
		faNotify(faFieldSource(counter, "count"));
	}
	faFlush();
	faDispose(subscription);
}

faRun(main);
