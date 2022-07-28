
import print from 'cute-print'
import fs from 'fs'
import path from 'path'
import Module from './Module.js'



function generate(input, output, namespace='') {
	if (!input) throw "Missing input"
	if (!output) throw "Missing output"
	input = path.resolve(input)
	output = path.resolve(output)

	let main = new Module(input)
	let outputContent = main.bundle(namespace)
	fs.writeFileSync(output, outputContent)

	print `[blue:>] [bold]${output}`
}

export default generate
