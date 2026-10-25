import { error } from "@sveltejs/kit";

import { getNeighbours, getPage, PAGE_SLUGS } from "#lib/docs/pages.server.ts";

import type { EntryGenerator, PageServerLoad } from "./$types";

export const entries: EntryGenerator = () =>
	PAGE_SLUGS.map((slug) => {
		const [section, page] = slug.split("/");
		return { section, page };
	});

export const load: PageServerLoad = async ({ params }) => {
	const slug = `${params.section}/${params.page}`;
	const page = await getPage(slug);
	if (!page) {
		error(404, "This page does not exist");
	}
	const { sections: _, ...content } = page;
	return { page: content, ...getNeighbours(slug) };
};
