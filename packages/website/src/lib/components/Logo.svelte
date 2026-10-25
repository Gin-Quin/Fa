<script lang="ts">
	let { size = 28, withName = true }: { size?: number; withName?: boolean } = $props();
</script>

<span class="logo">
	<img src="/fa_icon_64x64.webp" alt="" width={size} height={size} />
	{#if withName}
		<span class="name">Fa</span>
	{/if}
</span>

<style>
	.logo {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}

	img {
		filter: drop-shadow(0 4px 12px var(--glow-pink));
	}

	.name {
		font-family: var(--font-display);
		font-size: var(--text-lg);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		color: var(--color-text);
	}
</style>
