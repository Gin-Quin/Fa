// helpers: FaStop, faStop, faOverflow, faCheckInteger, faQuote, faFormatError, FaError, IndexError, faNormalizeIndex, faIndex, faAdd, faReferenceCellTag, faBorrowSlot, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

function Model$add(self, value) {
	return faAdd(self.nodes, { value: value });
}

function Model$firstValue(self) {
	const $t0 = self.values;
	const $t1 = faNormalizeIndex($t0.length, 0);
	const $t2 = $t0;
	const $t3 = $t1;
	return faBorrowSlot($t2, $t3, false, false);
}

function bump(model, node) {
	node.value = faCheckInteger(node.value + 1, "reference-objects.fa:8:83");
}

function main() {
	let model = { nodes: new Set(), values: [4] };
	const n = Model$add(model, 3);
	bump(model, n);
	const v = Model$firstValue(model);
	v.$v = 8;
	console.log(String(n.value));
	console.log(String(faIndex(model.values, 0)));
}

faRun(main);
