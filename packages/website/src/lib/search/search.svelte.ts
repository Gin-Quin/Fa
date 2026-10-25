/** A page of the documentation, as stored in the search index (`/search.json`). */
export type SearchPage = {
	slug: string;
	title: string;
	section: string;
	sections: Array<{ id: string; heading: string; text: string }>;
};

export type SearchResult = {
	href: string;
	title: string;
	section: string;
	heading: string;
	snippet: Array<{ text: string; match: boolean }>;
	score: number;
};

const SNIPPET_RADIUS = 70;
const MAX_RESULTS = 12;

class SearchState {
	open = $state(false);
	#index: Promise<SearchPage[]> | undefined;

	show() {
		this.open = true;
	}

	hide() {
		this.open = false;
	}

	/** Loads the index the first time the search is opened. */
	load() {
		this.#index ??= fetch("/search.json").then((response) => response.json());
		return this.#index;
	}
}

export const search = new SearchState();

function escapeRegExp(text: string) {
	return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Cuts the text around the first match, and marks every match. */
function makeSnippet(text: string, terms: string[]): SearchResult["snippet"] {
	const lower = text.toLowerCase();
	const first = Math.min(
		...terms.map((term) => lower.indexOf(term)).filter((index) => index >= 0),
		text.length,
	);
	const start = Math.max(0, first - SNIPPET_RADIUS);
	const end = Math.min(text.length, first + SNIPPET_RADIUS * 2);
	const excerpt = `${start > 0 ? "…" : ""}${text.slice(start, end)}${end < text.length ? "…" : ""}`;

	const pattern = new RegExp(`(${terms.map(escapeRegExp).join("|")})`, "gi");
	return excerpt
		.split(pattern)
		.filter(Boolean)
		.map((part) => ({ text: part, match: terms.includes(part.toLowerCase()) }));
}

export function query(pages: SearchPage[], input: string): SearchResult[] {
	const terms = input.toLowerCase().split(/\s+/).filter(Boolean);
	if (terms.length === 0) {
		return [];
	}

	const results: SearchResult[] = [];
	for (const page of pages) {
		const title = page.title.toLowerCase();
		for (const section of page.sections) {
			const heading = section.heading.toLowerCase();
			const text = section.text.toLowerCase();
			const haystack = `${title} ${heading} ${text}`;
			if (!terms.every((term) => haystack.includes(term))) {
				continue;
			}

			let score = 0;
			for (const term of terms) {
				if (title.includes(term)) score += section.id ? 4 : 12;
				if (heading.includes(term)) score += 8;
				if (title.startsWith(term) || heading.startsWith(term)) score += 3;
				if (text.includes(term)) score += 1;
			}

			results.push({
				href: section.id ? `/${page.slug}#${section.id}` : `/${page.slug}`,
				title: page.title,
				section: page.section,
				heading: section.heading,
				snippet: makeSnippet(section.text, terms),
				score,
			});
		}
	}

	return results.sort((a, b) => b.score - a.score).slice(0, MAX_RESULTS);
}
