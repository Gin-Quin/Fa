
const YAML = require('js-yaml')
const fs = require('fs')

const yaml = (file) => YAML.safeLoad(fs.readFileSync(`data/${file}.yaml`))
const arrayize = (object, property='token') => {
	let array = Object.keys(object).map(key => ({key, token: object[key]}))
	return Object.assign(array, object)
}

const data = {
	tokens: yaml('tokens'),
	symbols: arrayize(yaml('symbols')),
	keywords: arrayize(yaml('keywords')),
	// rules: yaml('rules'),
}

// console.log("keywords", data.keywords)
// console.log("symbols", data.symbols)

module.exports = data
