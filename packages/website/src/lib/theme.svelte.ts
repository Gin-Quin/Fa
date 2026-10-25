export type Theme = "light" | "dark";

/** The current color theme, applied with `data-theme` on the <html> element. */
class ThemeState {
	current = $state<Theme>("dark");

	/** Reads the theme applied by the script of app.html, before the first paint. */
	initialize() {
		this.current = document.documentElement.dataset.theme === "light" ? "light" : "dark";
	}

	toggle() {
		this.current = this.current === "dark" ? "light" : "dark";
		document.documentElement.dataset.theme = this.current;
		try {
			localStorage.setItem("theme", this.current);
		} catch {
			// the preference is not remembered when the storage is unavailable
		}
	}
}

export const theme = new ThemeState();
