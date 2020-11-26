<script context="module">
	export async function preload({ params }) {
		const res = await this.fetch(`docs/${params.page}.json`)
		const data = await res.json()

		if (res.status === 200) {
			return { page: data }
		} else {
			this.error(res.status, data.message)
		}
	}
</script>

<script>
	export let page
</script>

<style>
</style>

<svelte:head>
	<title>{page.title}</title>
</svelte:head>

<h1>{page.title}</h1>

<div class="content">
	{@html page.html}
</div>
