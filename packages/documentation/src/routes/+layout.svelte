<script lang="ts">
	import { colors } from "../colors";
	import "./global.css";

	const { children } = $props();
</script>

<!-- Leave this. Or you can add more content for your custom layout -->
<div
	class="theme"
	style:display="contents"
	style:--base-purpleLight={colors.purpleLight}
	style:--base-orangeDark={colors.orangeDark}
	style:--base-orangeLight={colors.orangeLight}
	style:--base-redDark={colors.redDark}
	style:--base-redLight={colors.redLight}
	style:--base-greenDark={colors.greenDark}
	style:--base-greenLight={colors.greenLight}
	style:--base-gray={colors.gray}
	style:--base-lightGray={colors.lightGray}
>
	{@render children?.()}
</div>

<style>
	:global(html.dark) .theme {
		--purpleLight: var(--base-purpleLight);
		--orangeDark: var(--base-orangeDark);
		--orangeLight: var(--base-orangeLight);
		--redDark: var(--base-redDark);
		--redLight: var(--base-redLight);
		--green: var(--base-greenDark);
		--gray: var(--base-lightGray);
		--lightGray: var(--base-gray);
		--white: black;
		--black: white;
	}

	:global(html:not(.dark)) .theme {
		--purpleLight: var(--base-purpleLight);
		--orangeDark: var(--base-orangeDark);
		--orangeLight: var(--base-orangeLight);
		--redDark: var(--base-redDark);
		--redLight: var(--base-redDark);
		--green: var(--base-greenLight);
		--gray: var(--base-gray);
		--lightGray: var(--base-lightGray);
		--gray: var(--base-gray);
		--white: white;
		--black: black;
	}
</style>
