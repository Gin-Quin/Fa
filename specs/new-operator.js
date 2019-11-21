
// the new operator call the constructor of the given variable

let zabu = Hero:
	name = "Zabu"
	strength = 985

new zabu:
	name = "Zabu II"
	strength = 986
// .. is similar to :
zabu = Hero:
	name = "Zabu II"
	strength = 986
// but we don't have to write the type again

// it's especially useful for structures with anonymous types :
let options:
	shouldUpdate = false
	autoRender = true

new options:
	shouldUpdate = true