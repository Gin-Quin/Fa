<script lang="ts">
	/** Renders a text where `backticks` mark inline code. */
	let { text }: { text: string } = $props();

	const parts = $derived(text.split("`"));
</script>

{#each parts as part, index (index)}{#if index % 2 === 1}<code>{part}</code
		>{:else}{part}{/if}{/each}
