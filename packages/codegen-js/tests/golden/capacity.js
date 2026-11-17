// helpers: faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function grow(count) {
	let values = [];
	for (let index = 0; index < count; index++) {
		values.push(index);
	}
	return values;
}

function room(count) {
	return Math.max(count, 1);
}

function main() {
	let text = "";
	room(8);
	text += "abc";
	let scores = new Map();
	console.log(`${grow(3).length} ${text.length} ${scores.size}`);
}

faRun(main);
