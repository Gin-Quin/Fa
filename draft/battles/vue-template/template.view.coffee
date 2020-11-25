is View<Template>



View circled
	borderRadius = 100%
	onClick -> print "Circle clicked!"


View bigger
	width = 84
	height = width

View black
	color = BLACK



> Table
	cursor = default
	
	> Header
		> Module "Module souscrit"

		> List for key in columns
			onHover -> width = 500
			onClick -> width = 800

			if key == 'selected'
				> Checkbox
					...circled  if key == 'names'
					...bigger  if key != 'zototo'
					...black
					value = selectAll
			else
				> key
