
import namespace from './namespace.js'
import parse from './parse.js'

parse()
// console.log("namespace:", namespace)

const validateCalls = /<<<(.+?)>>>\(((_\d)(?:,.*?)?)\)/g

const generate = () => {
   let result = ''

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

export default generate
// console.log(generate())
