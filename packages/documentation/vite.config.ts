import { defaultTheme } from "@sveltepress/theme-default"
import { sveltepress } from "@sveltepress/vite"
import { defineConfig } from "vite"
import { readFileSync } from "node:fs"
import yaml from "js-yaml"
import type { LinkItem } from "virtual:sveltepress/theme-default"

const navbar = yaml.load(readFileSync("./src/navbar.yaml", "utf-8"))
const sidebar = yaml.load(readFileSync("./src/sidebar.yaml", "utf-8"))

const config = defineConfig({
	server: {
		port: 8000,
	},
	plugins: [
		sveltepress({
			theme: defaultTheme({
				navbar: navbar as LinkItem[],
				sidebar: sidebar as Record<string, LinkItem[]>,
				github: "https://github.com/Blackman99/sveltepress",
				logo: "/sveltepress.svg",
			}),
			siteConfig: {
				title: "Fa",
				description: "A programming language for the future.",
			},
		}),
	],
})

export default config
