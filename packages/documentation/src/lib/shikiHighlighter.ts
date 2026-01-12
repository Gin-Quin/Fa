import {
	createHighlighter,
	type Highlighter,
	type ThemeRegistrationAny,
} from "shiki";
import andromeeda from "@shikijs/themes/andromeeda";
import catppuccinLatte from "@shikijs/themes/catppuccin-latte";
import faGrammar from "../shiki/fa.tmLanguage.json";

const themeDark = andromeeda;
const themeLight = catppuccinLatte;

let highlighterPromise: Promise<Highlighter> | null = null;

export function getHighlighter(): Promise<Highlighter> {
	if (!highlighterPromise) {
		highlighterPromise = createHighlighter({
			themes: [themeDark, themeLight],
			langs: [faGrammar],
		});
	}

	return highlighterPromise;
}

export const shikiThemes: {
	dark: ThemeRegistrationAny;
	light: ThemeRegistrationAny;
} = {
	dark: themeDark,
	light: themeLight,
};
