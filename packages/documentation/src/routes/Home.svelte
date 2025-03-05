<script lang="ts">
	import { colors } from "../colors"
	import EnterTheMatrix from "./EnterTheMatrix.svelte"
	import FancyButton from "./FancyButton.svelte"
</script>

<column class="Home" style:gap="3rem">
	<row class="intro" style:justify-content="space-between" style:gap="5rem">
		<row class="start" style:gap="1rem">
			<img src="/fa_main_icon_500x500.webp" alt="FA Logo" width="220" />
			<div class="description">
				<div class="title">Fa</div>
				<div class="subtitle">The <b>universal</b><br />programming language</div>
			</div>
		</row>

		<div class="end">
			<FancyButton href="/getting-started/introduction">Get Started</FancyButton>
		</div>
	</row>

	<div class="tagline">
		The Future of Web, Native, and Wasm development in one language.
	</div>

	<div style:padding="2rem 0">
		<EnterTheMatrix />
	</div>
</column>

<style>
	.Home {
		background-color: colors.purpleDark;
	}

	.intro {
		width: 800px;
		max-width: 100%;
		padding-top: 4rem;
		margin: auto;
	}

	.title {
		font-size: 3.75rem;
		font-family: "Playwrite IT Moderna", cursive;
		font-optical-sizing: auto;
		letter-spacing: 0.02em;
		font-style: normal;
		background: linear-gradient(45deg, #d900d5, #ff5943, #ed8600);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		font-size: 1.5rem;
	}

	.tagline {
		font-family: "Monomakh", monospace;
		font-size: 1.65rem;
		text-align: center;
		margin-top: 2rem;
		/* color: #000; */
	}
</style>
