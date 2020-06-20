#!/usr/bin/env node
const { exec, timer } = require('./utils')
const print = require('cute-print')
const fs = require('fs')
const path = require('path')
const { basename } = path

const isWindows = process.platform == 'win32'

const input = path.join('compiler', 'test', 'generate.cpp')
let output = path.join('bin', 'fa')
if (isWindows) output += '.exe'

const compiler = {
	'c++': 'c++',
	'clang++': 'clang++',
	'clang': 'clang++',
	'gcc': 'g++',
	'g++': 'g++',
	'cl': 'cl'
}[process.argv[2] || 'c++']

if (!compiler) {
	print `Unknown compiler : ${process.argv[2]}`
	return
}

let compilation = `${compiler} -std=c++17 ${input} -o ${output} -Ofast`
if (compiler == 'cl')
	compilation = `cl ${input} /std:c++17 /EHsc /utf-8 /O2`


;(async() => {

	try {
		print `[italic.brightBlue.bold]  [-- Generating source files from templates --]`
		timer.start()
		require('./generate')
		timer.print()
	
		print `\n[italic.brightBlue.bold]  [-- Compiling source --]`
		timer.start()
		if (compiler == 'cl') {
			await exec(compilation, false, false)
			timer.stop()
			let outputFile = basename(input)
			outputFile = outputFile.slice(0, outputFile.lastIndexOf('.'))
			fs.renameSync(outputFile + '.exe', output)
			fs.unlinkSync(outputFile + '.obj')
		}
		else {
			await exec(compilation)
			timer.stop()
		}
		print `[yellow:<] [italic]${input}`
		print `[brightBlue:>] [bold]${output}`
		timer.print()
	
		print `\n[italic.brightGreen.bold]  [-- Done --]\n`
	}

	catch (error) {
		console.error(error.message)

		print `\n[italic.brightRed.bold]  [-- An error occured --]\n`
	}

})()
