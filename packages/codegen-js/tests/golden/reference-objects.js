// helpers: FaStop, faStop, faOverflow, faCheckInteger, faQuote, faFormatError, FaError, IndexError, faNormalizeIndex, faIndex, faAdd, faReferenceKey, faReferenceResult, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function Model$add(self, value) {
	return faAdd(self.nodes, { value: value });
}

function Model$firstValue(self) {
	let $t0, $t1;
	$referenceReturn: {
		const $t2 = self.values;
		const $t3 = faNormalizeIndex($t2.length, 0);
		$t0 = $t2;
		$t1 = $t3;
		break $referenceReturn;
	}
	return faReferenceResult($t0, $t1);
}
function Model$firstValue$key(self) {
	const $t0 = self.values;
	const $t1 = faNormalizeIndex($t0.length, 0);
	return $t1;
}

function bump(model, node) {
	node.value = faCheckInteger(node.value + 1, "reference-objects.fa:8:83");
}

function main() {
	let model = { nodes: new Set(), values: [4] };
	const n = Model$add(model, 3);
	bump(model, n);
	const $t0 = Model$firstValue$key(model);
	model.values[$t0] = 8;
	console.log(String(n.value));
	console.log(String(faIndex(model.values, 0)));
}

faRun(main);
