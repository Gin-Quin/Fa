import designSystem.css/themes
import style.css/stylish

export type is View =
	theme: Key<themes> = "light"
	toggleButton = Button:
		on.click = toggleTheme
		- "Theme: {theme}"

	toggleTheme() ->
		theme = if theme == "light" then "dark" else "light"

	children =
		- Block:
			- toggleButton
		- Block:
			class = themes[theme]
			- Button:
				class = stylish
				- "Hello"
