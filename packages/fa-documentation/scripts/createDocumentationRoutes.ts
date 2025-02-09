import {
	mkdirSync,
	readdirSync,
	readFileSync,
	rmSync,
	statSync,
	writeFileSync,
} from "node:fs"
import { basename, join } from "node:path"

createDocumentationRoutes(
	join(__dirname, "..", "..", "..", "documentation"),
	join(__dirname, "..", "src", "routes")
)

/**
 * Create documentation routes from a directory of markdown files.
 *
 * @param inputDirectory - The directory containing the markdown files.
 * @param outputDirectory - The directory to write the routes to in SvelteKit format.
 */
export function createDocumentationRoutes(
	inputDirectory: string,
	outputDirectory: string
) {
	// 1. Clean the output directory
	for (const file of readdirSync(outputDirectory)) {
		if (statSync(join(outputDirectory, file)).isDirectory()) {
			rmSync(join(outputDirectory, file), { recursive: true, force: true })
		}
	}

	// 2. Read the input directory and write the routes to the output directory
	const files = readdirSync(inputDirectory)

	for (const file of files) {
		const filePath = join(inputDirectory, file)

		if (statSync(filePath).isDirectory()) {
			createDocumentationRoutes(filePath, join(outputDirectory, file))
		} else if (file.endsWith(".md") && file !== "README.md") {
			mkdirSync(outputDirectory, { recursive: true })

			const fileContent = readFileSync(filePath, "utf-8")
			const outputFilePath = join(outputDirectory, basename(file), "+page.md")
			writeFileSync(outputFilePath, fileContent)
		}
	}
}
