display Template

- Table
	cursor = default
	
	- Header
		- Module "Module souscrit"

		- List  for key in columns
			onHover
				width = 500
			onClick
				width = 800

			if key == 'selected'
				- Checkbox
					<< circled  if key == 'names'
					<< bigger  if key != 'zototo'
					<< black
					value = selectAll
			else
				- key



const circled = View:
	borderRadius = 100%
	onClick
		print "Circle clicked!"


const bigger = View:
	width = 84
	height = width

const black = View:
	color = BLACK

