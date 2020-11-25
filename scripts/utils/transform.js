
/**
* Transform and feed a template with data
* Usage :
* 	const transform = require('./utils/transform')
* 	transform(template)
*/

const generateRules = require('./rules/generate')
const rules = require('./rules/namespace')
const {
   tokens,
   symbols,
   keywords,
   nodes,
} = require('./data')

module.exports = (template) =>
   template.indexOf('${') == -1
   ? template
   : eval('`'+ template +'`')
