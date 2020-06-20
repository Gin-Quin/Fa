
const fs = require('fs')
const { join } = require('path').posix
const tokens = require('./tokens')
const print = require('cute-print')

const data = { tokens }

const transform = (function({tokens}, template) {
	return (eval('`'+ template +'`'))
}).bind(null, data)
 

for (let file of fs.readdirSync('templates')) {
	let template = fs.readFileSync(join('templates', file), 'utf8')
	let index = template.indexOf('\n')
	let firstLine = template.substr(0, index)
	template = template.substr(index + 1)

	let destination = firstLine.match(/^#destination "(.*)"$/)
	if (!destination) throw `The template file '${file}' has no destination`
	destination = join(destination[1], file)

	let content = transform(template)

	fs.writeFileSync(destination, content)
	print `[white.italic]Generated [reset.bold] ${destination}`
}
