<script lang="ts">
	import "#lib/landing/landing.css";

	import Footer from "#lib/components/Footer.svelte";
	import Header from "#lib/components/Header.svelte";
	import AgentLoop from "#lib/landing/AgentLoop.svelte";
	import CallToAction from "#lib/landing/CallToAction.svelte";
	import ComingFrom from "#lib/landing/ComingFrom.svelte";
	import FeatureGrid from "#lib/landing/FeatureGrid.svelte";
	import Hero from "#lib/landing/Hero.svelte";
	import MemorySection from "#lib/landing/MemorySection.svelte";
	import SignatureAnatomy from "#lib/landing/SignatureAnatomy.svelte";
	import Targets from "#lib/landing/Targets.svelte";

	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Fa: the programming language for AI agents and humans</title>
	<meta
		name="description"
		content="Fa is a programming language for AI agents and humans. Typed errors, requirements and mutations in every signature, memory safety without a garbage collector or a borrow checker, and one language for the web, native and WebAssembly."
	/>
	<meta property="og:title" content="Fa: if it compiles, it works" />
	<meta property="og:image" content="/fa_main_icon_512x512.webp" />
</svelte:head>

<Header />

<main>
	<Hero files={data.hero} />
	<SignatureAnatomy {...data.signature} />
	<FeatureGrid code={data.features} />
	<MemorySection {...data.memory} />
	<AgentLoop {...data.agentLoop} />
	<Targets />
	<ComingFrom />
	<CallToAction />
</main>

<Footer />
