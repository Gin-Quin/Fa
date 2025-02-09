<script lang="ts">
	import Countdown from "ui/kit/Countdown.svelte"
	import Tooltip from "ui/kit/Tooltip.svelte"
	import { defineContent } from "ui/locale/locales"
	import { betaDate } from "./betaDate"

	const content = defineContent({
		en: {
			countdown: "Talers is available in",
			wip: "Sign up to be one of the first to try Talers!",
		},
		fr: {
			countdown: "Talers est disponible dans",
			wip: "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!",
		},
	})
</script>

<Tooltip color="secondary">
	<div class="content">
		<div>
			{@html content.countdown}
		</div>
		<div class="countdown">
			<Countdown to={betaDate} variant="small" />
		</div>
		<div>
			{@html content.wip}
		</div>
	</div>
</Tooltip>

<style lang="stylus">
	@import "ui/definitions"

	.content
		padding: 4rem
	
	.countdown
		padding: 8rem 0
</style>
