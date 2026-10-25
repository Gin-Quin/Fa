<script lang="ts">
	import { theme } from "#lib/theme.svelte.ts";

	import Icon from "./Icon.svelte";
</script>

<button
	class="theme-toggle"
	type="button"
	onclick={() => theme.toggle()}
	aria-label={theme.current === "dark" ? "Switch to the light theme" : "Switch to the dark theme"}
	title={theme.current === "dark" ? "Light theme" : "Dark theme"}
>
	<span class="icon" class:visible={theme.current === "dark"}><Icon name="sun" /></span>
	<span class="icon" class:visible={theme.current === "light"}><Icon name="moon" /></span>
</button>

<style>
	.theme-toggle {
		position: relative;
		display: grid;
		place-items: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-full);
		color: var(--color-text-muted);
		transition:
			color var(--duration-fast),
			background-color var(--duration-fast);
	}

	.theme-toggle:hover {
		color: var(--color-text);
		background: var(--color-surface-hover);
	}

	.icon {
		position: absolute;
		display: grid;
		opacity: 0;
		transform: rotate(-60deg) scale(0.6);
		transition:
			opacity var(--duration-normal) var(--ease-out),
			transform var(--duration-normal) var(--ease-out);
	}

	.icon.visible {
		opacity: 1;
		transform: none;
	}
</style>
