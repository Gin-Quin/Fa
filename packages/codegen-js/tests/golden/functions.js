// helpers: FaStop, faStop, faOverflow, faCheckInteger, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function move(x, y = x, speed = 1) {
	console.log(`(${x}, ${y}) at ${speed}`);
}

function sum(...values) {
	return 0;
}

function twice(f, value) {
	return f(f(value));
}

function fib(n) {
	if (n < 2) {
		return n;
	} else {
		return faCheckInteger(fib(faCheckInteger(n - 1, "functions.fa:11:67")) + fib(faCheckInteger(n - 2, "functions.fa:11:80")), "functions.fa:11:63");
	}
}

function main() {
	move(1, undefined, 3);
	move(2);
	console.log(String(sum(1, 2, 3)));
	console.log(String(twice((n) => faCheckInteger(n * 3, "functions.fa:10:44"), 2)));
	const class$ = fib(10);
	console.log(String(class$));
}

faRun(main);
