// helpers: faQuote, faFormatError, FaError, faCaught, faSegmenter, faCharacters, faStringCodePoints, faCodePointCount, faUtf8Encoder, faUtf8LossyDecoder, faUtf8Decoder, Utf8Error, faInvalidUtf8, faFromUtf8, faUtf8Values, IndexError, faNormalizeIndex, faSliceBounds, faIndex, faBytesEqual, faIsBytes, faFormatRange, faSides, faFormatNested, faFormat, faFormatOptional, faFormatArray, faFormatBytes, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const format = (value) => faFormatOptional(value, String);
const format$1 = (value) => faFormatArray(value, faQuote);
const format$2 = (value) => faFormatArray(value, String);

function inspect(text, i, replacement) {
	console.log(`${text.length} ${text.length} ${Buffer.byteLength(text)} ${text.isWellFormed()}`);
	const $t1 = text.at(i);
	let $t0;
	try {
		$t0 = faIndex(text, i);
	} catch ($e) {
		const error = faCaught($e);
		$t0 = "?";
	}
	let $t2;
	try {
		$t2 = text.charCodeAt(faNormalizeIndex(text.length, i));
	} catch ($e) {
		const error = faCaught($e);
		$t2 = 0;
	}
	console.log(`${faFormat($t1)} ${$t0} ${$t2}`);
	let $t3;
	try {
		const $t4 = faSliceBounds(text.length, i, undefined, false);
		$t3 = text.slice($t4[0], $t4[1]);
	} catch ($e) {
		const error = faCaught($e);
		$t3 = "?";
	}
	let $t5;
	try {
		const $t6 = faSliceBounds(text.length, 0, i, true);
		$t5 = text.slice($t6[0], $t6[1]);
	} catch ($e) {
		const error = faCaught($e);
		$t5 = "?";
	}
	console.log(`${$t3} ${$t5}`);
	const $t7 = text.indexOf("😀");
	const $t9 = $t7 === -1 ? undefined : $t7;
	const $t8 = text.lastIndexOf("é");
	console.log(`${format($t9)} ${format($t8 === -1 ? undefined : $t8)} ${text.includes("e")} ${text.startsWith("日")} ${text.endsWith("😀")}`);
	console.log(`${format$1(text.split(""))} ${text.padStart(10)} ${text.padEnd(12, "é")} ${text < "￿"}`);
	const $t11 = text.replace("-", "x");
	const $t12 = text.replace("-", "$$&");
	const $t10 = replacement;
	console.log(`${$t11} ${$t12} ${text.replace("-", () => $t10)}`);
	const $t14 = text.replaceAll("-", "x");
	const $t15 = text.replaceAll("-", "$$&");
	const $t13 = replacement;
	console.log(`${$t14} ${$t15} ${text.replaceAll("-", () => $t13)}`);
	console.log(`${faCodePointCount(text)} ${format$2(faStringCodePoints(text))} ${format$1(faCharacters(text))} ${faFormatBytes(faUtf8Encoder.encode(text))}`);
	const at = ($t16) => text.charCodeAt(faNormalizeIndex(text.length, $t16));
	let $t17;
	try {
		$t17 = at(i);
	} catch ($e) {
		const error = faCaught($e);
		$t17 = 0;
	}
	console.log($t17);
	for (const point of text) {
		console.log(point);
	}
	for (const point of text) {
		console.log(point);
	}
	for (let index = 0, $t18; index < text.length; index += $t18 > 0xffff ? 2 : 1) {
		$t18 = text.codePointAt(index);
		const point = String.fromCodePoint($t18);
		console.log(`${index} ${point}`);
	}
	for (const byte of faUtf8Values(text)) {
		console.log(String(byte));
	}
	let $t19 = 0;
	for (const byte of faUtf8Values(text)) {
		const index = $t19++;
		console.log(`${index} ${byte}`);
	}
}

function scan(text) {
	for (let index = 0, $t0 = text.length; index < $t0; index++) {
		console.log(`${text.charCodeAt(index) === 97} ${26085 === text.charCodeAt(index)}`);
		let $t1;
		const $t2 = text.charCodeAt(index);
		if ($t2 === 97) {
			$t1 = 1;
		} else if ($t2 === 233) {
			$t1 = 2;
		} else {
			$t1 = 3;
		}
		console.log(faFormat($t1));
	}
}

function binary(input, index) {
	let data = new Uint8Array(3);
	console.log(faFormatBytes(Uint8Array.from(input)));
	data[0] = 255;
	const copy = data.slice(0, 2);
	const view = data.subarray(0, 1);
	console.log(`${data.length} ${format(data.at(index))} ${format$2(Array.from(data))} ${faFormatBytes(data)} ${faBytesEqual(data, copy)} ${faFormatBytes(view)}`);
	let $t0;
	try {
		$t0 = faFromUtf8(data);
	} catch ($e) {
		const error = faCaught($e);
		$t0 = `${error.position}`;
	}
	console.log(`${$t0} ${faUtf8LossyDecoder.decode(data)}`);
	console.log(`${faFormatBytes(new Uint8Array(3))} ${faFormatBytes(new Uint8Array(3).fill(7))}`);
}

function main() {
	inspect("é日😀-a", 0, "$1 $$ $&");
	scan("aé日😀");
	binary([97, 98, 99], 0);
}

faRun(main);
