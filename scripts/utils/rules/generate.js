const namespace = require('./namespace')
const parse = require('./parse')

parse()
// console.log("namespace:", namespace)

const validateCalls = /<<<(.+?)>>>\(((_\d)(?:,.*?)?)\)/g

const generate = () => {
   result = ''

   for (let name in namespace)
      result += namespace[name].declaration + '\n\t'

   result += '\n'

   for (let name in namespace)
      result += namespace[name].code

   result = result.replaceAll(validateCalls,
      (match, name, args, firstArgument) => {
         return namespace[name] ?
            `Validate::${name}(${args})`
         :  `Validate::Any(${firstArgument})`
      }
   )

   return result
}

module.exports = generate
// console.log(generate())
