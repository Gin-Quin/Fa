// helpers: 

function main() {
	const $t0 = [];
	$t0.push("before");
	for (const $t1 of [1, 2]) {
		$t0.push($t1);
	}
	$t0.push("after");
	console.log($t0.join(" "));
}

main();
