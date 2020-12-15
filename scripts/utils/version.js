
const fs = require('fs')
const package = JSON.parse(fs.readFileSync('./package.json', 'utf8'))
let [MAJOR, MINOR, PATCH] = package.version.split('.')
// MAJOR = +MAJOR
// MINOR = +MINOR
// PATCH = +PATCH
// package.version = `${MAJOR}.${MINOR}.${PATCH}`
// fs.writeFileSync('./package.json', JSON.stringify(package, null, '\t'))

module.exports = {
	MAJOR,
	MINOR,
	PATCH,
}
