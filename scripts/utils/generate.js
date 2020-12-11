
const print = require('cute-print')
const fs = require('fs')
const path = require('path')
const Module = require('./Module')



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

if (require.main === module)
	generate(...process.argv.slice(2))

module.exports = generate