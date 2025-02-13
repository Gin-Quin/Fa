<script lang="ts">
	import { themes } from "./designSystem.css"
	import { stylish } from "./style.css"

	let theme: keyof typeof themes = "light"

	function toggleTheme() {
		theme = theme == "light" ? "dark" : "light"
	}
</script>

<div>
	<button on:click={toggleTheme}>Theme: {theme}</button>
</div>

<div class={themes[theme]}>
	<button class={stylish}>Hello</button>
</div>
