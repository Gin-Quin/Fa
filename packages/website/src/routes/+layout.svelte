<script lang="ts">
	import "#lib/styles/base.css";

	import type { Snippet } from "svelte";
	import { onMount } from "svelte";

	import Search from "#lib/search/Search.svelte";
	import { theme } from "#lib/theme.svelte.ts";

	let { children }: { children: Snippet } = $props();

	onMount(() => theme.initialize());
</script>

{@render children()}

<Search />
