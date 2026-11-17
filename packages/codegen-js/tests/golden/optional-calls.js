// helpers: faIsBytes, faQuote, faFormatRange, faSides, faFormatNested, faFormat, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function Person$greet(self, suffix) {
	return self.name + suffix;
}

function text(value) {
	return value === undefined ? undefined : value.trim().toUpperCase();
}

function greet(value) {
	let $t1;
	if (value !== undefined) {
		let $t0;
		if (!(value === undefined)) {
			console.log("argument");
			$t0 = "!";
		} else {
			$t0 = "?";
		}
		$t1 = Person$greet(value, $t0);
	}
	return $t1;
}

function main() {
	console.log(`${faFormat(text(undefined))} ${faFormat(text(" Ada "))} ${faFormat(greet(undefined))} ${faFormat(greet({ name: "Bo" }))}`);
}

faRun(main);
