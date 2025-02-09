

# A type defined using colons:
type Coco = 
	name: String? = String()

	...if x is 12
		- call(babe)
		- call(daddy)
	...else if z is 321	
		- call(mommy)


# an anonymous object:
let object =
	name = 12
	coco = 123
	fruits: [...Fruit] =
		-	name = "banana"
			energy = 5312
		-	name = "orange"
			energy = 321

