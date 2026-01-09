type Point = {
	x: number;
	y: number;
};

enum Color {
	Red,
	Green,
	Blue,
}

type Result =
	| { kind: "Ok"; value: number }
	| { kind: "Err"; message: string };

const add = (left: Point, right: Point): Point => ({
	x: left.x + right.x,
	y: left.y + right.y,
});

const safeDivide = (left: number, right: number): Result => {
	if (right === 0) {
		return { kind: "Err", message: "divide by zero" };
	}
	return { kind: "Ok", value: left / right };
};

const points: Point[] = [
	{ x: 1, y: 2 },
	{ x: 3, y: 4 },
	{ x: 5, y: 6 },
];
let total: Point = { x: 0, y: 0 };

for (const point of points) {
	total = add(total, point);
}

const outcome = safeDivide(10, 2);

if (outcome.kind === "Ok") {
	outcome.value;
} else {
	0;
}
