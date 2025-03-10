import * as fs from "node:fs"
import * as path from "node:path"

/**
 * Generates test files in a directory structure
 * @param baseDir - Base directory where files will be created
 * @param subdirsPerDir - Number of subdirectories to create in each directory
 * @param filesPerDir - Number of files to create in each directory
 * @param depth - Maximum depth of directory nesting
 * @param currentDepth - Current depth (used for recursion)
 */
function generateTestFiles(
	baseDir: string,
	subdirsPerDir: number,
	filesPerDir: number,
	depth: number,
	currentDepth = 0
): void {
	// Create the base directory if it doesn't exist
	if (!fs.existsSync(baseDir)) {
		fs.mkdirSync(baseDir, { recursive: true })
	}

	// Create files in the current directory
	for (let i = 0; i < filesPerDir; i++) {
		const filePath = path.join(baseDir, `file-${i}.fa`)
		fs.writeFileSync(filePath, "") // Create empty file
		console.log(`Created file: ${filePath}`)
	}

	// Stop recursion if we've reached the maximum depth
	if (currentDepth >= depth) {
		return
	}

	// Create subdirectories and recurse
	for (let i = 0; i < subdirsPerDir; i++) {
		const subdirPath = path.join(baseDir, `subdir-${i}`)
		generateTestFiles(subdirPath, subdirsPerDir, filesPerDir, depth, currentDepth + 1)
	}
}

// Parse command line arguments
const args = process.argv.slice(2)
const subdirsPerDir = Number.parseInt(args[0] || "2", 10)
const filesPerDir = Number.parseInt(args[1] || "3", 10)
const depth = Number.parseInt(args[2] || "2", 10)

// Define the base directory
const baseDir = path.join(__dirname, "..", "src", "tests", "data")

console.log("Generating test files with the following parameters:")
console.log(`- Base directory: ${baseDir}`)
console.log(`- Subdirectories per directory: ${subdirsPerDir}`)
console.log(`- Files per directory: ${filesPerDir}`)
console.log(`- Maximum depth: ${depth}`)

// Remove all files in the base directory
for (const file of fs.readdirSync(baseDir)) {
	fs.rmSync(path.join(baseDir, file), { recursive: true, force: true })
}

// Generate the test files
generateTestFiles(baseDir, subdirsPerDir, filesPerDir, depth)

console.log("Test file generation complete!")
