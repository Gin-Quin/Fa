import /ui/kit/
	Countdown.fa
	Tooltip.fa
import /ui/locale/locales.fa/defineContent
import betaDate

# FUNCTION STYLE
export () ->
	let content = defineContent:
		en =
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		fr =
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"
	
	return tooltip(color = "secondary"):
		- text(content.wip)
		- Box:
			style.padding = 4rem
			- text(content.countdown)
		- text(content.wip)
		- Box:
			style.padding = [8rem, 0]
			- countdown(to = betaDate, variant = "small")
