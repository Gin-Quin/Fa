
/**
* Transform and feed a template with data
* Usage :
* 	import transform from './utils/transform.js'
* 	transform(template)
*/

import generateRules from './rules/generate.js'
import rules from './rules/namespace.js'
import data from './data.js'
import generate from './rules/generate.js'

export default (template) =>
   template.indexOf('${') == -1
   ? template
   : Function(`{${Object.keys(data).join()}}, rules, generateRules`, 'return `'+ template +'`')(data, rules, generateRules)
