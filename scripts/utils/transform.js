
/**
 * Transoform and feed a template with data
 * Usage :
 * 	const transform = require('./utils/transform')
 * 	transform(template)
 */
module.exports = (function transform({tokens, rules}, template) {
	return template.indexOf('${') == -1 ? template : (eval('`'+ template +'`'))
}).bind(null, require('./data'))
