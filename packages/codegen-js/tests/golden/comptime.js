// helpers: FaStop, faStop, faOverflow, faCheckInteger, faIsBytes, faQuote, faFormatRange, faSides, faFormatNested, faFormat, faFormatArray, faFormatBytes, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const format = (value) => faFormatArray(value, String);

function square(x) {
	return faCheckInteger(x * x, "comptime.fa:1:44");
}

function main() {
	console.log("12");
	console.log("25");
	{
		const fruit = "apple";
		console.log(`Fruit: ${faFormat(fruit)}`);
	}
	{
		const fruit = "banana";
		console.log(`Fruit: ${faFormat(fruit)}`);
	}
	{
		console.log("8080");
	}
	console.log(`${faFormatBytes(new Uint8Array([195, 169]))} ${format(Array.from(new Uint8Array([195, 169])))}`);
}

faRun(main);
