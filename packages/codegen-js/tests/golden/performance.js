// helpers: FaStop, faStop, faOverflow, faCheckInteger, faIsSimpleText, faSegmenter, faCharacters, faStringLength, faPadding, faStringPadStart, faSortIntegers, faSortFloats, faStringCompare, faCompare, faArraySortInPlace, faSortStrings, faQuote, faFormatArray

const format = (value) => faFormatArray(value, String);
const format$1 = (value) => faFormatArray(value, faQuote);

function primes(limit) {
	let isPrime = new Uint8Array(Math.max(limit + 1, 0)).fill(1);
	let count$1 = 0;
	for (let i = 2, $t0 = isPrime.length; i < $t0; i++) {
		const prime = isPrime[i] === 1;
		if (prime) {
			count$1 = faCheckInteger(count$1 + 1, "performance.fa:14:4");
			let multiple = i + i;
			while (multiple < isPrime.length) {
				isPrime[multiple] = false;
				multiple += i;
			}
		}
	}
	return count$1;
}

function shown(size) {
	let flags = [];
	for (let $t0 = 0; $t0 < size; $t0++) {
		flags.push(false);
	}
	console.log(format(flags));
	return count(flags);
}

function count(flags) {
	let total = 0;
	for (const flag of flags) {
		if (flag) {
			total = faCheckInteger(total + 1, "performance.fa:39:4");
		}
	}
	return total;
}

function sorted(numbers, floats, words) {
	faSortIntegers(numbers);
	const ordered = faSortFloats(floats.slice());
	const names = faSortStrings(words.slice());
	console.log(`${format(numbers)} ${format(ordered)} ${format$1(names)}`);
}

function middle(a, b) {
	return Math.floor(faCheckInteger(a + b, "performance.fa:54:48") / 2);
}

function eighth(a) {
	return Math.floor(a / 8);
}

function padded(i, label) {
	console.log(`${String(i).padStart(8, "0")} ${`${i}:${faCheckInteger(i * 2, "performance.fa:60:51")}`.padEnd(10, ".")} ${faStringPadStart(label, 10)}`);
	console.log(faStringPadStart(String(i), 8, "é"));
}

