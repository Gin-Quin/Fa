import designSystem.css/themes
import style.css/stylish

export type =
	...View
	
	theme: Key<themes> = "light"

	toggleTheme() ->
		theme = theme == "light" ? "dark" : "light"

	- Block:
		- Button:
			on.click = toggleTheme
			- "Theme: {theme}"
	- Block:
		class = themes[theme]
		- Button:
			class = stylish
			- "Hello"
