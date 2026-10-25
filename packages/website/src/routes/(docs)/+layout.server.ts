import { getNavigation } from "#lib/docs/pages.server.ts";

import type { LayoutServerLoad } from "./$types";

export const load: LayoutServerLoad = () => ({ navigation: getNavigation() });
