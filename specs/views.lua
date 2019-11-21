-- The .view is the official extension for fa template files
-- There is a lot of work to do with the views system!!
-- Especially the links between all properties
-- Every view property should correspond to two actual properties :
--	- the current value
--	- the getter (which can be inlined in C++)
-- whenever an update is necessary, ALL getters are called
-- if a value differ from the getter value, it's changed back to its initial value

-- Un exemple d'un fichier Calendar.view :
display the_class_to_display
use Table, Row
extends View  -- unecessary because we are inside a .view file

width = Layout.fill
height = 100%  -- '%' is an operator. Its transformed into (parent.height * 1)

- Table
	- Row for row in rows
		[width] = parent.width - 30
		[height] = 60
		data = 321321  -- we differentiate 'properties' (from model) and 'attributes' (from view)
		options:
			configurable = false
		hover = false
		.hover
			width = 23132 

		- x
		~ {x}
		~ Salut toi !
		~ Comment ça va ?
