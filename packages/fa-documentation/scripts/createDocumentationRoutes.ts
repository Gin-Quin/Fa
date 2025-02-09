import {
	existsSync,
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
	// 1. Read the output directory and delete all the files that do not exist in the input directory
	for (const file of readdirSync(outputDirectory)) {
		const outputFilePath = join(outputDirectory, file)

		if (statSync(outputFilePath).isDirectory()) {
			const inputFilePath = join(inputDirectory, `${file}.md`)
			if (!existsSync(inputFilePath)) {
				rmSync(outputFilePath, { recursive: true, force: true })
			}
		}
	}

	// 2. Read the input directory and write the routes to the output directory
	const files = readdirSync(inputDirectory)

	for (const file of files) {
		const filePath = join(inputDirectory, file)

		if (statSync(filePath).isDirectory()) {
			createDocumentationRoutes(filePath, join(outputDirectory, file))
		} else if (file.endsWith(".md") && file !== "README.md") {
			const fileContent = readFileSync(filePath, "utf-8")
			const outputFilePath = join(
				outputDirectory,
				basename(file).split(".")[0],
				"+page.md"
			)

			mkdirSync(join(outputFilePath, ".."), { recursive: true })
			writeFileSync(outputFilePath, fileContent)
		}
	}
}
