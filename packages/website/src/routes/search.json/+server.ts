import { json } from "@sveltejs/kit";

import { getAllPages } from "#lib/docs/pages.server.ts";
import type { SearchPage } from "#lib/search/search.svelte.ts";

import type { RequestHandler } from "./$types";

export const prerender = true;

export const GET: RequestHandler = async () => {
	const pages = await getAllPages();
	return json(
		pages.map((page): SearchPage => ({
			slug: page.slug,
			title: page.title,
			section: page.section,
			sections: page.sections,
		})),
	);
};
