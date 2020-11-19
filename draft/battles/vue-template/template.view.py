display Template

- Table
	cursor = default
	
	- Header
		- Module "Module souscris"

		- List  for key in columns
			onHover
				width = 500
			onClick
				width = 800

			if key == 'selected'
				- Checkbox
					... circled  if key == 'names'
					... bigger  if key != 'zototo'
					... black
					value = selectAll
			else
				- key



static circled = View:
	borderRadius = 100%
	onClick
		print "Circle clicked!"


static bigger = View:
	width = 84
	height = width

static black = View:
	color = BLACK

