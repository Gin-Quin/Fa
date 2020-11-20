/**
* Parse the data/ruls.yaml.ex file and return a simple AST
* This AST can be sent to rules/emit.js to generate C++ functions
*/
const YAML = require('js-yaml')
const ex = require('../ex')
const getChildrenCount = require('../getChildrenCount')

const isFunction = property => property.trim().endsWith(')')
const isThrow = rule => typeof rule == 'string' && rule.startsWith('throw "')

const parseFunctionSignature = signature => {
	let argsStart = signature.indexOf('(')
	let argsEnd = signature.lastIndexOf(')')
	if (argsStart == -1) throw "Missing '(' in function signature"
	if (argsEnd == -1) throw "Missing ')' in function signature"

	let name = signature.slice(0, argsStart).trim()
	let rawArguments = signature.slice(argsStart + 1, argsEnd)
	let arguments = rawArguments.replace(/  +/g, ' ').split(' ')
	return {name, arguments}
}

const parse = (data, content) => {
	let raw = YAML.safeLoad(ex(data, content))
	console.log("raw", raw)

	// now we refine the data to facilitate the emmiter work
	let refined = {
		variables: {},
		functions: {},
	}

	for (let property in raw) {
		if (isFunction(property)) {  // we declare a function
			let {name, arguments} = parseFunctionSignature(property)
			console.log("name:", name, "arguments:", arguments)
			let [token] = name.split('_')
			refined.functions[name] = { arguments, token }

			for (let rule of raw[property]) {
				if (isThrow(rule))
					refined.functions[name].error = rule
				else {
					refined.functions[name].children = []
						.concat(rule)
						.map(element => {
							element = element.trim()
							console.log("element:", element)
							if (isFunction(element))
								return parseFunctionSignature(element)
							if (element in refined.variables)
								return element
							throw `Trying to access variable '${element}' but it has not been set`
						})
				}
			}
		}

		else {  // we declare a variable
			let elements = []

			for (let element of raw[property]) {
				element = element.trim()
				if (isFunction(element))
					elements.push(parseFunctionSignature(elements).name)
				else if (element in refined.variables)
					elements.push(...refined.variables[element])
				else
					throw `Trying to access variable '${element}' but it has not been set`
			}
			
			refined.variables[property] = elements
		}

	}


	return refined
}


module.exports = parse
