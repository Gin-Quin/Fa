
import YAML from 'js-yaml'
import fs from 'fs'
import version from './version.js'

// import parseRules from './rules/parse.js'

const yaml = (file) => YAML.safeLoad(fs.readFileSync(`grammar/${file}.yaml`))
const open = (file) => fs.readFileSync(`grammar/${file}`, 'utf8')

const arrayize = (object, property = 'token') => {
	let array = Object.keys(object).map(key => ({key, token: object[key]}))
	return Object.assign(array, object)
}

// we load base data
const data = {
	tokens: yaml('tokens'),
	symbols: arrayize(yaml('symbols')),
	keywords: arrayize(yaml('keywords')),
	nodes: yaml('nodes'),
	...version,
}

// console.log("data", data)


// we load additional data
// Object.assign(data, {
// 	rules: parseRules(data, open('rules.yaml.ex')),
// })

// console.log("keywords", data.keywords)
// console.log("symbols", data.symbols)

export default data
