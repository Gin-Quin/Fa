// helpers: faHashMix, faHashString, faHashNumber, faSides, faHash, FaSet, faIsBytes, faBytesEqual, faDeepEquals, faKeyEquals, FaHashMap, faQuote, faFormatRange, faFormatNested, faFormat, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function main() {
	console.log(`${new FaSet([{ code: 1 }]).has({ code: 1 })} ${faFormat(new FaHashMap([[[1, "one"], "value"]]).get([1, "one"]))}`);
}

faRun(main);
