import atoms/Svelect

type Item =
	value: any
	text?: string

export default type extends View =
	label = ""
	name: string
	value: null | unknown = null
	placeholder = "Select an option..."
	items: Item[]

	- Block(class = "form-input"):
		- Label(for = name, value)
		- Svelect(name, value, placeholder, items):
			- Block(slot = "item"):
				- Slot(name = "item", item):
					- item.text || item.value
