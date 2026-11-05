// helpers: FaStop, faStop, faOverflow, faCheckInteger, faQuote, faFormatArray, faSides, faFormatRecord, faFormatMember, faFormatTagged, faFormatMap, faFormatSet, faFormatRange, faFormatNested, faFormat, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

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
}

faRun(main);
