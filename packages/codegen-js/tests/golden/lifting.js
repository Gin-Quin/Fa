// helpers: FaStop, faStop, faOverflow, faCheckInteger, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function f() {
	console.log("a");
	return 1;
}

function middle() {
	return 2;
}

function main() {
	let $t2;
	const $t1 = f();
	let $t0;
	{
		console.log("b");
		$t0 = 2;
	}
	const x = faCheckInteger($t1 + $t0, "lifting.fa:6:9");
	const label = x > 2 ? "big" : "small";
	let described;
	if (x > 2) {
		console.log("computing");
		described = "big";
	} else {
		described = "small";
	}
	let candidate = 1;
	let root;
	while (true) {
		if (faCheckInteger(candidate * candidate, "lifting.fa:19:5") >= 50) {
			root = candidate;
			break;
		}
		candidate = faCheckInteger(candidate + 1, "lifting.fa:22:2");
	}
	const ordered = 1 < ($t2 = middle()) && $t2 <= 3;
	console.log(`${x} ${label} ${described} ${root} ${ordered}`);
}

faRun(main);
