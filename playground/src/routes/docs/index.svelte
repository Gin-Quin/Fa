<script context="module">
	export function preload() {
		return this.fetch(`docs.json`).then(r => r.json()).then(pages => {
			return { pages };
		});
	}
</script>

<script>
	export let pages;
</script>

<style>
	ul {
		margin: 0 0 1em 0;
		line-height: 1.5;
	}
</style>

<svelte:head>
	<title>Blog</title>
</svelte:head>

<ul>
	{#each pages as page}
		<li>
			<a href="docs/{page.title}">{page.title}</a>
		</li>
	{/each}
</ul>
