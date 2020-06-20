#!/usr/bin/env node
const { exec, timer } = require('./utils')
const print = require('cute-print')

const input = 'compiler/test/generate.cpp'
const output = 'bin/fa'

;(async() => {

	try {
		print `[italic.brightBlue.bold]  [-- Generating source files from templates --]`
		timer.start()
		require('./generate')
		timer.print()
	
		print `\n[italic.brightBlue.bold]  [-- Compiling source --]`
		print `[yellow:<] [italic]${input}`
		timer.start()
		await exec(`c++ -std=c++17 ${input} -o ${output}`)
		timer.stop()
		print `[brightBlue:>] [bold]${output}`
		timer.print()
	
		print `\n[italic.brightBlue.bold]  [-- Run executable --]`
		timer.start()
		await exec(`./fa`)
		timer.print()
	
		print `\n[italic.brightGreen.bold]  [-- Done --]\n`
	}

	catch (error) {
		console.error(error)

		print `\n[italic.brightRed.bold][An error occured]\n`
	}

})()
