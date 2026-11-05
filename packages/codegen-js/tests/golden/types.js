// helpers: FaStop, faStop, faOverflow, faCheckInteger, faFormatObject, faExit, faStopMessage, faIsStackOverflow, faWriteError, faIsError, faReport, faRun

const copy$Point = (value) => ({ x: value.x, y: value.y });
const Shape$Empty = Object.freeze({ $t: "Shape.Empty" });
const format = (value) => faFormatObject(value, [["x", String], ["y", String]]);
const Color$names = ["red", "green"];

function Point$length2(self) {
	return faCheckInteger(faCheckInteger(self.x * self.x, "types.fa:5:40") + faCheckInteger(self.y * self.y, "types.fa:5:58"), "types.fa:5:40");
}

function describe(shape) {
	if (shape.$t === "Shape.Circle") {
		const radius = shape.radius;
		return `circle ${radius}`;
	} else if (shape.$t === "Shape.Label") {
		const text = shape.$v;
		return text;
	} else {
		return "empty";
	}
}

function main() {
	let point = { x: 1, y: 2 };
	point.x = faCheckInteger(point.x + 1, "types.fa:19:1");
	const copy = copy$Point(point);
	const color = 1;
	console.log(`${Point$length2(point)} ${format(copy)} ${describe({ $t: "Shape.Label", $v: "hi" })} ${describe(Shape$Empty)} ${Color$names[color]}`);
	let items = [1, 2];
	items.push(3);
	for (let index = 0; index < items.length; index++) {
		const item = items[index];
		console.log(`${index} ${item}`);
	}
}

faRun(main);
