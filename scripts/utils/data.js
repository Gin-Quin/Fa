
const YAML = require('js-yaml')
const fs = require('fs')

module.exports = {
	tokens: YAML.safeLoad(fs.readFileSync('data/tokens.yaml')),
	rules: YAML.safeLoad(fs.readFileSync('data/rules.yaml')),
}