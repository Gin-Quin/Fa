// helpers: faFormatObject, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const format = (value) =>
	faFormatObject(value, [
		["a", String],
		["z", String],
	]);
const format$1 = (value) =>
	faFormatObject(value, [
		["a", String],
		["z", String],
		["zz", String],
	]);

function pure(x, y) {
	return { a: y, z: x };
}

function effect(label, value) {
	console.log(label);
	return value;
}

function main() {
	const $t0 = effect("first", 9);
	const $t1 = effect("second", 1);
	const ordered = { a: $t1, z: $t0, zz: effect("last", 3) };
	console.log(`${format(pure(9, 1))} ${format$1(ordered)}`);
}

faRun(main);
