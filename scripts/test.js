#!/usr/bin/env node
const exec = require('./utils/exec')
const timer = require('./utils/timer')
const generate = require('./utils/generate')
const print = require('cute-print')
const fs = require('fs')
const path = require('path')
const { basename } = path

const isWindows = process.platform == 'win32'

let srcFile = path.join('src', 'fa.hpp')
let libFile = path.join('lib', 'fa.hpp')
let testInput = path.join('tests', 'test.cpp')
let testOutput = path.resolve('test')
if (isWindows) testOutput += '.exe'

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

let compilation = `${compiler} -std=c++17 ${testInput} -o ${testOutput}`
if (compiler == 'cl')
	compilation = `cl ${input} /std:c++17 /EHsc /utf-8`


;(async() => {

	try {
		print `[italic.brightBlue.bold]  [-- Generating source files from templates --]`
		timer.start()
		generate(srcFile, libFile, false)
		timer.print()
	
		print `\n[italic.brightBlue.bold]  [-- Compiling test file --]`
		if (compiler == 'cl') {
			timer.start()
			await exec(compilation, false, false)
			timer.stop()
			let outputFile = basename(testInput)
			outputFile = outputFile.slice(0, outputFile.lastIndexOf('.'))
			fs.renameSync(outputFile + '.exe', testOutput)
			fs.unlinkSync(outputFile + '.obj')
		}
		else {
			timer.start()
			await exec(compilation)
			timer.stop()
		}
		print `[yellow:<] [italic]${testInput}`
		// print `[brightBlue:>] [bold]${testOutput}`
		timer.print()

		print `\n[italic.brightBlue.bold]  [-- Running test --]`
		timer.start()
		await exec(testOutput)
		timer.print()
	
		print `\n[italic.brightGreen.bold]  [-- Done --]\n`
		fs.unlinkSync(testOutput)
	}

	catch (error) {
		console.error(error.message)

		print `\n[italic.brightRed.bold]  [-- An error occured --]\n`
	}

})()
