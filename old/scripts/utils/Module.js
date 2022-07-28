
import path from 'path'
import fs from 'fs'
import print from 'cute-print'
import transform from './transform.js'


const nextInclude = /^\s*#include\s+"(.*?)"\s*$/m
const nextGlobalInclude = /^\s*#include\s+<(.*?)>\s*$/m


export default class Module {
	static instances = {}
	static visited = null

	path = ''
	directory = ''
	name = ''
	content = ''
	dependencies = []
	globalDependencies = new Set

	constructor(filename) {
		Module.instances[filename] = this
		this.path = filename
		this.directory = path.dirname(filename)
		this.name = filename.substr(this.directory.length + 1)
		this.content = transform(fs.readFileSync(filename, 'utf8'))
		this.parse()
	}

	// parse the dependencies and create the corresponding modules
	parse() {
		for (let match; match = nextGlobalInclude.exec(this.content);) {
			this.globalDependencies.add(match[1])
			this.content = this.content.substr(0, match.index) + this.content.substr(match.index + match[0].length)
		}

		let i = 0
		for (let match; match = nextInclude.exec(this.content);) {
			let subPath = path.join(this.directory, match[1])
			let subModule = (subPath in Module.instances) ? Module.instances[subPath] : new Module(subPath)
			this.dependencies.push(subModule)

			// we add the global dependencies to the parent
			for (let g of subModule.globalDependencies)
				this.globalDependencies.add(g)
			
			// and we replace the include with an anchor in the content
			this.content = this.content.substr(0, match.index) + `<<[[${i++}]]>>` + this.content.substr(match.index + match[0].length)
		}
	}

	bundle(namespace='') {
		Module.visited = new Set
		let result = this._bundle()
		if (namespace)
			result = `\nnamespace ${namespace} {\n` + result + '\n}'
		result = [...this.globalDependencies].map(g => `#include <${g}>\n`).join('') + result
		// console.log(result)
		return result
	
	}

	_bundle() {
		if (Module.visited.has(this)) {
			// print `[white] x ${this.path}`
			return ''
		}
		Module.visited.add(this)

		let result = this.content
		print `[yellow]< [white]${this.path}`
		// print `[brightYellow]  [[BEFORE]]`
		// console.log(result+'\n')
		
		let i = 0
		for (let dependency of this.dependencies) {
			let anchor = `<<[[${i++}]]>>`
			let index = result.indexOf(anchor)
			result = result.substr(0, index) + dependency._bundle() + result.substr(index + anchor.length)
		}

		// print `[brightYellow]  [[AFTER]]`
		// console.log(result + '\n')
		return result
	}
}


