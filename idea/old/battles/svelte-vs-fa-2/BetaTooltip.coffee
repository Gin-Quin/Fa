import /ui/kit/
	Countdown.fa
	Tooltip.fa
import /ui/locale/locales.fa/defineContent
import betaDate

# 1. FUNCTION STYLE
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





# 2. TYPE STYLE
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


# 3. FUNCTION STYLE (brackets)
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

# 3. FUNCTION STYLE (brackets + Swift Style)
export (): Tooltip => {
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

	let body = Box(style.padding = 4rem) {
		Text(content.countdown)
	}

	let countdown = Box(
		style.padding = [8rem, 0]
	) {
		Countdown(to = betaDate, variant = "small")
	}

	return Tooltip(
		color = "secondary"
	) {
		body
		countdown
		Box {
			Text(content.wip)
		}
	}
}

# 3. FUNCTION STYLE (brackets + Separator)
export (): Tooltip => {
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
		--
		Text(content.countdown)
	)

	let countdown = Box(
		style.padding = [8rem, 0]
		--
		Countdown(to = betaDate, variant = "small")
	)

	return Tooltip(
		color = "secondary"
		--
		body
		countdown
		Box(-- Text(content.wip))
	)
}


# 4. STYLE (brackets)
export type = {
	...Tooltip
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
