
import fs from 'fs'
import data from '../data.js'
import getChildrenCount from '../getChildrenCount.js'
import Stack from '../Stack.js'
import namespace from './namespace.js'

const rulesContent = fs.readFileSync(`grammar/rules.ex`, 'utf8')
const lines = rulesContent.split('\n')

// separate a string in two parts
const separate = (str, separator) => {
   let index = str.indexOf(char)
   if (index == -1)
		return [str.trim(), '']
	return [str.slice(0, index).trim(), str.slice(index + 1).trim()]
}


const matchCase = /^\s*case\s+(?:(?::(\w+))|(?:(\w+)(?:::(\w+))?))(?:\((.*)\))?\s*(?:=>\s*(\w+))?\s*$/
const matchVariable = /^\s*:(\w+)\s*=\s*(.*)?\s*$/
const matchThrow = /^\s*#\s*(.*)?\s*$/
const matchAnd = /^\s*and\s+(?:(?::(\w+))|(?:(\w+)(?:::(\w+))?))(?:\((.*)\))?\s*(?:=>\s*(\w+))?\s*(?:#\s*(.*))?\s*$/
const matchValidator = /^(\w+)(?:::(\w+))?(?:\s*\((.*)?\))?\s*$/




// translate all rules and feed namespace with C++
function parse() {
   let stack = new Stack(1)
	let lineIndex = 0
   let name = ''
   let variant = ''
   let variables = {}
   let firstCase = true
   let childIndex = 0
   let match = null

   // ends the last block
   function endLastBlock(endWithThrow = true) {
      if (!name) return
      if (!endWithThrow)
         throw `Missing end-of-block throw for ${name}::${variant}`
      namespace[name + variant].code = stack.consume()
      name = variant = ''
   }


   // Let's start parsing / generating!
   for (let line of lines) {
      lineIndex++
      line = line.trimEnd()
      if (!line) continue

      // -- new case
      if (match = matchCase.exec(line)) {
         let [, variable, token, category, args, assignTo] = match

         if (token == 'Any') {
				if (args)
					throw `Error line ${lineIndex} : cannot pass arguments to "Any" :\n${line}`
            stack.push(`Validate::Any(_0);`)
         }
         else {
				args = args ? `_0, ${args}` : '_0'
            category = category ? '__' + category : ''

            if (firstCase)
               stack.newBlock(`switch (_0->type()) {`, '}')
            else
               stack.closeBlock()

            stack.newBlock(`case Token::${token}:`, '\treturn;\n')
                 .push(`<<<${token}${category}>>>(${args});`)
         }

         if (assignTo)
            stack.push(`${assignTo} = _0;`)

         firstCase = false
         childIndex = 1
      }

      // -- next child condition
      else if (match = matchAnd.exec(line)) {
         if (firstCase)
            throw `Expected a 'case' before 'and' at line ${lineIndex}`
         let [, variable, token, category, args, assignTo, error] = match

         args = `_${childIndex}` + (args ? `, ${args}` : '')
         if (token != 'Any' && !error)
            throw `Missing error message at line ${lineIndex} :\n${line}`

         stack.push('')

         if (token == 'Any') {
            stack.push(`Validate::Any(${args});`)
         }
         else if (token) {
            category = category ? '__' + category : ''
            stack.push(`if (_${childIndex}->type() != Token::${token})`)
                 .push(`\tthrow "${error}";`)
                 .push(`<<<${token}${category}>>>(${args});`)
         }
         else if (variable) {
            stack.newBlock(`switch (_${childIndex}->type()) {`, '}')

            for (let [token, category] of variables[variable]) {
               category = category ? '__' + category : ''
               stack.push(`case Token::${token}:`)
                    .push(`   <<<${token}${category}>>>(${args});`)
                    .push(`   break;`)
            }
            stack.push(`default:`)
                 .push(`\tthrow "${error}";`)
                 .closeBlock()
         }

         if (assignTo)
            stack.push(`${assignTo} = _${childIndex};`)
         childIndex++
      }

      // -- end of block
      else if (match = matchThrow.exec(line)) {
         let message = match[1]
         stack.closeBlock()
              .push('default:')
              .push(`\tthrow "${message}";`)
         endLastBlock()
      }

      // -- new variable
      else if (match = matchVariable.exec(line)) {
         endLastBlock(false)
         let [, name, value] = match
         if (name in variables)
            throw `Variable ${name} already exists`
         let values = value.split('|').map(e => e.split('::').map(e => e.trim()))
         variables[name] = values
      }

      // -- new validator block
      else if (match = matchValidator.exec(line)) {
         endLastBlock(false)
         let args
         ;[, name, variant, args] = match
         variant = variant ? '__' + variant : ''
         let fullName = name + variant
         let childrenCount = getChildrenCount(name)
			if (fullName in namespace)
				throw `Validator block ${fullName} already declared`

			args ??= ''
         let argList = args.split(',').map(a => a.trim()).filter(a => a)
         args = `Node* node`
         for (let arg of argList)
            args += `, Node*& ${arg}`

         let declaration = `void ${name}${variant}(${args})`
         namespace[fullName] = {
            declaration: `${declaration};`,
            code: '',
         }

         stack.newBlock(`${declaration} {`, '}')
			for (let i=0; i < childrenCount; i++)
         	stack.push(`Node* _${i} = node->children[${i}];`)


         if (name in data.nodes) {
            for (let property of data.nodes[name])
               stack.push(`Node*& ${property} = ((${name}Node*) node)->data.${property};`)
         }


			stack.push('')
         firstCase = true
      }

      // -- bad syntax
      else {
         throw `Bad syntax at line ${lineIndex} :\n\`${line}\``
      }
   }

   endLastBlock(false)
}

export default parse
