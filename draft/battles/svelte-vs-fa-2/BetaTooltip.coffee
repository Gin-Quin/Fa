import /ui/kit/
	Countdown.fa
	Tooltip.fa
import /ui/locale/locales.fa/defineContent
import betaDate

# FUNCTION STYLE

export main() ->
	let content = defineContent:
		en =
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		fr =
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"
	
	let body = view:
		style.padding = 4rem
		- text(content.countdown)

	let countdown = view:
		style.padding = [8rem, 0]
		- countdown(to = betaDate, variant = "small")

	return tooltip(color = "secondary"):
		- body
		- countdown
		- text(content.wip)

# TYPE STYLE
export main type extends Tooltip ->
	let content = defineContent:
		en =
			countdown =  "Talers is available in"
			wip =  "Sign up to be one of the first to try Talers!"
		fr =
			countdown = "Talers est disponible dans"
			wip =  "Inscris-toi pour être un des premiers à essayer Talers&nbsp;!"

	let body = View:
		style.padding = 4rem
		- Text(content.countdown)

	let countdown = View:
		style.padding = [8rem, 0]
		- Countdown(to = betaDate, variant = "small")	

	return
		color = "secondary"
		- body
		- countdown
		- View:
			- Text(content.wip)

# FUNCTION STYLE (brackets)
export main(): View {
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

	let body = View(
		style.padding = 4rem
		- Text(content.countdown)
	)

	let countdown = View(
		style.padding = [8rem, 0]
		- Countdown(to = betaDate, variant = "small")
	)

	return Tooltip(
		color = "secondary"
		- body
		- countdown
		- View(
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

	let body = View {
		style.padding = 4rem
		- Text(content.countdown)
	}

	let countdown = View {
		style.padding = [8rem, 0]
		- Countdown { to = betaDate, variant = "small" }
	}

	children = {
		- body
		- countdown
		- View {
			- Text(content.wip)
		}
	}
}
