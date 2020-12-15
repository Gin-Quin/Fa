
/**
* Transform and feed a template with data
* Usage :
* 	const transform = require('./utils/transform')
* 	transform(template)
*/

const generateRules = require('./rules/generate')
const rules = require('./rules/namespace')
const data = require('./data')
const generate = require('./rules/generate')

module.exports = (template) =>
   template.indexOf('${') == -1
   ? template
   : Function(`{${Object.keys(data).join()}}, rules, generateRules`, 'return `'+ template +'`')(data, rules, generateRules)
