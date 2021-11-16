# I had this idea lately: we use flat composition for objects

type Coco =
	x: Number
	y = 43
	const toString() = "{x} {y}"

let coco = Coco(x = 5321, y = 12)
let coco = Coco:
	x = 12312

type YT =
	y = "1212"
	t = 12

export type Coco =
	...YT # we inherit YT
	...Coco { x, toString } # we partly inherit Coco (using the `fields` syntax)
	...Coco:
		x
		toString
	coco = Coco # we compose Coco
	z = 11


# In JS
const Coco = (self = {
	x: 12,
	y: 43,
	toString: () => `${self.x} ${self.y}`,
}) => self

const YT = (self = {
	y: "1212",
	t: 12,
}) => self

const Zabu = ({ x, toString } = Coco(), self = {
	...YT(),
	x,
	toString: toString.bind(self),
	z: 11,
})) => self

