
/**
* Transform and feed a template with data
* Usage :
* 	const transform = require('./utils/transform')
* 	transform(template)
*/

const {
   tokens,
   rules,
   symbols,
   keywords,
} = require('./data')

module.exports = (template) =>
   template.indexOf('${') == -1
   ? template
   : eval('`'+ template +'`')
