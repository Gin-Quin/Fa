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
let executableInput = path.join('src', 'cli', 'main.cpp')
let executableOutput = path.join('bin', 'fa')
if (isWindows) executableOutput += '.exe'


async function build(compiler = 'c++') {
	try {
		compiler = {
			'c++': 'c++',
			'clang++': 'clang++',
			'clang': 'clang++',
			'gcc': 'g++',
			'g++': 'g++',
			'cl': 'cl'
		}[compiler]

		if (!compiler) {
			print `Unknown compiler : ${process.argv[2]}`
			return
		}

		let compilation = `${compiler} -std=c++17 ${executableInput} -o ${executableOutput}`
		if (compiler == 'cl')
			compilation = `cl ${input} /std:c++17 /EHsc /utf-8`

		print `[brightBlue.bold]  [── Generating library ──]`
		timer.start()
		generate(srcFile, libFile)
		timer.print()

		print `\n[brightBlue.bold]  [── Compiling binary ──]`
		if (compiler == 'cl') {
			timer.start()
			await exec(compilation, false, false)
			timer.stop()
			let outputFile = basename(executableInput)
			outputFile = outputFile.slice(0, outputFile.lastIndexOf('.'))
			fs.renameSync(outputFile + '.exe', executableOutput)
			fs.unlinkSync(outputFile + '.obj')
		}
		else {
			timer.start()
			await exec(compilation)
			timer.stop()
		}
		print `[yellow:<] ${executableInput}`
		print `[brightBlue:>] [bold]${executableOutput}`
		timer.print()

		// print `\n[italic.brightBlue.bold]  [── Running test ──]`
		// timer.start()
		// await exec(executableOutput)
		// timer.print()

		print `\n[brightGreen.bold]  [── Done ──]\n`
	}

	catch (error) {
		console.error(error.message)

		print `\n[brightRed.bold]  [── An error occured ──]\n`
	}
}

module.exports = build

if (require.main === module)
	build()
