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

# TYPE STYLE
export type extends Tooltip ->
	let content = defineContent:
		en =
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		fr =
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"

	return
		color = "secondary"
		- Box:
			style.padding = 4rem
			- Text(content.countdown)
		- Box:
			style.padding = [8rem, 0]
			- Countdown(to = betaDate, variant = "small")	
		- Text(content.wip)

# FUNCTION STYLE (brackets)
export (): Tooltip {
	let content = defineContent({
		en = {
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		}
		fr = {
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"
		}
	})

	let body = Box(
		style.padding = 4rem
		- Text(content.countdown)
	)

	let countdown = Box(
		style.padding = [8rem, 0]
		- Countdown(to = betaDate, variant = "small")
	)

	return Tooltip(
		color = "secondary"
		- body
		- countdown
		- Box(
			- Text(content.wip)
		)
	)
}

# TYPE STYLE (brackets)
export main type extends Tooltip {
	color = 'secondary'

	let content = defineContent({
		en = {
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		}
		fr = {
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"
		}
	})

	children = {
		- Box {
			style.padding = 4rem
			- Text(content.countdown)
		}
		- Box {
			style.padding = [8rem, 0]
			- Countdown { to = betaDate, variant = "small" }
		}
		- Box {
			- Text(content.wip)
		}
	}
}
