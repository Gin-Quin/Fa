const fs = require('fs')
const tokens = require('./tokens')

const output = ''

for (let token of tokens)
	output += `\t\t`

fs.writeFileSync('visiters.cpp', output)