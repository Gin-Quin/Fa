
const YAML = require('js-yaml')
const fs = require('fs')
const parseRules = require('./rules/parse')

const yaml = (file) => YAML.safeLoad(fs.readFileSync(`data/${file}.yaml`))
const open = (file) => fs.readFileSync(`data/${file}`, 'utf8')

const arrayize = (object, property = 'token') => {
	let array = Object.keys(object).map(key => ({key, token: object[key]}))
	return Object.assign(array, object)
}

// we load base data
const data = {
	tokens: yaml('tokens'),
	symbols: arrayize(yaml('symbols')),
	keywords: arrayize(yaml('keywords')),
	nodes: arrayize(yaml('nodes')),
}


// we load additional data
Object.assign(data, {
	rules: parseRules(data, open('rules.yaml.ex')),
})


if (require.main === module) {
	let { argv } = process
	let keys = argv[2] ? argv.slice(2) : Object.keys(data)
	for (let key of keys)
		console.log(key, data[key])
}

// console.log("keywords", data.keywords)
// console.log("symbols", data.symbols)

module.exports = data
