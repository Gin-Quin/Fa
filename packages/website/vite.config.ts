import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
	server: {
		port: 8000,
		fs: {
			// the Markdown pages live in the `documentation/` folder at the root of the repository
			allow: ["../../documentation"],
		},
	},
	plugins: [
		sveltekit({
			adapter: adapter(),
			compilerOptions: {
				runes: true,
			},
			prerender: {
				handleHttpError: "fail",
				handleMissingId: "fail",
			},
		}),
	],
});
